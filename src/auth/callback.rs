//! OAuth 루프백 콜백 처리와 붙여넣은 리다이렉트 URL 검증 (SRS-AUTH-003, SRS-AUTH-006).

use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

pub const CALLBACK_PATH: &str = "/auth/callback";
const MAX_REQUEST: usize = 16 * 1024;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CallbackParams {
    pub code: Option<String>,
    pub state: Option<String>,
    pub client_id: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum PasteError {
    #[error("not a URL")]
    NotUrl,
    #[error("unexpected scheme, host, port or path")]
    WrongTarget,
}

pub fn redirect_uri(port: u16) -> String {
    format!("http://127.0.0.1:{port}{CALLBACK_PATH}")
}

fn params_from_query(query: &str) -> CallbackParams {
    let mut p = CallbackParams::default();
    for (k, v) in url::form_urlencoded::parse(query.as_bytes()) {
        let v = Some(v.into_owned());
        match k.as_ref() {
            "code" => p.code = v,
            "state" => p.state = v,
            "client_id" => p.client_id = v,
            "error" => p.error = v,
            "error_description" => p.error_description = v,
            _ => {}
        }
    }
    p
}

/// 붙여넣은 URL 검증: `http://127.0.0.1:<port>/auth/callback?...` 형식만 허용.
pub fn parse_pasted_url(text: &str, port: u16) -> Result<CallbackParams, PasteError> {
    // 터미널 줄바꿈으로 끼어든 공백 제거, 주소창 복사 시 빠진 http:// 보완 (CR-002)
    let text: String = text
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .trim_matches(|c| c == '"' || c == '\'' || c == '<' || c == '>')
        .to_string();
    let text = if text.starts_with("127.0.0.1:") {
        format!("http://{text}")
    } else {
        text
    };
    let url = url::Url::parse(&text).map_err(|_| PasteError::NotUrl)?;
    let ok = url.scheme() == "http"
        && url.host_str() == Some("127.0.0.1")
        && url.port() == Some(port)
        && url.path() == CALLBACK_PATH;
    if !ok {
        return Err(PasteError::WrongTarget);
    }
    Ok(params_from_query(url.query().unwrap_or("")))
}

const SUCCESS_HTML: &str = "<!doctype html><html><head><meta charset=\"utf-8\"><title>howtui</title></head>\
<body style=\"font-family:sans-serif;margin:3em\"><h2>howtui</h2>\
<p>Sign-in finished. You can close this tab and return to the terminal.</p>\
<p>로그인 처리 완료. 이 탭을 닫고 터미널로 돌아가세요.</p></body></html>";

async fn respond(stream: &mut TcpStream, status: &str, body: &str) {
    let resp = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(resp.as_bytes()).await;
    let _ = stream.shutdown().await;
}

/// 요청 1건 처리. 콜백 경로이면 매개변수 반환, 그 외 404.
async fn handle(mut stream: TcpStream, port: u16) -> Option<CallbackParams> {
    let mut buf = Vec::with_capacity(2048);
    let mut chunk = [0u8; 2048];
    loop {
        let read = tokio::time::timeout(Duration::from_secs(5), stream.read(&mut chunk)).await;
        let n = match read {
            Ok(Ok(0)) | Err(_) | Ok(Err(_)) => break,
            Ok(Ok(n)) => n,
        };
        buf.extend_from_slice(&chunk[..n]);
        if buf.windows(4).any(|w| w == b"\r\n\r\n") || buf.len() > MAX_REQUEST {
            break;
        }
    }
    let mut headers = [httparse::EMPTY_HEADER; 32];
    let mut req = httparse::Request::new(&mut headers);
    if !matches!(req.parse(&buf), Ok(httparse::Status::Complete(_))) {
        respond(&mut stream, "400 Bad Request", "bad request").await;
        return None;
    }
    let host_ok = req.headers.iter().any(|h| {
        h.name.eq_ignore_ascii_case("host")
            && std::str::from_utf8(h.value).is_ok_and(|v| v == format!("127.0.0.1:{port}"))
    });
    let path = req.path.unwrap_or("");
    let (route, query) = path.split_once('?').unwrap_or((path, ""));
    if req.method != Some("GET") || route != CALLBACK_PATH || !host_ok {
        respond(&mut stream, "404 Not Found", "not found").await;
        return None;
    }
    let params = params_from_query(query);
    respond(&mut stream, "200 OK", SUCCESS_HTML).await;
    Some(params)
}

/// 콜백 요청이 올 때까지 연결을 받는다.
pub async fn wait_callback(listener: &TcpListener, port: u16) -> std::io::Result<CallbackParams> {
    loop {
        let (stream, peer) = listener.accept().await?;
        if !peer.ip().is_loopback() {
            continue;
        }
        if let Some(params) = handle(stream, port).await {
            return Ok(params);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pasted_url_validation() {
        let ok = parse_pasted_url(
            "  \"http://127.0.0.1:1455/auth/callback?code=abc&state=xyz&client_id=oaiapp_1\" ",
            1455,
        )
        .unwrap();
        assert_eq!(ok.code.as_deref(), Some("abc"));
        assert_eq!(ok.state.as_deref(), Some("xyz"));
        assert_eq!(ok.client_id.as_deref(), Some("oaiapp_1"));
        assert_eq!(
            parse_pasted_url("http://localhost:1455/auth/callback?code=a", 1455),
            Err(PasteError::WrongTarget)
        );
        assert_eq!(
            parse_pasted_url("http://127.0.0.1:1456/auth/callback?code=a", 1455),
            Err(PasteError::WrongTarget)
        );
        assert_eq!(
            parse_pasted_url("http://127.0.0.1:1455/callback?code=a", 1455),
            Err(PasteError::WrongTarget)
        );
        assert_eq!(
            parse_pasted_url("https://127.0.0.1:1455/auth/callback?code=a", 1455),
            Err(PasteError::WrongTarget)
        );
        assert_eq!(parse_pasted_url("abc", 1455), Err(PasteError::NotUrl));
    }

    #[test]
    fn pasted_url_without_scheme_or_with_line_breaks() {
        let p = parse_pasted_url("127.0.0.1:1455/auth/callback?code=abc&state=xyz", 1455).unwrap();
        assert_eq!(p.code.as_deref(), Some("abc"));
        let p = parse_pasted_url(
            "http://127.0.0.1:1455/auth/call\nback?code=abc&st\r\nate=xyz",
            1455,
        )
        .unwrap();
        assert_eq!(p.state.as_deref(), Some("xyz"));
    }

    #[test]
    fn denied_callback_params() {
        let p = parse_pasted_url(
            "http://127.0.0.1:9/auth/callback?error=access_denied&state=s",
            9,
        )
        .unwrap();
        assert_eq!(p.error.as_deref(), Some("access_denied"));
    }

    async fn send(port: u16, request: &str) -> String {
        let mut s = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        s.write_all(request.as_bytes()).await.unwrap();
        let mut out = String::new();
        s.read_to_string(&mut out).await.unwrap();
        out
    }

    #[tokio::test]
    async fn listener_ignores_other_paths_and_returns_callback() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let waiter = tokio::spawn(async move { wait_callback(&listener, port).await.unwrap() });
        let r = send(
            port,
            &format!("GET /favicon.ico HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n"),
        )
        .await;
        assert!(r.starts_with("HTTP/1.1 404"));
        let r = send(
            port,
            "GET /auth/callback?code=x HTTP/1.1\r\nHost: evil.example\r\n\r\n",
        )
        .await;
        assert!(r.starts_with("HTTP/1.1 404"), "Host 헤더 불일치 거부");
        let r = send(
            port,
            &format!(
                "GET /auth/callback?code=c1&state=s1 HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n"
            ),
        )
        .await;
        assert!(r.starts_with("HTTP/1.1 200"));
        let params = waiter.await.unwrap();
        assert_eq!(params.code.as_deref(), Some("c1"));
        assert_eq!(params.state.as_deref(), Some("s1"));
    }
}
