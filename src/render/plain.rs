use std::io::{self, Write};
use std::ops::Range;

use base64::Engine;

use crate::ipynb::model::{Cell, Notebook, Output, StreamName};
use crate::render::image::png_info;
use crate::render::traceback::strip_ansi_pub as strip_ansi;

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
    out: &Output,
    first: &mut bool,
    w: &mut impl Write,
) -> io::Result<()> {
    match out {
        Output::Stream { name, text } => {
            let prefix = match name {
                StreamName::Stdout => "stdout",
                StreamName::Stderr => "stderr",
            };
            emit_block(prefix, text, first, w)?;
        }
        Output::ExecuteResult { data, .. } | Output::DisplayData { data } => {
            if let Some(b64) = &data.image_png {
                let dims = base64::engine::general_purpose::STANDARD
                    .decode(b64)
                    .ok()
                    .and_then(|bytes| png_info::dimensions(&bytes));
                let body = match dims {
                    Some((w_px, h_px)) => format!("PNG {}x{}", w_px, h_px),
                    None => "PNG (size unknown)".to_string(),
                };
                emit_block("image", &body, first, w)?;
                return Ok(());
            }
            if let Some(t) = &data.text_plain {
                emit_block("result", t, first, w)?;
                return Ok(());
            }
            if let Some(h) = &data.text_html {
                emit_block("result", h, first, w)?;
                return Ok(());
            }
            // Unknown/empty mime bundle: skip — nothing useful to emit.
        }
        Output::Error { traceback, .. } => {
            let joined: String = traceback
                .iter()
                .map(|line| strip_ansi(line))
                .collect::<Vec<_>>()
                .join("\n");
            emit_block("error", &joined, first, w)?;
        }
        Output::Unknown => {
            // Same policy as the cell-level Unknown: skip silently.
        }
    }
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
    if !body.is_empty() && !body.ends_with('\n') {
        w.write_all(b"\n")?;
    }
    Ok(())
}
