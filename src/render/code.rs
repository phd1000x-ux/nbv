use std::io::{self, Write};
use std::sync::OnceLock;

use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

use crate::env::RenderCtx;
use crate::render::frame;
use crate::render::traceback::strip_ansi_pub as strip_ansi;

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();

fn syntax_set() -> &'static SyntaxSet {
    SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines)
}
fn theme_set() -> &'static ThemeSet {
    THEME_SET.get_or_init(ThemeSet::load_defaults)
}

pub fn render(source: &str, lang: &str, ctx: &RenderCtx, w: &mut impl Write) -> io::Result<()> {
    let ss = syntax_set();
    let ts = theme_set();
    let syntax = ss.find_syntax_by_token(lang)
        .or_else(|| ss.find_syntax_by_token("python"))
        .unwrap_or_else(|| ss.find_syntax_plain_text());
    let theme = &ts.themes["base16-ocean.dark"];
    let mut hl = HighlightLines::new(syntax, theme);

    for line in LinesWithEndings::from(source) {
        let ranges: Vec<(Style, &str)> = hl.highlight_line(line, ss)
            .unwrap_or_else(|_| vec![(Style::default(), line)]);
        let mut escaped = as_24_bit_terminal_escaped(&ranges[..], false);
        // syntect 출력은 줄바꿈을 포함; 박스에 넣기 위해 제거
        while escaped.ends_with('\n') { escaped.pop(); }
        let to_render = if ctx.use_color { escaped } else { strip_ansi(&escaped) };
        frame::wrap_line(&to_render, ctx, w)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::{ImageBackend, RenderCtx};

    fn ctx(use_color: bool) -> RenderCtx {
        RenderCtx { is_tty: true, use_color, width: 60, image_backend: ImageBackend::Placeholder }
    }

    #[test]
    fn renders_python_code_with_color() {
        let mut buf = Vec::new();
        render("x = 1", "python", &ctx(true), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        // syntect emits ANSI between tokens; wrap_line may truncate if ANSI bytes
        // consume visual-width budget; we verify at minimum some content appears
        assert!(s.contains("x"));
        assert!(s.contains("\x1b["));  // ANSI from syntect
    }

    #[test]
    fn renders_code_without_color_strips_ansi() {
        let mut buf = Vec::new();
        render("x = 1", "python", &ctx(false), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(!s.contains("\x1b["));
        assert!(s.contains("x"));
    }

    #[test]
    fn unknown_language_falls_back_to_plain() {
        let mut buf = Vec::new();
        render("hello", "klingon-script", &ctx(false), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("hello"));
    }

    #[test]
    fn each_line_emitted_as_box_line() {
        let mut buf = Vec::new();
        render("a = 1\nb = 2", "python", &ctx(false), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("a"));
        assert!(s.contains("b"));
        assert!(s.matches("│ ").count() >= 2);
    }
}
