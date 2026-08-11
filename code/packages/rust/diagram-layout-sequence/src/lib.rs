//! Backend-neutral sequence diagram layout.

use std::collections::{HashMap, HashSet};

use diagram_ir::{
    LayoutedSequenceDiagram, LayoutedSequenceItem, SequenceBlockKind, SequenceDiagram,
    SequenceEvent, SequenceNotePlacement, SequenceTextWrap,
};

pub const VERSION: &str = "0.18.0";

const MARGIN: f64 = 28.0;
const HEADER_Y: f64 = 42.0;
const HEADER_H: f64 = 44.0;
const MIN_LANE_W: f64 = 150.0;
const EVENT_H: f64 = 58.0;
const NOTE_H: f64 = 38.0;
const ACTIVATION_W: f64 = 12.0;
const BLOCK_HEADER_H: f64 = 48.0;
const BLOCK_BRANCH_H: f64 = 48.0;
const BLOCK_BOTTOM_PAD: f64 = 14.0;
const BLOCK_AFTER_GAP: f64 = 12.0;
const BLOCK_INSET: f64 = 10.0;
const WRAPPED_TEXT_MAX_WIDTH: f64 = 240.0;

struct BlockFrameState {
    kind: SequenceBlockKind,
    label: String,
    label_height: f64,
    fill: Option<String>,
    depth: usize,
    x: f64,
    y: f64,
    width: f64,
}

/// Lay out an ordered sequence diagram. Participant order is semantic and is
/// therefore retained exactly rather than optimized by the layout engine.
pub fn layout_sequence_diagram(diagram: &SequenceDiagram) -> LayoutedSequenceDiagram {
    let lane_widths: Vec<f64> = diagram
        .participants
        .iter()
        .map(|participant| {
            if participant.label_wrap == SequenceTextWrap::Wrap {
                MIN_LANE_W
            } else {
                ((participant.label.text.chars().count() as f64 * 8.0) + 36.0).max(MIN_LANE_W)
            }
        })
        .collect();
    let group_labels: HashMap<String, (Option<String>, f64)> = diagram
        .participant_groups
        .iter()
        .map(|group| {
            let group_width = diagram
                .participants
                .iter()
                .zip(&lane_widths)
                .filter(|(participant, _)| {
                    participant.group_id.as_deref() == Some(group.id.as_str())
                })
                .map(|(_, width)| *width)
                .sum::<f64>();
            let label = group.label.as_ref().map(|label| {
                wrap_sequence_text(label, &group.label_wrap, (group_width - 24.0).max(1.0))
            });
            let label_height = label
                .as_deref()
                .map_or(0.0, |label| line_count(label) as f64 * 16.0);
            (group.id.clone(), (label, label_height))
        })
        .collect();
    let group_header_height = group_labels
        .values()
        .map(|(_, height)| *height)
        .fold(0.0, f64::max);
    let header_y = HEADER_Y
        + if diagram.participant_groups.is_empty() {
            0.0
        } else {
            group_header_height + 12.0
        };
    let participant_labels: Vec<String> = diagram
        .participants
        .iter()
        .zip(&lane_widths)
        .map(|(participant, lane_width)| {
            wrap_sequence_text(
                &participant.label.text,
                &participant.label_wrap,
                (*lane_width - 40.0).max(1.0),
            )
        })
        .collect();
    let header_height = participant_labels
        .iter()
        .map(|label| HEADER_H + 16.0 * (line_count(label) as f64 - 1.0))
        .fold(HEADER_H, f64::max);
    let width = lane_widths.iter().sum::<f64>() + MARGIN * 2.0;
    let created_participants: HashSet<&str> = diagram
        .events
        .iter()
        .filter_map(|event| match event {
            SequenceEvent::ParticipantCreated { participant } => Some(participant.as_str()),
            _ => None,
        })
        .collect();
    let mut centers = HashMap::new();
    let mut lifeline_starts = HashMap::new();
    let mut lifeline_ends = HashMap::new();
    let mut items = Vec::new();
    let mut x = MARGIN;
    let mut lane_lefts = Vec::with_capacity(lane_widths.len());

    for ((participant, lane_width), label) in diagram
        .participants
        .iter()
        .zip(&lane_widths)
        .zip(&participant_labels)
    {
        lane_lefts.push(x);
        let box_width = (*lane_width - 24.0).max(100.0);
        let center = x + *lane_width / 2.0;
        centers.insert(participant.id.clone(), center);
        if !created_participants.contains(participant.id.as_str()) {
            items.push(LayoutedSequenceItem::ParticipantBox {
                id: participant.id.clone(),
                label: label.clone(),
                label_height: line_count(label) as f64 * 16.0,
                kind: participant.kind.clone(),
                links: participant.links.clone(),
                properties: participant.properties.clone(),
                details_reference: participant.details_reference.clone(),
                x: center - box_width / 2.0,
                y: header_y,
                width: box_width,
                height: header_height,
            });
            lifeline_starts.insert(participant.id.clone(), header_y + header_height);
        }
        x += *lane_width;
    }

    let event_start = header_y + header_height + 36.0;
    let mut y = event_start;
    let mut activation_starts: HashMap<String, Vec<f64>> = HashMap::new();
    let has_auto_number_events = diagram
        .events
        .iter()
        .any(|event| matches!(event, SequenceEvent::AutoNumber { .. }));
    let mut auto_number_visible = if has_auto_number_events {
        false
    } else {
        diagram.auto_number
    };
    let mut message_number = if has_auto_number_events {
        1.0
    } else {
        diagram.auto_number_start
    };
    let mut message_number_step = if has_auto_number_events {
        1.0
    } else {
        diagram.auto_number_step
    };
    let mut block_stack: Vec<BlockFrameState> = Vec::new();

    for event in &diagram.events {
        match event {
            SequenceEvent::AutoNumber {
                visible,
                start,
                step,
            } => {
                auto_number_visible = *visible;
                if *visible {
                    message_number = start.unwrap_or(1.0);
                    message_number_step = step.unwrap_or(1.0);
                }
            }
            SequenceEvent::Message {
                from,
                to,
                label,
                wrap,
                line_style,
                arrowhead,
                bidirectional,
                central_connection,
                activate,
                deactivate,
            } => {
                let Some(&from_x) = centers.get(from) else {
                    continue;
                };
                let Some(&to_x) = centers.get(to) else {
                    continue;
                };
                let label = wrap_sequence_text(
                    label,
                    wrap,
                    (to_x - from_x).abs().clamp(80.0, WRAPPED_TEXT_MAX_WIDTH),
                );
                let label_height = 16.0 * label.lines().count().max(1) as f64;
                let message_y = y + label_height + 6.0;
                items.push(LayoutedSequenceItem::Message {
                    from_x,
                    to_x,
                    y: message_y,
                    label: label.clone(),
                    label_height,
                    line_style: line_style.clone(),
                    arrowhead: arrowhead.clone(),
                    bidirectional: *bidirectional,
                    central_connection: central_connection.clone(),
                    number: auto_number_visible.then_some(message_number),
                });
                if auto_number_visible {
                    message_number += message_number_step;
                }
                if *activate {
                    activation_starts
                        .entry(to.clone())
                        .or_default()
                        .push(message_y);
                }
                if *deactivate {
                    close_activation(&mut items, &mut activation_starts, from, from_x, message_y);
                }
                y = message_y + EVENT_H;
            }
            SequenceEvent::Activation {
                participant,
                active,
            } => {
                let Some(&participant_x) = centers.get(participant) else {
                    continue;
                };
                if *active {
                    activation_starts
                        .entry(participant.clone())
                        .or_default()
                        .push(y);
                } else {
                    close_activation(
                        &mut items,
                        &mut activation_starts,
                        participant,
                        participant_x,
                        y,
                    );
                }
                y += EVENT_H / 2.0;
            }
            SequenceEvent::ParticipantCreated { participant } => {
                if let (Some(&center), Some(definition)) = (
                    centers.get(participant),
                    diagram
                        .participants
                        .iter()
                        .find(|item| item.id == *participant),
                ) {
                    let lane_index = diagram
                        .participants
                        .iter()
                        .position(|item| item.id == *participant)
                        .unwrap_or(0);
                    let box_width = (lane_widths[lane_index] - 24.0).max(100.0);
                    let created_header_height = HEADER_H
                        + 16.0 * (line_count(&participant_labels[lane_index]) as f64 - 1.0);
                    items.push(LayoutedSequenceItem::ParticipantBox {
                        id: definition.id.clone(),
                        label: participant_labels[lane_index].clone(),
                        label_height: line_count(&participant_labels[lane_index]) as f64 * 16.0,
                        kind: definition.kind.clone(),
                        links: definition.links.clone(),
                        properties: definition.properties.clone(),
                        details_reference: definition.details_reference.clone(),
                        x: center - box_width / 2.0,
                        y,
                        width: box_width,
                        height: created_header_height,
                    });
                    lifeline_starts.insert(participant.clone(), y + created_header_height);
                    y += (created_header_height + 14.0).max(EVENT_H);
                } else {
                    y += EVENT_H;
                }
            }
            SequenceEvent::ParticipantDestroyed { participant } => {
                if let Some(&center) = centers.get(participant) {
                    items.push(LayoutedSequenceItem::Destruction {
                        participant: participant.clone(),
                        x: center,
                        y,
                    });
                    lifeline_ends.insert(participant.clone(), y);
                }
                y += EVENT_H / 2.0;
            }
            SequenceEvent::Note {
                participants,
                placement,
                text,
                wrap,
            } => {
                let participant_centers: Vec<f64> = participants
                    .iter()
                    .filter_map(|id| centers.get(id).copied())
                    .collect();
                if participant_centers.is_empty() {
                    continue;
                }
                let min_x = participant_centers
                    .iter()
                    .copied()
                    .fold(f64::INFINITY, f64::min);
                let max_x = participant_centers
                    .iter()
                    .copied()
                    .fold(f64::NEG_INFINITY, f64::max);
                let text = wrap_sequence_text(text, wrap, WRAPPED_TEXT_MAX_WIDTH);
                let line_count = text.lines().count().max(1);
                let longest_line = text
                    .lines()
                    .map(|line| line.chars().count())
                    .max()
                    .unwrap_or(0);
                let note_width = ((longest_line as f64 * 7.5) + 28.0)
                    .max(100.0)
                    .min((width - MARGIN * 2.0).max(100.0));
                let note_x = match placement {
                    SequenceNotePlacement::LeftOf => min_x - note_width - 14.0,
                    SequenceNotePlacement::RightOf => max_x + 14.0,
                    SequenceNotePlacement::Over => (min_x + max_x - note_width) / 2.0,
                }
                .clamp(MARGIN, (width - MARGIN - note_width).max(MARGIN));
                items.push(LayoutedSequenceItem::Note {
                    x: note_x,
                    y: y - 10.0,
                    width: note_width,
                    height: NOTE_H + 16.0 * line_count.saturating_sub(1) as f64,
                    text,
                });
                y += NOTE_H + 16.0 * line_count.saturating_sub(1) as f64 + 20.0;
            }
            SequenceEvent::BlockStart {
                kind,
                label,
                wrap,
                fill,
            } => {
                let depth = block_stack.len();
                let x = MARGIN + depth as f64 * BLOCK_INSET;
                let frame_width = (width - x * 2.0).max(120.0);
                let label = wrap_sequence_text(label, wrap, frame_width - 16.0);
                let label_height = 16.0 * label.lines().count().max(1) as f64;
                block_stack.push(BlockFrameState {
                    kind: kind.clone(),
                    label,
                    label_height,
                    fill: fill.clone(),
                    depth,
                    x,
                    y,
                    width: frame_width,
                });
                if kind != &SequenceBlockKind::Rect {
                    y += BLOCK_HEADER_H + label_height - 16.0;
                }
            }
            SequenceEvent::BlockBranch { label, wrap } => {
                if let Some(frame) = block_stack.last() {
                    let label = wrap_sequence_text(label, wrap, frame.width - 16.0);
                    let label_height = 16.0 * label.lines().count().max(1) as f64;
                    items.push(LayoutedSequenceItem::BlockDivider {
                        label,
                        label_height,
                        x: frame.x,
                        y,
                        width: frame.width,
                    });
                    y += BLOCK_BRANCH_H + label_height - 16.0;
                }
            }
            SequenceEvent::BlockEnd { kind } => {
                if let Some(frame) = block_stack.pop() {
                    debug_assert_eq!(&frame.kind, kind);
                    y += BLOCK_BOTTOM_PAD;
                    let frame_height = y - frame.y;
                    items.push(LayoutedSequenceItem::BlockFrame {
                        kind: frame.kind,
                        label: frame.label,
                        label_height: frame.label_height,
                        fill: frame.fill,
                        depth: frame.depth,
                        x: frame.x,
                        y: frame.y,
                        width: frame.width,
                        height: frame_height,
                    });
                    y += BLOCK_AFTER_GAP;
                }
            }
        }
    }

    let height = (y + 36.0).max(180.0);
    for group in &diagram.participant_groups {
        let indexes: Vec<usize> = diagram
            .participants
            .iter()
            .enumerate()
            .filter_map(|(index, participant)| {
                (participant.group_id.as_deref() == Some(group.id.as_str())).then_some(index)
            })
            .collect();
        if let (Some(first), Some(last)) = (indexes.first(), indexes.last()) {
            let group_x = lane_lefts[*first] + 4.0;
            let group_right = lane_lefts[*last] + lane_widths[*last] - 4.0;
            let (label, label_height) = group_labels.get(&group.id).cloned().unwrap_or((None, 0.0));
            items.push(LayoutedSequenceItem::ParticipantGroup {
                id: group.id.clone(),
                label,
                label_height,
                fill: group.fill.clone(),
                x: group_x,
                y: HEADER_Y - 6.0,
                width: group_right - group_x,
                height: height - 22.0,
            });
        }
    }
    for participant in &diagram.participants {
        if let Some(&center) = centers.get(&participant.id) {
            items.push(LayoutedSequenceItem::Lifeline {
                participant: participant.id.clone(),
                x: center,
                y1: lifeline_starts
                    .get(&participant.id)
                    .copied()
                    .unwrap_or(header_y + header_height),
                y2: lifeline_ends
                    .get(&participant.id)
                    .copied()
                    .unwrap_or(height - 20.0),
            });
            if let Some(starts) = activation_starts.remove(&participant.id) {
                for start in starts {
                    items.push(LayoutedSequenceItem::Activation {
                        participant: participant.id.clone(),
                        x: center - ACTIVATION_W / 2.0,
                        y1: start,
                        y2: height - 20.0,
                    });
                }
            }
        }
    }

    LayoutedSequenceDiagram {
        width,
        height,
        title: diagram.title.clone(),
        accessibility_title: diagram.accessibility_title.clone(),
        accessibility_description: diagram.accessibility_description.clone(),
        items,
    }
}

fn wrap_sequence_text(text: &str, wrap: &SequenceTextWrap, max_width: f64) -> String {
    if wrap != &SequenceTextWrap::Wrap {
        return text.to_string();
    }
    let max_chars = (max_width / 7.5).floor().max(1.0) as usize;
    text.lines()
        .flat_map(|line| wrap_sequence_line(line, max_chars))
        .collect::<Vec<_>>()
        .join("\n")
}

fn line_count(text: &str) -> usize {
    text.lines().count().max(1)
}

fn wrap_sequence_line(line: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in line.split_whitespace() {
        let next_len =
            current.chars().count() + usize::from(!current.is_empty()) + word.chars().count();
        if !current.is_empty() && next_len > max_chars {
            lines.push(current);
            current = word.to_string();
        } else {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }
    }
    if !current.is_empty() || lines.is_empty() {
        lines.push(current);
    }
    lines
}

fn close_activation(
    items: &mut Vec<LayoutedSequenceItem>,
    starts: &mut HashMap<String, Vec<f64>>,
    participant: &str,
    center: f64,
    y: f64,
) {
    if let Some(start) = starts.get_mut(participant).and_then(Vec::pop) {
        items.push(LayoutedSequenceItem::Activation {
            participant: participant.to_string(),
            x: center - ACTIVATION_W / 2.0,
            y1: start,
            y2: y,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use diagram_ir::{
        DiagramLabel, SequenceArrowhead, SequenceCentralConnection, SequenceLineStyle,
        SequenceParticipant, SequenceParticipantKind,
    };

    fn participant(id: &str) -> SequenceParticipant {
        SequenceParticipant {
            id: id.into(),
            label: DiagramLabel::new(id),
            label_wrap: SequenceTextWrap::Default,
            kind: SequenceParticipantKind::Participant,
            style: None,
            group_id: None,
            links: vec![],
            properties: vec![],
            details_reference: None,
        }
    }

    #[test]
    fn lays_out_messages_and_lifelines() {
        let diagram = SequenceDiagram {
            title: None,
            accessibility_title: None,
            accessibility_description: None,
            auto_number: true,
            auto_number_start: 10.5,
            auto_number_step: 2.25,
            participants: vec![participant("Alice"), participant("Bob")],
            participant_groups: vec![],
            events: vec![SequenceEvent::Message {
                from: "Alice".into(),
                to: "Bob".into(),
                label: "Hello".into(),
                wrap: SequenceTextWrap::Default,
                line_style: SequenceLineStyle::Solid,
                arrowhead: SequenceArrowhead::Filled,
                bidirectional: false,
                central_connection: SequenceCentralConnection::None,
                activate: false,
                deactivate: false,
            }],
        };
        let layout = layout_sequence_diagram(&diagram);
        assert_eq!(
            layout
                .items
                .iter()
                .filter(|item| matches!(item, LayoutedSequenceItem::ParticipantBox { .. }))
                .count(),
            2
        );
        assert_eq!(
            layout
                .items
                .iter()
                .filter(|item| matches!(item, LayoutedSequenceItem::Lifeline { .. }))
                .count(),
            2
        );
        assert!(layout.items.iter().any(|item| matches!(
            item,
            LayoutedSequenceItem::Message {
                number: Some(10.5),
                ..
            }
        )));
    }

    #[test]
    fn applies_ordered_autonumber_toggles_and_resets() {
        let message = |label: &str| SequenceEvent::Message {
            from: "Alice".into(),
            to: "Bob".into(),
            label: label.into(),
            wrap: SequenceTextWrap::Default,
            line_style: SequenceLineStyle::Solid,
            arrowhead: SequenceArrowhead::Filled,
            bidirectional: false,
            central_connection: SequenceCentralConnection::None,
            activate: false,
            deactivate: false,
        };
        let diagram = SequenceDiagram {
            title: None,
            accessibility_title: None,
            accessibility_description: None,
            auto_number: true,
            auto_number_start: 20.0,
            auto_number_step: 5.0,
            participants: vec![participant("Alice"), participant("Bob")],
            participant_groups: vec![],
            events: vec![
                SequenceEvent::AutoNumber {
                    visible: true,
                    start: None,
                    step: None,
                },
                message("One"),
                SequenceEvent::AutoNumber {
                    visible: false,
                    start: None,
                    step: None,
                },
                message("Hidden"),
                SequenceEvent::AutoNumber {
                    visible: true,
                    start: Some(20.0),
                    step: Some(5.0),
                },
                message("Twenty"),
                message("Twenty-five"),
            ],
        };
        let numbers: Vec<_> = layout_sequence_diagram(&diagram)
            .items
            .iter()
            .filter_map(|item| match item {
                LayoutedSequenceItem::Message { number, .. } => Some(*number),
                _ => None,
            })
            .collect();
        assert_eq!(numbers, vec![Some(1.0), None, Some(20.0), Some(25.0)]);
    }

    #[test]
    fn wraps_sequence_text_only_when_requested() {
        let text = "A deliberately long message that must wrap across several native lines";
        let wrapped = wrap_sequence_text(text, &SequenceTextWrap::Wrap, 90.0);
        assert!(wrapped.contains('\n'));
        assert_eq!(
            wrap_sequence_text(text, &SequenceTextWrap::NoWrap, 90.0),
            text
        );
    }

    #[test]
    fn wrapped_participant_alias_expands_all_headers_and_lifelines() {
        let mut alice = participant("Alice");
        alice.label =
            DiagramLabel::new("A deliberately detailed public application programming interface");
        alice.label_wrap = SequenceTextWrap::Wrap;
        let diagram = SequenceDiagram {
            title: None,
            accessibility_title: None,
            accessibility_description: None,
            auto_number: false,
            auto_number_start: 1.0,
            auto_number_step: 1.0,
            participants: vec![alice, participant("Bob")],
            participant_groups: vec![],
            events: vec![],
        };
        let layout = layout_sequence_diagram(&diagram);
        let boxes: Vec<_> = layout
            .items
            .iter()
            .filter_map(|item| match item {
                LayoutedSequenceItem::ParticipantBox {
                    label,
                    label_height,
                    height,
                    ..
                } => Some((label, *label_height, *height)),
                _ => None,
            })
            .collect();
        assert!(boxes[0].0.contains('\n'));
        assert!(boxes[0].1 > 16.0);
        assert_eq!(boxes[0].2, boxes[1].2);
        assert!(boxes[0].2 > HEADER_H);
    }

    #[test]
    fn preserves_half_arrow_semantics() {
        let diagram = SequenceDiagram {
            title: None,
            accessibility_title: None,
            accessibility_description: None,
            auto_number: false,
            auto_number_start: 1.0,
            auto_number_step: 1.0,
            participants: vec![participant("Alice"), participant("Bob")],
            participant_groups: vec![],
            events: vec![SequenceEvent::Message {
                from: "Alice".into(),
                to: "Bob".into(),
                label: "Half".into(),
                wrap: SequenceTextWrap::Default,
                line_style: SequenceLineStyle::Dotted,
                arrowhead: SequenceArrowhead::ReverseStickBottom,
                bidirectional: false,
                central_connection: SequenceCentralConnection::Both,
                activate: false,
                deactivate: false,
            }],
        };
        let layout = layout_sequence_diagram(&diagram);
        assert!(layout.items.iter().any(|item| matches!(
            item,
            LayoutedSequenceItem::Message {
                line_style: SequenceLineStyle::Dotted,
                arrowhead: SequenceArrowhead::ReverseStickBottom,
                central_connection: SequenceCentralConnection::Both,
                ..
            }
        )));
    }

    #[test]
    fn closes_activation_on_deactivation_event() {
        let diagram = SequenceDiagram {
            title: None,
            accessibility_title: None,
            accessibility_description: None,
            auto_number: false,
            auto_number_start: 1.0,
            auto_number_step: 1.0,
            participants: vec![participant("Bob")],
            participant_groups: vec![],
            events: vec![
                SequenceEvent::Activation {
                    participant: "Bob".into(),
                    active: true,
                },
                SequenceEvent::Activation {
                    participant: "Bob".into(),
                    active: false,
                },
            ],
        };
        let layout = layout_sequence_diagram(&diagram);
        assert!(layout.items.iter().any(
            |item| matches!(item, LayoutedSequenceItem::Activation { y1, y2, .. } if y2 > y1)
        ));
    }

    #[test]
    fn lays_out_nested_block_frames_and_branch_dividers() {
        let diagram = SequenceDiagram {
            title: None,
            accessibility_title: None,
            accessibility_description: None,
            auto_number: false,
            auto_number_start: 1.0,
            auto_number_step: 1.0,
            participants: vec![participant("Alice"), participant("Bob")],
            participant_groups: vec![],
            events: vec![
                SequenceEvent::BlockStart {
                    kind: SequenceBlockKind::Alt,
                    label: "Ready for a deliberately detailed transfer acceptance path".into(),
                    wrap: SequenceTextWrap::Wrap,
                    fill: None,
                },
                SequenceEvent::BlockStart {
                    kind: SequenceBlockKind::Loop,
                    label: "Retry".into(),
                    wrap: SequenceTextWrap::Default,
                    fill: None,
                },
                SequenceEvent::Message {
                    from: "Alice".into(),
                    to: "Bob".into(),
                    label: "Ping".into(),
                    wrap: SequenceTextWrap::Default,
                    line_style: SequenceLineStyle::Solid,
                    arrowhead: SequenceArrowhead::Filled,
                    bidirectional: false,
                    central_connection: SequenceCentralConnection::None,
                    activate: false,
                    deactivate: false,
                },
                SequenceEvent::BlockEnd {
                    kind: SequenceBlockKind::Loop,
                },
                SequenceEvent::BlockBranch {
                    label: "Fallback after a deliberately detailed rejection path".into(),
                    wrap: SequenceTextWrap::Wrap,
                },
                SequenceEvent::BlockEnd {
                    kind: SequenceBlockKind::Alt,
                },
            ],
        };
        let layout = layout_sequence_diagram(&diagram);
        let frames: Vec<_> = layout
            .items
            .iter()
            .filter_map(|item| match item {
                LayoutedSequenceItem::BlockFrame {
                    depth, x, height, ..
                } => Some((*depth, *x, *height)),
                _ => None,
            })
            .collect();
        assert_eq!(frames.len(), 2);
        assert!(frames.iter().any(|(depth, _, _)| *depth == 1));
        assert!(frames.iter().all(|(_, _, height)| *height > BLOCK_HEADER_H));
        assert!(layout.items.iter().any(|item| matches!(
            item,
            LayoutedSequenceItem::BlockFrame { label, label_height, .. }
                if label.contains('\n') && *label_height > 16.0
        )));
        assert!(layout.items.iter().any(|item| matches!(
            item,
            LayoutedSequenceItem::BlockDivider { label, label_height, .. }
                if label.contains('\n') && *label_height > 16.0
        )));
    }

    #[test]
    fn created_participant_has_bounded_lifeline() {
        let diagram = SequenceDiagram {
            title: None,
            accessibility_title: None,
            accessibility_description: None,
            auto_number: false,
            auto_number_start: 1.0,
            auto_number_step: 1.0,
            participants: vec![participant("Alice"), participant("Worker")],
            participant_groups: vec![],
            events: vec![
                SequenceEvent::Message {
                    from: "Alice".into(),
                    to: "Alice".into(),
                    label: "Start".into(),
                    wrap: SequenceTextWrap::Default,
                    line_style: SequenceLineStyle::Solid,
                    arrowhead: SequenceArrowhead::Filled,
                    bidirectional: false,
                    central_connection: SequenceCentralConnection::None,
                    activate: false,
                    deactivate: false,
                },
                SequenceEvent::ParticipantCreated {
                    participant: "Worker".into(),
                },
                SequenceEvent::Message {
                    from: "Alice".into(),
                    to: "Worker".into(),
                    label: "Work".into(),
                    wrap: SequenceTextWrap::Default,
                    line_style: SequenceLineStyle::Solid,
                    arrowhead: SequenceArrowhead::Filled,
                    bidirectional: false,
                    central_connection: SequenceCentralConnection::None,
                    activate: false,
                    deactivate: false,
                },
                SequenceEvent::ParticipantDestroyed {
                    participant: "Worker".into(),
                },
            ],
        };
        let layout = layout_sequence_diagram(&diagram);
        let worker_box_y = layout
            .items
            .iter()
            .find_map(|item| match item {
                LayoutedSequenceItem::ParticipantBox { id, y, .. } if id == "Worker" => Some(*y),
                _ => None,
            })
            .unwrap();
        let (lifeline_y1, lifeline_y2) = layout
            .items
            .iter()
            .find_map(|item| match item {
                LayoutedSequenceItem::Lifeline {
                    participant,
                    y1,
                    y2,
                    ..
                } if participant == "Worker" => Some((*y1, *y2)),
                _ => None,
            })
            .unwrap();
        let destruction_y = layout
            .items
            .iter()
            .find_map(|item| match item {
                LayoutedSequenceItem::Destruction { participant, y, .. }
                    if participant == "Worker" =>
                {
                    Some(*y)
                }
                _ => None,
            })
            .unwrap();
        assert_eq!(lifeline_y1, worker_box_y + HEADER_H);
        assert_eq!(lifeline_y2, destruction_y);
        assert!(worker_box_y > HEADER_Y);
    }

    #[test]
    fn participant_group_encloses_only_member_lanes() {
        let mut alice = participant("Alice");
        alice.group_id = Some("client".into());
        let mut bob = participant("Bob");
        bob.group_id = Some("client".into());
        let diagram = SequenceDiagram {
            title: None,
            accessibility_title: None,
            accessibility_description: None,
            auto_number: false,
            auto_number_start: 1.0,
            auto_number_step: 1.0,
            participants: vec![alice, bob, participant("Database")],
            participant_groups: vec![diagram_ir::SequenceParticipantGroup {
                id: "client".into(),
                label: Some("Client tier".into()),
                label_wrap: SequenceTextWrap::Default,
                fill: Some("aqua".into()),
            }],
            events: vec![],
        };
        let layout = layout_sequence_diagram(&diagram);
        let (group_x, group_width) = layout
            .items
            .iter()
            .find_map(|item| match item {
                LayoutedSequenceItem::ParticipantGroup { x, width, .. } => Some((*x, *width)),
                _ => None,
            })
            .unwrap();
        let database_x = layout
            .items
            .iter()
            .find_map(|item| match item {
                LayoutedSequenceItem::ParticipantBox { id, x, .. } if id == "Database" => Some(*x),
                _ => None,
            })
            .unwrap();
        assert!(group_x + group_width < database_x);
    }

    #[test]
    fn wrapped_participant_group_label_reserves_header_space() {
        let mut alice = participant("Alice");
        alice.group_id = Some("client".into());
        let diagram = SequenceDiagram {
            title: None,
            accessibility_title: None,
            accessibility_description: None,
            auto_number: false,
            auto_number_start: 1.0,
            auto_number_step: 1.0,
            participants: vec![alice],
            participant_groups: vec![diagram_ir::SequenceParticipantGroup {
                id: "client".into(),
                label: Some("A deliberately detailed client application tier".into()),
                label_wrap: SequenceTextWrap::Wrap,
                fill: None,
            }],
            events: vec![],
        };
        let layout = layout_sequence_diagram(&diagram);
        let (label, label_height) = layout
            .items
            .iter()
            .find_map(|item| match item {
                LayoutedSequenceItem::ParticipantGroup {
                    label,
                    label_height,
                    ..
                } => Some((label.as_deref().unwrap(), *label_height)),
                _ => None,
            })
            .unwrap();
        let participant_y = layout
            .items
            .iter()
            .find_map(|item| match item {
                LayoutedSequenceItem::ParticipantBox { y, .. } => Some(*y),
                _ => None,
            })
            .unwrap();
        assert!(label.contains('\n'));
        assert!(label_height > 16.0);
        assert_eq!(participant_y, HEADER_Y + label_height + 12.0);
    }
}
