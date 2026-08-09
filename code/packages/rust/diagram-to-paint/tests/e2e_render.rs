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
    use diagram_layout_chart::layout_chart_diagram;
    use diagram_layout_graph::layout_graph_diagram;
    use diagram_layout_sequence::layout_sequence_diagram;
    use diagram_layout_structural::layout_structural_diagram;
    use diagram_layout_temporal::layout_temporal_diagram;
    use diagram_to_paint::{
        diagram_to_paint, diagram_to_paint_chart, diagram_to_paint_sequence,
        diagram_to_paint_structural, diagram_to_paint_temporal, DiagramToPaintOptions,
    };
    use dot_parser::parse_to_diagram;
    use layout_ir::font_spec;
    use mermaid_parser::{
        parse_c4_diagram, parse_er_diagram, parse_gitgraph, parse_pie, parse_sankey,
        parse_sequence_diagram, parse_to_diagram as parse_mermaid_to_diagram,
    };
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

    #[test]
    fn render_mermaid_pie_to_png() {
        let diagram = parse_pie(
            r#"pie showData
                "Graph" : 50
                "Chart" : 30
                "Temporal" : 20"#,
        )
        .expect("Mermaid pie parse failed");
        let layout = layout_chart_diagram(&diagram, 640.0, 480.0);

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
            label_font: font_spec("Helvetica", 12.0),
            title_font: font_spec("Helvetica", 16.0),
            shaper: &shaper,
            metrics: &metrics,
            resolver: &resolver,
        };

        let scene = diagram_to_paint_chart(&layout, &opts);
        let pixels = render(&scene);
        let path = "/tmp/mermaid_pie_e2e.png";
        write_png(&pixels, path).expect("PNG write failed");

        assert!(pixels.width > 0);
        assert!(pixels.height > 0);
        assert!(!scene.instructions.is_empty());
    }

    #[test]
    fn render_mermaid_sankey_to_png() {
        let diagram = parse_sankey(
            "sankey-beta\nElectricity,Heating,45\nElectricity,Lighting,30\nHeating,Losses,8",
        )
        .expect("Mermaid Sankey parse failed");
        let layout = layout_chart_diagram(&diagram, 700.0, 460.0);

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
                title_font: font_spec("Helvetica", 16.0),
                shaper: &shaper,
                metrics: &metrics,
                resolver: &resolver,
            },
        );

        let pixels = render(&scene);
        write_png(&pixels, "/tmp/mermaid_sankey_e2e.png").expect("PNG write failed");
        assert!(pixels.width > 0);
        assert!(pixels.height > 0);
        assert!(!scene.instructions.is_empty());
    }

    #[test]
    fn render_mermaid_gitgraph_to_png() {
        let git = parse_gitgraph(
            "gitGraph LR:\ncommit id: \"root\"\nbranch feature\ncheckout feature\ncommit id: \"work\" msg: \"Build parser\"\ncherry-pick id: \"root\" parent: \"work\"\ncheckout main\nmerge feature tag: \"v1\"",
        )
        .expect("Mermaid GitGraph parse failed");
        let temporal = diagram_ir::TemporalDiagram {
            kind: diagram_ir::TemporalKind::Git,
            title: Some("GitGraph pipeline".to_string()),
            body: diagram_ir::TemporalBody::Git(git),
        };
        let layout = layout_temporal_diagram(&temporal, 800.0);

        let shaper = CoreTextShaper;
        let metrics = CoreTextMetrics;
        let resolver = CoreTextResolver::new();
        let scene = diagram_to_paint_temporal(
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
                title_font: font_spec("Helvetica", 16.0),
                shaper: &shaper,
                metrics: &metrics,
                resolver: &resolver,
            },
        );

        let pixels = render(&scene);
        write_png(&pixels, "/tmp/mermaid_gitgraph_e2e.png").expect("PNG write failed");
        assert!(pixels.width > 0);
        assert!(pixels.height > 0);
        assert!(!scene.instructions.is_empty());
    }

    #[test]
    fn render_mermaid_er_to_png() {
        let diagram = parse_er_diagram(
            "erDiagram\nCUSTOMER ||--o{ ORDER : places\nCUSTOMER {\nstring name PK\nstring email UK\n}\nORDER {\nint id PK\n}",
        )
        .expect("Mermaid ER parse failed");
        let layout = layout_structural_diagram(&diagram);

        let shaper = CoreTextShaper;
        let metrics = CoreTextMetrics;
        let resolver = CoreTextResolver::new();
        let scene = diagram_to_paint_structural(
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
                title_font: font_spec("Helvetica", 16.0),
                shaper: &shaper,
                metrics: &metrics,
                resolver: &resolver,
            },
        );

        let pixels = render(&scene);
        write_png(&pixels, "/tmp/mermaid_er_e2e.png").expect("PNG write failed");
        assert!(pixels.width > 0);
        assert!(pixels.height > 0);
        assert!(!scene.instructions.is_empty());
    }

    #[test]
    fn render_mermaid_c4_to_png() {
        let diagram = parse_c4_diagram(
            "C4Context\nPerson(customer, \"Customer\", \"Uses online banking\")\nSystem_Boundary(bank, \"Bank\") {\nSystem(web, \"Internet Banking\", \"Handles accounts\")\n}\nRel(customer, web, \"Uses\", \"HTTPS\")",
        )
        .expect("Mermaid C4 parse failed");
        let layout = layout_structural_diagram(&diagram);
        assert_eq!(layout.groups.len(), 1);

        let shaper = CoreTextShaper;
        let metrics = CoreTextMetrics;
        let resolver = CoreTextResolver::new();
        let scene = diagram_to_paint_structural(
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
                title_font: font_spec("Helvetica", 16.0),
                shaper: &shaper,
                metrics: &metrics,
                resolver: &resolver,
            },
        );

        let pixels = render(&scene);
        write_png(&pixels, "/tmp/mermaid_c4_e2e.png").expect("PNG write failed");
        assert!(pixels.width > 0);
        assert!(pixels.height > 0);
        assert!(!scene.instructions.is_empty());
        assert!(scene.instructions.iter().any(|instruction| matches!(
            instruction,
            paint_instructions::PaintInstruction::Rect(rect) if rect.stroke_dash.is_some()
        )));
    }

    #[test]
    fn render_mermaid_sequence_to_png() {
        let diagram = parse_sequence_diagram(
            "sequenceDiagram\ntitle Native Mermaid sequence\nautonumber\nactor User\nparticipant API as Banking API\nparticipant DB as Ledger\nalt Transfer accepted\nUser->>+API: Submit transfer\ncreate participant Worker as Audit Worker\nAPI->>Worker: Start audit\nWorker-->>API: Audit complete\ndestroy Worker\nloop Persist until committed\nAPI->>DB: Record transaction\nDB-->>API: Committed\nend\nnote right of API: Metal paints this scene\nAPI-->>-User: Transfer complete\nelse Transfer rejected\nAPI-->>User: Validation failed\nend",
        )
        .expect("Mermaid sequence parse failed");
        let layout = layout_sequence_diagram(&diagram);

        let shaper = CoreTextShaper;
        let metrics = CoreTextMetrics;
        let resolver = CoreTextResolver::new();
        let scene = diagram_to_paint_sequence(
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
                title_font: font_spec("Helvetica", 17.0),
                shaper: &shaper,
                metrics: &metrics,
                resolver: &resolver,
            },
        );

        let pixels = render(&scene);
        write_png(&pixels, "/tmp/mermaid_sequence_e2e.png").expect("PNG write failed");
        assert!(pixels.width > 0);
        assert!(pixels.height > 0);
        assert!(scene.instructions.iter().any(|instruction| matches!(
            instruction,
            paint_instructions::PaintInstruction::Path(path) if path.stroke_dash.is_some()
        )));
        assert!(scene.instructions.iter().any(|instruction| matches!(
            instruction,
            paint_instructions::PaintInstruction::Path(path)
                if path.stroke.as_deref() == Some("#dc2626")
        )));
        assert!(scene.instructions.iter().any(|instruction| matches!(
            instruction,
            paint_instructions::PaintInstruction::Rect(rect)
                if rect.stroke.as_deref() == Some("#64748b")
        )));
    }
}
