use ironpress::{HtmlConverter, Margin, PageSize};

use crate::models::dto::{PageOptions, css_len_to_points};

pub struct PdfBuildOptions {
    pub page: PageOptions,
    pub header: Option<String>,
    pub footer: Option<String>,
}

pub fn render_html(html: &str, opts: &PdfBuildOptions) -> Result<Vec<u8>, String> {
    let mut conv = HtmlConverter::new();

    if opts.page.format.is_some() || opts.page.landscape {
        let mut size = match &opts.page.format {
            Some(fmt) => page_size_for(fmt)?,
            None => PageSize::A4,
        };
        if opts.page.landscape {
            size = PageSize::new(size.height, size.width);
        }
        conv = conv.page_size(size);
    }

    if let Some(margin) = &opts.page.margin {
        let pts = css_len_to_points(margin)? as f32;
        conv = conv.margin(Margin::uniform(pts));
    }

    if let Some(h) = &opts.header {
        conv = conv.header(h);
    }
    if let Some(f) = &opts.footer {
        conv = conv.footer(f);
    }

    conv.convert(html)
        .map_err(|e| format!("html_to_pdf failed: {e:?}"))
}

fn page_size_for(fmt: &str) -> Result<PageSize, String> {
    match fmt.to_ascii_lowercase().as_str() {
        "a4" => Ok(PageSize::A4),
        "letter" => Ok(PageSize::LETTER),
        "legal" => Ok(PageSize::LEGAL),
        other => Err(format!("unsupported page format '{other}'")),
    }
}

pub fn html_to_pdf(html: &str) -> Result<Vec<u8>, String> {
    ironpress::html_to_pdf(html).map_err(|e| format!("html_to_pdf failed: {e:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::dto::PageOptions;

    #[test]
    fn renders_with_options() {
        let opts = PdfBuildOptions {
            page: PageOptions {
                format: Some("A4".into()),
                landscape: false,
                margin: Some("1cm".into()),
            },
            header: None,
            footer: None,
        };
        let pdf = render_html("<h1>hello</h1>", &opts).expect("render");
        assert!(pdf.starts_with(b"%PDF"));
    }

    #[test]
    fn unknown_format_is_error() {
        let opts = PdfBuildOptions {
            page: PageOptions {
                format: Some("Tabloid-XL".into()),
                landscape: false,
                margin: None,
            },
            header: None,
            footer: None,
        };
        assert!(render_html("<h1>x</h1>", &opts).is_err());
    }
}
