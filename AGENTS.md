# AGENTS.md

`nbv` is a single-binary terminal viewer for `.ipynb` notebooks and `.md` files, written in Rust. Cross-platform (macOS/Linux/Windows). Published to crates.io and Homebrew; `README.md` is the full user-facing reference.

## Toolchain

- Rust **1.95.0** is pinned via `rust-toolchain.toml` (with `rustfmt` + `clippy`). Do not assume `stable` — the pin is intentional.
- Edition 2021. No `Makefile`/`Justfile`/pre-commit — everything is plain `cargo`.

## Verification (run before claiming done)

CI (`.github/workflows/ci.yml`) runs these in order; all three must pass locally:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings   # warnings are errors
cargo test --all                             # note the --all
```

Single-package workspace, so `cargo test --all` == full suite. `cargo build --release` produces `target/release/nbv`.

## Architecture

- **Lib + bin in one crate**, both named `nbv` (`src/main.rs`, `src/lib.rs`). The lib exists so unit tests can reach internals.
- `main.rs` is the sole dispatch point: parses `Args` (clap), routes subcommands (`setup` / `completion` / `mangen`) or hands a file to `render_notebook_file` / `render_markdown_file` based on extension.
- `env::detect()` builds a `RenderCtx` from terminal auto-detection + CLI overrides; this is the seam everything renders against. Rendering streams cells to a `BufWriter` with **per-cell flush** (the "fast" guarantee) — preserve that, don't buffer the whole notebook.
- Modules: `cli` (args + env-bool/cells parsers), `env` (RenderCtx), `ipynb` (parse/model), `render/` (the bulk: `code`, `markdown`, `document`, `frame`, `table`, `html_table`, `image/`, `output`, `plain`, `traceback`, …), `setup/`, `generate`, `theme`.

## Cross-platform seams

- `#[cfg(unix)]`: SIGPIPE handler via `signal-hook` (so `nbv x.ipynb | head` exits 0). `signal-hook` is a unix-only dep.
- `#[cfg(windows)]`: crossterm VT processing is enabled at startup for ANSI on legacy consoles.
- `setup/` splits into `unix.rs` (shell rc files) and `windows.rs` (PowerShell `PATH`). Any new terminal/OS behavior likely needs both sides.

## Testing

- **Unit tests** live inline in each `src/` module (`#[cfg(test)]`).
- **Integration tests** (`tests/integration.rs`, `tests/generate.rs`) are **black-box**: they spawn the compiled binary via `CARGO_BIN_EXE_nbv`. Fixtures live in `tests/fixtures/`.
- When adding an integration test, **`env_remove` all `NBV_*` vars** (see the `run` helpers) or results are non-deterministic. This is the single most important test convention here.

## Flags ↔ env vars

Every render flag has an `NBV_*` env fallback wired through clap's `env` feature (`cli.rs`): `--theme`/`NBV_THEME`, `--width`/`NBV_WIDTH`, `--cells`/`NBV_CELLS`, plus boolean flags `--no-output`/`--code-only`/`--plain` using `parse_env_bool` with `require_equals` (so `--plain x.ipynb` doesn't swallow the positional). **Adding a render flag means wiring its env twin too.**

Exit codes (for new error paths): `0` success, `1` file IO, `2` bad CLI args (clap), `3` malformed `.ipynb`.

## Conventions

- **Conventional Commits** with scopes, e.g. `feat(render): …`, `fix(ipynb): …`, `docs(readme): …`, `refactor(cli): …`, `test: …`, `release: …`. Match existing style.
- `README.ko.md` is a Korean translation of `README.md` — keep both in sync for user-facing changes.
- `sample.ipynb` (repo root) is the one-shot demo referenced by the README; treat as a fixture.
- Releases are **tag-driven**: pushing a `v*` tag triggers `.github/workflows/release.yml` (per-platform musl/darwin/windows binaries). Don't tag manually unless releasing.
- `pulldown-cmark` is 0.x — **minor bumps are breaking**; dependabot is configured to PR only patches (`.github/dependabot.yml`). Don't blindly bump it.

## Tooling artifacts (not source)

`target/`, `docs/superpowers/`, `.superpowers/`, `.codegraph/`, `.claude/` are generated/local — ignore them.
