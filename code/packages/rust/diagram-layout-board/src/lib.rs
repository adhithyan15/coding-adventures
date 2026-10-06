//! Deterministic column/card layout for board diagrams.

pub const VERSION: &str = "0.3.0";

use diagram_ir::{
    BoardDiagram, DiagramStyle, LayoutedBoardCard, LayoutedBoardColumn, LayoutedBoardDiagram,
    ResolvedDiagramStyle,
};

const PADDING: f64 = 24.0;
const COLUMN_WIDTH: f64 = 260.0;
const COLUMN_GAP: f64 = 20.0;
const HEADER_HEIGHT: f64 = 52.0;
const CARD_HEIGHT: f64 = 72.0;
const CARD_GAP: f64 = 12.0;

pub fn layout_board_diagram(board: &BoardDiagram) -> LayoutedBoardDiagram {
    let max_cards_height = board
        .columns
        .iter()
        .map(|column| {
            column.cards.iter().map(card_height).sum::<f64>()
                + column.cards.len().saturating_sub(1) as f64 * CARD_GAP
        })
        .fold(0.0, f64::max);
    let column_height = HEADER_HEIGHT + PADDING + max_cards_height;
    let columns = board
        .columns
        .iter()
        .enumerate()
        .map(|(column_index, column)| {
            let x = PADDING + column_index as f64 * (COLUMN_WIDTH + COLUMN_GAP);
            let mut next_card_y = PADDING + HEADER_HEIGHT + 12.0;
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
                        width: COLUMN_WIDTH - 24.0,
                        height,
                        style: card_style(column_index),
                        ticket: card.ticket.clone(),
                        assigned: card.assigned.clone(),
                        priority: card.priority.clone(),
                        icon: card.icon.clone(),
                    };
                    next_card_y += height + CARD_GAP;
                    layouted
                })
                .collect();
            LayoutedBoardColumn {
                id: column.id.clone(),
                label: column.label.clone(),
                x,
                y: PADDING,
                width: COLUMN_WIDTH,
                height: column_height,
                cards,
                style: column_style(column_index),
            }
        })
        .collect();
    LayoutedBoardDiagram {
        columns,
        width: PADDING * 2.0
            + board.columns.len() as f64 * COLUMN_WIDTH
            + board.columns.len().saturating_sub(1) as f64 * COLUMN_GAP,
        height: PADDING * 2.0 + column_height,
    }
}

fn card_height(card: &diagram_ir::BoardCard) -> f64 {
    if card.ticket.is_some() || card.assigned.is_some() || card.priority.is_some() {
        CARD_HEIGHT + 24.0
    } else {
        CARD_HEIGHT
    }
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
            columns: vec![BoardColumn {
                id: "todo".into(),
                label: DiagramLabel::new("Todo"),
                cards: vec![BoardCard {
                    id: "one".into(),
                    label: DiagramLabel::new("One"),
                    ticket: None,
                    assigned: None,
                    priority: None,
                    icon: None,
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
            columns: vec![BoardColumn {
                id: "todo".into(),
                label: DiagramLabel::new("Todo"),
                cards: vec![BoardCard {
                    id: "one".into(),
                    label: DiagramLabel::new("One"),
                    ticket: Some("MC-42".into()),
                    assigned: Some("Ada".into()),
                    priority: Some("high".into()),
                    icon: Some("heart".into()),
                }],
            }],
        };
        let layout = layout_board_diagram(&board);
        assert_eq!(layout.columns[0].cards[0].height, CARD_HEIGHT + 24.0);
        assert_eq!(layout.columns[0].cards[0].ticket.as_deref(), Some("MC-42"));
    }
}
