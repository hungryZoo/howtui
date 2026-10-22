//! UI 비의존 애플리케이션 코어 (ADR-0001, ADR-0004).
//! 이 모듈은 crossterm, ratatui, reqwest, tokio::net, std::fs를 사용하지 않는다 (SRS-MOD-002).

pub mod editor;
pub mod state;
pub mod update;

pub use state::*;
pub use update::update;
