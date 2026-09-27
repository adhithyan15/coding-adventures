//! A reusable CSS table formatting context over `layout-ir`.
//!
//! Producers attach computed values to `ext["table"]`; this crate owns only
//! anonymous-box normalization, slot placement, intrinsic track sizing, and
//! geometry. Recursive child layout remains supplied by the shared dispatcher.

use std::collections::HashMap;

use layout_ir::{
    Constraints, Content, ExtValue, LayoutNode, PositionedNode, SizeValue, TextMeasurer,
};
use layout_replaced::intrinsic_inline_size;

pub const VERSION: &str = "0.1.0";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TableLayout {
    #[default]
    Auto,
    Fixed,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BorderCollapse {
    #[default]
    Separate,
    Collapse,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CaptionSide {
    #[default]
    Top,
    Bottom,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VerticalAlign {
    #[default]
    Middle,
    Top,
    Bottom,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TableContainerStyle {
    pub layout: TableLayout,
    pub border_collapse: BorderCollapse,
    pub border_spacing_x: f64,
    pub border_spacing_y: f64,
    pub caption_side: CaptionSide,
}

impl Default for TableContainerStyle {
    fn default() -> Self {
        Self {
            layout: TableLayout::Auto,
            border_collapse: BorderCollapse::Separate,
            border_spacing_x: 2.0,
            border_spacing_y: 2.0,
            caption_side: CaptionSide::Top,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TableItemStyle {
    pub column_span: usize,
    pub row_span: usize,
    pub vertical_align: VerticalAlign,
    pub section_kind: Option<String>,
}

impl Default for TableItemStyle {
    fn default() -> Self {
        Self {
            column_span: 1,
            row_span: 1,
            vertical_align: VerticalAlign::Middle,
            section_kind: None,
        }
    }
}

impl TableContainerStyle {
    pub fn from_node(node: &LayoutNode) -> Self {
        let Some(values) = table_map(node) else {
            return Self::default();
        };
        Self {
            layout: match string(values, "layout") {
                Some("fixed") => TableLayout::Fixed,
                _ => TableLayout::Auto,
            },
            border_collapse: match string(values, "borderCollapse") {
                Some("collapse") => BorderCollapse::Collapse,
                _ => BorderCollapse::Separate,
            },
            border_spacing_x: number(values, "borderSpacingX").unwrap_or(2.0).max(0.0),
            border_spacing_y: number(values, "borderSpacingY").unwrap_or(2.0).max(0.0),
            caption_side: match string(values, "captionSide") {
                Some("bottom") => CaptionSide::Bottom,
                _ => CaptionSide::Top,
            },
        }
    }

    pub fn to_ext(self) -> ExtValue {
        ExtValue::Map(HashMap::from([
            (
                "layout".into(),
                ExtValue::Str(
                    match self.layout {
                        TableLayout::Auto => "auto",
                        TableLayout::Fixed => "fixed",
                    }
                    .into(),
                ),
            ),
            (
                "borderCollapse".into(),
                ExtValue::Str(
                    match self.border_collapse {
                        BorderCollapse::Separate => "separate",
                        BorderCollapse::Collapse => "collapse",
                    }
                    .into(),
                ),
            ),
            (
                "borderSpacingX".into(),
                ExtValue::Float(self.border_spacing_x),
            ),
            (
                "borderSpacingY".into(),
                ExtValue::Float(self.border_spacing_y),
            ),
            (
                "captionSide".into(),
                ExtValue::Str(
                    match self.caption_side {
                        CaptionSide::Top => "top",
                        CaptionSide::Bottom => "bottom",
                    }
                    .into(),
                ),
            ),
        ]))
    }
}

impl TableItemStyle {
    pub fn from_node(node: &LayoutNode) -> Self {
        let Some(values) = table_map(node) else {
            return Self::default();
        };
        Self {
            column_span: positive_integer(values, "columnSpan").unwrap_or(1),
            row_span: positive_integer(values, "rowSpan").unwrap_or(1),
            vertical_align: match string(values, "verticalAlign") {
                Some("top") => VerticalAlign::Top,
                Some("bottom") => VerticalAlign::Bottom,
                _ => VerticalAlign::Middle,
            },
            section_kind: string(values, "sectionKind").map(str::to_string),
        }
    }

    pub fn to_ext(&self) -> ExtValue {
        let mut values = HashMap::from([
            (
                "columnSpan".into(),
                ExtValue::Int(self.column_span.max(1) as i64),
            ),
            ("rowSpan".into(), ExtValue::Int(self.row_span.max(1) as i64)),
            (
                "verticalAlign".into(),
                ExtValue::Str(
                    match self.vertical_align {
                        VerticalAlign::Top => "top",
                        VerticalAlign::Middle => "middle",
                        VerticalAlign::Bottom => "bottom",
                    }
                    .into(),
                ),
            ),
        ]);
        if let Some(kind) = &self.section_kind {
            values.insert("sectionKind".into(), ExtValue::Str(kind.clone()));
        }
        ExtValue::Map(values)
    }
}

pub fn table_ext(container: TableContainerStyle, item: &TableItemStyle) -> ExtValue {
    let ExtValue::Map(mut values) = container.to_ext() else {
        unreachable!()
    };
    let ExtValue::Map(item_values) = item.to_ext() else {
        unreachable!()
    };
    values.extend(item_values);
    ExtValue::Map(values)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableDiagnostic {
    pub key: String,
    pub message: String,
}

pub fn diagnostics(node: &LayoutNode) -> Vec<TableDiagnostic> {
    let Some(values) = table_map(node) else {
        return Vec::new();
    };
    let mut result = Vec::new();
    for key in ["columnSpan", "rowSpan"] {
        if values.get(key).is_some() && positive_integer(values, key).is_none() {
            result.push(TableDiagnostic {
                key: key.into(),
                message: "expected a positive table span".into(),
            });
        }
    }
    for key in ["borderSpacingX", "borderSpacingY"] {
        if values.get(key).is_some() && number(values, key).is_none() {
            result.push(TableDiagnostic {
                key: key.into(),
                message: "expected finite numeric border spacing".into(),
            });
        }
    }
    result
}

#[derive(Clone)]
struct Row<'a> {
    node: Option<&'a LayoutNode>,
    cells: Vec<&'a LayoutNode>,
    section_rank: u8,
    source_order: usize,
}

#[derive(Clone, Copy)]
struct CellPlacement<'a> {
    node: &'a LayoutNode,
    column_start: usize,
    column_end: usize,
    row_start: usize,
    row_end: usize,
}

/// Lay out a table container while delegating cell and caption subtree layout.
pub fn layout_table_with<M, F>(
    container: &LayoutNode,
    constraints: Constraints,
    measurer: &M,
    mut layout_child: F,
) -> PositionedNode
where
    M: TextMeasurer,
    F: FnMut(&LayoutNode, Constraints) -> PositionedNode,
{
    let style = TableContainerStyle::from_node(container);
    let padding = container.padding.unwrap_or_default();
    let spacing_x = if style.border_collapse == BorderCollapse::Collapse {
        0.0
    } else {
        style.border_spacing_x
    };
    let spacing_y = if style.border_collapse == BorderCollapse::Collapse {
        0.0
    } else {
        style.border_spacing_y
    };
    let (captions, column_nodes, mut rows) = normalize_table(container);
    rows.sort_by_key(|row| (row.section_rank, row.source_order));
    let placements = place_cells(&rows);
    let column_count = placements
        .iter()
        .flat_map(|row| row.iter())
        .map(|cell| cell.column_end)
        .max()
        .unwrap_or(1);
    let row_count = rows.len().max(1);

    let horizontal_padding = padding.left + padding.right;
    let vertical_padding = padding.top + padding.bottom;
    let known_width = resolve_size(container.width, constraints.max_width)
        .map(|value| clamp_dimension(value, container.min_width, container.max_width));
    let constrained_width = if constraints.max_width.is_finite() {
        Some(constraints.max_width)
    } else {
        None
    };
    let available_outer = known_width.or_else(|| {
        (!matches!(container.width, Some(SizeValue::Wrap)))
            .then_some(constrained_width)
            .flatten()
    });
    let available_columns = available_outer.map(|width| {
        (width - horizontal_padding - spacing_x * (column_count.saturating_add(1) as f64)).max(0.0)
    });

    let mut minimums = vec![0.0_f64; column_count];
    let mut preferred = vec![0.0_f64; column_count];
    apply_column_hints(
        &column_nodes,
        available_columns,
        &mut minimums,
        &mut preferred,
    );
    for row in &placements {
        for cell in row {
            let min = min_content_width(cell.node, measurer);
            let max = preferred_width(cell.node, measurer).max(min);
            if cell.column_end == cell.column_start + 1 {
                minimums[cell.column_start] = minimums[cell.column_start].max(min);
                preferred[cell.column_start] = preferred[cell.column_start].max(max);
            }
        }
    }
    for row in &placements {
        for cell in row
            .iter()
            .filter(|cell| cell.column_end > cell.column_start + 1)
        {
            grow_span(
                &mut minimums,
                cell.column_start,
                cell.column_end,
                min_content_width(cell.node, measurer),
            );
            grow_span(
                &mut preferred,
                cell.column_start,
                cell.column_end,
                preferred_width(cell.node, measurer),
            );
        }
    }
    for (min, max) in minimums.iter().zip(preferred.iter_mut()) {
        *max = (*max).max(*min);
    }
    let first_row = placements.first().map(Vec::as_slice).unwrap_or_default();
    let column_sizes = resolve_columns(
        style.layout,
        available_columns,
        &minimums,
        &preferred,
        first_row,
    );
    let columns_width: f64 = column_sizes.iter().sum();
    let natural_width =
        columns_width + horizontal_padding + spacing_x * (column_count.saturating_add(1) as f64);
    let width = known_width.unwrap_or_else(|| {
        clamp_dimension(
            natural_width,
            Some(container.min_width.unwrap_or(constraints.min_width)),
            Some(container.max_width.unwrap_or(constraints.max_width)),
        )
    });

    let column_offsets = track_offsets(&column_sizes, spacing_x, padding.left + spacing_x);
    let mut measured_cells: Vec<Vec<(CellPlacement<'_>, PositionedNode)>> = Vec::new();
    let mut row_heights = vec![0.0_f64; row_count];
    for (row_index, row) in placements.iter().enumerate() {
        let mut measured_row = Vec::new();
        for cell in row {
            let cell_width =
                span_size(&column_sizes, cell.column_start, cell.column_end, spacing_x);
            let measured = layout_child(cell.node, width_constraints(cell_width));
            if cell.row_end == cell.row_start + 1 {
                row_heights[row_index] = row_heights[row_index].max(measured.height);
            }
            measured_row.push((*cell, measured));
        }
        if let Some(SizeValue::Fixed(height)) = rows[row_index].node.and_then(|node| node.height) {
            row_heights[row_index] = row_heights[row_index].max(height);
        }
        measured_cells.push(measured_row);
    }
    for row in &measured_cells {
        for (cell, measured) in row
            .iter()
            .filter(|(cell, _)| cell.row_end > cell.row_start + 1)
        {
            let current = span_size(&row_heights, cell.row_start, cell.row_end, spacing_y);
            if measured.height > current {
                let share = (measured.height - current) / (cell.row_end - cell.row_start) as f64;
                for height in &mut row_heights[cell.row_start..cell.row_end] {
                    *height += share;
                }
            }
        }
    }

    let caption_width = (width - horizontal_padding).max(0.0);
    let mut top_captions = Vec::new();
    let mut bottom_captions = Vec::new();
    for caption in captions {
        let positioned = layout_child(caption, width_constraints(caption_width));
        if TableContainerStyle::from_node(caption).caption_side == CaptionSide::Bottom
            || style.caption_side == CaptionSide::Bottom
        {
            bottom_captions.push(positioned);
        } else {
            top_captions.push(positioned);
        }
    }
    let top_height: f64 = top_captions.iter().map(|caption| caption.height).sum();
    let rows_start = padding.top + top_height + spacing_y;
    let row_offsets = track_offsets(&row_heights, spacing_y, rows_start);
    let mut positioned_children = Vec::new();
    let mut caption_y = padding.top;
    for mut caption in top_captions {
        caption.x = padding.left;
        caption.y = caption_y;
        caption.width = caption_width;
        caption_y += caption.height;
        positioned_children.push(caption);
    }

    for (row_index, (row, cells)) in rows.iter().zip(measured_cells).enumerate() {
        let mut positioned_row = PositionedNode {
            x: padding.left,
            y: row_offsets[row_index],
            width: (width - horizontal_padding).max(0.0),
            height: row_heights[row_index],
            id: row.node.and_then(|node| node.id.clone()),
            content: None,
            children: Vec::new(),
            ext: row
                .node
                .map(|node| node.ext.clone())
                .unwrap_or_else(anonymous_row_ext),
        };
        for (cell, mut positioned) in cells {
            let cell_width =
                span_size(&column_sizes, cell.column_start, cell.column_end, spacing_x);
            let cell_height = span_size(&row_heights, cell.row_start, cell.row_end, spacing_y);
            let natural_height = positioned.height;
            let free = (cell_height - natural_height).max(0.0);
            let offset = match TableItemStyle::from_node(cell.node).vertical_align {
                VerticalAlign::Top => 0.0,
                VerticalAlign::Middle => free / 2.0,
                VerticalAlign::Bottom => free,
            };
            for child in &mut positioned.children {
                child.y += offset;
            }
            positioned.x = column_offsets[cell.column_start] - padding.left;
            positioned.y = offset;
            positioned.width = cell_width;
            positioned.height = cell_height;
            positioned_row.children.push(positioned);
        }
        positioned_children.push(positioned_row);
    }

    let rows_height =
        row_heights.iter().sum::<f64>() + spacing_y * (row_count.saturating_sub(1) as f64);
    let mut bottom_y = rows_start + rows_height + spacing_y;
    for mut caption in bottom_captions {
        caption.x = padding.left;
        caption.y = bottom_y;
        caption.width = caption_width;
        bottom_y += caption.height;
        positioned_children.push(caption);
    }
    let content_height = bottom_y + padding.bottom;
    let height = resolve_size(container.height, constraints.max_height)
        .unwrap_or(content_height.max(vertical_padding));
    PositionedNode {
        x: 0.0,
        y: 0.0,
        width,
        height: clamp_dimension(height, container.min_height, container.max_height),
        id: container.id.clone(),
        content: container.content.clone(),
        children: positioned_children,
        ext: container.ext.clone(),
    }
}

fn normalize_table<'a>(
    container: &'a LayoutNode,
) -> (Vec<&'a LayoutNode>, Vec<&'a LayoutNode>, Vec<Row<'a>>) {
    let mut captions = Vec::new();
    let mut columns = Vec::new();
    let mut rows = Vec::new();
    let mut anonymous_cells = Vec::new();
    let mut order = 0;
    for child in &container.children {
        match display(child) {
            Some("table-caption") => captions.push(child),
            Some("table-column") => columns.push(child),
            Some("table-column-group") if child.children.is_empty() => columns.push(child),
            Some("table-column-group") => columns.extend(child.children.iter()),
            Some("table-row") => {
                flush_anonymous_row(&mut rows, &mut anonymous_cells, order);
                rows.push(row_from_node(child, 1, order));
                order += 1;
            }
            Some("table-header-group" | "table-row-group" | "table-footer-group") => {
                flush_anonymous_row(&mut rows, &mut anonymous_cells, order);
                let rank = section_rank(child);
                for row in &child.children {
                    if display(row) == Some("table-row") {
                        rows.push(row_from_node(row, rank, order));
                        order += 1;
                    } else {
                        rows.push(Row {
                            node: None,
                            cells: vec![row],
                            section_rank: rank,
                            source_order: order,
                        });
                        order += 1;
                    }
                }
            }
            Some("table-cell") => anonymous_cells.push(child),
            _ => anonymous_cells.push(child),
        }
    }
    flush_anonymous_row(&mut rows, &mut anonymous_cells, order);
    (captions, columns, rows)
}

fn row_from_node(node: &LayoutNode, section_rank: u8, source_order: usize) -> Row<'_> {
    Row {
        node: Some(node),
        cells: node.children.iter().collect(),
        section_rank,
        source_order,
    }
}

fn flush_anonymous_row<'a>(rows: &mut Vec<Row<'a>>, cells: &mut Vec<&'a LayoutNode>, order: usize) {
    if cells.is_empty() {
        return;
    }
    rows.push(Row {
        node: None,
        cells: std::mem::take(cells),
        section_rank: 1,
        source_order: order,
    });
}

fn section_rank(node: &LayoutNode) -> u8 {
    match display(node) {
        Some("table-header-group") => 0,
        Some("table-footer-group") => 2,
        _ => match TableItemStyle::from_node(node).section_kind.as_deref() {
            Some("thead") => 0,
            Some("tfoot") => 2,
            _ => 1,
        },
    }
}

fn place_cells<'a>(rows: &[Row<'a>]) -> Vec<Vec<CellPlacement<'a>>> {
    let mut result = Vec::with_capacity(rows.len());
    let mut occupied_until: Vec<usize> = Vec::new();
    for (row_index, row) in rows.iter().enumerate() {
        let mut cursor = 0;
        let mut placed = Vec::new();
        for cell in &row.cells {
            let item = TableItemStyle::from_node(cell);
            let column_span = item.column_span.clamp(1, 1024);
            let row_span = item.row_span.clamp(1, 65_534);
            loop {
                if occupied_until.len() < cursor + column_span {
                    occupied_until.resize(cursor + column_span, 0);
                }
                if occupied_until[cursor..cursor + column_span]
                    .iter()
                    .all(|until| *until <= row_index)
                {
                    break;
                }
                cursor += 1;
            }
            let placement = CellPlacement {
                node: cell,
                column_start: cursor,
                column_end: cursor + column_span,
                row_start: row_index,
                row_end: (row_index + row_span).min(rows.len().max(row_index + 1)),
            };
            for slot in &mut occupied_until[cursor..cursor + column_span] {
                *slot = row_index + row_span;
            }
            cursor += column_span;
            placed.push(placement);
        }
        result.push(placed);
    }
    result
}

fn resolve_columns(
    layout: TableLayout,
    available: Option<f64>,
    minimums: &[f64],
    preferred: &[f64],
    first_row: &[CellPlacement<'_>],
) -> Vec<f64> {
    let Some(target) = available else {
        return preferred.to_vec();
    };
    if layout == TableLayout::Fixed {
        let mut sizes = vec![0.0_f64; minimums.len()];
        for cell in first_row {
            if let Some(width) = fixed_width(cell.node, target) {
                let share = width / (cell.column_end - cell.column_start) as f64;
                for size in &mut sizes[cell.column_start..cell.column_end] {
                    *size = (*size).max(share);
                }
            }
        }
        let assigned: f64 = sizes.iter().sum();
        let empty = sizes.iter().filter(|size| **size == 0.0).count();
        if empty > 0 {
            let share = (target - assigned).max(0.0) / empty as f64;
            for size in &mut sizes {
                if *size == 0.0 {
                    *size = share;
                }
            }
        } else if assigned < target && !sizes.is_empty() {
            let extra = (target - assigned) / sizes.len() as f64;
            for size in &mut sizes {
                *size += extra;
            }
        }
        return sizes;
    }
    let min_total: f64 = minimums.iter().sum();
    let preferred_total: f64 = preferred.iter().sum();
    if target <= min_total || minimums.is_empty() {
        return minimums.to_vec();
    }
    if target < preferred_total {
        let ratio = (target - min_total) / (preferred_total - min_total).max(f64::EPSILON);
        return minimums
            .iter()
            .zip(preferred)
            .map(|(min, max)| min + (max - min) * ratio)
            .collect();
    }
    let extra = (target - preferred_total) / preferred.len() as f64;
    preferred.iter().map(|width| width + extra).collect()
}

fn apply_column_hints(
    columns: &[&LayoutNode],
    available: Option<f64>,
    minimums: &mut [f64],
    preferred: &mut [f64],
) {
    let mut index = 0;
    for column in columns {
        let span = TableItemStyle::from_node(column).column_span.max(1);
        if let Some(width) = fixed_width(column, available.unwrap_or(0.0)) {
            for target in index..(index + span).min(preferred.len()) {
                preferred[target] = preferred[target].max(width / span as f64);
                minimums[target] = minimums[target].max(width / span as f64);
            }
        }
        index += span;
    }
}

fn grow_span(values: &mut [f64], start: usize, end: usize, required: f64) {
    if start >= values.len() || end > values.len() || start >= end {
        return;
    }
    let current: f64 = values[start..end].iter().sum();
    if required > current {
        let share = (required - current) / (end - start) as f64;
        for value in &mut values[start..end] {
            *value += share;
        }
    }
}

fn preferred_width<M: TextMeasurer>(node: &LayoutNode, measurer: &M) -> f64 {
    if let Some(width) = fixed_width(node, 0.0) {
        return width;
    }
    let padding = node.padding.unwrap_or_default();
    let content = match &node.content {
        Some(Content::Text(text)) => measurer.measure(&text.value, &text.font, None).width,
        Some(Content::Image(_)) => intrinsic_inline_size(node),
        None => node
            .children
            .iter()
            .map(|child| preferred_width(child, measurer))
            .sum(),
    };
    clamp_dimension(
        content + padding.left + padding.right,
        node.min_width,
        node.max_width,
    )
}

fn min_content_width<M: TextMeasurer>(node: &LayoutNode, measurer: &M) -> f64 {
    let padding = node.padding.unwrap_or_default();
    let content = match &node.content {
        Some(Content::Text(text)) => text
            .value
            .split_whitespace()
            .map(|word| measurer.measure(word, &text.font, None).width)
            .fold(0.0, f64::max),
        Some(Content::Image(_)) => intrinsic_inline_size(node),
        None => node
            .children
            .iter()
            .map(|child| min_content_width(child, measurer))
            .fold(0.0, f64::max),
    };
    (content + padding.left + padding.right).max(node.min_width.unwrap_or(0.0))
}

fn fixed_width(node: &LayoutNode, available: f64) -> Option<f64> {
    match node.width {
        Some(SizeValue::Fixed(value)) => Some(value.max(0.0)),
        Some(SizeValue::Percent(value)) if available > 0.0 => Some(available * value.max(0.0)),
        _ => None,
    }
}

fn resolve_size(value: Option<SizeValue>, available: f64) -> Option<f64> {
    match value {
        Some(SizeValue::Fixed(value)) => Some(value.max(0.0)),
        Some(SizeValue::Percent(fraction)) if available.is_finite() => {
            Some(available * fraction.max(0.0))
        }
        Some(SizeValue::Fill) if available.is_finite() => Some(available),
        _ => None,
    }
}

fn clamp_dimension(value: f64, min: Option<f64>, max: Option<f64>) -> f64 {
    value
        .max(min.unwrap_or(0.0))
        .min(max.unwrap_or(f64::MAX))
        .max(0.0)
}

fn width_constraints(width: f64) -> Constraints {
    Constraints {
        min_width: width,
        max_width: width,
        min_height: 0.0,
        max_height: f64::MAX,
    }
}

fn track_offsets(sizes: &[f64], gap: f64, start: f64) -> Vec<f64> {
    let mut offsets = Vec::with_capacity(sizes.len());
    let mut cursor = start;
    for size in sizes {
        offsets.push(cursor);
        cursor += size + gap;
    }
    offsets
}

fn span_size(sizes: &[f64], start: usize, end: usize, gap: f64) -> f64 {
    sizes
        .get(start..end)
        .unwrap_or_default()
        .iter()
        .sum::<f64>()
        + gap * end.saturating_sub(start + 1) as f64
}

fn anonymous_row_ext() -> HashMap<String, ExtValue> {
    HashMap::from([
        (
            "block".into(),
            ExtValue::Map(HashMap::from([(
                "display".into(),
                ExtValue::Str("table-row".into()),
            )])),
        ),
        (
            "table".into(),
            table_ext(TableContainerStyle::default(), &TableItemStyle::default()),
        ),
    ])
}

fn display(node: &LayoutNode) -> Option<&str> {
    let ExtValue::Map(values) = node.ext.get("block")? else {
        return None;
    };
    string(values, "display")
}

fn table_map(node: &LayoutNode) -> Option<&HashMap<String, ExtValue>> {
    match node.ext.get("table") {
        Some(ExtValue::Map(values)) => Some(values),
        _ => None,
    }
}

fn string<'a>(values: &'a HashMap<String, ExtValue>, key: &str) -> Option<&'a str> {
    match values.get(key) {
        Some(ExtValue::Str(value)) => Some(value),
        _ => None,
    }
}

fn number(values: &HashMap<String, ExtValue>, key: &str) -> Option<f64> {
    match values.get(key) {
        Some(ExtValue::Float(value)) if value.is_finite() => Some(*value),
        Some(ExtValue::Int(value)) => Some(*value as f64),
        _ => None,
    }
}

fn positive_integer(values: &HashMap<String, ExtValue>, key: &str) -> Option<usize> {
    match values.get(key) {
        Some(ExtValue::Int(value)) if *value > 0 => usize::try_from(*value).ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use layout_ir::{font_spec, ImageContent, ImageFit, MeasureResult, TextAlign, TextContent};

    struct Mono;

    impl TextMeasurer for Mono {
        fn measure(
            &self,
            text: &str,
            font: &layout_ir::FontSpec,
            max_width: Option<f64>,
        ) -> MeasureResult {
            let width = text.chars().count() as f64 * font.size * 0.5;
            let lines = max_width
                .filter(|max| *max > 0.0)
                .map(|max| (width / max).ceil().max(1.0) as u32)
                .unwrap_or(1);
            MeasureResult {
                width: max_width.map_or(width, |max| width.min(max)),
                height: lines as f64 * font.size,
                baseline: font.size * 0.8,
                line_count: lines,
            }
        }
    }

    fn display_ext(value: &str) -> ExtValue {
        ExtValue::Map(HashMap::from([(
            "display".into(),
            ExtValue::Str(value.into()),
        )]))
    }

    fn node(display: &str, children: Vec<LayoutNode>) -> LayoutNode {
        LayoutNode::container(children).with_ext("block", display_ext(display))
    }

    fn text(value: &str) -> LayoutNode {
        LayoutNode::leaf_text(TextContent {
            value: value.into(),
            font: font_spec("Test", 10.0),
            color: layout_ir::color_black(),
            decoration: None,
            max_lines: None,
            wrap: true,
            text_align: TextAlign::Start,
        })
        .with_ext("block", display_ext("inline-text"))
    }

    fn cell(value: &str, colspan: usize, rowspan: usize) -> LayoutNode {
        node("table-cell", vec![text(value)]).with_ext(
            "table",
            table_ext(
                TableContainerStyle::default(),
                &TableItemStyle {
                    column_span: colspan,
                    row_span: rowspan,
                    ..TableItemStyle::default()
                },
            ),
        )
    }

    fn layout_child(node: &LayoutNode, constraints: Constraints) -> PositionedNode {
        let width = constraints.max_width;
        let height = if node.children.is_empty() { 10.0 } else { 20.0 };
        PositionedNode {
            x: 0.0,
            y: 0.0,
            width,
            height,
            id: node.id.clone(),
            content: node.content.clone(),
            children: Vec::new(),
            ext: node.ext.clone(),
        }
    }

    #[test]
    fn places_spans_without_collisions() {
        let table = node(
            "table",
            vec![
                node("table-row", vec![cell("A", 1, 2), cell("B", 2, 1)]),
                node("table-row", vec![cell("C", 1, 1), cell("D", 1, 1)]),
            ],
        )
        .with_width(SizeValue::Fixed(300.0));
        let positioned = layout_table_with(
            &table,
            Constraints {
                min_width: 0.0,
                max_width: 300.0,
                min_height: 0.0,
                max_height: f64::MAX,
            },
            &Mono,
            layout_child,
        );
        assert_eq!(positioned.children.len(), 2);
        assert_eq!(positioned.children[0].children.len(), 2);
        assert_eq!(positioned.children[1].children.len(), 2);
        assert!(positioned.children[1].children[0].x > 0.0);
        assert!(positioned.children[0].children[0].height > positioned.children[0].height);
    }

    #[test]
    fn header_and_footer_groups_are_reordered_around_body() {
        let section = |display: &str, id: &str| {
            node(
                display,
                vec![node("table-row", vec![cell(id, 1, 1)]).with_id(id)],
            )
        };
        let table = node(
            "table",
            vec![
                section("table-footer-group", "foot"),
                section("table-row-group", "body"),
                section("table-header-group", "head"),
            ],
        )
        .with_width(SizeValue::Fixed(120.0));
        let positioned = layout_table_with(&table, width_constraints(120.0), &Mono, layout_child);
        assert_eq!(positioned.children[0].id.as_deref(), Some("head"));
        assert_eq!(positioned.children[1].id.as_deref(), Some("body"));
        assert_eq!(positioned.children[2].id.as_deref(), Some("foot"));
    }

    #[test]
    fn malformed_extensions_are_diagnostic() {
        let node = LayoutNode::empty().with_ext(
            "table",
            ExtValue::Map(HashMap::from([
                ("columnSpan".into(), ExtValue::Int(0)),
                ("borderSpacingX".into(), ExtValue::Str("wide".into())),
            ])),
        );
        assert_eq!(diagnostics(&node).len(), 2);
    }

    #[test]
    fn inline_table_shrink_wraps_intrinsic_columns() {
        let table = node(
            "inline-table",
            vec![node("table-row", vec![cell("A", 1, 1), cell("BBBB", 1, 1)])],
        )
        .with_width(SizeValue::Wrap);
        let positioned = layout_table_with(
            &table,
            Constraints {
                min_width: 0.0,
                max_width: 300.0,
                min_height: 0.0,
                max_height: f64::MAX,
            },
            &Mono,
            layout_child,
        );
        assert_eq!(positioned.width, 31.0);
    }

    #[test]
    fn inline_table_uses_replaced_intrinsic_column_width() {
        let image = LayoutNode::leaf_image(ImageContent {
            src: "fixture.gif".into(),
            fit: ImageFit::Contain,
        })
        .with_ext(
            "replaced",
            layout_replaced::replaced_ext(Some(80.0), Some(40.0), None),
        );
        let table = node(
            "inline-table",
            vec![node("table-row", vec![node("table-cell", vec![image])])],
        )
        .with_width(SizeValue::Wrap);
        let positioned = layout_table_with(
            &table,
            Constraints {
                min_width: 0.0,
                max_width: 300.0,
                min_height: 0.0,
                max_height: f64::MAX,
            },
            &Mono,
            layout_child,
        );
        assert_eq!(
            (positioned.width, positioned.children[0].children[0].width),
            (84.0, 80.0)
        );
    }

    #[test]
    fn anonymous_row_and_collapsed_borders_share_track_edges() {
        let table = node("table", vec![cell("A", 1, 1), cell("B", 1, 1)])
            .with_width(SizeValue::Fixed(100.0))
            .with_ext(
                "table",
                table_ext(
                    TableContainerStyle {
                        border_collapse: BorderCollapse::Collapse,
                        ..TableContainerStyle::default()
                    },
                    &TableItemStyle::default(),
                ),
            );
        let positioned = layout_table_with(&table, width_constraints(100.0), &Mono, layout_child);
        assert_eq!(positioned.children.len(), 1);
        assert_eq!(positioned.children[0].children[0].x, 0.0);
        assert_eq!(positioned.children[0].children[1].x, 50.0);
        assert_eq!(positioned.children[0].children[1].width, 50.0);
    }
}
