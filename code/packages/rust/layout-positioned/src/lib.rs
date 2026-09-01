//! Reusable positioned formatting and clipping contracts over `layout-ir`.

use std::collections::HashMap;

use layout_ir::{Ext, ExtValue, LayoutNode, PositionedNode};

pub const VERSION: &str = "0.1.0";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Position {
    #[default]
    Static,
    Relative,
    Absolute,
    Fixed,
    Sticky,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Overflow {
    #[default]
    Visible,
    Hidden,
    Auto,
    Scroll,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Insets {
    pub top: Option<f64>,
    pub right: Option<f64>,
    pub bottom: Option<f64>,
    pub left: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PositionedStyle {
    pub position: Position,
    pub insets: Insets,
    pub z_index: Option<i64>,
    pub overflow_x: Overflow,
    pub overflow_y: Overflow,
}

impl Default for PositionedStyle {
    fn default() -> Self {
        Self {
            position: Position::Static,
            insets: Insets::default(),
            z_index: None,
            overflow_x: Overflow::Visible,
            overflow_y: Overflow::Visible,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PositionedDiagnostic {
    pub key: String,
    pub message: String,
}

impl PositionedStyle {
    pub fn from_layout(node: &LayoutNode) -> Self {
        Self::from_ext(&node.ext)
    }

    pub fn from_positioned(node: &PositionedNode) -> Self {
        Self::from_ext(&node.ext)
    }

    pub fn from_ext(ext: &Ext) -> Self {
        let Some(ExtValue::Map(values)) = ext.get("positioned") else {
            return Self::default();
        };
        Self {
            position: parse_position(string(values, "position")),
            insets: Insets {
                top: number(values, "top"),
                right: number(values, "right"),
                bottom: number(values, "bottom"),
                left: number(values, "left"),
            },
            z_index: integer(values, "zIndex"),
            overflow_x: parse_overflow(string(values, "overflowX")),
            overflow_y: parse_overflow(string(values, "overflowY")),
        }
    }

    pub fn to_ext(self) -> ExtValue {
        let mut values = HashMap::from([
            (
                "position".into(),
                ExtValue::Str(position_name(self.position).into()),
            ),
            (
                "overflowX".into(),
                ExtValue::Str(overflow_name(self.overflow_x).into()),
            ),
            (
                "overflowY".into(),
                ExtValue::Str(overflow_name(self.overflow_y).into()),
            ),
        ]);
        for (key, value) in [
            ("top", self.insets.top),
            ("right", self.insets.right),
            ("bottom", self.insets.bottom),
            ("left", self.insets.left),
        ] {
            if let Some(value) = value.filter(|value| value.is_finite()) {
                values.insert(key.into(), ExtValue::Float(value));
            }
        }
        if let Some(z_index) = self.z_index {
            values.insert("zIndex".into(), ExtValue::Int(z_index));
        }
        ExtValue::Map(values)
    }

    pub fn is_out_of_flow(self) -> bool {
        matches!(self.position, Position::Absolute | Position::Fixed)
    }

    pub fn establishes_containing_block(self) -> bool {
        self.position != Position::Static
    }

    pub fn clips_x(self) -> bool {
        self.overflow_x != Overflow::Visible
    }

    pub fn clips_y(self) -> bool {
        self.overflow_y != Overflow::Visible
    }

    pub fn apply_in_flow_offset(self, node: &mut PositionedNode) {
        if !matches!(self.position, Position::Relative | Position::Sticky) {
            return;
        }
        node.x += self
            .insets
            .left
            .unwrap_or_else(|| -self.insets.right.unwrap_or(0.0));
        node.y += self
            .insets
            .top
            .unwrap_or_else(|| -self.insets.bottom.unwrap_or(0.0));
    }

    pub fn resolve_out_of_flow(self, node: &mut PositionedNode, width: f64, height: f64) {
        if !self.is_out_of_flow() {
            return;
        }
        node.x = self
            .insets
            .left
            .unwrap_or_else(|| (width - self.insets.right.unwrap_or(width) - node.width).max(0.0));
        node.y = self.insets.top.unwrap_or_else(|| {
            (height - self.insets.bottom.unwrap_or(height) - node.height).max(0.0)
        });
        if let (Some(left), Some(right)) = (self.insets.left, self.insets.right) {
            node.width = (width - left - right).max(0.0);
        }
        if let (Some(top), Some(bottom)) = (self.insets.top, self.insets.bottom) {
            node.height = (height - top - bottom).max(0.0);
        }
    }
}

pub fn positioned_ext(style: PositionedStyle) -> ExtValue {
    style.to_ext()
}

pub fn stable_stack(children: &mut [PositionedNode]) {
    children.sort_by_key(|child| PositionedStyle::from_positioned(child).z_index.unwrap_or(0));
}

pub fn scroll_extent(node: &PositionedNode) -> (f64, f64) {
    let style = PositionedStyle::from_positioned(node);
    node.children
        .iter()
        .fold((node.width, node.height), |extent, child| {
            let child_extent = scroll_extent(child);
            (
                if style.clips_x() {
                    node.width
                } else {
                    extent.0.max(child.x + child_extent.0)
                },
                if style.clips_y() {
                    node.height
                } else {
                    extent.1.max(child.y + child_extent.1)
                },
            )
        })
}

pub fn diagnostics(ext: &Ext) -> Vec<PositionedDiagnostic> {
    let Some(value) = ext.get("positioned") else {
        return Vec::new();
    };
    let ExtValue::Map(values) = value else {
        return vec![diagnostic(
            "positioned",
            "positioned extension must be a map",
        )];
    };
    let mut result = Vec::new();
    validate_keyword(
        values,
        "position",
        &["static", "relative", "absolute", "fixed", "sticky"],
        &mut result,
    );
    validate_keyword(
        values,
        "overflowX",
        &["visible", "hidden", "auto", "scroll"],
        &mut result,
    );
    validate_keyword(
        values,
        "overflowY",
        &["visible", "hidden", "auto", "scroll"],
        &mut result,
    );
    for key in ["top", "right", "bottom", "left"] {
        if values
            .get(key)
            .is_some_and(|value| number_value(value).is_none())
        {
            result.push(diagnostic(
                key,
                "inset must be a finite number or omitted for auto",
            ));
        }
    }
    if values
        .get("zIndex")
        .is_some_and(|value| !matches!(value, ExtValue::Int(_)))
    {
        result.push(diagnostic(
            "zIndex",
            "z-index must be an integer or omitted for auto",
        ));
    }
    result
}

fn validate_keyword(
    values: &HashMap<String, ExtValue>,
    key: &str,
    allowed: &[&str],
    out: &mut Vec<PositionedDiagnostic>,
) {
    if let Some(value) = values.get(key) {
        let valid = matches!(value, ExtValue::Str(value) if allowed.contains(&value.as_str()));
        if !valid {
            out.push(diagnostic(key, &format!("unsupported {key} value")));
        }
    }
}

fn diagnostic(key: &str, message: &str) -> PositionedDiagnostic {
    PositionedDiagnostic {
        key: key.into(),
        message: message.into(),
    }
}

fn string<'a>(values: &'a HashMap<String, ExtValue>, key: &str) -> Option<&'a str> {
    match values.get(key) {
        Some(ExtValue::Str(value)) => Some(value),
        _ => None,
    }
}

fn number(values: &HashMap<String, ExtValue>, key: &str) -> Option<f64> {
    values.get(key).and_then(number_value)
}

fn number_value(value: &ExtValue) -> Option<f64> {
    match value {
        ExtValue::Float(value) if value.is_finite() => Some(*value),
        ExtValue::Int(value) => Some(*value as f64),
        _ => None,
    }
}

fn integer(values: &HashMap<String, ExtValue>, key: &str) -> Option<i64> {
    match values.get(key) {
        Some(ExtValue::Int(value)) => Some(*value),
        _ => None,
    }
}

fn parse_position(value: Option<&str>) -> Position {
    match value {
        Some("relative") => Position::Relative,
        Some("absolute") => Position::Absolute,
        Some("fixed") => Position::Fixed,
        Some("sticky") => Position::Sticky,
        _ => Position::Static,
    }
}

fn parse_overflow(value: Option<&str>) -> Overflow {
    match value {
        Some("hidden") => Overflow::Hidden,
        Some("auto") => Overflow::Auto,
        Some("scroll") => Overflow::Scroll,
        _ => Overflow::Visible,
    }
}

fn position_name(value: Position) -> &'static str {
    match value {
        Position::Static => "static",
        Position::Relative => "relative",
        Position::Absolute => "absolute",
        Position::Fixed => "fixed",
        Position::Sticky => "sticky",
    }
}

fn overflow_name(value: Overflow) -> &'static str {
    match value {
        Overflow::Visible => "visible",
        Overflow::Hidden => "hidden",
        Overflow::Auto => "auto",
        Overflow::Scroll => "scroll",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn positioned(x: f64, y: f64, width: f64, height: f64) -> PositionedNode {
        PositionedNode {
            x,
            y,
            width,
            height,
            id: None,
            content: None,
            children: Vec::new(),
            ext: Ext::new(),
        }
    }

    #[test]
    fn extension_round_trip_and_diagnostics_are_tolerant() {
        let style = PositionedStyle {
            position: Position::Absolute,
            insets: Insets {
                top: Some(12.0),
                right: Some(8.0),
                bottom: None,
                left: None,
            },
            z_index: Some(3),
            overflow_x: Overflow::Hidden,
            overflow_y: Overflow::Auto,
        };
        let node = LayoutNode::empty().with_ext("positioned", style.to_ext());
        assert_eq!(PositionedStyle::from_layout(&node), style);
        assert!(diagnostics(&node.ext).is_empty());
    }

    #[test]
    fn resolves_insets_stacking_and_scroll_extent() {
        let style = PositionedStyle {
            position: Position::Absolute,
            insets: Insets {
                top: Some(5.0),
                right: Some(10.0),
                bottom: None,
                left: Some(20.0),
            },
            z_index: Some(2),
            ..Default::default()
        };
        let mut node = positioned(0.0, 0.0, 10.0, 15.0);
        style.resolve_out_of_flow(&mut node, 100.0, 80.0);
        assert_eq!((node.x, node.y, node.width), (20.0, 5.0, 70.0));
        node.children.push(positioned(90.0, 70.0, 30.0, 20.0));
        assert_eq!(scroll_extent(&node), (120.0, 90.0));
    }
}
