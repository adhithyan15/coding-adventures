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
    let fields = diagram
        .fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            let row = field.start_bit / BITS_PER_ROW;
            let column = field.start_bit % BITS_PER_ROW;
            let bit_count = field.end_bit - field.start_bit + 1;
            LayoutedPacketField {
                start_bit: field.start_bit,
                end_bit: field.end_bit,
                label: field.label.clone(),
                x: PADDING + f64::from(column) * BIT_WIDTH,
                y: PADDING + title_inset + f64::from(row) * ROW_HEIGHT,
                width: f64::from(bit_count) * BIT_WIDTH,
                height: ROW_HEIGHT,
                style: packet_style(index),
            }
        })
        .collect();
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
}
