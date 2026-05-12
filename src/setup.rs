use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Eq)]
pub enum ShellKind {
    Zsh,
    Bash,
    Fish,
    Unknown(String),
}

impl ShellKind {
    pub fn from_shell_path(shell: &str) -> Self {
        let name = shell.rsplit('/').next().unwrap_or("");
        match name {
            "zsh" => ShellKind::Zsh,
            "bash" => ShellKind::Bash,
            "fish" => ShellKind::Fish,
            other => ShellKind::Unknown(other.to_string()),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            ShellKind::Zsh => "zsh",
            ShellKind::Bash => "bash",
            ShellKind::Fish => "fish",
            ShellKind::Unknown(s) => s,
        }
    }

    pub fn rc_path(&self, home: &Path) -> Option<PathBuf> {
        match self {
            ShellKind::Zsh => Some(home.join(".zshrc")),
            ShellKind::Bash => Some(home.join(".bashrc")),
            ShellKind::Fish => Some(home.join(".config/fish/config.fish")),
            ShellKind::Unknown(_) => None,
        }
    }

    pub fn path_line(&self, dir: &Path) -> String {
        let d = dir.display();
        match self {
            ShellKind::Fish => format!("fish_add_path {}", d),
            _ => format!("export PATH=\"{}:$PATH\"", d),
        }
    }
}

pub fn binary_dir() -> Option<PathBuf> {
    env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
}

pub fn path_already_includes(path_env: &str, dir: &Path) -> bool {
    let dir_str = dir.to_string_lossy();
    path_env.split(':').any(|p| p == dir_str)
}

pub fn rc_already_references(rc_content: &str, dir: &Path) -> bool {
    rc_content.contains(&*dir.to_string_lossy())
}

fn append_block(path: &Path, line: &str) -> io::Result<()> {
    let needs_leading_newline = match fs::read_to_string(path) {
        Ok(c) => !c.is_empty() && !c.ends_with('\n'),
        Err(_) => false,
    };
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    if needs_leading_newline {
        writeln!(f)?;
    }
    writeln!(f, "# Added by `nbv setup`")?;
    writeln!(f, "{}", line)?;
    Ok(())
}

pub fn run(yes: bool) -> i32 {
    let bin_dir = match binary_dir() {
        Some(p) => p,
        None => {
            eprintln!("nbv setup: could not determine the nbv binary directory");
            return 1;
        }
    };

    let path_env = env::var("PATH").unwrap_or_default();
    if path_already_includes(&path_env, &bin_dir) {
        println!("'{}' is already in PATH. Nothing to do.", bin_dir.display());
        return 0;
    }

    let shell = env::var("SHELL").unwrap_or_default();
    let kind = ShellKind::from_shell_path(&shell);
    let home = match env::var("HOME") {
        Ok(h) => PathBuf::from(h),
        Err(_) => {
            eprintln!("nbv setup: HOME environment variable is not set");
            return 1;
        }
    };
    let rc = match kind.rc_path(&home) {
        Some(p) => p,
        None => {
            eprintln!(
                "nbv setup: unsupported shell '{}'. Add this line to your shell config manually:",
                shell
            );
            eprintln!("    {}", ShellKind::Bash.path_line(&bin_dir));
            return 1;
        }
    };

    let rc_content = fs::read_to_string(&rc).unwrap_or_default();
    if rc_already_references(&rc_content, &bin_dir) {
        println!(
            "'{}' already references '{}'.",
            rc.display(),
            bin_dir.display()
        );
        println!("If nbv is still not found, open a new terminal or run:");
        println!("    source {}", rc.display());
        return 0;
    }

    let line = kind.path_line(&bin_dir);

    println!("nbv binary directory: {}", bin_dir.display());
    println!("detected shell:       {}", kind.name());
    println!("config file:          {}", rc.display());
    println!();
    println!("The following will be appended:");
    println!();
    println!("    # Added by `nbv setup`");
    println!("    {}", line);
    println!();

    if !yes && !confirm() {
        println!("Aborted.");
        return 0;
    }

    if let Err(e) = append_block(&rc, &line) {
        eprintln!("nbv setup: failed to write '{}': {}", rc.display(), e);
        return 1;
    }

    println!();
    println!("Done. Open a new terminal or run:");
    println!("    source {}", rc.display());
    0
}

fn confirm() -> bool {
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
    fn detects_zsh() {
        assert_eq!(ShellKind::from_shell_path("/bin/zsh"), ShellKind::Zsh);
        assert_eq!(
            ShellKind::from_shell_path("/usr/local/bin/zsh"),
            ShellKind::Zsh
        );
    }

    #[test]
    fn detects_bash() {
        assert_eq!(ShellKind::from_shell_path("/bin/bash"), ShellKind::Bash);
    }

    #[test]
    fn detects_fish() {
        assert_eq!(
            ShellKind::from_shell_path("/opt/homebrew/bin/fish"),
            ShellKind::Fish
        );
    }

    #[test]
    fn detects_unknown_shell() {
        match ShellKind::from_shell_path("/bin/csh") {
            ShellKind::Unknown(s) => assert_eq!(s, "csh"),
            _ => panic!("expected Unknown"),
        }
        match ShellKind::from_shell_path("") {
            ShellKind::Unknown(s) => assert_eq!(s, ""),
            _ => panic!("expected Unknown for empty"),
        }
    }

    #[test]
    fn rc_paths_match_convention() {
        let h = Path::new("/home/u");
        assert_eq!(
            ShellKind::Zsh.rc_path(h),
            Some(PathBuf::from("/home/u/.zshrc"))
        );
        assert_eq!(
            ShellKind::Bash.rc_path(h),
            Some(PathBuf::from("/home/u/.bashrc"))
        );
        assert_eq!(
            ShellKind::Fish.rc_path(h),
            Some(PathBuf::from("/home/u/.config/fish/config.fish"))
        );
        assert_eq!(ShellKind::Unknown("csh".into()).rc_path(h), None);
    }

    #[test]
    fn path_line_formats() {
        let dir = Path::new("/u/.cargo/bin");
        assert_eq!(
            ShellKind::Zsh.path_line(dir),
            r#"export PATH="/u/.cargo/bin:$PATH""#
        );
        assert_eq!(
            ShellKind::Bash.path_line(dir),
            r#"export PATH="/u/.cargo/bin:$PATH""#
        );
        assert_eq!(
            ShellKind::Fish.path_line(dir),
            "fish_add_path /u/.cargo/bin"
        );
    }

    #[test]
    fn path_already_includes_exact_segment() {
        assert!(path_already_includes(
            "/foo:/bar:/baz",
            Path::new("/bar")
        ));
        assert!(path_already_includes("/bar", Path::new("/bar")));
        assert!(!path_already_includes(
            "/foo:/barz:/baz",
            Path::new("/bar")
        ));
        assert!(!path_already_includes("", Path::new("/bar")));
    }

    #[test]
    fn rc_already_references_matches_substring() {
        assert!(rc_already_references(
            "export PATH=\"/home/u/.cargo/bin:$PATH\"\n",
            Path::new("/home/u/.cargo/bin")
        ));
        assert!(!rc_already_references(
            "export PATH=\"/usr/local/bin:$PATH\"\n",
            Path::new("/home/u/.cargo/bin")
        ));
    }

    fn unique_tmp(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!(
            "nbv_setup_test_{}_{}_{}",
            std::process::id(),
            nanos,
            name
        ))
    }

    #[test]
    fn append_block_creates_file_when_missing() {
        let tmp = unique_tmp("create");
        let _ = fs::remove_file(&tmp);
        append_block(&tmp, "export PATH=\"/x:$PATH\"").unwrap();
        let content = fs::read_to_string(&tmp).unwrap();
        assert!(content.contains("# Added by `nbv setup`"));
        assert!(content.contains("export PATH=\"/x:$PATH\""));
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn append_block_appends_leading_newline_when_needed() {
        let tmp = unique_tmp("noeol");
        fs::write(&tmp, "alias l=ls").unwrap();
        append_block(&tmp, "export PATH=\"/x:$PATH\"").unwrap();
        let content = fs::read_to_string(&tmp).unwrap();
        assert!(content.starts_with("alias l=ls\n"));
        assert!(content.contains("# Added by `nbv setup`"));
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn append_block_preserves_existing_when_ends_with_newline() {
        let tmp = unique_tmp("eol");
        fs::write(&tmp, "alias l=ls\n").unwrap();
        append_block(&tmp, "export PATH=\"/x:$PATH\"").unwrap();
        let content = fs::read_to_string(&tmp).unwrap();
        assert_eq!(
            content,
            "alias l=ls\n# Added by `nbv setup`\nexport PATH=\"/x:$PATH\"\n"
        );
        let _ = fs::remove_file(&tmp);
    }
}
