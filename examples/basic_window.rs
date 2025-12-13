use mkframe::{App, Color};

fn draw(canvas: &mut mkframe::Canvas) {
    // Dark blue background
    canvas.clear(Color::from_rgba8(30, 40, 60, 255));
    // Red rectangle
    canvas.fill_rect(
        100.0,
        100.0,
        200.0,
        150.0,
        Color::from_rgba8(200, 50, 50, 255),
    );
    // Green rectangle
    canvas.fill_rect(
        350.0,
        200.0,
        150.0,
        200.0,
        Color::from_rgba8(50, 200, 50, 255),
    );
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (mut app, mut event_queue) = App::new()?;
    let qh = event_queue.handle();

    let window_id = app.create_window(&qh, "mkframe - Basic Window", 800, 600);

    // Event loop - wait for configure before first render
    while app.running {
        event_queue.blocking_dispatch(&mut app)?;

        // Render when window is dirty (configured/resized)
        if app.is_window_dirty(window_id) {
            app.render_window(window_id, draw);
            app.flush();
        }
    }

    Ok(())
}
