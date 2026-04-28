use anyhow::Result;
use serde::Serialize;
use tauri::{Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder, Wry};

const OVERLAY_LABEL: &str = "status-overlay";
const OVERLAY_WIDTH: f64 = 240.0;
const OVERLAY_HEIGHT: f64 = 74.0;

#[derive(Clone, Copy)]
pub enum OverlayKind {
    Mic,
    Spinner,
    Success,
    Error,
}

impl OverlayKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Mic => "mic",
            Self::Spinner => "spinner",
            Self::Success => "success",
            Self::Error => "error",
        }
    }
}

#[derive(Serialize)]
struct OverlayPayload<'a> {
    kind: &'static str,
    message: &'a str,
}

pub fn show(handle: &tauri::AppHandle, kind: OverlayKind, message: &str) {
    if let Err(err) = show_inner(handle, kind, message) {
        log::error!("Failed to show status overlay: {}", err);
    }
}

pub fn hide(handle: &tauri::AppHandle) {
    if let Some(window) = handle.get_webview_window(OVERLAY_LABEL) {
        if let Err(err) = window.hide() {
            log::error!("Failed to hide status overlay: {}", err);
        }
    }
}

fn show_inner(handle: &tauri::AppHandle, kind: OverlayKind, message: &str) -> Result<()> {
    let window = ensure_overlay(handle)?;
    position_near_cursor(handle, &window);

    let payload = OverlayPayload {
        kind: kind.as_str(),
        message,
    };
    let payload = serde_json::to_string(&payload)?;
    window.eval(format!(
        "window.setWhisperingStatus && window.setWhisperingStatus({});",
        payload
    ))?;
    window.show()?;

    Ok(())
}

fn ensure_overlay(handle: &tauri::AppHandle) -> Result<WebviewWindow<Wry>> {
    if let Some(window) = handle.get_webview_window(OVERLAY_LABEL) {
        return Ok(window);
    }

    let window =
        WebviewWindowBuilder::new(handle, OVERLAY_LABEL, WebviewUrl::App("index.html".into()))
            .title("Whispering Status")
            .inner_size(OVERLAY_WIDTH, OVERLAY_HEIGHT)
            .min_inner_size(OVERLAY_WIDTH, OVERLAY_HEIGHT)
            .max_inner_size(OVERLAY_WIDTH, OVERLAY_HEIGHT)
            .decorations(false)
            .resizable(false)
            .transparent(true)
            .always_on_top(true)
            .visible_on_all_workspaces(true)
            .skip_taskbar(true)
            .focusable(false)
            .focused(false)
            .visible(false)
            .build()?;

    Ok(window)
}

fn position_near_cursor(handle: &tauri::AppHandle, window: &WebviewWindow<Wry>) {
    let Ok(cursor) = handle.cursor_position() else {
        return;
    };

    let position = PhysicalPosition::new(cursor.x + 18.0, cursor.y + 22.0);
    if let Err(err) = window.set_position(position) {
        log::error!("Failed to position status overlay: {}", err);
    }
}
