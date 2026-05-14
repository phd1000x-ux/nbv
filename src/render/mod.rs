pub mod code;
pub mod frame;
pub mod image;
pub mod markdown;
pub mod output;
pub mod table;
pub mod text;
pub mod traceback;

use std::io::{self, Write};

use crate::env::RenderCtx;
use crate::ipynb::model::{Cell, Notebook};
use crate::theme;

/// 노트북 전체를 셀 단위로 렌더. 매 셀 후 flush.
pub fn render_notebook(nb: &Notebook, ctx: &RenderCtx, w: &mut impl Write) -> io::Result<()> {
    let lang = nb
        .metadata
        .kernelspec
        .as_ref()
        .and_then(|k| k.language.clone())
        .or_else(|| nb.metadata.language_info.as_ref().map(|l| l.name.clone()))
        .unwrap_or_else(|| "python".into());
    for (idx, cell) in nb.cells.iter().enumerate() {
        render_cell(cell, idx, &lang, ctx, w)?;
        w.flush()?;
    }
    Ok(())
}

pub fn render_cell(
    cell: &Cell,
    idx: usize,
    lang: &str,
    ctx: &RenderCtx,
    w: &mut impl Write,
) -> io::Result<()> {
    match cell {
        Cell::Code {
            source,
            outputs,
            execution_count,
        } => {
            let n = execution_count
                .map(|n| n.to_string())
                .unwrap_or_else(|| " ".into());
            let label = format!("In [{}] ── code ({})", n, lang);
            let label = theme::colorize_code_header(&label, ctx.use_color);
            frame::open(&label, ctx, w)?;
            code::render(source, lang, ctx, w)?;
            frame::close(ctx, w)?;
            for (i, out) in outputs.iter().enumerate() {
                output::render(out, idx, i, ctx, w)?;
            }
        }
        Cell::Markdown { source } => {
            let label = theme::colorize_markdown_header("markdown", ctx.use_color);
            frame::open(&label, ctx, w)?;
            markdown::render(source, ctx, w)?;
            frame::close(ctx, w)?;
        }
        Cell::Raw { source } => {
            let label = theme::dim("raw", ctx.use_color);
            frame::open(&label, ctx, w)?;
            text::render(source, ctx, w)?;
            frame::close(ctx, w)?;
        }
        Cell::Unknown => {
            let label = theme::dim("Unknown cell", ctx.use_color);
            frame::open(&label, ctx, w)?;
            frame::wrap_line("(skipped)", ctx, w)?;
            frame::close(ctx, w)?;
            eprintln!("nbv: skipping cell {} with unknown cell_type", idx);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::ImageBackend;
    use crate::ipynb::parse;

    fn ctx() -> RenderCtx {
        RenderCtx {
            is_tty: true,
            use_color: false,
            width: 60,
            image_backend: ImageBackend::Placeholder,
        }
    }

    #[test]
    fn renders_minimal_notebook_without_panicking() {
        let nb = parse::from_str(r##"{"cells":[{"cell_type":"markdown","source":"# Hi","metadata":{}}],"metadata":{},"nbformat":4,"nbformat_minor":5}"##).unwrap();
        let mut buf = Vec::new();
        render_notebook(&nb, &ctx(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("Hi"));
    }

    #[test]
    fn renders_code_with_outputs() {
        let nb = parse::from_str(
            r#"{
            "cells":[{"cell_type":"code","source":"print(1)","metadata":{},"execution_count":1,
                "outputs":[{"output_type":"stream","name":"stdout","text":"1\n"}]}],
            "metadata":{},"nbformat":4,"nbformat_minor":5
        }"#,
        )
        .unwrap();
        let mut buf = Vec::new();
        render_notebook(&nb, &ctx(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("print(1)") || s.contains("print"));
        assert!(s.contains("In [1]"));
    }

    #[test]
    fn unknown_cell_logs_to_stderr_and_continues() {
        let nb = parse::from_str(
            r#"{
            "cells":[
                {"cell_type":"weird","source":"x","metadata":{}},
                {"cell_type":"markdown","source":"normal","metadata":{}}
            ],
            "metadata":{},"nbformat":4,"nbformat_minor":5
        }"#,
        )
        .unwrap();
        let mut buf = Vec::new();
        render_notebook(&nb, &ctx(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("Unknown") || s.contains("skipped"));
        assert!(s.contains("normal"));
    }
}
