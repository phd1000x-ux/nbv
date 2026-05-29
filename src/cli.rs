use clap::{Parser, Subcommand};
use std::num::NonZeroUsize;
use std::path::PathBuf;

/// Parse a `--cells` value of the form `N` or `N-M` (1-based, inclusive).
///
/// Returns `(start, end)` with `start <= end`. Rejects zero, reversed
/// ranges, open ranges (`-5`, `3-`), comma lists, and any non-numeric
/// input. The error message includes the offending input so clap's
/// stderr output stays self-explanatory.
pub fn parse_cells_spec(s: &str) -> Result<(NonZeroUsize, NonZeroUsize), String> {
    if s.is_empty() {
        return Err("empty --cells value".to_string());
    }
    let parse_one = |part: &str| -> Result<NonZeroUsize, String> {
        part.parse::<NonZeroUsize>()
            .map_err(|_| format!("invalid --cells value '{}' (expected N or N-M, 1-based)", s))
    };
    match s.split_once('-') {
        None => {
            let n = parse_one(s)?;
            Ok((n, n))
        }
        Some((a, b)) => {
            if a.is_empty() || b.is_empty() || b.contains('-') {
                return Err(format!(
                    "invalid --cells value '{}' (expected N or N-M, 1-based)",
                    s
                ));
            }
            let start = parse_one(a)?;
            let end = parse_one(b)?;
            if start > end {
                return Err(format!(
                    "invalid --cells value '{}' (start > end)",
                    s
                ));
            }
            Ok((start, end))
        }
    }
}

#[derive(Debug, Parser)]
#[command(
    name = "nbv",
    version,
    about = "A fast terminal-native Jupyter notebook viewer"
)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Path to the .ipynb file
    pub file: Option<PathBuf>,

    /// Disable ANSI color output
    #[arg(long)]
    pub no_color: bool,

    /// Disable inline image rendering (use placeholder)
    #[arg(long)]
    pub no_images: bool,

    /// Syntect theme for code blocks (default: base16-ocean.dark)
    #[arg(long, env = "NBV_THEME", value_name = "NAME")]
    pub theme: Option<String>,

    /// Force output width to N columns, min 20 (default: auto-detect)
    #[arg(long, env = "NBV_WIDTH", value_name = "N", value_parser = clap::value_parser!(u16).range(20..))]
    pub width: Option<u16>,

    /// Render only cell N or cells N-M (1-based, inclusive). Out-of-range silently clamped.
    #[arg(
        long,
        env = "NBV_CELLS",
        value_name = "SPEC",
        value_parser = parse_cells_spec,
    )]
    pub cells: Option<(NonZeroUsize, NonZeroUsize)>,

    /// Hide kernel outputs; render code and markdown only.
    #[arg(long, env = "NBV_NO_OUTPUT")]
    pub no_output: bool,

    /// Render only code-cell source. Implies --no-output.
    #[arg(long, env = "NBV_CODE_ONLY")]
    pub code_only: bool,

    /// Plain-text output: no box frames, prefixed sections. Implies --no-color and --no-images.
    #[arg(long, env = "NBV_PLAIN")]
    pub plain: bool,

    /// Print available syntect theme names and exit
    #[arg(long)]
    pub list_themes: bool,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Add the nbv binary directory to your shell PATH
    Setup {
        /// Skip the confirmation prompt
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// Generate a shell completion script to stdout
    Completion {
        /// Shell to generate completion for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
    /// Generate a section-1 man page to stdout
    Mangen,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parses_just_file() {
        let a = Args::try_parse_from(["nbv", "foo.ipynb"]).unwrap();
        assert_eq!(a.file.as_ref().unwrap().to_string_lossy(), "foo.ipynb");
        assert!(!a.no_color);
        assert!(!a.no_images);
        assert!(a.command.is_none());
    }

    #[test]
    fn parses_with_flags() {
        let a = Args::try_parse_from(["nbv", "x.ipynb", "--no-color", "--no-images"]).unwrap();
        assert!(a.no_color);
        assert!(a.no_images);
    }

    #[test]
    fn allows_empty_invocation_parses_with_none() {
        // No clap-level required check; main.rs decides what to do.
        let a = Args::try_parse_from(["nbv"]).unwrap();
        assert!(a.file.is_none());
        assert!(a.command.is_none());
    }

    #[test]
    fn parses_setup_subcommand() {
        let a = Args::try_parse_from(["nbv", "setup"]).unwrap();
        assert!(a.file.is_none());
        match a.command {
            Some(Command::Setup { yes }) => assert!(!yes),
            _ => panic!("expected Setup"),
        }
    }

    #[test]
    fn parses_setup_with_yes() {
        let a = Args::try_parse_from(["nbv", "setup", "--yes"]).unwrap();
        match a.command {
            Some(Command::Setup { yes }) => assert!(yes),
            _ => panic!("expected Setup"),
        }
    }

    #[test]
    fn parses_theme_flag() {
        let a = Args::try_parse_from(["nbv", "x.ipynb", "--theme", "InspiredGitHub"]).unwrap();
        assert_eq!(a.theme.as_deref(), Some("InspiredGitHub"));
    }

    #[test]
    fn parses_width_flag() {
        let a = Args::try_parse_from(["nbv", "x.ipynb", "--width", "120"]).unwrap();
        assert_eq!(a.width, Some(120));
    }

    #[test]
    fn theme_and_width_default_to_none() {
        let a = Args::try_parse_from(["nbv", "x.ipynb"]).unwrap();
        assert!(a.theme.is_none());
        assert!(a.width.is_none());
    }

    #[test]
    fn width_below_minimum_is_rejected() {
        // clap range validator rejects --width 5
        let r = Args::try_parse_from(["nbv", "x.ipynb", "--width", "5"]);
        assert!(r.is_err());
        let msg = r.unwrap_err().to_string();
        assert!(
            msg.contains("5") && (msg.contains("not in") || msg.contains("range")),
            "got: {msg}"
        );
    }

    #[test]
    fn parses_completion_bash() {
        let a = Args::try_parse_from(["nbv", "completion", "bash"]).unwrap();
        match a.command {
            Some(Command::Completion { shell }) => {
                assert_eq!(shell, clap_complete::Shell::Bash);
            }
            _ => panic!("expected Completion {{ shell: Bash }}"),
        }
    }

    #[test]
    fn parses_mangen() {
        let a = Args::try_parse_from(["nbv", "mangen"]).unwrap();
        assert!(matches!(a.command, Some(Command::Mangen)));
    }

    #[test]
    fn rejects_unknown_shell() {
        let r = Args::try_parse_from(["nbv", "completion", "fakeshell"]);
        assert!(r.is_err());
        let msg = r.unwrap_err().to_string();
        assert!(
            msg.contains("fakeshell") || msg.contains("possible values"),
            "error should mention the invalid value or possible values; got: {}",
            msg
        );
    }

    use std::num::NonZeroUsize;

    fn nz(n: usize) -> NonZeroUsize {
        NonZeroUsize::new(n).unwrap()
    }

    #[test]
    fn parse_cells_spec_single() {
        assert_eq!(super::parse_cells_spec("5").unwrap(), (nz(5), nz(5)));
        assert_eq!(super::parse_cells_spec("1").unwrap(), (nz(1), nz(1)));
    }

    #[test]
    fn parse_cells_spec_range() {
        assert_eq!(super::parse_cells_spec("3-7").unwrap(), (nz(3), nz(7)));
        assert_eq!(super::parse_cells_spec("1-1").unwrap(), (nz(1), nz(1)));
    }

    #[test]
    fn parse_cells_spec_rejects_zero() {
        assert!(super::parse_cells_spec("0").is_err());
        assert!(super::parse_cells_spec("0-5").is_err());
        assert!(super::parse_cells_spec("5-0").is_err());
    }

    #[test]
    fn parse_cells_spec_rejects_reverse() {
        let e = super::parse_cells_spec("7-3").unwrap_err();
        assert!(e.contains("7-3") || e.contains("reverse") || e.contains("start"));
    }

    #[test]
    fn parse_cells_spec_rejects_open_and_empty() {
        assert!(super::parse_cells_spec("").is_err());
        assert!(super::parse_cells_spec("-5").is_err());
        assert!(super::parse_cells_spec("3-").is_err());
    }

    #[test]
    fn parse_cells_spec_rejects_non_numeric() {
        assert!(super::parse_cells_spec("abc").is_err());
        assert!(super::parse_cells_spec("3-7-9").is_err());
        assert!(super::parse_cells_spec("3,5").is_err());
    }

    #[test]
    fn parses_cells_flag_single() {
        let a = Args::try_parse_from(["nbv", "x.ipynb", "--cells", "5"]).unwrap();
        assert_eq!(a.cells.unwrap(), (nz(5), nz(5)));
    }

    #[test]
    fn parses_cells_flag_range() {
        let a = Args::try_parse_from(["nbv", "x.ipynb", "--cells", "3-7"]).unwrap();
        assert_eq!(a.cells.unwrap(), (nz(3), nz(7)));
    }

    #[test]
    fn cells_flag_default_none() {
        let a = Args::try_parse_from(["nbv", "x.ipynb"]).unwrap();
        assert!(a.cells.is_none());
    }

    #[test]
    fn cells_flag_rejects_reverse_at_cli() {
        let r = Args::try_parse_from(["nbv", "x.ipynb", "--cells", "7-3"]);
        assert!(r.is_err());
        let msg = r.unwrap_err().to_string();
        assert!(msg.contains("7-3"), "stderr should mention bad input: {msg}");
    }

    #[test]
    fn parses_no_output_flag() {
        let a = Args::try_parse_from(["nbv", "x.ipynb", "--no-output"]).unwrap();
        assert!(a.no_output);
        let b = Args::try_parse_from(["nbv", "x.ipynb"]).unwrap();
        assert!(!b.no_output);
    }

    #[test]
    fn parses_code_only_flag() {
        let a = Args::try_parse_from(["nbv", "x.ipynb", "--code-only"]).unwrap();
        assert!(a.code_only);
    }

    #[test]
    fn parses_plain_flag() {
        let a = Args::try_parse_from(["nbv", "x.ipynb", "--plain"]).unwrap();
        assert!(a.plain);
    }
}
