//! Reusable form-control state and intrinsic layout contracts.

use std::collections::{HashMap, HashSet};

use layout_ir::{ExtValue, LayoutNode, PositionedNode};

pub const VERSION: &str = "0.1.0";
pub const EXT_KEY: &str = "control";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlKind {
    Text,
    Password,
    Search,
    Email,
    Url,
    Number,
    Date,
    Month,
    Week,
    Time,
    DateTimeLocal,
    Color,
    Button,
    TextArea,
    Select,
    Range,
    Checkbox,
    Radio,
}

impl ControlKind {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Password => "password",
            Self::Search => "search",
            Self::Email => "email",
            Self::Url => "url",
            Self::Number => "number",
            Self::Date => "date",
            Self::Month => "month",
            Self::Week => "week",
            Self::Time => "time",
            Self::DateTimeLocal => "datetime-local",
            Self::Color => "color",
            Self::Button => "button",
            Self::TextArea => "textarea",
            Self::Select => "select",
            Self::Range => "range",
            Self::Checkbox => "checkbox",
            Self::Radio => "radio",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "text" => Self::Text,
            "password" => Self::Password,
            "search" => Self::Search,
            "email" => Self::Email,
            "url" => Self::Url,
            "number" => Self::Number,
            "date" => Self::Date,
            "month" => Self::Month,
            "week" => Self::Week,
            "time" => Self::Time,
            "datetime-local" => Self::DateTimeLocal,
            "color" => Self::Color,
            "button" | "submit" | "reset" => Self::Button,
            "textarea" => Self::TextArea,
            "select" => Self::Select,
            "range" => Self::Range,
            "checkbox" => Self::Checkbox,
            "radio" => Self::Radio,
            _ => return None,
        })
    }

    pub const fn accepts_text(self) -> bool {
        matches!(
            self,
            Self::Text
                | Self::Password
                | Self::Search
                | Self::Email
                | Self::Url
                | Self::Number
                | Self::TextArea
        )
    }

    pub const fn supports_selection(self) -> bool {
        self.accepts_text() && !matches!(self, Self::Number)
    }

    pub const fn supports_maxlength(self) -> bool {
        self.accepts_text() && !matches!(self, Self::Number)
    }

    pub const fn is_temporal(self) -> bool {
        matches!(
            self,
            Self::Date | Self::Month | Self::Week | Self::Time | Self::DateTimeLocal
        )
    }

    pub const fn has_typed_value(self) -> bool {
        self.is_temporal() || matches!(self, Self::Color)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ControlAppearance {
    #[default]
    Auto,
    None,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ControlSize {
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ControlState {
    pub key: String,
    pub name: Option<String>,
    pub kind: ControlKind,
    pub value: String,
    pub placeholder: Option<String>,
    pub options: Vec<String>,
    pub option_disabled: Vec<bool>,
    pub selected_indices: Vec<usize>,
    pub selected_index: usize,
    pub columns: usize,
    pub rows: usize,
    pub disabled: bool,
    pub readonly: bool,
    pub required: bool,
    pub multiple: bool,
    pub checked: bool,
    pub indeterminate: bool,
    pub focused: bool,
    pub appearance: ControlAppearance,
}

impl ControlState {
    pub fn new(key: impl Into<String>, kind: ControlKind) -> Self {
        Self {
            key: key.into(),
            name: None,
            kind,
            value: String::new(),
            placeholder: None,
            options: Vec::new(),
            option_disabled: Vec::new(),
            selected_indices: Vec::new(),
            selected_index: 0,
            columns: 20,
            rows: if kind == ControlKind::TextArea { 2 } else { 1 },
            disabled: false,
            readonly: false,
            required: false,
            multiple: false,
            checked: false,
            indeterminate: false,
            focused: false,
            appearance: ControlAppearance::Auto,
        }
    }

    pub fn display_value(&self) -> String {
        match self.kind {
            ControlKind::Password => "*".repeat(self.value.chars().count()),
            ControlKind::Checkbox => if self.indeterminate {
                "[-]"
            } else if self.checked {
                "[x]"
            } else {
                "[ ]"
            }
            .to_string(),
            ControlKind::Radio => if self.checked { "(o)" } else { "( )" }.to_string(),
            ControlKind::Select if self.multiple => self
                .selected_indices
                .iter()
                .filter_map(|index| self.options.get(*index))
                .cloned()
                .collect::<Vec<_>>()
                .join(", "),
            ControlKind::Select => self
                .options
                .get(self.selected_index)
                .cloned()
                .unwrap_or_else(|| self.value.clone()),
            _ if self.value.is_empty() => self.placeholder.clone().unwrap_or_default(),
            _ => self.value.clone(),
        }
    }

    pub fn intrinsic_size(&self, font_size: f64) -> ControlSize {
        let font_size = finite_positive(font_size, 16.0);
        let advance = font_size * 0.55;
        let line = font_size * 1.2;
        match self.kind {
            ControlKind::Checkbox | ControlKind::Radio => ControlSize {
                width: font_size.max(13.0),
                height: font_size.max(13.0),
            },
            ControlKind::Button => ControlSize {
                width: (self.display_value().chars().count() as f64 * advance + 24.0).max(44.0),
                height: line + 12.0,
            },
            ControlKind::TextArea => ControlSize {
                width: self.columns.clamp(1, 200) as f64 * advance + 18.0,
                height: self.rows.clamp(1, 100) as f64 * line + 14.0,
            },
            ControlKind::Select => {
                let characters = self
                    .options
                    .iter()
                    .map(|option| option.chars().count())
                    .max()
                    .unwrap_or_else(|| self.value.chars().count())
                    .max(4);
                ControlSize {
                    width: characters as f64 * advance + 30.0,
                    height: line + 12.0,
                }
            }
            ControlKind::Range => ControlSize {
                width: (self.columns.clamp(8, 200) as f64 * advance + 18.0).max(129.0),
                height: line + 12.0,
            },
            _ => ControlSize {
                width: self.columns.clamp(1, 200) as f64 * advance + 18.0,
                height: line + 12.0,
            },
        }
    }

    pub fn to_ext(&self) -> ExtValue {
        ExtValue::Map(HashMap::from([
            ("key".into(), ExtValue::Str(self.key.clone())),
            (
                "name".into(),
                self.name.as_ref().map_or_else(
                    || ExtValue::Str(String::new()),
                    |value| ExtValue::Str(value.clone()),
                ),
            ),
            ("kind".into(), ExtValue::Str(self.kind.name().into())),
            ("value".into(), ExtValue::Str(self.value.clone())),
            (
                "placeholder".into(),
                self.placeholder.as_ref().map_or_else(
                    || ExtValue::Str(String::new()),
                    |value| ExtValue::Str(value.clone()),
                ),
            ),
            (
                "options".into(),
                ExtValue::List(self.options.iter().cloned().map(ExtValue::Str).collect()),
            ),
            (
                "optionDisabled".into(),
                ExtValue::List(
                    self.option_disabled
                        .iter()
                        .copied()
                        .map(ExtValue::Bool)
                        .collect(),
                ),
            ),
            (
                "selectedIndices".into(),
                ExtValue::List(
                    self.selected_indices
                        .iter()
                        .map(|index| ExtValue::Int(*index as i64))
                        .collect(),
                ),
            ),
            (
                "selectedIndex".into(),
                ExtValue::Int(self.selected_index as i64),
            ),
            ("columns".into(), ExtValue::Int(self.columns as i64)),
            ("rows".into(), ExtValue::Int(self.rows as i64)),
            ("disabled".into(), ExtValue::Bool(self.disabled)),
            ("readonly".into(), ExtValue::Bool(self.readonly)),
            ("required".into(), ExtValue::Bool(self.required)),
            ("multiple".into(), ExtValue::Bool(self.multiple)),
            ("checked".into(), ExtValue::Bool(self.checked)),
            ("indeterminate".into(), ExtValue::Bool(self.indeterminate)),
            ("focused".into(), ExtValue::Bool(self.focused)),
            (
                "appearance".into(),
                ExtValue::Str(
                    match self.appearance {
                        ControlAppearance::Auto => "auto",
                        ControlAppearance::None => "none",
                    }
                    .into(),
                ),
            ),
        ]))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlDiagnostic {
    pub key: String,
    pub message: String,
}

pub fn control_state(node: &LayoutNode) -> Option<ControlState> {
    decode_control(node.ext.get(EXT_KEY)?).ok()
}

pub fn positioned_control_state(node: &PositionedNode) -> Option<ControlState> {
    decode_control(node.ext.get(EXT_KEY)?).ok()
}

/// Validate every control contract in a layout tree without stopping layout.
pub fn diagnose_layout_controls(root: &LayoutNode) -> Vec<ControlDiagnostic> {
    let mut diagnostics = Vec::new();
    let mut keys = HashSet::new();
    diagnose_node(root, &mut keys, &mut diagnostics);
    diagnostics
}

fn diagnose_node(
    node: &LayoutNode,
    keys: &mut HashSet<String>,
    diagnostics: &mut Vec<ControlDiagnostic>,
) {
    if let Some(value) = node.ext.get(EXT_KEY) {
        match decode_control(value) {
            Ok(control) if control.key.is_empty() => {
                diagnostics.push(diagnostic("", "control key must not be empty"));
            }
            Ok(control) if !keys.insert(control.key.clone()) => {
                diagnostics.push(diagnostic(&control.key, "control key is duplicated"));
            }
            Ok(_) => {}
            Err(error) => diagnostics.push(error),
        }
    }
    for child in &node.children {
        diagnose_node(child, keys, diagnostics);
    }
}

pub fn decode_control(value: &ExtValue) -> Result<ControlState, ControlDiagnostic> {
    let ExtValue::Map(values) = value else {
        return Err(diagnostic("", "control metadata must be a map"));
    };
    let key = string(values, "key").unwrap_or_default();
    let kind = string(values, "kind")
        .and_then(ControlKind::parse)
        .ok_or_else(|| diagnostic(key, "control kind is missing or unsupported"))?;
    let mut state = ControlState::new(key, kind);
    state.name = string(values, "name")
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    state.value = string(values, "value").unwrap_or_default().to_string();
    state.placeholder = string(values, "placeholder")
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    state.options = values
        .get("options")
        .and_then(|value| match value {
            ExtValue::List(values) => Some(
                values
                    .iter()
                    .filter_map(|value| match value {
                        ExtValue::Str(value) => Some(value.clone()),
                        _ => None,
                    })
                    .collect(),
            ),
            _ => None,
        })
        .unwrap_or_default();
    state.option_disabled = values
        .get("optionDisabled")
        .and_then(|value| match value {
            ExtValue::List(values) => Some(
                values
                    .iter()
                    .map(|value| matches!(value, ExtValue::Bool(true)))
                    .collect(),
            ),
            _ => None,
        })
        .unwrap_or_default();
    state.selected_indices = values
        .get("selectedIndices")
        .and_then(|value| match value {
            ExtValue::List(values) => Some(
                values
                    .iter()
                    .filter_map(|value| match value {
                        ExtValue::Int(index) if *index >= 0 => Some(*index as usize),
                        _ => None,
                    })
                    .collect(),
            ),
            _ => None,
        })
        .unwrap_or_default();
    state.selected_index = integer(values, "selectedIndex").unwrap_or(0).max(0) as usize;
    state.columns = integer(values, "columns").unwrap_or(20).clamp(1, 200) as usize;
    state.rows = integer(values, "rows").unwrap_or(1).clamp(1, 100) as usize;
    state.disabled = boolean(values, "disabled");
    state.readonly = boolean(values, "readonly");
    state.required = boolean(values, "required");
    state.multiple = boolean(values, "multiple");
    state.checked = boolean(values, "checked");
    state.indeterminate = boolean(values, "indeterminate");
    state.focused = boolean(values, "focused");
    state.appearance = if string(values, "appearance") == Some("none") {
        ControlAppearance::None
    } else {
        ControlAppearance::Auto
    };
    Ok(state)
}

fn diagnostic(key: &str, message: &str) -> ControlDiagnostic {
    ControlDiagnostic {
        key: key.to_string(),
        message: message.to_string(),
    }
}

fn string<'a>(values: &'a HashMap<String, ExtValue>, key: &str) -> Option<&'a str> {
    match values.get(key) {
        Some(ExtValue::Str(value)) => Some(value),
        _ => None,
    }
}

fn integer(values: &HashMap<String, ExtValue>, key: &str) -> Option<i64> {
    match values.get(key) {
        Some(ExtValue::Int(value)) => Some(*value),
        _ => None,
    }
}

fn boolean(values: &HashMap<String, ExtValue>, key: &str) -> bool {
    matches!(values.get(key), Some(ExtValue::Bool(true)))
}

fn finite_positive(value: f64, fallback: f64) -> f64 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intrinsic_sizes_cover_each_control_family() {
        let text = ControlState::new("query", ControlKind::Text).intrinsic_size(16.0);
        let area = ControlState::new("notes", ControlKind::TextArea).intrinsic_size(16.0);
        let check = ControlState::new("ready", ControlKind::Checkbox).intrinsic_size(16.0);
        assert!(text.width > 150.0 && text.height > 20.0);
        assert!(area.height > text.height);
        assert_eq!(check.width, check.height);
    }

    #[test]
    fn typed_input_capabilities_are_explicit() {
        assert_eq!(ControlKind::parse("url"), Some(ControlKind::Url));
        assert!(ControlKind::Url.supports_selection());
        assert!(ControlKind::Url.supports_maxlength());
        assert!(!ControlKind::Number.supports_selection());
        assert!(!ControlKind::Number.supports_maxlength());
        assert!(ControlKind::Date.is_temporal());
        assert!(ControlKind::DateTimeLocal.has_typed_value());
        assert!(ControlKind::Color.has_typed_value());
        assert!(!ControlKind::Time.supports_selection());
        assert_eq!(
            ControlKind::parse("datetime-local"),
            Some(ControlKind::DateTimeLocal)
        );
    }

    #[test]
    fn metadata_round_trips_without_host_types() {
        let mut expected = ControlState::new("choice", ControlKind::Select);
        expected.options = vec!["One".into(), "Two".into()];
        expected.option_disabled = vec![false, true];
        expected.selected_indices = vec![0];
        expected.selected_index = 1;
        expected.focused = true;
        assert_eq!(decode_control(&expected.to_ext()), Ok(expected));
    }

    #[test]
    fn password_and_boolean_display_values_are_safe() {
        let mut password = ControlState::new("secret", ControlKind::Password);
        password.value = "hello".into();
        assert_eq!(password.display_value(), "*****");
        let mut check = ControlState::new("check", ControlKind::Checkbox);
        assert_eq!(check.display_value(), "[ ]");
        check.checked = true;
        assert_eq!(check.display_value(), "[x]");
        check.indeterminate = true;
        assert_eq!(check.display_value(), "[-]");
        let mut range = ControlState::new("volume", ControlKind::Range);
        range.value = "50".into();
        assert!(range.intrinsic_size(16.0).width >= 129.0);
    }

    #[test]
    fn diagnostics_bound_malformed_and_duplicate_metadata() {
        let control = ControlState::new("same", ControlKind::Text);
        let root = LayoutNode::container(vec![
            LayoutNode::empty().with_ext(EXT_KEY, control.to_ext()),
            LayoutNode::empty().with_ext(EXT_KEY, control.to_ext()),
            LayoutNode::empty().with_ext(EXT_KEY, ExtValue::Bool(true)),
        ]);
        let diagnostics = diagnose_layout_controls(&root);
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(diagnostics[0].message, "control key is duplicated");
        assert_eq!(diagnostics[1].message, "control metadata must be a map");
    }
}
