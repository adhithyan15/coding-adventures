//! End-to-end example: Code39 barcode → Metal GPU render → PNG file
//!
//! This example encodes "ADHITHYA" as a Code 39 barcode, renders it
//! using the Metal GPU, and writes the result to a PNG file.
//!
//! Run with:
//!   cargo run --example barcode_adhithya -p draw-instructions-metal
//!
//! Output: /tmp/adhithya_barcode.png

fn main() {
    // Step 1: Generate a Code39 barcode scene for "ADHITHYA"
    //
    // Code 39 supports uppercase letters, digits, and a few symbols.
    // The encoder wraps the input with start/stop markers (*ADHITHYA*)
    // and produces a DrawScene containing rectangles (bars) and text.
    let config = code39::default_render_config();
    let scene = code39::draw_code39("ADHITHYA", &config)
        .expect("ADHITHYA should be valid Code 39 input");

    println!("Scene: {}×{} pixels", scene.width, scene.height);
    println!("Instructions: {} draw commands", scene.instructions.len());

    // Step 2: Render the scene to pixels using Metal
    //
    // The Metal renderer:
    //   1. Creates a GPU device
    //   2. Compiles MSL shaders
    //   3. Converts rectangles to triangle vertices
    //   4. Rasterizes text via CoreText
    //   5. Executes the GPU render pass
    //   6. Reads back the pixels to CPU memory
    let pixels = draw_instructions_metal::render_metal(&scene);

    println!("Rendered: {}×{} pixels ({} bytes)",
        pixels.width, pixels.height, pixels.byte_count());

    // Step 3: Encode the pixels as PNG and write to file
    //
    // The PNG encoder takes the PixelBuffer (RGBA8, row-major) and
    // produces a compressed PNG file.  No conversion needed — the
    // pixel format matches what PNG expects.
    let path = "/tmp/adhithya_barcode.png";
    draw_instructions_png::write_png(&pixels, path)
        .expect("Failed to write PNG");

    println!("Written to: {}", path);
    println!("Done! Open the file to see the barcode.");
}
