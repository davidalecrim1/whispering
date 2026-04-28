mod audio;
mod inject;
mod permissions;
mod settings;
mod sounds;
mod status;
mod transcribe;
mod transcripts;

use audio::AudioCapture;
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
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

enum ToggleAction {
    Start,
    Stop(AudioCapture),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RuntimePhase {
    Idle,
    LoadingModel,
    Recording,
    Transcribing,
    Typing,
    Error,
}

impl RuntimePhase {
    fn menu_label(self) -> &'static str {
        match self {
            Self::Idle => "Start Recording",
            Self::LoadingModel => "Loading Model...",
            Self::Recording => "Stop Recording",
            Self::Transcribing => "Transcribing...",
            Self::Typing => "Typing...",
            Self::Error => "Error",
        }
    }

    fn menu_enabled(self) -> bool {
        matches!(self, Self::Idle | Self::Recording | Self::Error)
    }

    fn models_enabled(self) -> bool {
        matches!(self, Self::Idle | Self::Error)
    }
}

struct RuntimeStatus {
    phase: RuntimePhase,
    last_error: Option<String>,
}

struct WhisperingState {
    recording: Mutex<RecordingState>,
    transcriber: Mutex<Option<Transcriber>>,
    config: Mutex<settings::Config>,
    status: Mutex<RuntimeStatus>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    settings::ensure_dirs().ok();
    let config = settings::load();

    let state = Arc::new(WhisperingState {
        recording: Mutex::new(RecordingState::Idle),
        transcriber: Mutex::new(None),
        config: Mutex::new(config),
        status: Mutex::new(RuntimeStatus {
            phase: RuntimePhase::LoadingModel,
            last_error: None,
        }),
    });

    tauri::Builder::default()
        .manage(state.clone())
        .setup(move |app| {
            {
                let mut cfg = state.config.lock().unwrap();
                permissions::request_accessibility(&mut cfg);
                if let Err(err) = settings::save(&cfg) {
                    log::error!("Failed to save permission prompt state: {}", err);
                }
            }
            permissions::request_microphone();

            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let manager = GlobalHotKeyManager::new()?;
            let hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::META), Code::KeyM);
            manager.register(hotkey)?;
            app.manage(manager);

            let config = state.config.lock().unwrap().clone();
            let runtime_status = state.status.lock().unwrap();
            build_tray(app.handle(), &config, &runtime_status)?;
            drop(runtime_status);

            status::show(app.handle(), status::OverlayKind::Spinner, "Loading model");
            reload_transcriber(app.handle().clone(), state.clone());

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
    status: &RuntimeStatus,
) -> tauri::Result<()> {
    let menu = build_menu(handle, config, status)?;

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
                report_error(
                    app,
                    format!(
                        "Model {} is not installed. Use `make install` for {} or place it in ~/.whispering/models/",
                    name,
                    name
                    ),
                );
            }
        })
        .build(handle)?;

    Ok(())
}

fn build_menu(
    handle: &AppHandle,
    config: &settings::Config,
    status: &RuntimeStatus,
) -> tauri::Result<Menu<tauri::Wry>> {
    let toggle = MenuItem::with_id(
        handle,
        MENU_TOGGLE,
        status.phase.menu_label(),
        status.phase.menu_enabled(),
        None::<&str>,
    )?;

    let model_items =
        build_models_menu_items(handle, &config.model_path, status.phase.models_enabled())?;
    let model_refs = model_items
        .iter()
        .map(|item| item as &dyn IsMenuItem<_>)
        .collect::<Vec<_>>();
    let models = Submenu::with_items(handle, "Models", status.phase.models_enabled(), &model_refs)?;

    let quit = MenuItem::with_id(handle, MENU_QUIT, "Quit", true, None::<&str>)?;
    let items = vec![&toggle as &dyn IsMenuItem<_>, &models, &quit];

    Menu::with_items(handle, &items)
}

fn build_models_menu_items(
    handle: &AppHandle,
    selected_path: &Path,
    enabled: bool,
) -> tauri::Result<Vec<CheckMenuItem<tauri::Wry>>> {
    let installed_models = settings::installed_models();
    let mut items = Vec::new();

    for model in installed_models.iter() {
        let id = format!("{}{}", MODEL_ID_PREFIX, model.path.display());
        items.push(CheckMenuItem::with_id(
            handle,
            id,
            model.label(),
            enabled,
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
            enabled,
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

fn rebuild_tray_menu(handle: &AppHandle) {
    let id = TrayIconId::new(TRAY_ID);
    if let Some(tray) = handle.tray_by_id(&id) {
        let state = handle.state::<Arc<WhisperingState>>();
        let config = state.config.lock().unwrap().clone();
        let status = state.status.lock().unwrap();
        match build_menu(handle, &config, &status) {
            Ok(menu) => {
                let _ = tray.set_menu(Some(menu));
            }
            Err(err) => log::error!("Failed to rebuild tray menu: {}", err),
        }
    }
}

fn reload_transcriber(handle: AppHandle, state: Arc<WhisperingState>) {
    let model_path = {
        let cfg = state.config.lock().unwrap();
        cfg.model_path.clone()
    };

    *state.transcriber.lock().unwrap() = None;
    set_phase(
        &handle,
        RuntimePhase::LoadingModel,
        Some((status::OverlayKind::Spinner, "Loading model")),
        false,
    );

    std::thread::spawn(move || match Transcriber::load(&model_path) {
        Ok(t) => {
            *state.transcriber.lock().unwrap() = Some(t);
            log::info!("Whisper model loaded from {}", model_path.display());
            set_phase(&handle, RuntimePhase::Idle, None, true);
        }
        Err(e) => report_error(
            &handle,
            format!("Could not load model {}: {}", model_path.display(), e),
        ),
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

    reload_transcriber(handle.clone(), Arc::clone(&state));
    rebuild_tray_menu(handle);
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
    match take_toggle_action(&state) {
        ToggleAction::Start => start_recording(&handle, &state),
        ToggleAction::Stop(capture) => stop_recording(&handle, &state, capture),
    }
}

fn take_toggle_action(state: &WhisperingState) -> ToggleAction {
    let mut recording = state.recording.lock().unwrap();
    match std::mem::replace(&mut *recording, RecordingState::Idle) {
        RecordingState::Idle => ToggleAction::Start,
        RecordingState::Recording(capture) => ToggleAction::Stop(capture),
    }
}

fn start_recording(handle: &AppHandle, state: &WhisperingState) {
    if let Some(message) = current_busy_message(state) {
        status::show(handle, status::OverlayKind::Spinner, message);
        return;
    }

    if state.transcriber.lock().unwrap().is_none() {
        report_error(handle, "Whisper model is not available");
        return;
    }

    let device = state.config.lock().unwrap().input_device.clone();
    match AudioCapture::start(device.as_deref()) {
        Ok(capture) => recording_started(handle, state, capture),
        Err(e) => report_error(handle, format!("Failed to start recording: {}", e)),
    }
}

fn recording_started(handle: &AppHandle, state: &WhisperingState, capture: AudioCapture) {
    *state.recording.lock().unwrap() = RecordingState::Recording(capture);
    set_tray_icon(handle, true);
    set_phase(
        handle,
        RuntimePhase::Recording,
        Some((status::OverlayKind::Mic, "Recording")),
        false,
    );
    sounds::play_start();
    log::info!("Recording started");
}

fn stop_recording(handle: &AppHandle, state: &Arc<WhisperingState>, capture: AudioCapture) {
    set_tray_icon(handle, false);
    set_phase(
        handle,
        RuntimePhase::Transcribing,
        Some((status::OverlayKind::Spinner, "Transcribing")),
        false,
    );
    sounds::play_stop();
    log::info!("Recording stopped, transcribing...");

    let handle = handle.clone();
    let state = Arc::clone(state);
    std::thread::spawn(move || process_recording(handle, state, capture));
}

fn process_recording(handle: AppHandle, state: Arc<WhisperingState>, capture: AudioCapture) {
    let audio = match capture.stop() {
        Ok(audio) => audio,
        Err(e) => {
            report_error(&handle, format!("Failed to stop audio capture: {}", e));
            return;
        }
    };

    transcribe_audio(&handle, &state, &audio);
}

fn transcribe_audio(handle: &AppHandle, state: &WhisperingState, audio: &[f32]) {
    let model_path = state.config.lock().unwrap().model_path.clone();
    let language = settings::transcription_language(&model_path);

    let result = {
        let guard = state.transcriber.lock().unwrap();
        let Some(transcriber) = &*guard else {
            report_error(
                handle,
                "Model not loaded yet; recording was not transcribed",
            );
            return;
        };
        transcriber.transcribe(audio, language)
    };

    handle_transcription_result(handle, result);
}

fn handle_transcription_result(handle: &AppHandle, result: anyhow::Result<String>) {
    match result {
        Ok(text) if !text.is_empty() => save_and_type_text(handle, &text),
        Ok(_) => report_error(handle, "No speech detected"),
        Err(e) => report_error(handle, format!("Transcription failed: {}", e)),
    }
}

fn save_and_type_text(handle: &AppHandle, text: &str) {
    let saved_path = save_transcript(text);

    set_phase(
        handle,
        RuntimePhase::Typing,
        Some((status::OverlayKind::Spinner, "Typing")),
        false,
    );

    match inject::type_text(text) {
        Ok(()) if saved_path.is_some() => show_success(handle, "Typed"),
        Ok(()) => report_error(handle, "Transcription was typed, but recovery save failed"),
        Err(e) => report_injection_error(handle, e, saved_path),
    }
}

fn save_transcript(text: &str) -> Option<PathBuf> {
    match transcripts::save(text) {
        Ok(path) => {
            log::info!("Saved transcript to {}", path.display());
            Some(path)
        }
        Err(e) => {
            log::error!("Failed to save transcript: {}", e);
            None
        }
    }
}

fn report_injection_error(handle: &AppHandle, error: anyhow::Error, saved_path: Option<PathBuf>) {
    let recovery = saved_path
        .map(|path| format!(" Transcript was saved to {}.", path.display()))
        .unwrap_or_default();

    report_error(
        handle,
        format!("Failed to inject text: {}.{}", error, recovery),
    );
}

fn current_busy_message(state: &WhisperingState) -> Option<&'static str> {
    busy_message(state.status.lock().unwrap().phase)
}

fn busy_message(phase: RuntimePhase) -> Option<&'static str> {
    match phase {
        RuntimePhase::LoadingModel => Some("Model is still loading"),
        RuntimePhase::Transcribing => Some("Transcribing"),
        RuntimePhase::Typing => Some("Typing"),
        RuntimePhase::Idle | RuntimePhase::Recording | RuntimePhase::Error => None,
    }
}

fn set_phase(
    handle: &AppHandle,
    phase: RuntimePhase,
    overlay: Option<(status::OverlayKind, &str)>,
    clear_error: bool,
) {
    let state = handle.state::<Arc<WhisperingState>>();
    {
        let mut runtime_status = state.status.lock().unwrap();
        runtime_status.phase = phase;
        if clear_error {
            runtime_status.last_error = None;
        }
    }

    rebuild_tray_menu(handle);
    match overlay {
        Some((kind, message)) => status::show(handle, kind, message),
        None => status::hide(handle),
    }
}

fn show_success(handle: &AppHandle, message: &str) {
    set_phase(
        handle,
        RuntimePhase::Idle,
        Some((status::OverlayKind::Success, message)),
        true,
    );

    let handle = handle.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(900));
        if handle
            .state::<Arc<WhisperingState>>()
            .status
            .lock()
            .unwrap()
            .phase
            == RuntimePhase::Idle
        {
            status::hide(&handle);
        }
    });
}

fn report_error(handle: &AppHandle, message: impl Into<String>) {
    let message = message.into();
    log::error!("{}", message);

    let state = handle.state::<Arc<WhisperingState>>();
    {
        let mut runtime_status = state.status.lock().unwrap();
        runtime_status.phase = RuntimePhase::Error;
        runtime_status.last_error = Some(message.clone());
    }

    rebuild_tray_menu(handle);
    status::show(handle, status::OverlayKind::Error, &short_message(&message));

    let handle = handle.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(6));
        if handle
            .state::<Arc<WhisperingState>>()
            .status
            .lock()
            .unwrap()
            .phase
            == RuntimePhase::Error
        {
            status::hide(&handle);
        }
    });
}

fn short_message(message: &str) -> String {
    const MAX_CHARS: usize = 72;
    let mut chars = message.chars();
    let short = chars.by_ref().take(MAX_CHARS).collect::<String>();
    if chars.next().is_some() {
        format!("{}...", short)
    } else {
        short
    }
}

#[cfg(test)]
mod tests {
    use super::{busy_message, short_message, RuntimePhase};

    #[test]
    fn busy_message_identifies_non_recordable_phases() {
        assert_eq!(
            busy_message(RuntimePhase::LoadingModel),
            Some("Model is still loading")
        );
        assert_eq!(
            busy_message(RuntimePhase::Transcribing),
            Some("Transcribing")
        );
        assert_eq!(busy_message(RuntimePhase::Typing), Some("Typing"));
    }

    #[test]
    fn busy_message_allows_idle_recording_and_error_phases() {
        assert_eq!(busy_message(RuntimePhase::Idle), None);
        assert_eq!(busy_message(RuntimePhase::Recording), None);
        assert_eq!(busy_message(RuntimePhase::Error), None);
    }

    #[test]
    fn short_message_preserves_short_text() {
        assert_eq!(short_message("model failed"), "model failed");
    }

    #[test]
    fn short_message_truncates_long_text() {
        let message = "a".repeat(73);
        assert_eq!(short_message(&message), format!("{}...", "a".repeat(72)));
    }
}
