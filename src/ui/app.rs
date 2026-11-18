use crate::config::{default_config_path, load_config, save_config, AppConfig, ButtonConfig};
use crate::device::{DeviceCommand, DeviceEvent, DeviceInfo, StreamDeckManager};
use crate::input::{execute_command, WaylandInputInjector};
use crate::render::ButtonRenderer;
use eframe::egui::{
    self, Align, Align2, Color32, ColorImage, Context, CornerRadius, Frame, Layout, Margin, Pos2, Rect,
    Sense, Stroke, TextureOptions, Ui, Vec2,
};
use image::DynamicImage;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub struct StreamDeckApp {
    config: AppConfig,
    config_path: PathBuf,
    config_dirty: bool,

    selected_deck_id: String,
    current_page: u8,
    selected_button: Option<u8>,

    clipboard_button: Option<ButtonConfig>,
    toast: Option<(String, Instant)>,

    manager: StreamDeckManager,
    input_injector: Option<WaylandInputInjector>,
    renderer: ButtonRenderer,

    texture_cache: HashMap<String, egui::TextureHandle>,
    textures_dirty: bool,

    active_presses: HashMap<u8, Instant>,
    settings_open: bool,
}

impl StreamDeckApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_app_style(&cc.egui_ctx);

        let config_path = default_config_path();
        let config = load_config(&config_path).unwrap_or_default();

        let manager = StreamDeckManager::new();
        let devices = manager.get_devices();
        let initial_deck_id = devices
            .first()
            .map(|d| d.id.clone())
            .unwrap_or_else(|| "virtual-original".to_string());

        let input_injector = match WaylandInputInjector::new() {
            Ok(inj) => {
                tracing::info!("Wayland uinput virtual keyboard initialized successfully");
                Some(inj)
            }
            Err(e) => {
                tracing::warn!("Could not initialize /dev/uinput: {:?}", e);
                None
            }
        };

        let renderer = ButtonRenderer::new().expect("Embedded font must be valid");

        let mut app = Self {
            config,
            config_path,
            config_dirty: false,
            selected_deck_id: initial_deck_id,
            current_page: 0,
            selected_button: None,
            clipboard_button: None,
            toast: None,
            manager,
            input_injector,
            renderer,
            texture_cache: HashMap::new(),
            textures_dirty: true,
            active_presses: HashMap::new(),
            settings_open: false,
        };

        app.sync_hardware_deck();
        app
    }

    fn current_device_info(&self) -> DeviceInfo {
        let devices = self.manager.get_devices();
        if let Some(dev) = devices.iter().find(|d| d.id == self.selected_deck_id) {
            dev.clone()
        } else {
            DeviceInfo::virtual_original()
        }
    }

    fn trigger_button_action(&mut self, key: u8) {
        let deck_id = self.selected_deck_id.clone();
        let page = self.current_page;

        let btn_opt = self
            .config
            .state
            .get(&deck_id)
            .and_then(|d| d.get_button(page, key))
            .cloned();

        self.active_presses.insert(key, Instant::now());

        if let Some(btn) = btn_opt {
            if btn.switch_page > 0 {
                let target_page = btn.switch_page - 1;
                self.current_page = target_page;
                self.textures_dirty = true;
                self.set_toast(format!("Switched to Page {}", target_page + 1));
                self.sync_hardware_deck();
                return;
            }

            if !btn.command.trim().is_empty() {
                if let Err(e) = execute_command(&btn.command) {
                    self.set_toast(format!("Command error: {}", e));
                } else {
                    self.set_toast(format!("Executed: {}", btn.command));
                }
            }

            if !btn.keys.trim().is_empty() {
                if let Some(inj) = &self.input_injector {
                    if let Err(e) = inj.execute_hotkey_string(&btn.keys) {
                        self.set_toast(format!("Hotkey error: {}", e));
                    } else {
                        self.set_toast(format!("Pressed keys: {}", btn.keys));
                    }
                } else {
                    self.set_toast("Wayland uinput device unavailable".to_string());
                }
            }

            if !btn.write.trim().is_empty() {
                if let Some(inj) = &self.input_injector {
                    if let Err(e) = inj.type_text(&btn.write) {
                        self.set_toast(format!("Typing error: {}", e));
                    } else {
                        self.set_toast("Typed text to active window".to_string());
                    }
                } else {
                    self.set_toast("Wayland uinput device unavailable".to_string());
                }
            }

            if btn.brightness_change != 0 {
                let deck_state = self.config.state.entry(deck_id.clone()).or_default();
                let new_brightness = (deck_state.brightness as i32 + btn.brightness_change)
                    .max(0)
                    .min(100) as u8;
                deck_state.brightness = new_brightness;
                self.config_dirty = true;
                self.manager.send_command(DeviceCommand::SetBrightness {
                    deck_id: deck_id.clone(),
                    brightness: new_brightness,
                });
                self.set_toast(format!("Brightness set to {}%", new_brightness));
            }
        }
    }

    fn set_toast(&mut self, msg: String) {
        self.toast = Some((msg, Instant::now()));
    }

    fn sync_hardware_deck(&mut self) {
        let dev = self.current_device_info();
        if dev.is_virtual {
            return;
        }

        let deck_id = dev.id.clone();
        let page = self.current_page;
        let count = dev.key_count;

        for key in 0..count {
            let (text, icon, align, font_size) = self
                .config
                .state
                .get(&deck_id)
                .and_then(|d| d.get_button(page, key))
                .map(|b| {
                    (
                        b.text.clone(),
                        b.icon.clone(),
                        b.text_vertical_align.clone(),
                        b.font_size,
                    )
                })
                .unwrap_or_default();

            let rgba = self.renderer.render_button(
                dev.key_width,
                dev.key_height,
                &icon,
                &text,
                &align,
                font_size,
                false,
            );

            self.manager.send_command(DeviceCommand::SetImage {
                deck_id: deck_id.clone(),
                key,
                image: DynamicImage::ImageRgba8(rgba),
            });
        }
    }

    fn save_current_config(&mut self) {
        if let Err(e) = save_config(&self.config, &self.config_path) {
            self.set_toast(format!("Failed to save configuration: {}", e));
        } else {
            self.config_dirty = false;
            self.set_toast("Configuration saved".to_string());
        }
    }

    fn update_button_texture(&mut self, ctx: &Context, key: u8) {
        let dev = self.current_device_info();
        let deck_id = self.selected_deck_id.clone();
        let page = self.current_page;

        let (text, icon, align, font_size) = self
            .config
            .state
            .get(&deck_id)
            .and_then(|d| d.get_button(page, key))
            .map(|b| {
                (
                    b.text.clone(),
                    b.icon.clone(),
                    b.text_vertical_align.clone(),
                    b.font_size,
                )
            })
            .unwrap_or_default();

        let is_pressed = self
            .active_presses
            .get(&key)
            .map(|t| t.elapsed() < Duration::from_millis(150))
            .unwrap_or(false);

        let rgba = self.renderer.render_button(
            dev.key_width,
            dev.key_height,
            &icon,
            &text,
            &align,
            font_size,
            is_pressed,
        );

        let (w, h) = (rgba.width() as usize, rgba.height() as usize);
        let color_image = ColorImage::from_rgba_unmultiplied([w, h], rgba.as_raw());

        let tex_id = format!("{}:{}:{}", deck_id, page, key);
        let handle = ctx.load_texture(&tex_id, color_image, TextureOptions::LINEAR);
        self.texture_cache.insert(tex_id, handle);

        if !dev.is_virtual {
            self.manager.send_command(DeviceCommand::SetImage {
                deck_id,
                key,
                image: DynamicImage::ImageRgba8(rgba),
            });
        }
    }
}

impl eframe::App for StreamDeckApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        while let Some(event) = self.manager.try_recv_event() {
            match event {
                DeviceEvent::KeyPressed { deck_id, key } => {
                    if deck_id == self.selected_deck_id {
                        self.trigger_button_action(key);
                        self.update_button_texture(ctx, key);
                    }
                }
                DeviceEvent::KeyReleased { deck_id: _, key: _ } => {
                    ctx.request_repaint();
                }
                DeviceEvent::DeviceAttached(info) => {
                    self.set_toast(format!("Attached: {}", info.name));
                    self.selected_deck_id = info.id;
                    self.textures_dirty = true;
                    self.sync_hardware_deck();
                }
                DeviceEvent::DeviceDetached(serial) => {
                    self.set_toast(format!("Disconnected: {}", serial));
                    if self.selected_deck_id == serial {
                        self.selected_deck_id = "virtual-original".to_string();
                        self.textures_dirty = true;
                    }
                }
            }
        }

        self.active_presses.retain(|_, time| time.elapsed() < Duration::from_millis(200));

        if self.textures_dirty {
            let dev = self.current_device_info();
            for k in 0..dev.key_count {
                self.update_button_texture(ctx, k);
            }
            self.textures_dirty = false;
        }

        egui::TopBottomPanel::top("top_panel")
            .frame(Frame::new().fill(Color32::from_rgb(22, 26, 33)).inner_margin(Margin::symmetric(14, 10)))
            .show(ctx, |ui| {
                self.render_top_bar(ui, ctx);
            });

        egui::TopBottomPanel::bottom("bottom_bar")
            .frame(Frame::new().fill(Color32::from_rgb(18, 22, 28)).inner_margin(Margin::symmetric(14, 8)))
            .show(ctx, |ui| {
                self.render_bottom_bar(ui);
            });

        egui::CentralPanel::default()
            .frame(Frame::new().fill(Color32::from_rgb(14, 17, 22)).inner_margin(Margin::same(16)))
            .show(ctx, |ui| {
                self.render_main_content(ui, ctx);
            });

        if self.settings_open {
            self.render_settings_dialog(ctx);
        }
    }
}

impl StreamDeckApp {
    fn render_top_bar(&mut self, ui: &mut Ui, _ctx: &Context) {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("STREAMDECK")
                    .strong()
                    .color(Color32::from_rgb(240, 243, 246))
                    .size(16.0),
            );

            let wayland_bg = Color32::from_rgb(33, 44, 62);
            let wayland_border = Color32::from_rgb(56, 120, 205);
            Frame::new()
                .fill(wayland_bg)
                .stroke(Stroke::new(1.0_f32, wayland_border))
                .corner_radius(CornerRadius::same(4))
                .inner_margin(Margin::symmetric(8, 3))
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("Wayland Native")
                            .size(11.0)
                            .color(Color32::from_rgb(120, 180, 255)),
                    );
                });

            ui.add_space(16.0);

            ui.label(egui::RichText::new("Device:").size(13.0).color(Color32::from_rgb(180, 190, 200)));
            let devices = self.manager.get_devices();
            let mut current_name = "Virtual Stream Deck (3x5)".to_string();
            if let Some(dev) = devices.iter().find(|d| d.id == self.selected_deck_id) {
                current_name = dev.name.clone();
            }

            egui::ComboBox::from_id_salt("device_select")
                .selected_text(current_name)
                .width(220.0)
                .show_ui(ui, |ui| {
                    for dev in &devices {
                        let selected = dev.id == self.selected_deck_id;
                        if ui.selectable_label(selected, &dev.name).clicked() {
                            self.selected_deck_id = dev.id.clone();
                            self.selected_button = None;
                            self.textures_dirty = true;
                            self.sync_hardware_deck();
                        }
                    }
                });

            ui.add_space(20.0);

            ui.label(egui::RichText::new("Page:").size(13.0).color(Color32::from_rgb(180, 190, 200)));
            for p in 0..10 {
                let is_current = self.current_page == p;
                let page_btn = egui::Button::new(format!("{}", p + 1))
                    .min_size(Vec2::new(26.0, 24.0))
                    .fill(if is_current {
                        Color32::from_rgb(46, 88, 160)
                    } else {
                        Color32::from_rgb(28, 34, 44)
                    })
                    .stroke(Stroke::new(
                        1.0_f32,
                        if is_current {
                            Color32::from_rgb(90, 150, 240)
                        } else {
                            Color32::from_rgb(45, 55, 70)
                        },
                    ));

                if ui.add(page_btn).clicked() {
                    self.current_page = p;
                    self.selected_button = None;
                    self.textures_dirty = true;
                    self.sync_hardware_deck();
                }
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add(
                        egui::Button::new("Settings")
                            .fill(Color32::from_rgb(32, 38, 50))
                            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(60, 72, 90))),
                    )
                    .clicked()
                {
                    self.settings_open = true;
                }

                let save_color = if self.config_dirty {
                    Color32::from_rgb(40, 140, 75)
                } else {
                    Color32::from_rgb(32, 38, 50)
                };

                if ui
                    .add(
                        egui::Button::new("Save Config")
                            .fill(save_color)
                            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(60, 72, 90))),
                    )
                    .clicked()
                {
                    self.save_current_config();
                }

                if self.config_dirty {
                    ui.label(
                        egui::RichText::new("● Unsaved")
                            .size(11.0)
                            .color(Color32::from_rgb(255, 180, 50)),
                    );
                }
            });
        });
    }

    fn render_bottom_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let uinput_ready = self.input_injector.is_some();
            let (status_text, status_color) = if uinput_ready {
                ("Input: /dev/uinput Ready", Color32::from_rgb(80, 200, 120))
            } else {
                ("Input: /dev/uinput Denied", Color32::from_rgb(240, 100, 80))
            };

            ui.label(egui::RichText::new(status_text).size(12.0).color(status_color));

            ui.separator();

            let dev = self.current_device_info();
            let mode_str = if dev.is_virtual {
                "Mode: Virtual Deck"
            } else {
                "Mode: Physical USB Connected"
            };
            ui.label(egui::RichText::new(mode_str).size(12.0).color(Color32::from_rgb(160, 175, 195)));

            if let Some((msg, time)) = &self.toast {
                if time.elapsed() < Duration::from_secs(4) {
                    ui.separator();
                    ui.label(
                        egui::RichText::new(msg)
                            .size(12.0)
                            .strong()
                            .color(Color32::from_rgb(255, 220, 100)),
                    );
                }
            }
        });
    }

    fn render_main_content(&mut self, ui: &mut Ui, ctx: &Context) {
        let available_size = ui.available_size();

        ui.horizontal(|ui| {
            let left_w = (available_size.x - 360.0).max(400.0);
            ui.vertical(|ui| {
                ui.set_width(left_w);
                self.render_grid(ui, ctx);
            });

            ui.add_space(14.0);

            ui.vertical(|ui| {
                ui.set_min_width(340.0);
                self.render_inspector(ui, ctx);
            });
        });
    }

    fn render_grid(&mut self, ui: &mut Ui, ctx: &Context) {
        let dev = self.current_device_info();
        let rows = dev.rows as usize;
        let cols = dev.cols as usize;

        Frame::new()
            .fill(Color32::from_rgb(22, 27, 34))
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(45, 54, 68)))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::same(20))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "{} - Page {}",
                            dev.name,
                            self.current_page + 1
                        ))
                        .size(14.0)
                        .color(Color32::from_rgb(180, 195, 215))
                        .strong(),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("Click a button to edit. Double-click or use Test to trigger action.")
                            .size(11.0)
                            .color(Color32::from_rgb(120, 135, 155)),
                    );
                    ui.add_space(16.0);

                    let btn_size = Vec2::new(76.0, 76.0);
                    let spacing = Vec2::new(12.0, 12.0);

                    egui::Grid::new("streamdeck_buttons_grid")
                        .spacing(spacing)
                        .show(ui, |ui| {
                            for r in 0..rows {
                                for c in 0..cols {
                                    let key_index = (r * cols + c) as u8;
                                    let is_selected = self.selected_button == Some(key_index);
                                    let is_pressed = self
                                        .active_presses
                                        .get(&key_index)
                                        .map(|t| t.elapsed() < Duration::from_millis(150))
                                        .unwrap_or(false);

                                    let tex_id = format!(
                                        "{}:{}:{}",
                                        self.selected_deck_id, self.current_page, key_index
                                    );

                                    let (rect, response) =
                                        ui.allocate_exact_size(btn_size, Sense::click());

                                    if let Some(tex) = self.texture_cache.get(&tex_id) {
                                        ui.painter().image(
                                            tex.id(),
                                            rect,
                                            Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                                            Color32::WHITE,
                                        );
                                    } else {
                                        ui.painter().rect_filled(
                                            rect,
                                            CornerRadius::same(6),
                                            Color32::from_rgb(26, 32, 42),
                                        );
                                    }

                                    let border_color = if is_pressed {
                                        Color32::from_rgb(255, 200, 50)
                                    } else if is_selected {
                                        Color32::from_rgb(60, 140, 255)
                                    } else {
                                        Color32::from_rgb(50, 60, 75)
                                    };

                                    let stroke_w = if is_selected || is_pressed { 2.5_f32 } else { 1.0_f32 };
                                    ui.painter().rect_stroke(
                                        rect,
                                        CornerRadius::same(6),
                                        Stroke::new(stroke_w, border_color),
                                        egui::StrokeKind::Outside,
                                    );

                                    let num_pos = rect.min + Vec2::new(5.0, 4.0);
                                    ui.painter().text(
                                        num_pos,
                                        Align2::LEFT_TOP,
                                        format!("{}", key_index + 1),
                                        egui::FontId::proportional(10.0),
                                        Color32::from_rgba_unmultiplied(200, 210, 225, 180),
                                    );

                                    if response.clicked() {
                                        self.selected_button = Some(key_index);
                                    }

                                    if response.double_clicked() {
                                        self.trigger_button_action(key_index);
                                        self.update_button_texture(ctx, key_index);
                                    }
                                }
                                ui.end_row();
                            }
                        });
                });
            });
    }

    fn render_inspector(&mut self, ui: &mut Ui, ctx: &Context) {
        Frame::new()
            .fill(Color32::from_rgb(22, 27, 34))
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(45, 54, 68)))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::same(16))
            .show(ui, |ui| {
                let Some(btn_idx) = self.selected_button else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(60.0);
                        ui.label(
                            egui::RichText::new("No Button Selected")
                                .size(15.0)
                                .color(Color32::from_rgb(160, 175, 195))
                                .strong(),
                        );
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new(
                                "Click any button on the Stream Deck grid to customize its icon, label, and action.",
                            )
                            .size(12.0)
                            .color(Color32::from_rgb(110, 125, 145)),
                        );
                    });
                    return;
                };

                let deck_id = self.selected_deck_id.clone();
                let page = self.current_page;

                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("Button #{}", btn_idx + 1))
                            .size(16.0)
                            .strong()
                            .color(Color32::from_rgb(240, 245, 250)),
                    );

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .button("Clear")
                            .on_hover_text("Reset button to default")
                            .clicked()
                        {
                            let deck_state = self.config.state.entry(deck_id.clone()).or_default();
                            deck_state.set_button(page, btn_idx, ButtonConfig::default());
                            self.config_dirty = true;
                            self.update_button_texture(ctx, btn_idx);
                            self.set_toast(format!("Cleared button #{}", btn_idx + 1));
                        }

                        if let Some(clip) = &self.clipboard_button {
                            if ui
                                .button("Paste")
                                .on_hover_text("Paste button configuration")
                                .clicked()
                            {
                                let deck_state = self.config.state.entry(deck_id.clone()).or_default();
                                deck_state.set_button(page, btn_idx, clip.clone());
                                self.config_dirty = true;
                                self.update_button_texture(ctx, btn_idx);
                                self.set_toast(format!("Pasted button #{}", btn_idx + 1));
                            }
                        }

                        if ui
                            .button("Copy")
                            .on_hover_text("Copy button configuration")
                            .clicked()
                        {
                            let current_btn = self
                                .config
                                .state
                                .get(&deck_id)
                                .and_then(|d| d.get_button(page, btn_idx))
                                .cloned()
                                .unwrap_or_default();
                            self.clipboard_button = Some(current_btn);
                            self.set_toast(format!("Copied button #{}", btn_idx + 1));
                        }
                    });
                });

                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut btn_config = self
                        .config
                        .state
                        .get(&deck_id)
                        .and_then(|d| d.get_button(page, btn_idx))
                        .cloned()
                        .unwrap_or_default();

                    let mut modified = false;

                    ui.label(
                        egui::RichText::new("ICON")
                            .size(12.0)
                            .strong()
                            .color(Color32::from_rgb(130, 160, 200)),
                    );

                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(if btn_config.icon.is_empty() {
                                "None"
                            } else {
                                btn_config.icon.split('/').last().unwrap_or(&btn_config.icon)
                            })
                            .size(12.0)
                            .color(Color32::from_rgb(180, 190, 205)),
                        );

                        if ui.button("Browse...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Images", &["png", "jpg", "jpeg", "svg", "webp", "gif"])
                                .pick_file()
                            {
                                btn_config.icon = path.to_string_lossy().to_string();
                                modified = true;
                            }
                        }

                        if !btn_config.icon.is_empty() && ui.button("Remove").clicked() {
                            btn_config.icon.clear();
                            modified = true;
                        }
                    });

                    ui.add_space(10.0);

                    ui.label(
                        egui::RichText::new("LABEL")
                            .size(12.0)
                            .strong()
                            .color(Color32::from_rgb(130, 160, 200)),
                    );

                    ui.horizontal(|ui| {
                        ui.label("Text:");
                        if ui.text_edit_singleline(&mut btn_config.text).changed() {
                            modified = true;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Align:");
                        let mut align = btn_config.text_vertical_align.clone();
                        egui::ComboBox::from_id_salt("text_align_combo")
                            .selected_text(&align)
                            .show_ui(ui, |ui| {
                                for opt in &["top", "middle-top", "middle", "middle-bottom", "bottom"] {
                                    if ui.selectable_value(&mut align, opt.to_string(), *opt).clicked() {
                                        btn_config.text_vertical_align = align.clone();
                                        modified = true;
                                    }
                                }
                            });

                        ui.add_space(8.0);
                        ui.label("Size:");
                        if ui
                            .add(egui::Slider::new(&mut btn_config.font_size, 8..=28))
                            .changed()
                        {
                            modified = true;
                        }
                    });

                    ui.add_space(14.0);
                    ui.separator();

                    ui.label(
                        egui::RichText::new("ACTIONS")
                            .size(12.0)
                            .strong()
                            .color(Color32::from_rgb(130, 160, 200)),
                    );

                    ui.label(
                        egui::RichText::new("Run Shell Command:")
                            .size(11.0)
                            .color(Color32::from_rgb(160, 175, 195)),
                    );
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut btn_config.command)
                                .hint_text("e.g. alacritty -e btop or pkill -USR1 waybar")
                                .desired_width(f32::INFINITY),
                        )
                        .changed()
                    {
                        modified = true;
                    }

                    ui.add_space(8.0);

                    ui.label(
                        egui::RichText::new("Simulate Hotkey Combo:")
                            .size(11.0)
                            .color(Color32::from_rgb(160, 175, 195)),
                    );
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut btn_config.keys)
                                .hint_text("e.g. ctrl+alt+t or super+enter or volumeup")
                                .desired_width(f32::INFINITY),
                        )
                        .changed()
                    {
                        modified = true;
                    }

                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new("Add:").size(10.0));
                        for mod_key in &["Ctrl+", "Shift+", "Alt+", "Super+", "Enter", "Space", "Tab"] {
                            if ui.small_button(*mod_key).clicked() {
                                btn_config.keys.push_str(&mod_key.to_lowercase());
                                modified = true;
                            }
                        }
                    });

                    ui.add_space(8.0);

                    ui.label(
                        egui::RichText::new("Type Text to Focused App:")
                            .size(11.0)
                            .color(Color32::from_rgb(160, 175, 195)),
                    );
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut btn_config.write)
                                .hint_text("String typed via kernel uinput")
                                .desired_width(f32::INFINITY),
                        )
                        .changed()
                    {
                        modified = true;
                    }

                    ui.add_space(8.0);

                    ui.label(
                        egui::RichText::new("Switch to Page:")
                            .size(11.0)
                            .color(Color32::from_rgb(160, 175, 195)),
                    );
                    let mut page_sel = btn_config.switch_page;
                    egui::ComboBox::from_id_salt("switch_page_combo")
                        .selected_text(if page_sel == 0 {
                            "None (stay on page)".to_string()
                        } else {
                            format!("Page {}", page_sel)
                        })
                        .show_ui(ui, |ui| {
                            if ui.selectable_value(&mut page_sel, 0, "None").clicked() {
                                btn_config.switch_page = 0;
                                modified = true;
                            }
                            for p in 1..=10 {
                                if ui
                                    .selectable_value(&mut page_sel, p, format!("Page {}", p))
                                    .clicked()
                                {
                                    btn_config.switch_page = p;
                                    modified = true;
                                }
                            }
                        });

                    ui.add_space(8.0);

                    ui.label(
                        egui::RichText::new("Adjust Brightness Step:")
                            .size(11.0)
                            .color(Color32::from_rgb(160, 175, 195)),
                    );
                    ui.horizontal(|ui| {
                        if ui.button("-10%").clicked() {
                            btn_config.brightness_change = -10;
                            modified = true;
                        }
                        if ui.button("+10%").clicked() {
                            btn_config.brightness_change = 10;
                            modified = true;
                        }
                        if ui.button("Reset").clicked() {
                            btn_config.brightness_change = 0;
                            modified = true;
                        }
                        if btn_config.brightness_change != 0 {
                            ui.label(format!("{:+}%", btn_config.brightness_change));
                        }
                    });

                    ui.add_space(16.0);
                    ui.separator();

                    ui.vertical_centered(|ui| {
                        if ui
                            .add(
                                egui::Button::new("Test Button Action")
                                    .min_size(Vec2::new(180.0, 32.0))
                                    .fill(Color32::from_rgb(38, 75, 130))
                                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(70, 120, 200))),
                            )
                            .clicked()
                        {
                            self.trigger_button_action(btn_idx);
                            self.update_button_texture(ctx, btn_idx);
                        }
                    });

                    if modified {
                        let deck_state = self.config.state.entry(deck_id).or_default();
                        deck_state.set_button(page, btn_idx, btn_config);
                        self.config_dirty = true;
                        self.update_button_texture(ctx, btn_idx);
                    }
                });
            });
    }

    fn render_settings_dialog(&mut self, ctx: &Context) {
        let mut open = self.settings_open;
        let deck_id = self.selected_deck_id.clone();

        egui::Window::new("Device Settings")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .frame(
                Frame::new()
                    .fill(Color32::from_rgb(24, 29, 38))
                    .stroke(Stroke::new(1.5_f32, Color32::from_rgb(60, 75, 95)))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(Margin::same(20)),
            )
            .show(ctx, |ui| {
                ui.set_width(380.0);

                ui.heading("Stream Deck Settings");
                ui.add_space(8.0);

                let deck_state = self.config.state.entry(deck_id.clone()).or_default();

                ui.label(
                    egui::RichText::new("Display Brightness:")
                        .size(13.0)
                        .color(Color32::from_rgb(200, 210, 225)),
                );
                let mut brightness = deck_state.brightness;
                if ui
                    .add(egui::Slider::new(&mut brightness, 0..=100).suffix("%"))
                    .changed()
                {
                    deck_state.brightness = brightness;
                    self.config_dirty = true;
                    self.manager.send_command(DeviceCommand::SetBrightness {
                        deck_id: deck_id.clone(),
                        brightness,
                    });
                }

                ui.add_space(10.0);

                ui.label(
                    egui::RichText::new("Dimmed Brightness:")
                        .size(13.0)
                        .color(Color32::from_rgb(200, 210, 225)),
                );
                let mut dimmed = deck_state.brightness_dimmed;
                if ui
                    .add(egui::Slider::new(&mut dimmed, 0..=100).suffix("%"))
                    .changed()
                {
                    deck_state.brightness_dimmed = dimmed;
                    self.config_dirty = true;
                }

                ui.add_space(10.0);

                ui.label(
                    egui::RichText::new("Inactivity Dimming Timeout:")
                        .size(13.0)
                        .color(Color32::from_rgb(200, 210, 225)),
                );
                let mut timeout = deck_state.display_timeout;
                let timeout_label = match timeout {
                    0 => "Never".to_string(),
                    30 => "30 seconds".to_string(),
                    60 => "1 minute".to_string(),
                    120 => "2 minutes".to_string(),
                    300 => "5 minutes".to_string(),
                    600 => "10 minutes".to_string(),
                    _ => format!("{} seconds", timeout),
                };

                egui::ComboBox::from_id_salt("timeout_combo")
                    .selected_text(timeout_label)
                    .show_ui(ui, |ui| {
                        for (sec, label) in [
                            (0, "Never"),
                            (30, "30 seconds"),
                            (60, "1 minute"),
                            (120, "2 minutes"),
                            (300, "5 minutes"),
                            (600, "10 minutes"),
                        ] {
                            if ui.selectable_value(&mut timeout, sec, label).clicked() {
                                deck_state.display_timeout = timeout;
                                self.config_dirty = true;
                            }
                        }
                    });

                ui.add_space(16.0);
                ui.separator();

                ui.label(
                    egui::RichText::new("Configuration Files:")
                        .size(13.0)
                        .color(Color32::from_rgb(200, 210, 225)),
                );
                ui.label(
                    egui::RichText::new(format!("{:?}", self.config_path))
                        .size(11.0)
                        .color(Color32::from_rgb(140, 155, 175)),
                );

                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    if ui.button("Import JSON...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("JSON Config", &["json"])
                            .pick_file()
                        {
                            match load_config(&path) {
                                Ok(cfg) => {
                                    self.config = cfg;
                                    self.textures_dirty = true;
                                    self.sync_hardware_deck();
                                    self.set_toast("Configuration imported successfully".to_string());
                                }
                                Err(e) => {
                                    self.set_toast(format!("Failed to import: {}", e));
                                }
                            }
                        }
                    }

                    if ui.button("Export JSON...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("JSON Config", &["json"])
                            .set_file_name("streamdeck_config.json")
                            .save_file()
                        {
                            match save_config(&self.config, &path) {
                                Ok(_) => {
                                    self.set_toast(format!("Exported to {:?}", path));
                                }
                                Err(e) => {
                                    self.set_toast(format!("Export failed: {}", e));
                                }
                            }
                        }
                    }
                });

                ui.add_space(16.0);

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("Close").clicked() {
                        self.settings_open = false;
                    }
                });
            });

        self.settings_open = open;
    }
}

fn configure_app_style(ctx: &Context) {
    let mut visuals = egui::Visuals::dark();

    visuals.override_text_color = Some(Color32::from_rgb(230, 235, 242));
    visuals.window_fill = Color32::from_rgb(20, 24, 32);
    visuals.panel_fill = Color32::from_rgb(14, 17, 22);

    visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(24, 29, 38);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(45, 54, 68));
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(4);

    visuals.widgets.inactive.bg_fill = Color32::from_rgb(30, 36, 48);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(52, 62, 78));
    visuals.widgets.inactive.corner_radius = CornerRadius::same(4);

    visuals.widgets.hovered.bg_fill = Color32::from_rgb(42, 52, 70);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(80, 110, 160));
    visuals.widgets.hovered.corner_radius = CornerRadius::same(4);

    visuals.widgets.active.bg_fill = Color32::from_rgb(50, 70, 105);
    visuals.widgets.active.bg_stroke = Stroke::new(1.5_f32, Color32::from_rgb(90, 140, 220));
    visuals.widgets.active.corner_radius = CornerRadius::same(4);

    visuals.selection.bg_fill = Color32::from_rgb(45, 80, 140);
    visuals.selection.stroke = Stroke::new(1.0_f32, Color32::from_rgb(100, 160, 255));

    ctx.set_visuals(visuals);
}
