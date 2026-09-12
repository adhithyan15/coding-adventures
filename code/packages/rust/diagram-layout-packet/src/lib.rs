//! Deterministic 32-bit-row layout for Mermaid packet diagrams.

pub const VERSION: &str = "0.1.0";

use diagram_ir::{
    LayoutedPacketBitLabel, LayoutedPacketDiagram, LayoutedPacketField, PacketDiagram,
    ResolvedDiagramStyle, TextAlign,
};

/// Resolve absolute packet bit ranges into rectangular field geometry.
pub fn layout_packet_diagram(diagram: &PacketDiagram) -> LayoutedPacketDiagram {
    let config = &diagram.config;
    let bits_per_row = config.bits_per_row.max(1);
    let padding_y = config.padding_y + if config.show_bits { 10.0 } else { 0.0 };
    let total_row_height = config.row_height + padding_y;
    let mut fields = Vec::new();
    let mut bit_labels = Vec::new();
    for field in &diagram.fields {
        let mut start_bit = field.start_bit;
        while start_bit <= field.end_bit {
            let row = start_bit / bits_per_row;
            let row_end = row.checked_add(1)
                .and_then(|next_row| next_row.checked_mul(bits_per_row))
                .and_then(|next_row_start| next_row_start.checked_sub(1))
                .unwrap_or(u32::MAX);
            let end_bit = field.end_bit.min(row_end);
            let column = start_bit % bits_per_row;
            let x = f64::from(column) * config.bit_width + 1.0;
            let y = f64::from(row) * total_row_height + padding_y;
            let width = (f64::from(end_bit - start_bit + 1) * config.bit_width
                - config.padding_x)
                .max(0.0);
            fields.push(LayoutedPacketField {
                start_bit,
                end_bit,
                label: field.label.clone(),
                x,
                y,
                width,
                height: config.row_height,
                style: field_style(diagram),
            });
            if config.show_bits {
                let single_bit = start_bit == end_bit;
                bit_labels.push(LayoutedPacketBitLabel {
                    text: start_bit.to_string(),
                    x,
                    y: y - 12.0,
                    width,
                    height: 10.0,
                    align: if single_bit {
                        TextAlign::Center
                    } else {
                        TextAlign::Left
                    },
                    style: bit_label_style(diagram, true),
                });
                if !single_bit {
                    bit_labels.push(LayoutedPacketBitLabel {
                        text: end_bit.to_string(),
                        x,
                        y: y - 12.0,
                        width,
                        height: 10.0,
                        align: TextAlign::Right,
                        style: bit_label_style(diagram, false),
                    });
                }
            }
            let Some(next) = end_bit.checked_add(1) else { break };
            start_bit = next;
        }
    }
    let rows = diagram
        .fields
        .last()
        .map(|field| u64::from(field.end_bit / bits_per_row) + 1)
        .unwrap_or(0u64);
    let height = total_row_height * (rows + 1) as f64
        - if diagram.title.is_some() { 0.0 } else { config.row_height };

    LayoutedPacketDiagram {
        title: diagram.title.clone(),
        accessibility_title: diagram.accessibility_title.clone(),
        accessibility_description: diagram.accessibility_description.clone(),
        fields,
        bit_labels,
        title_style: title_style(diagram),
        title_y: height - total_row_height / 2.0,
        width: f64::from(bits_per_row) * config.bit_width + 2.0,
        height,
    }
}

fn field_style(diagram: &PacketDiagram) -> ResolvedDiagramStyle {
    ResolvedDiagramStyle {
        fill: diagram.theme.block_fill_color.clone(),
        stroke: diagram.theme.block_stroke_color.clone(),
        stroke_width: diagram.theme.block_stroke_width,
        text_color: diagram.theme.label_color.clone(),
        font_size: diagram.theme.label_font_size,
        corner_radius: 0.0,
        ..diagram_ir::resolve_style(None)
    }
}

fn bit_label_style(diagram: &PacketDiagram, start: bool) -> ResolvedDiagramStyle {
    ResolvedDiagramStyle {
        text_color: if start {
            diagram.theme.start_byte_color.clone()
        } else {
            diagram.theme.end_byte_color.clone()
        },
        font_size: diagram.theme.byte_font_size,
        ..diagram_ir::resolve_style(None)
    }
}

fn title_style(diagram: &PacketDiagram) -> ResolvedDiagramStyle {
    ResolvedDiagramStyle {
        text_color: diagram.theme.title_color.clone(),
        font_size: diagram.theme.title_font_size,
        ..diagram_ir::resolve_style(None)
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
        assert_eq!(layout.fields[0].width, 251.0);
        assert_eq!(layout.fields[1].x, 257.0);
        assert!(layout.fields[2].y > layout.fields[1].y);
        assert_eq!(layout.width, 1026.0);
        assert_eq!(layout.bit_labels.len(), 6);
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

    #[test]
    fn applies_packet_layout_configuration() {
        let layout = layout_packet_diagram(&PacketDiagram {
            fields: vec![PacketField {
                start_bit: 0,
                end_bit: 31,
                label: DiagramLabel::new("wide"),
            }],
            config: diagram_ir::PacketConfig {
                row_height: 40.0,
                bit_width: 20.0,
                bits_per_row: 16,
                show_bits: false,
                padding_x: 4.0,
                padding_y: 6.0,
            },
            ..PacketDiagram::default()
        });
        assert_eq!(layout.fields.len(), 2);
        assert_eq!(layout.fields[0].width, 316.0);
        assert_eq!(layout.fields[0].height, 40.0);
        assert_eq!(layout.fields[1].y, 52.0);
        assert_eq!(layout.width, 322.0);
        assert_eq!(layout.height, 98.0);
        assert!(layout.bit_labels.is_empty());
    }

    #[test]
    fn resolves_packet_theme_into_backend_neutral_styles() {
        let layout = layout_packet_diagram(&PacketDiagram {
            title: Some("Themed".into()),
            fields: vec![PacketField {
                start_bit: 0,
                end_bit: 7,
                label: DiagramLabel::new("byte"),
            }],
            theme: diagram_ir::PacketTheme {
                byte_font_size: 11.0,
                start_byte_color: "red".into(),
                end_byte_color: "blue".into(),
                label_color: "green".into(),
                label_font_size: 13.0,
                title_color: "orange".into(),
                title_font_size: 17.0,
                block_stroke_color: "#123456".into(),
                block_stroke_width: 2.5,
                block_fill_color: "#abcdef".into(),
            },
            ..PacketDiagram::default()
        });
        assert_eq!(layout.fields[0].style.fill, "#abcdef");
        assert_eq!(layout.fields[0].style.stroke, "#123456");
        assert_eq!(layout.fields[0].style.stroke_width, 2.5);
        assert_eq!(layout.fields[0].style.text_color, "green");
        assert_eq!(layout.fields[0].style.font_size, 13.0);
        assert_eq!(layout.bit_labels[0].style.text_color, "red");
        assert_eq!(layout.bit_labels[0].style.font_size, 11.0);
        assert_eq!(layout.bit_labels[1].style.text_color, "blue");
        assert_eq!(layout.title_style.text_color, "orange");
        assert_eq!(layout.title_style.font_size, 17.0);
    }
}
