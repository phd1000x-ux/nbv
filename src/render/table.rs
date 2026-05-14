use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

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
    match align {
        Align::Left => format!("{}{}", s, " ".repeat(pad)),
        Align::Right => format!("{}{}", " ".repeat(pad), s),
        Align::Center => {
            let left = pad / 2;
            let right = pad - left;
            format!("{}{}{}", " ".repeat(left), s, " ".repeat(right))
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
