//! End-to-end example: Mermaid classDiagram → PaintScene → Metal → PNG
//!
//! Run with:
//!   cargo run --example class_diagram -p diagram-to-paint
//!
//! Output: /tmp/class_diagram.png

#[cfg(target_vendor = "apple")]
fn main() {
    use diagram_layout_structural::layout_structural_diagram;
    use diagram_to_paint::{diagram_to_paint_structural, DiagramToPaintOptions};
    use layout_ir::font_spec;
    use mermaid_parser::parse_class_diagram;
    use paint_codec_png::write_png;
    use paint_metal::render;
    use text_native_coretext::{CoreTextMetrics, CoreTextResolver, CoreTextShaper};

    let src = "classDiagram
  class Shape { +color: String; +area() f64 }
  class Circle { +radius: f64; +area() f64 }
  class Rectangle { +width: f64; +height: f64; +area() f64 }
  Shape <|-- Circle
  Shape <|-- Rectangle";

    let diagram  = parse_class_diagram(src).expect("class parse failed");
    let layout   = layout_structural_diagram(&diagram);
    let shaper   = CoreTextShaper;
    let metrics  = CoreTextMetrics;
    let resolver = CoreTextResolver::new();

    let scene = diagram_to_paint_structural(
        &layout,
        &DiagramToPaintOptions {
            background: layout_ir::Color { r: 255, g: 255, b: 255, a: 255 },
            device_pixel_ratio: 2.0,
            label_font: font_spec("Helvetica", 12.0),
            title_font: { let mut f = font_spec("Helvetica", 14.0); f.weight = 700; f },
            shaper: &shaper,
            metrics: &metrics,
            resolver: &resolver,
        },
    );

    let path = "/tmp/class_diagram.png";
    write_png(&render(&scene), path).expect("PNG write failed");
    println!("Rendered class diagram to {path}");
    println!("Scene: {}×{} px, {} nodes, {} relationships",
        scene.width, scene.height, layout.nodes.len(), layout.relationships.len());
}

#[cfg(not(target_vendor = "apple"))]
fn main() { panic!("class_diagram example requires an Apple target (paint-metal)"); }
