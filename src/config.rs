use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct Config {
    pub port: u16,
    pub storage_enabled: bool,
    pub scheme: String,
    pub opendal_opts: HashMap<String, String>,
    pub public_base_url: String,
    pub signing_key: Vec<u8>,
    pub url_ttl_secs: u64,
    pub max_body_bytes: usize,
    pub max_html_bytes: usize,
    pub max_image_pages: usize,
    pub rate_limit_per_min: u32,
    pub max_concurrent_renders: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError {
    message: String,
}

impl ConfigError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ConfigError {}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let map: HashMap<String, String> = std::env::vars().collect();
        Self::try_from_map(&map)
    }

    pub fn from_map(map: &HashMap<String, String>) -> Self {
        Self::try_from_map(map).expect("invalid config")
    }

    pub fn try_from_map(map: &HashMap<String, String>) -> Result<Self, ConfigError> {
        let get = |k: &str| map.get(k).cloned();
        let parse = |k: &str, d: u64| parse_env(map, k, d);

        let mut opendal_opts = HashMap::new();
        for (k, v) in map {
            if let Some(rest) = k.strip_prefix("OPENDAL_") {
                let key = rest.to_ascii_lowercase();
                if key != "scheme" {
                    opendal_opts.insert(key, v.clone());
                }
            }
        }

        let signing_key = get("SIGNING_KEY")
            .map(|s| s.into_bytes())
            .unwrap_or_else(|| uuid::Uuid::new_v4().as_bytes().to_vec());

        Ok(Config {
            port: checked_u16(parse("PORT", 3000)?, "PORT")?,
            storage_enabled: get("STORAGE_ENABLED").as_deref() == Some("true"),
            scheme: get("OPENDAL_SCHEME").unwrap_or_else(|| "fs".into()),
            opendal_opts,
            public_base_url: get("PUBLIC_BASE_URL")
                .unwrap_or_else(|| "http://localhost:3000".into()),
            signing_key,
            url_ttl_secs: parse("URL_TTL_SECS", 3600)?,
            max_body_bytes: checked_usize(
                parse("MAX_BODY_BYTES", 8 * 1024 * 1024)?,
                "MAX_BODY_BYTES",
            )?,
            max_html_bytes: checked_usize(
                parse("MAX_HTML_BYTES", 2 * 1024 * 1024)?,
                "MAX_HTML_BYTES",
            )?,
            max_image_pages: checked_nonzero_usize(
                parse("MAX_IMAGE_PAGES", 10)?,
                "MAX_IMAGE_PAGES",
            )?,
            rate_limit_per_min: checked_nonzero_u32(
                parse("RATE_LIMIT_PER_MIN", 60)?,
                "RATE_LIMIT_PER_MIN",
            )?,
            max_concurrent_renders: checked_nonzero_usize(
                parse("MAX_CONCURRENT_RENDERS", 4)?,
                "MAX_CONCURRENT_RENDERS",
            )?,
        })
    }
}

fn parse_env<T>(map: &HashMap<String, String>, key: &str, default: T) -> Result<T, ConfigError>
where
    T: FromStr,
    T::Err: fmt::Display,
{
    match map.get(key) {
        Some(value) => value
            .parse()
            .map_err(|err| ConfigError::new(format!("{key} has invalid value '{value}': {err}"))),
        None => Ok(default),
    }
}

fn checked_u16(value: u64, key: &str) -> Result<u16, ConfigError> {
    u16::try_from(value).map_err(|_| ConfigError::new(format!("{key} must be <= {}", u16::MAX)))
}

fn checked_nonzero_u32(value: u64, key: &str) -> Result<u32, ConfigError> {
    let value = u32::try_from(value)
        .map_err(|_| ConfigError::new(format!("{key} must be <= {}", u32::MAX)))?;
    if value == 0 {
        return Err(ConfigError::new(format!("{key} must be greater than 0")));
    }
    Ok(value)
}

fn checked_usize(value: u64, key: &str) -> Result<usize, ConfigError> {
    usize::try_from(value).map_err(|_| ConfigError::new(format!("{key} is too large")))
}

fn checked_nonzero_usize(value: u64, key: &str) -> Result<usize, ConfigError> {
    let value = checked_usize(value, key)?;
    if value == 0 {
        return Err(ConfigError::new(format!("{key} must be greater than 0")));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_when_env_absent() {
        let c = Config::from_map(&std::collections::HashMap::new());
        assert!(!c.storage_enabled);
        assert_eq!(c.scheme, "fs");
        assert_eq!(c.url_ttl_secs, 3600);
        assert_eq!(c.max_body_bytes, 8 * 1024 * 1024);
        assert_eq!(c.max_html_bytes, 2 * 1024 * 1024);
        assert_eq!(c.max_image_pages, 10);
        assert_eq!(c.rate_limit_per_min, 60);
        assert_eq!(c.max_concurrent_renders, 4);
    }
    #[test]
    fn parses_opendal_passthrough_and_flags() {
        let mut m = std::collections::HashMap::new();
        m.insert("STORAGE_ENABLED".into(), "true".into());
        m.insert("OPENDAL_SCHEME".into(), "s3".into());
        m.insert("OPENDAL_BUCKET".into(), "mybucket".into());
        let c = Config::from_map(&m);
        assert!(c.storage_enabled);
        assert_eq!(c.scheme, "s3");
        assert_eq!(
            c.opendal_opts.get("bucket").map(String::as_str),
            Some("mybucket")
        );
    }

    #[test]
    fn rejects_out_of_range_numeric_config() {
        let mut m = std::collections::HashMap::new();
        m.insert("PORT".into(), "70000".into());
        assert!(Config::try_from_map(&m).is_err());

        m.clear();
        m.insert("RATE_LIMIT_PER_MIN".into(), "0".into());
        assert!(Config::try_from_map(&m).is_err());

        m.clear();
        m.insert("MAX_CONCURRENT_RENDERS".into(), "0".into());
        assert!(Config::try_from_map(&m).is_err());
    }
}
