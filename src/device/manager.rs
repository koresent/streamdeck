use super::model::DeviceInfo;
use elgato_streamdeck::{new_hidapi, refresh_device_list, list_devices, info::Kind, StreamDeck, StreamDeckInput};
use image::DynamicImage;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub enum DeviceEvent {
    KeyPressed { deck_id: String, key: u8 },
    KeyReleased { deck_id: String, key: u8 },
    DeviceAttached(DeviceInfo),
    DeviceDetached(String),
}

pub enum DeviceCommand {
    SetImage {
        deck_id: String,
        key: u8,
        image: DynamicImage,
    },
    ClearImage {
        deck_id: String,
        key: u8,
    },
    SetBrightness {
        deck_id: String,
        brightness: u8,
    },
}

#[allow(dead_code)]
struct ConnectedDevice {
    pub info: DeviceInfo,
    pub deck: StreamDeck,
    pub last_activity: Instant,
    pub is_dimmed: bool,
    pub button_states: Vec<bool>,
}

pub struct StreamDeckManager {
    cmd_tx: Sender<DeviceCommand>,
    event_rx: Mutex<Receiver<DeviceEvent>>,
    known_devices: Arc<Mutex<Vec<DeviceInfo>>>,
}

impl StreamDeckManager {
    pub fn new() -> Self {
        let (cmd_tx, cmd_rx) = channel::<DeviceCommand>();
        let (event_tx, event_rx) = channel::<DeviceEvent>();
        let known_devices = Arc::new(Mutex::new(vec![
            DeviceInfo::virtual_original(),
            DeviceInfo::virtual_xl(),
            DeviceInfo::virtual_mini(),
        ]));

        let thread_devices = Arc::clone(&known_devices);

        thread::spawn(move || {
            Self::worker_loop(cmd_rx, event_tx, thread_devices);
        });

        Self {
            cmd_tx,
            event_rx: Mutex::new(event_rx),
            known_devices,
        }
    }

    pub fn get_devices(&self) -> Vec<DeviceInfo> {
        self.known_devices.lock().unwrap().clone()
    }

    pub fn try_recv_event(&self) -> Option<DeviceEvent> {
        let rx = self.event_rx.lock().unwrap();
        rx.try_recv().ok()
    }

    pub fn send_command(&self, cmd: DeviceCommand) {
        let _ = self.cmd_tx.send(cmd);
    }

    fn worker_loop(
        cmd_rx: Receiver<DeviceCommand>,
        event_tx: Sender<DeviceEvent>,
        shared_devices: Arc<Mutex<Vec<DeviceInfo>>>,
    ) {
        let mut hid = match new_hidapi() {
            Ok(h) => h,
            Err(e) => {
                tracing::warn!("Failed to initialize HID API: {:?}", e);
                return;
            }
        };

        let mut physical_decks: HashMap<String, ConnectedDevice> = HashMap::new();
        let mut last_scan = Instant::now() - Duration::from_secs(10);

        loop {
            while let Ok(cmd) = cmd_rx.try_recv() {
                match cmd {
                    DeviceCommand::SetImage { deck_id, key, image } => {
                        if let Some(dev) = physical_decks.get_mut(&deck_id) {
                            let _ = dev.deck.set_button_image(key, image);
                        }
                    }
                    DeviceCommand::ClearImage { deck_id, key } => {
                        if let Some(dev) = physical_decks.get_mut(&deck_id) {
                            let _ = dev.deck.clear_button_image(key);
                        }
                    }
                    DeviceCommand::SetBrightness { deck_id, brightness } => {
                        if let Some(dev) = physical_decks.get_mut(&deck_id) {
                            let _ = dev.deck.set_brightness(brightness);
                        }
                    }
                }
            }

            if last_scan.elapsed() >= Duration::from_millis(1500) {
                last_scan = Instant::now();
                let _ = refresh_device_list(&mut hid);
                let discovered = list_devices(&hid);

                for (kind, serial) in &discovered {
                    if !physical_decks.contains_key(serial) {
                        if let Ok(deck) = StreamDeck::connect(&hid, *kind, serial) {
                            let (rows, cols) = kind.key_layout();
                            let format = kind.key_image_format();
                            let (key_w, key_h) = (format.size.0 as u32, format.size.1 as u32);

                            let name = match kind {
                                Kind::Original => "Elgato Stream Deck V1".to_string(),
                                Kind::OriginalV2 => "Elgato Stream Deck MK.2".to_string(),
                                Kind::Mini => "Elgato Stream Deck Mini".to_string(),
                                Kind::Xl | Kind::XlV2 => "Elgato Stream Deck XL".to_string(),
                                Kind::Plus => "Elgato Stream Deck +".to_string(),
                                Kind::Pedal => "Elgato Stream Deck Pedal".to_string(),
                                _ => format!("Elgato Stream Deck ({:?})", kind),
                            };

                            let key_count = kind.key_count();
                            let dev_info = DeviceInfo {
                                id: serial.clone(),
                                name: format!("{} ({})", name, serial),
                                rows,
                                cols,
                                key_count,
                                key_width: key_w,
                                key_height: key_h,
                                is_virtual: false,
                            };

                            let _ = deck.set_brightness(100);
                            let _ = deck.clear_all_button_images();

                            {
                                let mut list = shared_devices.lock().unwrap();
                                list.retain(|d| d.id != *serial);
                                list.insert(0, dev_info.clone());
                            }

                            let _ = event_tx.send(DeviceEvent::DeviceAttached(dev_info.clone()));

                            physical_decks.insert(
                                serial.clone(),
                                ConnectedDevice {
                                    info: dev_info,
                                    deck,
                                    last_activity: Instant::now(),
                                    is_dimmed: false,
                                    button_states: vec![false; key_count as usize],
                                },
                            );
                        }
                    }
                }

                let active_serials: Vec<String> = discovered.iter().map(|(_, s)| s.clone()).collect();
                let mut detached = Vec::new();
                for (serial, _) in physical_decks.iter() {
                    if !active_serials.contains(serial) {
                        detached.push(serial.clone());
                    }
                }
                for serial in detached {
                    physical_decks.remove(&serial);
                    {
                        let mut list = shared_devices.lock().unwrap();
                        list.retain(|d| d.id != serial);
                    }
                    let _ = event_tx.send(DeviceEvent::DeviceDetached(serial));
                }
            }

            let disconnected: Vec<String> = Vec::new();
            for (serial, conn) in physical_decks.iter_mut() {
                match conn.deck.read_input(Some(Duration::from_millis(5))) {
                    Ok(input) => match input {
                        StreamDeckInput::ButtonStateChange(new_states) => {
                            conn.last_activity = Instant::now();
                            for (idx, &is_down) in new_states.iter().enumerate() {
                                let was_down = conn.button_states.get(idx).copied().unwrap_or(false);
                                if is_down != was_down {
                                    if is_down {
                                        let _ = event_tx.send(DeviceEvent::KeyPressed {
                                            deck_id: serial.clone(),
                                            key: idx as u8,
                                        });
                                    } else {
                                        let _ = event_tx.send(DeviceEvent::KeyReleased {
                                            deck_id: serial.clone(),
                                            key: idx as u8,
                                        });
                                    }
                                }
                            }
                            conn.button_states = new_states;
                        }
                        _ => {}
                    },
                    Err(_) => {}
                }
            }

            for serial in disconnected {
                physical_decks.remove(&serial);
            }

            thread::sleep(Duration::from_millis(5));
        }
    }
}
