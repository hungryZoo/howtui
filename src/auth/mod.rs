//! ChatGPT 로그인 인증 관리 (SRS 11절, ADR-0007, ADR-0008).

pub mod callback;
pub mod idtoken;
pub mod oauth;
pub mod pkce;
pub mod secret;
pub mod state;
pub mod store;

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::net::TcpListener;

use crate::app::{LoginFailure, LoginInfo, LoginView};
use crate::llm::types::CancelToken;
use crate::runtime::{LoginFuture, LoginProgress, LoginService, LogoutFuture};
use oauth::{AuthorizeParams, Endpoints, OAuthError};
use secret::Secret;
use state::{Registration, StateFile};
use store::{CredentialStore, StoreKind, TokenSet};

pub const LOGIN_TIMEOUT: Duration = Duration::from_secs(600);
const JWKS_MAX_AGE: u64 = 24 * 3600;

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AuthError {
    #[error("not signed in")]
    NotLoggedIn,
    #[error("session rejected: {0}")]
    Rejected(String),
    #[error("invalid client configuration")]
    InvalidClient,
    #[error("network: {0}")]
    Network(String),
    #[error("temporarily unavailable ({0})")]
    Transient(u16),
    #[error("storage: {0}")]
    Storage(String),
}

impl From<OAuthError> for AuthError {
    fn from(e: OAuthError) -> Self {
        match e {
            e if e.is_invalid_client() => AuthError::InvalidClient,
            OAuthError::Rejected { code, .. } => AuthError::Rejected(code),
            OAuthError::Http { status } if status >= 500 || status == 429 => {
                AuthError::Transient(status)
            }
            OAuthError::Http { status } => AuthError::Rejected(format!("http {status}")),
            OAuthError::Network(d) => AuthError::Network(d),
            OAuthError::Parse(d) => AuthError::Network(d),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LoginOptions {
    pub no_browser: bool,
    pub port: Option<u16>,
}

impl LoginOptions {
    /// TUI 로그인 기본값: `HOWTUI_NO_BROWSER=1` 또는 SSH 세션이면 브라우저를 열지 않는다.
    pub fn from_env() -> Self {
        let set = |k: &str| std::env::var_os(k).is_some_and(|v| !v.is_empty());
        Self {
            no_browser: set("HOWTUI_NO_BROWSER") || set("SSH_CONNECTION") || set("SSH_TTY"),
            port: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthStatus {
    pub signed_in: bool,
    pub email: Option<String>,
    pub plan_enabled: bool,
    pub expires_at: Option<u64>,
    pub store: Option<StoreKind>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LogoutOutcome {
    Revoked,
    RevokeUnconfirmed,
    NotSignedIn,
}

pub struct AuthManager {
    data_dir: PathBuf,
    pub endpoints: Endpoints,
    http: reqwest::Client,
    cache: Mutex<Option<TokenSet>>,
    /// 새로 파일 저장소를 선택했을 때 사용자 고지용 경로
    pub file_store_notice: Mutex<Option<PathBuf>>,
}

impl AuthManager {
    pub fn new(data_dir: PathBuf, http: reqwest::Client) -> Self {
        Self {
            data_dir,
            endpoints: Endpoints::resolve(),
            http,
            cache: Mutex::new(None),
            file_store_notice: Mutex::new(None),
        }
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn http(&self) -> &reqwest::Client {
        &self.http
    }

    pub fn load_state(&self) -> StateFile {
        state::load(&self.data_dir)
    }

    pub fn save_state(&self, s: &StateFile) {
        let _ = state::save(&self.data_dir, s);
    }

    /// 기록된 저장소 종류. 환경변수 `HOWTUI_CREDENTIAL_STORE`(keyring, file)가 우선.
    fn store_kind(&self, s: &StateFile) -> Option<StoreKind> {
        if let Some(kind) = std::env::var("HOWTUI_CREDENTIAL_STORE")
            .ok()
            .as_deref()
            .and_then(StoreKind::parse)
        {
            return Some(kind);
        }
        s.credential_store.as_deref().and_then(StoreKind::parse)
    }

    fn store(&self, s: &StateFile) -> Option<CredentialStore> {
        self.store_kind(s)
            .map(|k| CredentialStore::new(k, &self.data_dir))
    }

    /// 첫 저장 시 저장소 선택: 보안 저장소 가능하면 keyring, 아니면 파일 (SRS-MOD-006).
    fn choose_store(&self, s: &mut StateFile) -> CredentialStore {
        let kind = self.store_kind(s).unwrap_or_else(|| {
            if store::keyring_available() {
                StoreKind::Keyring
            } else {
                StoreKind::File
            }
        });
        s.credential_store = Some(kind.name().into());
        let store = CredentialStore::new(kind, &self.data_dir);
        if kind == StoreKind::File {
            *self
                .file_store_notice
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = Some(store.file_path().to_path_buf());
        }
        store
    }

    /// 기록된 저장소 (없으면 None, 새로 선택하지 않음)
    pub fn store_if_recorded(&self) -> Option<CredentialStore> {
        self.store(&self.load_state())
    }

    /// API 키 저장소 (SRS-LLM-015). 기록된 종류 우선, 없으면 `write` 시 새로 선택 후 기록,
    /// 조회만 할 때는 선택 결과를 기록하지 않음.
    pub fn secret_store(&self, write: bool) -> CredentialStore {
        let mut s = self.load_state();
        if let Some(store) = self.store(&s) {
            return store;
        }
        if write {
            let store = self.choose_store(&mut s);
            self.save_state(&s);
            return store;
        }
        let kind = if store::keyring_available() {
            StoreKind::Keyring
        } else {
            StoreKind::File
        };
        CredentialStore::new(kind, &self.data_dir)
    }

    pub fn registration(&self) -> Option<Registration> {
        self.load_state().registration
    }

    pub fn tokens(&self) -> Option<TokenSet> {
        if let Some(t) = self.cache.lock().unwrap_or_else(|e| e.into_inner()).clone() {
            return Some(t);
        }
        let s = self.load_state();
        let reg = s.registration.as_ref()?;
        let tokens = self.store(&s)?.load(&reg.client_id).ok().flatten()?;
        *self.cache.lock().unwrap_or_else(|e| e.into_inner()) = Some(tokens.clone());
        Some(tokens)
    }

    pub fn has_credentials(&self) -> bool {
        self.tokens().is_some()
    }

    pub fn plan_enabled(&self) -> bool {
        self.tokens().is_some_and(|t| t.plan_enabled())
    }

    pub fn status(&self) -> AuthStatus {
        let s = self.load_state();
        let tokens = self.tokens();
        AuthStatus {
            signed_in: tokens.is_some(),
            email: s.registration.as_ref().and_then(|r| r.email.clone()),
            plan_enabled: tokens.as_ref().is_some_and(|t| t.plan_enabled()),
            expires_at: tokens.as_ref().map(|t| t.expires_at),
            store: self.store_kind(&s),
        }
    }

    fn save_tokens(&self, client_id: &str, tokens: &TokenSet) -> Result<(), AuthError> {
        let mut s = self.load_state();
        let store = match self.store(&s) {
            Some(st) => st,
            None => {
                let st = self.choose_store(&mut s);
                self.save_state(&s);
                st
            }
        };
        if let Err(e) = store.save(client_id, tokens) {
            // 보안 저장소 쓰기 실패(크기 제한 등) 시 파일 저장소로 전환
            if store.kind == StoreKind::Keyring {
                let _ = store.delete(client_id);
                let file = CredentialStore::new(StoreKind::File, &self.data_dir);
                file.save(client_id, tokens)
                    .map_err(|e| AuthError::Storage(e.to_string()))?;
                s.credential_store = Some(StoreKind::File.name().into());
                self.save_state(&s);
                *self
                    .file_store_notice
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = Some(file.file_path().to_path_buf());
            } else {
                return Err(AuthError::Storage(e.to_string()));
            }
        }
        *self.cache.lock().unwrap_or_else(|e| e.into_inner()) = Some(tokens.clone());
        Ok(())
    }

    /// 테스트에서 로그인 상태 구성.
    #[cfg(test)]
    pub fn seed_for_tests(&self, client_id: &str, tokens: &TokenSet) {
        let mut s = self.load_state();
        s.registration = Some(Registration {
            client_id: client_id.into(),
            ..Default::default()
        });
        if s.credential_store.is_none() {
            s.credential_store = Some("file".into());
        }
        self.save_state(&s);
        self.save_tokens(client_id, tokens).unwrap();
    }

    fn delete_tokens(&self, client_id: &str) {
        let s = self.load_state();
        if let Some(store) = self.store(&s) {
            let _ = store.delete(client_id);
        }
        *self.cache.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }

    fn token_set(resp: &oauth::TokenResponse, previous: Option<&TokenSet>, now: u64) -> TokenSet {
        let scope = resp.scopes();
        TokenSet {
            access_token: Secret::new(resp.access_token.clone()),
            refresh_token: Secret::new(
                resp.refresh_token
                    .clone()
                    .or_else(|| previous.map(|p| p.refresh_token.expose().to_string()))
                    .unwrap_or_default(),
            ),
            id_token: resp
                .id_token
                .clone()
                .map(Secret::new)
                .or_else(|| previous.and_then(|p| p.id_token.clone())),
            token_type: resp.token_type.clone().unwrap_or_else(|| "Bearer".into()),
            scope: if scope.is_empty() {
                previous.map(|p| p.scope.clone()).unwrap_or_default()
            } else {
                scope
            },
            expires_at: now + resp.expires_in.unwrap_or(3600),
            earliest_refresh_at: resp.earliest_refresh(now),
            saved_at: now,
        }
    }

    /// 유효한 액세스 토큰. 필요 시 프로세스 간 잠금 아래에서 갱신 (SRS-AUTH-008).
    pub async fn access_token(&self) -> Result<Secret, AuthError> {
        let tokens = self.tokens().ok_or(AuthError::NotLoggedIn)?;
        if !tokens.needs_refresh(now()) {
            return Ok(tokens.access_token);
        }
        self.refresh_locked(false).await
    }

    /// 401 수신 후 강제 갱신.
    pub async fn force_refresh(&self) -> Result<Secret, AuthError> {
        self.refresh_locked(true).await
    }

    async fn refresh_locked(&self, force: bool) -> Result<Secret, AuthError> {
        let reg = self.registration().ok_or(AuthError::NotLoggedIn)?;
        let lock_path = self.data_dir.join("auth.lock");
        crate::platform::fs::ensure_private_dir(&self.data_dir)
            .map_err(|e| AuthError::Storage(e.to_string()))?;
        let lock = tokio::task::spawn_blocking(move || -> std::io::Result<std::fs::File> {
            let f = std::fs::OpenOptions::new()
                .create(true)
                .truncate(false)
                .write(true)
                .open(lock_path)?;
            f.lock()?;
            Ok(f)
        })
        .await
        .map_err(|e| AuthError::Storage(e.to_string()))?
        .map_err(|e| AuthError::Storage(e.to_string()))?;

        // 잠금 획득 후 다른 프로세스의 갱신 여부 재확인
        *self.cache.lock().unwrap_or_else(|e| e.into_inner()) = None;
        let current = self.tokens().ok_or(AuthError::NotLoggedIn)?;
        let t = now();
        if !force && !current.needs_refresh(t) {
            drop(lock);
            return Ok(current.access_token);
        }
        if force && current.saved_at + 5 >= t && current.expires_at > t + 60 {
            // 다른 프로세스가 방금 갱신한 토큰
            drop(lock);
            return Ok(current.access_token);
        }
        let mut last_err = None;
        for attempt in 0..3u32 {
            if attempt > 0 {
                tokio::time::sleep(Duration::from_secs(1 << (attempt - 1))).await;
            }
            tracing::debug!(attempt, "token refresh");
            match oauth::refresh(
                &self.http,
                &self.endpoints,
                &reg.client_id,
                current.refresh_token.expose(),
            )
            .await
            {
                Ok(resp) => {
                    let tokens = Self::token_set(&resp, Some(&current), now());
                    self.save_tokens(&reg.client_id, &tokens)?;
                    drop(lock);
                    return Ok(tokens.access_token);
                }
                Err(e) if e.is_terminal_refresh() => {
                    self.delete_tokens(&reg.client_id);
                    drop(lock);
                    return Err(AuthError::Rejected(e.to_string()));
                }
                Err(e) if e.is_transient() => last_err = Some(AuthError::from(e)),
                Err(e) => {
                    drop(lock);
                    return Err(e.into());
                }
            }
        }
        drop(lock);
        // 일시 장애: 자격증명 보존, 아직 유효한 토큰이면 계속 사용 (SRS-AUTH-016)
        if current.expires_at > now() {
            return Ok(current.access_token);
        }
        Err(last_err.unwrap_or(AuthError::Network("refresh failed".into())))
    }

    async fn jwks(&self) -> Result<Vec<state::Jwk>, AuthError> {
        let mut s = self.load_state();
        if let Some(cache) = &s.jwks
            && cache.fetched_at + JWKS_MAX_AGE > now()
            && !cache.keys.is_empty()
        {
            return Ok(cache.keys.clone());
        }
        let keys = oauth::fetch_jwks(&self.http, &self.endpoints).await?;
        s.jwks = Some(state::JwksCache {
            fetched_at: now(),
            keys: keys.clone(),
        });
        self.save_state(&s);
        Ok(keys)
    }

    /// 로그인 8단계 (SRS 11.1절).
    pub async fn login(
        &self,
        opts: LoginOptions,
        progress: &(dyn Fn(LoginProgress) + Send + Sync),
        mut paste: tokio::sync::mpsc::UnboundedReceiver<String>,
        cancel: CancelToken,
    ) -> Result<LoginInfo, LoginFailure> {
        let other = |e: &dyn std::fmt::Display| LoginFailure::Other(e.to_string());
        // (1) host ID
        let host_id = state::ensure_host_id(&self.data_dir).map_err(|e| other(&e))?;
        let st = self.load_state();
        let registration = st.registration.clone();
        let new_registration = registration.is_none();
        let client_id = registration
            .as_ref()
            .map(|r| r.client_id.clone())
            .unwrap_or_else(|| oauth::DYNAMIC_CLIENT.into());
        let id_token_hint = if new_registration {
            None
        } else {
            self.tokens()
                .and_then(|t| t.id_token)
                .map(|s| s.expose().to_string())
        };
        // (2) state, nonce, PKCE
        let state_token = pkce::random_token();
        let nonce = pkce::random_token();
        let verifier = pkce::random_token();
        let challenge = pkce::challenge(&verifier);
        // (3) 루프백 리스너
        let listener = TcpListener::bind(("127.0.0.1", opts.port.unwrap_or(0)))
            .await
            .map_err(|e| other(&e))?;
        let port = listener.local_addr().map_err(|e| other(&e))?.port();
        let redirect_uri = callback::redirect_uri(port);
        // (4) 인가 URL
        let url = oauth::authorize_url(
            &self.endpoints,
            &AuthorizeParams {
                client_id: &client_id,
                new_registration,
                host_id: &host_id,
                redirect_uri: &redirect_uri,
                state: &state_token,
                nonce: &nonce,
                challenge: &challenge,
                id_token_hint: id_token_hint.as_deref(),
                prompt_consent: st.plan_disabled_last,
            },
        );
        // (5) 브라우저 실행
        let browser_failed = opts.no_browser || open::that_detached(&url).is_err();
        tracing::debug!(port, new_registration, browser_failed, "login started");
        progress(LoginProgress::Started(LoginView {
            url: Some(url),
            port: Some(port),
            browser_failed,
        }));

        // (6) 콜백 또는 붙여넣은 URL 대기
        let params = {
            let wait = async {
                loop {
                    tokio::select! {
                        r = callback::wait_callback(&listener, port) => return r.map_err(|e| other(&e)),
                        pasted = paste.recv() => match pasted {
                            Some(text) => match callback::parse_pasted_url(&text, port) {
                                Ok(p) => return Ok(p),
                                Err(_) => progress(LoginProgress::InvalidPaste),
                            },
                            None => return callback::wait_callback(&listener, port).await.map_err(|e| other(&e)),
                        },
                    }
                }
            };
            tokio::select! {
                r = wait => r?,
                _ = cancel.cancelled() => return Err(LoginFailure::Cancelled),
                _ = tokio::time::sleep(LOGIN_TIMEOUT) => return Err(LoginFailure::Timeout),
            }
        };
        drop(listener);
        if !params
            .state
            .as_deref()
            .is_some_and(|s| pkce::constant_time_eq(s, &state_token))
        {
            return Err(LoginFailure::StateMismatch);
        }
        if let Some(err) = &params.error {
            return Err(if err == "access_denied" {
                LoginFailure::Denied
            } else {
                LoginFailure::Other(err.clone())
            });
        }
        let code = params
            .code
            .clone()
            .ok_or_else(|| LoginFailure::Other("missing authorization code".into()))?;
        // 발급 client_id 확인과 코드 교환 전 저장
        let issued = match (&registration, &params.client_id) {
            (None, Some(id)) if id != oauth::DYNAMIC_CLIENT => id.clone(),
            (None, _) => {
                return Err(LoginFailure::Other(
                    "registration incomplete: no client_id in callback".into(),
                ));
            }
            (Some(reg), Some(id)) if id != &reg.client_id => {
                return Err(LoginFailure::Other(
                    "callback returned a different client_id".into(),
                ));
            }
            (Some(reg), _) => reg.client_id.clone(),
        };
        if new_registration {
            let mut s = self.load_state();
            s.registration = Some(Registration {
                client_id: issued.clone(),
                subject: None,
                email: None,
                created_at: now(),
            });
            self.save_state(&s);
        }
        // (7) 코드 교환
        let resp = oauth::exchange_code(
            &self.http,
            &self.endpoints,
            &issued,
            &code,
            &verifier,
            &redirect_uri,
        )
        .await
        .map_err(|e| other(&e))?;
        // (8) ID 토큰 검증, scope 확인, 저장
        let id_token = resp
            .id_token
            .clone()
            .ok_or_else(|| LoginFailure::Other("missing id_token".into()))?;
        let keys = self.jwks().await.map_err(|e| other(&e))?;
        let claims = idtoken::verify(
            &id_token,
            &keys,
            &idtoken::Expected {
                issuer: &self.endpoints.issuer,
                client_id: &issued,
                nonce: &nonce,
                now: now(),
            },
        )
        .map_err(|e| other(&e))?;
        let tokens = Self::token_set(&resp, None, now());
        let plan_enabled = tokens.plan_enabled();
        self.save_tokens(&issued, &tokens).map_err(|e| other(&e))?;
        let mut s = self.load_state();
        if let Some(reg) = s.registration.as_mut() {
            reg.subject = Some(claims.sub.clone());
            reg.email = claims.email.clone();
        }
        let show_plan_notice = plan_enabled && !s.plan_notice_shown;
        if show_plan_notice {
            s.plan_notice_shown = true;
        }
        s.plan_disabled_last = !plan_enabled;
        self.save_state(&s);
        Ok(LoginInfo {
            email: claims.email,
            plan_enabled,
            show_plan_notice,
        })
    }

    /// 로그아웃 (SRS 11.3절). client 매핑과 host ID는 `forget`이 아니면 보존.
    pub async fn logout(&self, forget: bool) -> LogoutOutcome {
        let mut s = self.load_state();
        let Some(reg) = s.registration.clone() else {
            return LogoutOutcome::NotSignedIn;
        };
        let tokens = self.tokens();
        let outcome = match &tokens {
            None => LogoutOutcome::NotSignedIn,
            Some(t) => {
                let mut ok = false;
                for attempt in 0..3u32 {
                    if attempt > 0 {
                        tokio::time::sleep(Duration::from_secs(1 << (attempt - 1))).await;
                    }
                    match oauth::revoke(
                        &self.http,
                        &self.endpoints,
                        &reg.client_id,
                        t.refresh_token.expose(),
                    )
                    .await
                    {
                        Ok(()) => {
                            ok = true;
                            break;
                        }
                        Err(e) if e.is_transient() => continue,
                        Err(_) => break,
                    }
                }
                if ok {
                    LogoutOutcome::Revoked
                } else {
                    LogoutOutcome::RevokeUnconfirmed
                }
            }
        };
        self.delete_tokens(&reg.client_id);
        if forget {
            s.registration = None;
            s.plan_notice_shown = false;
            s.plan_disabled_last = false;
            s.models = None;
        }
        self.save_state(&s);
        outcome
    }
}

/// TUI 로그인 연결 (PRD-AUTH-013).
pub struct TuiLogin {
    pub auth: std::sync::Arc<AuthManager>,
}

impl LoginService for TuiLogin {
    fn login(
        &self,
        progress: Box<dyn Fn(LoginProgress) + Send + Sync>,
        paste: tokio::sync::mpsc::UnboundedReceiver<String>,
        cancel: CancelToken,
    ) -> LoginFuture<'_> {
        Box::pin(async move {
            self.auth
                .login(LoginOptions::from_env(), &*progress, paste, cancel)
                .await
        })
    }

    fn logout(&self) -> LogoutFuture<'_> {
        Box::pin(async move {
            let _ = self.auth.logout(false).await;
        })
    }
}

/// 이메일 마스킹: 첫 글자와 도메인만 (SRS-AUTH-010).
pub fn mask_email(email: &str) -> String {
    match email.split_once('@') {
        Some((user, domain)) => {
            let first: String = user.chars().take(1).collect();
            format!("{first}***@{domain}")
        }
        None => "***".into(),
    }
}

#[cfg(test)]
mod tests;
