//! Closed literal complete products distinguish cumulative from indexed f64 positions.
use super::*;
use diagram_ir::{TemporalKind, TimelineConfig, TimelinePeriod, TimelineSection};

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

#[test]
fn cumulative_positions_keep_whole_fractional_and_default_products() {
    // This table is derived from the specified addition sequence outside the
    // implementation, including each rounded cursor and all later row advances.
    for (td, size, ch, eh, qy, sy, total, spine_end, event_ys, link_ys) in [
        (
            true,
            16.0,
            48.0,
            30.0,
            212.0,
            298.0,
            396.0,
            310.0,
            [50.0, 88.0, 126.0, 164.0],
            [65.0, 103.0, 141.0, 179.0],
        ),
        (
            true,
            16.01,
            48.03,
            30.018750000000004,
            212.07500000000005,
            298.105,
            396.12375000000003,
            310.12375000000003,
            [
                50.0,
                88.01875000000001,
                126.03750000000002,
                164.05625000000003,
            ],
            [
                65.009375,
                103.02812500000002,
                141.04687500000003,
                179.06562500000004,
            ],
        ),
        (
            false,
            16.0,
            48.0,
            30.0,
            118.0,
            494.0,
            678.0,
            106.0,
            [184.0, 222.0, 260.0, 298.0],
            [184.0, 222.0, 260.0, 298.0],
        ),
        (
            false,
            16.01,
            48.03,
            30.018750000000004,
            118.0,
            494.135,
            678.165,
            106.0,
            [184.03, 222.04875, 260.0675, 298.08625],
            [184.03, 222.04875, 260.0675, 298.08625],
        ),
    ] {
        let cfg = config(size);
        let input = TemporalDiagram {
            kind: TemporalKind::Timeline,
            title: None,
            body: TemporalBody::Timeline(TimelineDiagram {
                accessibility_title: Some("Cumulative history".into()),
                accessibility_description: Some("Four events followed by empty rows".into()),
                config: cfg.clone(),
                direction: if td {
                    TimelineDirection::TopDown
                } else {
                    TimelineDirection::LeftRight
                },
                sections: vec![
                    TimelineSection {
                        label: None,
                        periods: vec![
                            TimelinePeriod {
                                label: "P".into(),
                                events: vec!["E0".into(), "E1".into(), "E2".into(), "E3".into()],
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
            }),
        };
        let original = input.clone();
        let mut items = Vec::new();
        if td {
            items.push(LayoutedTemporalItem::TimelineSpine {
                x1: 206.0,
                y1: 66.0,
                x2: 206.0,
                y2: spine_end,
            });
        }
        items.push(LayoutedTemporalItem::TimelinePeriod {
            x: if td { 234.0 } else { 206.0 },
            y: if td { 50.0 } else { 118.0 },
            width: if td { 180.0 } else { 190.0 },
            height: ch,
            label: "P".into(),
            events: Vec::new(),
            color_index: 0,
        });
        for (i, label) in ["E0", "E1", "E2", "E3"].into_iter().enumerate() {
            items.push(LayoutedTemporalItem::TimelineEventLink {
                x1: if td { 414.0 } else { 301.0 },
                y1: if td { 50.0 + ch / 2.0 } else { 118.0 + ch },
                x2: if td { 450.0 } else { 301.0 },
                y2: link_ys[i],
                color_index: 0,
            });
            items.push(LayoutedTemporalItem::TimelineEvent {
                x: if td { 450.0 } else { 206.0 },
                y: event_ys[i],
                width: if td { 160.0 } else { 190.0 },
                height: eh,
                label: label.into(),
                color_index: 0,
            });
        }
        items.push(LayoutedTemporalItem::TimelinePeriod {
            x: if td { 234.0 } else { 420.0 },
            y: qy,
            width: if td { 180.0 } else { 190.0 },
            height: ch,
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
            y: sy,
            width: 136.0,
            height: eh,
            label: "S".into(),
            color_index: 2,
        });
        let expected = LayoutedTemporalDiagram {
            width: if td { 640.0 } else { 660.0 },
            height: total,
            accessibility_title: Some("Cumulative history".into()),
            accessibility_description: Some("Four events followed by empty rows".into()),
            timeline_config: Some(cfg),
            interactions: Vec::new(),
            items,
        };
        assert_eq!(
            try_layout_temporal_diagram(&input, 640.0),
            Ok(expected.clone())
        );
        assert_eq!(layout_temporal_diagram(&input, 640.0), expected);
        assert_eq!(input, original);
        println!("timeline-cumulative-product COMPLETE {td}:{size}");
    }
}

#[test]
fn rounded_cursor_span_keeps_complete_large_coordinate_product() {
    // ULP4 makes the final +38 cursor advance40. This full accepted witness
    // separates cumulative span40 from an indexed one-event span38.
    let p = 18_014_398_509_481_984.0;
    let mut cfg = config(16.0);
    cfg.padding = p;
    cfg.left_margin = 0.0;
    let input = TemporalDiagram {
        kind: TemporalKind::Timeline,
        title: None,
        body: TemporalBody::Timeline(TimelineDiagram {
            accessibility_title: None,
            accessibility_description: None,
            config: cfg.clone(),
            direction: TimelineDirection::LeftRight,
            sections: vec![TimelineSection {
                label: None,
                periods: vec![TimelinePeriod {
                    label: "P".into(),
                    events: vec!["E".into()],
                }],
            }],
        }),
    };
    let expected = LayoutedTemporalDiagram {
        width: 1_152_921_504_606_846_976.0,
        height: 36_028_797_018_964_168.0,
        accessibility_title: None,
        accessibility_description: None,
        timeline_config: Some(cfg),
        interactions: Vec::new(),
        items: vec![
            LayoutedTemporalItem::TimelinePeriod {
                x: p + 104.0,
                y: p + 68.0,
                width: 190.0,
                height: 48.0,
                label: "P".into(),
                events: Vec::new(),
                color_index: 0,
            },
            LayoutedTemporalItem::TimelineEventLink {
                x1: p + 200.0,
                y1: p + 116.0,
                x2: p + 200.0,
                y2: p + 136.0,
                color_index: 0,
            },
            LayoutedTemporalItem::TimelineEvent {
                x: p + 104.0,
                y: p + 136.0,
                width: 190.0,
                height: 30.0,
                label: "E".into(),
                color_index: 0,
            },
            LayoutedTemporalItem::TimelineSpine {
                x1: p + 200.0,
                y1: p + 56.0,
                x2: p + 200.0,
                y2: p + 56.0,
            },
        ],
    };
    // Preserve the complete old shape as an independent direct-validator
    // control; it is no longer the constructor's expected row association.
    let mut old = expected.clone();
    old.height = 36_028_797_018_964_160.0;
    assert_eq!(validate_timeline_layout(old.clone()), Ok(old));
    assert_eq!(
        try_layout_temporal_diagram(&input, 1_152_921_504_606_846_976.0),
        Ok(expected.clone())
    );
    assert_eq!(
        layout_temporal_diagram(&input, 1_152_921_504_606_846_976.0),
        expected
    );
    println!("timeline-cumulative-rounded COMPLETE exponent=54");
}
