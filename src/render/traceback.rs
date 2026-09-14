use std::io::{self, Write};

use crate::env::RenderCtx;
use crate::render::frame;

pub fn render(traceback: &[String], ctx: &RenderCtx, w: &mut impl Write) -> io::Result<()> {
    for entry in traceback {
        for line in entry.split('\n') {
            let line = if ctx.use_color {
                line.to_string()
            } else {
                strip_ansi(line)
            };
            frame::wrap_line(&line, ctx, w)?;
        }
    }
    Ok(())
}

/// CSI 시퀀스(`\x1b[...m`)와 OSC 시퀀스(`\x1b]...BEL`/`\x1b]...\x1b\\`),
/// 그리고 단독 ESC를 제거. 커널 traceback이나 stream 출력에 끼어든
/// 터미널 제어 시퀀스를 no-color 모드에서 완전히 걷어낸다.
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            match chars.peek() {
                Some('[') => {
                    chars.next(); // '['
                    while let Some(&nc) = chars.peek() {
                        chars.next();
                        // 종료는 0x40~0x7E 범위 ASCII
                        if ('@'..='~').contains(&nc) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    chars.next(); // ']'
                    for nc in chars.by_ref() {
                        if nc == '\x07' {
                            break; // BEL 종료
                        }
                        if nc == '\x1b' {
                            // ST(ESC \) 종료: \가 뒤따르면 함께 consume
                            if chars.peek() == Some(&'\\') {
                                chars.next();
                            }
                            break;
                        }
                    }
                }
                _ => {
                    // 단독 ESC 또는 기타 단일 문자 이스케이프: ESC만 제거
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// `code.rs`에서 ANSI strip이 필요할 때 재사용하기 위한 공개 진입점.
pub fn strip_ansi_pub(s: &str) -> String {
    strip_ansi(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::RenderCtx;

    fn ctx(use_color: bool) -> RenderCtx {
        crate::render::test_support::color(use_color)
    }

    #[test]
    fn renders_each_traceback_line() {
        let tb: Vec<String> = vec!["ValueError: bad".into(), "  at line 1".into()];
        let mut buf = Vec::new();
        render(&tb, &ctx(true), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("ValueError"));
        assert!(s.contains("at line 1"));
    }

    #[test]
    fn preserves_existing_ansi_escapes_when_color() {
        let tb: Vec<String> = vec!["\x1b[31mred text\x1b[0m".into()];
        let mut buf = Vec::new();
        render(&tb, &ctx(true), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("\x1b[31m"));
    }

    #[test]
    fn strips_ansi_escapes_when_no_color() {
        let tb: Vec<String> = vec!["\x1b[31mred text\x1b[0m".into()];
        let mut buf = Vec::new();
        render(&tb, &ctx(false), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(!s.contains("\x1b["));
        assert!(s.contains("red text"));
    }

    #[test]
    fn handles_lines_with_embedded_newlines() {
        // 일부 커널은 한 traceback 엔트리에 여러 줄을 넣음
        let tb: Vec<String> = vec!["line1\nline2".into()];
        let mut buf = Vec::new();
        render(&tb, &ctx(false), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("line1"));
        assert!(s.contains("line2"));
        assert!(s.matches('\n').count() >= 2);
    }
}

#[cfg(test)]
mod strip_tests {
    use super::strip_ansi;
    #[test]
    fn strips_csi_m() {
        assert_eq!(strip_ansi("\x1b[31mhi\x1b[0m"), "hi");
    }
    #[test]
    fn passes_through_plain() {
        assert_eq!(strip_ansi("abc"), "abc");
    }
    #[test]
    fn strips_osc_terminated_by_bel() {
        // OSC 0 (set title): ESC ] 0 ; title BEL — must not survive stripping.
        assert_eq!(strip_ansi("before\x1b]0;title\x07after"), "beforeafter");
    }
    #[test]
    fn strips_osc_terminated_by_st() {
        // OSC terminated by String Terminator (ESC \).
        assert_eq!(strip_ansi("\x1b]0;t\x1b\\x"), "x");
    }
    #[test]
    fn strips_bare_escape() {
        // A lone ESC not followed by [ or ] must still be removed.
        assert_eq!(strip_ansi("a\x1bb"), "ab");
    }
}
