use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_nbv");

fn run(args: &[&str]) -> (Vec<u8>, String, i32) {
    let out = Command::new(BIN)
        .args(args)
        .env_remove("NBV_THEME")
        .env_remove("NBV_WIDTH")
        .output()
        .expect("run nbv");
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let code = out.status.code().unwrap_or(-1);
    (out.stdout, stderr, code)
}

#[test]
fn completion_bash_succeeds_with_expected_markers() {
    let (out, _err, code) = run(&["completion", "bash"]);
    assert_eq!(code, 0);
    let s = String::from_utf8(out).expect("utf-8");
    assert!(
        s.contains("_nbv") || s.contains("complete -F"),
        "bash completion missing expected markers; got first 200 chars:\n{}",
        &s[..s.len().min(200)]
    );
}

#[test]
fn completion_zsh_succeeds() {
    let (out, _err, code) = run(&["completion", "zsh"]);
    assert_eq!(code, 0);
    assert!(!out.is_empty());
}

#[test]
fn completion_fish_succeeds() {
    let (out, _err, code) = run(&["completion", "fish"]);
    assert_eq!(code, 0);
    assert!(!out.is_empty());
}

#[test]
fn mangen_succeeds_and_starts_with_groff_header() {
    let (out, _err, code) = run(&["mangen"]);
    assert_eq!(code, 0);
    let s = String::from_utf8(out).expect("utf-8");
    assert!(
        s.contains(".TH nbv"),
        "mangen output should contain groff .TH header; got first 200 chars:\n{}",
        &s[..s.len().min(200)]
    );
}

#[test]
fn unknown_shell_exits_with_clap_error_and_code_2() {
    let (_out, err, code) = run(&["completion", "fakeshell"]);
    assert_eq!(code, 2);
    assert!(
        err.contains("fakeshell") || err.contains("possible values"),
        "stderr should mention the invalid value or list possible values; got:\n{}",
        err
    );
}
