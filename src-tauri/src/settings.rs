use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use crate::platform::RuntimeEnvironment;

pub const DEFAULT_LANGUAGE: &str = "en";
pub const DEFAULT_ENGLISH_MODEL_NAME: &str = "ggml-medium.en.bin";
pub const DEFAULT_MULTILINGUAL_MODEL_NAME: &str = "ggml-medium.bin";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub model_path: PathBuf,
    pub input_device: Option<String>,
    #[serde(default)]
    pub permission_prompts: PermissionPrompts,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PermissionPrompts {
    pub accessibility_requested: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledModel {
    pub path: PathBuf,
    pub filename: String,
    pub multilingual: bool,
}

impl InstalledModel {
    pub fn label(&self) -> String {
        let capability = if self.multilingual {
            "multilingual"
        } else {
            "english only"
        };
        format!("{} ({})", self.filename, capability)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model_path: PathBuf::from(DEFAULT_MULTILINGUAL_MODEL_NAME),
            input_device: None,
            permission_prompts: PermissionPrompts::default(),
        }
    }
}

pub fn default_english_model_path(environment: &RuntimeEnvironment) -> PathBuf {
    environment.model_dir.join(DEFAULT_ENGLISH_MODEL_NAME)
}

pub fn default_multilingual_model_path(environment: &RuntimeEnvironment) -> PathBuf {
    environment.model_dir.join(DEFAULT_MULTILINGUAL_MODEL_NAME)
}

pub fn is_multilingual_model_path(path: &Path) -> bool {
    path.file_name()
        .and_then(OsStr::to_str)
        .map(is_multilingual_model_name)
        .unwrap_or(false)
}

pub fn transcription_language(path: &Path) -> Option<&'static str> {
    if is_multilingual_model_path(path) {
        None
    } else {
        Some(DEFAULT_LANGUAGE)
    }
}

pub fn classify_model(path: PathBuf) -> Option<InstalledModel> {
    let filename = path.file_name()?.to_str()?.to_string();
    if path.extension() != Some(OsStr::new("bin")) {
        return None;
    }

    Some(InstalledModel {
        multilingual: is_multilingual_model_name(&filename),
        filename,
        path,
    })
}

pub fn installed_models(environment: &RuntimeEnvironment) -> Vec<InstalledModel> {
    let mut models = std::fs::read_dir(&environment.model_dir)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(|entry| entry.ok()))
        .filter_map(|entry| classify_model(entry.path()))
        .collect::<Vec<_>>();

    models.sort_by(|a, b| a.filename.cmp(&b.filename));
    models
}

fn is_multilingual_model_name(filename: &str) -> bool {
    !filename.ends_with(".en.bin")
}

pub fn load(environment: &RuntimeEnvironment) -> Config {
    let path = environment.config_path();
    let mut config = if !path.exists() {
        Config::default()
    } else {
        match std::fs::read_to_string(&path) {
            Ok(contents) => toml::from_str::<Config>(&contents).unwrap_or_default(),
            Err(_) => Config::default(),
        }
    };

    config.model_path = resolve_model_path(environment, &config.model_path);

    if !config.model_path.exists() {
        let multilingual = default_multilingual_model_path(environment);
        let english = default_english_model_path(environment);
        if multilingual.exists() {
            config.model_path = multilingual;
        } else if english.exists() {
            config.model_path = english;
        }
    }

    config
}

pub fn save(environment: &RuntimeEnvironment, config: &Config) -> Result<()> {
    let path = environment.config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let contents = toml::to_string_pretty(config)?;
    std::fs::write(path, contents)?;
    Ok(())
}

pub fn ensure_dirs(environment: &RuntimeEnvironment) -> Result<()> {
    std::fs::create_dir_all(&environment.model_dir)?;
    std::fs::create_dir_all(&environment.config_dir)?;
    Ok(())
}

fn resolve_model_path(environment: &RuntimeEnvironment, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        environment.model_dir.join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ensure_dirs, installed_models, load, save, Config, PermissionPrompts,
        DEFAULT_ENGLISH_MODEL_NAME, DEFAULT_MULTILINGUAL_MODEL_NAME,
    };
    use crate::platform::runtime::{
        Platform, PlatformCapabilities, PlatformHotkey, RuntimeEnvironment, StatusSurfaceMode,
        TraySupport,
    };
    use global_hotkey::hotkey::{Code, Modifiers};
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    struct TestEnvironment {
        root: PathBuf,
        runtime: RuntimeEnvironment,
    }

    impl TestEnvironment {
        fn new() -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be valid")
                .as_nanos();
            let root = std::env::temp_dir().join(format!("whispering-settings-test-{unique}"));
            let config_dir = root.join(".whispering");
            let model_dir = config_dir.join("models");
            let cache_dir = root.join("cache").join("Whispering");

            Self {
                root,
                runtime: RuntimeEnvironment {
                    platform: Platform::Linux,
                    arch: "x86_64",
                    model_dir,
                    cache_dir,
                    config_dir,
                    default_hotkey: PlatformHotkey::new(
                        Modifiers::CONTROL | Modifiers::ALT,
                        Code::KeyM,
                        "Ctrl+Alt+M",
                    ),
                    capabilities: PlatformCapabilities {
                        status_surface_mode: StatusSurfaceMode::FloatingWindow,
                        tray_support: TraySupport::Opportunistic,
                        supports_accessibility_prompt: false,
                        supports_microphone_prompt: false,
                        supports_system_sounds: false,
                    },
                },
            }
        }
    }

    impl Drop for TestEnvironment {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn ensure_dirs_creates_config_and_model_directories() {
        let test_environment = TestEnvironment::new();

        ensure_dirs(&test_environment.runtime).expect("directories should be created");

        assert!(test_environment.runtime.config_dir.is_dir());
        assert!(test_environment.runtime.model_dir.is_dir());
    }

    #[test]
    fn save_and_load_round_trip_resolves_relative_model_path() {
        let test_environment = TestEnvironment::new();
        ensure_dirs(&test_environment.runtime).expect("directories should be created");

        let config = Config {
            model_path: PathBuf::from(DEFAULT_ENGLISH_MODEL_NAME),
            input_device: Some("USB Microphone".to_string()),
            permission_prompts: PermissionPrompts {
                accessibility_requested: true,
            },
        };

        save(&test_environment.runtime, &config).expect("config should save");
        let loaded = load(&test_environment.runtime);

        assert_eq!(
            loaded.model_path,
            test_environment
                .runtime
                .model_dir
                .join(DEFAULT_ENGLISH_MODEL_NAME)
        );
        assert_eq!(loaded.input_device.as_deref(), Some("USB Microphone"));
        assert!(loaded.permission_prompts.accessibility_requested);
    }

    #[test]
    fn load_falls_back_to_existing_default_model_when_saved_model_is_missing() {
        let test_environment = TestEnvironment::new();
        ensure_dirs(&test_environment.runtime).expect("directories should be created");

        let multilingual_path = test_environment
            .runtime
            .model_dir
            .join(DEFAULT_MULTILINGUAL_MODEL_NAME);
        fs::write(&multilingual_path, b"model").expect("default model should exist");

        let config = Config {
            model_path: PathBuf::from("missing-model.bin"),
            input_device: None,
            permission_prompts: PermissionPrompts::default(),
        };
        save(&test_environment.runtime, &config).expect("config should save");

        let loaded = load(&test_environment.runtime);

        assert_eq!(loaded.model_path, multilingual_path);
    }

    #[test]
    fn installed_models_filters_non_bin_files_and_sorts_results() {
        let test_environment = TestEnvironment::new();
        ensure_dirs(&test_environment.runtime).expect("directories should be created");

        let files = [
            ("z-custom.bin", b"model".as_slice()),
            ("notes.txt", b"ignore".as_slice()),
            (DEFAULT_ENGLISH_MODEL_NAME, b"english".as_slice()),
            (DEFAULT_MULTILINGUAL_MODEL_NAME, b"multi".as_slice()),
        ];

        for (name, contents) in files {
            fs::write(test_environment.runtime.model_dir.join(name), contents)
                .expect("test fixture should write");
        }

        let models = installed_models(&test_environment.runtime);
        let filenames = models
            .iter()
            .map(|model| model.filename.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            filenames,
            vec![
                DEFAULT_MULTILINGUAL_MODEL_NAME,
                DEFAULT_ENGLISH_MODEL_NAME,
                "z-custom.bin",
            ]
        );
    }
}
