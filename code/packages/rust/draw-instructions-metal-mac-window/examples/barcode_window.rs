//! Display a manual barcode-style draw-instructions scene in a native macOS window.
//!
//! The real barcode pipeline now lives in `paint-metal` plus `code39::layout_code39`.

use draw_instructions::{create_scene, draw_rect, Metadata};

fn main() {
    let scene = create_scene(
        132,
        120,
        vec![
            draw_rect(20, 0, 8, 120, "#000000", Metadata::new()),
            draw_rect(32, 0, 8, 120, "#000000", Metadata::new()),
            draw_rect(52, 0, 12, 120, "#000000", Metadata::new()),
            draw_rect(72, 0, 8, 120, "#000000", Metadata::new()),
            draw_rect(84, 0, 12, 120, "#000000", Metadata::new()),
            draw_rect(104, 0, 8, 120, "#000000", Metadata::new()),
        ],
        "#ffffff",
        Metadata::new(),
    );

    println!(
        "Opening window with {}×{} scene...",
        scene.width, scene.height
    );
    draw_instructions_metal_mac_window::show_in_window(&scene);
    println!("Window closed.");
}
