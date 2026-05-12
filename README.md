# nbv

[한국어](README.ko.md)

A fast terminal-native Jupyter notebook viewer.

Browse `.ipynb` files in your terminal without firing up a browser, JupyterLab, or VS Code. Designed for the `cat`/`bat`-style workflow — one command, full output streamed to stdout, no UI to navigate.

## Demo

```
$ nbv analysis.ipynb
┌─ markdown ───────────────────────────────────────────────┐
│ # Simple Notebook                                        │
│ A basic test.                                            │
└──────────────────────────────────────────────────────────┘
┌─ In [1] ── code (python) ────────────────────────────────┐
│ x = 1 + 2                                                │
└──────────────────────────────────────────────────────────┘
┌─ Out [1] ── text/plain ──────────────────────────────────┐
│ 3                                                        │
└──────────────────────────────────────────────────────────┘
```

In Ghostty or iTerm2, matplotlib/seaborn PNG outputs render inline.

## Features

- **Fast.** Per-cell stdout flush — first cell visible in milliseconds, even on 200-cell notebooks (<200 ms total).
- **Single binary.** ~3 MB, no runtime dependencies, no Python required.
- **Inline images.** Ghostty and iTerm2 graphics protocols supported natively. Other terminals get a placeholder box with PNG dimensions.
- **Smart defaults.** Auto-detects TTY, terminal program, and color support. No environment variables needed.
- **Pipe-safe.** Detects non-TTY and degrades gracefully. `nbv x.ipynb | less -R` works; `SIGPIPE` from `| head` exits 0 cleanly.
- **Polished.** Cell boxes drawn with Unicode box-drawing, syntax-highlighted code via `syntect`, formatted markdown, ANSI-colored tracebacks preserved from the kernel.

## Install

```bash
cargo install nbv
```

Requires Rust 1.70 or newer. Or grab the prebuilt macOS arm64 binary from the [latest release](https://github.com/phd1000x-ux/nbv/releases/latest):

```bash
curl -L https://github.com/phd1000x-ux/nbv/releases/latest/download/nbv-v0.1.2-aarch64-apple-darwin.tar.gz \
  | tar -xz -C /usr/local/bin
```

Or build from source:

```bash
git clone https://github.com/phd1000x-ux/nbv.git
cd nbv
cargo install --path .
```

Tested on macOS arm64; Linux likely works but unverified for v0.1. Homebrew tap is planned for v0.2.

If `cargo install` warns that `~/.cargo/bin` is not on your `PATH`, run:

```bash
~/.cargo/bin/nbv setup
```

(Use the full path because `nbv` is not on `PATH` yet — that's the whole problem we're fixing.) `setup` detects your shell (zsh, bash, fish), shows the exact line it would append to your rc file, and asks for confirmation. After confirming, it prints a one-liner you can paste to activate the new `PATH` in the current terminal — different per shell (`export PATH=…` for zsh/bash, `fish_add_path …` for fish). Or just open a new terminal. Pass `--yes` to skip the confirmation prompt.

## Usage

```bash
nbv analysis.ipynb               # render to stdout
nbv --no-color analysis.ipynb    # disable ANSI colors
nbv --no-images analysis.ipynb   # force image placeholders
nbv -h                           # help
nbv -V                           # version
```

That is the full surface. Anything not on a flag is auto-detected from the environment.

## What gets rendered

| ipynb element | v0.1 behavior |
| --- | --- |
| Markdown cells | Headers (H1–H6), lists, blockquotes, inline code, fenced code blocks (highlighted via syntect), bold/italic, link text |
| Code cells | Syntect highlighting using the notebook's kernel language (defaults to Python) |
| `text/plain` output | Plain text inside a cell box |
| `image/png` output | Inline in Ghostty/iTerm2 (native protocol); placeholder box (`🖼 PNG W×H`) elsewhere |
| `text/html` (DataFrame) | Falls back to the kernel's `text/plain` representation — pandas always emits both |
| Error / traceback | Kernel's ANSI colors preserved when supported, stripped when `--no-color` |
| Stream `stdout`/`stderr` | Plain text inside its own cell box, labeled |
| Unknown cell or output type | Skipped with a `(skipped)` placeholder and a one-line stderr warning; rendering continues |

Not yet in v0.1: tables in markdown, math (LaTeX), interactive widgets, JPEG/SVG images, application/json pretty-printing.

## Terminal support

| Terminal | Color | Images |
| --- | --- | --- |
| Ghostty | ✓ | ✓ (kitty graphics protocol) |
| iTerm2 | ✓ | ✓ (OSC 1337) |
| kitty | ✓ | ✓ |
| Terminal.app | ✓ | placeholder |
| Alacritty | ✓ | placeholder |
| tmux (any) | ✓ | placeholder (passthrough is v0.2) |
| Pipe / non-TTY | (respects `NO_COLOR`) | placeholder |

Detection is automatic from `$TERM_PROGRAM` and `$TERM`. Override with `--no-color`, `--no-images`, or `NO_COLOR=1`.

## Exit codes

| code | meaning |
| --- | --- |
| 0 | success |
| 1 | file IO error (not found, permission denied) |
| 2 | invalid CLI arguments (handled by clap) |
| 3 | malformed `.ipynb` (JSON parse error or schema violation) |

## Develop

```bash
cargo test              # 85 tests (78 unit + 7 integration)
cargo build --release   # ~3 MB binary at target/release/nbv
```

Design spec: `docs/superpowers/specs/2026-05-12-nbv-design.md`
Implementation plan: `docs/superpowers/plans/2026-05-12-nbv-implementation.md`

## License

MIT
