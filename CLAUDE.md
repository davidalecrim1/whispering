# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

All commands are run from the repo root unless noted.

```bash
make dev          # Run in development mode (cargo tauri dev)
make build        # Debug build only (cargo build)
make release      # Bundle .app / .dmg (cargo tauri build)
make lint         # cargo fmt + clippy -D warnings
make install      # Download ggml-medium.en.bin model to ~/.whispering/models/
make clean        # Remove build artifacts
```

All Rust work lives in `src-tauri/`. If running cargo commands directly:

```bash
cd src-tauri
cargo build
cargo clippy -- -D warnings
cargo fmt
```

There are no tests yet. The `dist/` directory contains a stub `index.html` — Tauri requires a frontend dist even though this app has no web UI.

## Architecture

This is a macOS-only menu bar app (no Dock icon). The entire app logic is in `src-tauri/src/`.

**Entry point**: `lib.rs::run()` bootstraps everything — permissions, model loading, tray, and the hotkey event loop. `main.rs` just calls `whispering_lib::run()`.

**State machine**: A single `Arc<WhisperingState>` is shared across threads, managed via Tauri's `.manage()`. It holds:
- `recording: Mutex<RecordingState>` — `Idle` or `Recording(AudioCapture)`
- `transcriber: Mutex<Option<Transcriber>>` — loaded async at startup in a background thread
- `config: Mutex<Config>` — persisted to `~/.whispering/config.toml`

**Recording flow**:
1. `Ctrl+Cmd+M` (key-down only, not key-up) → `toggle_recording()`
2. **Idle → Recording**: `AudioCapture::start()` opens the default mic via `cpal`, begins buffering raw PCM f32 samples. Tray icon goes green, start sound plays.
3. **Recording → Idle**: `AudioCapture::stop()` drops the stream, returns the buffer resampled to 16kHz mono. A background thread runs `Transcriber::transcribe()` via `whisper-rs` with Metal backend. On success, `inject::type_text()` types the result at the cursor. Stop sound plays.

**Key design decisions**:
- The `global-hotkey` event fires on both `Pressed` and `Released` — we filter to `Pressed` only to avoid double-toggle.
- The `Transcriber` is loaded once at startup (warm Metal context) and reused for every transcription. Model path is configurable; default is `~/.whispering/models/ggml-medium.en.bin`.
- Icons are `include_bytes!`-embedded at compile time (`tray-idle@2x.png`, `tray-recording@2x.png`) so they work in both dev and release without path issues.
- `LSUIElement = true` in `Info.plist` hides the Dock icon. `ActivationPolicy::Accessory` is also set at runtime for dev mode.

**Module responsibilities**:

| Module | Responsibility |
|---|---|
| `lib.rs` | App entry, tray setup, hotkey loop, state transitions, menu rebuilding |
| `audio.rs` | `AudioCapture`: cpal stream open/close, PCM buffering, linear-interpolation resample to 16kHz |
| `transcribe.rs` | `Transcriber`: load whisper.cpp model, run inference with language param |
| `inject.rs` | `type_text()`: enigo keystroke injection at focused cursor |
| `settings.rs` | `Config` struct (model path, input device, language), load/save TOML |
| `permissions.rs` | Request Accessibility (AXIsProcessTrustedWithOptions) and Microphone (AVCaptureDevice) on startup |
| `sounds.rs` | Play `begin_record.caf` / `end_record.caf` from CoreAudio system sounds |

**macOS-specific deps** (under `[target.'cfg(target_os = "macos")'.dependencies]`):
- `accessibility-sys` + `core-foundation` — AX permission dialog
- `objc2` + `objc2-av-foundation` + `block2` — mic permission dialog
- `objc2-app-kit` — `NSSound` for system sounds

## Runtime prerequisites

- Model file at `~/.whispering/models/ggml-medium.en.bin` (run `make install`)
- macOS Accessibility permission granted (prompted on first launch)
- macOS Microphone permission granted (prompted on first launch)
