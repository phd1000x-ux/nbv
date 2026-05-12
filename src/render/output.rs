use std::io::{self, Write};
use base64::Engine;

use crate::env::RenderCtx;
use crate::ipynb::model::{MimeBundle, Output, StreamName};
use crate::render::{frame, image, text, traceback};
use crate::theme;

/// 단일 Output을 박스 안에서 렌더.
pub fn render(out: &Output, cell_idx: usize, out_idx: usize, ctx: &RenderCtx, w: &mut impl Write) -> io::Result<()> {
    match out {
        Output::Stream { name, text: t } => {
            let label = format!("Out [{}] ── stream ({})", cell_idx, stream_label(name));
            header(&label, ctx, w)?;
            text::render(t, ctx, w)?;
            frame::close(ctx, w)?;
        }
        Output::ExecuteResult { data, execution_count } => {
            render_bundle(data, *execution_count, cell_idx, out_idx, ctx, w)?;
        }
        Output::DisplayData { data } => {
            render_bundle(data, None, cell_idx, out_idx, ctx, w)?;
        }
        Output::Error { ename, evalue, traceback: tb } => {
            let label = format!("Error: {} — {}", ename, evalue);
            let label = theme::colorize_error_header(&label, ctx.use_color);
            frame::open(&label, ctx, w)?;
            traceback::render(tb, ctx, w)?;
            frame::close(ctx, w)?;
        }
        Output::Unknown => {
            let label = format!("Out [{}] ── unknown output", cell_idx);
            header(&label, ctx, w)?;
            frame::wrap_line("(skipped)", ctx, w)?;
            frame::close(ctx, w)?;
        }
    }
    Ok(())
}

fn render_bundle(bundle: &MimeBundle, exec_count: Option<u64>, cell_idx: usize, out_idx: usize, ctx: &RenderCtx, w: &mut impl Write) -> io::Result<()> {
    // 우선순위: image/png (백엔드 가능 시) → image/png (placeholder) → text/plain → 기타 placeholder
    if let Some(b64) = &bundle.image_png {
        let mime = "image/png";
        let label = match exec_count {
            Some(n) => format!("Out [{}] ── {}", n, mime),
            None => format!("Out ── {}", mime),
        };
        let label = theme::colorize_output_header(&label, ctx.use_color);
        frame::open(&label, ctx, w)?;
        match base64::engine::general_purpose::STANDARD.decode(b64) {
            Ok(bytes) => image::dispatch(&bytes, cell_idx, out_idx, ctx, w)?,
            Err(_) => frame::wrap_line("(image decode failed)", ctx, w)?,
        }
        frame::close(ctx, w)?;
        return Ok(());
    }
    if let Some(t) = &bundle.text_plain {
        let label = match exec_count {
            Some(n) => format!("Out [{}] ── text/plain", n),
            None => "Out ── text/plain".to_string(),
        };
        let label = theme::colorize_output_header(&label, ctx.use_color);
        frame::open(&label, ctx, w)?;
        text::render(t, ctx, w)?;
        frame::close(ctx, w)?;
        return Ok(());
    }
    // 기타 MIME: ipynb 권장 우선순위
    let priority = ["text/html", "text/latex", "application/json"];
    let key = priority.iter()
        .find(|p| bundle.other.contains_key(**p))
        .map(|s| s.to_string())
        .or_else(|| bundle.other.keys().min().cloned());
    let mime = key.as_deref().unwrap_or("(empty)");
    let label = format!("Out [{}] ── (unsupported: {})", cell_idx, mime);
    let label = theme::colorize_output_header(&label, ctx.use_color);
    frame::open(&label, ctx, w)?;
    frame::wrap_line("", ctx, w)?;
    frame::close(ctx, w)?;
    Ok(())
}

fn header(label: &str, ctx: &RenderCtx, w: &mut impl Write) -> io::Result<()> {
    let label = theme::colorize_output_header(label, ctx.use_color);
    frame::open(&label, ctx, w)
}

fn stream_label(name: &StreamName) -> &'static str {
    match name { StreamName::Stdout => "stdout", StreamName::Stderr => "stderr" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::{ImageBackend, RenderCtx};
    use crate::ipynb::model::{MimeBundle, Output, StreamName};
    use std::collections::HashMap;

    fn ctx_placeholder() -> RenderCtx {
        RenderCtx { is_tty: true, use_color: false, width: 60, image_backend: ImageBackend::Placeholder }
    }

    #[test]
    fn stream_stdout_renders_text() {
        let out = Output::Stream { name: StreamName::Stdout, text: "hello\n".into() };
        let mut buf = Vec::new();
        render(&out, 0, 0, &ctx_placeholder(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("Out [0]") || s.contains("hello"));
        assert!(s.contains("hello"));
    }

    #[test]
    fn execute_result_picks_image_when_present() {
        let png_b64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==";
        let bundle = MimeBundle {
            text_plain: Some("ignored".into()),
            image_png: Some(png_b64.into()),
            other: HashMap::new(),
        };
        let out = Output::ExecuteResult { data: bundle, execution_count: Some(1) };
        let mut buf = Vec::new();
        render(&out, 0, 0, &ctx_placeholder(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("PNG"));
        assert!(!s.contains("ignored"));
    }

    #[test]
    fn execute_result_falls_back_to_text_plain() {
        let bundle = MimeBundle { text_plain: Some("42".into()), image_png: None, other: HashMap::new() };
        let out = Output::ExecuteResult { data: bundle, execution_count: Some(1) };
        let mut buf = Vec::new();
        render(&out, 0, 0, &ctx_placeholder(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("42"));
    }

    #[test]
    fn execute_result_unknown_mime_shows_placeholder() {
        let mut other = HashMap::new();
        other.insert("text/html".to_string(), serde_json::Value::String("<table/>".into()));
        let bundle = MimeBundle { text_plain: None, image_png: None, other };
        let out = Output::ExecuteResult { data: bundle, execution_count: Some(1) };
        let mut buf = Vec::new();
        render(&out, 0, 0, &ctx_placeholder(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("unsupported") || s.contains("text/html"));
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
}
