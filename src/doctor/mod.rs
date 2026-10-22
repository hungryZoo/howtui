//! howtui doctor (SRS 15절).

use std::process::ExitCode;

pub fn run(
    _loaded: &crate::config::Loaded,
    _rt: &tokio::runtime::Runtime,
    _json: bool,
) -> ExitCode {
    ExitCode::from(1)
}
