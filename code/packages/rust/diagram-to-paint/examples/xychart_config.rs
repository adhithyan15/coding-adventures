//! End-to-end Mermaid XY init configuration to Metal-rendered PNG validation.

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
        r##"%%{init: {"xyChart": {"width": 720, "height": 440, "chartOrientation": "horizontal", "plotReservedSpacePercent": 55, "titleFontSize": 24, "titlePadding": 14, "showLegend": true, "legendFontSize": 21, "legendPadding": 16, "showDataLabel": true, "showDataLabelOutsideBar": true, "xAxis": {"labelFontSize": 13, "labelPadding": 7, "titleFontSize": 17, "titlePadding": 8, "showTick": false, "axisLineWidth": 4}, "yAxis": {"labelFontSize": 15, "labelPadding": 8, "titleFontSize": 18, "titlePadding": 9, "tickLength": 12, "tickWidth": 4, "axisLineWidth": 5}}, "themeVariables": {"xyChart": {"dataLabelColor": "#0b5d4b"}}}}%%
xychart
title "Quarterly Throughput"
x-axis "Quarter" [Q1, Q2, Q3, Q4]
y-axis "Requests" 0 --> 100
bar "Observed" [28, 46, 71, 88]
line "Target" [35, 50, 68, 82]"##,
    )
    .expect("configured XY chart parse failed");
    let layout = layout_chart_diagram(&diagram, 600.0, 400.0);
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
    let path = "/tmp/xychart-config.png";
    write_png(&render(&scene), path).expect("PNG write failed");
    println!("Rendered configured XY chart to {path}");
}

#[cfg(not(target_vendor = "apple"))]
fn main() {
    panic!("configured XY-chart example requires an Apple target (paint-metal)");
}
