//! End-to-end example: Mermaid flowchart -> PaintScene -> Metal -> PNG file.
//!
//! Run with:
//!   cargo run --example mermaid_flowchart -p diagram-to-paint
//!
//! Output: /tmp/mermaid_flowchart.png

#[cfg(target_vendor = "apple")]
fn main() {
    use diagram_layout_graph::layout_graph_diagram;
    use diagram_to_paint::{diagram_to_paint, DiagramToPaintOptions};
    use layout_ir::font_spec;
    use mermaid_parser::parse_to_diagram;
    use paint_codec_png::write_png;
    use paint_metal::render;
    use text_native_coretext::{CoreTextMetrics, CoreTextResolver, CoreTextShaper};

    let mermaid = r#"
        flowchart LR
        A[Mermaid] --> B{Diagram IR}
        B -->|layout| C((PaintScene))
        C --> D[Metal PNG]
    "#;

    let diagram = parse_to_diagram(mermaid).expect("Mermaid parse failed");
    let layout = layout_graph_diagram(&diagram, None, None);

    let shaper = CoreTextShaper;
    let metrics = CoreTextMetrics;
    let resolver = CoreTextResolver::new();

    let scene = diagram_to_paint(
        &layout,
        &DiagramToPaintOptions {
            background: layout_ir::Color {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            device_pixel_ratio: 2.0,
            label_font: font_spec("Helvetica", 14.0),
            title_font: {
                let mut f = font_spec("Helvetica", 18.0);
                f.weight = 700;
                f
            },
            shaper: &shaper,
            metrics: &metrics,
            resolver: &resolver,
        },
    );

    let pixels = render(&scene);
    let path = "/tmp/mermaid_flowchart.png";
    write_png(&pixels, path).expect("Failed to write PNG");

    println!("Rendered Mermaid flowchart to {path}");
    println!("Scene: {}×{} pixels", scene.width, scene.height);
    println!("Instructions: {} paint commands", scene.instructions.len());
}

#[cfg(not(target_vendor = "apple"))]
fn main() {
    panic!("The mermaid_flowchart example requires an Apple target because it renders through paint-metal.");
}
