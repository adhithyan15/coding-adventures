// Authored source -> whole semantic IR -> whole checked layout -> whole Scene.
// Golden geometry is declared independently before either production call.
mod timeline_event_scene_contract {
    use super::*;
    use diagram_ir::{
        TemporalBody, TemporalDiagram, TemporalKind, TimelineConfig, TimelineDiagram,
        TimelineDirection, TimelinePeriod, TimelineSection,
    };

    struct Resolver(std::cell::RefCell<Vec<FontQuery>>);
    impl FontResolver for Resolver {
        type Handle = FakeHandle;
        fn resolve(&self, query: &FontQuery) -> Result<FakeHandle, FontResolutionError> {
            self.0.borrow_mut().push(query.clone());
            Ok(FakeHandle)
        }
    }

    include!("timeline_legacy_scene_data.rs");
    include!("timeline_default_direction_contract.rs");
    include!("timeline_size_layout_product_contract.rs");
    include!("timeline_period_height_product_contract.rs");
    include!("timeline_family_scene_contract.rs");

    #[test]
    fn retained_embedded_event_layouts_keep_complete_legacy_scenes() {
        for (name, declared_layout, declared_scene, declared_queries) in
            declared_legacy_timeline_scene_cases()
        {
            // Retain the entire declared legacy Scene/query vectors above.
            // Retain original metadata/family transport; typography and period-role
            // migrations separately prove complete old glyphs and query vectors.
            let mut declared_scene = declared_scene;
            declared_scene.metadata.as_mut().unwrap().insert("timeline.fontFamily".into(), "\"trebuchet ms\", verdana, arial, sans-serif".into());
            let declared_queries: Vec<_> = declared_queries.into_iter().map(|mut query| {
                query.family_names = vec!["\"trebuchet ms\", verdana, arial, sans-serif".into()];
                query
            }).collect();
            let declared_scene = super::declared_timeline_typography(declared_scene, &declared_layout);
            assert_eq!(declared_queries.len(), 2);
            assert_ne!(declared_queries[0], declared_queries[1]);
            let declared_queries = super::declared_timeline_period_queries(declared_queries, &declared_layout, FontQuery::named("\"trebuchet ms\", verdana, arial, sans-serif"), FontQuery::named("\"trebuchet ms\", verdana, arial, sans-serif").with_weight(text_interfaces::FontWeight(700)));
            let defaults = make_opts(&FakeShaper, &FakeMetrics, &FakeResolver);
            let resolver = Resolver(std::cell::RefCell::new(Vec::new()));
            let options = DiagramToPaintOptions {
                label_font: defaults.label_font,
                title_font: defaults.title_font,
                background: defaults.background,
                device_pixel_ratio: defaults.device_pixel_ratio,
                resolver: &resolver,
                shaper: &FakeShaper,
                metrics: &FakeMetrics,
            };
            assert_eq!(
                diagram_to_paint_temporal(&declared_layout, &options),
                declared_scene,
                "whole legacy Scene {name}"
            );
            assert_eq!(
                *resolver.0.borrow(),
                declared_queries,
                "ordered legacy queries {name}"
            );
            println!("timeline-event-legacy-scene COMPLETE {name}");
        }
    }

    fn path(
        points: &[(f64, f64)],
        stroke: &str,
        width: f64,
        arrow: bool,
        dashed: bool,
    ) -> PaintInstruction {
        let mut commands = vec![PathCommand::MoveTo {
            x: points[0].0,
            y: points[0].1,
        }];
        commands.extend(
            points[1..]
                .iter()
                .map(|&(x, y)| PathCommand::LineTo { x, y }),
        );
        if arrow {
            commands.push(PathCommand::Close);
        }
        PaintInstruction::Path(PaintPath {
            base: PaintBase::default(),
            commands,
            fill: (!dashed).then_some(if arrow { stroke } else { "none" }.into()),
            fill_rule: None,
            stroke: (!arrow).then(|| stroke.into()),
            stroke_width: (!arrow).then_some(width),
            stroke_cap: (!arrow).then_some(StrokeCap::Round),
            stroke_join: (!arrow).then_some(StrokeJoin::Round),
            stroke_dash: dashed.then(|| vec![5.0, 5.0]),
            stroke_dash_offset: None,
        })
    }

    fn rect(
        frame: (f64, f64, f64, f64),
        fill: &str,
        stroke: Option<&str>,
        sw: f64,
        radius: f64,
    ) -> PaintInstruction {
        let (x, y, width, height) = frame;
        PaintInstruction::Rect(PaintRect {
            base: PaintBase::default(),
            x,
            y,
            width,
            height,
            fill: Some(fill.into()),
            stroke: stroke.map(Into::into),
            stroke_width: stroke.map(|_| sw),
            corner_radius: Some(radius),
            stroke_dash: None,
            stroke_dash_offset: None,
        })
    }

    fn glyph(label: &str, center: f64, baseline: f64, size: f64, color: &str) -> PaintInstruction {
        let start = center - label.chars().count() as f64 * size / 4.0;
        PaintInstruction::GlyphRun(PaintGlyphRun {
            base: PaintBase::default(),
            glyphs: label
                .chars()
                .enumerate()
                .map(|(i, c)| GlyphPosition {
                    glyph_id: c as u32,
                    x: start + i as f64 * size / 2.0,
                    y: baseline,
                })
                .collect(),
            font_ref: "fake:test".into(),
            font_size: size,
            fill: Some(color.into()),
        })
    }

    #[test]
    fn complete_source_separate_events_layout_scene_and_queries() {
        for direction in [TimelineDirection::TopDown, TimelineDirection::LeftRight] {
            for custom in [false, true] {
                for title in [false, true] {
                    for named in [false, true] {
                        for single in [false, true] {
                            let (padding, margin, origin) = if custom {
                                (72.0, 80.0, 152.0)
                            } else {
                                (50.0, 50.0, 100.0)
                            };
                            let fills = if custom {
                                [
                                    "#110000", "#110001", "#110002", "#110003", "#110004",
                                    "#110005",
                                ]
                            } else {
                                [
                                    "#dbeafe", "#dcfce7", "#fef3c7", "#fae8ff", "#ffe4e6",
                                    "#cffafe",
                                ]
                            };
                            let labels = if custom {
                                [
                                    "#220000", "#220001", "#220002", "#220003", "#220004",
                                    "#220005",
                                ]
                            } else {
                                ["#0f172a"; 6]
                            };
                            let label_rgba = if custom {
                                [
                                    "rgb(34, 0, 0)",
                                    "rgb(34, 0, 1)",
                                    "rgb(34, 0, 2)",
                                    "rgb(34, 0, 3)",
                                    "rgb(34, 0, 4)",
                                    "rgb(34, 0, 5)",
                                ]
                            } else {
                                ["rgb(15, 23, 42)"; 6]
                            };
                            let strokes = if custom {
                                [
                                    "#330000", "#330001", "#330002", "#330003", "#330004",
                                    "#330005",
                                ]
                            } else {
                                [
                                    "#2563eb", "#16a34a", "#d97706", "#a21caf", "#e11d48",
                                    "#0891b2",
                                ]
                            };
                            let cfg = TimelineConfig {
                                font_size: 16.0,
                                font_family: "\"trebuchet ms\", verdana, arial, sans-serif".into(),
                                padding,
                                left_margin: margin,
                                use_max_width: custom,
                                disable_multicolor: single,
                                section_colors: fills.map(str::to_string).to_vec(),
                                section_label_colors: labels.map(str::to_string).to_vec(),
                                section_inverse_colors: strokes.map(str::to_string).to_vec(),
                            };
                            let mut source = format!("%%{{init: {{\"timeline\":{{\"padding\":{padding},\"leftMargin\":{margin},\"useMaxWidth\":{custom},\"disableMulticolor\":{single}}}");
                            if custom {
                                source.push_str(",\"themeVariables\":{");
                                for (family, colors) in [
                                    ("cScale", fills),
                                    ("cScaleLabel", labels),
                                    ("cScaleInv", strokes),
                                ] {
                                    for (i, color) in colors.iter().enumerate() {
                                        if !source.ends_with('{') {
                                            source.push(',');
                                        }
                                        source.push_str(&format!("\"{family}{i}\":\"{color}\""));
                                    }
                                }
                                source.push('}');
                            }
                            source.push_str(if direction == TimelineDirection::TopDown {
                                "}}%%\ntimeline TD\n"
                            } else {
                                "}}%%\ntimeline LR\n"
                            });
                            source.push_str(
                                "accTitle: Accessible history\naccDescr: Ordered milestones\n",
                            );
                            if title {
                                source.push_str("title T\n");
                            }
                            if named {
                                source.push_str("section S\n");
                            }
                            for n in 0..7 {
                                if named && n == 3 {
                                    source.push_str("section Empty\nsection S2\n");
                                }
                                source.push_str(&format!("P{n} : E{n}\n"));
                            }
                            let periods = |start, end| {
                                (start..end)
                                    .map(|n| TimelinePeriod {
                                        label: format!("P{n}"),
                                        events: vec![format!("E{n}")],
                                    })
                                    .collect()
                            };
                            let semantic = TimelineDiagram {
                                accessibility_title: Some("Accessible history".into()),
                                accessibility_description: Some("Ordered milestones".into()),
                                config: cfg.clone(),
                                direction,
                                sections: if named {
                                    vec![
                                        TimelineSection {
                                            label: Some("S".into()),
                                            periods: periods(0, 3),
                                        },
                                        TimelineSection {
                                            label: Some("Empty".into()),
                                            periods: Vec::new(),
                                        },
                                        TimelineSection {
                                            label: Some("S2".into()),
                                            periods: periods(3, 7),
                                        },
                                    ]
                                } else {
                                    vec![TimelineSection {
                                        label: None,
                                        periods: periods(0, 7),
                                    }]
                                },
                            };
                            let expected_title = title.then(|| "T".to_string());
                            let base = padding + if title { 52.0 } else { 0.0 };
                            let td = direction == TimelineDirection::TopDown;
                            let width = if td {
                                1000.0
                            } else {
                                (origin + if named { 938.0 } else { 1580.0 } + padding).max(1000.0)
                            };
                            let height = base
                                + padding
                                + if td {
                                    if named {
                                        606.0
                                    } else {
                                        472.0
                                    }
                                } else if named {
                                    526.0
                                } else {
                                    196.0
                                };
                            let mut items = Vec::new();
                            let mut geometry = Vec::new();
                            let mut text = Vec::new();
                            if title {
                                items.push(LayoutedTemporalItem::TemporalTitle {
                                    x: padding,
                                    y: padding,
                                    width: 1000.0 - 2.0 * padding,
                                    height: 40.0,
                                    label: "T".into(),
                                });
                                text.push(glyph(
                                    "T",
                                    500.0,
                                    padding + 20.4,
                                    18.0,
                                    "rgb(17, 24, 39)",
                                ));
                            }
                            if td {
                                let (x, y1, y2) = (
                                    origin + 106.0,
                                    base + 16.0,
                                    base + if named { 570.0 } else { 436.0 },
                                );
                                items.push(LayoutedTemporalItem::TimelineSpine {
                                    x1: x,
                                    y1,
                                    x2: x,
                                    y2,
                                });
                                geometry.push(path(
                                    &[(x, y1), (x, y2)],
                                    strokes[0],
                                    3.0,
                                    false,
                                    false,
                                ));
                            }
                            for section in 0..if named { 3 } else { 1 } {
                                let (start, end, offset, label) = if !named {
                                    (0, 7, 0.0, "")
                                } else if section == 0 {
                                    (0, 3, 0.0, "S")
                                } else if section == 1 {
                                    (3, 3, if td { 246.0 } else { 196.0 }, "Empty")
                                } else {
                                    (3, 7, if td { 294.0 } else { 330.0 }, "S2")
                                };
                                let section_y = base + offset;
                                if named {
                                    let slot = start % 6;
                                    items.push(LayoutedTemporalItem::TimelineSection {
                                        x: origin,
                                        y: section_y,
                                        width: 136.0,
                                        height: 30.0,
                                        label: label.into(),
                                        color_index: start,
                                    });
                                    geometry.push(rect(
                                        (origin, section_y, 136.0, 30.0),
                                        fills[slot],
                                        None,
                                        0.0,
                                        15.0,
                                    ));
                                    text.push(glyph(
                                        label,
                                        origin + 68.0,
                                        section_y + 17.2,
                                        14.0,
                                        label_rgba[slot],
                                    ));
                                }
                                for n in start..end {
                                    let ordinal = if single && !named { 0 } else { n };
                                    let slot = ordinal % 6;
                                    let (x, y, w, ex, ey, ew, x1, y1, x2, y2) = if td {
                                        let y = section_y
                                            + if named { 38.0 } else { 0.0 }
                                            + (n - start) as f64 * 66.0;
                                        (
                                            origin + 134.0,
                                            y,
                                            180.0,
                                            origin + 350.0,
                                            y,
                                            1000.0 - origin - 350.0 - padding,
                                            origin + 314.0,
                                            y + 24.0,
                                            origin + 350.0,
                                            y + 15.0,
                                        )
                                    } else {
                                        let x = origin + 106.0 + (n - start) as f64 * 214.0;
                                        (
                                            x,
                                            section_y + 68.0,
                                            190.0,
                                            x,
                                            section_y + 134.0,
                                            190.0,
                                            x + 95.0,
                                            section_y + 116.0,
                                            x + 95.0,
                                            section_y + 134.0,
                                        )
                                    };
                                    items.push(LayoutedTemporalItem::TimelinePeriod {
                                        x,
                                        y,
                                        width: w,
                                        height: 48.0,
                                        label: format!("P{n}"),
                                        events: Vec::new(),
                                        color_index: ordinal,
                                    });
                                    items.push(LayoutedTemporalItem::TimelineEventLink {
                                        x1,
                                        y1,
                                        x2,
                                        y2,
                                        color_index: ordinal,
                                    });
                                    items.push(LayoutedTemporalItem::TimelineEvent {
                                        x: ex,
                                        y: ey,
                                        width: ew,
                                        height: 30.0,
                                        label: format!("E{n}"),
                                        color_index: ordinal,
                                    });
                                    geometry.push(rect(
                                        (x, y, w, 48.0),
                                        fills[slot],
                                        Some(strokes[slot]),
                                        2.0,
                                        8.0,
                                    ));
                                    geometry.push(path(
                                        &[(x1, y1), (x2, y2)],
                                        strokes[slot],
                                        2.0,
                                        false,
                                        true,
                                    ));
                                    // Fixed36/-9 and0/18 fixtures: independently evaluated
                                    // unit-vector products, with unchanged association.
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
                                    geometry.push(path(&arrow, strokes[slot], 0.0, true, false));
                                    geometry.push(rect(
                                        (ex, ey, ew, 30.0),
                                        fills[slot],
                                        Some(strokes[slot]),
                                        1.5,
                                        6.0,
                                    ));
                                    text.push(glyph(
                                        &format!("P{n}"),
                                        x + w / 2.0,
                                        y + 22.4,
                                        18.0,
                                        label_rgba[slot],
                                    ));
                                    text.push(glyph(
                                        &format!("E{n}"),
                                        ex + ew / 2.0,
                                        ey + 17.2,
                                        14.0,
                                        label_rgba[slot],
                                    ));
                                }
                                if !td && start != end {
                                    let x1 = origin + 201.0;
                                    let x2 = x1 + (end - start - 1) as f64 * 214.0;
                                    let y = section_y + 56.0;
                                    items.push(LayoutedTemporalItem::TimelineSpine {
                                        x1,
                                        y1: y,
                                        x2,
                                        y2: y,
                                    });
                                    geometry.push(path(
                                        &[(x1, y), (x2, y)],
                                        strokes[0],
                                        3.0,
                                        false,
                                        false,
                                    ));
                                }
                            }
                            let expected_layout = LayoutedTemporalDiagram {
                                width,
                                height,
                                accessibility_title: semantic.accessibility_title.clone(),
                                accessibility_description: semantic
                                    .accessibility_description
                                    .clone(),
                                timeline_config: Some(cfg),
                                interactions: Vec::new(),
                                items,
                            };
                            let mut metadata = HashMap::from([
                                ("accessibility.title".into(), "Accessible history".into()),
                                (
                                    "accessibility.description".into(),
                                    "Ordered milestones".into(),
                                ),
                                ("timeline.fontFamily".into(), "\"trebuchet ms\", verdana, arial, sans-serif".into()),
                                ("timeline.padding".into(), padding.to_string()),
                                ("timeline.leftMargin".into(), margin.to_string()),
                                ("timeline.useMaxWidth".into(), custom.to_string()),
                                ("timeline.disableMulticolor".into(), single.to_string()),
                            ]);
                            for (family, colors) in [
                                ("cScale", fills),
                                ("cScaleLabel", labels),
                                ("cScaleInv", strokes),
                            ] {
                                for (i, color) in colors.iter().enumerate() {
                                    metadata.insert(
                                        format!("timeline.theme.{family}{i}"),
                                        color.to_string(),
                                    );
                                }
                            }
                            geometry.extend(text);
                            let expected_scene = PaintScene {
                                width,
                                height,
                                background: "rgb(255,255,255)".into(),
                                id: None,
                                metadata: Some(metadata),
                                instructions: geometry,
                            };
                            let expected_scene = super::declared_timeline_typography(expected_scene, &expected_layout);
                            let title_query = FontQuery::named("\"trebuchet ms\", verdana, arial, sans-serif")
                                .with_weight(text_interfaces::FontWeight(700));
                            let label_query = FontQuery::named("\"trebuchet ms\", verdana, arial, sans-serif");
                            let expected_queries = if title || !named {
                                vec![title_query, label_query]
                            } else {
                                vec![label_query, title_query]
                            };
                            let expected_queries = super::declared_timeline_period_queries(expected_queries, &expected_layout, FontQuery::named("\"trebuchet ms\", verdana, arial, sans-serif"), FontQuery::named("\"trebuchet ms\", verdana, arial, sans-serif").with_weight(text_interfaces::FontWeight(700)));

                            let parsed = mermaid_parser::parse_timeline(&source).unwrap();
                            assert_eq!(
                                parsed,
                                (expected_title.clone(), semantic.clone()),
                                "complete source/IR: {source}"
                            );
                            let actual_layout =
                                diagram_layout_temporal::try_layout_temporal_diagram(
                                    &TemporalDiagram {
                                        kind: TemporalKind::Timeline,
                                        title: expected_title,
                                        body: TemporalBody::Timeline(parsed.1),
                                    },
                                    1000.0,
                                )
                                .expect("ordinary separate event cards must be admitted");
                            assert_eq!(
                                actual_layout, expected_layout,
                                "complete checked layout: {source}"
                            );
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
                            assert_eq!(
                                diagram_to_paint_temporal(&actual_layout, &options),
                                expected_scene,
                                "complete Scene: {source}"
                            );
                            assert_eq!(
                                *resolver.0.borrow(),
                                expected_queries,
                                "ordered resolver calls: {source}"
                            );
                            println!("timeline-event-source COMPLETE {direction:?}:{custom}:{title}:{named}:{single}");
                        }
                    }
                }
            }
        }
    }
}
