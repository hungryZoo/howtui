//! 바이너리 통합 테스트 (TC-CLI, TC-OUT, TC-CFG, TC-DOC, TC-I18N, TC-PRIV 일부).
//! Mock Provider와 테스트 엔드포인트 재정의가 필요하므로 `mock-provider` 기능에서만 실행.
#![cfg(feature = "mock-provider")]

use std::path::Path;
use std::process::Output;

use assert_cmd::Command;
use serde_json::{Value, json};

fn bin(data: &Path) -> Command {
    let mut cmd = Command::cargo_bin("howtui").unwrap();
    cmd.env_clear()
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .env("HOME", data)
        .env("HOWTUI_DATA_DIR", data.join("data"))
        .env("HOWTUI_CONFIG", data.join("config.toml"))
        .env("HOWTUI_LANG", "en");
    #[cfg(windows)]
    {
        for k in [
            "SYSTEMROOT",
            "TEMP",
            "TMP",
            "USERPROFILE",
            "APPDATA",
            "LOCALAPPDATA",
        ] {
            if let Ok(v) = std::env::var(k) {
                cmd.env(k, v);
            }
        }
    }
    cmd
}

fn fixture(name: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
        .display()
        .to_string()
}

fn scenario(dir: &Path, steps: Value) -> String {
    let path = dir.join("scenario.json");
    std::fs::write(
        &path,
        json!({ "model": "mock-model", "steps": steps }).to_string(),
    )
    .unwrap();
    path.display().to_string()
}

fn no_esc(out: &[u8]) -> bool {
    !out.contains(&0x1b)
}

fn run(cmd: &mut Command) -> Output {
    cmd.output().unwrap()
}

#[test]
fn version_prints_semver() {
    let dir = tempfile::tempdir().unwrap();
    let out = run(bin(dir.path()).arg("--version"));
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        format!("howtui {}\n", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn help_lists_commands_and_exit_codes() {
    let dir = tempfile::tempdir().unwrap();
    let out = run(bin(dir.path()).arg("--help"));
    let text = String::from_utf8(out.stdout).unwrap();
    for word in [
        "login",
        "logout",
        "auth",
        "doctor",
        "init",
        "Exit codes",
        "never runs commands",
    ] {
        assert!(text.contains(word), "{word} missing in help");
    }
    let ko = run(bin(dir.path()).env("HOWTUI_LANG", "ko").arg("--help"));
    assert!(String::from_utf8(ko.stdout).unwrap().contains("종료 코드"));
}

#[test]
fn usage_error_exits_2_without_stdout() {
    let dir = tempfile::tempdir().unwrap();
    let out = run(bin(dir.path()).arg("--unknown-flag"));
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    let out = run(bin(dir.path()).args(["--height", "99"]));
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn no_tui_prints_only_command_on_stdout() {
    let dir = tempfile::tempdir().unwrap();
    let out = run(bin(dir.path())
        .env("HOWTUI_MOCK_SCENARIO", fixture("mock_basic.json"))
        .args(["--no-tui", "list", "files"]));
    assert!(out.status.success(), "{out:?}");
    assert_eq!(
        out.stdout,
        "printf '%s\\n' \"안녕 세계 🌏\" \\\n  | grep -n '세계'\n".as_bytes()
    );
    let err = String::from_utf8(out.stderr.clone()).unwrap();
    assert!(err.contains("Prints a greeting"));
    assert!(no_esc(&out.stdout) && no_esc(&out.stderr));
}

#[test]
fn no_tui_warns_on_stderr_for_risky_commands() {
    let dir = tempfile::tempdir().unwrap();
    let s = scenario(
        dir.path(),
        json!([{ "command": "rm -rf ./build", "explanation": "deletes" }]),
    );
    let out = run(bin(dir.path())
        .env("HOWTUI_MOCK_SCENARIO", s)
        .args(["--no-tui", "clean"]));
    assert!(out.status.success());
    assert_eq!(out.stdout, b"rm -rf ./build\n");
    assert!(String::from_utf8(out.stderr).unwrap().contains("High risk"));
}

#[test]
fn no_tui_errors_map_to_exit_codes() {
    let dir = tempfile::tempdir().unwrap();
    for (error, code) in [
        ("usage_limit", 5),
        ("auth_required", 4),
        ("network", 6),
        ("unsupported", 7),
        ("client_not_enabled", 5),
    ] {
        let s = scenario(dir.path(), json!([{ "error": error }]));
        let out = run(bin(dir.path())
            .env("HOWTUI_MOCK_SCENARIO", s)
            .args(["--no-tui", "q"]));
        assert_eq!(out.status.code(), Some(code), "{error}");
        assert!(out.stdout.is_empty(), "{error}: stdout must be empty");
        assert!(!out.stderr.is_empty() && no_esc(&out.stderr));
    }
}

#[test]
fn no_tui_retries_invalid_response_once() {
    let dir = tempfile::tempdir().unwrap();
    let s = scenario(
        dir.path(),
        json!([{ "raw": "I cannot format this" }, { "command": "pwd" }]),
    );
    let out = run(bin(dir.path())
        .env("HOWTUI_MOCK_SCENARIO", s)
        .args(["--no-tui", "where am i"]));
    assert!(out.status.success(), "{out:?}");
    assert_eq!(out.stdout, b"pwd\n");
}

#[test]
fn no_tui_requires_question_and_login() {
    let dir = tempfile::tempdir().unwrap();
    let out = run(bin(dir.path())
        .env("HOWTUI_MOCK_SCENARIO", "echo")
        .arg("--no-tui"));
    assert_eq!(out.status.code(), Some(2));
    let out = run(bin(dir.path()).args(["--no-tui", "q"]));
    assert_eq!(out.status.code(), Some(4), "not signed in");
    assert!(out.stdout.is_empty());
}

#[test]
fn config_errors_exit_8_with_location() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("config.toml"),
        "[ui]\ninline_height = = 3\n",
    )
    .unwrap();
    let out = run(bin(dir.path())
        .env("HOWTUI_MOCK_SCENARIO", "echo")
        .args(["--no-tui", "q"]));
    assert_eq!(out.status.code(), Some(8));
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("line 2"), "{err}");
    assert!(out.stdout.is_empty());
}

#[test]
fn unknown_config_keys_are_not_fatal_and_doctor_warns() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("config.toml"), "[ui]\ncolour = 1\n").unwrap();
    let out = run(bin(dir.path())
        .env("HOWTUI_MOCK_SCENARIO", "echo")
        .args(["--no-tui", "q"]));
    assert!(out.status.success());
    let out = run(bin(dir.path()).args(["doctor", "--json"]));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    let config = v["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "config")
        .unwrap();
    assert_eq!(config["status"], "warn");
    assert_eq!(config["details"]["unknown_keys"][0], "ui.colour");
}

#[test]
fn doctor_json_is_stable_and_language_independent() {
    let dir = tempfile::tempdir().unwrap();
    let en = run(bin(dir.path()).args(["doctor", "--json"]));
    let ko = run(bin(dir.path())
        .env("HOWTUI_LANG", "ko")
        .args(["doctor", "--json"]));
    assert_eq!(en.status.code(), Some(1), "auth fails when signed out");
    let parse = |o: &Output| -> Value { serde_json::from_slice(&o.stdout).unwrap() };
    let (a, b) = (parse(&en), parse(&ko));
    assert_eq!(a["schema_version"], 1);
    let ids = |v: &Value| {
        v["checks"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| (c["id"].clone(), c["status"].clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(&a), ids(&b));
    for c in a["checks"].as_array().unwrap() {
        assert!(["pass", "warn", "fail", "info", "skip"].contains(&c["status"].as_str().unwrap()));
    }
    let home = dir.path().display().to_string();
    assert!(
        !String::from_utf8(en.stdout).unwrap().contains(&home),
        "home path masked"
    );
}

#[test]
fn auth_status_signed_out_exits_4() {
    let dir = tempfile::tempdir().unwrap();
    let out = run(bin(dir.path()).args(["auth", "status"]));
    assert_eq!(out.status.code(), Some(4));
}

/// 쉘 통합 스크립트 출력과 키 검증 (TC-SH-008, TC-SH-004). 설정 파일 오류와 무관하게 동작.
#[test]
fn init_prints_scripts_and_validates_key() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("config.toml"), "[ui\n").unwrap();
    for (shell, marker) in [
        ("bash", "eval \"$(howtui init bash)\""),
        ("zsh", "eval \"$(howtui init zsh)\""),
        ("powershell", "Invoke-Expression"),
        ("pwsh", "Invoke-Expression"),
    ] {
        let out = run(bin(dir.path()).args(["init", shell]));
        assert!(out.status.success(), "{shell}");
        let text = String::from_utf8(out.stdout).unwrap();
        assert!(text.contains(marker), "{shell}: 등록 방법 주석");
        assert!(text.contains("Disable:"), "{shell}: 제거 방법 주석");
        assert!(text.contains("Ctrl+G"));
        assert!(out.stderr.is_empty());
    }
    let ps = run(bin(dir.path()).args(["init", "powershell"]));
    assert!(ps.stdout.is_ascii());
    let out = run(bin(dir.path()).args(["init", "bash", "--key", "alt+x", "--no-wrap"]));
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains(r#""\ex": __howtui_widget"#));
    assert!(!text.contains("howtui() {"));
    for bad in [
        ["init", "bash", "--key", "ctrl+c"],
        ["init", "bash", "--key", "f5"],
    ] {
        let out = run(bin(dir.path()).args(bad));
        assert_eq!(out.status.code(), Some(2), "{bad:?}");
        assert!(out.stdout.is_empty());
        assert!(!out.stderr.is_empty());
    }
    let out = run(bin(dir.path()).args(["init", "fish"]));
    assert_eq!(out.status.code(), Some(2));
}

/// 위젯 모드는 쉘 스크립트가 만든 출력 파일 경로 필수 (SRS 9.2절).
#[test]
fn widget_without_output_file_exits_2() {
    let dir = tempfile::tempdir().unwrap();
    let out = run(bin(dir.path())
        .env("HOWTUI_MOCK_SCENARIO", "echo")
        .env("HOWTUI_BUFFER", "ls")
        .arg("--widget"));
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .contains("HOWTUI_OUTPUT_FILE")
    );
}

#[cfg(unix)]
#[test]
fn tui_without_controlling_terminal_exits_3() {
    use std::os::unix::process::CommandExt;
    let dir = tempfile::tempdir().unwrap();
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_howtui"));
    cmd.env("HOWTUI_MOCK_SCENARIO", "echo")
        .env("HOWTUI_LANG", "en")
        .env("HOWTUI_DATA_DIR", dir.path());
    unsafe {
        cmd.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    let out = cmd.stdin(std::process::Stdio::null()).output().unwrap();
    assert_eq!(out.status.code(), Some(3));
    assert!(out.stdout.is_empty());
    assert!(
        no_esc(&out.stderr),
        "{:?}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// 로그 파일에 질문, 명령어, 토큰이 기록되지 않고 stdout과 stderr에 로그가 섞이지 않음 (TC-OUT-011, TC-PRIV-003).
#[test]
fn logging_goes_to_file_without_content() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("howtui.log");
    let s = scenario(
        dir.path(),
        json!([{ "command": "echo secret-command-text" }]),
    );
    let out = run(bin(dir.path())
        .env("HOWTUI_MOCK_SCENARIO", s)
        .env("HOWTUI_LOG", "trace")
        .env("HOWTUI_LOG_FILE", &log)
        .args(["--no-tui", "secret-question-text"]));
    assert_eq!(out.stdout, b"echo secret-command-text\n");
    assert!(!String::from_utf8_lossy(&out.stderr).contains("TRACE"));
    let text = std::fs::read_to_string(&log).unwrap_or_default();
    assert!(
        !text.contains("secret-question-text") && !text.contains("secret-command-text"),
        "{text}"
    );
}

mod chatgpt_e2e {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn now() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    fn seed(data: &Path, scope: &str) {
        std::fs::create_dir_all(data).unwrap();
        std::fs::write(
            data.join("state.json"),
            json!({"version": 1, "host_id": "urn:uuid:test", "registration": {"client_id": "oaiapp_1"}, "credential_store": "file"}).to_string(),
        )
        .unwrap();
        std::fs::write(
            data.join("credentials.json"),
            json!({"oaiapp_1": {"access_token": "at-1", "refresh_token": "rt-1", "token_type": "Bearer",
                "scope": scope.split_whitespace().collect::<Vec<_>>(), "expires_at": now() + 3600, "saved_at": now()}})
            .to_string(),
        )
        .unwrap();
    }

    fn sse(text: &str) -> String {
        let done = json!({"type": "response.completed", "response": {"output": [{"type": "message", "content": [{"type": "output_text", "text": text}]}]}});
        format!("event: response.completed\ndata: {done}\n\n")
    }

    /// 로그인 상태의 ChatGPT 경로 전체: 모델 목록, 스트리밍 추론, 전송 필드 한정 (TC-LLM-014, TC-PRIV-001).
    #[tokio::test(flavor = "multi_thread")]
    async fn generates_through_chatgpt_api() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"models": [{"slug": "model-a", "visibility": "list"}]})),
            )
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/responses"))
            .respond_with(ResponseTemplate::new(200).set_body_string(sse(
                r#"{"command":"du -sh *","explanation":"sizes","notes":[]}"#,
            )))
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        seed(&dir.path().join("data"), "openid chatgpt.tokens.use.direct");
        let uri = server.uri();
        let d = dir.path().to_path_buf();
        let out = tokio::task::spawn_blocking(move || {
            bin(&d)
                .env("HOWTUI_TEST_API_BASE", format!("{uri}/v1"))
                .env("SECRET_ENV_FOR_TEST", "must-not-leak")
                .current_dir(&d)
                .args(["--no-tui", "--shell", "zsh", "disk usage"])
                .output()
                .unwrap()
        })
        .await
        .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(out.stdout, b"du -sh *\n");
        let reqs = server.received_requests().await.unwrap();
        let body = reqs
            .iter()
            .find(|r| r.url.path() == "/v1/responses")
            .unwrap();
        let raw = String::from_utf8_lossy(&body.body);
        assert!(
            !raw.contains("must-not-leak"),
            "environment variables must not be sent"
        );
        assert!(
            !raw.contains(&dir.path().display().to_string()),
            "working directory must not be sent"
        );
        let v: Value = serde_json::from_slice(&body.body).unwrap();
        assert_eq!(v["store"], false);
        assert_eq!(v["stream"], true);
        assert!(v["instructions"].as_str().unwrap().contains("shell=Zsh"));
    }

    /// 플랜 사용 불가: 원인 안내 후 중단, 추론 요청 없음 (TC-AUTH-024).
    #[tokio::test(flavor = "multi_thread")]
    async fn plan_not_granted_stops_without_request() {
        let server = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();
        seed(&dir.path().join("data"), "openid profile");
        let uri = server.uri();
        let d = dir.path().to_path_buf();
        let out = tokio::task::spawn_blocking(move || {
            bin(&d)
                .env("HOWTUI_TEST_API_BASE", format!("{uri}/v1"))
                .args(["--no-tui", "q"])
                .output()
                .unwrap()
        })
        .await
        .unwrap();
        assert_eq!(out.status.code(), Some(5));
        assert!(out.stdout.is_empty());
        assert!(String::from_utf8_lossy(&out.stderr).contains("not granted"));
        assert!(server.received_requests().await.unwrap().is_empty());
    }

    /// 서버 측 사용 불가 코드 (TC-AUTH-024).
    #[tokio::test(flavor = "multi_thread")]
    async fn client_not_enabled_is_reported() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(403).set_body_json(
                json!({"error": {"code": "subscription_sharing_v2_client_not_enabled"}}),
            ))
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        seed(&dir.path().join("data"), "chatgpt.tokens.use.direct");
        let uri = server.uri();
        let d = dir.path().to_path_buf();
        let out = tokio::task::spawn_blocking(move || {
            bin(&d)
                .env("HOWTUI_TEST_API_BASE", format!("{uri}/v1"))
                .args(["--no-tui", "q"])
                .output()
                .unwrap()
        })
        .await
        .unwrap();
        assert_eq!(out.status.code(), Some(5));
        assert!(String::from_utf8_lossy(&out.stderr).contains("not enabled"));
    }
}

mod signed_in {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// 로그인 상태 출력: 마스킹 이메일, 토큰 미출력 (TC-AUTH-015).
    #[test]
    fn auth_status_masks_email_and_hides_tokens() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("data");
        chatgpt_e2e_seed(&data);
        let out = run(bin(dir.path()).args(["auth", "status"]));
        assert!(out.status.success());
        let text = String::from_utf8(out.stdout).unwrap();
        assert!(text.contains("k***@example.com"), "{text}");
        assert!(
            !text.contains("kim@example.com") && !text.contains("at-1") && !text.contains("rt-1"),
            "{text}"
        );
        assert!(text.contains("UTC"));
    }

    /// doctor Provider 점검: 모델 목록 1회 조회, 추론 요청 없음 (TC-DOC-003).
    #[tokio::test(flavor = "multi_thread")]
    async fn doctor_lists_models_without_inference() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"models": [{"slug": "model-a", "visibility": "list"}]})),
            )
            .expect(1)
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        chatgpt_e2e_seed(&dir.path().join("data"));
        let uri = server.uri();
        let d = dir.path().to_path_buf();
        let out = tokio::task::spawn_blocking(move || {
            bin(&d)
                .env("HOWTUI_TEST_API_BASE", format!("{uri}/v1"))
                .args(["doctor", "--json"])
                .output()
                .unwrap()
        })
        .await
        .unwrap();
        let v: Value = serde_json::from_slice(&out.stdout).unwrap();
        let provider = v["checks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == "provider")
            .unwrap();
        assert_eq!(provider["status"], "pass", "{provider}");
        assert_eq!(provider["details"]["selected"], "model-a");
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(
            !text.contains("at-1") && !text.contains("kim@example.com"),
            "토큰과 이메일 비공개"
        );
        let reqs = server.received_requests().await.unwrap();
        assert!(reqs.iter().all(|r| r.url.path() != "/v1/responses"));
    }

    fn chatgpt_e2e_seed(data: &Path) {
        std::fs::create_dir_all(data).unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        std::fs::write(
            data.join("state.json"),
            json!({"version": 1, "registration": {"client_id": "oaiapp_1", "email": "kim@example.com"}, "credential_store": "file"}).to_string(),
        )
        .unwrap();
        std::fs::write(
            data.join("credentials.json"),
            json!({"oaiapp_1": {"access_token": "at-1", "refresh_token": "rt-1", "scope": ["chatgpt.tokens.use.direct"], "expires_at": now + 3600, "saved_at": now}})
                .to_string(),
        )
        .unwrap();
    }
}
