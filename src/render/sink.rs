//! Output seam shared by every renderer. `BoxedSink` draws the framed
//! `│ … │` lines used inside notebook cells; `BareSink` (Task 2) emits
//! word-wrapped, border-free document lines.

use std::io::{self, Write};

use base64::Engine;

use crate::env::RenderCtx;
use crate::render::frame;
use crate::render::frame::ansi_width;
use crate::render::image::{self, png_info};
use crate::theme;

/// A renderer's only way to emit a line. `text_line` is prose that a bare
/// sink may reflow; `raw_line` is pre-formatted (code, table borders, rules)
/// and is never reflowed.
pub trait LineSink {
    fn text_line(&mut self, content: &str, ctx: &RenderCtx) -> io::Result<()>;
    fn raw_line(&mut self, content: &str, ctx: &RenderCtx) -> io::Result<()>;
    /// Render an image reference. `local` is the file bytes if a local file was
    /// read successfully; `None` for remote URLs, missing files, or `--no-images`.
    fn image(
        &mut self,
        local: Option<&[u8]>,
        alt: &str,
        src: &str,
        ctx: &RenderCtx,
    ) -> io::Result<()>;
}

/// One-line text descriptor for an image we won't inline.
fn image_descriptor(local: Option<&[u8]>, alt: &str, src: &str) -> String {
    let label = if !alt.trim().is_empty() { alt } else { src };
    match local.and_then(png_info::dimensions) {
        Some((wd, ht)) => format!("🖼  [image: {} ({}×{})]", label, wd, ht),
        None => format!("🖼  [image: {}]", label),
    }
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
    fn image(
        &mut self,
        local: Option<&[u8]>,
        alt: &str,
        src: &str,
        ctx: &RenderCtx,
    ) -> io::Result<()> {
        match (ctx.image_backend, local) {
            (crate::env::ImageBackend::Placeholder, _) | (_, None) => {
                frame::wrap_line(&image_descriptor(local, alt, src), ctx, self.w)
            }
            (_, Some(bytes)) => {
                let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
                image::dispatch(&b64, 0, 0, ctx, self.w)
            }
        }
    }
}

/// Border-free document output: prose lines are word-wrapped to `ctx.width`;
/// pre-formatted lines are written verbatim.
pub struct BareSink<'w> {
    w: &'w mut dyn Write,
}

impl<'w> BareSink<'w> {
    pub fn new(w: &'w mut dyn Write) -> Self {
        BareSink { w }
    }
}

impl LineSink for BareSink<'_> {
    fn text_line(&mut self, content: &str, ctx: &RenderCtx) -> io::Result<()> {
        for line in wrap_ansi(content, ctx.width) {
            writeln!(self.w, "{}", line)?;
        }
        Ok(())
    }
    fn raw_line(&mut self, content: &str, ctx: &RenderCtx) -> io::Result<()> {
        let _ = ctx;
        writeln!(self.w, "{}", content)
    }
    fn image(
        &mut self,
        local: Option<&[u8]>,
        alt: &str,
        src: &str,
        ctx: &RenderCtx,
    ) -> io::Result<()> {
        match (ctx.image_backend, local) {
            (crate::env::ImageBackend::Placeholder, _) | (_, None) => {
                self.text_line(&image_descriptor(local, alt, src), ctx)
            }
            (_, Some(bytes)) => {
                let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
                image::dispatch(&b64, 0, 0, ctx, self.w)
            }
        }
    }
}

/// Append `seg` to `line`, advancing `col` by visible width and tracking the
/// active SGR state (`active`) so a wrap can close/reopen styles. ANSI CSI
/// `m` sequences accumulate into `active`; a RESET clears it.
fn push_tracking(line: &mut String, col: &mut usize, active: &mut String, seg: &str) {
    let mut chars = seg.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' && chars.peek() == Some(&'[') {
            let mut esc = String::from("\x1b[");
            chars.next();
            while let Some(&nc) = chars.peek() {
                esc.push(chars.next().unwrap());
                if ('@'..='~').contains(&nc) {
                    break;
                }
            }
            if esc == theme::RESET {
                active.clear();
            } else if esc.ends_with('m') {
                active.push_str(&esc);
            }
            line.push_str(&esc);
        } else {
            *col += unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
            line.push(c);
        }
    }
}

/// Split `content` into words (maximal non-space runs, ANSI escapes kept with
/// their word). Inter-word runs of spaces collapse to a single separator.
fn tokenize_words(content: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut cur = String::new();
    let mut chars = content.chars().peekable();
    while let Some(c) = chars.next() {
        if c == ' ' {
            if !cur.is_empty() {
                words.push(std::mem::take(&mut cur));
            }
        } else if c == '\x1b' && chars.peek() == Some(&'[') {
            cur.push('\x1b');
            cur.push(chars.next().unwrap());
            while let Some(&nc) = chars.peek() {
                cur.push(chars.next().unwrap());
                if ('@'..='~').contains(&nc) {
                    break;
                }
            }
        } else {
            cur.push(c);
        }
    }
    if !cur.is_empty() {
        words.push(cur);
    }
    words
}

/// Word-wrap `content` to `width` visible columns, ANSI-aware. Continuation
/// lines are indented by the content's leading-space count. A word wider than
/// the line is hard-broken.
fn wrap_ansi(content: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![content.to_string()];
    }
    let words = tokenize_words(content);
    if words.is_empty() {
        return vec![String::new()];
    }
    let indent_cols = content
        .chars()
        .take_while(|&c| c == ' ')
        .count()
        .min(width.saturating_sub(2));
    let indent = " ".repeat(indent_cols);

    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    let mut col = 0usize;
    let mut active = String::new();

    for word in &words {
        let ww = ansi_width(word);
        let at_line_start = line.is_empty();
        if at_line_start {
            if !lines.is_empty() {
                line.push_str(&indent);
                col = indent_cols;
                line.push_str(&active);
            }
            place_word(
                &mut lines,
                &mut line,
                &mut col,
                &mut active,
                word,
                width,
                &indent,
                indent_cols,
            );
        } else if col + 1 + ww <= width {
            line.push(' ');
            col += 1;
            push_tracking(&mut line, &mut col, &mut active, word);
        } else {
            if !active.is_empty() {
                line.push_str(theme::RESET);
            }
            lines.push(std::mem::take(&mut line));
            line.push_str(&indent);
            col = indent_cols;
            line.push_str(&active);
            place_word(
                &mut lines,
                &mut line,
                &mut col,
                &mut active,
                word,
                width,
                &indent,
                indent_cols,
            );
        }
    }
    lines.push(line);
    lines
}

/// Place `word` on `line`; if it doesn't fit and is longer than a full line,
/// hard-break it across lines (each continuation re-opens `active`).
#[allow(clippy::too_many_arguments)]
fn place_word(
    lines: &mut Vec<String>,
    line: &mut String,
    col: &mut usize,
    active: &mut String,
    word: &str,
    width: usize,
    indent: &str,
    indent_cols: usize,
) {
    if *col + ansi_width(word) <= width {
        push_tracking(line, col, active, word);
        return;
    }
    let mut chars = word.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' && chars.peek() == Some(&'[') {
            let mut esc = String::from("\x1b[");
            chars.next();
            while let Some(&nc) = chars.peek() {
                esc.push(chars.next().unwrap());
                if ('@'..='~').contains(&nc) {
                    break;
                }
            }
            if esc == theme::RESET {
                active.clear();
            } else if esc.ends_with('m') {
                active.push_str(&esc);
            }
            line.push_str(&esc);
        } else {
            let cw = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
            if *col + cw > width && *col > indent_cols {
                if !active.is_empty() {
                    line.push_str(theme::RESET);
                }
                lines.push(std::mem::take(line));
                line.push_str(indent);
                line.push_str(active);
                *col = indent_cols;
            }
            line.push(c);
            *col += cw;
        }
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

    fn bare_to_string(content: &str, width: usize) -> String {
        let mut buf = Vec::new();
        let c = crate::render::test_support::width(width);
        BareSink::new(&mut buf).text_line(content, &c).unwrap();
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn bare_has_no_borders() {
        let s = bare_to_string("hello world", 40);
        assert!(!s.contains('│'));
        assert_eq!(s, "hello world\n");
    }

    #[test]
    fn bare_wraps_long_prose_to_width() {
        // width 10: "alpha beta gamma" -> two visual lines ("alpha beta" fits exactly at width 10)
        let s = bare_to_string("alpha beta gamma", 10);
        for line in s.lines() {
            assert!(
                crate::render::frame::ansi_width(line) <= 10,
                "line too wide: {line:?}"
            );
        }
        assert!(s.contains("alpha"));
        assert!(s.contains("beta"));
        assert!(s.contains("gamma"));
    }

    #[test]
    fn bare_wraps_cjk_without_exceeding_width() {
        // 2-column-wide chars must never produce an over-width line.
        let s = bare_to_string("가나다라마바사아", 10);
        for line in s.lines() {
            assert!(
                crate::render::frame::ansi_width(line) <= 10,
                "over-width: {line:?}"
            );
        }
        // all eight syllables preserved
        for ch in ["가", "나", "다", "라", "마", "바", "사", "아"] {
            assert!(s.contains(ch), "missing {ch}");
        }
    }

    #[test]
    fn bare_hard_breaks_overlong_word() {
        let s = bare_to_string("xxxxxxxxxxxxxxxxxxxx", 8); // 20 x's, width 8
        for line in s.lines() {
            assert!(crate::render::frame::ansi_width(line) <= 8);
        }
        assert_eq!(s.replace('\n', "").len(), 20);
    }

    #[test]
    fn bare_indents_continuation_to_leading_whitespace() {
        // leading 2 spaces -> continuation lines indented by 2
        let s = bare_to_string("  aaaa bbbb cccc", 8);
        let lines: Vec<&str> = s.lines().collect();
        assert!(lines.len() >= 2);
        assert!(
            lines[1].starts_with("  "),
            "continuation should be indented: {:?}",
            lines[1]
        );
    }

    #[test]
    fn bare_reopens_style_across_wrap() {
        // a bold span spanning the wrap boundary: RESET closes line 1, bold re-opens line 2
        let content = format!("{}alpha beta{}", crate::theme::BOLD, crate::theme::RESET);
        let s = bare_to_string(&content, 7);
        let lines: Vec<&str> = s.lines().collect();
        assert!(lines.len() == 2, "expected wrap, got {s:?}");
        assert!(lines[0].ends_with(crate::theme::RESET));
        assert!(lines[1].contains(crate::theme::BOLD));
    }

    #[test]
    fn bare_raw_line_is_verbatim() {
        let mut buf = Vec::new();
        let c = crate::render::test_support::width(10);
        BareSink::new(&mut buf).raw_line("┌────────┐", &c).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "┌────────┐\n");
    }

    const ONE_PIXEL_B64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==";

    fn one_pixel_bytes() -> Vec<u8> {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD
            .decode(ONE_PIXEL_B64)
            .unwrap()
    }

    #[test]
    fn bare_image_descriptor_for_remote() {
        let mut buf = Vec::new();
        let c = crate::render::test_support::width(60); // placeholder backend
        BareSink::new(&mut buf)
            .image(None, "logo", "https://x/y.png", &c)
            .unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("image"));
        assert!(s.contains("logo"));
        assert!(!s.contains('│'));
    }

    #[test]
    fn bare_image_descriptor_shows_png_dims() {
        let mut buf = Vec::new();
        let c = crate::render::test_support::width(60); // placeholder backend
        let bytes = one_pixel_bytes();
        BareSink::new(&mut buf)
            .image(Some(&bytes), "pixel", "pixel.png", &c)
            .unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("1×1") || s.contains("1x1"));
    }

    #[test]
    fn bare_image_inlines_for_iterm() {
        let mut buf = Vec::new();
        let c = crate::render::test_support::backend(crate::env::ImageBackend::ITerm2);
        let bytes = one_pixel_bytes();
        BareSink::new(&mut buf)
            .image(Some(&bytes), "pixel", "pixel.png", &c)
            .unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(
            s.starts_with("\x1b]1337;"),
            "iTerm graphics escape expected: {s:?}"
        );
    }
}
