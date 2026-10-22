use super::Msg;

pub fn text(msg: Msg) -> &'static str {
    match msg {
        Msg::LabelQuestion => "Ask",
        Msg::LabelCommand => "Command",
        Msg::LabelExplain => "About",
        Msg::LabelWarning => "Warning",
        Msg::LabelNotes => "Notes",
        Msg::PlaceholderQuestion => "Describe what you want to do",
        Msg::PlaceholderFollowUp => "Ask for a change, or accept the command with Enter",
        Msg::PlaceholderLoginPaste => "Paste the redirected URL here if the browser runs elsewhere",
        Msg::HintQuestion => "Enter submit  Tab command  Esc quit",
        Msg::HintCommand => "Enter accept  Ctrl+J newline  Ctrl+E explain  Tab ask  Esc quit",
        Msg::HintRequesting => "Esc cancel",
        Msg::HintLogin => "Enter sign in  Esc quit",
        Msg::StatusGenerating => "Generating",
        Msg::StatusRevising => "Revising",
        Msg::StatusExplaining => "Explaining",
        Msg::StatusCancelled => "Request cancelled",
        Msg::StatusExplanationStale => "Command edited. Press Ctrl+E to refresh the explanation.",
        Msg::StatusConfirmDanger => {
            "High-risk command. Press Enter again to accept, any other key to keep editing."
        }
        Msg::StatusTooSmall => "Terminal too small (minimum {w}x{h})",
        Msg::StatusFullscreenUnavailable => {
            "Fullscreen mode is not available yet. Using inline mode."
        }
        Msg::UsingChatGptPlan => "Using ChatGPT plan",
        Msg::PlanNotice => {
            "You're using your ChatGPT plan. Manage usage: https://chatgpt.com/settings/usage"
        }
        Msg::LoginRequired => "Sign-in required. Press Enter to Continue with ChatGPT.",
        Msg::LoginWaiting => "Waiting for sign-in in the browser",
        Msg::LoginOpenUrl => "Open this URL to sign in:",
        Msg::LoginPasteHint => {
            "If the browser runs on another machine, paste the URL it was redirected to (starts with http://127.0.0.1) and press Enter."
        }
        Msg::LoginSshHint => {
            "On a remote machine you can also forward the port first: ssh -L {port}:127.0.0.1:{port} <host>"
        }
        Msg::LoginBrowserFailed => "Could not open a browser. Open the URL manually.",
        Msg::LoginSuccess => "Signed in as {email}.",
        Msg::LoginPlanDisabled => {
            "Signed in, but ChatGPT plan usage was not granted. Run `howtui login` again and allow \"Use your ChatGPT plan\"."
        }
        Msg::LoginCancelled => "Sign-in cancelled.",
        Msg::LoginInvalidPaste => "That is not a valid sign-in redirect URL for this attempt.",
        Msg::LoginStateMismatch => {
            "Sign-in response did not match this attempt (state mismatch). Start again."
        }
        Msg::LoginDenied => "Sign-in was declined in the browser.",
        Msg::LoginTimeout => "Sign-in timed out. Start again.",
        Msg::PrivacyNotice => {
            "howtui signs in with your ChatGPT account (Sign in with ChatGPT) and stores the issued tokens on this device \
             (OS credential store, or a file readable only by you). It receives your account email and name from OpenAI. \
             When you submit a request it sends your question, the current command (including your edits or the shell line \
             you started from), the OS, CPU architecture and shell name to OpenAI. It never sends environment variables, \
             files, shell history or the working directory, and it never runs commands."
        }
        Msg::LogoutDone => "Signed out.",
        Msg::LogoutForgetDone => "Signed out and removed the account registration.",
        Msg::LogoutRevokeUnconfirmed => {
            "Signed out locally, but OpenAI did not confirm token revocation. You can disconnect howtui in ChatGPT Settings > Security and login."
        }
        Msg::LogoutNotSignedIn => "Not signed in.",
        Msg::AuthStatusSignedIn => "Signed in: {email}",
        Msg::AuthStatusSignedOut => "Not signed in. Run `howtui login`.",
        Msg::AuthStatusPlan => "ChatGPT plan usage: enabled",
        Msg::AuthStatusPlanDisabled => "ChatGPT plan usage: not granted",
        Msg::AuthStatusExpires => "Access token expires: {time}",
        Msg::AuthStatusStore => "Credential storage: {store}",
        Msg::StoreFileFallback => {
            "OS credential store unavailable. Tokens are stored in a file readable only by you: {path}"
        }
        Msg::ErrAuthRequired => "Sign-in required. Run `howtui login` (or press Enter in the TUI).",
        Msg::ErrAuthFailed => "Authentication failed. Sign in again with `howtui login`.",
        Msg::ErrNotEligible => {
            "ChatGPT plan usage is not available for this account or workspace. Check your plan and workspace."
        }
        Msg::ErrClientNotEnabled => {
            "ChatGPT plan usage is not enabled for howtui yet. howtui cannot be used until it is enabled."
        }
        Msg::ErrUsageLimit => {
            "ChatGPT plan usage limit reached. Manage usage: https://chatgpt.com/settings/usage"
        }
        Msg::ErrRegionOrPolicy => "Request blocked by policy or region: {detail}",
        Msg::ErrPermissionConfig => "Sign-in permissions are inconsistent ({code}). Sign in again.",
        Msg::ErrUnsupported => {
            "The model or request feature is not supported ({param}). Check the model setting."
        }
        Msg::ErrModelNotFound => "Model not found. Check `provider.chatgpt.model` or remove it.",
        Msg::ErrTransient => "Service temporarily unavailable. Try again shortly.",
        Msg::ErrNetwork => "Network error: {detail}. Check the connection and try again.",
        Msg::ErrTimeout => "The request timed out. Try again.",
        Msg::ErrInvalidResponse => "Could not read the model response. Try again.",
        Msg::ErrNoTerminal => {
            "No usable terminal (TTY). For non-interactive use run: howtui --no-tui \"<question>\""
        }
        Msg::ErrConfig => "Configuration error: {detail}",
        Msg::ErrInternal => "Internal error: {detail}",
        Msg::ErrNoCommand => "The model answered without a command. Ask for a shell command.",
        Msg::ErrNoQuestion => "A question is required with --no-tui.",
        Msg::RiskHigh => "High risk",
        Msg::RiskCaution => "Caution",
        Msg::CatRecursiveDelete => "recursive delete",
        Msg::CatDelete => "deletes files",
        Msg::CatDisk => "writes to a disk or formats it",
        Msg::CatPermission => "changes permissions recursively",
        Msg::CatRemoteScript => "runs a downloaded script",
        Msg::CatMassKill => "kills many processes",
        Msg::CatPower => "shuts down, reboots or stops services",
        Msg::CatPrivilege => "runs with elevated privileges",
        Msg::CatOverwrite => "may overwrite files",
        Msg::CatGitHistory => "may discard git history or changes",
        Msg::CatSystemConfig => "changes system configuration",
        Msg::CatForkBomb => "fork bomb",
        Msg::DoctorTitle => "howtui doctor",
        Msg::DoctorPass => "ok",
        Msg::DoctorWarn => "warn",
        Msg::DoctorFail => "FAIL",
        Msg::DoctorInfo => "info",
        Msg::DoctorSkip => "skip",
        Msg::DoctorSummaryOk => "No failed checks.",
        Msg::DoctorSummaryFail => "{count} check(s) failed.",
        Msg::HelpAbout => {
            "AI terminal command assistant: describe a task, edit the suggested command, and get it on stdout"
        }
        Msg::HelpQuestion => "Question to ask right away",
        Msg::HelpNoTui => {
            "Generate once without the TUI (command on stdout, explanation on stderr)"
        }
        Msg::HelpShell => "Target shell (default: shell integration hint, then the OS default)",
        Msg::HelpHeight => "Inline UI height in lines (3-40)",
        Msg::HelpMode => "UI mode",
        Msg::HelpLogin => "Sign in with ChatGPT",
        Msg::HelpLogout => "Sign out and revoke the session",
        Msg::HelpAuth => "Authentication commands",
        Msg::HelpAuthStatus => "Show sign-in status",
        Msg::HelpDoctor => "Check the environment and configuration",
        Msg::HelpDoctorJson => "Machine-readable JSON output",
        Msg::HelpNoBrowser => "Do not open a browser; print the URL (for SSH or headless machines)",
        Msg::HelpPort => "Fixed local callback port (for SSH port forwarding)",
        Msg::HelpForget => "Also forget the account registration",
        Msg::HelpHelp => "Print help",
        Msg::HelpVersion => "Print version",
        Msg::HelpAfter => {
            "Exit codes: 0 accepted, 2 usage, 3 no terminal, 4 auth, 5 plan/usage, 6 network, 7 provider, 8 config, 130 cancelled.\n\
             Sent to the model: your question, the current command, OS, CPU architecture and shell name.\n\
             Danger warnings are rule-based hints, not a security guarantee. howtui never runs commands."
        }
    }
}
