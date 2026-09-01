//! Deterministic column/card layout for board diagrams.

pub const VERSION: &str = "0.1.0";

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
    let max_cards = board
        .columns
        .iter()
        .map(|column| column.cards.len())
        .max()
        .unwrap_or(0);
    let column_height = HEADER_HEIGHT + PADDING + max_cards as f64 * (CARD_HEIGHT + CARD_GAP);
    let columns = board
        .columns
        .iter()
        .enumerate()
        .map(|(column_index, column)| {
            let x = PADDING + column_index as f64 * (COLUMN_WIDTH + COLUMN_GAP);
            let cards = column
                .cards
                .iter()
                .enumerate()
                .map(|(card_index, card)| LayoutedBoardCard {
                    id: card.id.clone(),
                    label: card.label.clone(),
                    x: x + 12.0,
                    y: PADDING
                        + HEADER_HEIGHT
                        + 12.0
                        + card_index as f64 * (CARD_HEIGHT + CARD_GAP),
                    width: COLUMN_WIDTH - 24.0,
                    height: CARD_HEIGHT,
                    style: card_style(column_index),
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
                }],
            }],
        };
        let layout = layout_board_diagram(&board);
        assert_eq!(layout.columns.len(), 1);
        assert!(layout.columns[0].cards[0].y > layout.columns[0].y);
        assert!(layout.width > 0.0 && layout.height > 0.0);
    }
}
