//! 업데이트 확인과 적용 시험 (TC-UPD-001-004). 네트워크는 Mock HTTP 서버.

use std::io::Write;

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

#[test]
fn semver_and_release_url() {
    assert_eq!(parse_semver("v1.2.3"), Some((1, 2, 3)));
    assert_eq!(parse_semver("1.2"), None);
    assert_eq!(parse_semver("1.2.3-rc1"), None);
    assert!(is_newer("0.1.10", "0.1.9"));
    assert!(!is_newer("0.1.1", "0.1.1"));
    assert!(!is_newer("garbage", "0.1.1"));
    assert_eq!(
        version_from_release_url("https://github.com/hungryZoo/howtui/releases/tag/v0.2.0"),
        Some("0.2.0".into())
    );
    assert_eq!(
        version_from_release_url("https://github.com/x/releases"),
        None
    );
}

#[test]
fn cache_round_trip_staleness_and_corruption() {
    let c = Cache {
        checked_at: 100,
        latest: Some("0.2.0".into()),
    };
    assert_eq!(Cache::parse(&c.render()), c);
    assert!(Cache::default().is_stale(5), "최초 실행은 즉시 확인");
    assert!(!c.is_stale(100 + CHECK_INTERVAL - 1));
    assert!(c.is_stale(100 + CHECK_INTERVAL));
    assert_eq!(
        Cache::parse("garbage\nchecked_at=x\nlatest=1.2"),
        Cache::default()
    );
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("a").join("update-check");
    write_cache(&p, &c).unwrap();
    assert_eq!(read_cache(&p), c);
    assert_eq!(read_cache(&dir.path().join("missing")), Cache::default());
}

#[test]
fn cache_path_rules() {
    let env = |k: &str| match k {
        "HOME" | "USERPROFILE" => Some("/h".to_string()),
        "XDG_CACHE_HOME" => Some("/xdg".to_string()),
        "LOCALAPPDATA" => Some("/lad".to_string()),
        _ => None,
    };
    let p = cache_path(&env).unwrap();
    assert!(p.ends_with("update-check"));
    if cfg!(windows) {
        assert!(p.starts_with("/lad/howtui/cache"));
    } else if cfg!(target_os = "macos") {
        assert!(p.starts_with("/h/Library/Caches/howtui"));
    } else {
        assert_eq!(p, PathBuf::from("/xdg/howtui/update-check"));
    }
    assert!(disabled_by_env(
        &|k: &str| (k == "HOWTUI_NO_UPDATE_CHECK").then(|| "1".into())
    ));
    assert!(!disabled_by_env(
        &|k: &str| (k == "HOWTUI_NO_UPDATE_CHECK").then(|| "0".into())
    ));
}

#[test]
fn install_method_classification() {
    let home = Path::new("/home/u");
    let lad = Path::new("/c/Users/u/AppData/Local");
    let c = |exe: &str| classify_install(Path::new(exe), Some(home), None, Some(lad));
    assert_eq!(
        c("/home/linuxbrew/.linuxbrew/bin/howtui"),
        InstallMethod::Brew
    );
    assert_eq!(c("/opt/homebrew/bin/howtui"), InstallMethod::Brew);
    assert_eq!(c("/home/u/.cargo/bin/howtui"), InstallMethod::Cargo);
    assert_eq!(
        c("/home/u/.local/bin/howtui"),
        InstallMethod::Script {
            bin_dir: PathBuf::from("/home/u/.local/bin")
        }
    );
    assert_eq!(
        c("/c/Users/u/AppData/Local/Programs/howtui/bin/howtui.exe"),
        InstallMethod::Script {
            bin_dir: PathBuf::from("/c/Users/u/AppData/Local/Programs/howtui/bin")
        }
    );
    assert_eq!(c("/usr/local/bin/howtui"), InstallMethod::Unknown);
    let custom_cargo = classify_install(
        Path::new("/opt/cargo/bin/howtui"),
        Some(home),
        Some(Path::new("/opt/cargo")),
        None,
    );
    assert_eq!(custom_cargo, InstallMethod::Cargo);
}

#[test]
fn checksum_lines_and_asset_names() {
    let sums = "AAA  howtui-0.2.0-x86_64-unknown-linux-gnu.tar.gz\nbbb *howtui-0.2.0-x86_64-pc-windows-msvc.zip\n";
    assert_eq!(
        expected_sha256(sums, "howtui-0.2.0-x86_64-unknown-linux-gnu.tar.gz").as_deref(),
        Some("aaa")
    );
    assert_eq!(
        expected_sha256(sums, "howtui-0.2.0-x86_64-pc-windows-msvc.zip").as_deref(),
        Some("bbb")
    );
    assert_eq!(expected_sha256(sums, "other"), None);
    assert_eq!(
        asset_name("0.2.0", "aarch64-pc-windows-msvc"),
        "howtui-0.2.0-aarch64-pc-windows-msvc.zip"
    );
    assert_eq!(
        asset_name("0.2.0", "aarch64-apple-darwin"),
        "howtui-0.2.0-aarch64-apple-darwin.tar.gz"
    );
    assert!(release_target().is_some(), "릴리스 대상 플랫폼에서만 빌드");
}

pub(crate) fn tar_gz(files: &[(&str, &[u8])]) -> Vec<u8> {
    let enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    let mut b = tar::Builder::new(enc);
    for (name, data) in files {
        let mut h = tar::Header::new_gnu();
        h.set_size(data.len() as u64);
        h.set_mode(0o755);
        h.set_cksum();
        b.append_data(&mut h, name, *data).unwrap();
    }
    b.into_inner().unwrap().finish().unwrap()
}

pub(crate) fn zip_of(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut w = zip::ZipWriter::new(io::Cursor::new(Vec::new()));
    let opts =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, data) in files {
        w.start_file(*name, opts).unwrap();
        w.write_all(data).unwrap();
    }
    w.finish().unwrap().into_inner()
}

#[test]
fn extracts_binary_and_uninstall_script() {
    let t = tar_gz(&[
        ("howtui", b"BIN"),
        ("uninstall.sh", b"UN"),
        ("README.md", b"r"),
    ]);
    assert_eq!(
        extract(&t, false).unwrap(),
        (b"BIN".to_vec(), Some(b"UN".to_vec()))
    );
    let z = zip_of(&[("howtui.exe", b"EXE"), ("LICENSE", b"l")]);
    assert_eq!(extract(&z, true).unwrap(), (b"EXE".to_vec(), None));
    assert!(matches!(
        extract(&tar_gz(&[("README.md", b"r")]), false),
        Err(UpdateError::Archive(_))
    ));
    assert!(matches!(
        extract(b"not an archive", true),
        Err(UpdateError::Archive(_))
    ));
}

#[test]
fn replace_file_swaps_content_and_mode() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("howtui");
    fs::write(&p, b"old").unwrap();
    replace_file(&p, b"new", true).unwrap();
    assert_eq!(fs::read(&p).unwrap(), b"new");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&p).unwrap().permissions().mode() & 0o777,
            0o755
        );
    }
    let leftovers: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.contains(".new."))
        .collect();
    assert!(leftovers.is_empty(), "{leftovers:?}");
}

/// 리다이렉트 조회, 즉시 확인의 캐시 갱신, 접속 불가.
#[tokio::test]
async fn check_now_uses_redirect_and_updates_cache() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/releases/latest"))
        .respond_with(
            ResponseTemplate::new(302)
                .insert_header("location", format!("{}/releases/tag/v9.9.9", server.uri())),
        )
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().to_string_lossy().into_owned();
    let latest = format!("{}/releases/latest", server.uri());
    let env = |k: &str| match k {
        "HOME" | "USERPROFILE" | "XDG_CACHE_HOME" | "LOCALAPPDATA" => Some(home.clone()),
        "HOWTUI_LATEST_URL" => Some(latest.clone()),
        _ => None,
    };
    assert_eq!(
        check_now(&env).await,
        CheckOutcome::UpdateAvailable {
            latest: "9.9.9".into()
        }
    );
    let cache = read_cache(&cache_path(&env).unwrap());
    assert_eq!(cache.latest.as_deref(), Some("9.9.9"));
    assert!(cache.checked_at > 0);
    let dead = |k: &str| match k {
        "HOWTUI_LATEST_URL" => Some("http://127.0.0.1:9/releases/latest".to_string()),
        _ => env(k),
    };
    assert_eq!(check_now(&dead).await, CheckOutcome::Unreachable);
    assert_eq!(
        read_cache(&cache_path(&env).unwrap()).latest.as_deref(),
        Some("9.9.9"),
        "실패해도 이전 최신 버전 유지"
    );
}

/// 내려받기, 검증, 교체. 체크섬 불일치 시 기존 파일 유지.
#[tokio::test]
async fn apply_verifies_and_replaces() {
    let target = release_target().unwrap();
    let asset = asset_name("9.9.9", target);
    let zip = asset.ends_with(".zip");
    let (bin, un) = if zip {
        ("howtui.exe", "howtui-uninstall.ps1")
    } else {
        ("howtui", "howtui-uninstall.sh")
    };
    let archive = if zip {
        zip_of(&[("howtui.exe", b"NEW-BIN"), ("uninstall.ps1", b"NEW-UN")])
    } else {
        tar_gz(&[("howtui", b"NEW-BIN"), ("uninstall.sh", b"NEW-UN")])
    };
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v9.9.9/{asset}")))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(archive.clone()))
        .mount(&server)
        .await;
    let good = format!("{}  {asset}\n", sha256_hex(&archive));
    Mock::given(method("GET"))
        .and(path("/v9.9.9/SHA256SUMS"))
        .respond_with(ResponseTemplate::new(200).set_body_string(good))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v9.9.8/SHA256SUMS"))
        .respond_with(ResponseTemplate::new(200).set_body_string(format!(
            "{}  {}\n",
            "0".repeat(64),
            asset_name("9.9.8", target)
        )))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v9.9.8/{}", asset_name("9.9.8", target))))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(archive.clone()))
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join(bin), b"OLD-BIN").unwrap();
    fs::write(dir.path().join(un), b"OLD-UN").unwrap();
    let base = server.uri();
    let env = |k: &str| (k == "HOWTUI_DOWNLOAD_BASE").then(|| base.clone());
    assert!(matches!(
        apply(&env, "9.9.8", dir.path(), &|_| {}).await,
        Err(UpdateError::Checksum(_))
    ));
    assert_eq!(fs::read(dir.path().join(bin)).unwrap(), b"OLD-BIN");
    apply(&env, "9.9.9", dir.path(), &|_| {}).await.unwrap();
    assert_eq!(fs::read(dir.path().join(bin)).unwrap(), b"NEW-BIN");
    assert_eq!(fs::read(dir.path().join(un)).unwrap(), b"NEW-UN");
    assert!(matches!(
        apply(&env, "9.9.7", dir.path(), &|_| {}).await,
        Err(UpdateError::Network(_))
    ));
}
