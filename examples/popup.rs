use mkframe::{App, Color, PopupAnchor, PopupConfig, PopupGravity, PopupId};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (mut app, mut event_queue) = App::new()?;
    let qh = event_queue.handle();

    let window_id = app.create_window(&qh, "mkframe - Popup Demo", 800, 600);

    let mut popup_id: Option<PopupId> = None;
    let mut popup_created = false;

    println!("Popup anchored to right edge of window (close window to exit)");
    println!("The popup position is set once - it won't move with resize");

    while app.running {
        event_queue.blocking_dispatch(&mut app)?;

        // Render main window when dirty
        if app.is_window_dirty(window_id) {
            app.render_window(window_id, |canvas| {
                canvas.clear(Color::from_rgba8(40, 40, 45, 255));
            });
            app.flush();

            // Create popup once after first configure
            if !popup_created {
                let (w, h) = app.window_size(window_id).unwrap_or((800, 600));

                // Anchor to right edge, vertically centered
                let config = PopupConfig {
                    anchor: PopupAnchor::Right,
                    gravity: PopupGravity::Right,
                    offset: (5, 0),
                    size: (250, 150),
                    anchor_rect: Some((w as i32 - 10, (h as i32 / 2) - 50, 10, 100)),
                };

                if let Some(id) = app.create_popup(&qh, window_id, config) {
                    popup_id = Some(id);
                    popup_created = true;
                    println!("Popup created at right edge (window {}x{})", w, h);
                }
            }
        }

        // Render popup when dirty
        if let Some(pid) = popup_id {
            if app.is_popup_dirty(pid) {
                app.render_popup(pid, |canvas| {
                    canvas.clear(Color::from_rgba8(250, 250, 245, 255));

                    // Border
                    let w = 250.0;
                    let h = 150.0;
                    canvas.fill_rect(0.0, 0.0, w, 2.0, Color::from_rgba8(100, 100, 120, 255));
                    canvas.fill_rect(0.0, h - 2.0, w, 2.0, Color::from_rgba8(100, 100, 120, 255));
                    canvas.fill_rect(0.0, 0.0, 2.0, h, Color::from_rgba8(100, 100, 120, 255));
                    canvas.fill_rect(w - 2.0, 0.0, 2.0, h, Color::from_rgba8(100, 100, 120, 255));

                    // Content bars
                    canvas.fill_rect(
                        15.0,
                        15.0,
                        220.0,
                        25.0,
                        Color::from_rgba8(70, 130, 180, 255),
                    );
                    canvas.fill_rect(
                        15.0,
                        50.0,
                        220.0,
                        25.0,
                        Color::from_rgba8(100, 160, 210, 255),
                    );
                    canvas.fill_rect(
                        15.0,
                        85.0,
                        220.0,
                        25.0,
                        Color::from_rgba8(130, 190, 240, 255),
                    );
                });
                app.flush();
            }
        }
    }

    Ok(())
}
