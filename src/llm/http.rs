//! HTTP 클라이언트 구성 (SRS-MOD-005, SRS-SEC-001, SRS-SEC-002).

use std::sync::Once;
use std::time::Duration;

pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// rustls ring 암호 공급자 설치 (미설치 시 reqwest Client 생성 panic).
pub fn install_crypto() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

/// 인증서 검증 고정, 시스템 프록시 사용, 연결 제한 시간 10초.
pub fn client() -> reqwest::Client {
    install_crypto();
    reqwest::Client::builder()
        .user_agent(concat!("howtui/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(CONNECT_TIMEOUT)
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}
