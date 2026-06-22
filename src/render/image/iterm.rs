use std::io::{self, Write};

use crate::env::RenderCtx;
use crate::render::image::ImageRenderer;

pub struct ITermRenderer;

impl ImageRenderer for ITermRenderer {
    fn render(
        &self,
        b64: &str,
        _cell_idx: usize,
        _out_idx: usize,
        _ctx: &RenderCtx,
        w: &mut dyn Write,
    ) -> io::Result<()> {
        writeln!(w, "\x1b]1337;File=inline=1:{}\x07", b64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::{ImageBackend, RenderCtx};
    use crate::render::image::ImageRenderer;

    fn ctx() -> RenderCtx {
        crate::render::test_support::backend(ImageBackend::ITerm2)
    }

    #[test]
    fn emits_osc_1337_with_base64() {
        let b64 = "iVBORw0KGgoAAAANS";
        let mut buf = Vec::new();
        ITermRenderer.render(b64, 0, 0, &ctx(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.starts_with("\x1b]1337;File=inline=1"));
        assert!(s.contains(b64)); // pass-through verbatim
        assert!(s.contains("\x07"));
    }
}
