//! End-to-end example: geometric diagram → PaintScene → Metal → PNG
//!
//! Uses a hard-coded GeometricDiagram (no parser for this family).
//!
//! Run with:
//!   cargo run --example geometric -p diagram-to-paint
//!
//! Output: /tmp/geometric.png

#[cfg(target_vendor = "apple")]
fn main() {
    use diagram_ir::{GeoElement, GeometricDiagram, TextAlign};
    use diagram_layout_geometric::layout_geometric_diagram;
    use diagram_to_paint::{diagram_to_paint_geometric, DiagramToPaintOptions};
    use layout_ir::font_spec;
    use paint_codec_png::write_png;
    use paint_metal::render;
    use text_native_coretext::{CoreTextMetrics, CoreTextResolver, CoreTextShaper};

    let diagram = GeometricDiagram {
        title: Some("Geometric Demo".into()),
        width: None,
        height: None,
        elements: vec![
            GeoElement::Box {
                id: "input".into(), x: 50.0, y: 100.0, w: 120.0, h: 60.0,
                corner_radius: 6.0, label: Some("Input".into()),
                fill: Some("#dbeafe".into()), stroke: Some("#2563eb".into()),
            },
            GeoElement::Line {
                id: "l1".into(), x1: 170.0, y1: 130.0, x2: 260.0, y2: 130.0,
                arrow_end: true, arrow_start: false, stroke: None,
            },
            GeoElement::Circle {
                id: "proc".into(), cx: 300.0, cy: 130.0, r: 40.0,
                label: Some("Process".into()),
                fill: Some("#dcfce7".into()), stroke: Some("#16a34a".into()),
            },
            GeoElement::Line {
                id: "l2".into(), x1: 340.0, y1: 130.0, x2: 420.0, y2: 130.0,
                arrow_end: true, arrow_start: false, stroke: None,
            },
            GeoElement::Box {
                id: "output".into(), x: 420.0, y: 100.0, w: 120.0, h: 60.0,
                corner_radius: 6.0, label: Some("Output".into()),
                fill: Some("#fef9c3".into()), stroke: Some("#ca8a04".into()),
            },
            GeoElement::Text {
                id: "t1".into(), x: 270.0, y: 60.0,
                text: "Data Flow Pipeline".into(),
                align: TextAlign::Center,
            },
        ],
    };

    let layout   = layout_geometric_diagram(&diagram);
    let shaper   = CoreTextShaper;
    let metrics  = CoreTextMetrics;
    let resolver = CoreTextResolver::new();

    let scene = diagram_to_paint_geometric(
        &layout,
        &DiagramToPaintOptions {
            background: layout_ir::Color { r: 255, g: 255, b: 255, a: 255 },
            device_pixel_ratio: 2.0,
            label_font: font_spec("Helvetica", 13.0),
            title_font: { let mut f = font_spec("Helvetica", 16.0); f.weight = 700; f },
            shaper: &shaper,
            metrics: &metrics,
            resolver: &resolver,
        },
    );

    let path = "/tmp/geometric.png";
    write_png(&render(&scene), path).expect("PNG write failed");
    println!("Rendered geometric diagram to {path}");
    println!("Scene: {}×{} px, {} elements", scene.width, scene.height, layout.elements.len());
}

#[cfg(not(target_vendor = "apple"))]
fn main() { panic!("geometric example requires an Apple target (paint-metal)"); }
