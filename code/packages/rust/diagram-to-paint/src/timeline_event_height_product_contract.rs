// Full declared source/IR/layout/Scene/query products. No returned geometry
// supplies an expectation; even an unbroken word's overflowing glyph run stays.
#[test]
fn event_height_source_products_keep_complete_scenes_and_queries() {
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
                    periods: vec![
                        TimelinePeriod {
                            label: "P".into(),
                            events: vec![label.clone()],
                        },
                        TimelinePeriod {
                            label: "Q".into(),
                            events: Vec::new(),
                        },
                    ],
                }],
            };
            let (width, height, px, py, pw, ex, ey, ew, qx, qy) = if td {
                (
                    640.0, 263.6, 234.0, 50.0, 180.0, 450.0, 50.0, 160.0, 234.0, 137.6,
                )
            } else {
                (
                    660.0, 335.6, 206.0, 118.0, 190.0, 206.0, 184.0, 190.0, 420.0, 118.0,
                )
            };
            let spine = if td {
                LayoutedTemporalItem::TimelineSpine {
                    x1: 206.0,
                    y1: 66.0,
                    x2: 206.0,
                    y2: 177.6,
                }
            } else {
                LayoutedTemporalItem::TimelineSpine {
                    x1: 301.0,
                    y1: 106.0,
                    x2: 515.0,
                    y2: 106.0,
                }
            };
            let (x1, y1, x2, y2) = if td {
                (414.0, 74.0, 450.0, 84.8)
            } else {
                (301.0, 166.0, 301.0, 184.0)
            };
            let mut items = vec![
                LayoutedTemporalItem::TimelinePeriod {
                    x: px,
                    y: py,
                    width: pw,
                    height: 48.0,
                    label: "P".into(),
                    events: Vec::new(),
                    color_index: 0,
                },
                LayoutedTemporalItem::TimelineEventLink {
                    x1,
                    y1,
                    x2,
                    y2,
                    color_index: 0,
                },
                LayoutedTemporalItem::TimelineEvent {
                    x: ex,
                    y: ey,
                    width: ew,
                    height: 69.6,
                    label: label.clone(),
                    color_index: 0,
                },
                LayoutedTemporalItem::TimelinePeriod {
                    x: qx,
                    y: qy,
                    width: pw,
                    height: 48.0,
                    label: "Q".into(),
                    events: Vec::new(),
                    color_index: 1,
                },
            ];
            if td {
                items.insert(0, spine);
            } else {
                items.push(spine);
            }
            let expected_layout = LayoutedTemporalDiagram {
                width,
                height,
                accessibility_title: None,
                accessibility_description: None,
                timeline_config: Some(cfg.clone()),
                interactions: Vec::new(),
                items,
            };
            // Independent binary64 square/sqrt/normalize/corner arithmetic for
            // dx36, dy10.799999999999997, with the exact declared arrow order.
            let arrow = if td {
                [
                    (450.0, 84.8),
                    (438.9849977199567, 86.7156525704423),
                    (441.8584765756202, 77.13738971823079),
                ]
            } else {
                [(301.0, 184.0), (296.0, 174.0), (306.0, 174.0)]
            };
            let mut instructions = vec![
                rect((px, py, pw, 48.0), "#dbeafe", Some("#2563eb"), 2.0, 8.0),
                path(&[(x1, y1), (x2, y2)], "#2563eb", 2.0, false, true),
                path(&arrow, "#2563eb", 0.0, true, false),
                rect((ex, ey, ew, 69.6), "#dbeafe", Some("#2563eb"), 1.5, 6.0),
                rect((qx, qy, pw, 48.0), "#dcfce7", Some("#16a34a"), 2.0, 8.0),
            ];
            let spine_paint = if td {
                path(
                    &[(206.0, 66.0), (206.0, 177.6)],
                    "#2563eb",
                    3.0,
                    false,
                    false,
                )
            } else {
                path(
                    &[(301.0, 106.0), (515.0, 106.0)],
                    "#2563eb",
                    3.0,
                    false,
                    false,
                )
            };
            if td {
                instructions.insert(0, spine_paint);
            } else {
                instructions.push(spine_paint);
            }
            instructions.push(glyph(
                "P",
                if td { 324.0 } else { 301.0 },
                if td { 70.8 } else { 138.8 },
                16.0,
                "rgb(15, 23, 42)",
            ));
            if hard {
                for (value, baseline) in ["A", "B", "C"].into_iter().zip(if td {
                    [68.8, 84.8, 100.8]
                } else {
                    [202.8, 218.8, 234.8]
                }) {
                    instructions.push(glyph(
                        value,
                        if td { 530.0 } else { 301.0 },
                        baseline,
                        16.0,
                        "rgb(15, 23, 42)",
                    ));
                }
            } else {
                instructions.push(glyph(
                    &label,
                    if td { 530.0 } else { 301.0 },
                    if td { 68.8 } else { 202.8 },
                    16.0,
                    "rgb(15, 23, 42)",
                ));
            }
            instructions.push(glyph(
                "Q",
                if td { 324.0 } else { 515.0 },
                if td { 158.4 } else { 138.8 },
                16.0,
                "rgb(15, 23, 42)",
            ));
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
                for (index, color) in colors.iter().enumerate() {
                    metadata.insert(format!("timeline.theme.{name}{index}"), color.clone());
                }
            }
            let expected_scene = PaintScene {
                width,
                height,
                background: "rgb(255,255,255)".into(),
                id: None,
                metadata: Some(metadata),
                instructions,
            };
            let expected_queries = vec![FontQuery::named(FAMILY)];
            let source = format!("{header}\nP : {authored}\nQ\n");
            let parsed = mermaid_parser::parse_timeline(&source).unwrap();
            assert_eq!(parsed, (None, semantic), "whole event IR {header}:{hard}");
            let actual = diagram_layout_temporal::try_layout_temporal_diagram(
                &TemporalDiagram {
                    kind: TemporalKind::Timeline,
                    title: parsed.0,
                    body: TemporalBody::Timeline(parsed.1),
                },
                640.0,
            )
            .unwrap();
            assert_eq!(
                actual, expected_layout,
                "whole event layout {header}:{hard}"
            );
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
                "whole event Scene {header}:{hard}"
            );
            assert_eq!(
                *resolver.0.borrow(),
                expected_queries,
                "whole event query vector {header}:{hard}"
            );
            println!("timeline-event-height-product COMPLETE {header}:{hard}");
        }
    }
}
