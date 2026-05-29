use std::io::{self, Write};
use std::ops::Range;

use crate::ipynb::model::{Cell, Notebook};

/// Plain-text renderer: no frames, no color, no images.
/// Emits prefixed blocks (`[markdown]`, `[code]`, ...) separated by a
/// single blank line. Suitable for piping into an LLM or grep.
pub fn render_notebook_plain(
    nb: &Notebook,
    cells_range: Option<Range<usize>>,
    no_output: bool,
    code_only: bool,
    w: &mut impl Write,
) -> io::Result<()> {
    let range = cells_range.unwrap_or(0..nb.cells.len());
    let mut first = true;
    for idx in range {
        let Some(cell) = nb.cells.get(idx) else { break };
        if code_only && !matches!(cell, Cell::Code { .. }) {
            continue;
        }
        render_cell_plain(cell, no_output, &mut first, w)?;
        w.flush()?;
    }
    Ok(())
}

fn render_cell_plain(
    cell: &Cell,
    no_output: bool,
    first: &mut bool,
    w: &mut impl Write,
) -> io::Result<()> {
    match cell {
        Cell::Markdown { source } => {
            emit_block("markdown", source, first, w)?;
        }
        Cell::Code { source, outputs, .. } => {
            emit_block("code", source, first, w)?;
            if !no_output {
                for out in outputs {
                    render_output_plain(out, first, w)?;
                }
            }
        }
        Cell::Raw { source } => {
            emit_block("raw", source, first, w)?;
        }
        Cell::Unknown => {
            // Same policy as the box-drawn path's "unknown": skip silently
            // in the plain output (no useful text to emit).
        }
    }
    Ok(())
}

fn render_output_plain(
    _out: &crate::ipynb::model::Output,
    _first: &mut bool,
    _w: &mut impl Write,
) -> io::Result<()> {
    // Filled in by Task 8.
    Ok(())
}

/// Write one prefixed block. Inserts a single blank line before every
/// block except the very first.
fn emit_block(
    prefix: &str,
    body: &str,
    first: &mut bool,
    w: &mut impl Write,
) -> io::Result<()> {
    if !*first {
        w.write_all(b"\n")?;
    }
    *first = false;
    writeln!(w, "[{}]", prefix)?;
    w.write_all(body.as_bytes())?;
    if !body.ends_with('\n') {
        w.write_all(b"\n")?;
    }
    Ok(())
}
