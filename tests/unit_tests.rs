use streamdeck::config::{AppConfig, ButtonConfig, DeckState};
use streamdeck::device::DeviceInfo;
use streamdeck::input::wayland_uinput::{char_to_key_code, parse_key_code};
use streamdeck::render::ButtonRenderer;

#[test]
fn test_config_serialization() {
    let mut config = AppConfig::default();
    let mut deck = DeckState::default();

    let mut btn = ButtonConfig::default();
    btn.text = "Mute".to_string();
    btn.command = "wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle".to_string();
    btn.keys = "ctrl+alt+m".to_string();

    deck.set_button(0, 0, btn);
    config.state.insert("virtual-original".to_string(), deck);

    let json = serde_json::to_string(&config).expect("Serialization failed");
    let deserialized: AppConfig = serde_json::from_str(&json).expect("Deserialization failed");

    assert_eq!(config, deserialized);
    let loaded_btn = deserialized
        .state
        .get("virtual-original")
        .and_then(|d| d.get_button(0, 0))
        .expect("Button must exist");

    assert_eq!(loaded_btn.text, "Mute");
    assert_eq!(loaded_btn.command, "wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle");
    assert_eq!(loaded_btn.keys, "ctrl+alt+m");
}

#[test]
fn test_key_parsing() {
    use evdev::KeyCode;

    assert_eq!(parse_key_code("ctrl"), Some(KeyCode::KEY_LEFTCTRL));
    assert_eq!(parse_key_code("shift"), Some(KeyCode::KEY_LEFTSHIFT));
    assert_eq!(parse_key_code("alt"), Some(KeyCode::KEY_LEFTALT));
    assert_eq!(parse_key_code("super"), Some(KeyCode::KEY_LEFTMETA));
    assert_eq!(parse_key_code("enter"), Some(KeyCode::KEY_ENTER));
    assert_eq!(parse_key_code("space"), Some(KeyCode::KEY_SPACE));
    assert_eq!(parse_key_code("f13"), Some(KeyCode::KEY_F13));
    assert_eq!(parse_key_code("volumeup"), Some(KeyCode::KEY_VOLUMEUP));
    assert_eq!(parse_key_code("playpause"), Some(KeyCode::KEY_PLAYPAUSE));
}

#[test]
fn test_char_to_key_code() {
    use evdev::KeyCode;

    // Lowercase
    let (code, shift) = char_to_key_code('a').expect("Character 'a' must map");
    assert_eq!(code, KeyCode::KEY_A);
    assert!(!shift);

    // Uppercase
    let (code, shift) = char_to_key_code('Z').expect("Character 'Z' must map");
    assert_eq!(code, KeyCode::KEY_Z);
    assert!(shift);

    // Symbols
    let (code, shift) = char_to_key_code('!').expect("Character '!' must map");
    assert_eq!(code, KeyCode::KEY_1);
    assert!(shift);

    let (code, shift) = char_to_key_code(' ').expect("Character ' ' must map");
    assert_eq!(code, KeyCode::KEY_SPACE);
    assert!(!shift);
}

#[test]
fn test_button_renderer() {
    let renderer = ButtonRenderer::new().expect("Failed to initialize ButtonRenderer");
    let image = renderer.render_button(72, 72, "", "Test", "middle", 14, false);

    assert_eq!(image.width(), 72);
    assert_eq!(image.height(), 72);
}

#[test]
fn test_device_info_models() {
    let orig = DeviceInfo::virtual_original();
    assert_eq!(orig.rows, 3);
    assert_eq!(orig.cols, 5);
    assert_eq!(orig.key_count, 15);
    assert!(orig.is_virtual);

    let xl = DeviceInfo::virtual_xl();
    assert_eq!(xl.rows, 4);
    assert_eq!(xl.cols, 8);
    assert_eq!(xl.key_count, 32);

    let mini = DeviceInfo::virtual_mini();
    assert_eq!(mini.rows, 2);
    assert_eq!(mini.cols, 3);
    assert_eq!(mini.key_count, 6);
}
