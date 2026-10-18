// Complete old query vectors are proof inputs, never observed output. The
// declared fixtures contain Timeline items only; preserve first-use cache order.
fn declared_timeline_period_queries(
    old: Vec<FontQuery>,
    declared: &LayoutedTemporalDiagram,
    label: FontQuery,
    title: FontQuery,
) -> Vec<FontQuery> {
    assert!(declared.timeline_config.is_some());
    let mut old_expected = Vec::new();
    let mut new_expected = Vec::new();
    let mut text = |value: &str, old_title: bool, new_title: bool| {
        if value.is_empty() {
            return;
        }
        for (queries, is_title) in [
            (&mut old_expected, old_title),
            (&mut new_expected, new_title),
        ] {
            let query = if is_title { &title } else { &label };
            if !queries.contains(query) {
                queries.push(query.clone());
            }
        }
    };
    for item in &declared.items {
        match item {
            LayoutedTemporalItem::TemporalTitle { label, .. } => text(label, true, true),
            LayoutedTemporalItem::TimelineSection { label, .. }
            | LayoutedTemporalItem::TimelineEvent { label, .. } => text(label, false, false),
            LayoutedTemporalItem::TimelinePeriod { label, events, .. } => {
                text(label, true, false);
                if !events.is_empty() {
                    text(
                        &events
                            .iter()
                            .map(|e| format!("- {e}"))
                            .collect::<Vec<_>>()
                            .join("\n"),
                        false,
                        false,
                    );
                }
            }
            LayoutedTemporalItem::TimelineSpine { .. }
            | LayoutedTemporalItem::TimelineEventLink { .. } => {}
            _ => panic!("closed query migration admits declared Timeline items only"),
        }
    }
    assert_eq!(old, old_expected, "complete original declared query vector");
    new_expected
}

mod timeline_period_font_role_contract {
    use super::*;
    use std::cell::RefCell;

    struct Resolver(RefCell<Vec<FontQuery>>);
    impl FontResolver for Resolver {
        type Handle = FakeHandle;
        fn resolve(&self, q: &FontQuery) -> Result<FakeHandle, FontResolutionError> {
            self.0.borrow_mut().push(q.clone());
            Ok(FakeHandle)
        }
    }

    // Any backend operation is a test failure, including metrics without a
    // preceding resolver call. These probes do not mistake zero queries for
    // a complete zero-work claim.
    struct NoBackend;
    impl FontResolver for NoBackend {
        type Handle = FakeHandle;
        fn resolve(&self, _: &FontQuery) -> Result<FakeHandle, FontResolutionError> {
            panic!("unexpected resolve")
        }
    }
    impl FontMetrics for NoBackend {
        type Handle = FakeHandle;
        fn units_per_em(&self, _: &FakeHandle) -> u32 {
            panic!("unexpected metrics")
        }
        fn ascent(&self, _: &FakeHandle) -> i32 {
            panic!("unexpected metrics")
        }
        fn descent(&self, _: &FakeHandle) -> i32 {
            panic!("unexpected metrics")
        }
        fn line_gap(&self, _: &FakeHandle) -> i32 {
            panic!("unexpected metrics")
        }
        fn x_height(&self, _: &FakeHandle) -> Option<i32> {
            panic!("unexpected metrics")
        }
        fn cap_height(&self, _: &FakeHandle) -> Option<i32> {
            panic!("unexpected metrics")
        }
        fn family_name(&self, _: &FakeHandle) -> String {
            panic!("unexpected metrics")
        }
    }
    impl TextShaper for NoBackend {
        type Handle = FakeHandle;
        fn shape(
            &self,
            _: &str,
            _: &FakeHandle,
            _: f32,
            _: &ShapeOptions,
        ) -> Result<ShapedText, ShapingError> {
            panic!("unexpected shape")
        }
        fn font_ref(&self, _: &FakeHandle) -> String {
            panic!("unexpected font_ref")
        }
    }

    fn config(size: f64) -> diagram_ir::TimelineConfig {
        diagram_ir::TimelineConfig {
            font_size: size,
            font_family: "Role Family".into(),
            section_colors: vec!["#dbeafe".into()],
            section_label_colors: vec!["#0f172a".into()],
            section_inverse_colors: vec!["#2563eb".into()],
            ..Default::default()
        }
    }

    fn period(label: &str, embedded: bool) -> LayoutedTemporalItem {
        LayoutedTemporalItem::TimelinePeriod {
            x: 20.0,
            y: 20.0,
            width: 48.0,
            height: 80.0,
            label: label.into(),
            events: if embedded { vec!["E".into()] } else { vec![] },
            color_index: 0,
        }
    }

    fn input(size: Option<f64>, order: u8, full: bool) -> LayoutedTemporalDiagram {
        let title = LayoutedTemporalItem::TemporalTitle {
            x: 10.0,
            y: 140.0,
            width: 180.0,
            height: 40.0,
            label: "T".into(),
        };
        let mut items = vec![period(if full { "AA BB\nCC" } else { "P" }, full)];
        if full {
            items.push(LayoutedTemporalItem::TimelineEvent {
                x: 100.0,
                y: 20.0,
                width: 80.0,
                height: 30.0,
                label: "V".into(),
                color_index: 0,
            });
        }
        if order == 1 {
            items.insert(0, title);
        } else if order == 2 {
            items.push(title);
        }
        LayoutedTemporalDiagram {
            width: 300.0,
            height: 220.0,
            accessibility_title: Some("Role".into()),
            accessibility_description: Some("Complete".into()),
            timeline_config: size.map(config),
            interactions: vec![],
            items,
        }
    }

    fn metadata(size: Option<f64>) -> HashMap<String, String> {
        let mut values = HashMap::from([
            ("accessibility.title".into(), "Role".into()),
            ("accessibility.description".into(), "Complete".into()),
        ]);
        if let Some(size) = size {
            for (key, value) in [
                ("timeline.fontFamily", "Role Family"),
                ("timeline.padding", "50"),
                ("timeline.leftMargin", "50"),
                ("timeline.useMaxWidth", "false"),
                ("timeline.disableMulticolor", "false"),
                ("timeline.theme.cScale0", "#dbeafe"),
                ("timeline.theme.cScaleLabel0", "#0f172a"),
                ("timeline.theme.cScaleInv0", "#2563eb"),
            ] {
                values.insert(key.into(), value.into());
            }
            values.insert("timeline.fontSize".into(), size.to_string());
        }
        values
    }

    fn rect(x: f64, y: f64, width: f64, height: f64, event: bool) -> PaintInstruction {
        PaintInstruction::Rect(PaintRect {
            base: PaintBase::default(),
            x,
            y,
            width,
            height,
            fill: Some("#dbeafe".into()),
            stroke: Some("#2563eb".into()),
            stroke_width: Some(if event { 1.5 } else { 2.0 }),
            corner_radius: Some(if event { 6.0 } else { 8.0 }),
            stroke_dash: None,
            stroke_dash_offset: None,
        })
    }

    // Each call receives independently declared hard/soft lines and a literal
    // frame. Fake advances use f32 multiplication; metrics use f64 association.
    fn glyphs(
        lines: &[&str],
        x: f64,
        y: f64,
        width: f64,
        raw: f64,
        line_height: f64,
        fill: &str,
    ) -> Vec<PaintInstruction> {
        let mut baseline = y + 800.0 * raw / 1000.0;
        let step = ((1000.0 * raw / 1000.0).max(raw) * line_height) / 1.2;
        lines
            .iter()
            .map(|line| {
                let advance = raw as f32 / 2.0;
                let measured = (line.chars().count() as f32 * advance) as f64;
                assert!(measured <= width);
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
                        pen += advance as f64;
                        glyph
                    })
                    .collect();
                baseline += step;
                PaintInstruction::GlyphRun(PaintGlyphRun {
                    base: PaintBase::default(),
                    glyphs,
                    font_ref: "fake:test".into(),
                    font_size: (raw as f32) as f64,
                    fill: Some(fill.into()),
                })
            })
            .collect()
    }

    fn expected(size: Option<f64>, order: u8, full: bool) -> PaintScene {
        let label = size.unwrap_or(14.0);
        let title = size.map_or(18.0, |s| 2.0 * s);
        let period_size = size.unwrap_or(18.0);
        let period_height = if size.is_some() { 2.4 } else { 1.2 };
        let mut instructions = vec![rect(20.0, 20.0, 48.0, 80.0, false)];
        if full {
            instructions.push(rect(100.0, 20.0, 80.0, 30.0, true));
        }
        let title_glyphs = || glyphs(&["T"], 18.0, 146.0, 164.0, title, 1.2, "rgb(17, 24, 39)");
        if order == 1 {
            instructions.extend(title_glyphs());
        }
        let lines: &[&str] = if !full {
            &["P"]
        } else if size == Some(8.0000001) {
            &["AA BB", "CC"]
        } else {
            &["AA", "BB", "CC"]
        };
        instructions.extend(glyphs(
            lines,
            32.0,
            28.0,
            24.0,
            period_size,
            period_height,
            "rgb(15, 23, 42)",
        ));
        if full {
            instructions.extend(glyphs(
                &["- E"],
                32.0,
                52.0,
                24.0,
                label,
                2.4,
                "rgb(15, 23, 42)",
            ));
            instructions.extend(glyphs(
                &["V"],
                110.0,
                26.0,
                60.0,
                label,
                2.4,
                "rgb(15, 23, 42)",
            ));
        }
        if order == 2 {
            instructions.extend(title_glyphs());
        }
        PaintScene {
            width: 300.0,
            height: 220.0,
            background: "rgb(1,2,3)".into(),
            id: None,
            metadata: Some(metadata(size)),
            instructions,
        }
    }

    fn role_fonts(merged: bool) -> (FontSpec, FontSpec) {
        let defaults = make_opts(&FakeShaper, &FakeMetrics, &FakeResolver);
        let label = FontSpec {
            family: if merged {
                "Shared Caller"
            } else {
                "Label Caller"
            }
            .into(),
            weight: 500,
            italic: true,
            stretch: FontStretch::Condensed,
            line_height: 2.4,
            ..defaults.label_font
        };
        let title = FontSpec {
            family: if merged {
                "Shared Caller"
            } else {
                "Title Caller"
            }
            .into(),
            weight: if merged { 500 } else { 800 },
            italic: merged,
            stretch: if merged {
                FontStretch::Condensed
            } else {
                FontStretch::Expanded
            },
            line_height: 1.2,
            ..defaults.title_font
        };
        (label, title)
    }

    fn queries(size: Option<f64>, order: u8, merged: bool, full: bool) -> Vec<FontQuery> {
        let family = |title| {
            if size.is_some() {
                "Role Family"
            } else if merged {
                "Shared Caller"
            } else if title {
                "Title Caller"
            } else {
                "Label Caller"
            }
        };
        let label = FontQuery::named(family(false))
            .with_weight(text_interfaces::FontWeight(500))
            .with_style(text_interfaces::FontStyle::Italic)
            .with_stretch(text_interfaces::FontStretch::Condensed);
        let title = FontQuery::named(family(true))
            .with_weight(text_interfaces::FontWeight(if merged { 500 } else { 800 }))
            .with_style(if merged {
                text_interfaces::FontStyle::Italic
            } else {
                text_interfaces::FontStyle::Normal
            })
            .with_stretch(if merged {
                text_interfaces::FontStretch::Condensed
            } else {
                text_interfaces::FontStretch::Expanded
            });
        if merged {
            return vec![label];
        }
        if size.is_some() {
            match order {
                0 => vec![label],
                1 => vec![title, label],
                2 => vec![label, title],
                _ => unreachable!(),
            }
        } else if full {
            vec![title, label]
        } else {
            vec![title]
        }
    }

    #[test]
    fn configured_periods_use_label_role_and_none_retains_title_in_whole_scenes() {
        for (size_index, size) in [Some(16.0), Some(8.0000001), None].into_iter().enumerate() {
            for order in 0..3 {
                for merged in [false, true] {
                    let input = input(size, order, true);
                    let expected = expected(size, order, true);
                    let expected_queries = queries(size, order, merged, true);
                    let resolver = Resolver(RefCell::new(vec![]));
                    let (label_font, title_font) = role_fonts(merged);
                    let options = DiagramToPaintOptions {
                        label_font,
                        title_font,
                        background: Color {
                            r: 1,
                            g: 2,
                            b: 3,
                            a: 255,
                        },
                        device_pixel_ratio: 3.0,
                        resolver: &resolver,
                        metrics: &FakeMetrics,
                        shaper: &FakeShaper,
                    };
                    assert_eq!(
                        try_diagram_to_paint_temporal(&input, &options),
                        Ok(expected.clone())
                    );
                    assert_eq!(*resolver.0.borrow(), expected_queries);
                    assert_eq!(diagram_to_paint_temporal(&input, &options), expected);
                    assert_eq!(
                        *resolver.0.borrow(),
                        [expected_queries.clone(), expected_queries].concat()
                    );
                    println!("timeline-period-role-scene COMPLETE {size_index}:{order}:{merged}");
                }
            }
        }
    }

    #[test]
    fn period_only_resolves_label_identity_without_unused_title() {
        for (i, size) in [16.0, 8.0000001].into_iter().enumerate() {
            let input = input(Some(size), 0, false);
            let expected = expected(Some(size), 0, false);
            let resolver = Resolver(RefCell::new(vec![]));
            let (label_font, title_font) = role_fonts(false);
            let options = DiagramToPaintOptions {
                label_font,
                title_font,
                background: Color {
                    r: 1,
                    g: 2,
                    b: 3,
                    a: 255,
                },
                device_pixel_ratio: 3.0,
                resolver: &resolver,
                metrics: &FakeMetrics,
                shaper: &FakeShaper,
            };
            assert_eq!(
                try_diagram_to_paint_temporal(&input, &options),
                Ok(expected.clone())
            );
            assert_eq!(*resolver.0.borrow(), queries(Some(size), 0, false, false));
            assert_eq!(diagram_to_paint_temporal(&input, &options), expected);
            assert_eq!(
                *resolver.0.borrow(),
                [
                    queries(Some(size), 0, false, false),
                    queries(Some(size), 0, false, false)
                ]
                .concat()
            );
            println!("timeline-period-role-only COMPLETE {i}");
        }
    }

    #[test]
    fn period_only_and_empty_still_admit_both_requests_before_any_backend_work() {
        for (i, (size, error)) in [
            (0.0, TemporalPaintError::TimelineFontSize),
            (f32::MAX as f64, TemporalPaintError::TimelineTitleFontSize),
        ]
        .into_iter()
        .enumerate()
        {
            for populated in [false, true] {
                let mut input = input(Some(size), 0, false);
                if !populated {
                    input.items.clear();
                }
                let (label_font, title_font) = role_fonts(false);
                let options = DiagramToPaintOptions {
                    label_font,
                    title_font,
                    background: Color {
                        r: 1,
                        g: 2,
                        b: 3,
                        a: 255,
                    },
                    device_pixel_ratio: 3.0,
                    resolver: &NoBackend,
                    metrics: &NoBackend,
                    shaper: &NoBackend,
                };
                assert_eq!(try_diagram_to_paint_temporal(&input, &options), Err(error));
                println!("timeline-period-role-error COMPLETE {i}:{populated}");
            }
        }
    }

    #[test]
    #[should_panic(expected = "unsupported temporal typography")]
    fn period_only_compatibility_still_panics_for_unused_unrepresentable_title() {
        let (label_font, title_font) = role_fonts(false);
        let options = DiagramToPaintOptions {
            label_font,
            title_font,
            background: Color {
                r: 1,
                g: 2,
                b: 3,
                a: 255,
            },
            device_pixel_ratio: 3.0,
            resolver: &NoBackend,
            metrics: &NoBackend,
            shaper: &NoBackend,
        };
        diagram_to_paint_temporal(&input(Some(f32::MAX as f64), 0, false), &options);
    }
}
