use std::collections::HashMap;

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
}

impl Config {
    pub fn from_env() -> Self {
        let map: HashMap<String, String> = std::env::vars().collect();
        Self::from_map(&map)
    }

    pub fn from_map(map: &HashMap<String, String>) -> Self {
        let get = |k: &str| map.get(k).cloned();
        let parse = |k: &str, d: u64| get(k).and_then(|v| v.parse().ok()).unwrap_or(d);

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

        Config {
            port: parse("PORT", 3000) as u16,
            storage_enabled: get("STORAGE_ENABLED").as_deref() == Some("true"),
            scheme: get("OPENDAL_SCHEME").unwrap_or_else(|| "fs".into()),
            opendal_opts,
            public_base_url: get("PUBLIC_BASE_URL")
                .unwrap_or_else(|| "http://localhost:3000".into()),
            signing_key,
            url_ttl_secs: parse("URL_TTL_SECS", 3600),
            max_body_bytes: parse("MAX_BODY_BYTES", 8 * 1024 * 1024) as usize,
            max_html_bytes: parse("MAX_HTML_BYTES", 2 * 1024 * 1024) as usize,
            max_image_pages: parse("MAX_IMAGE_PAGES", 10) as usize,
            rate_limit_per_min: parse("RATE_LIMIT_PER_MIN", 60) as u32,
        }
    }
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
}
