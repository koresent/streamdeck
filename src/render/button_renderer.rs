use ab_glyph::{Font, FontRef, Glyph, PxScale, ScaleFont};
use anyhow::{Context, Result};
use image::{imageops, GenericImageView, ImageBuffer, Rgba, RgbaImage};
use std::fs;
use std::path::Path;

const DEFAULT_FONT_BYTES: &[u8] = include_bytes!("../../assets/fonts/Roboto-Regular.ttf");

pub struct ButtonRenderer {
    font: FontRef<'static>,
}

impl ButtonRenderer {
    pub fn new() -> Result<Self> {
        let font = FontRef::try_from_slice(DEFAULT_FONT_BYTES)
            .context("Failed to parse embedded default font")?;
        Ok(Self { font })
    }

    pub fn render_button(
        &self,
        width: u32,
        height: u32,
        icon_path: &str,
        text: &str,
        text_align: &str,
        font_size: u32,
        is_pressed: bool,
    ) -> RgbaImage {
        let base_color = if is_pressed {
            Rgba([45, 52, 65, 255])
        } else {
            Rgba([18, 22, 28, 255])
        };

        let mut image: RgbaImage = ImageBuffer::from_pixel(width, height, base_color);

        let trimmed_icon = icon_path.trim();
        if !trimmed_icon.is_empty() {
            if let Ok(icon_img) = self.load_icon(trimmed_icon, width, height) {
                let (w, h) = icon_img.dimensions();
                let x = (width.saturating_sub(w)) / 2;
                let y = (height.saturating_sub(h)) / 2;
                imageops::overlay(&mut image, &icon_img, x as i64, y as i64);
            }
        }

        let trimmed_text = text.trim();
        if !trimmed_text.is_empty() {
            self.draw_text_overlay(
                &mut image,
                trimmed_text,
                text_align,
                font_size.max(8).min(32),
                width,
                height,
            );
        }

        image
    }

    fn load_icon(&self, path_str: &str, target_w: u32, target_h: u32) -> Result<RgbaImage> {
        let path = Path::new(path_str);
        if !path.exists() {
            anyhow::bail!("Icon file does not exist: {:?}", path);
        }

        let is_svg = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.eq_ignore_ascii_case("svg"))
            .unwrap_or(false);

        if is_svg {
            self.render_svg(path, target_w, target_h)
        } else {
            let img = image::open(path)
                .with_context(|| format!("Failed to open image {:?}", path))?;

            // Scale preserving aspect ratio within target dimensions
            let (orig_w, orig_h) = img.dimensions();
            let scale = ((target_w as f32) / (orig_w as f32)).min((target_h as f32) / (orig_h as f32));
            let new_w = (orig_w as f32 * scale).round() as u32;
            let new_h = (orig_h as f32 * scale).round() as u32;

            let resized = img.resize_exact(
                new_w.max(1),
                new_h.max(1),
                imageops::FilterType::Lanczos3,
            );
            Ok(resized.to_rgba8())
        }
    }

    fn render_svg(&self, path: &Path, target_w: u32, target_h: u32) -> Result<RgbaImage> {
        let svg_data = fs::read(path)?;
        let opt = resvg::usvg::Options::default();
        let tree = resvg::usvg::Tree::from_data(&svg_data, &opt)?;

        let tree_size = tree.size();
        let (orig_w, orig_h) = (tree_size.width(), tree_size.height());

        let scale = ((target_w as f32) / orig_w).min((target_h as f32) / orig_h);
        let render_w = (orig_w * scale).round() as u32;
        let render_h = (orig_h * scale).round() as u32;

        let mut pixmap = resvg::tiny_skia::Pixmap::new(render_w.max(1), render_h.max(1))
            .context("Failed to allocate tiny_skia pixmap")?;

        let transform = resvg::tiny_skia::Transform::from_scale(scale, scale);
        resvg::render(&tree, transform, &mut pixmap.as_mut());

        let rgba_data = pixmap.data();
        let img = ImageBuffer::from_raw(render_w, render_h, rgba_data.to_vec())
            .context("Failed to build ImageBuffer from pixmap")?;

        Ok(img)
    }

    fn draw_text_overlay(
        &self,
        image: &mut RgbaImage,
        text: &str,
        alignment: &str,
        font_size: u32,
        width: u32,
        height: u32,
    ) {
        let scale = PxScale::from(font_size as f32);
        let scaled_font = self.font.as_scaled(scale);

        // Layout single line or multiple lines
        let lines: Vec<&str> = text.lines().collect();
        let line_height = (font_size as f32 * 1.25).round();
        let total_text_h = line_height * (lines.len() as f32);

        let start_y = match alignment.to_lowercase().as_str() {
            "top" => 4.0,
            "middle-top" => (height as f32) * 0.2,
            "middle" => ((height as f32) - total_text_h) / 2.0,
            "middle-bottom" => (height as f32) * 0.65 - total_text_h / 2.0,
            _ => (height as f32) - total_text_h - 4.0, // bottom default
        };

        for (i, line) in lines.iter().enumerate() {
            let line_y = start_y + (i as f32) * line_height;
            let mut glyphs: Vec<Glyph> = Vec::new();

            let mut caret = 0.0;
            for ch in line.chars() {
                let glyph_id = scaled_font.glyph_id(ch);
                let advance = scaled_font.h_advance(glyph_id);
                glyphs.push(scaled_font.scaled_glyph(ch));
                caret += advance;
            }

            let text_w = caret;
            let line_x = ((width as f32) - text_w) / 2.0;

            // Draw shadow/outline first (1px offset in each direction) for contrast
            let shadow_color = Rgba([0, 0, 0, 240]);
            let text_color = Rgba([255, 255, 255, 255]);

            for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (1, 1)] {
                self.rasterize_glyphs(
                    image,
                    &glyphs,
                    line_x + (dx as f32),
                    line_y + (dy as f32),
                    shadow_color,
                    scale,
                );
            }

            // Draw foreground text
            self.rasterize_glyphs(image, &glyphs, line_x, line_y, text_color, scale);
        }
    }

    fn rasterize_glyphs(
        &self,
        image: &mut RgbaImage,
        glyphs: &[Glyph],
        base_x: f32,
        base_y: f32,
        color: Rgba<u8>,
        scale: PxScale,
    ) {
        let scaled_font = self.font.as_scaled(scale);
        let (img_w, img_h) = image.dimensions();

        let mut current_x = base_x;
        for glyph in glyphs {
            let mut positioned = glyph.clone();
            positioned.position = ab_glyph::point(current_x, base_y + scaled_font.ascent());

            if let Some(outlined) = self.font.outline_glyph(positioned) {
                let bounds = outlined.px_bounds();
                outlined.draw(|px, py, coverage| {
                    if coverage > 0.05 {
                        let x = bounds.min.x as i32 + px as i32;
                        let y = bounds.min.y as i32 + py as i32;
                        if x >= 0 && x < (img_w as i32) && y >= 0 && y < (img_h as i32) {
                            let pixel = image.get_pixel_mut(x as u32, y as u32);
                            blend_pixel(pixel, color, coverage);
                        }
                    }
                });
            }

            current_x += scaled_font.h_advance(glyph.id);
        }
    }
}

fn blend_pixel(target: &mut Rgba<u8>, source: Rgba<u8>, alpha_mult: f32) {
    let src_a = (source[3] as f32 / 255.0) * alpha_mult;
    let dst_a = target[3] as f32 / 255.0;
    let out_a = src_a + dst_a * (1.0 - src_a);

    if out_a > 0.0 {
        for c in 0..3 {
            let src_c = source[c] as f32 / 255.0;
            let dst_c = target[c] as f32 / 255.0;
            let out_c = (src_c * src_a + dst_c * dst_a * (1.0 - src_a)) / out_a;
            target[c] = (out_c * 255.0).round().min(255.0) as u8;
        }
        target[3] = (out_a * 255.0).round().min(255.0) as u8;
    }
}
