//! Deterministic column/card layout for board diagrams.

pub const VERSION: &str = "0.9.0";

use diagram_ir::{
    BoardDiagram, DiagramStyle, LayoutedBoardCard, LayoutedBoardColumn, LayoutedBoardDiagram,
    ResolvedDiagramStyle,
};

const COLUMN_GAP: f64 = 20.0;
const HEADER_HEIGHT: f64 = 52.0;
const CARD_HEIGHT: f64 = 72.0;
const CARD_GAP: f64 = 12.0;

pub fn layout_board_diagram(board: &BoardDiagram) -> LayoutedBoardDiagram {
    let padding = board.config.padding;
    let column_width = board.config.section_width;
    let header_height = board
        .columns
        .iter()
        .map(|column| label_box_height(&column.label, 20.0, 28.0))
        .fold(HEADER_HEIGHT, f64::max);
    let max_cards_height = board
        .columns
        .iter()
        .map(|column| {
            column.cards.iter().map(card_height).sum::<f64>()
                + column.cards.len().saturating_sub(1) as f64 * CARD_GAP
        })
        .fold(0.0, f64::max);
    let column_height = header_height + padding + max_cards_height;
    let columns = board
        .columns
        .iter()
        .enumerate()
        .map(|(column_index, column)| {
            let x = padding + column_index as f64 * (column_width + COLUMN_GAP);
            let mut next_card_y = padding + header_height + 12.0;
            let cards = column
                .cards
                .iter()
                .map(|card| {
                    let height = card_height(card);
                    let layouted = LayoutedBoardCard {
                        id: card.id.clone(),
                        label: card.label.clone(),
                        x: x + 12.0,
                        y: next_card_y,
                        width: column_width - 24.0,
                        height,
                        style: card_style(column_index),
                        ticket: card.ticket.clone(),
                        ticket_url: board.ticket_base_url.as_ref().and_then(|base| {
                            card.ticket.as_ref().map(|ticket| base.replace("#TICKET#", ticket))
                        }),
                        assigned: card.assigned.clone(),
                        priority: card.priority.clone(),
                        icon: card.icon.clone(),
                        classes: card.classes.clone(),
                    };
                    next_card_y += height + CARD_GAP;
                    layouted
                })
                .collect();
            LayoutedBoardColumn {
                id: column.id.clone(),
                label: column.label.clone(),
                x,
                y: padding,
                width: column_width,
                height: column_height,
                header_height,
                cards,
                style: column_style(column_index),
                ticket: column.ticket.clone(),
                ticket_url: board.ticket_base_url.as_ref().and_then(|base| {
                    column.ticket.as_ref().map(|ticket| base.replace("#TICKET#", ticket))
                }),
                classes: column.classes.clone(),
            }
        })
        .collect();
    LayoutedBoardDiagram {
        columns,
        width: padding * 2.0
            + board.columns.len() as f64 * column_width
            + board.columns.len().saturating_sub(1) as f64 * COLUMN_GAP,
        height: padding * 2.0 + column_height,
    }
}

fn card_height(card: &diagram_ir::BoardCard) -> f64 {
    let content_height = label_box_height(&card.label, 18.0, 44.0).max(CARD_HEIGHT);
    content_height
        + if card.ticket.is_some() || card.assigned.is_some() {
            24.0
        } else {
            0.0
        }
}

fn label_box_height(label: &diagram_ir::DiagramLabel, line_height: f64, padding: f64) -> f64 {
    label.text.lines().count().max(1) as f64 * line_height + padding
}

fn column_style(index: usize) -> ResolvedDiagramStyle {
    let fills = ["#e0f2fe", "#fef3c7", "#dcfce7", "#fce7f3"];
    ResolvedDiagramStyle {
        fill: fills[index % fills.len()].into(),
        stroke: "#475569".into(),
        text_color: "#0f172a".into(),
        corner_radius: 10.0,
        ..diagram_ir::resolve_style(Some(&DiagramStyle::default()))
    }
}

fn card_style(index: usize) -> ResolvedDiagramStyle {
    ResolvedDiagramStyle {
        fill: "#ffffff".into(),
        stroke: if index.is_multiple_of(2) {
            "#0284c7"
        } else {
            "#d97706"
        }
        .into(),
        text_color: "#1e293b".into(),
        corner_radius: 8.0,
        ..diagram_ir::resolve_style(Some(&DiagramStyle::default()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use diagram_ir::{BoardCard, BoardColumn, DiagramLabel};

    #[test]
    fn lays_out_columns_and_cards() {
        let board = BoardDiagram {
            ticket_base_url: None,
            config: diagram_ir::BoardConfig::default(),
            columns: vec![BoardColumn {
                id: "todo".into(),
                label: DiagramLabel::new("Todo"),
                ticket: None,
                classes: Vec::new(),
                cards: vec![BoardCard {
                    id: "one".into(),
                    label: DiagramLabel::new("One"),
                    ticket: None,
                    assigned: None,
                    priority: None,
                    icon: None,
                    classes: Vec::new(),
                }],
            }],
        };
        let layout = layout_board_diagram(&board);
        assert_eq!(layout.columns.len(), 1);
        assert!(layout.columns[0].cards[0].y > layout.columns[0].y);
        assert!(layout.width > 0.0 && layout.height > 0.0);
    }

    #[test]
    fn metadata_reserves_card_footer_geometry() {
        let board = BoardDiagram {
            ticket_base_url: Some("https://tracker.example/issues/#TICKET#".into()),
            config: diagram_ir::BoardConfig { section_width: 300.0, padding: 32.0 },
            columns: vec![BoardColumn {
                id: "todo".into(),
                label: DiagramLabel::new("Todo"),
                ticket: Some("KB-7".into()),
                classes: vec!["backlog".into()],
                cards: vec![BoardCard {
                    id: "one".into(),
                    label: DiagramLabel::new("One"),
                    ticket: Some("MC-42".into()),
                    assigned: Some("Ada".into()),
                    priority: Some("high".into()),
                    icon: Some("heart".into()),
                    classes: vec!["urgent".into(), "blocked".into()],
                }],
            }],
        };
        let layout = layout_board_diagram(&board);
        assert_eq!(layout.columns[0].cards[0].height, CARD_HEIGHT + 24.0);
        assert_eq!(layout.columns[0].cards[0].ticket.as_deref(), Some("MC-42"));
        assert_eq!(
            layout.columns[0].cards[0].ticket_url.as_deref(),
            Some("https://tracker.example/issues/MC-42")
        );
        assert_eq!(layout.columns[0].classes, ["backlog"]);
        assert_eq!(layout.columns[0].ticket.as_deref(), Some("KB-7"));
        assert_eq!(
            layout.columns[0].ticket_url.as_deref(),
            Some("https://tracker.example/issues/KB-7")
        );
        assert_eq!(layout.columns[0].cards[0].classes, ["urgent", "blocked"]);
        assert_eq!(layout.columns[0].x, 32.0);
        assert_eq!(layout.columns[0].width, 300.0);
        assert_eq!(layout.width, 364.0);
    }

    #[test]
    fn multiline_labels_expand_headers_and_cards() {
        let board = BoardDiagram {
            ticket_base_url: None,
            config: diagram_ir::BoardConfig::default(),
            columns: vec![BoardColumn {
                id: "todo".into(),
                label: DiagramLabel::new("Todo\nqueue"),
                ticket: None,
                classes: Vec::new(),
                cards: vec![BoardCard {
                    id: "one".into(),
                    label: DiagramLabel::new("Line 1\nLine 2\nLine 3"),
                    ticket: None,
                    assigned: None,
                    priority: None,
                    icon: None,
                    classes: Vec::new(),
                }],
            }],
        };
        let layout = layout_board_diagram(&board);
        assert_eq!(layout.columns[0].header_height, 68.0);
        assert_eq!(layout.columns[0].cards[0].height, 98.0);
        assert_eq!(layout.columns[0].cards[0].y, 104.0);
    }

    #[test]
    fn priority_only_cards_do_not_reserve_a_footer() {
        let card = BoardCard {
            id: "urgent".into(),
            label: DiagramLabel::new("Urgent"),
            ticket: None,
            assigned: None,
            priority: Some("Very High".into()),
            icon: None,
            classes: Vec::new(),
        };
        assert_eq!(card_height(&card), CARD_HEIGHT);
    }
}
