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

> **Status: pre-release.** v0.1.0 has not been published yet, so the install commands start working with the first release.
> Inference uses **Sign in with ChatGPT**, which OpenAI currently offers as a limited preview.

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
   - [Update and uninstall](#update-and-uninstall)
3. [Quickstart](#3-quickstart)
4. [Usage](#4-usage)
   - [Commands](#commands)
   - [Keys](#keys)
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
- **Clean stdout.** The UI draws on the controlling terminal, so `cmd=$(howtui)` and `howtui | pbcopy` receive only the command.
- **Shell integration (optional).** Ctrl+G opens howtui on the current command line in Bash, Zsh or PowerShell, and typing `howtui` puts the result into the next prompt. You add one line to your rc file yourself; nothing edits it for you.
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
`SHA256SUMS`, verifies the checksum, and installs to `~/.local/bin` without sudo. It never edits your shell rc files; if the
directory is not on your `PATH`, it prints the line to add.

| Option | Environment variable | Meaning |
| --- | --- | --- |
| `--version X.Y.Z` | `HOWTUI_VERSION` | Install a specific release |
| `--bin-dir DIR` | `HOWTUI_INSTALL_DIR` | Install directory |
| `--print-target` | | Print the detected release target and exit |

### Windows

Windows PowerShell 5.1 or PowerShell 7:

```powershell
irm https://raw.githubusercontent.com/hungryZoo/howtui/main/scripts/install.ps1 | iex
```

Installs `howtui.exe` to `%LOCALAPPDATA%\Programs\howtui\bin`, verifies the checksum, and adds that directory to your user
`PATH`. It never edits PowerShell profiles.

| Environment variable | Meaning |
| --- | --- |
| `HOWTUI_VERSION` | Install a specific release |
| `HOWTUI_INSTALL_DIR` | Install directory |
| `HOWTUI_NO_MODIFY_PATH=1` | Do not touch the user `PATH` |
| `HOWTUI_ARCH` | Override the detected architecture (`x86_64` or `aarch64`) |

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

### Update and uninstall

- **Update:** run the same install command again; pass the same `--bin-dir` or `HOWTUI_INSTALL_DIR` if you changed it.
- **Uninstall:** run `howtui logout`, delete the binary, and remove the config and data directories listed in
  [Configuration](#5-configuration).

<p align="right"><a href="#table-of-contents">↑ back to top</a></p>

---

## 3. Quickstart

```bash
howtui                  # first run signs you in, then: ask, edit, Enter
howtui find large log files    # ask right away
eval "$(howtui init bash)"     # optional: add to ~/.bashrc (zsh: ~/.zshrc) for Ctrl+G and next-prompt insertion
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
| `howtui auth status` | Show the signed-in account (masked email), plan usage and token expiry |
| `howtui doctor [--json]` | Check terminal, locale, config, sign-in and model access; output contains no tokens or home paths |
| `howtui init bash\|zsh\|powershell` | Print the shell integration script (options: `--key ctrl+x`, `--no-bind`, `--no-wrap`) |

Common options: `--shell bash|zsh|powershell`, `--height N` (3-40).

### Keys

| Key | Ask line | Command editor |
| --- | --- | --- |
| Enter | Send (first request generates, later ones revise); on an empty ask line, accept the command | Accept (high-risk commands ask for a second Enter) |
| Ctrl+J, Alt+Enter | | New line (Shift+Enter where the terminal reports it) |
| Tab, Shift+Tab | Switch to the command | Switch to the ask line |
| Ctrl+E | | Explain the current command |
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

### Shell integration

Add one line to your shell startup file and open a new terminal. To remove it, delete the line.

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

[provider]
default = "chatgpt"

[provider.chatgpt]
# model = "<model slug>"   # default: first model your account lists
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

<p align="right"><a href="#table-of-contents">↑ back to top</a></p>

---

## 6. Platform support

Officially supported pairs: **Linux + Bash**, **macOS + Zsh**, **Windows + PowerShell**. Other shells are not claimed.

Shell integration, as tested (PTY end-to-end: Ctrl+G, next-prompt insertion, capture, cancel, errors, multi-line, fallback):

| Shell | Tested | Not tested yet |
| --- | --- | --- |
| Bash | 5.0 (Ubuntu 20.04), 5.2 (Ubuntu 24.04, CI); 3.2 on macOS (history only, CI); script loading on 4.0, 4.4, 5.1 | |
| Zsh | 5.9 (Ubuntu 24.04 and macOS 15 in CI, Debian 12) | 5.1-5.8 |
| PowerShell | 7.4 and 7.6 on Linux, 7.6 on macOS (PSReadLine 2.3.5, 2.4.5) | Windows PowerShell 5.1 and PowerShell 7 on Windows |

| Target | Use it for | Verified |
| --- | --- | --- |
| `x86_64-unknown-linux-gnu` | Linux PCs and servers | CI tests, PTY end-to-end tests |
| `aarch64-unknown-linux-gnu` | 64-bit Raspberry Pi OS (Pi 3, 4, 5), ARM servers | CI tests on ARM64 Linux |
| `armv7-unknown-linux-gnueabihf` | 32-bit Raspberry Pi OS (Pi 2-5) | Build and glibc check only |
| `aarch64-apple-darwin` | Apple Silicon Macs | CI tests, PTY end-to-end tests |
| `x86_64-apple-darwin` | Intel Macs | Build, minimum-version and signature checks only |
| `x86_64-pc-windows-msvc` | Windows x64 | CI tests, ConPTY end-to-end tests, installer tests (PowerShell 5.1 and 7) |
| `aarch64-pc-windows-msvc` | Windows on ARM | CI tests and ConPTY end-to-end tests on Windows ARM64 |

Linux builds need glibc 2.17 or newer (Raspberry Pi OS Buster and later, Ubuntu 20.04 and later); musl-based distributions are
not supported. macOS builds need macOS 11 or newer. Windows builds link the C runtime statically. Raspberry Pi 1, Zero and
Zero W (ARMv6) are not supported, and no Raspberry Pi hardware has been tested yet.

<p align="right"><a href="#table-of-contents">↑ back to top</a></p>

---

## 7. Privacy and safety

- **Sent to OpenAI when you submit a request:** your question, the current command (including your edits), your OS, CPU
  architecture and shell name, and earlier turns of the same session. Never sent: environment variables, file contents, shell
  history, the working directory.
- **Received at sign-in:** your account email and name. howtui keeps the email to show it masked.
- **Stored on your machine:** OAuth tokens in the OS credential store (Keychain, Credential Manager, Secret Service) or, when
  none is available, in a `0600` file in the data directory.
- **Danger warnings** are simple rules over the command text. They miss things and are not a security boundary.
- **howtui never runs the commands it suggests.** Review them before you press Enter in your shell.

<p align="right"><a href="#table-of-contents">↑ back to top</a></p>

---

## 8. Development

| Path | Content |
| --- | --- |
| `src/app` | Pure state machine and editor (no terminal or network access) |
| `src/ui` | Inline renderer, key map, terminal session |
| `src/llm` | Provider interface, prompts, response parsing, ChatGPT provider |
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

- [ ] Phase 1: inline UI, Sign in with ChatGPT, stdout output, installers (implemented, waiting for real-account and Windows checks)
- [ ] Phase 2: optional shell integration that puts the command into the Bash, Zsh or PowerShell prompt (implemented, Windows checks pending)
- [ ] Phase 3: fullscreen mode
- [ ] Phase 4: Raspberry Pi and Windows ARM64 hardware checks, update checks in `doctor`
- [ ] Phase 5: API key providers (OpenAI, Gemini, Anthropic, OpenAI-compatible)
- [ ] Phase 6: local models through Ollama and llama.cpp `llama-server`

<p align="right"><a href="#table-of-contents">↑ back to top</a></p>

---

## 10. License

[MIT](LICENSE)

<p align="right"><a href="#table-of-contents">↑ back to top</a></p>
