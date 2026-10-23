//! 비밀 아닌 인증 상태 파일 `state.json` (SRS 11.2절). host ID, 등록 정보, 캐시.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::platform::fs::write_private_atomic;

pub const STATE_VERSION: u32 = 1;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Registration {
    pub client_id: String,
    #[serde(default)]
    pub subject: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub created_at: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelInfo {
    pub slug: String,
    #[serde(default)]
    pub display_name: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelCache {
    pub fetched_at: u64,
    pub models: Vec<ModelInfo>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Jwk {
    #[serde(default)]
    pub kid: Option<String>,
    #[serde(default)]
    pub kty: String,
    #[serde(default)]
    pub alg: Option<String>,
    #[serde(default)]
    pub n: String,
    #[serde(default)]
    pub e: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct JwksCache {
    pub fetched_at: u64,
    pub keys: Vec<Jwk>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateFile {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub host_id: Option<String>,
    #[serde(default)]
    pub registration: Option<Registration>,
    #[serde(default)]
    pub plan_notice_shown: bool,
    /// 직전 로그인에서 플랜 scope 미부여 (다음 로그인에 prompt=consent 사용)
    #[serde(default)]
    pub plan_disabled_last: bool,
    #[serde(default)]
    pub credential_store: Option<String>,
    #[serde(default)]
    pub models: Option<ModelCache>,
    /// 설정 화면에서 고른 모델 (CR-005). 설정 파일 값보다 우선
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_model: Option<String>,
    #[serde(default)]
    pub structured_output: Option<bool>,
    #[serde(default)]
    pub jwks: Option<JwksCache>,
}

pub fn state_path(data_dir: &Path) -> PathBuf {
    data_dir.join("state.json")
}

/// 손상 또는 부재 시 기본값.
pub fn load(data_dir: &Path) -> StateFile {
    std::fs::read_to_string(state_path(data_dir))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save(data_dir: &Path, state: &StateFile) -> std::io::Result<()> {
    let mut s = state.clone();
    s.version = STATE_VERSION;
    let bytes = serde_json::to_vec_pretty(&s).map_err(std::io::Error::other)?;
    write_private_atomic(&state_path(data_dir), &bytes)
}

/// host ID 확인 후 없으면 생성하여 저장 (SRS-AUTH-002).
pub fn ensure_host_id(data_dir: &Path) -> std::io::Result<String> {
    let mut state = load(data_dir);
    if let Some(id) = &state.host_id {
        return Ok(id.clone());
    }
    let id = super::pkce::new_host_id();
    state.host_id = Some(id.clone());
    save(data_dir, &state)?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_id_persists() {
        let dir = tempfile::tempdir().unwrap();
        let a = ensure_host_id(dir.path()).unwrap();
        let b = ensure_host_id(dir.path()).unwrap();
        assert_eq!(a, b);
        assert!(a.starts_with("urn:uuid:"));
    }

    #[test]
    fn corrupt_state_falls_back_to_default() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(state_path(dir.path()), b"{not json").unwrap();
        assert_eq!(load(dir.path()), StateFile::default());
    }

    #[test]
    fn roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let s = StateFile {
            registration: Some(Registration {
                client_id: "oaiapp_x".into(),
                ..Default::default()
            }),
            models: Some(ModelCache {
                fetched_at: 5,
                models: vec![ModelInfo {
                    slug: "m".into(),
                    display_name: None,
                }],
            }),
            ..Default::default()
        };
        save(dir.path(), &s).unwrap();
        let loaded = load(dir.path());
        assert_eq!(loaded.registration.unwrap().client_id, "oaiapp_x");
        assert_eq!(loaded.version, STATE_VERSION);
    }
}
