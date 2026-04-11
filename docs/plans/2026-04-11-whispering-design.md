# Whispering — Design Document

**Date:** 2026-04-11
**Status:** Approved

## Overview

A macOS menu bar utility written in Rust (Tauri v2) that provides high-quality speech-to-text via a global hotkey. Replaces Apple Dictation with whisper.cpp running locally on Apple Silicon via Metal GPU acceleration. Transcribed text is typed directly at the cursor in whatever app is focused.

## Goals

- Global hotkey `Ctrl+Cmd+M` toggles recording on/off
- Transcription appears at cursor within ~1-2s after stopping (for typical utterances)
- Runs entirely offline — no network calls, no API keys
- Lives in the menu bar; no Dock icon

## Out of Scope

- Silence/VAD-based auto-stop
- Streaming or partial transcription results
- OpenAI API fallback
- Windows or Linux support

---

## Architecture

A Tauri v2 app shell with a Rust backend. No meaningful frontend — the only UI is a system tray icon and a small settings window.

```
┌─────────────────────────────────────────────┐
│                  Tauri v2                   │
│                                             │
│  ┌──────────────┐    ┌───────────────────┐  │
│  │ global-hotkey│    │   System Tray     │  │
│  │   Ctrl+Cmd+M │    │  (idle / record)  │  │
│  └──────┬───────┘    └───────────────────┘  │
│         │                                   │
│  ┌──────▼───────┐                           │
│  │  App State   │  (Idle | Recording)       │
│  └──────┬───────┘                           │
│         │                                   │
│  ┌──────▼───────┐    ┌───────────────────┐  │
│  │    cpal      │    │    whisper-rs     │  │
│  │ (mic capture)│───▶│  (whisper.cpp +   │  │
│  └──────────────┘    │   Metal backend)  │  │
│                      └────────┬──────────┘  │
│                               │             │
│                      ┌────────▼──────────┐  │
│                      │      enigo        │  │
│                      │ (keystroke inject)│  │
│                      └───────────────────┘  │
└─────────────────────────────────────────────┘
```

---

## Components

| Component | Crate | Responsibility |
|---|---|---|
| App shell | `tauri` v2 | Window lifecycle, tray, settings window, no Dock icon |
| Global hotkey | `global-hotkey` | Register `Ctrl+Cmd+M` system-wide, receive press events |
| Audio capture | `cpal` | Open default mic, stream PCM f32 samples into a buffer |
| Transcription | `whisper-rs` | Rust bindings to whisper.cpp; runs inference on buffered audio using Metal |
| Keystroke injection | `enigo` | Type transcribed text at the currently focused cursor |
| Settings | `serde` + TOML | Persist model path and mic device selection to `~/.whispering/config.toml` |

---

## Data Flow

### Start recording (`Ctrl+Cmd+M`, state = Idle)

1. Switch state to `Recording`
2. Update tray icon → green mic icon
3. Open mic stream via `cpal` (default input device)
4. Begin accumulating raw PCM f32 samples in a `Vec<f32>` buffer

### Stop recording (`Ctrl+Cmd+M`, state = Recording)

1. Close mic stream
2. Switch state to `Idle`
3. Update tray icon → normal icon
4. Resample buffer to 16kHz mono (whisper.cpp requirement) if needed
5. Pass audio to `whisper-rs` for inference
6. Receive transcribed `String`
7. Use `enigo` to type the string at the currently focused cursor

---

## Latency Budget

| Stage | Target |
|---|---|
| Hotkey → mic open | < 100ms |
| Inference: `medium.en`, ~5s audio, Metal | ~500–1000ms |
| Inference: `medium.en`, ~15s audio, Metal | ~1500–2500ms |
| Keystroke injection | < 50ms |

`medium.en` on M-series chips runs at roughly 5–10x real-time with Metal. A 10s clip transcribes in ~1–2s. This meets the design goal for typical dictation bursts.

---

## Model Configuration

Models are stored at `~/.whispering/models/`. The active model path is configurable via the settings window. Default: `~/.whispering/models/ggml-medium.en.bin`.

Models are not bundled with the app. On first launch, the app will prompt the user to download or locate a model file.

Supported model sizes (all configurable):

| Model | Size | Notes |
|---|---|---|
| `base.en` | ~140MB | Fastest, lower accuracy |
| `small.en` | ~460MB | Good balance |
| `medium.en` | ~1.5GB | **Default** — best accuracy |

---

## Settings Window

Accessible via tray menu → "Settings". Fields:

- **Model path** — file picker, points to a `.bin` ggml model file
- **Input device** — dropdown of available microphone inputs (via `cpal`)

Settings are persisted to `~/.whispering/config.toml`.

---

## System Tray

| State | Icon |
|---|---|
| Idle | Microphone icon (monochrome, matches macOS menu bar style) |
| Recording | Microphone icon (green, matches Apple's mic indicator style) |

Tray menu items:
- **Settings...**
- **Quit**

---

## macOS Permissions

The app requires:
- **Microphone access** — requested on first recording attempt via macOS permission dialog
- **Accessibility access** — required by `enigo` for keystroke injection; user must grant in System Settings → Privacy & Security → Accessibility

The app should detect missing permissions and surface a clear message in the tray menu or a modal rather than silently failing.

---

## Project Structure

```
whispering/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs          # Tauri app entry, tray setup
│   │   ├── hotkey.rs        # Global hotkey registration and event loop
│   │   ├── audio.rs         # cpal mic capture, PCM buffering
│   │   ├── transcribe.rs    # whisper-rs inference wrapper
│   │   ├── inject.rs        # enigo keystroke injection
│   │   └── settings.rs      # Config load/save (TOML)
│   ├── Cargo.toml
│   └── tauri.conf.json
├── docs/
│   └── plans/
│       └── 2026-04-11-whispering-design.md
└── README.md
```
