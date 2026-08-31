//! A reusable CSS flex formatting context over `layout-ir`.
//!
//! The crate owns only flex geometry. HTML/CSS computation and host painting
//! remain separate, while the child callback lets block, inline, and nested
//! flex containers share one recursive dispatcher without dependency cycles.

use std::collections::HashMap;

use layout_ir::{
    Constraints, Content, ExtValue, LayoutNode, PositionedNode, SizeValue, TextMeasurer,
};

pub const VERSION: &str = "0.1.0";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FlexDirection {
    #[default]
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}

impl FlexDirection {
    fn is_row(self) -> bool {
        matches!(self, Self::Row | Self::RowReverse)
    }

    fn is_reverse(self) -> bool {
        matches!(self, Self::RowReverse | Self::ColumnReverse)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FlexWrap {
    #[default]
    NoWrap,
    Wrap,
    WrapReverse,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum JustifyContent {
    #[default]
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AlignItems {
    Start,
    End,
    Center,
    #[default]
    Stretch,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AlignSelf {
    #[default]
    Auto,
    Start,
    End,
    Center,
    Stretch,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AlignContent {
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
    #[default]
    Stretch,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum FlexBasis {
    #[default]
    Auto,
    Content,
    MinContent,
    Points(f64),
    Percent(f64),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlexContainerStyle {
    pub direction: FlexDirection,
    pub wrap: FlexWrap,
    pub row_gap: f64,
    pub column_gap: f64,
    pub justify_content: JustifyContent,
    pub align_items: AlignItems,
    pub align_content: AlignContent,
}

impl Default for FlexContainerStyle {
    fn default() -> Self {
        Self {
            direction: FlexDirection::Row,
            wrap: FlexWrap::NoWrap,
            row_gap: 0.0,
            column_gap: 0.0,
            justify_content: JustifyContent::Start,
            align_items: AlignItems::Stretch,
            align_content: AlignContent::Stretch,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlexItemStyle {
    pub grow: f64,
    pub shrink: f64,
    pub basis: FlexBasis,
    pub align_self: AlignSelf,
    pub order: i64,
}

impl Default for FlexItemStyle {
    fn default() -> Self {
        Self {
            grow: 0.0,
            shrink: 1.0,
            basis: FlexBasis::Auto,
            align_self: AlignSelf::Auto,
            order: 0,
        }
    }
}

impl FlexContainerStyle {
    pub fn from_node(node: &LayoutNode) -> Self {
        let Some(values) = flex_map(node) else {
            return Self::default();
        };
        let gap = number(values, "gap").unwrap_or(0.0).max(0.0);
        Self {
            direction: match string(values, "direction") {
                Some("row-reverse") => FlexDirection::RowReverse,
                Some("column") => FlexDirection::Column,
                Some("column-reverse") => FlexDirection::ColumnReverse,
                _ => FlexDirection::Row,
            },
            wrap: match string(values, "wrap") {
                Some("wrap") => FlexWrap::Wrap,
                Some("wrap-reverse") => FlexWrap::WrapReverse,
                _ => FlexWrap::NoWrap,
            },
            row_gap: number(values, "rowGap").unwrap_or(gap).max(0.0),
            column_gap: number(values, "columnGap").unwrap_or(gap).max(0.0),
            justify_content: match string(values, "justifyContent") {
                Some("end") => JustifyContent::End,
                Some("center") => JustifyContent::Center,
                Some("space-between") => JustifyContent::SpaceBetween,
                Some("space-around") => JustifyContent::SpaceAround,
                Some("space-evenly") => JustifyContent::SpaceEvenly,
                _ => JustifyContent::Start,
            },
            align_items: parse_align_items(string(values, "alignItems")),
            align_content: match string(values, "alignContent") {
                Some("start") => AlignContent::Start,
                Some("end") => AlignContent::End,
                Some("center") => AlignContent::Center,
                Some("space-between") => AlignContent::SpaceBetween,
                Some("space-around") => AlignContent::SpaceAround,
                Some("space-evenly") => AlignContent::SpaceEvenly,
                _ => AlignContent::Stretch,
            },
        }
    }

    pub fn to_ext(self) -> ExtValue {
        let direction = match self.direction {
            FlexDirection::Row => "row",
            FlexDirection::RowReverse => "row-reverse",
            FlexDirection::Column => "column",
            FlexDirection::ColumnReverse => "column-reverse",
        };
        let wrap = match self.wrap {
            FlexWrap::NoWrap => "nowrap",
            FlexWrap::Wrap => "wrap",
            FlexWrap::WrapReverse => "wrap-reverse",
        };
        ExtValue::Map(HashMap::from([
            ("direction".into(), ExtValue::Str(direction.into())),
            ("wrap".into(), ExtValue::Str(wrap.into())),
            ("rowGap".into(), ExtValue::Float(self.row_gap)),
            ("columnGap".into(), ExtValue::Float(self.column_gap)),
            (
                "justifyContent".into(),
                ExtValue::Str(justify_name(self.justify_content).into()),
            ),
            (
                "alignItems".into(),
                ExtValue::Str(align_items_name(self.align_items).into()),
            ),
            (
                "alignContent".into(),
                ExtValue::Str(align_content_name(self.align_content).into()),
            ),
        ]))
    }
}

impl FlexItemStyle {
    pub fn from_node(node: &LayoutNode) -> Self {
        let Some(values) = flex_map(node) else {
            return Self::default();
        };
        let basis = match string(values, "basisType") {
            Some("points") => FlexBasis::Points(number(values, "basis").unwrap_or(0.0)),
            Some("percent") => FlexBasis::Percent(number(values, "basis").unwrap_or(0.0)),
            Some("content") => FlexBasis::Content,
            Some("min-content") => FlexBasis::MinContent,
            _ => FlexBasis::Auto,
        };
        Self {
            grow: number(values, "grow").unwrap_or(0.0).max(0.0),
            shrink: number(values, "shrink").unwrap_or(1.0).max(0.0),
            basis,
            align_self: match string(values, "alignSelf") {
                Some("start") => AlignSelf::Start,
                Some("end") => AlignSelf::End,
                Some("center") => AlignSelf::Center,
                Some("stretch") => AlignSelf::Stretch,
                _ => AlignSelf::Auto,
            },
            order: integer(values, "order").unwrap_or(0),
        }
    }

    pub fn to_ext(self) -> ExtValue {
        let (basis_type, basis) = match self.basis {
            FlexBasis::Auto => ("auto", 0.0),
            FlexBasis::Content => ("content", 0.0),
            FlexBasis::MinContent => ("min-content", 0.0),
            FlexBasis::Points(value) => ("points", value),
            FlexBasis::Percent(value) => ("percent", value),
        };
        ExtValue::Map(HashMap::from([
            ("grow".into(), ExtValue::Float(self.grow)),
            ("shrink".into(), ExtValue::Float(self.shrink)),
            ("basisType".into(), ExtValue::Str(basis_type.into())),
            ("basis".into(), ExtValue::Float(basis)),
            (
                "alignSelf".into(),
                ExtValue::Str(align_self_name(self.align_self).into()),
            ),
            ("order".into(), ExtValue::Int(self.order)),
        ]))
    }
}

/// Merge container and item fields into one `ext["flex"]` map.
pub fn flex_ext(container: FlexContainerStyle, item: FlexItemStyle) -> ExtValue {
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
pub struct FlexDiagnostic {
    pub key: String,
    pub message: String,
}

/// Report malformed producer-owned flex fields without coupling layout to a
/// particular CSS parser. Layout itself remains tolerant and uses defaults.
pub fn diagnostics(node: &LayoutNode) -> Vec<FlexDiagnostic> {
    let Some(values) = flex_map(node) else {
        return Vec::new();
    };
    let mut result = Vec::new();
    for key in ["gap", "rowGap", "columnGap", "grow", "shrink", "basis"] {
        if values.get(key).is_some() && number(values, key).is_none() {
            result.push(FlexDiagnostic {
                key: key.into(),
                message: "expected a finite numeric flex value".into(),
            });
        }
    }
    if values.get("order").is_some() && integer(values, "order").is_none() {
        result.push(FlexDiagnostic {
            key: "order".into(),
            message: "expected an integer flex order".into(),
        });
    }
    result
}

#[derive(Clone)]
struct Item<'a> {
    node: &'a LayoutNode,
    style: FlexItemStyle,
    main: f64,
    cross: f64,
    min_main: f64,
    cross_explicit: bool,
}

/// Lay out one flex container while delegating child subtree layout to the
/// caller's shared dispatcher.
pub fn layout_flexbox_with<M, F>(
    container: &LayoutNode,
    constraints: Constraints,
    measurer: &M,
    mut layout_child: F,
) -> PositionedNode
where
    M: TextMeasurer,
    F: FnMut(&LayoutNode, Constraints) -> PositionedNode,
{
    let style = FlexContainerStyle::from_node(container);
    let padding = container.padding.unwrap_or_default();
    let horizontal_padding = padding.left + padding.right;
    let vertical_padding = padding.top + padding.bottom;
    let width_min = container.min_width.unwrap_or(0.0);
    let width_max = container.max_width.unwrap_or(f64::MAX).max(width_min);
    let height_min = container.min_height.unwrap_or(0.0);
    let height_max = container.max_height.unwrap_or(f64::MAX).max(height_min);
    let known_width = resolve_container_size(container.width, constraints.max_width)
        .map(|value| value.clamp(width_min, width_max));
    let known_height = resolve_container_size(container.height, constraints.max_height)
        .map(|value| value.clamp(height_min, height_max));
    let inner_width = known_width.map(|value| (value - horizontal_padding).max(0.0));
    let inner_height = known_height.map(|value| (value - vertical_padding).max(0.0));
    let row = style.direction.is_row();
    let main_available = if row { inner_width } else { inner_height };
    let cross_available = if row { inner_height } else { inner_width };
    let main_gap = if row { style.column_gap } else { style.row_gap };
    let cross_gap = if row { style.row_gap } else { style.column_gap };

    let mut indexed: Vec<_> = container.children.iter().enumerate().collect();
    indexed.sort_by_key(|(index, child)| (FlexItemStyle::from_node(child).order, *index));
    let mut items = Vec::with_capacity(indexed.len());
    for (_, child) in indexed {
        let item_style = FlexItemStyle::from_node(child);
        let natural = layout_child(
            child,
            intrinsic_constraints(row, main_available, cross_available),
        );
        let natural_main = if row { natural.width } else { natural.height };
        let natural_cross = if row { natural.height } else { natural.width };
        let min_content = min_content_main(child, row, measurer);
        let explicit_min = if row {
            child.min_width
        } else {
            child.min_height
        };
        let min_main = explicit_min.unwrap_or(min_content).max(0.0);
        let hint = if row { child.width } else { child.height };
        let mut main = match item_style.basis {
            FlexBasis::Auto => resolve_item_hint(hint, main_available).unwrap_or(natural_main),
            FlexBasis::Content => natural_main,
            FlexBasis::MinContent => min_content,
            FlexBasis::Points(value) => value,
            FlexBasis::Percent(value) => main_available.unwrap_or(natural_main) * value,
        };
        main = clamp_main(main.max(min_main), child, row);
        let cross_hint = if row { child.height } else { child.width };
        let cross = resolve_item_hint(cross_hint, cross_available).unwrap_or(natural_cross);
        items.push(Item {
            node: child,
            style: item_style,
            main,
            cross: clamp_cross(cross, child, row),
            min_main,
            cross_explicit: matches!(
                cross_hint,
                Some(SizeValue::Fixed(_) | SizeValue::Percent(_))
            ),
        });
    }

    let mut lines: Vec<Vec<Item<'_>>> = Vec::new();
    for item in items {
        let should_wrap = style.wrap != FlexWrap::NoWrap
            && main_available.is_some()
            && lines.last().is_some_and(|line| {
                let used =
                    line.iter().map(|item| item.main).sum::<f64>() + main_gap * line.len() as f64;
                used + item.main > main_available.unwrap_or(f64::MAX)
            });
        if lines.is_empty() || should_wrap {
            lines.push(Vec::new());
        }
        lines.last_mut().expect("line exists").push(item);
    }

    for line in &mut lines {
        distribute_flexible_lengths(line, main_available, main_gap, row);
        for item in line {
            let measured = layout_child(
                item.node,
                cross_measure_constraints(row, item.main, cross_available),
            );
            item.cross = clamp_cross(
                if row { measured.height } else { measured.width },
                item.node,
                row,
            );
        }
    }

    let natural_main = lines
        .iter()
        .map(|line| line.iter().map(|item| item.main).sum::<f64>() + gaps(line.len(), main_gap))
        .fold(0.0, f64::max);
    let mut line_crosses: Vec<f64> = lines
        .iter()
        .map(|line| line.iter().map(|item| item.cross).fold(0.0, f64::max))
        .collect();
    let natural_cross = line_crosses.iter().sum::<f64>() + gaps(line_crosses.len(), cross_gap);
    let resolved_main = main_available.unwrap_or(natural_main);
    let resolved_cross = cross_available.unwrap_or(natural_cross);
    let cross_free = (resolved_cross - natural_cross).max(0.0);
    let (line_start, extra_line_gap) = if style.wrap == FlexWrap::NoWrap && lines.len() == 1 {
        line_crosses[0] = resolved_cross;
        (0.0, 0.0)
    } else {
        align_content_offsets(
            style.align_content,
            lines.len(),
            cross_free,
            &mut line_crosses,
        )
    };

    let mut positioned_children = Vec::with_capacity(container.children.len());
    let mut cross_cursor = line_start;
    for (line_index, line) in lines.iter().enumerate() {
        let line_cross = line_crosses[line_index];
        let used_main = line.iter().map(|item| item.main).sum::<f64>() + gaps(line.len(), main_gap);
        let free_main = (resolved_main - used_main).max(0.0);
        let (mut main_cursor, extra_item_gap) =
            justify_offsets(style.justify_content, line.len(), free_main);
        for item in line {
            let align = resolved_align(item.style.align_self, style.align_items);
            let cross_size = if align == AlignItems::Stretch && !item.cross_explicit {
                clamp_cross(line_cross, item.node, row)
            } else {
                item.cross.min(line_cross)
            };
            let cross_offset = match align {
                AlignItems::End => line_cross - cross_size,
                AlignItems::Center => (line_cross - cross_size) / 2.0,
                AlignItems::Start | AlignItems::Stretch => 0.0,
            };
            let (width, height) = if row {
                (item.main, cross_size)
            } else {
                (cross_size, item.main)
            };
            let mut positioned = layout_child(item.node, fixed_constraints(width, height));
            positioned.width = width;
            positioned.height = height;
            positioned.x = if row {
                padding.left + main_cursor
            } else {
                padding.left + cross_cursor + cross_offset
            };
            positioned.y = if row {
                padding.top + cross_cursor + cross_offset
            } else {
                padding.top + main_cursor
            };
            positioned_children.push(positioned);
            main_cursor += item.main + main_gap + extra_item_gap;
        }
        cross_cursor += line_cross + cross_gap + extra_line_gap;
    }

    if style.direction.is_reverse() {
        for child in &mut positioned_children {
            if row {
                child.x = padding.left + resolved_main - (child.x - padding.left) - child.width;
            } else {
                child.y = padding.top + resolved_main - (child.y - padding.top) - child.height;
            }
        }
    }
    if style.wrap == FlexWrap::WrapReverse {
        for child in &mut positioned_children {
            if row {
                child.y = padding.top + resolved_cross - (child.y - padding.top) - child.height;
            } else {
                child.x = padding.left + resolved_cross - (child.x - padding.left) - child.width;
            }
        }
    }
    let content_width = if row { resolved_main } else { resolved_cross };
    let content_height = if row { resolved_cross } else { resolved_main };
    let constrained_width_min = width_min.max(constraints.min_width);
    let constrained_width_max = width_max
        .min(constraints.max_width)
        .max(constrained_width_min);
    let constrained_height_min = height_min.max(constraints.min_height);
    let constrained_height_max = height_max
        .min(constraints.max_height)
        .max(constrained_height_min);
    let width = known_width.unwrap_or_else(|| {
        (content_width + horizontal_padding).clamp(constrained_width_min, constrained_width_max)
    });
    let height = known_height.unwrap_or_else(|| {
        (content_height + vertical_padding).clamp(constrained_height_min, constrained_height_max)
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

fn distribute_flexible_lengths(line: &mut [Item<'_>], available: Option<f64>, gap: f64, row: bool) {
    let Some(available) = available else { return };
    let used = line.iter().map(|item| item.main).sum::<f64>() + gaps(line.len(), gap);
    let free = available - used;
    if free > 0.0 {
        let total = line.iter().map(|item| item.style.grow).sum::<f64>();
        if total > 0.0 {
            for item in line {
                item.main = clamp_main(item.main + free * item.style.grow / total, item.node, row);
            }
        }
    } else if free < 0.0 {
        let total = line
            .iter()
            .map(|item| item.style.shrink * item.main)
            .sum::<f64>();
        if total > 0.0 {
            for item in line {
                let share = (-free) * item.style.shrink * item.main / total;
                item.main = clamp_main((item.main - share).max(item.min_main), item.node, row);
            }
        }
    }
}

fn min_content_main<M: TextMeasurer>(node: &LayoutNode, row: bool, measurer: &M) -> f64 {
    if let Some(Content::Text(text)) = &node.content {
        if row {
            return text
                .value
                .split_whitespace()
                .map(|word| measurer.measure(word, &text.font, None).width)
                .fold(0.0, f64::max);
        }
        return measurer.measure(&text.value, &text.font, None).height;
    }
    if row { node.min_width } else { node.min_height }.unwrap_or(0.0)
}

fn align_content_offsets(
    align: AlignContent,
    count: usize,
    free: f64,
    line_crosses: &mut [f64],
) -> (f64, f64) {
    match align {
        AlignContent::End => (free, 0.0),
        AlignContent::Center => (free / 2.0, 0.0),
        AlignContent::SpaceBetween if count > 1 => (0.0, free / (count - 1) as f64),
        AlignContent::SpaceAround if count > 0 => {
            let gap = free / count as f64;
            (gap / 2.0, gap)
        }
        AlignContent::SpaceEvenly if count > 0 => {
            let gap = free / (count + 1) as f64;
            (gap, gap)
        }
        AlignContent::Stretch if count > 0 => {
            let extra = free / count as f64;
            for cross in line_crosses {
                *cross += extra;
            }
            (0.0, 0.0)
        }
        _ => (0.0, 0.0),
    }
}

fn justify_offsets(justify: JustifyContent, count: usize, free: f64) -> (f64, f64) {
    match justify {
        JustifyContent::End => (free, 0.0),
        JustifyContent::Center => (free / 2.0, 0.0),
        JustifyContent::SpaceBetween if count > 1 => (0.0, free / (count - 1) as f64),
        JustifyContent::SpaceAround if count > 0 => {
            let gap = free / count as f64;
            (gap / 2.0, gap)
        }
        JustifyContent::SpaceEvenly if count > 0 => {
            let gap = free / (count + 1) as f64;
            (gap, gap)
        }
        _ => (0.0, 0.0),
    }
}

fn intrinsic_constraints(
    row: bool,
    main_available: Option<f64>,
    cross_available: Option<f64>,
) -> Constraints {
    if row {
        Constraints {
            min_width: 0.0,
            max_width: main_available.unwrap_or(f64::MAX),
            min_height: 0.0,
            max_height: cross_available.unwrap_or(f64::MAX),
        }
    } else {
        Constraints {
            min_width: 0.0,
            max_width: cross_available.unwrap_or(f64::MAX),
            min_height: 0.0,
            max_height: main_available.unwrap_or(f64::MAX),
        }
    }
}

fn cross_measure_constraints(row: bool, main: f64, cross: Option<f64>) -> Constraints {
    if row {
        Constraints {
            min_width: main,
            max_width: main,
            min_height: 0.0,
            max_height: cross.unwrap_or(f64::MAX),
        }
    } else {
        Constraints {
            min_width: 0.0,
            max_width: cross.unwrap_or(f64::MAX),
            min_height: main,
            max_height: main,
        }
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

fn resolve_item_hint(value: Option<SizeValue>, available: Option<f64>) -> Option<f64> {
    match value {
        Some(SizeValue::Fixed(value)) => Some(value),
        Some(SizeValue::Percent(value)) => available.map(|available| available * value),
        Some(SizeValue::Fill) => Some(0.0),
        Some(SizeValue::Wrap) | None => None,
    }
}

fn clamp_main(value: f64, node: &LayoutNode, row: bool) -> f64 {
    if row {
        clamp_dimension(value, node.min_width, node.max_width)
    } else {
        clamp_dimension(value, node.min_height, node.max_height)
    }
}

fn clamp_cross(value: f64, node: &LayoutNode, row: bool) -> f64 {
    if row {
        clamp_dimension(value, node.min_height, node.max_height)
    } else {
        clamp_dimension(value, node.min_width, node.max_width)
    }
}

fn clamp_dimension(value: f64, min: Option<f64>, max: Option<f64>) -> f64 {
    value.max(min.unwrap_or(0.0)).min(max.unwrap_or(f64::MAX))
}

fn gaps(count: usize, gap: f64) -> f64 {
    count.saturating_sub(1) as f64 * gap
}

fn resolved_align(value: AlignSelf, inherited: AlignItems) -> AlignItems {
    match value {
        AlignSelf::Auto => inherited,
        AlignSelf::Start => AlignItems::Start,
        AlignSelf::End => AlignItems::End,
        AlignSelf::Center => AlignItems::Center,
        AlignSelf::Stretch => AlignItems::Stretch,
    }
}

fn flex_map(node: &LayoutNode) -> Option<&HashMap<String, ExtValue>> {
    match node.ext.get("flex") {
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
fn parse_align_items(value: Option<&str>) -> AlignItems {
    match value {
        Some("start") => AlignItems::Start,
        Some("end") => AlignItems::End,
        Some("center") => AlignItems::Center,
        _ => AlignItems::Stretch,
    }
}
fn justify_name(value: JustifyContent) -> &'static str {
    match value {
        JustifyContent::Start => "start",
        JustifyContent::End => "end",
        JustifyContent::Center => "center",
        JustifyContent::SpaceBetween => "space-between",
        JustifyContent::SpaceAround => "space-around",
        JustifyContent::SpaceEvenly => "space-evenly",
    }
}
fn align_items_name(value: AlignItems) -> &'static str {
    match value {
        AlignItems::Start => "start",
        AlignItems::End => "end",
        AlignItems::Center => "center",
        AlignItems::Stretch => "stretch",
    }
}
fn align_self_name(value: AlignSelf) -> &'static str {
    match value {
        AlignSelf::Auto => "auto",
        AlignSelf::Start => "start",
        AlignSelf::End => "end",
        AlignSelf::Center => "center",
        AlignSelf::Stretch => "stretch",
    }
}
fn align_content_name(value: AlignContent) -> &'static str {
    match value {
        AlignContent::Start => "start",
        AlignContent::End => "end",
        AlignContent::Center => "center",
        AlignContent::SpaceBetween => "space-between",
        AlignContent::SpaceAround => "space-around",
        AlignContent::SpaceEvenly => "space-evenly",
        AlignContent::Stretch => "stretch",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use layout_ir::{font_spec, size_fixed, MeasureResult, TextContent};

    struct Mono;
    impl TextMeasurer for Mono {
        fn measure(
            &self,
            text: &str,
            font: &layout_ir::FontSpec,
            max_width: Option<f64>,
        ) -> MeasureResult {
            let natural = text.chars().count() as f64 * font.size * 0.5;
            let width = natural.min(max_width.unwrap_or(natural));
            MeasureResult {
                width,
                height: font.size,
                baseline: font.size * 0.8,
                line_count: 1,
            }
        }
    }

    fn leaf(value: &str, width: f64) -> LayoutNode {
        LayoutNode::leaf_text(TextContent {
            value: value.into(),
            font: font_spec("Mono", 10.0),
            color: layout_ir::color_black(),
            decoration: None,
            max_lines: None,
            wrap: true,
            text_align: layout_ir::TextAlign::Start,
        })
        .with_width(size_fixed(width))
        .with_height(size_fixed(10.0))
    }

    fn simple_layout(node: &LayoutNode, constraints: Constraints) -> PositionedNode {
        let width = match node.width {
            Some(SizeValue::Fixed(value)) => value,
            _ => constraints.max_width.min(50.0),
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
    fn grows_orders_and_centers_row_items() {
        let container_style = FlexContainerStyle {
            justify_content: JustifyContent::Center,
            column_gap: 10.0,
            ..Default::default()
        };
        let grow = FlexItemStyle {
            grow: 1.0,
            ..Default::default()
        };
        let mut first = leaf("first", 20.0).with_id("first");
        first.ext.insert(
            "flex".into(),
            flex_ext(Default::default(), FlexItemStyle { order: 2, ..grow }),
        );
        let mut second = leaf("second", 20.0).with_id("second");
        second
            .ext
            .insert("flex".into(), flex_ext(Default::default(), grow));
        let mut root = LayoutNode::container(vec![first, second])
            .with_width(size_fixed(110.0))
            .with_height(size_fixed(20.0));
        root.ext
            .insert("flex".into(), flex_ext(container_style, Default::default()));
        let result =
            layout_flexbox_with(&root, fixed_constraints(110.0, 20.0), &Mono, simple_layout);
        assert_eq!(result.children[0].id.as_deref(), Some("second"));
        assert_eq!(result.children[1].id.as_deref(), Some("first"));
        assert!((result.children[1].x - 62.5).abs() < 1e-6);
        assert!((result.children[0].width - 52.5).abs() < 1e-6);
    }

    #[test]
    fn wraps_and_distributes_lines_with_align_content() {
        let style = FlexContainerStyle {
            wrap: FlexWrap::Wrap,
            column_gap: 5.0,
            row_gap: 4.0,
            align_content: AlignContent::SpaceBetween,
            ..Default::default()
        };
        let mut root =
            LayoutNode::container(vec![leaf("a", 40.0), leaf("b", 40.0), leaf("c", 40.0)])
                .with_width(size_fixed(85.0))
                .with_height(size_fixed(50.0));
        root.ext
            .insert("flex".into(), flex_ext(style, Default::default()));
        let result =
            layout_flexbox_with(&root, fixed_constraints(85.0, 50.0), &Mono, simple_layout);
        assert_eq!(result.children[0].y, result.children[1].y);
        assert_eq!(result.children[2].y, 40.0);
    }

    #[test]
    fn automatic_minimum_preserves_longest_word() {
        let mut item = leaf("unbreakable word", 100.0);
        item.width = None;
        item.ext.insert(
            "flex".into(),
            flex_ext(
                Default::default(),
                FlexItemStyle {
                    shrink: 1.0,
                    ..Default::default()
                },
            ),
        );
        let mut root = LayoutNode::container(vec![item])
            .with_width(size_fixed(20.0))
            .with_height(size_fixed(10.0));
        root.ext.insert(
            "flex".into(),
            flex_ext(Default::default(), Default::default()),
        );
        let result =
            layout_flexbox_with(&root, fixed_constraints(20.0, 10.0), &Mono, simple_layout);
        assert_eq!(result.children[0].width, 55.0);
    }

    #[test]
    fn malformed_numeric_fields_are_diagnostic() {
        let node = LayoutNode::empty().with_ext(
            "flex",
            ExtValue::Map(HashMap::from([(
                "grow".into(),
                ExtValue::Str("many".into()),
            )])),
        );
        assert_eq!(diagnostics(&node)[0].key, "grow");
    }

    #[test]
    fn reverse_direction_and_self_alignment_use_resolved_axes() {
        let style = FlexContainerStyle {
            direction: FlexDirection::RowReverse,
            align_items: AlignItems::Center,
            ..Default::default()
        };
        let mut end = leaf("end", 20.0).with_id("end");
        end.ext.insert(
            "flex".into(),
            flex_ext(
                Default::default(),
                FlexItemStyle {
                    align_self: AlignSelf::End,
                    ..Default::default()
                },
            ),
        );
        let mut root = LayoutNode::container(vec![leaf("center", 20.0), end])
            .with_width(size_fixed(100.0))
            .with_height(size_fixed(40.0));
        root.ext
            .insert("flex".into(), flex_ext(style, Default::default()));
        let result =
            layout_flexbox_with(&root, fixed_constraints(100.0, 40.0), &Mono, simple_layout);
        assert_eq!((result.children[0].x, result.children[0].y), (70.0, 15.0));
        assert_eq!((result.children[1].x, result.children[1].y), (50.0, 30.0));
    }

    #[test]
    fn column_space_between_and_shrink_respect_explicit_minimums() {
        let style = FlexContainerStyle {
            direction: FlexDirection::Column,
            justify_content: JustifyContent::SpaceBetween,
            ..Default::default()
        };
        let mut first = leaf("first", 20.0).with_height(size_fixed(80.0));
        first.min_height = Some(50.0);
        first.ext.insert(
            "flex".into(),
            flex_ext(
                Default::default(),
                FlexItemStyle {
                    shrink: 1.0,
                    basis: FlexBasis::Points(80.0),
                    ..Default::default()
                },
            ),
        );
        let mut second = leaf("second", 20.0).with_height(size_fixed(40.0));
        second.ext.insert(
            "flex".into(),
            flex_ext(
                Default::default(),
                FlexItemStyle {
                    shrink: 1.0,
                    basis: FlexBasis::Points(40.0),
                    ..Default::default()
                },
            ),
        );
        let mut root = LayoutNode::container(vec![first, second])
            .with_width(size_fixed(40.0))
            .with_height(size_fixed(90.0));
        root.ext
            .insert("flex".into(), flex_ext(style, Default::default()));
        let result =
            layout_flexbox_with(&root, fixed_constraints(40.0, 90.0), &Mono, simple_layout);
        assert_eq!(result.children[0].height, 60.0);
        assert_eq!(result.children[1].height, 30.0);
        assert_eq!(result.children[1].y, 60.0);
    }
}
