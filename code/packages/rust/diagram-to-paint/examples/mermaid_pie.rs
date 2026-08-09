//! End-to-end example: Mermaid pie -> PaintScene -> Metal -> PNG.
//!
//! Run with:
//!   cargo run --example mermaid_pie -p diagram-to-paint
//!
//! Output: /tmp/mermaid_pie.png

#[cfg(target_vendor = "apple")]
fn main() {
    use diagram_layout_chart::layout_chart_diagram;
    use diagram_to_paint::{diagram_to_paint_chart, DiagramToPaintOptions};
    use layout_ir::font_spec;
    use mermaid_parser::parse_pie;
    use paint_codec_png::write_png;
    use paint_metal::render;
    use text_native_coretext::{CoreTextMetrics, CoreTextResolver, CoreTextShaper};

    let source = r#"pie showData
  "Flowchart" : 40
  "Class" : 20
  "Gantt" : 20
  "XY Chart" : 10
  "Pie" : 10"#;

    let diagram = parse_pie(source).expect("Mermaid pie parse failed");
    let layout = layout_chart_diagram(&diagram, 640.0, 480.0);
    let shaper = CoreTextShaper;
    let metrics = CoreTextMetrics;
    let resolver = CoreTextResolver::new();

    let scene = diagram_to_paint_chart(
        &layout,
        &DiagramToPaintOptions {
            background: layout_ir::Color {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            device_pixel_ratio: 2.0,
            label_font: font_spec("Helvetica", 12.0),
            title_font: {
                let mut font = font_spec("Helvetica", 16.0);
                font.weight = 700;
                font
            },
            shaper: &shaper,
            metrics: &metrics,
            resolver: &resolver,
        },
    );

    let path = "/tmp/mermaid_pie.png";
    write_png(&render(&scene), path).expect("PNG write failed");
    println!("Rendered Mermaid pie to {path}");
    println!(
        "Scene: {}x{} px, {} chart items",
        scene.width,
        scene.height,
        layout.items.len()
    );
}

#[cfg(not(target_vendor = "apple"))]
fn main() {
    panic!("mermaid_pie example requires an Apple target (paint-metal)");
}
