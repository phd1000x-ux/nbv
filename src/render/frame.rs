use std::io::{self, Write};
use unicode_width::UnicodeWidthStr;

use crate::env::RenderCtx;
use crate::theme;

/// 상단 박스 라인: `┌─ {label} ─...─┐`
pub fn open(label: &str, ctx: &RenderCtx, w: &mut impl Write) -> io::Result<()> {
    let label_str = format!(" {} ", label);
    let label_w = label_str.width();
    let inner_w = ctx.width.saturating_sub(2); // ┌, ┐ 제외
    let dashes = inner_w.saturating_sub(label_w + 1);
    let border = theme::frame_border(ctx.use_color);
    let reset = if ctx.use_color { theme::RESET } else { "" };
    writeln!(
        w,
        "{}┌─{}{}┐{}",
        border,
        label_str,
        "─".repeat(dashes),
        reset
    )
}

pub fn close(ctx: &RenderCtx, w: &mut impl Write) -> io::Result<()> {
    let inner_w = ctx.width.saturating_sub(2);
    let border = theme::frame_border(ctx.use_color);
    let reset = if ctx.use_color { theme::RESET } else { "" };
    writeln!(w, "{}└{}┘{}", border, "─".repeat(inner_w), reset)
}

/// 박스 내부 한 줄: `│ {content padded} │`
pub fn wrap_line(content: &str, ctx: &RenderCtx, w: &mut (impl Write + ?Sized)) -> io::Result<()> {
    let inner_w = ctx.width.saturating_sub(4); // `│ ` + content + ` │`
    let mut trimmed = String::new();
    let mut used = 0usize;
    for ch in content.chars() {
        let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + cw > inner_w { break; }
        trimmed.push(ch);
        used += cw;
    }
    let pad = inner_w - used;
    let border = theme::frame_border(ctx.use_color);
    let reset_b = if ctx.use_color { theme::RESET } else { "" };
    // ANSI escape가 content에 있을 수 있으므로 reset도 한 번 더 출력
    writeln!(
        w,
        "{0}│{1} {2}{0}{3} │{1}",
        border, reset_b, trimmed, " ".repeat(pad)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::{ImageBackend, RenderCtx};

    fn ctx(width: usize) -> RenderCtx {
        RenderCtx { is_tty: true, use_color: false, width, image_backend: ImageBackend::Placeholder }
    }

    #[test]
    fn open_writes_full_width_top_border() {
        let mut buf = Vec::new();
        open("In [1] code", &ctx(30), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let line = s.trim_end_matches('\n');
        assert_eq!(line.chars().count(), 30);
        assert!(line.starts_with("┌─"));
        assert!(line.contains("In [1] code"));
        assert!(line.ends_with("┐"));
    }

    #[test]
    fn close_writes_full_width_bottom_border() {
        let mut buf = Vec::new();
        close(&ctx(30), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let line = s.trim_end_matches('\n');
        assert_eq!(line.chars().count(), 30);
        assert!(line.starts_with("└"));
        assert!(line.ends_with("┘"));
    }

    #[test]
    fn wrap_line_pads_to_width() {
        let mut buf = Vec::new();
        wrap_line("hello", &ctx(30), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let line = s.trim_end_matches('\n');
        assert_eq!(line.chars().count(), 30);
        assert!(line.starts_with("│ hello"));
        assert!(line.ends_with(" │"));
    }

    #[test]
    fn wrap_line_truncates_long_content() {
        let mut buf = Vec::new();
        wrap_line(&"x".repeat(100), &ctx(20), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let line = s.trim_end_matches('\n');
        assert_eq!(line.chars().count(), 20);
    }

    #[test]
    fn wrap_line_handles_cjk_wide_chars() {
        // 한글 한 글자가 2칸을 차지함
        let mut buf = Vec::new();
        wrap_line("가나다", &ctx(20), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let line = s.trim_end_matches('\n');
        // unicode-width 기준 가시 폭이 20이어야 함 (chars().count()와 다름)
        use unicode_width::UnicodeWidthStr;
        assert_eq!(line.width(), 20);
    }
}
