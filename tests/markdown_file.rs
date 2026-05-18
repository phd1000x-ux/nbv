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
    let tmp = std::env::temp_dir().join(format!("nbv_sample_{}.MD", std::process::id()));
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
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn unknown_extension_errors_with_exit_2() {
    let (_out, err, code) = run(&["tests/fixtures/unknown.xyz"]);
    assert_eq!(code, 2, "exit code");
    assert!(
        err.contains("unsupported file type"),
        "stderr should mention unsupported file type; got:\n{}",
        err
    );
    assert!(err.contains("tests/fixtures/unknown.xyz"));
    assert!(err.contains(".ipynb"));
    assert!(err.contains(".md"));
}

#[test]
fn missing_markdown_file_errors_with_exit_1() {
    let (_out, err, code) = run(&["tests/fixtures/does-not-exist.md"]);
    assert_eq!(code, 1);
    assert!(err.contains("tests/fixtures/does-not-exist.md"));
    assert!(err.starts_with("nbv: tests/fixtures/does-not-exist.md:"));
}

#[test]
fn image_ref_renders_alt_text_only() {
    let tmp = std::env::temp_dir().join(format!("nbv_image_md_{}.md", std::process::id()));
    std::fs::write(&tmp, "Logo: ![the-logo](logo.png) inline.\n").unwrap();
    let (out, _err, code) = run(&[
        "--no-color",
        "--no-images",
        "--width",
        "40",
        tmp.to_str().unwrap(),
    ]);
    assert_eq!(code, 0);
    assert!(
        out.contains("the-logo"),
        "alt text should appear; got:\n{}",
        out
    );
    assert!(
        !out.contains("logo.png"),
        "URL should not appear; got:\n{}",
        out
    );
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn yaml_front_matter_does_not_crash() {
    let tmp = std::env::temp_dir().join(format!("nbv_frontmatter_{}.md", std::process::id()));
    std::fs::write(
        &tmp,
        "---\ntitle: Hello\nauthor: Me\n---\n\n# Body\n\nReal content.\n",
    )
    .unwrap();
    let (out, _err, code) = run(&[
        "--no-color",
        "--no-images",
        "--width",
        "40",
        tmp.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "front matter must not crash the renderer");
    assert!(out.contains("Real content."));
    let _ = std::fs::remove_file(&tmp);
}
