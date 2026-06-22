//! Shared `RenderCtx` builders for render unit tests. Test-only.

use crate::env::{ImageBackend, RenderCtx};

/// Baseline test context: TTY on, color off, width 60, placeholder images.
pub(crate) fn base() -> RenderCtx {
    RenderCtx {
        is_tty: true,
        use_color: false,
        width: 60,
        image_backend: ImageBackend::Placeholder,
        code_theme: "base16-ocean.dark".into(),
    }
}

/// Baseline with an overridden width.
pub(crate) fn width(width: usize) -> RenderCtx {
    RenderCtx { width, ..base() }
}

/// Baseline with an overridden `use_color`.
pub(crate) fn color(use_color: bool) -> RenderCtx {
    RenderCtx {
        use_color,
        ..base()
    }
}

/// Baseline with an overridden image backend.
pub(crate) fn backend(image_backend: ImageBackend) -> RenderCtx {
    RenderCtx {
        image_backend,
        ..base()
    }
}
