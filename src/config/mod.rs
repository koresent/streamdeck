pub mod model;
pub mod storage;

pub use model::{AppConfig, ButtonConfig, DeckState};
pub use storage::{default_config_path, load_config, save_config};
