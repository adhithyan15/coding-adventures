// Independently declared s=24 source/IR/layout/Scene/query products. The old
// default-size goldens remain untouched in their original modules.
#[test]
fn size_layout_minima_source_products_keep_complete_scenes_and_queries() {
    for (header, td) in [
        ("timeline TD", true),
        ("timeline LR", false),
        ("timeline", false),
    ] {
        let cfg = TimelineConfig {
            font_size: 24.0,
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
            direction: if td {
                TimelineDirection::TopDown
            } else {
                TimelineDirection::LeftRight
            },
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
                200.0, 234.0, 50.0, 180.0, 450.0, 50.0, 160.0, 414.0, 86.0, 450.0, 72.5,
            )
        } else {
            (
                335.0, 206.0, 118.0, 190.0, 206.0, 208.0, 190.0, 301.0, 190.0, 301.0, 208.0,
            )
        };
        let spine = if td {
            LayoutedTemporalItem::TimelineSpine {
                x1: 206.0,
                y1: 66.0,
                x2: 206.0,
                y2: 114.0,
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
                height: 72.0,
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
                height: 45.0,
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
                &[(206.0, 66.0), (206.0, 114.0)],
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
        // dx36/dy-13.5, norm sqrt(1478.25). These literal unit-vector
        // products are independent of any returned layout, path or Scene.
        let arrow = if td {
            [
                (450.0, 72.5),
                (
                    450.0 - 9.363291775690444 + 1.7556172079419585,
                    72.5 + 3.511234415883917 + 4.681645887845222,
                ),
                (
                    450.0 - 9.363291775690444 - 1.7556172079419585,
                    72.5 + 3.511234415883917 - 4.681645887845222,
                ),
            ]
        } else {
            [(301.0, 208.0), (296.0, 198.0), (306.0, 198.0)]
        };
        let mut instructions = vec![
            rect((px, py, pw, 72.0), "#dbeafe", Some("#2563eb"), 2.0, 8.0),
            path(&[(x1, y1), (x2, y2)], "#2563eb", 2.0, false, true),
            path(&arrow, "#2563eb", 0.0, true, false),
            rect((ex, ey, ew, 45.0), "#dbeafe", Some("#2563eb"), 1.5, 6.0),
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
                if td { 77.2 } else { 145.2 },
                24.0,
                "rgb(15, 23, 42)",
            ),
            glyph(
                "E",
                if td { 530.0 } else { 301.0 },
                if td { 75.2 } else { 233.2 },
                24.0,
                "rgb(15, 23, 42)",
            ),
        ]);
        let mut metadata = HashMap::from([
            (
                "timeline.fontFamily".into(),
                "\"trebuchet ms\", verdana, arial, sans-serif".into(),
            ),
            ("timeline.fontSize".into(), "24".into()),
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
        let expected_queries = vec![FontQuery::named(
            "\"trebuchet ms\", verdana, arial, sans-serif",
        )];
        let source = format!("%%{{init: {{\"fontSize\":24}}}}%%\n{header}\nP : E\n");
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
            label_font: defaults.label_font,
            title_font: defaults.title_font,
            resolver: &resolver,
            shaper: &FakeShaper,
            metrics: &FakeMetrics,
            background: defaults.background,
            device_pixel_ratio: defaults.device_pixel_ratio,
        };
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
        println!("timeline-minima-product COMPLETE {header}");
    }
}

// Closed migration of PREDECLARED root24 family products. Keep their entire
// original source, literal layout/Scene bodies and query vectors as proof inputs.
fn migrate_declared_family_size_layout(
    old_layout: LayoutedTemporalDiagram,
    old_scene: PaintScene,
    td: bool,
    titled: bool,
    family: &str,
) -> (LayoutedTemporalDiagram, PaintScene) {
    let (_, mut proof_layout, proof_scene) = declared_family_product(td, titled, family);
    proof_layout.timeline_config.as_mut().unwrap().font_size = 24.0;
    let proof_scene = super::declared_timeline_typography(proof_scene, &proof_layout);
    assert_eq!(
        old_layout, proof_layout,
        "complete old root24 literal layout"
    );
    assert_eq!(
        old_scene, proof_scene,
        "complete old root24 Scene after typography"
    );
    let (height, px, py, pw, ex, ey, ew, x1, y1, x2, y2, sx, sy1, sy2, pb, eb) = match (td, titled)
    {
        (true, false) => (
            200.0, 234.0, 50.0, 180.0, 450.0, 50.0, 160.0, 414.0, 86.0, 450.0, 72.5, 206.0, 66.0,
            114.0, 77.2, 75.2,
        ),
        (true, true) => (
            272.0, 234.0, 122.0, 180.0, 450.0, 122.0, 160.0, 414.0, 158.0, 450.0, 144.5, 206.0,
            138.0, 186.0, 149.2, 147.2,
        ),
        (false, false) => (
            335.0, 206.0, 118.0, 190.0, 206.0, 208.0, 190.0, 301.0, 190.0, 301.0, 208.0, 301.0,
            106.0, 106.0, 145.2, 233.2,
        ),
        (false, true) => (
            407.0, 206.0, 190.0, 190.0, 206.0, 280.0, 190.0, 301.0, 262.0, 301.0, 280.0, 301.0,
            178.0, 178.0, 217.2, 305.2,
        ),
    };
    let mut items = vec![
        LayoutedTemporalItem::TimelinePeriod {
            x: px,
            y: py,
            width: pw,
            height: 72.0,
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
            height: 45.0,
            label: "E".into(),
            color_index: 0,
        },
    ];
    let spine = LayoutedTemporalItem::TimelineSpine {
        x1: sx,
        y1: sy1,
        x2: sx,
        y2: sy2,
    };
    if td {
        items.insert(0, spine);
    } else {
        items.push(spine);
    }
    if titled {
        items.insert(
            0,
            LayoutedTemporalItem::TemporalTitle {
                x: 50.0,
                y: 50.0,
                width: 540.0,
                height: 60.0,
                label: "T".into(),
            },
        );
    }
    let layout = LayoutedTemporalDiagram {
        width: 640.0,
        height,
        accessibility_title: None,
        accessibility_description: None,
        timeline_config: old_layout.timeline_config,
        interactions: Vec::new(),
        items,
    };
    let arrow = if td {
        [
            (x2, y2),
            (
                x2 - 9.363291775690444 + 1.7556172079419585,
                y2 + 3.511234415883917 + 4.681645887845222,
            ),
            (
                x2 - 9.363291775690444 - 1.7556172079419585,
                y2 + 3.511234415883917 - 4.681645887845222,
            ),
        ]
    } else {
        [(x2, y2), (x2 - 5.0, y2 - 10.0), (x2 + 5.0, y2 - 10.0)]
    };
    let mut instructions = vec![
        rect((px, py, pw, 72.0), "#dbeafe", Some("#2563eb"), 2.0, 8.0),
        path(&[(x1, y1), (x2, y2)], "#2563eb", 2.0, false, true),
        path(&arrow, "#2563eb", 0.0, true, false),
        rect((ex, ey, ew, 45.0), "#dbeafe", Some("#2563eb"), 1.5, 6.0),
    ];
    let spine = path(&[(sx, sy1), (sx, sy2)], "#2563eb", 3.0, false, false);
    if td {
        instructions.insert(0, spine);
    } else {
        instructions.push(spine);
    }
    if titled {
        instructions.push(glyph("T", 320.0, 94.4, 48.0, "rgb(17, 24, 39)"));
    }
    instructions.extend([
        glyph("P", px + pw / 2.0, pb, 24.0, "rgb(15, 23, 42)"),
        glyph("E", ex + ew / 2.0, eb, 24.0, "rgb(15, 23, 42)"),
    ]);
    let scene = PaintScene {
        width: 640.0,
        height,
        instructions,
        ..old_scene
    };
    (layout, scene)
}
