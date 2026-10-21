# ADR-0013 TLS 백엔드: rustls + ring

- 상태: 제안 (교차 빌드 실측)
- 일자: 2026-10-22
- 관련: SRS-MOD-005, SRS-SEC-001, research/rust-crates.md 5절

## 결정

- reqwest 0.13 `rustls-no-provider` + rustls 0.23 `ring` 기능. 시작 시 ring 암호 공급자 설치.
- 인증서 검증 비활성화 옵션 미제공. 시스템 프록시 설정 사용.

## 근거

- native-tls: armv7, aarch64 교차 빌드 실패(openssl-sys), Linux 배포판별 OpenSSL 1.1과 3 차이.
- aws-lc-rs 대비 바이너리 증가분 작음 (1.46 MB 대 3.47 MB, x86_64 strip, fat LTO).
- ring은 ID 토큰 RS256 검증에도 재사용 (추가 crate 없음).

## 결과

- Windows ARM64 빌드에 clang 필요 (windows-11-arm 러너 구성 확인 필요).
- 인증서 신뢰 저장소: rustls 기본 동작 기준 Phase 1에서 플랫폼별 동작 확인 (사내 프록시 인증서 환경 포함).
