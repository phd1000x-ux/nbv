use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_nbv");

fn run(args: &[&str]) -> (String, String, i32) {
    let out = Command::new(BIN)
        .args(args)
        .env_remove("NBV_THEME")
        .env_remove("NBV_WIDTH")
        .output()
        .expect("run nbv");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let code = out.status.code().unwrap_or(-1);
    (stdout, stderr, code)
}

#[test]
fn renders_markdown_file_bare() {
    let (out, _err, code) = run(&[
        "--no-color",
        "--no-images",
        "--width",
        "60",
        "tests/fixtures/sample.md",
    ]);
    assert_eq!(code, 0, "exit code");
    assert!(!out.contains('│'), "bare md must not show │; got:\n{}", out);
    assert!(out.contains("Sample"), "heading text missing");
    assert!(out.contains("italic"));
    assert!(out.contains("bold"));
    assert!(out.contains("• one"));
    assert!(out.contains("col a") && out.contains("col b"));
    assert!(out.contains("println!"));
}

#[test]
fn renders_markdown_extension_uppercase() {
    let src = std::fs::read_to_string("tests/fixtures/sample.md").unwrap();
    let tmp = std::env::temp_dir().join("nbv_sample.MD");
    std::fs::write(&tmp, &src).unwrap();
    let (out, _err, code) = run(&[
        "--no-color",
        "--no-images",
        "--width",
        "60",
        tmp.to_str().unwrap(),
    ]);
    assert_eq!(code, 0);
    assert!(out.contains("Sample"));
}
