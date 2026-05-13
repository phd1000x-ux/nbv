use base64::Engine;
use std::io::{self, Write};

use crate::env::RenderCtx;
use crate::render::image::ImageRenderer;

pub struct KittyRenderer;

const CHUNK_SIZE: usize = 4096;

impl ImageRenderer for KittyRenderer {
    fn render(
        &self,
        png_bytes: &[u8],
        _cell_idx: usize,
        _out_idx: usize,
        _ctx: &RenderCtx,
        w: &mut dyn Write,
    ) -> io::Result<()> {
        let b64 = base64::engine::general_purpose::STANDARD.encode(png_bytes);
        let chunks: Vec<&str> = b64
            .as_bytes()
            .chunks(CHUNK_SIZE)
            .map(|c| std::str::from_utf8(c).unwrap())
            .collect();
        for (i, chunk) in chunks.iter().enumerate() {
            let is_last = i == chunks.len() - 1;
            let m = if is_last { 0 } else { 1 };
            if i == 0 {
                write!(w, "\x1b_Gf=100,a=T,m={};{}\x1b\\", m, chunk)?;
            } else {
                write!(w, "\x1b_Gm={};{}\x1b\\", m, chunk)?;
            }
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
        RenderCtx {
            is_tty: true,
            use_color: false,
            width: 60,
            image_backend: ImageBackend::Kitty,
        }
    }

    #[test]
    fn emits_kitty_apc_with_png_data() {
        let png = b"\x89PNG\r\n\x1a\nfake-data".to_vec();
        let mut buf = Vec::new();
        KittyRenderer.render(&png, 0, 0, &ctx(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.starts_with("\x1b_G"));
        assert!(s.contains("f=100"));
        assert!(s.contains("a=T"));
        assert!(s.ends_with("\x1b\\\n"));
    }

    #[test]
    fn chunks_large_payloads() {
        // 4096 base64 chars 넘는 분량 → 여러 APC 시퀀스
        let big = vec![0u8; 5000]; // base64 약 6668자
        let mut buf = Vec::new();
        KittyRenderer.render(&big, 0, 0, &ctx(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        // APC 시퀀스 종료자가 두 번 이상 등장해야 함
        assert!(s.matches("\x1b\\").count() >= 2);
        assert!(s.contains("m=1"));
        assert!(s.contains("m=0"));
    }
}
