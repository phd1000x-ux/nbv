//! `nbv setup` — add the nbv binary directory to the user's PATH.
//!
//! Platform-specific behavior lives in [`unix`] (shell rc files) and
//! [`windows`] (user environment via PowerShell). This module holds the
//! cross-platform entry point and shared helpers.

use std::env;
use std::path::{Path, PathBuf};

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

/// Entry point for the `setup` subcommand. Returns a process exit code.
pub fn run(yes: bool) -> i32 {
    #[cfg(unix)]
    {
        unix::run(yes)
    }
    #[cfg(windows)]
    {
        windows::run(yes)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = yes;
        eprintln!("nbv setup: unsupported platform");
        1
    }
}

/// Directory containing the running `nbv` executable.
pub fn binary_dir() -> Option<PathBuf> {
    env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
}

/// True if `dir` is already a `PATH` entry. Uses [`std::env::split_paths`] so
/// the list separator matches the platform (`:` on unix, `;` on Windows).
pub fn path_already_includes(path_env: &str, dir: &Path) -> bool {
    env::split_paths(path_env).any(|p| p == dir)
}

/// Interactive `[y/N]` confirmation on stdin.
fn confirm() -> bool {
    use std::io::{self, BufRead, Write};
    print!("Apply? [y/N] ");
    if io::stdout().flush().is_err() {
        return false;
    }
    let stdin = io::stdin();
    let mut line = String::new();
    if stdin.lock().read_line(&mut line).is_err() {
        return false;
    }
    let t = line.trim().to_lowercase();
    t == "y" || t == "yes"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_already_includes_detects_exact_entry() {
        // Build PATH with the platform separator so this is meaningful on both
        // unix (':') and Windows (';').
        let sep = if cfg!(windows) { ';' } else { ':' };
        let dir = if cfg!(windows) { r"C:\bar" } else { "/bar" };
        let foo = if cfg!(windows) { r"C:\foo" } else { "/foo" };
        let barz = if cfg!(windows) { r"C:\barz" } else { "/barz" };
        let path_env = format!("{foo}{sep}{dir}{sep}{barz}");
        assert!(path_already_includes(&path_env, Path::new(dir)));
        // substring-but-not-component must NOT match
        assert!(!path_already_includes(barz, Path::new(dir)));
        assert!(!path_already_includes("", Path::new(dir)));
    }
}
