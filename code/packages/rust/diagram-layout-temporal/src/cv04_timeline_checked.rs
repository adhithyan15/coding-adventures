//! Independent, complete typed layout oracles. Scene/palette transport is tested
//! separately; these tests establish only admission, layout, spine and work.
use super::*;
use diagram_ir::{TemporalKind, TimelineConfig, TimelinePeriod, TimelineSection};

fn config(padding: f64, left_margin: f64, disable_multicolor: bool) -> TimelineConfig {
    TimelineConfig {
        padding, left_margin, use_max_width: true, disable_multicolor,
        section_colors: ["#dbeafe", "#dcfce7", "#fef3c7", "#fae8ff", "#ffe4e6", "#cffafe"].map(str::to_string).to_vec(),
        section_label_colors: ["#0f172a"; 6].map(str::to_string).to_vec(),
        section_inverse_colors: ["#2563eb", "#16a34a", "#d97706", "#a21caf", "#e11d48", "#0891b2"].map(str::to_string).to_vec(),
    }
}

fn input(padding: f64, margin: f64, direction: TimelineDirection, title: bool, named: bool, single_color: bool, periods: usize) -> TemporalDiagram {
    TemporalDiagram {
        kind: TemporalKind::Timeline,
        title: title.then(|| "History".into()),
        body: TemporalBody::Timeline(TimelineDiagram {
            accessibility_title: Some("Accessible history".into()),
            accessibility_description: Some("Ordered milestones".into()),
            config: config(padding, margin, single_color), direction,
            sections: vec![TimelineSection {
                label: named.then(|| "Foundation".into()),
                periods: (0..periods).map(|n| TimelinePeriod {
                    label: format!("P{n}"), events: Vec::new(),
                }).collect(),
            }],
        }),
    }
}

fn empty(padding: f64, margin: f64) -> TemporalDiagram {
    let mut d = input(padding, margin, TimelineDirection::TopDown, false, false, false, 0);
    let TemporalBody::Timeline(t) = &mut d.body else { unreachable!() };
    t.sections.clear();
    d
}

fn expected(width: f64, height: f64, cfg: TimelineConfig, items: Vec<LayoutedTemporalItem>) -> LayoutedTemporalDiagram {
    LayoutedTemporalDiagram {
        width, height, accessibility_title: Some("Accessible history".into()),
        accessibility_description: Some("Ordered milestones".into()),
        timeline_config: Some(cfg), interactions: Vec::new(), items,
    }
}

macro_rules! refusal {
    ($name:ident, $d:expr, $cw:expr, $error:ident) => {
        #[test]
        fn $name() {
            assert_eq!(try_layout_temporal_diagram(&$d, $cw), Err(TemporalLayoutError::$error));
            println!("timeline-checked COMPLETE cell={}", stringify!($name));
        }
    };
}

// Each priority case deliberately contains lower-priority invalid values too.
refusal!(canvas_nan_priority, empty(f64::NAN, f64::NAN), f64::NAN, TimelineCanvas);
refusal!(canvas_infinity_priority, empty(-1.0, -1.0), f64::INFINITY, TimelineCanvas);
refusal!(canvas_negative_infinity, empty(50.0, 50.0), f64::NEG_INFINITY, TimelineCanvas);
refusal!(padding_nan_priority, empty(f64::NAN, f64::NAN), 640.0, TimelinePadding);
refusal!(padding_infinity, empty(f64::INFINITY, -1.0), 640.0, TimelinePadding);
refusal!(padding_negative_infinity, empty(f64::NEG_INFINITY, 50.0), 640.0, TimelinePadding);
refusal!(padding_negative, empty(-1.0, f64::NAN), 640.0, TimelinePadding);
refusal!(margin_nan, empty(50.0, f64::NAN), 640.0, TimelineLeftMargin);
refusal!(margin_infinity, empty(50.0, f64::INFINITY), 640.0, TimelineLeftMargin);
refusal!(margin_negative_infinity, empty(50.0, f64::NEG_INFINITY), 640.0, TimelineLeftMargin);
refusal!(margin_negative, empty(50.0, -1.0), 640.0, TimelineLeftMargin);
refusal!(empty_origin_overflow, empty(1e308, 1e308), 640.0, TimelineGeometry);
refusal!(empty_height_overflow, empty(1e308, 0.0), 640.0, TimelineGeometry);
refusal!(td_inner_zero, input(50.0, 408.0, TimelineDirection::TopDown, false, false, false, 1), 640.0, TimelineGeometry);
refusal!(td_inner_negative, input(50.0, 409.0, TimelineDirection::TopDown, false, false, false, 1), 640.0, TimelineGeometry);
refusal!(td_large_padding, input(1000.0, 50.0, TimelineDirection::TopDown, false, false, false, 1), 640.0, TimelineGeometry);
refusal!(lr_large_padding_title, input(1000.0, 50.0, TimelineDirection::LeftRight, true, false, false, 1), 640.0, TimelineGeometry);
refusal!(title_inner_zero, input(312.0, 0.0, TimelineDirection::LeftRight, true, false, false, 1), 640.0, TimelineGeometry);
refusal!(title_inner_negative, input(313.0, 0.0, TimelineDirection::LeftRight, true, false, false, 1), 640.0, TimelineGeometry);
refusal!(lr_swallowed_period_extent, input(50.0, 2f64.powi(65), TimelineDirection::LeftRight, false, false, false, 1), 640.0, TimelineGeometry);
refusal!(swallowed_section_extent, input(50.0, 2f64.powi(65), TimelineDirection::LeftRight, false, true, false, 0), 640.0, TimelineGeometry);
refusal!(huge_finite_td_margin, input(50.0, 1e308, TimelineDirection::TopDown, false, false, false, 1), 640.0, TimelineGeometry);

#[test]
#[should_panic(expected = "unsupported temporal geometry")]
fn explicit_compatibility_panic() {
    layout_temporal_diagram(&empty(f64::NAN, 50.0), 640.0);
}

#[test]
fn empty_zero_height_and_finite_canvas_normalization() {
    // No minimum-height policy or blanket finite-canvas lower bound is added.
    let oracle = expected(560.0, 0.0, config(0.0, 0.0, false), Vec::new());
    for cw in [0.0, -640.0] {
        assert_eq!(try_layout_temporal_diagram(&empty(0.0, 0.0), cw), Ok(oracle.clone()));
    }
    println!("timeline-checked COMPLETE cell=empty_zero_height_and_finite_canvas_normalization");
}

#[test]
fn td_inner_one_complete() {
    let oracle = expected(640.0, 176.0, config(50.0, 407.0, false), vec![
        LayoutedTemporalItem::TimelineSpine { x1: 563.0, y1: 66.0, x2: 563.0, y2: 90.0 },
        LayoutedTemporalItem::TimelinePeriod { x: 591.0, y: 50.0, width: 25.0, height: 48.0, label: "P0".into(), events: Vec::new(), color_index: 0 },
    ]);
    assert_eq!(try_layout_temporal_diagram(&input(50.0, 407.0, TimelineDirection::TopDown, false, false, false, 1), 640.0), Ok(oracle));
    println!("timeline-checked COMPLETE cell=td_inner_one_complete");
}

#[test]
fn lr_large_margin_complete() {
    let oracle = expected(1396.0, 234.0, config(50.0, 1000.0, false), vec![
        LayoutedTemporalItem::TimelinePeriod { x: 1156.0, y: 118.0, width: 190.0, height: 48.0, label: "P0".into(), events: Vec::new(), color_index: 0 },
        LayoutedTemporalItem::TimelineSpine { x1: 1251.0, y1: 106.0, x2: 1251.0, y2: 106.0 },
    ]);
    assert_eq!(try_layout_temporal_diagram(&input(50.0, 1000.0, TimelineDirection::LeftRight, false, false, false, 1), 640.0), Ok(oracle));
    println!("timeline-checked COMPLETE cell=lr_large_margin_complete");
}

#[test]
fn lr_large_padding_complete() {
    let oracle = expected(2346.0, 2134.0, config(1000.0, 50.0, false), vec![
        LayoutedTemporalItem::TimelinePeriod { x: 1156.0, y: 1068.0, width: 190.0, height: 48.0, label: "P0".into(), events: Vec::new(), color_index: 0 },
        LayoutedTemporalItem::TimelineSpine { x1: 1251.0, y1: 1056.0, x2: 1251.0, y2: 1056.0 },
    ]);
    assert_eq!(try_layout_temporal_diagram(&input(1000.0, 50.0, TimelineDirection::LeftRight, false, false, false, 1), 640.0), Ok(oracle));
    println!("timeline-checked COMPLETE cell=lr_large_padding_complete");
}

#[test]
fn complete_direction_title_section_boolean_matrix() {
    // Declare every coordinate independently of the production layout before
    // calling it. Header/title insertion and spine order are part of equality.
    let rows = [
        (TimelineDirection::TopDown, false, false, 50.0, 0.0, 50.0, 66.0, 156.0, 242.0, 640.0),
        (TimelineDirection::TopDown, false, true, 50.0, 0.0, 88.0, 66.0, 194.0, 280.0, 640.0),
        (TimelineDirection::TopDown, true, false, 102.0, 0.0, 102.0, 118.0, 208.0, 294.0, 640.0),
        (TimelineDirection::TopDown, true, true, 102.0, 0.0, 140.0, 118.0, 246.0, 332.0, 640.0),
        (TimelineDirection::LeftRight, false, false, 50.0, 118.0, 0.0, 106.0, 106.0, 234.0, 660.0),
        (TimelineDirection::LeftRight, false, true, 50.0, 118.0, 0.0, 106.0, 106.0, 234.0, 660.0),
        (TimelineDirection::LeftRight, true, false, 102.0, 170.0, 0.0, 158.0, 158.0, 286.0, 660.0),
        (TimelineDirection::LeftRight, true, true, 102.0, 170.0, 0.0, 158.0, 158.0, 286.0, 660.0),
    ];
    for (row, &(direction, title, named, section_y, lr_y, td_y, sy1, sy2, height, width)) in rows.iter().enumerate() {
        for single in [false, true] {
            let mut items = Vec::new();
            if title {
                items.push(LayoutedTemporalItem::TemporalTitle { x: 50.0, y: 50.0, width: 540.0, height: 40.0, label: "History".into() });
            }
            if direction == TimelineDirection::TopDown {
                items.push(LayoutedTemporalItem::TimelineSpine { x1: 206.0, y1: sy1, x2: 206.0, y2: sy2 });
            }
            if named {
                items.push(LayoutedTemporalItem::TimelineSection { x: 100.0, y: section_y, width: 136.0, height: 30.0, label: "Foundation".into(), color_index: 0 });
            }
            let coordinates = if direction == TimelineDirection::TopDown { [(234.0, td_y, 382.0), (234.0, td_y + 66.0, 382.0)] } else { [(206.0, lr_y, 190.0), (420.0, lr_y, 190.0)] };
            for (n, &(x, y, w)) in coordinates.iter().enumerate() {
                items.push(LayoutedTemporalItem::TimelinePeriod { x, y, width: w, height: 48.0, label: format!("P{n}"), events: Vec::new(), color_index: if single && !named { 0 } else { n } });
            }
            if direction == TimelineDirection::LeftRight {
                items.push(LayoutedTemporalItem::TimelineSpine { x1: 301.0, y1: sy1, x2: 515.0, y2: sy2 });
            }
            let oracle = expected(width, height, config(50.0, 50.0, single), items);
            let actual = try_layout_temporal_diagram(&input(50.0, 50.0, direction, title, named, single, 2), 640.0);
            assert_eq!(actual, Ok(oracle), "row={row} single={single}");
            println!("timeline-checked COMPLETE matrix={row}:{single}");
        }
    }
}

#[test]
fn actual_predicate_and_section_visits_are_linear() {
    for (direction, height) in [(TimelineDirection::TopDown, 1380.0), (TimelineDirection::LeftRight, 17252.0)] {
        let mut d = input(50.0, 50.0, direction, false, false, true, 0);
        let TemporalBody::Timeline(t) = &mut d.body else { unreachable!() };
        t.sections = vec![TimelineSection { label: None, periods: Vec::new() }; 128];
        let oracle = expected(640.0, height, config(50.0, 50.0, true), Vec::new());
        TIMELINE_WORK.with(|c| c.set((0, 0)));
        assert_eq!(try_layout_temporal_diagram(&d, 640.0), Ok(oracle));
        assert_eq!(TIMELINE_WORK.with(|c| c.get()), (128, 128));
    }
    println!("timeline-checked COMPLETE cell=actual_predicate_and_section_visits_are_linear");
}
