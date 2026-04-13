mod audio;
mod inject;
mod permissions;
mod settings;
mod sounds;
mod transcribe;

use audio::AudioCapture;
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tauri::{
    image::Image,
    menu::{CheckMenuItem, IsMenuItem, Menu, MenuItem, Submenu},
    tray::{TrayIconBuilder, TrayIconId},
    AppHandle, Manager,
};
use transcribe::Transcriber;

const TRAY_ID: &str = "whispering-tray";
const MENU_TOGGLE: &str = "toggle-recording";
const MENU_QUIT: &str = "quit";
const MODEL_ID_PREFIX: &str = "model-select:";
const MODEL_INSTALL_PREFIX: &str = "model-install:";

// Use @2x (44px) assets — macOS renders them crisp on both Retina and non-Retina
static ICON_IDLE: &[u8] = include_bytes!("../icons/tray-idle@2x.png");
static ICON_RECORDING: &[u8] = include_bytes!("../icons/tray-recording@2x.png");

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
            permissions::request_accessibility();
            permissions::request_microphone();

            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            reload_transcriber(state.clone());

            let manager = GlobalHotKeyManager::new()?;
            let hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::META), Code::KeyM);
            manager.register(hotkey)?;
            app.manage(manager);

            let config = state.config.lock().unwrap().clone();
            build_tray(app.handle(), &config, false)?;

            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let receiver = GlobalHotKeyEvent::receiver();
                loop {
                    if let Ok(event) = receiver.recv() {
                        if event.state() == HotKeyState::Pressed {
                            toggle_recording(handle.clone());
                        }
                    }
                }
            });

            Ok(())
        })
        .plugin(tauri_plugin_log::Builder::default().build())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn build_tray(
    handle: &AppHandle,
    config: &settings::Config,
    is_recording: bool,
) -> tauri::Result<()> {
    let menu = build_menu(handle, config, is_recording)?;

    TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .icon(idle_icon())
        .on_menu_event(move |app, event| {
            let id = event.id().as_ref();
            if id == MENU_TOGGLE {
                toggle_recording(app.clone());
            } else if id == MENU_QUIT {
                graceful_shutdown(app);
            } else if let Some(path) = menu_model_path(id) {
                set_model(app, path);
            } else if let Some(name) = install_model_name(id) {
                log::warn!(
                    "Model {} is not installed. Use `make install` for {} or place it in ~/.whispering/models/",
                    name,
                    name
                );
            }
        })
        .build(handle)?;

    Ok(())
}

fn build_menu(
    handle: &AppHandle,
    config: &settings::Config,
    is_recording: bool,
) -> tauri::Result<Menu<tauri::Wry>> {
    let toggle_label = if is_recording {
        "Stop Recording"
    } else {
        "Start Recording"
    };
    let toggle = MenuItem::with_id(handle, MENU_TOGGLE, toggle_label, true, None::<&str>)?;

    let model_items = build_models_menu_items(handle, &config.model_path)?;
    let model_refs = model_items
        .iter()
        .map(|item| item as &dyn IsMenuItem<_>)
        .collect::<Vec<_>>();
    let models = Submenu::with_items(handle, "Models", true, &model_refs)?;

    let quit = MenuItem::with_id(handle, MENU_QUIT, "Quit", true, None::<&str>)?;
    Menu::with_items(handle, &[&toggle, &models, &quit])
}

fn build_models_menu_items(
    handle: &AppHandle,
    selected_path: &Path,
) -> tauri::Result<Vec<CheckMenuItem<tauri::Wry>>> {
    let installed_models = settings::installed_models();
    let mut items = Vec::new();

    for model in installed_models.iter() {
        let id = format!("{}{}", MODEL_ID_PREFIX, model.path.display());
        items.push(CheckMenuItem::with_id(
            handle,
            id,
            model.label(),
            true,
            model.path == selected_path,
            None::<&str>,
        )?);
    }

    if installed_models.is_empty() {
        items.push(CheckMenuItem::with_id(
            handle,
            "model-none",
            "No models installed",
            false,
            false,
            None::<&str>,
        )?);
    }

    for (name, path) in [
        (
            settings::DEFAULT_MULTILINGUAL_MODEL_NAME,
            settings::default_multilingual_model_path(),
        ),
        (
            settings::DEFAULT_ENGLISH_MODEL_NAME,
            settings::default_english_model_path(),
        ),
    ] {
        if path.exists() {
            continue;
        }

        let label = if settings::is_multilingual_model_path(&path) {
            format!("Download {} (multilingual)", name)
        } else {
            format!("Download {} (english only)", name)
        };
        let id = format!("{}{}", MODEL_INSTALL_PREFIX, name);
        items.push(CheckMenuItem::with_id(
            handle,
            id,
            label,
            true,
            false,
            None::<&str>,
        )?);
    }

    Ok(items)
}

fn menu_model_path(id: &str) -> Option<PathBuf> {
    id.strip_prefix(MODEL_ID_PREFIX).map(PathBuf::from)
}

fn install_model_name(id: &str) -> Option<&str> {
    id.strip_prefix(MODEL_INSTALL_PREFIX)
}

fn rebuild_tray_menu(handle: &AppHandle, config: &settings::Config, is_recording: bool) {
    let id = TrayIconId::new(TRAY_ID);
    if let Some(tray) = handle.tray_by_id(&id) {
        match build_menu(handle, config, is_recording) {
            Ok(menu) => {
                let _ = tray.set_menu(Some(menu));
            }
            Err(err) => log::error!("Failed to rebuild tray menu: {}", err),
        }
    }
}

fn reload_transcriber(state: Arc<WhisperingState>) {
    let model_path = {
        let cfg = state.config.lock().unwrap();
        cfg.model_path.clone()
    };

    *state.transcriber.lock().unwrap() = None;

    std::thread::spawn(move || match Transcriber::load(&model_path) {
        Ok(t) => {
            *state.transcriber.lock().unwrap() = Some(t);
            log::info!("Whisper model loaded from {}", model_path.display());
        }
        Err(e) => log::warn!("Could not load model {}: {}", model_path.display(), e),
    });
}

fn graceful_shutdown(handle: &AppHandle) {
    let state = handle.state::<Arc<WhisperingState>>();
    let mut recording = state.recording.lock().unwrap();

    if matches!(*recording, RecordingState::Recording(_)) {
        let prev = std::mem::replace(&mut *recording, RecordingState::Idle);
        drop(recording);
        sounds::play_stop();
        if let RecordingState::Recording(capture) = prev {
            drop(capture);
        }
    }

    handle.exit(0);
}

fn set_model(handle: &AppHandle, model_path: PathBuf) {
    let state = handle.state::<Arc<WhisperingState>>();
    let config = {
        let mut cfg = state.config.lock().unwrap();
        cfg.model_path = model_path;
        if let Err(err) = settings::save(&cfg) {
            log::error!("Failed to save model setting: {}", err);
        }
        cfg.clone()
    };

    reload_transcriber(Arc::clone(&state));
    let is_recording = matches!(
        *state.recording.lock().unwrap(),
        RecordingState::Recording(_)
    );
    rebuild_tray_menu(handle, &config, is_recording);
    log::info!("Model set to {}", config.model_path.display());
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
        let icon = if recording {
            recording_icon()
        } else {
            idle_icon()
        };
        let _ = tray.set_icon(Some(icon));
    }
}

fn toggle_recording(handle: AppHandle) {
    let state = handle.state::<Arc<WhisperingState>>();
    let mut recording = state.recording.lock().unwrap();

    if matches!(*recording, RecordingState::Idle) {
        let device = {
            let cfg = state.config.lock().unwrap();
            cfg.input_device.clone()
        };
        match AudioCapture::start(device.as_deref()) {
            Ok(capture) => {
                *recording = RecordingState::Recording(capture);
                drop(recording);
                set_tray_icon(&handle, true);
                let config = state.config.lock().unwrap().clone();
                rebuild_tray_menu(&handle, &config, true);
                sounds::play_start();
                log::info!("Recording started");
            }
            Err(e) => log::error!("Failed to start recording: {}", e),
        }
    } else {
        let prev = std::mem::replace(&mut *recording, RecordingState::Idle);
        drop(recording);
        set_tray_icon(&handle, false);
        let config = state.config.lock().unwrap().clone();
        rebuild_tray_menu(&handle, &config, false);
        sounds::play_stop();
        log::info!("Recording stopped, transcribing...");

        if let RecordingState::Recording(capture) = prev {
            let state_clone = Arc::clone(&state);
            std::thread::spawn(move || match capture.stop() {
                Ok(audio) => {
                    let language = {
                        let cfg = state_clone.config.lock().unwrap();
                        settings::transcription_language(&cfg.model_path)
                    };
                    let guard = state_clone.transcriber.lock().unwrap();
                    if let Some(t) = &*guard {
                        match t.transcribe(&audio, language) {
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
            });
        }
    }
}
