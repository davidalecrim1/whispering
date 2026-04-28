# Whispering

Whispering is a macOS menu bar speech-to-text app built with Tauri and Rust. It records from the microphone, runs Whisper locally with `whisper-rs`, and types the transcription at the current cursor position.

## What You Need

- macOS 10.15 or newer
- Rust toolchain (`rustup`)
- Node.js and npm
- Tauri CLI (`cargo install tauri-cli`)
- A local Whisper ggml model file

This app is macOS-only. The current implementation depends on macOS permissions, tray APIs, Metal-backed Whisper inference, and a minimum macOS version of 10.15.

## Get Started
Clone the repo, then from the repo root:

```bash
npm install
make install
make dev
```

`make install` downloads the default multilingual model to `~/.whispering/models/ggml-medium.bin`.

Useful commands:

```bash
make dev      # run the app in development mode
make build    # debug build
make release  # build the macOS app bundle and DMG
make lint     # frontend typecheck/build + cargo fmt + clippy -D warnings
make clippy-review # extended Rust review lint pass
make clean    # remove build artifacts
```

If you prefer raw Cargo commands:

```bash
cargo build --manifest-path src-tauri/Cargo.toml
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```

## Install the App on macOS

To install a local build of the app instead of running `make dev`:

1. Build the release bundle:

```bash
make install
make release
```

2. Find the app bundle at:

```text
src-tauri/target/release/bundle/macos/Whispering.app
```

3. Move `Whispering.app` into `/Applications` if you want it installed like a normal Mac app.

4. Launch the app.

For local unsigned builds, macOS may block the first launch. If that happens, open it from Finder with `Open`, then confirm the security prompt.

Tauri also produces a DMG during `make release` under:

```text
src-tauri/target/release/bundle/dmg/
```

## First Run Checklist

On startup, Whispering requests:

- Accessibility access, so it can type text into the focused app
- Microphone access, so it can record audio

You need both permissions enabled in System Settings for the app to work correctly.

The app also expects a model in:

```text
~/.whispering/models/
```

The default model is:

```text
ggml-medium.bin
```

If you also install an English-only model such as `ggml-medium.en.bin`, place it in the same directory. The tray menu will list installed models automatically.

## How to Use It

After launch, Whispering runs as a menu bar app with no Dock icon.

- Press `Ctrl+Cmd+M` to start recording
- Press `Ctrl+Cmd+M` again to stop recording and transcribe
- The transcribed text is typed at the current cursor position
- During recording and transcription, a small status popover appears under the menu-bar icon.
- If text injection fails or focus was wrong, recover the last successful transcription with:

```bash
cat ~/Library/Caches/Whispering/transcripts/latest.txt
```

## Notes

- The default config is stored at `~/.whispering/config.toml`.
- Transcription recovery files are stored in `~/Library/Caches/Whispering/transcripts/`, so macOS can clean them when storage is needed.
