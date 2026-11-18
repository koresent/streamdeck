#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub rows: u8,
    pub cols: u8,
    pub key_count: u8,
    pub key_width: u32,
    pub key_height: u32,
    pub is_virtual: bool,
}

impl DeviceInfo {
    pub fn virtual_original() -> Self {
        Self {
            id: "virtual-original".to_string(),
            name: "Virtual Stream Deck (3x5)".to_string(),
            rows: 3,
            cols: 5,
            key_count: 15,
            key_width: 72,
            key_height: 72,
            is_virtual: true,
        }
    }

    pub fn virtual_xl() -> Self {
        Self {
            id: "virtual-xl".to_string(),
            name: "Virtual Stream Deck XL (4x8)".to_string(),
            rows: 4,
            cols: 8,
            key_count: 32,
            key_width: 96,
            key_height: 96,
            is_virtual: true,
        }
    }

    pub fn virtual_mini() -> Self {
        Self {
            id: "virtual-mini".to_string(),
            name: "Virtual Stream Deck Mini (2x3)".to_string(),
            rows: 2,
            cols: 3,
            key_count: 6,
            key_width: 80,
            key_height: 80,
            is_virtual: true,
        }
    }
}
