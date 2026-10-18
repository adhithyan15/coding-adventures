// Whole typed-layout/Scene oracles, declared before first rendering. These
// stable public TimelinePeriod items also preserve legacy embedded-event Paint
// transport while the separate incoming event-card layout is reconciled.
#[test]
fn timeline_independent_palette_lengths_keep_whole_scenes_and_queries() {
    let fills = ["#110000", "#110001", "#110002", "#110003", "#110004", "#110005", "#110006", "#110007"];
    let labels = ["#220000", "#220001", "#220002", "#220003", "#220004", "#220005", "#220006", "#220007"];
    let strokes = ["#330000", "#330001", "#330002", "#330003", "#330004", "#330005", "#330006", "#330007"];
    let label_rgba = ["rgb(34, 0, 0)", "rgb(34, 0, 1)", "rgb(34, 0, 2)", "rgb(34, 0, 3)", "rgb(34, 0, 4)", "rgb(34, 0, 5)", "rgb(34, 0, 6)", "rgb(34, 0, 7)"];
    // Independent slot table for indices0,1,5,6,7,MAX. Both32/64-bit MAX have
    // residue3 modulo6 and residue7 modulo8; no production style helper is used.
    let slot = |length, column| match length {
        0 => None,
        1 => Some([0, 0, 0, 0, 0, 0][column]),
        2 => Some([0, 1, 1, 0, 1, 1][column]),
        6 => Some([0, 1, 5, 0, 1, 3][column]),
        8 => Some([0, 1, 5, 6, 7, 7][column]),
        _ => unreachable!(),
    };
    let glyph = |text: &str, x, y, size: f64, fill: &str| {
        PaintInstruction::GlyphRun(PaintGlyphRun {
            base: PaintBase::default(),
            glyphs: text.chars().enumerate().map(|(i, c)| GlyphPosition {
                glyph_id: c as u32, x: x + i as f64 * (size / 2.0), y,
            }).collect(),
            font_ref: "fake:test".into(), font_size: size, fill: Some(fill.into()),
        })
    };
    let profiles = [(0, 0, 0), (1, 1, 1), (2, 2, 2), (6, 6, 6), (8, 8, 8), (1, 2, 8), (8, 0, 2), (0, 8, 1)];
    for (profile, (nf, nl, ns)) in profiles.into_iter().enumerate() {
        for (column, index) in [0, 1, 5, 6, 7, usize::MAX].into_iter().enumerate() {
            let cfg = diagram_ir::TimelineConfig {
                font_family: "\"trebuchet ms\", verdana, arial, sans-serif".into(),
                padding: 50.0, left_margin: 50.0, use_max_width: true, disable_multicolor: false,
                section_colors: fills[..nf].iter().map(|s| s.to_string()).collect(),
                section_label_colors: labels[..nl].iter().map(|s| s.to_string()).collect(),
                section_inverse_colors: strokes[..ns].iter().map(|s| s.to_string()).collect(),
            };
            let fill = slot(nf, column).map(|i| fills[i]);
            let stroke = slot(ns, column).map(|i| strokes[i]);
            let rgba = slot(nl, column).map(|i| label_rgba[i]);
            let mut metadata = HashMap::from([
                ("accessibility.title".into(), "Palette history".into()),
                ("accessibility.description".into(), "Complete palette transport".into()),
                ("timeline.fontFamily".into(), "\"trebuchet ms\", verdana, arial, sans-serif".into()),
                ("timeline.padding".into(), "50".into()),
                ("timeline.leftMargin".into(), "50".into()),
                ("timeline.useMaxWidth".into(), "true".into()),
                ("timeline.disableMulticolor".into(), "false".into()),
            ]);
            for (family, values) in [("cScale", &fills[..nf]), ("cScaleLabel", &labels[..nl]), ("cScaleInv", &strokes[..ns])] {
                for (i, value) in values.iter().enumerate() {
                    metadata.insert(format!("timeline.theme.{family}{i}"), value.to_string());
                }
            }
            let oracle = PaintScene {
                width: 300.0, height: 200.0, background: "rgb(255,255,255)".into(), id: None,
                metadata: Some(metadata), instructions: vec![
                    PaintInstruction::Path(PaintPath {
                        base: PaintBase::default(),
                        commands: vec![PathCommand::MoveTo { x: 0.0, y: 0.0 }, PathCommand::LineTo { x: 0.0, y: 200.0 }],
                        fill: Some("none".into()), fill_rule: None,
                        stroke: Some(if ns == 0 { "#475569" } else { "#330000" }.into()),
                        stroke_width: Some(3.0), stroke_cap: Some(StrokeCap::Round), stroke_join: Some(StrokeJoin::Round),
                        stroke_dash: None, stroke_dash_offset: None,
                    }),
                    PaintInstruction::Rect(PaintRect {
                        base: PaintBase::default(), x: 20.0, y: 20.0, width: 136.0, height: 30.0,
                        fill: Some(fill.unwrap_or("#0f172a").into()), stroke: None, stroke_width: None,
                        corner_radius: Some(15.0), stroke_dash: None, stroke_dash_offset: None,
                    }),
                    PaintInstruction::Rect(PaintRect {
                        base: PaintBase::default(), x: 20.0, y: 70.0, width: 190.0, height: 66.0,
                        fill: Some(fill.unwrap_or("#dbeafe").into()), stroke: Some(stroke.unwrap_or("#2563eb").into()), stroke_width: Some(2.0),
                        corner_radius: Some(8.0), stroke_dash: None, stroke_dash_offset: None,
                    }),
                    glyph("S", 84.5, 37.2, 14.0, rgba.unwrap_or("rgb(255, 255, 255)")),
                    glyph("P", 110.5, 92.4, 18.0, rgba.unwrap_or("rgb(15, 23, 42)")),
                    glyph("- E", 104.5, 113.2, 14.0, rgba.unwrap_or("rgb(51, 65, 85)")),
                ],
            };
            let expected_queries = vec![
                FontQuery::named("\"trebuchet ms\", verdana, arial, sans-serif"),
                FontQuery::named("\"trebuchet ms\", verdana, arial, sans-serif").with_weight(text_interfaces::FontWeight(700)),
            ];
            let input = LayoutedTemporalDiagram {
                width: 300.0, height: 200.0, accessibility_title: Some("Palette history".into()),
                accessibility_description: Some("Complete palette transport".into()), timeline_config: Some(cfg), interactions: Vec::new(),
                items: vec![
                    LayoutedTemporalItem::TimelineSpine { x1: 0.0, y1: 0.0, x2: 0.0, y2: 200.0 },
                    LayoutedTemporalItem::TimelineSection { x: 20.0, y: 20.0, width: 136.0, height: 30.0, label: "S".into(), color_index: index },
                    LayoutedTemporalItem::TimelinePeriod { x: 20.0, y: 70.0, width: 190.0, height: 66.0, label: "P".into(), events: vec!["E".into()], color_index: index },
                ],
            };
            let resolver = VennDebugRecordingResolver(std::cell::RefCell::new(vec![]));
            let defaults = make_opts(&FakeShaper, &FakeMetrics, &FakeResolver);
            let options = DiagramToPaintOptions {
                background: defaults.background, device_pixel_ratio: defaults.device_pixel_ratio,
                label_font: FontSpec { family: "Georgia".into(), ..defaults.label_font },
                title_font: FontSpec { family: "Courier New".into(), ..defaults.title_font },
                shaper: &FakeShaper, metrics: &FakeMetrics, resolver: &resolver,
            };
            assert_eq!(diagram_to_paint_temporal(&input, &options), oracle, "profile={profile} column={column}");
            assert_eq!(*resolver.0.borrow(), expected_queries, "profile={profile} column={column}");
            println!("timeline-palette COMPLETE cell={profile}:{column}");
        }
    }
}
