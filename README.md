<div align="center">

# `howtui`

**An AI command assistant that lives inside your terminal.**<br/>
Describe a task, edit the suggested shell command in a small inline UI, and get it on stdout. It never runs anything for you.

[![License: MIT](https://img.shields.io/badge/license-MIT-89b4fa?style=flat-square&labelColor=1e1e2e)](LICENSE)
[![Rust · ratatui](https://img.shields.io/badge/built%20with-Rust%20%C2%B7%20ratatui-fab387?style=flat-square&logo=rust&logoColor=white&labelColor=1e1e2e)](https://ratatui.rs)
[![CI](https://img.shields.io/github/actions/workflow/status/hungryZoo/howtui/ci.yml?branch=main&style=flat-square&label=ci&labelColor=1e1e2e)](https://github.com/hungryZoo/howtui/actions/workflows/ci.yml)

```bash
curl -fsSL https://raw.githubusercontent.com/hungryZoo/howtui/main/scripts/install.sh | sh
```

[![Linux](https://img.shields.io/badge/Linux-x86__64%20%C2%B7%20ARM64%20%C2%B7%20ARMv7-a6e3a1?style=flat-square&logo=linux&logoColor=white&labelColor=1e1e2e)](#6-platform-support)
[![macOS](https://img.shields.io/badge/macOS-Apple%20Silicon%20%C2%B7%20Intel-f5c2e7?style=flat-square&logo=apple&logoColor=white&labelColor=1e1e2e)](#6-platform-support)
[![Windows](https://img.shields.io/badge/Windows-x64%20%C2%B7%20ARM64-89dceb?style=flat-square&logo=windows&logoColor=white&labelColor=1e1e2e)](#6-platform-support)

</div>

> **Status: early release** ([latest](https://github.com/hungryZoo/howtui/releases/latest)). Inference uses **Sign in with
> ChatGPT**, which OpenAI currently offers as a limited preview.

---

Run `howtui` and a few lines open under your prompt; nothing else on the screen moves. Type what you want in plain language,
for example *"find files over 100 MB here"*. A command and a one-line explanation appear. Edit the command directly, or type a
follow-up such as *"also sort by size"* and the model revises **your edited version**. Press Enter and the inline UI disappears,
leaving only the command on stdout. With the optional [shell integration](#shell-integration) the command lands in your
next prompt line instead, ready to edit or run. Running it is your decision.

```text
$ howtui
Ask     > Type a change, or press Enter to accept the command (find files over 100 MB here)
Command > find . -type f -size +100M -exec ls -lh {} +
About   > Lists regular files larger than 100 MB below the current directory with their sizes.
Enter accept or revise  ↓ edit command  Ctrl+E explain  Esc quit             gpt-model  bash  Using ChatGPT plan
```

---

## Table of Contents

1. [Overview](#1-overview)
   - [Features](#features)
   - [How it works](#how-it-works)
2. [Installation](#2-installation)
   - [Linux and macOS](#linux-and-macos)
   - [Windows](#windows)
   - [Review the script first](#review-the-script-first)
   - [Manual download and checksum](#manual-download-and-checksum)
   - [From source](#from-source)
   - [Update](#update)
   - [Uninstall](#uninstall)
3. [Quickstart](#3-quickstart)
4. [Usage](#4-usage)
   - [Commands](#commands)
   - [Keys](#keys)
   - [Settings: agent and model](#settings-agent-and-model)
   - [Agents and API keys](#agents-and-api-keys)
   - [Local models](#local-models)
   - [Folder listing](#folder-listing)
   - [Shell integration](#shell-integration)
   - [Non-interactive mode](#non-interactive-mode)
   - [Exit codes](#exit-codes)
5. [Configuration](#5-configuration)
   - [Config file](#config-file)
   - [Environment variables](#environment-variables)
6. [Platform support](#6-platform-support)
7. [Privacy and safety](#7-privacy-and-safety)
8. [Development](#8-development)
9. [Roadmap](#9-roadmap)
10. [License](#10-license)

---

## 1. Overview

### Features

- **Inline UI.** Uses only a few lines below your prompt (10 by default) and never switches to the alternate screen.
- **Edit, then ask.** Edit the command in place (multi-line, undo and redo, Unicode aware), then ask a follow-up; the model sees your edits.
- **Explain on demand.** After editing by hand, press Ctrl+E to explain the current command.
- **Folder-aware on request.** Turn on the folder listing (Ctrl+F or the `[folder: off]` button) and *"extract the archive"* picks the archive that is actually in the current folder. It sends an `ls -al` style listing (no file contents, owners or paths); it is off until you turn it on.
- **Pick the agent and model.** Use your ChatGPT plan, your own API key for OpenAI, Anthropic or Gemini, a local model through Ollama or llama.cpp `llama-server`, or any OpenAI-compatible server. Ctrl+O, `/model` or a click on the model name in the status line opens the settings view to switch agents and models; the choice is kept for next time.
- **Clean stdout.** The UI draws on the controlling terminal, so `cmd=$(howtui)` and `howtui | pbcopy` receive only the command.
- **Shell integration (optional).** Ctrl+G opens howtui on the current command line in Bash, Zsh or PowerShell, and typing `howtui` puts the result into the next prompt. The installer asks before adding it to your rc file; nothing is changed without a yes.
- **Never executes.** howtui prints or hands over the command; it has no code path that runs generated commands.
- **Danger hints.** Rule-based warnings for recursive deletes, disk writes, permission changes, piping downloads into a shell and more. High-risk commands need a second Enter.
- **English and Korean UI.** Chosen from `HOWTUI_LANG`, `ui.language` or the system locale. Explanations follow the language of your question.
- **Single binary.** Seven release targets, including Raspberry Pi (ARMv7 and ARM64) and Windows on ARM.

### How it works

| Step | What happens |
| --- | --- |
| Sign in | The first run (or `howtui login`) starts the official Sign in with ChatGPT flow (OAuth with PKCE, `127.0.0.1` callback). Tokens go to the OS credential store, or to a file readable only by you. |
| Ask | Your question, the current command and your OS, CPU architecture and shell name are sent to the OpenAI Responses API with your ChatGPT plan. |
| Edit | The command editor is local. Follow-up requests include the edited command. |
| Accept | The inline UI is cleared, the terminal is restored, and the command plus one newline is written to stdout (or, with shell integration, handed to the prompt line). |

<p align="right"><a href="#table-of-contents">↑ back to top</a></p>

---

## 2. Installation

### Linux and macOS

```bash
curl -fsSL https://raw.githubusercontent.com/hungryZoo/howtui/main/scripts/install.sh | sh
```

The script detects the OS and CPU (including a 32-bit Raspberry Pi OS on a 64-bit kernel), downloads the release archive and
`SHA256SUMS`, verifies the checksum, and installs `howtui` and `howtui-uninstall.sh` to `~/.local/bin` without sudo.

At the end it asks one question:

```text
Shell integration for bash: Ctrl+G opens howtui on the current command line, and a
typed `howtui` puts the accepted command into your next prompt. Nothing runs by itself.
This adds a marked block to ~/.bashrc.
Remove it later with: howtui init bash --uninstall
Enable shell integration? [Y/n]
```

Press Enter (yes) and open a new terminal. Only a yes adds the marked block to `~/.bashrc` or `~/.zshrc` (with a `PATH`
line when `~/.local/bin` is not on your `PATH` yet). Without a terminal the script asks nothing and changes nothing.

| Option | Environment variable | Meaning |
| --- | --- | --- |
| `--version X.Y.Z` | `HOWTUI_VERSION` | Install a specific release |
| `--bin-dir DIR` | `HOWTUI_INSTALL_DIR` | Install directory |
| `--print-target` | | Print the detected release target and exit |
| `--shell-integration`, `--no-shell-integration` | `HOWTUI_SETUP_SHELL=yes\|no` | Enable shell integration without asking, or skip the question |

### Windows

Windows PowerShell 5.1 or PowerShell 7:

```powershell
irm https://raw.githubusercontent.com/hungryZoo/howtui/main/scripts/install.ps1 | iex
```

Installs `howtui.exe` and `howtui-uninstall.ps1` to `%LOCALAPPDATA%\Programs\howtui\bin`, verifies the checksum, and
adds that directory to your user `PATH`. At the end it asks the same shell integration question; a yes adds a marked
block to your PowerShell 7 and Windows PowerShell 5.1 profiles (`Documents\PowerShell` and `Documents\WindowsPowerShell`).
If the execution policy blocks profiles (the Windows PowerShell 5.1 default `Restricted`), it shows the command to allow
them for your account (`Set-ExecutionPolicy -Scope CurrentUser RemoteSigned`) and does not change the policy itself.

| Environment variable | Meaning |
| --- | --- |
| `HOWTUI_VERSION` | Install a specific release |
| `HOWTUI_INSTALL_DIR` | Install directory |
| `HOWTUI_NO_MODIFY_PATH=1` | Do not touch the user `PATH` |
| `HOWTUI_ARCH` | Override the detected architecture (`x86_64` or `aarch64`) |
| `HOWTUI_SETUP_SHELL=yes\|no` | Enable shell integration without asking, or skip the question |

### Review the script first

```bash
curl -fsSLO https://raw.githubusercontent.com/hungryZoo/howtui/main/scripts/install.sh
less install.sh
sh install.sh
```

Every release also ships its own `install.sh` and `install.ps1`, and you can pin the script to a tag:
`https://raw.githubusercontent.com/hungryZoo/howtui/v<version>/scripts/install.sh`.

### Manual download and checksum

Download `howtui-<version>-<target>.tar.gz` (or `.zip` on Windows) and `SHA256SUMS` from the
[releases page](https://github.com/hungryZoo/howtui/releases), then:

```bash
sha256sum -c SHA256SUMS --ignore-missing      # macOS: shasum -a 256 -c SHA256SUMS --ignore-missing
tar -xzf howtui-<version>-<target>.tar.gz howtui && mv howtui ~/.local/bin/
```

```powershell
(Get-FileHash .\howtui-<version>-x86_64-pc-windows-msvc.zip -Algorithm SHA256).Hash   # compare with SHA256SUMS
```

Releases carry build provenance attestations: `gh attestation verify <archive> -R hungryZoo/howtui`.
Binaries downloaded with a browser on macOS may need `xattr -d com.apple.quarantine ./howtui`.

### From source

```bash
git clone https://github.com/hungryZoo/howtui && cd howtui
cargo build --release          # Rust 1.89 or newer; rust-toolchain.toml pins the tested version
```

### Update

When a new release exists, the status line shows `[v0.2.0 available]` (from 0.1.2 on). Click it, type `/update`, or pick
**Check for updates** in Settings (Ctrl+O), then press Enter. howtui quits, downloads the release for your platform,
verifies it against `SHA256SUMS`, replaces itself and tells you to run it again. From a script or a terminal:

| Command | Does | Exit code |
| --- | --- | --- |
| `howtui --check-update` | Check now and print the result | 0 up to date, 10 update available, 11 GitHub unreachable |
| `howtui --update` | Check now and update if there is a new release | 0 updated or up to date, 1 failed, 11 GitHub unreachable |

How it checks: at most once a day, in the background, a single request to
`https://github.com/hungryZoo/howtui/releases/latest` (nothing about you or your use is sent). Start-up never waits for
it; the result shows on the next start. Turn it off with `HOWTUI_NO_UPDATE_CHECK=1` or `update.check = false`.
The last result is kept here (set `checked_at=0` in the file to check again on the next start):

| OS | Cache file |
| --- | --- |
| Linux | `${XDG_CACHE_HOME:-~/.cache}/howtui/update-check` |
| macOS | `~/Library/Caches/howtui/update-check` |
| Windows | `%LOCALAPPDATA%\howtui\cache\update-check` |

How it updates depends on how howtui was installed:

| Installed with | Detected by | Update |
| --- | --- | --- |
| `install.sh`, `install.ps1`, or a release archive copied to your home | path under your home or `%LOCALAPPDATA%` | built in: download, SHA-256 check, replace in place (same folder, uninstall script too) |
| `cargo install` | `~/.cargo/bin` (or `$CARGO_HOME/bin`) | prints `cargo install --git https://github.com/hungryZoo/howtui --locked` |
| Homebrew | `/Cellar/`, `/homebrew/` or `/linuxbrew/` in the path | prints `brew upgrade howtui` |
| anything else | | prints the release page URL |

Running the install command again also updates in place; pass the same `--bin-dir` or `HOWTUI_INSTALL_DIR` if you
changed it. Your sign-in, model choice, settings and shell integration stay as they are. Versions 0.1.0 and 0.1.1 have
no update check, so update them once with the install command.

### Uninstall

The installer puts an uninstall script next to the binary. It lists what it will remove and asks before doing anything.

```bash
~/.local/bin/howtui-uninstall.sh            # Linux and macOS
~/.local/bin/howtui-uninstall.sh --purge    # also delete ~/.config/howtui
```

```powershell
& "$env:LOCALAPPDATA\Programs\howtui\bin\howtui-uninstall.ps1"           # Windows
& "$env:LOCALAPPDATA\Programs\howtui\bin\howtui-uninstall.ps1" -Purge    # also delete %APPDATA%\howtui
```

| Removed | Kept |
| --- | --- |
| Saved sign-in (signs out, deletes the tokens and the app registration) | Config file, unless `--purge` / `-Purge` |
| Shell integration blocks the installer or `--install` added (only the marked blocks) | Anything else in your rc files and profiles |
| `howtui`, the uninstall script, the install folder entry in the Windows user `PATH` | Files you put into the data folder yourself |
| Data folder: `~/.local/share/howtui` or `%LOCALAPPDATA%\howtui` | |

Add `-y` (`-Yes` on Windows) to skip the question. Open a new terminal afterwards so the current shell forgets the
integration. Manual removal, for installs without the script (for example `cargo install`):

```bash
howtui logout --forget
howtui init bash --uninstall        # or zsh, powershell
cargo uninstall howtui              # or delete the howtui binary you copied
rm -rf ~/.local/share/howtui ~/.config/howtui
```

<p align="right"><a href="#table-of-contents">↑ back to top</a></p>

---

## 3. Quickstart

```bash
howtui                  # first run signs you in, then: ask, edit, Enter
howtui find large log files    # ask right away
howtui init bash --install    # optional (the installer offers this): Ctrl+G and next-prompt insertion
```

Sign-in starts by itself the first time (or run `howtui login`):

1. A browser window opens. Sign in and press **Allow**.
2. Only if the browser then shows an error page (`127.0.0.1` can't be reached, for example because howtui runs on another
   machine over SSH): copy the whole address bar and paste it into the terminal. No Enter needed in the UI.

`howtui login --no-browser` only prints the URL. Alternatively forward the callback port first:
`ssh -L <port>:127.0.0.1:<port> <host>` with a fixed `--port`.

<p align="right"><a href="#table-of-contents">↑ back to top</a></p>

---

## 4. Usage

### Commands

| Command | Description |
| --- | --- |
| `howtui [QUESTION]...` | Open the inline UI; a question argument is sent immediately |
| `howtui --no-tui QUESTION...` | Generate once without the UI |
| `howtui login [--no-browser] [--port N]` | Sign in with ChatGPT |
| `howtui logout [--forget]` | Revoke the session and delete local tokens (`--forget` also drops the account registration) |
| `howtui auth status` | Show the agent in use, where each API key comes from, and the ChatGPT account (masked email), plan usage and token expiry |
| `howtui auth set-key AGENT`, `howtui auth remove-key AGENT` | Save or delete an API key for `openai`, `anthropic`, `gemini` or `openai_compatible` ([Agents and API keys](#agents-and-api-keys)) |
| `howtui doctor [--json]` | Check terminal, locale, config, sign-in and model access; output contains no tokens or home paths |
| `howtui init bash\|zsh\|powershell` | Print the shell integration script (options: `--key ctrl+x`, `--no-bind`, `--no-wrap`) |
| `howtui --check-update`, `howtui --update` | Check for a new release, or update now ([Update](#update)) |

Common options: `--shell bash|zsh|powershell`, `--height N` (3-40), `--provider AGENT`.

### Keys

| Key | Ask line | Command editor |
| --- | --- | --- |
| Enter | Send (first request generates, later ones revise); on an empty ask line, accept the command | Accept (high-risk commands ask for a second Enter) |
| Ctrl+J, Alt+Enter | | New line (Shift+Enter where the terminal reports it) |
| Tab, Shift+Tab | Switch to the command | Switch to the ask line |
| Ctrl+E | | Explain the current command |
| Ctrl+O | Settings (agent and model) | Settings |
| Ctrl+F | Folder listing on or off | Same |
| Esc | Quit | Quit (cancels a running request first) |
| Ctrl+C | Quit | Quit |
| Ctrl+Z, Ctrl+Y | Undo, redo (on an empty ask line: the command) | Undo, redo (an AI revision is one undo step) |
| ↓ | Go to the command editor | Next line |
| ↑ | | Previous line; on the first line, go to the ask line |
| ← →, Home, End, Ctrl+A | Move | Move |
| Ctrl+← →, Alt+B, Alt+F | Word left, right | Word left, right |
| Backspace, Delete, Ctrl+W, Ctrl+U, Ctrl+K | Delete char, word, to start, to end | Same |

Pasting multi-line text into the editor keeps the lines and never triggers Accept. After each answer the cursor stays on
the ask line, so you can type the next change right away or press Enter to accept.

### Settings: agent and model

Press **Ctrl+O**, type `/model` in the ask line, or click the `[model]` name in the status line.

```text
Settings  Agent: ChatGPT (h***@gmail.com)                                          [Close]
Agent  (*) ChatGPT  signed in
       ( ) OpenAI  key from OPENAI_API_KEY
       ( ) Anthropic  key saved with set-key
       ( ) Gemini  no key: howtui auth set-key gemini
       ( ) OpenAI compatible  set provider.openai_compatible.base_url
Model  ( ) GPT-6.1-Sol  gpt-6.1-sol
       (*) GPT-6-Luna  gpt-6-luna
       Send current folder listing (Ctrl+F): off
       Sign out (switch account)
Up/Down move  Enter choose  Esc close  (click works too)
```

Choose with the arrow keys and Enter, or click a line. Agent and model are saved for next time and used before
`provider.default` and `provider.<agent>.model` from the config file; `--provider` overrides both for one run. Switching
keeps the conversation, so a follow-up goes to the new agent with the earlier turns. An agent without a key or address
stays greyed out and tells you what is missing. **Sign out** (ChatGPT only) revokes the session; press Enter afterwards to
sign in with another account. Local models (Phase 6) will be added to the agent list; the Codex CLI was evaluated as an
agent and not adopted, see [ADR-0017](docs/adr/0017-no-codex-cli-delegation.md).

### Agents and API keys

| Agent | `--provider` | Default model | Key | Endpoint |
| --- | --- | --- | --- | --- |
| ChatGPT | `chatgpt` | GPT-6-Luna when listed | Sign in with ChatGPT (`howtui login`) | OpenAI, ChatGPT plan usage |
| OpenAI | `openai` | `gpt-6-astra` | `OPENAI_API_KEY` or `howtui auth set-key openai` | `api.openai.com` (Responses API, `store: false`) |
| Anthropic | `anthropic` | `claude-opus-5-5` | `ANTHROPIC_API_KEY` or `howtui auth set-key anthropic` | `api.anthropic.com` (Messages API) |
| Gemini | `gemini` | `gemini-3.8-flash` | `GEMINI_API_KEY` or `howtui auth set-key gemini` | `generativelanguage.googleapis.com` |
| OpenAI compatible | `openai_compatible` | first listed model | optional, `howtui auth set-key openai_compatible` or `api_key_env` | `provider.openai_compatible.base_url` (`/chat/completions`) |
| Ollama | `ollama` | first listed model | none | `http://127.0.0.1:11434/v1` or `provider.ollama.base_url` |
| llama-server | `llama_server` | the loaded model | only when started with `--api-key` | `http://127.0.0.1:9931/v1`, then `:8080` (older builds), or `provider.llama_server.base_url` |

```bash
howtui auth set-key openai        # paste the key; nothing is shown while you type or paste
printf '%s\n' "$KEY" | howtui auth set-key gemini   # or pipe it in
howtui auth status                # where each key comes from (never the key itself)
howtui auth remove-key openai
howtui --provider anthropic "find large files"
```

An environment variable wins over a saved key. `set-key` stores the key in the OS credential store (or, when none is
available, in a `0600` file in the data directory). Keys are never read from the config file: a `provider.<agent>.api_key`
entry stops howtui with a configuration error. For Anthropic models that support it, howtui asks for low reasoning
effort and turns on Anthropic's server-side fallback model for declined requests.

### Local models

howtui connects to a local server that is already running; it never downloads, loads or starts models itself.

```bash
# Ollama
ollama pull gemma4:e2b-it-qat
howtui --provider ollama

# llama.cpp
llama-server -m gemma-4-E2B_q4_0-it.gguf
howtui --provider llama_server
```

For local servers howtui turns the model's thinking mode off (`reasoning_effort: "none"`) to keep answers fast, asks
for JSON output through `response_format`, and waits up to 300 seconds per request (CPU-only machines are slow). If the
server is not running you get `Cannot reach Ollama at http://127.0.0.1:11434/v1` and exit code 6. `howtui doctor` shows the
server type, version, whether the model is still loading, and the model in use. Measured results for Gemma 4 E2B and E4B
on CPU: [docs/research/local-llm-eval.md](docs/research/local-llm-eval.md).

### Folder listing

Off by default. Turn it on with **Ctrl+F**, a click on `[folder: off]` at the right of the status line, or the line in
Settings; the choice is kept for next time (`context.folder = true` in the config file sets the starting value). While it
is on, every request also carries an `ls -al` style listing of the current folder, built by howtui itself (no `ls` is run,
so it works the same on Windows):

```text
listed 2026-10-24 01:40 UTC; mode, size, modified (UTC), name; owners and link targets omitted
drwxr-xr-x       - 2026-10-20 14:03 data/
-rw-------      48 2026-10-22 18:40 .env
-rw-r--r--   48.8K 2026-10-23 09:12 backup-2026-10.tar.gz
-rwxr-xr-x     310 2026-10-01 11:00 run.sh
lrwxrwxrwx       - 2026-09-30 08:00 current@
```

Only the first level, at most 150 entries, hidden entries included: type and permissions, size, modification time, name.
Not sent: file contents, owner and group (your account name), link targets, what is inside subfolders, the folder path.
File names are passed to the model as data, and howtui still never runs the suggested command. `--no-tui` follows the same
setting.

While howtui is open it receives mouse clicks, so selecting terminal text needs Shift+drag (Option+drag in iTerm2). Set
`ui.mouse = false` to keep the terminal's own mouse handling. Clicks are off on the sign-in screen so you can copy the URL.

### Shell integration

The installer asks whether to enable it. Answer yes (the default) and open a new terminal. Without the installer, run
`howtui init bash --install` (or `zsh`, `powershell`). Both add a marked block to `~/.bashrc`, `~/.zshrc` or the PowerShell profiles and nothing else;
`howtui init bash --uninstall` removes it. Nothing is changed unless you agree. To set it up by hand, add one line:

| Shell | Add to | Line |
| --- | --- | --- |
| Bash | `~/.bashrc` | `eval "$(howtui init bash)"` |
| Zsh | `~/.zshrc` | `eval "$(howtui init zsh)"` |
| PowerShell | `$PROFILE` | `howtui init powershell \| Out-String \| Invoke-Expression` |

| You do | Result |
| --- | --- |
| Press **Ctrl+G** at the prompt | howtui opens with the current line as the command. Accepting replaces the line; Esc or an error leaves it untouched. |
| Type `howtui ...` and accept | The command appears in the next prompt line (Bash and PowerShell also add it to history). |
| `x=$(howtui)`, `howtui \| pbcopy`, `howtui > file` | Unchanged: the command goes to stdout. |

Nothing is executed for you in any case. Options: `--key alt+h` (or another `ctrl+`/`alt+` letter), `--no-bind` (no key),
`--no-wrap` (no `howtui` function, so typed runs print to stdout as before). Bash 3.2 (the macOS default) gets the history
part only and prints a one-line notice instead of binding the key. If writing the result back to the shell fails, howtui
prints the command so you can copy it.

### Non-interactive mode

```bash
howtui --no-tui "count lines in all rust files" > cmd.txt
```

stdout gets the command only; the explanation, model notes and danger warnings go to stderr as plain text. No terminal is needed.

### Exit codes

| Code | Meaning |
| --- | --- |
| 0 | Command accepted and printed |
| 2 | Usage error |
| 3 | No usable terminal (no TTY, or the terminal does not answer cursor queries) |
| 4 | Sign-in required or failed |
| 5 | ChatGPT plan usage not available, not granted, or limit reached |
| 6 | Network error or temporary outage |
| 7 | Provider error (model or feature unsupported, unreadable response) |
| 8 | Configuration error |
| 130 | Cancelled |

<p align="right"><a href="#table-of-contents">↑ back to top</a></p>

---

## 5. Configuration

### Config file

| OS | Path |
| --- | --- |
| Linux, macOS | `$XDG_CONFIG_HOME/howtui/config.toml` (default `~/.config/howtui/config.toml`) |
| Windows | `%APPDATA%\howtui\config.toml` |

The file is optional and never created automatically.

```toml
[ui]
mode = "inline"        # "fullscreen" is planned
inline_height = 10     # 3-40
language = "auto"      # "auto", "en" or "ko"
mouse = true           # false: no clicks, terminal keeps its own text selection and scrolling

[update]
check = true           # false: no daily check and no "available" note (same as HOWTUI_NO_UPDATE_CHECK=1)

[context]
folder = false         # true: send the ls -al style folder listing unless turned off with Ctrl+F

[provider]
# default = "chatgpt"      # chatgpt, openai, anthropic, gemini, openai_compatible, ollama, llama_server (Settings choice and --provider win)

[provider.chatgpt]
# model = "<model slug>"   # used when nothing was chosen in Settings; default gpt-6-luna, else the first listed

[provider.openai]          # same keys for [provider.anthropic] and [provider.gemini]
# model = "gpt-6-astra"
# api_key_env = "OPENAI_API_KEY"   # name of the variable to read; never the key itself

[provider.ollama]          # same keys for [provider.llama_server] (plus api_key_env)
# base_url = "http://127.0.0.1:11434/v1"
# model = "gemma4:e2b-it-qat"

[provider.openai_compatible]
# base_url = "http://localhost:8080/v1"
# model = "<model id>"
# api_key_env = "MY_SERVER_KEY"
```

Sign-in state and the file-based token store live in the data directory: `$XDG_DATA_HOME/howtui`
(default `~/.local/share/howtui`) on Linux and macOS, `%LOCALAPPDATA%\howtui` on Windows.

### Environment variables

| Variable | Meaning |
| --- | --- |
| `HOWTUI_LANG` | UI language (`en`, `ko`), overrides `ui.language` |
| `HOWTUI_CONFIG` | Config file path |
| `HOWTUI_DATA_DIR` | Data directory |
| `HOWTUI_NO_BROWSER=1` | Never open a browser during sign-in (also implied in SSH sessions) |
| `HOWTUI_CREDENTIAL_STORE` | `keyring` or `file`, overrides the automatic choice |
| `HOWTUI_LOG`, `HOWTUI_LOG_FILE` | Write a diagnostic log (metadata only, no questions, commands or tokens) |
| `HOWTUI_NO_UPDATE_CHECK=1` | No daily update check and no update note |

<p align="right"><a href="#table-of-contents">↑ back to top</a></p>

---

## 6. Platform support

Officially supported pairs: **Linux + Bash**, **macOS + Zsh**, **Windows + PowerShell**. Other shells are not claimed.

Shell integration, as tested (PTY end-to-end: Ctrl+G, next-prompt insertion, capture, cancel, errors, multi-line, fallback):

| Shell | Tested | Not tested yet |
| --- | --- | --- |
| Bash | 5.0 (Ubuntu 20.04), 5.2 (Ubuntu 24.04, CI); 3.2 on macOS (history only, CI); script loading on 4.0, 4.4, 5.1 | |
| Zsh | 5.9 (Ubuntu 24.04 and macOS 15 in CI, Debian 12) | 5.1-5.8 |
| PowerShell | 7.4 and 7.6 on Linux, 7.6 on macOS (PSReadLine 2.3.5, 2.4.5); Windows checked by hand | Windows with code page 949 |

| Target | Use it for | Verified |
| --- | --- | --- |
| `x86_64-unknown-linux-gnu` | Linux PCs and servers | CI tests, PTY end-to-end tests |
| `aarch64-unknown-linux-gnu` | 64-bit Raspberry Pi OS (Pi 3, 4, 5), ARM servers | CI tests on ARM64 Linux |
| `armv7-unknown-linux-gnueabihf` | 32-bit Raspberry Pi OS (Pi 2-5) | Build and glibc check only |
| `aarch64-apple-darwin` | Apple Silicon Macs | CI tests, PTY end-to-end tests |
| `x86_64-apple-darwin` | Intel Macs | Build, minimum-version and signature checks only |
| `x86_64-pc-windows-msvc` | Windows x64 | CI tests, ConPTY end-to-end tests, installer tests (PowerShell 5.1 and 7), manual check (Windows Terminal, PowerShell) |
| `aarch64-pc-windows-msvc` | Windows on ARM | CI tests and ConPTY end-to-end tests on Windows ARM64 |

Linux builds need glibc 2.17 or newer (Raspberry Pi OS Buster and later, Ubuntu 20.04 and later); musl-based distributions are
not supported. macOS builds need macOS 11 or newer. Windows builds link the C runtime statically. Raspberry Pi 1, Zero and
Zero W (ARMv6) are not supported, and no Raspberry Pi hardware has been tested yet.

<p align="right"><a href="#table-of-contents">↑ back to top</a></p>

---

## 7. Privacy and safety

- **Sent to the agent you chose when you submit a request** (OpenAI for ChatGPT and OpenAI, Anthropic, Google for
  Gemini, your local Ollama or llama-server, or the server at your `base_url`): your question, the current command (including your edits), your OS, CPU
  architecture and shell name, and earlier turns of the same session. Only when you turn on the
  [folder listing](#folder-listing): an `ls -al` style listing of the current folder (names including hidden ones,
  permissions, sizes, modification times). Never sent: environment variables, file contents, owner names, shell history,
  the folder path.
- **Received at sign-in:** your account email and name. howtui keeps the email to show it masked.
- **Update check:** at most once a day, one request to the GitHub release page to read the newest version number
  ([Update](#update)). Off with `HOWTUI_NO_UPDATE_CHECK=1`.
- **Stored on your machine:** OAuth tokens and API keys saved with `howtui auth set-key` in the OS credential store
  (Keychain, Credential Manager, Secret Service) or, when none is available, in a `0600` file in the data directory. Keys
  are never printed, logged or sent anywhere except in the request header to their own provider.
- **Danger warnings** are simple rules over the command text. They miss things and are not a security boundary.
- **howtui never runs the commands it suggests.** Review them before you press Enter in your shell.

<p align="right"><a href="#table-of-contents">↑ back to top</a></p>

---

## 8. Development

| Path | Content |
| --- | --- |
| `src/app` | Pure state machine and editor (no terminal or network access) |
| `src/ui` | Inline renderer, key map, terminal session |
| `src/llm` | Provider interface, prompts, response parsing, ChatGPT provider, API key providers, agent switching |
| `src/auth` | Sign in with ChatGPT, token storage and refresh |
| `docs/` | Specs (MRD, PRD, SRS, TC), traceability, ADRs, research (Korean) |
| `tests/` | CLI integration tests, PTY end-to-end tests, installer tests |

```bash
cargo test --features mock-provider          # unit and integration tests
cargo build --features mock-provider && tests/pty/run.sh   # PTY end-to-end (needs tmux)
tests/pty/shell.sh bash                      # shell integration end-to-end (also zsh, powershell)
uv run scripts/trace_check.py                # requirement traceability check
```

The `mock-provider` feature adds a scripted provider and test-only endpoint overrides; release builds do not include it.
Releases are built by `.github/workflows/release.yml` from `v*` tags as drafts and published by hand.

<p align="right"><a href="#table-of-contents">↑ back to top</a></p>

---

## 9. Roadmap

- [x] Phase 1: inline UI, Sign in with ChatGPT, stdout output, installers
- [x] Phase 2: optional shell integration that puts the command into the Bash, Zsh or PowerShell prompt; settings view for the model
- [ ] Phase 3: fullscreen mode
- [x] Phase 4: update checks in `doctor`, release provenance, uninstall; Raspberry Pi and Windows ARM64 hardware not checked yet
- [x] Phase 5: API key providers (OpenAI, Gemini, Anthropic, OpenAI-compatible) and agent switching in Settings
- [x] Phase 6: local models through Ollama and llama.cpp `llama-server`, Gemma 4 E2B and E4B measured on CPU

<p align="right"><a href="#table-of-contents">↑ back to top</a></p>

---

## 10. License

[MIT](LICENSE)

<p align="right"><a href="#table-of-contents">↑ back to top</a></p>
