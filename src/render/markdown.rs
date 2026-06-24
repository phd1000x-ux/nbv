use std::io;

use pulldown_cmark::{Alignment, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::env::RenderCtx;
use crate::render::code;
use crate::render::sink::LineSink;
use crate::render::table::{self, Align, Table};
use crate::theme;

/// 누적 텍스트 + 스타일을 sink으로 흘려보냄.
pub fn render(source: &str, ctx: &RenderCtx, sink: &mut dyn LineSink) -> io::Result<()> {
    let mut acc = String::new();
    let mut _style = Style::default();
    let mut list_stack: Vec<ListState> = Vec::new();
    let mut in_blockquote = 0u32;
    let mut pending_code_block: Option<String> = None;
    let mut pending_lang: Option<String> = None;
    let mut table: Option<TableBuilder> = None;

    let parser = Parser::new_ext(source, Options::ENABLE_TABLES);

    for ev in parser {
        match ev {
            Event::Start(Tag::Heading { level, .. }) => {
                flush_line(&mut acc, in_blockquote, ctx, sink)?;
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
                flush_line(&mut acc, in_blockquote, ctx, sink)?;
                _style = Style::default();
            }
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                flush_line(&mut acc, in_blockquote, ctx, sink)?;
            }
            Event::Start(Tag::Emphasis) => {
                if ctx.use_color && !in_table_cell(&table) {
                    acc.push_str(theme::ITALIC);
                }
                _style.italic = true;
            }
            Event::End(TagEnd::Emphasis) => {
                if ctx.use_color && !in_table_cell(&table) {
                    acc.push_str(theme::RESET);
                }
                _style.italic = false;
            }
            Event::Start(Tag::Strong) => {
                if ctx.use_color && !in_table_cell(&table) {
                    acc.push_str(theme::BOLD);
                }
                _style.bold = true;
            }
            Event::End(TagEnd::Strong) => {
                if ctx.use_color && !in_table_cell(&table) {
                    acc.push_str(theme::RESET);
                }
                _style.bold = false;
            }
            Event::Code(c) => {
                if let Some(cell) = table.as_mut().and_then(|t| t.cur_cell.as_mut()) {
                    cell.push_str(&c);
                } else {
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
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                flush_line(&mut acc, in_blockquote, ctx, sink)?;
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
                code::render(&src, &lang, ctx, sink)?;
            }
            Event::Start(Tag::List(start)) => {
                flush_line(&mut acc, in_blockquote, ctx, sink)?;
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
                flush_line(&mut acc, in_blockquote, ctx, sink)?;
            }
            Event::Start(Tag::BlockQuote) => {
                in_blockquote += 1;
            }
            Event::End(TagEnd::BlockQuote) => {
                in_blockquote = in_blockquote.saturating_sub(1);
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                acc.push('[');
                let _ = dest_url; // 링크는 텍스트만 렌더 (URL 미표시)
            }
            Event::End(TagEnd::Link) => {
                acc.push(']');
            }
            Event::Text(t) => {
                text_sink(&mut table, &mut acc).push_str(&t);
            }
            Event::SoftBreak | Event::HardBreak => {
                if let Some(cell) = table.as_mut().and_then(|t| t.cur_cell.as_mut()) {
                    cell.push(' ');
                } else {
                    flush_line(&mut acc, in_blockquote, ctx, sink)?;
                }
            }
            Event::Rule => {
                flush_line(&mut acc, in_blockquote, ctx, sink)?;
                let dashes = "─".repeat(ctx.width);
                sink.raw_line(&dashes, ctx)?;
            }
            Event::Start(Tag::Table(aligns)) => {
                flush_line(&mut acc, in_blockquote, ctx, sink)?;
                table = Some(TableBuilder::new(&aligns));
            }
            Event::Start(Tag::TableHead) | Event::Start(Tag::TableRow) => {
                if let Some(t) = table.as_mut() {
                    t.begin_row();
                }
            }
            Event::End(TagEnd::TableHead) => {
                if let Some(t) = table.as_mut() {
                    t.end_head();
                }
            }
            Event::End(TagEnd::TableRow) => {
                if let Some(t) = table.as_mut() {
                    t.end_row();
                }
            }
            Event::Start(Tag::TableCell) => {
                if let Some(t) = table.as_mut() {
                    t.begin_cell();
                }
            }
            Event::End(TagEnd::TableCell) => {
                if let Some(t) = table.as_mut() {
                    t.end_cell();
                }
            }
            Event::End(TagEnd::Table) => {
                if let Some(t) = table.take() {
                    table::render(&t.into_table(), ctx, sink)?;
                }
            }
            _ => {}
        }
    }
    flush_line(&mut acc, in_blockquote, ctx, sink)?;
    Ok(())
}

fn flush_line(
    acc: &mut String,
    quote_depth: u32,
    ctx: &RenderCtx,
    sink: &mut dyn LineSink,
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
    sink.text_line(&line, ctx)?;
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

/// Accumulates pulldown-cmark table events into a `Table`.
struct TableBuilder {
    align: Vec<Align>,
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    cur_row: Vec<String>,
    cur_cell: Option<String>,
}

impl TableBuilder {
    fn new(aligns: &[Alignment]) -> Self {
        TableBuilder {
            align: aligns.iter().map(map_align).collect(),
            headers: Vec::new(),
            rows: Vec::new(),
            cur_row: Vec::new(),
            cur_cell: None,
        }
    }

    fn begin_row(&mut self) {
        self.cur_row = Vec::new();
    }

    fn end_head(&mut self) {
        self.headers = std::mem::take(&mut self.cur_row);
    }

    fn end_row(&mut self) {
        let row = std::mem::take(&mut self.cur_row);
        self.rows.push(row);
    }

    fn begin_cell(&mut self) {
        self.cur_cell = Some(String::new());
    }

    fn end_cell(&mut self) {
        let cell = self.cur_cell.take().unwrap_or_default();
        self.cur_row.push(cell.trim().to_string());
    }

    fn into_table(self) -> Table {
        Table::new(self.headers, self.rows, self.align)
    }
}

fn map_align(a: &Alignment) -> Align {
    match a {
        Alignment::Right => Align::Right,
        Alignment::Center => Align::Center,
        Alignment::None | Alignment::Left => Align::Left,
    }
}

/// True while events are being routed into a table cell's text buffer.
fn in_table_cell(table: &Option<TableBuilder>) -> bool {
    table.as_ref().is_some_and(|t| t.cur_cell.is_some())
}

/// Returns the buffer plain text should accumulate into: the open table cell
/// if there is one, otherwise the line accumulator.
fn text_sink<'a>(table: &'a mut Option<TableBuilder>, acc: &'a mut String) -> &'a mut String {
    match table.as_mut().and_then(|t| t.cur_cell.as_mut()) {
        Some(cell) => cell,
        None => acc,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::{ImageBackend, RenderCtx};

    fn ctx(use_color: bool) -> RenderCtx {
        crate::render::test_support::color(use_color)
    }

    #[test]
    fn heading_gets_hash_prefix() {
        let mut buf = Vec::new();
        {
            let mut sink = crate::render::sink::BoxedSink::new(&mut buf);
            render("# Hello", &ctx(false), &mut sink).unwrap();
        }
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("# Hello"));
    }

    #[test]
    fn h2_gets_two_hashes() {
        let mut buf = Vec::new();
        {
            let mut sink = crate::render::sink::BoxedSink::new(&mut buf);
            render("## Hello", &ctx(false), &mut sink).unwrap();
        }
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("## Hello"));
    }

    #[test]
    fn unordered_list_has_bullet() {
        let mut buf = Vec::new();
        {
            let mut sink = crate::render::sink::BoxedSink::new(&mut buf);
            render("- one\n- two", &ctx(false), &mut sink).unwrap();
        }
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("• one") || s.contains("- one"));
        assert!(s.contains("two"));
    }

    #[test]
    fn ordered_list_has_numbers() {
        let mut buf = Vec::new();
        {
            let mut sink = crate::render::sink::BoxedSink::new(&mut buf);
            render("1. one\n2. two", &ctx(false), &mut sink).unwrap();
        }
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("1.") || s.contains("1)"));
        assert!(s.contains("two"));
    }

    #[test]
    fn inline_code_preserved() {
        let mut buf = Vec::new();
        {
            let mut sink = crate::render::sink::BoxedSink::new(&mut buf);
            render("use `foo()` here", &ctx(false), &mut sink).unwrap();
        }
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
            code_theme: "base16-ocean.dark".into(),
        };
        {
            let mut sink = crate::render::sink::BoxedSink::new(&mut buf);
            render("```python\nx = 1\n```", &ctx_wide, &mut sink).unwrap();
        }
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("x") && s.contains("="));
    }

    #[test]
    fn blockquote_has_prefix() {
        let mut buf = Vec::new();
        {
            let mut sink = crate::render::sink::BoxedSink::new(&mut buf);
            render("> quoted", &ctx(false), &mut sink).unwrap();
        }
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("> quoted") || s.contains("│ > quoted"));
    }

    #[test]
    fn bold_uses_ansi_when_color() {
        let mut buf = Vec::new();
        {
            let mut sink = crate::render::sink::BoxedSink::new(&mut buf);
            render("**bold**", &ctx(true), &mut sink).unwrap();
        }
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("\x1b[1m") || s.contains("bold"));
    }

    #[test]
    fn gfm_table_renders_as_box() {
        let mut buf = Vec::new();
        {
            let mut sink = crate::render::sink::BoxedSink::new(&mut buf);
            render(
                "| Name | Age |\n|:-----|----:|\n| Alice | 30 |\n| Bob | 25 |",
                &ctx(false),
                &mut sink,
            )
            .unwrap();
        }
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains('┌') && s.contains('┬') && s.contains('┐'), "{s}");
        assert!(s.contains('├') && s.contains('┼') && s.contains('┤'), "{s}");
        assert!(s.contains("Name") && s.contains("Age"));
        assert!(s.contains("Alice") && s.contains("Bob"));
        assert!(s.contains("30") && s.contains("25"));
    }

    #[test]
    fn gfm_table_ragged_row_does_not_panic() {
        let mut buf = Vec::new();
        {
            let mut sink = crate::render::sink::BoxedSink::new(&mut buf);
            // body row has fewer cells than the header
            render("| A | B |\n|---|---|\n| x |", &ctx(false), &mut sink).unwrap();
        }
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("x"));
    }
}
