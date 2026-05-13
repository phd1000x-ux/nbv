use crate::env::RenderCtx;
use std::io::{self, Write};

pub mod iterm;
pub mod kitty;
pub mod placeholder;
pub mod png_info;

pub trait ImageRenderer {
    fn render(
        &self,
        png_bytes: &[u8],
        cell_idx: usize,
        out_idx: usize,
        ctx: &RenderCtx,
        w: &mut dyn Write,
    ) -> io::Result<()>;
}

pub fn dispatch(
    png_bytes: &[u8],
    cell_idx: usize,
    out_idx: usize,
    ctx: &RenderCtx,
    w: &mut dyn Write,
) -> io::Result<()> {
    use crate::env::ImageBackend;
    match ctx.image_backend {
        ImageBackend::Kitty => kitty::KittyRenderer.render(png_bytes, cell_idx, out_idx, ctx, w),
        ImageBackend::ITerm2 => iterm::ITermRenderer.render(png_bytes, cell_idx, out_idx, ctx, w),
        ImageBackend::Placeholder => {
            placeholder::PlaceholderRenderer.render(png_bytes, cell_idx, out_idx, ctx, w)
        }
    }
}

#[cfg(test)]
mod dispatch_tests {
    use super::*;
    use crate::env::{ImageBackend, RenderCtx};

    fn ctx_with(b: ImageBackend) -> RenderCtx {
        RenderCtx {
            is_tty: true,
            use_color: false,
            width: 60,
            image_backend: b,
        }
    }

    #[test]
    fn placeholder_dispatches_to_placeholder() {
        let mut buf = Vec::new();
        dispatch(
            b"garbage",
            0,
            0,
            &ctx_with(ImageBackend::Placeholder),
            &mut buf,
        )
        .unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("│")); // 박스 안에서 그려짐
    }

    #[test]
    fn kitty_dispatches_to_kitty() {
        let png = b"\x89PNG\r\n\x1a\nfake".to_vec();
        let mut buf = Vec::new();
        dispatch(&png, 0, 0, &ctx_with(ImageBackend::Kitty), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.starts_with("\x1b_G"));
    }

    #[test]
    fn iterm2_dispatches_to_iterm() {
        let png = b"\x89PNG\r\n\x1a\nfake".to_vec();
        let mut buf = Vec::new();
        dispatch(&png, 0, 0, &ctx_with(ImageBackend::ITerm2), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.starts_with("\x1b]1337;"));
    }
}
