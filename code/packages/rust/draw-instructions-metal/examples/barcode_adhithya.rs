//! Legacy draw-instructions example: manual barcode-style scene -> Metal -> PNG.
//!
//! The real barcode pipeline now lives in `paint-metal` plus `code39::layout_code39`.
//! This example stays on the legacy stack intentionally so the old renderer
//! still has a minimal manual scene to exercise.

use draw_instructions::{create_scene, draw_rect, Metadata};

fn main() {
    let instructions = vec![
        draw_rect(20, 0, 8, 120, "#000000", Metadata::new()),
        draw_rect(32, 0, 8, 120, "#000000", Metadata::new()),
        draw_rect(52, 0, 12, 120, "#000000", Metadata::new()),
        draw_rect(72, 0, 8, 120, "#000000", Metadata::new()),
        draw_rect(84, 0, 12, 120, "#000000", Metadata::new()),
        draw_rect(104, 0, 8, 120, "#000000", Metadata::new()),
    ];

    let mut metadata = Metadata::new();
    metadata.insert("label".into(), "Legacy manual barcode scene".into());
    let scene = create_scene(132, 120, instructions, "#ffffff", metadata);

    println!("Scene: {}×{} pixels", scene.width, scene.height);
    println!("Instructions: {} draw commands", scene.instructions.len());

    let pixels = draw_instructions_metal::render_metal(&scene);
    println!(
        "Rendered: {}×{} pixels ({} bytes)",
        pixels.width,
        pixels.height,
        pixels.byte_count()
    );

    let path = "/tmp/adhithya_barcode.png";
    draw_instructions_png::write_png(&pixels, path).expect("Failed to write PNG");

    println!("Written to: {}", path);
    println!("Done! Open the file to see the barcode-style scene.");
}
