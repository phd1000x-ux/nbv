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

/// CSI 시퀀스(`\x1b[...m`)와 단순 `\x1b[X` 형태 escape를 제거.
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' && chars.peek() == Some(&'[') {
            chars.next(); // '['
            while let Some(&nc) = chars.peek() {
                chars.next();
                // 종료는 0x40~0x7E 범위 ASCII
                if ('@'..='~').contains(&nc) {
                    break;
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
}
