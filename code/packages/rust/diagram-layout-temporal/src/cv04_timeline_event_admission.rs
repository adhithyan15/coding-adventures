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
    for (x, y, w, h, accepted) in [
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
    ] {
        let expected = layout(vec![event(x, y, w, h)]);
        let actual = validate_timeline_layout(expected.clone());
        if accepted {
            assert_eq!(actual, Ok(expected));
        } else {
            assert_eq!(actual, Err(TemporalLayoutError::TimelineGeometry));
        }
        println!("timeline-event-frame COMPLETE {x:?}:{y:?}:{w:?}:{h:?}:{accepted}");
    }
}

#[test]
fn event_link_certifies_actual_arrow_triangle() {
    for (x1, y1, x2, y2, accepted) in [
        (20.0, 20.0, 56.0, 11.0, true),
        (20.0, 20.0, 20.0, 38.0, true),
        (56.0, 11.0, 20.0, 20.0, true),
        (20.0, 20.0, 20.0, 20.0, false),
        (-f64::MAX, 0.0, f64::MAX, 1.0, false),
        (0.0, 0.0, 1e200, 1.0, false),
        (0.0, 0.0, 1e154, 1e154, false),
        (0.0, 0.0, f64::MIN_POSITIVE, 0.0, false),
        (0.0, 0.0, f64::INFINITY, 1.0, false),
        (0.0, 0.0, 1.0, f64::NAN, false),
        // Nonzero represented delta, but the triangle's corners coincide.
        (
            2f64.powi(56),
            2f64.powi(56),
            2f64.powi(56),
            2f64.powi(56) + 32.0,
            false,
        ),
    ] {
        let expected = layout(vec![link(x1, y1, x2, y2)]);
        let actual = validate_timeline_layout(expected.clone());
        if accepted {
            assert_eq!(actual, Ok(expected));
        } else {
            assert_eq!(actual, Err(TemporalLayoutError::TimelineGeometry));
        }
        println!("timeline-event-arrow COMPLETE {x1:?}:{y1:?}:{x2:?}:{y2:?}:{accepted}");
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
    for exponent in [56, 57] {
        assert_eq!(
            try_layout_temporal_diagram(
                &input(TimelineDirection::LeftRight, 2f64.powi(exponent)),
                2f64.powi(60)
            ),
            Err(TemporalLayoutError::TimelineGeometry)
        );
    }
}
