//! Display an "ADHITHYA" Code39 barcode in a native macOS window.
//!
//! Run with:
//!   cargo run --example barcode_window -p draw-instructions-metal-mac-window

fn main() {
    let config = code39::default_render_config();
    let scene = code39::draw_code39("ADHITHYA", &config)
        .expect("ADHITHYA should be valid Code 39 input");

    println!("Opening window with {}×{} barcode...", scene.width, scene.height);
    draw_instructions_metal_mac_window::show_in_window(&scene);
    println!("Window closed.");
}
