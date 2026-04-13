use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

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
            model_path: default_multilingual_model_path(),
            input_device: None,
            permission_prompts: PermissionPrompts::default(),
        }
    }
}

pub fn default_english_model_path() -> PathBuf {
    models_dir().join(DEFAULT_ENGLISH_MODEL_NAME)
}

pub fn default_multilingual_model_path() -> PathBuf {
    models_dir().join(DEFAULT_MULTILINGUAL_MODEL_NAME)
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

pub fn installed_models() -> Vec<InstalledModel> {
    let mut models = std::fs::read_dir(models_dir())
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

fn models_dir() -> PathBuf {
    dirs_next::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".whispering")
        .join("models")
}

fn config_path() -> PathBuf {
    dirs_next::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".whispering")
        .join("config.toml")
}

pub fn load() -> Config {
    let path = config_path();
    let mut config = if !path.exists() {
        Config::default()
    } else {
        match std::fs::read_to_string(&path) {
            Ok(contents) => toml::from_str::<Config>(&contents).unwrap_or_default(),
            Err(_) => Config::default(),
        }
    };

    if !config.model_path.exists() {
        if default_multilingual_model_path().exists() {
            config.model_path = default_multilingual_model_path();
        } else if default_english_model_path().exists() {
            config.model_path = default_english_model_path();
        }
    }

    config
}

pub fn save(config: &Config) -> Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let contents = toml::to_string_pretty(config)?;
    std::fs::write(path, contents)?;
    Ok(())
}

pub fn ensure_dirs() -> Result<()> {
    std::fs::create_dir_all(models_dir())?;
    Ok(())
}
