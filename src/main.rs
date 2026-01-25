mod ui;

use std::time::Duration;

use ui::{Color, Graphics, InputSource, RatatuiBackend, Rect, Style};

fn main() -> std::io::Result<()> {
    let mut backend = RatatuiBackend::new()?;
    backend.start()?;

    let result = run(&mut backend);

    backend.stop()?;
    result
}

fn run(backend: &mut RatatuiBackend) -> std::io::Result<()> {
    loop {
        // Poll for input (non-blocking with short timeout for ~60fps)
        if let Some(event) = backend.poll_event(Duration::from_millis(16)) {
            // Handle 'q' to quit
            if event.is_char('q') {
                break;
            }
        }

        // Begin frame
        let mut frame = backend.begin_frame()?;

        // Get terminal size and calculate centered box
        let (width, height) = frame.size();
        let box_width = 30;
        let box_height = 10;
        let rect = Rect::centered(width, height, box_width, box_height);

        // Set style and draw box (use black for light terminals)
        frame.set_style(Style::new().fg(Color::BLACK));
        frame.draw_box(rect, Some(" tuidaw "));

        // End frame and render
        backend.end_frame(frame)?;
    }

    Ok(())
}
