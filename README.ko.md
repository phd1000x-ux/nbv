# nbv

[English](README.md)

터미널에서 바로 보는 빠른 주피터 노트북 뷰어.

브라우저나 JupyterLab, VS Code를 띄우지 않고 `.ipynb` 파일을 터미널에서 그대로 본다. `cat`/`bat` 같은 흐름을 그대로 — 한 번 호출하면 stdout으로 전체 내용을 쏟아내고 끝.

## 데모

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

Ghostty나 iTerm2에서는 matplotlib/seaborn으로 만든 PNG 출력이 셀 안에 인라인으로 그려진다.

## 주요 기능

- **빠르다.** 셀 단위로 stdout flush — 200셀 노트북도 200ms 이내에 끝나고, 첫 셀은 ms 단위로 보인다.
- **단일 바이너리.** 약 3 MB. 런타임 의존성 없음. 파이썬 안 깔려도 됨.
- **이미지 인라인.** Ghostty와 iTerm2의 그래픽 프로토콜을 네이티브로 지원. 그 외 터미널에서는 PNG 크기를 표시하는 placeholder 박스로 폴백.
- **자동 감지.** TTY 여부, 터미널 종류, 컬러 지원 여부 — 환경변수 설정 없이 그냥 동작한다.
- **파이프 안전.** non-TTY 감지해서 우아하게 폴백. `nbv x.ipynb | less -R` 그대로 됨. `| head`로 끊겨도 `SIGPIPE`를 잡아서 exit 0.
- **시각적 마감.** Unicode box-drawing으로 그린 셀 경계, `syntect` 기반 코드 하이라이팅, 마크다운 헤더/리스트 정렬, 커널이 준 ANSI 색이 살아있는 traceback.

## 설치

**Homebrew (macOS arm64):**

```bash
brew install phd1000x-ux/tap/nbv
```

**Cargo (Rust 1.70 이상이면 어디서나):**

```bash
cargo install nbv
```

**Prebuilt 바이너리 (macOS arm64):**

```bash
curl -L https://github.com/phd1000x-ux/nbv/releases/latest/download/nbv-v0.1.2-aarch64-apple-darwin.tar.gz \
  | tar -xz -C /usr/local/bin
```

**소스에서 빌드:**

```bash
git clone https://github.com/phd1000x-ux/nbv.git
cd nbv
cargo install --path .
```

macOS arm64에서 테스트됨. Linux도 빌드는 되지만 v0.1에서는 우선순위 아님.

`cargo install` 후 `~/.cargo/bin`이 `PATH`에 없다는 경고가 뜨면:

```bash
~/.cargo/bin/nbv setup
```

(아직 `nbv`가 `PATH`에 없으니 풀패스로 호출 — 그게 바로 해결하려는 문제니까.) `setup`은 셸(zsh / bash / fish)을 감지해서 rc 파일에 추가할 한 줄을 보여주고 y/N로 확인받는다. 적용 후엔 현재 터미널에서 즉시 활성화할 수 있는 한 줄도 같이 출력해준다 — zsh/bash는 `export PATH=…`, fish는 `fish_add_path …`. 또는 새 터미널을 열어도 된다. 확인 프롬프트 건너뛰려면 `--yes`.

## 사용법

```bash
nbv analysis.ipynb               # stdout으로 렌더링
nbv --no-color analysis.ipynb    # ANSI 색 끄기
nbv --no-images analysis.ipynb   # 이미지 강제 placeholder
nbv -h                           # 도움말
nbv -V                           # 버전
```

이게 전부. 플래그에 없는 동작은 모두 환경에서 자동 감지된다.

## 무엇을 어떻게 렌더링하나

| ipynb 요소 | v0.1 동작 |
| --- | --- |
| 마크다운 셀 | 헤더(H1~H6), 리스트, 블록인용, 인라인 코드, 코드 펜스(syntect로 하이라이팅), 굵게/기울임, 링크 텍스트 |
| 코드 셀 | 커널 언어로 syntect 하이라이팅 (기본 Python) |
| `text/plain` 출력 | 박스 안 평문 |
| `image/png` 출력 | Ghostty/iTerm2 인라인 (네이티브 프로토콜), 그 외 placeholder 박스 (`🖼 PNG W×H`) |
| `text/html` (DataFrame) | 커널이 같이 넣어주는 `text/plain` 표현으로 폴백 — pandas는 항상 둘 다 emit |
| 에러 / traceback | TTY/색 지원 시 커널 ANSI 색 보존, `--no-color`면 strip |
| `stdout`/`stderr` 스트림 | 라벨 붙은 박스 안에 평문 |
| 알 수 없는 셀/출력 타입 | `(skipped)` placeholder + stderr 경고 한 줄, 렌더링 계속 진행 |

v0.1 미지원: 마크다운 표, 수식(LaTeX), 인터랙티브 위젯, JPEG/SVG 이미지, application/json pretty-print.

## 터미널 지원

| 터미널 | 색 | 이미지 |
| --- | --- | --- |
| Ghostty | ✓ | ✓ (kitty graphics protocol) |
| iTerm2 | ✓ | ✓ (OSC 1337) |
| kitty | ✓ | ✓ |
| Terminal.app | ✓ | placeholder |
| Alacritty | ✓ | placeholder |
| tmux (모든 종류) | ✓ | placeholder (passthrough는 v0.2) |
| 파이프 / non-TTY | (`NO_COLOR` 따름) | placeholder |

감지는 `$TERM_PROGRAM`과 `$TERM`을 보고 자동. `--no-color`, `--no-images`, `NO_COLOR=1`로 강제 가능.

## Exit code

| 코드 | 의미 |
| --- | --- |
| 0 | 정상 |
| 1 | 파일 IO 오류 (없음/권한) |
| 2 | 잘못된 CLI 인자 (clap이 자동 처리) |
| 3 | `.ipynb` 파싱 실패 (JSON 오류 또는 스키마 위반) |

## 개발

```bash
cargo test              # 85 tests (78 unit + 7 integration)
cargo build --release   # target/release/nbv (약 3 MB)
```

설계 스펙: `docs/superpowers/specs/2026-05-12-nbv-design.md`
구현 플랜: `docs/superpowers/plans/2026-05-12-nbv-implementation.md`

## 라이선스

MIT
