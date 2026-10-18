//! Whole literal products for scalar estimates; no actual-output normalization.
use super::*;
use diagram_ir::{TemporalKind, TimelineConfig, TimelinePeriod, TimelineSection};

#[test]
fn private_width_and_estimate_contract_keeps_exact_results() {
    for (name, width) in [
        ("nan", f64::NAN),
        ("infinity", f64::INFINITY),
        ("negative-infinity", f64::NEG_INFINITY),
        ("negative", -1.0),
        ("zero", 0.0),
        ("below-inner", f64::from_bits(20.0f64.to_bits() - 1)),
        ("zero-inner", 20.0),
    ] {
        assert_eq!(
            timeline_period_height("A", width, 16.0, 48.0),
            Err(TemporalLayoutError::TimelineGeometry)
        );
        println!("timeline-period-height-private COMPLETE {name}");
    }
    assert_eq!(
        timeline_period_height("", f64::from_bits(20.0f64.to_bits() + 1), 16.0, 48.0),
        Ok(48.0)
    );
    println!("timeline-period-height-private COMPLETE positive-inner");
    assert_eq!(
        timeline_period_height("A\nB\nC", 180.0, 16.0, 48.0),
        Ok(69.6)
    );
    println!("timeline-period-height-private COMPLETE growing");
    assert_eq!(
        timeline_period_height("x", 180.0, f64::from_bits(1), 48.0),
        Ok(48.0)
    );
    println!("timeline-period-height-private COMPLETE saturated-subnormal");
}

fn config(size: f64) -> TimelineConfig {
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

fn input(label: &str, size: f64, td: bool) -> TemporalDiagram {
    TemporalDiagram {
        kind: TemporalKind::Timeline,
        title: None,
        body: TemporalBody::Timeline(TimelineDiagram {
            accessibility_title: None,
            accessibility_description: None,
            config: config(size),
            direction: if td {
                TimelineDirection::TopDown
            } else {
                TimelineDirection::LeftRight
            },
            sections: vec![TimelineSection {
                label: None,
                periods: vec![TimelinePeriod {
                    label: label.into(),
                    events: Vec::new(),
                }],
            }],
        }),
    }
}

// The three declared whole outputs are selected by expected line count,
// never by the actual layout, estimator or the label's current character count.
fn one(label: &str, size: f64, td: bool, lines: usize) -> LayoutedTemporalDiagram {
    let (ch, height, end) = match (td, lines) {
        (true, 1) => (48.0, 176.0, 90.0),
        (true, 2) => (50.4, 178.4, 92.4),
        (true, 3) => (69.6, 197.6, 111.6),
        (false, 1) => (48.0, 234.0, 106.0),
        (false, 2) => (50.4, 236.4, 106.0),
        (false, 3) => (69.6, 255.6, 106.0),
        _ => panic!("closed literal output table"),
    };
    let period = LayoutedTemporalItem::TimelinePeriod {
        x: if td { 234.0 } else { 206.0 },
        y: if td { 50.0 } else { 118.0 },
        width: if td { 180.0 } else { 190.0 },
        height: ch,
        label: label.into(),
        events: Vec::new(),
        color_index: 0,
    };
    let spine = LayoutedTemporalItem::TimelineSpine {
        x1: if td { 206.0 } else { 301.0 },
        y1: if td { 66.0 } else { 106.0 },
        x2: if td { 206.0 } else { 301.0 },
        y2: end,
    };
    LayoutedTemporalDiagram {
        width: 640.0,
        height,
        accessibility_title: None,
        accessibility_description: None,
        timeline_config: Some(config(size)),
        interactions: Vec::new(),
        items: if td {
            vec![spine, period]
        } else {
            vec![period, spine]
        },
    }
}

fn check(label: &str, size: f64, td: bool, lines: usize) {
    let expected = one(label, size, td, lines);
    let d = input(label, size, td);
    TIMELINE_WORK.with(|c| c.set((0, 0)));
    assert_eq!(try_layout_temporal_diagram(&d, 640.0), Ok(expected.clone()));
    assert_eq!(TIMELINE_WORK.with(|c| c.get()), (1, 1));
    assert_eq!(layout_temporal_diagram(&d, 640.0), expected);
}

#[test]
fn scalar_capacity_transitions_keep_complete_products() {
    // Capacities are22/24, not a backend's measured19/20 scalar glyph capacity.
    for (count, td_lines, lr_lines) in [
        (22, 1, 1),
        (23, 2, 1),
        (24, 2, 1),
        (25, 2, 2),
        (44, 2, 2),
        (45, 3, 2),
        (48, 3, 2),
        (49, 3, 3),
    ] {
        for td in [true, false] {
            check(
                &"x".repeat(count),
                16.0,
                td,
                if td { td_lines } else { lr_lines },
            );
            println!("timeline-period-height-capacity COMPLETE {count}:{td}");
        }
    }
}

#[test]
fn hard_lines_unicode_and_subnormal_keep_complete_products() {
    let cases = [
        ("empty", String::new(), 1, 1),
        ("one-terminator", "\n".into(), 1, 1),
        ("two-terminators", "\n\n".into(), 2, 2),
        ("crlf", "A\r\nB\r\n".into(), 2, 2),
        ("trailing-three", "A\nB\nC\n".into(), 3, 3),
        ("empty-middle", "A\n\nC".into(), 3, 3),
        ("spaces", " ".repeat(49), 3, 3),
        ("wide", "界".repeat(23), 2, 1),
        ("combining", "e\u{301}".repeat(12), 2, 1),
        ("unicode-separator", "a\u{2028}b".into(), 1, 1),
    ];
    for (name, label, td_lines, lr_lines) in cases {
        for td in [true, false] {
            check(&label, 16.0, td, if td { td_lines } else { lr_lines });
            println!("timeline-period-height-lines COMPLETE {name}:{td}");
        }
    }
    for td in [true, false] {
        check(&"x".repeat(49), f64::from_bits(1), td, 1);
        println!("timeline-period-height-subnormal COMPLETE {td}");
    }
}

#[test]
fn unequal_periods_preserve_complete_indexed_events_and_empty_rows() {
    let long = "x".repeat(49);
    for td in [true, false] {
        let mut d = input(&long, 16.0, td);
        d.title = Some("T".into());
        let TemporalBody::Timeline(t) = &mut d.body else {
            unreachable!()
        };
        t.accessibility_title = Some("Accessible".into());
        t.accessibility_description = Some("Description".into());
        t.sections[0].label = Some("S".into());
        t.sections[0].periods[0].events = vec!["E".into(), "F".into()];
        t.sections[0].periods.push(TimelinePeriod {
            label: "Q".into(),
            events: vec!["G".into()],
        });
        t.sections.push(TimelineSection {
            label: None,
            periods: Vec::new(),
        });
        let mut items = vec![LayoutedTemporalItem::TemporalTitle {
            x: 50.0,
            y: 50.0,
            width: 540.0,
            height: 40.0,
            label: "T".into(),
        }];
        if td {
            items.push(LayoutedTemporalItem::TimelineSpine {
                x1: 206.0,
                y1: 118.0,
                x2: 206.0,
                y2: 277.6,
            });
        }
        items.push(LayoutedTemporalItem::TimelineSection {
            x: 100.0,
            y: 102.0,
            width: 136.0,
            height: 30.0,
            label: "S".into(),
            color_index: 0,
        });
        items.push(LayoutedTemporalItem::TimelinePeriod {
            x: if td { 234.0 } else { 206.0 },
            y: if td { 140.0 } else { 170.0 },
            width: if td { 180.0 } else { 190.0 },
            height: 69.6,
            label: long.clone(),
            events: Vec::new(),
            color_index: 0,
        });
        for (label, td_y, lr_y) in [("E", 140.0, 257.6), ("F", 178.0, 295.6)] {
            items.push(LayoutedTemporalItem::TimelineEventLink {
                x1: if td { 414.0 } else { 301.0 },
                y1: if td { 174.8 } else { 239.6 },
                x2: if td { 450.0 } else { 301.0 },
                y2: if td { td_y + 15.0 } else { lr_y },
                color_index: 0,
            });
            items.push(LayoutedTemporalItem::TimelineEvent {
                x: if td { 450.0 } else { 206.0 },
                y: if td { td_y } else { lr_y },
                width: if td { 160.0 } else { 190.0 },
                height: 30.0,
                label: label.into(),
                color_index: 0,
            });
        }
        items.push(LayoutedTemporalItem::TimelinePeriod {
            x: if td { 234.0 } else { 420.0 },
            y: if td { 227.6 } else { 170.0 },
            width: if td { 180.0 } else { 190.0 },
            height: 48.0,
            label: "Q".into(),
            events: Vec::new(),
            color_index: 1,
        });
        items.push(LayoutedTemporalItem::TimelineEventLink {
            x1: if td { 414.0 } else { 515.0 },
            y1: if td { 251.6 } else { 218.0 },
            x2: if td { 450.0 } else { 515.0 },
            y2: if td { 242.6 } else { 236.0 },
            color_index: 1,
        });
        items.push(LayoutedTemporalItem::TimelineEvent {
            x: if td { 450.0 } else { 420.0 },
            y: if td { 227.6 } else { 236.0 },
            width: if td { 160.0 } else { 190.0 },
            height: 30.0,
            label: "G".into(),
            color_index: 1,
        });
        if !td {
            items.push(LayoutedTemporalItem::TimelineSpine {
                x1: 301.0,
                y1: 158.0,
                x2: 515.0,
                y2: 158.0,
            });
        }
        let expected = LayoutedTemporalDiagram {
            width: if td { 640.0 } else { 660.0 },
            height: if td { 363.6 } else { 541.6 },
            accessibility_title: Some("Accessible".into()),
            accessibility_description: Some("Description".into()),
            timeline_config: Some(config(16.0)),
            interactions: Vec::new(),
            items,
        };
        let patches = if td {
            vec![
                (usize::MAX,"height",363.6,364.4),(1,"y2",277.6,278.4),
                (4,"y2",155.0,155.6),(5,"height",30.0,31.2),
                (6,"y2",193.0,194.79999999999998),(7,"y",178.0,179.2),(7,"height",30.0,31.2),
                (8,"y",227.6,228.39999999999998),(9,"y2",242.6,243.99999999999997),
                (10,"y",227.6,228.39999999999998),(10,"height",30.0,31.2),
            ]
        } else {
            vec![
                (usize::MAX,"height",541.6,544.0),(4,"height",30.0,31.2),
                (5,"y2",295.6,296.8),(6,"y",295.6,296.8),(6,"height",30.0,31.2),
                (9,"height",30.0,31.2),
            ]
        };
        let expected = super::cv04_timeline_event_height::migrate_declared(expected, &patches);
        TIMELINE_WORK.with(|c| c.set((0, 0)));
        assert_eq!(try_layout_temporal_diagram(&d, 640.0), Ok(expected.clone()));
        assert_eq!(TIMELINE_WORK.with(|c| c.get()), (1, 2));
        assert_eq!(layout_temporal_diagram(&d, 640.0), expected);
        println!("timeline-period-height-multi COMPLETE {td}");
    }
}

#[test]
fn adjacent_derived_overflow_keeps_complete_result_and_eager_priority() {
    for (bits, accepted) in [(0x7fd1c71c71c71c71, true), (0x7fd1c71c71c71c72, false)] {
        let size = f64::from_bits(bits);
        assert!((size * 3.0).is_finite());
        assert_eq!(((3.0 * size) * 1.2).is_finite(), accepted);
        let mut d = input("A\nB\nC", size, true);
        let TemporalBody::Timeline(t) = &mut d.body else {
            unreachable!()
        };
        t.config.padding = 0.0;
        t.config.left_margin = 0.0;
        let mut cfg = config(size);
        cfg.padding = 0.0;
        cfg.left_margin = 0.0;
        let expected = LayoutedTemporalDiagram {
            width: 640.0,
            height: f64::MAX,
            accessibility_title: None,
            accessibility_description: None,
            timeline_config: Some(cfg),
            interactions: Vec::new(),
            items: vec![
                LayoutedTemporalItem::TimelineSpine {
                    x1: 106.0,
                    y1: 16.0,
                    x2: 106.0,
                    y2: f64::MAX,
                },
                LayoutedTemporalItem::TimelinePeriod {
                    x: 134.0,
                    y: 0.0,
                    width: 180.0,
                    height: f64::MAX,
                    label: "A\nB\nC".into(),
                    events: Vec::new(),
                    color_index: 0,
                },
            ],
        };
        assert_eq!(
            try_layout_temporal_diagram(&d, 640.0),
            if accepted {
                Ok(expected)
            } else {
                Err(TemporalLayoutError::TimelineGeometry)
            }
        );
        println!("timeline-period-height-boundary COMPLETE {bits:x}:{accepted}");
    }
    // A still larger planned role refuses before predicates/visits, even empty.
    let mut d = input("A\nB\nC", f64::MAX, true);
    let TemporalBody::Timeline(t) = &mut d.body else {
        unreachable!()
    };
    t.sections.clear();
    TIMELINE_WORK.with(|c| c.set((0, 0)));
    assert_eq!(
        try_layout_temporal_diagram(&d, 640.0),
        Err(TemporalLayoutError::TimelineGeometry)
    );
    assert_eq!(TIMELINE_WORK.with(|c| c.get()), (0, 0));
    assert_eq!(
        try_layout_temporal_diagram(&d, f64::NAN),
        Err(TemporalLayoutError::TimelineCanvas)
    );
    println!("timeline-period-height-priority COMPLETE eager-empty");
}
