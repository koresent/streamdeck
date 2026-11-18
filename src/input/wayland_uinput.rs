use anyhow::{Context, Result};
use evdev::{uinput::VirtualDevice, AttributeSet, EventType, InputEvent, KeyCode};
use std::sync::Mutex;
use std::thread::sleep;
use std::time::Duration;

pub struct WaylandInputInjector {
    device: Mutex<VirtualDevice>,
}

impl WaylandInputInjector {
    pub fn new() -> Result<Self> {
        let mut keys = AttributeSet::<KeyCode>::new();

        keys.insert(KeyCode::KEY_A);
        keys.insert(KeyCode::KEY_B);
        keys.insert(KeyCode::KEY_C);
        keys.insert(KeyCode::KEY_D);
        keys.insert(KeyCode::KEY_E);
        keys.insert(KeyCode::KEY_F);
        keys.insert(KeyCode::KEY_G);
        keys.insert(KeyCode::KEY_H);
        keys.insert(KeyCode::KEY_I);
        keys.insert(KeyCode::KEY_J);
        keys.insert(KeyCode::KEY_K);
        keys.insert(KeyCode::KEY_L);
        keys.insert(KeyCode::KEY_M);
        keys.insert(KeyCode::KEY_N);
        keys.insert(KeyCode::KEY_O);
        keys.insert(KeyCode::KEY_P);
        keys.insert(KeyCode::KEY_Q);
        keys.insert(KeyCode::KEY_R);
        keys.insert(KeyCode::KEY_S);
        keys.insert(KeyCode::KEY_T);
        keys.insert(KeyCode::KEY_U);
        keys.insert(KeyCode::KEY_V);
        keys.insert(KeyCode::KEY_W);
        keys.insert(KeyCode::KEY_X);
        keys.insert(KeyCode::KEY_Y);
        keys.insert(KeyCode::KEY_Z);

        keys.insert(KeyCode::KEY_0);
        keys.insert(KeyCode::KEY_1);
        keys.insert(KeyCode::KEY_2);
        keys.insert(KeyCode::KEY_3);
        keys.insert(KeyCode::KEY_4);
        keys.insert(KeyCode::KEY_5);
        keys.insert(KeyCode::KEY_6);
        keys.insert(KeyCode::KEY_7);
        keys.insert(KeyCode::KEY_8);
        keys.insert(KeyCode::KEY_9);

        keys.insert(KeyCode::KEY_LEFTCTRL);
        keys.insert(KeyCode::KEY_RIGHTCTRL);
        keys.insert(KeyCode::KEY_LEFTSHIFT);
        keys.insert(KeyCode::KEY_RIGHTSHIFT);
        keys.insert(KeyCode::KEY_LEFTALT);
        keys.insert(KeyCode::KEY_RIGHTALT);
        keys.insert(KeyCode::KEY_LEFTMETA);
        keys.insert(KeyCode::KEY_RIGHTMETA);

        keys.insert(KeyCode::KEY_F1);
        keys.insert(KeyCode::KEY_F2);
        keys.insert(KeyCode::KEY_F3);
        keys.insert(KeyCode::KEY_F4);
        keys.insert(KeyCode::KEY_F5);
        keys.insert(KeyCode::KEY_F6);
        keys.insert(KeyCode::KEY_F7);
        keys.insert(KeyCode::KEY_F8);
        keys.insert(KeyCode::KEY_F9);
        keys.insert(KeyCode::KEY_F10);
        keys.insert(KeyCode::KEY_F11);
        keys.insert(KeyCode::KEY_F12);
        keys.insert(KeyCode::KEY_F13);
        keys.insert(KeyCode::KEY_F14);
        keys.insert(KeyCode::KEY_F15);
        keys.insert(KeyCode::KEY_F16);
        keys.insert(KeyCode::KEY_F17);
        keys.insert(KeyCode::KEY_F18);
        keys.insert(KeyCode::KEY_F19);
        keys.insert(KeyCode::KEY_F20);
        keys.insert(KeyCode::KEY_F21);
        keys.insert(KeyCode::KEY_F22);
        keys.insert(KeyCode::KEY_F23);
        keys.insert(KeyCode::KEY_F24);

        keys.insert(KeyCode::KEY_ENTER);
        keys.insert(KeyCode::KEY_SPACE);
        keys.insert(KeyCode::KEY_TAB);
        keys.insert(KeyCode::KEY_BACKSPACE);
        keys.insert(KeyCode::KEY_ESC);
        keys.insert(KeyCode::KEY_DELETE);
        keys.insert(KeyCode::KEY_INSERT);
        keys.insert(KeyCode::KEY_HOME);
        keys.insert(KeyCode::KEY_END);
        keys.insert(KeyCode::KEY_PAGEUP);
        keys.insert(KeyCode::KEY_PAGEDOWN);
        keys.insert(KeyCode::KEY_UP);
        keys.insert(KeyCode::KEY_DOWN);
        keys.insert(KeyCode::KEY_LEFT);
        keys.insert(KeyCode::KEY_RIGHT);
        keys.insert(KeyCode::KEY_PRINT);

        keys.insert(KeyCode::KEY_MINUS);
        keys.insert(KeyCode::KEY_EQUAL);
        keys.insert(KeyCode::KEY_LEFTBRACE);
        keys.insert(KeyCode::KEY_RIGHTBRACE);
        keys.insert(KeyCode::KEY_SEMICOLON);
        keys.insert(KeyCode::KEY_APOSTROPHE);
        keys.insert(KeyCode::KEY_GRAVE);
        keys.insert(KeyCode::KEY_BACKSLASH);
        keys.insert(KeyCode::KEY_COMMA);
        keys.insert(KeyCode::KEY_DOT);
        keys.insert(KeyCode::KEY_SLASH);

        keys.insert(KeyCode::KEY_MUTE);
        keys.insert(KeyCode::KEY_VOLUMEUP);
        keys.insert(KeyCode::KEY_VOLUMEDOWN);
        keys.insert(KeyCode::KEY_PLAYPAUSE);
        keys.insert(KeyCode::KEY_NEXTSONG);
        keys.insert(KeyCode::KEY_PREVIOUSSONG);
        keys.insert(KeyCode::KEY_STOPCD);
        keys.insert(KeyCode::KEY_BRIGHTNESSUP);
        keys.insert(KeyCode::KEY_BRIGHTNESSDOWN);

        let device = VirtualDevice::builder()
            .context("Failed to build VirtualDeviceBuilder for /dev/uinput")?
            .name("StreamDeck Wayland Virtual Keyboard")
            .with_keys(&keys)
            .context("Failed to attach key attributes to virtual device")?
            .build()
            .context("Failed to register virtual device on /dev/uinput")?;

        Ok(Self {
            device: Mutex::new(device),
        })
    }

    pub fn press_key(&self, key: KeyCode) -> Result<()> {
        let mut dev = self.device.lock().unwrap();
        let ev = InputEvent::new_now(EventType::KEY.0, key.0, 1);
        dev.emit(&[ev])
            .with_context(|| format!("Failed to press key {:?}", key))?;
        Ok(())
    }

    pub fn release_key(&self, key: KeyCode) -> Result<()> {
        let mut dev = self.device.lock().unwrap();
        let ev = InputEvent::new_now(EventType::KEY.0, key.0, 0);
        dev.emit(&[ev])
            .with_context(|| format!("Failed to release key {:?}", key))?;
        Ok(())
    }

    pub fn send_key_click(&self, key: KeyCode) -> Result<()> {
        self.press_key(key)?;
        sleep(Duration::from_millis(15));
        self.release_key(key)?;
        Ok(())
    }

    pub fn send_combo(&self, keys: &[KeyCode]) -> Result<()> {
        if keys.is_empty() {
            return Ok(());
        }

        for key in keys {
            self.press_key(*key)?;
            sleep(Duration::from_millis(5));
        }

        sleep(Duration::from_millis(20));

        for key in keys.iter().rev() {
            self.release_key(*key)?;
            sleep(Duration::from_millis(5));
        }

        Ok(())
    }

    pub fn execute_hotkey_string(&self, hotkeys: &str) -> Result<()> {
        let trimmed = hotkeys.trim();
        if trimmed.is_empty() {
            return Ok(());
        }

        // Handle comma-separated sequence of combos or delays (streamdeck-ui compatibility)
        for section in trimmed.split(',') {
            let section = section.trim();
            if section.is_empty() {
                continue;
            }

            if let Some(delay_str) = section.strip_prefix("delay") {
                let ms = delay_str
                    .parse::<f64>()
                    .map(|s| (s * 1000.0) as u64)
                    .unwrap_or(500);
                sleep(Duration::from_millis(ms));
                continue;
            }

            let parts: Vec<&str> = section.split('+').map(|s| s.trim()).collect();
            let mut keys = Vec::new();
            for part in parts {
                if let Some(code) = parse_key_code(part) {
                    keys.push(code);
                } else {
                    tracing::warn!("Unrecognized key in combo: {}", part);
                }
            }

            self.send_combo(&keys)?;
            sleep(Duration::from_millis(10));
        }

        Ok(())
    }

    pub fn type_text(&self, text: &str) -> Result<()> {
        for ch in text.chars() {
            if let Some((code, shift)) = char_to_key_code(ch) {
                if shift {
                    self.press_key(KeyCode::KEY_LEFTSHIFT)?;
                    sleep(Duration::from_millis(5));
                }

                self.send_key_click(code)?;

                if shift {
                    self.release_key(KeyCode::KEY_LEFTSHIFT)?;
                    sleep(Duration::from_millis(5));
                }

                sleep(Duration::from_millis(8));
            } else {
                tracing::warn!("Character not supported for direct typing: {:?}", ch);
            }
        }
        Ok(())
    }
}

pub fn parse_key_code(s: &str) -> Option<KeyCode> {
    let lower = s.to_lowercase();
    match lower.as_str() {
        "ctrl" | "control" | "lctrl" => Some(KeyCode::KEY_LEFTCTRL),
        "rctrl" => Some(KeyCode::KEY_RIGHTCTRL),
        "shift" | "lshift" => Some(KeyCode::KEY_LEFTSHIFT),
        "rshift" => Some(KeyCode::KEY_RIGHTSHIFT),
        "alt" | "lalt" => Some(KeyCode::KEY_LEFTALT),
        "altgr" | "ralt" => Some(KeyCode::KEY_RIGHTALT),
        "super" | "win" | "windows" | "meta" | "cmd" | "logo" => Some(KeyCode::KEY_LEFTMETA),
        "enter" | "return" => Some(KeyCode::KEY_ENTER),
        "space" => Some(KeyCode::KEY_SPACE),
        "tab" => Some(KeyCode::KEY_TAB),
        "backspace" => Some(KeyCode::KEY_BACKSPACE),
        "esc" | "escape" => Some(KeyCode::KEY_ESC),
        "del" | "delete" => Some(KeyCode::KEY_DELETE),
        "ins" | "insert" => Some(KeyCode::KEY_INSERT),
        "home" => Some(KeyCode::KEY_HOME),
        "end" => Some(KeyCode::KEY_END),
        "pgup" | "pageup" => Some(KeyCode::KEY_PAGEUP),
        "pgdown" | "pagedown" => Some(KeyCode::KEY_PAGEDOWN),
        "up" => Some(KeyCode::KEY_UP),
        "down" => Some(KeyCode::KEY_DOWN),
        "left" => Some(KeyCode::KEY_LEFT),
        "right" => Some(KeyCode::KEY_RIGHT),
        "print" | "printscreen" | "prtsc" => Some(KeyCode::KEY_PRINT),

        "mute" | "volumemute" => Some(KeyCode::KEY_MUTE),
        "volumeup" | "volup" => Some(KeyCode::KEY_VOLUMEUP),
        "volumedown" | "voldown" => Some(KeyCode::KEY_VOLUMEDOWN),
        "playpause" | "play" | "pause" => Some(KeyCode::KEY_PLAYPAUSE),
        "next" | "nextsong" | "nexttrack" => Some(KeyCode::KEY_NEXTSONG),
        "prev" | "prevsong" | "previoussong" | "prevtrack" => Some(KeyCode::KEY_PREVIOUSSONG),
        "stop" => Some(KeyCode::KEY_STOPCD),
        "brightnessup" => Some(KeyCode::KEY_BRIGHTNESSUP),
        "brightnessdown" => Some(KeyCode::KEY_BRIGHTNESSDOWN),

        "f1" => Some(KeyCode::KEY_F1),
        "f2" => Some(KeyCode::KEY_F2),
        "f3" => Some(KeyCode::KEY_F3),
        "f4" => Some(KeyCode::KEY_F4),
        "f5" => Some(KeyCode::KEY_F5),
        "f6" => Some(KeyCode::KEY_F6),
        "f7" => Some(KeyCode::KEY_F7),
        "f8" => Some(KeyCode::KEY_F8),
        "f9" => Some(KeyCode::KEY_F9),
        "f10" => Some(KeyCode::KEY_F10),
        "f11" => Some(KeyCode::KEY_F11),
        "f12" => Some(KeyCode::KEY_F12),
        "f13" => Some(KeyCode::KEY_F13),
        "f14" => Some(KeyCode::KEY_F14),
        "f15" => Some(KeyCode::KEY_F15),
        "f16" => Some(KeyCode::KEY_F16),
        "f17" => Some(KeyCode::KEY_F17),
        "f18" => Some(KeyCode::KEY_F18),
        "f19" => Some(KeyCode::KEY_F19),
        "f20" => Some(KeyCode::KEY_F20),
        "f21" => Some(KeyCode::KEY_F21),
        "f22" => Some(KeyCode::KEY_F22),
        "f23" => Some(KeyCode::KEY_F23),
        "f24" => Some(KeyCode::KEY_F24),

        "a" => Some(KeyCode::KEY_A),
        "b" => Some(KeyCode::KEY_B),
        "c" => Some(KeyCode::KEY_C),
        "d" => Some(KeyCode::KEY_D),
        "e" => Some(KeyCode::KEY_E),
        "f" => Some(KeyCode::KEY_F),
        "g" => Some(KeyCode::KEY_G),
        "h" => Some(KeyCode::KEY_H),
        "i" => Some(KeyCode::KEY_I),
        "j" => Some(KeyCode::KEY_J),
        "k" => Some(KeyCode::KEY_K),
        "l" => Some(KeyCode::KEY_L),
        "m" => Some(KeyCode::KEY_M),
        "n" => Some(KeyCode::KEY_N),
        "o" => Some(KeyCode::KEY_O),
        "p" => Some(KeyCode::KEY_P),
        "q" => Some(KeyCode::KEY_Q),
        "r" => Some(KeyCode::KEY_R),
        "s" => Some(KeyCode::KEY_S),
        "t" => Some(KeyCode::KEY_T),
        "u" => Some(KeyCode::KEY_U),
        "v" => Some(KeyCode::KEY_V),
        "w" => Some(KeyCode::KEY_W),
        "x" => Some(KeyCode::KEY_X),
        "y" => Some(KeyCode::KEY_Y),
        "z" => Some(KeyCode::KEY_Z),

        "0" => Some(KeyCode::KEY_0),
        "1" => Some(KeyCode::KEY_1),
        "2" => Some(KeyCode::KEY_2),
        "3" => Some(KeyCode::KEY_3),
        "4" => Some(KeyCode::KEY_4),
        "5" => Some(KeyCode::KEY_5),
        "6" => Some(KeyCode::KEY_6),
        "7" => Some(KeyCode::KEY_7),
        "8" => Some(KeyCode::KEY_8),
        "9" => Some(KeyCode::KEY_9),

        "-" => Some(KeyCode::KEY_MINUS),
        "=" => Some(KeyCode::KEY_EQUAL),
        "[" => Some(KeyCode::KEY_LEFTBRACE),
        "]" => Some(KeyCode::KEY_RIGHTBRACE),
        ";" => Some(KeyCode::KEY_SEMICOLON),
        "'" => Some(KeyCode::KEY_APOSTROPHE),
        "`" => Some(KeyCode::KEY_GRAVE),
        "\\" => Some(KeyCode::KEY_BACKSLASH),
        "," => Some(KeyCode::KEY_COMMA),
        "." => Some(KeyCode::KEY_DOT),
        "/" => Some(KeyCode::KEY_SLASH),

        _ => None,
    }
}

pub fn char_to_key_code(ch: char) -> Option<(KeyCode, bool)> {
    match ch {
        'a'..='z' => {
            let offset = ch as u8 - b'a';
            let key = match offset {
                0 => KeyCode::KEY_A,
                1 => KeyCode::KEY_B,
                2 => KeyCode::KEY_C,
                3 => KeyCode::KEY_D,
                4 => KeyCode::KEY_E,
                5 => KeyCode::KEY_F,
                6 => KeyCode::KEY_G,
                7 => KeyCode::KEY_H,
                8 => KeyCode::KEY_I,
                9 => KeyCode::KEY_J,
                10 => KeyCode::KEY_K,
                11 => KeyCode::KEY_L,
                12 => KeyCode::KEY_M,
                13 => KeyCode::KEY_N,
                14 => KeyCode::KEY_O,
                15 => KeyCode::KEY_P,
                16 => KeyCode::KEY_Q,
                17 => KeyCode::KEY_R,
                18 => KeyCode::KEY_S,
                19 => KeyCode::KEY_T,
                20 => KeyCode::KEY_U,
                21 => KeyCode::KEY_V,
                22 => KeyCode::KEY_W,
                23 => KeyCode::KEY_X,
                24 => KeyCode::KEY_Y,
                25 => KeyCode::KEY_Z,
                _ => return None,
            };
            Some((key, false))
        }
        'A'..='Z' => {
            let offset = ch as u8 - b'A';
            let key = match offset {
                0 => KeyCode::KEY_A,
                1 => KeyCode::KEY_B,
                2 => KeyCode::KEY_C,
                3 => KeyCode::KEY_D,
                4 => KeyCode::KEY_E,
                5 => KeyCode::KEY_F,
                6 => KeyCode::KEY_G,
                7 => KeyCode::KEY_H,
                8 => KeyCode::KEY_I,
                9 => KeyCode::KEY_J,
                10 => KeyCode::KEY_K,
                11 => KeyCode::KEY_L,
                12 => KeyCode::KEY_M,
                13 => KeyCode::KEY_N,
                14 => KeyCode::KEY_O,
                15 => KeyCode::KEY_P,
                16 => KeyCode::KEY_Q,
                17 => KeyCode::KEY_R,
                18 => KeyCode::KEY_S,
                19 => KeyCode::KEY_T,
                20 => KeyCode::KEY_U,
                21 => KeyCode::KEY_V,
                22 => KeyCode::KEY_W,
                23 => KeyCode::KEY_X,
                24 => KeyCode::KEY_Y,
                25 => KeyCode::KEY_Z,
                _ => return None,
            };
            Some((key, true))
        }
        '0' => Some((KeyCode::KEY_0, false)),
        '1' => Some((KeyCode::KEY_1, false)),
        '2' => Some((KeyCode::KEY_2, false)),
        '3' => Some((KeyCode::KEY_3, false)),
        '4' => Some((KeyCode::KEY_4, false)),
        '5' => Some((KeyCode::KEY_5, false)),
        '6' => Some((KeyCode::KEY_6, false)),
        '7' => Some((KeyCode::KEY_7, false)),
        '8' => Some((KeyCode::KEY_8, false)),
        '9' => Some((KeyCode::KEY_9, false)),

        ' ' => Some((KeyCode::KEY_SPACE, false)),
        '\n' => Some((KeyCode::KEY_ENTER, false)),
        '\t' => Some((KeyCode::KEY_TAB, false)),

        '!' => Some((KeyCode::KEY_1, true)),
        '@' => Some((KeyCode::KEY_2, true)),
        '#' => Some((KeyCode::KEY_3, true)),
        '$' => Some((KeyCode::KEY_4, true)),
        '%' => Some((KeyCode::KEY_5, true)),
        '^' => Some((KeyCode::KEY_6, true)),
        '&' => Some((KeyCode::KEY_7, true)),
        '*' => Some((KeyCode::KEY_8, true)),
        '(' => Some((KeyCode::KEY_9, true)),
        ')' => Some((KeyCode::KEY_0, true)),

        '-' => Some((KeyCode::KEY_MINUS, false)),
        '_' => Some((KeyCode::KEY_MINUS, true)),
        '=' => Some((KeyCode::KEY_EQUAL, false)),
        '+' => Some((KeyCode::KEY_EQUAL, true)),
        '[' => Some((KeyCode::KEY_LEFTBRACE, false)),
        '{' => Some((KeyCode::KEY_LEFTBRACE, true)),
        ']' => Some((KeyCode::KEY_RIGHTBRACE, false)),
        '}' => Some((KeyCode::KEY_RIGHTBRACE, true)),
        ';' => Some((KeyCode::KEY_SEMICOLON, false)),
        ':' => Some((KeyCode::KEY_SEMICOLON, true)),
        '\'' => Some((KeyCode::KEY_APOSTROPHE, false)),
        '"' => Some((KeyCode::KEY_APOSTROPHE, true)),
        '`' => Some((KeyCode::KEY_GRAVE, false)),
        '~' => Some((KeyCode::KEY_GRAVE, true)),
        '\\' => Some((KeyCode::KEY_BACKSLASH, false)),
        '|' => Some((KeyCode::KEY_BACKSLASH, true)),
        ',' => Some((KeyCode::KEY_COMMA, false)),
        '<' => Some((KeyCode::KEY_COMMA, true)),
        '.' => Some((KeyCode::KEY_DOT, false)),
        '>' => Some((KeyCode::KEY_DOT, true)),
        '/' => Some((KeyCode::KEY_SLASH, false)),
        '?' => Some((KeyCode::KEY_SLASH, true)),

        _ => None,
    }
}
