//! Deterministic grid layout for Mermaid block diagrams.

pub const VERSION: &str = "0.6.0";

use std::collections::HashMap;

use diagram_ir::{
    resolve_style, resolve_style_with_base, DiagramDirection, DiagramStyle, GridCell, GridColumns, GridDiagram, GridGroup,
    LayoutedGraphDiagram, LayoutedGraphEdge, LayoutedGraphGroup, LayoutedGraphNode, Point, ResolvedDiagramStyle,
};

const PADDING: f64 = 24.0;
const CELL_WIDTH: f64 = 150.0;
const CELL_HEIGHT: f64 = 58.0;
const COLUMN_GAP: f64 = 24.0;
const ROW_GAP: f64 = 24.0;
const TITLE_INSET: f64 = 38.0;
const GROUP_HEADER: f64 = 52.0;
const GROUP_PADDING: f64 = 12.0;

#[derive(Clone, Copy)]
enum GridEntry<'a> {
    Cell(&'a GridCell),
    Group(&'a GridGroup),
}

type GridRow<'a> = (Vec<(GridEntry<'a>, usize, usize)>, f64);

/// Lay out a Mermaid block grid and its composite groups into shared graph geometry.
pub fn layout_grid_diagram(diagram: &GridDiagram) -> LayoutedGraphDiagram {
    let root_entries = grid_entries(diagram, None);
    let columns = match diagram.columns {
        GridColumns::Auto => root_entries.iter().map(|entry| entry_span(*entry)).sum::<usize>().max(1),
        GridColumns::Fixed(columns) => columns.max(1),
    };
    let title_inset = if diagram.title.is_some() {
        TITLE_INSET
    } else {
        0.0
    };
    let mut nodes = Vec::new();
    let mut groups = Vec::new();
    let mut positions = HashMap::new();
    let content_width = columns as f64 * CELL_WIDTH + (columns - 1) as f64 * COLUMN_GAP;
    let content_height = layout_grid_entries(
        diagram,
        None,
        PADDING,
        PADDING + title_inset,
        content_width,
        columns,
        &mut nodes,
        &mut groups,
        &mut positions,
    );

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
            let (from, from_width, from_height) = positions.get(&connection.from)?;
            let (to, to_width, to_height) = positions.get(&connection.to)?;
            let (start, end) = connection_endpoints(from, *from_width, *from_height, to, *to_width, *to_height);
            Some(LayoutedGraphEdge {
                id: None,
                from_node_id: connection.from.clone(),
                to_node_id: connection.to.clone(),
                kind: connection.kind.clone(),
                points: vec![start, end],
                label: connection.label.clone(),
                label_position: connection.label.as_ref().map(|_| {
                    edge_label_position(from, *from_width, *from_height, to, *to_width, *to_height)
                }),
                style: edge_style.clone(),
            })
        })
        .collect();

    LayoutedGraphDiagram {
        direction: DiagramDirection::Tb,
        requested_width: None,
        hide_empty_descriptions: false,
        title: diagram.title.clone(),
        accessibility_title: diagram.accessibility_title.clone(),
        accessibility_description: diagram.accessibility_description.clone(),
        links: Vec::new(),
        groups,
        width: PADDING * 2.0 + columns as f64 * CELL_WIDTH + (columns - 1) as f64 * COLUMN_GAP,
        height: PADDING * 2.0 + title_inset + content_height,
        nodes,
        edges,
    }
}

fn grid_entries<'a>(diagram: &'a GridDiagram, parent_id: Option<&str>) -> Vec<GridEntry<'a>> {
    let mut entries: Vec<_> = diagram.cells.iter()
        .filter(|cell| cell.parent_id.as_deref() == parent_id)
        .map(GridEntry::Cell)
        .chain(diagram.groups.iter().filter(|group| group.parent_id.as_deref() == parent_id).map(GridEntry::Group))
        .collect();
    entries.sort_by_key(|entry| match entry { GridEntry::Cell(cell) => cell.order, GridEntry::Group(group) => group.order });
    entries
}

fn entry_span(entry: GridEntry<'_>) -> usize {
    match entry { GridEntry::Cell(cell) => cell.column_span, GridEntry::Group(group) => group.column_span }
}

fn entry_height(diagram: &GridDiagram, entry: GridEntry<'_>, width: f64) -> f64 {
    match entry {
        GridEntry::Cell(_) => CELL_HEIGHT,
        GridEntry::Group(group) => {
            GROUP_HEADER + GROUP_PADDING + measure_grid_entries(diagram, Some(&group.id), width - GROUP_PADDING * 2.0, &group.columns)
        }
    }
}

fn measure_grid_entries(diagram: &GridDiagram, parent_id: Option<&str>, width: f64, columns: &GridColumns) -> f64 {
    let entries = grid_entries(diagram, parent_id);
    let column_count = match columns {
        GridColumns::Auto => entries.iter().map(|entry| entry_span(*entry)).sum::<usize>().max(1),
        GridColumns::Fixed(columns) => (*columns).max(1),
    };
    let rows = grid_rows(diagram, &entries, width, column_count);
    rows.iter().map(|row| row.1).sum::<f64>() + ROW_GAP * rows.len().saturating_sub(1) as f64
}

fn grid_rows<'a>(diagram: &GridDiagram, entries: &[GridEntry<'a>], width: f64, columns: usize) -> Vec<GridRow<'a>> {
    let column_width = (width - COLUMN_GAP * columns.saturating_sub(1) as f64) / columns as f64;
    let mut rows: Vec<GridRow<'a>> = Vec::new();
    let mut row = Vec::new();
    let mut occupied = 0usize;
    let mut row_height: f64 = 0.0;
    for entry in entries {
        if occupied == columns {
            rows.push((row, row_height));
            row = Vec::new(); occupied = 0; row_height = 0.0;
        }
        let span = entry_span(*entry).min(columns - occupied).max(1);
        let entry_width = span as f64 * column_width + span.saturating_sub(1) as f64 * COLUMN_GAP;
        row_height = row_height.max(entry_height(diagram, *entry, entry_width));
        row.push((*entry, occupied, span));
        occupied += span;
    }
    if !row.is_empty() { rows.push((row, row_height)); }
    rows
}

#[allow(clippy::too_many_arguments)]
fn layout_grid_entries(
    diagram: &GridDiagram, parent_id: Option<&str>, x: f64, y: f64, width: f64, columns: usize,
    nodes: &mut Vec<LayoutedGraphNode>, groups: &mut Vec<LayoutedGraphGroup>,
    positions: &mut HashMap<String, (Point, f64, f64)>,
) -> f64 {
    let entries = grid_entries(diagram, parent_id);
    let rows = grid_rows(diagram, &entries, width, columns);
    let column_width = (width - COLUMN_GAP * columns.saturating_sub(1) as f64) / columns as f64;
    let mut row_y = y;
    for (row, row_height) in &rows {
        for (entry, column, span) in row {
            let entry_x = x + *column as f64 * (column_width + COLUMN_GAP);
            let entry_width = *span as f64 * column_width + span.saturating_sub(1) as f64 * COLUMN_GAP;
            match entry {
                GridEntry::Cell(cell) => {
                    positions.insert(cell.id.clone(), (Point { x: entry_x + entry_width / 2.0, y: row_y + CELL_HEIGHT / 2.0 }, entry_width, CELL_HEIGHT));
                    if cell.visible {
                        nodes.push(LayoutedGraphNode { id: cell.id.clone(), label: cell.label.clone(), shape: cell.shape.clone(),
                            x: entry_x, y: row_y, width: entry_width, height: CELL_HEIGHT,
                            style: resolve_style_with_base(cell.style.as_ref(), resolve_style(Some(&grid_style(nodes.len())))) });
                    }
                }
                GridEntry::Group(group) => {
                    let height = entry_height(diagram, *entry, entry_width);
                    positions.insert(group.id.clone(), (Point { x: entry_x + entry_width / 2.0, y: row_y + height / 2.0 }, entry_width, height));
                    groups.push(LayoutedGraphGroup { id: group.id.clone(), label: group.label.clone(), parent_id: group.parent_id.clone(),
                        x: entry_x, y: row_y, width: entry_width, height, divider_y: Vec::new(), direction: None,
                        style: resolve_style(Some(&group_style())) });
                    let child_entries = grid_entries(diagram, Some(&group.id));
                    let child_columns = match group.columns {
                        GridColumns::Auto => child_entries.iter().map(|entry| entry_span(*entry)).sum::<usize>().max(1),
                        GridColumns::Fixed(columns) => columns.max(1),
                    };
                    layout_grid_entries(diagram, Some(&group.id), entry_x + GROUP_PADDING, row_y + GROUP_HEADER,
                        entry_width - GROUP_PADDING * 2.0, child_columns, nodes, groups, positions);
                }
            }
        }
        row_y += *row_height + ROW_GAP;
    }
    rows.iter().map(|row| row.1).sum::<f64>() + ROW_GAP * rows.len().saturating_sub(1) as f64
}

fn edge_label_position(from: &Point, from_width: f64, from_height: f64, to: &Point, to_width: f64, to_height: f64) -> Point {
    let center_x = (from.x + to.x) / 2.0;
    let center_y = (from.y + to.y) / 2.0;
    if (to.x - from.x).abs() >= (to.y - from.y).abs() {
        Point { x: center_x, y: (from.y - from_height / 2.0).min(to.y - to_height / 2.0) - 4.0 }
    } else {
        let half_width = from_width.max(to_width) / 2.0;
        let left = center_x - half_width;
        let x = if left >= PADDING + 55.0 { left - 55.0 } else { center_x + half_width + 55.0 };
        Point { x, y: center_y }
    }
}

fn connection_endpoints(
    from: &Point,
    from_width: f64,
    from_height: f64,
    to: &Point,
    to_width: f64,
    to_height: f64,
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
                y: from.y + direction * from_height / 2.0,
            },
            Point {
                x: to.x,
                y: to.y - direction * to_height / 2.0,
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

fn group_style() -> DiagramStyle {
    DiagramStyle {
        fill: Some("#f8fafc".into()),
        stroke: Some("#94a3b8".into()),
        text_color: Some("#334155".into()),
        corner_radius: Some(8.0),
        ..DiagramStyle::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use diagram_ir::{DiagramLabel, DiagramShape, EdgeKind, GridCell, GridColumns, GridConnection, GridGroup};

    #[test]
    fn places_cells_in_authored_grid_slots() {
        let diagram = GridDiagram {
            columns: GridColumns::Fixed(2),
            title: None,
            accessibility_title: None,
            accessibility_description: None,
            cells: ["a", "b", "c"]
                .into_iter()
                .enumerate()
                .map(|(order, id)| GridCell {
                    id: id.into(),
                    label: DiagramLabel::new(id),
                    shape: DiagramShape::RoundedRect,
                    column_span: 1,
                    parent_id: None,
                    order,
                    visible: true,
                    style: None,
                })
                .collect(),
            groups: Vec::new(),
            connections: Vec::new(),
        };
        let layout = layout_grid_diagram(&diagram);
        assert_eq!(layout.nodes.len(), 3);
        assert!(layout.nodes[0].x < layout.nodes[1].x);
        assert!(layout.nodes[2].y > layout.nodes[0].y);
    }

    #[test]
    fn auto_columns_place_all_authored_slots_in_one_row() {
        let diagram = GridDiagram {
            columns: GridColumns::Auto,
            title: None, accessibility_title: None, accessibility_description: None,
            cells: ["a", "b", "c"].into_iter().enumerate().map(|(order, id)| GridCell {
                id: id.into(), label: DiagramLabel::new(id), shape: DiagramShape::Rect,
                column_span: 1, parent_id: None, order, visible: true, style: None,
            }).collect(),
            groups: Vec::new(),
            connections: Vec::new(),
        };
        let layout = layout_grid_diagram(&diagram);
        assert_eq!(layout.nodes[0].y, layout.nodes[2].y);
        assert_eq!(layout.width, PADDING * 2.0 + 3.0 * CELL_WIDTH + 2.0 * COLUMN_GAP);
    }

    #[test]
    fn routes_connections_to_cell_boundaries() {
        let diagram = GridDiagram {
            columns: GridColumns::Fixed(2),
            title: None,
            accessibility_title: None,
            accessibility_description: None,
            cells: ["a", "b"]
                .into_iter()
                .enumerate()
                .map(|(order, id)| GridCell {
                    id: id.into(),
                    label: DiagramLabel::new(id),
                    shape: DiagramShape::Rect,
                    column_span: 1,
                    parent_id: None,
                    order,
                    visible: true,
                    style: None,
                })
                .collect(),
            groups: Vec::new(),
            connections: vec![GridConnection {
                from: "a".into(),
                to: "b".into(),
                kind: EdgeKind::Directed,
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
            columns: GridColumns::Fixed(3),
            title: None,
            accessibility_title: None,
            accessibility_description: None,
            cells: vec![
                GridCell {
                    id: "wide".into(),
                    label: DiagramLabel::new("Wide"),
                    shape: DiagramShape::Rect,
                    column_span: 2,
                    parent_id: None,
                    order: 0,
                    visible: true,
                    style: None,
                },
                GridCell {
                    id: "tail".into(),
                    label: DiagramLabel::new("Tail"),
                    shape: DiagramShape::Rect,
                    column_span: 2,
                    parent_id: None,
                    order: 1,
                    visible: true,
                    style: None,
                },
                GridCell {
                    id: "next".into(),
                    label: DiagramLabel::new("Next"),
                    shape: DiagramShape::Rect,
                    column_span: 1,
                    parent_id: None,
                    order: 2,
                    visible: true,
                    style: None,
                },
            ],
            groups: Vec::new(),
            connections: vec![GridConnection {
                from: "wide".into(),
                to: "tail".into(),
                kind: EdgeKind::Directed,
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

    #[test]
    fn resolves_authored_cell_style_over_grid_defaults() {
        let diagram = GridDiagram {
            columns: GridColumns::Fixed(1), title: None, accessibility_title: None, accessibility_description: None,
            cells: vec![GridCell {
                id: "styled".into(), label: DiagramLabel::new("Styled"), shape: DiagramShape::Rect,
                column_span: 1, parent_id: None, order: 0, visible: true,
                style: Some(DiagramStyle { fill: Some("#123456".into()), stroke_width: Some(5.0), ..DiagramStyle::default() }),
            }], groups: Vec::new(), connections: Vec::new(),
        };
        let style = &layout_grid_diagram(&diagram).nodes[0].style;
        assert_eq!(style.fill, "#123456");
        assert_eq!(style.stroke, "#0284c7");
        assert_eq!(style.stroke_width, 5.0);
    }

    #[test]
    fn lays_out_composite_groups_around_their_children() {
        let diagram = GridDiagram {
            columns: GridColumns::Fixed(2), title: None, accessibility_title: None, accessibility_description: None,
            cells: vec![GridCell { id: "inside".into(), label: DiagramLabel::new("Inside"), shape: DiagramShape::Rect,
                column_span: 1, parent_id: Some("nested".into()), order: 0, visible: true, style: None }],
            groups: vec![
                GridGroup { id: "pipeline".into(), label: DiagramLabel::new("pipeline"), parent_id: None,
                    columns: GridColumns::Fixed(1), column_span: 2, order: 0 },
                GridGroup { id: "nested".into(), label: DiagramLabel::new(""), parent_id: Some("pipeline".into()),
                    columns: GridColumns::Fixed(1), column_span: 1, order: 0 },
            ], connections: Vec::new(),
        };
        let layout = layout_grid_diagram(&diagram);
        assert_eq!(layout.groups.len(), 2);
        assert!(layout.groups[1].x > layout.groups[0].x);
        assert!(layout.groups[1].y > layout.groups[0].y);
        assert!(layout.nodes[0].x > layout.groups[1].x);
        assert!(layout.nodes[0].y > layout.groups[1].y);
        assert!(layout.nodes[0].x + layout.nodes[0].width < layout.groups[1].x + layout.groups[1].width);
        assert!(layout.nodes[0].y + layout.nodes[0].height < layout.groups[1].y + layout.groups[1].height);
    }

    #[test]
    fn positions_edge_labels_outside_node_bounds() {
        let horizontal = edge_label_position(&Point { x: 100.0, y: 100.0 }, 80.0, 58.0, &Point { x: 240.0, y: 100.0 }, 80.0, 58.0);
        assert_eq!(horizontal, Point { x: 170.0, y: 67.0 });
        let vertical = edge_label_position(&Point { x: 150.0, y: 100.0 }, 80.0, 58.0, &Point { x: 150.0, y: 200.0 }, 80.0, 58.0);
        assert_eq!(vertical, Point { x: 55.0, y: 150.0 });
    }

}
