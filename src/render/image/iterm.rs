use base64::Engine;
use std::io::{self, Write};

use crate::env::RenderCtx;
use crate::render::image::ImageRenderer;

pub struct ITermRenderer;

impl ImageRenderer for ITermRenderer {
    fn render(
        &self,
        png_bytes: &[u8],
        _cell_idx: usize,
        _out_idx: usize,
        _ctx: &RenderCtx,
        w: &mut dyn Write,
    ) -> io::Result<()> {
        let b64 = base64::engine::general_purpose::STANDARD.encode(png_bytes);
        writeln!(w, "\x1b]1337;File=inline=1:{}\x07", b64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::{ImageBackend, RenderCtx};
    use crate::render::image::ImageRenderer;

    fn ctx() -> RenderCtx {
        RenderCtx {
            is_tty: true,
            use_color: false,
            width: 60,
            image_backend: ImageBackend::ITerm2,
            code_theme: "base16-ocean.dark".into(),
        }
    }

    #[test]
    fn emits_osc_1337_with_base64() {
        let png = b"\x89PNG\r\n\x1a\nfake".to_vec();
        let mut buf = Vec::new();
        ITermRenderer.render(&png, 0, 0, &ctx(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.starts_with("\x1b]1337;File=inline=1"));
        assert!(s.contains(":"));
        assert!(s.contains("\x07"));
    }
}
