use std::io::{self, Write};

use crate::env::RenderCtx;
use crate::render::pad::write_spaces;
use crate::theme;

// 박스 자체(┌─┐│└┘)는 항상 터미널 기본색으로 그린다 — label만 자체 ANSI를 가질 수 있고,
// content는 wrap_line이 RESET을 inject해 padding이 색을 물려받지 않도록 한다.
// 박스선 자체에 별도 색을 입히면 상/하/좌/우가 균일하지 않게 보이는 문제가 있어 제거.

/// Terminal hard-tab stop width (cursor jumps to the next multiple of this).
const TAB_STOP: usize = 8;

/// Visible display width, skipping ANSI CSI escape sequences (`\x1b[...<final>`).
pub(crate) fn ansi_width(s: &str) -> usize {
    let mut w = 0usize;
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip the escape: peek for '[' then consume until 0x40..=0x7E final byte.
            if let Some('[') = chars.clone().next() {
                chars.next();
                for nc in chars.by_ref() {
                    if ('@'..='~').contains(&nc) {
                        break;
                    }
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
    let inner_w = ctx.width.saturating_sub(2);
    let dashes = inner_w.saturating_sub(label_w + 1);
    let dashes_str = "─".repeat(dashes);
    if ctx.use_color {
        // DIM the box-drawing chars (┌─...─┐); keep the label at its own colors.
        writeln!(
            w,
            "{}┌─{}{}{}{}┐{}",
            theme::DIM,
            theme::RESET,
            label_str,
            theme::DIM,
            dashes_str,
            theme::RESET
        )
    } else {
        writeln!(w, "┌─{}{}┐", label_str, dashes_str)
    }
}

pub fn close(ctx: &RenderCtx, w: &mut impl Write) -> io::Result<()> {
    let inner_w = ctx.width.saturating_sub(2);
    let body = format!("└{}┘", "─".repeat(inner_w));
    if ctx.use_color {
        writeln!(w, "{}{}{}", theme::DIM, body, theme::RESET)
    } else {
        writeln!(w, "{}", body)
    }
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
                if ('@'..='~').contains(&nc) {
                    break;
                }
            }
        } else if ch == '\r' {
            // Carriage return은 터미널에서 cursor를 라인 시작으로 보내 박스를 덮어쓴다.
            // 일반 stream output(`\r\n`, tqdm progress 등)에서 흔하므로 drop.
            continue;
        } else if ch == '\t' {
            // Tab은 unicode-width 기준 0이지만 터미널은 다음 8-col stop으로 cursor 점프.
            // 그대로 두면 visible width가 padding 계산보다 커져 박스 폭을 넘어 wrap된다.
            // content position 기준 8-col stop으로 spaces expand.
            let to_add = (TAB_STOP - (used % TAB_STOP)).min(inner_w.saturating_sub(used));
            if to_add == 0 {
                break;
            }
            for _ in 0..to_add {
                trimmed.push(' ');
            }
            used += to_add;
        } else {
            let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            if used + cw > inner_w {
                break;
            }
            trimmed.push(ch);
            used += cw;
        }
    }
    if had_style {
        trimmed.push_str(theme::RESET);
    }
    let pad = inner_w - used;
    if ctx.use_color {
        write!(w, "{}│{} {}", theme::DIM, theme::RESET, trimmed)?;
        write_spaces(w, pad)?;
        writeln!(w, " {}│{}", theme::DIM, theme::RESET)
    } else {
        write!(w, "│ {}", trimmed)?;
        write_spaces(w, pad)?;
        writeln!(w, " │")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::{ImageBackend, RenderCtx};

    fn ctx(width: usize) -> RenderCtx {
        crate::render::test_support::width(width)
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
        let ctx = RenderCtx {
            is_tty: true,
            use_color: true,
            width: 30,
            image_backend: ImageBackend::Placeholder,
            code_theme: "base16-ocean.dark".into(),
        };
        let label = theme::colorize_code_header("In [1] code (python)", true);
        open(&label, &ctx, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let line = s.trim_end_matches('\n');
        // The visible width must equal ctx.width (30), even though the byte length is much larger.
        let visible = ansi_width(line);
        assert_eq!(
            visible, 30,
            "visible width should equal ctx.width, got {visible}"
        );
    }

    #[test]
    fn wrap_line_with_colored_content_keeps_visible_width() {
        // 컬러 escape가 들어간 content도 visible width 기준으로 정확히 padding되어야 한다.
        let mut buf = Vec::new();
        let ctx = RenderCtx {
            is_tty: true,
            use_color: true,
            width: 30,
            image_backend: ImageBackend::Placeholder,
            code_theme: "base16-ocean.dark".into(),
        };
        let content = format!("{}red{}", theme::FG_RED, theme::RESET);
        wrap_line(&content, &ctx, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let line = s.trim_end_matches('\n');
        let visible = ansi_width(line);
        assert_eq!(
            visible, 30,
            "visible width should equal ctx.width, got {visible}"
        );
    }

    #[test]
    fn wrap_line_truncates_colored_content_at_visible_width() {
        // 색이 켜진 채로 짤리면 padding이 그 색으로 새지 않도록 RESET 삽입.
        let mut buf = Vec::new();
        let ctx = RenderCtx {
            is_tty: true,
            use_color: true,
            width: 12,
            image_backend: ImageBackend::Placeholder,
            code_theme: "base16-ocean.dark".into(),
        };
        // inner_w = 8. 9자보다 길게 보내야 truncation.
        let content = format!("{}abcdefghijklmnop", theme::FG_RED);
        wrap_line(&content, &ctx, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(
            s.contains("\x1b[0m"),
            "RESET should be injected after truncated colored content"
        );
        let line = s.trim_end_matches('\n');
        assert_eq!(ansi_width(line), 12);
    }

    #[test]
    fn wrap_line_expands_tab_to_next_8col_stop() {
        // \t는 다음 8-col stop까지 spaces로 변환되어야 한다. 그래야 박스 폭이 정확.
        let mut buf = Vec::new();
        let ctx = RenderCtx {
            is_tty: true,
            use_color: false,
            width: 40,
            image_backend: ImageBackend::Placeholder,
            code_theme: "base16-ocean.dark".into(),
        };
        // "hello"(5) + \t → 다음 stop=8 → 3 spaces, 그 후 "world"(5)
        wrap_line("hello\tworld", &ctx, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(!s.contains('\t'), "tab must be expanded; got {:?}", s);
        let line = s.trim_end_matches('\n');
        assert_eq!(line.chars().count(), 40);
        // 정확히 3 spaces가 hello와 world 사이에 들어가야
        assert!(line.contains("hello   world"));
    }

    #[test]
    fn wrap_line_tab_aligned_correctly_after_long_content() {
        // 8 cols 이후의 \t는 그 다음 16-col stop까지 패딩
        let mut buf = Vec::new();
        let ctx = RenderCtx {
            is_tty: true,
            use_color: false,
            width: 60,
            image_backend: ImageBackend::Placeholder,
            code_theme: "base16-ocean.dark".into(),
        };
        // 9 chars + \t → next stop = 16 → 7 spaces
        wrap_line("123456789\tnext", &ctx, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(
            s.contains("123456789       next"),
            "tab to col 16, got {:?}",
            s
        );
    }

    #[test]
    fn wrap_line_drops_carriage_return() {
        // stream output에 흔한 \r\n에서 \r이 박스 padding을 덮어쓰지 않도록 drop.
        let mut buf = Vec::new();
        let ctx = RenderCtx {
            is_tty: true,
            use_color: false,
            width: 30,
            image_backend: ImageBackend::Placeholder,
            code_theme: "base16-ocean.dark".into(),
        };
        // text::render가 `\n`은 떼고 보내지만 `\r`은 남아 들어옴
        wrap_line("hello\r", &ctx, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(!s.contains('\r'), "output must not contain CR; got {:?}", s);
        let line = s.trim_end_matches('\n');
        assert_eq!(line.chars().count(), 30);
        assert!(line.starts_with("│ hello"));
        assert!(line.ends_with(" │"));
    }

    #[test]
    fn wrap_line_handles_long_syntect_like_escapes_without_phantom_width() {
        // syntect가 emit하는 24-bit 컬러 escape는 \x1b[38;2;R;G;Bm 형태로 17-18 chars.
        // 이런 escape이 잔뜩 들어 있어도 visible width는 짧아야 한다.
        let mut buf = Vec::new();
        let ctx = RenderCtx {
            is_tty: true,
            use_color: true,
            width: 30,
            image_backend: ImageBackend::Placeholder,
            code_theme: "base16-ocean.dark".into(),
        };
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
