use cosmic_text::{Attrs, Buffer, Color, Family, FontSystem, Metrics, Shaping, SwashCache};

use crate::render::Canvas;
use crate::widget::Rect;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HAlign {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VAlign {
    Top,
    #[default]
    Center,
    Bottom,
}

pub struct TextRenderer {
    font_system: FontSystem,
    swash_cache: SwashCache,
}

impl TextRenderer {
    pub fn new() -> Self {
        Self {
            font_system: FontSystem::new(),
            swash_cache: SwashCache::new(),
        }
    }

    pub fn draw_text(
        &mut self,
        canvas: &mut Canvas,
        text: &str,
        x: i32,
        y: i32,
        font_size: f32,
        color: Color,
    ) {
        let metrics = Metrics::new(font_size, font_size * 1.2);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);

        let attrs = Attrs::new().family(Family::Monospace);
        buffer.set_text(&mut self.font_system, text, attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut self.font_system, false);

        // y is top of text area - no adjustment needed, render_buffer handles baseline
        self.render_buffer(canvas, &buffer, x, y, color);
    }

    pub fn draw_text_with_attrs(
        &mut self,
        canvas: &mut Canvas,
        text: &str,
        x: i32,
        y: i32,
        metrics: Metrics,
        attrs: Attrs,
        color: Color,
    ) {
        let mut buffer = Buffer::new(&mut self.font_system, metrics);
        buffer.set_text(&mut self.font_system, text, attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut self.font_system, false);

        // y is top of text area - no adjustment needed
        self.render_buffer(canvas, &buffer, x, y, color);
    }

    pub fn draw_text_in_rect(
        &mut self,
        canvas: &mut Canvas,
        text: &str,
        rect: Rect,
        font_size: f32,
        color: Color,
        h_align: HAlign,
        v_align: VAlign,
    ) {
        let (text_width, text_height) = self.measure_text(text, font_size);

        let x = match h_align {
            HAlign::Left => rect.x,
            HAlign::Center => rect.x + (rect.width as i32 - text_width as i32) / 2,
            HAlign::Right => rect.x + rect.width as i32 - text_width as i32,
        };

        let y = match v_align {
            VAlign::Top => rect.y,
            VAlign::Center => rect.y + (rect.height as i32 - text_height as i32) / 2,
            VAlign::Bottom => rect.y + rect.height as i32 - text_height as i32,
        };

        self.draw_text(canvas, text, x, y, font_size, color);
    }

    fn render_buffer(
        &mut self,
        canvas: &mut Canvas,
        buffer: &Buffer,
        x: i32,
        y: i32,
        color: Color,
    ) {
        let canvas_width = canvas.width() as i32;
        let canvas_height = canvas.height() as i32;

        for run in buffer.layout_runs() {
            // run.line_y is the baseline position for this line
            let line_y = y as f32 + run.line_y;
            for glyph in run.glyphs.iter() {
                let physical_glyph = glyph.physical((x as f32, line_y), 1.0);

                let Some(image) = self
                    .swash_cache
                    .get_image(&mut self.font_system, physical_glyph.cache_key)
                else {
                    continue;
                };

                let glyph_x = physical_glyph.x + image.placement.left;
                let glyph_y = physical_glyph.y - image.placement.top;

                // Draw glyph pixels
                for (row_idx, row) in image
                    .data
                    .chunks(image.placement.width as usize)
                    .enumerate()
                {
                    let py = glyph_y + row_idx as i32;
                    if py < 0 || py >= canvas_height {
                        continue;
                    }

                    for (col_idx, &alpha) in row.iter().enumerate() {
                        let px = glyph_x + col_idx as i32;
                        if px < 0 || px >= canvas_width {
                            continue;
                        }

                        if alpha == 0 {
                            continue;
                        }

                        let offset = ((py as u32 * canvas.width() + px as u32) * 4) as usize;
                        let data = canvas.data_mut();
                        if offset + 3 >= data.len() {
                            continue;
                        }

                        // Blend with alpha
                        let alpha_f = alpha as f32 / 255.0;
                        let inv_alpha = 1.0 - alpha_f;

                        // BGRA format for Wayland
                        data[offset] = ((color.b() as f32 * alpha_f)
                            + (data[offset] as f32 * inv_alpha))
                            as u8;
                        data[offset + 1] = ((color.g() as f32 * alpha_f)
                            + (data[offset + 1] as f32 * inv_alpha))
                            as u8;
                        data[offset + 2] = ((color.r() as f32 * alpha_f)
                            + (data[offset + 2] as f32 * inv_alpha))
                            as u8;
                        data[offset + 3] = 255;
                    }
                }
            }
        }
    }

    pub fn measure_text(&mut self, text: &str, font_size: f32) -> (f32, f32) {
        let metrics = Metrics::new(font_size, font_size * 1.2);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);

        let attrs = Attrs::new().family(Family::Monospace);
        buffer.set_text(&mut self.font_system, text, attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut self.font_system, false);

        let mut width = 0.0f32;
        let mut height = 0.0f32;

        for run in buffer.layout_runs() {
            width = width.max(run.line_w);
            height += metrics.line_height;
        }

        (width, height)
    }
}

impl Default for TextRenderer {
    fn default() -> Self {
        Self::new()
    }
}
