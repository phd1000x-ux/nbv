use crate::env::RenderCtx;
use std::io::{self, Write};

pub mod iterm;
pub mod kitty;
pub mod placeholder;
pub mod png_info;

pub trait ImageRenderer {
    fn render(
        &self,
        b64: &str,
        cell_idx: usize,
        out_idx: usize,
        ctx: &RenderCtx,
        w: &mut dyn Write,
    ) -> io::Result<()>;
}

pub fn dispatch(
    b64: &str,
    cell_idx: usize,
    out_idx: usize,
    ctx: &RenderCtx,
    w: &mut dyn Write,
) -> io::Result<()> {
    use crate::env::ImageBackend;
    match ctx.image_backend {
        ImageBackend::Kitty => kitty::KittyRenderer.render(b64, cell_idx, out_idx, ctx, w),
        ImageBackend::ITerm2 => iterm::ITermRenderer.render(b64, cell_idx, out_idx, ctx, w),
        ImageBackend::Placeholder => {
            placeholder::PlaceholderRenderer.render(b64, cell_idx, out_idx, ctx, w)
        }
    }
}

#[cfg(test)]
mod dispatch_tests {
    use super::*;
    use crate::env::{ImageBackend, RenderCtx};

    fn ctx_with(b: ImageBackend) -> RenderCtx {
        crate::render::test_support::backend(b)
    }

    #[test]
    fn placeholder_dispatches_to_placeholder() {
        let mut buf = Vec::new();
        // "Z2FyYmFnZQ==" decodes to b"garbage" — valid base64, non-PNG
        dispatch(
            "Z2FyYmFnZQ==",
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
        let mut buf = Vec::new();
        dispatch("Zm9v", 0, 0, &ctx_with(ImageBackend::Kitty), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.starts_with("\x1b_G"));
    }

    #[test]
    fn iterm2_dispatches_to_iterm() {
        let mut buf = Vec::new();
        dispatch("Zm9v", 0, 0, &ctx_with(ImageBackend::ITerm2), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.starts_with("\x1b]1337;"));
    }
}
