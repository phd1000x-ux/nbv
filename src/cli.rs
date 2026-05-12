use std::path::PathBuf;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "nbv", version, about = "A fast terminal Jupyter notebook viewer")]
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
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Add the nbv binary directory to your shell PATH
    Setup {
        /// Skip the confirmation prompt
        #[arg(long, short = 'y')]
        yes: bool,
    },
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
}
