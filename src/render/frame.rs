use std::io::{self, Write};

use crate::env::RenderCtx;
use crate::theme;

/// Visible display width, skipping ANSI CSI escape sequences (`\x1b[...<final>`).
fn ansi_width(s: &str) -> usize {
    let mut w = 0usize;
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip the escape: peek for '[' then consume until 0x40..=0x7E final byte.
            if let Some('[') = chars.clone().next() {
                chars.next();
                for nc in chars.by_ref() {
                    if ('@'..='~').contains(&nc) { break; }
                }
            }
            // Non-CSI escapes (OSC, etc.): the \x1b is consumed; next iteration continues.
            // For v0.1 we don't expect those in labels.
        } else {
            w += unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
        }
    }
    w
}

/// 상단 박스 라인: `┌─ {label} ─...─┐`
pub fn open(label: &str, ctx: &RenderCtx, w: &mut impl Write) -> io::Result<()> {
    let label_str = format!(" {} ", label);
    let label_w = ansi_width(&label_str);
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

/// 박스 내부 한 줄: `│ {content padded} │`.
///
/// content에 ANSI CSI escape가 있어도 visible width만 카운트해서 정확히 padding/truncate한다.
/// 컬러 escape가 emit된 경우 padding이 그 색을 물려받지 않도록 끝에 RESET을 inject한다.
pub fn wrap_line(content: &str, ctx: &RenderCtx, w: &mut (impl Write + ?Sized)) -> io::Result<()> {
    let inner_w = ctx.width.saturating_sub(4); // `│ ` + content + ` │`
    let mut trimmed = String::with_capacity(content.len());
    let mut used = 0usize;
    let mut had_style = false;
    let mut chars = content.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' && chars.peek() == Some(&'[') {
            // CSI 시퀀스: width 0으로 취급하고 그대로 보존
            trimmed.push(ch);
            trimmed.push(chars.next().unwrap()); // '['
            had_style = true;
            while let Some(&nc) = chars.peek() {
                trimmed.push(chars.next().unwrap());
                if ('@'..='~').contains(&nc) { break; }
            }
        } else {
            let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            if used + cw > inner_w { break; }
            trimmed.push(ch);
            used += cw;
        }
    }
    if had_style {
        trimmed.push_str(theme::RESET);
    }
    let pad = inner_w - used;
    let border = theme::frame_border(ctx.use_color);
    let reset_b = if ctx.use_color { theme::RESET } else { "" };
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

    #[test]
    fn open_with_colored_label_still_fills_full_width() {
        use crate::theme;
        let mut buf = Vec::new();
        let ctx = RenderCtx { is_tty: true, use_color: true, width: 30, image_backend: ImageBackend::Placeholder };
        let label = theme::colorize_code_header("In [1] code (python)", true);
        open(&label, &ctx, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let line = s.trim_end_matches('\n');
        // The visible width must equal ctx.width (30), even though the byte length is much larger.
        let visible = ansi_width(line);
        assert_eq!(visible, 30, "visible width should equal ctx.width, got {visible}");
    }

    #[test]
    fn wrap_line_with_colored_content_keeps_visible_width() {
        // 컬러 escape가 들어간 content도 visible width 기준으로 정확히 padding되어야 한다.
        let mut buf = Vec::new();
        let ctx = RenderCtx { is_tty: true, use_color: true, width: 30, image_backend: ImageBackend::Placeholder };
        let content = format!("{}red{}", theme::FG_RED, theme::RESET);
        wrap_line(&content, &ctx, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let line = s.trim_end_matches('\n');
        let visible = ansi_width(line);
        assert_eq!(visible, 30, "visible width should equal ctx.width, got {visible}");
    }

    #[test]
    fn wrap_line_truncates_colored_content_at_visible_width() {
        // 색이 켜진 채로 짤리면 padding이 그 색으로 새지 않도록 RESET 삽입.
        let mut buf = Vec::new();
        let ctx = RenderCtx { is_tty: true, use_color: true, width: 12, image_backend: ImageBackend::Placeholder };
        // inner_w = 8. 9자보다 길게 보내야 truncation.
        let content = format!("{}abcdefghijklmnop", theme::FG_RED);
        wrap_line(&content, &ctx, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("\x1b[0m"), "RESET should be injected after truncated colored content");
        let line = s.trim_end_matches('\n');
        assert_eq!(ansi_width(line), 12);
    }

    #[test]
    fn wrap_line_handles_long_syntect_like_escapes_without_phantom_width() {
        // syntect가 emit하는 24-bit 컬러 escape는 \x1b[38;2;R;G;Bm 형태로 17-18 chars.
        // 이런 escape이 잔뜩 들어 있어도 visible width는 짧아야 한다.
        let mut buf = Vec::new();
        let ctx = RenderCtx { is_tty: true, use_color: true, width: 30, image_backend: ImageBackend::Placeholder };
        let content = "\x1b[38;2;192;197;206mx\x1b[0m \x1b[38;2;192;197;206m=\x1b[0m \x1b[38;2;192;197;206m1\x1b[0m";
        wrap_line(content, &ctx, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let line = s.trim_end_matches('\n');
        assert_eq!(ansi_width(line), 30);
        // visible 문자가 정상적으로 다 들어와야 함 (x = 1)
        assert!(s.contains("x"));
        assert!(s.contains("="));
        assert!(s.contains("1"));
    }
}
