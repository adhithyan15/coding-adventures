// Complete authored source/IR/layout/Scene/query products, independent of
// returned values. Taller cards preserve the painter's fixed text height22.
#[test]
fn period_height_source_products_keep_complete_scenes_and_queries() {
    const FAMILY: &str = "\"trebuchet ms\", verdana, arial, sans-serif";
    for (header, td) in [
        ("timeline TD", true),
        ("timeline LR", false),
        ("timeline", false),
    ] {
        for hard in [false, true] {
            let authored = if hard {
                "A<br/>B<br/>C".into()
            } else {
                "x".repeat(49)
            };
            let label = if hard {
                "A\nB\nC".into()
            } else {
                "x".repeat(49)
            };
            let cfg = TimelineConfig {
                font_family: FAMILY.into(),
                font_size: 16.0,
                padding: 50.0,
                left_margin: 50.0,
                use_max_width: false,
                disable_multicolor: false,
                section_colors: [
                    "#dbeafe", "#dcfce7", "#fef3c7", "#fae8ff", "#ffe4e6", "#cffafe",
                ]
                .map(str::to_string)
                .to_vec(),
                section_label_colors: vec!["#0f172a".into(); 6],
                section_inverse_colors: [
                    "#2563eb", "#16a34a", "#d97706", "#a21caf", "#e11d48", "#0891b2",
                ]
                .map(str::to_string)
                .to_vec(),
            };
            let semantic = TimelineDiagram {
                accessibility_title: None,
                accessibility_description: None,
                config: cfg.clone(),
                direction: if td {
                    TimelineDirection::TopDown
                } else {
                    TimelineDirection::LeftRight
                },
                sections: vec![TimelineSection {
                    label: None,
                    periods: vec![TimelinePeriod {
                        label: label.clone(),
                        events: Vec::new(),
                    }],
                }],
            };
            let (px, py, pw, height, sx, sy1, sy2, center, baseline) = if td {
                (234.0, 50.0, 180.0, 197.6, 206.0, 66.0, 111.6, 324.0, 70.8)
            } else {
                (
                    206.0, 118.0, 190.0, 255.6, 301.0, 106.0, 106.0, 301.0, 138.8,
                )
            };
            let period = LayoutedTemporalItem::TimelinePeriod {
                x: px,
                y: py,
                width: pw,
                height: 69.6,
                label: label.clone(),
                events: Vec::new(),
                color_index: 0,
            };
            let spine = LayoutedTemporalItem::TimelineSpine {
                x1: sx,
                y1: sy1,
                x2: sx,
                y2: sy2,
            };
            let expected_layout = LayoutedTemporalDiagram {
                width: 640.0,
                height,
                accessibility_title: None,
                accessibility_description: None,
                timeline_config: Some(cfg.clone()),
                interactions: Vec::new(),
                items: if td {
                    vec![spine, period]
                } else {
                    vec![period, spine]
                },
            };
            let mut instructions = vec![rect(
                (px, py, pw, 69.6),
                "#dbeafe",
                Some("#2563eb"),
                2.0,
                8.0,
            )];
            let spine_paint = path(&[(sx, sy1), (sx, sy2)], "#2563eb", 3.0, false, false);
            if td {
                instructions.insert(0, spine_paint);
            } else {
                instructions.push(spine_paint);
            }
            if hard {
                // FakeMetrics baseline starts at text_y+12.8 then advances16.
                // This is deliberate evidence that the unchanged22-high text
                // box does not clip the three glyph runs to the estimate.
                let baselines = if td {
                    [70.8, 86.8, 102.8]
                } else {
                    [138.8, 154.8, 170.8]
                };
                for (value, y) in ["A", "B", "C"].into_iter().zip(baselines) {
                    instructions.push(glyph(value, center, y, 16.0, "rgb(15, 23, 42)"));
                }
            } else {
                // Normal word breaking keeps the49-scalar unbroken word.
                // Its glyph run can exceed the frame: no backend-fit claim.
                instructions.push(glyph(&label, center, baseline, 16.0, "rgb(15, 23, 42)"));
            }
            let mut metadata = HashMap::from([
                ("timeline.fontFamily".into(), FAMILY.into()),
                ("timeline.fontSize".into(), "16".into()),
                ("timeline.padding".into(), "50".into()),
                ("timeline.leftMargin".into(), "50".into()),
                ("timeline.useMaxWidth".into(), "false".into()),
                ("timeline.disableMulticolor".into(), "false".into()),
            ]);
            for (name, colors) in [
                ("cScale", &cfg.section_colors),
                ("cScaleLabel", &cfg.section_label_colors),
                ("cScaleInv", &cfg.section_inverse_colors),
            ] {
                for (i, color) in colors.iter().enumerate() {
                    metadata.insert(format!("timeline.theme.{name}{i}"), color.clone());
                }
            }
            let expected_scene = PaintScene {
                width: 640.0,
                height,
                background: "rgb(255,255,255)".into(),
                id: None,
                metadata: Some(metadata),
                instructions,
            };
            let expected_queries = vec![FontQuery::named(FAMILY)];
            let source = format!("{header}\n{authored}\n");
            let parsed = mermaid_parser::parse_timeline(&source).unwrap();
            assert_eq!(parsed, (None, semantic), "whole IR {header}:{hard}");
            let actual = diagram_layout_temporal::try_layout_temporal_diagram(
                &TemporalDiagram {
                    kind: TemporalKind::Timeline,
                    title: parsed.0,
                    body: TemporalBody::Timeline(parsed.1),
                },
                640.0,
            )
            .unwrap();
            assert_eq!(actual, expected_layout, "whole layout {header}:{hard}");
            let resolver = Resolver(std::cell::RefCell::new(Vec::new()));
            let defaults = make_opts(&FakeShaper, &FakeMetrics, &FakeResolver);
            let options = DiagramToPaintOptions {
                label_font: defaults.label_font,
                title_font: defaults.title_font,
                resolver: &resolver,
                shaper: &FakeShaper,
                metrics: &FakeMetrics,
                background: defaults.background,
                device_pixel_ratio: defaults.device_pixel_ratio,
            };
            assert_eq!(
                diagram_to_paint_temporal(&actual, &options),
                expected_scene,
                "whole Scene {header}:{hard}"
            );
            assert_eq!(
                *resolver.0.borrow(),
                expected_queries,
                "whole query vector {header}:{hard}"
            );
            println!("timeline-period-height-product COMPLETE {header}:{hard}");
        }
    }
}
