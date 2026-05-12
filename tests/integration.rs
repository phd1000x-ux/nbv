use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_nbv");

fn run(args: &[&str]) -> (String, String, i32) {
    let out = Command::new(BIN).args(args).output().expect("run nbv");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let code = out.status.code().unwrap_or(-1);
    (stdout, stderr, code)
}

#[test]
fn simple_notebook_renders_markdown_and_code() {
    let (out, _err, code) = run(&["--no-color", "--no-images", "tests/fixtures/simple.ipynb"]);
    assert_eq!(code, 0);
    assert!(out.contains("Simple Notebook"));
    assert!(out.contains("x = 1 + 2"));
    assert!(out.contains("3"));
}

#[test]
fn with_image_uses_placeholder_when_no_images() {
    let (out, _err, code) = run(&["--no-color", "--no-images", "tests/fixtures/with_image.ipynb"]);
    assert_eq!(code, 0);
    assert!(out.contains("PNG"));
    assert!(out.contains("1\u{00d7}1") || out.contains("1x1"));
}

#[test]
fn with_error_renders_traceback() {
    let (out, _err, code) = run(&["--no-color", "tests/fixtures/with_error.ipynb"]);
    assert_eq!(code, 0);
    assert!(out.contains("ZeroDivisionError") || out.contains("division by zero"));
}

#[test]
fn missing_file_exits_with_code_1() {
    let (_out, err, code) = run(&["/nonexistent/path.ipynb"]);
    assert_eq!(code, 1);
    assert!(err.contains("nbv:"));
}

#[test]
fn invalid_json_exits_with_code_3() {
    let tmp = std::env::temp_dir().join("nbv_invalid.ipynb");
    std::fs::write(&tmp, "not-json").unwrap();
    let (_out, err, code) = run(&[tmp.to_str().unwrap()]);
    assert_eq!(code, 3);
    assert!(err.contains("failed to parse"));
    std::fs::remove_file(&tmp).ok();
}

#[test]
fn missing_arg_exits_with_code_2() {
    let (_out, _err, code) = run(&[]);
    assert_eq!(code, 2);
}

#[test]
fn setup_help_lists_subcommand() {
    let (out, _err, code) = run(&["--help"]);
    assert_eq!(code, 0);
    assert!(out.contains("setup"), "top-level help should mention setup: {}", out);
}

#[test]
fn setup_subcommand_help_works() {
    let (out, _err, code) = run(&["setup", "--help"]);
    assert_eq!(code, 0);
    assert!(out.contains("--yes") || out.contains("-y"), "setup --help should mention --yes: {}", out);
}

#[test]
fn setup_idempotent_when_bin_dir_already_in_path() {
    let bin_dir = std::path::Path::new(BIN)
        .parent()
        .expect("BIN has a parent")
        .to_string_lossy()
        .into_owned();
    let extra = format!("{}:{}", bin_dir, std::env::var("PATH").unwrap_or_default());
    let out = Command::new(BIN)
        .arg("setup")
        .env("PATH", extra)
        .output()
        .expect("run nbv setup");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let code = out.status.code().unwrap_or(-1);
    assert_eq!(code, 0);
    assert!(
        stdout.contains("already in PATH"),
        "expected idempotent message, got: {}",
        stdout
    );
}

#[test]
fn large_notebook_renders_in_reasonable_time() {
    let start = std::time::Instant::now();
    let (out, _err, code) = run(&["--no-color", "--no-images", "tests/fixtures/large.ipynb"]);
    let dur = start.elapsed();
    assert_eq!(code, 0);
    assert!(out.contains("x_199"));
    assert!(dur.as_secs() < 5, "rendering 200 cells should be < 5s, was {:?}", dur);
}
