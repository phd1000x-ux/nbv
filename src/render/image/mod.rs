use std::io::{self, Write};
use crate::env::RenderCtx;

pub mod png_info;
pub mod placeholder;
// kitty/iterm/dispatch는 후속 태스크에서 추가

/// 모든 이미지 백엔드는 PNG bytes를 받아 stdout으로 출력한다.
pub trait ImageRenderer {
    fn render(&self, png_bytes: &[u8], cell_idx: usize, out_idx: usize, ctx: &RenderCtx, w: &mut dyn Write) -> io::Result<()>;
}
