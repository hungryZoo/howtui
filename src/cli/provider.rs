//! Provider와 인증 구성.

use std::process::ExitCode;
use std::sync::Arc;

use crate::config::Loaded;
use crate::llm::types::Provider;
use crate::runtime::LoginService;

pub struct Built {
    pub provider: Arc<dyn Provider>,
    pub login: Option<Arc<dyn LoginService>>,
    pub logged_in: bool,
}

pub fn build(_loaded: &Loaded) -> Result<Built, String> {
    #[cfg(feature = "mock-provider")]
    {
        if let Some(path) = crate::platform::paths::process_env("HOWTUI_MOCK_SCENARIO") {
            let mock = if path == "echo" {
                crate::llm::mock::MockProvider::echo()
            } else {
                crate::llm::mock::MockProvider::from_file(std::path::Path::new(&path))?
            };
            return Ok(Built {
                provider: Arc::new(mock),
                login: None,
                logged_in: true,
            });
        }
    }
    Err("ChatGPT provider not available in this build yet".into())
}

pub fn login_command(
    _loaded: &Loaded,
    _rt: &tokio::runtime::Runtime,
    _no_browser: bool,
    _port: Option<u16>,
) -> ExitCode {
    ExitCode::from(1)
}

pub fn logout_command(_loaded: &Loaded, _rt: &tokio::runtime::Runtime, _forget: bool) -> ExitCode {
    ExitCode::from(1)
}

pub fn status_command(_loaded: &Loaded, _rt: &tokio::runtime::Runtime) -> ExitCode {
    ExitCode::from(1)
}
