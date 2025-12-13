use tiny_skia::{Color, Paint, Pixmap, PixmapMut, Rect, Transform};

pub struct Canvas<'a> {
    data: &'a mut [u8],
    width: u32,
    height: u32,
}

impl<'a> Canvas<'a> {
    pub fn new(data: &'a mut [u8], width: u32, height: u32) -> Self {
        Self {
            data,
            width,
            height,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn clear(&mut self, color: Color) {
        let Some(mut pixmap) = PixmapMut::from_bytes(self.data, self.width, self.height) else {
            return;
        };
        pixmap.fill(color);
    }

    pub fn fill_rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: Color) {
        let Some(mut pixmap) = PixmapMut::from_bytes(self.data, self.width, self.height) else {
            return;
        };

        let rect = match Rect::from_xywh(x, y, w, h) {
            Some(r) => r,
            None => return,
        };

        let mut paint = Paint::default();
        paint.set_color(color);
        paint.anti_alias = false;

        pixmap.fill_rect(rect, &paint, Transform::identity(), None);
    }

    pub fn draw_image(&mut self, x: i32, y: i32, image: &Pixmap) {
        let Some(mut pixmap) = PixmapMut::from_bytes(self.data, self.width, self.height) else {
            return;
        };

        pixmap.draw_pixmap(
            x,
            y,
            image.as_ref(),
            &tiny_skia::PixmapPaint::default(),
            Transform::identity(),
            None,
        );
    }

    pub fn draw_rgba(&mut self, x: i32, y: i32, width: u32, height: u32, rgba_data: &[u8]) {
        // Validate dimensions
        let Some(size) = tiny_skia::IntSize::from_wh(width, height) else {
            return;
        };

        // Validate data length matches dimensions
        let expected_len = (width * height * 4) as usize;
        if rgba_data.len() != expected_len {
            return;
        }

        // Convert RGBA to tiny-skia Pixmap and draw
        let Some(pixmap) = Pixmap::from_vec(Self::rgba_to_premultiplied_argb(rgba_data), size)
        else {
            return;
        };

        self.draw_image(x, y, &pixmap);
    }

    fn rgba_to_premultiplied_argb(rgba: &[u8]) -> Vec<u8> {
        let mut result = Vec::with_capacity(rgba.len());
        for chunk in rgba.chunks(4) {
            let r = chunk[0];
            let g = chunk[1];
            let b = chunk[2];
            let a = chunk[3];

            // Premultiply and convert to ARGB
            let alpha = a as f32 / 255.0;
            result.push((b as f32 * alpha) as u8); // B
            result.push((g as f32 * alpha) as u8); // G
            result.push((r as f32 * alpha) as u8); // R
            result.push(a); // A
        }
        result
    }

    pub fn data(&self) -> &[u8] {
        self.data
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        self.data
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, color: Color) {
        if x >= self.width || y >= self.height {
            return;
        }
        let offset = ((y * self.width + x) * 4) as usize;
        if offset + 3 >= self.data.len() {
            return;
        }
        // tiny-skia Color uses f32 in 0.0-1.0 range, convert to u8
        self.data[offset] = (color.red() * 255.0) as u8;
        self.data[offset + 1] = (color.green() * 255.0) as u8;
        self.data[offset + 2] = (color.blue() * 255.0) as u8;
        self.data[offset + 3] = (color.alpha() * 255.0) as u8;
    }

    /// Convert from tiny-skia's RGBA to Wayland's BGRA format.
    /// Call this after all drawing is complete, before sending to compositor.
    pub fn finalize_for_wayland(&mut self) {
        // Swap R and B channels: RGBA -> BGRA
        for chunk in self.data.chunks_exact_mut(4) {
            chunk.swap(0, 2); // Swap R (index 0) with B (index 2)
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub fn to_color(&self) -> Color {
        Color::from_rgba8(self.r, self.g, self.b, self.a)
    }
}

// Common colors
impl Rgba {
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    pub const WHITE: Self = Self::rgb(255, 255, 255);
    pub const RED: Self = Self::rgb(255, 0, 0);
    pub const GREEN: Self = Self::rgb(0, 255, 0);
    pub const BLUE: Self = Self::rgb(0, 0, 255);
    pub const TRANSPARENT: Self = Self::new(0, 0, 0, 0);
}
