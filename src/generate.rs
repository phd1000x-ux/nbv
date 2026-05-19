use std::io::{self, Write};

use clap::CommandFactory;
use clap_complete::Shell;

use crate::cli::Args;

pub fn completion(shell: Shell, w: &mut dyn Write) -> io::Result<()> {
    let mut cmd = Args::command();
    clap_complete::generate(shell, &mut cmd, "nbv", w);
    Ok(())
}

pub fn mangen(w: &mut dyn Write) -> io::Result<()> {
    let cmd = Args::command();
    clap_mangen::Man::new(cmd).render(w)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_bash_emits_complete_function() {
        let mut buf = Vec::new();
        completion(Shell::Bash, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(!s.is_empty(), "bash completion output must not be empty");
        assert!(
            s.contains("_nbv") || s.contains("complete -F"),
            "bash completion should define `_nbv` or call `complete -F`; got first 200 chars:\n{}",
            &s[..s.len().min(200)]
        );
    }

    #[test]
    fn completion_zsh_emits_compdef_header() {
        let mut buf = Vec::new();
        completion(Shell::Zsh, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(
            s.contains("#compdef nbv") || s.contains("_nbv"),
            "zsh completion should contain `#compdef nbv` or `_nbv`; got first 200 chars:\n{}",
            &s[..s.len().min(200)]
        );
    }

    #[test]
    fn completion_fish_emits_complete_lines() {
        let mut buf = Vec::new();
        completion(Shell::Fish, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(
            s.contains("complete -c nbv"),
            "fish completion should contain `complete -c nbv`; got first 200 chars:\n{}",
            &s[..s.len().min(200)]
        );
    }

    #[test]
    fn completion_powershell_emits_register_argument_completer() {
        let mut buf = Vec::new();
        completion(Shell::PowerShell, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(
            s.contains("Register-ArgumentCompleter"),
            "powershell completion should contain `Register-ArgumentCompleter`; got first 200 chars:\n{}",
            &s[..s.len().min(200)]
        );
    }

    #[test]
    fn completion_elvish_emits_completion_arg_completer() {
        let mut buf = Vec::new();
        completion(Shell::Elvish, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(
            s.contains("edit:completion:arg-completer") || s.contains("set edit:completion"),
            "elvish completion should reference `edit:completion:arg-completer`; got first 200 chars:\n{}",
            &s[..s.len().min(200)]
        );
    }

    #[test]
    fn mangen_emits_section_1_header_for_nbv() {
        let mut buf = Vec::new();
        mangen(&mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(
            s.contains(".TH"),
            "output should contain a groff .TH directive; got first 200 chars:\n{}",
            &s[..s.len().min(200)]
        );
        let th_line = s.lines().find(|l| l.contains(".TH")).unwrap_or("");
        assert!(
            th_line.contains("\"1\"") || th_line.contains(" 1 "),
            ".TH line should reference section 1; got: {}",
            th_line
        );
        assert!(
            s.contains("nbv"),
            "man page should reference program name `nbv`; got first 200 chars:\n{}",
            &s[..s.len().min(200)]
        );
    }
}
