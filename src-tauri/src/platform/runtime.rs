use crate::settings::Config;
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use std::path::PathBuf;
use tauri::App;

use super::{linux::LinuxRuntime, macos::MacOsRuntime, windows::WindowsRuntime};

const APP_DIR_NAME: &str = ".whispering";
const APP_CACHE_DIR_NAME: &str = "Whispering";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    MacOs,
    Linux,
    Windows,
}

impl Platform {
    pub fn detect() -> Self {
        match std::env::consts::OS {
            "macos" => Self::MacOs,
            "linux" => Self::Linux,
            "windows" => Self::Windows,
            other => panic!("unsupported operating system: {other}"),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::MacOs => "macos",
            Self::Linux => "linux",
            Self::Windows => "windows",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusSurfaceMode {
    TrayPreferred,
    FloatingWindow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraySupport {
    Native,
    Opportunistic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlatformCapabilities {
    pub status_surface_mode: StatusSurfaceMode,
    pub tray_support: TraySupport,
    pub supports_accessibility_prompt: bool,
    pub supports_microphone_prompt: bool,
    pub supports_system_sounds: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlatformHotkey {
    modifiers: Modifiers,
    code: Code,
    label: &'static str,
}

impl PlatformHotkey {
    pub const fn new(modifiers: Modifiers, code: Code, label: &'static str) -> Self {
        Self {
            modifiers,
            code,
            label,
        }
    }

    pub fn hotkey(self) -> HotKey {
        HotKey::new(Some(self.modifiers), self.code)
    }

    pub fn label(self) -> &'static str {
        self.label
    }
}

#[derive(Clone, Debug)]
pub struct RuntimeEnvironment {
    pub platform: Platform,
    pub arch: &'static str,
    pub model_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub config_dir: PathBuf,
    pub default_hotkey: PlatformHotkey,
    pub capabilities: PlatformCapabilities,
}

impl RuntimeEnvironment {
    pub fn config_path(&self) -> PathBuf {
        self.config_dir.join("config.toml")
    }
}

#[derive(Clone, Debug)]
pub enum PlatformRuntime {
    MacOs(MacOsRuntime),
    Linux(LinuxRuntime),
    Windows(WindowsRuntime),
}

impl PlatformRuntime {
    pub fn detect() -> Self {
        match Platform::detect() {
            Platform::MacOs => Self::MacOs(MacOsRuntime::new(runtime_environment(Platform::MacOs))),
            Platform::Linux => Self::Linux(LinuxRuntime::new(runtime_environment(Platform::Linux))),
            Platform::Windows => {
                Self::Windows(WindowsRuntime::new(runtime_environment(Platform::Windows)))
            }
        }
    }

    pub fn environment(&self) -> &RuntimeEnvironment {
        match self {
            Self::MacOs(runtime) => runtime.environment(),
            Self::Linux(runtime) => runtime.environment(),
            Self::Windows(runtime) => runtime.environment(),
        }
    }

    pub fn configure_app(&self, app: &mut App) {
        match self {
            Self::MacOs(runtime) => runtime.configure_app(app),
            Self::Linux(runtime) => runtime.configure_app(app),
            Self::Windows(runtime) => runtime.configure_app(app),
        }
    }

    pub fn request_permissions(&self, config: &mut Config) {
        match self {
            Self::MacOs(runtime) => runtime.request_permissions(config),
            Self::Linux(runtime) => runtime.request_permissions(config),
            Self::Windows(runtime) => runtime.request_permissions(config),
        }
    }

    pub fn play_start_sound(&self) {
        match self {
            Self::MacOs(runtime) => runtime.play_start_sound(),
            Self::Linux(runtime) => runtime.play_start_sound(),
            Self::Windows(runtime) => runtime.play_start_sound(),
        }
    }

    pub fn play_stop_sound(&self) {
        match self {
            Self::MacOs(runtime) => runtime.play_stop_sound(),
            Self::Linux(runtime) => runtime.play_stop_sound(),
            Self::Windows(runtime) => runtime.play_stop_sound(),
        }
    }

    pub fn status_surface_mode(&self) -> StatusSurfaceMode {
        self.environment().capabilities.status_surface_mode
    }

    pub fn log_startup(&self) {
        let environment = self.environment();
        log::info!(
            "Detected platform={} arch={} hotkey={}",
            environment.platform.as_str(),
            environment.arch,
            environment.default_hotkey.label()
        );
        log::info!("Resolved config dir {}", environment.config_dir.display());
        log::info!("Resolved model dir {}", environment.model_dir.display());
        log::info!("Resolved cache dir {}", environment.cache_dir.display());
    }
}

fn runtime_environment(platform: Platform) -> RuntimeEnvironment {
    let home_dir = dirs_next::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let config_dir = home_dir.join(APP_DIR_NAME);
    let model_dir = config_dir.join("models");
    let cache_dir = dirs_next::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join(APP_CACHE_DIR_NAME);

    let (default_hotkey, capabilities) = match platform {
        Platform::MacOs => (
            PlatformHotkey::new(
                Modifiers::CONTROL | Modifiers::META,
                Code::KeyM,
                "Ctrl+Cmd+M",
            ),
            PlatformCapabilities {
                status_surface_mode: StatusSurfaceMode::TrayPreferred,
                tray_support: TraySupport::Native,
                supports_accessibility_prompt: true,
                supports_microphone_prompt: true,
                supports_system_sounds: true,
            },
        ),
        Platform::Linux => (
            PlatformHotkey::new(
                Modifiers::CONTROL | Modifiers::ALT,
                Code::KeyM,
                "Ctrl+Alt+M",
            ),
            PlatformCapabilities {
                status_surface_mode: StatusSurfaceMode::FloatingWindow,
                tray_support: TraySupport::Opportunistic,
                supports_accessibility_prompt: false,
                supports_microphone_prompt: false,
                supports_system_sounds: false,
            },
        ),
        Platform::Windows => (
            PlatformHotkey::new(
                Modifiers::CONTROL | Modifiers::ALT,
                Code::KeyM,
                "Ctrl+Alt+M",
            ),
            PlatformCapabilities {
                status_surface_mode: StatusSurfaceMode::TrayPreferred,
                tray_support: TraySupport::Native,
                supports_accessibility_prompt: false,
                supports_microphone_prompt: false,
                supports_system_sounds: false,
            },
        ),
    };

    RuntimeEnvironment {
        platform,
        arch: std::env::consts::ARCH,
        model_dir,
        cache_dir,
        config_dir,
        default_hotkey,
        capabilities,
    }
}

#[cfg(test)]
mod tests {
    use super::{runtime_environment, Platform, StatusSurfaceMode};

    #[test]
    fn macos_runtime_uses_menu_bar_style_surface() {
        let environment = runtime_environment(Platform::MacOs);
        assert_eq!(environment.platform, Platform::MacOs);
        assert_eq!(
            environment.capabilities.status_surface_mode,
            StatusSurfaceMode::TrayPreferred
        );
        assert_eq!(environment.default_hotkey.label(), "Ctrl+Cmd+M");
    }

    #[test]
    fn linux_runtime_uses_floating_status_surface() {
        let environment = runtime_environment(Platform::Linux);
        assert_eq!(environment.platform, Platform::Linux);
        assert_eq!(
            environment.capabilities.status_surface_mode,
            StatusSurfaceMode::FloatingWindow
        );
        assert_eq!(environment.default_hotkey.label(), "Ctrl+Alt+M");
    }

    #[test]
    fn runtime_environment_builds_expected_paths() {
        let environment = runtime_environment(Platform::Windows);
        assert!(environment.model_dir.ends_with(".whispering/models"));
        assert!(environment
            .config_path()
            .ends_with(".whispering/config.toml"));
        assert!(environment.cache_dir.ends_with("Whispering"));
    }
}
