//! End-to-end example: Mermaid xychart-beta → PaintScene → Metal → PNG
//!
//! Run with:
//!   cargo run --example xychart -p diagram-to-paint
//!
//! Output: /tmp/xychart.png

#[cfg(target_vendor = "apple")]
fn main() {
    use diagram_layout_chart::layout_chart_diagram;
    use diagram_to_paint::{diagram_to_paint_chart, DiagramToPaintOptions};
    use layout_ir::font_spec;
    use mermaid_parser::parse_xychart;
    use paint_codec_png::write_png;
    use paint_metal::render;
    use text_native_coretext::{CoreTextMetrics, CoreTextResolver, CoreTextShaper};

    let src = r#"xychart-beta
  title "Monthly Revenue"
  x-axis [Jan, Feb, Mar, Apr, May, Jun]
  y-axis "USD (k)" 0 --> 120
  bar [45, 62, 38, 71, 85, 94]
  line [40, 58, 42, 68, 79, 90]"#;

    let diagram  = parse_xychart(src).expect("xychart parse failed");
    let layout   = layout_chart_diagram(&diagram, 700.0, 420.0);
    let shaper   = CoreTextShaper;
    let metrics  = CoreTextMetrics;
    let resolver = CoreTextResolver::new();

    let scene = diagram_to_paint_chart(
        &layout,
        &DiagramToPaintOptions {
            background: layout_ir::Color { r: 255, g: 255, b: 255, a: 255 },
            device_pixel_ratio: 2.0,
            label_font: font_spec("Helvetica", 11.0),
            title_font: { let mut f = font_spec("Helvetica", 16.0); f.weight = 700; f },
            shaper: &shaper,
            metrics: &metrics,
            resolver: &resolver,
        },
    );

    let path = "/tmp/xychart.png";
    write_png(&render(&scene), path).expect("PNG write failed");
    println!("Rendered XY chart to {path}");
    println!("Scene: {}×{} px, {} chart items",
        scene.width, scene.height, layout.items.len());
}

#[cfg(not(target_vendor = "apple"))]
fn main() { panic!("xychart example requires an Apple target (paint-metal)"); }
