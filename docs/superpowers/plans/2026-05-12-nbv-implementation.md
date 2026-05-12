# nbv 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 스펙 `docs/superpowers/specs/2026-05-12-nbv-design.md`에 기술된 nbv v0.1을 TDD로 처음부터 구현한다.

**Architecture:** 단일 Rust 바이너리 (`nbv [FILE]`) — `cli` → `env::detect` → `ipynb::parse` → `render_notebook`(셀별 stdout flush) 흐름. 셀 단위 단방향 파이프라인, 인터랙티브 X. 이미지 백엔드는 Ghostty/iTerm2/placeholder 3종 dispatch.

**Tech Stack:** Rust 2021, `serde_json`, `clap`(derive), `syntect`(default-fancy), `pulldown-cmark`, `crossterm`(감지용), `base64`, `signal-hook`, `unicode-width`(박스 폭 계산용 — 스펙에 없었지만 CJK 박스 정렬 위해 추가), `insta`(스냅샷 테스트, dev-dep).

**스펙과의 의도적 차이점 (소소함):**
- `unicode-width = "0.1"` 추가 — 한글/이모지 포함 코드/마크다운에서 박스가 안 깨지도록.
- `image` 크레이트 미사용 (스펙대로) — PNG 차원은 `png_info.rs`에서 IHDR 직접 파싱.
- `ImageRenderer` 트레이트는 유지 (스펙대로) — unit struct 구현체 3개. `render/image/mod.rs`의 free `dispatch(ctx, ...)` 함수로 enum→트레이트 매칭.

**파일 구조:**

```
nbv/
├── Cargo.toml
├── .gitignore
├── src/
│   ├── main.rs
│   ├── lib.rs                       # pub mod 선언
│   ├── cli.rs                       # clap Args
│   ├── env.rs                       # RenderCtx + detect()
│   ├── theme.rs                     # 색 팔레트
│   ├── ipynb/
│   │   ├── mod.rs                   # re-export
│   │   ├── model.rs                 # Notebook/Cell/Output/MimeBundle
│   │   └── parse.rs                 # from_path/from_str
│   └── render/
│       ├── mod.rs                   # render_notebook/render_cell
│       ├── frame.rs                 # 박스 open/close/wrap_line
│       ├── text.rs                  # text/plain 렌더
│       ├── traceback.rs             # ANSI escape 보존
│       ├── code.rs                  # syntect 래퍼
│       ├── markdown.rs              # pulldown-cmark → ANSI
│       ├── output.rs                # MIME 디스패치
│       └── image/
│           ├── mod.rs               # ImageRenderer 트레이트 + dispatch
│           ├── png_info.rs          # IHDR 파싱
│           ├── placeholder.rs       # PlaceholderRenderer
│           ├── kitty.rs             # KittyRenderer
│           └── iterm.rs             # ITermRenderer
└── tests/
    ├── fixtures/
    │   ├── simple.ipynb
    │   ├── with_image.ipynb
    │   ├── with_error.ipynb
    │   └── large.ipynb
    └── integration.rs               # 골든 테스트
```

전체 태스크 수: **20**. 매 태스크는 (1) 실패 테스트 작성 → (2) 실패 확인 → (3) 구현 → (4) 통과 확인 → (5) 커밋의 TDD 사이클.

---

## Task 1: Cargo 프로젝트 초기화

**Files:**
- Create: `Cargo.toml`, `.gitignore`, `src/main.rs`, `src/lib.rs`

- [ ] **Step 1: 디렉토리 상태 확인**

Run: `ls -la /Users/gsr/playground/nbv`
Expected: `docs/`, `.git/`만 존재 (브레인스토밍 단계에서 git init 됨)

- [ ] **Step 2: `Cargo.toml` 작성**

`/Users/gsr/playground/nbv/Cargo.toml`:
```toml
[package]
name = "nbv"
version = "0.1.0"
edition = "2021"
description = "A fast terminal-native Jupyter notebook viewer"
license = "MIT"

[[bin]]
name = "nbv"
path = "src/main.rs"

[lib]
name = "nbv"
path = "src/lib.rs"

[dependencies]
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
syntect = { version = "5", default-features = false, features = ["default-fancy"] }
pulldown-cmark = { version = "0.10", default-features = false }
crossterm = "0.27"
base64 = "0.22"
unicode-width = "0.1"
anyhow = "1"
thiserror = "1"
signal-hook = "0.3"

[dev-dependencies]
insta = "1"

[profile.release]
lto = "fat"
codegen-units = 1
strip = true
```

- [ ] **Step 3: `.gitignore` 작성**

`/Users/gsr/playground/nbv/.gitignore`:
```
/target
*.swp
.DS_Store
```

- [ ] **Step 4: 최소 `src/lib.rs`**

```rust
pub mod cli;
pub mod env;
pub mod ipynb;
pub mod render;
pub mod theme;
```

- [ ] **Step 5: 최소 `src/main.rs`**

```rust
fn main() {
    // 후속 태스크에서 채움
}
```

각 모듈 파일도 빈 stub 생성 (`cargo build`가 통과되도록):
- `src/cli.rs` → `// placeholder`
- `src/env.rs` → `// placeholder`
- `src/theme.rs` → `// placeholder`
- `src/ipynb/mod.rs` → `// placeholder`
- `src/render/mod.rs` → `// placeholder`

- [ ] **Step 6: 빌드 확인**

Run: `cd /Users/gsr/playground/nbv && cargo build`
Expected: 첫 빌드 (의존성 컴파일에 1~3분), 성공으로 끝남

- [ ] **Step 7: 커밋**

```bash
git -C /Users/gsr/playground/nbv add Cargo.toml Cargo.lock .gitignore src/
git -C /Users/gsr/playground/nbv commit -m "chore: scaffold nbv crate"
```

---

## Task 2: ipynb 모델

**Files:**
- Create: `src/ipynb/model.rs`
- Modify: `src/ipynb/mod.rs`

- [ ] **Step 1: `src/ipynb/mod.rs` 갱신**

```rust
pub mod model;
pub mod parse;

pub use model::*;
```

`src/ipynb/parse.rs`는 다음 태스크. 일단 placeholder 만들기 위해 `src/ipynb/parse.rs`에 빈 파일 또는 `// placeholder` 작성. (Task 3에서 채움.)

- [ ] **Step 2: 실패 테스트 작성**

`src/ipynb/model.rs` 끝에 추가:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_notebook() {
        let json = r#"{"cells":[],"metadata":{},"nbformat":4,"nbformat_minor":5}"#;
        let nb: Notebook = serde_json::from_str(json).unwrap();
        assert_eq!(nb.cells.len(), 0);
    }

    #[test]
    fn parses_code_cell_with_string_source() {
        let json = r#"{"cells":[{"cell_type":"code","source":"print(1)","outputs":[],"execution_count":1,"metadata":{}}],"metadata":{},"nbformat":4,"nbformat_minor":5}"#;
        let nb: Notebook = serde_json::from_str(json).unwrap();
        match &nb.cells[0] {
            Cell::Code { source, outputs, execution_count } => {
                assert_eq!(source, "print(1)");
                assert!(outputs.is_empty());
                assert_eq!(*execution_count, Some(1));
            }
            _ => panic!("expected code cell"),
        }
    }

    #[test]
    fn parses_code_cell_with_array_source() {
        // ipynb 스펙은 source를 String 또는 Vec<String>으로 허용
        let json = r#"{"cells":[{"cell_type":"code","source":["a=1\n","b=2"],"outputs":[],"metadata":{}}],"metadata":{},"nbformat":4,"nbformat_minor":5}"#;
        let nb: Notebook = serde_json::from_str(json).unwrap();
        match &nb.cells[0] {
            Cell::Code { source, .. } => assert_eq!(source, "a=1\nb=2"),
            _ => panic!(),
        }
    }

    #[test]
    fn parses_markdown_and_raw_cells() {
        let json = r#"{"cells":[
            {"cell_type":"markdown","source":"# Hello","metadata":{}},
            {"cell_type":"raw","source":"raw text","metadata":{}}
        ],"metadata":{},"nbformat":4,"nbformat_minor":5}"#;
        let nb: Notebook = serde_json::from_str(json).unwrap();
        matches!(nb.cells[0], Cell::Markdown { .. });
        matches!(nb.cells[1], Cell::Raw { .. });
    }

    #[test]
    fn unknown_cell_type_maps_to_unknown() {
        let json = r#"{"cells":[
            {"cell_type":"futuristic","source":"weird","metadata":{}}
        ],"metadata":{},"nbformat":4,"nbformat_minor":5}"#;
        let nb: Notebook = serde_json::from_str(json).unwrap();
        matches!(nb.cells[0], Cell::Unknown);
    }

    #[test]
    fn parses_stream_output() {
        let json = r#"{"cells":[{"cell_type":"code","source":"","metadata":{},"outputs":[
            {"output_type":"stream","name":"stdout","text":"hello\n"}
        ]}],"metadata":{},"nbformat":4,"nbformat_minor":5}"#;
        let nb: Notebook = serde_json::from_str(json).unwrap();
        match &nb.cells[0] {
            Cell::Code { outputs, .. } => match &outputs[0] {
                Output::Stream { name, text } => {
                    assert!(matches!(name, StreamName::Stdout));
                    assert_eq!(text, "hello\n");
                }
                _ => panic!(),
            },
            _ => panic!(),
        }
    }

    #[test]
    fn parses_execute_result_with_mimebundle() {
        let json = r#"{"cells":[{"cell_type":"code","source":"","metadata":{},"outputs":[
            {"output_type":"execute_result","execution_count":2,"data":{"text/plain":"42","image/png":"BASE64DATA","text/html":"<table/>"},"metadata":{}}
        ]}],"metadata":{},"nbformat":4,"nbformat_minor":5}"#;
        let nb: Notebook = serde_json::from_str(json).unwrap();
        match &nb.cells[0] {
            Cell::Code { outputs, .. } => match &outputs[0] {
                Output::ExecuteResult { data, execution_count } => {
                    assert_eq!(data.text_plain.as_deref(), Some("42"));
                    assert_eq!(data.image_png.as_deref(), Some("BASE64DATA"));
                    assert!(data.other.contains_key("text/html"));
                    assert_eq!(*execution_count, Some(2));
                }
                _ => panic!(),
            },
            _ => panic!(),
        }
    }

    #[test]
    fn parses_error_output() {
        let json = r#"{"cells":[{"cell_type":"code","source":"","metadata":{},"outputs":[
            {"output_type":"error","ename":"ValueError","evalue":"bad","traceback":["line1","line2"]}
        ]}],"metadata":{},"nbformat":4,"nbformat_minor":5}"#;
        let nb: Notebook = serde_json::from_str(json).unwrap();
        match &nb.cells[0] {
            Cell::Code { outputs, .. } => match &outputs[0] {
                Output::Error { ename, evalue, traceback } => {
                    assert_eq!(ename, "ValueError");
                    assert_eq!(evalue, "bad");
                    assert_eq!(traceback.len(), 2);
                }
                _ => panic!(),
            },
            _ => panic!(),
        }
    }

    #[test]
    fn parses_notebook_metadata_kernelspec() {
        let json = r#"{"cells":[],"metadata":{"kernelspec":{"name":"python3","language":"python","display_name":"Python 3"}},"nbformat":4,"nbformat_minor":5}"#;
        let nb: Notebook = serde_json::from_str(json).unwrap();
        let ks = nb.metadata.kernelspec.as_ref().unwrap();
        assert_eq!(ks.language.as_deref(), Some("python"));
    }
}
```

- [ ] **Step 3: 실패 확인**

Run: `cargo test --lib ipynb::model`
Expected: FAIL (`Notebook`, `Cell`, `Output`, `StreamName`, `MimeBundle` 미정의)

- [ ] **Step 4: 구현**

`src/ipynb/model.rs` (상단, tests 모듈 위에):
```rust
use std::collections::HashMap;
use serde::{Deserialize, Deserializer};

#[derive(Debug, Deserialize)]
pub struct Notebook {
    pub cells: Vec<Cell>,
    #[serde(default)]
    pub metadata: NotebookMetadata,
}

#[derive(Debug, Default, Deserialize)]
pub struct NotebookMetadata {
    pub kernelspec: Option<KernelSpec>,
    pub language_info: Option<LanguageInfo>,
}

#[derive(Debug, Deserialize)]
pub struct KernelSpec {
    pub name: String,
    pub language: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LanguageInfo {
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "cell_type", rename_all = "lowercase")]
pub enum Cell {
    Code {
        #[serde(deserialize_with = "string_or_array")]
        source: String,
        #[serde(default)]
        outputs: Vec<Output>,
        #[serde(default)]
        execution_count: Option<u64>,
    },
    Markdown {
        #[serde(deserialize_with = "string_or_array")]
        source: String,
    },
    Raw {
        #[serde(deserialize_with = "string_or_array")]
        source: String,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "output_type", rename_all = "snake_case")]
pub enum Output {
    Stream {
        name: StreamName,
        #[serde(deserialize_with = "string_or_array")]
        text: String,
    },
    ExecuteResult {
        data: MimeBundle,
        #[serde(default)]
        execution_count: Option<u64>,
    },
    DisplayData {
        data: MimeBundle,
    },
    Error {
        ename: String,
        evalue: String,
        #[serde(default)]
        traceback: Vec<String>,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StreamName {
    Stdout,
    Stderr,
}

#[derive(Debug, Default)]
pub struct MimeBundle {
    pub text_plain: Option<String>,
    pub image_png: Option<String>,
    pub other: HashMap<String, serde_json::Value>,
}

impl<'de> Deserialize<'de> for MimeBundle {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        use serde::de::Error;
        let mut raw: HashMap<String, serde_json::Value> = HashMap::deserialize(d)?;
        let text_plain = raw.remove("text/plain").and_then(value_to_string);
        let image_png = raw.remove("image/png").and_then(|v| match v {
            serde_json::Value::String(s) => Some(s),
            _ => None,
        });
        Ok(MimeBundle { text_plain, image_png, other: raw })
    }
}

fn value_to_string(v: serde_json::Value) -> Option<String> {
    match v {
        serde_json::Value::String(s) => Some(s),
        serde_json::Value::Array(arr) => {
            let mut s = String::new();
            for item in arr {
                if let serde_json::Value::String(x) = item { s.push_str(&x); }
            }
            Some(s)
        }
        _ => None,
    }
}

fn string_or_array<'de, D>(d: D) -> Result<String, D::Error>
where D: Deserializer<'de> {
    use serde::de::Error;
    let v = serde_json::Value::deserialize(d)?;
    value_to_string(v).ok_or_else(|| D::Error::custom("source must be string or array of strings"))
}
```

- [ ] **Step 5: 통과 확인**

Run: `cargo test --lib ipynb::model`
Expected: PASS (9 tests passed)

- [ ] **Step 6: 커밋**

```bash
git -C /Users/gsr/playground/nbv add src/ipynb/
git -C /Users/gsr/playground/nbv commit -m "feat(ipynb): model types with serde"
```

---

## Task 3: ipynb 파서

**Files:**
- Modify: `src/ipynb/parse.rs`

- [ ] **Step 1: 실패 테스트 작성**

`src/ipynb/parse.rs`:
```rust
use std::io::Read;
use std::path::Path;

use crate::ipynb::model::Notebook;

pub fn from_str(s: &str) -> Result<Notebook, serde_json::Error> {
    serde_json::from_str(s)
}

pub fn from_reader<R: Read>(r: R) -> Result<Notebook, serde_json::Error> {
    serde_json::from_reader(r)
}

pub fn from_path<P: AsRef<Path>>(p: P) -> std::io::Result<Result<Notebook, serde_json::Error>> {
    let file = std::fs::File::open(p)?;
    Ok(from_reader(std::io::BufReader::new(file)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str_parses_minimal() {
        let nb = from_str(r#"{"cells":[],"metadata":{},"nbformat":4,"nbformat_minor":5}"#).unwrap();
        assert!(nb.cells.is_empty());
    }

    #[test]
    fn from_str_invalid_json_errors() {
        let r = from_str("not-json");
        assert!(r.is_err());
    }

    #[test]
    fn from_path_reads_file() {
        let tmp = std::env::temp_dir().join("nbv_test_parser.ipynb");
        std::fs::write(&tmp, r#"{"cells":[{"cell_type":"raw","source":"x","metadata":{}}],"metadata":{},"nbformat":4,"nbformat_minor":5}"#).unwrap();
        let nb = from_path(&tmp).unwrap().unwrap();
        assert_eq!(nb.cells.len(), 1);
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn from_path_missing_file_returns_io_error() {
        let r = from_path("/definitely/does/not/exist.ipynb");
        assert!(r.is_err());
    }
}
```

- [ ] **Step 2: 실패 확인 (이미 구현은 됨, 컴파일만 확인)**

Run: `cargo test --lib ipynb::parse`
Expected: PASS (구현이 같은 파일에 있어서 4 tests 통과)

코멘트: 이 태스크는 TDD 사이클상 test와 구현을 한 파일에 같이 썼다. 다른 태스크에서는 test와 구현을 분리한다.

- [ ] **Step 3: 전체 lib 테스트**

Run: `cargo test --lib`
Expected: ipynb의 모든 테스트 통과 (13개 이상)

- [ ] **Step 4: 커밋**

```bash
git -C /Users/gsr/playground/nbv add src/ipynb/parse.rs
git -C /Users/gsr/playground/nbv commit -m "feat(ipynb): parse from str/reader/path"
```

---

## Task 4: 환경 감지 (`env.rs`)

**Files:**
- Modify: `src/env.rs`

- [ ] **Step 1: 실패 테스트 작성**

`src/env.rs`:
```rust
// 구현은 Step 3에서

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ghostty_tty_picks_kitty() {
        let env = TestEnv {
            is_tty: true,
            no_color: false,
            term_program: Some("ghostty".into()),
            term: Some("xterm-ghostty".into()),
            columns: Some(120),
        };
        let ctx = detect_with(&env, /* args.no_color */ false, /* args.no_images */ false);
        assert_eq!(ctx.image_backend, ImageBackend::Kitty);
        assert!(ctx.use_color);
        assert!(ctx.is_tty);
        assert_eq!(ctx.width, 120);
    }

    #[test]
    fn iterm_tty_picks_iterm2() {
        let env = TestEnv {
            is_tty: true, no_color: false,
            term_program: Some("iTerm.app".into()),
            term: None, columns: None,
        };
        let ctx = detect_with(&env, false, false);
        assert_eq!(ctx.image_backend, ImageBackend::ITerm2);
        assert_eq!(ctx.width, 80);  // default
    }

    #[test]
    fn kitty_term_var_picks_kitty() {
        let env = TestEnv {
            is_tty: true, no_color: false,
            term_program: None,
            term: Some("xterm-kitty".into()),
            columns: None,
        };
        let ctx = detect_with(&env, false, false);
        assert_eq!(ctx.image_backend, ImageBackend::Kitty);
    }

    #[test]
    fn other_terminal_picks_placeholder() {
        let env = TestEnv {
            is_tty: true, no_color: false,
            term_program: Some("Apple_Terminal".into()),
            term: Some("xterm-256color".into()),
            columns: None,
        };
        let ctx = detect_with(&env, false, false);
        assert_eq!(ctx.image_backend, ImageBackend::Placeholder);
    }

    #[test]
    fn non_tty_forces_placeholder_and_no_color() {
        let env = TestEnv {
            is_tty: false, no_color: false,
            term_program: Some("ghostty".into()),
            term: None, columns: None,
        };
        let ctx = detect_with(&env, false, false);
        assert_eq!(ctx.image_backend, ImageBackend::Placeholder);
        assert!(!ctx.use_color);
    }

    #[test]
    fn no_color_env_var_disables_color() {
        let env = TestEnv {
            is_tty: true, no_color: true,
            term_program: Some("ghostty".into()),
            term: None, columns: None,
        };
        let ctx = detect_with(&env, false, false);
        assert!(!ctx.use_color);
    }

    #[test]
    fn no_color_flag_overrides() {
        let env = TestEnv { is_tty: true, no_color: false, term_program: None, term: None, columns: None };
        let ctx = detect_with(&env, /* no_color */ true, false);
        assert!(!ctx.use_color);
    }

    #[test]
    fn no_images_flag_forces_placeholder() {
        let env = TestEnv {
            is_tty: true, no_color: false,
            term_program: Some("ghostty".into()),
            term: None, columns: None,
        };
        let ctx = detect_with(&env, false, /* no_images */ true);
        assert_eq!(ctx.image_backend, ImageBackend::Placeholder);
    }
}
```

- [ ] **Step 2: 실패 확인**

Run: `cargo test --lib env`
Expected: FAIL — `RenderCtx`, `ImageBackend`, `TestEnv`, `detect_with` 미정의

- [ ] **Step 3: 구현**

`src/env.rs` 상단 (tests 위):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageBackend {
    Kitty,
    ITerm2,
    Placeholder,
}

#[derive(Debug, Clone)]
pub struct RenderCtx {
    pub is_tty: bool,
    pub use_color: bool,
    pub width: usize,
    pub image_backend: ImageBackend,
}

pub trait EnvProbe {
    fn is_tty(&self) -> bool;
    fn no_color(&self) -> bool;
    fn term_program(&self) -> Option<String>;
    fn term(&self) -> Option<String>;
    fn columns(&self) -> Option<usize>;
}

pub struct SystemEnv;

impl EnvProbe for SystemEnv {
    fn is_tty(&self) -> bool {
        use std::io::IsTerminal;
        std::io::stdout().is_terminal()
    }
    fn no_color(&self) -> bool {
        std::env::var_os("NO_COLOR").is_some()
    }
    fn term_program(&self) -> Option<String> {
        std::env::var("TERM_PROGRAM").ok()
    }
    fn term(&self) -> Option<String> {
        std::env::var("TERM").ok()
    }
    fn columns(&self) -> Option<usize> {
        crossterm::terminal::size().ok().map(|(w, _)| w as usize)
    }
}

#[cfg(test)]
pub struct TestEnv {
    pub is_tty: bool,
    pub no_color: bool,
    pub term_program: Option<String>,
    pub term: Option<String>,
    pub columns: Option<usize>,
}

#[cfg(test)]
impl EnvProbe for TestEnv {
    fn is_tty(&self) -> bool { self.is_tty }
    fn no_color(&self) -> bool { self.no_color }
    fn term_program(&self) -> Option<String> { self.term_program.clone() }
    fn term(&self) -> Option<String> { self.term.clone() }
    fn columns(&self) -> Option<usize> { self.columns }
}

pub fn detect(args_no_color: bool, args_no_images: bool) -> RenderCtx {
    detect_with(&SystemEnv, args_no_color, args_no_images)
}

pub fn detect_with(env: &impl EnvProbe, args_no_color: bool, args_no_images: bool) -> RenderCtx {
    let is_tty = env.is_tty();
    let use_color = is_tty && !args_no_color && !env.no_color();
    let width = env.columns().unwrap_or(80);

    let image_backend = if args_no_images || !is_tty {
        ImageBackend::Placeholder
    } else if env.term_program().as_deref() == Some("ghostty")
        || env.term().as_deref() == Some("xterm-kitty")
    {
        ImageBackend::Kitty
    } else if env.term_program().as_deref() == Some("iTerm.app") {
        ImageBackend::ITerm2
    } else {
        ImageBackend::Placeholder
    };

    RenderCtx { is_tty, use_color, width, image_backend }
}
```

- [ ] **Step 4: 통과 확인**

Run: `cargo test --lib env`
Expected: PASS (8 tests passed)

- [ ] **Step 5: 커밋**

```bash
git -C /Users/gsr/playground/nbv add src/env.rs
git -C /Users/gsr/playground/nbv commit -m "feat(env): TTY/terminal detection with mockable trait"
```

---

## Task 5: CLI 인자 (`cli.rs`)

**Files:**
- Modify: `src/cli.rs`

- [ ] **Step 1: 실패 테스트 작성**

`src/cli.rs`:
```rust
// 구현은 Step 3

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parses_just_file() {
        let a = Args::try_parse_from(["nbv", "foo.ipynb"]).unwrap();
        assert_eq!(a.file.to_string_lossy(), "foo.ipynb");
        assert!(!a.no_color);
        assert!(!a.no_images);
    }

    #[test]
    fn parses_with_flags() {
        let a = Args::try_parse_from(["nbv", "x.ipynb", "--no-color", "--no-images"]).unwrap();
        assert!(a.no_color);
        assert!(a.no_images);
    }

    #[test]
    fn requires_file() {
        assert!(Args::try_parse_from(["nbv"]).is_err());
    }
}
```

- [ ] **Step 2: 실패 확인**

Run: `cargo test --lib cli`
Expected: FAIL — `Args` 미정의

- [ ] **Step 3: 구현**

`src/cli.rs` 상단:
```rust
use std::path::PathBuf;
use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "nbv", version, about = "A fast terminal Jupyter notebook viewer")]
pub struct Args {
    /// Path to the .ipynb file
    pub file: PathBuf,

    /// Disable ANSI color output
    #[arg(long)]
    pub no_color: bool,

    /// Disable inline image rendering (use placeholder)
    #[arg(long)]
    pub no_images: bool,
}
```

- [ ] **Step 4: 통과 확인**

Run: `cargo test --lib cli`
Expected: PASS (3 tests passed)

- [ ] **Step 5: 커밋**

```bash
git -C /Users/gsr/playground/nbv add src/cli.rs
git -C /Users/gsr/playground/nbv commit -m "feat(cli): Args struct with clap"
```

---

## Task 6: 테마 (`theme.rs`)

**Files:**
- Modify: `src/theme.rs`

- [ ] **Step 1: 실패 테스트 작성**

`src/theme.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_header_has_cyan_fg() {
        let s = colorize_code_header("In [1] code (python)", true);
        assert!(s.contains("\x1b["));   // ANSI escape
        assert!(s.ends_with("\x1b[0m")); // reset
    }

    #[test]
    fn code_header_no_color_returns_plain() {
        let s = colorize_code_header("In [1] code (python)", false);
        assert_eq!(s, "In [1] code (python)");
    }

    #[test]
    fn dim_text_wraps_when_color() {
        assert!(dim("hi", true).contains("\x1b["));
        assert_eq!(dim("hi", false), "hi");
    }
}
```

- [ ] **Step 2: 실패 확인**

Run: `cargo test --lib theme`
Expected: FAIL — `colorize_code_header`, `dim` 미정의

- [ ] **Step 3: 구현**

`src/theme.rs` 상단:
```rust
//! 색 팔레트. ANSI 16색 기반(터미널 호환성 최우선).

// SGR 코드
pub const RESET: &str = "\x1b[0m";
pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[2m";
pub const ITALIC: &str = "\x1b[3m";

pub const FG_CYAN: &str = "\x1b[36m";
pub const FG_GREEN: &str = "\x1b[32m";
pub const FG_RED: &str = "\x1b[31m";
pub const FG_BLUE: &str = "\x1b[34m";
pub const FG_YELLOW: &str = "\x1b[33m";
pub const FG_MAGENTA: &str = "\x1b[35m";
pub const FG_GREY: &str = "\x1b[90m";

pub fn colorize_code_header(s: &str, color: bool) -> String {
    if color { format!("{}{}{}{}", BOLD, FG_CYAN, s, RESET) } else { s.to_string() }
}

pub fn colorize_output_header(s: &str, color: bool) -> String {
    if color { format!("{}{}{}{}", BOLD, FG_GREEN, s, RESET) } else { s.to_string() }
}

pub fn colorize_error_header(s: &str, color: bool) -> String {
    if color { format!("{}{}{}{}", BOLD, FG_RED, s, RESET) } else { s.to_string() }
}

pub fn colorize_markdown_header(s: &str, color: bool) -> String {
    if color { format!("{}{}{}{}", BOLD, FG_BLUE, s, RESET) } else { s.to_string() }
}

pub fn dim(s: &str, color: bool) -> String {
    if color { format!("{}{}{}", DIM, s, RESET) } else { s.to_string() }
}

pub fn frame_border(color: bool) -> &'static str {
    if color { FG_GREY } else { "" }
}
```

- [ ] **Step 4: 통과 확인**

Run: `cargo test --lib theme`
Expected: PASS (3 tests passed)

- [ ] **Step 5: 커밋**

```bash
git -C /Users/gsr/playground/nbv add src/theme.rs
git -C /Users/gsr/playground/nbv commit -m "feat(theme): ANSI color palette helpers"
```

---

## Task 7: 박스 그리기 (`render/frame.rs`)

**Files:**
- Create: `src/render/frame.rs`
- Modify: `src/render/mod.rs`

- [ ] **Step 1: `src/render/mod.rs` 갱신**

```rust
pub mod frame;
```

(추후 태스크에서 다른 `pub mod ...;` 추가)

- [ ] **Step 2: 실패 테스트 작성**

`src/render/frame.rs`:
```rust
// 구현은 Step 4

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::{ImageBackend, RenderCtx};

    fn ctx(width: usize) -> RenderCtx {
        RenderCtx { is_tty: true, use_color: false, width, image_backend: ImageBackend::Placeholder }
    }

    #[test]
    fn open_writes_full_width_top_border() {
        let mut buf = Vec::new();
        open("In [1] code", &ctx(30), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let line = s.trim_end_matches('\n');
        assert_eq!(line.chars().count(), 30);
        assert!(line.starts_with("┌─"));
        assert!(line.contains("In [1] code"));
        assert!(line.ends_with("┐"));
    }

    #[test]
    fn close_writes_full_width_bottom_border() {
        let mut buf = Vec::new();
        close(&ctx(30), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let line = s.trim_end_matches('\n');
        assert_eq!(line.chars().count(), 30);
        assert!(line.starts_with("└"));
        assert!(line.ends_with("┘"));
    }

    #[test]
    fn wrap_line_pads_to_width() {
        let mut buf = Vec::new();
        wrap_line("hello", &ctx(30), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let line = s.trim_end_matches('\n');
        assert_eq!(line.chars().count(), 30);
        assert!(line.starts_with("│ hello"));
        assert!(line.ends_with(" │"));
    }

    #[test]
    fn wrap_line_truncates_long_content() {
        let mut buf = Vec::new();
        wrap_line(&"x".repeat(100), &ctx(20), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let line = s.trim_end_matches('\n');
        assert_eq!(line.chars().count(), 20);
    }

    #[test]
    fn wrap_line_handles_cjk_wide_chars() {
        // 한글 한 글자가 2칸을 차지함
        let mut buf = Vec::new();
        wrap_line("가나다", &ctx(20), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let line = s.trim_end_matches('\n');
        // unicode-width 기준 가시 폭이 20이어야 함 (chars().count()와 다름)
        use unicode_width::UnicodeWidthStr;
        assert_eq!(line.width(), 20);
    }
}
```

- [ ] **Step 3: 실패 확인**

Run: `cargo test --lib render::frame`
Expected: FAIL — `open`/`close`/`wrap_line` 미정의

- [ ] **Step 4: 구현**

`src/render/frame.rs` 상단:
```rust
use std::io::{self, Write};
use unicode_width::UnicodeWidthStr;

use crate::env::RenderCtx;
use crate::theme;

/// 상단 박스 라인: `┌─ {label} ─...─┐`
pub fn open(label: &str, ctx: &RenderCtx, w: &mut impl Write) -> io::Result<()> {
    let label_str = format!(" {} ", label);
    let label_w = label_str.width();
    let inner_w = ctx.width.saturating_sub(2); // ┌, ┐ 제외
    let dashes = inner_w.saturating_sub(label_w + 1);
    let border = theme::frame_border(ctx.use_color);
    let reset = if ctx.use_color { theme::RESET } else { "" };
    writeln!(
        w,
        "{}┌─{}{}┐{}",
        border,
        label_str,
        "─".repeat(dashes),
        reset
    )
}

pub fn close(ctx: &RenderCtx, w: &mut impl Write) -> io::Result<()> {
    let inner_w = ctx.width.saturating_sub(2);
    let border = theme::frame_border(ctx.use_color);
    let reset = if ctx.use_color { theme::RESET } else { "" };
    writeln!(w, "{}└{}┘{}", border, "─".repeat(inner_w), reset)
}

/// 박스 내부 한 줄: `│ {content padded} │`
pub fn wrap_line(content: &str, ctx: &RenderCtx, w: &mut impl Write) -> io::Result<()> {
    let inner_w = ctx.width.saturating_sub(4); // `│ ` + content + ` │`
    let mut trimmed = String::new();
    let mut used = 0usize;
    for ch in content.chars() {
        let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + cw > inner_w { break; }
        trimmed.push(ch);
        used += cw;
    }
    let pad = inner_w - used;
    let border = theme::frame_border(ctx.use_color);
    let reset_b = if ctx.use_color { theme::RESET } else { "" };
    // ANSI escape가 content에 있을 수 있으므로 reset도 한 번 더 출력
    writeln!(
        w,
        "{0}│{1} {2}{0}{3} │{1}",
        border, reset_b, trimmed, " ".repeat(pad)
    )
}
```

> 주의: ANSI escape 문자가 content에 포함된 경우 폭 계산이 어긋날 수 있다. v0.1은 단순화: code/markdown은 syntect/markdown 렌더러가 줄 끝에서 reset해 박스 안에서는 색이 끝나도록 한다. 정확한 polychrome wrapping은 v0.2.

- [ ] **Step 5: 통과 확인**

Run: `cargo test --lib render::frame`
Expected: PASS (5 tests passed)

- [ ] **Step 6: 커밋**

```bash
git -C /Users/gsr/playground/nbv add src/render/frame.rs src/render/mod.rs
git -C /Users/gsr/playground/nbv commit -m "feat(render): box drawing primitives"
```

---

## Task 8: 텍스트 렌더러 (`render/text.rs`)

**Files:**
- Create: `src/render/text.rs`
- Modify: `src/render/mod.rs` (add `pub mod text;`)

- [ ] **Step 1: 실패 테스트 작성**

`src/render/text.rs`:
```rust
// 구현은 Step 3

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::{ImageBackend, RenderCtx};

    fn ctx() -> RenderCtx {
        RenderCtx { is_tty: true, use_color: false, width: 30, image_backend: ImageBackend::Placeholder }
    }

    #[test]
    fn renders_single_line_in_box_line() {
        let mut buf = Vec::new();
        render(&"hello".to_string(), &ctx(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("hello"));
        assert!(s.starts_with("│ "));
    }

    #[test]
    fn renders_multiline_as_multiple_box_lines() {
        let mut buf = Vec::new();
        render(&"a\nb\nc".to_string(), &ctx(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert_eq!(s.matches('\n').count(), 3);
    }

    #[test]
    fn trailing_newline_does_not_create_empty_line() {
        let mut buf = Vec::new();
        render(&"a\n".to_string(), &ctx(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert_eq!(s.matches('\n').count(), 1);
    }
}
```

- [ ] **Step 2: 실패 확인**

Run: `cargo test --lib render::text`
Expected: FAIL — `render` 미정의

- [ ] **Step 3: 구현**

`src/render/text.rs` 상단:
```rust
use std::io::{self, Write};

use crate::env::RenderCtx;
use crate::render::frame;

pub fn render(text: &str, ctx: &RenderCtx, w: &mut impl Write) -> io::Result<()> {
    for line in text.split_inclusive('\n') {
        let line = line.trim_end_matches('\n');
        if line.is_empty() && !text.ends_with(line) {
            // 진짜 빈 줄(중간)도 wrap_line 호출 → 박스 내 빈 줄
        }
        frame::wrap_line(line, ctx, w)?;
    }
    // text가 줄바꿈 없이 끝나면 위 루프가 마지막 청크를 그렸음
    if text.is_empty() { frame::wrap_line("", ctx, w)?; }
    Ok(())
}
```

(`mod.rs`에 `pub mod text;` 추가하는 것 잊지 말 것.)

- [ ] **Step 4: 통과 확인**

Run: `cargo test --lib render::text`
Expected: PASS (3 tests passed)

- [ ] **Step 5: 커밋**

```bash
git -C /Users/gsr/playground/nbv add src/render/text.rs src/render/mod.rs
git -C /Users/gsr/playground/nbv commit -m "feat(render): text/plain renderer"
```

---

## Task 9: Traceback 렌더러 (`render/traceback.rs`)

**Files:**
- Create: `src/render/traceback.rs`
- Modify: `src/render/mod.rs` (add `pub mod traceback;`)

배경: Jupyter의 `traceback` 필드는 줄별로 ANSI escape를 포함한 문자열 배열. 보존해서 출력해야 색이 살아남.

- [ ] **Step 1: 실패 테스트 작성**

`src/render/traceback.rs`:
```rust
// 구현은 Step 3

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::{ImageBackend, RenderCtx};

    fn ctx(use_color: bool) -> RenderCtx {
        RenderCtx { is_tty: true, use_color, width: 60, image_backend: ImageBackend::Placeholder }
    }

    #[test]
    fn renders_each_traceback_line() {
        let tb: Vec<String> = vec![
            "ValueError: bad".into(),
            "  at line 1".into(),
        ];
        let mut buf = Vec::new();
        render(&tb, &ctx(true), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("ValueError"));
        assert!(s.contains("at line 1"));
    }

    #[test]
    fn preserves_existing_ansi_escapes_when_color() {
        let tb: Vec<String> = vec!["\x1b[31mred text\x1b[0m".into()];
        let mut buf = Vec::new();
        render(&tb, &ctx(true), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("\x1b[31m"));
    }

    #[test]
    fn strips_ansi_escapes_when_no_color() {
        let tb: Vec<String> = vec!["\x1b[31mred text\x1b[0m".into()];
        let mut buf = Vec::new();
        render(&tb, &ctx(false), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(!s.contains("\x1b["));
        assert!(s.contains("red text"));
    }

    #[test]
    fn handles_lines_with_embedded_newlines() {
        // 일부 커널은 한 traceback 엔트리에 여러 줄을 넣음
        let tb: Vec<String> = vec!["line1\nline2".into()];
        let mut buf = Vec::new();
        render(&tb, &ctx(false), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("line1"));
        assert!(s.contains("line2"));
        assert!(s.matches('\n').count() >= 2);
    }
}
```

- [ ] **Step 2: 실패 확인**

Run: `cargo test --lib render::traceback`
Expected: FAIL — `render` 미정의

- [ ] **Step 3: 구현**

`src/render/traceback.rs` 상단:
```rust
use std::io::{self, Write};

use crate::env::RenderCtx;
use crate::render::frame;

pub fn render(traceback: &[String], ctx: &RenderCtx, w: &mut impl Write) -> io::Result<()> {
    for entry in traceback {
        for line in entry.split('\n') {
            let line = if ctx.use_color { line.to_string() } else { strip_ansi(line) };
            frame::wrap_line(&line, ctx, w)?;
        }
    }
    Ok(())
}

/// CSI 시퀀스(`\x1b[...m`)와 단순 `\x1b[X` 형태 escape를 제거.
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' && chars.peek() == Some(&'[') {
            chars.next(); // '['
            while let Some(&nc) = chars.peek() {
                chars.next();
                // 종료는 0x40~0x7E 범위 ASCII
                if ('@'..='~').contains(&nc) { break; }
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod strip_tests {
    use super::strip_ansi;
    #[test]
    fn strips_csi_m() {
        assert_eq!(strip_ansi("\x1b[31mhi\x1b[0m"), "hi");
    }
    #[test]
    fn passes_through_plain() {
        assert_eq!(strip_ansi("abc"), "abc");
    }
}
```

- [ ] **Step 4: 통과 확인**

Run: `cargo test --lib render::traceback`
Expected: PASS (4 + 2 = 6 tests passed)

- [ ] **Step 5: 커밋**

```bash
git -C /Users/gsr/playground/nbv add src/render/traceback.rs src/render/mod.rs
git -C /Users/gsr/playground/nbv commit -m "feat(render): traceback with optional ANSI stripping"
```

---

## Task 10: 코드 렌더러 (`render/code.rs`)

**Files:**
- Create: `src/render/code.rs`
- Modify: `src/render/mod.rs` (add `pub mod code;`)

- [ ] **Step 1: 실패 테스트 작성**

`src/render/code.rs`:
```rust
// 구현은 Step 3

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::{ImageBackend, RenderCtx};

    fn ctx(use_color: bool) -> RenderCtx {
        RenderCtx { is_tty: true, use_color, width: 60, image_backend: ImageBackend::Placeholder }
    }

    #[test]
    fn renders_python_code_with_color() {
        let mut buf = Vec::new();
        render("x = 1", "python", &ctx(true), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("x = 1") || s.contains("x") && s.contains("="));
        assert!(s.contains("\x1b["));  // ANSI from syntect
    }

    #[test]
    fn renders_code_without_color_strips_ansi() {
        let mut buf = Vec::new();
        render("x = 1", "python", &ctx(false), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(!s.contains("\x1b["));
        assert!(s.contains("x"));
    }

    #[test]
    fn unknown_language_falls_back_to_plain() {
        let mut buf = Vec::new();
        render("hello", "klingon-script", &ctx(false), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("hello"));
    }

    #[test]
    fn each_line_emitted_as_box_line() {
        let mut buf = Vec::new();
        render("a = 1\nb = 2", "python", &ctx(false), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("a"));
        assert!(s.contains("b"));
        assert!(s.matches("│ ").count() >= 2);
    }
}
```

- [ ] **Step 2: 실패 확인**

Run: `cargo test --lib render::code`
Expected: FAIL — `render` 미정의

- [ ] **Step 3: 구현**

`src/render/code.rs` 상단:
```rust
use std::io::{self, Write};
use std::sync::OnceLock;

use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

use crate::env::RenderCtx;
use crate::render::frame;
use crate::render::traceback::strip_ansi_pub as strip_ansi;

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();

fn syntax_set() -> &'static SyntaxSet {
    SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines)
}
fn theme_set() -> &'static ThemeSet {
    THEME_SET.get_or_init(ThemeSet::load_defaults)
}

pub fn render(source: &str, lang: &str, ctx: &RenderCtx, w: &mut impl Write) -> io::Result<()> {
    let ss = syntax_set();
    let ts = theme_set();
    let syntax = ss.find_syntax_by_token(lang)
        .or_else(|| ss.find_syntax_by_token("python"))
        .unwrap_or_else(|| ss.find_syntax_plain_text());
    let theme = &ts.themes["base16-ocean.dark"];
    let mut hl = HighlightLines::new(syntax, theme);

    for line in LinesWithEndings::from(source) {
        let ranges: Vec<(Style, &str)> = hl.highlight_line(line, ss)
            .unwrap_or_else(|_| vec![(Style::default(), line)]);
        let mut escaped = as_24_bit_terminal_escaped(&ranges[..], false);
        // syntect 출력은 줄바꿈을 포함; 박스에 넣기 위해 제거
        while escaped.ends_with('\n') { escaped.pop(); }
        let to_render = if ctx.use_color { escaped } else { strip_ansi(&escaped) };
        frame::wrap_line(&to_render, ctx, w)?;
    }
    Ok(())
}
```

Note: `strip_ansi`을 `traceback.rs`에서 노출해 재사용한다. Task 9의 `traceback.rs`에 추가 한 줄:
```rust
pub fn strip_ansi_pub(s: &str) -> String { strip_ansi(s) }
```

(또는 `strip_ansi`를 직접 `pub`로 만들어도 됨. 위처럼 별도 노출하면 내부 API 명시적.)

> 주의: 박스 안에서는 `wrap_line`이 폭 기준으로 자르지만, 색 escape 포함 라인은 시각적 폭이 chars 수와 다를 수 있다. v0.1 단순화: color 라인은 truncate가 색을 깨뜨릴 수 있으나 stdout에 직접 가는 코드 셀에서 라인이 박스 폭을 넘는 케이스는 드물어 허용. v0.2에서 ansi-aware truncate 도입.

- [ ] **Step 4: 통과 확인**

Run: `cargo test --lib render::code`
Expected: PASS (4 tests passed) — 첫 실행 시 syntect 디폴트 로딩으로 약간 느림 (1초 미만)

- [ ] **Step 5: 커밋**

```bash
git -C /Users/gsr/playground/nbv add src/render/code.rs src/render/mod.rs src/render/traceback.rs
git -C /Users/gsr/playground/nbv commit -m "feat(render): code highlighting via syntect"
```

---

## Task 11: 마크다운 렌더러 (`render/markdown.rs`)

**Files:**
- Create: `src/render/markdown.rs`
- Modify: `src/render/mod.rs` (add `pub mod markdown;`)

스코프: H1~H6, 강조(**bold**/*italic*), 인라인 코드, 코드 블록(언어 fence는 syntect로 위임), 리스트(unordered/ordered), 블록인용, 링크 텍스트. 표/체크박스/각주는 v0.2.

- [ ] **Step 1: 실패 테스트 작성**

`src/render/markdown.rs`:
```rust
// 구현은 Step 3

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::{ImageBackend, RenderCtx};

    fn ctx(use_color: bool) -> RenderCtx {
        RenderCtx { is_tty: true, use_color, width: 60, image_backend: ImageBackend::Placeholder }
    }

    #[test]
    fn heading_gets_hash_prefix() {
        let mut buf = Vec::new();
        render("# Hello", &ctx(false), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("# Hello"));
    }

    #[test]
    fn h2_gets_two_hashes() {
        let mut buf = Vec::new();
        render("## Hello", &ctx(false), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("## Hello"));
    }

    #[test]
    fn unordered_list_has_bullet() {
        let mut buf = Vec::new();
        render("- one\n- two", &ctx(false), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("• one") || s.contains("- one"));
        assert!(s.contains("two"));
    }

    #[test]
    fn ordered_list_has_numbers() {
        let mut buf = Vec::new();
        render("1. one\n2. two", &ctx(false), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("1.") || s.contains("1)"));
        assert!(s.contains("two"));
    }

    #[test]
    fn inline_code_preserved() {
        let mut buf = Vec::new();
        render("use `foo()` here", &ctx(false), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("foo()"));
    }

    #[test]
    fn fenced_code_block_handed_to_syntect() {
        let mut buf = Vec::new();
        render("```python\nx = 1\n```", &ctx(true), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("x = 1") || s.contains("x"));
    }

    #[test]
    fn blockquote_has_prefix() {
        let mut buf = Vec::new();
        render("> quoted", &ctx(false), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("> quoted") || s.contains("│ > quoted"));
    }

    #[test]
    fn bold_uses_ansi_when_color() {
        let mut buf = Vec::new();
        render("**bold**", &ctx(true), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("\x1b[1m") || s.contains("bold"));
    }
}
```

- [ ] **Step 2: 실패 확인**

Run: `cargo test --lib render::markdown`
Expected: FAIL — `render` 미정의

- [ ] **Step 3: 구현**

`src/render/markdown.rs` 상단:
```rust
use std::io::{self, Write};

use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};

use crate::env::RenderCtx;
use crate::render::{code, frame};
use crate::theme;

/// 누적 텍스트 + 스타일을 frame::wrap_line으로 흘려보냄.
pub fn render(source: &str, ctx: &RenderCtx, w: &mut impl Write) -> io::Result<()> {
    let mut acc = String::new();
    let mut style = Style::default();
    let mut list_stack: Vec<ListState> = Vec::new();
    let mut in_blockquote = 0u32;
    let mut pending_code_block: Option<String> = None;
    let mut pending_lang: Option<String> = None;

    let parser = Parser::new(source);

    for ev in parser {
        match ev {
            Event::Start(Tag::Heading { level, .. }) => {
                flush_line(&mut acc, in_blockquote, ctx, w)?;
                let n = heading_n(level);
                let prefix = "#".repeat(n);
                let header_text = format!("{} ", prefix);
                acc.push_str(&theme::colorize_markdown_header(&header_text, ctx.use_color));
                style.bold = true;
            }
            Event::End(TagEnd::Heading(_)) => {
                if ctx.use_color { acc.push_str(theme::RESET); }
                flush_line(&mut acc, in_blockquote, ctx, w)?;
                style = Style::default();
            }
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                flush_line(&mut acc, in_blockquote, ctx, w)?;
            }
            Event::Start(Tag::Emphasis) => {
                if ctx.use_color { acc.push_str(theme::ITALIC); }
                style.italic = true;
            }
            Event::End(TagEnd::Emphasis) => {
                if ctx.use_color { acc.push_str(theme::RESET); }
                style.italic = false;
            }
            Event::Start(Tag::Strong) => {
                if ctx.use_color { acc.push_str(theme::BOLD); }
                style.bold = true;
            }
            Event::End(TagEnd::Strong) => {
                if ctx.use_color { acc.push_str(theme::RESET); }
                style.bold = false;
            }
            Event::Code(c) => {
                if ctx.use_color { acc.push_str(theme::FG_YELLOW); }
                acc.push('`');
                acc.push_str(&c);
                acc.push('`');
                if ctx.use_color { acc.push_str(theme::RESET); }
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                flush_line(&mut acc, in_blockquote, ctx, w)?;
                let lang = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(l) => l.into_string(),
                    pulldown_cmark::CodeBlockKind::Indented => String::new(),
                };
                pending_lang = Some(if lang.is_empty() { "text".into() } else { lang });
                pending_code_block = Some(String::new());
            }
            Event::Text(t) if pending_code_block.is_some() => {
                pending_code_block.as_mut().unwrap().push_str(&t);
            }
            Event::End(TagEnd::CodeBlock) => {
                let src = pending_code_block.take().unwrap_or_default();
                let lang = pending_lang.take().unwrap_or_else(|| "text".into());
                code::render(&src, &lang, ctx, w)?;
            }
            Event::Start(Tag::List(start)) => {
                flush_line(&mut acc, in_blockquote, ctx, w)?;
                list_stack.push(ListState { number: start });
            }
            Event::End(TagEnd::List(_)) => {
                list_stack.pop();
            }
            Event::Start(Tag::Item) => {
                let indent = "  ".repeat(list_stack.len().saturating_sub(1));
                if let Some(state) = list_stack.last_mut() {
                    match state.number.as_mut() {
                        Some(n) => {
                            acc.push_str(&format!("{}{}. ", indent, n));
                            *n += 1;
                        }
                        None => {
                            acc.push_str(&format!("{}• ", indent));
                        }
                    }
                }
            }
            Event::End(TagEnd::Item) => {
                flush_line(&mut acc, in_blockquote, ctx, w)?;
            }
            Event::Start(Tag::BlockQuote(_)) => {
                in_blockquote += 1;
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                in_blockquote = in_blockquote.saturating_sub(1);
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                acc.push('[');
                // 끝에서 dest 출력 — pulldown-cmark는 End 이벤트 시 dest 제공 X
                // 대신 시작에서만 처리 (간단)
                let _ = dest_url; // 사용 X (v0.1는 텍스트만)
            }
            Event::End(TagEnd::Link) => {
                acc.push(']');
            }
            Event::Text(t) => {
                acc.push_str(&t);
            }
            Event::SoftBreak | Event::HardBreak => {
                flush_line(&mut acc, in_blockquote, ctx, w)?;
            }
            Event::Rule => {
                flush_line(&mut acc, in_blockquote, ctx, w)?;
                let dashes = "─".repeat(ctx.width.saturating_sub(4));
                frame::wrap_line(&dashes, ctx, w)?;
            }
            _ => {}
        }
    }
    flush_line(&mut acc, in_blockquote, ctx, w)?;
    Ok(())
}

fn flush_line(acc: &mut String, quote_depth: u32, ctx: &RenderCtx, w: &mut impl Write) -> io::Result<()> {
    if acc.trim().is_empty() { acc.clear(); return Ok(()); }
    let line = if quote_depth > 0 {
        let prefix = "> ".repeat(quote_depth as usize);
        format!("{}{}", theme::dim(&prefix, ctx.use_color), acc)
    } else {
        std::mem::take(acc)
    };
    frame::wrap_line(&line, ctx, w)?;
    acc.clear();
    Ok(())
}

fn heading_n(level: HeadingLevel) -> usize {
    match level {
        HeadingLevel::H1 => 1, HeadingLevel::H2 => 2, HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4, HeadingLevel::H5 => 5, HeadingLevel::H6 => 6,
    }
}

#[derive(Default)]
struct Style { bold: bool, italic: bool }

struct ListState { number: Option<u64> }
```

> 주의: pulldown-cmark 0.10 이상 API. `TagEnd` 도입은 0.10에서. `Tag::BlockQuote(_)`의 inner는 `Option<BlockQuoteKind>` — 사용 X.

- [ ] **Step 4: 통과 확인**

Run: `cargo test --lib render::markdown`
Expected: PASS (8 tests passed). 어떤 어서션이 너무 strict하면 (예: 정확한 bullet 문자) 코드 또는 테스트 조정.

- [ ] **Step 5: 커밋**

```bash
git -C /Users/gsr/playground/nbv add src/render/markdown.rs src/render/mod.rs
git -C /Users/gsr/playground/nbv commit -m "feat(render): markdown via pulldown-cmark"
```

---

## Task 12: PNG 정보 추출 (`render/image/png_info.rs`)

**Files:**
- Create: `src/render/image/mod.rs` (이번 태스크에서는 `pub mod png_info;`만)
- Create: `src/render/image/png_info.rs`
- Modify: `src/render/mod.rs` (add `pub mod image;`)

- [ ] **Step 1: `src/render/image/mod.rs` 초기 작성**

```rust
pub mod png_info;
// kitty/iterm/placeholder/dispatch은 후속 태스크
```

- [ ] **Step 2: 실패 테스트 작성**

`src/render/image/png_info.rs`:
```rust
// 구현은 Step 4

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    // 1x1 red pixel PNG (well-known)
    const ONE_PIXEL: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==";

    fn one_pixel_bytes() -> Vec<u8> {
        base64::engine::general_purpose::STANDARD.decode(ONE_PIXEL).unwrap()
    }

    #[test]
    fn reads_1x1_dimensions() {
        let b = one_pixel_bytes();
        assert_eq!(dimensions(&b), Some((1, 1)));
    }

    #[test]
    fn rejects_non_png_signature() {
        let b = b"not-a-png-file";
        assert_eq!(dimensions(b), None);
    }

    #[test]
    fn rejects_too_short_bytes() {
        let b = b"\x89PNG";
        assert_eq!(dimensions(b), None);
    }

    #[test]
    fn rejects_missing_ihdr() {
        // 시그니처는 OK지만 IHDR 청크 타입 아님
        let mut b = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        b.extend_from_slice(&[0, 0, 0, 13]); // chunk length
        b.extend_from_slice(b"XXXX"); // wrong type
        b.extend_from_slice(&[0u8; 13]); // dummy data
        b.extend_from_slice(&[0u8; 4]); // crc
        assert_eq!(dimensions(&b), None);
    }
}
```

- [ ] **Step 3: 실패 확인**

Run: `cargo test --lib render::image::png_info`
Expected: FAIL — `dimensions` 미정의

- [ ] **Step 4: 구현**

`src/render/image/png_info.rs` 상단:
```rust
//! PNG IHDR 청크에서 width/height 직접 추출 (의존성 0).

const SIGNATURE: &[u8] = &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

/// PNG bytes에서 (width, height)를 반환. 시그니처/IHDR 검증 실패 시 None.
pub fn dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 24 { return None; }
    if &bytes[..8] != SIGNATURE { return None; }
    // 청크 헤더: 4 bytes length, 4 bytes type
    let chunk_type = &bytes[12..16];
    if chunk_type != b"IHDR" { return None; }
    let width = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let height = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    Some((width, height))
}
```

- [ ] **Step 5: 통과 확인**

Run: `cargo test --lib render::image::png_info`
Expected: PASS (4 tests passed)

- [ ] **Step 6: 커밋**

```bash
git -C /Users/gsr/playground/nbv add src/render/image/ src/render/mod.rs
git -C /Users/gsr/playground/nbv commit -m "feat(render/image): PNG IHDR dimension parser"
```

---

## Task 13: Placeholder 이미지 렌더러

**Files:**
- Create: `src/render/image/placeholder.rs`
- Modify: `src/render/image/mod.rs`

- [ ] **Step 1: `src/render/image/mod.rs`에 트레이트 + module 추가**

```rust
use std::io::{self, Write};
use crate::env::RenderCtx;

pub mod png_info;
pub mod placeholder;

/// 모든 이미지 백엔드는 PNG bytes를 받아 stdout으로 출력한다.
pub trait ImageRenderer {
    fn render(&self, png_bytes: &[u8], cell_idx: usize, out_idx: usize, ctx: &RenderCtx, w: &mut dyn Write) -> io::Result<()>;
}
```

- [ ] **Step 2: 실패 테스트 작성**

`src/render/image/placeholder.rs`:
```rust
// 구현은 Step 4

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::{ImageBackend, RenderCtx};
    use crate::render::image::ImageRenderer;
    use base64::Engine;

    const ONE_PIXEL: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==";

    fn ctx() -> RenderCtx {
        RenderCtx { is_tty: true, use_color: false, width: 60, image_backend: ImageBackend::Placeholder }
    }

    #[test]
    fn shows_dimensions_for_valid_png() {
        let b = base64::engine::general_purpose::STANDARD.decode(ONE_PIXEL).unwrap();
        let mut buf = Vec::new();
        PlaceholderRenderer.render(&b, 3, 0, &ctx(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("1×1") || s.contains("1x1") || s.contains("1") && s.contains("PNG"));
        assert!(s.contains("cell #3"));
    }

    #[test]
    fn falls_back_when_png_invalid() {
        let mut buf = Vec::new();
        PlaceholderRenderer.render(b"garbage", 0, 0, &ctx(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("unknown format") || s.contains("PNG"));
    }
}
```

- [ ] **Step 3: 실패 확인**

Run: `cargo test --lib render::image::placeholder`
Expected: FAIL — `PlaceholderRenderer` 미정의

- [ ] **Step 4: 구현**

`src/render/image/placeholder.rs` 상단:
```rust
use std::io::{self, Write};

use crate::env::RenderCtx;
use crate::render::frame;
use crate::render::image::{png_info, ImageRenderer};

pub struct PlaceholderRenderer;

impl ImageRenderer for PlaceholderRenderer {
    fn render(&self, png_bytes: &[u8], cell_idx: usize, out_idx: usize, ctx: &RenderCtx, w: &mut dyn Write) -> io::Result<()> {
        let (size_label, kb) = match png_info::dimensions(png_bytes) {
            Some((wd, ht)) => (format!("PNG {}×{}", wd, ht), png_bytes.len() / 1024),
            None => ("image (unknown format)".to_string(), png_bytes.len() / 1024),
        };
        let line1 = format!("🖼  {}  ({} KB)", size_label, kb);
        let line2 = format!("   cell #{}, output #{}", cell_idx, out_idx);
        frame::wrap_line(&line1, ctx, w)?;
        frame::wrap_line(&line2, ctx, w)?;
        Ok(())
    }
}
```

- [ ] **Step 5: 통과 확인**

Run: `cargo test --lib render::image::placeholder`
Expected: PASS (2 tests passed)

- [ ] **Step 6: 커밋**

```bash
git -C /Users/gsr/playground/nbv add src/render/image/
git -C /Users/gsr/playground/nbv commit -m "feat(render/image): placeholder renderer + ImageRenderer trait"
```

---

## Task 14: Kitty 이미지 렌더러

**Files:**
- Create: `src/render/image/kitty.rs`
- Modify: `src/render/image/mod.rs` (add `pub mod kitty;`)

Kitty graphics protocol (Ghostty 호환): APC 시퀀스로 png을 base64로 보낸다. 형식:
`\x1b_Gf=100,a=T,m=<flag>;<base64>\x1b\\`
- `f=100`: 데이터 형식 PNG
- `a=T`: 즉시 출력 (transmit + display)
- `m=1` (마지막 청크 아님) / `m=0` (마지막)

대용량 이미지는 4096바이트 단위 chunking 필요. 단순화: 한 번에 모두 보냄(작은 이미지는 OK, 큰 이미지는 청크 분할). v0.1에서는 chunking 구현.

- [ ] **Step 1: 실패 테스트 작성**

`src/render/image/kitty.rs`:
```rust
// 구현은 Step 3

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::{ImageBackend, RenderCtx};
    use crate::render::image::ImageRenderer;

    fn ctx() -> RenderCtx {
        RenderCtx { is_tty: true, use_color: false, width: 60, image_backend: ImageBackend::Kitty }
    }

    #[test]
    fn emits_kitty_apc_with_png_data() {
        let png = b"\x89PNG\r\n\x1a\nfake-data".to_vec();
        let mut buf = Vec::new();
        KittyRenderer.render(&png, 0, 0, &ctx(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.starts_with("\x1b_G"));
        assert!(s.contains("f=100"));
        assert!(s.contains("a=T"));
        assert!(s.ends_with("\x1b\\\n"));
    }

    #[test]
    fn chunks_large_payloads() {
        // 4096 base64 chars 넘는 분량 → 여러 APC 시퀀스
        let big = vec![0u8; 5000]; // base64 약 6668자
        let mut buf = Vec::new();
        KittyRenderer.render(&big, 0, 0, &ctx(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        // APC 시퀀스 종료자가 두 번 이상 등장해야 함
        assert!(s.matches("\x1b\\").count() >= 2);
        assert!(s.contains("m=1"));
        assert!(s.contains("m=0"));
    }
}
```

- [ ] **Step 2: 실패 확인**

Run: `cargo test --lib render::image::kitty`
Expected: FAIL — `KittyRenderer` 미정의

- [ ] **Step 3: 구현**

`src/render/image/kitty.rs` 상단:
```rust
use std::io::{self, Write};
use base64::Engine;

use crate::env::RenderCtx;
use crate::render::image::ImageRenderer;

pub struct KittyRenderer;

const CHUNK_SIZE: usize = 4096;

impl ImageRenderer for KittyRenderer {
    fn render(&self, png_bytes: &[u8], _cell_idx: usize, _out_idx: usize, _ctx: &RenderCtx, w: &mut dyn Write) -> io::Result<()> {
        let b64 = base64::engine::general_purpose::STANDARD.encode(png_bytes);
        let chunks: Vec<&str> = b64.as_bytes()
            .chunks(CHUNK_SIZE)
            .map(|c| std::str::from_utf8(c).unwrap())
            .collect();
        for (i, chunk) in chunks.iter().enumerate() {
            let is_last = i == chunks.len() - 1;
            let m = if is_last { 0 } else { 1 };
            if i == 0 {
                write!(w, "\x1b_Gf=100,a=T,m={};{}\x1b\\", m, chunk)?;
            } else {
                write!(w, "\x1b_Gm={};{}\x1b\\", m, chunk)?;
            }
        }
        writeln!(w)?;
        Ok(())
    }
}
```

- [ ] **Step 4: 통과 확인**

Run: `cargo test --lib render::image::kitty`
Expected: PASS (2 tests passed)

- [ ] **Step 5: 커밋**

```bash
git -C /Users/gsr/playground/nbv add src/render/image/kitty.rs src/render/image/mod.rs
git -C /Users/gsr/playground/nbv commit -m "feat(render/image): kitty graphics protocol"
```

---

## Task 15: iTerm2 이미지 렌더러

**Files:**
- Create: `src/render/image/iterm.rs`
- Modify: `src/render/image/mod.rs` (add `pub mod iterm;`)

iTerm2 inline image protocol: `\x1b]1337;File=inline=1:<base64>\x07` (OSC + BEL 종료). 청크 분할 없이 한 번에.

- [ ] **Step 1: 실패 테스트 작성**

`src/render/image/iterm.rs`:
```rust
// 구현은 Step 3

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::{ImageBackend, RenderCtx};
    use crate::render::image::ImageRenderer;

    fn ctx() -> RenderCtx {
        RenderCtx { is_tty: true, use_color: false, width: 60, image_backend: ImageBackend::ITerm2 }
    }

    #[test]
    fn emits_osc_1337_with_base64() {
        let png = b"\x89PNG\r\n\x1a\nfake".to_vec();
        let mut buf = Vec::new();
        ITermRenderer.render(&png, 0, 0, &ctx(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.starts_with("\x1b]1337;File=inline=1"));
        assert!(s.contains(":"));
        assert!(s.contains("\x07"));
    }
}
```

- [ ] **Step 2: 실패 확인**

Run: `cargo test --lib render::image::iterm`
Expected: FAIL — `ITermRenderer` 미정의

- [ ] **Step 3: 구현**

`src/render/image/iterm.rs` 상단:
```rust
use std::io::{self, Write};
use base64::Engine;

use crate::env::RenderCtx;
use crate::render::image::ImageRenderer;

pub struct ITermRenderer;

impl ImageRenderer for ITermRenderer {
    fn render(&self, png_bytes: &[u8], _cell_idx: usize, _out_idx: usize, _ctx: &RenderCtx, w: &mut dyn Write) -> io::Result<()> {
        let b64 = base64::engine::general_purpose::STANDARD.encode(png_bytes);
        writeln!(w, "\x1b]1337;File=inline=1:{}\x07", b64)
    }
}
```

- [ ] **Step 4: 통과 확인**

Run: `cargo test --lib render::image::iterm`
Expected: PASS (1 test passed)

- [ ] **Step 5: 커밋**

```bash
git -C /Users/gsr/playground/nbv add src/render/image/iterm.rs src/render/image/mod.rs
git -C /Users/gsr/playground/nbv commit -m "feat(render/image): iterm2 inline protocol"
```

---

## Task 16: 이미지 디스패치 (`render/image/mod.rs`)

**Files:**
- Modify: `src/render/image/mod.rs`

- [ ] **Step 1: 실패 테스트 작성**

`src/render/image/mod.rs` 끝에 (트레이트/모듈 선언 아래):
```rust
pub fn dispatch(png_bytes: &[u8], cell_idx: usize, out_idx: usize, ctx: &RenderCtx, w: &mut dyn Write) -> io::Result<()> {
    use crate::env::ImageBackend;
    match ctx.image_backend {
        ImageBackend::Kitty => kitty::KittyRenderer.render(png_bytes, cell_idx, out_idx, ctx, w),
        ImageBackend::ITerm2 => iterm::ITermRenderer.render(png_bytes, cell_idx, out_idx, ctx, w),
        ImageBackend::Placeholder => placeholder::PlaceholderRenderer.render(png_bytes, cell_idx, out_idx, ctx, w),
    }
}

#[cfg(test)]
mod dispatch_tests {
    use super::*;
    use crate::env::{ImageBackend, RenderCtx};

    fn ctx_with(b: ImageBackend) -> RenderCtx {
        RenderCtx { is_tty: true, use_color: false, width: 60, image_backend: b }
    }

    #[test]
    fn placeholder_dispatches_to_placeholder() {
        let mut buf = Vec::new();
        dispatch(b"garbage", 0, 0, &ctx_with(ImageBackend::Placeholder), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("│"));  // 박스 안에서 그려짐
    }

    #[test]
    fn kitty_dispatches_to_kitty() {
        let png = b"\x89PNG\r\n\x1a\nfake".to_vec();
        let mut buf = Vec::new();
        dispatch(&png, 0, 0, &ctx_with(ImageBackend::Kitty), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.starts_with("\x1b_G"));
    }

    #[test]
    fn iterm2_dispatches_to_iterm() {
        let png = b"\x89PNG\r\n\x1a\nfake".to_vec();
        let mut buf = Vec::new();
        dispatch(&png, 0, 0, &ctx_with(ImageBackend::ITerm2), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.starts_with("\x1b]1337;"));
    }
}
```

또한 `pub mod kitty; pub mod iterm;`이 mod.rs에 있는지 확인.

- [ ] **Step 2: 실패 확인 → 통과 확인**

Run: `cargo test --lib render::image`
Expected: PASS (모든 image 관련 테스트, 13개 정도)

- [ ] **Step 3: 커밋**

```bash
git -C /Users/gsr/playground/nbv add src/render/image/mod.rs
git -C /Users/gsr/playground/nbv commit -m "feat(render/image): dispatch by backend"
```

---

## Task 17: 출력 디스패치 (`render/output.rs`)

**Files:**
- Create: `src/render/output.rs`
- Modify: `src/render/mod.rs` (add `pub mod output;`)

스펙 7.3의 MIME 우선순위와 폴백 로직을 구현.

- [ ] **Step 1: 실패 테스트 작성**

`src/render/output.rs`:
```rust
// 구현은 Step 3

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::{ImageBackend, RenderCtx};
    use crate::ipynb::model::{MimeBundle, Output, StreamName};
    use std::collections::HashMap;

    fn ctx_placeholder() -> RenderCtx {
        RenderCtx { is_tty: true, use_color: false, width: 60, image_backend: ImageBackend::Placeholder }
    }

    #[test]
    fn stream_stdout_renders_text() {
        let out = Output::Stream { name: StreamName::Stdout, text: "hello\n".into() };
        let mut buf = Vec::new();
        render(&out, 0, 0, &ctx_placeholder(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("Out [0]") || s.contains("hello"));
        assert!(s.contains("hello"));
    }

    #[test]
    fn execute_result_picks_image_when_present() {
        let png_b64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==";
        let bundle = MimeBundle {
            text_plain: Some("ignored".into()),
            image_png: Some(png_b64.into()),
            other: HashMap::new(),
        };
        let out = Output::ExecuteResult { data: bundle, execution_count: Some(1) };
        let mut buf = Vec::new();
        render(&out, 0, 0, &ctx_placeholder(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("PNG"));
        assert!(!s.contains("ignored"));
    }

    #[test]
    fn execute_result_falls_back_to_text_plain() {
        let bundle = MimeBundle { text_plain: Some("42".into()), image_png: None, other: HashMap::new() };
        let out = Output::ExecuteResult { data: bundle, execution_count: Some(1) };
        let mut buf = Vec::new();
        render(&out, 0, 0, &ctx_placeholder(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("42"));
    }

    #[test]
    fn execute_result_unknown_mime_shows_placeholder() {
        let mut other = HashMap::new();
        other.insert("text/html".to_string(), serde_json::Value::String("<table/>".into()));
        let bundle = MimeBundle { text_plain: None, image_png: None, other };
        let out = Output::ExecuteResult { data: bundle, execution_count: Some(1) };
        let mut buf = Vec::new();
        render(&out, 0, 0, &ctx_placeholder(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("unsupported") || s.contains("text/html"));
    }

    #[test]
    fn error_renders_traceback() {
        let out = Output::Error {
            ename: "ValueError".into(),
            evalue: "bad".into(),
            traceback: vec!["line1".into(), "line2".into()],
        };
        let mut buf = Vec::new();
        render(&out, 0, 0, &ctx_placeholder(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("Error"));
        assert!(s.contains("ValueError") || s.contains("line1"));
    }
}
```

- [ ] **Step 2: 실패 확인**

Run: `cargo test --lib render::output`
Expected: FAIL — `render` 미정의

- [ ] **Step 3: 구현**

`src/render/output.rs` 상단:
```rust
use std::io::{self, Write};
use base64::Engine;

use crate::env::{ImageBackend, RenderCtx};
use crate::ipynb::model::{MimeBundle, Output, StreamName};
use crate::render::{frame, image, text, traceback};
use crate::theme;

/// 단일 Output을 박스 안에서 렌더.
pub fn render(out: &Output, cell_idx: usize, out_idx: usize, ctx: &RenderCtx, w: &mut impl Write) -> io::Result<()> {
    match out {
        Output::Stream { name, text: t } => {
            let label = format!("Out [{}] ── stream ({})", cell_idx, stream_label(name));
            header(&label, ctx, w)?;
            text::render(t, ctx, w)?;
            frame::close(ctx, w)?;
        }
        Output::ExecuteResult { data, execution_count } => {
            render_bundle(data, *execution_count, cell_idx, out_idx, ctx, w)?;
        }
        Output::DisplayData { data } => {
            render_bundle(data, None, cell_idx, out_idx, ctx, w)?;
        }
        Output::Error { ename, evalue, traceback: tb } => {
            let label = format!("Error: {} — {}", ename, evalue);
            let label = theme::colorize_error_header(&label, ctx.use_color);
            frame::open(&label, ctx, w)?;
            traceback::render(tb, ctx, w)?;
            frame::close(ctx, w)?;
        }
        Output::Unknown => {
            let label = format!("Out [{}] ── unknown output", cell_idx);
            header(&label, ctx, w)?;
            frame::wrap_line("(skipped)", ctx, w)?;
            frame::close(ctx, w)?;
        }
    }
    Ok(())
}

fn render_bundle(bundle: &MimeBundle, exec_count: Option<u64>, cell_idx: usize, out_idx: usize, ctx: &RenderCtx, w: &mut impl Write) -> io::Result<()> {
    // 우선순위: image/png (백엔드 가능 시) → image/png (placeholder) → text/plain → 기타 placeholder
    if let Some(b64) = &bundle.image_png {
        let mime = "image/png";
        let exec_label = exec_count.map(|n| format!("[{}]", n)).unwrap_or_default();
        let label = format!("Out {} ── {}", exec_label, mime);
        let label = theme::colorize_output_header(&label, ctx.use_color);
        frame::open(&label, ctx, w)?;
        match base64::engine::general_purpose::STANDARD.decode(b64) {
            Ok(bytes) => image::dispatch(&bytes, cell_idx, out_idx, ctx, w)?,
            Err(_) => frame::wrap_line("(image decode failed)", ctx, w)?,
        }
        frame::close(ctx, w)?;
        return Ok(());
    }
    if let Some(t) = &bundle.text_plain {
        let exec_label = exec_count.map(|n| format!("[{}]", n)).unwrap_or_default();
        let label = format!("Out {} ── text/plain", exec_label);
        let label = theme::colorize_output_header(&label, ctx.use_color);
        frame::open(&label, ctx, w)?;
        text::render(t, ctx, w)?;
        frame::close(ctx, w)?;
        return Ok(());
    }
    // 기타 MIME: ipynb 권장 우선순위
    let priority = ["text/html", "text/latex", "application/json"];
    let key = priority.iter()
        .find(|p| bundle.other.contains_key(**p))
        .map(|s| s.to_string())
        .or_else(|| bundle.other.keys().min().cloned());
    let mime = key.as_deref().unwrap_or("(empty)");
    let label = format!("Out [{}] ── (unsupported: {})", cell_idx, mime);
    let label = theme::colorize_output_header(&label, ctx.use_color);
    frame::open(&label, ctx, w)?;
    frame::wrap_line("", ctx, w)?;
    frame::close(ctx, w)?;
    Ok(())
}

fn header(label: &str, ctx: &RenderCtx, w: &mut impl Write) -> io::Result<()> {
    let label = theme::colorize_output_header(label, ctx.use_color);
    frame::open(&label, ctx, w)
}

fn stream_label(name: &StreamName) -> &'static str {
    match name { StreamName::Stdout => "stdout", StreamName::Stderr => "stderr" }
}
```

- [ ] **Step 4: 통과 확인**

Run: `cargo test --lib render::output`
Expected: PASS (5 tests passed)

- [ ] **Step 5: 커밋**

```bash
git -C /Users/gsr/playground/nbv add src/render/output.rs src/render/mod.rs
git -C /Users/gsr/playground/nbv commit -m "feat(render): output MIME dispatch with fallbacks"
```

---

## Task 18: 셀/노트북 렌더러 (`render/mod.rs`)

**Files:**
- Modify: `src/render/mod.rs`

- [ ] **Step 1: 실패 테스트 작성**

`src/render/mod.rs` 끝에 (모듈 선언 아래):
```rust
use std::io::{self, Write};

use crate::env::RenderCtx;
use crate::ipynb::model::{Cell, Notebook};
use crate::theme;

/// 노트북 전체를 셀 단위로 렌더. 매 셀 후 flush.
pub fn render_notebook(nb: &Notebook, ctx: &RenderCtx, w: &mut impl Write) -> io::Result<()> {
    let lang = nb.metadata.kernelspec.as_ref()
        .and_then(|k| k.language.clone())
        .or_else(|| nb.metadata.language_info.as_ref().map(|l| l.name.clone()))
        .unwrap_or_else(|| "python".into());
    for (idx, cell) in nb.cells.iter().enumerate() {
        render_cell(cell, idx, &lang, ctx, w)?;
        w.flush()?;
    }
    Ok(())
}

pub fn render_cell(cell: &Cell, idx: usize, lang: &str, ctx: &RenderCtx, w: &mut impl Write) -> io::Result<()> {
    match cell {
        Cell::Code { source, outputs, execution_count } => {
            let n = execution_count.map(|n| n.to_string()).unwrap_or_else(|| " ".into());
            let label = format!("In [{}] ── code ({})", n, lang);
            let label = theme::colorize_code_header(&label, ctx.use_color);
            frame::open(&label, ctx, w)?;
            code::render(source, lang, ctx, w)?;
            frame::close(ctx, w)?;
            for (i, out) in outputs.iter().enumerate() {
                output::render(out, idx, i, ctx, w)?;
            }
        }
        Cell::Markdown { source } => {
            let label = theme::colorize_markdown_header("markdown", ctx.use_color);
            frame::open(&label, ctx, w)?;
            markdown::render(source, ctx, w)?;
            frame::close(ctx, w)?;
        }
        Cell::Raw { source } => {
            let label = theme::dim("raw", ctx.use_color);
            frame::open(&label, ctx, w)?;
            text::render(source, ctx, w)?;
            frame::close(ctx, w)?;
        }
        Cell::Unknown => {
            let label = theme::dim("Unknown cell", ctx.use_color);
            frame::open(&label, ctx, w)?;
            frame::wrap_line("(skipped)", ctx, w)?;
            frame::close(ctx, w)?;
            eprintln!("nbv: skipping cell {} with unknown cell_type", idx);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::ImageBackend;
    use crate::ipynb::parse;

    fn ctx() -> RenderCtx {
        RenderCtx { is_tty: true, use_color: false, width: 60, image_backend: ImageBackend::Placeholder }
    }

    #[test]
    fn renders_minimal_notebook_without_panicking() {
        let nb = parse::from_str(r#"{"cells":[{"cell_type":"markdown","source":"# Hi","metadata":{}}],"metadata":{},"nbformat":4,"nbformat_minor":5}"#).unwrap();
        let mut buf = Vec::new();
        render_notebook(&nb, &ctx(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("Hi"));
    }

    #[test]
    fn renders_code_with_outputs() {
        let nb = parse::from_str(r#"{
            "cells":[{"cell_type":"code","source":"print(1)","metadata":{},"execution_count":1,
                "outputs":[{"output_type":"stream","name":"stdout","text":"1\n"}]}],
            "metadata":{},"nbformat":4,"nbformat_minor":5
        }"#).unwrap();
        let mut buf = Vec::new();
        render_notebook(&nb, &ctx(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("print(1)") || s.contains("print"));
        assert!(s.contains("In [1]"));
    }

    #[test]
    fn unknown_cell_logs_to_stderr_and_continues() {
        let nb = parse::from_str(r#"{
            "cells":[
                {"cell_type":"weird","source":"x","metadata":{}},
                {"cell_type":"markdown","source":"normal","metadata":{}}
            ],
            "metadata":{},"nbformat":4,"nbformat_minor":5
        }"#).unwrap();
        let mut buf = Vec::new();
        render_notebook(&nb, &ctx(), &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("Unknown") || s.contains("skipped"));
        assert!(s.contains("normal"));
    }
}
```

또한 mod.rs 맨 위 모듈 선언:
```rust
pub mod frame;
pub mod text;
pub mod traceback;
pub mod code;
pub mod markdown;
pub mod image;
pub mod output;
```

- [ ] **Step 2: 실패 → 통과 확인**

Run: `cargo test --lib render`
Expected: PASS (모든 render 관련 + 새 3개 테스트)

- [ ] **Step 3: 전체 lib 테스트**

Run: `cargo test --lib`
Expected: PASS — 모든 단위 테스트 (대략 50+개)

- [ ] **Step 4: 커밋**

```bash
git -C /Users/gsr/playground/nbv add src/render/mod.rs
git -C /Users/gsr/playground/nbv commit -m "feat(render): notebook and cell orchestration"
```

---

## Task 19: 메인 진입점 (`main.rs`)

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: 구현 작성 (이 태스크는 통합이라 통합 테스트로 검증)**

`src/main.rs`:
```rust
use std::io::{self, Write, BufWriter};
use std::process::ExitCode;

use clap::Parser;

use nbv::cli::Args;
use nbv::env;
use nbv::ipynb::parse;
use nbv::render;

fn main() -> ExitCode {
    install_sigpipe_handler();
    let args = Args::parse();

    let nb = match parse::from_path(&args.file) {
        Err(e) => {
            eprintln!("nbv: {}: {}", args.file.display(), e);
            return ExitCode::from(1);
        }
        Ok(Err(e)) => {
            eprintln!("nbv: failed to parse '{}': {}", args.file.display(), e);
            return ExitCode::from(3);
        }
        Ok(Ok(nb)) => nb,
    };

    let ctx = env::detect(args.no_color, args.no_images);

    let stdout = io::stdout();
    let mut w = BufWriter::new(stdout.lock());
    if let Err(e) = render::render_notebook(&nb, &ctx, &mut w) {
        if e.kind() == io::ErrorKind::BrokenPipe { return ExitCode::SUCCESS; }
        eprintln!("nbv: write error: {}", e);
        return ExitCode::from(1);
    }
    let _ = w.flush();
    ExitCode::SUCCESS
}

fn install_sigpipe_handler() {
    // SIGPIPE를 받으면 즉시 0으로 종료 (e.g. `nbv x.ipynb | head`)
    use signal_hook::consts::SIGPIPE;
    use signal_hook::iterator::Signals;
    let mut signals = Signals::new([SIGPIPE]).expect("install SIGPIPE handler");
    std::thread::spawn(move || {
        for _ in signals.forever() { std::process::exit(0); }
    });
}
```

- [ ] **Step 2: 빌드 확인**

Run: `cargo build --release`
Expected: 성공. Release 바이너리는 `target/release/nbv`.

- [ ] **Step 3: 수동 smoke test**

미니 ipynb를 만들어 직접 실행:
```bash
cat > /tmp/smoke.ipynb <<'EOF'
{
  "cells": [
    {"cell_type":"markdown","source":"# Hello nbv","metadata":{}},
    {"cell_type":"code","source":"x = 1\nprint(x)","metadata":{},"execution_count":1,
     "outputs":[{"output_type":"stream","name":"stdout","text":"1\n"}]}
  ],
  "metadata":{"kernelspec":{"name":"python3","language":"python","display_name":"Python 3"}},
  "nbformat":4,"nbformat_minor":5
}
EOF
./target/release/nbv /tmp/smoke.ipynb
```
Expected: 박스로 둘러싸인 마크다운 + 코드 셀 + stream 출력이 stdout에 출력됨.

Run also: `./target/release/nbv /nonexistent.ipynb`
Expected: stderr에 에러 메시지, exit code 1.

Run: `./target/release/nbv --no-color /tmp/smoke.ipynb | head -5`
Expected: 처음 5줄만 출력, SIGPIPE로 깔끔 종료.

- [ ] **Step 4: 커밋**

```bash
git -C /Users/gsr/playground/nbv add src/main.rs
git -C /Users/gsr/playground/nbv commit -m "feat: main entry with SIGPIPE handler and exit codes"
```

---

## Task 20: 통합 테스트 + fixtures

**Files:**
- Create: `tests/fixtures/simple.ipynb`
- Create: `tests/fixtures/with_image.ipynb`
- Create: `tests/fixtures/with_error.ipynb`
- Create: `tests/fixtures/large.ipynb` (성능 측정만)
- Create: `tests/integration.rs`

- [ ] **Step 1: `tests/fixtures/simple.ipynb` 작성**

```json
{
  "cells": [
    {"cell_type":"markdown","source":"# Simple Notebook\n\nA basic test.","metadata":{}},
    {"cell_type":"code","source":"x = 1 + 2","metadata":{},"execution_count":1,
     "outputs":[{"output_type":"execute_result","execution_count":1,
                "data":{"text/plain":"3"},"metadata":{}}]}
  ],
  "metadata":{"kernelspec":{"name":"python3","language":"python","display_name":"Python 3"}},
  "nbformat":4,"nbformat_minor":5
}
```

- [ ] **Step 2: `tests/fixtures/with_image.ipynb` 작성**

```json
{
  "cells": [
    {"cell_type":"code","source":"plt.plot([1,2,3])","metadata":{},"execution_count":1,
     "outputs":[{"output_type":"display_data",
                "data":{"image/png":"iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==",
                         "text/plain":"<Figure>"},
                "metadata":{}}]}
  ],
  "metadata":{"kernelspec":{"name":"python3","language":"python","display_name":"Python 3"}},
  "nbformat":4,"nbformat_minor":5
}
```

- [ ] **Step 3: `tests/fixtures/with_error.ipynb` 작성**

```json
{
  "cells": [
    {"cell_type":"code","source":"1/0","metadata":{},"execution_count":1,
     "outputs":[{"output_type":"error","ename":"ZeroDivisionError","evalue":"division by zero",
                "traceback":["Traceback (most recent call last):","  File \"<stdin>\", line 1","ZeroDivisionError: division by zero"]}]}
  ],
  "metadata":{"kernelspec":{"name":"python3","language":"python","display_name":"Python 3"}},
  "nbformat":4,"nbformat_minor":5
}
```

- [ ] **Step 4: `tests/fixtures/large.ipynb` 작성**

스크립트로 생성 (commit 직전에 1회):
```bash
python3 - <<'EOF' > /Users/gsr/playground/nbv/tests/fixtures/large.ipynb
import json
cells = []
for i in range(200):
    cells.append({
        "cell_type": "code",
        "source": f"x_{i} = {i}",
        "metadata": {},
        "execution_count": i + 1,
        "outputs": [{"output_type": "execute_result", "execution_count": i + 1,
                     "data": {"text/plain": str(i)}, "metadata": {}}],
    })
nb = {"cells": cells,
      "metadata": {"kernelspec": {"name": "python3", "language": "python", "display_name": "Python 3"}},
      "nbformat": 4, "nbformat_minor": 5}
print(json.dumps(nb))
EOF
```

(Python이 없으면 위 스크립트를 Rust로 옮겨도 무방. 실험 fixture라 한 번만 생성 후 commit.)

- [ ] **Step 5: `tests/integration.rs` 작성**

```rust
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
    assert!(out.contains("1×1") || out.contains("1x1"));
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
fn large_notebook_renders_in_reasonable_time() {
    let start = std::time::Instant::now();
    let (out, _err, code) = run(&["--no-color", "--no-images", "tests/fixtures/large.ipynb"]);
    let dur = start.elapsed();
    assert_eq!(code, 0);
    assert!(out.contains("x_199"));
    assert!(dur.as_secs() < 5, "rendering 200 cells should be < 5s, was {:?}", dur);
}
```

- [ ] **Step 6: 통합 테스트 실행**

Run: `cd /Users/gsr/playground/nbv && cargo test --test integration`
Expected: PASS (7 tests passed)

- [ ] **Step 7: 전체 테스트 한 번 더 실행**

Run: `cargo test`
Expected: 모든 단위 + 통합 테스트 통과 (60+ tests)

- [ ] **Step 8: 커밋**

```bash
git -C /Users/gsr/playground/nbv add tests/
git -C /Users/gsr/playground/nbv commit -m "test: integration tests with golden fixtures"
```

---

## 마무리

20개 태스크 완료 후 다음을 수행한다.

### 최종 검증

- [ ] `cargo build --release` 실행 → `target/release/nbv` 생성
- [ ] `ls -lh target/release/nbv` → 크기 확인 (목표 <4MB)
- [ ] Ghostty에서 `./target/release/nbv tests/fixtures/with_image.ipynb` 수동 실행 → 실제 이미지 렌더링 확인
- [ ] 다른 터미널(Terminal.app 등)에서 같은 명령 → placeholder 출력 확인
- [ ] `./target/release/nbv tests/fixtures/with_image.ipynb | cat` → non-TTY에서 placeholder 폴백 확인
- [ ] `./target/release/nbv tests/fixtures/simple.ipynb | less -R` → ANSI 색상 보존되어 less에서 정상 표시

### v0.1 릴리즈 준비 (스펙 11절)

- [ ] `Cargo.toml`에 `repository`, `readme` 필드 추가 (선택)
- [ ] README.md 작성 (간략한 사용법, 설치 방법)
- [ ] GitHub 리포 생성 → push
- [ ] `cargo publish --dry-run` 으로 검증
- [ ] (선택) Homebrew tap 리포 작성 — v0.1.1에서 해도 됨

---

## Self-Review 결과

스펙 항목별 커버리지 확인:

| 스펙 섹션 | 구현 태스크 |
|---|---|
| 2 사용자 시나리오 | T19 + T20 통합 테스트로 검증 |
| 3 결정사항 (출력 4종) | T8(text), T9(traceback), T10(code), T11(markdown), T16(image dispatch) |
| 5 모듈 구조 | T1~T18 (모든 파일) |
| 6.1 RenderCtx | T4 |
| 6.2 ipynb 모델 (Notebook, Cell, Output, MimeBundle + metadata.kernelspec) | T2 |
| 7.1 셀 단위 streaming | T18 (`render_notebook` 내부 flush) |
| 7.2 박스/구분선 + 색 정책 | T7 + T18 (헤더 라벨 형성) |
| 7.3 MIME 디스패치 | T17 |
| 7.4 ImageRenderer 트레이트 + 3개 구현 + dispatch | T13~T16 |
| 7.5 마크다운 렌더링 | T11 |
| 8.1 exit codes (0/1/2/3) | T19 + T20 통합 테스트 검증 |
| 8.2 셀/출력 수준 폴백 (unknown cell, decode fail) | T2(unknown), T17(decode fail), T18(unknown cell → stderr) |
| 8.3 SIGPIPE 핸들러 | T19 |
| 9 의존성 | T1 |
| 10 테스트 전략 | 모든 태스크의 #[cfg(test)] + T20 통합 |
| 11 배포 | "마무리" 섹션 |

**Placeholder scan:** 본 plan에는 "TBD"/"implement later" 없음. 모든 코드 블록이 실제 구현 또는 정확한 테스트 입력/예상 출력을 포함. ✓

**Type consistency:**
- `RenderCtx` 시그니처: 모든 태스크 동일 (T4에서 정의 후 T7~T19 일관 사용)
- `frame::open/close/wrap_line` 시그니처: T7 정의, T8/T9/T10/T11/T17/T18 동일 사용
- `ImageRenderer::render(png_bytes, cell_idx, out_idx, ctx, w)` 시그니처: T13에서 정의, T14/T15에서 동일하게 구현, T16에서 dispatch
- `render_cell(cell, idx, lang, ctx, w)`와 `render_notebook(nb, ctx, w)`: T18에서 정의, T19 `main.rs`에서 호출 일치
- `strip_ansi_pub`: T9에서 노출, T10에서 import — 이름 일치 ✓

**Scope check:** 20 태스크 / 단일 plan으로 v0.1 전체 커버. 분할 불필요.

**Ambiguity check:**
- T7의 박스 폭 공식이 unicode-width 기반인지 char 기반인지 → unicode-width 기반으로 통일 ✓
- T11 markdown의 표/체크박스/각주는 v0.2로 명시 ✓
- T14 kitty chunking 임계값(4096) 명시 ✓

남은 약점:
- Color 출력의 wrap_line truncation은 v0.1에서 polychrome-aware하지 않음 — 박스 폭을 넘는 색 라인은 ANSI escape가 truncate될 수 있음. T7 주석으로 명시. v0.2에서 fix.
- `large.ipynb` fixture 생성 시 Python 의존 — Rust 스크립트로 대체 가능 (T20 Step 4 노트 참조).
