//! 새 버전 확인과 업데이트 (CR-009, self-update-check 스킬 기반, SRS-UPD-001-004).
//!
//! - 시작 지연 0: 확인은 백그라운드, 결과는 다음 실행 상태줄에 표시
//! - 하루 1회, 요청 전에 시도 시각 기록 (오프라인 재시도 폭주 방지)
//! - `HOWTUI_NO_UPDATE_CHECK=1` 또는 설정 `update.check = false`로 비활성화
//! - 최신 버전은 `/releases/latest` 리다이렉트로 조회 (API 한도 무관)
//! - 설치 스크립트 설치본: 릴리스 압축 파일을 내려받아 SHA-256 검증 후 실행 파일 교체 (외부 명령 실행 없음)
//! - cargo, Homebrew, 그 외: 해당 명령 또는 릴리스 주소 안내만

use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const LATEST_URL: &str = "https://github.com/hungryZoo/howtui/releases/latest";
const DOWNLOAD_BASE: &str = "https://github.com/hungryZoo/howtui/releases/download";
pub const CARGO_COMMAND: &str = "cargo install --git https://github.com/hungryZoo/howtui --locked";
const CHECK_INTERVAL: u64 = 24 * 60 * 60;
const CHECK_TIMEOUT: Duration = Duration::from_secs(2);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(120);

type Env<'a> = &'a dyn Fn(&str) -> Option<String>;

pub fn latest_url(env: Env) -> String {
    env("HOWTUI_LATEST_URL").unwrap_or_else(|| LATEST_URL.into())
}

fn download_base(env: Env) -> String {
    env("HOWTUI_DOWNLOAD_BASE")
        .map(|b| b.trim_end_matches('/').to_string())
        .unwrap_or_else(|| DOWNLOAD_BASE.into())
}

/// `MAJOR.MINOR.PATCH`만 허용 (접두사 `v` 허용, 그 외 접미사는 거부).
pub fn parse_semver(s: &str) -> Option<(u32, u32, u32)> {
    let s = s.trim();
    let s = s.strip_prefix('v').unwrap_or(s);
    let mut it = s.split('.');
    let v = (
        it.next()?.parse().ok()?,
        it.next()?.parse().ok()?,
        it.next()?.parse().ok()?,
    );
    it.next().is_none().then_some(v)
}

pub fn is_newer(candidate: &str, current: &str) -> bool {
    match (parse_semver(candidate), parse_semver(current)) {
        (Some(c), Some(cur)) => c > cur,
        _ => false,
    }
}

pub fn version_from_release_url(url: &str) -> Option<String> {
    let (_, tag) = url.rsplit_once("/tag/")?;
    let v = tag.trim().trim_end_matches('/').trim_start_matches('v');
    parse_semver(v).map(|_| v.to_string())
}

/// 환경변수 비활성화 (`HOWTUI_NO_UPDATE_CHECK`가 비어 있지 않고 0이 아님).
pub fn disabled_by_env(env: Env) -> bool {
    env("HOWTUI_NO_UPDATE_CHECK").is_some_and(|v| v != "0")
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// 캐시

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Cache {
    /// 마지막 시도 시각 (성공 여부 무관, Unix 초)
    pub checked_at: u64,
    /// 확인된 최신 버전 (`v` 없음)
    pub latest: Option<String>,
}

impl Cache {
    /// key=value 줄. 손상 시 기본값 (즉시 재확인).
    pub fn parse(text: &str) -> Cache {
        let mut c = Cache::default();
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            match k.trim() {
                "checked_at" => c.checked_at = v.trim().parse().unwrap_or(0),
                "latest" => {
                    let v = v.trim();
                    if parse_semver(v).is_some() {
                        c.latest = Some(v.to_string());
                    }
                }
                _ => {}
            }
        }
        c
    }

    pub fn render(&self) -> String {
        format!(
            "checked_at={}\nlatest={}\n",
            self.checked_at,
            self.latest.as_deref().unwrap_or("")
        )
    }

    pub fn is_stale(&self, now: u64) -> bool {
        self.checked_at == 0 || now.saturating_sub(self.checked_at) >= CHECK_INTERVAL
    }
}

/// Linux `${XDG_CACHE_HOME:-~/.cache}/howtui/update-check`, macOS `~/Library/Caches/howtui/update-check`,
/// Windows `%LOCALAPPDATA%\howtui\cache\update-check`.
pub fn cache_path(env: Env) -> Option<PathBuf> {
    let home = crate::platform::paths::home(env);
    let base = if cfg!(windows) {
        env("LOCALAPPDATA")
            .map(PathBuf::from)
            .or_else(|| home.map(|h| h.join("AppData").join("Local")))?
            .join("howtui")
            .join("cache")
    } else if cfg!(target_os = "macos") {
        home?.join("Library").join("Caches").join("howtui")
    } else {
        env("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .or_else(|| home.map(|h| h.join(".cache")))?
            .join("howtui")
    };
    Some(base.join("update-check"))
}

pub fn read_cache(path: &Path) -> Cache {
    fs::read_to_string(path)
        .map(|t| Cache::parse(&t))
        .unwrap_or_default()
}

pub fn write_cache(path: &Path, cache: &Cache) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, cache.render())
}

// 네트워크

fn check_client() -> Option<reqwest::Client> {
    crate::llm::http::install_crypto();
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(CHECK_TIMEOUT)
        .user_agent(concat!("howtui/", env!("CARGO_PKG_VERSION")))
        .build()
        .ok()
}

fn download_client() -> Option<reqwest::Client> {
    crate::llm::http::install_crypto();
    reqwest::Client::builder()
        .timeout(DOWNLOAD_TIMEOUT)
        .user_agent(concat!("howtui/", env!("CARGO_PKG_VERSION")))
        .build()
        .ok()
}

/// `/releases/latest`의 리다이렉트 대상 `/tag/vX.Y.Z`에서 버전 추출.
pub async fn fetch_latest(url: &str) -> Option<String> {
    let resp = check_client()?.get(url).send().await.ok()?;
    let location = resp
        .headers()
        .get(reqwest::header::LOCATION)?
        .to_str()
        .ok()?;
    version_from_release_url(location)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckOutcome {
    UpdateAvailable { latest: String },
    UpToDate { latest: String },
    Unreachable,
}

/// 즉시 확인 (하루 주기 무시), 캐시 갱신. `--check-update`, `--update`, doctor, 업데이트 화면용.
/// 환경 조회는 호출 시점에 끝내므로 반환 Future는 스레드 간 이동 가능.
pub fn check_now(env: Env) -> impl std::future::Future<Output = CheckOutcome> + Send + 'static {
    check_with(latest_url(env), cache_path(env))
}

async fn check_with(url: String, path: Option<PathBuf>) -> CheckOutcome {
    let mut cache = path.as_deref().map(read_cache).unwrap_or_default();
    cache.checked_at = now_secs();
    let latest = fetch_latest(&url).await;
    if let Some(l) = &latest {
        cache.latest = Some(l.clone());
    }
    if let Some(p) = &path {
        let _ = write_cache(p, &cache);
    }
    match latest {
        Some(latest) if is_newer(&latest, CURRENT_VERSION) => {
            CheckOutcome::UpdateAvailable { latest }
        }
        Some(latest) => CheckOutcome::UpToDate { latest },
        None => CheckOutcome::Unreachable,
    }
}

/// 캐시가 하루 지났으면 시도 시각을 먼저 기록하고 백그라운드 확인. 즉시 반환.
pub fn maybe_spawn_check(rt: &tokio::runtime::Handle, env: Env) {
    let Some(path) = cache_path(env) else {
        return;
    };
    let mut cache = read_cache(&path);
    let now = now_secs();
    if !cache.is_stale(now) {
        return;
    }
    cache.checked_at = now;
    if write_cache(&path, &cache).is_err() {
        return;
    }
    let url = latest_url(env);
    rt.spawn(async move {
        if let Some(latest) = fetch_latest(&url).await {
            cache.latest = Some(latest);
            let _ = write_cache(&path, &cache);
        }
    });
}

// 설치 방식

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InstallMethod {
    Brew,
    Cargo,
    /// install.sh, install.ps1 (또는 압축 파일 직접 복사)로 사용자 디렉터리에 설치
    Script {
        bin_dir: PathBuf,
    },
    Unknown,
}

/// 순수 판정. Homebrew는 `$HOME` 아래(Linuxbrew)일 수 있어 먼저 검사.
pub fn classify_install(
    exe: &Path,
    home: Option<&Path>,
    cargo_home: Option<&Path>,
    local_app_data: Option<&Path>,
) -> InstallMethod {
    let s = exe.to_string_lossy().replace('\\', "/");
    if s.contains("/Cellar/") || s.contains("/homebrew/") || s.contains("/linuxbrew/") {
        return InstallMethod::Brew;
    }
    let cargo_bin = cargo_home
        .map(Path::to_path_buf)
        .or_else(|| home.map(|h| h.join(".cargo")))
        .map(|c| c.join("bin"));
    if cargo_bin.is_some_and(|c| exe.starts_with(c)) {
        return InstallMethod::Cargo;
    }
    let user_dir = |base: Option<&Path>| base.is_some_and(|b| exe.starts_with(b));
    if (user_dir(home) || user_dir(local_app_data))
        && let Some(dir) = exe.parent()
    {
        return InstallMethod::Script {
            bin_dir: dir.to_path_buf(),
        };
    }
    InstallMethod::Unknown
}

pub fn detect_install_method(env: Env) -> InstallMethod {
    let exe = std::env::current_exe()
        .and_then(fs::canonicalize)
        .unwrap_or_default();
    let home = crate::platform::paths::home(env).and_then(|h| fs::canonicalize(h).ok());
    let cargo_home = env("CARGO_HOME").map(PathBuf::from);
    let local = env("LOCALAPPDATA").and_then(|p| fs::canonicalize(p).ok());
    classify_install(
        &exe,
        home.as_deref(),
        cargo_home.as_deref(),
        local.as_deref(),
    )
}

impl InstallMethod {
    pub fn to_app(&self) -> crate::app::UpdateMethod {
        use crate::app::UpdateMethod as M;
        match self {
            InstallMethod::Script { bin_dir } => M::Script {
                dir: bin_dir.display().to_string(),
            },
            InstallMethod::Cargo => M::Cargo,
            InstallMethod::Brew => M::Brew,
            InstallMethod::Unknown => M::Unknown,
        }
    }
}

/// 설치 방식별 갱신 명령 또는 주소 (Script는 howtui가 직접 적용).
pub fn upgrade_hint(method: &crate::app::UpdateMethod) -> String {
    use crate::app::UpdateMethod as M;
    match method {
        M::Script { .. } => "howtui --update".into(),
        M::Cargo => CARGO_COMMAND.into(),
        M::Brew => "brew upgrade howtui".into(),
        M::Unknown => LATEST_URL.into(),
    }
}

pub fn info_for(latest: &str, method: &InstallMethod) -> crate::app::UpdateInfo {
    crate::app::UpdateInfo {
        current: CURRENT_VERSION.into(),
        latest: latest.into(),
        method: method.to_app(),
    }
}

/// 즉시 확인 결과를 업데이트 화면 상태로.
pub fn view_for(outcome: &CheckOutcome, method: &InstallMethod) -> crate::app::UpdateView {
    use crate::app::UpdateView as V;
    match outcome {
        CheckOutcome::UpdateAvailable { latest } => V::Ready(info_for(latest, method)),
        CheckOutcome::UpToDate { latest } => V::UpToDate {
            current: CURRENT_VERSION.into(),
            latest: latest.clone(),
        },
        CheckOutcome::Unreachable => V::Unreachable,
    }
}

/// 상태줄, 업데이트 화면, `--update`가 쓰는 정보.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdatePlan {
    pub latest: String,
    pub method: InstallMethod,
}

/// 캐시 기준 계획 (네트워크 없음). 새 버전이 아니거나 비활성화면 None.
pub fn plan(env: Env, enabled: bool) -> Option<UpdatePlan> {
    if !enabled || disabled_by_env(env) {
        return None;
    }
    let latest = read_cache(&cache_path(env)?).latest?;
    is_newer(&latest, CURRENT_VERSION).then(|| UpdatePlan {
        latest,
        method: detect_install_method(env),
    })
}

// 적용 (설치 스크립트 설치본)

/// 이 바이너리의 릴리스 대상 이름 (릴리스 7개 대상과 동일).
pub fn release_target() -> Option<&'static str> {
    if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some("x86_64-unknown-linux-gnu")
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        Some("aarch64-unknown-linux-gnu")
    } else if cfg!(all(target_os = "linux", target_arch = "arm")) {
        Some("armv7-unknown-linux-gnueabihf")
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Some("aarch64-apple-darwin")
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        Some("x86_64-apple-darwin")
    } else if cfg!(all(windows, target_arch = "x86_64")) {
        Some("x86_64-pc-windows-msvc")
    } else if cfg!(all(windows, target_arch = "aarch64")) {
        Some("aarch64-pc-windows-msvc")
    } else {
        None
    }
}

pub fn asset_name(version: &str, target: &str) -> String {
    let ext = if target.contains("windows") {
        "zip"
    } else {
        "tar.gz"
    };
    format!("howtui-{version}-{target}.{ext}")
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum UpdateError {
    #[error("no release build for this platform")]
    NoTarget,
    #[error("network: {0}")]
    Network(String),
    #[error("SHA256SUMS has no entry for {0}")]
    NoChecksum(String),
    #[error("checksum mismatch for {0}")]
    Checksum(String),
    #[error("archive: {0}")]
    Archive(String),
    #[error("cannot replace {path}: {detail}")]
    Replace { path: String, detail: String },
}

/// SHA256SUMS에서 자산의 해시 (`<hex>  <name>` 또는 `<hex> *<name>`).
pub fn expected_sha256(sums: &str, asset: &str) -> Option<String> {
    sums.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        let hash = parts.next()?;
        let name = parts.next()?.trim_start_matches('*');
        (name == asset).then(|| hash.to_ascii_lowercase())
    })
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// 압축 파일에서 실행 파일과 제거 스크립트 추출 (루트 항목만).
pub fn extract(archive: &[u8], zip: bool) -> Result<(Vec<u8>, Option<Vec<u8>>), UpdateError> {
    let (bin_name, uninstall_name) = if zip {
        ("howtui.exe", "uninstall.ps1")
    } else {
        ("howtui", "uninstall.sh")
    };
    let mut bin = None;
    let mut uninstall = None;
    let err = |e: &dyn std::fmt::Display| UpdateError::Archive(e.to_string());
    if zip {
        let mut z = zip::ZipArchive::new(io::Cursor::new(archive)).map_err(|e| err(&e))?;
        for i in 0..z.len() {
            let mut f = z.by_index(i).map_err(|e| err(&e))?;
            let name = f.name().map_err(|e| err(&e))?.to_string();
            let mut buf = Vec::new();
            if name == bin_name || name == uninstall_name {
                f.read_to_end(&mut buf).map_err(|e| err(&e))?;
                if name == bin_name {
                    bin = Some(buf);
                } else {
                    uninstall = Some(buf);
                }
            }
        }
    } else {
        let gz = flate2::read::GzDecoder::new(archive);
        let mut tar = tar::Archive::new(gz);
        for entry in tar.entries().map_err(|e| err(&e))? {
            let mut entry = entry.map_err(|e| err(&e))?;
            let name = entry
                .path()
                .map_err(|e| err(&e))?
                .to_string_lossy()
                .trim_start_matches("./")
                .to_string();
            if name == bin_name || name == uninstall_name {
                let mut buf = Vec::new();
                entry.read_to_end(&mut buf).map_err(|e| err(&e))?;
                if name == bin_name {
                    bin = Some(buf);
                } else {
                    uninstall = Some(buf);
                }
            }
        }
    }
    let bin = bin.ok_or_else(|| UpdateError::Archive(format!("{bin_name} not found")))?;
    Ok((bin, uninstall))
}

/// 같은 디렉터리 임시 파일 기록 후 이름 변경. Windows 실행 중 파일은 `.old`로 옮긴 뒤 교체.
pub fn replace_file(path: &Path, bytes: &[u8], executable: bool) -> Result<(), UpdateError> {
    let fail = |e: io::Error| UpdateError::Replace {
        path: path.display().to_string(),
        detail: e.to_string(),
    };
    let dir = path.parent().unwrap_or(Path::new("."));
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tmp = dir.join(format!(".{name}.new.{}", std::process::id()));
    fs::write(&tmp, bytes).map_err(fail)?;
    #[cfg(unix)]
    if executable {
        use std::os::unix::fs::PermissionsExt;
        if let Err(e) = fs::set_permissions(&tmp, fs::Permissions::from_mode(0o755)) {
            let _ = fs::remove_file(&tmp);
            return Err(fail(e));
        }
    }
    #[cfg(not(unix))]
    let _ = executable;
    #[cfg(windows)]
    let old = {
        let old = dir.join(format!("{name}.old"));
        let _ = fs::remove_file(&old);
        if path.exists()
            && let Err(e) = fs::rename(path, &old)
        {
            let _ = fs::remove_file(&tmp);
            return Err(fail(e));
        }
        old
    };
    if let Err(e) = fs::rename(&tmp, path) {
        #[cfg(windows)]
        let _ = fs::rename(&old, path);
        let _ = fs::remove_file(&tmp);
        return Err(fail(e));
    }
    Ok(())
}

/// Windows: 이전 업데이트가 남긴 `howtui.exe.old` 정리 (실행 중이던 파일, 다음 실행부터 삭제 가능).
pub fn cleanup_previous() {
    #[cfg(windows)]
    if let Ok(exe) = std::env::current_exe() {
        let mut old = exe.into_os_string();
        old.push(".old");
        let _ = fs::remove_file(std::path::PathBuf::from(old));
    }
}

async fn get_bytes(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, UpdateError> {
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| UpdateError::Network(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(UpdateError::Network(format!(
            "{url}: HTTP {}",
            resp.status().as_u16()
        )));
    }
    resp.bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| UpdateError::Network(e.to_string()))
}

/// 릴리스 내려받기, SHA-256 검증, `bin_dir`의 실행 파일과 제거 스크립트 교체.
pub async fn apply(
    env: Env<'_>,
    version: &str,
    bin_dir: &Path,
    progress: &dyn Fn(&str),
) -> Result<(), UpdateError> {
    let target = release_target().ok_or(UpdateError::NoTarget)?;
    let asset = asset_name(version, target);
    let base = download_base(env);
    let client = download_client().ok_or_else(|| UpdateError::Network("client".into()))?;
    progress(&asset);
    let sums = get_bytes(&client, &format!("{base}/v{version}/SHA256SUMS")).await?;
    let expected = expected_sha256(&String::from_utf8_lossy(&sums), &asset)
        .ok_or_else(|| UpdateError::NoChecksum(asset.clone()))?;
    let archive = get_bytes(&client, &format!("{base}/v{version}/{asset}")).await?;
    if sha256_hex(&archive) != expected {
        return Err(UpdateError::Checksum(asset));
    }
    let zip = asset.ends_with(".zip");
    let (bin, uninstall) = extract(&archive, zip)?;
    let (bin_name, uninstall_name) = if zip {
        ("howtui.exe", "howtui-uninstall.ps1")
    } else {
        ("howtui", "howtui-uninstall.sh")
    };
    replace_file(&bin_dir.join(bin_name), &bin, true)?;
    // 제거 스크립트는 이미 설치된 경우만 갱신
    let uninstall_path = bin_dir.join(uninstall_name);
    if let Some(script) = uninstall
        && uninstall_path.exists()
    {
        replace_file(&uninstall_path, &script, true)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
