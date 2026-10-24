use super::Msg;

pub fn text(msg: Msg) -> &'static str {
    match msg {
        Msg::LabelQuestion => "Ask",
        Msg::LabelCommand => "Command",
        Msg::LabelExplain => "About",
        Msg::LabelWarning => "Warning",
        Msg::LabelNotes => "Notes",
        Msg::PlaceholderQuestion => "Describe what you want to do",
        Msg::PlaceholderFollowUp => "Type a change, or press Enter to accept the command",
        Msg::PlaceholderLoginPaste => "Paste the address here (step 2 only)",
        Msg::HintQuestion => "Enter submit  Tab command  Ctrl+O model  Esc quit",
        Msg::HintCommand => "Enter accept  Ctrl+J newline  Ctrl+E explain  Tab ask  Esc quit",
        Msg::HintFollowUp => "Enter accept or revise  ↓ edit command  Ctrl+E explain  Esc quit",
        Msg::HintRequesting => "Esc cancel",
        Msg::HintLogin => "Enter sign in  Esc quit",
        Msg::HintSettings => "Up/Down move  Enter choose  Esc close  (click works too)",
        Msg::SettingsTitle => "Settings",
        Msg::SettingsAgent => "Agent",
        Msg::SettingsModel => "Model",
        Msg::SettingsSignOut => "Sign out (switch account)",
        Msg::SettingsFolder => "Send current folder listing (Ctrl+F): {state}",
        Msg::FolderOn => "folder: on",
        Msg::FolderOff => "folder: off",
        Msg::StateOn => "on",
        Msg::StateOff => "off",
        Msg::StatusFolderOn => "Folder listing on: an ls -al style listing is sent (no contents).",
        Msg::StatusFolderOff => "Folder listing off.",
        Msg::UpdateNotice => "v{latest} available",
        Msg::UpdateTitle => "Update  howtui {current} -> {latest}",
        Msg::UpdateChecking => "Checking for a new version",
        Msg::UpdateUpToDate => "howtui {current} is up to date (latest release {latest}).",
        Msg::UpdateUnreachable => "Could not reach GitHub to check for a new version.",
        Msg::UpdateMethodLine => "Installed with: {method}",
        Msg::UpdateActionBuiltIn => "[Enter] Download, verify SHA-256, replace {path}, then quit",
        Msg::UpdateActionCommand => "[Enter] Quit and print the update command: {command}",
        Msg::HintUpdate => "Enter update  Esc close",
        Msg::HintUpdateClose => "Esc close",
        Msg::MethodScript => "install script ({dir})",
        Msg::MethodCargo => "cargo",
        Msg::MethodBrew => "Homebrew",
        Msg::MethodUnknown => "manual install",
        Msg::SettingsUpdate => "Check for updates (/update)",
        Msg::CheckUpdateAvailable => "howtui {current} -> {latest} available.",
        Msg::CheckUpdateHowTo => "Update: {hint}",
        Msg::CheckUpdateUnreachable => "howtui {current}: could not reach {url}.",
        Msg::UpdateDownloading => "Downloading {asset}",
        Msg::UpdateDone => "Updated howtui {current} -> {latest}. Run howtui again.",
        Msg::UpdateNotSelf => "This copy was installed with {method}. Update it with: {command}",
        Msg::UpdateFailed => "Update failed: {detail}. The installed version is unchanged.",
        Msg::HelpCheckUpdate => {
            "Check for a new version now (exit 0 up to date, 10 update available, 11 unreachable)"
        }
        Msg::HelpUpdate => {
            "Update to the newest release now (exit 0 done or up to date, 1 failed, 11 unreachable)"
        }
        Msg::SettingsLoading => "Loading models",
        Msg::SettingsApplying => "Applying",
        Msg::SettingsClose => "[Close]",
        Msg::StatusModelChanged => "Model: {model} (saved for next time)",
        Msg::StatusSignedOut => "Signed out.",
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
        Msg::LoginRequired => "Sign-in required. Press Enter to sign in with ChatGPT.",
        Msg::LoginWaiting => "Waiting for sign-in",
        Msg::LoginPrivacyShort => {
            "Requests send your question, the current command, OS, CPU and shell name to OpenAI (plus the folder listing only if you turn it on with Ctrl+F). No file contents, environment variables or history. howtui never runs commands."
        }
        Msg::LoginOpenUrl => {
            "1. Sign in in the browser window and press Allow. If no browser opened, open this URL:"
        }
        Msg::LoginPasteHint => {
            "2. Only if the browser then shows an error page (127.0.0.1 can't be reached): copy the whole address bar and paste it here."
        }
        Msg::LoginPressEnter => "Then press Enter.",
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
             you started from), the OS, CPU architecture and shell name to OpenAI. Only if you turn on the folder listing \
             (Ctrl+F) it also sends an ls -al style listing of the current folder (names including hidden ones, types and \
             permissions, sizes, modification times). It never sends environment variables, file contents, owner names, \
             shell history or the folder path, and it never runs commands."
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
        Msg::ErrModelNotFound => {
            "Model not found. Choose another model with Ctrl+O, or check `provider.<agent>.model` in the config."
        }
        Msg::ErrTransient => "Service temporarily unavailable. Try again shortly.",
        Msg::ErrNetwork => "Network error: {detail}. Check the connection and try again.",
        Msg::ErrTimeout => "The request timed out. Try again.",
        Msg::ErrNoApiKey => "No API key for {provider}. Run `howtui auth set-key {provider}`.",
        Msg::ErrNoApiKeyEnv => {
            "No API key for {provider}. Run `howtui auth set-key {provider}` or set {env}."
        }
        Msg::ErrKeyRejected => {
            "The {provider} API key was rejected. Replace it with `howtui auth set-key {provider}`."
        }
        Msg::ErrQuota => {
            "Account quota or billing limit reached at the provider. Check the provider's billing page."
        }
        Msg::ErrRefused => "The model declined this request. Rephrase it or choose another model.",
        Msg::AgentSignedIn => "signed in",
        Msg::AgentNeedsLogin => "sign-in needed",
        Msg::AgentKeyEnv => "key from {env}",
        Msg::AgentKeyStore => "key saved with set-key",
        Msg::AgentNoKey => "no key: howtui auth set-key {id}",
        Msg::AgentNoUrl => "set provider.openai_compatible.base_url",
        Msg::StatusAgentChanged => "Agent: {agent} (saved for next time)",
        Msg::KeyPrompt => "Paste the {provider} API key and press Enter (input is hidden):",
        Msg::KeySaved => "{provider} API key saved ({store}).",
        Msg::KeyRemoved => "{provider} API key removed.",
        Msg::KeyNotFound => "No saved {provider} API key.",
        Msg::KeyEmpty => "No key entered. Nothing was saved.",
        Msg::KeyEnvShadow => "Note: {env} is set and is used before the saved key.",
        Msg::AuthStatusProvider => "Agent: {agent}",
        Msg::AuthStatusKeys => "API keys:",
        Msg::KeySourceEnv => "environment variable {env}",
        Msg::KeySourceStore => "saved with howtui auth set-key",
        Msg::KeySourceNone => "not set",
        Msg::HelpProvider => {
            "Agent for this run (chatgpt, openai, anthropic, gemini, openai_compatible)"
        }
        Msg::HelpSetKey => "Save an API key in the OS credential store",
        Msg::HelpRemoveKey => "Delete a saved API key",
        Msg::ErrInvalidResponse => "Could not read the model response. Try again.",
        Msg::ErrNoTerminal => {
            "No usable terminal (TTY). For non-interactive use run: howtui --no-tui \"<question>\""
        }
        Msg::ErrConfig => "Configuration error: {detail}",
        Msg::ErrInternal => "Internal error: {detail}",
        Msg::ErrNoCommand => "The model answered without a command. Ask for a shell command.",
        Msg::ErrNoQuestion => "A question is required with --no-tui.",
        Msg::ErrKeyFormat => {
            "Invalid key \"{key}\". Use ctrl+<letter> or alt+<letter>, for example ctrl+g."
        }
        Msg::ErrKeyReserved => {
            "{key} is needed by the shell (Enter, Tab, Backspace, Ctrl+C, Ctrl+D, Ctrl+Z, Ctrl+S, Ctrl+Q). Choose another key."
        }
        Msg::ErrWidgetNoOutput => {
            "--widget is used by shell integration scripts (HOWTUI_OUTPUT_FILE is not set). See `howtui init --help`."
        }
        Msg::ErrWidgetWrite => {
            "Could not hand the command to the shell ({detail}). Copy it manually:"
        }
        Msg::ErrInstallUnsupported => {
            "Could not find your home directory for the shell startup file."
        }
        Msg::ErrRcWrite => "Could not update {path}: {detail}",
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
        Msg::HelpInit => {
            "Print the shell integration script (key binding and next-prompt insertion). Add `eval \"$(howtui init bash)\"` to ~/.bashrc, `eval \"$(howtui init zsh)\"` to ~/.zshrc, or `howtui init powershell | Out-String | Invoke-Expression` to $PROFILE"
        }
        Msg::HelpInitShell => "Shell to integrate with",
        Msg::HelpInitKey => {
            "Key that opens howtui with the current command line (ctrl+<letter> or alt+<letter>)"
        }
        Msg::HelpNoBind => "Do not bind a key",
        Msg::HelpNoWrap => {
            "Do not define the howtui shell function (results then go to stdout only)"
        }
        Msg::HelpInstall => {
            "Add the integration line to your shell startup file (~/.bashrc, ~/.zshrc or the PowerShell profile) in a marked block"
        }
        Msg::HelpUninstall => "Remove the block added by --install",
        Msg::InitAdded => "Shell integration added to {path}. Open a new terminal to use it.",
        Msg::InitUpdated => "Shell integration in {path} updated. Open a new terminal to use it.",
        Msg::InitUnchanged => "Shell integration is already in {path}.",
        Msg::InitRemoved => "Shell integration removed from {path}. Open a new terminal.",
        Msg::InitNotPresent => "No howtui shell integration block in {path}.",
        Msg::HelpNoBrowser => "Do not open a browser; print the URL (for SSH or headless machines)",
        Msg::HelpPort => "Fixed local callback port (for SSH port forwarding)",
        Msg::HelpForget => "Also forget the account registration",
        Msg::HelpHelp => "Print help",
        Msg::HelpVersion => "Print version",
        Msg::HelpAfter => {
            "Exit codes: 0 accepted, 2 usage, 3 no terminal, 4 auth, 5 plan/usage, 6 network, 7 provider, 8 config, 130 cancelled.\n\
             Sent to the model: your question, the current command, OS, CPU architecture and shell name; an ls -al style listing of the current folder (no contents) only when turned on (Ctrl+F, context.folder).\n\
             Danger warnings are rule-based hints, not a security guarantee. howtui never runs commands."
        }
    }
}
