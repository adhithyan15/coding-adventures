//! Deterministic 32-bit-row layout for Mermaid packet diagrams.

pub const VERSION: &str = "0.1.0";

use diagram_ir::{
    DiagramStyle, LayoutedPacketDiagram, LayoutedPacketField, PacketDiagram, ResolvedDiagramStyle,
};

const BITS_PER_ROW: u32 = 32;
const BIT_WIDTH: f64 = 24.0;
const ROW_HEIGHT: f64 = 68.0;
const PADDING: f64 = 24.0;
const TITLE_INSET: f64 = 38.0;

/// Resolve absolute packet bit ranges into rectangular field geometry.
pub fn layout_packet_diagram(diagram: &PacketDiagram) -> LayoutedPacketDiagram {
    let title_inset = if diagram.title.is_some() {
        TITLE_INSET
    } else {
        0.0
    };
    let mut fields = Vec::new();
    for (index, field) in diagram.fields.iter().enumerate() {
        let mut start_bit = field.start_bit;
        while start_bit <= field.end_bit {
            let row = start_bit / BITS_PER_ROW;
            let row_end = row.checked_add(1)
                .and_then(|next_row| next_row.checked_mul(BITS_PER_ROW))
                .and_then(|next_row_start| next_row_start.checked_sub(1))
                .unwrap_or(u32::MAX);
            let end_bit = field.end_bit.min(row_end);
            let column = start_bit % BITS_PER_ROW;
            fields.push(LayoutedPacketField {
                start_bit,
                end_bit,
                label: field.label.clone(),
                x: PADDING + f64::from(column) * BIT_WIDTH,
                y: PADDING + title_inset + f64::from(row) * ROW_HEIGHT,
                width: f64::from(end_bit - start_bit + 1) * BIT_WIDTH,
                height: ROW_HEIGHT,
                style: packet_style(index),
            });
            let Some(next) = end_bit.checked_add(1) else { break };
            start_bit = next;
        }
    }
    let rows = diagram
        .fields
        .last()
        .map(|field| field.end_bit / BITS_PER_ROW + 1)
        .unwrap_or(1);

    LayoutedPacketDiagram {
        title: diagram.title.clone(),
        accessibility_title: diagram.accessibility_title.clone(),
        accessibility_description: diagram.accessibility_description.clone(),
        fields,
        width: PADDING * 2.0 + f64::from(BITS_PER_ROW) * BIT_WIDTH,
        height: PADDING * 2.0 + title_inset + f64::from(rows) * ROW_HEIGHT,
    }
}

fn packet_style(index: usize) -> ResolvedDiagramStyle {
    let fills = ["#dbeafe", "#dcfce7", "#fef3c7", "#fee2e2", "#e0e7ff"];
    ResolvedDiagramStyle {
        fill: fills[index % fills.len()].into(),
        stroke: "#334155".into(),
        stroke_width: 1.5,
        text_color: "#0f172a".into(),
        corner_radius: 0.0,
        ..diagram_ir::resolve_style(Some(&DiagramStyle::default()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use diagram_ir::{DiagramLabel, PacketField};

    #[test]
    fn lays_out_ranges_on_32_bit_rows() {
        let layout = layout_packet_diagram(&PacketDiagram {
            title: Some("Packet".into()),
            fields: vec![
                PacketField {
                    start_bit: 0,
                    end_bit: 7,
                    label: DiagramLabel::new("Header"),
                },
                PacketField {
                    start_bit: 8,
                    end_bit: 31,
                    label: DiagramLabel::new("Payload"),
                },
                PacketField {
                    start_bit: 32,
                    end_bit: 63,
                    label: DiagramLabel::new("Data"),
                },
            ],
            ..PacketDiagram::default()
        });
        assert_eq!(layout.fields[0].width, 192.0);
        assert_eq!(layout.fields[1].x, 216.0);
        assert!(layout.fields[2].y > layout.fields[1].y);
        assert_eq!(layout.width, 816.0);
    }

    #[test]
    fn splits_fields_that_cross_row_boundaries() {
        let layout = layout_packet_diagram(&PacketDiagram {
            fields: vec![PacketField { start_bit: 0, end_bit: 63, label: DiagramLabel::new("wide") }],
            ..PacketDiagram::default()
        });
        assert_eq!(layout.fields.len(), 2);
        assert_eq!((layout.fields[0].start_bit, layout.fields[0].end_bit), (0, 31));
        assert_eq!((layout.fields[1].start_bit, layout.fields[1].end_bit), (32, 63));
        assert!(layout.fields[1].y > layout.fields[0].y);
    }
}
