//! Deterministic grid layout for Mermaid block diagrams.

pub const VERSION: &str = "0.1.0";

use std::collections::HashMap;

use diagram_ir::{
    resolve_style, DiagramDirection, DiagramStyle, EdgeKind, GridDiagram, LayoutedGraphDiagram,
    LayoutedGraphEdge, LayoutedGraphNode, Point, ResolvedDiagramStyle,
};

const PADDING: f64 = 24.0;
const CELL_WIDTH: f64 = 150.0;
const CELL_HEIGHT: f64 = 58.0;
const COLUMN_GAP: f64 = 24.0;
const ROW_GAP: f64 = 24.0;
const TITLE_INSET: f64 = 38.0;

/// Lay out a flat Mermaid block grid into shared graph geometry.
pub fn layout_grid_diagram(diagram: &GridDiagram) -> LayoutedGraphDiagram {
    let columns = diagram.columns.max(1);
    let title_inset = if diagram.title.is_some() {
        TITLE_INSET
    } else {
        0.0
    };
    let mut nodes = Vec::new();
    let mut positions = HashMap::new();
    let mut occupied_slots = 0usize;

    for (index, cell) in diagram.cells.iter().enumerate() {
        let column = occupied_slots % columns;
        let row = occupied_slots / columns;
        let column_span = cell.column_span.min(columns - column);
        let x = PADDING + column as f64 * (CELL_WIDTH + COLUMN_GAP);
        let y = PADDING + title_inset + row as f64 * (CELL_HEIGHT + ROW_GAP);
        let width = column_span as f64 * CELL_WIDTH
            + column_span.saturating_sub(1) as f64 * COLUMN_GAP;
        positions.insert(
            cell.id.clone(),
            (
                Point {
                    x: x + width / 2.0,
                    y: y + CELL_HEIGHT / 2.0,
                },
                width,
            ),
        );
        occupied_slots += column_span;
        if !cell.visible {
            continue;
        }
        nodes.push(LayoutedGraphNode {
            id: cell.id.clone(),
            label: cell.label.clone(),
            shape: cell.shape.clone(),
            x,
            y,
            width,
            height: CELL_HEIGHT,
            style: resolve_style(Some(&grid_style(index))),
        });
    }

    let edge_style = ResolvedDiagramStyle {
        fill: "none".into(),
        stroke: "#475569".into(),
        text_color: "#334155".into(),
        ..ResolvedDiagramStyle::default()
    };
    let edges = diagram
        .connections
        .iter()
        .filter_map(|connection| {
            let (from, from_width) = positions.get(&connection.from)?;
            let (to, to_width) = positions.get(&connection.to)?;
            let (start, end) = connection_endpoints(from, *from_width, to, *to_width);
            Some(LayoutedGraphEdge {
                id: None,
                from_node_id: connection.from.clone(),
                to_node_id: connection.to.clone(),
                kind: EdgeKind::Directed,
                points: vec![start, end],
                label: connection.label.clone(),
                label_position: connection.label.as_ref().map(|_| Point {
                    x: (from.x + to.x) / 2.0,
                    y: (from.y + to.y) / 2.0 - 10.0,
                }),
                style: edge_style.clone(),
            })
        })
        .collect();

    let rows = occupied_slots.div_ceil(columns).max(1);
    LayoutedGraphDiagram {
        direction: DiagramDirection::Tb,
        requested_width: None,
        hide_empty_descriptions: false,
        title: diagram.title.clone(),
        accessibility_title: diagram.accessibility_title.clone(),
        accessibility_description: diagram.accessibility_description.clone(),
        links: Vec::new(),
        groups: Vec::new(),
        width: PADDING * 2.0 + columns as f64 * CELL_WIDTH + (columns - 1) as f64 * COLUMN_GAP,
        height: PADDING * 2.0
            + title_inset
            + rows as f64 * CELL_HEIGHT
            + (rows - 1) as f64 * ROW_GAP,
        nodes,
        edges,
    }
}

fn connection_endpoints(
    from: &Point,
    from_width: f64,
    to: &Point,
    to_width: f64,
) -> (Point, Point) {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    if dx.abs() >= dy.abs() {
        let direction = dx.signum();
        (
            Point {
                x: from.x + direction * from_width / 2.0,
                y: from.y,
            },
            Point {
                x: to.x - direction * to_width / 2.0,
                y: to.y,
            },
        )
    } else {
        let direction = dy.signum();
        (
            Point {
                x: from.x,
                y: from.y + direction * CELL_HEIGHT / 2.0,
            },
            Point {
                x: to.x,
                y: to.y - direction * CELL_HEIGHT / 2.0,
            },
        )
    }
}

fn grid_style(index: usize) -> DiagramStyle {
    let (fill, stroke, text) = match index % 4 {
        0 => ("#e0f2fe", "#0284c7", "#0c4a6e"),
        1 => ("#ecfccb", "#65a30d", "#365314"),
        2 => ("#ffedd5", "#ea580c", "#7c2d12"),
        _ => ("#fce7f3", "#db2777", "#831843"),
    };
    DiagramStyle {
        fill: Some(fill.into()),
        stroke: Some(stroke.into()),
        text_color: Some(text.into()),
        corner_radius: Some(10.0),
        ..DiagramStyle::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use diagram_ir::{DiagramLabel, DiagramShape, GridCell, GridConnection};

    #[test]
    fn places_cells_in_authored_grid_slots() {
        let diagram = GridDiagram {
            columns: 2,
            title: None,
            accessibility_title: None,
            accessibility_description: None,
            cells: ["a", "b", "c"]
                .into_iter()
                .map(|id| GridCell {
                    id: id.into(),
                    label: DiagramLabel::new(id),
                    shape: DiagramShape::RoundedRect,
                    column_span: 1,
                    visible: true,
                    style: None,
                })
                .collect(),
            connections: Vec::new(),
        };
        let layout = layout_grid_diagram(&diagram);
        assert_eq!(layout.nodes.len(), 3);
        assert!(layout.nodes[0].x < layout.nodes[1].x);
        assert!(layout.nodes[2].y > layout.nodes[0].y);
    }

    #[test]
    fn routes_connections_to_cell_boundaries() {
        let diagram = GridDiagram {
            columns: 2,
            title: None,
            accessibility_title: None,
            accessibility_description: None,
            cells: ["a", "b"]
                .into_iter()
                .map(|id| GridCell {
                    id: id.into(),
                    label: DiagramLabel::new(id),
                    shape: DiagramShape::Rect,
                    column_span: 1,
                    visible: true,
                    style: None,
                })
                .collect(),
            connections: vec![GridConnection {
                from: "a".into(),
                to: "b".into(),
                label: None,
            }],
        };
        let layout = layout_grid_diagram(&diagram);
        assert_eq!(layout.edges.len(), 1);
        assert_eq!(layout.edges[0].points[0].x, layout.nodes[0].x + CELL_WIDTH);
        assert_eq!(layout.edges[0].points[1].x, layout.nodes[1].x);
    }

    #[test]
    fn spans_cells_and_advances_by_occupied_columns() {
        let diagram = GridDiagram {
            columns: 3,
            title: None,
            accessibility_title: None,
            accessibility_description: None,
            cells: vec![
                GridCell {
                    id: "wide".into(),
                    label: DiagramLabel::new("Wide"),
                    shape: DiagramShape::Rect,
                    column_span: 2,
                    visible: true,
                    style: None,
                },
                GridCell {
                    id: "tail".into(),
                    label: DiagramLabel::new("Tail"),
                    shape: DiagramShape::Rect,
                    column_span: 2,
                    visible: true,
                    style: None,
                },
                GridCell {
                    id: "next".into(),
                    label: DiagramLabel::new("Next"),
                    shape: DiagramShape::Rect,
                    column_span: 1,
                    visible: true,
                    style: None,
                },
            ],
            connections: vec![GridConnection {
                from: "wide".into(),
                to: "tail".into(),
                label: None,
            }],
        };
        let layout = layout_grid_diagram(&diagram);
        assert_eq!(layout.nodes[0].width, CELL_WIDTH * 2.0 + COLUMN_GAP);
        assert_eq!(layout.nodes[1].width, CELL_WIDTH);
        assert!(layout.nodes[1].x > layout.nodes[0].x);
        assert!(layout.nodes[2].y > layout.nodes[0].y);
        assert_eq!(
            layout.edges[0].points[0].x,
            layout.nodes[0].x + layout.nodes[0].width
        );
    }
}
