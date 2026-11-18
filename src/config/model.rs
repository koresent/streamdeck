use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ButtonConfig {
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default = "default_align")]
    pub text_vertical_align: String,
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub keys: String,
    #[serde(default)]
    pub write: String,
    #[serde(default)]
    pub brightness_change: i32,
    #[serde(default)]
    pub switch_page: u8,
    #[serde(default = "default_font_size")]
    pub font_size: u32,
}

fn default_align() -> String {
    "bottom".to_string()
}

fn default_font_size() -> u32 {
    14
}

impl ButtonConfig {
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
            && self.icon.is_empty()
            && self.command.is_empty()
            && self.keys.is_empty()
            && self.write.is_empty()
            && self.brightness_change == 0
            && self.switch_page == 0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeckState {
    #[serde(default)]
    pub page: u8,
    #[serde(default = "default_brightness")]
    pub brightness: u8,
    #[serde(default)]
    pub brightness_dimmed: u8,
    #[serde(default)]
    pub display_timeout: u32,
    #[serde(default)]
    pub buttons: HashMap<String, HashMap<String, ButtonConfig>>,
}

fn default_brightness() -> u8 {
    100
}

impl Default for DeckState {
    fn default() -> Self {
        Self {
            page: 0,
            brightness: 100,
            brightness_dimmed: 0,
            display_timeout: 0,
            buttons: HashMap::new(),
        }
    }
}

impl DeckState {
    pub fn get_button(&self, page: u8, button: u8) -> Option<&ButtonConfig> {
        let p_str = page.to_string();
        let b_str = button.to_string();
        self.buttons.get(&p_str)?.get(&b_str)
    }

    pub fn get_button_mut(&mut self, page: u8, button: u8) -> &mut ButtonConfig {
        let p_str = page.to_string();
        let b_str = button.to_string();
        self.buttons
            .entry(p_str)
            .or_default()
            .entry(b_str)
            .or_default()
    }

    pub fn set_button(&mut self, page: u8, button: u8, config: ButtonConfig) {
        let p_str = page.to_string();
        let b_str = button.to_string();
        self.buttons
            .entry(p_str)
            .or_default()
            .insert(b_str, config);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppConfig {
    #[serde(default = "default_version")]
    pub streamdeck_ui_version: u32,
    #[serde(default)]
    pub state: HashMap<String, DeckState>,
}

fn default_version() -> u32 {
    1
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            streamdeck_ui_version: 1,
            state: HashMap::new(),
        }
    }
}
