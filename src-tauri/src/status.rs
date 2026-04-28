use anyhow::Result;
use serde::Serialize;
use tauri::{
    tray::TrayIconId, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    Wry,
};

const OVERLAY_LABEL: &str = "status-overlay";
const TRAY_ID: &str = "whispering-tray";
const OVERLAY_WIDTH: f64 = 240.0;
const OVERLAY_HEIGHT: f64 = 74.0;
const MENU_BAR_MARGIN: f64 = 8.0;

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
    position_under_tray_icon(handle, &window);

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

fn position_under_tray_icon(handle: &tauri::AppHandle, window: &WebviewWindow<Wry>) {
    let Some(position) = tray_anchor_position(handle).or_else(|| primary_menu_bar_position(handle))
    else {
        return;
    };

    if let Err(err) = window.set_position(position) {
        log::error!("Failed to position status overlay: {}", err);
    }
}

fn tray_anchor_position(handle: &tauri::AppHandle) -> Option<PhysicalPosition<f64>> {
    let tray = handle.tray_by_id(&TrayIconId::new(TRAY_ID))?;
    let rect = tray.rect().ok()??;
    let position = rect.position.to_physical::<f64>(1.0);
    let size = rect.size.to_physical::<f64>(1.0);
    let center_x = position.x + size.width / 2.0;
    let x = (center_x - OVERLAY_WIDTH / 2.0).max(MENU_BAR_MARGIN);
    let y = position.y + size.height + MENU_BAR_MARGIN;

    Some(PhysicalPosition::new(x, y))
}

fn primary_menu_bar_position(handle: &tauri::AppHandle) -> Option<PhysicalPosition<f64>> {
    let monitor = handle.primary_monitor().ok()??;
    let origin = monitor.position();
    let size = monitor.size();
    let x = f64::from(origin.x) + f64::from(size.width) - OVERLAY_WIDTH - MENU_BAR_MARGIN;
    let y = f64::from(origin.y) + MENU_BAR_MARGIN;

    Some(PhysicalPosition::new(x, y))
}
