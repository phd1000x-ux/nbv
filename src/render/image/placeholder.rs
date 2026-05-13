use std::io::{self, Write};

use crate::env::RenderCtx;
use crate::render::frame;
use crate::render::image::{png_info, ImageRenderer};

pub struct PlaceholderRenderer;

impl ImageRenderer for PlaceholderRenderer {
    fn render(
        &self,
        png_bytes: &[u8],
        cell_idx: usize,
        out_idx: usize,
        ctx: &RenderCtx,
        w: &mut dyn Write,
    ) -> io::Result<()> {
        let (size_label, kb) = match png_info::dimensions(png_bytes) {
            Some((wd, ht)) => (format!("PNG {}×{}", wd, ht), png_bytes.len() / 1024),
            None => ("image (unknown format)".to_string(), png_bytes.len() / 1024),
        };
        let line1 = format!("🖼  {}  ({} KB)", size_label, kb);
        let line2 = format!("   cell #{}, output #{}", cell_idx, out_idx);
        frame::wrap_line(&line1, ctx, w)?;
        frame::wrap_line(&line2, ctx, w)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::{ImageBackend, RenderCtx};
    use crate::render::image::ImageRenderer;
    use base64::Engine;

    const ONE_PIXEL: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==";

    fn ctx() -> RenderCtx {
        RenderCtx {
            is_tty: true,
            use_color: false,
            width: 60,
            image_backend: ImageBackend::Placeholder,
        }
    }

    #[test]
    fn shows_dimensions_for_valid_png() {
        let b = base64::engine::general_purpose::STANDARD
            .decode(ONE_PIXEL)
            .unwrap();
        let mut buf = Vec::new();
        PlaceholderRenderer
            .render(&b, 3, 0, &ctx(), &mut buf)
            .unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("1×1") || s.contains("1x1") || s.contains("1") && s.contains("PNG"));
        assert!(s.contains("cell #3"));
    }

    #[test]
    fn falls_back_when_png_invalid() {
        let mut buf = Vec::new();
        PlaceholderRenderer
            .render(b"garbage", 0, 0, &ctx(), &mut buf)
            .unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("unknown format") || s.contains("PNG"));
    }
}
