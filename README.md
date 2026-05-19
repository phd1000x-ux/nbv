<p align="center">
  <img src="assets/banner.png" alt="nbv — terminal-native Jupyter notebook viewer" width="820">
</p>

<p align="center">
  <a href="https://crates.io/crates/nbv"><img src="https://img.shields.io/crates/v/nbv.svg" alt="crates.io"></a>
  <a href="https://github.com/phd1000x-ux/nbv/releases"><img src="https://img.shields.io/github/v/release/phd1000x-ux/nbv" alt="GitHub release"></a>
  <a href="LICENSE"><img src="https://img.shields.io/crates/l/nbv.svg" alt="MIT License"></a>
</p>

<p align="center">
  <a href="README.ko.md">Korean</a>
</p>

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
- **Tables.** GFM pipe tables in markdown cells and pandas DataFrame `text/html` output render as box-drawn terminal tables with column alignment.

## Install

**Homebrew (macOS arm64):**

```bash
brew install phd1000x-ux/tap/nbv
```

**Cargo (any platform with Rust 1.70+):**

```bash
cargo install nbv
```

**Prebuilt binary (macOS arm64):**

```bash
curl -L https://github.com/phd1000x-ux/nbv/releases/latest/download/nbv-v0.4.1-aarch64-apple-darwin.tar.gz \
  | tar -xz -C /usr/local/bin
```

**Prebuilt binary (Linux x86_64, static musl):**

```bash
curl -L https://github.com/phd1000x-ux/nbv/releases/latest/download/nbv-v0.4.1-x86_64-unknown-linux-musl.tar.gz \
  | tar -xz -C /usr/local/bin
```

**From source:**

```bash
git clone https://github.com/phd1000x-ux/nbv.git
cd nbv
cargo install --path .
```

If `cargo install` warns that `~/.cargo/bin` is not on your `PATH`, run:

```bash
~/.cargo/bin/nbv setup
```

(Use the full path because `nbv` is not on `PATH` yet — that's the whole problem we're fixing.) `setup` detects your shell (zsh, bash, fish), shows the exact line it would append to your rc file, and asks for confirmation. After confirming, it prints a one-liner you can paste to activate the new `PATH` in the current terminal — different per shell (`export PATH=…` for zsh/bash, `fish_add_path …` for fish). Or just open a new terminal. Pass `--yes` to skip the confirmation prompt.

## Usage

```bash
nbv analysis.ipynb                          # render to stdout
nbv --no-color analysis.ipynb               # disable ANSI colors
nbv --no-images analysis.ipynb              # force image placeholders
nbv --theme InspiredGitHub analysis.ipynb   # use a different syntect theme for code blocks
nbv --list-themes                           # print available syntect theme names
nbv --width 120 analysis.ipynb              # force output width (min 20; default: auto-detect)
NBV_THEME=InspiredGitHub nbv analysis.ipynb # env-var fallback for --theme (flag wins when both)
NBV_WIDTH=120 nbv analysis.ipynb            # env-var fallback for --width
nbv -h                                      # help
nbv -V                                      # version
nbv completion bash                         # print bash completion script
nbv mangen                                  # print section-1 man page
```

That is the full surface. Anything not on a flag is auto-detected from the environment.
`--theme` and `--width` also read `NBV_THEME` / `NBV_WIDTH` from the environment when the flag is absent, so you can `export` them once per shell.

## Shell completion

nbv ships completion scripts for bash, zsh, fish, powershell, and elvish via
clap_complete. Pipe the output to the location your shell expects:

```bash
nbv completion bash       > /etc/bash_completion.d/nbv
nbv completion zsh        > ~/.zfunc/_nbv
nbv completion fish       > ~/.config/fish/completions/nbv.fish
nbv completion powershell > $PROFILE.nbv-completion.ps1   # then dot-source from your profile
```

A man page is available via `nbv mangen`:

```bash
nbv mangen | gzip > /usr/local/share/man/man1/nbv.1.gz   # then `man nbv`
```

## What gets rendered

| ipynb element | v0.4 behavior |
| --- | --- |
| Markdown cells | Headers (H1–H6), lists, blockquotes, inline code, fenced code blocks (highlighted via syntect), bold/italic, link text, GFM tables |
| Code cells | Syntect highlighting using the notebook's kernel language (defaults to Python) |
| `text/plain` output | Plain text inside a cell box |
| `image/png` output | Inline in Ghostty/iTerm2 (native protocol); placeholder box (`🖼 PNG W×H`) elsewhere |
| `text/html` (DataFrame) | Rendered as a box-drawn table; falls back to the kernel's `text/plain` representation when the HTML is not a parseable table |
| Error / traceback | Kernel's ANSI colors preserved when supported, stripped when `--no-color` |
| Stream `stdout`/`stderr` | Plain text inside its own cell box, labeled |
| Unknown cell or output type | Skipped with a `(skipped)` placeholder and a one-line stderr warning; rendering continues |

Not yet in v0.4: math (LaTeX), interactive widgets, JPEG/SVG images, application/json pretty-printing.

## Terminal support

| Terminal | Color | Images |
| --- | --- | --- |
| Ghostty | ✓ | ✓ (kitty graphics protocol) |
| iTerm2 | ✓ | ✓ (OSC 1337) |
| kitty | ✓ | ✓ |
| Terminal.app | ✓ | placeholder |
| Alacritty | ✓ | placeholder |
| tmux (any) | ✓ | placeholder (passthrough on roadmap) |
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
cargo test              # 106 tests (96 unit + 10 integration)
cargo build --release   # ~3 MB binary at target/release/nbv
```

## License

[MIT](LICENSE)
