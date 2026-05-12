# nbv — 터미널 주피터 노트북 뷰어 디자인

- 상태: Draft v1
- 작성일: 2026-05-12
- 작성자: gsr (with Claude)

## 1. 목적과 비-목적

### 목적

`.ipynb` 파일을 브라우저나 IDE 없이 macOS 터미널에서 즉시 렌더링한다.
`cat` / `bat` 같은 흐름으로 한 번 호출하면 stdout으로 전체 내용을 쏟아내고 종료한다.

### 비-목적 (v0.1)

- 노트북 **편집**
- 셀 **실행** (Jupyter 커널 연결)
- **인터랙티브 TUI 모드** — 추후 `-i` 플래그로 별도 모드. v0.1 아님
- **자동 페이저** — 자체 페이저 X, `less` 자동 호출 X. 사용자가 직접 `nbv x.ipynb | less -R`
- **검색 / 셀 선택** (v0.2)
- **위젯**, **PDF**, **오디오/비디오 출력** (v0.2 또는 보류)

## 2. 사용자 시나리오

```
$ ls
analysis.ipynb  data.csv
$ nbv analysis.ipynb
┌─ In [1] ── code (python) ──────────────────────┐
│ import pandas as pd                            │
│ df = pd.read_csv("data.csv")                   │
└────────────────────────────────────────────────┘
... (셀 단위로 즉시 flush) ...
$ _
```

장점: 브라우저/IDE 띄울 필요 없음, 첫 셀이 ms 내에 보임, 다음 명령을 끊김 없이 이어 침.

## 3. 결정사항 요약

| 결정 | 선택 | 근거 |
|---|---|---|
| 출력 타입 스코프 (v0.1) | code, markdown, text/plain (stream + execute_result), image/png, traceback | 데이터/실험 노트북 95% 커버. text/html · text/latex · application/json은 text/plain 폴백 |
| 이미지 폴백 전략 | 환경 자동 감지 → 3단 | Ghostty/iTerm2 TTY → 네이티브 프로토콜; 그 외 TTY → placeholder 박스; non-TTY → 동일 placeholder |
| 페이저 동작 | 자동 페이저 사용 안 함 | Ghostty/iTerm2 이미지 프로토콜은 `less`에서 보존되지 않음. 자동 호출하면 이미지가 깨짐. 사용자가 필요 시 직접 파이프 |
| CLI 인터페이스 | 최소 세트: `nbv [FILE]` + `-h/-V/--no-color/--no-images` | 환경 자동 감지가 우선. PRD의 "끊김 없는 vibe coding" 가치 |
| 아키텍처 | 직접 렌더링 + 셀 단위 stdout flush | ratatui는 인터랙티브용이라 과함. crossterm은 감지 용도만 |
| PNG 차원 추출 | IHDR 직접 파싱 (의존성 0) | 본문 디코드 불필요. base64 원본만 프로토콜로 전송 |
| 마크다운 처리 | `pulldown-cmark` 파서 + 직접 ANSI 렌더링 | 자체 박스/팔레트와 통합 가능, 헤더 H1~H6 차별화 통제권 확보 |
| CI | v0.1에는 없음 | 로컬 `cargo test` 그린만 통과. v0.2부터 GitHub Actions |

## 4. 아키텍처

```
                                        ┌─────────────────────────┐
$ nbv x.ipynb  ──►  main.rs             │  for cell in cells {    │
                  │                     │     render(cell, w);    │
                  ├─► cli::parse()      │     w.flush();          │
                  │                     │  }                      │
                  ├─► env::detect() ────┤                         │
                  │   (TTY, $TERM_PROGRAM,                        │
                  │    $NO_COLOR, $COLUMNS)                       │
                  │                     │                         │
                  ├─► ipynb::parse() ───┤                         │
                  │   (serde_json)      │                         │
                  │                     │                         │
                  └─► render loop ──────┘
```

단일 바이너리. 단방향 파이프라인. 셀 하나 그릴 때마다 `stdout.flush()` → 첫 셀이 즉시 보인다.

## 5. 모듈 구조

```
nbv/
├── Cargo.toml
├── src/
│   ├── main.rs            # 진입점, exit codes
│   ├── cli.rs             # clap Args 구조체
│   ├── env.rs             # 환경/터미널 감지 → RenderCtx
│   ├── ipynb/
│   │   ├── mod.rs
│   │   ├── model.rs       # Notebook, Cell, Output (serde derive)
│   │   └── parse.rs       # from_path() / from_reader()
│   ├── render/
│   │   ├── mod.rs         # render_notebook(), render_cell()
│   │   ├── frame.rs       # 박스/구분선 그리기
│   │   ├── code.rs        # syntect 래퍼
│   │   ├── markdown.rs    # pulldown-cmark → ANSI
│   │   ├── output.rs      # MIME 디스패치
│   │   ├── text.rs        # text/plain + stream
│   │   ├── traceback.rs   # ANSI escape passthrough
│   │   └── image/
│   │       ├── mod.rs     # ImageRenderer 트레이트
│   │       ├── kitty.rs   # Ghostty/kitty graphics
│   │       ├── iterm.rs   # iTerm2 inline images
│   │       ├── placeholder.rs
│   │       └── png_info.rs  # IHDR 직접 파싱
│   └── theme.rs           # dark-friendly 단일 팔레트
└── tests/
    ├── fixtures/*.ipynb
    └── snapshot/*.txt
```

원칙: 각 파일은 한 가지 책임만. 새 출력 타입 추가 = 새 파일 추가. `render/` 내부 파일은 서로 모름 (`output.rs`가 MIME 보고 dispatch).

## 6. 데이터 모델

### 6.1 `RenderCtx` (전역, 1회 계산)

```rust
pub struct RenderCtx {
    pub is_tty: bool,
    pub use_color: bool,
    pub width: usize,                 // 최소 80 fallback
    pub image_backend: ImageBackend,
}

pub enum ImageBackend { Kitty, ITerm2, Placeholder }
```

`image_backend` 결정 순서:
1. `--no-images` OR `!is_tty` → `Placeholder`
2. `$TERM_PROGRAM == "ghostty"` OR `$TERM == "xterm-kitty"` → `Kitty`
3. `$TERM_PROGRAM == "iTerm.app"` → `ITerm2`
4. else → `Placeholder`

`tmux` 내부는 보수적으로 `Placeholder` (tmux passthrough는 v0.2).

### 6.2 ipynb 모델 (nbformat v4 스키마 부분 집합)

```rust
pub struct Notebook {
    pub cells: Vec<Cell>,
    pub metadata: NotebookMetadata,
    // nbformat, nbformat_minor 등은 무시
}

pub struct NotebookMetadata {
    pub kernelspec: Option<KernelSpec>,        // 코드 셀 syntax 추측용
    pub language_info: Option<LanguageInfo>,   // kernelspec 없을 때 폴백
}

pub enum Cell {
    Code { source: String, outputs: Vec<Output>, execution_count: Option<u64> },
    Markdown { source: String },
    Raw { source: String },
    Unknown,                                   // forward-compat 폴백
}

pub enum Output {
    Stream { name: StreamName, text: String },     // name: stdout | stderr
    ExecuteResult { data: MimeBundle, execution_count: Option<u64> },
    DisplayData { data: MimeBundle },
    Error { ename: String, evalue: String, traceback: Vec<String> },
}

pub struct MimeBundle {
    pub text_plain: Option<String>,
    pub image_png: Option<String>,             // base64
    pub other: HashMap<String, serde_json::Value>,  // 폴백용 보존
}
```

`source` 필드는 ipynb 스펙상 `string | Vec<string>` 모두 합법 — serde의 untagged enum으로 처리해 항상 `String`으로 정규화한다.

**코드 셀 언어 결정 순서** (syntect 신택스 선택용):
1. `metadata.kernelspec.language` (예: `"python"`)
2. `metadata.language_info.name`
3. 둘 다 없으면 `"python"` 폴백 (95% 노트북이 파이썬)

셀별 metadata는 v0.1에서 무시.

## 7. 렌더링 흐름

### 7.1 셀 단위 streaming

```rust
pub fn render_notebook(nb: &Notebook, ctx: &RenderCtx, w: &mut impl Write) -> io::Result<()> {
    for (idx, cell) in nb.cells.iter().enumerate() {
        render_cell(cell, idx, ctx, w)?;
        w.flush()?;
    }
    Ok(())
}
```

JSON 파싱은 한 번에 끝나지만(serde_json은 100 MB도 sub-second), 렌더링은 셀 단위로 flush → 사용자 체감 latency 최소화.

### 7.2 박스 / 구분선 (`render/frame.rs`)

```
┌─ In [3] ── code (python) ──────────────────────┐
│ import pandas as pd                            │
│ df = pd.read_csv("data.csv")                   │
└────────────────────────────────────────────────┘
┌─ Out [3] ── text/plain ────────────────────────┐
│    idx  name    score                          │
│ 0    0  Alice    91.2                          │
└────────────────────────────────────────────────┘
```

- 폭은 `ctx.width`에 맞춤
- 셀 타입별 헤더 라벨: `In [n] ── code (<lang>)`, `markdown`, `raw`, `Out [n] ── <mime>`, `Error`, `Unknown cell`
- 박스 문자는 Unicode box-drawing 고정 (`use_color` 와 무관). `less`/`tmux` 모두 Unicode 통과
- `use_color=true`: 헤더에 ANSI dim/bold + 셀 타입별 색조 (theme.rs)
- `use_color=false`: ANSI escape 없이 텍스트만. 박스는 그대로

### 7.3 MIME 디스패치 (`render/output.rs`)

`Output::ExecuteResult` 또는 `DisplayData`의 `MimeBundle`에서 우선순위로 선택:

1. `image/png` 존재 AND `ctx.image_backend != Placeholder` → 이미지 렌더링
2. `image/png` 존재 AND `Placeholder` → placeholder 박스 (`png_info`로 W×H 추출)
3. `text/plain` 존재 → text 렌더링
4. 그 외 → `MimeBundle.other`의 키들을 ipynb 스펙 권장 우선순위(`text/html`, `text/latex`, `application/json`, 나머지는 알파벳 순)로 정렬해 첫 MIME 이름만 `(unsupported: <mime>)` placeholder로 표시

`Stream` 출력은 즉시 text 렌더. `Error`는 traceback 렌더.

### 7.4 이미지 백엔드 트레이트

```rust
trait ImageRenderer {
    fn render(&self, png_bytes: &[u8], ctx: &RenderCtx, w: &mut dyn Write) -> io::Result<()>;
}
```

- `KittyRenderer`: APC `_Gf=100,a=T;<base64-png>` ST
- `ITermRenderer`: OSC 1337 `File=inline=1:<base64>` BEL
- `PlaceholderRenderer`: `🖼  PNG WxH  (N KB)  cell #i, output #j` 박스
  - W×H는 `png_info::dimensions(&bytes)` — PNG 시그니처 검증 + IHDR 청크 첫 8바이트(width/height BE u32) 직접 파싱, ~30 LoC, 의존성 0
  - 시그니처 불일치 또는 너무 짧으면 `None` 반환 → "(image: unknown format)"

### 7.5 마크다운 렌더링 (`render/markdown.rs`)

`pulldown-cmark`를 파서로만 사용. 이벤트 스트림을 받아 직접 ANSI로 변환:

- `Heading(level)`: `# ` prefix × level + bold + 색. H1~H6 차별
- `Strong` / `Emphasis`: ANSI bold / italic
- `Code` (inline `\`...\``): syntect inline (언어 추측 없음, mono 색)
- `CodeBlock(fence_lang)`: syntect로 위임 (`render/code.rs` 재사용)
- `List(ordered)`: 들여쓰기 + `•` 또는 `1.`
- `Link`: 텍스트만 표시 + 옵션으로 (URL)
- `BlockQuote`: `> ` prefix + dim
- 표 / 체크박스 / 각주: v0.2 (v0.1은 plain text로 폴백)

대략 ~150 LoC. 자체 박스(`frame.rs`)와 폭/팔레트 일치.

## 8. 에러 처리

### 8.1 exit codes

| 코드 | 상황 | stderr |
|---|---|---|
| 0 | 정상 | — |
| 1 | 파일 IO 오류 (파일 없음, 권한 없음) | `nbv: 'x.ipynb': No such file or directory` |
| 2 | CLI 인자 오류 (잘못된 플래그) | clap default 메시지 |
| 3 | 파싱 실패 (JSON 불량, ipynb 스키마 위반) | `nbv: failed to parse 'x.ipynb': <serde 에러>` |

clap의 derive 매크로는 인자 오류 시 자동으로 exit code 2를 반환한다. 그 컨벤션을 유지하고 파싱 실패는 별도 코드 3을 쓴다.

### 8.2 셀/출력 수준 폴백

빠른 실패 X, 부분 출력 후 진행 O — PRD의 "끊김 없는" 가치와 일치.

- **알 수 없는 cell type** (`raw` 외): stderr 경고 한 줄 + raw text 폴백
- **알 수 없는 MIME**: `text/plain` 폴백 → 없으면 `(unsupported: <mime>)` placeholder
- **base64 디코드 실패**: `(image decode failed)` placeholder
- **PNG 시그니처 불일치**: `(image: unknown format)` placeholder
- **이미지 백엔드 write 실패**: 무시하고 다음 셀로 (broken pipe 등)

### 8.3 SIGPIPE

`signal-hook`으로 SIGPIPE 핸들러 설정. `nbv x.ipynb | head` 같은 경우 조용히 0으로 종료.

## 9. 의존성

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
syntect = { version = "5", default-features = false, features = ["default-fancy"] }
pulldown-cmark = { version = "0.10", default-features = false }
crossterm = "0.27"          # 터미널 크기/색 능력 감지 용도만
base64 = "0.22"
anyhow = "1"
thiserror = "1"
signal-hook = "0.3"

[profile.release]
lto = "fat"
codegen-units = 1
strip = true
```

**도입 안 함**: `ratatui` (인터랙티브용), `viuer` (직접 인코딩이 더 가볍고 정확), `bat-lib`, `tokio` (단방향 stdout에 비동기 불필요), `image` (디코딩 불필요), `termimad` (이중 박스/팔레트 충돌).

**릴리즈 바이너리 목표**: macOS arm64 strip 후 < 4 MB.

## 10. 테스트 전략 (v0.1)

- **단위 테스트**: 각 렌더러 (`render/code.rs`, `markdown.rs`, `image/kitty.rs` 등) — 입력 → 출력 바이트 검증
- **golden file 테스트**: `tests/fixtures/*.ipynb` ↔ `tests/snapshot/*.txt`. `insta` 크레이트로 차이 발견 시 명시적 승인
- **환경 감지 테스트**: `env::detect()` 시그니처를 trait로 분리 → env var를 mock해 4가지 백엔드 분기 모두 검증
- **이미지 인코딩 테스트**: kitty/iTerm2 출력 바이트가 프로토콜 형식과 일치하는지 (시작/종료 시퀀스 + base64 본문)
- **CI 없음**: 로컬 `cargo test` 그린만 통과 후 출시. v0.2에서 GitHub Actions 추가

샘플 fixtures (v0.1 필수):
- `simple.ipynb` — 셀 3개, 코드+마크다운+text/plain
- `with_image.ipynb` — matplotlib PNG 1장
- `with_error.ipynb` — traceback 셀
- `large.ipynb` — 셀 200개, 성능 회귀 감지 (시간 측정만, 골든 비교 X)

## 11. 배포 (v0.1)

- **Cargo**: `cargo publish` → `cargo install nbv` 가능
- **Homebrew tap**: `gsr/homebrew-nbv` 별도 리포에 Formula
  - `brew install gsr/nbv/nbv` 한 줄로 설치
  - 사전 빌드된 macOS arm64 바이너리 GitHub Releases에서 다운로드
- **CI/자동화**: v0.1은 수동 `cargo build --release` + 수동 tarball 업로드. v0.2부터 GitHub Actions로 자동 릴리즈

대상 환경: macOS (Apple Silicon 우선). Linux는 동작하겠지만 우선순위 아님.

## 12. v0.2 이후 아이디어 (참고용, 본 스펙 범위 아님)

- 인터랙티브 모드 `-i` (ratatui 기반 스크롤/검색/셀 점프)
- `--cells=1,3,5-7`, `--code-only`, `--no-markdown` 등 필터 플래그
- DataFrame `text/html` 전용 파서 (Rich-style 표)
- LaTeX → unicode 근사
- tmux passthrough 감지
- 마크다운 표/체크박스/각주
- JPEG/GIF/SVG 이미지 지원
- 환경변수 `NBV_*` 추가
- Linux 빌드 & 멀티아키 릴리즈
