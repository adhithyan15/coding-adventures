// Independent fixed frames and glyph positions; queried fonts are declared data.
fn declared_family_product(
    td: bool,
    titled: bool,
    family: &str,
) -> (TimelineDiagram, LayoutedTemporalDiagram, PaintScene) {
    let cfg = TimelineConfig {
        font_size: 16.0,
        font_family: family.into(),
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
    let (height, px, py, pw, ex, ey, ew, x1, y1, x2, y2, sx, sy1, sy2, pbase, ebase) =
        match (td, titled) {
            (true, false) => (
                176.0, 234.0, 50.0, 180.0, 450.0, 50.0, 160.0, 414.0, 74.0, 450.0, 65.0, 206.0,
                66.0, 90.0, 72.4, 67.2,
            ),
            (true, true) => (
                228.0, 234.0, 102.0, 180.0, 450.0, 102.0, 160.0, 414.0, 126.0, 450.0, 117.0, 206.0,
                118.0, 142.0, 124.4, 119.2,
            ),
            (false, false) => (
                296.0, 206.0, 118.0, 190.0, 206.0, 184.0, 190.0, 301.0, 166.0, 301.0, 184.0, 301.0,
                106.0, 106.0, 140.4, 201.2,
            ),
            (false, true) => (
                348.0, 206.0, 170.0, 190.0, 206.0, 236.0, 190.0, 301.0, 218.0, 301.0, 236.0, 301.0,
                158.0, 158.0, 192.4, 253.2,
            ),
        };
    let spine = LayoutedTemporalItem::TimelineSpine {
        x1: sx,
        y1: sy1,
        x2: sx,
        y2: sy2,
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
    if titled {
        items.insert(
            0,
            LayoutedTemporalItem::TemporalTitle {
                x: 50.0,
                y: 50.0,
                width: 540.0,
                height: 40.0,
                label: "T".into(),
            },
        );
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
    let arrow = if td {
        [
            (x2, y2),
            (
                x2 - 9.70142500145332 + 1.212678125181665,
                y2 + 2.42535625036333 + 4.85071250072666,
            ),
            (
                x2 - 9.70142500145332 - 1.212678125181665,
                y2 + 2.42535625036333 - 4.85071250072666,
            ),
        ]
    } else {
        [(x2, y2), (x2 - 5.0, y2 - 10.0), (x2 + 5.0, y2 - 10.0)]
    };
    let mut instructions = vec![
        rect((px, py, pw, 48.0), "#dbeafe", Some("#2563eb"), 2.0, 8.0),
        path(&[(x1, y1), (x2, y2)], "#2563eb", 2.0, false, true),
        path(&arrow, "#2563eb", 0.0, true, false),
        rect((ex, ey, ew, 30.0), "#dbeafe", Some("#2563eb"), 1.5, 6.0),
    ];
    let spine_paint = path(&[(sx, sy1), (sx, sy2)], "#2563eb", 3.0, false, false);
    if td {
        instructions.insert(0, spine_paint);
    } else {
        instructions.push(spine_paint);
    }
    if titled {
        instructions.push(glyph("T", 320.0, 70.4, 18.0, "rgb(17, 24, 39)"));
    }
    instructions.extend([
        glyph("P", px + pw / 2.0, pbase, 18.0, "rgb(15, 23, 42)"),
        glyph("E", ex + ew / 2.0, ebase, 14.0, "rgb(15, 23, 42)"),
    ]);
    let mut metadata = HashMap::from([
        ("timeline.fontFamily".into(), family.into()),
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
    (semantic, expected_layout, expected_scene)
}

#[test]
fn timeline_family_source_keeps_complete_product_contracts() {
    const FAMILY: &str = "\"trebuchet ms\", verdana, arial, sans-serif";
    for (header, td) in [
        ("timeline TD", true),
        ("timeline LR", false),
        ("timeline", false),
    ] {
        for titled in [false, true] {
            for (name,prefix,family) in [
                ("default","",FAMILY),
                ("custom","%%{init: {\"fontSize\":24,\"themeVariables\":{\"fontFamily\":\"Avenir Next\"}}}%%\n","Avenir Next"),
                ("trim","---\nthemeVariables:\n  fontFamily: '  Georgia  '\n---\n","Georgia"),
                ("invalid","---\nthemeVariables:\n  fontFamily: 'url(font)'\n---\n",FAMILY),
            ] {
                for merged in [false,true] {
                    let (mut semantic,mut expected_layout,expected_scene) = declared_family_product(td,titled,family);
                    // The retained authored custom prefix declares root fontSize24.
                    // Only expected semantic/layout config changes; Scene/query data stay exact.
                    if name == "custom" {
                        semantic.config.font_size = 24.0;
                        expected_layout.timeline_config.as_mut().unwrap().font_size = 24.0;
                    }
                    let expected_scene = super::declared_timeline_typography(expected_scene, &expected_layout);
                    let title = titled.then(|| "T".to_string());
                    let source = format!("{prefix}{header}\n{}P : E\n",if titled { "title T\n" } else { "" });
                    let expected_queries = if merged {
                        vec![FontQuery::named(family).with_weight(text_interfaces::FontWeight(600)).with_style(text_interfaces::FontStyle::Italic).with_stretch(text_interfaces::FontStretch::Condensed)]
                    } else {
                        vec![FontQuery::named(family).with_weight(text_interfaces::FontWeight(700)),FontQuery::named(family)]
                    };
                    let parsed = mermaid_parser::parse_timeline(&source).unwrap();
                    assert_eq!(parsed,(title.clone(),semantic),"whole IR {name}:{header}:{titled}:{merged}");
                    let actual_layout = diagram_layout_temporal::try_layout_temporal_diagram(&TemporalDiagram { kind:TemporalKind::Timeline,title,body:TemporalBody::Timeline(parsed.1) },640.0).unwrap();
                    assert_eq!(actual_layout,expected_layout,"whole layout {name}:{header}:{titled}:{merged}");
                    let resolver = Resolver(std::cell::RefCell::new(Vec::new()));
                    let defaults = make_opts(&FakeShaper,&FakeMetrics,&FakeResolver);
                    let mut label_font = FontSpec { family:"Caller Label".into(),..defaults.label_font };
                    let mut title_font = FontSpec { family:"Caller Title".into(),..defaults.title_font };
                    if merged {
                        for font in [&mut label_font,&mut title_font] { font.weight=600;font.italic=true;font.stretch=FontStretch::Condensed; }
                    }
                    let options = DiagramToPaintOptions { label_font,title_font,resolver:&resolver,shaper:&FakeShaper,metrics:&FakeMetrics,background:defaults.background,device_pixel_ratio:defaults.device_pixel_ratio };
                    assert_eq!(diagram_to_paint_temporal(&actual_layout,&options),expected_scene,"whole Scene {name}:{header}:{titled}:{merged}");
                    assert_eq!(*resolver.0.borrow(),expected_queries,"whole queries/cache {name}:{header}:{titled}:{merged}");
                    println!("timeline-family-product COMPLETE {name}:{header}:{titled}:{merged}");
                }
            }
        }
    }
}

#[test]
fn timeline_family_direct_domains_keep_full_scenes_and_queries() {
    const FAMILY: &str = "\"trebuchet ms\", verdana, arial, sans-serif";
    for (name, raw, effective) in [
        ("trim", "\nGeorgia\t", "Georgia"),
        ("empty", " \t ", FAMILY),
        ("nul", "A\0B", FAMILY),
        ("interior_tab", "A\tB", FAMILY),
        ("unicode", "東京", FAMILY),
        ("punctuation", "  A_1-B.C, D  ", "A_1-B.C, D"),
        ("commas", ",", ","),
        ("quote", "A\"Open", "A\"Open"),
        ("semicolon", "A;B", FAMILY),
    ] {
        let (mut semantic, mut expected_layout, expected_scene) =
            declared_family_product(true, true, effective);
        semantic.config.font_family = raw.into();
        expected_layout
            .timeline_config
            .as_mut()
            .unwrap()
            .font_family = raw.into();
        let expected_scene = super::declared_timeline_typography(expected_scene, &expected_layout);
        let expected_queries = vec![
            FontQuery::named(effective).with_weight(text_interfaces::FontWeight(700)),
            FontQuery::named(effective),
        ];
        let actual_layout = diagram_layout_temporal::try_layout_temporal_diagram(
            &TemporalDiagram {
                kind: TemporalKind::Timeline,
                title: Some("T".into()),
                body: TemporalBody::Timeline(semantic),
            },
            640.0,
        )
        .unwrap();
        assert_eq!(
            actual_layout, expected_layout,
            "whole raw typed layout {name}"
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
            diagram_to_paint_temporal(&actual_layout, &options),
            expected_scene,
            "whole direct Scene {name}"
        );
        assert_eq!(
            *resolver.0.borrow(),
            expected_queries,
            "whole direct queries {name}"
        );
        println!("timeline-family-direct COMPLETE {name}");
    }
}

#[test]
fn timeline_family_none_keeps_complete_generic_temporal_title() {
    let input = LayoutedTemporalDiagram {
        width: 200.0,
        height: 100.0,
        accessibility_title: None,
        accessibility_description: None,
        timeline_config: None,
        interactions: Vec::new(),
        items: vec![LayoutedTemporalItem::TemporalTitle {
            x: 10.0,
            y: 10.0,
            width: 180.0,
            height: 40.0,
            label: "T".into(),
        }],
    };
    let expected = PaintScene {
        width: 200.0,
        height: 100.0,
        background: "rgb(255,255,255)".into(),
        id: None,
        metadata: None,
        instructions: vec![glyph("T", 100.0, 30.4, 18.0, "rgb(17, 24, 39)")],
    };
    let expected_queries =
        vec![FontQuery::named("Caller Title").with_weight(text_interfaces::FontWeight(700))];
    let resolver = Resolver(std::cell::RefCell::new(Vec::new()));
    let defaults = make_opts(&FakeShaper, &FakeMetrics, &FakeResolver);
    let options = DiagramToPaintOptions {
        label_font: FontSpec {
            family: "Caller Label".into(),
            ..defaults.label_font
        },
        title_font: FontSpec {
            family: "Caller Title".into(),
            ..defaults.title_font
        },
        resolver: &resolver,
        shaper: &FakeShaper,
        metrics: &FakeMetrics,
        background: defaults.background,
        device_pixel_ratio: defaults.device_pixel_ratio,
    };
    assert_eq!(diagram_to_paint_temporal(&input, &options), expected);
    assert_eq!(*resolver.0.borrow(), expected_queries);
    println!("timeline-family-none COMPLETE generic-title");
}

#[test]
fn timeline_family_none_keeps_complete_gantt_axis_and_queries() {
    // Gantt's ordinary axis path consumes label_font, independently of the
    // generic title-font control above. Baseline42+800/1000*14 =53.2.
    let input = LayoutedTemporalDiagram {
        width: 200.0,
        height: 100.0,
        accessibility_title: None,
        accessibility_description: None,
        timeline_config: None,
        interactions: Vec::new(),
        items: vec![
            LayoutedTemporalItem::TemporalTitle {
                x: 10.0,
                y: 10.0,
                width: 180.0,
                height: 40.0,
                label: "T".into(),
            },
            LayoutedTemporalItem::TimeAxisTick {
                x: 50.0,
                y: 40.0,
                label: "L".into(),
                label_above: false,
            },
        ],
    };
    let expected = PaintScene {
        width: 200.0,
        height: 100.0,
        background: "rgb(255,255,255)".into(),
        id: None,
        metadata: None,
        instructions: vec![
            path(&[(50.0, 36.0), (50.0, 40.0)], "#374151", 1.0, false, false),
            glyph("T", 100.0, 30.4, 18.0, "rgb(17, 24, 39)"),
            glyph("L", 50.0, 53.2, 14.0, "rgb(107, 114, 128)"),
        ],
    };
    let expected_queries = vec![
        FontQuery::named("Caller Title").with_weight(text_interfaces::FontWeight(700)),
        FontQuery::named("Caller Label"),
    ];
    let resolver = Resolver(std::cell::RefCell::new(Vec::new()));
    let defaults = make_opts(&FakeShaper, &FakeMetrics, &FakeResolver);
    let options = DiagramToPaintOptions {
        label_font: FontSpec {
            family: "Caller Label".into(),
            ..defaults.label_font
        },
        title_font: FontSpec {
            family: "Caller Title".into(),
            ..defaults.title_font
        },
        resolver: &resolver,
        shaper: &FakeShaper,
        metrics: &FakeMetrics,
        background: defaults.background,
        device_pixel_ratio: defaults.device_pixel_ratio,
    };
    assert_eq!(diagram_to_paint_temporal(&input, &options), expected);
    assert_eq!(*resolver.0.borrow(), expected_queries);
    println!("timeline-family-none COMPLETE gantt-axis");
}
