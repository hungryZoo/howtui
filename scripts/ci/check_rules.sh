#!/usr/bin/env sh
# 소스 규칙 검사 (SRS-MOD-002, SRS-MOD-008, SRS-AUTH-014, SRS-SAFE-004). 위반 시 종료 코드 1.
set -eu
cd "$(dirname "$0")/../.."
fail=0

# 주석 줄 제외 grep
code_grep() { grep -rnE "$1" $2 | grep -vE '^[^:]+:[0-9]+:[[:space:]]*//' || true; }

# (1) app 코어는 터미널, 네트워크, 파일 시스템 crate 미사용
if [ -n "$(code_grep '\b(crossterm|ratatui|reqwest|tokio::net|std::fs)\b' src/app)" ]; then
    code_grep '\b(crossterm|ratatui|reqwest|tokio::net|std::fs)\b' src/app
    echo "rule: src/app must not use crossterm, ratatui, reqwest, tokio::net or std::fs"; fail=1
fi
# (2) ratatui init/restore 금지 (stdout 고정 출력)
if [ -n "$(code_grep 'ratatui::(init|restore|try_init|try_restore)\b' src)" ]; then
    echo "rule: ratatui::init/restore are hard-wired to stdout"; fail=1
fi
# (3) 비공식 인증 경로 문자열 금지
if [ -n "$(code_grep '\.codex|backend-api|cookies\.sqlite' src)" ]; then
    echo "rule: unofficial auth paths are forbidden"; fail=1
fi
# (4) 프로세스 실행 위치 제한 (브라우저 실행 1곳)
if [ -n "$(code_grep 'process::Command|Command::new\(' src)" ]; then
    code_grep 'process::Command|Command::new\(' src
    echo "rule: no direct process execution in src (browser opening goes through the open crate)"; fail=1
fi
exit $fail
