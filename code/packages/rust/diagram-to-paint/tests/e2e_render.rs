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
    use diagram_ir::{
        EdgeKind, SequenceBlockKind, SequenceEvent, SequenceLink, SequenceProperty, TemporalBody,
        TemporalDiagram, TemporalKind,
    };
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
        parse_c4_diagram, parse_er_diagram, parse_gantt, parse_gitgraph, parse_journey, parse_pie,
        parse_quadrant_chart, parse_requirement_diagram, parse_sankey, parse_sequence_diagram,
        parse_state_diagram, parse_to_diagram as parse_mermaid_to_diagram, parse_xychart,
    };
    use paint_codec_png::write_png;
    use paint_instructions::PaintInstruction;
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
    fn render_mermaid_state_to_png() {
        let graph = parse_state_diagram(
            "stateDiagram-v2\n# Native state fixture comment\naccTitle: Native state lifecycle\naccDescr {\nState flow rendered through Metal\nwith native accessibility metadata\n}\ndirection LR\nReady: Status: awaiting work\nstate Decision <<choice>>\nstate WorkFork <<fork>>\nstate WorkJoin [[join]]\nstyle Ready background:#dbeafe,border:3px solid red,color:#1e3a8a,font-size:22px,font-weight:bold,font-style:italic,font-family:\"Avenir Next\"\nclassDef active fill:#dcfce7,stroke:#166534,color:#14532d\nclassDef phase fill:#fef3c7,stroke:#b45309,color:#78350f\nclassDef emphasis stroke-width:4px\nclass Auditing active emphasis\nclick Ready \"https://example.com/ready\" \"Open ready state\"\nstate \"Processing Queue\" as Processing {\nQueued --> Running\n--\nAuditing --> Reviewing\nstate \"Review queue\" as Reviewing: Audit detail\n}\nclass Processing phase\n[*] --> Ready\nReady --> Processing: Trigger: enter # inline transition comment\nProcessing --> Decision: inspect\nDecision --> WorkFork: start\nWorkFork --> Running:::active\nWorkFork --> Auditing\nnote right of Running\nNative Metal note\nSecond shaped line\nend note\nnote \"Detached reminder\" as Reminder\nRunning --> WorkJoin\nAuditing --> WorkJoin\nWorkJoin --> [*]: stop\nDecision --> Ready: wait\n",
        )
        .expect("Mermaid state parse failed");
        let layout = layout_graph_diagram(&graph, None, None);
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
                title_font: font_spec("Helvetica", 18.0),
                shaper: &shaper,
                metrics: &metrics,
                resolver: &resolver,
            },
        );
        let pixels = render(&scene);
        write_png(&pixels, "/tmp/mermaid_state_e2e.png").expect("PNG write failed");

        assert!(pixels.width > 0);
        assert!(pixels.height > 0);
        assert!(!scene.instructions.is_empty());
        let metadata = scene.metadata.as_ref().expect("accessibility metadata");
        assert_eq!(metadata["accessibility.title"], "Native state lifecycle");
        assert!(metadata["accessibility.description"].contains("rendered through Metal"));
        assert_eq!(
            metadata["graph.node.Ready.link.url"],
            "https://example.com/ready"
        );
        assert!(metadata.contains_key("graph.node.Ready.link.bounds"));
        let ready = layout
            .nodes
            .iter()
            .find(|node| node.id == "Ready")
            .expect("styled Ready node");
        assert_eq!(ready.style.fill, "#dbeafe");
        assert_eq!(ready.style.stroke, "red");
        assert_eq!(ready.style.stroke_width, 3.0);
        let group = layout
            .groups
            .iter()
            .find(|group| group.id == "Processing")
            .expect("aliased composite group");
        assert_eq!(group.label.text, "Processing Queue");
        assert_eq!(group.style.fill, "#fef3c7");
        assert_eq!(group.divider_y.len(), 1);
        assert!(layout
            .edges
            .iter()
            .any(|edge| edge.to_node_id == "Processing"));
        assert!(layout
            .edges
            .iter()
            .any(|edge| edge.from_node_id == "Processing"));
    }

    #[test]
    fn render_mermaid_composite_state_note_to_png() {
        let graph = parse_state_diagram(
            "stateDiagram-v2\nstate \"Not Shooting State\" as NotShooting {\nIdle --> Configuring\n}\nnote right of NotShooting: Composite state note\n",
        )
        .expect("Mermaid composite state note parse failed");
        assert!(!graph.nodes.iter().any(|node| node.id == "NotShooting"));

        let layout = layout_graph_diagram(&graph, None, None);
        assert!(layout.edges.iter().any(|edge| {
            edge.kind == EdgeKind::NoteAssociation && edge.from_node_id == "NotShooting"
        }));

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
                title_font: font_spec("Helvetica", 18.0),
                shaper: &shaper,
                metrics: &metrics,
                resolver: &resolver,
            },
        );
        assert!(scene.instructions.iter().any(|instruction| {
            matches!(instruction, paint_instructions::PaintInstruction::Path(path) if path.stroke_dash.is_some())
        }));

        let pixels = render(&scene);
        write_png(&pixels, "/tmp/mermaid_composite_state_note_e2e.png").expect("PNG write failed");
        assert!(pixels.width > 0);
        assert!(pixels.height > 0);
    }

    #[test]
    fn render_mermaid_state_line_break_variants_to_png() {
        let graph = parse_state_diagram(
            "stateDiagram-v2\nReady: First<br>Second<br/>Third<br />Fourth<br\t/>Fifth\nReady --> Done\n",
        )
        .expect("Mermaid state line-break variants should parse");
        assert_eq!(
            graph.nodes[0].label.text,
            "First\nSecond\nThird\nFourth\nFifth"
        );

        let layout = layout_graph_diagram(&graph, None, None);
        let ready = layout
            .nodes
            .iter()
            .find(|node| node.id == "Ready")
            .expect("multiline Ready node");
        assert!(ready.height > 100.0);

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
                title_font: font_spec("Helvetica", 18.0),
                shaper: &shaper,
                metrics: &metrics,
                resolver: &resolver,
            },
        );
        let glyph_runs = scene
            .instructions
            .iter()
            .filter(|instruction| {
                matches!(
                    instruction,
                    paint_instructions::PaintInstruction::GlyphRun(_)
                )
            })
            .count();
        assert!(glyph_runs >= 6, "five label lines plus the Done label");

        let pixels = render(&scene);
        write_png(&pixels, "/tmp/mermaid_state_line_breaks_e2e.png").expect("PNG write failed");
        assert!(pixels.width > 0);
        assert!(pixels.height > 0);
    }

    #[test]
    fn render_mermaid_single_percent_states_to_png() {
        let graph = parse_state_diagram(
            "stateDiagram-v2\n% not a comment\nMoving --> Still %inline\nStill%Active\n",
        )
        .expect("single-percent state syntax should parse");
        assert_eq!(graph.nodes.len(), 8);

        let layout = layout_graph_diagram(&graph, None, None);
        for (index, node) in layout.nodes.iter().enumerate() {
            for other in layout.nodes.iter().skip(index + 1) {
                let separated = node.x + node.width <= other.x
                    || other.x + other.width <= node.x
                    || node.y + node.height <= other.y
                    || other.y + other.height <= node.y;
                assert!(
                    separated,
                    "layout nodes {} and {} must not overlap",
                    node.id, other.id
                );
            }
        }
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
                title_font: font_spec("Helvetica", 18.0),
                shaper: &shaper,
                metrics: &metrics,
                resolver: &resolver,
            },
        );
        assert!(!scene.instructions.is_empty());

        let pixels = render(&scene);
        write_png(&pixels, "/tmp/mermaid_single_percent_states_e2e.png").expect("PNG write failed");
        assert!(pixels.width > 0);
        assert!(pixels.height > 0);
    }

    #[test]
    fn render_mermaid_pie_to_png() {
        let diagram = parse_pie(
            r#"pie showData title Native chart pipeline
                accTitle: Native pie chart
                accDescr: Pie rendered through Metal
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
        let metadata = scene.metadata.as_ref().expect("accessibility metadata");
        assert_eq!(metadata["accessibility.title"], "Native pie chart");
        assert_eq!(
            metadata["accessibility.description"],
            "Pie rendered through Metal"
        );
    }

    #[test]
    fn render_configured_mermaid_xy_to_png() {
        let diagram = parse_xychart(
        r##"%%{init: {"xyChart": {"width": 720, "height": 440, "chartOrientation": "horizontal", "plotReservedSpacePercent": 55, "titleFontSize": 24, "titlePadding": 14, "showLegend": true, "legendFontSize": 21, "legendPadding": 16, "showDataLabel": true, "showDataLabelOutsideBar": true, "xAxis": {"labelFontSize": 13, "labelPadding": 7, "titleFontSize": 17, "titlePadding": 8, "showTick": false, "axisLineWidth": 4}, "yAxis": {"labelFontSize": 15, "labelPadding": 8, "titleFontSize": 18, "titlePadding": 9, "tickLength": 12, "tickWidth": 4, "axisLineWidth": 5}}, "themeVariables": {"xyChart": {"backgroundColor": "#fff8e7", "titleColor": "#264653", "plotColorPalette": "#2a9d8f, #e76f51", "dataLabelColor": "#0b5d4b", "xAxisLabelColor": "#005f73", "xAxisTitleColor": "#0a9396", "xAxisTickColor": "#94d2bd", "xAxisLineColor": "#001219", "yAxisLabelColor": "#9b2226", "yAxisTitleColor": "#ae2012", "yAxisTickColor": "#ee9b00", "yAxisLineColor": "#ca6702"}}}}%%
xychart
title "Quarterly Throughput"
x-axis "Quarter" [Q1, Q2, Q3, Q4]
y-axis "Requests" 0 --> 100
bar "Observed" [28, 46, 71, 88]
line "Target" [35, 50, 68, 82]"##,
        )
        .expect("configured XY chart should parse");
        let layout = layout_chart_diagram(&diagram, 600.0, 400.0);
        assert_eq!((layout.width, layout.height), (720.0, 440.0));
        assert_eq!(layout.background_color.as_deref(), Some("#fff8e7"));
        assert_eq!(diagram.orientation, diagram_ir::ChartOrientation::Horizontal);
        assert!(layout.items.iter().any(|item| matches!(
            item,
            diagram_ir::LayoutedChartItem::AxisSpine { y1, y2, .. }
                if (*y2 - *y1) >= 242.0
        )));
        assert_eq!(
            layout
                .items
                .iter()
                .filter(|item| matches!(item, diagram_ir::LayoutedChartItem::BarLabel { .. }))
                .count(),
            4
        );
        assert!(layout.items.iter().any(|item| matches!(
            item,
            diagram_ir::LayoutedChartItem::AxisSpine { stroke_width, color, .. }
                if *stroke_width == 5.0 && color == "#ca6702"
        )));
        assert!(layout.items.iter().any(|item| matches!(
            item,
            diagram_ir::LayoutedChartItem::AxisTick { font_size, color, .. }
                if *font_size == 13.0 && color == "#005f73"
        )));
        assert_eq!(
            layout
                .items
                .iter()
                .filter(|item| matches!(item, diagram_ir::LayoutedChartItem::AxisTickMark { .. }))
                .count(),
            6
        );
        assert!(layout.items.iter().any(|item| matches!(
            item,
            diagram_ir::LayoutedChartItem::AxisTickMark { y1, y2, stroke_width, .. }
                if (*y2 - *y1 - 12.0).abs() < f64::EPSILON && *stroke_width == 4.0
        )));
        assert!(layout.items.iter().any(|item| matches!(
            item,
            diagram_ir::LayoutedChartItem::Bar { width, height, .. } if width > height
        )));
        assert!(layout.items.iter().any(|item| matches!(
            item,
            diagram_ir::LayoutedChartItem::Bar { color, .. } if color == "#2a9d8f"
        )));
        assert!(layout.items.iter().any(|item| matches!(
            item,
            diagram_ir::LayoutedChartItem::LinePath { color, .. } if color == "#e76f51"
        )));
        assert!(layout.items.iter().any(|item| matches!(
            item,
            diagram_ir::LayoutedChartItem::DataLabel { text, color, .. }
                if text == "Quarterly Throughput" && color.as_deref() == Some("#264653")
        )));
        assert!(layout.items.iter().any(|item| matches!(
            item,
            diagram_ir::LayoutedChartItem::Legend { font_size, entries, .. }
                if *font_size == Some(21.0) && entries.len() == 2
        )));

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
        write_png(&pixels, "/tmp/mermaid_xy_config_e2e.png").expect("PNG write failed");

        assert_eq!((pixels.width, pixels.height), (720, 440));
        assert_eq!(scene.background, "#fff8e7");
        assert!(!scene.instructions.is_empty());
        assert!(scene.instructions.iter().any(|instruction| matches!(
            instruction,
            PaintInstruction::Path(path)
                if path.stroke.as_deref() == Some("#ca6702")
                    && path.stroke_width == Some(5.0)
        )));
        assert!(scene.instructions.iter().any(|instruction| matches!(
            instruction,
            PaintInstruction::GlyphRun(run) if run.font_size == 21.0
        )));
    }

    #[test]
    fn render_rotated_mermaid_xy_labels_to_png() {
        let diagram = parse_xychart(
            "%%{init: {\"xyChart\": {\"xAxis\": {\"labelRotation\": -45}}}}%%\n\
             xychart vertical\n\
             x-axis [January forecast, February forecast, March forecast]\n\
             y-axis 0 --> 100\n\
             bar [35, 62, 88]\n",
        )
        .expect("rotated XY chart should parse");
        let layout = layout_chart_diagram(&diagram, 700.0, 500.0);
        assert!(layout.items.iter().any(|item| matches!(
            item,
            diagram_ir::LayoutedChartItem::AxisTick {
                rotation_degrees,
                ..
            } if *rotation_degrees == -45.0
        )));

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
        assert!(scene.instructions.iter().any(|instruction| matches!(
            instruction,
            PaintInstruction::Group(group) if group.transform.is_some()
        )));

        let pixels = render(&scene);
        write_png(&pixels, "/tmp/mermaid_xy_rotation_e2e.png").expect("PNG write failed");
        assert_eq!((pixels.width, pixels.height), (700, 500));
    }

    #[test]
    fn render_mermaid_quadrant_to_png() {
        let diagram = parse_quadrant_chart(
            "%%{init: {\"quadrantChart\": {\"chartWidth\": 680, \"chartHeight\": 560, \"xAxisPosition\": \"top\", \"yAxisPosition\": \"right\", \"pointRadius\": 7, \"quadrantPadding\": 18, \"quadrantInternalBorderStrokeWidth\": 3, \"quadrantExternalBorderStrokeWidth\": 5, \"titleFontSize\": 22, \"titlePadding\": 12, \"xAxisLabelFontSize\": 15, \"xAxisLabelPadding\": 21, \"yAxisLabelFontSize\": 16, \"yAxisLabelPadding\": 23, \"quadrantLabelFontSize\": 17, \"quadrantTextTopPadding\": 19, \"pointLabelFontSize\": 14, \"pointTextPadding\": 9}, \"themeVariables\": {\"quadrant1Fill\": \"#b4dcff\", \"quadrant2Fill\": \"#fef0ff\", \"quadrant3Fill\": \"#fffaf0\", \"quadrant4Fill\": \"#f0fff2\", \"quadrantPointFill\": \"#0149ff\", \"quadrantPointTextFill\": \"#dc00ff\", \"quadrantInternalBorderStrokeFill\": \"#3636f2\", \"quadrantExternalBorderStrokeFill\": \"#ff1010\"}}}%%\n\
             QuAdRaNtChArT\n\
             title Native rendering portfolio\n\
             X-AxIs \"Low reach 📉\" ---> \"`High reach Ω`\" %% axis comment\n\
             y-axis Low impact --> High impact\n\
             QuAdRaNt-1 \"`Invest 🚀`\"\n\
             quadrant-2 Explore\n\
             quadrant-3 Retire\n\
             quadrant-4 Maintain\n\
             Metal:::native: [0.78, 0.82] color: #ff3300\n\
             Direct2D: [0.58, 0.67] radius: 9, stroke-color: #166534, stroke-width: 3px\n\
             \"SVG [portable]\": [0.34, 0.45] %% point comment\n\
             ClAsSdEf native color: #109060, radius: 12, stroke-color: #310085, stroke-width: 4px\n",
        )
        .expect("quadrant chart should parse");
        assert_eq!(
            diagram.x_axis.as_ref().expect("x axis").categories,
            ["Low reach 📉", "High reach Ω"]
        );
        assert_eq!(diagram.quadrant_labels[0].as_deref(), Some("Invest 🚀"));
        let layout = layout_chart_diagram(&diagram, 640.0, 520.0);
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
                label_font: font_spec("Helvetica", 14.0),
                title_font: font_spec("Helvetica", 18.0),
                shaper: &shaper,
                metrics: &metrics,
                resolver: &resolver,
            },
        );
        assert_eq!(
            scene
                .instructions
                .iter()
                .filter(|instruction| matches!(instruction, PaintInstruction::Rect(_)))
                .count(),
            5
        );
        assert_eq!(
            scene
                .instructions
                .iter()
                .filter(|instruction| matches!(instruction, PaintInstruction::Ellipse(_)))
                .count(),
            3
        );
        let ellipses = scene
            .instructions
            .iter()
            .filter_map(|instruction| match instruction {
                PaintInstruction::Ellipse(ellipse) => Some(ellipse),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(ellipses[0].rx, 12.0);
        assert_eq!(ellipses[0].fill.as_deref(), Some("#ff3300"));
        assert_eq!(ellipses[0].stroke.as_deref(), Some("#310085"));
        assert_eq!(ellipses[0].stroke_width, Some(4.0));
        assert_eq!(ellipses[1].rx, 9.0);
        assert_eq!(ellipses[1].fill.as_deref(), Some("#0149ff"));
        assert_eq!(ellipses[1].stroke_width, Some(3.0));
        assert_eq!(ellipses[2].rx, 7.0);
        assert_eq!((scene.width, scene.height), (680.0, 560.0));
        let border = scene
            .instructions
            .iter()
            .find_map(|instruction| match instruction {
                PaintInstruction::Rect(rect) if rect.fill.as_deref() == Some("none") => Some(rect),
                _ => None,
            })
            .expect("external quadrant border");
        assert_eq!(border.stroke_width, Some(5.0));
        assert_eq!(border.stroke.as_deref(), Some("#ff1010"));
        assert!(
            scene
                .instructions
                .iter()
                .filter(|instruction| {
                    matches!(instruction, PaintInstruction::Path(path)
                if path.stroke_width == Some(3.0) && path.stroke.as_deref() == Some("#3636f2"))
                })
                .count()
                >= 2
        );
        assert!(scene
            .instructions
            .iter()
            .any(|instruction| matches!(instruction,
                PaintInstruction::Rect(rect) if rect.fill.as_deref() == Some("#b4dcff")
            )));

        let pixels = render(&scene);
        write_png(&pixels, "/tmp/mermaid_quadrant_e2e.png").expect("PNG write failed");
        assert!(pixels.width > 0);
        assert!(pixels.height > 0);
    }

    #[test]
    fn render_mermaid_sankey_to_png() {
        let diagram = parse_sankey(
            "SANKEY-BETA\nElectricity,\"Heating, homes\",\"45\"\nElectricity,Lighting,30\n\"Heating, homes\",Losses,8",
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
            "gitGraph TB:\ntitle GitGraph pipeline\naccTitle: Native GitGraph\naccDescr: GitGraph rendered through Metal\ncommit id: \"root\" tag: \"base\" tag: \"stable\" type: HIGHLIGHT\nbranch feature order: 2\ncommit id: \"work\" msg: \"Build parser\" type: REVERSE\ncherry-pick id: \"root\" tag: \"picked\" tag: \"backport\"\nbranch hotfix order: 1\ncommit id: \"fix\" msg: \"Patch release\"\ncheckout main\nmerge feature tag: \"v1\" tag: \"latest\"",
        )
        .expect("Mermaid GitGraph parse failed");
        let temporal = diagram_ir::TemporalDiagram {
            kind: diagram_ir::TemporalKind::Git,
            title: git.title.clone(),
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
        let metadata = scene.metadata.as_ref().expect("accessibility metadata");
        assert_eq!(metadata["accessibility.title"], "Native GitGraph");
        assert_eq!(
            metadata["accessibility.description"],
            "GitGraph rendered through Metal"
        );
    }

    #[test]
    fn render_mermaid_gantt_to_png() {
        let gantt = parse_gantt(
            "gantt\naccTitle: Native Gantt\naccDescr: Gantt rendered through Metal\ntitle Release timeline\ndateFormat YYYY-MM-DD\naxisFormat %m/%d\ntickInterval 1day\ninclusiveEndDates\ntopAxis\ntodayMarker off\nexcludes weekends\nincludes 2026-01-03\nsection Build\nParser :done, parser, 2026-01-01, 2026-01-04\nclick parser href \"https://example.com/parser\" call inspectTask(parser)\nWindow :window, 2025-12-29, until parser docs\nPaint :active, paint, after parser docs, 3d\nsection Ship\nDocs :docs, 2026-01-02, 2d\nReview :after docs, 1d\nPackage :active, 1d\nRelease :milestone, release, after task2, 0d",
        )
        .expect("Mermaid Gantt parse failed");
        let temporal = TemporalDiagram {
            kind: TemporalKind::Gantt,
            title: gantt.title.clone(),
            body: TemporalBody::Gantt(gantt),
        };
        let layout = layout_temporal_diagram(&temporal, 800.0);
        assert!(layout.items.iter().any(|item| matches!(
            item,
            diagram_ir::LayoutedTemporalItem::TimeAxisTick { label, .. }
                if label == "01/01"
        )));
        assert_eq!(layout.items.iter().filter(|item| matches!(
            item,
            diagram_ir::LayoutedTemporalItem::TimeAxisSpine { .. }
        )).count(), 2);

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
        write_png(&pixels, "/tmp/mermaid_gantt_e2e.png").expect("PNG write failed");
        assert!(pixels.width > 0);
        assert!(pixels.height > 0);
        assert!(!scene.instructions.is_empty());
        let metadata = scene.metadata.as_ref().expect("accessibility metadata");
        assert_eq!(metadata["accessibility.title"], "Native Gantt");
        assert_eq!(
            metadata["accessibility.description"],
            "Gantt rendered through Metal"
        );
        assert_eq!(
            metadata["gantt.task.parser.link.url"],
            "https://example.com/parser"
        );
        assert_eq!(metadata["gantt.task.parser.callback.name"], "inspectTask");
        assert_eq!(metadata["gantt.task.parser.callback.args"], "parser");
        assert!(metadata.contains_key("gantt.task.parser.bounds"));
    }

    #[test]
    fn render_mermaid_journey_to_png() {
        let (title, journey) = parse_journey(
            "%%{init: {\"journey\": {\"diagramMarginX\": 24, \"diagramMarginY\": 12, \"width\": 280, \"height\": 52, \"taskMargin\": 18, \"taskFontSize\": \"18px\", \"taskFontFamily\": \"Avenir Next\", \"titleFontSize\": \"22px\", \"titleFontFamily\": \"Georgia\", \"titleColor\": \"#123456\", \"actorColours\": [\"#010203\", \"#040506\"], \"sectionFills\": [\"#112233\", \"#445566\"], \"sectionColours\": [\"#fefefe\"], \"leftMargin\": 120, \"maxLabelWidth\": 56}}}%%\njourney\naccTitle: Checkout journey\naccDescr: Native checkout experience\ntitle Checkout<br/>experience\nsection Discover<br>products\nFind<br />product: 5: Alice Wonderland, Bob\nsection Payment\nPay: 2: Bob",
        )
        .expect("journey parse failed");
        let layout = layout_temporal_diagram(
            &TemporalDiagram {
                kind: TemporalKind::Journey,
                title,
                body: TemporalBody::Journey(Box::new(journey)),
            },
            720.0,
        );
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
                label_font: font_spec("Helvetica", 13.0),
                title_font: font_spec("Helvetica", 17.0),
                shaper: &shaper,
                metrics: &metrics,
                resolver: &resolver,
            },
        );
        assert_eq!(
            scene
                .instructions
                .iter()
                .filter(|instruction| matches!(
                    instruction,
                    PaintInstruction::Rect(rect) if rect.corner_radius == Some(6.0)
                ))
                .count(),
            2
        );
        assert_eq!(
            scene
                .instructions
                .iter()
                .filter(|instruction| matches!(
                    instruction,
                    PaintInstruction::Ellipse(ellipse) if ellipse.rx == 7.0 && ellipse.ry == 7.0
                ))
                .count(),
            2
        );
        assert_eq!(
            scene
                .instructions
                .iter()
                .filter(|instruction| matches!(
                    instruction,
                    PaintInstruction::Ellipse(ellipse) if ellipse.rx == 12.0 && ellipse.ry == 12.0
                ))
                .count(),
            2
        );
        for expected in ["#112233", "#445566"] {
            assert!(scene.instructions.iter().any(|instruction| matches!(
                instruction,
                PaintInstruction::Rect(rect) if rect.fill.as_deref() == Some(expected)
            )));
        }
        assert!(scene.instructions.iter().any(|instruction| matches!(
            instruction,
            PaintInstruction::Ellipse(ellipse) if ellipse.fill.as_deref() == Some("#010203")
        )));
        assert_eq!(
            scene
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.get("accessibility.title"))
                .map(String::as_str),
            Some("Checkout journey")
        );
        assert_eq!(
            scene
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.get("accessibility.description"))
                .map(String::as_str),
            Some("Native checkout experience")
        );
        let pixels = render(&scene);
        write_png(&pixels, "/tmp/mermaid_journey_e2e.png").expect("PNG write failed");
        assert!(pixels.width > 0 && pixels.height > 0);
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
    fn render_mermaid_requirement_to_png() {
        let diagram = parse_requirement_diagram(
            "ReQuIrEmEnTdIaGrAm\nAcCtItLe: Native requirement graph\nAcCdEsCr: Requirement graph rendered through Metal\nDiReCtIoN lr\nClAsSdEf \"important class\" fill:#fff1a8,stroke:#b45309,stroke-width:4px,color:#7c2d12,font-size:22px,font-weight:bold,font-style:italic,font-family:Helvetica\nReQuIrEmEnT \"Test requirement\" {\nID: 1\nTeXt: Test\nRiSk: LOW\nVeRiFyMeThOd: TeSt\n}\nClAsS \"Test requirement\" \"important class\"\nElEmEnT \"System element\" {\nTyPe: service\n}\n\"System element\" - SaTiSfIeS -> \"Test requirement\"",
        )
        .expect("Mermaid requirement parse failed");
        let layout = layout_structural_diagram(&diagram);
        assert!(layout.nodes[1].x > layout.nodes[0].x);
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
        write_png(&pixels, "/tmp/mermaid_requirement_e2e.png").expect("PNG write failed");
        assert!(pixels.width > 0 && pixels.height > 0);
        assert!(!scene.instructions.is_empty());
        assert!(scene.instructions.iter().any(|instruction| matches!(
            instruction,
            PaintInstruction::Rect(rect)
                if rect.fill.as_deref() == Some("#fff1a8")
                    && rect.stroke.as_deref() == Some("#b45309")
                    && rect.stroke_width == Some(4.0)
        )));
        assert!(scene.instructions.iter().any(|instruction| matches!(
            instruction,
            PaintInstruction::GlyphRun(run) if run.font_size == 22.0
        )));
        let metadata = scene.metadata.as_ref().expect("accessibility metadata");
        assert_eq!(metadata["accessibility.title"], "Native requirement graph");
        assert_eq!(
            metadata["accessibility.description"],
            "Requirement graph rendered through Metal"
        );
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
        let mut diagram = parse_sequence_diagram(
            "---\ntitle: Host-owned front matter title\nconfig:\n  theme: neutral\n---\n%%{init: {'logLevel': 0}}%%\nSeQuEnCeDiAgRaM;%%{wrap}%%\nTiTlE: Native Mermaid sequence;AcCtItLe: Native transfer sequence;AcCdEsCr {\n  Banking transfer\n  interaction\n}\nautonumber\nbox hsl(180, 100%, 50%) wrap: A deliberately detailed client application tier\nactor Banking User-Primary as User # actor comment\nparticipant API@{ \"type\": \"boundary\" } as wrap: A deliberately detailed public application programming interface\nend\nbox Services\nparticipant DB@{ \"type\": \"database\", \"alias\": \"Ledger, \\\"primary\\\"\" }\nend\nalt wrap: Transfer accepted after a deliberately detailed native policy and compliance evaluation\nBanking User-Primary->>+API: wrap: Submit a deliberately detailed native transfer request\nautonumber off\ncreate participant Worker as Audit Worker\nAPI()->>()Worker: Start audit\nactivate Worker\nactivate Worker\nWorker()->>()Worker: Check nested audit state\ndeactivate Worker\ndeactivate Worker\ndeactivate Worker\ndeactivate Worker\ndeactivate Worker\nautonumber 20.05 0.1\ndestroy Worker\nWorker--|\\API: Audit complete\nloop Persist until committed\nautonumber off\nAPI->>DB: Record transaction # persistence comment\nautonumber\nDB-->>API: Committed\nend\npar_over Reconcile with parallel annotations\nAPI->>DB: Reconcile ledger\nnote left of API: Client view\nnote right of DB: Ledger view\nend\nnote right of API: Metal #9829;<br/>native scene # note comment\nAPI-->>-Banking User-Primary: Transfer complete\nelse wrap: Transfer rejected after a deliberately detailed native policy and compliance evaluation\nAPI-->>Banking User-Primary: Validation failed\nend",
        )
        .expect("Mermaid sequence parse failed");
        diagram.auto_number_start = 10.5;
        diagram.auto_number_step = 2.25;
        diagram.events.insert(
            0,
            SequenceEvent::BlockStart {
                kind: SequenceBlockKind::Rect,
                label: String::new(),
                wrap: diagram_ir::SequenceTextWrap::Default,
                fill: Some("rgba(255, 200, 100, 0.12)".into()),
            },
        );
        diagram.events.push(SequenceEvent::BlockEnd {
            kind: SequenceBlockKind::Rect,
        });
        diagram.participants[0].links.push(SequenceLink {
            label: "Dashboard".into(),
            url: "https://example.com/dashboard".into(),
        });
        diagram.participants[0].properties.push(SequenceProperty {
            name: "role".into(),
            value_json: "\"administrator\"".into(),
        });
        diagram.participants[0].properties.push(SequenceProperty {
            name: "icon".into(),
            value_json: "\"@clock\"".into(),
        });
        diagram.participants[0].details_reference = Some("user-details".into());
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
        assert_eq!(
            scene
                .metadata
                .as_ref()
                .and_then(|metadata| {
                    metadata.get("sequence.participant.Banking User-Primary.link.Dashboard")
                })
                .map(String::as_str),
            Some("https://example.com/dashboard")
        );
        assert_eq!(
            scene
                .metadata
                .as_ref()
                .and_then(|metadata| {
                    metadata.get("sequence.participant.Banking User-Primary.property.role")
                })
                .map(String::as_str),
            Some("\"administrator\"")
        );
        assert_eq!(
            scene
                .metadata
                .as_ref()
                .and_then(|metadata| {
                    metadata.get("sequence.participant.Banking User-Primary.details_reference")
                })
                .map(String::as_str),
            Some("user-details")
        );
        assert_eq!(
            scene
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.get("accessibility.title"))
                .map(String::as_str),
            Some("Native transfer sequence")
        );
        assert_eq!(
            scene
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.get("accessibility.description"))
                .map(String::as_str),
            Some("Banking transfer\n  interaction")
        );
        assert!(!scene.instructions.iter().any(|instruction| matches!(
            instruction,
            paint_instructions::PaintInstruction::Path(path)
                if path.stroke.as_deref() == Some("#dc2626")
        )));
        assert!(scene.instructions.iter().any(|instruction| matches!(
            instruction,
            paint_instructions::PaintInstruction::Rect(rect)
                if rect.stroke.as_deref() == Some("#64748b")
        )));
        assert!(scene.instructions.iter().any(|instruction| matches!(
            instruction,
            paint_instructions::PaintInstruction::Rect(rect)
                if rect.stroke.as_deref() == Some("#94a3b8")
        )));
        assert!(scene.instructions.iter().any(|instruction| matches!(
            instruction,
            paint_instructions::PaintInstruction::Ellipse(ellipse)
                if ellipse.rx == 5.0 && ellipse.ry == 5.0
        )));
        assert!(scene.instructions.iter().any(|instruction| matches!(
            instruction,
            paint_instructions::PaintInstruction::Ellipse(ellipse)
                if ellipse.rx == 7.0 && ellipse.ry == 7.0
        )));
        assert!(scene.instructions.iter().any(|instruction| matches!(
            instruction,
            paint_instructions::PaintInstruction::Rect(rect)
                if rect.fill.as_deref() == Some("rgba(255, 200, 100, 0.12)")
        )));
    }

    #[test]
    fn render_mermaid_sequence_default_rect_to_png() {
        let diagram = parse_sequence_diagram(
            "sequenceDiagram\nrect\nAlice->>Bob: Theme default background\nend\n",
        )
        .expect("Mermaid sequence default rect parse failed");
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
        write_png(&pixels, "/tmp/mermaid_sequence_default_rect_e2e.png").expect("PNG write failed");
        assert!(pixels.width > 0);
        assert!(pixels.height > 0);
        assert!(scene.instructions.iter().any(|instruction| matches!(
            instruction,
            paint_instructions::PaintInstruction::Rect(rect)
                if rect.fill.as_deref() == Some("#fff7ed")
        )));
    }
}
