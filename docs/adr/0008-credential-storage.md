# ADR-0008 자격증명 저장: OS 보안 저장소 우선, 0600 파일 대체

- 상태: 제안
- 일자: 2026-10-22
- 관련: PRD-AUTH-008, PRD-PRIV-006, SRS-MOD-006, SRS 11.2절

## 맥락

- 지시서: 자격증명은 OS 보안 저장소 우선 검토, 로그와 저장소 노출 금지.
- SIWC 문서: 0600 권한 원자적 파일 저장 허용 (research/chatgpt-oauth.md 9.2절).
- Raspberry Pi, 헤드리스 Linux에는 Secret Service 데몬 부재가 일반적.

## 결정

- keyring 4.2 (macOS Keychain, Windows Credential Manager, Linux Secret Service via zbus) 우선 사용. 서비스 이름 `howtui`, 계정 `chatgpt:<client_id>`, 값은 토큰 JSON.
- 저장소 사용 불가(NoDefaultStore, 2초 제한 시간 초과) 시 데이터 디렉터리 `credentials.json` 사용: 디렉터리 0700, 파일 0600, 임시 파일 기록 후 이름 변경. 최초 사용 시 1회 고지.
- 비밀 아닌 상태(host ID, client 매핑, 캐시)는 `state.json`에 분리 저장.
- 갱신 직렬화는 `auth.lock` 파일의 `std::fs::File::lock` (Rust 1.89 안정화) 사용.

## 근거

- keyring 4.2는 순수 Rust(zbus) 구현, libdbus 불필요, armv7과 aarch64, Windows, macOS `cargo check` 통과 (research/rust-crates.md 6절).
- Secret Service 부재 시 즉시 오류 반환 실측 (1-28ms).

## 결과

- Linux 바이너리 크기 약 1.9 MB, 의존 crate 약 60개 증가. Phase 1 크기 측정 후 재검토 가능.

## 대안

- (a) 파일 저장 단독: SIWC 문서 기준 허용, 지시서의 OS 보안 저장소 우선 조건 미충족으로 기각.
- (b) Linux keyutils: 재부팅 시 소멸, 기각.
