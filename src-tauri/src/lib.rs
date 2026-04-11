mod audio;
mod inject;
mod settings;
mod transcribe;

use audio::AudioCapture;
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager,
};
use std::sync::{Arc, Mutex};
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconId},
    AppHandle, Manager,
};
use transcribe::Transcriber;

const TRAY_ID: &str = "whispering-tray";

// Embed icons at compile time so they work in both dev and release
static ICON_IDLE: &[u8] = include_bytes!("../icons/tray-idle.png");
static ICON_RECORDING: &[u8] = include_bytes!("../icons/tray-recording.png");

enum RecordingState {
    Idle,
    Recording(AudioCapture),
}

struct WhisperingState {
    recording: Mutex<RecordingState>,
    transcriber: Mutex<Option<Transcriber>>,
    config: Mutex<settings::Config>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    settings::ensure_dirs().ok();
    let config = settings::load();

    let state = Arc::new(WhisperingState {
        recording: Mutex::new(RecordingState::Idle),
        transcriber: Mutex::new(None),
        config: Mutex::new(config),
    });

    tauri::Builder::default()
        .manage(state.clone())
        .setup(move |app| {
            // Load model in background so startup is fast
            let state_clone = state.clone();
            let model_path = {
                let cfg = state_clone.config.lock().unwrap();
                cfg.model_path.clone()
            };
            std::thread::spawn(move || {
                match Transcriber::load(&model_path) {
                    Ok(t) => {
                        *state_clone.transcriber.lock().unwrap() = Some(t);
                        log::info!("Whisper model loaded");
                    }
                    Err(e) => log::warn!("Could not load model: {}", e),
                }
            });

            // Register global hotkey Ctrl+Cmd+M
            let manager = GlobalHotKeyManager::new()?;
            let hotkey = HotKey::new(
                Some(Modifiers::CONTROL | Modifiers::META),
                Code::KeyM,
            );
            manager.register(hotkey)?;
            // Keep manager alive for the duration of the app
            app.manage(manager);

            // Build system tray
            build_tray(app.handle())?;

            // Spawn hotkey event loop
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let receiver = GlobalHotKeyEvent::receiver();
                loop {
                    if receiver.recv().is_ok() {
                        toggle_recording(handle.clone());
                    }
                }
            });

            Ok(())
        })
        .plugin(tauri_plugin_log::Builder::default().build())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn build_tray(handle: &AppHandle) -> tauri::Result<()> {
    let quit = MenuItem::with_id(handle, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(handle, &[&quit])?;

    TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .icon(idle_icon())
        .on_menu_event(|app, event| {
            if event.id() == "quit" {
                app.exit(0);
            }
        })
        .build(handle)?;

    Ok(())
}

fn idle_icon() -> Image<'static> {
    Image::from_bytes(ICON_IDLE).expect("idle icon is valid PNG")
}

fn recording_icon() -> Image<'static> {
    Image::from_bytes(ICON_RECORDING).expect("recording icon is valid PNG")
}

fn set_tray_icon(handle: &AppHandle, recording: bool) {
    let id = TrayIconId::new(TRAY_ID);
    if let Some(tray) = handle.tray_by_id(&id) {
        let icon = if recording { recording_icon() } else { idle_icon() };
        let _ = tray.set_icon(Some(icon));
    }
}

fn toggle_recording(handle: AppHandle) {
    let state = handle.state::<Arc<WhisperingState>>();
    let mut recording = state.recording.lock().unwrap();

    // Determine current state and decide action, taking ownership if stopping
    let is_idle = matches!(*recording, RecordingState::Idle);

    if is_idle {
        let device = {
            let cfg = state.config.lock().unwrap();
            cfg.input_device.clone()
        };
        match AudioCapture::start(device.as_deref()) {
            Ok(capture) => {
                *recording = RecordingState::Recording(capture);
                drop(recording);
                set_tray_icon(&handle, true);
                log::info!("Recording started");
            }
            Err(e) => log::error!("Failed to start recording: {}", e),
        }
    } else {
        let prev = std::mem::replace(&mut *recording, RecordingState::Idle);
        drop(recording);
        set_tray_icon(&handle, false);
        log::info!("Recording stopped, transcribing...");

        if let RecordingState::Recording(capture) = prev {
            let state_clone = Arc::clone(&state);
            std::thread::spawn(move || {
                match capture.stop() {
                    Ok(audio) => {
                        let guard = state_clone.transcriber.lock().unwrap();
                        if let Some(t) = &*guard {
                            match t.transcribe(&audio) {
                                Ok(text) if !text.is_empty() => {
                                    drop(guard);
                                    if let Err(e) = inject::type_text(&text) {
                                        log::error!("Failed to inject text: {}", e);
                                    }
                                }
                                Ok(_) => log::info!("Transcription was empty"),
                                Err(e) => log::error!("Transcription failed: {}", e),
                            }
                        } else {
                            log::warn!("Model not loaded yet, discarding recording");
                        }
                    }
                    Err(e) => log::error!("Failed to stop audio capture: {}", e),
                }
            });
        }
    }
}
