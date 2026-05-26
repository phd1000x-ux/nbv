use clap::{Parser, Subcommand};
use std::path::PathBuf;

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
}
