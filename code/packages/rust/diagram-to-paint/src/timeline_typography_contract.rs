// A closed migration of predeclared Scenes, never of returned production data.
// All retained cases use the existing fake backend and short unwrapped lines.
// Assert the complete old glyphs and both fit inequalities before declaring the
// new glyphs; a future wrapping case must supply its own explicit line oracle.
fn declared_timeline_typography(
    mut old: PaintScene,
    declared: &LayoutedTemporalDiagram,
) -> PaintScene {
    let size = declared.timeline_config.as_ref().unwrap().font_size;
    let original: Vec<_> = old
        .instructions
        .iter()
        .filter_map(|i| match i {
            PaintInstruction::GlyphRun(run) => Some(run.clone()),
            _ => None,
        })
        .collect();
    let mut cursor = 0;
    let mut replacement = Vec::new();
    let mut text = |value: &str, x: f64, y: f64, width: f64, title: bool| {
        let old_size = if title { 18.0 } else { 14.0 };
        let new_size = if title { size * 2.0 } else { size };
        let mut old_y = y + 800.0 * old_size / 1000.0;
        let mut new_y = y + 800.0 * new_size / 1000.0;
        if value.is_empty() {
            return;
        }
        for line in value.split('\n') {
            assert!(
                !line.is_empty(),
                "closed retained oracle has no empty hard line"
            );
            let template = &original[cursor];
            let run = |request: f64, baseline: f64| {
                let advance = (request as f32 / 2.0) as f64;
                let measured = (line.chars().count() as f32 * (request as f32 / 2.0)) as f64;
                assert!(
                    measured <= width,
                    "explicit new wrapping oracle required for {line:?}"
                );
                let start = x + (width - measured) / 2.0;
                let mut pen = 0.0;
                let glyphs = line
                    .chars()
                    .map(|c| {
                        let glyph = GlyphPosition {
                            glyph_id: c as u32,
                            x: start + pen,
                            y: baseline,
                        };
                        pen += advance;
                        glyph
                    })
                    .collect();
                PaintGlyphRun {
                    glyphs,
                    font_size: (request as f32) as f64,
                    ..template.clone()
                }
            };
            assert_eq!(
                *template,
                run(old_size, old_y),
                "retained complete old glyph {line:?}"
            );
            replacement.push(PaintInstruction::GlyphRun(run(new_size, new_y)));
            cursor += 1;
            // Preserve the exact metrics/line-height operation order (DPR1).
            old_y += ((1000.0 * old_size / 1000.0).max(old_size) * 1.2) / 1.2;
            new_y += ((1000.0 * new_size / 1000.0).max(new_size) * 1.2) / 1.2;
        }
    };
    for item in &declared.items {
        match item {
            LayoutedTemporalItem::TemporalTitle {
                x, y, width, label, ..
            } => text(label, x + 8.0, y + 6.0, width - 16.0, true),
            LayoutedTemporalItem::TimelineSection {
                x, y, width, label, ..
            } => text(label, x + 10.0, y + 6.0, width - 20.0, false),
            LayoutedTemporalItem::TimelinePeriod {
                x,
                y,
                width,
                label,
                events,
                ..
            } => {
                text(label, x + 12.0, y + 8.0, width - 24.0, true);
                if !events.is_empty() {
                    text(
                        &events
                            .iter()
                            .map(|e| format!("- {e}"))
                            .collect::<Vec<_>>()
                            .join("\n"),
                        x + 12.0,
                        y + 32.0,
                        width - 24.0,
                        false,
                    );
                }
            }
            LayoutedTemporalItem::TimelineEvent {
                x, y, width, label, ..
            } => text(label, x + 10.0, y + 6.0, width - 20.0, false),
            LayoutedTemporalItem::TimelineSpine { .. }
            | LayoutedTemporalItem::TimelineEventLink { .. } => {}
            _ => panic!("closed migration only admits declared Timeline items"),
        }
    }
    assert_eq!(
        cursor,
        original.len(),
        "all retained old GlyphRuns accounted for"
    );
    let first = old
        .instructions
        .iter()
        .position(|i| matches!(i, PaintInstruction::GlyphRun(_)))
        .unwrap_or(old.instructions.len());
    assert!(old.instructions[first..]
        .iter()
        .all(|i| matches!(i, PaintInstruction::GlyphRun(_))));
    old.instructions.truncate(first);
    old.instructions.extend(replacement);
    assert!(old
        .metadata
        .as_mut()
        .unwrap()
        .insert("timeline.fontSize".into(), size.to_string())
        .is_none());
    old
}

mod timeline_typography_contract {
    use super::*;
    use std::cell::{Cell, RefCell};

    struct Resolver(RefCell<Vec<FontQuery>>);
    impl FontResolver for Resolver {
        type Handle = FakeHandle;
        fn resolve(&self, q: &FontQuery) -> Result<FakeHandle, FontResolutionError> {
            self.0.borrow_mut().push(q.clone());
            Ok(FakeHandle)
        }
    }
    struct Metrics(Cell<usize>);
    impl FontMetrics for Metrics {
        type Handle = FakeHandle;
        fn units_per_em(&self, _: &FakeHandle) -> u32 {
            self.0.set(self.0.get() + 1);
            1000
        }
        fn ascent(&self, _: &FakeHandle) -> i32 {
            self.0.set(self.0.get() + 1);
            800
        }
        fn descent(&self, _: &FakeHandle) -> i32 {
            self.0.set(self.0.get() + 1);
            200
        }
        fn line_gap(&self, _: &FakeHandle) -> i32 {
            self.0.set(self.0.get() + 1);
            0
        }
        fn x_height(&self, _: &FakeHandle) -> Option<i32> {
            self.0.set(self.0.get() + 1);
            Some(500)
        }
        fn cap_height(&self, _: &FakeHandle) -> Option<i32> {
            self.0.set(self.0.get() + 1);
            Some(700)
        }
        fn family_name(&self, _: &FakeHandle) -> String {
            self.0.set(self.0.get() + 1);
            "Fake".into()
        }
    }
    struct Shaper(Cell<usize>);
    impl TextShaper for Shaper {
        type Handle = FakeHandle;
        fn shape(
            &self,
            text: &str,
            font: &FakeHandle,
            size: f32,
            opts: &ShapeOptions,
        ) -> Result<ShapedText, ShapingError> {
            self.0.set(self.0.get() + 1);
            FakeShaper.shape(text, font, size, opts)
        }
        fn font_ref(&self, font: &FakeHandle) -> String {
            self.0.set(self.0.get() + 1);
            FakeShaper.font_ref(font)
        }
    }
    fn empty(size: f64) -> LayoutedTemporalDiagram {
        LayoutedTemporalDiagram {
            width: 300.0,
            height: 200.0,
            accessibility_title: None,
            accessibility_description: None,
            timeline_config: Some(diagram_ir::TimelineConfig {
                font_size: size,
                ..Default::default()
            }),
            interactions: Vec::new(),
            items: Vec::new(),
        }
    }
    fn empty_scene(size: f64) -> PaintScene {
        let mut metadata = HashMap::from([
            (
                "timeline.fontFamily".into(),
                "\"trebuchet ms\", verdana, arial, sans-serif".into(),
            ),
            ("timeline.fontSize".into(), size.to_string()),
            ("timeline.padding".into(), "50".into()),
            ("timeline.leftMargin".into(), "50".into()),
            ("timeline.useMaxWidth".into(), "false".into()),
            ("timeline.disableMulticolor".into(), "false".into()),
        ]);
        for (family, colors) in [
            (
                "cScale",
                [
                    "#dbeafe", "#dcfce7", "#fef3c7", "#fae8ff", "#ffe4e6", "#cffafe",
                ],
            ),
            ("cScaleLabel", ["#0f172a"; 6]),
            (
                "cScaleInv",
                [
                    "#2563eb", "#16a34a", "#d97706", "#a21caf", "#e11d48", "#0891b2",
                ],
            ),
        ] {
            for (i, c) in colors.iter().enumerate() {
                metadata.insert(format!("timeline.theme.{family}{i}"), (*c).into());
            }
        }
        PaintScene {
            width: 300.0,
            height: 200.0,
            background: "rgb(255,255,255)".into(),
            id: None,
            metadata: Some(metadata),
            instructions: Vec::new(),
        }
    }

    #[test]
    fn checked_font_request_errors_precede_all_backend_work() {
        let min = f32::from_bits(1) as f64;
        for (i, (size, error)) in [
            (0.0, TemporalPaintError::TimelineFontSize),
            (-0.0, TemporalPaintError::TimelineFontSize),
            (-1.0, TemporalPaintError::TimelineFontSize),
            (f64::NAN, TemporalPaintError::TimelineFontSize),
            (f64::INFINITY, TemporalPaintError::TimelineFontSize),
            (f64::NEG_INFINITY, TemporalPaintError::TimelineFontSize),
            (f64::from_bits(1), TemporalPaintError::TimelineFontSize),
            (min / 2.0, TemporalPaintError::TimelineFontSize),
            (f64::MAX, TemporalPaintError::TimelineFontSize),
            (
                (f32::MAX as f64) * 2.0,
                TemporalPaintError::TimelineFontSize,
            ),
            (f32::MAX as f64, TemporalPaintError::TimelineTitleFontSize),
            // Raw label slightly above f32MAX still rounds to finite f32MAX.
            (
                f64::from_bits((f32::MAX as f64).to_bits() + 1),
                TemporalPaintError::TimelineTitleFontSize,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            for populated in [false, true] {
                let mut input = empty(size);
                if populated {
                    input.items.push(LayoutedTemporalItem::TemporalTitle {
                        x: 10.0,
                        y: 10.0,
                        width: 100.0,
                        height: 40.0,
                        label: "T".into(),
                    });
                }
                let resolver = Resolver(RefCell::new(Vec::new()));
                let metrics = Metrics(Cell::new(0));
                let shaper = Shaper(Cell::new(0));
                let defaults = make_opts(&FakeShaper, &FakeMetrics, &FakeResolver);
                let options = DiagramToPaintOptions {
                    label_font: defaults.label_font,
                    title_font: defaults.title_font,
                    background: defaults.background,
                    device_pixel_ratio: defaults.device_pixel_ratio,
                    resolver: &resolver,
                    metrics: &metrics,
                    shaper: &shaper,
                };
                assert_eq!(try_diagram_to_paint_temporal(&input, &options), Err(error));
                assert!(resolver.0.borrow().is_empty());
                assert_eq!(metrics.0.get(), 0);
                assert_eq!(shaper.0.get(), 0);
                println!("timeline-typography-error COMPLETE {i}:{populated}");
            }
        }
    }

    #[test]
    fn checked_empty_font_request_boundaries_keep_whole_scenes_without_backend_work() {
        let min = f32::from_bits(1) as f64;
        for (i, size) in [
            f64::from_bits((min / 2.0).to_bits() + 1),
            min,
            1.0,
            16.0,
            24.0,
            10.0000001,
            (f32::MAX as f64) / 2.0,
            f64::from_bits(((f32::MAX as f64) / 2.0).to_bits() + 1),
        ]
        .into_iter()
        .enumerate()
        {
            let input = empty(size);
            let expected = empty_scene(size);
            let resolver = Resolver(RefCell::new(Vec::new()));
            let metrics = Metrics(Cell::new(0));
            let shaper = Shaper(Cell::new(0));
            let defaults = make_opts(&FakeShaper, &FakeMetrics, &FakeResolver);
            let options = DiagramToPaintOptions {
                label_font: defaults.label_font,
                title_font: defaults.title_font,
                background: defaults.background,
                device_pixel_ratio: 3.0,
                resolver: &resolver,
                metrics: &metrics,
                shaper: &shaper,
            };
            assert_eq!(
                try_diagram_to_paint_temporal(&input, &options),
                Ok(expected.clone())
            );
            assert_eq!(diagram_to_paint_temporal(&input, &options), expected);
            assert!(resolver.0.borrow().is_empty());
            assert_eq!(metrics.0.get(), 0);
            assert_eq!(shaper.0.get(), 0);
            println!("timeline-typography-empty COMPLETE {i}");
        }
    }

    #[test]
    fn whole_wrapped_scene_keeps_f32_glyphs_f64_metrics_and_caller_attributes() {
        for (row, size, em, first_x) in [(0, 8.0, 16.0, 22.0), (1, 10.0000001, 20.0, 20.0)] {
            for line_height in [1.2, 2.4] {
                let mut input = empty(size);
                input.items.push(LayoutedTemporalItem::TemporalTitle {
                    x: 10.0,
                    y: 10.0,
                    width: 40.0,
                    height: 40.0,
                    label: "AA BB\nCC".into(),
                });
                let mut expected = empty_scene(size);
                // Fixed frame(18,16,24,28): AA BB exceeds24 at both sizes;
                // the declared lines are AA/BB plus the authored hard line CC.
                // Glyph advance is rounded f32; baseline metrics retain f64.
                let raw = size * 2.0;
                let first_y = 16.0 + 800.0 * raw / 1000.0;
                let step = ((1000.0 * raw / 1000.0).max(raw) * line_height) / 1.2;
                let ys = [first_y, first_y + step, (first_y + step) + step];
                expected.instructions = [('A', ys[0]), ('B', ys[1]), ('C', ys[2])]
                    .into_iter()
                    .map(|(c, y)| {
                        PaintInstruction::GlyphRun(PaintGlyphRun {
                            base: PaintBase::default(),
                            glyphs: vec![
                                GlyphPosition {
                                    glyph_id: c as u32,
                                    x: first_x,
                                    y,
                                },
                                GlyphPosition {
                                    glyph_id: c as u32,
                                    x: first_x + em / 2.0,
                                    y,
                                },
                            ],
                            font_ref: "fake:test".into(),
                            font_size: em,
                            fill: Some("rgb(17, 24, 39)".into()),
                        })
                    })
                    .collect();
                let query = FontQuery::named("\"trebuchet ms\", verdana, arial, sans-serif")
                    .with_weight(text_interfaces::FontWeight(600))
                    .with_style(text_interfaces::FontStyle::Italic)
                    .with_stretch(text_interfaces::FontStretch::Condensed);
                let resolver = Resolver(RefCell::new(Vec::new()));
                let defaults = make_opts(&FakeShaper, &FakeMetrics, &FakeResolver);
                let options = DiagramToPaintOptions {
                    label_font: FontSpec {
                        size: -1.0,
                        ..defaults.label_font
                    },
                    title_font: FontSpec {
                        size: f64::INFINITY,
                        weight: 600,
                        italic: true,
                        stretch: FontStretch::Condensed,
                        line_height,
                        ..defaults.title_font
                    },
                    background: defaults.background,
                    device_pixel_ratio: 3.0,
                    resolver: &resolver,
                    metrics: &FakeMetrics,
                    shaper: &FakeShaper,
                };
                assert_eq!(
                    try_diagram_to_paint_temporal(&input, &options),
                    Ok(expected.clone())
                );
                assert_eq!(*resolver.0.borrow(), vec![query.clone()]);
                assert_eq!(diagram_to_paint_temporal(&input, &options), expected);
                assert_eq!(*resolver.0.borrow(), vec![query.clone(), query]);
                println!("timeline-typography-wrapped COMPLETE {row}:{line_height}");
            }
        }
    }

    #[test]
    fn mixed_some_config_keeps_ordinary_axis_and_journey_font_roles() {
        let mut input = empty(16.0);
        input.items = vec![
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
            LayoutedTemporalItem::JourneyTitle {
                x: 10.0,
                y: 70.0,
                width: 180.0,
                height: 40.0,
                label: "J".into(),
                font_size: None,
                font_family: None,
                color: None,
            },
            LayoutedTemporalItem::TimelineEvent {
                x: 20.0,
                y: 120.0,
                width: 100.0,
                height: 30.0,
                label: "E".into(),
                color_index: 0,
            },
        ];
        let glyph = |c: char, x, y, font_size, fill: &str| {
            PaintInstruction::GlyphRun(PaintGlyphRun {
                base: PaintBase::default(),
                glyphs: vec![GlyphPosition {
                    glyph_id: c as u32,
                    x,
                    y,
                }],
                font_ref: "fake:test".into(),
                font_size,
                fill: Some(fill.into()),
            })
        };
        let mut expected = empty_scene(16.0);
        expected.instructions = vec![
            PaintInstruction::Path(PaintPath {
                base: PaintBase::default(),
                commands: vec![
                    PathCommand::MoveTo { x: 50.0, y: 36.0 },
                    PathCommand::LineTo { x: 50.0, y: 40.0 },
                ],
                fill: Some("none".into()),
                fill_rule: None,
                stroke: Some("#374151".into()),
                stroke_width: Some(1.0),
                stroke_cap: Some(StrokeCap::Round),
                stroke_join: Some(StrokeJoin::Round),
                stroke_dash: None,
                stroke_dash_offset: None,
            }),
            PaintInstruction::Rect(PaintRect {
                base: PaintBase::default(),
                x: 20.0,
                y: 120.0,
                width: 100.0,
                height: 30.0,
                fill: Some("#dbeafe".into()),
                stroke: Some("#2563eb".into()),
                stroke_width: Some(1.5),
                corner_radius: Some(6.0),
                stroke_dash: None,
                stroke_dash_offset: None,
            }),
            glyph('T', 92.0, 41.6, 32.0, "rgb(17, 24, 39)"),
            glyph('L', 46.5, 53.2, 14.0, "rgb(107, 114, 128)"),
            glyph('J', 95.5, 90.4, 18.0, "rgb(17, 24, 39)"),
            glyph('E', 66.0, 138.8, 16.0, "rgb(15, 23, 42)"),
        ];
        let queries = vec![
            FontQuery::named("\"trebuchet ms\", verdana, arial, sans-serif")
                .with_weight(text_interfaces::FontWeight(700)),
            FontQuery::named("Caller Label"),
            FontQuery::named("Caller Title").with_weight(text_interfaces::FontWeight(700)),
            FontQuery::named("\"trebuchet ms\", verdana, arial, sans-serif"),
        ];
        let resolver = Resolver(RefCell::new(Vec::new()));
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
            background: defaults.background,
            device_pixel_ratio: 3.0,
            resolver: &resolver,
            metrics: &FakeMetrics,
            shaper: &FakeShaper,
        };
        assert_eq!(
            try_diagram_to_paint_temporal(&input, &options),
            Ok(expected.clone())
        );
        assert_eq!(*resolver.0.borrow(), queries);
        assert_eq!(diagram_to_paint_temporal(&input, &options), expected);
        assert_eq!(*resolver.0.borrow(), queries.repeat(2));
        println!("timeline-typography-mixed COMPLETE axis-journey");
    }

    #[test]
    #[should_panic(expected = "unsupported temporal typography")]
    fn compatibility_entry_explicitly_panics_for_unrepresentable_requests() {
        let options = make_opts(&FakeShaper, &FakeMetrics, &FakeResolver);
        diagram_to_paint_temporal(&empty(f64::MAX), &options);
    }
}
