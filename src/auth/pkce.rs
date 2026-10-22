//! PKCE S256, state, nonce, host ID 생성 (SRS 11.1절).

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};

pub fn random_bytes<const N: usize>() -> [u8; N] {
    let mut buf = [0u8; N];
    getrandom::fill(&mut buf).expect("OS random source unavailable");
    buf
}

/// 32바이트 난수의 base64url (패딩 없음).
pub fn random_token() -> String {
    URL_SAFE_NO_PAD.encode(random_bytes::<32>())
}

/// code_challenge = base64url(SHA-256(verifier)), 패딩 없음.
pub fn challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

/// `urn:uuid:<UUIDv4>` 형식 host ID.
pub fn new_host_id() -> String {
    let mut b = random_bytes::<16>();
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let hex: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!(
        "urn:uuid:{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// 상수 시간 비교 (state 검증).
pub fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.bytes()
        .zip(b.bytes())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC 7636 부록 B 테스트 벡터.
    #[test]
    fn rfc7636_vector() {
        assert_eq!(
            challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn tokens_are_url_safe_and_unique() {
        let a = random_token();
        let b = random_token();
        assert_ne!(a, b);
        assert_eq!(a.len(), 43);
        assert!(
            a.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        );
    }

    #[test]
    fn host_id_is_uuid_v4() {
        let id = new_host_id();
        let uuid = id.strip_prefix("urn:uuid:").unwrap();
        let parts: Vec<&str> = uuid.split('-').collect();
        assert_eq!(
            parts.iter().map(|p| p.len()).collect::<Vec<_>>(),
            vec![8, 4, 4, 4, 12]
        );
        assert!(parts[2].starts_with('4'));
        assert!(matches!(
            parts[3].chars().next(),
            Some('8' | '9' | 'a' | 'b')
        ));
    }

    #[test]
    fn constant_time_compare() {
        assert!(constant_time_eq("abc", "abc"));
        assert!(!constant_time_eq("abc", "abd"));
        assert!(!constant_time_eq("abc", "ab"));
    }
}
