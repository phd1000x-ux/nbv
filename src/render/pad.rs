//! Single source of truth for emitting runs of ASCII spaces.
//!
//! Two sinks need this: the framed (`io::Write`) path and the table
//! (`String`-building) path. Both slice from the same const to avoid an
//! intermediate `" ".repeat(n)` allocation.

use std::io::{self, Write};

/// 32 ASCII spaces — the chunk both helpers slice from.
pub(crate) const SPACES: &str = "                                ";

/// Write `n` ASCII spaces to `w` without allocating a `String`.
pub(crate) fn write_spaces(w: &mut (impl Write + ?Sized), n: usize) -> io::Result<()> {
    let bytes = SPACES.as_bytes();
    let mut remaining = n;
    while remaining >= bytes.len() {
        w.write_all(bytes)?;
        remaining -= bytes.len();
    }
    if remaining > 0 {
        w.write_all(&bytes[..remaining])?;
    }
    Ok(())
}

/// Push `n` ASCII spaces onto `out` without an intermediate `" ".repeat(n)` allocation.
pub(crate) fn push_spaces(out: &mut String, n: usize) {
    let mut remaining = n;
    while remaining >= SPACES.len() {
        out.push_str(SPACES);
        remaining -= SPACES.len();
    }
    if remaining > 0 {
        out.push_str(&SPACES[..remaining]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_spaces_emits_exact_count_for_boundary_values() {
        for n in [0usize, 1, 31, 32, 33, 100] {
            let mut buf = Vec::new();
            write_spaces(&mut buf, n).unwrap();
            assert_eq!(
                buf.len(),
                n,
                "write_spaces({}) wrote {} bytes",
                n,
                buf.len()
            );
            assert!(
                buf.iter().all(|&b| b == b' '),
                "write_spaces({}) emitted non-space bytes: {:?}",
                n,
                buf
            );
        }
    }

    #[test]
    fn push_spaces_emits_exact_count_for_boundary_values() {
        for n in [0usize, 1, 31, 32, 33, 100] {
            let mut s = String::new();
            push_spaces(&mut s, n);
            assert_eq!(s.len(), n);
            assert!(s.chars().all(|c| c == ' '));
        }
    }
}
