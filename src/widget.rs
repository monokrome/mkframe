use crate::input::{KeyEvent, PointerEvent};
use crate::render::Canvas;
use crate::text::TextRenderer;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WidgetId(pub u64);

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x
            && px < self.x + self.width as i32
            && py >= self.y
            && py < self.y + self.height as i32
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Constraints {
    pub min_width: u32,
    pub max_width: u32,
    pub min_height: u32,
    pub max_height: u32,
}

impl Constraints {
    pub fn tight(width: u32, height: u32) -> Self {
        Self {
            min_width: width,
            max_width: width,
            min_height: height,
            max_height: height,
        }
    }

    pub fn loose(max_width: u32, max_height: u32) -> Self {
        Self {
            min_width: 0,
            max_width,
            min_height: 0,
            max_height,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

impl Size {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

pub struct LayoutContext<'a> {
    pub text: &'a mut TextRenderer,
}

pub struct RenderContext<'a> {
    pub canvas: &'a mut Canvas<'a>,
    pub text: &'a mut TextRenderer,
}

pub trait Widget {
    fn id(&self) -> WidgetId;

    fn layout(&mut self, constraints: Constraints, ctx: &mut LayoutContext) -> Size;

    fn render(&self, bounds: Rect, ctx: &mut RenderContext);

    fn handle_key(&mut self, _event: &KeyEvent) -> bool {
        false
    }

    fn handle_pointer(&mut self, _event: &PointerEvent, _bounds: Rect) -> bool {
        false
    }

    fn is_focusable(&self) -> bool {
        false
    }
}

// Simple container for vertical layout
pub struct VStack {
    id: WidgetId,
    children: Vec<Box<dyn Widget>>,
    spacing: u32,
    cached_sizes: Vec<Size>,
}

impl VStack {
    pub fn new(id: WidgetId) -> Self {
        Self {
            id,
            children: Vec::new(),
            spacing: 0,
            cached_sizes: Vec::new(),
        }
    }

    pub fn spacing(mut self, spacing: u32) -> Self {
        self.spacing = spacing;
        self
    }

    pub fn child(mut self, widget: impl Widget + 'static) -> Self {
        self.children.push(Box::new(widget));
        self
    }

    pub fn add_child(&mut self, widget: impl Widget + 'static) {
        self.children.push(Box::new(widget));
    }
}

impl Widget for VStack {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&mut self, constraints: Constraints, ctx: &mut LayoutContext) -> Size {
        self.cached_sizes.clear();

        let mut total_height = 0u32;
        let mut max_width = 0u32;

        let child_constraints = Constraints {
            min_width: constraints.min_width,
            max_width: constraints.max_width,
            min_height: 0,
            max_height: constraints.max_height,
        };

        for (i, child) in self.children.iter_mut().enumerate() {
            let size = child.layout(child_constraints, ctx);
            self.cached_sizes.push(size);

            total_height += size.height;
            if i > 0 {
                total_height += self.spacing;
            }
            max_width = max_width.max(size.width);
        }

        Size {
            width: max_width.clamp(constraints.min_width, constraints.max_width),
            height: total_height.clamp(constraints.min_height, constraints.max_height),
        }
    }

    fn render(&self, bounds: Rect, ctx: &mut RenderContext) {
        let mut y = bounds.y;

        for (child, size) in self.children.iter().zip(self.cached_sizes.iter()) {
            let child_bounds = Rect {
                x: bounds.x,
                y,
                width: size.width,
                height: size.height,
            };
            child.render(child_bounds, ctx);
            y += size.height as i32 + self.spacing as i32;
        }
    }

    fn handle_key(&mut self, event: &KeyEvent) -> bool {
        for child in &mut self.children {
            if child.handle_key(event) {
                return true;
            }
        }
        false
    }

    fn handle_pointer(&mut self, event: &PointerEvent, bounds: Rect) -> bool {
        let mut y = bounds.y;

        for (child, size) in self.children.iter_mut().zip(self.cached_sizes.iter()) {
            let child_bounds = Rect {
                x: bounds.x,
                y,
                width: size.width,
                height: size.height,
            };

            if child_bounds.contains(event.x as i32, event.y as i32)
                && child.handle_pointer(event, child_bounds)
            {
                return true;
            }

            y += size.height as i32 + self.spacing as i32;
        }
        false
    }
}

// Simple text label widget
pub struct Label {
    id: WidgetId,
    text: String,
    font_size: f32,
    color: cosmic_text::Color,
    cached_size: Size,
}

impl Label {
    pub fn new(id: WidgetId, text: impl Into<String>) -> Self {
        Self {
            id,
            text: text.into(),
            font_size: 14.0,
            color: cosmic_text::Color::rgb(255, 255, 255),
            cached_size: Size::default(),
        }
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    pub fn color(mut self, color: cosmic_text::Color) -> Self {
        self.color = color;
        self
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }
}

impl Widget for Label {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&mut self, _constraints: Constraints, ctx: &mut LayoutContext) -> Size {
        let (width, height) = ctx.text.measure_text(&self.text, self.font_size);
        self.cached_size = Size {
            width: width.ceil() as u32,
            height: height.ceil() as u32,
        };
        self.cached_size
    }

    fn render(&self, bounds: Rect, ctx: &mut RenderContext) {
        ctx.text.draw_text(
            ctx.canvas,
            &self.text,
            bounds.x,
            bounds.y,
            self.font_size,
            self.color,
        );
    }
}
