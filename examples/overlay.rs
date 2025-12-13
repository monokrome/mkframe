use mkframe::{Anchor, App, Color, KeyboardInteractivity, Layer};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (mut app, mut event_queue) = App::new()?;
    let qh = event_queue.handle();

    // Check if layer-shell is available
    if !app.has_layer_shell() {
        eprintln!("Layer shell not available - this compositor doesn't support wlr-layer-shell");
        eprintln!("Try running on a wlroots-based compositor (Sway, Hyprland, dwl, etc.)");
        return Ok(());
    }

    println!("Creating layer-shell overlay...");

    // Create an overlay anchored to the right edge of the screen
    // Using Layer::Top (not Layer::Overlay which blocks all input like a lock screen)
    let overlay_id = app
        .create_overlay(
            &qh,
            300,                         // width
            200,                         // height
            Layer::Top, // layer level - Top is above normal windows but doesn't capture all input
            Anchor::RIGHT | Anchor::TOP, // anchor to top-right
            (50, 20, 0, 0), // margins: top, right, bottom, left
            KeyboardInteractivity::None, // no keyboard grab - overlay is display-only
        )
        .expect("Failed to create overlay");

    println!("Overlay created! It should appear at the top-right of your screen.");
    println!("Close the overlay window or press Ctrl+C to exit.");

    while app.running {
        event_queue.blocking_dispatch(&mut app)?;

        if app.is_overlay_dirty(overlay_id) {
            app.render_overlay(overlay_id, |canvas| {
                // Semi-transparent dark background
                canvas.clear(Color::from_rgba8(30, 30, 35, 230));

                // Title bar area
                canvas.fill_rect(0.0, 0.0, 300.0, 30.0, Color::from_rgba8(50, 50, 60, 255));

                // Content area - some colored bars
                canvas.fill_rect(
                    15.0,
                    45.0,
                    270.0,
                    25.0,
                    Color::from_rgba8(70, 130, 180, 255),
                );
                canvas.fill_rect(
                    15.0,
                    80.0,
                    270.0,
                    25.0,
                    Color::from_rgba8(100, 160, 210, 255),
                );
                canvas.fill_rect(
                    15.0,
                    115.0,
                    270.0,
                    25.0,
                    Color::from_rgba8(130, 190, 240, 255),
                );
                canvas.fill_rect(
                    15.0,
                    150.0,
                    270.0,
                    25.0,
                    Color::from_rgba8(160, 210, 250, 255),
                );
            });
            app.flush();
        }
    }

    Ok(())
}
