//! Backend-neutral sequence diagram layout.

use std::collections::{HashMap, HashSet};

use diagram_ir::{
    LayoutedSequenceDiagram, LayoutedSequenceItem, SequenceBlockKind, SequenceCentralConnection,
    SequenceDiagram, SequenceEvent, SequenceNotePlacement, SequenceParticipantKind,
    SequenceTextWrap,
};

pub const VERSION: &str = "0.27.0";

const MARGIN: f64 = 28.0;
const HEADER_Y: f64 = 42.0;
const HEADER_H: f64 = 44.0;
const MIN_LANE_W: f64 = 150.0;
const EVENT_H: f64 = 58.0;
const NOTE_H: f64 = 38.0;
const ACTIVATION_W: f64 = 12.0;
const NESTED_ACTIVATION_OFFSET: f64 = 4.0;
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
    note_overlay_y: Option<f64>,
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
        .zip(&diagram.participants)
        .map(|(label, participant)| participant_header_height(&participant.kind, line_count(label)))
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
                mirrored: false,
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
    let mut activation_starts: HashMap<String, Vec<(f64, usize)>> = HashMap::new();
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

    for (event_index, event) in diagram.events.iter().enumerate() {
        match event {
            SequenceEvent::AutoNumber {
                visible,
                start,
                step,
            } => {
                auto_number_visible = *visible;
                if *visible {
                    if let Some(start) = start {
                        message_number = *start;
                    }
                    if let Some(step) = step {
                        message_number_step = *step;
                    }
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
                let Some(&from_center_x) = centers.get(from) else {
                    continue;
                };
                let Some(&to_center_x) = centers.get(to) else {
                    continue;
                };
                let created_receiver = event_index.checked_sub(1).and_then(|previous_index| {
                    match &diagram.events[previous_index] {
                        SequenceEvent::ParticipantCreated { participant } if participant == to => {
                            Some(participant.as_str())
                        }
                        _ => None,
                    }
                });
                let destroyed_participant =
                    diagram
                        .events
                        .get(event_index + 1)
                        .and_then(|next| match next {
                            SequenceEvent::ParticipantDestroyed { participant }
                                if participant == from || participant == to =>
                            {
                                Some(participant.as_str())
                            }
                            _ => None,
                        });
                let label = wrap_sequence_text(
                    label,
                    wrap,
                    (to_center_x - from_center_x)
                        .abs()
                        .clamp(80.0, WRAPPED_TEXT_MAX_WIDTH),
                );
                let label_height = 16.0 * label.lines().count().max(1) as f64;
                let message_y = y + label_height + 6.0;
                let from_x =
                    activation_endpoint(&activation_starts, from, from_center_x, to_center_x);
                let mut to_x =
                    activation_endpoint(&activation_starts, to, to_center_x, from_center_x);
                if let Some(participant) = created_receiver {
                    let lane_index = diagram
                        .participants
                        .iter()
                        .position(|item| item.id == participant)
                        .unwrap_or(0);
                    let definition = &diagram.participants[lane_index];
                    let box_width = (lane_widths[lane_index] - 24.0).max(100.0);
                    let created_header_height = participant_header_height(
                        &definition.kind,
                        line_count(&participant_labels[lane_index]),
                    );
                    let box_y = message_y - created_header_height / 2.0;
                    items.push(LayoutedSequenceItem::ParticipantBox {
                        id: definition.id.clone(),
                        label: participant_labels[lane_index].clone(),
                        label_height: line_count(&participant_labels[lane_index]) as f64 * 16.0,
                        mirrored: false,
                        kind: definition.kind.clone(),
                        links: definition.links.clone(),
                        properties: definition.properties.clone(),
                        details_reference: definition.details_reference.clone(),
                        x: to_center_x - box_width / 2.0,
                        y: box_y,
                        width: box_width,
                        height: created_header_height,
                    });
                    lifeline_starts.insert(participant.to_string(), box_y + created_header_height);
                    to_x = endpoint_at_box_edge(to_center_x, from_center_x, box_width / 2.0 + 3.0);
                } else if *activate
                    && activation_starts.get(to).is_none_or(Vec::is_empty)
                    && from_center_x != to_center_x
                {
                    // The opening message points at the bar that starts on this line.
                    to_x =
                        endpoint_at_box_edge(to_center_x, from_center_x, ACTIVATION_W / 2.0 - 1.0);
                }
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
                    message_number =
                        ((message_number + message_number_step) * 100.0).round() / 100.0;
                }
                match central_connection {
                    SequenceCentralConnection::Source => {
                        open_activation(&mut activation_starts, from, message_y);
                    }
                    SequenceCentralConnection::Destination => {
                        open_activation(&mut activation_starts, to, message_y);
                    }
                    SequenceCentralConnection::Both => {
                        open_activation(&mut activation_starts, from, message_y);
                        open_activation(&mut activation_starts, to, message_y);
                    }
                    SequenceCentralConnection::None => {}
                }
                if *activate {
                    open_activation(&mut activation_starts, to, message_y);
                }
                if *deactivate {
                    close_activation(
                        &mut items,
                        &mut activation_starts,
                        from,
                        from_center_x,
                        message_y,
                    );
                }
                if let Some(participant) = destroyed_participant {
                    if let Some(&center) = centers.get(participant) {
                        items.push(LayoutedSequenceItem::Destruction {
                            participant: participant.to_string(),
                            x: center,
                            y: message_y,
                        });
                        lifeline_ends.insert(participant.to_string(), message_y);
                    }
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
                    let starts = activation_starts.entry(participant.clone()).or_default();
                    starts.push((y, starts.len()));
                } else {
                    close_activation(
                        &mut items,
                        &mut activation_starts,
                        participant,
                        participant_x,
                        y,
                    );
                }
            }
            SequenceEvent::ParticipantCreated { participant } => {
                if matches!(
                    diagram.events.get(event_index + 1),
                    Some(SequenceEvent::Message { to, .. }) if to == participant
                ) {
                    continue;
                }
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
                    let created_header_height = participant_header_height(
                        &definition.kind,
                        line_count(&participant_labels[lane_index]),
                    );
                    items.push(LayoutedSequenceItem::ParticipantBox {
                        id: definition.id.clone(),
                        label: participant_labels[lane_index].clone(),
                        label_height: line_count(&participant_labels[lane_index]) as f64 * 16.0,
                        mirrored: false,
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
                if matches!(
                    event_index.checked_sub(1).map(|index| &diagram.events[index]),
                    Some(SequenceEvent::Message { from, to, .. })
                        if from == participant || to == participant
                ) {
                    continue;
                }
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
                let note_height = NOTE_H + 16.0 * line_count.saturating_sub(1) as f64;
                let note_y = block_stack
                    .last()
                    .and_then(|frame| frame.note_overlay_y)
                    .unwrap_or(y);
                items.push(LayoutedSequenceItem::Note {
                    x: note_x,
                    y: note_y - 10.0,
                    width: note_width,
                    height: note_height,
                    text,
                });
                y = y.max(note_y + note_height + 20.0);
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
                let content_y = if kind == &SequenceBlockKind::Rect {
                    y
                } else {
                    y + BLOCK_HEADER_H + label_height - 16.0
                };
                block_stack.push(BlockFrameState {
                    kind: kind.clone(),
                    label,
                    label_height,
                    fill: fill.clone(),
                    depth,
                    x,
                    y,
                    width: frame_width,
                    note_overlay_y: (kind == &SequenceBlockKind::ParOver).then_some(content_y),
                });
                y = content_y;
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

    let footer_y = (y + 36.0).max(180.0);
    let height = footer_y + header_height + 28.0;
    for (((participant, lane_width), label), center) in diagram
        .participants
        .iter()
        .zip(&lane_widths)
        .zip(&participant_labels)
        .zip(
            diagram
                .participants
                .iter()
                .map(|participant| centers[&participant.id]),
        )
    {
        let box_width = (*lane_width - 24.0).max(100.0);
        let footer_height = participant_header_height(&participant.kind, line_count(label));
        items.push(LayoutedSequenceItem::ParticipantBox {
            id: participant.id.clone(),
            label: label.clone(),
            label_height: line_count(label) as f64 * 16.0,
            mirrored: true,
            kind: participant.kind.clone(),
            links: participant.links.clone(),
            properties: participant.properties.clone(),
            details_reference: participant.details_reference.clone(),
            x: center - box_width / 2.0,
            y: footer_y,
            width: box_width,
            height: footer_height,
        });
    }
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
                    .unwrap_or(footer_y),
            });
            if let Some(starts) = activation_starts.remove(&participant.id) {
                for (start, depth) in starts {
                    items.push(LayoutedSequenceItem::Activation {
                        participant: participant.id.clone(),
                        x: activation_x(center, depth),
                        y1: start,
                        y2: footer_y,
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

fn participant_header_height(kind: &SequenceParticipantKind, lines: usize) -> f64 {
    let base = if kind == &SequenceParticipantKind::Actor {
        64.0
    } else {
        HEADER_H
    };
    base + 16.0 * (lines.max(1) as f64 - 1.0)
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
    starts: &mut HashMap<String, Vec<(f64, usize)>>,
    participant: &str,
    center: f64,
    y: f64,
) {
    if let Some((start, depth)) = starts.get_mut(participant).and_then(Vec::pop) {
        items.push(LayoutedSequenceItem::Activation {
            participant: participant.to_string(),
            x: activation_x(center, depth),
            y1: start,
            y2: y.max(start + 12.0),
        });
    }
}

fn open_activation(starts: &mut HashMap<String, Vec<(f64, usize)>>, participant: &str, y: f64) {
    let participant_starts = starts.entry(participant.to_string()).or_default();
    participant_starts.push((y, participant_starts.len()));
}

fn activation_x(center: f64, depth: usize) -> f64 {
    center - ACTIVATION_W / 2.0 + depth as f64 * NESTED_ACTIVATION_OFFSET
}

fn activation_endpoint(
    starts: &HashMap<String, Vec<(f64, usize)>>,
    participant: &str,
    center: f64,
    other: f64,
) -> f64 {
    let Some(depth) = starts
        .get(participant)
        .and_then(|participant_starts| participant_starts.len().checked_sub(1))
    else {
        return center;
    };
    if center < other {
        activation_x(center, depth) + ACTIVATION_W
    } else if center > other {
        activation_x(center, 0)
    } else {
        center
    }
}

fn endpoint_at_box_edge(center: f64, other: f64, offset: f64) -> f64 {
    if other < center {
        center - offset
    } else if other > center {
        center + offset
    } else {
        center
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

    fn message(from: &str, to: &str, label: &str) -> SequenceEvent {
        SequenceEvent::Message {
            from: from.into(),
            to: to.into(),
            label: label.into(),
            wrap: SequenceTextWrap::Default,
            line_style: SequenceLineStyle::Solid,
            arrowhead: SequenceArrowhead::Filled,
            bidirectional: false,
            central_connection: SequenceCentralConnection::None,
            activate: false,
            deactivate: false,
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
            4
        );
        assert_eq!(
            layout
                .items
                .iter()
                .filter(|item| matches!(
                    item,
                    LayoutedSequenceItem::ParticipantBox { mirrored: true, .. }
                ))
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
    fn resumes_autonumber_without_reset_and_rounds_each_increment() {
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
            auto_number: false,
            auto_number_start: 1.0,
            auto_number_step: 1.0,
            participants: vec![participant("Alice"), participant("Bob")],
            participant_groups: vec![],
            events: vec![
                SequenceEvent::AutoNumber {
                    visible: true,
                    start: Some(0.1),
                    step: Some(0.1),
                },
                message("One tenth"),
                message("Two tenths"),
                SequenceEvent::AutoNumber {
                    visible: false,
                    start: None,
                    step: None,
                },
                message("Hidden"),
                SequenceEvent::AutoNumber {
                    visible: true,
                    start: None,
                    step: None,
                },
                message("Three tenths"),
                message("Four tenths"),
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

        assert_eq!(
            numbers,
            vec![Some(0.1), Some(0.2), None, Some(0.3), Some(0.4)]
        );
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
    fn actor_headers_reserve_symbol_geometry() {
        let mut actor = participant("Alice");
        actor.kind = SequenceParticipantKind::Actor;
        let diagram = SequenceDiagram {
            title: None,
            accessibility_title: None,
            accessibility_description: None,
            auto_number: false,
            auto_number_start: 1.0,
            auto_number_step: 1.0,
            participants: vec![actor, participant("Service")],
            participant_groups: vec![],
            events: vec![],
        };
        let layout = layout_sequence_diagram(&diagram);
        let heights: Vec<_> = layout
            .items
            .iter()
            .filter_map(|item| match item {
                LayoutedSequenceItem::ParticipantBox { height, .. } => Some(*height),
                _ => None,
            })
            .collect();
        assert_eq!(heights, vec![64.0, 64.0, 64.0, HEADER_H]);
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
        let activated: HashSet<_> = layout
            .items
            .iter()
            .filter_map(|item| match item {
                LayoutedSequenceItem::Activation { participant, .. } => Some(participant.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(activated, HashSet::from(["Alice", "Bob"]));
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
    fn activation_statements_do_not_consume_event_rows() {
        let plain = SequenceDiagram {
            title: None,
            accessibility_title: None,
            accessibility_description: None,
            auto_number: false,
            auto_number_start: 1.0,
            auto_number_step: 1.0,
            participants: vec![participant("Alice"), participant("Bob")],
            participant_groups: vec![],
            events: vec![message("Alice", "Bob", "request")],
        };
        let active = SequenceDiagram {
            events: vec![
                SequenceEvent::Activation {
                    participant: "Alice".into(),
                    active: true,
                },
                message("Alice", "Bob", "request"),
                SequenceEvent::Activation {
                    participant: "Alice".into(),
                    active: false,
                },
            ],
            ..plain.clone()
        };
        let plain_layout = layout_sequence_diagram(&plain);
        let active_layout = layout_sequence_diagram(&active);
        let message_y = |layout: &LayoutedSequenceDiagram| {
            layout.items.iter().find_map(|item| match item {
                LayoutedSequenceItem::Message { y, .. } => Some(*y),
                _ => None,
            })
        };

        assert_eq!(message_y(&active_layout), message_y(&plain_layout));
        assert_eq!(active_layout.height, plain_layout.height);
    }

    #[test]
    fn offsets_nested_activation_bars_by_stack_depth() {
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
                    active: true,
                },
                SequenceEvent::Activation {
                    participant: "Bob".into(),
                    active: false,
                },
                SequenceEvent::Activation {
                    participant: "Bob".into(),
                    active: false,
                },
            ],
        };
        let mut activations: Vec<_> = layout_sequence_diagram(&diagram)
            .items
            .iter()
            .filter_map(|item| match item {
                LayoutedSequenceItem::Activation { x, y1, y2, .. } => Some((*x, *y1, *y2)),
                _ => None,
            })
            .collect();
        activations.sort_by(|left, right| left.0.total_cmp(&right.0));

        assert_eq!(activations.len(), 2);
        assert_eq!(
            activations[1].0 - activations[0].0,
            NESTED_ACTIVATION_OFFSET
        );
        assert_eq!(activations[0].1, activations[1].1);
        assert_eq!(activations[0].2, activations[1].2);
    }

    #[test]
    fn active_messages_terminate_at_activation_edges() {
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
                SequenceEvent::Activation {
                    participant: "Alice".into(),
                    active: true,
                },
                SequenceEvent::Activation {
                    participant: "Bob".into(),
                    active: true,
                },
                message("Alice", "Bob", "right"),
                message("Bob", "Alice", "left"),
            ],
        };
        let layout = layout_sequence_diagram(&diagram);
        let centers: HashMap<_, _> = layout
            .items
            .iter()
            .filter_map(|item| match item {
                LayoutedSequenceItem::Lifeline { participant, x, .. } => {
                    Some((participant.as_str(), *x))
                }
                _ => None,
            })
            .collect();
        let messages: HashMap<_, _> = layout
            .items
            .iter()
            .filter_map(|item| match item {
                LayoutedSequenceItem::Message {
                    label,
                    from_x,
                    to_x,
                    ..
                } => Some((label.as_str(), (*from_x, *to_x))),
                _ => None,
            })
            .collect();

        assert!(messages["right"].0 > centers["Alice"]);
        assert!(messages["right"].1 < centers["Bob"]);
        assert!(messages["left"].0 < centers["Bob"]);
        assert!(messages["left"].1 > centers["Alice"]);
    }

    #[test]
    fn opening_message_terminates_at_new_activation_edge() {
        let mut opening = message("Alice", "Bob", "open");
        let SequenceEvent::Message { activate, .. } = &mut opening else {
            unreachable!();
        };
        *activate = true;
        let diagram = SequenceDiagram {
            title: None,
            accessibility_title: None,
            accessibility_description: None,
            auto_number: false,
            auto_number_start: 1.0,
            auto_number_step: 1.0,
            participants: vec![participant("Alice"), participant("Bob")],
            participant_groups: vec![],
            events: vec![opening],
        };
        let layout = layout_sequence_diagram(&diagram);
        let bob_x = layout.items.iter().find_map(|item| match item {
            LayoutedSequenceItem::Lifeline { participant, x, .. } if participant == "Bob" => {
                Some(*x)
            }
            _ => None,
        });
        let to_x = layout.items.iter().find_map(|item| match item {
            LayoutedSequenceItem::Message { to_x, .. } => Some(*to_x),
            _ => None,
        });

        assert_eq!(bob_x.unwrap() - to_x.unwrap(), ACTIVATION_W / 2.0 - 1.0);
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
    fn par_over_places_notes_on_the_parallel_content_origin() {
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
                    kind: SequenceBlockKind::ParOver,
                    label: "Parallel overlap".into(),
                    wrap: SequenceTextWrap::Default,
                    fill: None,
                },
                SequenceEvent::Message {
                    from: "Alice".into(),
                    to: "Bob".into(),
                    label: "Message".into(),
                    wrap: SequenceTextWrap::Default,
                    line_style: SequenceLineStyle::Solid,
                    arrowhead: SequenceArrowhead::Filled,
                    bidirectional: false,
                    central_connection: SequenceCentralConnection::None,
                    activate: false,
                    deactivate: false,
                },
                SequenceEvent::Note {
                    participants: vec!["Alice".into()],
                    placement: SequenceNotePlacement::LeftOf,
                    text: "Alice note".into(),
                    wrap: SequenceTextWrap::Default,
                },
                SequenceEvent::Note {
                    participants: vec!["Bob".into()],
                    placement: SequenceNotePlacement::RightOf,
                    text: "Bob note".into(),
                    wrap: SequenceTextWrap::Default,
                },
                SequenceEvent::BlockEnd {
                    kind: SequenceBlockKind::ParOver,
                },
            ],
        };
        let layout = layout_sequence_diagram(&diagram);
        let message_y = layout
            .items
            .iter()
            .find_map(|item| match item {
                LayoutedSequenceItem::Message { y, .. } => Some(*y),
                _ => None,
            })
            .unwrap();
        let note_ys: Vec<_> = layout
            .items
            .iter()
            .filter_map(|item| match item {
                LayoutedSequenceItem::Note { y, .. } => Some(*y),
                _ => None,
            })
            .collect();

        assert_eq!(note_ys.len(), 2);
        assert_eq!(note_ys[0], note_ys[1]);
        assert!(note_ys[0] < message_y);
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
        let (worker_box_x, worker_box_y, worker_box_height) = layout
            .items
            .iter()
            .find_map(|item| match item {
                LayoutedSequenceItem::ParticipantBox {
                    id,
                    mirrored: false,
                    x,
                    y,
                    height,
                    ..
                } if id == "Worker" => Some((*x, *y, *height)),
                _ => None,
            })
            .unwrap();
        let (message_to_x, message_y) = layout
            .items
            .iter()
            .find_map(|item| match item {
                LayoutedSequenceItem::Message { label, to_x, y, .. } if label == "Work" => {
                    Some((*to_x, *y))
                }
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
        assert_eq!(worker_box_y + worker_box_height / 2.0, message_y);
        assert_eq!(message_to_x, worker_box_x - 3.0);
        assert_eq!(lifeline_y1, worker_box_y + worker_box_height);
        assert_eq!(lifeline_y2, destruction_y);
        assert_eq!(destruction_y, message_y);
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
