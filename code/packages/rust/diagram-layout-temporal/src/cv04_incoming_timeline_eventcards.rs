//! Exact incoming semantic shape test; checked new-item admission remains pending.
use super::*;
use diagram_ir::{TemporalKind, TimelinePeriod, TimelineSection};

    #[test]
    fn timeline_layout_preserves_sections_periods_and_events() {
        let diagram = TemporalDiagram {
            kind: TemporalKind::Timeline,
            title: Some("Product history".into()),
            body: TemporalBody::Timeline(TimelineDiagram {
                accessibility_title: Some("Release history".into()),
                accessibility_description: Some("Milestones by year".into()),
                config: Default::default(),
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
        let layout = layout_temporal_diagram(&diagram, 640.0);
        assert_eq!(layout.accessibility_title.as_deref(), Some("Release history"));
        assert!(layout.items.iter().any(|item| matches!(item,
            LayoutedTemporalItem::TimelineSection { label, .. } if label == "Foundation"
        )));
        assert!(layout.items.iter().any(|item| matches!(item,
            LayoutedTemporalItem::TimelinePeriod { label, events, .. }
                if label == "2024" && events.is_empty()
        )));
        assert_eq!(layout.items.iter().filter(|item| matches!(
            item, LayoutedTemporalItem::TimelineEvent { .. }
        )).count(), 2);
        assert_eq!(layout.items.iter().filter(|item| matches!(
            item, LayoutedTemporalItem::TimelineEventLink { .. }
        )).count(), 2);
        assert!(layout.items.iter().any(|item| matches!(
            item,
            LayoutedTemporalItem::TimelineSpine { .. }
        )));
    }
