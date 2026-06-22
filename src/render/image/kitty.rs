use std::io::{self, Write};

use crate::env::RenderCtx;
use crate::render::image::ImageRenderer;

pub struct KittyRenderer;

const CHUNK_SIZE: usize = 4096;

impl ImageRenderer for KittyRenderer {
    fn render(
        &self,
        b64: &str,
        _cell_idx: usize,
        _out_idx: usize,
        _ctx: &RenderCtx,
        w: &mut dyn Write,
    ) -> io::Result<()> {
        let chunks: Vec<&[u8]> = b64.as_bytes().chunks(CHUNK_SIZE).collect();
        for (i, chunk) in chunks.iter().enumerate() {
            let is_last = i == chunks.len() - 1;
            let m = if is_last { 0 } else { 1 };
            if i == 0 {
                write!(w, "\x1b_Gf=100,a=T,m={};", m)?;
            } else {
                write!(w, "\x1b_Gm={};", m)?;
            }
            w.write_all(chunk)?;
            write!(w, "\x1b\\")?;
        }
        writeln!(w)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::{ImageBackend, RenderCtx};
    use crate::render::image::ImageRenderer;

    fn ctx() -> RenderCtx {
        crate::render::test_support::backend(ImageBackend::Kitty)
    }

    #[test]
    fn emits_kitty_apc_with_png_data() {
        let b64 = "iVBORw0KGgoAAAANS"; // < 4096 → 단일 chunk
        let mut buf = Vec::new();
        KittyRenderer.render(b64, 0, 0, &ctx(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.starts_with("\x1b_G"));
        assert!(s.contains("f=100"));
        assert!(s.contains("a=T"));
        assert!(s.contains(b64)); // pass-through: 입력 b64가 그대로 등장
        assert!(s.ends_with("\x1b\\\n"));
    }

    #[test]
    fn chunks_large_payloads() {
        // 4096 base64 chars 넘는 분량 → 여러 APC 시퀀스
        let big = "A".repeat(5000);
        let mut buf = Vec::new();
        KittyRenderer.render(&big, 0, 0, &ctx(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.matches("\x1b\\").count() >= 2);
        assert!(s.contains("m=1"));
        assert!(s.contains("m=0"));
    }

    #[test]
    fn empty_b64_does_not_panic() {
        let mut buf = Vec::new();
        KittyRenderer.render("", 0, 0, &ctx(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        // 빈 입력: APC 시퀀스 없이 trailing newline만
        assert!(!s.contains("\x1b_G"));
    }
}
