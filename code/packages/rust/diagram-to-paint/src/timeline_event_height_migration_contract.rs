// Closed migration of DECLARED one-event and seven-period products. Returned
// layouts/Scenes never enter this seam. The complete predecessors are also
// painted independently below, with a separate resolver and exact query vector.
fn prove_event_predecessor(
    product: &(LayoutedTemporalDiagram, PaintScene),
    options: &DiagramToPaintOptions<'_, FakeShaper, FakeMetrics, Resolver>,
    queries: &[FontQuery],
) {
    let resolver = Resolver(std::cell::RefCell::new(Vec::new()));
    let separate = DiagramToPaintOptions {
        label_font: options.label_font.clone(),
        title_font: options.title_font.clone(),
        shaper: options.shaper,
        metrics: options.metrics,
        resolver: &resolver,
        background: options.background,
        device_pixel_ratio: options.device_pixel_ratio,
    };
    assert_eq!(
        diagram_to_paint_temporal(&product.0, &separate),
        product.1,
        "full predecessor Scene"
    );
    assert_eq!(
        resolver.0.borrow().as_slice(),
        queries,
        "full predecessor query order"
    );
}

fn migrate_event_product(
    old: LayoutedTemporalDiagram,
    old_scene: PaintScene,
    td: bool,
) -> (LayoutedTemporalDiagram, PaintScene) {
    let cfg = old.timeline_config.as_ref().unwrap();
    assert_eq!(cfg.font_size.to_bits(), 16.0f64.to_bits());
    let periods = old
        .items
        .iter()
        .filter(|i| matches!(i, LayoutedTemporalItem::TimelinePeriod { .. }))
        .count();
    assert!(periods == 1 || periods == 7, "closed period census");
    let named = old
        .items
        .iter()
        .any(|i| matches!(i, LayoutedTemporalItem::TimelineSection { .. }));
    assert!(!named || periods == 7);
    let titled = matches!(
        old.items.first(),
        Some(LayoutedTemporalItem::TemporalTitle { .. })
    );
    let base = cfg.padding + if titled { 52.0 } else { 0.0 };
    assert_eq!(old_scene.width, old.width);
    assert_eq!(old_scene.height, old.height);
    // Each row: section/period/event/link-start/spine y, followed by the three
    // explicit FakeMetrics baselines. Empty rows use only their section entry.
    let first = match base {
        50.0 => [50.0, 118.0, 184.0, 166.0, 106.0, 68.8, 138.8, 202.8],
        102.0 => [102.0, 170.0, 236.0, 218.0, 158.0, 120.8, 190.8, 254.8],
        72.0 => [72.0, 140.0, 206.0, 188.0, 128.0, 90.8, 160.8, 224.8],
        124.0 => [124.0, 192.0, 258.0, 240.0, 180.0, 142.8, 212.8, 276.8],
        _ => panic!("closed row origin"),
    };
    let (next_height, rows) = match (base, named) {
        (50.0, false) => (297.2, [first; 3]),
        (102.0, false) => (349.2, [first; 3]),
        (72.0, false) => (341.2, [first; 3]),
        (124.0, false) => (393.2, [first; 3]),
        (50.0, true) => (
            628.4000000000001,
            [
                first,
                [247.2, 315.2, 381.2, 363.2, 303.2, 266.0, 336.0, 400.0],
                [381.2, 449.2, 515.2, 497.2, 437.2, 400.0, 470.0, 534.0],
            ],
        ),
        (102.0, true) => (
            680.4000000000001,
            [
                first,
                [299.2, 367.2, 433.2, 415.2, 355.2, 318.0, 388.0, 452.0],
                [433.2, 501.2, 567.2, 549.2, 489.2, 452.0, 522.0, 586.0],
            ],
        ),
        (72.0, true) => (
            672.4000000000001,
            [
                first,
                [269.2, 337.2, 403.2, 385.2, 325.2, 288.0, 358.0, 422.0],
                [403.2, 471.2, 537.2, 519.2, 459.2, 422.0, 492.0, 556.0],
            ],
        ),
        (124.0, true) => (
            724.4000000000001,
            [
                first,
                [321.2, 389.2, 455.2, 437.2, 377.2, 340.0, 410.0, 474.0],
                [455.2, 523.2, 589.2, 571.2, 511.2, 474.0, 544.0, 608.0],
            ],
        ),
        _ => panic!("closed row table"),
    };
    if !td {
        assert_eq!(
            old.height,
            base + cfg.padding + if named { 526.0 } else { 196.0 }
        );
    }
    let mut next = old.clone();
    let mut scene = old_scene.clone();
    if !td {
        next.height = next_height;
        scene.height = next_height;
    }
    let mut geometry = 0;
    let mut glyph_changes = Vec::new();
    let mut row = 0;
    let mut sections = 0;
    let mut p = 0;
    let mut e = 0;
    for (before, after) in old.items.iter().zip(next.items.iter_mut()) {
        match (before, after) {
            (
                LayoutedTemporalItem::TemporalTitle { label, height, .. },
                LayoutedTemporalItem::TemporalTitle { .. },
            ) => {
                assert_eq!(label, "T");
                assert_eq!(*height, 40.0);
                glyph_changes.push(None);
            }
            (
                LayoutedTemporalItem::TimelineSection { label, y, .. },
                LayoutedTemporalItem::TimelineSection { y: ny, .. },
            ) => {
                row = sections;
                sections += 1;
                assert_eq!(label, ["S", "Empty", "S2"][row]);
                if !td {
                    assert_eq!(*y, base + [0.0, 196.0, 330.0][row]);
                    *ny = rows[row][0];
                }
                change_declared_rect(&mut scene.instructions[geometry], *y, *ny, 30.0, 30.0);
                geometry += 1;
                glyph_changes.push((!td).then_some(((*y + 6.0) + 12.8, rows[row][5])));
            }
            (
                LayoutedTemporalItem::TimelinePeriod {
                    label,
                    events,
                    y,
                    height,
                    ..
                },
                LayoutedTemporalItem::TimelinePeriod { y: ny, .. },
            ) => {
                assert_eq!(
                    label,
                    &if periods == 1 {
                        "P".into()
                    } else {
                        format!("P{p}")
                    }
                );
                p += 1;
                assert!(events.is_empty());
                assert_eq!(*height, 48.0);
                if !td {
                    assert_eq!(*y, base + [0.0, 196.0, 330.0][row] + 68.0);
                    *ny = rows[row][1];
                }
                change_declared_rect(&mut scene.instructions[geometry], *y, *ny, 48.0, 48.0);
                geometry += 1;
                glyph_changes.push((!td).then_some(((*y + 8.0) + 12.8, rows[row][6])));
            }
            (
                LayoutedTemporalItem::TimelineEventLink { x1, y1, x2, y2, .. },
                LayoutedTemporalItem::TimelineEventLink {
                    y1: ny1, y2: ny2, ..
                },
            ) => {
                if td {
                    assert_eq!(*x2 - *x1, 36.0);
                    assert_eq!(*y1 - *y2, 9.0);
                    let y = *y2 - 15.0;
                    assert!([
                        50.0, 72.0, 88.0, 102.0, 110.0, 116.0, 124.0, 138.0, 140.0, 154.0, 162.0,
                        168.0, 176.0, 182.0, 190.0, 204.0, 206.0, 220.0, 228.0, 234.0, 242.0,
                        248.0, 256.0, 270.0, 272.0, 294.0, 300.0, 314.0, 322.0, 336.0, 366.0,
                        380.0, 382.0, 388.0, 402.0, 404.0, 432.0, 434.0, 446.0, 448.0, 454.0,
                        456.0, 468.0, 470.0, 498.0, 500.0, 514.0, 520.0, 522.0, 536.0, 566.0,
                        580.0, 588.0, 602.0, 632.0, 654.0
                    ]
                    .contains(&y));
                    *ny2 = y + 15.6;
                } else {
                    assert_eq!(*y1, base + [0.0, 196.0, 330.0][row] + 116.0);
                    assert_eq!(*y2, base + [0.0, 196.0, 330.0][row] + 134.0);
                    *ny1 = rows[row][3];
                    *ny2 = rows[row][2];
                }
                change_declared_path(
                    &mut scene.instructions[geometry],
                    &[(*x1, *y1), (*x2, *y2)],
                    &[(*x1, *ny1), (*x2, *ny2)],
                    false,
                );
                let old_arrow = if td {
                    [
                        (*x2, *y2),
                        (
                            *x2 - 9.70142500145332 + 1.212678125181665,
                            *y2 + 2.42535625036333 + 4.85071250072666,
                        ),
                        (
                            *x2 - 9.70142500145332 - 1.212678125181665,
                            *y2 + 2.42535625036333 - 4.85071250072666,
                        ),
                    ]
                } else {
                    [(*x2, *y2), (*x2 - 5.0, *y2 - 10.0), (*x2 + 5.0, *y2 - 10.0)]
                };
                let arrow = if td {
                    let (uy10, uy5) = match *ny2 - *ny1 {
                        -8.400000000000006 => (-2.272296156064186, -1.136148078032093),
                        -8.399999999999977 => (-2.272296156064178, -1.136148078032089),
                        _ => panic!("closed TD arrow delta"),
                    };
                    [
                        (*x2, *ny2),
                        (
                            *x2 - 9.738412097417932 - uy5,
                            *ny2 - uy10 + 4.869206048708966,
                        ),
                        (
                            *x2 - 9.738412097417932 + uy5,
                            *ny2 - uy10 - 4.869206048708966,
                        ),
                    ]
                } else {
                    assert!([18.0, 18.000000000000057].contains(&(*ny2 - *ny1)));
                    [
                        (*x2, *ny2),
                        (*x2 - 5.0, *ny2 - 10.0),
                        (*x2 + 5.0, *ny2 - 10.0),
                    ]
                };
                change_declared_path(
                    &mut scene.instructions[geometry + 1],
                    &old_arrow,
                    &arrow,
                    true,
                );
                geometry += 2;
            }
            (
                LayoutedTemporalItem::TimelineEvent {
                    label, y, height, ..
                },
                LayoutedTemporalItem::TimelineEvent {
                    y: ny, height: nh, ..
                },
            ) => {
                assert_eq!(
                    label,
                    &if periods == 1 {
                        "E".into()
                    } else {
                        format!("E{e}")
                    }
                );
                e += 1;
                assert_eq!(*height, 30.0);
                *nh = 31.2;
                if !td {
                    assert_eq!(*y, base + [0.0, 196.0, 330.0][row] + 134.0);
                    *ny = rows[row][2];
                }
                change_declared_rect(&mut scene.instructions[geometry], *y, *ny, 30.0, 31.2);
                geometry += 1;
                glyph_changes.push((!td).then_some(((*y + 6.0) + 12.8, rows[row][7])));
            }
            (
                LayoutedTemporalItem::TimelineSpine { x1, y1, x2, y2 },
                LayoutedTemporalItem::TimelineSpine {
                    y1: ny1, y2: ny2, ..
                },
            ) => {
                if !td {
                    assert_eq!(*y1, base + [0.0, 196.0, 330.0][row] + 56.0);
                    assert_eq!(*y1, *y2);
                    *ny1 = rows[row][4];
                    *ny2 = rows[row][4];
                }
                change_declared_path(
                    &mut scene.instructions[geometry],
                    &[(*x1, *y1), (*x2, *y2)],
                    &[(*x1, *ny1), (*x2, *ny2)],
                    false,
                );
                geometry += 1;
            }
            _ => panic!("closed predecessor item"),
        }
    }
    assert_eq!(p, periods);
    assert_eq!(e, periods);
    assert_eq!(sections, if named { 3 } else { 0 });
    assert_eq!(scene.instructions.len(), geometry + glyph_changes.len());
    for (instruction, change) in scene.instructions[geometry..].iter_mut().zip(glyph_changes) {
        let PaintInstruction::GlyphRun(run) = instruction else {
            panic!("declared trailing glyph")
        };
        if let Some((before, after)) = change {
            assert_eq!(run.font_size, 16.0);
            for glyph in &mut run.glyphs {
                assert_eq!(glyph.y.to_bits(), before.to_bits());
                glyph.y = after;
            }
        }
    }
    (next, scene)
}

fn change_declared_rect(
    instruction: &mut PaintInstruction,
    old_y: f64,
    new_y: f64,
    old_h: f64,
    new_h: f64,
) {
    let PaintInstruction::Rect(rect) = instruction else {
        panic!("declared rectangle")
    };
    assert_eq!(rect.y.to_bits(), old_y.to_bits());
    assert_eq!(rect.height.to_bits(), old_h.to_bits());
    rect.y = new_y;
    rect.height = new_h;
}

fn change_declared_path(
    instruction: &mut PaintInstruction,
    before: &[(f64, f64)],
    after: &[(f64, f64)],
    close: bool,
) {
    let commands = |points: &[(f64, f64)]| {
        let mut result = vec![PathCommand::MoveTo {
            x: points[0].0,
            y: points[0].1,
        }];
        result.extend(
            points[1..]
                .iter()
                .map(|&(x, y)| PathCommand::LineTo { x, y }),
        );
        if close {
            result.push(PathCommand::Close);
        }
        result
    };
    let PaintInstruction::Path(path) = instruction else {
        panic!("declared path")
    };
    assert_eq!(path.commands, commands(before), "full path preimage");
    path.commands = commands(after);
}
