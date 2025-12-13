use std::collections::HashMap;

use smithay_client_toolkit::{
    reexports::client::protocol::{wl_subsurface, wl_surface},
    shell::{
        WaylandSurface,
        wlr_layer::LayerSurface,
        xdg::{popup::Popup as XdgPopup, window::Window as XdgWindow},
    },
};
use wayland_protocols::xdg::shell::client::xdg_positioner::{Anchor, Gravity};

use crate::attached_surface::{AttachedSurface, AttachedSurfaceId};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WindowId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PopupId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct OverlayId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SubsurfaceId(pub u64);

pub struct Window {
    pub id: WindowId,
    pub xdg: XdgWindow,
    pub width: u32,
    pub height: u32,
    pub dirty: bool,
}

impl Window {
    pub fn surface(&self) -> &wl_surface::WlSurface {
        self.xdg.wl_surface()
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}

#[derive(Clone, Debug)]
pub struct PopupConfig {
    pub anchor: PopupAnchor,
    pub gravity: PopupGravity,
    pub offset: (i32, i32),
    pub size: (u32, u32),
    pub anchor_rect: Option<(i32, i32, i32, i32)>,
}

impl Default for PopupConfig {
    fn default() -> Self {
        Self {
            anchor: PopupAnchor::Right,
            gravity: PopupGravity::Right,
            offset: (0, 0),
            size: (200, 200),
            anchor_rect: None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum PopupAnchor {
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl From<PopupAnchor> for Anchor {
    fn from(a: PopupAnchor) -> Self {
        match a {
            PopupAnchor::Top => Anchor::Top,
            PopupAnchor::Bottom => Anchor::Bottom,
            PopupAnchor::Left => Anchor::Left,
            PopupAnchor::Right => Anchor::Right,
            PopupAnchor::TopLeft => Anchor::TopLeft,
            PopupAnchor::TopRight => Anchor::TopRight,
            PopupAnchor::BottomLeft => Anchor::BottomLeft,
            PopupAnchor::BottomRight => Anchor::BottomRight,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum PopupGravity {
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl From<PopupGravity> for Gravity {
    fn from(g: PopupGravity) -> Self {
        match g {
            PopupGravity::Top => Gravity::Top,
            PopupGravity::Bottom => Gravity::Bottom,
            PopupGravity::Left => Gravity::Left,
            PopupGravity::Right => Gravity::Right,
            PopupGravity::TopLeft => Gravity::TopLeft,
            PopupGravity::TopRight => Gravity::TopRight,
            PopupGravity::BottomLeft => Gravity::BottomLeft,
            PopupGravity::BottomRight => Gravity::BottomRight,
        }
    }
}

pub struct Popup {
    pub id: PopupId,
    pub parent: WindowId,
    pub xdg: XdgPopup,
    pub width: u32,
    pub height: u32,
    pub dirty: bool,
}

impl Popup {
    pub fn surface(&self) -> &wl_surface::WlSurface {
        self.xdg.wl_surface()
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}

pub struct Overlay {
    pub id: OverlayId,
    pub layer: LayerSurface,
    pub width: u32,
    pub height: u32,
    pub dirty: bool,
}

impl Overlay {
    pub fn surface(&self) -> &wl_surface::WlSurface {
        self.layer.wl_surface()
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}

pub struct Subsurface {
    pub id: SubsurfaceId,
    pub parent: WindowId,
    pub surface: wl_surface::WlSurface,
    pub subsurface: wl_subsurface::WlSubsurface,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub dirty: bool,
}

impl Subsurface {
    pub fn wl_surface(&self) -> &wl_surface::WlSurface {
        &self.surface
    }

    pub fn set_position(&self, x: i32, y: i32) {
        self.subsurface.set_position(x, y);
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}

pub struct WindowManager {
    pub windows: HashMap<WindowId, Window>,
    pub popups: HashMap<PopupId, Popup>,
    pub overlays: HashMap<OverlayId, Overlay>,
    pub subsurfaces: HashMap<SubsurfaceId, Subsurface>,
    pub attached_surfaces: HashMap<AttachedSurfaceId, AttachedSurface>,
    next_window_id: u64,
    next_popup_id: u64,
    next_overlay_id: u64,
    next_subsurface_id: u64,
    next_attached_surface_id: u64,
}

impl Default for WindowManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowManager {
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
            popups: HashMap::new(),
            overlays: HashMap::new(),
            subsurfaces: HashMap::new(),
            attached_surfaces: HashMap::new(),
            next_window_id: 1,
            next_popup_id: 1,
            next_overlay_id: 1,
            next_subsurface_id: 1,
            next_attached_surface_id: 1,
        }
    }

    pub fn next_window_id(&mut self) -> WindowId {
        let id = WindowId(self.next_window_id);
        self.next_window_id += 1;
        id
    }

    pub fn next_popup_id(&mut self) -> PopupId {
        let id = PopupId(self.next_popup_id);
        self.next_popup_id += 1;
        id
    }

    pub fn next_overlay_id(&mut self) -> OverlayId {
        let id = OverlayId(self.next_overlay_id);
        self.next_overlay_id += 1;
        id
    }

    pub fn next_subsurface_id(&mut self) -> SubsurfaceId {
        let id = SubsurfaceId(self.next_subsurface_id);
        self.next_subsurface_id += 1;
        id
    }

    pub fn next_attached_surface_id(&mut self) -> AttachedSurfaceId {
        let id = AttachedSurfaceId(self.next_attached_surface_id);
        self.next_attached_surface_id += 1;
        id
    }

    pub fn get_window(&self, id: WindowId) -> Option<&Window> {
        self.windows.get(&id)
    }

    pub fn get_window_mut(&mut self, id: WindowId) -> Option<&mut Window> {
        self.windows.get_mut(&id)
    }

    pub fn get_popup(&self, id: PopupId) -> Option<&Popup> {
        self.popups.get(&id)
    }

    pub fn get_popup_mut(&mut self, id: PopupId) -> Option<&mut Popup> {
        self.popups.get_mut(&id)
    }

    pub fn get_overlay(&self, id: OverlayId) -> Option<&Overlay> {
        self.overlays.get(&id)
    }

    pub fn get_overlay_mut(&mut self, id: OverlayId) -> Option<&mut Overlay> {
        self.overlays.get_mut(&id)
    }

    pub fn get_subsurface(&self, id: SubsurfaceId) -> Option<&Subsurface> {
        self.subsurfaces.get(&id)
    }

    pub fn get_subsurface_mut(&mut self, id: SubsurfaceId) -> Option<&mut Subsurface> {
        self.subsurfaces.get_mut(&id)
    }

    pub fn find_window_by_surface(&self, surface: &wl_surface::WlSurface) -> Option<WindowId> {
        self.windows
            .iter()
            .find(|(_, w)| w.surface() == surface)
            .map(|(id, _)| *id)
    }

    pub fn find_popup_by_surface(&self, surface: &wl_surface::WlSurface) -> Option<PopupId> {
        self.popups
            .iter()
            .find(|(_, p)| p.surface() == surface)
            .map(|(id, _)| *id)
    }

    pub fn find_overlay_by_surface(&self, surface: &wl_surface::WlSurface) -> Option<OverlayId> {
        self.overlays
            .iter()
            .find(|(_, o)| o.surface() == surface)
            .map(|(id, _)| *id)
    }

    pub fn find_subsurface_by_surface(
        &self,
        surface: &wl_surface::WlSurface,
    ) -> Option<SubsurfaceId> {
        self.subsurfaces
            .iter()
            .find(|(_, s)| s.wl_surface() == surface)
            .map(|(id, _)| *id)
    }

    pub fn get_attached_surface(&self, id: AttachedSurfaceId) -> Option<&AttachedSurface> {
        self.attached_surfaces.get(&id)
    }

    pub fn get_attached_surface_mut(
        &mut self,
        id: AttachedSurfaceId,
    ) -> Option<&mut AttachedSurface> {
        self.attached_surfaces.get_mut(&id)
    }

    pub fn find_attached_surface_by_surface(
        &self,
        surface: &wl_surface::WlSurface,
    ) -> Option<AttachedSurfaceId> {
        self.attached_surfaces
            .iter()
            .find(|(_, s)| s.wl_surface() == surface)
            .map(|(id, _)| *id)
    }
}
