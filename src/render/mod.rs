pub mod code;
pub mod document;
pub mod frame;
pub mod html_table;
pub mod image;
pub mod markdown;
pub mod output;
mod pad;
pub mod plain;
pub mod sink;
pub mod table;
#[cfg(test)]
pub(crate) mod test_support;
pub mod text;
pub mod traceback;

use std::io::{self, Write};
use std::ops::Range;

use crate::env::RenderCtx;
use crate::ipynb::model::{Cell, Notebook};
use crate::theme;

/// Render-time filtering decisions, resolved in `main.rs` after flag/env
/// precedence and after applying implications (`--code-only` ⟹ `--no-output`).
#[derive(Debug, Default, Clone)]
pub struct RenderFilters {
    /// 0-based half-open cell range. `None` = all cells.
    /// Already clamped against `nb.cells.len()` by the caller.
    pub cells_range: Option<Range<usize>>,
    /// Skip the `outputs[]` of every code cell.
    pub no_output: bool,
    /// Only render `Cell::Code`; drop everything else.
    pub code_only: bool,
    /// Use the plain-text render path instead of box-drawing.
    pub plain: bool,
}

/// 노트북 전체를 셀 단위로 렌더. 매 셀 후 flush.
pub fn render_notebook(
    nb: &Notebook,
    filters: &RenderFilters,
    ctx: &RenderCtx,
    w: &mut impl Write,
) -> io::Result<()> {
    if filters.plain {
        return plain::render_notebook_plain(
            nb,
            filters.cells_range.clone(),
            filters.no_output,
            filters.code_only,
            w,
        );
    }
    let lang = nb
        .metadata
        .kernelspec
        .as_ref()
        .and_then(|k| k.language.clone())
        .or_else(|| nb.metadata.language_info.as_ref().map(|l| l.name.clone()))
        .unwrap_or_else(|| "python".into());
    let range = filters.cells_range.clone().unwrap_or(0..nb.cells.len());
    for idx in range {
        let Some(cell) = nb.cells.get(idx) else { break };
        if filters.code_only && !matches!(cell, Cell::Code { .. }) {
            continue;
        }
        render_cell(cell, idx, &lang, ctx, filters.no_output, w)?;
        w.flush()?;
    }
    Ok(())
}

pub fn render_cell(
    cell: &Cell,
    idx: usize,
    lang: &str,
    ctx: &RenderCtx,
    no_output: bool,
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
            {
                let mut sink = crate::render::sink::BoxedSink::new(w);
                code::render(source, lang, ctx, &mut sink)?;
            }
            frame::close(ctx, w)?;
            if !no_output {
                for (i, out) in outputs.iter().enumerate() {
                    output::render(out, idx, i, ctx, w)?;
                }
            }
        }
        Cell::Markdown { source } => {
            let label = theme::colorize_markdown_header("markdown", ctx.use_color);
            frame::open(&label, ctx, w)?;
            {
                let mut sink = crate::render::sink::BoxedSink::new(w);
                markdown::render(source, ctx, &mut sink)?;
            }
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
    use crate::ipynb::parse;

    fn ctx() -> RenderCtx {
        crate::render::test_support::base()
    }

    #[test]
    fn renders_minimal_notebook_without_panicking() {
        let nb = parse::from_str(r##"{"cells":[{"cell_type":"markdown","source":"# Hi","metadata":{}}],"metadata":{},"nbformat":4,"nbformat_minor":5}"##).unwrap();
        let mut buf = Vec::new();
        render_notebook(&nb, &RenderFilters::default(), &ctx(), &mut buf).unwrap();
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
        render_notebook(&nb, &RenderFilters::default(), &ctx(), &mut buf).unwrap();
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
        render_notebook(&nb, &RenderFilters::default(), &ctx(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("Unknown") || s.contains("skipped"));
        assert!(s.contains("normal"));
    }

    #[test]
    fn default_filters_render_all_cells() {
        let nb = parse::from_str(
            r##"{
            "cells":[
                {"cell_type":"markdown","source":"# A","metadata":{}},
                {"cell_type":"code","source":"x=1","metadata":{},"execution_count":1,"outputs":[]}
            ],
            "metadata":{},"nbformat":4,"nbformat_minor":5
        }"##,
        )
        .unwrap();
        let mut buf = Vec::new();
        render_notebook(&nb, &RenderFilters::default(), &ctx(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("# A"));
        assert!(s.contains("x=1"));
    }

    fn fixture_5_cells() -> Notebook {
        parse::from_str(
            r##"{
            "cells":[
                {"cell_type":"markdown","source":"MD0","metadata":{}},
                {"cell_type":"markdown","source":"MD1","metadata":{}},
                {"cell_type":"markdown","source":"MD2","metadata":{}},
                {"cell_type":"markdown","source":"MD3","metadata":{}},
                {"cell_type":"markdown","source":"MD4","metadata":{}}
            ],
            "metadata":{},"nbformat":4,"nbformat_minor":5
        }"##,
        )
        .unwrap()
    }

    #[test]
    fn cells_range_renders_only_slice() {
        let nb = fixture_5_cells();
        // 0-based half-open: cells 1 and 2
        let f = RenderFilters {
            cells_range: Some(1..3),
            ..Default::default()
        };
        let mut buf = Vec::new();
        render_notebook(&nb, &f, &ctx(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(!s.contains("MD0"));
        assert!(s.contains("MD1"));
        assert!(s.contains("MD2"));
        assert!(!s.contains("MD3"));
        assert!(!s.contains("MD4"));
    }

    #[test]
    fn empty_cells_range_renders_nothing() {
        let nb = fixture_5_cells();
        let f = RenderFilters {
            cells_range: Some(10..20),
            ..Default::default()
        };
        let mut buf = Vec::new();
        render_notebook(&nb, &f, &ctx(), &mut buf).unwrap();
        assert!(buf.is_empty());
    }

    fn fixture_md_code_code() -> Notebook {
        parse::from_str(
            r##"{
            "cells":[
                {"cell_type":"markdown","source":"INTRO_MD","metadata":{}},
                {"cell_type":"code","source":"print('A')","metadata":{},"execution_count":1,
                    "outputs":[{"output_type":"stream","name":"stdout","text":"A\n"}]},
                {"cell_type":"code","source":"print('B')","metadata":{},"execution_count":2,
                    "outputs":[{"output_type":"stream","name":"stdout","text":"B\n"}]}
            ],
            "metadata":{},"nbformat":4,"nbformat_minor":5
        }"##,
        )
        .unwrap()
    }

    #[test]
    fn no_output_hides_stream_outputs_keeps_md_and_code() {
        let nb = fixture_md_code_code();
        let f = RenderFilters {
            no_output: true,
            ..Default::default()
        };
        let mut buf = Vec::new();
        render_notebook(&nb, &f, &ctx(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("INTRO_MD"));
        assert!(s.contains("print('A')"));
        assert!(
            !s.contains("stream (stdout)"),
            "no-output must hide Out frames: {s}"
        );
    }

    #[test]
    fn code_only_drops_markdown_and_outputs() {
        let nb = fixture_md_code_code();
        let f = RenderFilters {
            code_only: true,
            no_output: true,
            ..Default::default()
        };
        // main.rs will set no_output when code_only is set; we mirror that here.
        let mut buf = Vec::new();
        render_notebook(&nb, &f, &ctx(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(!s.contains("INTRO_MD"));
        assert!(s.contains("print('A')"));
        assert!(s.contains("print('B')"));
        assert!(!s.contains("stream (stdout)"));
    }

    #[test]
    fn plain_renders_markdown_with_prefix() {
        let nb = parse::from_str(
            r##"{
            "cells":[{"cell_type":"markdown","source":"# Hello\nWorld","metadata":{}}],
            "metadata":{},"nbformat":4,"nbformat_minor":5
        }"##,
        )
        .unwrap();
        let f = RenderFilters {
            plain: true,
            ..Default::default()
        };
        let mut buf = Vec::new();
        render_notebook(&nb, &f, &ctx(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.starts_with("[markdown]\n"));
        assert!(s.contains("# Hello\nWorld"));
        assert!(!s.contains('┌'), "plain must not draw frames: {s}");
    }

    #[test]
    fn plain_renders_code_source_with_prefix() {
        let nb = parse::from_str(r##"{
            "cells":[{"cell_type":"code","source":"x = 1","metadata":{},"execution_count":1,"outputs":[]}],
            "metadata":{},"nbformat":4,"nbformat_minor":5
        }"##).unwrap();
        let f = RenderFilters {
            plain: true,
            ..Default::default()
        };
        let mut buf = Vec::new();
        render_notebook(&nb, &f, &ctx(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("[code]\nx = 1"));
        assert!(!s.contains('\x1b'), "plain must not contain ANSI: {s}");
    }

    #[test]
    fn plain_separates_blocks_with_one_blank_line() {
        let nb = parse::from_str(
            r##"{
            "cells":[
                {"cell_type":"markdown","source":"A","metadata":{}},
                {"cell_type":"markdown","source":"B","metadata":{}}
            ],
            "metadata":{},"nbformat":4,"nbformat_minor":5
        }"##,
        )
        .unwrap();
        let f = RenderFilters {
            plain: true,
            ..Default::default()
        };
        let mut buf = Vec::new();
        render_notebook(&nb, &f, &ctx(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert_eq!(s, "[markdown]\nA\n\n[markdown]\nB\n");
    }

    #[test]
    fn plain_emits_stream_with_stdout_prefix() {
        let nb = parse::from_str(
            r##"{
            "cells":[{"cell_type":"code","source":"print(1)","metadata":{},"execution_count":1,
                "outputs":[{"output_type":"stream","name":"stdout","text":"1\n"}]}],
            "metadata":{},"nbformat":4,"nbformat_minor":5
        }"##,
        )
        .unwrap();
        let f = RenderFilters {
            plain: true,
            ..Default::default()
        };
        let mut buf = Vec::new();
        render_notebook(&nb, &f, &ctx(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("[code]\nprint(1)"));
        assert!(s.contains("[stdout]\n1"));
    }

    #[test]
    fn plain_emits_stderr_with_stderr_prefix() {
        let nb = parse::from_str(
            r##"{
            "cells":[{"cell_type":"code","source":"x","metadata":{},"execution_count":1,
                "outputs":[{"output_type":"stream","name":"stderr","text":"warn\n"}]}],
            "metadata":{},"nbformat":4,"nbformat_minor":5
        }"##,
        )
        .unwrap();
        let f = RenderFilters {
            plain: true,
            ..Default::default()
        };
        let mut buf = Vec::new();
        render_notebook(&nb, &f, &ctx(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("[stderr]\nwarn"));
    }

    #[test]
    fn plain_emits_execute_result_text_plain() {
        let nb = parse::from_str(
            r##"{
            "cells":[{"cell_type":"code","source":"42","metadata":{},"execution_count":1,
                "outputs":[{"output_type":"execute_result","execution_count":1,
                    "data":{"text/plain":"42"},"metadata":{}}]}],
            "metadata":{},"nbformat":4,"nbformat_minor":5
        }"##,
        )
        .unwrap();
        let f = RenderFilters {
            plain: true,
            ..Default::default()
        };
        let mut buf = Vec::new();
        render_notebook(&nb, &f, &ctx(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("[result]\n42"));
    }

    #[test]
    fn plain_emits_error_with_traceback() {
        let nb = parse::from_str(
            r##"{
            "cells":[{"cell_type":"code","source":"raise","metadata":{},"execution_count":1,
                "outputs":[{"output_type":"error","ename":"ValueError","evalue":"bad",
                    "traceback":["Traceback...","ValueError: bad"]}]}],
            "metadata":{},"nbformat":4,"nbformat_minor":5
        }"##,
        )
        .unwrap();
        let f = RenderFilters {
            plain: true,
            ..Default::default()
        };
        let mut buf = Vec::new();
        render_notebook(&nb, &f, &ctx(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("[error]"));
        assert!(s.contains("ValueError: bad"));
    }

    #[test]
    fn plain_image_emits_dimensions_placeholder() {
        // 1x1 PNG (smallest valid)
        let png_b64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==";
        let nb = parse::from_str(&format!(
            r##"{{
            "cells":[{{"cell_type":"code","source":"plt.show()","metadata":{{}},"execution_count":1,
                "outputs":[{{"output_type":"display_data",
                    "data":{{"image/png":"{}"}},"metadata":{{}}}}]}}],
            "metadata":{{}},"nbformat":4,"nbformat_minor":5
        }}"##,
            png_b64
        ))
        .unwrap();
        let f = RenderFilters {
            plain: true,
            ..Default::default()
        };
        let mut buf = Vec::new();
        render_notebook(&nb, &f, &ctx(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("[image]"));
        assert!(s.contains("PNG"));
    }
}
