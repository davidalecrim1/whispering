use crate::{permissions, settings::Config, sounds};
use tauri::{ActivationPolicy, App};

use super::runtime::RuntimeEnvironment;

#[derive(Clone, Debug)]
pub struct MacOsRuntime {
    environment: RuntimeEnvironment,
}

impl MacOsRuntime {
    pub fn new(environment: RuntimeEnvironment) -> Self {
        Self { environment }
    }

    pub fn environment(&self) -> &RuntimeEnvironment {
        &self.environment
    }

    pub fn configure_app(&self, app: &mut App) {
        app.set_activation_policy(ActivationPolicy::Accessory);
    }

    pub fn request_permissions(&self, config: &mut Config) {
        permissions::request_accessibility(config);
        permissions::request_microphone();
    }

    pub fn play_start_sound(&self) {
        sounds::play_start();
    }

    pub fn play_stop_sound(&self) {
        sounds::play_stop();
    }
}
