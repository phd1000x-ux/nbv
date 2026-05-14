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
}
