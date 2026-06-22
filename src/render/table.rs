use std::io::{self, Write};

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::env::RenderCtx;
use crate::render::frame;
use crate::render::pad::push_spaces;
use crate::theme;

/// Per-column text alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

/// A rectangular table: one header row, N body rows, per-column alignment.
/// Construct with `Table::new`, which guarantees every row and `align` are
/// exactly `headers.len()` wide.
pub struct Table {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub align: Vec<Align>,
}

impl Table {
    /// Build a `Table`, normalizing every body row and the alignment vector to
    /// `headers.len()` columns: short rows are padded with empty cells, long
    /// rows truncated; a short `align` is padded with `Align::Left`.
    pub fn new(headers: Vec<String>, mut rows: Vec<Vec<String>>, mut align: Vec<Align>) -> Table {
        let ncols = headers.len();
        for row in &mut rows {
            normalize_row(row, ncols);
        }
        if align.len() < ncols {
            align.resize(ncols, Align::Left);
        } else {
            align.truncate(ncols);
        }
        Table {
            headers,
            rows,
            align,
        }
    }
}

/// Pad `row` with empty cells or truncate it so it has exactly `ncols` entries.
fn normalize_row(row: &mut Vec<String>, ncols: usize) {
    if row.len() < ncols {
        row.resize(ncols, String::new());
    } else {
        row.truncate(ncols);
    }
}

/// Truncate `s` to at most `width` display columns, appending `…` if it was cut.
/// Unicode-width aware: a CJK char counts as 2 columns.
fn truncate_cell(s: &str, width: usize) -> String {
    if UnicodeWidthStr::width(s) <= width {
        return s.to_string();
    }
    if width == 0 {
        return String::new();
    }
    let budget = width - 1; // reserve 1 column for the ellipsis
    let mut out = String::new();
    let mut used = 0usize;
    for ch in s.chars() {
        let cw = UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + cw > budget {
            break;
        }
        out.push(ch);
        used += cw;
    }
    out.push('…');
    out
}

/// Pad `s` to exactly `width` display columns according to `align`.
/// Assumes `s` already fits within `width` (caller truncates first).
fn pad_cell(s: &str, width: usize, align: Align) -> String {
    let w = UnicodeWidthStr::width(s);
    if w >= width {
        return s.to_string();
    }
    let pad = width - w;
    let mut out = String::with_capacity(s.len() + pad);
    match align {
        Align::Left => {
            out.push_str(s);
            push_spaces(&mut out, pad);
        }
        Align::Right => {
            push_spaces(&mut out, pad);
            out.push_str(s);
        }
        Align::Center => {
            let left = pad / 2;
            push_spaces(&mut out, left);
            out.push_str(s);
            push_spaces(&mut out, pad - left);
        }
    }
    out
}

/// Largest natural width a single column is allowed before shrinking kicks in.
const MAX_COL: usize = 40;
/// Smallest width a column may be shrunk to before columns get dropped instead.
const MIN_COL: usize = 5;

/// Result of fitting a table's columns to a width budget.
struct Fit {
    /// Final display width of each kept column.
    widths: Vec<usize>,
    /// Number of columns dropped from the right (0 if all fit).
    dropped: usize,
}

/// Total rendered width of a table whose columns have the given widths:
/// a leading `│`, then `" {content} │"` (content + 3) per column.
fn rendered_width(widths: &[usize]) -> usize {
    1 + widths.iter().map(|w| w + 3).sum::<usize>()
}

/// Compute per-column display widths that fit the table into `budget` columns.
///
/// 1. natural width = min(widest cell in the column, `MAX_COL`)
/// 2. if the table already fits, done
/// 3. otherwise repeatedly shrink the widest column down toward `MIN_COL`
/// 4. if every column at `MIN_COL` still overflows, drop columns from the
///    right, reserving room for a trailing `+N` indicator column
fn fit_columns(table: &Table, budget: usize) -> Fit {
    let n = table.headers.len();
    if n == 0 {
        return Fit {
            widths: Vec::new(),
            dropped: 0,
        };
    }

    let mut widths: Vec<usize> = (0..n)
        .map(|c| {
            let header_w = UnicodeWidthStr::width(table.headers[c].as_str());
            let cell_w = table
                .rows
                .iter()
                .map(|r| UnicodeWidthStr::width(r[c].as_str()))
                .max()
                .unwrap_or(0);
            header_w.max(cell_w).clamp(1, MAX_COL)
        })
        .collect();

    if rendered_width(&widths) <= budget {
        return Fit { widths, dropped: 0 };
    }

    // Shrink the widest column (above the floor) until it fits or all are at MIN_COL.
    while rendered_width(&widths) > budget {
        let widest = widths
            .iter()
            .enumerate()
            .filter(|(_, w)| **w > MIN_COL)
            .max_by_key(|(_, w)| **w)
            .map(|(i, _)| i);
        match widest {
            Some(i) => widths[i] -= 1,
            None => break,
        }
    }

    if rendered_width(&widths) <= budget {
        return Fit { widths, dropped: 0 };
    }

    // Drop columns from the right, leaving room for a "+N" indicator column.
    let mut dropped = 0;
    while widths.len() > 1 {
        widths.pop();
        dropped += 1;
        let indicator_w = format!("+{}", dropped).len() + 3;
        if rendered_width(&widths) + indicator_w <= budget {
            break;
        }
    }

    Fit { widths, dropped }
}

/// Render `table` as a box-drawn grid, emitting each full line through
/// `frame::wrap_line` so it sits inside the surrounding cell box.
pub fn render(table: &Table, ctx: &RenderCtx, w: &mut impl Write) -> io::Result<()> {
    let budget = ctx.width.saturating_sub(4);
    let fit = fit_columns(table, budget);
    if fit.widths.is_empty() {
        return Ok(());
    }
    let kept = fit.widths.len();

    let mut widths = fit.widths;
    let mut headers: Vec<String> = table.headers[..kept].to_vec();
    let mut align: Vec<Align> = table.align[..kept].to_vec();
    if fit.dropped > 0 {
        let label = format!("+{}", fit.dropped);
        widths.push(UnicodeWidthStr::width(label.as_str()).max(1));
        headers.push(label);
        align.push(Align::Left);
    }

    border_line(&widths, '┌', '┬', '┐', ctx, w)?;

    let header_cells: Vec<String> = headers
        .iter()
        .enumerate()
        .map(|(i, h)| pad_cell(&truncate_cell(h, widths[i]), widths[i], align[i]))
        .collect();
    data_line(&header_cells, ctx.use_color, ctx, w)?;

    border_line(&widths, '├', '┼', '┤', ctx, w)?;

    for row in &table.rows {
        let mut cells: Vec<String> = (0..kept)
            .map(|i| pad_cell(&truncate_cell(&row[i], widths[i]), widths[i], align[i]))
            .collect();
        if fit.dropped > 0 {
            cells.push(pad_cell("…", widths[kept], Align::Left));
        }
        data_line(&cells, false, ctx, w)?;
    }

    border_line(&widths, '└', '┴', '┘', ctx, w)?;
    Ok(())
}

/// Emit a horizontal border line, e.g. `┌───┬───┐`, through `frame::wrap_line`.
fn border_line(
    widths: &[usize],
    left: char,
    mid: char,
    right: char,
    ctx: &RenderCtx,
    w: &mut impl Write,
) -> io::Result<()> {
    let mut line = String::new();
    line.push(left);
    for (i, width) in widths.iter().enumerate() {
        line.push_str(&"─".repeat(width + 2));
        line.push(if i + 1 == widths.len() { right } else { mid });
    }
    // DIM the whole border line (it is pure box-drawing characters).
    let line = if ctx.use_color {
        format!("{}{}{}", theme::DIM, line, theme::RESET)
    } else {
        line
    };
    frame::wrap_line(&line, ctx, w)
}

/// Emit a content row, e.g. `│ a │ b │`, through `frame::wrap_line`.
/// When `bold` is true each cell is wrapped in BOLD/RESET (the header row).
fn data_line(cells: &[String], bold: bool, ctx: &RenderCtx, w: &mut impl Write) -> io::Result<()> {
    // DIM the `│` separators; cell content stays at its own intensity.
    let bar = if ctx.use_color {
        format!("{}│{}", theme::DIM, theme::RESET)
    } else {
        "│".to_string()
    };
    let mut line = String::new();
    line.push_str(&bar);
    for cell in cells {
        line.push(' ');
        if bold {
            line.push_str(theme::BOLD);
            line.push_str(cell);
            line.push_str(theme::RESET);
        } else {
            line.push_str(cell);
        }
        line.push(' ');
        line.push_str(&bar);
    }
    frame::wrap_line(&line, ctx, w)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::env::{ImageBackend, RenderCtx};

    fn ctx(width: usize) -> RenderCtx {
        RenderCtx {
            is_tty: true,
            use_color: false,
            width,
            image_backend: ImageBackend::Placeholder,
            code_theme: "base16-ocean.dark".into(),
        }
    }

    #[test]
    fn new_pads_short_rows_and_truncates_long_rows() {
        let t = Table::new(
            vec!["a".into(), "b".into()],
            vec![vec!["1".into()], vec!["1".into(), "2".into(), "3".into()]],
            vec![Align::Left, Align::Left],
        );
        assert_eq!(t.rows[0], vec!["1".to_string(), String::new()]);
        assert_eq!(t.rows[1], vec!["1".to_string(), "2".to_string()]);
    }

    #[test]
    fn new_pads_short_align_with_left() {
        let t = Table::new(
            vec!["a".into(), "b".into(), "c".into()],
            vec![],
            vec![Align::Right],
        );
        assert_eq!(t.align, vec![Align::Right, Align::Left, Align::Left]);
    }

    #[test]
    fn truncate_cell_leaves_short_strings_untouched() {
        assert_eq!(truncate_cell("abc", 5), "abc");
    }

    #[test]
    fn truncate_cell_cuts_with_ellipsis() {
        assert_eq!(truncate_cell("abcdef", 4), "abc…");
    }

    #[test]
    fn truncate_cell_is_width_aware_for_cjk() {
        // each CJK char is 2 columns wide; width 5 fits 2 chars (4 cols) + ellipsis
        assert_eq!(truncate_cell("가나다라", 5), "가나…");
    }

    #[test]
    fn pad_cell_left_right_center() {
        assert_eq!(pad_cell("ab", 5, Align::Left), "ab   ");
        assert_eq!(pad_cell("ab", 5, Align::Right), "   ab");
        assert_eq!(pad_cell("ab", 5, Align::Center), " ab  ");
    }

    #[test]
    fn pad_cell_spans_chunk_boundary() {
        // 70-col pad exceeds the 32-byte SPACES chunk, exercising the loop +
        // remainder split in push_spaces. Output stays exactly `width` wide.
        let out = pad_cell("x", 70, Align::Left);
        assert_eq!(UnicodeWidthStr::width(out.as_str()), 70);
        assert_eq!(out, format!("x{}", " ".repeat(69)));
        assert_eq!(
            pad_cell("x", 70, Align::Right),
            format!("{}x", " ".repeat(69))
        );
        // Center: 69 pad -> 34 left, 35 right
        assert_eq!(
            pad_cell("x", 70, Align::Center),
            format!("{}x{}", " ".repeat(34), " ".repeat(35))
        );
    }

    #[test]
    fn fit_keeps_natural_widths_when_table_fits() {
        let t = Table::new(
            vec!["name".into(), "age".into()],
            vec![vec!["Alice".into(), "30".into()]],
            vec![Align::Left, Align::Left],
        );
        let fit = fit_columns(&t, 80);
        // "Alice" is 5 wide, "name" is 4 -> 5; "age" is 3 -> "30" is 2 -> 3
        assert_eq!(fit.widths, vec![5, 3]);
        assert_eq!(fit.dropped, 0);
    }

    #[test]
    fn fit_shrinks_wide_columns_to_budget() {
        let wide = "x".repeat(60);
        let t = Table::new(
            vec!["a".into(), "b".into()],
            vec![vec![wide.clone(), wide]],
            vec![Align::Left, Align::Left],
        );
        let fit = fit_columns(&t, 30);
        assert_eq!(fit.dropped, 0);
        assert!(rendered_width(&fit.widths) <= 30, "widths {:?}", fit.widths);
    }

    #[test]
    fn fit_drops_columns_when_budget_too_small() {
        let t = Table::new(
            vec!["a".into(), "b".into(), "c".into(), "d".into()],
            vec![vec!["1".into(), "2".into(), "3".into(), "4".into()]],
            vec![Align::Left; 4],
        );
        let fit = fit_columns(&t, 16);
        assert!(fit.dropped > 0, "expected some columns dropped");
        assert_eq!(fit.widths.len() + fit.dropped, 4);
        let indicator_w = format!("+{}", fit.dropped).len() + 3;
        assert!(
            rendered_width(&fit.widths) + indicator_w <= 16,
            "kept+indicator must fit: {:?} + {} > 16",
            fit.widths,
            indicator_w
        );
    }

    #[test]
    fn render_draws_grid_with_headers_and_rows() {
        let t = Table::new(
            vec!["name".into(), "age".into()],
            vec![
                vec!["Alice".into(), "30".into()],
                vec!["Bob".into(), "25".into()],
            ],
            vec![Align::Left, Align::Right],
        );
        let mut buf = Vec::new();
        render(&t, &ctx(40), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        // box-drawing grid
        assert!(s.contains('┌') && s.contains('┬') && s.contains('┐'));
        assert!(s.contains('├') && s.contains('┼') && s.contains('┤'));
        assert!(s.contains('└') && s.contains('┴') && s.contains('┘'));
        // header and cell values
        assert!(s.contains("name") && s.contains("age"));
        assert!(s.contains("Alice") && s.contains("Bob"));
        // every emitted line is exactly ctx.width wide (frame::wrap_line pads)
        for line in s.lines() {
            assert_eq!(line.chars().count(), 40, "line not full width: {line:?}");
        }
    }

    #[test]
    fn render_header_only_table_emits_no_body_rows() {
        let t = Table::new(vec!["a".into(), "b".into()], vec![], vec![Align::Left; 2]);
        let mut buf = Vec::new();
        render(&t, &ctx(30), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        // top border, header, separator, bottom border = 4 lines
        assert_eq!(s.lines().count(), 4);
    }
}
