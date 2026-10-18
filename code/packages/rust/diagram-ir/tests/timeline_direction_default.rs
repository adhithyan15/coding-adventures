//! Typed callers and source ingestion share the same Timeline default.
use diagram_ir::TimelineDirection;

#[test]
fn timeline_direction_default_is_left_right() {
    assert_eq!(TimelineDirection::default(), TimelineDirection::LeftRight);
}
