//! The new scalar is transported/admitted while every old frame stays literal.
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
                    let expected = declared_size_layout(size, td, titled, named);
                    assert_eq!(
                        try_layout_temporal_diagram(
                            &size_fixture(size, direction, titled, named),
                            640.0
                        ),
                        Ok(expected),
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
                    Ok(expected),
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
