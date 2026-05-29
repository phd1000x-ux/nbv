use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_nbv");

fn run(args: &[&str]) -> (String, String, i32) {
    let out = Command::new(BIN)
        .args(args)
        .env_remove("NBV_THEME")
        .env_remove("NBV_WIDTH")
        .env_remove("NBV_CELLS")
        .env_remove("NBV_NO_OUTPUT")
        .env_remove("NBV_CODE_ONLY")
        .env_remove("NBV_PLAIN")
        .output()
        .expect("run nbv");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let code = out.status.code().unwrap_or(-1);
    (stdout, stderr, code)
}

fn run_with_env(env: &[(&str, &str)], args: &[&str]) -> (String, String, i32) {
    let mut cmd = Command::new(BIN);
    cmd.args(args)
        .env_remove("NBV_THEME")
        .env_remove("NBV_WIDTH")
        .env_remove("NBV_CELLS")
        .env_remove("NBV_NO_OUTPUT")
        .env_remove("NBV_CODE_ONLY")
        .env_remove("NBV_PLAIN");
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.output().expect("run nbv");
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
    let (out, _err, code) = run(&[
        "--no-color",
        "--no-images",
        "tests/fixtures/with_image.ipynb",
    ]);
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
fn missing_arg_exits_with_code_2_and_prints_usage() {
    let (_out, err, code) = run(&[]);
    assert_eq!(code, 2);
    assert!(
        err.contains("Usage:"),
        "stderr should include usage: {}",
        err
    );
    assert!(
        err.contains("setup"),
        "stderr should mention setup subcommand: {}",
        err
    );
    assert!(
        err.contains("--help"),
        "stderr should hint at --help: {}",
        err
    );
}

#[test]
fn setup_help_lists_subcommand() {
    let (out, _err, code) = run(&["--help"]);
    assert_eq!(code, 0);
    assert!(
        out.contains("setup"),
        "top-level help should mention setup: {}",
        out
    );
}

#[test]
fn setup_subcommand_help_works() {
    let (out, _err, code) = run(&["setup", "--help"]);
    assert_eq!(code, 0);
    assert!(
        out.contains("--yes") || out.contains("-y"),
        "setup --help should mention --yes: {}",
        out
    );
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
    assert!(
        dur.as_secs() < 5,
        "rendering 200 cells should be < 5s, was {:?}",
        dur
    );
}

#[test]
fn tables_notebook_renders_gfm_and_dataframe() {
    let (out, _err, code) = run(&[
        "--no-color",
        "--no-images",
        "tests/fixtures/with_tables.ipynb",
    ]);
    assert_eq!(code, 0);
    // GFM table from the markdown cell
    assert!(out.contains("Alice"), "GFM cell value missing:\n{out}");
    assert!(out.contains("Bob"), "GFM cell value Bob missing:\n{out}");
    // DataFrame rendered from text/html, not the plain repr
    assert!(out.contains("city"), "DataFrame header missing:\n{out}");
    assert!(out.contains("NYC"), "DataFrame cell NYC missing:\n{out}");
    assert!(
        !out.contains("0  NYC   80"),
        "should render the html table, not the raw text/plain repr:\n{out}"
    );
    // box-drawing grid present
    assert!(
        out.contains('┬') && out.contains('┼') && out.contains('┴'),
        "box-drawing grid missing:\n{out}"
    );
}

#[test]
fn invalid_theme_exits_with_helpful_error() {
    let (_out, err, code) = run(&["--theme", "not-a-real-theme", "tests/fixtures/simple.ipynb"]);
    assert_eq!(code, 2, "expected exit 2 on invalid theme, got {code}");
    assert!(
        err.contains("unknown theme"),
        "stderr should explain the error: {err}"
    );
    assert!(
        err.contains("base16-ocean.dark"),
        "stderr should list available themes (including default): {err}"
    );
}

#[test]
fn width_flag_forces_output_columns() {
    let (out, _err, code) = run(&[
        "--no-color",
        "--no-images",
        "--width",
        "120",
        "tests/fixtures/simple.ipynb",
    ]);
    assert_eq!(code, 0);
    // Every emitted line should be exactly 120 chars (frame::wrap_line pads to width).
    for line in out.lines() {
        assert_eq!(
            line.chars().count(),
            120,
            "expected 120-col line, got {} cols: {line:?}",
            line.chars().count()
        );
    }
}

#[test]
fn theme_flag_accepted_end_to_end() {
    let (out, _err, code) = run(&[
        "--no-color",
        "--no-images",
        "--theme",
        "InspiredGitHub",
        "tests/fixtures/simple.ipynb",
    ]);
    assert_eq!(code, 0);
    assert!(
        out.contains("x = 1 + 2"),
        "code content should still render under --theme"
    );
}

#[test]
fn env_theme_used_when_flag_absent() {
    // If clap reads NBV_THEME, args.theme becomes Some("not-a-real-theme"),
    // main.rs validation rejects it → exit 2.
    // If clap ignores NBV_THEME (current behavior before this task),
    // args.theme is None, validation is skipped, default theme used → exit 0.
    let (_out, err, code) = run_with_env(
        &[("NBV_THEME", "not-a-real-theme")],
        &["tests/fixtures/simple.ipynb"],
    );
    assert_eq!(
        code, 2,
        "NBV_THEME should be read into args.theme and fail validation"
    );
    assert!(err.contains("unknown theme"), "got stderr: {err}");
}

#[test]
fn env_theme_valid_renders_with_env_theme() {
    let (out, _err, code) = run_with_env(
        &[("NBV_THEME", "InspiredGitHub")],
        &["--no-color", "--no-images", "tests/fixtures/simple.ipynb"],
    );
    assert_eq!(code, 0, "valid env-sourced theme should be accepted");
    assert!(out.contains("x = 1 + 2"), "{out}");
}

#[test]
fn flag_overrides_env_theme() {
    // NBV_THEME is INVALID; if env beat flag, this would exit 2.
    // Flag is VALID; if flag wins (correct), this exits 0.
    let (out, _err, code) = run_with_env(
        &[("NBV_THEME", "not-a-real-theme")],
        &[
            "--no-color",
            "--no-images",
            "--theme",
            "InspiredGitHub",
            "tests/fixtures/simple.ipynb",
        ],
    );
    assert_eq!(code, 0, "flag must override env when both present");
    assert!(out.contains("x = 1 + 2"));
}

#[test]
fn env_width_forces_output_columns() {
    let (out, _err, code) = run_with_env(
        &[("NBV_WIDTH", "120")],
        &["--no-color", "--no-images", "tests/fixtures/simple.ipynb"],
    );
    assert_eq!(code, 0);
    for line in out.lines() {
        assert_eq!(
            line.chars().count(),
            120,
            "expected 120-col line from NBV_WIDTH=120: {line:?}",
        );
    }
}

#[test]
fn list_themes_prints_known_names_and_exits_zero() {
    let (out, _err, code) = run(&["--list-themes"]);
    assert_eq!(code, 0, "--list-themes should exit 0, got {code}");
    assert!(
        out.contains("base16-ocean.dark"),
        "default theme missing from list: {out}"
    );
    assert!(
        out.contains("InspiredGitHub"),
        "InspiredGitHub missing from list: {out}"
    );
    let lines: Vec<&str> = out.lines().filter(|l| !l.is_empty()).collect();
    assert!(
        lines.len() >= 5,
        "expected at least 5 themes, got {}: {out}",
        lines.len()
    );
}

#[test]
fn env_width_below_minimum_rejected() {
    let (_out, err, code) = run_with_env(&[("NBV_WIDTH", "5")], &["tests/fixtures/simple.ipynb"]);
    assert_eq!(
        code, 2,
        "NBV_WIDTH=5 should be rejected by clap's range validator"
    );
    // clap's error includes the offending value
    assert!(
        err.contains("5"),
        "stderr should mention the bad value: {err}"
    );
}

#[test]
fn stream_output_has_no_mismatched_number() {
    let (out, _err, code) = run(&[
        "--no-color",
        "--no-images",
        "tests/fixtures/with_stream.ipynb",
    ]);
    assert_eq!(code, 0);
    assert!(
        out.contains("hello from stdout"),
        "stream text missing:\n{out}"
    );
    assert!(
        out.contains("stream (stdout)"),
        "stream label missing:\n{out}"
    );
    // The code cell keeps its real execution count.
    assert!(
        out.contains("In [7]"),
        "code cell exec count missing:\n{out}"
    );
    // Bug was: the stream output was labeled with the cell index ("Out [1]").
    // A stream output has no execution count, so it must carry no bracketed number.
    // The fixture has only one output (a stream, no execute_result), so "Out ["
    // must not appear anywhere — the stream label carries no bracketed number.
    assert!(
        !out.contains("Out ["),
        "stream output must not show a bracketed number:\n{out}"
    );
}

fn frame_count(out: &str) -> usize {
    out.matches("┌─").count()
}

#[test]
fn cells_range_renders_only_subset() {
    let (out, _err, code) = run(&[
        "--no-color", "--no-images",
        "--cells", "5-10",
        "tests/fixtures/large.ipynb",
    ]);
    assert_eq!(code, 0);
    let frames = frame_count(&out);
    assert!(frames >= 6, "expected >=6 frames, got {frames}");
    let full = run(&["--no-color", "--no-images", "tests/fixtures/large.ipynb"]).0;
    assert!(out.len() < full.len(), "filtered output should be smaller");
}

#[test]
fn cells_range_past_end_clamps_silently() {
    let (out, err, code) = run(&[
        "--no-color", "--no-images",
        "--cells", "100-9999",
        "tests/fixtures/large.ipynb",
    ]);
    assert_eq!(code, 0);
    assert!(err.is_empty(), "no stderr expected: {err}");
    assert!(!out.is_empty(), "should render some cells");
}

#[test]
fn cells_range_entirely_past_end_renders_nothing() {
    let (out, _err, code) = run(&[
        "--no-color", "--no-images",
        "--cells", "9999",
        "tests/fixtures/large.ipynb",
    ]);
    assert_eq!(code, 0);
    assert!(out.is_empty(), "expected empty output, got: {out}");
}

#[test]
fn cells_full_range_matches_unfiltered() {
    let filtered = run(&[
        "--no-color", "--no-images",
        "--cells", "1-200",
        "tests/fixtures/large.ipynb",
    ]).0;
    let full = run(&[
        "--no-color", "--no-images",
        "tests/fixtures/large.ipynb",
    ]).0;
    assert_eq!(filtered, full);
}

#[test]
fn no_output_hides_kernel_outputs() {
    let (out, _err, code) = run(&[
        "--no-color", "--no-images",
        "--no-output",
        "tests/fixtures/with_stream.ipynb",
    ]);
    assert_eq!(code, 0);
    assert!(!out.contains("Out ── stream"), "Out frames must be hidden: {out}");
    assert!(out.contains("In ["), "In frames must remain");
}

#[test]
fn code_only_drops_markdown_and_outputs() {
    let (out, _err, code) = run(&[
        "--no-color", "--no-images",
        "--code-only",
        "tests/fixtures/simple.ipynb",
    ]);
    assert_eq!(code, 0);
    assert!(!out.contains("markdown"), "no markdown frames: {out}");
    assert!(out.contains("In ["), "code frames must remain");
}

#[test]
fn plain_strips_boxes_and_uses_prefixes() {
    let (out, _err, code) = run(&[
        "--plain",
        "tests/fixtures/simple.ipynb",
    ]);
    assert_eq!(code, 0);
    assert!(!out.contains('┌'), "no frame chars: {out}");
    assert!(!out.contains('│'), "no frame chars: {out}");
    assert!(out.contains("[markdown]"), "markdown prefix expected: {out}");
    assert!(out.contains("[code]"), "code prefix expected: {out}");
}

#[test]
fn plain_implies_no_color_no_images() {
    let (out, _err, code) = run(&[
        "--plain",
        "tests/fixtures/with_image.ipynb",
    ]);
    assert_eq!(code, 0);
    assert!(!out.contains('\x1b'), "no ANSI escapes: {out:?}");
    assert!(out.contains("[image]"), "image prefix expected: {out}");
    assert!(out.contains("PNG"), "image dimensions expected: {out}");
}

#[test]
fn plain_and_code_only_compose() {
    let (out, _err, code) = run(&[
        "--plain",
        "--code-only",
        "tests/fixtures/simple.ipynb",
    ]);
    assert_eq!(code, 0);
    assert!(out.contains("[code]"));
    assert!(!out.contains("[markdown]"));
    assert!(!out.contains("[stdout]"));
}

#[test]
fn nbv_cells_env_var_works() {
    let (out, _err, code) = run_with_env(
        &[("NBV_CELLS", "5-10")],
        &["--no-color", "--no-images", "tests/fixtures/large.ipynb"],
    );
    assert_eq!(code, 0);
    let full = run(&["--no-color", "--no-images", "tests/fixtures/large.ipynb"]).0;
    assert!(out.len() < full.len());
}
