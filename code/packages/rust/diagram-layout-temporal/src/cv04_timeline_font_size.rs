//! Retain the original literal frames below, then explicitly migrate expected minima.
use super::*;
use diagram_ir::{TemporalKind, TimelineConfig, TimelinePeriod, TimelineSection};

fn size_config(size: f64) -> TimelineConfig {
    TimelineConfig {
        font_family: "\"trebuchet ms\", verdana, arial, sans-serif".into(),
        font_size: size,
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
    }
}

fn size_fixture(
    size: f64,
    direction: TimelineDirection,
    titled: bool,
    named: bool,
) -> TemporalDiagram {
    TemporalDiagram {
        kind: TemporalKind::Timeline,
        title: titled.then(|| "T".into()),
        body: TemporalBody::Timeline(TimelineDiagram {
            accessibility_title: Some("Accessible".into()),
            accessibility_description: Some("Description".into()),
            config: size_config(size),
            direction,
            sections: vec![TimelineSection {
                label: named.then(|| "S".into()),
                periods: vec![TimelinePeriod {
                    label: "P".into(),
                    events: vec!["E".into()],
                }],
            }],
        }),
    }
}

fn declared_size_layout(size: f64, td: bool, titled: bool, named: bool) -> LayoutedTemporalDiagram {
    let offset = if titled { 52.0 } else { 0.0 };
    let sy = 50.0 + offset;
    let py = if td {
        sy + if named { 38.0 } else { 0.0 }
    } else {
        118.0 + offset
    };
    let (px, pw, ex, ey, ew, x1, y1, x2, y2) = if td {
        (
            234.0,
            180.0,
            450.0,
            py,
            160.0,
            414.0,
            py + 24.0,
            450.0,
            py + 15.0,
        )
    } else {
        (
            206.0,
            190.0,
            206.0,
            184.0 + offset,
            190.0,
            301.0,
            166.0 + offset,
            301.0,
            184.0 + offset,
        )
    };
    let mut items = Vec::new();
    if titled {
        items.push(LayoutedTemporalItem::TemporalTitle {
            x: 50.0,
            y: 50.0,
            width: 540.0,
            height: 40.0,
            label: "T".into(),
        });
    }
    if td {
        items.push(LayoutedTemporalItem::TimelineSpine {
            x1: 206.0,
            y1: 66.0 + offset,
            x2: 206.0,
            y2: 90.0 + offset + if named { 38.0 } else { 0.0 },
        });
    }
    if named {
        items.push(LayoutedTemporalItem::TimelineSection {
            x: 100.0,
            y: sy,
            width: 136.0,
            height: 30.0,
            label: "S".into(),
            color_index: 0,
        });
    }
    items.extend([
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
    ]);
    if !td {
        items.push(LayoutedTemporalItem::TimelineSpine {
            x1: 301.0,
            y1: 106.0 + offset,
            x2: 301.0,
            y2: 106.0 + offset,
        });
    }
    LayoutedTemporalDiagram {
        width: 640.0,
        height: if td {
            176.0 + offset + if named { 38.0 } else { 0.0 }
        } else {
            296.0 + offset
        },
        accessibility_title: Some("Accessible".into()),
        accessibility_description: Some("Description".into()),
        timeline_config: Some(size_config(size)),
        interactions: Vec::new(),
        items,
    }
}

const SIZES: [f64; 7] = [
    f64::from_bits(1),
    f64::MIN_POSITIVE,
    1.0,
    16.0,
    24.0,
    65536.0,
    f64::MAX,
];

#[test]
fn timeline_size_positive_domain_keeps_complete_frames() {
    for (i, size) in SIZES.into_iter().enumerate() {
        for (direction, td) in [
            (TimelineDirection::TopDown, true),
            (TimelineDirection::LeftRight, false),
        ] {
            for titled in [false, true] {
                for named in [false, true] {
                    let old = declared_size_layout(size, td, titled, named);
                    let expected = migrated_size_layout(old, size, td, titled, named);
                    assert_eq!(
                        try_layout_temporal_diagram(
                            &size_fixture(size, direction, titled, named),
                            640.0
                        ),
                        expected,
                        "whole size layout {i}:{td}:{titled}:{named}"
                    );
                    println!("timeline-size-layout COMPLETE {i}:{td}:{titled}:{named}");
                }
            }
        }
    }
}

#[test]
fn timeline_size_empty_positive_domain_keeps_zero_height() {
    for (i, size) in SIZES.into_iter().enumerate() {
        for direction in [TimelineDirection::TopDown, TimelineDirection::LeftRight] {
            for cw in [0.0, -640.0] {
                let mut input = size_fixture(size, direction, false, false);
                let TemporalBody::Timeline(t) = &mut input.body else {
                    unreachable!()
                };
                t.config.padding = 0.0;
                t.config.left_margin = 0.0;
                t.sections.clear();
                let mut cfg = size_config(size);
                cfg.padding = 0.0;
                cfg.left_margin = 0.0;
                let expected = LayoutedTemporalDiagram {
                    width: 560.0,
                    height: 0.0,
                    accessibility_title: Some("Accessible".into()),
                    accessibility_description: Some("Description".into()),
                    timeline_config: Some(cfg),
                    interactions: Vec::new(),
                    items: Vec::new(),
                };
                assert_eq!(
                    try_layout_temporal_diagram(&input, cw),
                    if size == f64::MAX {
                        // Keep the whole old empty body above; only its Result changes.
                        assert!(expected.items.is_empty());
                        Err(TemporalLayoutError::TimelineGeometry)
                    } else {
                        Ok(expected)
                    },
                    "empty size layout {i}:{direction:?}:{cw}"
                );
                println!("timeline-size-empty COMPLETE {i}:{direction:?}:{cw}");
            }
        }
    }
}

#[test]
fn timeline_size_invalid_domain_is_exact_typed_error_before_work() {
    for (i, size) in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.0, -0.0, -1.0]
        .into_iter()
        .enumerate()
    {
        for direction in [TimelineDirection::TopDown, TimelineDirection::LeftRight] {
            TIMELINE_WORK.with(|v| v.set((0, 0)));
            assert_eq!(
                try_layout_temporal_diagram(&size_fixture(size, direction, true, true), 640.0),
                Err(TemporalLayoutError::TimelineFontSize),
                "typed invalid size {i}:{direction:?}"
            );
            assert_eq!(TIMELINE_WORK.with(|v| v.get()), (0, 0));
            println!("timeline-size-invalid COMPLETE {i}:{direction:?}");
        }
    }
}

#[test]
fn timeline_size_input_priority_is_exact_before_derived_geometry() {
    for (i, (cw, pad, margin, size, error)) in [
        (
            f64::NAN,
            f64::NAN,
            f64::NAN,
            f64::NAN,
            TemporalLayoutError::TimelineCanvas,
        ),
        (
            f64::INFINITY,
            -1.0,
            -1.0,
            0.0,
            TemporalLayoutError::TimelineCanvas,
        ),
        (
            640.0,
            f64::NAN,
            f64::NAN,
            f64::NAN,
            TemporalLayoutError::TimelinePadding,
        ),
        (640.0, -1.0, -1.0, 0.0, TemporalLayoutError::TimelinePadding),
        (
            640.0,
            50.0,
            f64::NAN,
            f64::NAN,
            TemporalLayoutError::TimelineLeftMargin,
        ),
        (
            640.0,
            50.0,
            -1.0,
            0.0,
            TemporalLayoutError::TimelineLeftMargin,
        ),
        (
            640.0,
            1e308,
            1e308,
            f64::NAN,
            TemporalLayoutError::TimelineFontSize,
        ),
        (
            640.0,
            1e308,
            1e308,
            f64::MAX,
            TemporalLayoutError::TimelineGeometry,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let mut input = size_fixture(size, TimelineDirection::TopDown, true, true);
        let TemporalBody::Timeline(t) = &mut input.body else {
            unreachable!()
        };
        t.config.padding = pad;
        t.config.left_margin = margin;
        TIMELINE_WORK.with(|v| v.set((0, 0)));
        assert_eq!(
            try_layout_temporal_diagram(&input, cw),
            Err(error),
            "size priority {i}"
        );
        assert_eq!(TIMELINE_WORK.with(|v| v.get()), (0, 0));
        println!("timeline-size-priority COMPLETE {i}");
    }
}

#[test]
#[should_panic(expected = "unsupported temporal geometry")]
fn timeline_size_compatibility_entry_panics_on_invalid_size() {
    layout_temporal_diagram(
        &size_fixture(0.0, TimelineDirection::TopDown, false, false),
        640.0,
    );
}

// Only these two historical cells grow. The constants are independent literal
// products/positions for the retained ONE-period, ONE-event fixture; they are
// intentionally not a second generic layout algorithm. Refuse unknown sizes.
fn migrated_size_layout(
    old: LayoutedTemporalDiagram,
    size: f64,
    td: bool,
    titled: bool,
    named: bool,
) -> Result<LayoutedTemporalDiagram, TemporalLayoutError> {
    let mut literal_control = declared_size_layout(16.0, td, titled, named);
    literal_control.timeline_config = Some(size_config(size));
    assert_eq!(
        old, literal_control,
        "all original declared fields retained"
    );
    if size == f64::MAX {
        return Err(TemporalLayoutError::TimelineGeometry);
    }
    if size <= 16.0 {
        return Ok(old);
    }
    let (
        th,
        sh,
        ch,
        eh,
        title_advance,
        section_advance,
        td_height,
        td_end,
        lr_event,
        lr_link,
        lr_height,
    ) = if size == 24.0 {
        (
            60.0, 45.0, 72.0, 45.0, 72.0, 53.0, 200.0, 114.0, 208.0, 190.0, 335.0,
        )
    } else {
        assert_eq!(size, 65536.0);
        (
            163840.0, 122880.0, 196608.0, 122880.0, 163852.0, 122888.0, 196736.0, 196650.0,
            196744.0, 196726.0, 319706.0,
        )
    };
    let offset = if titled { title_advance } else { 0.0 };
    let section_offset = if td && named { section_advance } else { 0.0 };
    let mut expected = old;
    expected.height = if td {
        td_height + offset + section_offset
    } else {
        lr_height + offset
    };
    for item in &mut expected.items {
        match item {
            LayoutedTemporalItem::TemporalTitle { height, .. } => *height = th,
            LayoutedTemporalItem::TimelineSection { y, height, .. } => {
                *y = 50.0 + offset;
                *height = sh;
            }
            LayoutedTemporalItem::TimelinePeriod { y, height, .. } => {
                *y = if td {
                    50.0 + offset + section_offset
                } else {
                    118.0 + offset
                };
                *height = ch;
            }
            LayoutedTemporalItem::TimelineEvent { y, height, .. } => {
                *y = if td {
                    50.0 + offset + section_offset
                } else {
                    lr_event + offset
                };
                *height = eh;
            }
            LayoutedTemporalItem::TimelineEventLink { y1, y2, .. } => {
                *y1 = if td {
                    50.0 + offset + section_offset + ch / 2.0
                } else {
                    lr_link + offset
                };
                *y2 = if td {
                    50.0 + offset + section_offset + eh / 2.0
                } else {
                    lr_event + offset
                };
            }
            LayoutedTemporalItem::TimelineSpine { y1, y2, .. } => {
                *y1 = if td { 66.0 + offset } else { 106.0 + offset };
                *y2 = if td {
                    td_end + offset + section_offset
                } else {
                    106.0 + offset
                };
            }
            other => panic!("closed original shape: {other:?}"),
        }
    }
    Ok(expected)
}

// Independent complete diagrams distinguish indexed event pitch, next-period
// row advance and empty-section work. Every number is declared for s=24.
#[test]
fn timeline_size_minima_multi_period_event_and_empty_row_complete() {
    for (direction, td) in [
        (TimelineDirection::TopDown, true),
        (TimelineDirection::LeftRight, false),
    ] {
        let mut input = size_fixture(24.0, direction, true, true);
        let TemporalBody::Timeline(t) = &mut input.body else {
            unreachable!()
        };
        t.sections[0].periods[0].events.push("F".into());
        t.sections[0].periods.push(TimelinePeriod {
            label: "Q".into(),
            events: Vec::new(),
        });
        t.sections.push(TimelineSection {
            label: None,
            periods: Vec::new(),
        });
        let mut items = vec![LayoutedTemporalItem::TemporalTitle {
            x: 50.0,
            y: 50.0,
            width: 540.0,
            height: 60.0,
            label: "T".into(),
        }];
        if td {
            items.push(LayoutedTemporalItem::TimelineSpine {
                x1: 206.0,
                y1: 138.0,
                x2: 206.0,
                y2: 365.0,
            });
        }
        items.push(LayoutedTemporalItem::TimelineSection {
            x: 100.0,
            y: 122.0,
            width: 136.0,
            height: 45.0,
            label: "S".into(),
            color_index: 0,
        });
        let (px, py, pw, ex, first, second, ew, lx, ly, ly0, ly1, qx, qy) = if td {
            (
                234.0, 175.0, 180.0, 450.0, 175.0, 228.0, 160.0, 414.0, 211.0, 197.5, 250.5, 234.0,
                291.0,
            )
        } else {
            (
                206.0, 190.0, 190.0, 206.0, 280.0, 333.0, 190.0, 301.0, 262.0, 280.0, 333.0, 420.0,
                190.0,
            )
        };
        items.push(LayoutedTemporalItem::TimelinePeriod {
            x: px,
            y: py,
            width: pw,
            height: 72.0,
            label: "P".into(),
            events: Vec::new(),
            color_index: 0,
        });
        for (label, y, y2) in [("E", first, ly0), ("F", second, ly1)] {
            items.push(LayoutedTemporalItem::TimelineEventLink {
                x1: lx,
                y1: ly,
                x2: if td { 450.0 } else { 301.0 },
                y2,
                color_index: 0,
            });
            items.push(LayoutedTemporalItem::TimelineEvent {
                x: ex,
                y,
                width: ew,
                height: 45.0,
                label: label.into(),
                color_index: 0,
            });
        }
        items.push(LayoutedTemporalItem::TimelinePeriod {
            x: qx,
            y: qy,
            width: pw,
            height: 72.0,
            label: "Q".into(),
            events: Vec::new(),
            color_index: 1,
        });
        if !td {
            items.push(LayoutedTemporalItem::TimelineSpine {
                x1: 301.0,
                y1: 178.0,
                x2: 515.0,
                y2: 178.0,
            });
        }
        let expected = LayoutedTemporalDiagram {
            width: if td { 640.0 } else { 660.0 },
            height: if td { 451.0 } else { 618.0 },
            accessibility_title: Some("Accessible".into()),
            accessibility_description: Some("Description".into()),
            timeline_config: Some(size_config(24.0)),
            interactions: Vec::new(),
            items,
        };
        assert_eq!(
            try_layout_temporal_diagram(&input, 640.0),
            Ok(expected.clone())
        );
        assert_eq!(layout_temporal_diagram(&input, 640.0), expected);
        println!("timeline-minima-multi COMPLETE {td}");
    }
}

#[test]
fn timeline_size_minima_fractional_complete() {
    for (direction, td) in [
        (TimelineDirection::TopDown, true),
        (TimelineDirection::LeftRight, false),
    ] {
        let input = size_fixture(17.25, direction, true, true);
        let mut items = vec![LayoutedTemporalItem::TemporalTitle {
            x: 50.0,
            y: 50.0,
            width: 540.0,
            height: 43.125,
            label: "T".into(),
        }];
        if td {
            items.push(LayoutedTemporalItem::TimelineSpine {
                x1: 206.0,
                y1: 121.125,
                x2: 206.0,
                y2: 189.21875,
            });
        }
        items.push(LayoutedTemporalItem::TimelineSection {
            x: 100.0,
            y: 105.125,
            width: 136.0,
            height: 32.34375,
            label: "S".into(),
            color_index: 0,
        });
        items.push(LayoutedTemporalItem::TimelinePeriod {
            x: if td { 234.0 } else { 206.0 },
            y: if td { 145.46875 } else { 173.125 },
            width: if td { 180.0 } else { 190.0 },
            height: 51.75,
            label: "P".into(),
            events: Vec::new(),
            color_index: 0,
        });
        items.push(LayoutedTemporalItem::TimelineEventLink {
            x1: if td { 414.0 } else { 301.0 },
            y1: if td { 171.34375 } else { 224.875 },
            x2: if td { 450.0 } else { 301.0 },
            y2: if td { 161.640625 } else { 242.875 },
            color_index: 0,
        });
        items.push(LayoutedTemporalItem::TimelineEvent {
            x: if td { 450.0 } else { 206.0 },
            y: if td { 145.46875 } else { 242.875 },
            width: if td { 160.0 } else { 190.0 },
            height: 32.34375,
            label: "E".into(),
            color_index: 0,
        });
        if !td {
            items.push(LayoutedTemporalItem::TimelineSpine {
                x1: 301.0,
                y1: 161.125,
                x2: 301.0,
                y2: 161.125,
            });
        }
        let expected = LayoutedTemporalDiagram {
            width: 640.0,
            height: if td { 275.21875 } else { 357.21875 },
            accessibility_title: Some("Accessible".into()),
            accessibility_description: Some("Description".into()),
            timeline_config: Some(size_config(17.25)),
            interactions: Vec::new(),
            items,
        };
        assert_eq!(
            try_layout_temporal_diagram(&input, 640.0),
            Ok(expected.clone())
        );
        assert_eq!(layout_temporal_diagram(&input, 640.0), expected);
        println!("timeline-minima-fractional COMPLETE {td}");
    }
}

#[test]
fn timeline_size_minima_exact_product_boundary_before_work() {
    // These adjacent f64 values straddle the rounded s*3 overflow boundary.
    // Division MAX/3 itself rounds UP to the refused neighbor; it is no oracle.
    let last = f64::from_bits(0x7fd5_5555_5555_5554);
    let next = f64::from_bits(0x7fd5_5555_5555_5555);
    assert_eq!((last * 3.0).to_bits(), 0x7fef_ffff_ffff_fffe);
    assert!((next * 3.0).is_infinite());
    for direction in [TimelineDirection::TopDown, TimelineDirection::LeftRight] {
        for (i, size) in [last, next, f64::MAX].into_iter().enumerate() {
            for populated in [false, true] {
                let mut input = size_fixture(size, direction, false, false);
                let TemporalBody::Timeline(t) = &mut input.body else {
                    unreachable!()
                };
                if !populated {
                    t.sections.clear();
                }
                TIMELINE_WORK.with(|v| v.set((0, 0)));
                let actual = try_layout_temporal_diagram(&input, 640.0);
                if i == 0 && !populated {
                    assert_eq!(
                        actual,
                        Ok(LayoutedTemporalDiagram {
                            width: 640.0,
                            height: 100.0,
                            accessibility_title: Some("Accessible".into()),
                            accessibility_description: Some("Description".into()),
                            timeline_config: Some(size_config(size)),
                            interactions: Vec::new(),
                            items: Vec::new()
                        })
                    );
                } else {
                    // Even a finite planned product may fail the actual arrow,
                    // rectangle endpoints or row arithmetic on a populated input.
                    assert_eq!(actual, Err(TemporalLayoutError::TimelineGeometry));
                }
                if i > 0 || !populated {
                    assert_eq!(TIMELINE_WORK.with(|v| v.get()), (0, 0));
                } else {
                    assert_eq!(TIMELINE_WORK.with(|v| v.get()), (1, 1));
                }
                println!("timeline-minima-boundary COMPLETE {direction:?}:{i}:{populated}");
            }
        }
    }
}

#[test]
#[should_panic(expected = "unsupported temporal geometry")]
fn timeline_size_minima_compatibility_panics_on_product_overflow() {
    layout_temporal_diagram(
        &size_fixture(f64::MAX, TimelineDirection::LeftRight, false, false),
        640.0,
    );
}

#[test]
fn timeline_size_minima_default_title_keeps_represented_association() {
    // A valid complete layout distinguishes p+(40+12) from (p+40)+12.
    // Large LR event coordinates would instead be refused by arrow checks;
    // this accepted title-only TD witness cannot hide behind that refusal.
    let padding = f64::from_bits(0x4370_0000_0000_0001);
    let canvas = f64::from_bits(0x4390_0000_0000_0001);
    let mut input = size_fixture(16.0, TimelineDirection::TopDown, true, false);
    let TemporalBody::Timeline(t) = &mut input.body else {
        unreachable!()
    };
    t.sections.clear();
    t.config.padding = padding;
    t.config.left_margin = 0.0;
    let mut cfg = size_config(16.0);
    cfg.padding = padding;
    cfg.left_margin = 0.0;
    let expected = LayoutedTemporalDiagram {
        width: canvas,
        height: f64::from_bits(0x4380_0000_0000_0002),
        accessibility_title: Some("Accessible".into()),
        accessibility_description: Some("Description".into()),
        timeline_config: Some(cfg),
        interactions: Vec::new(),
        items: vec![LayoutedTemporalItem::TemporalTitle {
            x: padding,
            y: padding,
            width: f64::from_bits(0x4380_0000_0000_0001),
            height: 40.0,
            label: "T".into(),
        }],
    };
    assert_eq!(
        try_layout_temporal_diagram(&input, canvas),
        Ok(expected.clone())
    );
    assert_eq!(layout_temporal_diagram(&input, canvas), expected);
    println!("timeline-minima-association COMPLETE default-title");
}
