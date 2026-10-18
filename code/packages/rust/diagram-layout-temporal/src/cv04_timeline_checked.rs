//! Independent, complete typed layout oracles. Scene/palette transport is tested
//! separately; these tests establish only admission, layout, spine and work.
use super::*;
use diagram_ir::{TemporalKind, TimelineConfig, TimelinePeriod, TimelineSection};

fn config(padding: f64, left_margin: f64, disable_multicolor: bool) -> TimelineConfig {
    TimelineConfig {
        font_size: 16.0,
        font_family: "\"trebuchet ms\", verdana, arial, sans-serif".into(),
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
#[test]
fn td_inner_zero() {
    // Fixed180-wide cards intentionally retire the old canvas-coupled width.
    let oracle = expected(640.0, 176.0, config(50.0, 408.0, false), vec![
        LayoutedTemporalItem::TimelineSpine { x1: 564.0, y1: 66.0, x2: 564.0, y2: 90.0 },
        LayoutedTemporalItem::TimelinePeriod { x: 592.0, y: 50.0, width: 180.0, height: 48.0, label: "P0".into(), events: Vec::new(), color_index: 0 },
    ]);
    assert_eq!(try_layout_temporal_diagram(&input(50.0, 408.0, TimelineDirection::TopDown, false, false, false, 1), 640.0), Ok(oracle));
    println!("timeline-checked COMPLETE cell=td_inner_zero");
}
#[test]
fn td_inner_negative() {
    // Fixed180-wide cards intentionally retire the old canvas-coupled width.
    let oracle = expected(640.0, 176.0, config(50.0, 409.0, false), vec![
        LayoutedTemporalItem::TimelineSpine { x1: 565.0, y1: 66.0, x2: 565.0, y2: 90.0 },
        LayoutedTemporalItem::TimelinePeriod { x: 593.0, y: 50.0, width: 180.0, height: 48.0, label: "P0".into(), events: Vec::new(), color_index: 0 },
    ]);
    assert_eq!(try_layout_temporal_diagram(&input(50.0, 409.0, TimelineDirection::TopDown, false, false, false, 1), 640.0), Ok(oracle));
    println!("timeline-checked COMPLETE cell=td_inner_negative");
}
#[test]
fn td_large_padding() {
    // Fixed180-wide cards intentionally retire the old canvas-coupled width.
    let oracle = expected(640.0, 2076.0, config(1000.0, 50.0, false), vec![
        LayoutedTemporalItem::TimelineSpine { x1: 1156.0, y1: 1016.0, x2: 1156.0, y2: 1040.0 },
        LayoutedTemporalItem::TimelinePeriod { x: 1184.0, y: 1000.0, width: 180.0, height: 48.0, label: "P0".into(), events: Vec::new(), color_index: 0 },
    ]);
    assert_eq!(try_layout_temporal_diagram(&input(1000.0, 50.0, TimelineDirection::TopDown, false, false, false, 1), 640.0), Ok(oracle));
    println!("timeline-checked COMPLETE cell=td_large_padding");
}
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
        LayoutedTemporalItem::TimelinePeriod { x: 591.0, y: 50.0, width: 180.0, height: 48.0, label: "P0".into(), events: Vec::new(), color_index: 0 },
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
            // Retain every old matrix layout as a direct legacy validator control.
            let legacy = expected(width, height, config(50.0, 50.0, single), items.clone());
            assert_eq!(validate_timeline_layout(legacy.clone()), Ok(legacy));
            if direction == TimelineDirection::TopDown {
                for item in &mut items {
                    if let LayoutedTemporalItem::TimelinePeriod { width, .. } = item { *width = 180.0; }
                }
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

#[test]
fn nonempty_events_keep_complete_layouts() {
    let rows = [
        (TimelineDirection::TopDown, 1, 194.0, 66.0, 108.0, 234.0, 50.0, 382.0, 66.0),
        (TimelineDirection::TopDown, 2, 212.0, 66.0, 126.0, 234.0, 50.0, 382.0, 84.0),
        (TimelineDirection::LeftRight, 1, 252.0, 106.0, 106.0, 206.0, 118.0, 190.0, 66.0),
        (TimelineDirection::LeftRight, 2, 270.0, 106.0, 106.0, 206.0, 118.0, 190.0, 84.0),
    ];
    for (direction, count, height, sy1, sy2, x, y, width, card_height) in rows {
        let events = ["A", "B"][..count].iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let period = LayoutedTemporalItem::TimelinePeriod {
            x, y, width, height: card_height, label: "P0".into(), events: events.clone(), color_index: 0,
        };
        let (sx1, sx2) = if direction == TimelineDirection::TopDown { (206.0, 206.0) } else { (301.0, 301.0) };
        let spine = LayoutedTemporalItem::TimelineSpine { x1: sx1, y1: sy1, x2: sx2, y2: sy2 };
        let items = if direction == TimelineDirection::TopDown { vec![spine, period] } else { vec![period, spine] };
        let oracle = expected(640.0, height, config(50.0, 50.0, false), items);
        let mut d = input(50.0, 50.0, direction, false, false, false, 1);
        let TemporalBody::Timeline(t) = &mut d.body else { unreachable!() };
        t.sections[0].periods[0].events = events;
        // Preserve the complete old embedded-event layout as a direct control.
        assert_eq!(validate_timeline_layout(oracle.clone()), Ok(oracle));
        let (new_height, new_spine_end) = match (direction, count) {
            (TimelineDirection::TopDown, 1) => (176.0, 90.0),
            (TimelineDirection::TopDown, 2) => (196.0, 110.0),
            (TimelineDirection::LeftRight, 1) => (296.0, 106.0),
            (TimelineDirection::LeftRight, 2) => (334.0, 106.0),
            _ => unreachable!(),
        };
        let td = direction == TimelineDirection::TopDown;
        let new_spine = LayoutedTemporalItem::TimelineSpine {
            x1: sx1, y1: sy1, x2: sx2, y2: new_spine_end,
        };
        let mut new_items = Vec::new();
        if td { new_items.push(new_spine.clone()); }
        new_items.push(LayoutedTemporalItem::TimelinePeriod {
            x, y, width: if td { 180.0 } else { 190.0 }, height: 48.0,
            label: "P0".into(), events: Vec::new(), color_index: 0,
        });
        for (i, label) in ["A", "B"][..count].iter().enumerate() {
            let (ex, ey, ew, lx1, ly1, lx2, ly2) = if td {
                (450.0, 50.0 + i as f64 * 38.0, 160.0, 414.0, 74.0, 450.0, 65.0 + i as f64 * 38.0)
            } else {
                (206.0, 184.0 + i as f64 * 38.0, 190.0, 301.0, 166.0, 301.0, 184.0 + i as f64 * 38.0)
            };
            new_items.push(LayoutedTemporalItem::TimelineEventLink { x1: lx1, y1: ly1, x2: lx2, y2: ly2, color_index: 0 });
            new_items.push(LayoutedTemporalItem::TimelineEvent { x: ex, y: ey, width: ew, height: 30.0, label: (*label).into(), color_index: 0 });
        }
        if !td { new_items.push(new_spine); }
        let new_oracle = expected(640.0, new_height, config(50.0, 50.0, false), new_items);
        let mut patches = vec![(if td { 3 } else { 2 }, "height", 30.0, 31.2)];
        if td {
            patches.push((2, "y2", 65.0, 65.6));
            if count == 2 {
                patches.extend([(4,"y2",103.0,104.8),(5,"y",88.0,89.2),(5,"height",30.0,31.2),
                    (0,"y2",110.0,112.4),(usize::MAX,"height",196.0,198.4)]);
            }
        } else {
            patches.push((usize::MAX,"height",new_height,if count == 1 {297.2} else {336.4}));
            if count == 2 {
                patches.extend([(3,"y2",222.0,223.2),(4,"y",222.0,223.2),(4,"height",30.0,31.2)]);
            }
        }
        let new_oracle = super::cv04_timeline_event_height::migrate_declared(new_oracle, &patches);
        assert_eq!(try_layout_temporal_diagram(&d, 640.0), Ok(new_oracle));
    }
    println!("timeline-checked COMPLETE cell=nonempty_events_keep_complete_layouts");
}

#[test]
fn event_frame_extent_can_fail_after_outer_and_title_extents_pass() {
    // At this exact f64 boundary the outer card and its title still advance,
    // while the event frame's positive28 height is swallowed at2^58.
    let y = 2f64.powi(58) - 32.0;
    assert!(y + 66.0 > y);
    assert!((y + 8.0) + 22.0 > y + 8.0);
    assert_eq!((y + 32.0) + 28.0, y + 32.0);
    let mut d = input(y, 0.0, TimelineDirection::TopDown, false, false, false, 1);
    let TemporalBody::Timeline(t) = &mut d.body else { unreachable!() };
    t.sections[0].periods[0].events = vec!["A".into()];
    assert_eq!(try_layout_temporal_diagram(&d, 2f64.powi(60)), Err(TemporalLayoutError::TimelineGeometry));
    // The new separate Event frame passes here; its arrow collapses. Preserve
    // the retired embedded28-height proof explicitly instead of relabeling it.
    let legacy = expected(2f64.powi(60), ((y + 84.0) + 10.0) + y, config(y, 0.0, false), vec![
        LayoutedTemporalItem::TimelineSpine { x1: y + 106.0, y1: y + 16.0, x2: y + 106.0, y2: ((y + 84.0) + 10.0) - 36.0 },
        LayoutedTemporalItem::TimelinePeriod { x: (y + 106.0) + 28.0, y, width: (2f64.powi(60) - (y + 106.0)) - 52.0, height: 66.0, label: "P0".into(), events: vec!["A".into()], color_index: 0 },
    ]);
    assert_eq!(validate_timeline_layout(legacy), Err(TemporalLayoutError::TimelineGeometry));
    println!("timeline-checked COMPLETE cell=event_frame_extent_can_fail_after_outer_and_title_extents_pass");
}
