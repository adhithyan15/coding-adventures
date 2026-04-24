// End-to-end rendering test: DOT → diagram-ir → layout → paint → Metal → PNG
//
// Run with:
//   cargo test -p diagram-to-paint --test e2e_render -- --nocapture
//
// Output: /tmp/diagram_e2e.png
//
// This test only compiles on Apple platforms (CoreText + Metal).

#[cfg(target_vendor = "apple")]
mod apple {
    use diagram_layout_graph::layout_graph_diagram;
    use diagram_to_paint::{diagram_to_paint, DiagramToPaintOptions};
    use dot_parser::parse_to_diagram;
    use layout_ir::font_spec;
    use mermaid_parser::parse_to_diagram as parse_mermaid_to_diagram;
    use paint_codec_png::write_png;
    use paint_metal::render;
    use text_native_coretext::{CoreTextMetrics, CoreTextResolver, CoreTextShaper};

    #[test]
    fn render_dot_diagram_to_png() {
        let dot = r#"
            digraph Pipeline {
                rankdir=LR;
                DOT -> Parser -> Layout -> Paint -> Metal;
                Metal -> PNG;
            }
        "#;

        let graph = parse_to_diagram(dot).expect("DOT parse failed");
        let layout = layout_graph_diagram(&graph, None, None);

        let shaper = CoreTextShaper;
        let metrics = CoreTextMetrics;
        let resolver = CoreTextResolver::new();

        let opts = DiagramToPaintOptions {
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
        };

        let scene = diagram_to_paint(&layout, &opts);
        let pixels = render(&scene);
        let path = "/tmp/diagram_e2e.png";
        write_png(&pixels, path).expect("PNG write failed");

        println!("Rendered {}×{} scene → {}", scene.width, scene.height, path);
        println!("  {} paint instructions", scene.instructions.len());
        let glyph_runs = scene
            .instructions
            .iter()
            .filter(|i| matches!(i, paint_instructions::PaintInstruction::GlyphRun(_)))
            .count();
        println!(
            "  {} PaintGlyphRun instructions (real glyph IDs)",
            glyph_runs
        );

        assert!(pixels.width > 0);
        assert!(pixels.height > 0);
        assert!(
            glyph_runs > 0,
            "expected at least one PaintGlyphRun from shaping pipeline"
        );
    }

    #[test]
    fn render_mermaid_diagram_to_png() {
        let mermaid = r#"
            flowchart LR
            A[Mermaid] --> B{Layout}
            B -->|paint| C((Metal))
            C --> D[PNG]
        "#;

        let graph = parse_mermaid_to_diagram(mermaid).expect("Mermaid parse failed");
        let layout = layout_graph_diagram(&graph, None, None);

        let shaper = CoreTextShaper;
        let metrics = CoreTextMetrics;
        let resolver = CoreTextResolver::new();

        let opts = DiagramToPaintOptions {
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
        };

        let scene = diagram_to_paint(&layout, &opts);
        let pixels = render(&scene);
        let path = "/tmp/mermaid_e2e.png";
        write_png(&pixels, path).expect("PNG write failed");

        println!(
            "Rendered Mermaid {}×{} scene → {}",
            scene.width, scene.height, path
        );
        println!("  {} paint instructions", scene.instructions.len());
        let glyph_runs = scene
            .instructions
            .iter()
            .filter(|i| matches!(i, paint_instructions::PaintInstruction::GlyphRun(_)))
            .count();
        println!(
            "  {} PaintGlyphRun instructions (real glyph IDs)",
            glyph_runs
        );

        assert!(pixels.width > 0);
        assert!(pixels.height > 0);
        assert!(
            glyph_runs > 0,
            "expected at least one PaintGlyphRun from shaping pipeline"
        );
    }
}
