use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfRequest {
    pub html: String,
    #[serde(default)]
    pub page: PageOptions,
    pub header: Option<String>,
    pub footer: Option<String>,
    pub filename: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PageOptions {
    pub format: Option<String>,
    #[serde(default)]
    pub landscape: bool,
    pub margin: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PdfInlineResponse {
    pub filename: String,
    pub pdf: String,
}

#[derive(Debug, Serialize)]
pub struct PdfUrlResponse {
    pub filename: String,
    #[serde(rename = "downloadUrl")]
    pub download_url: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageRequest {
    pub pdf_base64: String,
    #[serde(default = "default_format")]
    pub format: ImageFormat,
    pub scale: Option<f32>,
    #[serde(default)]
    pub pages: String,
}

fn default_format() -> ImageFormat {
    ImageFormat::Png
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    Png,
    Jpeg,
}

impl ImageFormat {
    pub fn ext(self) -> &'static str {
        match self {
            ImageFormat::Png => "png",
            ImageFormat::Jpeg => "jpeg",
        }
    }
}

impl ImageRequest {
    pub fn validate(&self) -> Result<(), String> {
        Ok(())
    }
    pub fn scale_factor(&self) -> f32 {
        self.scale.unwrap_or(1.0)
    }
}

#[derive(Debug, Serialize)]
pub struct ImageItem {
    pub page: usize,
    pub width: u32,
    pub height: u32,
    pub format: ImageFormat,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(rename = "downloadUrl", skip_serializing_if = "Option::is_none")]
    pub download_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ImagesResponse {
    pub images: Vec<ImageItem>,
}

pub fn css_len_to_points(s: &str) -> Result<f64, String> {
    let s = s.trim();
    let (num, unit) = s
        .find(|c: char| c.is_ascii_alphabetic())
        .map(|i| (s[..i].trim(), &s[i..]))
        .ok_or_else(|| format!("missing unit in length '{s}'"))?;
    let n: f64 = num
        .parse()
        .map_err(|_| format!("invalid number in '{s}'"))?;
    let pts = match unit {
        "pt" => n,
        "px" => n * 72.0 / 96.0,
        "in" => n * 72.0,
        "cm" => n * 72.0 / 2.54,
        "mm" => n * 72.0 / 25.4,
        other => return Err(format!("unsupported unit '{other}'")),
    };
    Ok(pts)
}

pub fn parse_pages(spec: &str, total: usize) -> Result<Vec<usize>, String> {
    if spec.trim().is_empty() {
        return Ok((1..=total).collect());
    }
    let mut raw = Vec::new();
    for part in spec.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((a, b)) = part.split_once('-') {
            let (a, b) = (parse_one(a)?, parse_one(b)?);
            if a > b {
                return Err(format!("invalid range '{part}'"));
            }
            for p in a..=b {
                raw.push(p);
            }
        } else {
            raw.push(parse_one(part)?);
        }
    }
    if let Some(&p) = raw.iter().find(|&&p| p < 1 || p > total) {
        return Err(format!("page {p} out of range 1..={total}"));
    }
    let mut seen = std::collections::HashSet::new();
    let out: Vec<usize> = raw.into_iter().filter(|p| seen.insert(*p)).collect();
    Ok(out)
}

fn parse_one(s: &str) -> Result<usize, String> {
    s.trim().parse().map_err(|_| format!("invalid page '{s}'"))
}

pub fn sanitize_filename(name: &str) -> String {
    if name.trim().is_empty() {
        return "document.pdf".into();
    }
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pdf_request_minimal_ok() {
        let r: PdfRequest = serde_json::from_str(r#"{"html":"<h1>x</h1>"}"#).unwrap();
        assert_eq!(r.html, "<h1>x</h1>");
        assert!(!r.page.landscape);
        assert!(r.filename.is_none());
    }

    #[test]
    fn pdf_request_rejects_browser_only_field() {
        let err = serde_json::from_str::<PdfRequest>(r#"{"html":"x","scale":2}"#);
        assert!(err.is_err(), "scale must be rejected");
    }

    #[test]
    fn margin_to_points_parses_units() {
        assert!((css_len_to_points("1in").unwrap() - 72.0).abs() < 0.01);
        assert!((css_len_to_points("72pt").unwrap() - 72.0).abs() < 0.01);
        assert!((css_len_to_points("2.54cm").unwrap() - 72.0).abs() < 0.1);
        assert!(css_len_to_points("nonsense").is_err());
    }

    #[test]
    fn pages_spec_parses_ranges() {
        assert_eq!(parse_pages("1-3,5", 10).unwrap(), vec![1, 2, 3, 5]);
        assert_eq!(parse_pages("", 4).unwrap(), vec![1, 2, 3, 4]);
        assert!(parse_pages("9-12", 5).is_err());
    }

    #[test]
    fn pages_dedup_preserves_request_order() {
        assert_eq!(parse_pages("2,2,1", 10).unwrap(), vec![2, 1]);
    }

    #[test]
    fn filename_is_sanitized() {
        assert_eq!(
            sanitize_filename("My Report (final).pdf"),
            "My_Report__final_.pdf"
        );
        assert_eq!(sanitize_filename(""), "document.pdf");
    }

    #[test]
    fn image_request_defaults_scale_to_one() {
        let r: ImageRequest = serde_json::from_str(r#"{"pdf_base64":"AA=="}"#).unwrap();
        assert_eq!(r.scale_factor(), 1.0);
    }

    #[test]
    fn image_request_rejects_unknown_dpi_field() {
        let r: Result<ImageRequest, _> = serde_json::from_str(r#"{"pdf_base64":"AA==","dpi":144}"#);
        assert!(r.is_err());
    }
}
