// Allow dead code for library features that are planned but not yet used
#![allow(dead_code)]
// Allow too many arguments for text rendering functions
#![allow(clippy::too_many_arguments)]

mod app;
mod attached_surface;
mod gpu;
mod input;
mod render;
mod split;
mod text;
mod widget;
mod window;

pub use app::{App, DropEvent};
pub use attached_surface::{
    Anchor as AttachedAnchor, AttachedSurface, AttachedSurfaceHandler, AttachedSurfaceId,
    AttachedSurfaceManager,
};
#[cfg(feature = "gpu")]
pub use gpu::GpuRenderTarget;
pub use gpu::{Renderer, RendererBackend};
pub use input::{
    Key, KeyEvent, KeyState, Modifiers, PointerButton, PointerEvent, PointerEventKind,
};
pub use render::{Canvas, Rgba};
pub use split::{LeafId, SplitDirection, SplitTree};
pub use text::{HAlign, TextRenderer, VAlign};
pub use widget::{Constraints, LayoutContext, Rect, RenderContext, Size, Widget, WidgetId};
pub use window::{
    Overlay, OverlayId, Popup, PopupAnchor, PopupConfig, PopupGravity, PopupId, Subsurface,
    SubsurfaceId, Window, WindowId, WindowManager,
};

// Re-export key dependencies for users
pub use cosmic_text::Color as TextColor;
pub use smithay_client_toolkit::reexports::client::{EventQueue, QueueHandle};
pub use smithay_client_toolkit::shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer};
pub use tiny_skia::Color;
