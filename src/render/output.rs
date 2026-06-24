use std::io::{self, Write};

use crate::env::RenderCtx;
use crate::ipynb::model::{MimeBundle, Output};
use crate::render::{frame, html_table, image, sink, table, text, traceback};
use crate::theme;

/// 단일 Output을 박스 안에서 렌더.
pub fn render(
    out: &Output,
    cell_idx: usize,
    out_idx: usize,
    ctx: &RenderCtx,
    w: &mut impl Write,
) -> io::Result<()> {
    match out {
        Output::Stream { name, text: t } => {
            let label = out_label(None, &format!("stream ({})", name.display_name()));
            header(&label, ctx, w)?;
            text::render(t, ctx, w)?;
            frame::close(ctx, w)?;
        }
        Output::ExecuteResult {
            data,
            execution_count,
        } => {
            render_bundle(data, *execution_count, cell_idx, out_idx, ctx, w)?;
        }
        Output::DisplayData { data } => {
            render_bundle(data, None, cell_idx, out_idx, ctx, w)?;
        }
        Output::Error {
            ename,
            evalue,
            traceback: tb,
        } => {
            let label = format!("Error: {} — {}", ename, evalue);
            let label = theme::colorize_error_header(&label, ctx.use_color);
            frame::open(&label, ctx, w)?;
            traceback::render(tb, ctx, w)?;
            frame::close(ctx, w)?;
        }
        Output::Unknown => {
            let label = out_label(None, "unknown output");
            header(&label, ctx, w)?;
            frame::wrap_line("(skipped)", ctx, w)?;
            frame::close(ctx, w)?;
        }
    }
    Ok(())
}

fn render_bundle(
    bundle: &MimeBundle,
    exec_count: Option<u64>,
    cell_idx: usize,
    out_idx: usize,
    ctx: &RenderCtx,
    w: &mut impl Write,
) -> io::Result<()> {
    // 우선순위: image/png → text/html (표로 파싱되면) → text/plain → 기타 placeholder
    if let Some(b64) = &bundle.image_png {
        return frame_section(&out_label(exec_count, "image/png"), ctx, w, |w| {
            image::dispatch(b64, cell_idx, out_idx, ctx, w)
        });
    }
    // text/html as a table (DataFrames): prefer it over the plain-text repr.
    if let Some(html) = &bundle.text_html {
        if let Some(parsed) = html_table::parse(html) {
            return frame_section(&out_label(exec_count, "text/html"), ctx, w, |w| {
                let mut s = sink::BoxedSink::new(w);
                table::render(&parsed, ctx, &mut s)
            });
        }
    }
    if let Some(t) = &bundle.text_plain {
        return frame_section(&out_label(exec_count, "text/plain"), ctx, w, |w| {
            text::render(t, ctx, w)
        });
    }
    // 기타 MIME: ipynb 권장 우선순위
    let priority = ["text/latex", "application/json"];
    let key = priority
        .iter()
        .find(|p| bundle.other.contains_key(**p))
        .map(|s| s.to_string())
        .or_else(|| bundle.other.keys().min().cloned());
    let mime = key.as_deref().unwrap_or("(empty)");
    frame_section(
        &out_label(exec_count, &format!("(unsupported: {mime})")),
        ctx,
        w,
        |w| frame::wrap_line("", ctx, w),
    )
}

/// Open an output frame with a green header `label`, run `body`, then close the frame.
/// Collapses the repeated open → content → close sequence used per MIME type.
fn frame_section<W: Write>(
    label: &str,
    ctx: &RenderCtx,
    w: &mut W,
    body: impl FnOnce(&mut W) -> io::Result<()>,
) -> io::Result<()> {
    let label = theme::colorize_output_header(label, ctx.use_color);
    frame::open(&label, ctx, w)?;
    body(w)?;
    frame::close(ctx, w)
}

fn header(label: &str, ctx: &RenderCtx, w: &mut impl Write) -> io::Result<()> {
    let label = theme::colorize_output_header(label, ctx.use_color);
    frame::open(&label, ctx, w)
}

/// Build an output box label. Outputs with a real execution count get
/// `Out [n] ── mime`; outputs without one (display_data, streams) get
/// `Out ── mime`.
fn out_label(exec_count: Option<u64>, mime: &str) -> String {
    match exec_count {
        Some(n) => format!("Out [{n}] ── {mime}"),
        None => format!("Out ── {mime}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::RenderCtx;
    use crate::ipynb::model::{MimeBundle, Output, StreamName};
    use std::collections::HashMap;

    fn ctx_placeholder() -> RenderCtx {
        crate::render::test_support::base()
    }

    #[test]
    fn stream_stdout_renders_text() {
        let out = Output::Stream {
            name: StreamName::Stdout,
            text: "hello\n".into(),
        };
        let mut buf = Vec::new();
        render(&out, 0, 0, &ctx_placeholder(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("stream (stdout)"), "{s}");
        assert!(s.contains("hello"));
        // stream output has no execution count → no bracketed number
        assert!(!s.contains("Out ["), "stream must not be numbered: {s}");
    }

    #[test]
    fn execute_result_picks_image_when_present() {
        let png_b64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==";
        let bundle = MimeBundle {
            text_plain: Some("ignored".into()),
            text_html: None,
            image_png: Some(png_b64.into()),
            other: HashMap::new(),
        };
        let out = Output::ExecuteResult {
            data: bundle,
            execution_count: Some(1),
        };
        let mut buf = Vec::new();
        render(&out, 0, 0, &ctx_placeholder(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("PNG"));
        assert!(!s.contains("ignored"));
    }

    #[test]
    fn execute_result_falls_back_to_text_plain() {
        let bundle = MimeBundle {
            text_plain: Some("42".into()),
            text_html: None,
            image_png: None,
            other: HashMap::new(),
        };
        let out = Output::ExecuteResult {
            data: bundle,
            execution_count: Some(1),
        };
        let mut buf = Vec::new();
        render(&out, 0, 0, &ctx_placeholder(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("42"));
    }

    #[test]
    fn execute_result_unknown_mime_shows_placeholder() {
        let mut other = HashMap::new();
        other.insert(
            "application/json".to_string(),
            serde_json::Value::String("{}".into()),
        );
        let bundle = MimeBundle {
            text_plain: None,
            text_html: None,
            image_png: None,
            other,
        };
        let out = Output::ExecuteResult {
            data: bundle,
            execution_count: Some(1),
        };
        let mut buf = Vec::new();
        render(&out, 0, 0, &ctx_placeholder(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("unsupported") || s.contains("application/json"));
    }

    #[test]
    fn error_renders_traceback() {
        let out = Output::Error {
            ename: "ValueError".into(),
            evalue: "bad".into(),
            traceback: vec!["line1".into(), "line2".into()],
        };
        let mut buf = Vec::new();
        render(&out, 0, 0, &ctx_placeholder(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("Error"));
        assert!(s.contains("ValueError") || s.contains("line1"));
    }

    #[test]
    fn execute_result_renders_html_table_over_text_plain() {
        let bundle = MimeBundle {
            text_plain: Some("PLAIN_REPR".into()),
            text_html: Some(
                "<table><thead><tr><th>c</th></tr></thead><tbody><tr><td>v</td></tr></tbody></table>"
                    .into(),
            ),
            image_png: None,
            other: HashMap::new(),
        };
        let out = Output::ExecuteResult {
            data: bundle,
            execution_count: Some(1),
        };
        let mut buf = Vec::new();
        render(&out, 0, 0, &ctx_placeholder(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains('┌'), "should render a table box: {s}");
        assert!(s.contains("v"));
        assert!(!s.contains("PLAIN_REPR"), "should prefer html table: {s}");
    }

    #[test]
    fn execute_result_unparseable_html_falls_back_to_text_plain() {
        let bundle = MimeBundle {
            text_plain: Some("PLAIN_REPR".into()),
            text_html: Some("<div>not a table</div>".into()),
            image_png: None,
            other: HashMap::new(),
        };
        let out = Output::ExecuteResult {
            data: bundle,
            execution_count: Some(1),
        };
        let mut buf = Vec::new();
        render(&out, 0, 0, &ctx_placeholder(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("PLAIN_REPR"));
    }
}
