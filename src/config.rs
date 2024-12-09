use serde::{Serialize, Deserialize};
use std::fs;
use std::path::Path;
use std::sync::Mutex;
use crate::MfdState;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub button_bindings: ButtonBindings,
    pub selected_mfd: MfdState,
    pub sound_enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ButtonBindings {
    pub up: (u32, u32),    // (device_id, button_code)
    pub right: (u32, u32),
    pub down: (u32, u32),
    pub left: (u32, u32),
}

impl Default for Config {
    fn default() -> Self {
        Config {
            button_bindings: ButtonBindings {
                up: (0, 0),    // Invalid binding
                right: (0, 0), // Invalid binding
                down: (0, 0),  // Invalid binding
                left: (0, 0),  // Invalid binding
            },
            selected_mfd: MfdState::LeftMfd,
            sound_enabled: true,
        }
    }
}

pub static CONFIG: Mutex<Option<Config>> = Mutex::new(None);

pub fn save_config(config: &Config) {
    let config_str = toml::to_string(config).unwrap();
    fs::write("superhat.cfg", config_str).expect("Failed to write config file");
}

pub fn load_config() -> Config {
    if Path::new("superhat.cfg").exists() {
        let config_str = fs::read_to_string("superhat.cfg").expect("Failed to read config file");
        toml::from_str(&config_str).unwrap_or_default()
    } else {
        Config::default()
    }
}

pub fn save_mfd_state(mfd: MfdState) {
    if let Ok(mut config_lock) = CONFIG.lock() {
        if let Some(config) = config_lock.as_mut() {
            config.selected_mfd = mfd;
            save_config(&config);
        }
    }
} 