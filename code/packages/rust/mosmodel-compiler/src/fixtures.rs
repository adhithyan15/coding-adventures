//! # List fixtures (#15428)
//!
//! A story fixture sets a slot's value for a preview. Scalars have always
//! travelled as plain text (`"Board"`, `"3"`, `"true"`); every emitter turns
//! that text into a literal of the slot's type.
//!
//! A list could not travel at all: `mosaic-compile` dropped it with a warning,
//! so every list-driven component (`ButtonGroup`, `Tabs`, `SegmentedControl`,
//! most of Engram) previewed with nothing in it. Lists now travel as compact
//! JSON text in the same `slot -> String` map, and this module is the one place
//! that reads that text back, against the slot's declared type, for every
//! emitter.
//!
//! ```text
//!   stories.json              mosaic-compile               emitter
//!   "options": [["A","a"]] -> "[[\"A\",\"a\"]]"   -> parse_list_fixture(list<list<text>>, …)
//!                                                         = ListFixture::TextRows([["A","a"]])
//!                                                  -> [["A", "a"]]        (TypeScript)
//! ```
//!
//! Only the shapes the kernel's list slots actually carry are accepted:
//!
//! | slot type          | accepted fixture       | result             |
//! |--------------------|------------------------|--------------------|
//! | `list<text>`       | `["a", "b"]`           | `Text`             |
//! | `list<list<text>>` | `[["a"], ["b", "c"]]`  | `TextRows`         |
//! | anything else      | anything               | `None`             |
//!
//! `None` means "not a fixture this slot can use"; the emitter keeps its
//! generated sample rather than writing something that does not type-check.
//! Rejecting mismatches loudly is the compiler's job (`--strict-fixtures`),
//! which sees the value before any emitter does.

use crate::{ListInnerType, SlotType};
use coding_adventures_bounded_json::{parse, JsonValue};

/// A list fixture, decoded for a slot whose type it matches.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ListFixture {
    /// For a `list<text>` slot.
    Text(Vec<String>),
    /// For a `list<list<text>>` slot: one row per outer element.
    TextRows(Vec<Vec<String>>),
}

/// Decode the JSON text `mosaic-compile` writes for a list fixture, if it is
/// the right shape for `slot_type`.
///
/// Returns `None` when the slot is not a text list, when the text is not JSON,
/// or when its shape does not match (a flat list for a list of rows, a number
/// inside a text list, and so on).
pub fn parse_list_fixture(slot_type: &SlotType, value: &str) -> Option<ListFixture> {
    let SlotType::List(inner) = slot_type else {
        return None;
    };
    let JsonValue::Array(items) = parse(value).ok()? else {
        return None;
    };
    match inner.as_ref() {
        ListInnerType::Text => items
            .iter()
            .map(text)
            .collect::<Option<Vec<_>>>()
            .map(ListFixture::Text),
        ListInnerType::List(row) if **row == ListInnerType::Text => items
            .iter()
            .map(|item| match item {
                JsonValue::Array(cells) => cells.iter().map(text).collect::<Option<Vec<_>>>(),
                _ => None,
            })
            .collect::<Option<Vec<_>>>()
            .map(ListFixture::TextRows),
        _ => None,
    }
}

fn text(value: &JsonValue) -> Option<String> {
    match value {
        JsonValue::String(s) => Some(s.clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_list() -> SlotType {
        SlotType::List(Box::new(ListInnerType::Text))
    }

    fn text_rows() -> SlotType {
        SlotType::List(Box::new(ListInnerType::List(Box::new(ListInnerType::Text))))
    }

    #[test]
    fn text_lists_decode() {
        assert_eq!(
            parse_list_fixture(&text_list(), r#"["a","b \"quoted\""]"#),
            Some(ListFixture::Text(vec!["a".into(), "b \"quoted\"".into()]))
        );
        assert_eq!(
            parse_list_fixture(&text_list(), "[]"),
            Some(ListFixture::Text(vec![]))
        );
    }

    #[test]
    fn rows_decode() {
        assert_eq!(
            parse_list_fixture(&text_rows(), r#"[["List","List, selected"],[]]"#),
            Some(ListFixture::TextRows(vec![
                vec!["List".into(), "List, selected".into()],
                vec![],
            ]))
        );
    }

    #[test]
    fn mismatched_shapes_are_refused() {
        // A flat list for a list of rows, and rows for a flat list.
        assert_eq!(parse_list_fixture(&text_rows(), r#"["a"]"#), None);
        assert_eq!(parse_list_fixture(&text_list(), r#"[["a"]]"#), None);
        // Non-text cells, non-JSON, non-arrays, non-list slots.
        assert_eq!(parse_list_fixture(&text_list(), r#"["a", 1]"#), None);
        assert_eq!(parse_list_fixture(&text_list(), "not json"), None);
        assert_eq!(parse_list_fixture(&text_list(), r#""a""#), None);
        assert_eq!(parse_list_fixture(&SlotType::Text, r#"["a"]"#), None);
        let numbers = SlotType::List(Box::new(ListInnerType::Number));
        assert_eq!(parse_list_fixture(&numbers, "[1,2]"), None);
    }
}
