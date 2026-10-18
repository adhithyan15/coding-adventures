// Included in the existing portable oracle module to reuse its declared paint
// primitives. No oracle geometry is obtained from returned production output.
#[test]
fn default_and_explicit_direction_keep_complete_product_contracts() {
    for (header, td) in [
        ("timeline TD", true),
        ("timeline LR", false),
        ("timeline", false),
    ] {
        let direction = if td {
            TimelineDirection::TopDown
        } else {
            TimelineDirection::LeftRight
        };
        let cfg = TimelineConfig {
            font_size: 16.0,
            font_family: "\"trebuchet ms\", verdana, arial, sans-serif".into(),
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
            direction,
            sections: vec![TimelineSection {
                label: None,
                periods: vec![TimelinePeriod {
                    label: "P".into(),
                    events: vec!["E".into()],
                }],
            }],
        };
        let (height, px, py, pw, ex, ey, ew, x1, y1, x2, y2) = if td {
            (
                176.0, 234.0, 50.0, 180.0, 450.0, 50.0, 160.0, 414.0, 74.0, 450.0, 65.0,
            )
        } else {
            (
                296.0, 206.0, 118.0, 190.0, 206.0, 184.0, 190.0, 301.0, 166.0, 301.0, 184.0,
            )
        };
        let spine = if td {
            LayoutedTemporalItem::TimelineSpine {
                x1: 206.0,
                y1: 66.0,
                x2: 206.0,
                y2: 90.0,
            }
        } else {
            LayoutedTemporalItem::TimelineSpine {
                x1: 301.0,
                y1: 106.0,
                x2: 301.0,
                y2: 106.0,
            }
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
                height: 30.0,
                label: "E".into(),
                color_index: 0,
            },
        ];
        if td {
            items.insert(0, spine);
        } else {
            items.push(spine);
        }
        let expected_layout = LayoutedTemporalDiagram {
            width: 640.0,
            height,
            accessibility_title: None,
            accessibility_description: None,
            timeline_config: Some(cfg.clone()),
            interactions: Vec::new(),
            items,
        };
        let spine_paint = if td {
            path(
                &[(206.0, 66.0), (206.0, 90.0)],
                "#2563eb",
                3.0,
                false,
                false,
            )
        } else {
            path(
                &[(301.0, 106.0), (301.0, 106.0)],
                "#2563eb",
                3.0,
                false,
                false,
            )
        };
        let arrow = if td {
            [
                (450.0, 65.0),
                (
                    450.0 - 9.70142500145332 + 1.212678125181665,
                    65.0 + 2.42535625036333 + 4.85071250072666,
                ),
                (
                    450.0 - 9.70142500145332 - 1.212678125181665,
                    65.0 + 2.42535625036333 - 4.85071250072666,
                ),
            ]
        } else {
            [(301.0, 184.0), (296.0, 174.0), (306.0, 174.0)]
        };
        let mut instructions = vec![
            rect((px, py, pw, 48.0), "#dbeafe", Some("#2563eb"), 2.0, 8.0),
            path(&[(x1, y1), (x2, y2)], "#2563eb", 2.0, false, true),
            path(&arrow, "#2563eb", 0.0, true, false),
            rect((ex, ey, ew, 30.0), "#dbeafe", Some("#2563eb"), 1.5, 6.0),
        ];
        if td {
            instructions.insert(0, spine_paint);
        } else {
            instructions.push(spine_paint);
        }
        instructions.extend([
            glyph(
                "P",
                if td { 324.0 } else { 301.0 },
                if td { 72.4 } else { 140.4 },
                18.0,
                "rgb(15, 23, 42)",
            ),
            glyph(
                "E",
                if td { 530.0 } else { 301.0 },
                if td { 67.2 } else { 201.2 },
                14.0,
                "rgb(15, 23, 42)",
            ),
        ]);
        let mut metadata = HashMap::from([
            ("timeline.fontFamily".into(), "\"trebuchet ms\", verdana, arial, sans-serif".into()),
            ("timeline.padding".into(), "50".into()),
            ("timeline.leftMargin".into(), "50".into()),
            ("timeline.useMaxWidth".into(), "false".into()),
            ("timeline.disableMulticolor".into(), "false".into()),
        ]);
        for (family, colors) in [
            ("cScale", &cfg.section_colors),
            ("cScaleLabel", &cfg.section_label_colors),
            ("cScaleInv", &cfg.section_inverse_colors),
        ] {
            for (i, color) in colors.iter().enumerate() {
                metadata.insert(format!("timeline.theme.{family}{i}"), color.clone());
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
        let expected_scene = super::declared_timeline_typography(expected_scene, &expected_layout);
        let predecessor = (expected_layout.clone(), expected_scene.clone());
        let (expected_layout, expected_scene) = migrate_event_product(expected_layout, expected_scene, td);
        let expected_queries = vec![
            FontQuery::named("\"trebuchet ms\", verdana, arial, sans-serif").with_weight(text_interfaces::FontWeight(700)),
            FontQuery::named("\"trebuchet ms\", verdana, arial, sans-serif"),
        ];
        let expected_queries = super::declared_timeline_period_queries(expected_queries, &expected_layout, FontQuery::named("\"trebuchet ms\", verdana, arial, sans-serif"), FontQuery::named("\"trebuchet ms\", verdana, arial, sans-serif").with_weight(text_interfaces::FontWeight(700)));
        let source = format!("{header}\nP : E\n");
        let parsed = mermaid_parser::parse_timeline(&source).unwrap();
        assert_eq!(parsed, (None, semantic), "whole IR {header}");
        let actual_layout = diagram_layout_temporal::try_layout_temporal_diagram(
            &TemporalDiagram {
                kind: TemporalKind::Timeline,
                title: parsed.0,
                body: TemporalBody::Timeline(parsed.1),
            },
            640.0,
        )
        .unwrap();
        assert_eq!(actual_layout, expected_layout, "whole layout {header}");
        let resolver = Resolver(std::cell::RefCell::new(Vec::new()));
        let defaults = make_opts(&FakeShaper, &FakeMetrics, &FakeResolver);
        let options = DiagramToPaintOptions {
            label_font: FontSpec {
                family: "Georgia".into(),
                ..defaults.label_font
            },
            title_font: FontSpec {
                family: "Courier New".into(),
                ..defaults.title_font
            },
            resolver: &resolver,
            shaper: &FakeShaper,
            metrics: &FakeMetrics,
            background: defaults.background,
            device_pixel_ratio: defaults.device_pixel_ratio,
        };
        prove_event_predecessor(&predecessor, &options, &expected_queries);
        assert_eq!(
            diagram_to_paint_temporal(&actual_layout, &options),
            expected_scene,
            "whole Scene {header}"
        );
        assert_eq!(
            *resolver.0.borrow(),
            expected_queries,
            "ordered queries {header}"
        );
        println!("timeline-direction-product COMPLETE {header}");
    }
}
