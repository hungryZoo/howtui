# howtui 셸 통합 메커니즘 조사

- 조사일: 2026-10-22
- 조사 목적: 단축키 또는 `howtui` 실행 후 인라인 TUI에서 확정한 명령의 원래 셸 입력 버퍼 반영(실행 없음) 방식 결정
- 대상 조합: Linux와 Bash, macOS와 Zsh, Windows와 PowerShell(Windows PowerShell 5.1, PowerShell 7.x)
- 근거 표기: `확인(문서 URL)`, `확인(소스 경로)`, `확인(로컬 실측)`, `미확인`. 소스 경로 앞 저장소 약칭은 1.2절 표 기준.
- 코드 블록: 전부 실측 검증 전 스케치

---

## 0. 핵심 결론

| 항목 | Bash (Linux) | Zsh (macOS) | PowerShell (Windows) |
|---|---|---|---|
| 버퍼 접근 메커니즘 | `bind -x` + `READLINE_LINE`, `READLINE_POINT` | ZLE 사용자 위젯(`zle -N`) + `BUFFER`, `LBUFFER`, `CURSOR` | `Set-PSReadLineKeyHandler -ScriptBlock` + `GetBufferState`, `Replace`, `InvokePrompt` |
| 최소 버전 | 4.0(변수 도입), 권장 5.0 이상(문자 단위 커서), 5.1 이상(키맵별 바인딩) | `reset-prompt` 기준 4.2 계열, bracketed paste 기준 5.1 | PSReadLine 2.0.0(Windows PowerShell 5.1 탑재본) |
| 핸들러 안 TUI 입출력 | 셸 fd 상속, `</dev/tty >/dev/tty 2>/dev/tty` 명시 권장 | 표준 입력 /dev/null 고정, `</dev/tty` 필수 | `System.Diagnostics.Process` 리다이렉션 없는 실행, 콘솔 상속 |
| 결과 전달 | 임시 파일 | 임시 파일 | 임시 파일(UTF-8 명시 읽기) |
| 입력 전달 | 환경 변수 | 환경 변수 | 환경 변수 |
| 화면 복구 | bash 자동 재그림(4.4 이상 마지막 프롬프트 줄) | `zle reset-prompt` | `InvokePrompt($null, $y)` |
| 기본 키 권고 | Ctrl+G (`\C-g`) | Ctrl+G (`^G`) | Ctrl+G (`Ctrl+g`) |

핵심 근거 요약 (세부 근거는 각 절)

- ratatui `Viewport::Inline` 초기화 시 커서 위치 질의 필수, crossterm Unix 구현의 질의 문자열(`ESC[6n`) 기록 대상은 프로세스 stdout. stdout을 `$(...)`로 캡처하면 인라인 뷰포트 초기화 실패 위험. 확인(소스 crossterm/src/cursor/sys/unix.rs, ratatui/ratatui-core/src/terminal/inline.rs). 따라서 위젯 모드의 결과 전달 수단은 stdout 캡처 대신 임시 파일 권장.
- PowerShell 네이티브 출력 캡처의 디코딩 기준은 `[Console]::OutputEncoding`, PSReadLine은 ScriptBlock 핸들러 실행 동안 이 값을 사용자 원래 값으로 복원. 한국어 Windows에서 UTF-8 stdout 캡처 시 한글 깨짐 위험. 확인(문서 about_Preference_Variables 7.7, 소스 PSReadLine/PSReadLine/ReadLine.cs CallPossibleExternalApplication). 따라서 PowerShell도 임시 파일과 UTF-8 명시 읽기 권장(atuin 방식).
- Ctrl+G 충돌 지점: bash emacs `abort`, zsh emacs `send-break`, PSReadLine Emacs와 Vi 모드 `Abort`, VS Code 통합 터미널(Windows, Linux)의 "Go to Recent Directory" 기본 가로챔. 확인(소스 각 키맵 파일, microsoft/vscode terminal.history.contribution.ts). `--key` 옵션과 VS Code 설정 안내 필수.

---

## 1. 조사 범위와 방법

### 1.1 방법

- 공식 문서: GNU Bash 매뉴얼(texinfo 원문 bash/doc/bashref.texi 열람), zsh 매뉴얼 zshzle, Microsoft Learn(PSReadLine, about_* 문서), Windows Console 문서, VS Code 문서, Apple 지원 문서, iTerm2 문서, tmux CHANGES
- 소스: 아래 저장소 shallow clone 후 읽기 전용 열람, 클론 코드 실행 없음
- 클론 위치: `<로컬 임시 디렉터리>/clones/shellint/`
- 로컬 실측: bash 5.0.17에서 `BASH_VERSION` 환경 변수 노출 여부 1건

### 1.2 저장소 약칭

| 약칭 | 저장소 | 커밋 | 커밋 날짜 | 비고 |
|---|---|---|---|---|
| bash | https://git.savannah.gnu.org/git/bash.git (master) | 9c465866b7d8 | 2026-09-15 | 태그 `bash-4.0`, `bash-4.4` 추가 열람, doc/version.texi 기준 5.3 |
| zsh | https://github.com/zsh-users/zsh (미러) | fc3cff4f8e28 | 2026-10-10 | Config/version.mk 5.9.999.3-test |
| fzf | https://github.com/junegunn/fzf | ac28b8c3ba27 | 2026-10-21 | |
| atuin | https://github.com/atuinsh/atuin | 92a8b9ae1d0b | 2026-10-21 | crates/atuin-ai 포함 |
| zoxide | https://github.com/ajeetdsouza/zoxide | d48a7ab5d7cb | 2026-10-18 | |
| PSFzf | https://github.com/kelleyma49/PSFzf | 7e9871df7b58 | 2026-09-20 | |
| PSReadLine | https://github.com/PowerShell/PSReadLine | a2d143f78904 | 2026-10-19 | 태그 `v2.0.0` 추가 열람, master 매니페스트 3.0.0 |
| starship | https://github.com/starship/starship | ae8fecf316ed | 2026-10-20 | |
| bash-preexec | https://github.com/rcaloras/bash-preexec | 5ae4758c36e8 | 2026-08-23 | |
| crossterm | https://github.com/crossterm-rs/crossterm | db287b8c6873 | 2026-10-19 | Cargo.toml 0.29.0, CHANGELOG에 Unreleased 항목 존재 |
| ratatui | https://github.com/ratatui/ratatui | f57b06ffb720 | 2026-10-20 | ratatui 0.30.2 |
| crossterm_winapi | 로컬 cargo 레지스트리 사본 0.9.1 | - | - | `$HOME/.cargo/registry/src/*/crossterm_winapi-0.9.1` |

단일 파일 조회(GitHub API, 2026-10-22, 기본 브랜치)

| 약칭 | 파일 |
|---|---|
| pwsh-src | PowerShell/PowerShell `src/System.Management.Automation/engine/NativeCommandProcessor.cs` (해당 파일 최종 커밋 34f6e267, 2026-02-03) |
| rust-std | rust-lang/rust `library/std/src/sys/stdio/windows.rs` |
| vscode | microsoft/vscode `src/vs/workbench/contrib/terminal/common/terminal.ts`, `src/vs/workbench/contrib/terminal/terminalContribExports.ts`, `src/vs/workbench/contrib/terminalContrib/history/common/terminal.history.ts`, `.../history/browser/terminal.history.contribution.ts`(최종 커밋 a95abce2, 2026-02-06) |
| wt | microsoft/terminal `src/cascadia/TerminalSettingsModel/defaults.json` |
| navi | denisidoro/navi `shell/navi.plugin.bash`, `shell/navi.plugin.zsh`, `shell/navi.plugin.ps1` |
| aichat | sigoden/aichat `scripts/shell-integration/integration.bash`, `integration.zsh`, `integration.ps1` |

---

## 2. Bash 통합

### 2.1 bind -x와 READLINE 변수 버전 이력

| bash 버전 | 변경 | 근거 |
|---|---|---|
| 2.04 | `bind -x` 도입 | 확인(소스 bash/NEWS, bash-2.04 항목 c) |
| 4.0 | `READLINE_LINE`, `READLINE_POINT` 도입. NEWS 표기는 `READLINE_LINE_BUFFER`, 실제 변수명은 `READLINE_LINE` | 확인(소스 bash/NEWS bash-4.0 항목 bb, 태그 bash-4.0 bashline.c bash_execute_unix_command) |
| 4.0 | 핸들러 실행 전 `rl_crlf()`로 새 줄 이동, 실행 후 `rl_forced_update_display()` 전체 재그림, `READLINE_POINT`는 `rl_point` 그대로(바이트 오프셋) | 확인(소스 태그 bash-4.0 bashline.c). 4.1-4.3 동일 여부 미확인 |
| 4.4 | 실행 전 현재 입력 표시 줄 삭제(`rl_clear_visible_line`, 터미널 `ce` 기능 존재 시), 실행 후 다중 줄 프롬프트의 마지막 줄만 재그림, 핸들러 반환값 124는 전체 프롬프트 재그림 요청 | 확인(소스 bash/CHANGES bash-4.4-alpha 항목 ggg, 태그 bash-4.4 bashline.c) |
| 5.0 | `READLINE_POINT` 단위 변경(바이트에서 문자 수) | 확인(소스 bash/CHANGES bash-5.0-alpha 항목 xxxx) |
| 5.1 | `READLINE_MARK` 도입, 편집 모드와 키맵별 개별 `bind -x` 지원, readline bracketed paste 기본 활성 | 확인(소스 bash/NEWS bash-5.1 항목 v, a, readline 항목 h) |
| 5.2 | `READLINE_ARGUMENT`(숫자 인자) 도입 | 확인(소스 bash/NEWS bash-5.2 항목 n) |
| 5.3 | 공백 구분 신규 `bind -x` 문법(명령 문자열 큰따옴표 필수), `bind -x` 실행 중 창 크기 변경 검사, `bind -x` 실행 중 자식 종료 후 tty 상태 재설정 생략 | 확인(소스 bash/NEWS bash-5.3 항목 aa, o, bash/CHANGES bash-5.3-beta 항목 i) |

보충 사항

- bash 5.0 이하의 `bind -x` 명령 표: emacs, vi-insert, vi-command 키맵 전체 공유, 키맵별 다른 명령 지정 불가. 확인(소스 atuin crates/atuin/src/shell/atuin.bash `__atuin_bind_impl` 주석). bash 5.1 NEWS 항목 a와 일치.
- bash 5.1 결함: 기존 `bind -x` 바인딩 덮어쓰기 시 다른 `bind -x` 바인딩 손상, 선행 `bind -r` 회피. 확인(소스 atuin atuin.bash 주석, 인용 링크 https://lists.gnu.org/archive/html/bug-bash/2021-04/msg00135.html). 결함 재현 미확인.
- bash 4.2 이하: 2바이트 초과 키 시퀀스의 `bind -x` 불가. 확인(소스 atuin atuin.bash 주석). bash 소스 직접 확인 미확인. Ctrl+G 같은 1바이트 키는 영향 없음.
- 커서 값 범위 보정: 0 미만은 0, 버퍼 길이 초과는 끝. 확인(소스 bash/bashline.c readline_set_char_offset, 태그 bash-4.0 동일 로직). fzf의 줄 끝 이동 관용구 `READLINE_POINT=0x7fffffff`. 확인(소스 fzf/shell/key-bindings.bash `__fzf_history__`).
- `READLINE_*` 변수의 export 속성: 핸들러에서 실행한 자식 프로세스 환경에 노출. 확인(소스 bash/bashline.c `VSETATTR (v, att_exported)`).
- `READLINE_LINE` 변경 반영: undo 그룹 안에서 기존 텍스트 삭제 후 새 텍스트 삽입. 확인(소스 bash/bashline.c maybe_make_readline_line). readline undo 키로 원복 가능한 구조, 실사용 동작 미확인.
- `bind -m` 키맵 이름: `emacs`(= `emacs-standard`), `emacs-meta`, `emacs-ctlx`, `vi`(= `vi-command` = `vi-move`), `vi-insert`. 확인(소스 bash/doc/bashref.texi bind 항목, 게시본 https://www.gnu.org/software/bash/manual/html_node/Bash-Builtins.html).

### 2.2 핸들러 실행 환경과 tty 처리

- 실행 위치: 현재 셸 프로세스 안 `parse_and_execute` 호출, 별도 서브셸 없음. 확인(소스 bash/bashline.c bash_execute_unix_command).
- 표준 입출력: 핸들러 실행 경로에 리다이렉션 없음, 대화형 셸의 fd 0, 1, 2 상속 구조. 미확인(소스 기반 추론). 뒷받침 사례: fzf bash 위젯의 리다이렉션 없는 `$(fzf ...)` 실행. 확인(소스 fzf/shell/key-bindings.bash).
- 터미널 모드: readline 입력 모드 유지 상태의 핸들러 실행 추정. 뒷받침 사례: atuin의 init 시점 `stty -g` 저장, 핸들러 안 명령 실행 전 복원과 실행 후 재설정. 확인(소스 atuin atuin.bash `__atuin_accept_line`). bash 소스 직접 확인 미확인.
- 시그널: 핸들러 실행 동안 readline 시그널 처리 해제(`rl_clear_signals`), 종료 후 복원. 확인(소스 bash/bashline.c).
- crossterm raw 모드: 진입 시 기존 termios 저장, 해제 시 저장값 복원. 확인(소스 crossterm/src/terminal/sys/unix.rs `TERMINAL_MODE_PRIOR_RAW_MODE`). readline 상태 그대로 반환되는 구조.
- 화면 상태: bash 4.4 이상에서 핸들러 진입 시점 현재 입력 표시 줄 삭제, 커서 0열. 확인(소스 bash/lib/readline/display.c rl_clear_visible_line). 핸들러 종료 후 `rl_redraw_prompt_last_line`(마지막 프롬프트 줄 재그림, 반환값 124 시 전체 재그림). 확인(소스 bash/bashline.c, display.c). 화면 결과 실측 미확인.
- TUI 규약 결론: 핸들러 진입 시 커서 0열 보장, TUI는 현재 행부터 그림, 종료 시 자기 영역 삭제 후 커서를 시작 행 0열에 둘 것. 5.4절 규약과 동일.
- 명시 리다이렉션 권장(`</dev/tty >/dev/tty 2>/dev/tty`): (a) 사용자의 `exec >log` 등 셸 stdout 변경 방어, (b) crossterm 커서 위치 질의의 stdout 의존(2.3절). 설계 판단.

### 2.3 `$(...)` 캡처와 종료 코드

- fzf 방식: fzf 자체의 /dev/tty 열기, TUI 출력과 키 입력은 tty, 선택 결과만 stdout. `$(...)` 캡처 성립. 확인(소스 fzf/src/tui/light.go `DefaultTtyDevice`, fzf/src/tui/light_unix.go `openTty`).
- atuin 방식: `atuin search -i 3>&1 1>&2 2>&3 3>&-`로 stdout과 stderr 교환, TUI는 원래 stderr(터미널) 출력, 결과는 stderr 출력 후 캡처. 확인(소스 atuin atuin.bash `__atuin_search_cmd`, atuin.zsh). atuin.zsh 해당 위치에 `# TODO: not this` 주석 존재.
- ratatui와 crossterm 조합의 함정
  - crossterm `cursor::position()`: `ESC[6n` 질의를 `io::stdout()`에 기록, 응답은 입력 tty에서 최대 2000 ms 대기. 확인(소스 crossterm/src/cursor/sys/unix.rs `read_position_raw`).
  - ratatui `Viewport::Inline`: `Terminal::with_options`에서 `compute_inline_size` 호출, 내부에서 `backend.get_cursor_position()` 호출. CrosstermBackend 구현은 `crossterm::cursor::position()`. 확인(소스 ratatui/ratatui-core/src/terminal/init.rs, inline.rs, ratatui/ratatui-crossterm/src/lib.rs).
  - 결과: stdout 캡처 상태에서 질의 문자열의 결과 혼입과 위치 질의 실패 위험. 미확인(소스 기반 추론, 실측 필요).
  - atuin 대응: stdout 비터미널 시 인라인 모드 포기, 전체 화면 전환. 소스 주석 "inline mode won't work as it requires cursor position queries that don't work when stdout is captured". 확인(소스 atuin crates/atuin/src/command/client/search/interactive.rs).
- crossterm 입력 fd: stdin이 tty면 stdin, 아니면 /dev/tty 열기. 확인(소스 crossterm/src/terminal/sys/file_descriptor.rs `tty_fd`). 창 크기 조회는 /dev/tty 우선 후 stdout, stderr, stdin 순. 확인(소스 crossterm/src/terminal/sys/unix.rs `window_size`).
- 결론: 위젯 모드에서 howtui stdout은 터미널 유지, 결과는 임시 파일 전달.
- `$(...)` 관련 bash 규칙
  - 후행 개행 삭제. 확인(소스 bash/doc/bashref.texi Command Substitution, 게시본 https://www.gnu.org/software/bash/manual/html_node/Command-Substitution.html).
  - 명령 이름 없는 단순 명령(대입만)의 종료 상태는 마지막 명령 치환의 종료 상태. 확인(소스 bash/doc/bashref.texi Simple Command Expansion).
  - `local` 반환값은 오용이 아니면 0, `local v=$(cmd)` 형태의 종료 코드 소실. 확인(소스 bash/doc/bashref.texi local 항목). 선언과 대입 분리 필요.
- 종료 코드 제안: 0 수락, 1 사용자 취소, 2 사용법 또는 설정 오류, 3 실행 오류(LLM 호출, 네트워크), 130 SIGINT. 설계 제안. 핸들러 함수 자체는 항상 0 반환(124 오사용 방지).

### 2.4 멀티라인 READLINE_LINE

- 개행 포함 값 대입 사례: fzf 히스토리 위젯의 다중 줄 명령 대입. 확인(소스 fzf/shell/key-bindings.bash `__fzf_history__`).
- Enter 시 버퍼 전체의 단일 입력 파싱 여부, 화면 표시 품질, 히스토리 기록 형태(`cmdhist`, `lithist` 영향): 미확인(실측 필요).
- 권장: howtui 출력의 CRLF에서 LF 정규화, 후행 개행 제거.

### 2.5 macOS 기본 bash 3.2 (지원 범위 외)

- `READLINE_LINE`, `READLINE_POINT` 부재(4.0 도입). 확인(소스 bash/NEWS).
- 대안 기법: readline 매크로 연쇄와 `shell-expand-line` 활용. 확인(소스 fzf/shell/key-bindings.bash `BASH_VERSINFO[0] < 4` 분기, atuin atuin.bash `__atuin_macro_*`).
- macOS 기본 셸: 10.15부터 zsh. 확인(문서 https://support.apple.com/en-us/102360). 탑재 bash 세부 버전 미확인.
- 선례: atuin AI 위젯의 bash 4.0 미만 바인딩 생략. 확인(소스 atuin crates/atuin-ai/src/shell.rs `BASH_INIT`).
- 권장: bash 4.0 미만에서 init 스크립트의 바인딩 생략과 안내 1줄 출력.

### 2.6 bash-preexec 상호작용

- 현행 bash-preexec: `COMP_POINT` 또는 `READLINE_POINT` 존재 시 preexec 미호출(키 바인딩 내부 명령 제외). 확인(소스 bash-preexec/bash-preexec.sh `__bp_preexec_invoke_exec`).
- 구버전 bash-preexec: 키 바인딩 명령에도 preexec 호출. 확인(소스 atuin atuin.bash `__atuin_preexec` 주석). 터미널 제목 hook 등 부작용 가능성, howtui 측 대응 필요성 미확인.
- DEBUG trap 기반 구조: 핸들러 안 단순 명령마다 trap 실행. 핸들러 명령 수 최소화 권장. 설계 판단.
- `READLINE_*` export의 하위 bash 오인 문제: atuin의 `export -n READLINE_LINE READLINE_POINT` 처리. 확인(소스 atuin atuin.bash 주석). howtui는 하위 셸 생성 없음, 영향 미확인.

### 2.7 emacs와 vi 키맵

- 관행: fzf와 atuin 모두 emacs(또는 emacs-standard), vi-insert, vi-command 3개 키맵 개별 바인딩. 확인(소스 fzf/shell/key-bindings.bash, atuin atuin.bash).
- Alt 조합(ESC 접두) 주의: vi-insert에서 ESC는 명령 모드 전환 키, 모호한 시퀀스 판정은 `keyseq-timeout` 의존. 확인(소스 bash/lib/readline/doc/rluser.texi keyseq-timeout, 게시본 https://www.gnu.org/software/bash/manual/html_node/Readline-Init-File-Syntax.html). 구버전 bash vi-insert의 ESC 포함 시퀀스 처리 중단 문제. 확인(소스 atuin atuin.bash 주석).

### 2.8 readline 기본 키 할당 (충돌 검토)

| 키 | emacs | vi-insert | vi-command | 근거 |
|---|---|---|---|---|
| Ctrl+G | `abort` | self-insert | `abort` | 확인(소스 bash/lib/readline/emacs_keymap.c, vi_keymap.c) |
| Alt+E (ESC e) | 미할당(Meta-e 0) | ESC 명령 모드 전환 후 e 해석 위험 | 미확인 | 확인(소스 emacs_keymap.c), vi-insert 해석은 2.7절 근거 |
| Ctrl+] | `character-search` | self-insert | 미할당 | 확인(소스 emacs_keymap.c, vi_keymap.c) |
| Ctrl+O | `operate-and-get-next` | self-insert | 미할당 | 확인(소스 emacs_keymap.c, vi_keymap.c) |
| Ctrl+X | 접두 키맵(emacs-ctlx) | self-insert | 미할당 | 확인(소스 emacs_keymap.c, vi_keymap.c) |

- bash 추가 기본값: `M-g` glob-complete-word, `C-x g` glob-list-expansions, `C-x *` glob-expand-word, `M-C-e` shell-expand-line, `C-x C-e` edit-and-execute-command 등. 확인(소스 bash/bashline.c initialize_readline).

---

## 3. Zsh 통합

### 3.1 ZLE 위젯 구성 요소

| 요소 | 동작 | 근거 |
|---|---|---|
| `BUFFER` | 편집 버퍼 전체, 대입 시 커서 오프셋 유지(범위 밖이면 보정) | 확인(문서 https://zsh.sourceforge.io/Doc/Release/Zsh-Line-Editor.html, 소스 zsh/Doc/Zsh/zle.yo) |
| `LBUFFER`, `RBUFFER` | 커서 왼쪽, 오른쪽 부분, 대입 시 해당 부분만 교체 | 확인(문서 동일) |
| `CURSOR` | 0-`$#BUFFER` 범위 오프셋, 범위 밖 대입은 끝으로 보정 | 확인(문서 동일) |
| `zle -N 위젯 [함수]` | 사용자 위젯 생성, 동명 위젯 덮어쓰기 | 확인(문서 동일) |
| `bindkey -M 키맵 키 위젯` | 키맵별 바인딩 | 확인(소스 fzf/shell/key-bindings.zsh 사용 사례) |
| `zle reset-prompt` | 프롬프트 재확장 후 편집 버퍼 재표시, 구현은 `reexpandprompt` 후 `redisplay` | 확인(소스 zsh/Src/Zle/zle_main.c resetprompt, zsh/Doc/Zsh/zle.yo) |
| `zle redisplay` | 구현: `moveto(0, 0)`, CR, 프롬프트 높이-1 행 위로 이동, 전체 재그림 플래그(`resetneeded`) 설정 | 확인(소스 zsh/Src/Zle/zle_refresh.c redisplay) |
| `zle -I` | 표시 무효화, 문서상 주 용도는 trap 함수 출력. 구현 `trashzle`: 편집 표시 다음 줄 0열 이동, 화면 끝까지 삭제(조건부), 셸 tty 설정 복원, 재그림 플래그 | 확인(문서 동일, 소스 zsh/Src/Zle/zle_main.c trashzle, zle_thingy.c bin_zle_invalidate) |

- `reset-prompt` 도입: zsh/NEWS의 4.2 계열 항목(헤더 표기 "Changes in 4.2 since version 4.2.0"). 확인(소스 zsh/NEWS). macOS 탑재 zsh 세부 버전 미확인.

### 3.2 위젯 안 TUI 실행

- 위젯 함수 표준 입력: /dev/null 리다이렉션("to prevent external commands from unintentionally blocking ZLE by reading from the terminal"). 확인(문서 zshzle User-Defined Widgets, 소스 zsh/Doc/Zsh/zle.yo).
- fzf: CTRL-T, ALT-C 위젯에서 `< /dev/tty` 명시, 종료 후 `zle reset-prompt` 또는 `zle redisplay`. 확인(소스 fzf/shell/key-bindings.zsh).
- fzf 인라인 렌더러: 시작 시 커서 위치 질의, 열 위치 0 초과 시 한 줄 아래 공간 확보, 종료 시 자기 영역 삭제 후 원래 줄과 열 복귀(주석 "so nothing on that line is touched"). 확인(소스 fzf/src/tui/light.go `Init`, `Close`).
- atuin 검색 위젯: `zle -I`, fd 교환 캡처, `zle reset-prompt`, bracketed paste 재활성(`zle_bracketed_paste[1]` 출력). 확인(소스 atuin atuin.zsh `_atuin_search`).
- atuin AI 위젯 주석: `reset-prompt`의 커서 행 기준 재그림, 다중 줄 프롬프트의 위쪽 확장과 TUI 잔여 행 덮어쓰기 문제, 프롬프트 높이-1개 개행 선출력 회피. 확인(소스 atuin crates/atuin-ai/src/shell.rs `_atuin_ai_reset_prompt`).
- 해석: `redisplay`의 zle 내부 커서 위치 기준 상대 이동 → TUI 종료 시 원래 행과 열 복원 필수. `zle -I` 사용 시 편집 표시 아래 새 위치의 프롬프트 재출력과 기존 프롬프트 줄 화면 잔류 가능성. 미확인(소스 기반 추론, 실측 필요).
- 권장: `zle -I` 미사용, fzf식 커서 복원과 `zle reset-prompt` 조합, `</dev/tty >/dev/tty 2>/dev/tty` 명시.

### 3.3 멀티라인 버퍼

- `BUFFER`의 개행 포함 대입 사례: fzf 히스토리 위젯의 다중 선택 명령 개행 결합 후 `BUFFER` 대입, `CURSOR=${#BUFFER}`. 확인(소스 fzf/shell/key-bindings.zsh `fzf-history-widget`).
- atuin: 버퍼에 개행 포함 시 위쪽 화살표의 `up-line` 처리(멀티라인 편집 우선). 확인(소스 atuin atuin.zsh `_atuin_up_search`).
- zsh `$(<file)`의 후행 개행 처리: 미확인(zsh 확장 문서 미열람).

### 3.4 vi 모드 키맵

- 초기 키맵 8종: `emacs`, `viins`, `vicmd`, `viopp`, `visual`, `isearch`, `command`, `.safe`. 확인(문서 zshzle Keymaps).
- `main` 키맵 선택: 셸 시작 시 `VISUAL` 또는 `EDITOR`에 "vi" 포함이면 `viins`, 아니면 `emacs`. 확인(문서 zshzle Keymaps).
- 관행: fzf와 atuin 모두 `bindkey -M emacs`, `-M viins`, `-M vicmd` 개별 지정. 확인(소스 fzf/shell/key-bindings.zsh, atuin crates/atuin/src/command/client/init/zsh.rs).
- zsh-vi-mode 등 플러그인의 지연 키맵 재설정에 따른 바인딩 소실 가능성: 미확인.

### 3.5 bracketed paste

- ZLE bracketed paste 지원: zsh 5.1부터. 확인(소스 zsh/NEWS "Changes from 5.0.8 to 5.1").
- TUI 종료 시 `ESC[?2004l` 송출에 따른 셸 측 비활성 잔류 위험, atuin의 위젯 종료 시 재활성 처리. 확인(소스 atuin atuin.zsh).
- fzf: 시작 시 DECRQM(`ESC[?2004$p`) 질의, 종료 시 원래 상태 복원. 확인(소스 fzf/src/tui/light.go `pasteModeQuery`, `pasteWasSet`).
- 권장: howtui의 bracketed paste 토글 시 시작 상태 질의와 복원, 셸 스크립트 측 재활성 이중 방어.

### 3.6 zsh 기본 키 할당 (충돌 검토)

| 키 | emacs | viins | vicmd | 근거 |
|---|---|---|---|---|
| Ctrl+G | `send-break` | `list-expand` | `list-expand` | 확인(소스 zsh/Src/Zle/zle_bindings.c) |
| Alt+E (ESC e) | 미할당 | ESC 접두 해석 위험 | 미확인 | 확인(소스 zle_bindings.c metabind M-e) |
| Ctrl+] | 미할당 | self-insert | 미할당 | 확인(소스 zle_bindings.c) |
| Ctrl+O | `accept-line-and-down-history` | self-insert | 미확인 | 확인(소스 zle_bindings.c) |
| Ctrl+X 접두 | `^X*`, `^Xg`, `^XG`, `^Xr`, `^Xs`, `^Xu`, `^X^K`, `^X^O` 등 | 미할당 | 미확인 | 확인(소스 zsh/Src/Zle/zle_keymap.c) |

---

## 4. PowerShell 통합

### 4.1 PSReadLine API

| API | 동작 | 2.0.0 존재 | 근거 |
|---|---|---|---|
| `Set-PSReadLineKeyHandler -Chord <s> -ScriptBlock <sb> [-BriefDescription] [-ViMode Insert\|Command]` | 키와 스크립트 블록 연결, 문자 키는 소문자 표기(`Ctrl+g`) | 존재(`-ViMode` 포함) | 확인(문서 https://learn.microsoft.com/en-us/powershell/module/psreadline/set-psreadlinekeyhandler?view=powershell-7.5, 소스 PSReadLine 태그 v2.0.0 PSReadLine/Cmdlets.cs `ChangePSReadLineKeyHandlerCommandBase`) |
| `GetBufferState([ref]$line, [ref]$cursor)` | 버퍼 문자열과 커서 인덱스, 인덱스 단위는 StringBuilder 인덱스(UTF-16 코드 유닛) | 존재 | 확인(소스 PSReadLine/PSReadLine/PublicAPI.cs, v2.0.0) |
| `Replace(start, length, text)` | 범위 검증 실패 시 버퍼 변경 전 `ArgumentException`, undo 지원, 커서는 교체 끝 | 존재 | 확인(소스 PublicAPI.cs, v2.0.0) |
| `Insert(string)` | 현행 소스는 `"\r\n"`에서 `"\n"` 정규화, v2.0.0은 정규화 없음 | 존재 | 확인(소스 PublicAPI.cs, v2.0.0 PublicAPI.cs) |
| `RevertLine()` | undo 이력 전체 되돌림(예측 제안 표시 중이면 제안 취소만) | 존재 | 확인(소스 PSReadLine/PSReadLine/BasicEditing.cs) |
| `InvokePrompt($null, $y)` | 인자가 int면 해당 Y로 이동 후 프롬프트 재출력, 내부 `_initialY` 재설정, 인자 없으면 `_initialY - ExtraPromptLineCount` 위치 | 존재(int 인자 지원, 범위 검사 없음) | 확인(소스 PSReadLine/PSReadLine/ReadLine.cs, v2.0.0 ReadLine.cs) |
| `InvokePrompt` 범위 검사 | Y 범위 밖이면 `ArgumentOutOfRangeException` | 2.4.3-beta3 추가 | 확인(소스 PSReadLine/PSReadLine/Changes.txt #4791) |
| `AcceptLine()`, `SetCursorPosition(int)` | 입력 확정, 커서 이동 | 존재 | 확인(소스 BasicEditing.cs, PublicAPI.cs, v2.0.0) |
| `ExtraPromptLineCount` 옵션 | 다중 줄 프롬프트 추가 줄 수 | 존재 | 확인(소스 v2.0.0 ReadLine.cs InvokePrompt 사용처) |

- 공식 문서 예제: `RevertLine`, `Insert`, `AcceptLine` 조합의 ScriptBlock 핸들러. 확인(문서 Set-PSReadLineKeyHandler Example 2).

### 4.2 PSReadLine 탑재 버전

| PowerShell | PSReadLine | 근거 |
|---|---|---|
| Windows PowerShell 5.1 | 2.0.0 | 확인(문서 https://learn.microsoft.com/en-us/powershell/module/psreadline/?view=powershell-7.5) |
| 7.0.11 | 2.0.4 최초 탑재 | 확인(문서 동일) |
| 7.2.5 | 2.1.0 최초 탑재 | 확인(문서 동일) |
| 7.3.0 | 2.2.6 최초 탑재 | 확인(문서 동일) |
| 7.4.0-rc.1, 7.4.2, 7.4.7 | 2.3.4, 2.3.5, 2.3.6 순차 탑재 | 확인(문서 동일) |
| 7.5.0 | 2.3.6 | 확인(문서 동일) |
| 7.6 이후 | 미확인 | - |

- PSReadLine Changes.txt 최신 항목: 2.4.5(2025-10-22). 2.4.0-beta0에서 최소 PowerShell 버전 5.1 명시. 확인(소스 PSReadLine/PSReadLine/Changes.txt).
- PSReadLine master 매니페스트: `ModuleVersion = '3.0.0'`, `PowerShellVersion = '7.4'`, `net8.0` 대상. 확인(소스 PSReadLine/PSReadLine/PSReadLine.psd1, PSReadLine.csproj). 3.x 정식 출시 여부와 5.1 지원 중단 확정 여부 미확인.
- PowerShell 7.0 이상 Windows에서 화면 낭독기 감지 시 PSReadLine 자동 로드 생략. 확인(문서 PSReadLine 모듈 페이지).
- Windows 10 초기 빌드의 PSReadLine 1.x 탑재 여부: 미확인. init 스크립트의 `(Get-Module PSReadLine).Version` 2.0 이상 검사 권장.
- 결론: howtui 사용 API 전체가 2.0.0에 존재, 5.1과 7.x 공통 코드 경로 가능. 2.0.0 대응 조치는 CRLF 직접 정규화와 `InvokePrompt` Y 값 직접 범위 보정 2건.

### 4.3 키 핸들러 실행 환경

- ScriptBlock 핸들러 호출 경로: `ProcessOneKey`에서 `CallPossibleExternalApplication`으로 감싼 실행. 확인(소스 PSReadLine/PSReadLine/ReadLine.cs). v2.0.0 동일 구조. 확인(소스 v2.0.0 ReadLine.cs).
- 감싼 실행의 효과: (a) `[Console]::OutputEncoding`을 ReadLine 진입 시점 원래 값으로 복원, 종료 후 UTF-8 재설정, (b) Windows에서 콘솔 입력 모드를 PSReadLine 이전 모드로 복원, 종료 후 원복. 확인(소스 ReadLine.cs, PSReadLine/PSReadLine/PlatformWindows.cs `CallPossibleExternalApplication`).
- PSReadLine 편집 중 출력 인코딩: UTF-8 전환, 단 이미 UTF-8이거나 Windows 래스터 글꼴이면 생략. 확인(소스 ReadLine.cs `_skipOutputEncodingChange`).
- 의미: 핸들러 안의 `[Console]::OutputEncoding`은 사용자 원래 값. 한국어 Windows 기본값 CP949 여부 미확인(실측 필요).
- 핸들러 동기 실행: 외부 프로세스 대기 동안 PSReadLine 입력 처리 정지 구조. 미확인(소스 기반 추론).
- raw 모드 진입 전 Ctrl+C 위험: 복원된 입력 모드에서 Ctrl+C가 콘솔 전체 제어 이벤트로 전달될 가능성, 핸들러 중단 시에도 `finally`로 임시 파일 정리와 버퍼 무변경 유지 필요. 미확인(실측 필요).

### 4.4 네이티브 exe 실행과 출력 캡처

- 캡처 디코딩 기준: `$PSApplicationOutputEncoding`(7.7-preview.1 도입, 기본값 `[Console]::OutputEncoding`) 또는 `[Console]::OutputEncoding`. 확인(문서 https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_preference_variables?view=powershell-7.7, 소스 pwsh-src `GetOutputEncoding`). Windows PowerShell 5.1 내부 구현 미확인. zoxide, PSFzf, atuin 모두 `[Console]::OutputEncoding` 임시 UTF-8 설정 관행. 확인(소스 zoxide/templates/powershell.txt `__zoxide_bin`, PSFzf/PSFzf.Base.ps1 `InvokePromptHack`, atuin atuin.ps1 `Invoke-AtuinSearch`).
- stdout과 stderr 리다이렉션 판정: 파이프라인 마지막이고 출력 대상이 Out-Default일 때만 stdout 비리다이렉션, stderr는 오류 파이프 대상이 Out-Default일 때만 비리다이렉션, 트랜스크립트 중 화면 스크랩 불가 시 전부 리다이렉션. 확인(소스 pwsh-src `CalculateIORedirection`).
- `$cmd = & howtui.exe`의 함의: stdout 리다이렉션 확정. stderr는 PSReadLine 핸들러 문맥의 오류 파이프 구성에 좌우, stderr로 그린 TUI의 콘솔 도달 보장 불가. 미확인(실측 필요).
- PSFzf 주석: `--height`가 `Invoke-Expression` 경로에서 비동작, `System.Diagnostics.Process` 직접 사용. 확인(소스 PSFzf/PSFzf.Base.ps1 `Invoke-FzfDefaultSystem`).
- Rust 표준 출력의 Windows 동작: 핸들이 콘솔이면 UTF-8을 UTF-16으로 변환 후 `WriteConsoleW`, 파이프면 바이트 그대로 기록. 확인(소스 rust-std `write`, `write_console_utf16`). 콘솔 직접 연결 시 코드 페이지 무관.
- `CONOUT$` 파일 직접 쓰기: `WriteFile` 경로, 콘솔 출력 코드 페이지 영향. atuin의 `SetConsoleOutputCP(65001)` 설정과 종료 시 원복. 확인(소스 atuin interactive.rs `TerminalWriter`).
- crossterm Windows 구현: 커서 위치, 창 크기, 콘솔 모드, 이벤트 입력에 `CONOUT$`, `CONIN$`(CreateFileW) 핸들 사용, stdout 리다이렉션과 무관. VT 처리 활성화도 `CONOUT$` 대상. 확인(소스 crossterm_winapi 0.9.1 src/handle.rs, crossterm/src/cursor/sys/windows.rs, terminal/sys/windows.rs, event/source/windows.rs, ansi_support.rs).
- fzf Windows 구현: TUI 출력 `os.Stderr`, 콘솔 모드 설정은 `CONOUT$`, `CONIN$`. 확인(소스 fzf/src/tui/light_windows.go). fzf 저장소의 PowerShell 키 바인딩 스크립트 부재(shell/ 아래 bash, zsh, fish, nu만 존재). 확인(소스 fzf/shell).
- 결론: PowerShell 위젯은 `& exe` 캡처 대신 `System.Diagnostics.Process`의 리다이렉션 없는 실행, 결과는 임시 파일. TUI는 stdout(콘솔)에 그림, Rust `WriteConsoleW` 경로로 코드 페이지 문제 회피.

### 4.5 인코딩 함정과 5.1, 7.x 차이

| 항목 | Windows PowerShell 5.1 | PowerShell 7.x | 근거 |
|---|---|---|---|
| `$OutputEncoding` 기본값(네이티브 stdin 방향) | `ASCIIEncoding` | `UTF8Encoding` | 확인(문서 about_Preference_Variables 5.1, 7.5) |
| 네이티브 출력 디코딩 | `[Console]::OutputEncoding`(5.1 구현 미확인) | `[Console]::OutputEncoding`, 7.7-preview.1부터 `$PSApplicationOutputEncoding` 우선 | 확인(문서 7.7, 소스 pwsh-src) |
| BOM 없는 스크립트 파일 해석 | ANSI 코드 페이지 | UTF-8 | 확인(문서 https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_character_encoding?view=powershell-7.5) |
| 네이티브 인자 전달 | Legacy | 7.3부터 `$PSNativeCommandArgumentPassing`, Windows 기본 `Windows` 모드 | 확인(문서 about_Preference_Variables 7.5) |
| `ProcessStartInfo.ArgumentList` | 없음(.NET Framework) | 있음 | 확인(소스 starship src/init/starship.ps1 `Invoke-Native` 주석) |
| PSReadLine 기본 버전 | 2.0.0 | 2.3.x 이상 | 확인(문서 PSReadLine 모듈 페이지) |
| `Insert`의 CRLF 정규화 | 없음 | 있음(현행 소스) | 확인(소스 PublicAPI.cs, v2.0.0) |
| 프로필 경로 | `$HOME\Documents\WindowsPowerShell\Microsoft.PowerShell_profile.ps1` | `$HOME\Documents\PowerShell\Microsoft.PowerShell_profile.ps1` | 확인(문서 https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_profiles?view=powershell-7.5) |

한국어 로캘 위험

- 핸들러 안 `[Console]::OutputEncoding`이 CP949일 때 UTF-8 stdout 캡처의 한글 깨짐 구조. 기본 코드 페이지 값 미확인(실측 필요), 디코딩 기준은 4.4절 근거로 확인.
- 콘솔 코드 페이지를 바꾸는 방식의 부작용: `[Console]::OutputEncoding` 변경의 프로세스 전역 적용(7.7 문서의 `$PSApplicationOutputEncoding` 설명 "won't change a process wide setting" 대비). 확인(문서 about_Preference_Variables 7.7). 임시 변경 후 반드시 원복 필요.
- init 스크립트 출력 자체도 캡처 후 디코딩 대상. 비ASCII 포함 시 5.1 한국어 환경 깨짐 위험. 미확인(소스 기반 추론). 권장: init 스크립트 ASCII 전용, 실행 파일 절대 경로 삽입(starship 방식, 확인 소스 starship src/init/mod.rs `sprint_pwsh`) 대신 런타임 `Get-Command` 조회(사용자 폴더 한글 이름 대비).

전달 방식 비교

| 방식 | 사례 | 장점 | 단점 |
|---|---|---|---|
| stdout 캡처 + `[Console]::OutputEncoding` 임시 UTF-8 | zoxide, PSFzf | 구현 단순 | 전역 설정 변경, 줄 배열 분할(미확인), stderr 경로 불확실(4.4절) |
| 임시 파일 + UTF-8 명시 읽기 | atuin(`--result-file`, `Get-Content -Raw -Encoding UTF8`) | 코드 페이지 무관, 개행 보존, TUI stdout 자유 | 파일 생성과 정리 |
| base64 stdout | 사례 미발견 | 코드 페이지 무관 | Unix DSR 문제 잔존, 양측 인코딩 코드 필요 |

- 권장: 결과는 임시 파일 UTF-8(BOM 없음), 읽기는 `[System.IO.File]::ReadAllText(path, UTF8Encoding)`(5.1과 7.x 공통 .NET API, BOM 처리 세부 미확인).
- 입력(현재 버퍼)은 환경 변수 전달. atuin 주석 "PowerShell doesn't handle double quotes in native command line arguments the same way depending on its version ... so we go through an environment variable". 확인(소스 atuin atuin.ps1 `PSConsoleHostReadLine`).

### 4.6 기존 도구의 PowerShell 방식

| 도구 | 실행 방식 | 결과 수신 | 인코딩 처리 | 화면 복구 | 기본 키 | 근거 |
|---|---|---|---|---|---|---|
| atuin | `System.Diagnostics.Process`, stdout 비리다이렉션, stderr만 리다이렉션 | `--result-file` 임시 파일 | `[Console]::OutputEncoding` 임시 UTF-8, `StandardErrorEncoding` UTF-8, 결과 파일 UTF-8 읽기 | `InvokePrompt($null, $y)`, `$y` = 현재 Y + 프롬프트 줄 수 기반 오프셋, 0과 BufferHeight-1 사이 보정 | Ctrl+r, UpArrow | 확인(소스 atuin atuin.ps1) |
| PSFzf | Process, stdin과 stdout 리다이렉션 | `OutputDataReceived` 이벤트 | `StandardOutputEncoding` UTF-8, `InvokePrompt` 전후 `OutputEncoding` UTF-8(PSFzf issue #71 주석) | `InvokePrompt()` | Ctrl+t, Ctrl+r | 확인(소스 PSFzf/PSFzf.Base.ps1, PSFzf.PSConsoleReadLineWrappers.ps1) |
| zoxide | `& zoxide` 직접 호출(`zi`는 fzf 경유) | 변수 캡처 | `[Console]::OutputEncoding` 임시 UTF-8 | 해당 없음(디렉터리 이동) | 없음 | 확인(소스 zoxide/templates/powershell.txt) |
| navi | Process, stdout 리다이렉션 | `ReadToEnd` | 별도 처리 없음 | `RevertLine`, `InvokePrompt()`, `Insert` | Ctrl+g | 확인(소스 navi shell/navi.plugin.ps1) |
| aichat | `aichat -e $_old` 직접 호출(TUI 없음) | 변수 캡처 | 별도 처리 없음 | `DeleteLine` 후 `Insert` | Alt+e | 확인(소스 aichat integration.ps1) |
| starship | Process, stdout과 stderr 비동기 읽기 | 프롬프트 문자열 | `StandardOutputEncoding` UTF-8 | 해당 없음 | 해당 없음 | 확인(소스 starship src/init/starship.ps1) |

- atuin 주석: PowerShell `Set-Location`의 프로세스 작업 디렉터리 미변경, `WorkingDirectory` 명시 필요. 확인(소스 atuin atuin.ps1).
- atuin 주석: "PSReadLine maintains its own cursor position, which will no longer be valid if Atuin scrolls the display in inline mode. Fortunately, InvokePrompt can receive a new Y position and reset the internal state." 확인(소스 atuin atuin.ps1).

### 4.7 PSReadLine 기본 키 할당 (충돌 검토)

| 키 | Windows 모드 | Emacs 모드 | Vi 모드 | 근거 |
|---|---|---|---|---|
| Ctrl+g | 미할당 | `Abort` | `Abort` | 확인(소스 PSReadLine/PSReadLine/KeyBindings.cs, KeyBindings.vi.cs) |
| Alt+e | 미할당 | 미할당 | 미확인 | 확인(소스 KeyBindings.cs 검색 결과 AltE 부재) |
| Ctrl+] | `GotoBrace` | `CharacterSearch` | 삽입 모드 `GotoBrace` | 확인(소스 KeyBindings.cs, KeyBindings.vi.cs) |
| Ctrl+Space | `MenuComplete`(Windows 플랫폼 한정 추가) | `MenuComplete`(Windows 플랫폼 한정 추가) | 미확인 | 확인(소스 KeyBindings.cs) |
| Ctrl+o | 미할당 | `AcceptAndGetNext` | 미확인 | 확인(소스 KeyBindings.cs) |
| Ctrl+x | `Cut` | 코드 접두(`ChordFirstKey`) | 미확인 | 확인(소스 KeyBindings.cs) |

- Windows에서 기본 EditMode가 Windows 모드라는 전제: 미확인(Set-PSReadLineOption 문서 미열람).

---

## 5. 공통 설계

### 5.1 init 스크립트 패턴

| 도구 | Bash | Zsh | PowerShell | 근거 |
|---|---|---|---|---|
| atuin | `eval "$(atuin init bash)"` | `eval "$(atuin init zsh)"` | `atuin init powershell \| Out-String \| Invoke-Expression` | 확인(소스 atuin/install.sh, atuin atuin.ps1 헤더) |
| zoxide | rc 파일 끝 추가 안내 | rc 파일 끝 추가 안내 | `Invoke-Expression (& { (zoxide init powershell \| Out-String) })` | 확인(소스 zoxide/README.md, zoxide/templates/powershell.txt) |
| starship | `eval -- "$(starship init bash --print-full-init)"`(2단계 init, bash 3.2와 POSIX 모드 호환 사유 주석) | 2단계 init | `Invoke-Expression (& '경로' init powershell --print-full-init \| Out-String)` | 확인(소스 starship src/init/mod.rs `init_stub`) |

- starship 실행 파일 경로: `which` 실패 시 `current_exe` 사용, PowerShell용은 작은따옴표 이스케이프. 확인(소스 starship src/init/mod.rs `StarshipPath`).
- `Out-String` 필요 사유(네이티브 출력의 줄 단위 배열 수신): 미확인(일반 동작, 문서 미열람).
- 셸 종류는 필수 위치 인자, 자동 감지 없음. 확인(소스 atuin crates/atuin/src/command/client/init.rs `Shell` enum, starship `init_stub(shell_name)`).
- atuin 위젯 정의와 바인딩 분리: 위젯은 항상 정의, 바인딩은 옵션(`--disable-ctrl-r`, `--disable-up-arrow`, `ATUIN_NOBIND`, `--bind-ai-resume` 기본 비활성). 확인(소스 atuin init.rs, init/zsh.rs, init/bash.rs).

설치 스크립트의 rc 파일 처리 실태

| 도구 | 동작 | 근거 |
|---|---|---|
| atuin install.sh | `.zshrc`, `.bashrc`에 grep 검사 후 init 줄 자동 추가, fish 설정 파일 존재 시 추가 | 확인(소스 atuin/install.sh) |
| fzf install | 대화형 질의("Do you want to update your shell configuration files?"), `--[no-]update-rc` 옵션 | 확인(소스 fzf/install) |
| starship install.sh | 셸별 추가 문구 안내 출력만 | 확인(소스 starship/install/install.sh `print_install`) |
| zoxide | README의 수동 추가 안내 | 확인(소스 zoxide/README.md) |

권고

- howtui 설치기의 rc 파일과 `$PROFILE` 무수정 원칙, 추가할 한 줄의 안내 출력만.
- PowerShell 안내 포함 사항: (a) 5.1과 7.x 프로필 경로 분리(두 셸 사용 시 양쪽 등록), (b) 실행 정책 `Restricted`에서 프로필 미실행. 확인(문서 about_Profiles "The Restricted execution policy is the default. It prevents all scripts from running, including the profiles.").
- 명령 형태: `howtui init <bash|zsh|powershell> [--key <키>] [--no-bind]`. 위젯 함수 항상 정의, 바인딩만 선택.

### 5.2 현재 셸 감지

| 단서 | 성질 | 근거 |
|---|---|---|
| `$SHELL` | 셸 시작 시 미설정이면 bash가 로그인 셸 경로 대입, 상속값 유지 구조로 현재 실행 셸과 불일치 가능 | 확인(소스 bash/doc/bashref.texi SHELL 항목) |
| `$BASH_VERSION` | 셸 변수, 자식 프로세스 환경에 미노출 | 확인(로컬 실측 bash 5.0.17, `env` 결과 0건) |
| `$ZSH_VERSION` | 자식 프로세스 노출 여부 | 미확인(zsh 미설치) |
| `PSModulePath` | atuin import의 Windows PowerShell 판정 근거, 자식 프로세스 상속에 따른 오판 가능성 | 확인(소스 atuin crates/atuin/src/command/client/import.rs), 오판 가능성 미확인 |
| 부모 프로세스 이름 | atuin `Shell::current`의 sysinfo 기반 부모 이름 조회, 래퍼(sudo, script, tmux 등) 개입 시 오판 가능성 | 확인(소스 atuin crates/atuin-common/src/shell.rs), 오판 가능성 미확인 |
| 통합 스크립트의 명시 전달 | atuin의 호출마다 `ATUIN_SHELL=zsh atuin ...`, 바이너리의 `ATUIN_SHELL` 우선 사용 | 확인(소스 atuin atuin.zsh, atuin-common shell.rs `from_env`) |

- 권고: 위젯 함수가 호출 시점에만 `HOWTUI_SHELL=<bash|zsh|powershell>` 전달(전역 export 금지, 중첩 셸 오염 방지). 미설정이면 비통합 모드 판정.

### 5.3 셸 함수와 바이너리 간 프로토콜

전달 수단 비교

| 방식 | Bash | Zsh | PowerShell | 문제점 |
|---|---|---|---|---|
| stdout 캡처 | 가능 | 가능 | 가능 | Unix 인라인 뷰포트 DSR 충돌(2.3절), PowerShell 코드 페이지 디코딩(4.4절), 후행 개행 삭제 |
| stdout과 stderr 교환(atuin) | 가능 | 가능 | 불가 | 진단 메시지와 패닉 출력의 결과 혼입 위험(설계 판단) |
| fd 3 | 가능 | 가능 | 개념 부재 | 3개 셸 공통 불가 |
| 임시 파일 | 가능 | 가능 | 가능 | 생성과 정리 비용, 권한 관리 |

권장 프로토콜 (4단계 구조)

- (1) 입력 전달: 환경 변수
  - `HOWTUI_SHELL`: `bash`, `zsh`, `powershell`
  - `HOWTUI_BUFFER`: 현재 버퍼 전체
  - `HOWTUI_LBUFFER`: 커서 앞 부분 문자열(bash 4.x에서는 생략)
  - `HOWTUI_OUTPUT_FILE`: 셸이 미리 만든 임시 파일 경로
  - argv 미사용 사유: PowerShell 버전별 인자 인용 차이(확인 4.5절), 프로세스 목록 노출 위험(미확인)
- (2) TUI 실행: 표준 입출력 전부 터미널 연결. bash와 zsh는 `</dev/tty >/dev/tty 2>/dev/tty`, PowerShell은 리다이렉션 없는 Process.
- (3) 결과 기록: 수락 시에만 기록, UTF-8(BOM 없음), LF 개행, 후행 개행 없음.
- (4) 판정과 반영: 종료 코드 0이고 파일 비어 있지 않을 때만 버퍼 교체, 그 외 버퍼 무변경. 오류 코드면 TUI 정리 후 1줄 안내.

종료 코드 규약 (설계 제안)

| 코드 | 의미 | 셸 측 처리 |
|---|---|---|
| 0 | 수락, 결과 파일 기록 완료 | 버퍼 교체, 커서 끝 |
| 1 | 사용자 취소(Esc, Ctrl+C 키 이벤트) | 무변경 |
| 2 | 사용법, 설정, 터미널 열기 실패 | 무변경, 안내 |
| 3 | 실행 오류(LLM 호출, 네트워크) | 무변경, 안내 |
| 130 | SIGINT(raw 모드 진입 전) | 무변경 |

커서 정보 단위 차이와 `LBUFFER` 문자열 전달 사유

| 셸 | 커서 단위 | 근거 |
|---|---|---|
| bash 4.x | 바이트 | 확인(소스 태그 bash-4.0 bashline.c) |
| bash 5.0 이상 | `MB_STRLEN` 기준 문자 수 | 확인(소스 bash/bashline.c readline_get_char_offset) |
| zsh | 문자 오프셋(`$#LBUFFER`와 동일) | 확인(문서 zshzle CURSOR) |
| PSReadLine | UTF-16 코드 유닛 | 확인(소스 PublicAPI.cs) |

- `LBUFFER` 직접 전달로 단위 변환 제거. bash 5.0 이상 `${READLINE_LINE:0:READLINE_POINT}`의 문자 기준 일치 여부: 미확인(bash 부분 문자열 확장 소스 미열람).
- 반영 방식: 기본 전체 교체(기존 버퍼는 맥락 입력, 제안 명령으로 교체), 커서 끝 이동. undo 가능성은 bash(undo 그룹, 확인 2.1절), PSReadLine(`Replace` undo 지원, 확인 4.1절), zsh 미확인.

### 5.4 TUI 화면 규약 (커서 복원 규약)

- 시작
  - 커서 위치 질의 `(x0, y0)`.
  - `x0 > 0`이면 줄바꿈으로 다음 줄 확보(fzf 방식, 확인 3.2절). bash는 핸들러 진입 시 0열(확인 2.2절)이므로 줄바꿈 없음.
  - ratatui `Viewport::Inline(h)`: 커서 행 0열 고정, 공간 부족 시 줄 추가와 스크롤. 확인(소스 ratatui/ratatui-core/src/terminal/init.rs `with_options` 주석, inline.rs).
- 종료
  - 뷰포트 첫 행 이동 후 `ESC[J`.
  - 줄바꿈으로 시작했으면 한 줄 위 `x0` 열 복귀, 아니면 뷰포트 첫 행 0열.
  - raw 모드 해제, 커서 표시, bracketed paste와 마우스 모드 원복, 패닉 훅에서 동일 처리.
- 셸별 재그림
  - bash: 자동 재그림(4.4 이상 마지막 프롬프트 줄, 4.0 계열 전체).
  - zsh: `zle reset-prompt`.
  - PowerShell: `InvokePrompt($null, $y)`, `$y = [Console]::CursorTop - ExtraPromptLineCount`, 0과 `BufferHeight - 1` 사이 보정(2.0.0 범위 검사 부재 대비).
- 멀티라인 버퍼에서 커서가 첫 줄이 아닐 때 PowerShell `$y` 계산 오차 가능성: 미확인.
- conhost 좌표: 화면 버퍼 절대 좌표와 창 상대 좌표 차이, crossterm의 상대 좌표 변환(`parse_relative_y`). 확인(소스 crossterm/src/cursor/sys/windows.rs). PowerShell 스크립트 측은 `[Console]::CursorTop`(버퍼 절대 좌표) 일관 사용 필요.

### 5.5 비통합 모드 (stdout 출력 전용)

- stdout 터미널: TUI 종료 후 최종 명령만 stdout 출력.
- stdout 비터미널(`cmd=$(howtui)`, 파이프)
  - Unix: TUI 출력 /dev/tty. crossterm DSR의 stdout 의존으로 인라인 뷰포트 불가, 대안 (a) 전체 화면 전환(atuin 방식, 확인 2.3절), (b) /dev/tty 대상 자체 DSR 질의 후 `Viewport::Fixed` 사용(설계 제안, 미검증).
  - Windows: TUI 출력은 콘솔인 stderr 또는 `CONOUT$`(`SetConsoleOutputCP(65001)` 후 원복, atuin 방식). crossterm 커서 질의의 `CONOUT$` 콘솔 API 경로로 인라인 유지 가능성. 미확인(소스 기반 추론).
- PowerShell `$c = howtui` 직접 사용 시 한글 깨짐 위험(4.5절). 안내: 위젯 사용 또는 `[Console]::OutputEncoding` UTF-8 설정. 선택 옵션 제안: `--print-encoding utf8|console`(설계 제안).
- stdout에는 최종 명령만, 진단은 stderr(요구사항 준수).

### 5.6 알려진 실패 유형

| 환경 | 위험 | 대응 | 근거 |
|---|---|---|---|
| tmux | escape-time 지연으로 TUI Esc 반응 지연, 3.5에서 기본 10 ms로 축소, 이전 기본값 수치 | Ctrl+C 취소 병행 지원 | 확인(문서 https://raw.githubusercontent.com/tmux/tmux/master/CHANGES "CHANGES FROM 3.4 TO 3.5"), 이전 수치 미확인 |
| tmux | 팝업 렌더링 대안(`display-popup`, 3.2 이상) | 선택 기능 후보 | 확인(소스 atuin atuin.zsh `__atuin_tmux_popup_check`, tmux CHANGES 3.2 popup 항목) |
| GNU screen | DSR, bracketed paste 동작 | 실측 | 미확인 |
| SSH | 대화형 pty 필요, 비대화형 실행에서 /dev/tty 부재, 고지연 링크에서 crossterm DSR 2000 ms 제한 초과 | /dev/tty 열기 실패 시 코드 2와 안내 | 제한값 확인(소스 crossterm unix.rs), 나머지 미확인 |
| Windows Terminal | 기본 키 바인딩에 Ctrl+G, Alt+E 없음, Alt+Enter 전체 화면 | 영향 없음 | 확인(소스 wt defaults.json) |
| conhost | 레거시 콘솔 모드의 VT 미지원 가능성, DSR 응답은 입력 스트림 전달 | VT 활성 실패 시 오류 안내 | DSR 확인(문서 https://learn.microsoft.com/en-us/windows/console/console-virtual-terminal-sequences), 레거시 모드 미확인 |
| VS Code 통합 터미널 | Ctrl+G(Windows, Linux), Cmd+G(macOS)의 "Go to Recent Directory" 바인딩, 기본 `commandsToSkipShell` 포함으로 셸 미전달 | `terminal.integrated.commandsToSkipShell`에 `-workbench.action.terminal.goToRecentDirectory` 추가 안내 또는 키 변경 | 바인딩 확인(소스 vscode terminal.history.contribution.ts, terminal.history.ts), `-` 접두 규칙 확인(문서 https://code.visualstudio.com/docs/terminal/advanced), 해당 ID 적용 실측 미확인 |
| VS Code 통합 터미널 | Alt 조합: `allowMnemonics` 기본 비활성으로 셸 전달 | 영향 없음 | 확인(문서 동일) |
| 시맨틱 프롬프트 터미널(Ghostty 등) | 크기 변경 시 프롬프트 영역 리플로우로 TUI 출력 삭제 | TUI 시작 전 `OSC 133;C` 송출(atuin AI 방식) | 확인(소스 atuin crates/atuin-ai/src/shell.rs 주석), 실제 영향 미확인 |
| 다중 줄 프롬프트 | bash 마지막 줄만 재그림, zsh 커서 기준 위쪽 재그림, PowerShell `ExtraPromptLineCount` 필요 | 5.4절 커서 복원 규약 | 확인(2.1절, 3.1절, 4.1절 근거) |
| bracketed paste | TUI 종료 후 셸 측 비활성 잔류 | 상태 질의와 복원, zsh 재활성 | 확인(3.5절 근거), bash 측 영향 미확인 |
| PowerShell 실행 정책 | `Restricted`에서 프로필 미실행 | 설치 안내 | 확인(문서 about_Profiles) |
| 화면 낭독기 | PowerShell 7 PSReadLine 자동 로드 생략 | init 스크립트의 PSReadLine 부재 검사와 경고 | 확인(문서 PSReadLine 모듈 페이지) |
| ble.sh | readline 대체, atuin의 별도 분기 존재 | 지원 범위 외 명시 | 확인(소스 atuin atuin.bash `BLE_ATTACHED`) |
| 한국어 IME | 조합 중 키 입력과 raw 모드 상호작용 | 실측 | 미확인 |
| 비 UTF-8 로캘(`LANG=C` 등) | bash 커서 단위와 표시 | 실측 | 미확인 |

---

## 6. 기본 단축키 권고

후보 비교

| 후보 | bash 기본 | zsh 기본 | PSReadLine 기본 | macOS 터미널 | VS Code(Win, Linux) | 선례 |
|---|---|---|---|---|---|---|
| Ctrl+G | emacs `abort`, vi-insert self-insert, vi-command `abort` | emacs `send-break`, viins와 vicmd `list-expand` | Windows 모드 미할당, Emacs와 Vi `Abort` | Option 설정 무관 | 가로챔(Go to Recent Directory) | navi(bash, zsh, PowerShell 전부 Ctrl+G, 확인 소스 navi) |
| Alt+E | emacs 미할당, vi-insert ESC 해석 위험 | emacs 미할당 | 미할당 | Option의 Esc+ 설정 필요(iTerm2 문서 권장, 기본값 미확인) | 셸 전달 | aichat(확인 소스 aichat) |
| Ctrl+] | emacs `character-search` | emacs 미할당 | `GotoBrace`, `CharacterSearch` | 비 US 배열 입력 난이도 미확인 | 미확인 | atuin AI resume(옵트인, 확인 소스 atuin init.rs) |
| 빈 줄 `?` | bash 4.0 이상 가능 | 가능 | 미확인 | 수정 키 불필요 | 영향 없음 | atuin AI(bash, zsh, 확인 소스 atuin-ai shell.rs) |

권고

- (1) 기본 키: Ctrl+G, 세 셸 동일 표기(bash `\C-g`, zsh `^G`, PSReadLine `Ctrl+g`).
- (2) 선정 근거: 단일 Ctrl 조합(macOS Option 설정 무관), PSReadLine Windows 모드 미할당, vi-insert 계열 손실 작음, navi 선례. tty 특수 문자(intr, quit, susp, start, stop, discard 등) 비해당 여부는 stty 기본값 기준 추론, 미확인.
- (3) 충돌 대상: bash `abort`, zsh `send-break`, PSReadLine Emacs와 Vi `Abort` 대체. VS Code 통합 터미널(Windows, Linux) 가로챔. navi 사용자 중복.
- (4) 대응: `howtui init <셸> --key <키>`, `--no-bind`, VS Code 설정 안내 문서화, 같은 키 재바인딩 시 bash 5.1 `bind -r` 선행.
- (5) 대안 키: Alt+E(Linux와 Windows 사용자 선택지, macOS는 Option 설정 필요, vi-insert ESC 지연 영향).
- (6) 선택 기능: 빈 줄 `?` 트리거(atuin AI 선례). PowerShell에서 `?`의 `Where-Object` 별칭 관계와 충돌 여부 미확인, 기본 비활성 권장.

---

## 7. 코드 스케치 (검증 필요)

### 7.1 Bash (`howtui init bash` 출력 예시)

```bash
# howtui init bash 출력 스케치. 실측 검증 전 초안.
# 사용: $HOME/.bashrc 에 eval "$(howtui init bash)"
if [[ $- == *i* ]]; then
  if ((BASH_VERSINFO[0] < 4)); then
    printf 'howtui: bash 4.0 or newer required, key binding skipped\n' >&2
  else

__howtui_widget() {
  local tmp rc out
  tmp=$(command mktemp "${TMPDIR:-/tmp}/howtui.XXXXXX") || return 0
  # 서브셸 안에서만 export, 대화형 셸 환경 오염 없음
  (
    export HOWTUI_SHELL=bash HOWTUI_BUFFER="$READLINE_LINE" HOWTUI_OUTPUT_FILE="$tmp"
    # bash 5.0 이상: READLINE_POINT 문자 단위. 4.x: 바이트 단위라 LBUFFER 미전달
    if ((BASH_VERSINFO[0] >= 5)); then
      export HOWTUI_LBUFFER="${READLINE_LINE:0:READLINE_POINT}"
    fi
    exec howtui --widget </dev/tty >/dev/tty 2>/dev/tty
  )
  rc=$?
  if ((rc == 0)) && [[ -s $tmp ]]; then
    out=$(<"$tmp")                 # 후행 개행 제거
    READLINE_LINE=$out
    READLINE_POINT=0x7fffffff      # 줄 끝, bash가 길이로 보정 (fzf 관용구)
  fi
  command rm -f -- "$tmp"
  return 0                         # 124 반환 금지 (전체 재그림 의미)
}

__howtui_bind() {
  local key=${1-'\C-g'} km
  [[ -n $key ]] || return 0
  for km in emacs vi-insert vi-command; do
    # bash 5.1: 기존 bind -x 덮어쓰기 결함 회피 (atuin 방식)
    if ((BASH_VERSINFO[0] == 5 && BASH_VERSINFO[1] == 1)); then
      bind -m "$km" -r "$key"
    fi
    bind -m "$km" -x "\"$key\": __howtui_widget"
  done
}
__howtui_bind '\C-g'   # --key 값 삽입 위치, --no-bind 시 이 줄 생략
unset -f __howtui_bind

  fi
fi
```

### 7.2 Zsh (`howtui init zsh` 출력 예시)

```zsh
# howtui init zsh 출력 스케치. 실측 검증 전 초안.
# 사용: $HOME/.zshrc 에 eval "$(howtui init zsh)"
if [[ -o interactive ]]; then

__howtui_widget() {
  emulate -L zsh
  local tmp rc
  tmp=$(command mktemp "${TMPDIR:-/tmp}/howtui.XXXXXX") || return 0
  # 위젯 표준 입력은 /dev/null 고정이므로 /dev/tty 명시 필수
  HOWTUI_SHELL=zsh \
  HOWTUI_BUFFER=$BUFFER \
  HOWTUI_LBUFFER=$LBUFFER \
  HOWTUI_OUTPUT_FILE=$tmp \
    command howtui --widget </dev/tty >/dev/tty 2>/dev/tty
  rc=$?
  if (( rc == 0 )) && [[ -s $tmp ]]; then
    BUFFER=$(<$tmp)
    CURSOR=$#BUFFER
  fi
  command rm -f -- $tmp
  # howtui가 bracketed paste를 끈 경우 대비 재활성 (atuin 방식)
  if (( ${+zle_bracketed_paste} )); then
    print -rn -- "${zle_bracketed_paste[1]}" >/dev/tty
  fi
  zle reset-prompt                 # howtui가 원래 행과 열로 커서 복원한 전제
  return 0
}
zle -N howtui-widget __howtui_widget

() {
  local key=${1-'^G'}              # --key 값 삽입 위치
  [[ -n $key ]] || return 0
  bindkey -M emacs "$key" howtui-widget
  bindkey -M viins "$key" howtui-widget
  bindkey -M vicmd "$key" howtui-widget
} '^G'

fi
```

### 7.3 PowerShell (`howtui init powershell` 출력 예시)

init 스크립트 ASCII 전용 원칙(4.5절)에 따라 주석도 영문.

```powershell
# howtui init powershell output sketch. Not yet verified. ASCII only.
# Usage in $PROFILE:  howtui init powershell | Out-String | Invoke-Expression
$__howtuiRl = Get-Module PSReadLine
if ($null -eq $__howtuiRl -or $__howtuiRl.Version -lt [version]'2.0.0') {
    Write-Warning 'howtui: PSReadLine 2.0+ not loaded; key binding skipped'
    return
}

function global:Invoke-HowtuiWidget {
    $line = $null; $cursor = $null
    [Microsoft.PowerShell.PSConsoleReadLine]::GetBufferState([ref]$line, [ref]$cursor)
    $tmp = [System.IO.Path]::GetTempFileName()
    try {
        $exe = (Get-Command howtui -CommandType Application -ErrorAction Stop |
                Select-Object -First 1).Source
        $psi = New-Object System.Diagnostics.ProcessStartInfo
        $psi.FileName = $exe
        $psi.Arguments = '--widget'
        $psi.UseShellExecute = $false
        # No Redirect* flags: stdin, stdout and stderr stay attached to the console.
        $psi.WorkingDirectory = (Get-Location -PSProvider FileSystem).ProviderPath
        $psi.EnvironmentVariables['HOWTUI_SHELL'] = 'powershell'
        $psi.EnvironmentVariables['HOWTUI_BUFFER'] = $line
        $psi.EnvironmentVariables['HOWTUI_LBUFFER'] = $line.Substring(0, $cursor)
        $psi.EnvironmentVariables['HOWTUI_OUTPUT_FILE'] = $tmp
        $p = [System.Diagnostics.Process]::Start($psi)
        $p.WaitForExit()
        $rc = $p.ExitCode

        # Resync PSReadLine's prompt row after the TUI (atuin approach).
        $extra = (Get-PSReadLineOption).ExtraPromptLineCount
        $y = [Console]::CursorTop - $extra
        $y = [Math]::Max(0, [Math]::Min($y, [Console]::BufferHeight - 1))
        [Microsoft.PowerShell.PSConsoleReadLine]::InvokePrompt($null, $y)

        if ($rc -eq 0) {
            $utf8 = New-Object System.Text.UTF8Encoding($false)
            $new = [System.IO.File]::ReadAllText($tmp, $utf8)
            $new = $new.Replace("`r`n", "`n").TrimEnd("`n")
            if ($new.Length -gt 0) {
                # Replace validates the range before touching the buffer.
                [Microsoft.PowerShell.PSConsoleReadLine]::Replace(0, $line.Length, $new)
            }
        }
    }
    catch {
        # Keep the original buffer untouched on any failure.
    }
    finally {
        Remove-Item -LiteralPath $tmp -Force -ErrorAction SilentlyContinue
    }
}

$__howtuiKey = 'Ctrl+g'   # --key value goes here; omit the block below with --no-bind
if ((Get-PSReadLineOption).EditMode -eq 'Vi') {
    Set-PSReadLineKeyHandler -Chord $__howtuiKey -ViMode Insert  -BriefDescription 'howtui' -ScriptBlock { Invoke-HowtuiWidget }
    Set-PSReadLineKeyHandler -Chord $__howtuiKey -ViMode Command -BriefDescription 'howtui' -ScriptBlock { Invoke-HowtuiWidget }
} else {
    Set-PSReadLineKeyHandler -Chord $__howtuiKey -BriefDescription 'howtui' -ScriptBlock { Invoke-HowtuiWidget }
}
Remove-Variable __howtuiRl, __howtuiKey -ErrorAction SilentlyContinue
```

- Vi 모드에서 `-ViMode` 미지정 바인딩의 적용 범위: 미확인. 스케치는 Insert와 Command 양쪽 지정.
- `ProcessStartInfo.EnvironmentVariables`에 빈 문자열 대입 시 변수 존재 여부: 미확인. 바이너리 측은 미설정과 빈 값 동일 취급.

### 7.4 howtui 바이너리 측 골격 (Rust 의사 코드)

```rust
// howtui --widget 모드 골격 스케치. API 이름은 crossterm 0.29, ratatui 0.30 기준 가정.
fn run_widget() -> ExitCode {
    let shell = env::var("HOWTUI_SHELL").ok();                 // bash | zsh | powershell
    let buffer = env::var("HOWTUI_BUFFER").unwrap_or_default();
    let lbuffer = env::var("HOWTUI_LBUFFER").ok();             // bash 4.x 에서는 None
    let Some(out_path) = env::var_os("HOWTUI_OUTPUT_FILE") else { return ExitCode::from(2) };

    // 위젯 모드: 셸 스크립트가 stdout 을 터미널(/dev/tty 또는 콘솔)로 연결한 전제
    if !io::stdout().is_terminal() { return ExitCode::from(2); }

    install_panic_hook_restoring_terminal();                   // raw 모드 해제, 커서 표시, 모드 원복
    let (x0, _y0) = crossterm::cursor::position()?;            // Unix: stdout 으로 ESC[6n
    let moved_down = x0 > 0;                                    // bash 는 항상 0열
    if moved_down { print!("\r\n"); }
    let paste_was_on = query_bracketed_paste_state();          // DECRQM ?2004, 실패 시 None

    let mut term = Terminal::with_options(
        CrosstermBackend::new(io::stdout()),
        TerminalOptions { viewport: Viewport::Inline(HEIGHT) },
    )?;
    let outcome = app_loop(&mut term, &buffer, lbuffer.as_deref()); // Accept(cmd) | Cancel | Error

    let top = term.get_frame().area().y;                        // 뷰포트 첫 행
    restore_screen(top, moved_down, x0);                        // 첫 행 이동, ESC[J, 원래 행과 열 복귀
    restore_modes(paste_was_on);

    match outcome {
        Outcome::Accept(cmd) => {
            let cmd = cmd.replace("\r\n", "\n");
            let cmd = cmd.trim_end_matches('\n');
            fs::write(&out_path, cmd.as_bytes())?;              // UTF-8, BOM 없음
            ExitCode::from(0)
        }
        Outcome::Cancel => ExitCode::from(1),
        Outcome::Error(e) => { eprintln!("howtui: {e}"); ExitCode::from(3) }
    }
}
```

- `term.get_frame().area()`의 뷰포트 좌표 취득 방식과 Windows의 줄바꿈 처리 세부: 미확인(구현 단계 검증).

---

## 8. 검증 과제 목록

- (1) bash 4.4 이상에서 TUI 종료 후 마지막 프롬프트 줄 재그림 결과, 다중 줄 프롬프트 상단 보존 여부
- (2) bash 4.1-4.3의 `rl_crlf` 경로 동일 여부
- (3) bash 핸들러 안 터미널 모드(readline 입력 모드 유지 여부) 직접 확인
- (4) 멀티라인 `READLINE_LINE` 표시, Enter 실행, 히스토리 기록 형태
- (5) zsh `zle reset-prompt`와 커서 복원 규약 조합의 다중 줄 프롬프트, 다중 줄 버퍼 재그림 결과
- (6) zsh `$(<file)` 후행 개행 처리, `ZSH_VERSION` 환경 노출 여부
- (7) PowerShell 키 핸들러 안 `& exe`의 stderr 리다이렉션 여부(TUI 도달 여부)
- (8) 한국어 Windows(5.1, 7.x)의 핸들러 안 `[Console]::OutputEncoding` 실제 값과 캡처 깨짐 재현
- (9) `InvokePrompt($null, $y)` 계산의 conhost와 Windows Terminal 차이, 화면 하단 스크롤 상황
- (10) PSReadLine Vi 모드의 `-ViMode` 미지정 바인딩 범위
- (11) raw 모드 진입 전 Ctrl+C 입력 시 PowerShell 핸들러 중단 동작
- (12) VS Code `-workbench.action.terminal.goToRecentDirectory` 설정 후 Ctrl+G 셸 전달 여부
- (13) macOS Terminal.app와 iTerm2의 Option 키 기본값, Ctrl+G 전달 여부
- (14) tmux 3.5 미만 escape-time 기본값, GNU screen 동작, 고지연 SSH의 DSR 시간 초과
- (15) Windows 레거시 콘솔 모드의 VT 지원 여부
- (16) 한국어 IME 조합 중 raw 모드 입력 처리(Windows conhost, Windows Terminal, macOS)
- (17) Windows 10 구형 빌드의 PSReadLine 1.x 탑재 여부
- (18) bash 5.0 이상 `${READLINE_LINE:0:READLINE_POINT}`의 문자 기준 일치 여부(조합형 한글 포함)

---

## 9. 출처

### 9.1 공식 문서

- GNU Bash 매뉴얼(원문 bash/doc/bashref.texi 열람, 게시본)
  - https://www.gnu.org/software/bash/manual/html_node/Bash-Builtins.html
  - https://www.gnu.org/software/bash/manual/html_node/Bash-Variables.html
  - https://www.gnu.org/software/bash/manual/html_node/Command-Substitution.html
  - https://www.gnu.org/software/bash/manual/html_node/Simple-Command-Expansion.html
  - https://www.gnu.org/software/bash/manual/html_node/Readline-Init-File-Syntax.html
- zsh 매뉴얼 zshzle: https://zsh.sourceforge.io/Doc/Release/Zsh-Line-Editor.html
- PSReadLine 모듈: https://learn.microsoft.com/en-us/powershell/module/psreadline/?view=powershell-7.5
- Set-PSReadLineKeyHandler: https://learn.microsoft.com/en-us/powershell/module/psreadline/set-psreadlinekeyhandler?view=powershell-7.5
- about_Character_Encoding: https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_character_encoding?view=powershell-7.5
- about_Preference_Variables
  - https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_preference_variables?view=powershell-5.1
  - https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_preference_variables?view=powershell-7.5
  - https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_preference_variables?view=powershell-7.7
- about_Profiles: https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_profiles?view=powershell-7.5
- Console Virtual Terminal Sequences: https://learn.microsoft.com/en-us/windows/console/console-virtual-terminal-sequences
- VS Code Terminal Advanced: https://code.visualstudio.com/docs/terminal/advanced
- Apple 지원 zsh 기본 셸: https://support.apple.com/en-us/102360
- iTerm2 Profiles Keys: https://iterm2.com/documentation-preferences-profiles-keys.html
- tmux CHANGES: https://raw.githubusercontent.com/tmux/tmux/master/CHANGES
- bug-bash 2021-04 (atuin 주석 인용): https://lists.gnu.org/archive/html/bug-bash/2021-04/msg00135.html

### 9.2 소스 저장소 (1.2절 커밋 기준)

- bash: https://git.savannah.gnu.org/git/bash.git (master 9c465866b7d8, 태그 bash-4.0, bash-4.4)
  - NEWS, CHANGES, bashline.c(bash_execute_unix_command, readline_get_char_offset, readline_set_char_offset, maybe_make_readline_line, initialize_readline), lib/readline/display.c, lib/readline/emacs_keymap.c, lib/readline/vi_keymap.c, lib/readline/doc/rluser.texi, doc/bashref.texi
- zsh: https://github.com/zsh-users/zsh (fc3cff4f8e28)
  - Doc/Zsh/zle.yo, Src/Zle/zle_bindings.c, Src/Zle/zle_keymap.c, Src/Zle/zle_main.c, Src/Zle/zle_refresh.c, Src/Zle/zle_thingy.c, NEWS
- fzf: https://github.com/junegunn/fzf (ac28b8c3ba27)
  - shell/key-bindings.bash, shell/key-bindings.zsh, src/tui/light.go, src/tui/light_unix.go, src/tui/light_windows.go, install
- atuin: https://github.com/atuinsh/atuin (92a8b9ae1d0b)
  - crates/atuin/src/shell/atuin.bash, atuin.zsh, atuin.ps1, crates/atuin/src/command/client/init.rs, init/bash.rs, init/zsh.rs, search.rs, search/interactive.rs, import.rs, crates/atuin-common/src/shell.rs, crates/atuin-ai/src/shell.rs, install.sh
- zoxide: https://github.com/ajeetdsouza/zoxide (d48a7ab5d7cb)
  - templates/powershell.txt, templates/bash.txt, templates/zsh.txt, README.md, install.sh
- PSFzf: https://github.com/kelleyma49/PSFzf (7e9871df7b58)
  - PSFzf.Base.ps1, PSFzf.PSConsoleReadLineWrappers.ps1
- PSReadLine: https://github.com/PowerShell/PSReadLine (a2d143f78904, 태그 v2.0.0)
  - PSReadLine/PublicAPI.cs, ReadLine.cs, BasicEditing.cs, PlatformWindows.cs, KeyBindings.cs, KeyBindings.vi.cs, Cmdlets.cs, Options.cs, Changes.txt, PSReadLine.psd1, PSReadLine.csproj
- starship: https://github.com/starship/starship (ae8fecf316ed)
  - src/init/mod.rs, src/init/starship.ps1, install/install.sh, README.md
- bash-preexec: https://github.com/rcaloras/bash-preexec (5ae4758c36e8)
  - bash-preexec.sh
- crossterm: https://github.com/crossterm-rs/crossterm (db287b8c6873)
  - src/cursor/sys/unix.rs, src/cursor/sys/windows.rs, src/terminal/sys/unix.rs, src/terminal/sys/windows.rs, src/terminal/sys/file_descriptor.rs, src/event/source/windows.rs, src/ansi_support.rs
- crossterm_winapi 0.9.1(로컬 cargo 레지스트리): src/handle.rs
- ratatui: https://github.com/ratatui/ratatui (f57b06ffb720)
  - ratatui-core/src/terminal/init.rs, ratatui-core/src/terminal/inline.rs, ratatui-crossterm/src/lib.rs

### 9.3 단일 파일 조회 (GitHub API, 2026-10-22)

- PowerShell: https://github.com/PowerShell/PowerShell/blob/master/src/System.Management.Automation/engine/NativeCommandProcessor.cs
- Rust 표준 라이브러리: https://github.com/rust-lang/rust/blob/master/library/std/src/sys/stdio/windows.rs
- VS Code
  - https://github.com/microsoft/vscode/blob/main/src/vs/workbench/contrib/terminal/common/terminal.ts
  - https://github.com/microsoft/vscode/blob/main/src/vs/workbench/contrib/terminal/terminalContribExports.ts
  - https://github.com/microsoft/vscode/blob/main/src/vs/workbench/contrib/terminalContrib/history/common/terminal.history.ts
  - https://github.com/microsoft/vscode/blob/main/src/vs/workbench/contrib/terminalContrib/history/browser/terminal.history.contribution.ts
- Windows Terminal: https://github.com/microsoft/terminal/blob/main/src/cascadia/TerminalSettingsModel/defaults.json
- navi: https://github.com/denisidoro/navi/tree/master/shell (navi.plugin.bash, navi.plugin.zsh, navi.plugin.ps1)
- aichat: https://github.com/sigoden/aichat/tree/main/scripts/shell-integration (integration.bash, integration.zsh, integration.ps1)
