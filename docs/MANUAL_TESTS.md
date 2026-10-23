# 수동 검증 절차 (Phase 1, Phase 2 게이트)

- 작성일: 2026-10-22
- 대상: 자동 테스트로 대체할 수 없는 TC (실계정, Windows 대화형, macOS 대화형)
- 결과 기록: [TC](TC.md) 결과 열과 17절 실행 기록 표

## 1. 준비

```bash
cd ~/dev/howtui
cargo build --release          # target/release/howtui (Mock 기능 미포함)
./target/release/howtui --version
```

Windows는 릴리스 시험 실행 산출물(`dist-all` 아티팩트) 또는 `cargo build --release`의 `target\release\howtui.exe` 사용.

## 2. 실계정 검증 (U-1)

필요 조건: ChatGPT Plus 또는 Pro 계정.

| 순서 | 절차 | 확인 항목 | TC |
| --- | --- | --- | --- |
| (1) | `./target/release/howtui login --no-browser` 실행 | 개인정보 고지, 인가 URL, 붙여넣기 안내, SSH 포트 포워딩 안내 표시 | TC-AUTH-018, TC-AUTH-007 |
| (2) | URL을 브라우저(다른 기기 가능)에서 열고 로그인, 워크스페이스 선택, 에이전트 이름 `howtui` 확인, "Use your ChatGPT plan" 허용 | 동의 화면 문구 기록 | TC-AUTH-022 |
| (3) | 브라우저가 `http://127.0.0.1:<포트>/auth/callback?...` 주소에서 연결 실패하면 주소창 전체를 복사해 터미널에 붙여넣고 Enter | "Signed in as" 표시, 종료 코드 0. 실패 시 출력 전문 기록 | TC-AUTH-008 |
| (4) | `./target/release/howtui auth status` | 마스킹 이메일, 플랜 사용 허용, 만료 시각, 저장소 종류 | TC-AUTH-015 |
| (5) | `./target/release/howtui doctor` | provider 항목 통과, 모델 수와 선택 모델 | TC-DOC-003 |
| (6) | `./target/release/howtui` 실행 후 질문, 직접 편집, 후속 요청, Ctrl+E, Enter | 생성, 수정, 설명 3작업 성공, stdout 출력 | TC-LLM-017 |
| (7) | `./target/release/howtui --no-tui "현재 폴더에서 가장 큰 파일 5개"` | stdout 명령어, stderr 한국어 설명 | TC-LLM-017, PRD-LLM-008 |
| (8) | 1시간 이상 경과 후 다시 (6) 실행 | 자동 갱신 후 정상 동작 | TC-AUTH-022 |
| (9) | `./target/release/howtui logout` | 폐기 성공 또는 미확인 안내 | TC-AUTH-022 |

실측으로 확정할 미확인 항목 (research/chatgpt-oauth.md 13절):

| 항목 | 확인 방법 |
| --- | --- |
| 신규 동적 등록 앱 활성화 여부 | (6)에서 `ChatGPT plan usage is not enabled for howtui` 오류 여부 |
| 한국 지역 허용 여부 | (6)에서 정책 또는 지역 제한 오류 여부 |
| 구조화 출력 지원 여부 | (6) 이후 `~/.local/share/howtui/state.json`의 `structured_output` 값 (`true` 또는 `false`) |
| 리다이렉트 URL 붙여넣기 허용 여부 | (3) 성공 여부 |

문제 발생 시 진단 로그: `HOWTUI_LOG=debug ./target/release/howtui ...` 실행 후 `~/.local/share/howtui/logs/howtui.log` (토큰, 질문, 명령어 미기록).

## 3. Windows 대화형 검증 (U-2)

| 순서 | 절차 | 확인 항목 | TC |
| --- | --- | --- | --- |
| (1) | Windows Terminal의 PowerShell 7에서 `howtui.exe` 실행, 질문, 편집, Enter | 화면 표시, 종료 후 프롬프트 정상, 명령어 출력 | TC-UI-014, TC-PLAT-001 |
| (2) | conhost(`powershell.exe` 단독 창)에서 (1) 반복 | 동일 | TC-UI-014 |
| (3) | `howtui.exe > out.txt` 실행 후 확정, `Get-Content out.txt` | 명령어만 기록 | TC-OUT-009 |
| (4) | `$x = howtui.exe` 실행 후 `$x` | 한글 포함 명령어 손상 여부 기록 (PowerShell 캡처 인코딩) | TC-OUT-009 |
| (5) | 명령어 영역에서 Ctrl+J, Tab, Esc, Ctrl+Z, Ctrl+Y | 키 표와 동일 동작 | TC-KEY-005 |
| (6) | 한국어 Windows에서 `ui.language` 미설정 실행 | 한국어 화면 | TC-I18N-007 |
| (7) | `irm https://raw.githubusercontent.com/hungryZoo/howtui/main/scripts/install.ps1 \| iex` (첫 공개 릴리스 이후) | 설치, PATH 등록, 새 터미널에서 `howtui --version` | TC-INS-005 |

## 4. macOS 대화형 검증 (U-3)

| 순서 | 절차 | 확인 항목 | TC |
| --- | --- | --- | --- |
| (1) | Terminal.app 또는 iTerm2의 Zsh에서 `howtui` 실행, 질문, 편집, Enter | 화면 표시, 종료 후 프롬프트 정상 | TC-PLAT-001 |
| (2) | `cmd=$(howtui); echo "$cmd"` | 명령어만 캡처 | TC-OUT-002 |

## 5. 쉘 통합 대화형 검증 (Phase 2)

준비: `~/.zshrc`(macOS) 또는 `~/.bashrc`(Linux) 끝에 `eval "$(howtui init zsh)"` 또는 `eval "$(howtui init bash)"` 추가 후 새 터미널. 제거는 해당 줄 삭제.

| 순서 | 절차 | 확인 항목 | TC |
| --- | --- | --- | --- |
| (1) | 프롬프트에 `ls` 입력 후 Ctrl+G, 질문, Enter 2회 | 입력란이 생성 명령어로 교체, 자동 실행 없음 | TC-SH-001, TC-SH-002 |
| (2) | 프롬프트에 `howtui 현재 폴더 용량` 입력 후 Enter, 확정 | 다음 프롬프트 입력란에 명령어, 자동 실행 없음 | TC-SH-016 |
| (3) | VS Code 통합 터미널에서 Ctrl+G | 동작 여부 기록 (VS Code 단축키 충돌 시 `terminal.integrated.commandsToSkipShell` 안내 필요) | SRS 9.4절 |
| (4) | Windows PowerShell 5.1과 PowerShell 7에서 `howtui init powershell \| Out-String \| Invoke-Expression` 후 (1), (2) 반복 | 입력란 교체, 다음 입력란 삽입, 한글 명령어 손상 여부 | TC-SH-003 (사용자 확인 완료), TC-SH-013 |
| (5) | 사용 중인 터미널에서 Ctrl+O 후 모델 클릭, 상태줄 `[모델]` 클릭, Shift+드래그 텍스트 선택 | 클릭 동작, 종료 후 일반 마우스 선택과 스크롤 복구 | TC-UI-020 |
