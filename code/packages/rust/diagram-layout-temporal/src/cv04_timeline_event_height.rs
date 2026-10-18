//! Declared whole products for each event's scalar estimate and cumulative stack.
use super::*;
use diagram_ir::{TemporalKind, TimelineConfig, TimelinePeriod, TimelineSection};

fn config() -> TimelineConfig {
    TimelineConfig {
        font_family: "\"trebuchet ms\", verdana, arial, sans-serif".into(),
        font_size: 16.0,
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

fn input(td: bool, sections: Vec<TimelineSection>) -> TemporalDiagram {
    TemporalDiagram {
        kind: TemporalKind::Timeline,
        title: None,
        body: TemporalBody::Timeline(TimelineDiagram {
            accessibility_title: Some("Event estimate".into()),
            accessibility_description: Some("Original labels and complete frames".into()),
            config: config(),
            direction: if td {
                TimelineDirection::TopDown
            } else {
                TimelineDirection::LeftRight
            },
            sections,
        }),
    }
}

fn expected(width: f64, height: f64, items: Vec<LayoutedTemporalItem>) -> LayoutedTemporalDiagram {
    LayoutedTemporalDiagram {
        width,
        height,
        accessibility_title: Some("Event estimate".into()),
        accessibility_description: Some("Original labels and complete frames".into()),
        timeline_config: Some(config()),
        interactions: Vec::new(),
        items,
    }
}

// These three closed products were derived from the specified represented
// arithmetic independently. The expected line count is supplied by each case;
// no estimator, label measurement or actual output constructs this oracle.
fn one(label: &str, td: bool, canvas: f64, lines: usize) {
    let (h, mid, total, end) = match (td, lines) {
        (true, 1) => (31.2, 65.6, 176.0, 90.0),
        (true, 2) => (50.4, 75.2, 178.4, 92.4),
        (true, 3) => (69.6, 84.8, 197.6, 111.6),
        (false, 1) => (31.2, 184.0, 297.2, 106.0),
        (false, 2) => (50.4, 184.0, 316.4, 106.0),
        (false, 3) => (69.6, 184.0, 335.6, 106.0),
        _ => panic!("closed event product table"),
    };
    let d = input(
        td,
        vec![TimelineSection {
            label: None,
            periods: vec![TimelinePeriod {
                label: "P".into(),
                events: vec![label.into()],
            }],
        }],
    );
    let original = d.clone();
    let spine = LayoutedTemporalItem::TimelineSpine {
        x1: if td { 206.0 } else { 301.0 },
        y1: if td { 66.0 } else { 106.0 },
        x2: if td { 206.0 } else { 301.0 },
        y2: end,
    };
    let mut items = Vec::new();
    if td {
        items.push(spine.clone());
    }
    items.extend([
        LayoutedTemporalItem::TimelinePeriod {
            x: if td { 234.0 } else { 206.0 },
            y: if td { 50.0 } else { 118.0 },
            width: if td { 180.0 } else { 190.0 },
            height: 48.0,
            label: "P".into(),
            events: Vec::new(),
            color_index: 0,
        },
        LayoutedTemporalItem::TimelineEventLink {
            x1: if td { 414.0 } else { 301.0 },
            y1: if td { 74.0 } else { 166.0 },
            x2: if td { 450.0 } else { 301.0 },
            y2: mid,
            color_index: 0,
        },
        LayoutedTemporalItem::TimelineEvent {
            x: if td { 450.0 } else { 206.0 },
            y: if td { 50.0 } else { 184.0 },
            width: if td {
                if canvas == 720.0 {
                    220.0
                } else {
                    160.0
                }
            } else {
                190.0
            },
            height: h,
            label: label.into(),
            color_index: 0,
        },
    ]);
    if !td {
        items.push(spine);
    }
    let oracle = expected(canvas, total, items);
    TIMELINE_WORK.with(|c| c.set((0, 0)));
    assert_eq!(try_layout_temporal_diagram(&d, canvas), Ok(oracle.clone()));
    assert_eq!(TIMELINE_WORK.with(|c| c.get()), (1, 1));
    assert_eq!(layout_temporal_diagram(&d, canvas), oracle);
    assert_eq!(d, original);
}

#[test]
fn event_capacity_boundaries_keep_complete_products() {
    // At size16, TD width160 holds19 scalars; LR width190 holds24.
    for (count, td_lines, lr_lines) in [
        (19, 1, 1),
        (20, 2, 1),
        (24, 2, 1),
        (25, 2, 2),
        (38, 2, 2),
        (39, 3, 2),
        (48, 3, 2),
        (49, 3, 3),
    ] {
        for td in [true, false] {
            one(
                &"x".repeat(count),
                td,
                640.0,
                if td { td_lines } else { lr_lines },
            );
            println!("timeline-event-height-capacity COMPLETE {count}:{td}");
        }
    }
}

#[test]
fn event_hard_lines_unicode_and_actual_width_keep_complete_products() {
    for (name, label, td_lines, lr_lines) in [
        ("empty", "".to_string(), 1, 1),
        ("terminator", "\n".into(), 1, 1),
        ("two-terminators", "\n\n".into(), 2, 2),
        ("crlf", "A\r\nB\r\n".into(), 2, 2),
        ("empty-middle", "A\n\nC\n".into(), 3, 3),
        ("unicode-scalars", "🦀".repeat(20), 2, 1),
        ("combining-scalars", "e\u{301}".repeat(20), 3, 2),
    ] {
        for td in [true, false] {
            one(&label, td, 640.0, if td { td_lines } else { lr_lines });
            println!("timeline-event-height-lines COMPLETE {name}:{td}");
        }
    }
    // Width220 admits28 scalars. This witnesses the actual TD width rather
    // than the160 minimum or the180 period-card width.
    for (count, lines) in [(20, 1), (28, 1), (29, 2)] {
        one(&"x".repeat(count), true, 720.0, lines);
        println!("timeline-event-height-width COMPLETE {count}");
    }
}

#[test]
fn unequal_event_estimates_keep_complete_cumulative_products() {
    for td in [true, false] {
        let labels = [
            "E".to_string(),
            "x".repeat(20),
            "A\n\nC".into(),
            String::new(),
        ];
        let d = input(
            td,
            vec![
                TimelineSection {
                    label: None,
                    periods: vec![
                        TimelinePeriod {
                            label: "P".into(),
                            events: labels.to_vec(),
                        },
                        TimelinePeriod {
                            label: "Q".into(),
                            events: Vec::new(),
                        },
                    ],
                },
                TimelineSection {
                    label: None,
                    periods: Vec::new(),
                },
                TimelineSection {
                    label: Some("S".into()),
                    periods: Vec::new(),
                },
            ],
        );
        let original = d.clone();
        let mut items = Vec::new();
        if td {
            items.push(LayoutedTemporalItem::TimelineSpine {
                x1: 206.0,
                y1: 66.0,
                x2: 206.0,
                y2: 372.4,
            });
        }
        items.push(LayoutedTemporalItem::TimelinePeriod {
            x: if td { 234.0 } else { 206.0 },
            y: if td { 50.0 } else { 118.0 },
            width: if td { 180.0 } else { 190.0 },
            height: 48.0,
            label: "P".into(),
            events: Vec::new(),
            color_index: 0,
        });
        let rows = if td {
            [
                (50.0, 31.2, 65.6),
                (89.2, 50.4, 114.4),
                (147.6, 69.6, 182.39999999999998),
                (225.2, 31.2, 240.79999999999998),
            ]
        } else {
            [
                (184.0, 31.2, 184.0),
                (223.2, 31.2, 223.2),
                (262.4, 69.6, 262.4),
                (340.0, 31.2, 340.0),
            ]
        };
        for (label, (y, height, link_y)) in labels.into_iter().zip(rows) {
            items.extend([
                LayoutedTemporalItem::TimelineEventLink {
                    x1: if td { 414.0 } else { 301.0 },
                    y1: if td { 74.0 } else { 166.0 },
                    x2: if td { 450.0 } else { 301.0 },
                    y2: link_y,
                    color_index: 0,
                },
                LayoutedTemporalItem::TimelineEvent {
                    x: if td { 450.0 } else { 206.0 },
                    y,
                    width: if td { 160.0 } else { 190.0 },
                    height,
                    label,
                    color_index: 0,
                },
            ]);
        }
        items.push(LayoutedTemporalItem::TimelinePeriod {
            x: if td { 234.0 } else { 420.0 },
            y: if td { 274.4 } else { 118.0 },
            width: if td { 180.0 } else { 190.0 },
            height: 48.0,
            label: "Q".into(),
            events: Vec::new(),
            color_index: 1,
        });
        if !td {
            items.push(LayoutedTemporalItem::TimelineSpine {
                x1: 301.0,
                y1: 106.0,
                x2: 515.0,
                y2: 106.0,
            });
        }
        items.push(LayoutedTemporalItem::TimelineSection {
            x: 100.0,
            y: if td { 360.4 } else { 537.2 },
            width: 136.0,
            height: 30.0,
            label: "S".into(),
            color_index: 2,
        });
        let oracle = expected(
            if td { 640.0 } else { 660.0 },
            if td { 458.4 } else { 721.2 },
            items,
        );
        assert_eq!(try_layout_temporal_diagram(&d, 640.0), Ok(oracle.clone()));
        assert_eq!(layout_temporal_diagram(&d, 640.0), oracle);
        assert_eq!(d, original);
        println!("timeline-event-height-stack COMPLETE {td}");
    }
}
