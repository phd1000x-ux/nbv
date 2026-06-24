//! Standalone Markdown document rendering: strip leading YAML frontmatter,
//! then drive the shared Markdown renderer through a border-free `BareSink`.

use std::io::{self, Write};

use crate::env::RenderCtx;
use crate::render::markdown;
use crate::render::sink::BareSink;

/// Render a Markdown document as bare, word-wrapped text (no cell frame).
pub fn render_document(source: &str, ctx: &RenderCtx, w: &mut dyn Write) -> io::Result<()> {
    let body = strip_frontmatter(source);
    let mut sink = BareSink::new(w);
    markdown::render(body, ctx, &mut sink)
}

/// If `source` opens with a YAML frontmatter block — a first line of exactly
/// `---` closed by a later line of `---` or `...` — return the slice after it;
/// otherwise return `source` unchanged. Only a block at the very start counts.
pub fn strip_frontmatter(source: &str) -> &str {
    let mut lines = source.split_inclusive('\n');
    let Some(first) = lines.next() else {
        return source;
    };
    if first.trim_end_matches(['\r', '\n']) != "---" {
        return source;
    }
    let mut consumed = first.len();
    for line in lines {
        consumed += line.len();
        let t = line.trim_end_matches(['\r', '\n']);
        if t == "---" || t == "..." {
            return &source[consumed..];
        }
    }
    // No closing delimiter: not frontmatter, leave untouched.
    source
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_leading_frontmatter() {
        let s = "---\ntitle: x\ndate: y\n---\n# Body\n";
        assert_eq!(strip_frontmatter(s), "# Body\n");
    }

    #[test]
    fn strips_with_dotdotdot_close() {
        let s = "---\ntitle: x\n...\nBody\n";
        assert_eq!(strip_frontmatter(s), "Body\n");
    }

    #[test]
    fn leaves_mid_document_rule() {
        let s = "# Title\n\n---\n\nMore\n";
        assert_eq!(strip_frontmatter(s), s);
    }

    #[test]
    fn leaves_unterminated_block() {
        let s = "---\ntitle: x\n# no close\n";
        assert_eq!(strip_frontmatter(s), s);
    }

    #[test]
    fn render_document_emits_no_box_borders() {
        let c = crate::render::test_support::width(40);
        let mut buf = Vec::new();
        render_document("# Hi\n\nSome **bold** text.\n", &c, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(!s.contains('│'));
        assert!(!s.contains('┌'));
        assert!(s.contains("# Hi"));
        assert!(s.contains("bold"));
    }
}
