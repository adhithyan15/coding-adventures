//! Backend-neutral sequence diagram layout.

use std::collections::{HashMap, HashSet};

use diagram_ir::{
    LayoutedSequenceDiagram, LayoutedSequenceItem, SequenceBlockKind, SequenceDiagram,
    SequenceEvent, SequenceNotePlacement,
};

pub const VERSION: &str = "0.12.0";

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

struct BlockFrameState {
    kind: SequenceBlockKind,
    label: String,
    fill: Option<String>,
    depth: usize,
    x: f64,
    y: f64,
    width: f64,
}

/// Lay out an ordered sequence diagram. Participant order is semantic and is
/// therefore retained exactly rather than optimized by the layout engine.
pub fn layout_sequence_diagram(diagram: &SequenceDiagram) -> LayoutedSequenceDiagram {
    let header_y = HEADER_Y
        + if diagram.participant_groups.is_empty() {
            0.0
        } else {
            28.0
        };
    let lane_widths: Vec<f64> = diagram
        .participants
        .iter()
        .map(|participant| {
            ((participant.label.text.chars().count() as f64 * 8.0) + 36.0).max(MIN_LANE_W)
        })
        .collect();
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

    for (participant, lane_width) in diagram.participants.iter().zip(&lane_widths) {
        lane_lefts.push(x);
        let box_width = (*lane_width - 24.0).max(100.0);
        let center = x + *lane_width / 2.0;
        centers.insert(participant.id.clone(), center);
        if !created_participants.contains(participant.id.as_str()) {
            items.push(LayoutedSequenceItem::ParticipantBox {
                id: participant.id.clone(),
                label: participant.label.text.clone(),
                kind: participant.kind.clone(),
                links: participant.links.clone(),
                properties: participant.properties.clone(),
                details_reference: participant.details_reference.clone(),
                x: center - box_width / 2.0,
                y: header_y,
                width: box_width,
                height: HEADER_H,
            });
            lifeline_starts.insert(participant.id.clone(), header_y + HEADER_H);
        }
        x += *lane_width;
    }

    let event_start = header_y + HEADER_H + 36.0;
    let mut y = event_start;
    let mut activation_starts: HashMap<String, Vec<f64>> = HashMap::new();
    let mut message_number = diagram.auto_number_start;
    let mut block_stack: Vec<BlockFrameState> = Vec::new();

    for event in &diagram.events {
        match event {
            SequenceEvent::Message {
                from,
                to,
                label,
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
                items.push(LayoutedSequenceItem::Message {
                    from_x,
                    to_x,
                    y,
                    label: label.clone(),
                    line_style: line_style.clone(),
                    arrowhead: arrowhead.clone(),
                    bidirectional: *bidirectional,
                    central_connection: central_connection.clone(),
                    number: diagram.auto_number.then_some(message_number),
                });
                message_number += diagram.auto_number_step;
                if *activate {
                    activation_starts.entry(to.clone()).or_default().push(y);
                }
                if *deactivate {
                    close_activation(&mut items, &mut activation_starts, from, from_x, y);
                }
                y += EVENT_H;
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
                    items.push(LayoutedSequenceItem::ParticipantBox {
                        id: definition.id.clone(),
                        label: definition.label.text.clone(),
                        kind: definition.kind.clone(),
                        links: definition.links.clone(),
                        properties: definition.properties.clone(),
                        details_reference: definition.details_reference.clone(),
                        x: center - box_width / 2.0,
                        y,
                        width: box_width,
                        height: HEADER_H,
                    });
                    lifeline_starts.insert(participant.clone(), y + HEADER_H);
                }
                y += EVENT_H;
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
                let note_width = ((text.chars().count() as f64 * 7.5) + 28.0)
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
                    height: NOTE_H,
                    text: text.clone(),
                });
                y += NOTE_H + 20.0;
            }
            SequenceEvent::BlockStart { kind, label, fill } => {
                let depth = block_stack.len();
                let x = MARGIN + depth as f64 * BLOCK_INSET;
                block_stack.push(BlockFrameState {
                    kind: kind.clone(),
                    label: label.clone(),
                    fill: fill.clone(),
                    depth,
                    x,
                    y,
                    width: (width - x * 2.0).max(120.0),
                });
                if kind != &SequenceBlockKind::Rect {
                    y += BLOCK_HEADER_H;
                }
            }
            SequenceEvent::BlockBranch { label } => {
                if let Some(frame) = block_stack.last() {
                    items.push(LayoutedSequenceItem::BlockDivider {
                        label: label.clone(),
                        x: frame.x,
                        y,
                        width: frame.width,
                    });
                }
                y += BLOCK_BRANCH_H;
            }
            SequenceEvent::BlockEnd { kind } => {
                if let Some(frame) = block_stack.pop() {
                    debug_assert_eq!(&frame.kind, kind);
                    y += BLOCK_BOTTOM_PAD;
                    let frame_height = y - frame.y;
                    items.push(LayoutedSequenceItem::BlockFrame {
                        kind: frame.kind,
                        label: frame.label,
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
            items.push(LayoutedSequenceItem::ParticipantGroup {
                id: group.id.clone(),
                label: group.label.clone(),
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
                    .unwrap_or(header_y + HEADER_H),
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
                    label: "Ready".into(),
                    fill: None,
                },
                SequenceEvent::BlockStart {
                    kind: SequenceBlockKind::Loop,
                    label: "Retry".into(),
                    fill: None,
                },
                SequenceEvent::Message {
                    from: "Alice".into(),
                    to: "Bob".into(),
                    label: "Ping".into(),
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
                    label: "Fallback".into(),
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
        assert!(layout.items.iter().any(|item| matches!(item, LayoutedSequenceItem::BlockDivider { label, .. } if label == "Fallback")));
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
}
