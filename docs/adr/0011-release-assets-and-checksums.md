# ADR-0011 릴리스 자산 명명과 체크섬 검증

- 상태: 제안
- 일자: 2026-10-22
- 관련: PRD-REL-002, PRD-INS-001, PRD-INS-002, PRD-INS-009, SRS 13.3절, SRS 14절

## 결정

- 태그 `vX.Y.Z`, 압축 파일 `howtui-<X.Y.Z>-<target>.tar.gz` (Windows `.zip`), `SHA256SUMS`(압축 파일과 설치 스크립트 전체), 버전 고정 `install.sh`, `install.ps1` 사본.
- 설치 스크립트는 같은 릴리스의 `SHA256SUMS`로 검증, 불일치 또는 항목 부재 시 설치 중단.
- 릴리스는 초안 생성 후 전체 자산 업로드, 사용자 승인 후 공개. 첫 릴리스 전 immutable releases 활성화.
- 더 강한 검증 수단(P4): 빌드 출처 증명(`actions/attest`), 사용자 검증 명령 `gh attestation verify -R hungryZoo/howtui`, `gh release verify-asset`.

## 근거

- `SHA256SUMS`는 API 할당량 없이 다운로드 가능. GitHub 자산 `digest` 필드는 API 호출 필요 (research/platform-build.md 6절).
- 체크섬 파일과 자산이 같은 저장소에서 제공되므로 체크섬 단독으로는 저장소 탈취 방어 불가. 출처 증명과 immutable releases로 보완.

## 대안

- (a) 체크섬 검증 생략 (uv, bun, deno 설치 스크립트 방식): 지시서 9.1절 조건 미충족으로 기각.
