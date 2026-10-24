//! 실제 ChatGPT 경로의 PTY E2E (TC-AUTH-016, TC-PRIV-005): Mock 인증 서버와 Mock API 서버를 붙여
//! 의사 터미널에서 TUI 로그인, 제출 전 전송 없음, 위젯 모드 버퍼 전송을 확인한다.
#![cfg(all(unix, feature = "mock-provider"))]

use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use howtui::auth::idtoken::test_support;
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

struct Session {
    output: Arc<Mutex<Vec<u8>>>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
}

impl Session {
    fn start(args: &[&str], envs: &[(&str, String)]) -> Session {
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: 30,
                cols: 200,
                pixel_width: 0,
                pixel_height: 0,
            })
            .unwrap();
        let mut cmd = CommandBuilder::new(env!("CARGO_BIN_EXE_howtui"));
        cmd.args(args);
        cmd.env_clear();
        cmd.env("PATH", std::env::var("PATH").unwrap_or_default());
        cmd.env("TERM", "xterm-256color");
        cmd.env("LANG", "C.UTF-8");
        cmd.env("HOWTUI_LANG", "en");
        cmd.env("HOWTUI_NO_UPDATE_CHECK", "1");
        cmd.env("HOWTUI_NO_BROWSER", "1");
        cmd.env("HOWTUI_CREDENTIAL_STORE", "file");
        for (k, v) in envs {
            cmd.env(k, v);
        }
        let child = pair.slave.spawn_command(cmd).unwrap();
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader().unwrap();
        let writer = Arc::new(Mutex::new(pair.master.take_writer().unwrap()));
        let output = Arc::new(Mutex::new(Vec::new()));
        let (out, w) = (output.clone(), writer.clone());
        std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 {
                    break;
                }
                let chunk = &buf[..n];
                // 커서 위치 질의 응답 (실제 터미널 역할)
                if chunk.windows(4).any(|x| x == b"\x1b[6n") {
                    let _ = w.lock().unwrap().write_all(b"\x1b[1;1R");
                }
                out.lock().unwrap().extend_from_slice(chunk);
            }
        });
        std::mem::forget(pair.master);
        Session {
            output,
            writer,
            child,
        }
    }

    /// 제어 시퀀스를 지운 화면 출력 (그린 순서대로 이어 붙임).
    fn text(&self) -> String {
        let raw = String::from_utf8_lossy(&self.output.lock().unwrap()).into_owned();
        let mut out = String::with_capacity(raw.len());
        let mut chars = raw.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                if chars.peek() == Some(&'[') {
                    chars.next();
                    for d in chars.by_ref() {
                        if ('@'..='~').contains(&d) {
                            break;
                        }
                    }
                } else {
                    chars.next();
                }
            } else if c != '\r' && c != '\n' {
                out.push(c);
            }
        }
        out
    }

    /// 터미널은 공백 칸을 커서 이동으로 그리기도 하므로 공백을 뺀 문자열로 비교.
    fn wait_for(&self, needle: &str, timeout: Duration) -> bool {
        let needle: String = needle.chars().filter(|c| *c != ' ').collect();
        let start = Instant::now();
        while start.elapsed() < timeout {
            let text: String = self.text().chars().filter(|c| *c != ' ').collect();
            if text.contains(&needle) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        false
    }

    fn send(&self, bytes: &[u8]) {
        let mut w = self.writer.lock().unwrap();
        w.write_all(bytes).unwrap();
        w.flush().unwrap();
    }

    fn wait_exit(&mut self, timeout: Duration) -> Option<u32> {
        let start = Instant::now();
        while start.elapsed() < timeout {
            if let Ok(Some(status)) = self.child.try_wait() {
                return Some(status.exit_code());
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let _ = self.child.kill();
        None
    }
}

fn sse(text: &str) -> String {
    let done = json!({"type": "response.completed", "response": {"output": [{"type": "message", "content": [{"type": "output_text", "text": text}]}]}});
    format!("event: response.completed\ndata: {done}\n\n")
}

async fn api_mocks(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"models": [{"slug": "model-a", "visibility": "list"}]})),
        )
        .mount(server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_string(sse(
            r#"{"command":"ls -la","explanation":"lists files","notes":[]}"#,
        )))
        .mount(server)
        .await;
}

async fn responses_bodies(server: &MockServer) -> Vec<String> {
    server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|r: &&Request| r.url.path() == "/v1/responses")
        .map(|r| String::from_utf8_lossy(&r.body).into_owned())
        .collect()
}

fn query_param(text: &str, key: &str) -> Option<String> {
    let start = text.find(&format!("&{key}="))? + key.len() + 2;
    Some(
        text[start..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_' || *c == '%')
            .collect(),
    )
}

/// 자격증명 없음 → 로그인 자동 시작 → 붙여넣기로 콜백 제출 → 토큰 교환과 ID 토큰 검증 → 입력 대기.
/// 이어서 질문 입력만 하고 제출 전에는 추론 요청 0건, 제출 후 1건 (TC-AUTH-016, TC-PRIV-005).
#[tokio::test(flavor = "multi_thread")]
async fn tui_login_then_nothing_sent_before_submit() {
    let server = MockServer::start().await;
    let base = server.uri();
    Mock::given(method("GET"))
        .and(path("/.well-known/jwks.json"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"keys": [test_support::jwk()]})),
        )
        .mount(&server)
        .await;
    api_mocks(&server).await;
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let envs = vec![
        ("HOWTUI_DATA_DIR", data.display().to_string()),
        (
            "HOWTUI_CONFIG",
            dir.path().join("none.toml").display().to_string(),
        ),
        ("HOWTUI_TEST_AUTH_BASE", base.clone()),
        ("HOWTUI_TEST_API_BASE", format!("{base}/v1")),
        ("HOME", dir.path().display().to_string()),
    ];
    let mut s = tokio::task::spawn_blocking(move || Session::start(&[], &envs))
        .await
        .unwrap();
    let s_ref = &s;
    assert!(
        s_ref.wait_for("code_challenge=", Duration::from_secs(10)),
        "authorize URL: {}",
        s_ref.text()
    );
    let text = s.text();
    let state = query_param(&text, "state").expect("state");
    let nonce = query_param(&text, "nonce").expect("nonce");
    let redirect = {
        let start = text.find("redirect_uri=").unwrap() + "redirect_uri=".len();
        let enc: String = text[start..].chars().take_while(|c| *c != '&').collect();
        url::form_urlencoded::parse(format!("x={enc}").as_bytes())
            .next()
            .unwrap()
            .1
            .into_owned()
    };
    let claims = json!({"iss": base, "aud": "oaiapp_1", "exp": now() + 3600, "iat": now(),
        "sub": "user-1", "nonce": nonce, "email": "kim@example.com"});
    Mock::given(method("POST"))
        .and(path("/api/accounts/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "at-1", "refresh_token": "rt-1", "id_token": test_support::sign(&claims, "k1"),
            "token_type": "Bearer", "expires_in": 3600,
            "scope": "openid profile email offline_access resource.invoke chatgpt.tokens.use.direct"
        })))
        .expect(1)
        .mount(&server)
        .await;
    let callback = format!("{redirect}?code=code-1&state={state}&client_id=oaiapp_1");
    // bracketed paste (Enter 없이 자동 제출, CR-002)
    s.send(format!("\x1b[200~{callback}\x1b[201~").as_bytes());
    let s_ref = &s;
    assert!(
        s_ref.wait_for("Describe what you want to do", Duration::from_secs(10)),
        "ready screen: {}",
        s_ref.text()
    );
    assert!(data.join("credentials.json").exists(), "tokens stored");

    // 질문 입력 후 대기: 추론 요청 없음
    s.send(b"list files");
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert!(
        responses_bodies(&server).await.is_empty(),
        "nothing sent before submit"
    );
    s.send(b"\r");
    let s_ref = &s;
    assert!(
        s_ref.wait_for("ls -la", Duration::from_secs(10)),
        "{}",
        s_ref.text()
    );
    let bodies = responses_bodies(&server).await;
    assert_eq!(bodies.len(), 1);
    assert!(bodies[0].contains("list files"));
    s.send(b"\x1b");
    assert_eq!(s.wait_exit(Duration::from_secs(5)), Some(130));
}

/// 위젯 모드: 쉘 입력란 내용은 제출 시에만 전송 (TC-PRIV-005).
#[tokio::test(flavor = "multi_thread")]
async fn widget_buffer_is_sent_only_on_submit() {
    let server = MockServer::start().await;
    api_mocks(&server).await;
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    std::fs::create_dir_all(&data).unwrap();
    std::fs::write(
        data.join("state.json"),
        json!({"version": 1, "host_id": "urn:uuid:test", "registration": {"client_id": "oaiapp_1"}, "credential_store": "file"}).to_string(),
    )
    .unwrap();
    std::fs::write(
        data.join("credentials.json"),
        json!({"oaiapp_1": {"access_token": "at-1", "refresh_token": "rt-1", "token_type": "Bearer",
            "scope": ["openid", "chatgpt.tokens.use.direct"], "expires_at": now() + 3600, "saved_at": now()}})
        .to_string(),
    )
    .unwrap();
    let out_file = dir.path().join("out");
    std::fs::write(&out_file, "").unwrap();
    let envs = vec![
        ("HOWTUI_DATA_DIR", data.display().to_string()),
        (
            "HOWTUI_CONFIG",
            dir.path().join("none.toml").display().to_string(),
        ),
        ("HOWTUI_TEST_API_BASE", format!("{}/v1", server.uri())),
        ("HOWTUI_BUFFER", "tar -x secret-archive.tgz".to_string()),
        ("HOWTUI_OUTPUT_FILE", out_file.display().to_string()),
        ("HOME", dir.path().display().to_string()),
    ];
    let mut s = tokio::task::spawn_blocking(move || Session::start(&["--widget"], &envs))
        .await
        .unwrap();
    let s_ref = &s;
    assert!(
        s_ref.wait_for("secret-archive.tgz", Duration::from_secs(10)),
        "{}",
        s_ref.text()
    );
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert!(
        responses_bodies(&server).await.is_empty(),
        "buffer not sent before submit"
    );
    s.send(b"explain it\r");
    let s_ref = &s;
    assert!(
        s_ref.wait_for("lists files", Duration::from_secs(10)),
        "{}",
        s_ref.text()
    );
    let bodies = responses_bodies(&server).await;
    assert_eq!(bodies.len(), 1);
    assert!(bodies[0].contains("tar -x secret-archive.tgz"));
    // 빈 질문 Enter로 확정 → 출력 파일
    s.send(b"\r");
    assert_eq!(s.wait_exit(Duration::from_secs(5)), Some(0));
    assert_eq!(std::fs::read_to_string(&out_file).unwrap(), "ls -la");
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// API Key Provider TUI 경로와 설정 화면 에이전트 전환 (TC-LLM-018, TC-LLM-025): OpenAI로 생성, Ctrl+O 목록,
/// 키 없는 Anthropic 전환 거부 안내, 키 있는 OpenAI 호환 전환 후 다음 요청은 새 주소, 선택 저장.
#[tokio::test(flavor = "multi_thread")]
async fn api_agent_generate_and_switch_in_settings() {
    let openai = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"data":[{"id":"gpt-6-astra"}]})),
        )
        .mount(&openai)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status":"completed","output":[{"type":"message","content":[{"type":"output_text","text":"{\"command\":\"ls -al\",\"explanation\":\"Lists files from OpenAI.\",\"notes\":[]}"}]}]})))
        .mount(&openai)
        .await;
    let local = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"data":[{"id":"local-model"}]})),
        )
        .mount(&local)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"choices":[{"finish_reason":"stop","message":{"content":"{\"command\":\"ls -al | sort\",\"explanation\":\"Sorted by the local server.\",\"notes\":[]}"}}]})))
        .mount(&local)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let config = dir.path().join("config.toml");
    std::fs::write(
        &config,
        format!("[provider]\ndefault = \"openai\"\n[provider.openai_compatible]\nbase_url = \"{}/v1\"\n", local.uri()),
    )
    .unwrap();
    let envs = vec![
        ("HOWTUI_DATA_DIR", data.display().to_string()),
        ("HOWTUI_CONFIG", config.display().to_string()),
        ("HOWTUI_TEST_OPENAI_BASE", format!("{}/v1", openai.uri())),
        ("OPENAI_API_KEY", "sk-pty".to_string()),
        ("HOME", dir.path().display().to_string()),
    ];
    let mut s = tokio::task::spawn_blocking(move || Session::start(&[], &envs))
        .await
        .unwrap();
    let wait = |s: &Session, needle: &str| {
        assert!(
            s.wait_for(needle, Duration::from_secs(10)),
            "{needle}: {}",
            s.text()
        );
    };
    wait(&s, "[gpt-6-astra]");
    s.send(b"list files\r");
    wait(&s, "Lists files from OpenAI.");

    // 설정 화면: 에이전트 구역 (ChatGPT, OpenAI, Anthropic, Gemini, OpenAI compatible)
    s.send(b"\x0f");
    wait(&s, "(*) OpenAI");
    wait(&s, "no key: howtui auth set-key anthropic");
    // 커서는 현재 모델(목록 6번째 항목)에서 시작. Home으로 처음, ↓2로 Anthropic
    s.send(b"\x1b[H");
    s.send(b"\x1b[B\x1b[B\r");
    wait(&s, "No API key for anthropic");
    // ↓2로 OpenAI compatible 선택
    s.send(b"\x1b[B\x1b[B\r");
    wait(&s, "Agent: OpenAI compatible (saved for next time)");
    s.send(b"sort it\r");
    wait(&s, "ls -al | sort");
    s.send(b"\x1b");
    s.send(b"\x1b");
    assert_eq!(s.wait_exit(Duration::from_secs(5)), Some(130));
    let state: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(data.join("state.json")).unwrap()).unwrap();
    assert_eq!(state["selected_provider"], "openai_compatible");
    // 후속 요청은 이전 대화 이력 포함 (전환 후 이력 유지)
    let reqs = local.received_requests().await.unwrap();
    let body = reqs
        .iter()
        .find(|r| r.url.path() == "/v1/chat/completions")
        .unwrap();
    let raw = String::from_utf8_lossy(&body.body);
    assert!(
        raw.contains("list files") && raw.contains("Lists files from OpenAI."),
        "{raw}"
    );
    assert!(!raw.contains("sk-pty"));
    assert!(
        reqs.iter()
            .all(|r| !r.headers.contains_key("authorization"))
    );
}
