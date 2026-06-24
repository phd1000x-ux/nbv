//! Output seam shared by every renderer. `BoxedSink` draws the framed
//! `│ … │` lines used inside notebook cells; `BareSink` (Task 2) emits
//! word-wrapped, border-free document lines.

use std::io::{self, Write};

use crate::env::RenderCtx;
use crate::render::frame;

/// A renderer's only way to emit a line. `text_line` is prose that a bare
/// sink may reflow; `raw_line` is pre-formatted (code, table borders, rules)
/// and is never reflowed.
pub trait LineSink {
    fn text_line(&mut self, content: &str, ctx: &RenderCtx) -> io::Result<()>;
    fn raw_line(&mut self, content: &str, ctx: &RenderCtx) -> io::Result<()>;
}

/// Framed output: every line becomes `│ {content} │`, padded/truncated to
/// `ctx.width`. This is exactly the legacy `frame::wrap_line` behavior, so the
/// notebook path is unchanged.
pub struct BoxedSink<'w> {
    w: &'w mut dyn Write,
}

impl<'w> BoxedSink<'w> {
    pub fn new(w: &'w mut dyn Write) -> Self {
        BoxedSink { w }
    }
}

impl LineSink for BoxedSink<'_> {
    fn text_line(&mut self, content: &str, ctx: &RenderCtx) -> io::Result<()> {
        frame::wrap_line(content, ctx, self.w)
    }
    fn raw_line(&mut self, content: &str, ctx: &RenderCtx) -> io::Result<()> {
        frame::wrap_line(content, ctx, self.w)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> RenderCtx {
        crate::render::test_support::base()
    }

    #[test]
    fn boxed_text_line_matches_wrap_line() {
        let mut a = Vec::new();
        BoxedSink::new(&mut a).text_line("hello", &ctx()).unwrap();
        let mut b = Vec::new();
        frame::wrap_line("hello", &ctx(), &mut b).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn boxed_raw_line_matches_wrap_line() {
        let mut a = Vec::new();
        BoxedSink::new(&mut a)
            .raw_line("│ inner │", &ctx())
            .unwrap();
        let mut b = Vec::new();
        frame::wrap_line("│ inner │", &ctx(), &mut b).unwrap();
        assert_eq!(a, b);
    }
}
