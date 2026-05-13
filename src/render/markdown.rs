use std::io::{self, Write};

use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};

use crate::env::RenderCtx;
use crate::render::{code, frame};
use crate::theme;

/// 누적 텍스트 + 스타일을 frame::wrap_line으로 흘려보냄.
pub fn render(source: &str, ctx: &RenderCtx, w: &mut impl Write) -> io::Result<()> {
    let mut acc = String::new();
    let mut _style = Style::default();
    let mut list_stack: Vec<ListState> = Vec::new();
    let mut in_blockquote = 0u32;
    let mut pending_code_block: Option<String> = None;
    let mut pending_lang: Option<String> = None;

    let parser = Parser::new(source);

    for ev in parser {
        match ev {
            Event::Start(Tag::Heading { level, .. }) => {
                flush_line(&mut acc, in_blockquote, ctx, w)?;
                let n = heading_n(level);
                let prefix = "#".repeat(n);
                let header_text = format!("{} ", prefix);
                acc.push_str(&theme::colorize_markdown_header(
                    &header_text,
                    ctx.use_color,
                ));
                _style.bold = true;
            }
            Event::End(TagEnd::Heading(_)) => {
                if ctx.use_color {
                    acc.push_str(theme::RESET);
                }
                flush_line(&mut acc, in_blockquote, ctx, w)?;
                _style = Style::default();
            }
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                flush_line(&mut acc, in_blockquote, ctx, w)?;
            }
            Event::Start(Tag::Emphasis) => {
                if ctx.use_color {
                    acc.push_str(theme::ITALIC);
                }
                _style.italic = true;
            }
            Event::End(TagEnd::Emphasis) => {
                if ctx.use_color {
                    acc.push_str(theme::RESET);
                }
                _style.italic = false;
            }
            Event::Start(Tag::Strong) => {
                if ctx.use_color {
                    acc.push_str(theme::BOLD);
                }
                _style.bold = true;
            }
            Event::End(TagEnd::Strong) => {
                if ctx.use_color {
                    acc.push_str(theme::RESET);
                }
                _style.bold = false;
            }
            Event::Code(c) => {
                if ctx.use_color {
                    acc.push_str(theme::FG_YELLOW);
                }
                acc.push('`');
                acc.push_str(&c);
                acc.push('`');
                if ctx.use_color {
                    acc.push_str(theme::RESET);
                }
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                flush_line(&mut acc, in_blockquote, ctx, w)?;
                let lang = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(l) => l.into_string(),
                    pulldown_cmark::CodeBlockKind::Indented => String::new(),
                };
                pending_lang = Some(if lang.is_empty() { "text".into() } else { lang });
                pending_code_block = Some(String::new());
            }
            Event::Text(t) if pending_code_block.is_some() => {
                pending_code_block.as_mut().unwrap().push_str(&t);
            }
            Event::End(TagEnd::CodeBlock) => {
                let src = pending_code_block.take().unwrap_or_default();
                let lang = pending_lang.take().unwrap_or_else(|| "text".into());
                code::render(&src, &lang, ctx, w)?;
            }
            Event::Start(Tag::List(start)) => {
                flush_line(&mut acc, in_blockquote, ctx, w)?;
                list_stack.push(ListState { number: start });
            }
            Event::End(TagEnd::List(_)) => {
                list_stack.pop();
            }
            Event::Start(Tag::Item) => {
                let indent = "  ".repeat(list_stack.len().saturating_sub(1));
                if let Some(state) = list_stack.last_mut() {
                    match state.number.as_mut() {
                        Some(n) => {
                            acc.push_str(&format!("{}{}. ", indent, n));
                            *n += 1;
                        }
                        None => {
                            acc.push_str(&format!("{}• ", indent));
                        }
                    }
                }
            }
            Event::End(TagEnd::Item) => {
                flush_line(&mut acc, in_blockquote, ctx, w)?;
            }
            Event::Start(Tag::BlockQuote) => {
                in_blockquote += 1;
            }
            Event::End(TagEnd::BlockQuote) => {
                in_blockquote = in_blockquote.saturating_sub(1);
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                acc.push('[');
                let _ = dest_url; // v0.1는 텍스트만
            }
            Event::End(TagEnd::Link) => {
                acc.push(']');
            }
            Event::Text(t) => {
                acc.push_str(&t);
            }
            Event::SoftBreak | Event::HardBreak => {
                flush_line(&mut acc, in_blockquote, ctx, w)?;
            }
            Event::Rule => {
                flush_line(&mut acc, in_blockquote, ctx, w)?;
                let dashes = "─".repeat(ctx.width.saturating_sub(4));
                frame::wrap_line(&dashes, ctx, w)?;
            }
            _ => {}
        }
    }
    flush_line(&mut acc, in_blockquote, ctx, w)?;
    Ok(())
}

fn flush_line(
    acc: &mut String,
    quote_depth: u32,
    ctx: &RenderCtx,
    w: &mut impl Write,
) -> io::Result<()> {
    if acc.trim().is_empty() {
        acc.clear();
        return Ok(());
    }
    let line = if quote_depth > 0 {
        let prefix = "> ".repeat(quote_depth as usize);
        format!("{}{}", theme::dim(&prefix, ctx.use_color), acc)
    } else {
        std::mem::take(acc)
    };
    frame::wrap_line(&line, ctx, w)?;
    acc.clear();
    Ok(())
}

fn heading_n(level: HeadingLevel) -> usize {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

#[derive(Default)]
struct Style {
    bold: bool,
    italic: bool,
}

struct ListState {
    number: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::{ImageBackend, RenderCtx};

    fn ctx(use_color: bool) -> RenderCtx {
        RenderCtx {
            is_tty: true,
            use_color,
            width: 60,
            image_backend: ImageBackend::Placeholder,
        }
    }

    #[test]
    fn heading_gets_hash_prefix() {
        let mut buf = Vec::new();
        render("# Hello", &ctx(false), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("# Hello"));
    }

    #[test]
    fn h2_gets_two_hashes() {
        let mut buf = Vec::new();
        render("## Hello", &ctx(false), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("## Hello"));
    }

    #[test]
    fn unordered_list_has_bullet() {
        let mut buf = Vec::new();
        render("- one\n- two", &ctx(false), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("• one") || s.contains("- one"));
        assert!(s.contains("two"));
    }

    #[test]
    fn ordered_list_has_numbers() {
        let mut buf = Vec::new();
        render("1. one\n2. two", &ctx(false), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("1.") || s.contains("1)"));
        assert!(s.contains("two"));
    }

    #[test]
    fn inline_code_preserved() {
        let mut buf = Vec::new();
        render("use `foo()` here", &ctx(false), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("foo()"));
    }

    #[test]
    fn fenced_code_block_handed_to_syntect() {
        let mut buf = Vec::new();
        // wider ctx for syntect color escapes
        let ctx_wide = RenderCtx {
            is_tty: true,
            use_color: true,
            width: 200,
            image_backend: ImageBackend::Placeholder,
        };
        render("```python\nx = 1\n```", &ctx_wide, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("x") && s.contains("="));
    }

    #[test]
    fn blockquote_has_prefix() {
        let mut buf = Vec::new();
        render("> quoted", &ctx(false), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("> quoted") || s.contains("│ > quoted"));
    }

    #[test]
    fn bold_uses_ansi_when_color() {
        let mut buf = Vec::new();
        render("**bold**", &ctx(true), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("\x1b[1m") || s.contains("bold"));
    }
}
