use crate::settings::Config;
use tauri::App;

use super::runtime::{RuntimeEnvironment, StatusSurfaceMode, TraySupport};

#[derive(Clone, Debug)]
pub struct LinuxRuntime {
    environment: RuntimeEnvironment,
}

impl LinuxRuntime {
    pub fn new(environment: RuntimeEnvironment) -> Self {
        Self { environment }
    }

    pub fn environment(&self) -> &RuntimeEnvironment {
        &self.environment
    }

    pub fn configure_app(&self, _app: &mut App) {
        let tray_support = match self.environment.capabilities.tray_support {
            TraySupport::Native => "native",
            TraySupport::Opportunistic => "opportunistic",
        };
        let status_surface = match self.environment.capabilities.status_surface_mode {
            StatusSurfaceMode::TrayPreferred => "tray preferred",
            StatusSurfaceMode::FloatingWindow => "floating window",
        };

        log::info!(
            "Configuring Linux runtime: tray_support={} status_surface={}",
            tray_support,
            status_surface
        );
    }

    pub fn request_permissions(&self, _config: &mut Config) {
        log::info!(
            "Linux runtime does not request desktop permissions automatically; microphone and input access remain desktop-environment managed"
        );
    }

    pub fn play_start_sound(&self) {
        self.log_sound_fallback("start");
    }

    pub fn play_stop_sound(&self) {
        self.log_sound_fallback("stop");
    }

    fn log_sound_fallback(&self, sound_name: &str) {
        log::debug!(
            "Linux {} sound is disabled; no cross-desktop sound backend is configured yet",
            sound_name
        );
    }
}
