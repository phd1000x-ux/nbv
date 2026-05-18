use std::io::{self, Write};

use crate::env::RenderCtx;
use crate::render::frame;

pub fn render(text: &str, ctx: &RenderCtx, w: &mut impl Write) -> io::Result<()> {
    for line in text.split_inclusive('\n') {
        let line = line.trim_end_matches('\n');
        frame::wrap_line(line, ctx, w)?;
    }
    if text.is_empty() {
        frame::wrap_line("", ctx, w)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::{ImageBackend, RenderCtx};

    fn ctx() -> RenderCtx {
        RenderCtx {
            is_tty: true,
            use_color: false,
            width: 30,
            image_backend: ImageBackend::Placeholder,
            code_theme: "base16-ocean.dark".into(),
            framed: true,
        }
    }

    #[test]
    fn renders_single_line_in_box_line() {
        let mut buf = Vec::new();
        render("hello", &ctx(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("hello"));
        assert!(s.starts_with("│ "));
    }

    #[test]
    fn renders_multiline_as_multiple_box_lines() {
        let mut buf = Vec::new();
        render("a\nb\nc", &ctx(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert_eq!(s.matches('\n').count(), 3);
    }

    #[test]
    fn trailing_newline_does_not_create_empty_line() {
        let mut buf = Vec::new();
        render("a\n", &ctx(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert_eq!(s.matches('\n').count(), 1);
    }
}
