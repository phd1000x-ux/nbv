use std::path::PathBuf;
use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "nbv", version, about = "A fast terminal Jupyter notebook viewer")]
pub struct Args {
    /// Path to the .ipynb file
    pub file: PathBuf,

    /// Disable ANSI color output
    #[arg(long)]
    pub no_color: bool,

    /// Disable inline image rendering (use placeholder)
    #[arg(long)]
    pub no_images: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parses_just_file() {
        let a = Args::try_parse_from(["nbv", "foo.ipynb"]).unwrap();
        assert_eq!(a.file.to_string_lossy(), "foo.ipynb");
        assert!(!a.no_color);
        assert!(!a.no_images);
    }

    #[test]
    fn parses_with_flags() {
        let a = Args::try_parse_from(["nbv", "x.ipynb", "--no-color", "--no-images"]).unwrap();
        assert!(a.no_color);
        assert!(a.no_images);
    }

    #[test]
    fn requires_file() {
        assert!(Args::try_parse_from(["nbv"]).is_err());
    }
}
