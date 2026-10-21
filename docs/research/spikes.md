# 기술 검증 실험 결과 (Phase 0)

- 작성일: 2026-10-22
- 실험 코드: [`spikes/`](../../spikes) (제품 코드 아님, Phase 0 위험 검증 전용)
- 원본 로그: 실험 시점의 `logs/` 디렉터리 (`.gitignore` 대상, 저장소 미포함). 본 문서에 핵심 출력 전사.
- 판정 표기: 통과(실측), 실패(실측), 미검증

## 0. 실험 환경

| 항목 | 값 |
| --- | --- |
| 호스트 OS | Ubuntu 20.04.6 LTS, x86_64, Linux 5.15.0-91, glibc 2.31 |
| Rust | rustc 1.96.0 (2026-05-25), cargo 1.96.0 |
| 핵심 crate | ratatui 0.30.2 (ratatui-core 0.1.2, ratatui-crossterm 0.1.2), crossterm 0.29.0, tokio 1.54.0, rustix 1.1.5 |
| PTY 구동 | tmux 세션 `howtui` (창 `spike`, 100x30 기본), `send-keys`로 키 입력, `capture-pane`으로 화면 수집 |
| 교차 빌드 | cargo-zigbuild 0.22.3, zig 0.17.0 (uv 가상환경의 `ziglang` 패키지) |
| 에뮬레이션 | qemu-user 10.2.3 (`tonistiigi/binfmt` 이미지에서 바이너리 추출, binfmt 미등록 상태로 직접 실행), Debian 11 armhf, arm64 루트 파일시스템 (glibc 2.31) |
| 쉘 | Bash 5.0.17 (호스트), Bash 5.2.15 와 Zsh 5.9 (Debian 12 컨테이너), PowerShell 7.4.2 와 PSReadLine 2.3.5 (`mcr.microsoft.com/powershell` Linux 컨테이너) |

## 1. SPIKE-01 Inline TUI와 stdout 분리

### 1.1 목적

- ratatui `Viewport::Inline` 사용 시 stdout 순수성 유지 가능 여부
- 비동기 요청, 취소, 편집, 확정, 터미널 복구의 기본 동작 확인

### 1.2 사전 발견 사항 (소스 확인)

| 항목 | 내용 | 근거 |
| --- | --- | --- |
| crossterm 커서 위치 질의 | `cursor::position()`이 `\x1B[6n` 질의를 항상 `io::stdout()`에 기록 | crossterm 0.29.0 `src/cursor/sys/unix.rs:32-36` |
| ratatui Inline 초기화 | 뷰포트 계산 시 `backend.get_cursor_position()` 호출 | ratatui-core 0.1.2 `src/terminal/inline.rs:396` |
| ratatui Inline 리사이즈 | 크기 변경 시 `compute_inline_size()`로 커서 위치 재질의 | ratatui-core 0.1.2 `src/terminal/resize.rs:24-37` |
| ratatui CrosstermBackend | `get_cursor_position()`이 `crossterm::cursor::position()` 직접 호출 | ratatui-crossterm 0.1.2 `src/lib.rs:302-306` |
| crossterm 터미널 크기 | `/dev/tty` 우선, 실패 시 stdout fd | crossterm 0.29.0 `src/terminal/sys/unix.rs:61-99` |

결론: 렌더링 대상을 stderr 또는 `/dev/tty`로 바꾸는 것만으로는 부족. 커서 위치 질의가 stdout으로 출력.

### 1.3 방법

- 실험 바이너리 `inline-spike`: 질문 입력, 1.5초 지연 Mock 응답, 명령어 편집, Enter 확정, Esc 요청 취소, Ctrl+C 종료
- 기본 모드: `/dev/tty`를 열어 백엔드 출력 대상으로 사용하고, 동시에 fd 1을 `dup2`로 `/dev/tty`에 임시 전환. 종료 시 fd 1 복구 후 명령어 출력.
- `--naive` 모드: fd 1 전환 없이 `/dev/tty` 백엔드만 사용 (위험 재현용)
- Mock 명령어: `printf '%s\n' "안녕 세계 🌏" \` 와 `  | grep -n '세계'` 2줄 (한글, 이모지, 여러 줄 포함)
- 검증 스크립트: `spikes/inline-spike/check.sh` (실행 ID `spike01_20261022-033718`)

### 1.4 결과

| 케이스 | 실행 형태 | 종료 코드 | stdout | 터미널 모드 (종료 후) | 판정 |
| --- | --- | --- | --- | --- | --- |
| naive_pipe | `inline-spike --naive \| od -c` | 오류 종료 | `\033[6n` 4바이트 유입, stderr에 "The cursor position could not be read within a normal duration" | isig icanon echo 정상 | 위험 재현 (예상대로 실패) |
| pipe | `inline-spike \| od -c` | 0 | 명령어 2줄 + LF 1개, ESC 0건 | 정상 | 통과 |
| capture | `x=$(inline-spike)` | 0 | 바이트 단위 일치 (명령 치환이 끝 LF 제거) | 정상 | 통과 |
| file | `inline-spike > file` | 0 | 58바이트, ESC 0건 | 정상 | 통과 |
| cancel | Ctrl+C | 130 | 0바이트 | 정상 | 통과 |
| esc | 요청 중 Esc, 이후 2초 대기 | 130 (이후 Ctrl+C) | 0바이트, 취소 후 Mock 응답 미반영 | 정상 | 통과 |
| multiline | 질문 "한글 질문 😀", Alt+Enter와 Ctrl+J로 줄 추가 | 0 | 4줄 출력, 추가 줄 포함 | 정상 | 통과 |
| resize | 실행 중 60x30, 100x12, 100x30 순 변경 | 0 | 정상 출력 | 정상 | 통과 |
| panic | 첫 렌더링 후 panic | 101 | 0바이트 | 정상 (panic hook 복구) | 통과 |
| notty | `setsid -w inline-spike </dev/null` | 3 | 0바이트, stderr에 ESC 없는 평문 오류 | 해당 없음 | 통과 |
| stdin_pipe | `echo from-pipe \| inline-spike` | 0 | 정상 출력 (입력 이벤트는 `/dev/tty`에서 수신) | 정상 | 통과 |
| tiny | 40x5 터미널, Inline 높이 7 | 0 | 정상 출력, 뷰포트 5줄로 축소 | 정상 | 통과 |

화면 관찰:

- 확정 후 Inline 영역 삭제, 다음 쉘 프롬프트가 영역 시작 위치에서 이어짐.
- 가로 축소 시 ratatui가 화면 전체를 지우고 뷰포트를 0행으로 이동 (`resize.rs`의 "clear screen on horizontal shrink" 동작). 화면에 보이던 이전 출력이 지워지는 현상 존재. 스크롤백 손실 여부는 터미널별 확인 필요.
- 한글과 이모지 표시 폭 정상 (tmux 기준).

### 1.5 결론

- fd 1 임시 전환 + `/dev/tty` 백엔드 방식으로 Unix stdout 계약 충족 가능 (Linux 실측).
- 미검증: macOS 실기기 (`/dev/tty`와 `dup2` 동작 동일 예상, 실측 필요), Windows 전체 (crossterm Windows 커서 위치는 `CONOUT$` 핸들 기반 WinAPI 사용, `crossterm/src/cursor/sys/windows.rs:38-45`, `crossterm_winapi-0.9.1/src/screen_buffer.rs:35-39`. stdout 리디렉션 상태의 VT 출력 동작 실측 필요).

## 2. SPIKE-02 Linux 교차 빌드와 ARM 실행

### 2.1 방법

- `spikes/inline-spike/xbuild.sh`: `cargo zigbuild --release --target <triple>.2.17` (glibc 2.17 기준 심볼 버전 고정)
- qemu-user로 Debian 11 armhf, arm64 루트 파일시스템에서 TUI 실행, tmux로 질문 입력과 확정

### 2.2 결과

| 타깃 | 빌드 | 파일 형식 | 최대 GLIBC 심볼 | 크기 (spike, release) | qemu 실행 |
| --- | --- | --- | --- | --- | --- |
| armv7-unknown-linux-gnueabihf | 통과 | ELF 32-bit ARM EABI5, `Tag_CPU_arch: v7`, `Tag_FP_arch: VFPv3` | GLIBC_2.17 | 944,188 B | 통과 (출력 바이트 일치) |
| aarch64-unknown-linux-gnu | 통과 | ELF 64-bit ARM aarch64 | GLIBC_2.17 | 998,624 B | 통과 |
| x86_64-unknown-linux-gnu | 통과 | ELF 64-bit x86-64 | GLIBC_2.17 | 1,041,456 B | 호스트 실행 통과 |

추가 발견:

- Ubuntu 20.04 기본 qemu-user 4.2.1에서 `Unsupported ioctl: cmd=0xffffffff802c542a` (termios2 계열 ioctl) 후 `Function not implemented (os error 38)` 실패. qemu 10.2.3에서는 통과. 실기기 커널 문제 아님, 에뮬레이터 버전 요구 사항.
- 결론: CI에서 ARM 바이너리 에뮬레이션 실행 검사 시 최신 qemu 필요.
- 미검증: Raspberry Pi 실기기 (Pi 2 Cortex-A7 포함), Raspberry Pi OS 각 버전, 실기기 터미널 환경.

## 3. SPIKE-03 Bash `bind -x` 입력란 삽입

### 3.1 방법

- `spikes/shell/bind_spike.bash`: `bind -x '"\C-g": 위젯'`, 위젯에서 `out=$(inline-spike --quiet </dev/tty 2>/dev/tty)`, 성공 시 `READLINE_LINE`, `READLINE_POINT` 갱신
- 검증 스크립트: `spikes/shell/check_shells.sh` (실행 ID `spike03_shells_20261022-035131`), Bash 5.2.15 별도 실행 (`spike03_bash52_20261022-035224`)

### 3.2 결과

| 케이스 | Bash 5.0.17 | Bash 5.2.15 |
| --- | --- | --- |
| 기존 입력 "ls -la 기존입력" 상태에서 호출 후 확정 | 입력란이 2줄 명령어로 교체, 실행 없음 (위젯 rc=0, 길이 42) | 동일 |
| 기존 입력 "echo 보존확인" 상태에서 호출 후 Ctrl+C | 입력란 보존 (위젯 rc=130) | 미실행 |
| 빈 입력란에서 호출 후 확정 | 입력란 교체, 실행 없음 | 미실행 |
| TUI 표시 중 프롬프트 줄 | TUI가 프롬프트 줄 위치부터 그려져 프롬프트 줄이 일시적으로 가려짐. 종료 후 readline이 프롬프트와 새 입력란을 다시 그림 | 동일 |

## 4. SPIKE-04 Zsh ZLE 입력란 삽입

- `spikes/shell/zdot/.zshrc`: `zle -N` 위젯, `BUFFER`, `CURSOR` 갱신 후 `zle reset-prompt`, emacs와 viins 키맵 모두 `^G` 등록
- 결과 (Zsh 5.9):

| 케이스 | 결과 |
| --- | --- |
| 기존 입력 상태에서 호출 후 확정 | `BUFFER` 교체, 실행 없음, 프롬프트 줄 유지 상태로 TUI가 다음 줄부터 표시 |
| 호출 후 Ctrl+C | 기존 입력 보존 |
| `bindkey -v` (vi 모드)에서 호출 후 확정 | 교체 성공 |

- 실험 프로그램 보완 사항: 커서가 프롬프트 줄 중간이면 줄바꿈 후 Inline 영역 시작, 종료 시 원래 커서 위치(열 유지, 영역 시작 직전 행)로 복귀. 이 처리로 Zsh 프롬프트 줄 보존 확인.

## 5. SPIKE-05 PowerShell PSReadLine 입력란 삽입

- `spikes/shell/psrl_spike.ps1`: `Set-PSReadLineKeyHandler -Chord 'Ctrl+g'`, `GetBufferState`, 네이티브 실행 결과를 `-join "`n"`으로 결합 후 `Replace(0, $line.Length, $text)`, `InvokePrompt()`
- 결과 (PowerShell 7.4.2, PSReadLine 2.3.5, Linux 컨테이너):

| 케이스 | 결과 |
| --- | --- |
| 기존 입력 "Get-ChildItem 기존입력" 상태에서 호출 후 확정 | 입력란이 2줄 명령어로 교체 (연속 프롬프트 `>>` 표시), 실행 없음 |
| 호출 후 Ctrl+C | 기존 입력 "Write-Output 보존확인" 보존 |

- 한계: Linux 컨테이너 결과는 PSReadLine API 동작 확인에 한정. Windows 콘솔(conhost, Windows Terminal), Windows PowerShell 5.1(기본 탑재 PSReadLine 버전), 한국어 코드 페이지 환경의 출력 인코딩은 미검증.

## 6. 실험 운영 중 확인한 사항

- 이 장비의 tmux는 snap 패키지. 파일로 직접 리디렉션한 stdout에 쓰지 못함 (`tmux capture-pane -p > file` 결과 0바이트). 파이프 경유(`| cat > file`)로 해결.
- 대화형 실험 쉘은 기본적으로 `~/.bash_history`에 기록. 실험용 rc 파일에 `unset HISTFILE` 적용.
- 쉘 입력란 정리용 키: 여러 줄 입력란은 Ctrl+U로 일부만 삭제되는 경우 존재 (Zsh). Ctrl+C로 입력 줄 전체 폐기 권장. PSReadLine Linux 기본(Emacs 모드)에서 Esc는 접두 키로 동작.

## 7. 명세 반영 위치

| 실험 결과 | 반영 문서 |
| --- | --- |
| fd 1 임시 전환과 `/dev/tty` 렌더링 | [SRS](../SRS.md) 6절, [ADR-0002](../adr/0002-tui-output-to-controlling-terminal.md) |
| stdout 계약, 종료 코드 130과 3 | [PRD](../PRD.md) PRD-OUT 계열, [ADR-0003](../adr/0003-stdout-contract-and-exit-codes.md) |
| Alt+Enter, Ctrl+J 줄바꿈 | [PRD](../PRD.md) 키 바인딩 표 |
| 프롬프트 줄 보존 처리 | [SRS](../SRS.md) 6절, PRD-SH 계열 |
| glibc 2.17 교차 빌드, qemu 버전 요구 | [SRS](../SRS.md) 13절, [ADR-0010](../adr/0010-linux-gnu-targets-with-glibc-floor.md) |
| 쉘별 버퍼 삽입 API | [SRS](../SRS.md) 9절, [ADR-0009](../adr/0009-shell-integration-protocol.md) |
