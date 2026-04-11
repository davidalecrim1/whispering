# Whispering — Implementation Plan

## Phase 1: Project Scaffold

- [ ] Initialize Tauri v2 project with `cargo tauri init`
- [ ] Configure `tauri.conf.json`: no Dock icon (`LSUIElement = true`), app name, bundle ID
- [ ] Add all dependencies to `Cargo.toml`: `tauri`, `global-hotkey`, `cpal`, `whisper-rs`, `enigo`, `serde`, `toml`
- [ ] Verify `cargo build` succeeds on a blank slate

## Phase 2: Settings

- [ ] Define `Config` struct in `settings.rs` (model path, input device name)
- [ ] Implement load from `~/.whispering/config.toml`, with defaults if missing
- [ ] Implement save back to file
- [ ] Create `~/.whispering/models/` directory on first run if absent

## Phase 3: System Tray

- [ ] Set up system tray in `main.rs` with idle mic icon
- [ ] Add tray menu items: "Settings..." and "Quit"
- [ ] Create two tray icon assets: monochrome (idle) and green (recording)
- [ ] Wire tray state to `AppState` (idle / recording)

## Phase 4: Global Hotkey

- [ ] Register `Ctrl+Cmd+M` via `global-hotkey` crate in `hotkey.rs`
- [ ] Set up event loop to receive hotkey press events
- [ ] Toggle `AppState` on each press: Idle → Recording → Idle
- [ ] Update tray icon on each state change

## Phase 5: Audio Capture

- [ ] Implement mic stream open/close in `audio.rs` using `cpal`
- [ ] Use default input device (or configured device from settings)
- [ ] Accumulate raw PCM f32 samples into a `Vec<f32>` while recording
- [ ] On stop: resample to 16kHz mono if the device sample rate differs

## Phase 6: Transcription

- [ ] Implement `whisper-rs` inference wrapper in `transcribe.rs`
- [ ] Load model from path in config on app start (warm up Metal context)
- [ ] Accept `Vec<f32>` audio buffer, return `String`
- [ ] Run inference on a background thread to avoid blocking the main thread
- [ ] Surface error if model file not found (missing model message in tray)

## Phase 7: Keystroke Injection

- [ ] Implement `inject_text(text: &str)` in `inject.rs` using `enigo`
- [ ] Type text at currently focused cursor
- [ ] Handle Accessibility permission missing: show tray notification

## Phase 8: Permissions

- [ ] Microphone: trigger macOS permission dialog on first recording attempt
- [ ] Accessibility: detect missing permission, show actionable error in tray menu
- [ ] Add required `Info.plist` keys: `NSMicrophoneUsageDescription`

## Phase 9: Settings Window

- [ ] Create minimal HTML/JS settings window (model path file picker, mic dropdown)
- [ ] Populate mic dropdown from `cpal` device enumeration via Tauri command
- [ ] Save settings on form submit, reload config in backend

## Phase 10: Polish & Verification

- [ ] Test full flow: hotkey → record → stop → transcribe → text appears at cursor
- [ ] Verify latency meets budget (<100ms to start mic, ~1-2s to transcribe 5-10s audio)
- [ ] Test permission denial paths (mic denied, accessibility denied)
- [ ] Test model missing path (clear error shown)
- [ ] Build release binary and verify no Dock icon appears

---

## Review

_To be filled after implementation._
