//! 자격증명 저장소 (ADR-0008, SRS-MOD-006). OS 보안 저장소 우선, 사용 불가 시 0600 파일.
//!
//! Windows Credential Manager의 항목 크기 제한(2560바이트)을 고려해 토큰별 항목으로 나눠 저장한다.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::secret::Secret;
use crate::platform::fs::write_private_atomic;

const SERVICE: &str = "howtui";
const PROBE_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenSet {
    pub access_token: Secret,
    pub refresh_token: Secret,
    #[serde(default)]
    pub id_token: Option<Secret>,
    #[serde(default)]
    pub token_type: String,
    #[serde(default)]
    pub scope: Vec<String>,
    pub expires_at: u64,
    #[serde(default)]
    pub earliest_refresh_at: Option<u64>,
    #[serde(default)]
    pub saved_at: u64,
}

pub const PLAN_SCOPE: &str = "chatgpt.tokens.use.direct";

impl TokenSet {
    pub fn plan_enabled(&self) -> bool {
        self.scope.iter().any(|s| s == PLAN_SCOPE)
    }

    /// 만료 60초 전 갱신, `earliest_refresh_at` 이전 보류 (만료 후에는 즉시).
    pub fn needs_refresh(&self, now: u64) -> bool {
        if now >= self.expires_at {
            return true;
        }
        let near = now + 60 >= self.expires_at;
        near && self.earliest_refresh_at.is_none_or(|e| now >= e)
    }
}

/// 토큰을 제외한 메타데이터 (보안 저장소 분할 저장용).
#[derive(Serialize, Deserialize)]
struct Meta {
    token_type: String,
    scope: Vec<String>,
    expires_at: u64,
    earliest_refresh_at: Option<u64>,
    saved_at: u64,
    has_id_token: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoreKind {
    Keyring,
    File,
}

impl StoreKind {
    pub fn name(self) -> &'static str {
        match self {
            StoreKind::Keyring => "keyring",
            StoreKind::File => "file",
        }
    }

    pub fn parse(s: &str) -> Option<StoreKind> {
        match s {
            "keyring" => Some(StoreKind::Keyring),
            "file" => Some(StoreKind::File),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct CredentialStore {
    pub kind: StoreKind,
    file: PathBuf,
}

#[derive(Debug, thiserror::Error)]
#[error("credential storage: {0}")]
pub struct StoreError(pub String);

/// OS 보안 저장소 사용 가능 여부 (제한 시간 2초).
pub fn keyring_available() -> bool {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let ok = keyring::Entry::store_status().is_ok()
            && keyring::Entry::new(SERVICE, "probe").is_ok_and(|e| {
                !matches!(
                    e.get_password(),
                    Err(keyring::Error::NoStorageAccess(_))
                        | Err(keyring::Error::PlatformFailure(_))
                )
            });
        let _ = tx.send(ok);
    });
    rx.recv_timeout(PROBE_TIMEOUT).unwrap_or(false)
}

impl CredentialStore {
    pub fn new(kind: StoreKind, data_dir: &Path) -> Self {
        Self {
            kind,
            file: data_dir.join("credentials.json"),
        }
    }

    pub fn file_path(&self) -> &Path {
        &self.file
    }

    fn account(client_id: &str, part: &str) -> String {
        format!("chatgpt:{client_id}:{part}")
    }

    pub fn load(&self, client_id: &str) -> Result<Option<TokenSet>, StoreError> {
        match self.kind {
            StoreKind::File => {
                let text = match std::fs::read_to_string(&self.file) {
                    Ok(t) => t,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
                    Err(e) => return Err(StoreError(e.to_string())),
                };
                let map: std::collections::BTreeMap<String, TokenSet> =
                    serde_json::from_str(&text).map_err(|e| StoreError(e.to_string()))?;
                Ok(map.get(client_id).cloned())
            }
            StoreKind::Keyring => {
                let get = |part: &str| -> Result<Option<String>, StoreError> {
                    let entry = keyring::Entry::new(SERVICE, &Self::account(client_id, part))
                        .map_err(|e| StoreError(e.to_string()))?;
                    match entry.get_password() {
                        Ok(v) => Ok(Some(v)),
                        Err(keyring::Error::NoEntry) => Ok(None),
                        Err(e) => Err(StoreError(e.to_string())),
                    }
                };
                let (Some(meta), Some(access), Some(refresh)) =
                    (get("meta")?, get("access")?, get("refresh")?)
                else {
                    return Ok(None);
                };
                let meta: Meta =
                    serde_json::from_str(&meta).map_err(|e| StoreError(e.to_string()))?;
                let id_token = if meta.has_id_token {
                    get("id")?.map(Secret::new)
                } else {
                    None
                };
                Ok(Some(TokenSet {
                    access_token: Secret::new(access),
                    refresh_token: Secret::new(refresh),
                    id_token,
                    token_type: meta.token_type,
                    scope: meta.scope,
                    expires_at: meta.expires_at,
                    earliest_refresh_at: meta.earliest_refresh_at,
                    saved_at: meta.saved_at,
                }))
            }
        }
    }

    pub fn save(&self, client_id: &str, tokens: &TokenSet) -> Result<(), StoreError> {
        match self.kind {
            StoreKind::File => {
                let mut map: std::collections::BTreeMap<String, TokenSet> =
                    std::fs::read_to_string(&self.file)
                        .ok()
                        .and_then(|t| serde_json::from_str(&t).ok())
                        .unwrap_or_default();
                map.insert(client_id.to_string(), tokens.clone());
                let bytes =
                    serde_json::to_vec_pretty(&map).map_err(|e| StoreError(e.to_string()))?;
                write_private_atomic(&self.file, &bytes).map_err(|e| StoreError(e.to_string()))
            }
            StoreKind::Keyring => {
                let set = |part: &str, value: &str| -> Result<(), StoreError> {
                    keyring::Entry::new(SERVICE, &Self::account(client_id, part))
                        .and_then(|e| e.set_password(value))
                        .map_err(|e| StoreError(e.to_string()))
                };
                let meta = Meta {
                    token_type: tokens.token_type.clone(),
                    scope: tokens.scope.clone(),
                    expires_at: tokens.expires_at,
                    earliest_refresh_at: tokens.earliest_refresh_at,
                    saved_at: tokens.saved_at,
                    has_id_token: tokens.id_token.is_some(),
                };
                // 토큰 먼저, 메타데이터 마지막 (불완전 저장 시 load가 None)
                set("access", tokens.access_token.expose())?;
                set("refresh", tokens.refresh_token.expose())?;
                if let Some(id) = &tokens.id_token {
                    set("id", id.expose())?;
                }
                set(
                    "meta",
                    &serde_json::to_string(&meta).map_err(|e| StoreError(e.to_string()))?,
                )
            }
        }
    }

    pub fn delete(&self, client_id: &str) -> Result<(), StoreError> {
        match self.kind {
            StoreKind::File => {
                let Ok(text) = std::fs::read_to_string(&self.file) else {
                    return Ok(());
                };
                let mut map: std::collections::BTreeMap<String, TokenSet> =
                    serde_json::from_str(&text).unwrap_or_default();
                map.remove(client_id);
                if map.is_empty() {
                    let _ = std::fs::remove_file(&self.file);
                    Ok(())
                } else {
                    let bytes =
                        serde_json::to_vec_pretty(&map).map_err(|e| StoreError(e.to_string()))?;
                    write_private_atomic(&self.file, &bytes).map_err(|e| StoreError(e.to_string()))
                }
            }
            StoreKind::Keyring => {
                for part in ["meta", "access", "refresh", "id"] {
                    if let Ok(entry) = keyring::Entry::new(SERVICE, &Self::account(client_id, part))
                    {
                        match entry.delete_credential() {
                            Ok(()) | Err(keyring::Error::NoEntry) => {}
                            Err(e) => return Err(StoreError(e.to_string())),
                        }
                    }
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(expires_at: u64) -> TokenSet {
        TokenSet {
            access_token: Secret::new("a"),
            refresh_token: Secret::new("r"),
            id_token: Some(Secret::new("i")),
            token_type: "Bearer".into(),
            scope: vec!["openid".into(), PLAN_SCOPE.into()],
            expires_at,
            earliest_refresh_at: None,
            saved_at: 1,
        }
    }

    #[test]
    fn file_store_roundtrip_and_delete() {
        let dir = tempfile::tempdir().unwrap();
        let store = CredentialStore::new(StoreKind::File, dir.path());
        assert_eq!(store.load("c1").unwrap(), None);
        store.save("c1", &tokens(100)).unwrap();
        store.save("c2", &tokens(200)).unwrap();
        assert_eq!(store.load("c1").unwrap().unwrap().expires_at, 100);
        store.delete("c1").unwrap();
        assert_eq!(store.load("c1").unwrap(), None);
        assert!(store.load("c2").unwrap().is_some());
        store.delete("c2").unwrap();
        assert!(!store.file_path().exists());
    }

    #[cfg(unix)]
    #[test]
    fn file_store_is_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let store = CredentialStore::new(StoreKind::File, &dir.path().join("d"));
        store.save("c", &tokens(1)).unwrap();
        assert_eq!(
            std::fs::metadata(store.file_path())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }

    #[test]
    fn refresh_timing() {
        let mut t = tokens(1_000);
        assert!(!t.needs_refresh(900));
        assert!(t.needs_refresh(950));
        assert!(t.needs_refresh(1_000));
        t.earliest_refresh_at = Some(980);
        assert!(!t.needs_refresh(950), "earliest_refresh_at 이전 보류");
        assert!(t.needs_refresh(985));
        assert!(t.needs_refresh(1_001), "만료 후 즉시 갱신");
    }

    #[test]
    fn plan_scope_check() {
        let mut t = tokens(1);
        assert!(t.plan_enabled());
        t.scope.retain(|s| s != PLAN_SCOPE);
        assert!(!t.plan_enabled());
    }
}
