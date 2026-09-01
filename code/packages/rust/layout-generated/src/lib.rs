//! Reusable generated-content, counter, and list-marker primitives.

use std::collections::HashMap;

use layout_ir::{ExtValue, LayoutNode};

pub const VERSION: &str = "0.1.0";
pub const DEFAULT_MARKER_GAP: f64 = 8.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeneratedKind {
    Before,
    After,
    Marker,
}

impl GeneratedKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Before => "before",
            Self::After => "after",
            Self::Marker => "marker",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MarkerPosition {
    Inside,
    #[default]
    Outside,
}

impl MarkerPosition {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Inside => "inside",
            Self::Outside => "outside",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CounterStyle {
    None,
    #[default]
    Decimal,
    LowerAlpha,
    UpperAlpha,
    LowerRoman,
    UpperRoman,
    Disc,
    Circle,
    Square,
}

impl CounterStyle {
    pub fn parse(value: &str) -> Option<Self> {
        if value == "A" {
            return Some(Self::UpperAlpha);
        }
        if value == "I" {
            return Some(Self::UpperRoman);
        }
        match value.to_ascii_lowercase().as_str() {
            "none" => Some(Self::None),
            "decimal" | "1" => Some(Self::Decimal),
            "lower-alpha" | "lower-latin" | "a" => Some(Self::LowerAlpha),
            "upper-alpha" | "upper-latin" => Some(Self::UpperAlpha),
            "lower-roman" | "i" => Some(Self::LowerRoman),
            "upper-roman" => Some(Self::UpperRoman),
            "disc" => Some(Self::Disc),
            "circle" => Some(Self::Circle),
            "square" => Some(Self::Square),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContentPart {
    Text(String),
    Attribute(String),
    Counter {
        name: String,
        style: CounterStyle,
    },
    Counters {
        name: String,
        separator: String,
        style: CounterStyle,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CounterChange {
    pub name: String,
    pub value: i64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CounterContext {
    stacks: HashMap<String, Vec<i64>>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CounterScope {
    pushed: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedDiagnostic {
    pub key: String,
    pub message: String,
}

impl CounterContext {
    pub fn enter(&mut self, resets: &[CounterChange], sets: &[CounterChange]) -> CounterScope {
        let mut scope = CounterScope::default();
        for reset in resets {
            self.stacks
                .entry(reset.name.clone())
                .or_default()
                .push(reset.value);
            scope.pushed.push(reset.name.clone());
        }
        for set in sets {
            self.set(&set.name, set.value);
        }
        scope
    }

    pub fn exit(&mut self, scope: CounterScope) {
        for name in scope.pushed.into_iter().rev() {
            if let Some(values) = self.stacks.get_mut(&name) {
                values.pop();
                if values.is_empty() {
                    self.stacks.remove(&name);
                }
            }
        }
    }

    pub fn increment(&mut self, changes: &[CounterChange]) {
        for change in changes {
            let values = self.stacks.entry(change.name.clone()).or_default();
            if values.is_empty() {
                values.push(0);
            }
            if let Some(value) = values.last_mut() {
                *value = value.saturating_add(change.value);
            }
        }
    }

    pub fn set(&mut self, name: &str, value: i64) {
        let values = self.stacks.entry(name.to_string()).or_default();
        if let Some(current) = values.last_mut() {
            *current = value;
        } else {
            values.push(value);
        }
    }

    pub fn value(&self, name: &str) -> i64 {
        self.stacks
            .get(name)
            .and_then(|values| values.last())
            .copied()
            .unwrap_or(0)
    }

    pub fn values(&self, name: &str) -> impl Iterator<Item = i64> + '_ {
        self.stacks.get(name).into_iter().flatten().copied()
    }
}

pub fn evaluate_content<F>(parts: &[ContentPart], counters: &CounterContext, attribute: F) -> String
where
    F: Fn(&str) -> Option<String>,
{
    let mut output = String::new();
    for part in parts {
        match part {
            ContentPart::Text(value) => output.push_str(value),
            ContentPart::Attribute(name) => {
                if let Some(value) = attribute(name) {
                    output.push_str(&value);
                }
            }
            ContentPart::Counter { name, style } => {
                output.push_str(&format_counter(counters.value(name), *style));
            }
            ContentPart::Counters {
                name,
                separator,
                style,
            } => {
                let rendered = counters
                    .values(name)
                    .map(|value| format_counter(value, *style))
                    .collect::<Vec<_>>()
                    .join(separator);
                output.push_str(&rendered);
            }
        }
    }
    output
}

pub fn format_marker(value: i64, style: CounterStyle) -> String {
    match style {
        CounterStyle::None => String::new(),
        CounterStyle::Disc => "•".into(),
        CounterStyle::Circle => "◦".into(),
        CounterStyle::Square => "▪".into(),
        _ => format!("{}.", format_counter(value, style)),
    }
}

pub fn format_counter(value: i64, style: CounterStyle) -> String {
    match style {
        CounterStyle::None => String::new(),
        CounterStyle::Decimal => value.to_string(),
        CounterStyle::LowerAlpha => alphabetic(value, false),
        CounterStyle::UpperAlpha => alphabetic(value, true),
        CounterStyle::LowerRoman => roman(value, false),
        CounterStyle::UpperRoman => roman(value, true),
        CounterStyle::Disc => "•".into(),
        CounterStyle::Circle => "◦".into(),
        CounterStyle::Square => "▪".into(),
    }
}

pub fn generated_ext(kind: GeneratedKind, position: MarkerPosition) -> ExtValue {
    ExtValue::Map(HashMap::from([
        ("kind".into(), ExtValue::Str(kind.as_str().into())),
        ("position".into(), ExtValue::Str(position.as_str().into())),
        ("markerGap".into(), ExtValue::Float(DEFAULT_MARKER_GAP)),
        ("semanticOwner".into(), ExtValue::Bool(false)),
    ]))
}

pub fn generated_kind(node: &LayoutNode) -> Option<GeneratedKind> {
    let ExtValue::Map(values) = node.ext.get("generated")? else {
        return None;
    };
    match values.get("kind") {
        Some(ExtValue::Str(value)) if value == "before" => Some(GeneratedKind::Before),
        Some(ExtValue::Str(value)) if value == "after" => Some(GeneratedKind::After),
        Some(ExtValue::Str(value)) if value == "marker" => Some(GeneratedKind::Marker),
        _ => None,
    }
}

pub fn marker_position(node: &LayoutNode) -> MarkerPosition {
    let Some(ExtValue::Map(values)) = node.ext.get("generated") else {
        return MarkerPosition::Inside;
    };
    match values.get("position") {
        Some(ExtValue::Str(value)) if value == "outside" => MarkerPosition::Outside,
        _ => MarkerPosition::Inside,
    }
}

pub fn marker_gap(node: &LayoutNode) -> f64 {
    let Some(ExtValue::Map(values)) = node.ext.get("generated") else {
        return DEFAULT_MARKER_GAP;
    };
    match values.get("markerGap") {
        Some(ExtValue::Float(value)) if value.is_finite() && *value >= 0.0 => *value,
        Some(ExtValue::Int(value)) if *value >= 0 => *value as f64,
        _ => DEFAULT_MARKER_GAP,
    }
}

pub fn diagnostics(node: &LayoutNode) -> Vec<GeneratedDiagnostic> {
    let Some(ExtValue::Map(values)) = node.ext.get("generated") else {
        return Vec::new();
    };
    let mut diagnostics = Vec::new();
    if !matches!(values.get("kind"), Some(ExtValue::Str(value)) if matches!(value.as_str(), "before" | "after" | "marker"))
    {
        diagnostics.push(GeneratedDiagnostic {
            key: "kind".into(),
            message: "generated box has an unknown kind".into(),
        });
    }
    if matches!(values.get("markerGap"), Some(value) if !matches!(value, ExtValue::Float(number) if number.is_finite() && *number >= 0.0) && !matches!(value, ExtValue::Int(number) if *number >= 0))
    {
        diagnostics.push(GeneratedDiagnostic {
            key: "markerGap".into(),
            message: "generated marker gap must be finite and non-negative".into(),
        });
    }
    diagnostics
}

fn alphabetic(value: i64, uppercase: bool) -> String {
    if value <= 0 {
        return value.to_string();
    }
    let mut value = value as u64;
    let mut chars = Vec::new();
    while value > 0 {
        value -= 1;
        let base = if uppercase { b'A' } else { b'a' };
        chars.push((base + (value % 26) as u8) as char);
        value /= 26;
    }
    chars.into_iter().rev().collect()
}

fn roman(value: i64, uppercase: bool) -> String {
    if !(1..=3999).contains(&value) {
        return value.to_string();
    }
    let mut remaining = value;
    let mut output = String::new();
    for (amount, symbol) in [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ] {
        while remaining >= amount {
            remaining -= amount;
            output.push_str(symbol);
        }
    }
    if uppercase {
        output
    } else {
        output.to_ascii_lowercase()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_numeric_alphabetic_roman_and_symbolic_markers() {
        assert_eq!(format_marker(27, CounterStyle::LowerAlpha), "aa.");
        assert_eq!(format_marker(14, CounterStyle::UpperRoman), "XIV.");
        assert_eq!(format_marker(1, CounterStyle::Disc), "•");
        assert_eq!(format_marker(-2, CounterStyle::Decimal), "-2.");
    }

    #[test]
    fn scoped_counters_restore_outer_values_and_join_stacks() {
        let mut counters = CounterContext::default();
        let outer = counters.enter(
            &[CounterChange {
                name: "section".into(),
                value: 0,
            }],
            &[],
        );
        counters.increment(&[CounterChange {
            name: "section".into(),
            value: 1,
        }]);
        let inner = counters.enter(
            &[CounterChange {
                name: "section".into(),
                value: 0,
            }],
            &[],
        );
        counters.increment(&[CounterChange {
            name: "section".into(),
            value: 2,
        }]);
        assert_eq!(counters.values("section").collect::<Vec<_>>(), vec![1, 2]);
        counters.exit(inner);
        assert_eq!(counters.value("section"), 1);
        counters.exit(outer);
        assert_eq!(counters.value("section"), 0);
    }

    #[test]
    fn generated_content_combines_text_attributes_and_counter_stacks() {
        let mut counters = CounterContext::default();
        counters.enter(
            &[CounterChange {
                name: "chapter".into(),
                value: 2,
            }],
            &[],
        );
        counters.enter(
            &[CounterChange {
                name: "chapter".into(),
                value: 4,
            }],
            &[],
        );
        let value = evaluate_content(
            &[
                ContentPart::Text("Chapter ".into()),
                ContentPart::Counters {
                    name: "chapter".into(),
                    separator: ".".into(),
                    style: CounterStyle::Decimal,
                },
                ContentPart::Text(": ".into()),
                ContentPart::Attribute("title".into()),
            ],
            &counters,
            |name| (name == "title").then(|| "Overview".into()),
        );
        assert_eq!(value, "Chapter 2.4: Overview");
    }

    #[test]
    fn diagnostics_reject_unknown_kinds_and_non_finite_gaps() {
        let mut node = LayoutNode::empty();
        node.ext.insert(
            "generated".into(),
            ExtValue::Map(HashMap::from([
                ("kind".into(), ExtValue::Str("mystery".into())),
                ("markerGap".into(), ExtValue::Float(f64::NAN)),
            ])),
        );
        assert_eq!(diagnostics(&node).len(), 2);
    }
}
