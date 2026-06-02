use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_syntax::Pdf;
use hayro::vello_cpu::color::palette::css::WHITE;
use hayro::{RenderCache, RenderSettings, render};

use crate::models::dto::ImageFormat;

pub struct RenderOptions {
    pub format: ImageFormat,
    pub scale: f32,
    pub pages: Vec<usize>,
}

pub struct RenderedImage {
    pub page: usize,
    pub width: u32,
    pub height: u32,
    pub bytes: Vec<u8>,
}

pub fn render_pdf(pdf_bytes: Vec<u8>, opts: &RenderOptions) -> Result<Vec<RenderedImage>, String> {
    let pdf = Pdf::new(pdf_bytes).map_err(|e| format!("failed to parse PDF: {e:?}"))?;
    let all = pdf.pages();
    let total = all.len();

    let cache = RenderCache::new();
    let interp = InterpreterSettings::default();

    let mut out = Vec::with_capacity(opts.pages.len());
    for &p in &opts.pages {
        let page = all
            .get(p - 1)
            .ok_or_else(|| format!("page {p} out of range 1..={total}"))?;
        let settings = RenderSettings {
            x_scale: opts.scale,
            y_scale: opts.scale,
            bg_color: WHITE,
            ..Default::default()
        };
        let pixmap = render(page, &cache, &interp, &settings);
        let png = pixmap
            .into_png()
            .map_err(|e| format!("failed to encode PNG: {e}"))?;

        let (width, height, bytes) = match opts.format {
            ImageFormat::Png => {
                let img = image::load_from_memory(&png).map_err(|e| format!("decode png: {e}"))?;
                (img.width(), img.height(), png)
            }
            ImageFormat::Jpeg => {
                let img = image::load_from_memory(&png).map_err(|e| format!("decode png: {e}"))?;
                let mut buf = std::io::Cursor::new(Vec::new());
                img.write_to(&mut buf, image::ImageFormat::Jpeg)
                    .map_err(|e| format!("encode jpeg: {e}"))?;
                (img.width(), img.height(), buf.into_inner())
            }
        };

        out.push(RenderedImage {
            page: p,
            width,
            height,
            bytes,
        });
    }
    Ok(out)
}

pub fn pdf_to_pngs(pdf_bytes: Vec<u8>, scale: f32) -> Result<Vec<Vec<u8>>, String> {
    let pdf = Pdf::new(pdf_bytes).map_err(|e| format!("failed to parse PDF: {e:?}"))?;
    let cache = RenderCache::new();
    let interp = InterpreterSettings::default();
    let mut pages = Vec::new();
    for page in pdf.pages().iter() {
        let settings = RenderSettings {
            x_scale: scale,
            y_scale: scale,
            bg_color: WHITE,
            ..Default::default()
        };
        let pixmap = render(page, &cache, &interp, &settings);
        pages.push(
            pixmap
                .into_png()
                .map_err(|e| format!("failed to encode PNG: {e}"))?,
        );
    }
    Ok(pages)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::dto::ImageFormat;

    fn sample_pdf() -> Vec<u8> {
        crate::services::html_to_pdf::html_to_pdf("<h1>page one</h1>").unwrap()
    }

    #[test]
    fn renders_png_pages() {
        let imgs = render_pdf(
            sample_pdf(),
            &RenderOptions {
                format: ImageFormat::Png,
                scale: 1.0,
                pages: vec![1],
            },
        )
        .unwrap();
        assert_eq!(imgs.len(), 1);
        assert_eq!(imgs[0].page, 1);
        assert!(imgs[0].width > 0 && imgs[0].height > 0);
        assert!(imgs[0].bytes.starts_with(&[0x89, b'P', b'N', b'G']));
    }

    #[test]
    fn renders_jpeg() {
        let imgs = render_pdf(
            sample_pdf(),
            &RenderOptions {
                format: ImageFormat::Jpeg,
                scale: 1.0,
                pages: vec![1],
            },
        )
        .unwrap();
        assert_eq!(&imgs[0].bytes[0..2], &[0xFF, 0xD8]);
    }
}
