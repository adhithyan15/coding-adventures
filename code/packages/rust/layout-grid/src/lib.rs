//! A reusable CSS grid formatting context over `layout-ir`.
//!
//! CSS parsing and host rendering stay outside this crate. Producers attach a
//! typed `ext["grid"]` map and the shared block dispatcher supplies recursive
//! child layout through [`layout_grid_with`].

use std::collections::{HashMap, HashSet};

use layout_ir::{
    Constraints, Content, ExtValue, LayoutNode, PositionedNode, SizeValue, TextMeasurer,
};

pub const VERSION: &str = "0.1.0";

#[derive(Clone, Debug, PartialEq)]
pub enum GridTrack {
    Auto,
    MinContent,
    Points(f64),
    Percent(f64),
    Fraction(f64),
    MinMax(Box<GridTrack>, Box<GridTrack>),
}

impl GridTrack {
    pub fn parse(source: &str) -> Result<Self, GridDiagnostic> {
        let source = source.trim();
        if source == "auto" {
            return Ok(Self::Auto);
        }
        if source == "min-content" {
            return Ok(Self::MinContent);
        }
        if let Some(value) = source.strip_suffix("px") {
            return finite_non_negative(value, "track").map(Self::Points);
        }
        if let Some(value) = source.strip_suffix('%') {
            return finite_non_negative(value, "track").map(|value| Self::Percent(value / 100.0));
        }
        if let Some(value) = source.strip_suffix("fr") {
            return finite_non_negative(value, "track").map(Self::Fraction);
        }
        if source.starts_with("minmax(") && source.ends_with(')') {
            let inner = &source[7..source.len() - 1];
            let parts = split_top_level(inner, ',');
            if parts.len() == 2 {
                return Ok(Self::MinMax(
                    Box::new(Self::parse(parts[0])?),
                    Box::new(Self::parse(parts[1])?),
                ));
            }
        }
        Err(GridDiagnostic {
            key: "track".into(),
            message: format!("unsupported grid track `{source}`"),
        })
    }

    pub fn parse_list(source: &str) -> Result<Vec<Self>, GridDiagnostic> {
        let mut result = Vec::new();
        for token in split_track_tokens(source) {
            if token.starts_with("repeat(") && token.ends_with(')') {
                let inner = &token[7..token.len() - 1];
                let parts = split_top_level(inner, ',');
                if parts.len() != 2 {
                    return Err(GridDiagnostic {
                        key: "track-list".into(),
                        message: "repeat() requires a count and track list".into(),
                    });
                }
                let count = parts[0]
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| GridDiagnostic {
                        key: "track-list".into(),
                        message: "repeat() count must be a positive integer".into(),
                    })?;
                if count == 0 || count > 1024 {
                    return Err(GridDiagnostic {
                        key: "track-list".into(),
                        message: "repeat() count must be between 1 and 1024".into(),
                    });
                }
                let repeated = Self::parse_list(parts[1])?;
                for _ in 0..count {
                    result.extend(repeated.iter().cloned());
                }
            } else {
                result.push(Self::parse(token)?);
            }
        }
        if result.is_empty() {
            return Err(GridDiagnostic {
                key: "track-list".into(),
                message: "grid track list must not be empty".into(),
            });
        }
        Ok(result)
    }

    fn css(&self) -> String {
        match self {
            Self::Auto => "auto".into(),
            Self::MinContent => "min-content".into(),
            Self::Points(value) => format!("{value}px"),
            Self::Percent(value) => format!("{}%", value * 100.0),
            Self::Fraction(value) => format!("{value}fr"),
            Self::MinMax(min, max) => format!("minmax({}, {})", min.css(), max.css()),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GridAutoFlow {
    #[default]
    Row,
    Column,
    RowDense,
    ColumnDense,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GridAlignment {
    Start,
    End,
    Center,
    #[default]
    Stretch,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GridSelfAlignment {
    #[default]
    Auto,
    Start,
    End,
    Center,
    Stretch,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GridContentAlignment {
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
    #[default]
    Stretch,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GridContainerStyle {
    pub template_columns: Vec<GridTrack>,
    pub template_rows: Vec<GridTrack>,
    pub template_areas: Vec<Vec<Option<String>>>,
    pub auto_columns: GridTrack,
    pub auto_rows: GridTrack,
    pub auto_flow: GridAutoFlow,
    pub row_gap: f64,
    pub column_gap: f64,
    pub justify_items: GridAlignment,
    pub align_items: GridAlignment,
    pub justify_content: GridContentAlignment,
    pub align_content: GridContentAlignment,
}

impl Default for GridContainerStyle {
    fn default() -> Self {
        Self {
            template_columns: vec![GridTrack::Fraction(1.0)],
            template_rows: vec![GridTrack::Auto],
            template_areas: Vec::new(),
            auto_columns: GridTrack::Auto,
            auto_rows: GridTrack::Auto,
            auto_flow: GridAutoFlow::Row,
            row_gap: 0.0,
            column_gap: 0.0,
            justify_items: GridAlignment::Stretch,
            align_items: GridAlignment::Stretch,
            justify_content: GridContentAlignment::Stretch,
            align_content: GridContentAlignment::Stretch,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GridItemStyle {
    pub column_start: Option<usize>,
    pub column_end: Option<usize>,
    pub column_span: usize,
    pub row_start: Option<usize>,
    pub row_end: Option<usize>,
    pub row_span: usize,
    pub area: Option<String>,
    pub justify_self: GridSelfAlignment,
    pub align_self: GridSelfAlignment,
    pub order: i64,
}

impl GridContainerStyle {
    pub fn from_node(node: &LayoutNode) -> Self {
        let Some(values) = grid_map(node) else {
            return Self::default();
        };
        let mut style = Self::default();
        if let Some(value) = string(values, "templateColumns") {
            if let Ok(tracks) = GridTrack::parse_list(value) {
                style.template_columns = tracks;
            }
        }
        if let Some(value) = string(values, "templateRows") {
            if let Ok(tracks) = GridTrack::parse_list(value) {
                style.template_rows = tracks;
            }
        }
        if let Some(value) = string(values, "autoColumns") {
            if let Ok(track) = GridTrack::parse(value) {
                style.auto_columns = track;
            }
        }
        if let Some(value) = string(values, "autoRows") {
            if let Ok(track) = GridTrack::parse(value) {
                style.auto_rows = track;
            }
        }
        style.template_areas = area_rows(values.get("templateAreas"));
        style.auto_flow = match string(values, "autoFlow") {
            Some("column") => GridAutoFlow::Column,
            Some("row dense") | Some("dense") => GridAutoFlow::RowDense,
            Some("column dense") => GridAutoFlow::ColumnDense,
            _ => GridAutoFlow::Row,
        };
        style.row_gap = number(values, "rowGap").unwrap_or(0.0).max(0.0);
        style.column_gap = number(values, "columnGap").unwrap_or(0.0).max(0.0);
        style.justify_items = parse_alignment(string(values, "justifyItems"));
        style.align_items = parse_alignment(string(values, "alignItems"));
        style.justify_content = parse_content_alignment(string(values, "justifyContent"));
        style.align_content = parse_content_alignment(string(values, "alignContent"));
        style
    }

    pub fn to_ext(&self) -> ExtValue {
        let areas = self
            .template_areas
            .iter()
            .map(|row| {
                ExtValue::List(
                    row.iter()
                        .map(|name| ExtValue::Str(name.as_deref().unwrap_or(".").into()))
                        .collect(),
                )
            })
            .collect();
        ExtValue::Map(HashMap::from([
            (
                "templateColumns".into(),
                ExtValue::Str(track_list_css(&self.template_columns)),
            ),
            (
                "templateRows".into(),
                ExtValue::Str(track_list_css(&self.template_rows)),
            ),
            ("templateAreas".into(), ExtValue::List(areas)),
            ("autoColumns".into(), ExtValue::Str(self.auto_columns.css())),
            ("autoRows".into(), ExtValue::Str(self.auto_rows.css())),
            (
                "autoFlow".into(),
                ExtValue::Str(auto_flow_name(self.auto_flow).into()),
            ),
            ("rowGap".into(), ExtValue::Float(self.row_gap)),
            ("columnGap".into(), ExtValue::Float(self.column_gap)),
            (
                "justifyItems".into(),
                ExtValue::Str(alignment_name(self.justify_items).into()),
            ),
            (
                "alignItems".into(),
                ExtValue::Str(alignment_name(self.align_items).into()),
            ),
            (
                "justifyContent".into(),
                ExtValue::Str(content_alignment_name(self.justify_content).into()),
            ),
            (
                "alignContent".into(),
                ExtValue::Str(content_alignment_name(self.align_content).into()),
            ),
        ]))
    }
}

impl GridItemStyle {
    pub fn from_node(node: &LayoutNode) -> Self {
        let Some(values) = grid_map(node) else {
            return Self {
                column_span: 1,
                row_span: 1,
                ..Self::default()
            };
        };
        Self {
            column_start: positive_integer(values, "columnStart"),
            column_end: positive_integer(values, "columnEnd"),
            column_span: positive_integer(values, "columnSpan").unwrap_or(1),
            row_start: positive_integer(values, "rowStart"),
            row_end: positive_integer(values, "rowEnd"),
            row_span: positive_integer(values, "rowSpan").unwrap_or(1),
            area: string(values, "area")
                .filter(|value| !value.is_empty())
                .map(str::to_string),
            justify_self: parse_self_alignment(string(values, "justifySelf")),
            align_self: parse_self_alignment(string(values, "alignSelf")),
            order: integer(values, "order").unwrap_or(0),
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
                "justifySelf".into(),
                ExtValue::Str(self_alignment_name(self.justify_self).into()),
            ),
            (
                "alignSelf".into(),
                ExtValue::Str(self_alignment_name(self.align_self).into()),
            ),
            ("order".into(), ExtValue::Int(self.order)),
        ]);
        for (key, value) in [
            ("columnStart", self.column_start),
            ("columnEnd", self.column_end),
            ("rowStart", self.row_start),
            ("rowEnd", self.row_end),
        ] {
            if let Some(value) = value {
                values.insert(key.into(), ExtValue::Int(value as i64));
            }
        }
        if let Some(area) = &self.area {
            values.insert("area".into(), ExtValue::Str(area.clone()));
        }
        ExtValue::Map(values)
    }
}

pub fn grid_ext(container: &GridContainerStyle, item: &GridItemStyle) -> ExtValue {
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
pub struct GridDiagnostic {
    pub key: String,
    pub message: String,
}

pub fn diagnostics(node: &LayoutNode) -> Vec<GridDiagnostic> {
    let Some(values) = grid_map(node) else {
        return Vec::new();
    };
    let mut result = Vec::new();
    for key in ["templateColumns", "templateRows"] {
        if let Some(value) = values.get(key) {
            match value {
                ExtValue::Str(value) => {
                    if let Err(mut diagnostic) = GridTrack::parse_list(value) {
                        diagnostic.key = key.into();
                        result.push(diagnostic);
                    }
                }
                _ => result.push(GridDiagnostic {
                    key: key.into(),
                    message: "expected a grid track-list string".into(),
                }),
            }
        }
    }
    for key in ["rowGap", "columnGap"] {
        if values.get(key).is_some() && number(values, key).is_none() {
            result.push(GridDiagnostic {
                key: key.into(),
                message: "expected a finite numeric grid gap".into(),
            });
        }
    }
    for key in [
        "columnStart",
        "columnEnd",
        "columnSpan",
        "rowStart",
        "rowEnd",
        "rowSpan",
    ] {
        if values.get(key).is_some() && positive_integer(values, key).is_none() {
            result.push(GridDiagnostic {
                key: key.into(),
                message: "expected a positive grid line or span".into(),
            });
        }
    }
    result
}

#[derive(Clone, Copy, Debug)]
struct Placement {
    row_start: usize,
    row_end: usize,
    column_start: usize,
    column_end: usize,
}

#[derive(Clone)]
struct Item<'a> {
    node: &'a LayoutNode,
    style: GridItemStyle,
    placement: Placement,
    natural_width: f64,
    min_width: f64,
    natural_height: f64,
}

pub fn layout_grid_with<M, F>(
    container: &LayoutNode,
    constraints: Constraints,
    measurer: &M,
    mut layout_child: F,
) -> PositionedNode
where
    M: TextMeasurer,
    F: FnMut(&LayoutNode, Constraints) -> PositionedNode,
{
    let style = GridContainerStyle::from_node(container);
    let padding = container.padding.unwrap_or_default();
    let horizontal_padding = padding.left + padding.right;
    let vertical_padding = padding.top + padding.bottom;
    let known_width = resolve_container_size(container.width, constraints.max_width)
        .map(|value| clamp_dimension(value, container.min_width, container.max_width));
    let known_height = resolve_container_size(container.height, constraints.max_height)
        .map(|value| clamp_dimension(value, container.min_height, container.max_height));
    let available_width = known_width
        .map(|value| (value - horizontal_padding).max(0.0))
        .or_else(|| {
            constraints
                .max_width
                .is_finite()
                .then(|| (constraints.max_width - horizontal_padding).max(0.0))
        });
    let available_height = known_height.map(|value| (value - vertical_padding).max(0.0));

    let mut indexed: Vec<_> = container.children.iter().enumerate().collect();
    indexed.sort_by_key(|(index, child)| (GridItemStyle::from_node(child).order, *index));
    let placements = place_items(&indexed, &style);
    let max_column = placements
        .iter()
        .map(|value| value.column_end)
        .max()
        .unwrap_or(1);
    let max_row = placements
        .iter()
        .map(|value| value.row_end)
        .max()
        .unwrap_or(1);
    let mut columns = style.template_columns.clone();
    let mut rows = style.template_rows.clone();
    while columns.len() < max_column {
        columns.push(style.auto_columns.clone());
    }
    while rows.len() < max_row {
        rows.push(style.auto_rows.clone());
    }

    let mut items = Vec::with_capacity(indexed.len());
    for ((_, node), placement) in indexed.iter().zip(placements) {
        let natural = layout_child(node, intrinsic_constraints(available_width));
        let intrinsic_width = intrinsic_item_width(node, measurer).unwrap_or(natural.width);
        items.push(Item {
            node,
            style: GridItemStyle::from_node(node),
            placement,
            natural_width: intrinsic_width,
            min_width: min_content_width(node, measurer),
            natural_height: natural.height,
        });
    }

    let mut column_sizes = resolve_tracks(
        &columns,
        available_width,
        style.column_gap,
        &items,
        Axis::Columns,
    );
    for item in &mut items {
        let width = span_size(
            &column_sizes,
            item.placement.column_start,
            item.placement.column_end,
            style.column_gap,
        );
        let measured = layout_child(item.node, intrinsic_constraints(Some(width)));
        item.natural_height = measured.height;
    }
    let mut row_sizes = resolve_tracks(&rows, available_height, style.row_gap, &items, Axis::Rows);

    let natural_width = sum_tracks(&column_sizes, style.column_gap);
    let natural_height = sum_tracks(&row_sizes, style.row_gap);
    let inner_width = available_width.unwrap_or(natural_width);
    let inner_height = available_height.unwrap_or(natural_height);
    let width_free = (inner_width - natural_width).max(0.0);
    let height_free = (inner_height - natural_height).max(0.0);
    let (column_start, extra_column_gap) = content_offsets(
        style.justify_content,
        width_free,
        &mut column_sizes,
        &columns,
    );
    let (row_start, extra_row_gap) =
        content_offsets(style.align_content, height_free, &mut row_sizes, &rows);
    let column_offsets = track_offsets(
        &column_sizes,
        style.column_gap + extra_column_gap,
        column_start + padding.left,
    );
    let row_offsets = track_offsets(
        &row_sizes,
        style.row_gap + extra_row_gap,
        row_start + padding.top,
    );

    let mut positioned_children = Vec::with_capacity(items.len());
    for item in items {
        let cell_x = column_offsets[item.placement.column_start];
        let cell_y = row_offsets[item.placement.row_start];
        let cell_width = span_size(
            &column_sizes,
            item.placement.column_start,
            item.placement.column_end,
            style.column_gap + extra_column_gap,
        );
        let cell_height = span_size(
            &row_sizes,
            item.placement.row_start,
            item.placement.row_end,
            style.row_gap + extra_row_gap,
        );
        let justify = preserve_explicit_size(
            resolve_self(item.style.justify_self, style.justify_items),
            item.node.width,
        );
        let align = preserve_explicit_size(
            resolve_self(item.style.align_self, style.align_items),
            item.node.height,
        );
        let (x, width) = align_item(
            cell_x,
            cell_width,
            clamp_dimension(item.natural_width, item.node.min_width, item.node.max_width),
            justify,
        );
        let (y, height) = align_item(
            cell_y,
            cell_height,
            clamp_dimension(
                item.natural_height,
                item.node.min_height,
                item.node.max_height,
            ),
            align,
        );
        let mut positioned = layout_child(item.node, fixed_constraints(width, height));
        positioned.x = x;
        positioned.y = y;
        positioned.width = width;
        positioned.height = height;
        positioned_children.push(positioned);
    }

    let constrained_width_min = container
        .min_width
        .unwrap_or(0.0)
        .max(constraints.min_width);
    let constrained_width_max = container
        .max_width
        .unwrap_or(f64::MAX)
        .min(constraints.max_width)
        .max(constrained_width_min);
    let constrained_height_min = container
        .min_height
        .unwrap_or(0.0)
        .max(constraints.min_height);
    let constrained_height_max = container
        .max_height
        .unwrap_or(f64::MAX)
        .min(constraints.max_height)
        .max(constrained_height_min);
    let width = known_width.unwrap_or_else(|| {
        (sum_tracks(&column_sizes, style.column_gap + extra_column_gap) + horizontal_padding)
            .clamp(constrained_width_min, constrained_width_max)
    });
    let height = known_height.unwrap_or_else(|| {
        (sum_tracks(&row_sizes, style.row_gap + extra_row_gap) + vertical_padding)
            .clamp(constrained_height_min, constrained_height_max)
    });
    PositionedNode {
        x: 0.0,
        y: 0.0,
        width,
        height,
        id: container.id.clone(),
        content: container.content.clone(),
        children: positioned_children,
        ext: container.ext.clone(),
    }
}

#[derive(Clone, Copy)]
enum Axis {
    Columns,
    Rows,
}

fn resolve_tracks(
    definitions: &[GridTrack],
    available: Option<f64>,
    gap: f64,
    items: &[Item<'_>],
    axis: Axis,
) -> Vec<f64> {
    let total = available.unwrap_or(0.0);
    let mut sizes: Vec<f64> = definitions
        .iter()
        .map(|track| track_floor(track, total))
        .collect();
    for (index, track) in definitions.iter().enumerate() {
        let Some(intrinsic_mode) = track_intrinsic_mode(track) else {
            continue;
        };
        for item in items {
            let (start, end, intrinsic) = match axis {
                Axis::Columns => (
                    item.placement.column_start,
                    item.placement.column_end,
                    match intrinsic_mode {
                        IntrinsicMode::Minimum => item.min_width,
                        IntrinsicMode::Maximum => item.natural_width.max(item.min_width),
                    },
                ),
                Axis::Rows => (
                    item.placement.row_start,
                    item.placement.row_end,
                    item.natural_height,
                ),
            };
            if start == index && end == index + 1 {
                sizes[index] = sizes[index].max(intrinsic);
            }
        }
        sizes[index] = clamp_track_max(sizes[index], track, total);
    }
    for item in items {
        let (start, end) = match axis {
            Axis::Columns => (item.placement.column_start, item.placement.column_end),
            Axis::Rows => (item.placement.row_start, item.placement.row_end),
        };
        if end <= start + 1 || end > sizes.len() {
            continue;
        }
        let candidates: Vec<_> = (start..end)
            .filter(|index| track_intrinsic_mode(&definitions[*index]).is_some())
            .collect();
        if candidates.is_empty() {
            continue;
        }
        let intrinsic = match axis {
            Axis::Columns
                if candidates.iter().any(|index| {
                    track_intrinsic_mode(&definitions[*index]) == Some(IntrinsicMode::Maximum)
                }) =>
            {
                item.natural_width.max(item.min_width)
            }
            Axis::Columns => item.min_width,
            Axis::Rows => item.natural_height,
        };
        let current = span_size(&sizes, start, end, gap);
        if intrinsic > current {
            let share = (intrinsic - current) / candidates.len() as f64;
            for index in candidates {
                sizes[index] += share;
            }
        }
    }
    if let Some(available) = available {
        let total_fraction: f64 = definitions.iter().map(track_fraction).sum();
        if total_fraction > 0.0 {
            let non_flexible = definitions
                .iter()
                .enumerate()
                .filter(|(_, track)| track_fraction(track) == 0.0)
                .map(|(index, _)| sizes[index])
                .sum::<f64>();
            let flexible_space =
                (available - definitions.len().saturating_sub(1) as f64 * gap - non_flexible)
                    .max(0.0);
            for (index, track) in definitions.iter().enumerate() {
                let fraction = track_fraction(track);
                if fraction > 0.0 {
                    sizes[index] = clamp_track_max(
                        sizes[index].max(flexible_space * fraction / total_fraction),
                        track,
                        available,
                    );
                }
            }
        }
    }
    sizes
}

fn place_items(indexed: &[(usize, &LayoutNode)], style: &GridContainerStyle) -> Vec<Placement> {
    let area_map = named_areas(&style.template_areas);
    let mut placements = vec![None; indexed.len()];
    let mut occupied = HashSet::new();
    for (index, (_, node)) in indexed.iter().enumerate() {
        let item = GridItemStyle::from_node(node);
        let row_span = explicit_span(item.row_start, item.row_end, item.row_span);
        let column_span = explicit_span(item.column_start, item.column_end, item.column_span);
        let area = item
            .area
            .as_ref()
            .and_then(|name| area_map.get(name))
            .copied();
        let row_start = area
            .map(|area| area.row_start)
            .or_else(|| anchored_start(item.row_start, item.row_end, row_span));
        let column_start = area
            .map(|area| area.column_start)
            .or_else(|| anchored_start(item.column_start, item.column_end, column_span));
        if let (Some(row_start), Some(column_start)) = (row_start, column_start) {
            let placement = Placement {
                row_start,
                row_end: area
                    .map(|area| area.row_end)
                    .unwrap_or(row_start + row_span),
                column_start,
                column_end: area
                    .map(|area| area.column_end)
                    .unwrap_or(column_start + column_span),
            };
            occupy(&mut occupied, placement);
            placements[index] = Some(placement);
        }
    }
    let column_flow = matches!(
        style.auto_flow,
        GridAutoFlow::Column | GridAutoFlow::ColumnDense
    );
    let dense = matches!(
        style.auto_flow,
        GridAutoFlow::RowDense | GridAutoFlow::ColumnDense
    );
    let explicit_columns = style
        .template_columns
        .len()
        .max(area_width(&style.template_areas))
        .max(1);
    let explicit_rows = style
        .template_rows
        .len()
        .max(style.template_areas.len())
        .max(1);
    let mut cursor = (0usize, 0usize);
    for (index, (_, node)) in indexed.iter().enumerate() {
        if placements[index].is_some() {
            continue;
        }
        let item = GridItemStyle::from_node(node);
        let row_span = explicit_span(item.row_start, item.row_end, item.row_span);
        let column_span = explicit_span(item.column_start, item.column_end, item.column_span);
        let fixed_row = anchored_start(item.row_start, item.row_end, row_span);
        let fixed_column = anchored_start(item.column_start, item.column_end, column_span);
        let mut probe = if dense { (0, 0) } else { cursor };
        loop {
            let row = fixed_row.unwrap_or(probe.0);
            let column = fixed_column.unwrap_or(probe.1);
            if is_free(&occupied, row, column, row_span, column_span) {
                let placement = Placement {
                    row_start: row,
                    row_end: row + row_span,
                    column_start: column,
                    column_end: column + column_span,
                };
                occupy(&mut occupied, placement);
                placements[index] = Some(placement);
                cursor = advance(
                    row,
                    column,
                    row_span,
                    column_span,
                    column_flow,
                    explicit_rows,
                    explicit_columns,
                );
                break;
            }
            probe = match (fixed_row, fixed_column) {
                (Some(_), None) => (probe.0, probe.1 + 1),
                (None, Some(_)) => (probe.0 + 1, probe.1),
                _ => advance(
                    probe.0,
                    probe.1,
                    1,
                    1,
                    column_flow,
                    explicit_rows,
                    explicit_columns,
                ),
            };
        }
    }
    placements
        .into_iter()
        .map(|value| value.expect("every grid item is placed"))
        .collect()
}

fn explicit_span(start: Option<usize>, end: Option<usize>, span: usize) -> usize {
    match (start, end) {
        (Some(start), Some(end)) if end > start => end - start,
        _ => span.max(1),
    }
}

fn anchored_start(start: Option<usize>, end: Option<usize>, span: usize) -> Option<usize> {
    start
        .map(|line| line - 1)
        .or_else(|| end.map(|line| line.saturating_sub(span + 1)))
}

fn advance(
    row: usize,
    column: usize,
    row_span: usize,
    column_span: usize,
    column_flow: bool,
    rows: usize,
    columns: usize,
) -> (usize, usize) {
    if column_flow {
        let next_row = row + row_span;
        if next_row >= rows {
            (0, column + column_span)
        } else {
            (next_row, column)
        }
    } else {
        let next_column = column + column_span;
        if next_column >= columns {
            (row + row_span, 0)
        } else {
            (row, next_column)
        }
    }
}

fn occupy(occupied: &mut HashSet<(usize, usize)>, placement: Placement) {
    for row in placement.row_start..placement.row_end {
        for column in placement.column_start..placement.column_end {
            occupied.insert((row, column));
        }
    }
}

fn is_free(
    occupied: &HashSet<(usize, usize)>,
    row: usize,
    column: usize,
    row_span: usize,
    column_span: usize,
) -> bool {
    (row..row + row_span)
        .all(|row| (column..column + column_span).all(|column| !occupied.contains(&(row, column))))
}

fn named_areas(rows: &[Vec<Option<String>>]) -> HashMap<String, Placement> {
    let mut result = HashMap::new();
    for (row, columns) in rows.iter().enumerate() {
        for (column, name) in columns.iter().enumerate() {
            let Some(name) = name else { continue };
            result
                .entry(name.clone())
                .and_modify(|area: &mut Placement| {
                    area.row_start = area.row_start.min(row);
                    area.row_end = area.row_end.max(row + 1);
                    area.column_start = area.column_start.min(column);
                    area.column_end = area.column_end.max(column + 1);
                })
                .or_insert(Placement {
                    row_start: row,
                    row_end: row + 1,
                    column_start: column,
                    column_end: column + 1,
                });
        }
    }
    result
}

fn content_offsets(
    alignment: GridContentAlignment,
    free: f64,
    tracks: &mut [f64],
    definitions: &[GridTrack],
) -> (f64, f64) {
    let count = tracks.len();
    match alignment {
        GridContentAlignment::End => (free, 0.0),
        GridContentAlignment::Center => (free / 2.0, 0.0),
        GridContentAlignment::SpaceBetween if count > 1 => (0.0, free / (count - 1) as f64),
        GridContentAlignment::SpaceAround if count > 0 => {
            let gap = free / count as f64;
            (gap / 2.0, gap)
        }
        GridContentAlignment::SpaceEvenly if count > 0 => {
            let gap = free / (count + 1) as f64;
            (gap, gap)
        }
        GridContentAlignment::Stretch => {
            let auto_tracks: Vec<_> = definitions
                .iter()
                .enumerate()
                .filter_map(|(index, track)| track_is_auto_sized(track).then_some(index))
                .collect();
            if !auto_tracks.is_empty() {
                let extra = free / auto_tracks.len() as f64;
                for index in auto_tracks {
                    tracks[index] += extra;
                }
            }
            (0.0, 0.0)
        }
        _ => (0.0, 0.0),
    }
}

fn track_is_auto_sized(track: &GridTrack) -> bool {
    matches!(track, GridTrack::Auto)
        || matches!(track, GridTrack::MinMax(_, max) if matches!(max.as_ref(), GridTrack::Auto))
}

fn align_item(offset: f64, area: f64, natural: f64, alignment: GridAlignment) -> (f64, f64) {
    let size = natural.min(area);
    match alignment {
        GridAlignment::Start => (offset, size),
        GridAlignment::End => (offset + area - size, size),
        GridAlignment::Center => (offset + (area - size) / 2.0, size),
        GridAlignment::Stretch => (offset, area),
    }
}

fn resolve_self(value: GridSelfAlignment, inherited: GridAlignment) -> GridAlignment {
    match value {
        GridSelfAlignment::Auto => inherited,
        GridSelfAlignment::Start => GridAlignment::Start,
        GridSelfAlignment::End => GridAlignment::End,
        GridSelfAlignment::Center => GridAlignment::Center,
        GridSelfAlignment::Stretch => GridAlignment::Stretch,
    }
}

fn preserve_explicit_size(alignment: GridAlignment, size: Option<SizeValue>) -> GridAlignment {
    if alignment == GridAlignment::Stretch
        && matches!(size, Some(SizeValue::Fixed(_) | SizeValue::Percent(_)))
    {
        GridAlignment::Start
    } else {
        alignment
    }
}

fn span_size(sizes: &[f64], start: usize, end: usize, gap: f64) -> f64 {
    sizes
        .get(start..end)
        .unwrap_or_default()
        .iter()
        .sum::<f64>()
        + end.saturating_sub(start + 1) as f64 * gap
}

fn sum_tracks(sizes: &[f64], gap: f64) -> f64 {
    sizes.iter().sum::<f64>() + sizes.len().saturating_sub(1) as f64 * gap
}

fn track_offsets(sizes: &[f64], gap: f64, start: f64) -> Vec<f64> {
    let mut result = Vec::with_capacity(sizes.len() + 1);
    let mut cursor = start;
    result.push(cursor);
    for size in sizes {
        cursor += size + gap;
        result.push(cursor);
    }
    result
}

fn track_floor(track: &GridTrack, total: f64) -> f64 {
    match track {
        GridTrack::Points(value) => *value,
        GridTrack::Percent(value) => total * value,
        GridTrack::MinMax(min, _) => track_floor(min, total),
        _ => 0.0,
    }
}

fn track_fraction(track: &GridTrack) -> f64 {
    match track {
        GridTrack::Fraction(value) => *value,
        GridTrack::MinMax(_, max) => track_fraction(max),
        _ => 0.0,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum IntrinsicMode {
    Minimum,
    Maximum,
}

fn track_intrinsic_mode(track: &GridTrack) -> Option<IntrinsicMode> {
    match track {
        GridTrack::Auto => Some(IntrinsicMode::Maximum),
        GridTrack::MinContent | GridTrack::Fraction(_) => Some(IntrinsicMode::Minimum),
        GridTrack::MinMax(min, _) => track_intrinsic_mode(min),
        GridTrack::Points(_) | GridTrack::Percent(_) => None,
    }
}

fn clamp_track_max(value: f64, track: &GridTrack, total: f64) -> f64 {
    match track {
        GridTrack::MinMax(_, max) => match max.as_ref() {
            GridTrack::Points(max) => value.min(*max),
            GridTrack::Percent(max) => value.min(total * max),
            _ => value,
        },
        _ => value,
    }
}

fn min_content_width<M: TextMeasurer>(node: &LayoutNode, measurer: &M) -> f64 {
    if let Some(Content::Text(text)) = &node.content {
        return text
            .value
            .split_whitespace()
            .map(|word| measurer.measure(word, &text.font, None).width)
            .fold(0.0, f64::max);
    }
    node.min_width.unwrap_or_else(|| {
        node.children
            .iter()
            .map(|child| min_content_width(child, measurer))
            .fold(0.0, f64::max)
    })
}

fn intrinsic_item_width<M: TextMeasurer>(node: &LayoutNode, measurer: &M) -> Option<f64> {
    match node.width {
        Some(SizeValue::Fixed(value)) => return Some(value),
        Some(SizeValue::Percent(_) | SizeValue::Fill | SizeValue::Wrap) | None => {}
    }
    if let Some(Content::Text(text)) = &node.content {
        return Some(measurer.measure(&text.value, &text.font, None).width);
    }
    node.children
        .iter()
        .filter_map(|child| intrinsic_item_width(child, measurer))
        .reduce(f64::max)
}

fn intrinsic_constraints(width: Option<f64>) -> Constraints {
    Constraints {
        min_width: 0.0,
        max_width: width.unwrap_or(f64::MAX),
        min_height: 0.0,
        max_height: f64::MAX,
    }
}

fn fixed_constraints(width: f64, height: f64) -> Constraints {
    Constraints {
        min_width: width,
        max_width: width,
        min_height: height,
        max_height: height,
    }
}

fn resolve_container_size(value: Option<SizeValue>, available: f64) -> Option<f64> {
    match value {
        Some(SizeValue::Fixed(value)) => Some(value),
        Some(SizeValue::Percent(value)) => Some(available * value),
        Some(SizeValue::Fill) => Some(available),
        Some(SizeValue::Wrap) | None => None,
    }
}

fn clamp_dimension(value: f64, min: Option<f64>, max: Option<f64>) -> f64 {
    value.max(min.unwrap_or(0.0)).min(max.unwrap_or(f64::MAX))
}

fn split_track_tokens(source: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut start = 0;
    let mut depth = 0;
    for (index, character) in source.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => depth = (depth - 1).max(0),
            c if c.is_whitespace() && depth == 0 => {
                if start < index {
                    result.push(source[start..index].trim());
                }
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    if start < source.len() {
        result.push(source[start..].trim());
    }
    result
        .into_iter()
        .filter(|token| !token.is_empty())
        .collect()
}

fn split_top_level(source: &str, delimiter: char) -> Vec<&str> {
    let mut result = Vec::new();
    let mut start = 0;
    let mut depth = 0;
    for (index, character) in source.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => depth = (depth - 1).max(0),
            value if value == delimiter && depth == 0 => {
                result.push(source[start..index].trim());
                start = index + value.len_utf8();
            }
            _ => {}
        }
    }
    result.push(source[start..].trim());
    result
}

fn finite_non_negative(source: &str, key: &str) -> Result<f64, GridDiagnostic> {
    source
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite() && *value >= 0.0)
        .ok_or_else(|| GridDiagnostic {
            key: key.into(),
            message: "expected a finite non-negative grid size".into(),
        })
}

fn track_list_css(tracks: &[GridTrack]) -> String {
    tracks
        .iter()
        .map(GridTrack::css)
        .collect::<Vec<_>>()
        .join(" ")
}
fn area_width(rows: &[Vec<Option<String>>]) -> usize {
    rows.iter().map(Vec::len).max().unwrap_or(0)
}
fn grid_map(node: &LayoutNode) -> Option<&HashMap<String, ExtValue>> {
    match node.ext.get("grid") {
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
fn integer(values: &HashMap<String, ExtValue>, key: &str) -> Option<i64> {
    match values.get(key) {
        Some(ExtValue::Int(value)) => Some(*value),
        _ => None,
    }
}
fn positive_integer(values: &HashMap<String, ExtValue>, key: &str) -> Option<usize> {
    integer(values, key)
        .filter(|value| *value > 0)
        .map(|value| value as usize)
}
fn area_rows(value: Option<&ExtValue>) -> Vec<Vec<Option<String>>> {
    match value {
        Some(ExtValue::List(rows)) => rows
            .iter()
            .filter_map(|row| match row {
                ExtValue::List(columns) => Some(
                    columns
                        .iter()
                        .filter_map(|value| match value {
                            ExtValue::Str(name) => Some((name != ".").then(|| name.clone())),
                            _ => None,
                        })
                        .collect(),
                ),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}
fn parse_alignment(value: Option<&str>) -> GridAlignment {
    match value {
        Some("start") => GridAlignment::Start,
        Some("end") => GridAlignment::End,
        Some("center") => GridAlignment::Center,
        _ => GridAlignment::Stretch,
    }
}
fn parse_self_alignment(value: Option<&str>) -> GridSelfAlignment {
    match value {
        Some("start") => GridSelfAlignment::Start,
        Some("end") => GridSelfAlignment::End,
        Some("center") => GridSelfAlignment::Center,
        Some("stretch") => GridSelfAlignment::Stretch,
        _ => GridSelfAlignment::Auto,
    }
}
fn parse_content_alignment(value: Option<&str>) -> GridContentAlignment {
    match value {
        Some("start") => GridContentAlignment::Start,
        Some("end") => GridContentAlignment::End,
        Some("center") => GridContentAlignment::Center,
        Some("space-between") => GridContentAlignment::SpaceBetween,
        Some("space-around") => GridContentAlignment::SpaceAround,
        Some("space-evenly") => GridContentAlignment::SpaceEvenly,
        _ => GridContentAlignment::Stretch,
    }
}
fn alignment_name(value: GridAlignment) -> &'static str {
    match value {
        GridAlignment::Start => "start",
        GridAlignment::End => "end",
        GridAlignment::Center => "center",
        GridAlignment::Stretch => "stretch",
    }
}
fn self_alignment_name(value: GridSelfAlignment) -> &'static str {
    match value {
        GridSelfAlignment::Auto => "auto",
        GridSelfAlignment::Start => "start",
        GridSelfAlignment::End => "end",
        GridSelfAlignment::Center => "center",
        GridSelfAlignment::Stretch => "stretch",
    }
}
fn content_alignment_name(value: GridContentAlignment) -> &'static str {
    match value {
        GridContentAlignment::Start => "start",
        GridContentAlignment::End => "end",
        GridContentAlignment::Center => "center",
        GridContentAlignment::SpaceBetween => "space-between",
        GridContentAlignment::SpaceAround => "space-around",
        GridContentAlignment::SpaceEvenly => "space-evenly",
        GridContentAlignment::Stretch => "stretch",
    }
}
fn auto_flow_name(value: GridAutoFlow) -> &'static str {
    match value {
        GridAutoFlow::Row => "row",
        GridAutoFlow::Column => "column",
        GridAutoFlow::RowDense => "row dense",
        GridAutoFlow::ColumnDense => "column dense",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use layout_ir::{color_black, font_spec, size_fixed, MeasureResult, TextContent};

    struct Mono;
    impl TextMeasurer for Mono {
        fn measure(
            &self,
            text: &str,
            font: &layout_ir::FontSpec,
            max_width: Option<f64>,
        ) -> MeasureResult {
            let natural = text.chars().count() as f64 * font.size;
            MeasureResult {
                width: natural.min(max_width.unwrap_or(natural)),
                height: font.size,
                baseline: font.size * 0.8,
                line_count: 1,
            }
        }
    }

    fn leaf(id: &str, width: f64, height: f64) -> LayoutNode {
        LayoutNode::leaf_text(TextContent {
            value: id.into(),
            font: font_spec("Mono", 10.0),
            color: color_black(),
            decoration: None,
            max_lines: None,
            wrap: true,
            text_align: layout_ir::TextAlign::Start,
        })
        .with_id(id)
        .with_width(size_fixed(width))
        .with_height(size_fixed(height))
    }

    fn simple_layout(node: &LayoutNode, constraints: Constraints) -> PositionedNode {
        let width = match node.width {
            Some(SizeValue::Fixed(value)) => value,
            _ => constraints.max_width.min(20.0),
        };
        let height = match node.height {
            Some(SizeValue::Fixed(value)) => value,
            _ => constraints.max_height.min(10.0),
        };
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
    fn parses_repeat_minmax_and_percent_tracks() {
        let tracks = GridTrack::parse_list("80px repeat(2, minmax(20px, 1fr)) 25%").unwrap();
        assert_eq!(tracks.len(), 4);
        assert!(matches!(tracks[1], GridTrack::MinMax(_, _)));
        assert_eq!(tracks[3], GridTrack::Percent(0.25));
    }

    #[test]
    fn places_explicit_spans_and_auto_items_in_order() {
        let style = GridContainerStyle {
            template_columns: vec![GridTrack::Fraction(1.0), GridTrack::Fraction(1.0)],
            template_rows: vec![GridTrack::Points(30.0)],
            column_gap: 10.0,
            ..Default::default()
        };
        let mut explicit = leaf("explicit", 20.0, 10.0);
        explicit.ext.insert(
            "grid".into(),
            grid_ext(
                &Default::default(),
                &GridItemStyle {
                    column_start: Some(2),
                    row_start: Some(1),
                    column_span: 1,
                    row_span: 1,
                    order: -1,
                    ..Default::default()
                },
            ),
        );
        let mut root = LayoutNode::container(vec![leaf("auto", 20.0, 10.0), explicit])
            .with_width(size_fixed(210.0))
            .with_height(size_fixed(30.0));
        root.ext
            .insert("grid".into(), grid_ext(&style, &Default::default()));
        let result = layout_grid_with(&root, fixed_constraints(210.0, 30.0), &Mono, simple_layout);
        assert_eq!(result.children[0].id.as_deref(), Some("explicit"));
        assert_eq!(result.children[0].x, 110.0);
        assert_eq!(result.children[0].width, 20.0);
        assert_eq!(result.children[1].x, 0.0);
    }

    #[test]
    fn named_area_and_content_alignment_resolve_both_axes() {
        let style = GridContainerStyle {
            template_columns: vec![GridTrack::Points(40.0), GridTrack::Points(40.0)],
            template_rows: vec![GridTrack::Points(20.0)],
            template_areas: vec![vec![Some("hero".into()), Some("hero".into())]],
            justify_content: GridContentAlignment::Center,
            align_content: GridContentAlignment::End,
            ..Default::default()
        };
        let mut hero = leaf("hero", 30.0, 10.0);
        hero.ext.insert(
            "grid".into(),
            grid_ext(
                &Default::default(),
                &GridItemStyle {
                    area: Some("hero".into()),
                    column_span: 1,
                    row_span: 1,
                    justify_self: GridSelfAlignment::Center,
                    align_self: GridSelfAlignment::End,
                    ..Default::default()
                },
            ),
        );
        let mut root = LayoutNode::container(vec![hero])
            .with_width(size_fixed(120.0))
            .with_height(size_fixed(60.0));
        root.ext
            .insert("grid".into(), grid_ext(&style, &Default::default()));
        let result = layout_grid_with(&root, fixed_constraints(120.0, 60.0), &Mono, simple_layout);
        assert_eq!((result.children[0].x, result.children[0].y), (45.0, 50.0));
    }

    #[test]
    fn one_axis_placement_grows_the_other_axis_without_looping() {
        let style = GridContainerStyle {
            template_columns: vec![GridTrack::Points(20.0), GridTrack::Points(20.0)],
            template_rows: vec![GridTrack::Points(20.0)],
            auto_columns: GridTrack::Points(20.0),
            ..Default::default()
        };
        let mut children = Vec::new();
        for (id, column_start) in [("first", Some(1)), ("second", Some(2)), ("third", None)] {
            let mut child = leaf(id, 10.0, 10.0);
            child.ext.insert(
                "grid".into(),
                grid_ext(
                    &Default::default(),
                    &GridItemStyle {
                        row_start: Some(1),
                        column_start,
                        column_span: 1,
                        row_span: 1,
                        ..Default::default()
                    },
                ),
            );
            children.push(child);
        }
        let mut root = LayoutNode::container(children)
            .with_width(size_fixed(60.0))
            .with_height(size_fixed(20.0));
        root.ext
            .insert("grid".into(), grid_ext(&style, &Default::default()));

        let result = layout_grid_with(&root, fixed_constraints(60.0, 20.0), &Mono, simple_layout);

        assert_eq!(result.children[2].id.as_deref(), Some("third"));
        assert_eq!(result.children[2].x, 40.0);
    }

    #[test]
    fn min_content_tracks_use_the_longest_word_not_max_content() {
        let style = GridContainerStyle {
            template_columns: vec![GridTrack::MinContent, GridTrack::Fraction(1.0)],
            template_rows: vec![GridTrack::Points(20.0)],
            ..Default::default()
        };
        let text = LayoutNode::leaf_text(TextContent {
            value: "wide tiny".into(),
            font: font_spec("Mono", 10.0),
            color: color_black(),
            decoration: None,
            max_lines: None,
            wrap: true,
            text_align: layout_ir::TextAlign::Start,
        })
        .with_width(SizeValue::Wrap)
        .with_height(SizeValue::Wrap);
        let mut root = LayoutNode::container(vec![text, leaf("second", 10.0, 10.0)])
            .with_width(size_fixed(100.0))
            .with_height(size_fixed(20.0));
        root.ext
            .insert("grid".into(), grid_ext(&style, &Default::default()));

        let result = layout_grid_with(&root, fixed_constraints(100.0, 20.0), &Mono, simple_layout);

        assert_eq!(result.children[0].width, 40.0);
        assert_eq!(result.children[1].x, 40.0);
    }

    #[test]
    fn malformed_tracks_and_lines_are_diagnostic() {
        let node = LayoutNode::empty().with_ext(
            "grid",
            ExtValue::Map(HashMap::from([
                ("templateColumns".into(), ExtValue::Str("wat".into())),
                ("rowSpan".into(), ExtValue::Int(0)),
            ])),
        );
        let values = diagnostics(&node);
        assert_eq!(values.len(), 2);
    }
}
