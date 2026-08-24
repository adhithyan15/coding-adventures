//! End-to-end Mermaid numeric-x XY chart to Metal-rendered PNG validation.

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
        r#"xychart
title "Calibration Curve"
x-axis "Concentration" 0 --> 100
y-axis "Response" 0 --> 80
bar "Observed" [8, 24, 38, 57, 72]
line "Fit" [10 "Low", 22, 40 "Mid", 58, 74 "High"]"#,
    )
    .expect("numeric-x XY chart parse failed");
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
    let path = "/tmp/xychart-numeric.png";
    write_png(&render(&scene), path).expect("PNG write failed");
    println!("Rendered numeric-x XY chart to {path}");
}

#[cfg(not(target_vendor = "apple"))]
fn main() {
    panic!("numeric-x XY-chart example requires an Apple target (paint-metal)");
}
