use base64::Engine;
use std::io::{self, Write};

use crate::env::RenderCtx;
use crate::render::frame;
use crate::render::image::{png_info, ImageRenderer};

pub struct PlaceholderRenderer;

impl ImageRenderer for PlaceholderRenderer {
    fn render(
        &self,
        b64: &str,
        cell_idx: usize,
        out_idx: usize,
        ctx: &RenderCtx,
        w: &mut dyn Write,
    ) -> io::Result<()> {
        let bytes = match base64::engine::general_purpose::STANDARD.decode(b64) {
            Ok(b) => b,
            Err(_) => return frame::wrap_line("(image decode failed)", ctx, w),
        };
        let (size_label, kb) = match png_info::dimensions(&bytes) {
            Some((wd, ht)) => (format!("PNG {}×{}", wd, ht), bytes.len() / 1024),
            None => ("image (unknown format)".to_string(), bytes.len() / 1024),
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

    const ONE_PIXEL: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==";

    fn ctx() -> RenderCtx {
        RenderCtx {
            is_tty: true,
            use_color: false,
            width: 60,
            image_backend: ImageBackend::Placeholder,
            code_theme: "base16-ocean.dark".into(),
        }
    }

    #[test]
    fn shows_dimensions_for_valid_png() {
        let mut buf = Vec::new();
        PlaceholderRenderer
            .render(ONE_PIXEL, 3, 0, &ctx(), &mut buf)
            .unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("1×1") || s.contains("1x1") || s.contains("1") && s.contains("PNG"));
        assert!(s.contains("cell #3"));
    }

    #[test]
    fn unknown_format_for_valid_base64_non_png() {
        // "Z2FyYmFnZQ==" → b"garbage" (유효 base64, PNG 아님)
        let mut buf = Vec::new();
        PlaceholderRenderer
            .render("Z2FyYmFnZQ==", 0, 0, &ctx(), &mut buf)
            .unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("unknown format"));
    }

    #[test]
    fn decode_failed_for_invalid_base64() {
        // '!' 와 '-' 는 STANDARD base64 알파벳이 아님 → decode 실패
        let mut buf = Vec::new();
        PlaceholderRenderer
            .render("!!!not-base64!!!", 0, 0, &ctx(), &mut buf)
            .unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("image decode failed"));
    }
}
