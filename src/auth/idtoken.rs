//! ID 토큰 검증 (SRS-AUTH-004): RS256 서명(ring), iss, aud, exp, iat, nonce, azp.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::Deserialize;

use super::state::Jwk;

const CLOCK_SKEW: u64 = 5;

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct IdClaims {
    #[serde(default)]
    pub iss: String,
    #[serde(default)]
    pub aud: Audience,
    #[serde(default)]
    pub exp: u64,
    #[serde(default)]
    pub iat: u64,
    #[serde(default)]
    pub sub: String,
    #[serde(default)]
    pub nonce: Option<String>,
    #[serde(default)]
    pub azp: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum Audience {
    #[default]
    None,
    One(String),
    Many(Vec<String>),
}

impl Audience {
    fn contains(&self, v: &str) -> bool {
        match self {
            Audience::None => false,
            Audience::One(a) => a == v,
            Audience::Many(list) => list.iter().any(|a| a == v),
        }
    }
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum IdTokenError {
    #[error("malformed token")]
    Malformed,
    #[error("unsupported algorithm {0}")]
    Algorithm(String),
    #[error("signing key not found")]
    KeyNotFound,
    #[error("bad signature")]
    Signature,
    #[error("claim check failed: {0}")]
    Claim(&'static str),
}

pub struct Expected<'a> {
    pub issuer: &'a str,
    pub client_id: &'a str,
    pub nonce: &'a str,
    pub now: u64,
}

#[derive(Deserialize)]
struct Header {
    alg: String,
    #[serde(default)]
    kid: Option<String>,
}

fn b64(part: &str) -> Result<Vec<u8>, IdTokenError> {
    URL_SAFE_NO_PAD
        .decode(part.trim_end_matches('='))
        .map_err(|_| IdTokenError::Malformed)
}

/// 서명 확인 없이 헤더의 kid 추출 (JWKS 캐시 선택용).
pub fn key_id(token: &str) -> Option<String> {
    let header = token.split('.').next()?;
    serde_json::from_slice::<Header>(&b64(header).ok()?)
        .ok()?
        .kid
}

pub fn verify(token: &str, keys: &[Jwk], expected: &Expected) -> Result<IdClaims, IdTokenError> {
    let mut parts = token.split('.');
    let (Some(h), Some(p), Some(s), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(IdTokenError::Malformed);
    };
    let header: Header = serde_json::from_slice(&b64(h)?).map_err(|_| IdTokenError::Malformed)?;
    if header.alg != "RS256" {
        return Err(IdTokenError::Algorithm(header.alg));
    }
    let key = keys
        .iter()
        .filter(|k| k.kty == "RSA")
        .find(|k| header.kid.is_none() || k.kid == header.kid)
        .ok_or(IdTokenError::KeyNotFound)?;
    let n = b64(&key.n)?;
    let e = b64(&key.e)?;
    let signature = b64(s)?;
    let message = format!("{h}.{p}");
    ring::signature::RsaPublicKeyComponents {
        n: n.as_slice(),
        e: e.as_slice(),
    }
    .verify(
        &ring::signature::RSA_PKCS1_2048_8192_SHA256,
        message.as_bytes(),
        &signature,
    )
    .map_err(|_| IdTokenError::Signature)?;

    let claims: IdClaims = serde_json::from_slice(&b64(p)?).map_err(|_| IdTokenError::Malformed)?;
    if claims.iss != expected.issuer {
        return Err(IdTokenError::Claim("iss"));
    }
    if !claims.aud.contains(expected.client_id) {
        return Err(IdTokenError::Claim("aud"));
    }
    if claims.exp + CLOCK_SKEW < expected.now {
        return Err(IdTokenError::Claim("exp"));
    }
    if claims.iat > expected.now + CLOCK_SKEW {
        return Err(IdTokenError::Claim("iat"));
    }
    if claims.sub.is_empty() {
        return Err(IdTokenError::Claim("sub"));
    }
    if claims.nonce.as_deref() != Some(expected.nonce) {
        return Err(IdTokenError::Claim("nonce"));
    }
    if let Some(azp) = &claims.azp
        && azp != expected.client_id
    {
        return Err(IdTokenError::Claim("azp"));
    }
    Ok(claims)
}

#[cfg(any(test, feature = "mock-provider"))]
pub mod test_support {
    //! 테스트용 RSA 키(PKCS#8 DER)로 ID 토큰 서명.
    use super::*;

    pub const TEST_KEY: &[u8] = include_bytes!("../../tests/fixtures/test_rsa_pkcs8.der");

    pub fn jwk() -> Jwk {
        let pair = ring::rsa::KeyPair::from_pkcs8(TEST_KEY).unwrap();
        let public = pair.public();
        let comp: ring::rsa::PublicKeyComponents<Vec<u8>> = public.into();
        Jwk {
            kid: Some("k1".into()),
            kty: "RSA".into(),
            alg: Some("RS256".into()),
            n: URL_SAFE_NO_PAD.encode(&comp.n),
            e: URL_SAFE_NO_PAD.encode(&comp.e),
        }
    }

    pub fn sign(claims: &serde_json::Value, kid: &str) -> String {
        let pair = ring::rsa::KeyPair::from_pkcs8(TEST_KEY).unwrap();
        let header = URL_SAFE_NO_PAD
            .encode(serde_json::json!({"alg": "RS256", "kid": kid, "typ": "JWT"}).to_string());
        let payload = URL_SAFE_NO_PAD.encode(claims.to_string());
        let message = format!("{header}.{payload}");
        let mut sig = vec![0u8; pair.public().modulus_len()];
        pair.sign(
            &ring::signature::RSA_PKCS1_SHA256,
            &ring::rand::SystemRandom::new(),
            message.as_bytes(),
            &mut sig,
        )
        .unwrap();
        format!("{message}.{}", URL_SAFE_NO_PAD.encode(sig))
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::*;
    use super::*;
    use serde_json::json;

    fn claims() -> serde_json::Value {
        json!({"iss": "https://auth.openai.com", "aud": ["oaiapp_1"], "exp": 2000, "iat": 1000, "sub": "user-1", "nonce": "n1", "email": "a@example.com"})
    }

    fn expected() -> Expected<'static> {
        Expected {
            issuer: "https://auth.openai.com",
            client_id: "oaiapp_1",
            nonce: "n1",
            now: 1500,
        }
    }

    #[test]
    fn valid_token_passes() {
        let token = sign(&claims(), "k1");
        let c = verify(&token, &[jwk()], &expected()).unwrap();
        assert_eq!(c.sub, "user-1");
        assert_eq!(key_id(&token).as_deref(), Some("k1"));
    }

    #[test]
    fn tampered_payload_fails() {
        let token = sign(&claims(), "k1");
        let mut parts: Vec<&str> = token.split('.').collect();
        let other = URL_SAFE_NO_PAD.encode(json!({"iss": "https://auth.openai.com", "aud": "oaiapp_1", "exp": 2000, "iat": 1000, "sub": "attacker", "nonce": "n1"}).to_string());
        parts[1] = &other;
        assert_eq!(
            verify(&parts.join("."), &[jwk()], &expected()),
            Err(IdTokenError::Signature)
        );
    }

    #[test]
    fn claim_checks() {
        let cases: Vec<(serde_json::Value, &str)> = vec![
            (
                json!({"iss": "https://evil", "aud": "oaiapp_1", "exp": 2000, "iat": 1000, "sub": "u", "nonce": "n1"}),
                "iss",
            ),
            (
                json!({"iss": "https://auth.openai.com", "aud": "other", "exp": 2000, "iat": 1000, "sub": "u", "nonce": "n1"}),
                "aud",
            ),
            (
                json!({"iss": "https://auth.openai.com", "aud": "oaiapp_1", "exp": 1000, "iat": 900, "sub": "u", "nonce": "n1"}),
                "exp",
            ),
            (
                json!({"iss": "https://auth.openai.com", "aud": "oaiapp_1", "exp": 2000, "iat": 1000, "sub": "u", "nonce": "bad"}),
                "nonce",
            ),
            (
                json!({"iss": "https://auth.openai.com", "aud": "oaiapp_1", "exp": 2000, "iat": 1000, "sub": "u", "nonce": "n1", "azp": "x"}),
                "azp",
            ),
        ];
        for (c, name) in cases {
            assert_eq!(
                verify(&sign(&c, "k1"), &[jwk()], &expected()),
                Err(IdTokenError::Claim(name)),
                "{name}"
            );
        }
    }

    #[test]
    fn unknown_key_and_algorithm() {
        let token = sign(&claims(), "other");
        assert_eq!(
            verify(&token, &[jwk()], &expected()),
            Err(IdTokenError::KeyNotFound)
        );
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"none"}"#);
        let payload = URL_SAFE_NO_PAD.encode(claims().to_string());
        assert_eq!(
            verify(&format!("{header}.{payload}."), &[jwk()], &expected()),
            Err(IdTokenError::Algorithm("none".into()))
        );
    }
}
