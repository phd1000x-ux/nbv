use std::io::{self, Write};
use std::sync::OnceLock;

use syntect::easy::HighlightLines;
use syntect::highlighting::{Color, Style, Theme, ThemeSet};
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
    let (bg, default_fg) = theme_palette(theme);

    for line in LinesWithEndings::from(source) {
        let ranges: Vec<(Style, &str)> = hl.highlight_line(line, ss)
            .unwrap_or_else(|_| vec![(Style::default(), line)]);
        let safe = sanitize_invisible_fg(ranges, bg, default_fg);
        let mut escaped = as_24_bit_terminal_escaped(&safe[..], false);
        // syntect 출력은 줄바꿈을 포함; 박스에 넣기 위해 제거
        while escaped.ends_with('\n') { escaped.pop(); }
        let to_render = if ctx.use_color { escaped } else { strip_ansi(&escaped) };
        frame::wrap_line(&to_render, ctx, w)?;
    }
    Ok(())
}

fn theme_palette(theme: &Theme) -> (Option<Color>, Color) {
    let bg = theme.settings.background;
    let default_fg = theme
        .settings
        .foreground
        .unwrap_or(Color { r: 220, g: 220, b: 220, a: 255 });
    (bg, default_fg)
}

/// syntect의 일부 default 테마(예: base16-*.dark)는 `./path` 같은 토큰의 fg를
/// 정확히 theme background 색으로 emit해서 화면에서 보이지 않는다. 이런 토큰만
/// 골라 default fg로 교체한다. 다른 토큰은 그대로.
fn sanitize_invisible_fg<'a>(
    ranges: Vec<(Style, &'a str)>,
    bg: Option<Color>,
    default_fg: Color,
) -> Vec<(Style, &'a str)> {
    let Some(bg) = bg else { return ranges; };
    ranges
        .into_iter()
        .map(|(mut style, text)| {
            let fg = style.foreground;
            if fg.r == bg.r && fg.g == bg.g && fg.b == bg.b {
                style.foreground = default_fg;
            }
            (style, text)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::{ImageBackend, RenderCtx};

    fn ctx(use_color: bool) -> RenderCtx {
        RenderCtx { is_tty: true, use_color, width: 60, image_backend: ImageBackend::Placeholder }
    }

    fn ctx_wide(use_color: bool) -> RenderCtx {
        RenderCtx { is_tty: true, use_color, width: 200, image_backend: ImageBackend::Placeholder }
    }

    #[test]
    fn renders_python_code_with_color() {
        let mut buf = Vec::new();
        render("x = 1", "python", &ctx_wide(true), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("x"));
        assert!(s.contains("="));
        assert!(s.contains("1"));
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

    #[test]
    fn sanitize_replaces_bg_colored_fg_with_default() {
        // syntect의 base16-ocean.dark이 `./artifacts`에 fg=bg를 emit하는 케이스.
        // 후처리 후 그 토큰은 default fg로 교체되어 보이게 된다.
        let ss = syntax_set();
        let ts = theme_set();
        let syntax = ss.find_syntax_by_token("python").unwrap();
        let theme = &ts.themes["base16-ocean.dark"];
        let mut hl = HighlightLines::new(syntax, theme);
        let ranges = hl.highlight_line("!ls ./artifacts", ss).unwrap();
        let (bg, default_fg) = theme_palette(theme);
        let safe = sanitize_invisible_fg(ranges, bg, default_fg);
        let bg_color = bg.unwrap();
        for (style, text) in &safe {
            let fg = style.foreground;
            let same = fg.r == bg_color.r && fg.g == bg_color.g && fg.b == bg_color.b;
            assert!(
                !same,
                "token {:?} still has fg=bg=({},{},{}) after sanitize",
                text, fg.r, fg.g, fg.b
            );
        }
    }

    #[test]
    fn sanitize_leaves_normal_fg_alone() {
        // 일반 token의 색은 건드리지 않음
        let ss = syntax_set();
        let ts = theme_set();
        let syntax = ss.find_syntax_by_token("python").unwrap();
        let theme = &ts.themes["base16-ocean.dark"];
        let mut hl = HighlightLines::new(syntax, theme);
        let ranges = hl.highlight_line("x = 1", ss).unwrap();
        let original: Vec<_> = ranges.iter().map(|(s, t)| (s.foreground, *t)).collect();
        let (bg, default_fg) = theme_palette(theme);
        let safe = sanitize_invisible_fg(ranges, bg, default_fg);
        for ((orig_fg, orig_text), (s, t)) in original.iter().zip(safe.iter()) {
            assert_eq!(orig_text, t);
            assert_eq!(orig_fg.r, s.foreground.r);
            assert_eq!(orig_fg.g, s.foreground.g);
            assert_eq!(orig_fg.b, s.foreground.b);
        }
    }
}
