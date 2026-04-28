# CLAUDE.md

This file provides guidance to agents working in this repository. `AGENTS.md` is a symbolic link to this file.

## Core Principles

- Keep changes simple and tightly scoped.
- Find root causes. Do not paper over failures.
- Prefer small Rust functions with explicit state transitions over deeply nested control flow.
- Comments should explain why a constraint exists, not restate what the code does.
- Keep user-facing text professional and concise.

## Commands

Run commands from the repository root.

```bash
npm install        # install React/Vite frontend dependencies
make dev           # run Tauri dev mode
make build         # build frontend and Rust app with Cargo.lock respected
make lint          # frontend build, rustfmt check, clippy, and tests
make clippy        # clippy with -D warnings across all Rust targets
make clippy-review # extended Rust review lint pass
make fmt           # apply rustfmt
make fmt-check     # check rustfmt without rewriting
make test          # run Rust tests
make release       # bundle .app and build DMG
make install       # download ggml-medium.bin model
make clean         # remove Rust build artifacts
```

All Rust work lives in `src-tauri/`. If running cargo directly, use the manifest path and keep CI-style commands locked:

```bash
cargo build --locked --manifest-path src-tauri/Cargo.toml
cargo check --locked --manifest-path src-tauri/Cargo.toml
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --locked --manifest-path src-tauri/Cargo.toml
```

Use `make clippy-review` for deeper Rust review. It enables `clippy::all`, `clippy::pedantic`, `clippy::nursery`, and `clippy::cargo`; this target is intentionally noisier than the merge gate.

## Rust Practices

- Treat `make lint` as the default pre-commit gate.
- Prefer `cargo fmt -- --check` in verification paths; use `make fmt` only when intentionally rewriting formatting.
- Run clippy with `--all-targets` and `-D warnings` for normal work.
- Keep lock lifetimes tight. Do not hold a `MutexGuard` across unrelated work.
- Do not use `unwrap()` in new non-test code unless the invariant is local and obvious; prefer error handling or `expect("reason")`.
- Avoid `let _ =` on fallible operations unless failure is deliberately non-fatal and logged nearby.
- Put pure decision logic in standalone functions where possible so behavior can be unit tested without Tauri, audio, filesystem, or model dependencies.

## Architecture

This is a macOS-only menu bar app with no Dock icon.

**Rust entry point**: `lib.rs::run()` bootstraps permissions, model loading, tray setup, and the hotkey event loop. `main.rs` just calls `whispering_lib::run()`.

**Frontend**: The lightweight status popover is React + TypeScript under `src/`, built by Vite into ignored `dist/`. Tauri loads the generated `index.html` for a non-focus-stealing window anchored under the menu-bar tray icon.

**State machine**: A single `Arc<WhisperingState>` is shared across threads via Tauri `.manage()`. It owns:

- `recording: Mutex<RecordingState>` — `Idle` or `Recording(AudioCapture)`
- `transcriber: Mutex<Option<Transcriber>>` — loaded async at startup and reused
- `config: Mutex<Config>` — persisted to `~/.whispering/config.toml`
- `status: Mutex<RuntimeStatus>` — visible runtime phase and last user-facing error

**Recording flow**:

1. `Ctrl+Cmd+M` on key-down only calls `toggle_recording()`.
2. Idle starts `AudioCapture`, sets tray/status popover to recording, and plays the start sound.
3. Recording stops capture, switches to transcribing, and runs Whisper on a background thread.
4. Successful transcription is saved to cache before text injection.
5. Injection success shows a short success state; failures are shown in the status popover and remain recoverable from cache.

**Recovery location**:

```text
~/Library/Caches/Whispering/transcripts/latest.txt
```

This is cache data so macOS may clean it under storage pressure. Do not save raw audio.

## Module Responsibilities

| Module | Responsibility |
|---|---|
| `lib.rs` | App entry, tray setup, hotkey loop, state transitions, transcription flow |
| `audio.rs` | `AudioCapture`: cpal stream open/close, PCM buffering, resample to 16kHz |
| `transcribe.rs` | `Transcriber`: load whisper.cpp model, run inference |
| `inject.rs` | `type_text()`: enigo keystroke injection at focused cursor |
| `settings.rs` | Config load/save, model discovery, model language behavior |
| `permissions.rs` | Accessibility and microphone permission prompts |
| `sounds.rs` | Start/stop system sounds |
| `status.rs` | Menu-bar anchored status popover control |
| `transcripts.rs` | macOS cache transcript recovery writes |

## Runtime Prerequisites

- Model file at `~/.whispering/models/ggml-medium.bin` through `make install`
- macOS Accessibility permission granted
- macOS Microphone permission granted
