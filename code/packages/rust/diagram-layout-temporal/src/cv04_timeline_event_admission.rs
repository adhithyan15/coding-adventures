//! Checked separate cards: complete ordinary layouts and represented geometry.
//! The link controls exercise the unchanged painter's actual triangle, including
//! finite links whose arrow corners collapse after floating-point rounding.
use super::*;
use diagram_ir::{TemporalKind, TimelineConfig, TimelinePeriod, TimelineSection};

fn layout(items: Vec<LayoutedTemporalItem>) -> LayoutedTemporalDiagram {
    LayoutedTemporalDiagram {
        width: 640.0,
        height: 300.0,
        accessibility_title: None,
        accessibility_description: None,
        timeline_config: None,
        interactions: Vec::new(),
        items,
    }
}

fn event(x: f64, y: f64, width: f64, height: f64) -> LayoutedTemporalItem {
    LayoutedTemporalItem::TimelineEvent {
        x,
        y,
        width,
        height,
        label: "E".into(),
        color_index: 0,
    }
}

fn link(x1: f64, y1: f64, x2: f64, y2: f64) -> LayoutedTemporalItem {
    LayoutedTemporalItem::TimelineEventLink {
        x1,
        y1,
        x2,
        y2,
        color_index: 0,
    }
}

#[test]
fn separate_event_actual_outer_and_text_frames() {
    let x = 2f64.powi(59);
    assert!(x + 80.0 > x);
    assert_eq!((x + 10.0) + 60.0, x + 10.0);
    let y = 2f64.powi(58);
    assert!(y + 40.0 > y);
    assert_eq!((y + 6.0) + 28.0, y + 6.0);
    for (cell, (x, y, w, h, accepted)) in [
        (20.0, 20.0, 160.0, 30.0, true),
        (20.0, 20.0, 21.0, 13.0, true),
        (20.0, 20.0, 20.0, 30.0, false),
        (20.0, 20.0, 19.0, 30.0, false),
        (20.0, 20.0, 160.0, 12.0, false),
        (20.0, 20.0, 160.0, 11.0, false),
        // The outer80 advances at2^59; the inner60 is swallowed.
        (2f64.powi(59), 20.0, 80.0, 30.0, false),
        (20.0, 2f64.powi(58), 160.0, 40.0, false),
        (20.0, 2f64.powi(58), 160.0, 30.0, false),
        (f64::MAX, 20.0, f64::MAX, 30.0, false),
        (20.0, 20.0, f64::NAN, 30.0, false),
    ]
    .into_iter()
    .rev()
    .enumerate()
    {
        let expected = layout(vec![event(x, y, w, h)]);
        let actual = validate_timeline_layout(expected.clone());
        if accepted {
            assert_eq!(actual, Ok(expected));
        } else {
            assert_eq!(actual, Err(TemporalLayoutError::TimelineGeometry));
        }
        println!("timeline-event-frame COMPLETE cell={cell} {x:?}:{y:?}:{w:?}:{h:?}:{accepted}");
    }
}

#[test]
fn event_link_certifies_actual_arrow_triangle() {
    for (cell, (x1, y1, x2, y2, accepted)) in [
        (20.0, 20.0, 56.0, 11.0, true),
        (20.0, 20.0, 20.0, 38.0, true),
        (56.0, 11.0, 20.0, 20.0, true),
        (
            2f64.powi(56),
            2f64.powi(56),
            2f64.powi(56),
            2f64.powi(56) + 32.0,
            true,
        ),
        (20.0, 20.0, 20.0, 20.0, false),
        (-f64::MAX, 0.0, f64::MAX, 1.0, false),
        (0.0, 0.0, 1e200, 1.0, false),
        (0.0, 0.0, 1e154, 1e154, false),
        (0.0, 0.0, f64::MIN_POSITIVE, 0.0, false),
        (0.0, 0.0, f64::INFINITY, 1.0, false),
        (0.0, 0.0, 1.0, f64::NAN, false),
        // Nonzero represented delta, but the triangle's corners coincide.
        (
            2f64.powi(56) + 32.0,
            2f64.powi(56),
            2f64.powi(56) + 32.0,
            2f64.powi(56) + 32.0,
            false,
        ),
    ]
    .into_iter()
    .rev()
    .enumerate()
    {
        let expected = layout(vec![link(x1, y1, x2, y2)]);
        let actual = validate_timeline_layout(expected.clone());
        if accepted {
            assert_eq!(actual, Ok(expected));
        } else {
            assert_eq!(actual, Err(TemporalLayoutError::TimelineGeometry));
        }
        println!(
            "timeline-event-arrow COMPLETE cell={cell} {x1:?}:{y1:?}:{x2:?}:{y2:?}:{accepted}"
        );
    }
    // A chronology spine is still permitted to have a single represented point.
    let spine = layout(vec![LayoutedTemporalItem::TimelineSpine {
        x1: 20.0,
        y1: 20.0,
        x2: 20.0,
        y2: 20.0,
    }]);
    assert_eq!(validate_timeline_layout(spine.clone()), Ok(spine));
}

fn input(direction: TimelineDirection, padding: f64) -> TemporalDiagram {
    TemporalDiagram {
        kind: TemporalKind::Timeline,
        title: None,
        body: TemporalBody::Timeline(TimelineDiagram {
            accessibility_title: None,
            accessibility_description: None,
            config: TimelineConfig {
                padding,
                left_margin: 0.0,
                ..Default::default()
            },
            direction,
            sections: vec![TimelineSection {
                label: None,
                periods: vec![TimelinePeriod {
                    label: "P".into(),
                    events: vec!["E".into()],
                }],
            }],
        }),
    }
}

#[test]
fn lr_large_coordinates_distinguish_valid_arrow_from_rounded_collapse() {
    for exponent in [56, 57] {
        assert_eq!(
            try_layout_temporal_diagram(
                &input(TimelineDirection::LeftRight, 2f64.powi(exponent)),
                2f64.powi(60)
            ),
            Err(TemporalLayoutError::TimelineGeometry)
        );
        println!("timeline-event-public-refusal COMPLETE exponent={exponent}");
    }
    // Literal f64 results from the independent incoming-order model. Expected
    // coordinates are established before calling the checked constructor.
    let p = 36_028_797_018_963_968.0; // 2^55, ULP8.
    let mut expected = layout(vec![
        LayoutedTemporalItem::TimelinePeriod {
            x: p + 104.0,
            y: p + 72.0,
            width: 190.0,
            height: 48.0,
            label: "P".into(),
            events: Vec::new(),
            color_index: 0,
        },
        link(p + 200.0, p + 120.0, p + 200.0, p + 136.0),
        event(p + 104.0, p + 136.0, 190.0, 30.0),
        LayoutedTemporalItem::TimelineSpine {
            x1: p + 200.0,
            y1: p + 64.0,
            x2: p + 200.0,
            y2: p + 64.0,
        },
    ]);
    expected.width = 2f64.powi(60);
    expected.height = p * 2.0 + 192.0;
    expected.timeline_config = Some(TimelineConfig {
        padding: p,
        left_margin: 0.0,
        ..Default::default()
    });
    assert_eq!(
        try_layout_temporal_diagram(&input(TimelineDirection::LeftRight, p), 2f64.powi(60)),
        Ok(expected)
    );
    println!("timeline-event-public-positive COMPLETE exponent=55");
}

#[test]
fn retired_canvas_coupled_widths_remain_direct_legacy_guards() {
    for (x, sx, width, accepted) in [
        (591.0, 563.0, 25.0, true),
        (592.0, 564.0, 24.0, false),
        (593.0, 565.0, 23.0, false),
    ] {
        let mut legacy = layout(vec![
            LayoutedTemporalItem::TimelineSpine {
                x1: sx,
                y1: 66.0,
                x2: sx,
                y2: 90.0,
            },
            LayoutedTemporalItem::TimelinePeriod {
                x,
                y: 50.0,
                width,
                height: 48.0,
                label: "P0".into(),
                events: Vec::new(),
                color_index: 0,
            },
        ]);
        legacy.height = 176.0;
        legacy.accessibility_title = Some("Accessible history".into());
        legacy.accessibility_description = Some("Ordered milestones".into());
        legacy.timeline_config = Some(TimelineConfig {
            padding: 50.0,
            left_margin: sx - 156.0,
            use_max_width: true,
            ..Default::default()
        });
        let actual = validate_timeline_layout(legacy.clone());
        if accepted {
            assert_eq!(actual, Ok(legacy));
        } else {
            assert_eq!(actual, Err(TemporalLayoutError::TimelineGeometry));
        }
    }
    let mut large_padding = layout(vec![
        LayoutedTemporalItem::TimelineSpine {
            x1: 1156.0,
            y1: 1016.0,
            x2: 1156.0,
            y2: 1040.0,
        },
        LayoutedTemporalItem::TimelinePeriod {
            x: 1184.0,
            y: 1000.0,
            width: 24.0,
            height: 48.0,
            label: "P0".into(),
            events: Vec::new(),
            color_index: 0,
        },
    ]);
    large_padding.height = 2076.0;
    large_padding.accessibility_title = Some("Accessible history".into());
    large_padding.accessibility_description = Some("Ordered milestones".into());
    large_padding.timeline_config = Some(TimelineConfig {
        padding: 1000.0,
        left_margin: 50.0,
        use_max_width: true,
        ..Default::default()
    });
    // This24-wide forged payload isolates a zero inner frame. The actual
    // retired constructor used raw640 - 1156 - 52, without a width clamp.
    assert_eq!(
        validate_timeline_layout(large_padding.clone()),
        Err(TemporalLayoutError::TimelineGeometry)
    );
    let mut retired_large_padding = large_padding;
    if let LayoutedTemporalItem::TimelinePeriod { width, .. } = &mut retired_large_padding.items[1]
    {
        *width = -568.0;
    }
    assert_eq!(
        validate_timeline_layout(retired_large_padding),
        Err(TemporalLayoutError::TimelineGeometry)
    );
    println!("timeline-event-retired-large-padding COMPLETE width=-568");
    println!("timeline-event-legacy-widths COMPLETE cells=4");
}

#[test]
fn product_history_keeps_full_labels_accessibility_order_and_legacy_layout() {
    let cfg = TimelineConfig::default();
    let diagram = TemporalDiagram {
        kind: TemporalKind::Timeline,
        title: Some("Product history".into()),
        body: TemporalBody::Timeline(TimelineDiagram {
            accessibility_title: Some("Release history".into()),
            accessibility_description: Some("Milestones by year".into()),
            config: cfg.clone(),
            direction: TimelineDirection::TopDown,
            sections: vec![TimelineSection {
                label: Some("Foundation".into()),
                periods: vec![TimelinePeriod {
                    label: "2024".into(),
                    events: vec!["Prototype".into(), "First customer".into()],
                }],
            }],
        }),
    };
    let title = LayoutedTemporalItem::TemporalTitle {
        x: 50.0,
        y: 50.0,
        width: 540.0,
        height: 40.0,
        label: "Product history".into(),
    };
    let section = LayoutedTemporalItem::TimelineSection {
        x: 100.0,
        y: 102.0,
        width: 136.0,
        height: 30.0,
        label: "Foundation".into(),
        color_index: 0,
    };
    let mut legacy = layout(vec![
        title.clone(),
        LayoutedTemporalItem::TimelineSpine {
            x1: 206.0,
            y1: 118.0,
            x2: 206.0,
            y2: 216.0,
        },
        section.clone(),
        LayoutedTemporalItem::TimelinePeriod {
            x: 234.0,
            y: 140.0,
            width: 382.0,
            height: 84.0,
            label: "2024".into(),
            events: vec!["Prototype".into(), "First customer".into()],
            color_index: 0,
        },
    ]);
    legacy.height = 302.0;
    legacy.accessibility_title = Some("Release history".into());
    legacy.accessibility_description = Some("Milestones by year".into());
    legacy.timeline_config = Some(cfg);
    assert_eq!(validate_timeline_layout(legacy.clone()), Ok(legacy.clone()));
    let expected = LayoutedTemporalDiagram {
        height: 286.0,
        items: vec![
            title,
            LayoutedTemporalItem::TimelineSpine {
                x1: 206.0,
                y1: 118.0,
                x2: 206.0,
                y2: 200.0,
            },
            section,
            LayoutedTemporalItem::TimelinePeriod {
                x: 234.0,
                y: 140.0,
                width: 180.0,
                height: 48.0,
                label: "2024".into(),
                events: Vec::new(),
                color_index: 0,
            },
            link(414.0, 164.0, 450.0, 155.0),
            LayoutedTemporalItem::TimelineEvent {
                x: 450.0,
                y: 140.0,
                width: 160.0,
                height: 30.0,
                label: "Prototype".into(),
                color_index: 0,
            },
            link(414.0, 164.0, 450.0, 193.0),
            LayoutedTemporalItem::TimelineEvent {
                x: 450.0,
                y: 178.0,
                width: 160.0,
                height: 30.0,
                label: "First customer".into(),
                color_index: 0,
            },
        ],
        ..legacy
    };
    assert_eq!(try_layout_temporal_diagram(&diagram, 640.0), Ok(expected));
}
