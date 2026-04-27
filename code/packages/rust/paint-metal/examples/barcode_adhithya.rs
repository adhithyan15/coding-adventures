//! End-to-end example: Code 39 barcode -> PaintScene -> Metal -> PNG file.
//!
//! Run with:
//!   cargo run --example barcode_adhithya -p paint-metal
//!
//! Output: /tmp/adhithya_barcode.png

fn main() {
    let scene = code39::layout_code39("ADHITHYA", &code39::PaintBarcode1DOptions::default())
        .expect("ADHITHYA should be valid Code 39 input");

    println!("Scene: {}×{} pixels", scene.width, scene.height);
    println!("Instructions: {} paint commands", scene.instructions.len());

    let pixels = paint_metal::render(&scene);
    println!(
        "Rendered: {}×{} pixels ({} bytes)",
        pixels.width,
        pixels.height,
        pixels.byte_count()
    );

    let path = "/tmp/adhithya_barcode.png";
    paint_codec_png::write_png(&pixels, path).expect("Failed to write PNG");

    println!("Written to: {}", path);
    println!("Done! Open the file to see the barcode.");
}
