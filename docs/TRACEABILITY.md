# 요구사항 추적성 표

- 생성: `uv run scripts/trace_check.py` (수동 편집 금지, 원본은 MRD, PRD, SRS, TC 문서 표)
- 연결 구조: MRD -> PRD -> SRS -> TC -> 구현 위치 -> 테스트 결과

## 1. 요약

| 항목 | 값 |
| --- | --- |
| MRD 항목 | 77 |
| PRD 요구사항 | 169 |
| SRS 설계 요구사항 | 191 |
| TC | 214 |
| 오류 (누락, 불일치) | 0 |
| 경고 (TC 미연결 SRS) | 0 |

## 2. 검사 결과

- 오류와 경고 없음

## 3. PRD 기준 추적성

| PRD | 단계 | 우선순위 | MRD | SRS | TC | 구현 위치 | TC 결과 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| PRD-CLI-001 | P1 | 필수 | MRD-SCN-001, MRD-SCP-001 | SRS-CLI-001, SRS-CLI-002 | TC-CLI-001 | `src/cli/mod.rs` | 통과 1 |
| PRD-CLI-002 | P1 | 필수 | MRD-SCP-007 | SRS-CLI-005 | TC-CLI-002 | `src/cli/mod.rs` | 통과 1 |
| PRD-CLI-003 | P1 | 필수 | MRD-SCP-004 | SRS-CLI-001 | TC-CLI-003 | `src/cli/mod.rs` | 통과 1 |
| PRD-CLI-004 | P1 | 필수 | MRD-SCP-006 | SRS-CLI-001 | TC-CLI-003 | `src/cli/mod.rs` | 통과 1 |
| PRD-CLI-005 | P1 | 필수 | MRD-SCP-005 | SRS-CLI-003, SRS-OUT-003 | TC-CLI-004 | `src/cli/mod.rs`<br>`src/cli/no_tui.rs` | 통과 1 |
| PRD-CLI-006 | P1 | 권장 | MRD-SCP-001 | SRS-CLI-003, SRS-CLI-006 | TC-CLI-005 | `src/cli/mod.rs` | 통과 1 |
| PRD-CLI-007 | P1 | 필수 | MRD-SCN-001 | SRS-SH-007 | TC-CLI-006 | `src/shell/mod.rs` | 통과 1 |
| PRD-CLI-008 | P2 | 필수 | MRD-SCP-101 | SRS-SH-001 | TC-CLI-007 | `src/shell/init.rs`, `src/shell/scripts/`, `src/cli/mod.rs` | 통과 1 |
| PRD-CLI-009 | P3 | 필수 | MRD-SCP-102 | SRS-CLI-007 | TC-CLI-008 | `src/cli/mod.rs` | 미실행 1 |
| PRD-CLI-010 | P1 | 필수 | MRD-SCP-005 | SRS-CLI-003, SRS-ERR-001 | TC-CLI-009, TC-CLI-010 | `src/cli/mod.rs`<br>`src/llm/types.rs`, `src/cli/` | 통과 2 |
| PRD-CLI-011 | P1 | 필수 | MRD-SCN-001 | SRS-CLI-004 | TC-CLI-003 | `src/cli/mod.rs` | 통과 1 |
| PRD-UI-001 | P1 | 필수 | MRD-SCP-001, MRD-DIF-006 | SRS-ASYNC-006, SRS-PERF-001, SRS-CMP-003 | TC-UI-001, TC-UI-015, TC-UI-014, TC-UI-016, TC-NFR-002 | `src/runtime.rs`<br>`src/ui/terminal.rs`<br>측정 스크립트 | 통과 5 |
| PRD-UI-002 | P1 | 필수 | MRD-SCP-001 | SRS-TERM-007, SRS-TUI-003 | TC-UI-002, TC-UI-004 | `src/ui/inline.rs` | 미실행 1, 통과 1 |
| PRD-UI-003 | P1 | 필수 | MRD-SCP-001 | SRS-TUI-003, SRS-TUI-004 | TC-UI-003 | `src/ui/inline.rs`<br>`tests/`, `src/ui/` | 통과 1 |
| PRD-UI-004 | P1 | 필수 | MRD-SCP-001 | SRS-TUI-003, SRS-TUI-004 | TC-UI-003 | `src/ui/inline.rs`<br>`tests/`, `src/ui/` | 통과 1 |
| PRD-UI-005 | P1 | 필수 | MRD-SCP-001 | SRS-TUI-003, SRS-TUI-004 | TC-UI-003 | `src/ui/inline.rs`<br>`tests/`, `src/ui/` | 통과 1 |
| PRD-UI-006 | P1 | 필수 | MRD-SCP-001 | SRS-TUI-003, SRS-TUI-004 | TC-UI-003 | `src/ui/inline.rs`<br>`tests/`, `src/ui/` | 통과 1 |
| PRD-UI-007 | P1 | 필수 | MRD-SCP-003 | SRS-ASYNC-002, SRS-ASYNC-004, SRS-ASYNC-005, SRS-PERF-002 | TC-UI-005, TC-NFR-003 | `src/runtime.rs` | 통과 2 |
| PRD-UI-008 | P1 | 필수 | MRD-SCP-001 | SRS-TERM-008 | TC-UI-008 | `src/ui/inline.rs` | 통과 1 |
| PRD-UI-009 | P1 | 필수 | MRD-SCP-001 | SRS-TERM-007 | TC-UI-009 | `src/ui/inline.rs` | 통과 1 |
| PRD-UI-010 | P1 | 필수 | MRD-KPI-003 | SRS-TERM-001, SRS-TERM-006, SRS-TEST-006 | TC-UI-010, TC-UI-014, TC-UI-016 | `src/platform/tty.rs`, `src/ui/terminal.rs`<br>`src/ui/terminal.rs`, `src/runtime.rs`<br>`tests/pty/`, `tests/conpty.rs` | 통과 3 |
| PRD-UI-011 | P1 | 필수 | MRD-KPI-001 | SRS-TERM-005 | TC-UI-011 | `src/ui/terminal.rs` | 통과 1 |
| PRD-UI-012 | P1 | 필수 | MRD-SCN-002 | SRS-TUI-005 | TC-UI-007, TC-UI-017 | `src/app/update.rs` | 통과 2 |
| PRD-UI-013 | P1 | 필수 | MRD-SCP-001 | SRS-ARC-001, SRS-DATA-001, SRS-TUI-001 | TC-UI-006 | `src/app/`, `src/runtime.rs`<br>`src/app/state.rs`<br>`src/app/update.rs` | 통과 1 |
| PRD-UI-014 | P1 | 필수 | MRD-EXC-012 | SRS-TUI-013 | TC-UI-013 | `src/ui/`, `README.md` | 통과 1 |
| PRD-UI-015 | P2 | 필수 | MRD-SCN-002 | SRS-LLM-013, SRS-TUI-014 | TC-UI-019 | `src/app/update.rs`, `src/ui/inline.rs`<br>`src/llm/types.rs`, `src/llm/chatgpt.rs`, `src/runtime.rs` | 통과 1 |
| PRD-UI-016 | P2 | 필수 | MRD-SCN-002 | SRS-TUI-015 | TC-UI-020 | `src/runtime.rs`, `src/ui/terminal.rs`, `src/config/mod.rs` | 통과 1 |
| PRD-CTX-001 | P2 | 필수 | MRD-SCN-002, MRD-EXC-006 | SRS-CTX-001, SRS-CTX-002 | TC-CTX-001, TC-CTX-002 | `src/app/update.rs`, `src/ui/inline.rs`, `src/cli/mod.rs`, `src/config/mod.rs`<br>`src/context/mod.rs`, `src/platform/time.rs`, `src/llm/prompt.rs`, `src/runtime.rs`, `src/cli/no_tui.rs` | 통과 2 |
| PRD-UPD-001 | P2 | 필수 | MRD-SCP-103, MRD-SCN-009 | SRS-UPD-001, SRS-UPD-002, SRS-UPD-005 | TC-UPD-001, TC-UPD-005 | `src/ui/inline.rs`, `src/app/update.rs`<br>`src/update/mod.rs`<br>`src/update/mod.rs`, `src/cli/mod.rs` | 통과 2 |
| PRD-UPD-002 | P2 | 필수 | MRD-SCN-009 | SRS-UPD-003, SRS-UPD-004, SRS-UPD-005 | TC-UPD-002, TC-UPD-004, TC-UPD-005 | `src/ui/inline.rs`, `src/app/update.rs`<br>`src/update/mod.rs`<br>`src/update/mod.rs`, `src/cli/mod.rs`, `src/runtime.rs` | 통과 3 |
| PRD-UPD-003 | P2 | 필수 | MRD-SCN-009 | SRS-UPD-002, SRS-UPD-004 | TC-UPD-003, TC-UPD-004 | `src/update/mod.rs`<br>`src/update/mod.rs`, `src/cli/mod.rs`, `src/runtime.rs` | 통과 2 |
| PRD-KEY-001 | P1 | 필수 | MRD-SCP-001, MRD-SCP-002 | SRS-TUI-002, SRS-TUI-005 | TC-KEY-001, TC-KEY-002, TC-KEY-005 | `src/app/update.rs`<br>`src/ui/keymap.rs` | 미실행 1, 통과 2 |
| PRD-KEY-002 | P1 | 권장 | MRD-SCP-002 | SRS-TUI-008 | TC-KEY-003 | `src/ui/terminal.rs` | 미실행 1 |
| PRD-KEY-003 | P1 | 필수 | MRD-SCP-001 | SRS-TUI-012 | TC-KEY-004 | `src/config/mod.rs` | 통과 1 |
| PRD-EDIT-001 | P1 | 필수 | MRD-DIF-001, MRD-SCP-002 | SRS-TUI-011 | TC-EDIT-001 | `src/app/editor.rs`, `src/ui/widgets.rs` | 통과 1 |
| PRD-EDIT-002 | P1 | 필수 | MRD-SCP-002 | SRS-TUI-011 | TC-KEY-002, TC-KEY-005, TC-EDIT-002 | `src/app/editor.rs`, `src/ui/widgets.rs` | 미실행 1, 통과 2 |
| PRD-EDIT-003 | P1 | 필수 | MRD-SCP-002 | SRS-DATA-003 | TC-EDIT-003 | `src/app/editor.rs` | 통과 1 |
| PRD-EDIT-004 | P1 | 필수 | MRD-SCP-002 | SRS-TUI-011 | TC-EDIT-004 | `src/app/editor.rs`, `src/ui/widgets.rs` | 통과 1 |
| PRD-EDIT-005 | P1 | 필수 | MRD-SCP-002 | SRS-DATA-002, SRS-TUI-011 | TC-EDIT-005 | `src/app/editor.rs`<br>`src/app/editor.rs`, `src/ui/widgets.rs` | 통과 1 |
| PRD-EDIT-006 | P1 | 필수 | MRD-DIF-001, MRD-SCN-003 | SRS-DATA-003, SRS-DATA-004 | TC-EDIT-006, TC-EDIT-007 | `src/app/editor.rs`<br>`src/app/update.rs` | 통과 2 |
| PRD-EDIT-007 | P1 | 필수 | MRD-SCN-004 | SRS-TUI-005 | TC-UI-018, TC-EDIT-008 | `src/app/update.rs` | 통과 2 |
| PRD-EDIT-008 | P2 | 필수 | MRD-SCN-005 | SRS-SH-005 | TC-EDIT-009 | `src/cli/widget.rs` | 통과 1 |
| PRD-LLM-001 | P1 | 필수 | MRD-SCP-003 | SRS-MOD-005, SRS-LLM-002, SRS-LLM-004, SRS-SEC-002 | TC-LLM-001, TC-LLM-013, TC-LLM-017, TC-NFR-007 | `src/llm/http.rs`<br>`src/llm/prompt.rs`<br>`src/llm/sse.rs` | 미실행 2, 통과 2 |
| PRD-LLM-002 | P1 | 필수 | MRD-SCP-003, MRD-DIF-001 | SRS-LLM-002 | TC-LLM-002 | `src/llm/prompt.rs` | 통과 1 |
| PRD-LLM-003 | P1 | 필수 | MRD-SCP-003 | SRS-LLM-002 | TC-LLM-003 | `src/llm/prompt.rs` | 통과 1 |
| PRD-LLM-004 | P1 | 필수 | MRD-SCP-003 | SRS-DATA-001, SRS-LLM-010, SRS-ASYNC-001, SRS-ASYNC-003, SRS-ASYNC-004 | TC-LLM-004, TC-NFR-011 | `src/app/state.rs`<br>`src/app/update.rs`<br>`src/llm/chatgpt.rs`<br>`src/main.rs`<br>`src/runtime.rs` | 통과 2 |
| PRD-LLM-005 | P1 | 필수 | MRD-DIF-007 | SRS-LLM-001 | TC-LLM-005 | `src/llm/types.rs` | 통과 1 |
| PRD-LLM-006 | P1 | 필수 | MRD-SCN-001 | SRS-LLM-002 | TC-LLM-001, TC-LLM-006 | `src/llm/prompt.rs` | 통과 2 |
| PRD-LLM-007 | P1 | 필수 | MRD-SCN-003 | SRS-DATA-005 | TC-LLM-002 | `src/app/state.rs`, `src/llm/prompt.rs` | 통과 1 |
| PRD-LLM-008 | P1 | 권장 | MRD-SCN-001 | SRS-LLM-002 | TC-LLM-007 | `src/llm/prompt.rs` | 통과 1 |
| PRD-LLM-009 | P1 | 필수 | MRD-SCP-004 | SRS-LLM-005, SRS-LLM-006, SRS-LLM-013 | TC-LLM-008, TC-LLM-014, TC-LLM-017, TC-LLM-022 | `src/llm/chatgpt.rs`<br>`src/llm/chatgpt.rs`, `src/auth/state.rs`<br>`src/llm/types.rs`, `src/llm/chatgpt.rs`, `src/runtime.rs` | 미실행 1, 통과 3 |
| PRD-LLM-010 | P1 | 필수 | MRD-RSK-003 | SRS-LLM-003, SRS-LLM-008 | TC-LLM-009, TC-LLM-010, TC-LLM-016 | `src/llm/chatgpt.rs`<br>`src/llm/parse.rs`, `src/runtime.rs` | 통과 3 |
| PRD-LLM-011 | P1 | 필수 | MRD-SCP-003 | SRS-LLM-007 | TC-LLM-011 | `src/llm/chatgpt.rs` | 미실행 1 |
| PRD-LLM-012 | P1 | 필수 | MRD-KPI-002 | SRS-MOD-003, SRS-LLM-009, SRS-TEST-001, SRS-TEST-002 | TC-LLM-012, TC-NFR-009 | `src/llm/mock.rs`<br>`src/llm/mock.rs`, `src/auth/oauth.rs`, `src/runtime.rs`<br>`tests/` | 통과 2 |
| PRD-LLM-013 | P5 | 필수 | MRD-SCP-104 | SRS-LLM-011 | TC-LLM-018 | `src/llm/` | 미실행 1 |
| PRD-LLM-014 | P5 | 필수 | MRD-SCP-104 | SRS-LLM-011 | TC-LLM-018 | `src/llm/` | 미실행 1 |
| PRD-LLM-015 | P5 | 필수 | MRD-SCP-104 | SRS-LLM-011 | TC-LLM-018 | `src/llm/` | 미실행 1 |
| PRD-LLM-016 | P5 | 필수 | MRD-SCP-104 | SRS-LLM-011 | TC-LLM-018 | `src/llm/` | 미실행 1 |
| PRD-LLM-017 | P5 | 필수 | MRD-SCP-104 | SRS-LLM-011 | TC-LLM-019 | `src/llm/` | 미실행 1 |
| PRD-LLM-018 | P6 | 필수 | MRD-SCP-105 | SRS-LLM-012 | TC-LLM-020 | `src/llm/local.rs` | 미실행 1 |
| PRD-LLM-019 | P6 | 필수 | MRD-SCP-105 | SRS-LLM-012 | TC-LLM-020 | `src/llm/local.rs` | 미실행 1 |
| PRD-LLM-020 | P6 | 필수 | MRD-SCP-105 | SRS-LLM-012 | TC-LLM-021 | `src/llm/local.rs` | 미실행 1 |
| PRD-AUTH-001 | P1 | 필수 | MRD-SCP-004, MRD-USR-005 | SRS-MOD-005, SRS-AUTH-001, SRS-AUTH-003, SRS-SEC-001 | TC-AUTH-001, TC-AUTH-002, TC-AUTH-003, TC-AUTH-022, TC-NFR-006 | `src/auth/callback.rs`<br>`src/auth/oauth.rs`<br>`src/llm/`, `src/auth/`<br>`src/llm/http.rs` | 미실행 2, 통과 3 |
| PRD-AUTH-002 | P1 | 필수 | MRD-SCP-004 | SRS-AUTH-001 | TC-AUTH-001 | `src/auth/oauth.rs` | 통과 1 |
| PRD-AUTH-003 | P1 | 필수 | MRD-SCP-004 | SRS-AUTH-002 | TC-AUTH-004 | `src/auth/state.rs` | 통과 1 |
| PRD-AUTH-004 | P1 | 필수 | MRD-SCP-004 | SRS-AUTH-004 | TC-AUTH-005, TC-AUTH-006 | `src/auth/idtoken.rs` | 통과 2 |
| PRD-AUTH-005 | P1 | 필수 | MRD-SCN-007, MRD-RSK-004 | SRS-AUTH-005 | TC-AUTH-007, TC-AUTH-009 | `src/auth/mod.rs`, `src/cli/provider.rs` | 미실행 1, 통과 1 |
| PRD-AUTH-006 | P1 | 필수 | MRD-SCN-007 | SRS-AUTH-006 | TC-AUTH-008, TC-AUTH-023, TC-AUTH-025 | `src/auth/callback.rs`, `src/app/update.rs` | 통과 3 |
| PRD-AUTH-007 | P4 | 권장 | MRD-SCN-007 | SRS-AUTH-015 | TC-AUTH-010 | `src/auth/transfer.rs` | 미실행 1 |
| PRD-AUTH-008 | P1 | 필수 | MRD-SCP-004 | SRS-MOD-006, SRS-AUTH-007 | TC-AUTH-011 | `src/auth/store.rs` | 통과 1 |
| PRD-AUTH-009 | P1 | 필수 | MRD-SCP-004 | SRS-AUTH-008 | TC-AUTH-012, TC-AUTH-013, TC-AUTH-022 | `src/auth/session.rs` | 미실행 1, 통과 2 |
| PRD-AUTH-010 | P1 | 필수 | MRD-SCP-004 | SRS-AUTH-009 | TC-AUTH-014, TC-AUTH-022 | `src/auth/session.rs` | 미실행 1, 통과 1 |
| PRD-AUTH-011 | P1 | 필수 | MRD-SCP-004 | SRS-AUTH-010 | TC-AUTH-015 | `src/auth/status.rs` | 통과 1 |
| PRD-AUTH-012 | P1 | 필수 | MRD-RSK-002 | SRS-AUTH-011 | TC-AUTH-006 | `src/auth/oauth.rs` | 통과 1 |
| PRD-AUTH-019 | P1 | 필수 | MRD-RSK-001 | SRS-AUTH-017 | TC-AUTH-024 | `src/llm/chatgpt.rs`, `src/app/update.rs` | 통과 1 |
| PRD-AUTH-013 | P1 | 필수 | MRD-SCN-001 | SRS-TUI-009 | TC-UI-012, TC-AUTH-016 | `src/ui/inline.rs`, `src/runtime.rs` | 미실행 1, 통과 1 |
| PRD-AUTH-014 | P1 | 필수 | MRD-SCP-004 | SRS-TUI-010 | TC-AUTH-017 | `src/ui/inline.rs`, `src/auth/state.rs` | 통과 1 |
| PRD-AUTH-015 | P1 | 필수 | MRD-SCP-004 | SRS-AUTH-013 | TC-AUTH-018 | `src/auth/notice.rs` | 통과 1 |
| PRD-AUTH-016 | P1 | 필수 | MRD-EXC-007 | SRS-LLM-005, SRS-AUTH-014 | TC-AUTH-019 | `src/llm/chatgpt.rs`<br>저장소 전체 | 통과 1 |
| PRD-AUTH-017 | P1 | 필수 | MRD-SCP-004 | SRS-LLM-008 | TC-LLM-015, TC-AUTH-021 | `src/llm/chatgpt.rs` | 통과 2 |
| PRD-AUTH-018 | P1 | 필수 | MRD-SCP-004 | SRS-ERR-004, SRS-AUTH-016 | TC-AUTH-020 | `src/auth/session.rs`<br>`src/llm/chatgpt.rs`, `src/auth/session.rs` | 통과 1 |
| PRD-OUT-001 | P1 | 필수 | MRD-SCP-005, MRD-DIF-005 | SRS-OUT-001 | TC-OUT-001, TC-OUT-009, TC-OUT-012 | `src/output/mod.rs` | 통과 3 |
| PRD-OUT-002 | P1 | 필수 | MRD-SCP-005 | SRS-MOD-008, SRS-TERM-001, SRS-TERM-002, SRS-TERM-003 | TC-OUT-008, TC-SH-017 | `src/platform/tty.rs`<br>`src/platform/tty.rs`, `src/ui/terminal.rs`<br>`src/ui/terminal.rs` | 통과 2 |
| PRD-OUT-003 | P1 | 필수 | MRD-SCP-005 | SRS-OUT-002 | TC-OUT-004 | `src/main.rs` | 통과 1 |
| PRD-OUT-004 | P1 | 필수 | MRD-SCP-005 | SRS-OUT-002 | TC-OUT-005 | `src/main.rs` | 통과 1 |
| PRD-OUT-005 | P1 | 필수 | MRD-SCP-005 | SRS-DATA-006, SRS-OUT-001 | TC-OUT-002 | `src/app/editor.rs`<br>`src/output/mod.rs` | 통과 1 |
| PRD-OUT-006 | P1 | 필수 | MRD-KPI-002 | SRS-TERM-002, SRS-TEST-006 | TC-OUT-001, TC-OUT-002, TC-OUT-003, TC-OUT-007, TC-OUT-009, TC-OUT-012 | `src/platform/tty.rs`<br>`tests/pty/`, `tests/conpty.rs` | 통과 6 |
| PRD-OUT-007 | P1 | 필수 | MRD-SCP-005 | SRS-TERM-004, SRS-CMP-003 | TC-UI-015, TC-OUT-006 | `src/platform/tty.rs`, `src/main.rs`<br>`src/ui/terminal.rs` | 통과 2 |
| PRD-OUT-008 | P1 | 필수 | MRD-SCP-005 | SRS-OUT-003 | TC-CLI-004 | `src/cli/no_tui.rs` | 통과 1 |
| PRD-OUT-009 | P2 | 필수 | MRD-SCP-101 | SRS-OUT-005, SRS-SH-005 | TC-OUT-010, TC-SH-012 | `src/cli/widget.rs`<br>`src/output/mod.rs` | 미실행 1, 통과 1 |
| PRD-OUT-010 | P1 | 필수 | MRD-SCP-005 | SRS-OUT-004 | TC-OUT-011 | `src/main.rs` | 통과 1 |
| PRD-SH-001 | P2 | 필수 | MRD-SCP-101, MRD-DIF-003 | SRS-SH-002, SRS-CMP-002 | TC-SH-001, TC-SH-015 | `src/shell/scripts/`<br>`src/shell/scripts/howtui.bash` | 통과 2 |
| PRD-SH-002 | P2 | 필수 | MRD-SCP-101, MRD-DIF-003 | SRS-SH-003, SRS-CMP-002 | TC-SH-002 | `src/shell/scripts/`<br>`src/shell/scripts/howtui.zsh` | 통과 1 |
| PRD-SH-003 | P2 | 필수 | MRD-SCP-101, MRD-DIF-003 | SRS-SH-004, SRS-SH-011, SRS-CMP-002 | TC-SH-003, TC-SH-013, TC-SH-015, TC-SH-018 | `src/shell/scripts/`<br>`src/shell/scripts/howtui.ps1`<br>`src/shell/scripts/howtui.ps1`, 문서 | 미실행 1, 통과 3 |
| PRD-SH-004 | P2 | 필수 | MRD-SCN-005 | SRS-SH-006 | TC-SH-004 | `src/shell/init.rs` | 통과 1 |
| PRD-SH-005 | P2 | 필수 | MRD-SCN-005 | SRS-SH-005 | TC-SH-005 | `src/cli/widget.rs` | 통과 1 |
| PRD-SH-006 | P2 | 필수 | MRD-DIF-002 | SRS-SH-002, SRS-SH-003, SRS-SH-004 | TC-SH-001, TC-SH-002, TC-SH-003, TC-SH-014 | `src/shell/scripts/howtui.bash`<br>`src/shell/scripts/howtui.ps1`<br>`src/shell/scripts/howtui.zsh` | 통과 4 |
| PRD-SH-007 | P2 | 필수 | MRD-KPI-005 | SRS-SH-005, SRS-SH-009 | TC-SH-005, TC-SH-006 | `src/cli/widget.rs` | 통과 2 |
| PRD-SH-008 | P2 | 필수 | MRD-SCN-006 | SRS-SH-009 | TC-SH-007 | `src/cli/widget.rs` | 통과 1 |
| PRD-SH-009 | P2 | 필수 | MRD-SCP-101 | SRS-SH-001, SRS-SH-010 | TC-SH-008 | `src/shell/init.rs`, `src/shell/scripts/`, `src/cli/mod.rs`<br>`src/shell/scripts/`, `scripts/` | 통과 1 |
| PRD-SH-010 | P1 | 필수 | MRD-EXC-004 | SRS-SH-010, SRS-SH-015, SRS-INS-005, SRS-INS-014 | TC-SH-009 | `scripts/install.sh`<br>`src/shell/rc.rs`, `src/cli/mod.rs`, `scripts/ci/check_rules.sh`<br>`src/shell/scripts/`, `scripts/` | 통과 1 |
| PRD-SH-011 | P2 | 필수 | MRD-SCN-008 | SRS-SH-008 | TC-SH-010 | `src/shell/scripts/`, `src/doctor/` | 통과 1 |
| PRD-SH-012 | P2 | 필수 | MRD-EXC-003 | SRS-SH-012, SRS-CMP-006, SRS-TEST-004 | TC-SH-011 | `README.md`<br>`README.md` 6절<br>`docs/TC.md` | 미실행 1 |
| PRD-SH-013 | P2 | 필수 | MRD-SCN-005 | SRS-SH-013, SRS-SH-014 | TC-SH-016, TC-SH-017 | `src/cli/widget.rs`, `src/shell/scripts/`<br>`src/shell/scripts/` | 통과 2 |
| PRD-SH-014 | P2 | 필수 | MRD-SCN-009, MRD-EXC-004 | SRS-SH-015, SRS-INS-014, SRS-INS-015 | TC-SH-020, TC-SH-021, TC-INS-011 | `scripts/install.ps1`, `src/shell/rc.rs`<br>`scripts/install.sh`<br>`src/shell/rc.rs`, `src/cli/mod.rs`, `scripts/ci/check_rules.sh` | 통과 3 |
| PRD-FS-001 | P3 | 필수 | MRD-SCP-102 | SRS-ARC-003, SRS-TUI-007 | TC-FS-001 | `src/ui/`<br>`src/ui/fullscreen.rs` | 미실행 1 |
| PRD-FS-002 | P3 | 필수 | MRD-SCP-102 | SRS-ARC-001, SRS-ARC-002, SRS-ARC-003, SRS-MOD-001, SRS-MOD-002, SRS-TUI-007 | TC-FS-002, TC-NFR-001 | `src/`<br>`src/app/`<br>`src/app/`, `src/runtime.rs`<br>`src/ui/`<br>`src/ui/fullscreen.rs` | 미실행 1, 통과 1 |
| PRD-FS-003 | P3 | 필수 | MRD-SCN-010 | SRS-TUI-007 | TC-FS-003 | `src/ui/fullscreen.rs` | 미실행 1 |
| PRD-FS-004 | P3 | 필수 | MRD-KPI-003 | SRS-TUI-007 | TC-FS-001, TC-FS-005 | `src/ui/fullscreen.rs` | 미실행 2 |
| PRD-FS-005 | P3 | 필수 | MRD-SCP-102 | SRS-TUI-007 | TC-CLI-008, TC-FS-004 | `src/ui/fullscreen.rs` | 미실행 2 |
| PRD-CFG-001 | P1 | 필수 | MRD-SCP-001 | SRS-CFG-001 | TC-CFG-001 | `src/config/paths.rs` | 통과 1 |
| PRD-CFG-002 | P1 | 필수 | MRD-SCP-001 | SRS-CFG-003 | TC-CFG-002 | `src/config/mod.rs` | 통과 1 |
| PRD-CFG-003 | P1 | 필수 | MRD-SCP-001 | SRS-CFG-002 | TC-CFG-003 | `src/config/mod.rs` | 통과 1 |
| PRD-CFG-004 | P3 | 필수 | MRD-SCP-102 | SRS-CFG-007 | TC-FS-004 | `src/config/mod.rs` | 미실행 1 |
| PRD-CFG-005 | P1 | 필수 | MRD-SCN-008 | SRS-CFG-004 | TC-CFG-004 | `src/config/mod.rs` | 통과 1 |
| PRD-CFG-006 | P1 | 필수 | MRD-SCN-008 | SRS-CFG-005 | TC-CFG-005 | `src/config/mod.rs` | 통과 1 |
| PRD-CFG-007 | P1 | 필수 | MRD-SCP-104 | SRS-CFG-006 | TC-CFG-006 | `src/config/mod.rs` | 통과 1 |
| PRD-DOC-001 | P1 | 필수 | MRD-SCP-006 | SRS-DOC-001 | TC-DOC-001 | `src/doctor/` | 통과 1 |
| PRD-DOC-002 | P1 | 필수 | MRD-SCP-006 | SRS-DOC-001 | TC-DOC-002 | `src/doctor/` | 통과 1 |
| PRD-DOC-003 | P1 | 필수 | MRD-SCP-006 | SRS-DOC-001, SRS-DOC-006 | TC-DOC-003 | `src/doctor/` | 통과 1 |
| PRD-DOC-004 | P1 | 필수 | MRD-SCN-008 | SRS-DOC-002 | TC-DOC-004 | `src/doctor/json.rs` | 통과 1 |
| PRD-DOC-005 | P1 | 필수 | MRD-SCN-008 | SRS-DOC-003 | TC-DOC-005 | `src/doctor/mask.rs` | 통과 1 |
| PRD-DOC-006 | P2 | 필수 | MRD-SCP-101 | SRS-SH-008, SRS-DOC-005 | TC-SH-010 | `src/doctor/`<br>`src/shell/scripts/`, `src/doctor/` | 통과 1 |
| PRD-DOC-007 | P2 | 필수 | MRD-SCP-103 | SRS-DOC-005 | TC-DOC-007 | `src/doctor/` | 통과 1 |
| PRD-DOC-008 | P6 | 필수 | MRD-SCP-105 | SRS-DOC-005 | TC-DOC-008 | `src/doctor/` | 미실행 1 |
| PRD-DOC-009 | P1 | 필수 | MRD-SCN-008 | SRS-DOC-004 | TC-DOC-006 | `src/doctor/mod.rs` | 통과 1 |
| PRD-INS-001 | P1 | 필수 | MRD-SCP-007, MRD-SCN-009 | SRS-INS-001, SRS-INS-003, SRS-INS-004, SRS-INS-005, SRS-INS-006, SRS-INS-007, SRS-INS-014, SRS-SEC-006 | TC-SH-009, TC-INS-001, TC-INS-002, TC-INS-003, TC-INS-011, TC-REL-008 | `scripts/`<br>`scripts/install.sh`<br>`scripts/install.sh`, `scripts/install.ps1` | 통과 6 |
| PRD-INS-002 | P1 | 필수 | MRD-SCP-007 | SRS-INS-008, SRS-INS-009, SRS-INS-010, SRS-INS-015 | TC-INS-005, TC-INS-006, TC-INS-013, TC-REL-008 | `scripts/install.ps1`<br>`scripts/install.ps1`, `src/shell/rc.rs` | 미실행 1, 통과 3 |
| PRD-INS-003 | P1 | 필수 | MRD-PLT-004 | SRS-INS-002 | TC-INS-004 | `scripts/install.sh` | 통과 1 |
| PRD-INS-004 | P1 | 필수 | MRD-SCP-007 | SRS-INS-002, SRS-INS-009 | TC-INS-004 | `scripts/install.ps1`<br>`scripts/install.sh` | 통과 1 |
| PRD-INS-005 | P1 | 필수 | MRD-EXC-005 | SRS-INS-005 | TC-INS-001 | `scripts/install.sh` | 통과 1 |
| PRD-INS-006 | P1 | 필수 | MRD-SCN-009 | SRS-INS-011 | TC-INS-007 | `README.md` | 미실행 1 |
| PRD-INS-007 | P2 | 필수 | MRD-SCN-009 | SRS-INS-012 | TC-INS-008 | `README.md`, `scripts/` | 통과 1 |
| PRD-INS-008 | P2 | 필수 | MRD-SCN-009 | SRS-INS-013 | TC-INS-009, TC-INS-012, TC-INS-013, TC-REL-008 | `scripts/uninstall.sh`, `scripts/uninstall.ps1`, `scripts/install.sh`, `scripts/install.ps1`, `README.md` | 미실행 1, 통과 3 |
| PRD-INS-009 | P1 | 필수 | MRD-SCN-009 | SRS-INS-004, SRS-SEC-006 | TC-INS-003, TC-INS-010 | `scripts/`<br>`scripts/install.sh`, `scripts/install.ps1` | 통과 2 |
| PRD-REL-001 | P1 | 필수 | MRD-SCP-007 | SRS-MOD-004, SRS-BLD-005, SRS-BLD-007, SRS-BLD-008, SRS-SEC-005, SRS-PERF-003, SRS-TEST-005 | TC-REL-001, TC-REL-007, TC-NFR-004, TC-NFR-008, TC-NFR-010, TC-NFR-012 | CI<br>`.github/workflows/`<br>`.github/workflows/ci.yml`<br>`.github/workflows/release.yml`<br>`Cargo.toml`<br>`scripts/trace_check.py` | 미실행 2, 통과 4 |
| PRD-REL-002 | P1 | 필수 | MRD-SCP-007 | SRS-BLD-006 | TC-REL-002 | `.github/workflows/release.yml` | 통과 1 |
| PRD-REL-003 | P4 | 필수 | MRD-SCP-103, MRD-DIF-008 | SRS-BLD-001, SRS-BLD-009 | TC-REL-003 | `.github/workflows/release.yml` | 통과 1 |
| PRD-REL-004 | P1 | 필수 | MRD-SCP-007 | SRS-BLD-001, SRS-BLD-002, SRS-CMP-001 | TC-REL-004 | CI<br>`.github/workflows/`<br>`.github/workflows/release.yml` | 통과 1 |
| PRD-REL-005 | P1 | 필수 | MRD-SCN-009 | SRS-BLD-011 | TC-REL-005 | `.github/release-notes/` | 미실행 1 |
| PRD-REL-006 | P4 | 권장 | MRD-SCN-009 | SRS-BLD-010 | TC-REL-006 | `.github/workflows/release.yml`, 문서 | 미실행 1 |
| PRD-REL-007 | P1 | 필수 | MRD-SCP-007 | SRS-BLD-008 | TC-REL-001 | `.github/workflows/release.yml` | 통과 1 |
| PRD-SAFE-001 | P1 | 필수 | MRD-EXC-001, MRD-DIF-002 | SRS-SAFE-004 | TC-SAFE-001 | 저장소 전체 | 통과 1 |
| PRD-SAFE-002 | P1 | 필수 | MRD-SCP-008, MRD-DIF-009 | SRS-SAFE-001, SRS-SAFE-002, SRS-SAFE-003 | TC-SAFE-002, TC-SAFE-007 | `src/app/update.rs`<br>`src/safety/`<br>`src/safety/lexer.rs` | 통과 2 |
| PRD-SAFE-003 | P1 | 필수 | MRD-SCP-008 | SRS-TUI-006, SRS-SAFE-001, SRS-SAFE-003 | TC-SAFE-003 | `src/app/update.rs`<br>`src/safety/` | 통과 1 |
| PRD-SAFE-004 | P1 | 필수 | MRD-SCP-008 | SRS-CLI-004, SRS-SAFE-006 | TC-SAFE-004 | `src/cli/`, `README.md`<br>`src/cli/mod.rs` | 통과 1 |
| PRD-SAFE-005 | P1 | 필수 | MRD-SCP-008 | SRS-SAFE-007 | TC-SAFE-005 | `src/ui/inline.rs` | 통과 1 |
| PRD-SAFE-006 | P1 | 필수 | MRD-EXC-001 | SRS-SAFE-004 | TC-SAFE-001 | 저장소 전체 | 통과 1 |
| PRD-SAFE-007 | P1 | 필수 | MRD-SCP-008 | SRS-SAFE-005, SRS-TEST-003 | TC-SAFE-006 | `src/safety/`<br>`src/safety/tests.rs` | 통과 1 |
| PRD-PRIV-001 | P1 | 필수 | MRD-EXC-006 | SRS-CTX-001, SRS-SEC-004 | TC-PRIV-001 | `src/context/mod.rs`, `src/platform/time.rs`, `src/llm/prompt.rs`, `src/runtime.rs`, `src/cli/no_tui.rs`<br>`src/llm/prompt.rs` | 통과 1 |
| PRD-PRIV-002 | P1 | 필수 | MRD-EXC-006 | SRS-CTX-001, SRS-SEC-004 | TC-CTX-001, TC-PRIV-001 | `src/context/mod.rs`, `src/platform/time.rs`, `src/llm/prompt.rs`, `src/runtime.rs`, `src/cli/no_tui.rs`<br>`src/llm/prompt.rs` | 통과 2 |
| PRD-PRIV-003 | P1 | 필수 | MRD-EXC-006 | SRS-CLI-004, SRS-AUTH-013 | TC-AUTH-018, TC-PRIV-002 | `src/auth/notice.rs`<br>`src/cli/mod.rs` | 통과 2 |
| PRD-PRIV-004 | P1 | 필수 | MRD-EXC-006 | SRS-SEC-007 | TC-PRIV-002, TC-PRIV-005 | `src/app/update.rs`, `src/llm/prompt.rs` | 미실행 1, 통과 1 |
| PRD-PRIV-005 | P1 | 필수 | MRD-EXC-006 | SRS-OUT-004, SRS-AUTH-012 | TC-OUT-011, TC-PRIV-003 | `src/auth/secret.rs`<br>`src/main.rs` | 통과 2 |
| PRD-PRIV-006 | P1 | 필수 | MRD-SCP-004 | SRS-AUTH-007, SRS-CFG-008, SRS-SEC-003 | TC-CFG-007, TC-PRIV-004 | `src/auth/store.rs`<br>`src/platform/fs.rs` | 통과 2 |
| PRD-PLAT-001 | P1 | 필수 | MRD-PLT-001, MRD-PLT-002, MRD-PLT-003 | SRS-ARC-004, SRS-CMP-004, SRS-CMP-006 | TC-PLAT-001 | `README.md`<br>`src/platform/`, `src/shell/`<br>`src/platform/tty.rs` | 미실행 1 |
| PRD-PLAT-002 | P4 | 필수 | MRD-DIF-008 | SRS-MOD-007, SRS-BLD-001, SRS-BLD-003, SRS-BLD-004, SRS-CMP-005 | TC-REL-003, TC-PLAT-002, TC-PLAT-005 | CI<br>`.cargo/config.toml`<br>`.github/workflows/release.yml` | 미실행 2, 통과 1 |
| PRD-PLAT-003 | P4 | 필수 | MRD-PLT-004 | SRS-BLD-002, SRS-BLD-012, SRS-PERF-004, SRS-CMP-001 | TC-PLAT-002, TC-PLAT-003, TC-NFR-005 | CI<br>Phase 4<br>`.github/workflows/`<br>`README.md`, `docs/` | 미실행 3 |
| PRD-PLAT-004 | P1 | 필수 | MRD-PLT-004 | SRS-BLD-009, SRS-BLD-012, SRS-TEST-004 | TC-PLAT-004 | `.github/workflows/release.yml`<br>`README.md`, `docs/`<br>`docs/TC.md` | 미실행 1 |
| PRD-I18N-001 | P1 | 필수 | MRD-SCP-009 | SRS-I18N-001 | TC-I18N-001, TC-I18N-004 | `src/i18n/` | 통과 2 |
| PRD-I18N-002 | P1 | 필수 | MRD-SCP-009 | SRS-I18N-002 | TC-I18N-002, TC-I18N-004, TC-I18N-007 | `src/i18n/detect.rs` | 미실행 1, 통과 2 |
| PRD-I18N-003 | P1 | 필수 | MRD-SCP-009 | SRS-I18N-001, SRS-I18N-003 | TC-I18N-001, TC-I18N-003 | `src/i18n/` | 통과 2 |
| PRD-I18N-004 | P1 | 권장 | MRD-SCP-009 | SRS-I18N-004 | TC-I18N-005 | `src/cli/mod.rs` | 통과 1 |
| PRD-I18N-005 | P1 | 필수 | MRD-SCP-009 | SRS-I18N-005 | TC-I18N-006 | `src/doctor/json.rs`, `src/output/` | 통과 1 |
| PRD-ERR-001 | P1 | 필수 | MRD-SCN-008 | SRS-ERR-002 | TC-ERR-001 | `src/app/update.rs`, `src/llm/error.rs` | 통과 1 |
| PRD-ERR-002 | P1 | 필수 | MRD-SCP-003 | SRS-ERR-002 | TC-ERR-002 | `src/app/update.rs`, `src/llm/error.rs` | 통과 1 |
| PRD-ERR-003 | P1 | 필수 | MRD-SCP-004 | SRS-ERR-003 | TC-ERR-005 | `src/llm/error.rs` | 통과 1 |
| PRD-ERR-004 | P1 | 필수 | MRD-KPI-003 | SRS-TERM-006 | TC-ERR-004 | `src/ui/terminal.rs`, `src/runtime.rs` | 통과 1 |
| PRD-ERR-005 | P1 | 필수 | MRD-KPI-003 | SRS-TERM-006 | TC-ERR-003 | `src/ui/terminal.rs`, `src/runtime.rs` | 통과 1 |
