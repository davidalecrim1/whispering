use anyhow::Result;
use serde::Serialize;
use tauri::{
    tray::TrayIconId, AppHandle, LogicalPosition, Manager, PhysicalPosition, PhysicalSize,
    Position, Rect, Size, WebviewUrl, WebviewWindow, WebviewWindowBuilder, Wry,
};

use crate::platform::StatusSurfaceMode;

const OVERLAY_LABEL: &str = "status-overlay";
const TRAY_ID: &str = "whispering-tray";
const OVERLAY_WIDTH: f64 = 252.0;
const OVERLAY_HEIGHT: f64 = 88.0;
const MENU_BAR_MARGIN: f64 = 8.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SessionAnchor {
    Tray,
}

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
    #[serde(skip_serializing_if = "Option::is_none")]
    level: Option<f32>,
}

pub fn show(
    handle: &AppHandle,
    surface_mode: StatusSurfaceMode,
    kind: OverlayKind,
    message: &str,
    anchor: Option<SessionAnchor>,
    level: Option<f32>,
) {
    if let Err(err) = show_inner(handle, surface_mode, kind, message, anchor, level) {
        log::error!("Failed to show status surface: {}", err);
    }
}

pub fn update(handle: &AppHandle, kind: OverlayKind, message: &str, level: Option<f32>) {
    if let Err(err) = update_inner(handle, kind, message, level) {
        log::error!("Failed to update status surface: {}", err);
    }
}

pub fn hide(handle: &AppHandle) {
    if let Some(window) = handle.get_webview_window(OVERLAY_LABEL) {
        if let Err(err) = window.hide() {
            log::error!("Failed to hide status surface: {}", err);
        }
    }
}

pub fn capture_session_anchor(surface_mode: StatusSurfaceMode) -> Option<SessionAnchor> {
    match surface_mode {
        StatusSurfaceMode::TrayPreferred => Some(SessionAnchor::Tray),
        StatusSurfaceMode::FloatingWindow => None,
    }
}

fn show_inner(
    handle: &AppHandle,
    surface_mode: StatusSurfaceMode,
    kind: OverlayKind,
    message: &str,
    anchor: Option<SessionAnchor>,
    level: Option<f32>,
) -> Result<()> {
    let window = ensure_overlay(handle)?;
    position_surface(handle, &window, surface_mode, anchor);
    dispatch_payload(&window, kind, message, level)?;
    window.show()?;

    Ok(())
}

fn update_inner(
    handle: &AppHandle,
    kind: OverlayKind,
    message: &str,
    level: Option<f32>,
) -> Result<()> {
    let Some(window) = handle.get_webview_window(OVERLAY_LABEL) else {
        return Ok(());
    };

    dispatch_payload(&window, kind, message, level)
}

fn dispatch_payload(
    window: &WebviewWindow<Wry>,
    kind: OverlayKind,
    message: &str,
    level: Option<f32>,
) -> Result<()> {
    let payload = OverlayPayload {
        kind: kind.as_str(),
        message,
        level,
    };
    let payload = serde_json::to_string(&payload)?;
    window.eval(format!(
        "window.setWhisperingStatus && window.setWhisperingStatus({});",
        payload
    ))?;

    Ok(())
}

fn ensure_overlay(handle: &AppHandle) -> Result<WebviewWindow<Wry>> {
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

fn position_surface(
    handle: &AppHandle,
    window: &WebviewWindow<Wry>,
    surface_mode: StatusSurfaceMode,
    anchor: Option<SessionAnchor>,
) {
    let Some(position) = surface_position(handle, surface_mode, anchor) else {
        return;
    };

    if let Err(err) = window.set_position(position) {
        log::error!("Failed to position status surface: {}", err);
    }
}

fn surface_position(
    handle: &AppHandle,
    surface_mode: StatusSurfaceMode,
    anchor: Option<SessionAnchor>,
) -> Option<PhysicalPosition<f64>> {
    match surface_mode {
        StatusSurfaceMode::TrayPreferred => match anchor.unwrap_or(SessionAnchor::Tray) {
            SessionAnchor::Tray => {
                tray_anchor_position(handle).or_else(|| floating_position(handle))
            }
        },
        StatusSurfaceMode::FloatingWindow => floating_position(handle),
    }
}

fn tray_anchor_position(handle: &AppHandle) -> Option<PhysicalPosition<f64>> {
    let tray = handle.tray_by_id(&TrayIconId::new(TRAY_ID))?;
    let rect = tray.rect().ok()??;
    let (position, size) = physical_rect(handle, rect)?;

    Some(tray_anchor_position_for_rect(position, size))
}

fn floating_position(handle: &AppHandle) -> Option<PhysicalPosition<f64>> {
    let monitor = handle.primary_monitor().ok()??;
    let origin = monitor.position();
    let size = monitor.size();
    let x = f64::from(origin.x) + f64::from(size.width) - OVERLAY_WIDTH - MENU_BAR_MARGIN;
    let y = f64::from(origin.y) + MENU_BAR_MARGIN;

    Some(PhysicalPosition::new(x, y))
}

fn tray_anchor_position_for_rect(
    position: PhysicalPosition<f64>,
    size: PhysicalSize<f64>,
) -> PhysicalPosition<f64> {
    let center_x = position.x + size.width / 2.0;
    let x = (center_x - OVERLAY_WIDTH / 2.0).max(MENU_BAR_MARGIN);
    let y = position.y + size.height + MENU_BAR_MARGIN;

    PhysicalPosition::new(x, y)
}

fn physical_rect(
    handle: &AppHandle,
    rect: Rect,
) -> Option<(PhysicalPosition<f64>, PhysicalSize<f64>)> {
    match (rect.position, rect.size) {
        (Position::Physical(position), Size::Physical(size)) => {
            Some((position.cast(), size.cast()))
        }
        (Position::Logical(position), Size::Logical(size)) => {
            let scale = tray_rect_scale_factor(handle, position)?;
            Some((position.to_physical(scale), size.to_physical(scale)))
        }
        (Position::Physical(position), Size::Logical(size)) => {
            let scale = tray_rect_scale_factor_for_physical(handle, position.cast())?;
            Some((position.cast(), size.to_physical(scale)))
        }
        (Position::Logical(position), Size::Physical(size)) => {
            let scale = tray_rect_scale_factor(handle, position)?;
            Some((position.to_physical(scale), size.cast()))
        }
    }
}

fn tray_rect_scale_factor(handle: &AppHandle, position: LogicalPosition<f64>) -> Option<f64> {
    let monitors = handle.available_monitors().ok()?;

    monitors
        .into_iter()
        .find(|monitor| logical_monitor_bounds(monitor).contains(position))
        .map(|monitor| monitor.scale_factor())
        .or_else(|| {
            handle
                .primary_monitor()
                .ok()
                .flatten()
                .map(|monitor| monitor.scale_factor())
        })
}

fn tray_rect_scale_factor_for_physical(
    handle: &AppHandle,
    position: PhysicalPosition<f64>,
) -> Option<f64> {
    let monitors = handle.available_monitors().ok()?;

    monitors
        .into_iter()
        .find(|monitor| physical_monitor_bounds(monitor).contains(position))
        .map(|monitor| monitor.scale_factor())
        .or_else(|| {
            handle
                .primary_monitor()
                .ok()
                .flatten()
                .map(|monitor| monitor.scale_factor())
        })
}

fn logical_monitor_bounds(monitor: &tauri::Monitor) -> LogicalMonitorBounds {
    let scale_factor = monitor.scale_factor();
    let position = monitor.position().to_logical::<f64>(scale_factor);
    let size = monitor.size().to_logical::<f64>(scale_factor);

    LogicalMonitorBounds {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
    }
}

fn physical_monitor_bounds(monitor: &tauri::Monitor) -> PhysicalMonitorBounds {
    let position = monitor.position();
    let size = monitor.size();

    PhysicalMonitorBounds {
        x: f64::from(position.x),
        y: f64::from(position.y),
        width: f64::from(size.width),
        height: f64::from(size.height),
    }
}

struct LogicalMonitorBounds {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl LogicalMonitorBounds {
    fn contains(self, point: LogicalPosition<f64>) -> bool {
        point.x >= self.x
            && point.x < self.x + self.width
            && point.y >= self.y
            && point.y < self.y + self.height
    }
}

struct PhysicalMonitorBounds {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl PhysicalMonitorBounds {
    fn contains(self, point: PhysicalPosition<f64>) -> bool {
        point.x >= self.x
            && point.x < self.x + self.width
            && point.y >= self.y
            && point.y < self.y + self.height
    }
}

#[cfg(test)]
mod tests {
    use super::{
        capture_session_anchor, tray_anchor_position_for_rect, MENU_BAR_MARGIN, OVERLAY_WIDTH,
    };
    use crate::platform::StatusSurfaceMode;
    use tauri::PhysicalPosition;
    use tauri::PhysicalSize;

    #[test]
    fn tray_anchor_uses_physical_rect_without_rescaling() {
        let position = tray_anchor_position_for_rect(
            PhysicalPosition::new(640.0, 12.0),
            PhysicalSize::new(24.0, 22.0),
        );

        assert_eq!(
            position,
            PhysicalPosition::new(
                640.0 + 12.0 - OVERLAY_WIDTH / 2.0,
                12.0 + 22.0 + MENU_BAR_MARGIN
            )
        );
    }

    #[test]
    fn floating_surfaces_do_not_capture_tray_anchor() {
        assert_eq!(
            capture_session_anchor(StatusSurfaceMode::FloatingWindow),
            None
        );
    }
}
