use std::io::{self, Write};
use crate::env::RenderCtx;

pub mod png_info;
pub mod placeholder;
pub mod kitty;
// iterm/dispatch는 후속 태스크에서

pub trait ImageRenderer {
    fn render(&self, png_bytes: &[u8], cell_idx: usize, out_idx: usize, ctx: &RenderCtx, w: &mut dyn Write) -> io::Result<()>;
}
