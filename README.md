# Whispering

Whispering is a local speech-to-text app built with Tauri, Rust, and `whisper-rs`. It records from the microphone, transcribes with Whisper, and types the result at the focused cursor.

The repo now has OS-aware runtime routing and OS-aware Makefile targets for `macos`, `windows`, and `linux`. The main goal is simple local usage: clone the repo, build it, install the model, and run it.

## What You Need

- Rust toolchain via `rustup`
- Node.js and npm
- Tauri CLI (`cargo install tauri-cli`)
- A local Whisper ggml model file

macOS-specific runtime expectations:

- macOS 10.15 or newer
- Accessibility permission for text injection
- Microphone permission for recording

Linux build prerequisites:

- WebKitGTK and tray-related development packages
- AppImage-compatible Linux desktop environment
- A desktop environment with system tray support is preferred, but the app falls back to a floating status surface when tray behavior is limited

On Debian or Ubuntu hosts, install the packages Tauri documents for Linux builds:

```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

Windows build prerequisites:

- PowerShell
- Microsoft C++ Build Tools with "Desktop development with C++"
- Microsoft Edge WebView2 runtime
- Desktop install permissions for MSI packages on the current machine
- If MSI bundling fails with `light.exe` errors, enable the Windows VBSCRIPT optional feature

## Get Started

Clone the repo, then from the repo root:

```bash
make install
```

`make install` detects the current OS and runs the full platform install flow. It installs the default model, builds the native release artifact for the host OS, installs the app, and prints how to launch it.

`make run` remains the source-run path for local development. Use `make run-macos-app`, `make run-linux-app`, or `make run-windows-app` to launch the installed app.

If you want to bypass auto-detection, use one of:

```bash
make install-model-macos
make install-model-linux
make install-model-windows
make install-macos
make install-linux
make install-windows
```

## Useful Commands

```bash
make build          # debug build
make install        # detect the host OS and run the full platform install flow
make install-model  # detect the host OS and install only the default model
make run            # run the app from the repo
make dev            # alias for running in development mode
make release        # detect the host OS and dispatch to a release target
make release-macos  # build the macOS app bundle and DMG
make release-linux  # build the Linux AppImage release artifact on a Linux host
make release-windows # build the Windows MSI release artifact on a Windows host
make install-macos  # install the macOS app into /Applications
make install-linux  # install the Linux AppImage into ~/.local/opt/Whispering
make install-windows # install the Windows MSI on a Windows host
make run-macos-app  # launch the installed macOS app
make run-linux-app  # launch the installed Linux AppImage
make run-windows-app # launch the installed Windows app
make lint           # frontend build + rustfmt check + clippy -D warnings + tests
make clean          # remove Rust build artifacts
```

## Platform Shortcuts

Use the global shortcut below to start and stop recording:

| Platform | Shortcut |
|---|---|
| macOS | `Ctrl+Cmd+M` |
| Windows | `Ctrl+Alt+M` |
| Linux | `Ctrl+Alt+M` |

## Runtime Notes

- The default config is stored at `~/.whispering/config.toml`.
- Models are stored at `~/.whispering/models/`.
- Transcript recovery files are stored under the resolved platform cache directory in `Whispering/transcripts/`.
- Transcription uses `whisper-rs` on every supported OS. macOS enables the Metal backend; Linux and Windows use the shared non-Metal backend.

## Install and Run on macOS

To install a local macOS build and launch it like a normal app:

```bash
make install-macos
```

The local app bundle is built at:

```text
src-tauri/target/release/bundle/macos/Whispering.app
```

Unsigned local builds may need to be opened once from Finder before macOS will trust them.

## Install and Run on Linux

Install the model, build the AppImage, install it into the user-local app path, and register the desktop entry:

```bash
make install-linux
make run-linux-app
```

Installed Linux paths:

```text
~/.local/opt/Whispering/Whispering.AppImage
~/.local/share/applications/whispering.desktop
```

If the desktop entry is available in your environment, you can also launch Whispering from the applications menu.

## Install and Run on Windows

Install the model, build the MSI, run the installer, then launch the installed app:

```bash
make install-windows
make run-windows-app
```

`make install-windows` runs the native MSI installer on a Windows host. After that, Whispering should be available from the Start menu and through `make run-windows-app`.
