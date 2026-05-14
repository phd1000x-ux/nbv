use crate::render::table::{Align, Table};

/// Decode the small set of HTML entities pandas `.to_html()` emits, plus
/// numeric character references (`&#NN;`). Unknown entities and stray `&`
/// without a closing `;` are left untouched.
fn decode_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp..];
        let Some(semi) = rest.find(';') else {
            out.push_str(rest);
            rest = "";
            break;
        };
        let entity = &rest[1..semi];
        let decoded = match entity {
            "lt" => Some('<'),
            "gt" => Some('>'),
            "amp" => Some('&'),
            "quot" => Some('"'),
            "#39" | "apos" => Some('\''),
            "nbsp" => Some(' '),
            _ => entity
                .strip_prefix('#')
                .and_then(|n| n.parse::<u32>().ok())
                .and_then(char::from_u32),
        };
        match decoded {
            Some(c) => {
                out.push(c);
                rest = &rest[semi + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// A token from a raw HTML fragment: either a tag or a run of text.
enum Token<'a> {
    Tag { name: String, closing: bool },
    Text(&'a str),
}

/// Split an HTML fragment into tags and text runs. Tag names are lowercased;
/// attributes are discarded. Unterminated `<` is treated as text.
fn tokenize(s: &str) -> Vec<Token<'_>> {
    let mut tokens = Vec::new();
    let mut rest = s;
    while !rest.is_empty() {
        let Some(lt) = rest.find('<') else {
            tokens.push(Token::Text(rest));
            break;
        };
        if lt > 0 {
            tokens.push(Token::Text(&rest[..lt]));
        }
        rest = &rest[lt..];
        let Some(gt) = rest.find('>') else {
            tokens.push(Token::Text(rest));
            break;
        };
        let raw = &rest[1..gt];
        let closing = raw.starts_with('/');
        let name = raw
            .trim_start_matches('/')
            .trim()
            .split(|c: char| c.is_whitespace())
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        tokens.push(Token::Tag { name, closing });
        rest = &rest[gt + 1..];
    }
    tokens
}

/// Parse the first `<table>` element in `html` into a `Table`.
///
/// Returns `None` (caller falls back to `text/plain`) when there is no
/// `<table>`, when `<thead>` holds more than one `<tr>` (a pandas MultiIndex),
/// or when there are no body rows.
pub fn parse(html: &str) -> Option<Table> {
    let lower = html.to_ascii_lowercase();
    let table_start = lower.find("<table")?;
    let table_end = lower[table_start..]
        .find("</table>")
        .map(|e| table_start + e)?;
    let body = &html[table_start..table_end];

    let mut rows_in_thead: Vec<Vec<String>> = Vec::new();
    let mut rows_in_body: Vec<Vec<String>> = Vec::new();
    let mut in_thead = false;
    let mut current_row: Vec<String> = Vec::new();
    let mut current_cell: Option<String> = None;
    let mut row_open = false;

    for tok in tokenize(body) {
        match tok {
            Token::Tag { name, closing } => match (name.as_str(), closing) {
                ("thead", false) => in_thead = true,
                ("thead", true) => in_thead = false,
                ("tr", false) => {
                    current_row = Vec::new();
                    row_open = true;
                }
                ("tr", true) if row_open => {
                    if in_thead {
                        rows_in_thead.push(std::mem::take(&mut current_row));
                    } else {
                        rows_in_body.push(std::mem::take(&mut current_row));
                    }
                    row_open = false;
                }
                ("th", false) | ("td", false) => current_cell = Some(String::new()),
                ("th", true) | ("td", true) => {
                    if let Some(cell) = current_cell.take() {
                        current_row.push(decode_entities(&cell).trim().to_string());
                    }
                }
                _ => {}
            },
            Token::Text(t) => {
                if let Some(cell) = current_cell.as_mut() {
                    cell.push_str(t);
                }
            }
        }
    }

    if rows_in_thead.len() > 1 {
        return None; // MultiIndex
    }

    let mut headers = if rows_in_thead.len() == 1 {
        rows_in_thead.pop().unwrap()
    } else if !rows_in_body.is_empty() {
        rows_in_body.remove(0)
    } else {
        return None;
    };
    let rows = rows_in_body;
    if rows.is_empty() {
        return None;
    }

    let ncols = headers
        .len()
        .max(rows.iter().map(Vec::len).max().unwrap_or(0));
    if ncols == 0 {
        return None;
    }
    headers.resize(ncols, String::new());

    // A column whose every non-empty body cell is numeric is right-aligned.
    let align: Vec<Align> = (0..ncols)
        .map(|c| {
            let non_empty: Vec<&str> = rows
                .iter()
                .filter_map(|r| r.get(c))
                .map(String::as_str)
                .filter(|v| !v.trim().is_empty())
                .collect();
            if !non_empty.is_empty() && non_empty.iter().all(|v| v.trim().parse::<f64>().is_ok()) {
                Align::Right
            } else {
                Align::Left
            }
        })
        .collect();

    Some(Table::new(headers, rows, align))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_named_entities() {
        assert_eq!(decode_entities("&lt;a&gt;"), "<a>");
        assert_eq!(decode_entities("x &amp; y"), "x & y");
        assert_eq!(decode_entities("say &quot;hi&quot;"), "say \"hi\"");
        assert_eq!(decode_entities("it&#39;s"), "it's");
        assert_eq!(decode_entities("a&nbsp;b"), "a b");
    }

    #[test]
    fn decodes_numeric_entities() {
        assert_eq!(decode_entities("&#65;&#66;"), "AB");
    }

    #[test]
    fn leaves_unknown_or_unterminated_alone() {
        assert_eq!(decode_entities("100% &done"), "100% &done");
        assert_eq!(decode_entities("&notanentity;"), "&notanentity;");
        assert_eq!(decode_entities("plain text"), "plain text");
    }

    const PANDAS_HTML: &str = r#"<table border="1" class="dataframe"><thead><tr style="text-align: right;"><th></th><th>name</th><th>age</th></tr></thead><tbody><tr><th>0</th><td>Alice</td><td>30</td></tr><tr><th>1</th><td>Bob</td><td>25</td></tr></tbody></table>"#;

    #[test]
    fn parses_pandas_dataframe_table() {
        let t = parse(PANDAS_HTML).expect("should parse");
        assert_eq!(t.headers, vec!["", "name", "age"]);
        assert_eq!(t.rows[0], vec!["0", "Alice", "30"]);
        assert_eq!(t.rows[1], vec!["1", "Bob", "25"]);
    }

    #[test]
    fn numeric_columns_are_right_aligned() {
        let t = parse(PANDAS_HTML).expect("should parse");
        // col 0 ("0","1") numeric, col 1 ("Alice","Bob") text, col 2 ("30","25") numeric
        assert_eq!(t.align, vec![Align::Right, Align::Left, Align::Right]);
    }

    #[test]
    fn decodes_entities_in_cells() {
        let html = "<table><thead><tr><th>op</th></tr></thead><tbody><tr><td>a &lt; b</td></tr></tbody></table>";
        let t = parse(html).expect("should parse");
        assert_eq!(t.rows[0], vec!["a < b"]);
    }

    #[test]
    fn rejects_multiindex_header() {
        let html = "<table><thead><tr><th>a</th></tr><tr><th>b</th></tr></thead><tbody><tr><td>1</td></tr></tbody></table>";
        assert!(parse(html).is_none());
    }

    #[test]
    fn rejects_non_table_html() {
        assert!(parse("<div>not a table</div>").is_none());
    }

    #[test]
    fn rejects_table_with_no_body_rows() {
        let html = "<table><thead><tr><th>a</th></tr></thead></table>";
        assert!(parse(html).is_none());
    }

    #[test]
    fn strips_nested_tags_inside_cells() {
        let html = "<table><thead><tr><th>v</th></tr></thead><tbody><tr><td><b>bold</b></td></tr></tbody></table>";
        let t = parse(html).expect("should parse");
        assert_eq!(t.rows[0], vec!["bold"]);
    }
}
