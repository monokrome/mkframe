# mkframe

A minimal Wayland UI toolkit with proper popup/overlay support.

## Features

- Native Wayland support via smithay-client-toolkit
- GPU-accelerated rendering (wgpu) with software fallback (tiny-skia)
- Layer shell support for panels, overlays, and desktop widgets
- Popup and overlay windows with proper positioning
- Split pane layouts
- Text rendering with cosmic-text
- Keyboard and pointer input handling
- Drag and drop support

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
mkframe = "0.1"
```

## Requirements

- Linux with Wayland compositor
- System dependencies:
  - `libwayland-dev`
  - `libxkbcommon-dev`
  - `libfontconfig1-dev`

## License

GPL-3.0
