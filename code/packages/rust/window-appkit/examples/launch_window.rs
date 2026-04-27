use window_appkit::AppKitBackend;
use window_core::{LogicalSize, SurfacePreference, Window, WindowBuilder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut backend = AppKitBackend::new();
    let window = WindowBuilder::new()
        .title("window-appkit smoke test")
        .initial_size(LogicalSize::new(480.0, 320.0))
        .preferred_surface(SurfacePreference::Default)
        .build_with(&mut backend)?;

    println!(
        "Launched AppKit window {:?} with target kind {}",
        window.id(),
        window.render_target().kind()
    );

    // Keep the smoke test quick and non-interactive.
    backend.terminate_after(1.5)?;
    backend.run()?;
    Ok(())
}
