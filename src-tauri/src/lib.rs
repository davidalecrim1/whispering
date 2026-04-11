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
use std::sync::{Arc, Mutex};
use tauri::{
    image::Image,
    menu::{CheckMenuItem, Menu, MenuItem, Submenu},
    tray::{TrayIconBuilder, TrayIconId},
    AppHandle, Manager,
};
use transcribe::Transcriber;

const TRAY_ID: &str = "whispering-tray";
const MENU_LANG_EN: &str = "lang-en";
const MENU_LANG_PT: &str = "lang-pt";
const MENU_TOGGLE: &str = "toggle-recording";

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
            // Request permissions upfront so the system dialogs appear on first launch
            permissions::request_accessibility();
            permissions::request_microphone();

            // Hide from Dock — menu bar only
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
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

            let manager = GlobalHotKeyManager::new()?;
            let hotkey = HotKey::new(
                Some(Modifiers::CONTROL | Modifiers::META),
                Code::KeyM,
            );
            manager.register(hotkey)?;
            app.manage(manager);

            let current_lang = {
                let cfg = state.config.lock().unwrap();
                cfg.language.clone()
            };
            build_tray(app.handle(), &current_lang, false)?;

            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let receiver = GlobalHotKeyEvent::receiver();
                loop {
                    if let Ok(event) = receiver.recv() {
                        // Only act on key-down, ignore key-up
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

fn build_tray(handle: &AppHandle, current_lang: &str, is_recording: bool) -> tauri::Result<()> {
    let toggle_label = if is_recording { "Stop Recording" } else { "Start Recording" };
    let toggle = MenuItem::with_id(handle, MENU_TOGGLE, toggle_label, true, None::<&str>)?;

    let lang_en = CheckMenuItem::with_id(
        handle, MENU_LANG_EN, "English", true, current_lang == "en", None::<&str>,
    )?;
    let lang_pt = CheckMenuItem::with_id(
        handle, MENU_LANG_PT, "Portuguese", true, current_lang == "pt", None::<&str>,
    )?;
    let lang_submenu = Submenu::with_items(handle, "Language", true, &[&lang_en, &lang_pt])?;
    let quit = MenuItem::with_id(handle, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(handle, &[&toggle, &lang_submenu, &quit])?;

    TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .icon(idle_icon())
        .on_menu_event(move |app, event| match event.id().as_ref() {
            MENU_TOGGLE => toggle_recording(app.clone()),
            MENU_LANG_EN => set_language(app, "en"),
            MENU_LANG_PT => set_language(app, "pt"),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(handle)?;

    Ok(())
}

fn rebuild_tray_menu(handle: &AppHandle, current_lang: &str, is_recording: bool) {
    let id = TrayIconId::new(TRAY_ID);
    if let Some(tray) = handle.tray_by_id(&id) {
        let toggle_label = if is_recording { "Stop Recording" } else { "Start Recording" };
        if let (Ok(toggle), Ok(lang_en), Ok(lang_pt), Ok(quit)) = (
            MenuItem::with_id(handle, MENU_TOGGLE, toggle_label, true, None::<&str>),
            CheckMenuItem::with_id(handle, MENU_LANG_EN, "English", true, current_lang == "en", None::<&str>),
            CheckMenuItem::with_id(handle, MENU_LANG_PT, "Portuguese", true, current_lang == "pt", None::<&str>),
            MenuItem::with_id(handle, "quit", "Quit", true, None::<&str>),
        ) {
            if let Ok(lang_submenu) = Submenu::with_items(handle, "Language", true, &[&lang_en, &lang_pt]) {
                if let Ok(menu) = Menu::with_items(handle, &[&toggle, &lang_submenu, &quit]) {
                    let _ = tray.set_menu(Some(menu));
                }
            }
        }
    }
}

fn set_language(handle: &AppHandle, lang: &str) {
    let state = handle.state::<Arc<WhisperingState>>();
    let is_recording = {
        let cfg = state.config.lock().unwrap();
        let _ = settings::save(&cfg);
        matches!(*state.recording.lock().unwrap(), RecordingState::Recording(_))
    };
    {
        let mut cfg = state.config.lock().unwrap();
        cfg.language = lang.to_string();
        settings::save(&cfg).ok();
    }
    rebuild_tray_menu(handle, lang, is_recording);
    log::info!("Language set to {}", lang);
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
                let lang = state.config.lock().unwrap().language.clone();
                rebuild_tray_menu(&handle, &lang, true);
                sounds::play_start();
                log::info!("Recording started");
            }
            Err(e) => log::error!("Failed to start recording: {}", e),
        }
    } else {
        let prev = std::mem::replace(&mut *recording, RecordingState::Idle);
        drop(recording);
        set_tray_icon(&handle, false);
        let lang = state.config.lock().unwrap().language.clone();
        rebuild_tray_menu(&handle, &lang, false);
        sounds::play_stop();
        log::info!("Recording stopped, transcribing...");

        if let RecordingState::Recording(capture) = prev {
            let state_clone = Arc::clone(&state);
            std::thread::spawn(move || {
                match capture.stop() {
                    Ok(audio) => {
                        let language = state_clone.config.lock().unwrap().language.clone();
                        let guard = state_clone.transcriber.lock().unwrap();
                        if let Some(t) = &*guard {
                            match t.transcribe(&audio, &language) {
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
