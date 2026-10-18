// Independent source -> whole IR -> checked layout -> whole Scene/query cases.
// Width400/800 and height300/padding25 keep the old binary-exact two-circle
// fixture. Only normal text centers change: y163.125/170.625; two-text x
// centers are canvas/2 +/-10.6875. No returned layout constructs an oracle.
struct VennNormalCase {
    source: String,
    ir: diagram_ir::VennDiagram,
    layout: LayoutedVennDiagram,
    scene: PaintScene,
    family: String,
}

fn venn_normal_case(
    width: f64,
    family: &str,
    authored: bool,
    debug: bool,
    double: bool,
) -> VennNormalCase {
    let palette = [
        "#112233", "#445566", "#34d399", "#f472b6", "#a78bfa", "#22d3ee", "#fb7185", "#a3e635",
    ]
    .map(str::to_string)
    .to_vec();
    let source = format!("---\nconfig:\n  venn:\n    width: {width}\n    height: 300\n    padding: 25\n    useDebugLayout: {debug}\nthemeVariables:\n  fontFamily: {family}\n  venn1: '#112233'\n  venn2: '#445566'\n---\nvenn-beta\nset a[\"A\"]: 1\nset b[\"B\"]: 1\nunion a,b[\"U\"]: 0.5\ntext n[\"N\"]\n{}style a fill:#abcdef,stroke:#123456,stroke-width:3,fill-opacity:0.5{}\n{}",
        if double { "text m[\"M\"]\n" } else { "" },
        if authored { ",color:#ff0000" } else { "" },
        if authored { "style n color:#00ff00" } else { "" });
    let special = diagram_ir::VennStyle {
        fill: Some("#abcdef".into()),
        stroke: Some("#123456".into()),
        stroke_width: Some(3.0),
        text_color: authored.then(|| "#ff0000".into()),
        fill_opacity: Some(0.5),
    };
    let normal_style = diagram_ir::VennStyle {
        text_color: authored.then(|| "#00ff00".into()),
        ..Default::default()
    };
    let config = diagram_ir::VennConfig {
        width,
        height: 300.0,
        padding: 25.0,
        use_debug_layout: debug,
        font_family: family.into(),
        theme_colors: palette.clone(),
        ..Default::default()
    };
    let mut texts = vec![diagram_ir::VennText {
        sets: vec!["a".into(), "b".into()],
        id: "n".into(),
        label: Some("N".into()),
        style: normal_style.clone(),
    }];
    if double {
        texts.push(diagram_ir::VennText {
            sets: vec!["a".into(), "b".into()],
            id: "m".into(),
            label: Some("M".into()),
            style: Default::default(),
        });
    }
    let ir = diagram_ir::VennDiagram {
        title: None,
        config: config.clone(),
        regions: vec![
            diagram_ir::VennRegion {
                sets: vec!["a".into()],
                size: 1.0,
                label: Some("A".into()),
                style: special.clone(),
            },
            diagram_ir::VennRegion {
                sets: vec!["b".into()],
                size: 1.0,
                label: Some("B".into()),
                style: Default::default(),
            },
            diagram_ir::VennRegion {
                sets: vec!["a".into(), "b".into()],
                size: 0.5,
                label: Some("U".into()),
                style: Default::default(),
            },
        ],
        texts,
    };
    // Independently tabulated: available250/orbit40/radius62.5, inner22.5,
    // grid42.75 square. Small-count label offsets are13.125 and20.625.
    let (
        center,
        left,
        right,
        set_left,
        set_right,
        set_size,
        text_size,
        normal_y,
        cell_y,
        default_stroke,
    ) = match width {
        400.0 => (
            200.0, 240.0, 160.0, 276.25, 123.75, 12.0, 10.0, 163.125, 141.75, 1.25,
        ),
        800.0 => (
            400.0, 440.0, 360.0, 476.25, 323.75, 24.0, 20.0, 170.625, 149.25, 2.5,
        ),
        _ => panic!("only declared widths"),
    };
    let mut labels = vec![
        diagram_ir::LayoutedVennLabel {
            text: "A".into(),
            x: set_left,
            y: 150.0,
            font_size: set_size,
            style: special.clone(),
        },
        diagram_ir::LayoutedVennLabel {
            text: "B".into(),
            x: set_right,
            y: 150.0,
            font_size: set_size,
            style: Default::default(),
        },
        diagram_ir::LayoutedVennLabel {
            text: "U".into(),
            x: center,
            y: 150.0,
            font_size: set_size,
            style: Default::default(),
        },
        diagram_ir::LayoutedVennLabel {
            text: "N".into(),
            x: if double { center - 10.6875 } else { center },
            y: normal_y,
            font_size: text_size,
            style: normal_style,
        },
    ];
    if double {
        labels.push(diagram_ir::LayoutedVennLabel {
            text: "M".into(),
            x: center + 10.6875,
            y: normal_y,
            font_size: text_size,
            style: Default::default(),
        });
    }
    let cells = if double {
        vec![(center - 21.375, 21.375), (center, 21.375)]
    } else {
        vec![(center - 21.375, 42.75)]
    };
    let layout = LayoutedVennDiagram {
        width,
        height: 300.0,
        title: None,
        config,
        circles: vec![
            diagram_ir::LayoutedVennCircle {
                id: "a".into(),
                label: "A".into(),
                cx: left,
                cy: 150.0,
                radius: 62.5,
                style: special,
            },
            diagram_ir::LayoutedVennCircle {
                id: "b".into(),
                label: "B".into(),
                cx: right,
                cy: 150.0,
                radius: 62.5,
                style: Default::default(),
            },
        ],
        labels,
        debug_circles: if debug {
            vec![diagram_ir::LayoutedVennDebugCircle {
                cx: center,
                cy: 150.0,
                radius: 22.5,
            }]
        } else {
            vec![]
        },
        debug_cells: if debug {
            cells
                .iter()
                .map(|&(x, w)| diagram_ir::LayoutedVennDebugCell {
                    x,
                    y: cell_y,
                    width: w,
                    height: 42.75,
                })
                .collect()
        } else {
            vec![]
        },
    };
    let ellipse = |cx, fill: &str, stroke: &str, stroke_width| {
        PaintInstruction::Ellipse(PaintEllipse {
            base: PaintBase::default(),
            cx,
            cy: 150.0,
            rx: 62.5,
            ry: 62.5,
            fill: Some(fill.into()),
            stroke: Some(stroke.into()),
            stroke_width: Some(stroke_width),
            stroke_dash: None,
            stroke_dash_offset: None,
        })
    };
    let mut instructions = vec![
        ellipse(left, "rgba(171,205,239,0.5)", "#123456", 3.0),
        ellipse(right, "rgba(68,85,102,0.1)", "#445566", default_stroke),
    ];
    if debug {
        let scale = if width == 400.0 { 0.25 } else { 0.5 };
        instructions.push(PaintInstruction::Ellipse(PaintEllipse {
            base: PaintBase::default(),
            cx: center,
            cy: 150.0,
            rx: 22.5,
            ry: 22.5,
            fill: None,
            stroke: Some("#800080".into()),
            stroke_width: Some(1.5 * scale),
            stroke_dash: Some(vec![6.0 * scale, 4.0 * scale]),
            stroke_dash_offset: None,
        }));
        for &(x, w) in &cells {
            instructions.push(PaintInstruction::Rect(PaintRect {
                base: PaintBase::default(),
                x,
                y: cell_y,
                width: w,
                height: 42.75,
                fill: None,
                stroke: Some("#008080".into()),
                stroke_width: Some(scale),
                corner_radius: None,
                stroke_dash: Some(vec![4.0 * scale, 3.0 * scale]),
                stroke_dash_offset: None,
            }));
        }
    }
    // FakeShaper advances half an em and FakeMetrics has0.8em ascent. These
    // expected glyphs use the authored numeric table, not layout.labels.
    let glyph = |c: char, cx, y, size, fill: &str| {
        PaintInstruction::GlyphRun(PaintGlyphRun {
            base: PaintBase::default(),
            glyphs: vec![GlyphPosition {
                glyph_id: c as u32,
                x: cx - size / 4.0,
                y: y - 12.0 + size * 0.8,
            }],
            font_ref: "fake:test".into(),
            font_size: size,
            fill: Some(fill.into()),
        })
    };
    let ink = "rgb(15, 23, 42)";
    instructions.extend([
        glyph(
            'A',
            set_left,
            150.0,
            set_size,
            if authored { "rgb(255, 0, 0)" } else { ink },
        ),
        glyph('B', set_right, 150.0, set_size, ink),
        glyph('U', center, 150.0, set_size, ink),
        glyph(
            'N',
            if double { center - 10.6875 } else { center },
            normal_y,
            text_size,
            if authored { "rgb(0, 255, 0)" } else { ink },
        ),
    ]);
    if double {
        instructions.push(glyph('M', center + 10.6875, normal_y, text_size, ink));
    }
    let scene = PaintScene {
        width,
        height: 300.0,
        background: "rgb(255,255,255)".into(),
        instructions,
        id: None,
        metadata: Some(venn_debug_metadata(
            width, 300.0, 25.0, debug, family, &palette,
        )),
    };
    VennNormalCase {
        source,
        ir,
        layout,
        scene,
        family: family.into(),
    }
}

fn assert_venn_normal_scene(layout: &LayoutedVennDiagram, gold: &PaintScene, family: &str) {
    let resolver = VennDebugRecordingResolver(std::cell::RefCell::new(vec![]));
    let defaults = make_opts(&FakeShaper, &FakeMetrics, &FakeResolver);
    let options = DiagramToPaintOptions {
        background: defaults.background,
        device_pixel_ratio: defaults.device_pixel_ratio,
        label_font: FontSpec {
            family: "Ignored Caller".into(),
            weight: 600,
            italic: true,
            stretch: FontStretch::Condensed,
            ..defaults.label_font
        },
        title_font: defaults.title_font,
        shaper: &FakeShaper,
        metrics: &FakeMetrics,
        resolver: &resolver,
    };
    assert_eq!(
        &diagram_to_paint_venn(layout, &options),
        gold,
        "complete normal Scene {family}"
    );
    assert_eq!(
        *resolver.0.borrow(),
        vec![FontQuery::named(family)
            .with_weight(text_interfaces::FontWeight(600))
            .with_style(text_interfaces::FontStyle::Italic)
            .with_stretch(text_interfaces::FontStretch::Condensed)],
        "complete ordered cached font-query trace"
    );
}

#[test]
fn venn_normal_direct_grid_scenes_and_queries_are_exact_controls() {
    for width in [400.0, 800.0] {
        for family in ["Georgia", "CourierNew"] {
            for authored in [false, true] {
                for debug in [false, true] {
                    for double in [false, true] {
                        let case = venn_normal_case(width, family, authored, debug, double);
                        assert_venn_normal_scene(&case.layout, &case.scene, &case.family);
                        println!("VENN_NORMAL_DIRECT width={width} family={family} authored={authored} debug={debug} double={double}");
                    }
                }
            }
        }
    }
}

#[test]
fn venn_normal_source_ir_layout_scene_and_cached_queries() {
    for width in [400.0, 800.0] {
        for family in ["Georgia", "CourierNew"] {
            for authored in [false, true] {
                for debug in [false, true] {
                    for double in [false, true] {
                        let case = venn_normal_case(width, family, authored, debug, double);
                        let parsed = mermaid_parser::parse_venn(&case.source).unwrap();
                        assert_eq!(parsed, case.ir, "complete authored IR {}", case.source);
                        let layout = diagram_layout_geometric::try_layout_venn(&parsed).unwrap();
                        assert_eq!(
                            layout, case.layout,
                            "complete source normal grid {}",
                            case.source
                        );
                        assert_venn_normal_scene(&layout, &case.scene, &case.family);
                        println!("VENN_NORMAL_SOURCE width={width} family={family} authored={authored} debug={debug} double={double}");
                    }
                }
            }
        }
    }
}
