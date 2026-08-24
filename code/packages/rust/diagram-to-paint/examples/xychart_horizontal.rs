//! End-to-end Mermaid horizontal XY chart to Metal-rendered PNG validation.

#[cfg(target_vendor = "apple")]
fn main() {
    use diagram_layout_chart::layout_chart_diagram;
    use diagram_to_paint::{diagram_to_paint_chart, DiagramToPaintOptions};
    use layout_ir::font_spec;
    use mermaid_parser::parse_xychart;
    use paint_codec_png::write_png;
    use paint_metal::render;
    use text_native_coretext::{CoreTextMetrics, CoreTextResolver, CoreTextShaper};

    let diagram = parse_xychart(
        r#"xychart horizontal
title "Regional Revenue"
x-axis "Region" [North, South, East, West]
y-axis "USD (k)" 0 --> 120
bar "Actual" [88, 64, 102, 76]
line "Forecast" [94, 72, 110, 82]"#,
    )
    .expect("horizontal XY chart parse failed");
    let layout = layout_chart_diagram(&diagram, 720.0, 440.0);
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
            label_font: font_spec("Helvetica", 11.0),
            title_font: font_spec("Helvetica", 16.0),
            shaper: &shaper,
            metrics: &metrics,
            resolver: &resolver,
        },
    );
    let path = "/tmp/xychart-horizontal.png";
    write_png(&render(&scene), path).expect("PNG write failed");
    println!("Rendered horizontal XY chart to {path}");
}

#[cfg(not(target_vendor = "apple"))]
fn main() {
    panic!("horizontal XY-chart example requires an Apple target (paint-metal)");
}
