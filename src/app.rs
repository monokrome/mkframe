use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    data_device_manager::{
        DataDeviceManagerState, WritePipe,
        data_device::DataDeviceHandler,
        data_offer::DataOfferHandler,
        data_source::{DataSourceHandler, DragSource},
    },
    output::{OutputHandler, OutputState},
    reexports::client::{
        Connection, Dispatch, EventQueue, QueueHandle,
        globals::registry_queue_init,
        protocol::{
            wl_keyboard, wl_output, wl_pointer, wl_seat, wl_shm, wl_subcompositor, wl_subsurface,
            wl_surface,
        },
    },
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        SeatHandler, SeatState,
        keyboard::{KeyEvent as SctkKeyEvent, KeyboardHandler, Keysym, Modifiers},
        pointer::{PointerEvent as SctkPointerEvent, PointerHandler},
    },
    shell::{
        WaylandSurface,
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
        xdg::{
            XdgPositioner, XdgShell, XdgSurface,
            popup::{Popup as XdgPopup, PopupConfigure, PopupHandler},
            window::{Window as XdgWindow, WindowConfigure, WindowDecorations, WindowHandler},
        },
    },
    shm::{Shm, ShmHandler, slot::SlotPool},
};
use wayland_protocols::xdg::shell::client::xdg_positioner::ConstraintAdjustment;

use crate::attached_surface::{
    Anchor as AttachedAnchor, AttachedSurface, AttachedSurfaceData, AttachedSurfaceHandler,
    AttachedSurfaceId, AttachedSurfaceManager,
    protocol::zwlr_attached_surface_manager_v1::ZwlrAttachedSurfaceManagerV1,
    protocol::zwlr_attached_surface_v1::ZwlrAttachedSurfaceV1,
};
use crate::input::{Key, KeyEvent, KeyState, Modifiers as InputModifiers, PointerEvent};
use crate::render::Canvas;
use crate::window::{
    Overlay, OverlayId, Popup, PopupConfig, PopupId, Subsurface, SubsurfaceId, Window, WindowId,
    WindowManager,
};

pub trait AppHandler {
    fn on_window_configure(&mut self, app: &mut App, window_id: WindowId, width: u32, height: u32);
    fn on_popup_configure(&mut self, app: &mut App, popup_id: PopupId, width: u32, height: u32);
    fn on_key(&mut self, app: &mut App, window_id: WindowId, event: KeyEvent);
    fn on_pointer(&mut self, app: &mut App, window_id: WindowId, event: PointerEvent);
    fn on_render(&mut self, app: &mut App, window_id: WindowId, canvas: &mut Canvas);
    fn on_render_popup(&mut self, app: &mut App, popup_id: PopupId, canvas: &mut Canvas);
    fn on_close_request(&mut self, app: &mut App, window_id: WindowId) -> bool;
}

pub struct App {
    pub running: bool,
    conn: Connection,
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    compositor_state: CompositorState,
    subcompositor: Option<wl_subcompositor::WlSubcompositor>,
    xdg_shell: XdgShell,
    layer_shell: Option<LayerShell>,
    attached_surface_manager: Option<AttachedSurfaceManager>,
    shm: Shm,
    pool: Option<SlotPool>,
    pub windows: WindowManager,
    keyboard_focus: Option<WindowId>,
    pointer_focus: Option<WindowId>,
    last_serial: u32,
    key_events: Vec<KeyEvent>,
    current_modifiers: InputModifiers,
    // Key repeat state
    repeat_key: Option<KeyEvent>,
    repeat_start: Option<std::time::Instant>,
    last_repeat: Option<std::time::Instant>,
    repeat_delay_ms: u32,
    repeat_rate_ms: u32,
    // Pointer state
    pointer_events: Vec<crate::input::PointerEvent>,
    pointer_x: f64,
    pointer_y: f64,
    // Data device state (drag & drop, clipboard)
    data_device_manager: Option<DataDeviceManagerState>,
    drop_events: Vec<DropEvent>,
    pending_drag_source: Option<DragSource>,
    pending_drag_data: Option<Vec<u8>>,
    // Seat for drag & drop
    current_seat: Option<wl_seat::WlSeat>,
}

/// Represents a completed drop event with file URIs
#[derive(Debug, Clone)]
pub struct DropEvent {
    pub x: f64,
    pub y: f64,
    pub files: Vec<std::path::PathBuf>,
}

impl App {
    pub fn new() -> Result<(Self, EventQueue<Self>), Box<dyn std::error::Error>> {
        let conn = Connection::connect_to_env()?;
        let (globals, event_queue) = registry_queue_init(&conn)?;
        let qh = event_queue.handle();

        let registry_state = RegistryState::new(&globals);
        let seat_state = SeatState::new(&globals, &qh);
        let output_state = OutputState::new(&globals, &qh);
        let compositor_state = CompositorState::bind(&globals, &qh)?;

        // Bind subcompositor for subsurface support
        let subcompositor: Option<wl_subcompositor::WlSubcompositor> =
            globals.bind(&qh, 1..=1, ()).ok();

        let xdg_shell = XdgShell::bind(&globals, &qh)?;
        let layer_shell = LayerShell::bind(&globals, &qh).ok(); // Optional - not all compositors support it
        let shm = Shm::bind(&globals, &qh)?;

        // Try to bind the attached surface manager (only available on supporting compositors)
        let attached_surface_manager: Option<AttachedSurfaceManager> = globals
            .bind(&qh, 1..=1, ())
            .ok()
            .map(AttachedSurfaceManager::new);

        // Bind data device manager for drag & drop and clipboard support
        let data_device_manager = DataDeviceManagerState::bind(&globals, &qh).ok();

        let pool = SlotPool::new(1920 * 1080 * 4, &shm)?;

        Ok((
            Self {
                running: true,
                conn,
                registry_state,
                seat_state,
                output_state,
                compositor_state,
                subcompositor,
                xdg_shell,
                layer_shell,
                attached_surface_manager,
                shm,
                pool: Some(pool),
                windows: WindowManager::new(),
                keyboard_focus: None,
                pointer_focus: None,
                last_serial: 0,
                key_events: Vec::new(),
                current_modifiers: InputModifiers::default(),
                repeat_key: None,
                repeat_start: None,
                last_repeat: None,
                repeat_delay_ms: 400, // Typical default: 400ms delay
                repeat_rate_ms: 33,   // ~30 repeats per second
                pointer_events: Vec::new(),
                pointer_x: 0.0,
                pointer_y: 0.0,
                data_device_manager,
                drop_events: Vec::new(),
                pending_drag_source: None,
                pending_drag_data: None,
                current_seat: None,
            },
            event_queue,
        ))
    }

    pub fn has_layer_shell(&self) -> bool {
        self.layer_shell.is_some()
    }

    pub fn create_window(
        &mut self,
        qh: &QueueHandle<Self>,
        title: &str,
        width: u32,
        height: u32,
    ) -> WindowId {
        self.create_window_full(qh, title, None, width, height, true)
    }

    pub fn create_window_with_decorations(
        &mut self,
        qh: &QueueHandle<Self>,
        title: &str,
        width: u32,
        height: u32,
        decorations: bool,
    ) -> WindowId {
        self.create_window_full(qh, title, None, width, height, decorations)
    }

    pub fn create_window_full(
        &mut self,
        qh: &QueueHandle<Self>,
        title: &str,
        app_id: Option<&str>,
        width: u32,
        height: u32,
        decorations: bool,
    ) -> WindowId {
        let surface = self.compositor_state.create_surface(qh);
        let decoration_mode = if decorations {
            WindowDecorations::ServerDefault
        } else {
            WindowDecorations::None
        };
        let xdg = self.xdg_shell.create_window(surface, decoration_mode, qh);
        xdg.set_title(title.to_string());
        if let Some(id) = app_id {
            xdg.set_app_id(id.to_string());
        }
        xdg.set_min_size(Some((100, 100)));
        xdg.commit();

        let id = self.windows.next_window_id();
        self.windows.windows.insert(
            id,
            Window {
                id,
                xdg,
                width,
                height,
                dirty: true,
            },
        );

        id
    }

    pub fn create_popup(
        &mut self,
        qh: &QueueHandle<Self>,
        parent_id: WindowId,
        config: PopupConfig,
    ) -> Option<PopupId> {
        let parent = self.windows.get_window(parent_id)?;

        let surface = self.compositor_state.create_surface(qh);

        let positioner = XdgPositioner::new(&self.xdg_shell).ok()?;
        positioner.set_size(config.size.0 as i32, config.size.1 as i32);

        let anchor_rect =
            config
                .anchor_rect
                .unwrap_or((0, 0, parent.width as i32, parent.height as i32));
        positioner.set_anchor_rect(anchor_rect.0, anchor_rect.1, anchor_rect.2, anchor_rect.3);
        positioner.set_anchor(config.anchor.into());
        positioner.set_gravity(config.gravity.into());
        positioner.set_offset(config.offset.0, config.offset.1);
        positioner
            .set_constraint_adjustment(ConstraintAdjustment::FlipX | ConstraintAdjustment::FlipY);

        // Get the parent's xdg_surface - this is the key to making popups work!
        let parent_xdg_surface = parent.xdg.xdg_surface();

        // from_surface creates the popup's xdg_surface from our wl_surface
        let popup = XdgPopup::from_surface(
            Some(parent_xdg_surface),
            &positioner,
            qh,
            surface,
            &self.xdg_shell,
        )
        .ok()?;

        popup.wl_surface().commit();

        let id = self.windows.next_popup_id();
        self.windows.popups.insert(
            id,
            Popup {
                id,
                parent: parent_id,
                xdg: popup,
                width: config.size.0,
                height: config.size.1,
                dirty: false, // Wait for configure event
            },
        );

        Some(id)
    }

    pub fn close_popup(&mut self, popup_id: PopupId) {
        // Just remove from our map - sctk's Popup handles cleanup on drop
        self.windows.popups.remove(&popup_id);
    }

    pub fn close_window(&mut self, window_id: WindowId) {
        // Close all popups for this window first
        let popup_ids: Vec<PopupId> = self
            .windows
            .popups
            .iter()
            .filter(|(_, p)| p.parent == window_id)
            .map(|(id, _)| *id)
            .collect();

        for id in popup_ids {
            self.close_popup(id);
        }

        self.windows.windows.remove(&window_id);
    }

    /// Create a layer-shell overlay (persistent, screen-level surface).
    /// Only works on wlroots-based compositors (Sway, Hyprland, dwl, etc.)
    pub fn create_overlay(
        &mut self,
        qh: &QueueHandle<Self>,
        width: u32,
        height: u32,
        layer_level: Layer,
        anchor: Anchor,
        margin: (i32, i32, i32, i32), // top, right, bottom, left
        keyboard_interactivity: KeyboardInteractivity,
    ) -> Option<OverlayId> {
        let layer_shell = self.layer_shell.as_ref()?;

        let surface = self.compositor_state.create_surface(qh);
        let layer = layer_shell.create_layer_surface(
            qh,
            surface,
            layer_level,
            Some("mkframe-overlay"),
            None, // output - None means current output
        );

        layer.set_size(width, height);
        layer.set_anchor(anchor);
        layer.set_margin(margin.0, margin.1, margin.2, margin.3);
        layer.set_keyboard_interactivity(keyboard_interactivity);
        layer.set_exclusive_zone(-1); // Don't reserve space, allow input passthrough when not focused
        layer.commit();

        let id = self.windows.next_overlay_id();
        self.windows.overlays.insert(
            id,
            Overlay {
                id,
                layer,
                width,
                height,
                dirty: false, // Wait for configure
            },
        );

        Some(id)
    }

    pub fn close_overlay(&mut self, overlay_id: OverlayId) {
        self.windows.overlays.remove(&overlay_id);
    }

    pub fn has_subcompositor(&self) -> bool {
        self.subcompositor.is_some()
    }

    /// Create a subsurface attached to a parent window.
    /// Subsurfaces are positioned relative to their parent and move with it.
    pub fn create_subsurface(
        &mut self,
        qh: &QueueHandle<Self>,
        parent_id: WindowId,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) -> Option<SubsurfaceId> {
        let subcompositor = self.subcompositor.as_ref()?;
        let parent = self.windows.get_window(parent_id)?;
        let parent_surface = parent.xdg.wl_surface();

        // Create a new surface for the subsurface
        let surface = self.compositor_state.create_surface(qh);

        // Create the subsurface relationship
        let subsurface = subcompositor.get_subsurface(&surface, parent_surface, qh, ());

        // Position relative to parent
        subsurface.set_position(x, y);

        // Use desync mode so we can update independently
        subsurface.set_desync();

        // Place above parent
        subsurface.place_above(parent_surface);

        let id = self.windows.next_subsurface_id();
        self.windows.subsurfaces.insert(
            id,
            Subsurface {
                id,
                parent: parent_id,
                surface,
                subsurface,
                x,
                y,
                width,
                height,
                dirty: true, // Ready to render immediately
            },
        );

        Some(id)
    }

    pub fn close_subsurface(&mut self, subsurface_id: SubsurfaceId) {
        if let Some(sub) = self.windows.subsurfaces.remove(&subsurface_id) {
            sub.subsurface.destroy();
            sub.surface.destroy();
        }
    }

    pub fn is_subsurface_dirty(&self, subsurface_id: SubsurfaceId) -> bool {
        self.windows
            .get_subsurface(subsurface_id)
            .map(|s| s.dirty)
            .unwrap_or(false)
    }

    pub fn set_subsurface_position(&mut self, subsurface_id: SubsurfaceId, x: i32, y: i32) {
        if let Some(sub) = self.windows.get_subsurface_mut(subsurface_id) {
            sub.x = x;
            sub.y = y;
            sub.subsurface.set_position(x, y);
        }
    }

    /// Check if the attached surface protocol is available
    pub fn has_attached_surface(&self) -> bool {
        self.attached_surface_manager.is_some()
    }

    /// Create an attached surface that can extend beyond parent window bounds.
    /// Only works on compositors that support the wlr-attached-surface protocol.
    pub fn create_attached_surface(
        &mut self,
        qh: &QueueHandle<Self>,
        parent_id: WindowId,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) -> Option<AttachedSurfaceId> {
        let manager = self.attached_surface_manager.clone()?;
        let parent = self.windows.get_window(parent_id)?;

        // Get the parent's xdg_toplevel resource and clone it before mutating windows
        let parent_toplevel = parent.xdg.xdg_toplevel().clone();

        // Create a new surface for the attached surface
        let surface = self.compositor_state.create_surface(qh);

        // Create the attached surface
        let id = self.windows.next_attached_surface_id();
        let attached = manager.inner().get_attached_surface(
            &surface,
            &parent_toplevel,
            qh,
            AttachedSurfaceData { id },
        );

        // Set initial position and size
        attached.set_position(x, y);
        attached.set_size(width, height);

        // Initial commit without buffer
        surface.commit();

        self.windows.attached_surfaces.insert(
            id,
            AttachedSurface {
                id,
                parent_window_id: parent_id,
                surface,
                attached,
                x,
                y,
                width,
                height,
                dirty: false, // Wait for configure
                configured: false,
                pending_configure: None,
            },
        );

        Some(id)
    }

    pub fn close_attached_surface(&mut self, id: AttachedSurfaceId) {
        if let Some(attached) = self.windows.attached_surfaces.remove(&id) {
            attached.attached.destroy();
            attached.surface.destroy();
        }
    }

    pub fn is_attached_surface_dirty(&self, id: AttachedSurfaceId) -> bool {
        self.windows
            .get_attached_surface(id)
            .map(|s| s.dirty)
            .unwrap_or(false)
    }

    pub fn set_attached_surface_position(&mut self, id: AttachedSurfaceId, x: i32, y: i32) {
        if let Some(attached) = self.windows.get_attached_surface_mut(id) {
            attached.x = x;
            attached.y = y;
            attached.attached.set_position(x, y);
        }
    }

    pub fn set_attached_surface_anchor(
        &mut self,
        id: AttachedSurfaceId,
        anchor: AttachedAnchor,
        margin: i32,
        offset: i32,
    ) {
        if let Some(attached) = self.windows.get_attached_surface_mut(id) {
            attached.set_anchor(anchor, margin, offset);
        }
    }

    pub fn render_attached_surface<F>(&mut self, id: AttachedSurfaceId, mut draw: F)
    where
        F: FnMut(&mut Canvas),
    {
        let Some(attached) = self.windows.get_attached_surface_mut(id) else {
            return;
        };

        if !attached.configured {
            return;
        }

        let width = attached.width;
        let height = attached.height;
        let surface = attached.surface.clone();
        attached.dirty = false;

        let Some(pool) = self.pool.as_mut() else {
            return;
        };

        let stride = width * 4;
        let buffer_size = (stride * height) as usize;

        if pool.len() < buffer_size {
            pool.resize(buffer_size).ok();
        }

        let (buffer, canvas_data) = match pool.create_buffer(
            width as i32,
            height as i32,
            stride as i32,
            wl_shm::Format::Argb8888,
        ) {
            Ok((buf, data)) => (buf, data),
            Err(_) => return,
        };

        {
            let mut canvas = Canvas::new(canvas_data, width, height);
            draw(&mut canvas);
            canvas.finalize_for_wayland();
        }

        surface.attach(Some(buffer.wl_buffer()), 0, 0);
        surface.damage_buffer(0, 0, width as i32, height as i32);
        surface.commit();
    }

    pub fn quit(&mut self) {
        self.running = false;
    }

    /// Drain and return all pending key events (including repeat events)
    pub fn poll_key_events(&mut self) -> Vec<KeyEvent> {
        // Generate repeat events if a key is held
        if let (Some(key), Some(start)) = (&self.repeat_key, self.repeat_start) {
            let now = std::time::Instant::now();
            let elapsed = now.duration_since(start);
            let delay = std::time::Duration::from_millis(self.repeat_delay_ms as u64);

            if elapsed >= delay {
                // Past initial delay, check for repeats
                let rate = std::time::Duration::from_millis(self.repeat_rate_ms as u64);
                let should_repeat = match self.last_repeat {
                    None => true,
                    Some(last) => now.duration_since(last) >= rate,
                };

                if should_repeat {
                    self.last_repeat = Some(now);
                    let mut repeat_event = key.clone();
                    repeat_event.state = KeyState::Pressed;
                    self.key_events.push(repeat_event);
                }
            }
        }

        std::mem::take(&mut self.key_events)
    }

    /// Poll for pointer events (clicks, motion, scroll)
    pub fn poll_pointer_events(&mut self) -> Vec<crate::input::PointerEvent> {
        std::mem::take(&mut self.pointer_events)
    }

    /// Get current pointer position
    pub fn pointer_position(&self) -> (f64, f64) {
        (self.pointer_x, self.pointer_y)
    }

    /// Get current modifier state
    pub fn modifiers(&self) -> InputModifiers {
        self.current_modifiers
    }

    /// Poll for completed drop events
    pub fn poll_drop_events(&mut self) -> Vec<DropEvent> {
        std::mem::take(&mut self.drop_events)
    }

    /// Check if drag & drop is supported
    pub fn has_data_device(&self) -> bool {
        self.data_device_manager.is_some()
    }

    /// Check if we have a seat for input
    pub fn has_seat(&self) -> bool {
        self.current_seat.is_some()
    }

    /// Start a drag operation with the given file paths
    /// Returns true if the drag was started successfully
    pub fn start_drag(
        &mut self,
        qh: &QueueHandle<Self>,
        window_id: WindowId,
        files: &[std::path::PathBuf],
    ) -> bool {
        // Need data device manager, seat, and a window surface
        let Some(ref ddm) = self.data_device_manager else {
            return false;
        };
        let Some(ref seat) = self.current_seat else {
            return false;
        };
        let Some(window) = self.windows.get_window(window_id) else {
            return false;
        };

        // Build URI list data with absolute paths
        let uri_list: String = files
            .iter()
            .filter_map(|p| p.canonicalize().ok())
            .filter_map(|p| p.to_str().map(|s| s.to_string()))
            .map(|s| format!("file://{}\r\n", s))
            .collect();

        if uri_list.is_empty() {
            return false;
        }

        // Store data to send later
        self.pending_drag_data = Some(uri_list.into_bytes());

        // Create drag source with text/uri-list MIME type
        use smithay_client_toolkit::reexports::client::protocol::wl_data_device_manager::DndAction;
        let drag_source = ddm.create_drag_and_drop_source(
            qh,
            ["text/uri-list"],
            DndAction::Copy | DndAction::Move,
        );

        // Get data device for this seat
        let data_device = ddm.get_data_device(qh, seat);

        // Start the drag
        let surface = window.xdg.wl_surface();
        drag_source.start_drag(&data_device, surface, None, self.last_serial);

        // Store the drag source to keep it alive
        self.pending_drag_source = Some(drag_source);

        true
    }

    pub fn render_window<F>(&mut self, window_id: WindowId, mut draw: F)
    where
        F: FnMut(&mut Canvas),
    {
        let Some(window) = self.windows.get_window_mut(window_id) else {
            return;
        };

        let width = window.width;
        let height = window.height;
        let surface = window.xdg.wl_surface().clone();
        window.dirty = false;

        let Some(pool) = self.pool.as_mut() else {
            return;
        };

        let stride = width * 4;
        let buffer_size = (stride * height) as usize;

        // Resize pool if needed
        if pool.len() < buffer_size {
            pool.resize(buffer_size).ok();
        }

        let (buffer, canvas_data) = match pool.create_buffer(
            width as i32,
            height as i32,
            stride as i32,
            wl_shm::Format::Argb8888,
        ) {
            Ok((buf, data)) => (buf, data),
            Err(_) => return,
        };

        // Create canvas and let user draw
        {
            let mut canvas = Canvas::new(canvas_data, width, height);
            draw(&mut canvas);
            canvas.finalize_for_wayland();
        }

        // Attach and commit
        surface.attach(Some(buffer.wl_buffer()), 0, 0);
        surface.damage_buffer(0, 0, width as i32, height as i32);
        surface.commit();
    }

    pub fn render_popup<F>(&mut self, popup_id: PopupId, mut draw: F)
    where
        F: FnMut(&mut Canvas),
    {
        let Some(popup) = self.windows.get_popup_mut(popup_id) else {
            return;
        };

        let width = popup.width;
        let height = popup.height;
        let surface = popup.xdg.wl_surface().clone();
        popup.dirty = false;

        let Some(pool) = self.pool.as_mut() else {
            return;
        };

        let stride = width * 4;
        let buffer_size = (stride * height) as usize;

        if pool.len() < buffer_size {
            pool.resize(buffer_size).ok();
        }

        let (buffer, canvas_data) = match pool.create_buffer(
            width as i32,
            height as i32,
            stride as i32,
            wl_shm::Format::Argb8888,
        ) {
            Ok((buf, data)) => (buf, data),
            Err(_) => return,
        };

        {
            let mut canvas = Canvas::new(canvas_data, width, height);
            draw(&mut canvas);
            canvas.finalize_for_wayland();
        }

        surface.attach(Some(buffer.wl_buffer()), 0, 0);
        surface.damage_buffer(0, 0, width as i32, height as i32);
        surface.commit();
    }

    pub fn is_window_dirty(&self, window_id: WindowId) -> bool {
        self.windows
            .get_window(window_id)
            .map(|w| w.dirty)
            .unwrap_or(false)
    }

    pub fn is_popup_dirty(&self, popup_id: PopupId) -> bool {
        self.windows
            .get_popup(popup_id)
            .map(|p| p.dirty)
            .unwrap_or(false)
    }

    pub fn is_overlay_dirty(&self, overlay_id: OverlayId) -> bool {
        self.windows
            .get_overlay(overlay_id)
            .map(|o| o.dirty)
            .unwrap_or(false)
    }

    pub fn render_overlay<F>(&mut self, overlay_id: OverlayId, mut draw: F)
    where
        F: FnMut(&mut Canvas),
    {
        let Some(overlay) = self.windows.get_overlay_mut(overlay_id) else {
            return;
        };

        let width = overlay.width;
        let height = overlay.height;
        let surface = overlay.layer.wl_surface().clone();
        overlay.dirty = false;

        let Some(pool) = self.pool.as_mut() else {
            return;
        };

        let stride = width * 4;
        let buffer_size = (stride * height) as usize;

        if pool.len() < buffer_size {
            pool.resize(buffer_size).ok();
        }

        let (buffer, canvas_data) = match pool.create_buffer(
            width as i32,
            height as i32,
            stride as i32,
            wl_shm::Format::Argb8888,
        ) {
            Ok((buf, data)) => (buf, data),
            Err(_) => return,
        };

        {
            let mut canvas = Canvas::new(canvas_data, width, height);
            draw(&mut canvas);
            canvas.finalize_for_wayland();
        }

        surface.attach(Some(buffer.wl_buffer()), 0, 0);
        surface.damage_buffer(0, 0, width as i32, height as i32);
        surface.commit();
    }

    pub fn render_subsurface<F>(&mut self, subsurface_id: SubsurfaceId, mut draw: F)
    where
        F: FnMut(&mut Canvas),
    {
        let Some(subsurface) = self.windows.get_subsurface_mut(subsurface_id) else {
            return;
        };

        let width = subsurface.width;
        let height = subsurface.height;
        let surface = subsurface.surface.clone();
        subsurface.dirty = false;

        let Some(pool) = self.pool.as_mut() else {
            return;
        };

        let stride = width * 4;
        let buffer_size = (stride * height) as usize;

        if pool.len() < buffer_size {
            pool.resize(buffer_size).ok();
        }

        let (buffer, canvas_data) = match pool.create_buffer(
            width as i32,
            height as i32,
            stride as i32,
            wl_shm::Format::Argb8888,
        ) {
            Ok((buf, data)) => (buf, data),
            Err(_) => return,
        };

        {
            let mut canvas = Canvas::new(canvas_data, width, height);
            draw(&mut canvas);
            canvas.finalize_for_wayland();
        }

        surface.attach(Some(buffer.wl_buffer()), 0, 0);
        surface.damage_buffer(0, 0, width as i32, height as i32);
        surface.commit();
    }

    pub fn window_size(&self, window_id: WindowId) -> Option<(u32, u32)> {
        self.windows
            .get_window(window_id)
            .map(|w| (w.width, w.height))
    }

    pub fn flush(&self) {
        let _ = self.conn.flush();
    }

    /// Returns the Wayland connection file descriptor for polling
    pub fn connection_fd(&self) -> std::os::unix::io::RawFd {
        use std::os::unix::io::{AsFd, AsRawFd};
        self.conn.as_fd().as_raw_fd()
    }

    /// Returns the suggested timeout in milliseconds for key repeat
    /// Returns None if no key repeat is pending (can block indefinitely)
    pub fn key_repeat_timeout(&self) -> Option<u32> {
        let start = match (&self.repeat_key, self.repeat_start) {
            (Some(_), Some(s)) => s,
            _ => return None,
        };

        let now = std::time::Instant::now();
        let elapsed = now.duration_since(start);
        let delay = std::time::Duration::from_millis(self.repeat_delay_ms as u64);

        if elapsed < delay {
            // Still in initial delay
            Some((delay - elapsed).as_millis() as u32)
        } else {
            // Past delay, use repeat rate
            let rate = std::time::Duration::from_millis(self.repeat_rate_ms as u64);
            match self.last_repeat {
                None => Some(0), // Should repeat immediately
                Some(last) => {
                    let since_last = now.duration_since(last);
                    if since_last >= rate {
                        Some(0)
                    } else {
                        Some((rate - since_last).as_millis() as u32)
                    }
                }
            }
        }
    }
}

// Implement required sctk traits

impl CompositorHandler for App {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
}

impl OutputHandler for App {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
}

impl WindowHandler for App {
    fn request_close(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, window: &XdgWindow) {
        if let Some(id) = self.windows.find_window_by_surface(window.wl_surface()) {
            self.close_window(id);
            if self.windows.windows.is_empty() {
                self.quit();
            }
        }
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        window: &XdgWindow,
        configure: WindowConfigure,
        _serial: u32,
    ) {
        if let Some(id) = self.windows.find_window_by_surface(window.wl_surface())
            && let Some(w) = self.windows.get_window_mut(id)
        {
            let (width, height) = configure.new_size;
            if let (Some(width), Some(height)) = (width, height) {
                w.width = width.get();
                w.height = height.get();
            }
            w.dirty = true;
        }
    }
}

impl PopupHandler for App {
    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        popup: &XdgPopup,
        _configure: PopupConfigure,
    ) {
        if let Some(id) = self.windows.find_popup_by_surface(popup.wl_surface())
            && let Some(p) = self.windows.get_popup_mut(id)
        {
            p.dirty = true;
        }
    }

    fn done(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, popup: &XdgPopup) {
        eprintln!("[mkframe] Popup done event received (compositor dismissed popup)");
        if let Some(id) = self.windows.find_popup_by_surface(popup.wl_surface()) {
            self.close_popup(id);
        }
    }
}

impl SeatHandler for App {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: smithay_client_toolkit::seat::Capability,
    ) {
        use smithay_client_toolkit::seat::Capability;

        // Store the seat for drag & drop
        if self.current_seat.is_none() {
            self.current_seat = Some(seat.clone());
        }

        if capability == Capability::Keyboard
            && self.seat_state.get_keyboard(qh, &seat, None).is_err()
        {
            eprintln!("[mkframe] Failed to get keyboard");
        }

        if capability == Capability::Pointer && self.seat_state.get_pointer(qh, &seat).is_err() {
            eprintln!("[mkframe] Failed to get pointer");
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wl_seat::WlSeat,
        _capability: smithay_client_toolkit::seat::Capability,
    ) {
    }

    fn remove_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: wl_seat::WlSeat) {
    }
}

impl KeyboardHandler for App {
    fn enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        surface: &wl_surface::WlSurface,
        _serial: u32,
        _raw: &[u32],
        _keysyms: &[Keysym],
    ) {
        self.keyboard_focus = self.windows.find_window_by_surface(surface);
    }

    fn leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _surface: &wl_surface::WlSurface,
        _serial: u32,
    ) {
        self.keyboard_focus = None;
    }

    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        event: SctkKeyEvent,
    ) {
        let key_event = KeyEvent {
            key: Key::from_keysym(event.keysym.raw()),
            text: event.utf8.clone(),
            modifiers: self.current_modifiers,
            state: KeyState::Pressed,
        };
        self.key_events.push(key_event.clone());

        // Start tracking for key repeat (only for non-modifier keys)
        if !matches!(
            key_event.key,
            Key::Shift | Key::Control | Key::Alt | Key::Super
        ) {
            self.repeat_key = Some(key_event);
            self.repeat_start = Some(std::time::Instant::now());
            self.last_repeat = None;
        }
    }

    fn release_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        event: SctkKeyEvent,
    ) {
        let key_event = KeyEvent {
            key: Key::from_keysym(event.keysym.raw()),
            text: event.utf8.clone(),
            modifiers: self.current_modifiers,
            state: KeyState::Released,
        };

        // Stop repeat if releasing the repeated key
        if let Some(ref repeat_key) = self.repeat_key
            && repeat_key.key == key_event.key
        {
            self.repeat_key = None;
            self.repeat_start = None;
            self.last_repeat = None;
        }

        self.key_events.push(key_event);
    }

    fn update_modifiers(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        modifiers: Modifiers,
        _layout: u32,
    ) {
        self.current_modifiers = InputModifiers {
            shift: modifiers.shift,
            ctrl: modifiers.ctrl,
            alt: modifiers.alt,
            super_: modifiers.logo,
        };
    }
}

impl PointerHandler for App {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        events: &[SctkPointerEvent],
    ) {
        use crate::input::{PointerButton, PointerEvent, PointerEventKind};
        use smithay_client_toolkit::seat::pointer::PointerEventKind as SctkPointerEventKind;

        for event in events {
            let (x, y) = event.position;

            match &event.kind {
                SctkPointerEventKind::Enter { .. } => {
                    // Try to find which window this surface belongs to
                    self.pointer_focus = self.windows.find_window_by_surface(&event.surface);
                    self.pointer_x = x;
                    self.pointer_y = y;
                    self.pointer_events.push(PointerEvent {
                        kind: PointerEventKind::Enter,
                        x,
                        y,
                    });
                }
                SctkPointerEventKind::Leave { .. } => {
                    self.pointer_focus = None;
                    self.pointer_events.push(PointerEvent {
                        kind: PointerEventKind::Leave,
                        x: self.pointer_x,
                        y: self.pointer_y,
                    });
                }
                SctkPointerEventKind::Motion { .. } => {
                    self.pointer_x = x;
                    self.pointer_y = y;
                    self.pointer_events.push(PointerEvent {
                        kind: PointerEventKind::Motion,
                        x,
                        y,
                    });
                }
                SctkPointerEventKind::Press { button, serial, .. } => {
                    self.last_serial = *serial;
                    let btn = match button {
                        272 => PointerButton::Left,   // BTN_LEFT
                        273 => PointerButton::Right,  // BTN_RIGHT
                        274 => PointerButton::Middle, // BTN_MIDDLE
                        other => PointerButton::Other(*other),
                    };
                    self.pointer_events.push(PointerEvent {
                        kind: PointerEventKind::Press(btn),
                        x: self.pointer_x,
                        y: self.pointer_y,
                    });
                }
                SctkPointerEventKind::Release { button, .. } => {
                    let btn = match button {
                        272 => PointerButton::Left,
                        273 => PointerButton::Right,
                        274 => PointerButton::Middle,
                        other => PointerButton::Other(*other),
                    };
                    self.pointer_events.push(PointerEvent {
                        kind: PointerEventKind::Release(btn),
                        x: self.pointer_x,
                        y: self.pointer_y,
                    });
                }
                SctkPointerEventKind::Axis {
                    horizontal,
                    vertical,
                    ..
                } => {
                    // Convert discrete scroll amounts to deltas
                    let dx = horizontal.discrete;
                    let dy = vertical.discrete;
                    if dx != 0 || dy != 0 {
                        self.pointer_events.push(PointerEvent {
                            kind: PointerEventKind::Scroll { dx, dy },
                            x: self.pointer_x,
                            y: self.pointer_y,
                        });
                    }
                }
            }
        }
    }
}

// Data device handlers for drag & drop support
impl DataDeviceHandler for App {
    fn enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _data_device: &smithay_client_toolkit::reexports::client::protocol::wl_data_device::WlDataDevice,
        _x: f64,
        _y: f64,
        _wl_surface: &wl_surface::WlSurface,
    ) {
        // A drag has entered our surface
    }

    fn leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _data_device: &smithay_client_toolkit::reexports::client::protocol::wl_data_device::WlDataDevice,
    ) {
        // Drag left our surface
    }

    fn motion(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _data_device: &smithay_client_toolkit::reexports::client::protocol::wl_data_device::WlDataDevice,
        _x: f64,
        _y: f64,
    ) {
        // Drag is moving over our surface
    }

    fn selection(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _data_device: &smithay_client_toolkit::reexports::client::protocol::wl_data_device::WlDataDevice,
    ) {
        // Selection (clipboard) changed
    }

    fn drop_performed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _data_device: &smithay_client_toolkit::reexports::client::protocol::wl_data_device::WlDataDevice,
    ) {
        // Drop was performed
    }
}

impl DataOfferHandler for App {
    fn source_actions(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _offer: &mut smithay_client_toolkit::data_device_manager::data_offer::DragOffer,
        _actions: smithay_client_toolkit::reexports::client::protocol::wl_data_device_manager::DndAction,
    ) {
    }

    fn selected_action(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _offer: &mut smithay_client_toolkit::data_device_manager::data_offer::DragOffer,
        _action: smithay_client_toolkit::reexports::client::protocol::wl_data_device_manager::DndAction,
    ) {
    }
}

impl DataSourceHandler for App {
    fn accept_mime(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _source: &smithay_client_toolkit::reexports::client::protocol::wl_data_source::WlDataSource,
        _mime: Option<String>,
    ) {
        // Destination accepted a MIME type
    }

    fn send_request(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _source: &smithay_client_toolkit::reexports::client::protocol::wl_data_source::WlDataSource,
        mime: String,
        mut fd: WritePipe,
    ) {
        // Receiver requested data - write to fd
        if mime == "text/uri-list"
            && let Some(ref data) = self.pending_drag_data
        {
            use std::io::Write;
            let _ = fd.write_all(data);
        }
        // fd is automatically closed when dropped
    }

    fn cancelled(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _source: &smithay_client_toolkit::reexports::client::protocol::wl_data_source::WlDataSource,
    ) {
        self.pending_drag_source = None;
        self.pending_drag_data = None;
    }

    fn dnd_dropped(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _source: &smithay_client_toolkit::reexports::client::protocol::wl_data_source::WlDataSource,
    ) {
    }

    fn dnd_finished(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _source: &smithay_client_toolkit::reexports::client::protocol::wl_data_source::WlDataSource,
    ) {
        self.pending_drag_source = None;
        self.pending_drag_data = None;
    }

    fn action(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _source: &smithay_client_toolkit::reexports::client::protocol::wl_data_source::WlDataSource,
        _action: smithay_client_toolkit::reexports::client::protocol::wl_data_device_manager::DndAction,
    ) {
    }
}

impl ShmHandler for App {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl LayerShellHandler for App {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, layer: &LayerSurface) {
        if let Some(id) = self.windows.find_overlay_by_surface(layer.wl_surface()) {
            self.close_overlay(id);
        }
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        if let Some(id) = self.windows.find_overlay_by_surface(layer.wl_surface())
            && let Some(overlay) = self.windows.get_overlay_mut(id)
        {
            if configure.new_size.0 > 0 {
                overlay.width = configure.new_size.0;
            }
            if configure.new_size.1 > 0 {
                overlay.height = configure.new_size.1;
            }
            overlay.dirty = true;
        }
    }
}

impl ProvidesRegistryState for App {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState];
}

smithay_client_toolkit::delegate_compositor!(App);
smithay_client_toolkit::delegate_output!(App);
smithay_client_toolkit::delegate_shm!(App);
smithay_client_toolkit::delegate_seat!(App);
smithay_client_toolkit::delegate_keyboard!(App);
smithay_client_toolkit::delegate_pointer!(App);
smithay_client_toolkit::delegate_data_device!(App);
smithay_client_toolkit::delegate_xdg_shell!(App);
smithay_client_toolkit::delegate_xdg_window!(App);
smithay_client_toolkit::delegate_xdg_popup!(App);
smithay_client_toolkit::delegate_layer!(App);
smithay_client_toolkit::delegate_registry!(App);

// WlSubcompositor has no events - it's a factory interface
impl Dispatch<wl_subcompositor::WlSubcompositor, ()> for App {
    fn event(
        _state: &mut Self,
        _proxy: &wl_subcompositor::WlSubcompositor,
        _event: wl_subcompositor::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // No events defined for wl_subcompositor
    }
}

// WlSubsurface has no events - position/stacking are client-side only
impl Dispatch<wl_subsurface::WlSubsurface, ()> for App {
    fn event(
        _state: &mut Self,
        _proxy: &wl_subsurface::WlSubsurface,
        _event: wl_subsurface::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // No events defined for wl_subsurface
    }
}

// Attached surface handler implementation
impl AttachedSurfaceHandler for App {
    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        surface_id: AttachedSurfaceId,
        serial: u32,
        width: u32,
        height: u32,
    ) {
        if let Some(attached) = self.windows.get_attached_surface_mut(surface_id) {
            // Only update dimensions if compositor provides non-zero size
            // Otherwise keep our requested dimensions
            if width > 0 && height > 0 {
                attached.width = width;
                attached.height = height;
            }
            attached.ack_configure(serial);
            attached.dirty = true;
        }
    }

    fn closed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        surface_id: AttachedSurfaceId,
    ) {
        self.close_attached_surface(surface_id);
    }
}

// Attached surface manager has no events
impl Dispatch<ZwlrAttachedSurfaceManagerV1, ()> for App {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrAttachedSurfaceManagerV1,
        _event: crate::attached_surface::protocol::zwlr_attached_surface_manager_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // Manager has no events
    }
}

// Attached surface events (configure, closed)
impl Dispatch<ZwlrAttachedSurfaceV1, AttachedSurfaceData> for App {
    fn event(
        state: &mut Self,
        _proxy: &ZwlrAttachedSurfaceV1,
        event: crate::attached_surface::protocol::zwlr_attached_surface_v1::Event,
        data: &AttachedSurfaceData,
        conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        use crate::attached_surface::protocol::zwlr_attached_surface_v1::Event;
        match event {
            Event::Configure {
                serial,
                width,
                height,
            } => {
                AttachedSurfaceHandler::configure(state, conn, qh, data.id, serial, width, height);
            }
            Event::Closed => {
                AttachedSurfaceHandler::closed(state, conn, qh, data.id);
            }
        }
    }
}
