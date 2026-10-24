//! 네트워크 보안 시험 (TC-NFR-006): 자체 서명 인증서 서버 연결 거부, 시스템 인증서 저장소만 신뢰.

use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// 자체 서명 인증서로 HTTPS 응답하는 서버 (연결 성공 시 200 응답).
async fn self_signed_server() -> u16 {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let cert =
        rcgen::generate_simple_self_signed(vec!["127.0.0.1".into(), "localhost".into()]).unwrap();
    let der = cert.cert.der().clone();
    let key = rustls::pki_types::PrivateKeyDer::Pkcs8(cert.signing_key.serialize_der().into());
    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![der], key)
        .unwrap();
    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        while let Ok((sock, _)) = listener.accept().await {
            let acceptor = acceptor.clone();
            tokio::spawn(async move {
                if let Ok(mut tls) = acceptor.accept(sock).await {
                    let mut buf = [0u8; 4096];
                    let _ = tls.read(&mut buf).await;
                    let _ = tls
                        .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\n\r\nok")
                        .await;
                }
            });
        }
    });
    port
}

#[tokio::test]
async fn self_signed_certificate_is_rejected() {
    let port = self_signed_server().await;
    let client = howtui::llm::http::client();
    let err = client
        .get(format!("https://127.0.0.1:{port}/v1/models"))
        .send()
        .await
        .expect_err("self-signed certificate must be rejected");
    assert!(err.is_connect() || err.is_request(), "{err:?}");
    let chain = format!("{err:?}").to_lowercase();
    assert!(
        chain.contains("certificate") || chain.contains("unknownissuer"),
        "{chain}"
    );
}
