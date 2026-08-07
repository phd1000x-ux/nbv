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
    // SECURITY: a notebook's image/png string may carry terminal escape
    // sequences instead of real base64. Round-trip through decode/encode so
    // only the canonical base64 alphabet reaches the protocol renderers
    // (iterm/kitty interpolate b64 verbatim into escape sequences). Invalid
    // base64 → empty payload (renders as a harmless placeholder).
    let clean = sanitize_base64(b64);
    match ctx.image_backend {
        ImageBackend::Kitty => kitty::KittyRenderer.render(&clean, cell_idx, out_idx, ctx, w),
        ImageBackend::ITerm2 => iterm::ITermRenderer.render(&clean, cell_idx, out_idx, ctx, w),
        ImageBackend::Placeholder => {
            placeholder::PlaceholderRenderer.render(&clean, cell_idx, out_idx, ctx, w)
        }
    }
}

/// Decode then re-encode `b64` so only canonical base64 alphabet survives.
/// Returns an empty string for input that is not valid standard base64.
fn sanitize_base64(b64: &str) -> String {
    use base64::Engine;
    match base64::engine::general_purpose::STANDARD.decode(b64) {
        Ok(bytes) => base64::engine::general_purpose::STANDARD.encode(&bytes),
        Err(_) => String::new(),
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

    // SECURITY: a notebook's image/png string may contain terminal escape
    // sequences (OSC/CSI) instead of real base64. dispatch must sanitize the
    // payload so injected escapes can't reach the terminal via the image
    // protocol. See iterm.rs / kitty.rs which interpolate b64 verbatim.
    #[test]
    fn dispatch_strips_injected_escapes_from_iterm2_payload() {
        let mut buf = Vec::new();
        // ESC ] 0 ; PWNED BEL  embedded among valid-looking base64 chars
        let malicious = "iVBORw\u{001b}]0;PWNED\u{0007}w0KGgo";
        dispatch(malicious, 0, 0, &ctx_with(ImageBackend::ITerm2), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(
            !s.contains("PWNED"),
            "injected OSC must not reach terminal output: {s:?}"
        );
    }

    #[test]
    fn dispatch_strips_injected_escapes_from_kitty_payload() {
        let mut buf = Vec::new();
        let malicious = "iVBORw\u{001b}]0;PWNED\u{0007}w0KGgo";
        dispatch(malicious, 0, 0, &ctx_with(ImageBackend::Kitty), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(
            !s.contains("PWNED"),
            "injected OSC must not reach terminal output: {s:?}"
        );
    }

    #[test]
    fn dispatch_preserves_valid_base64_payload() {
        // A legitimate PNG base64 string must still reach the renderer intact.
        let one_pixel = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==";
        let mut buf = Vec::new();
        dispatch(one_pixel, 0, 0, &ctx_with(ImageBackend::ITerm2), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(
            s.contains(one_pixel),
            "valid base64 must pass through: {s:?}"
        );
    }
}
