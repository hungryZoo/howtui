//! 다국어 메시지 카탈로그 (ADR-0016, SRS 7.5절).
//!
//! 언어별 `text` 함수는 `Msg` 전체를 와일드카드 없이 열거한다. 번역 누락은 컴파일 오류가 된다.

mod detect;
mod en;
mod ko;

use std::sync::OnceLock;

pub use detect::{LangSetting, detect_system_lang, resolve_lang};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    En,
    Ko,
}

impl Lang {
    pub const ALL: &'static [Lang] = &[Lang::En, Lang::Ko];

    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Ko => "ko",
        }
    }

    pub fn parse(value: &str) -> Option<Lang> {
        let v = value.trim().to_ascii_lowercase();
        if v.starts_with("ko") {
            Some(Lang::Ko)
        } else if v.starts_with("en") {
            Some(Lang::En)
        } else {
            None
        }
    }
}

macro_rules! messages {
    ($($name:ident),* $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum Msg { $($name),* }
        impl Msg {
            pub const ALL: &'static [Msg] = &[$(Msg::$name),*];
        }
    };
}

messages! {
    // 화면 레이블
    LabelQuestion,
    LabelCommand,
    LabelExplain,
    LabelWarning,
    LabelNotes,
    PlaceholderQuestion,
    PlaceholderFollowUp,
    PlaceholderLoginPaste,
    // 상태줄
    HintQuestion,
    HintCommand,
    HintFollowUp,
    HintRequesting,
    HintLogin,
    StatusGenerating,
    StatusRevising,
    StatusExplaining,
    StatusCancelled,
    StatusExplanationStale,
    StatusConfirmDanger,
    StatusTooSmall,
    StatusFullscreenUnavailable,
    UsingChatGptPlan,
    PlanNotice,
    // 로그인
    LoginRequired,
    LoginWaiting,
    LoginPrivacyShort,
    LoginOpenUrl,
    LoginPasteHint,
    LoginPressEnter,
    LoginSshHint,
    LoginBrowserFailed,
    LoginSuccess,
    LoginPlanDisabled,
    LoginCancelled,
    LoginInvalidPaste,
    LoginStateMismatch,
    LoginDenied,
    LoginTimeout,
    PrivacyNotice,
    LogoutDone,
    LogoutForgetDone,
    LogoutRevokeUnconfirmed,
    LogoutNotSignedIn,
    AuthStatusSignedIn,
    AuthStatusSignedOut,
    AuthStatusPlan,
    AuthStatusPlanDisabled,
    AuthStatusExpires,
    AuthStatusStore,
    StoreFileFallback,
    // 오류 (원인 + 조치)
    ErrAuthRequired,
    ErrAuthFailed,
    ErrNotEligible,
    ErrClientNotEnabled,
    ErrUsageLimit,
    ErrRegionOrPolicy,
    ErrPermissionConfig,
    ErrUnsupported,
    ErrModelNotFound,
    ErrTransient,
    ErrNetwork,
    ErrTimeout,
    ErrInvalidResponse,
    ErrNoTerminal,
    ErrConfig,
    ErrInternal,
    ErrNoQuestion,
    ErrNoCommand,
    ErrKeyFormat,
    ErrKeyReserved,
    ErrWidgetNoOutput,
    ErrWidgetWrite,
    ErrInstallUnsupported,
    ErrRcWrite,
    // 위험 분류
    RiskHigh,
    RiskCaution,
    CatRecursiveDelete,
    CatDelete,
    CatDisk,
    CatPermission,
    CatRemoteScript,
    CatMassKill,
    CatPower,
    CatPrivilege,
    CatOverwrite,
    CatGitHistory,
    CatSystemConfig,
    CatForkBomb,
    // doctor
    DoctorTitle,
    DoctorPass,
    DoctorWarn,
    DoctorFail,
    DoctorInfo,
    DoctorSkip,
    DoctorSummaryOk,
    DoctorSummaryFail,
    // 도움말
    HelpAbout,
    HelpQuestion,
    HelpNoTui,
    HelpShell,
    HelpHeight,
    HelpMode,
    HelpLogin,
    HelpLogout,
    HelpAuth,
    HelpAuthStatus,
    HelpDoctor,
    HelpDoctorJson,
    HelpInit,
    HelpInitShell,
    HelpInitKey,
    HelpNoBind,
    HelpNoWrap,
    HelpInstall,
    HelpUninstall,
    InitAdded,
    InitUpdated,
    InitUnchanged,
    InitRemoved,
    InitNotPresent,
    HelpNoBrowser,
    HelpPort,
    HelpForget,
    HelpHelp,
    HelpVersion,
    HelpAfter,
}

static CURRENT: OnceLock<Lang> = OnceLock::new();

/// 프로세스 언어 설정 (최초 1회). 이후 호출은 무시한다.
pub fn set_lang(lang: Lang) {
    let _ = CURRENT.set(lang);
}

pub fn lang() -> Lang {
    *CURRENT.get().unwrap_or(&Lang::En)
}

pub fn tr_in(lang: Lang, msg: Msg) -> &'static str {
    match lang {
        Lang::En => en::text(msg),
        Lang::Ko => ko::text(msg),
    }
}

pub fn tr(msg: Msg) -> &'static str {
    tr_in(lang(), msg)
}

/// `{name}` 자리를 채운 메시지.
pub fn trf_in(lang: Lang, msg: Msg, args: &[(&str, &str)]) -> String {
    let mut out = tr_in(lang, msg).to_string();
    for (name, value) in args {
        out = out.replace(&format!("{{{name}}}"), value);
    }
    out
}

pub fn trf(msg: Msg, args: &[(&str, &str)]) -> String {
    trf_in(lang(), msg, args)
}

/// 메시지 문자열의 `{name}` 자리 이름 집합.
pub fn placeholders(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find('{') {
        let after = &rest[start + 1..];
        match after.find('}') {
            Some(end) => {
                let name = &after[..end];
                if !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                    names.push(name.to_string());
                }
                rest = &after[end + 1..];
            }
            None => break,
        }
    }
    names.sort();
    names.dedup();
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_message_is_translated() {
        for &lang in Lang::ALL {
            for &msg in Msg::ALL {
                assert!(
                    !tr_in(lang, msg).trim().is_empty(),
                    "{lang:?} {msg:?} 비어 있음"
                );
            }
        }
    }

    #[test]
    fn placeholders_match_across_languages() {
        for &msg in Msg::ALL {
            let base = placeholders(tr_in(Lang::En, msg));
            for &lang in Lang::ALL {
                assert_eq!(
                    placeholders(tr_in(lang, msg)),
                    base,
                    "{lang:?} {msg:?} 자리 이름 불일치"
                );
            }
        }
    }

    #[test]
    fn fill_replaces_named_placeholders() {
        let s = trf_in(Lang::En, Msg::StatusTooSmall, &[("w", "30"), ("h", "3")]);
        assert!(s.contains("30x3"), "{s}");
    }

    #[test]
    fn parse_lang_codes() {
        assert_eq!(Lang::parse("ko_KR.UTF-8"), Some(Lang::Ko));
        assert_eq!(Lang::parse("EN"), Some(Lang::En));
        assert_eq!(Lang::parse("fr"), None);
    }
}
