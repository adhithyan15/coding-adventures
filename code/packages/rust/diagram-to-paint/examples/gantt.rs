//! End-to-end example: Mermaid gantt → PaintScene → Metal → PNG
//!
//! Run with:
//!   cargo run --example gantt -p diagram-to-paint
//!
//! Output: /tmp/gantt.png

#[cfg(target_vendor = "apple")]
fn main() {
    use diagram_ir::{TemporalBody, TemporalDiagram, TemporalKind};
    use diagram_layout_temporal::layout_temporal_diagram;
    use diagram_to_paint::{diagram_to_paint_temporal, DiagramToPaintOptions};
    use layout_ir::font_spec;
    use mermaid_parser::parse_gantt;
    use paint_codec_png::write_png;
    use paint_metal::render;
    use text_native_coretext::{CoreTextMetrics, CoreTextResolver, CoreTextShaper};

    let src = "gantt
  title Diagram Pipeline
  dateFormat YYYY-MM-DD
  section Spec
    DG00 spec     :done, s1, 2026-01-01, 3d
    DG04 spec     :done, s2, after s1, 2d
  section Implementation
    diagram-ir    :active, i1, after s2, 4d
    layout-chart  :i2, after i1, 5d
    diagram-paint :i3, after i2, 3d";

    let gantt    = parse_gantt(src).expect("gantt parse failed");
    let temporal = TemporalDiagram {
        kind: TemporalKind::Gantt,
        title: Some("Diagram Pipeline".into()),
        body: TemporalBody::Gantt(gantt),
    };
    let layout   = layout_temporal_diagram(&temporal, 900.0);
    let shaper   = CoreTextShaper;
    let metrics  = CoreTextMetrics;
    let resolver = CoreTextResolver::new();

    let scene = diagram_to_paint_temporal(
        &layout,
        &DiagramToPaintOptions {
            background: layout_ir::Color { r: 255, g: 255, b: 255, a: 255 },
            device_pixel_ratio: 2.0,
            label_font: font_spec("Helvetica", 11.0),
            title_font: { let mut f = font_spec("Helvetica", 13.0); f.weight = 700; f },
            shaper: &shaper,
            metrics: &metrics,
            resolver: &resolver,
        },
    );

    let path = "/tmp/gantt.png";
    write_png(&render(&scene), path).expect("PNG write failed");
    println!("Rendered Gantt chart to {path}");
    println!("Scene: {}×{} px, {} items", scene.width, scene.height, layout.items.len());
}

#[cfg(not(target_vendor = "apple"))]
fn main() { panic!("gantt example requires an Apple target (paint-metal)"); }
