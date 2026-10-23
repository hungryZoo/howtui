//! 비대화형 모드 (PRD-CLI-005, SRS-OUT-003). 제어 터미널을 사용하지 않고 ANSI를 출력하지 않는다.

use std::process::ExitCode;

use super::provider::Built;
use crate::app::update::error_message;
use crate::i18n::{Msg, tr, tr_in, trf, trf_in};
use crate::llm::complete_with_retry;
use crate::llm::types::{CancelToken, EnvContext, LlmError, LlmRequest, Operation};
use crate::safety;

pub fn run(
    question: &str,
    built: Built,
    env: EnvContext,
    send_folder: bool,
    rt: &tokio::runtime::Runtime,
) -> ExitCode {
    if question.trim().is_empty() {
        eprintln!("{}", tr(Msg::ErrNoQuestion));
        return ExitCode::from(2);
    }
    if !built.logged_in {
        eprintln!("{}", tr(Msg::ErrAuthRequired));
        return ExitCode::from(4);
    }
    let shell = env.shell;
    let req = LlmRequest {
        id: 1,
        op: Operation::Generate,
        env,
        question: Some(question.trim().to_string()),
        command: None,
        history: vec![],
        include_folder: send_folder,
        folder: send_folder
            .then(|| std::env::current_dir().ok())
            .flatten()
            .map(|d| crate::context::folder_listing(&d)),
    };
    let provider = built.provider.clone();
    let result = rt.block_on(async move {
        provider.prepare().await?;
        complete_with_retry(&*provider, req, CancelToken::new()).await
    });
    match result {
        Ok(resp) => {
            let Some(command) = resp.command.as_deref() else {
                if !resp.explanation.is_empty() {
                    eprintln!("{}", resp.explanation);
                }
                eprintln!("{}", tr(Msg::ErrNoCommand));
                return ExitCode::from(7);
            };
            if let Err(e) = crate::output::emit_stdout(command) {
                eprintln!("{}", trf(Msg::ErrInternal, &[("detail", &e.to_string())]));
                return ExitCode::from(1);
            }
            if !resp.explanation.is_empty() {
                eprintln!("{}", resp.explanation);
            }
            for note in &resp.notes {
                eprintln!("{}: {note}", tr(Msg::LabelNotes));
            }
            let lang = crate::i18n::lang();
            for f in safety::analyze(command, shell) {
                let level = match f.level {
                    safety::Level::High => Msg::RiskHigh,
                    safety::Level::Caution => Msg::RiskCaution,
                };
                eprintln!(
                    "{}: {}: {} ({})",
                    tr_in(lang, Msg::LabelWarning),
                    tr_in(lang, level),
                    tr_in(lang, f.category.msg()),
                    f.evidence
                );
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            report_error(&err);
            ExitCode::from(err.exit_code())
        }
    }
}

pub fn report_error(err: &LlmError) {
    let (msg, args) = error_message(err);
    let args: Vec<(&str, &str)> = args.iter().map(|(k, v)| (*k, v.as_str())).collect();
    eprintln!("{}", trf_in(crate::i18n::lang(), msg, &args));
}
