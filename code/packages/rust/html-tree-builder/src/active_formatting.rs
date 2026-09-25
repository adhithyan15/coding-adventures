//! §13.2.4.3 The list of active formatting elements.
//!
//! HTML lets formatting run across block boundaries that the tree cannot
//! represent:
//!
//! ```html
//! <p><b>bold <p>still bold</b> plain
//! ```
//!
//! The second `<p>` closes the first one, and the `<b>` inside it, but the
//! author meant the bold to carry on. So when `<b>` opens, the parser also
//! notes it in this list; before inserting more text it *reconstructs* any
//! listed element that is no longer open (a fresh `<b>` inside the second
//! `<p>`). The adoption agency algorithm uses the same list to repair
//! mis-nested end tags.
//!
//! *Markers* fence off regions: entering `<applet>`, `<object>`, `<marquee>`,
//! `<template>`, a table cell or a caption pushes one, so formatting from
//! outside is never reconstructed inside.

use crate::arena::{Arena, NodeId};
use dom_core::Attribute;

/// The token an element was created for. Reconstruction and the adoption
/// agency create a *new* element "for the token for which the element was
/// created", so the list keeps a copy of each one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormattingToken {
    pub name: String,
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    Marker,
    Element {
        node: NodeId,
        token: FormattingToken,
    },
}

impl Entry {
    pub fn node(&self) -> Option<NodeId> {
        match self {
            Entry::Marker => None,
            Entry::Element { node, .. } => Some(*node),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ActiveFormatting {
    entries: Vec<Entry>,
}

impl ActiveFormatting {
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn push_marker(&mut self) {
        self.entries.push(Entry::Marker);
    }

    /// "Push onto the list of active formatting elements", including the
    /// Noah's Ark clause: if three entries after the last marker already have
    /// the same tag name and the same attributes as this one, the earliest of
    /// them is removed first. That caps how much `<b><b><b><b>…` a page can
    /// make every later text insertion reconstruct.
    pub fn push(&mut self, node: NodeId, token: FormattingToken) {
        let mut identical = Vec::new();
        for (index, entry) in self.entries.iter().enumerate().rev() {
            match entry {
                Entry::Marker => break,
                Entry::Element {
                    token: existing, ..
                } => {
                    if existing.name == token.name
                        && same_attributes(&existing.attributes, &token.attributes)
                    {
                        identical.push(index);
                    }
                }
            }
        }
        if identical.len() >= 3 {
            // `identical` runs from the latest to the earliest.
            let earliest = *identical.last().expect("three matches");
            self.entries.remove(earliest);
        }
        self.entries.push(Entry::Element { node, token });
    }

    pub fn position(&self, node: NodeId) -> Option<usize> {
        self.entries
            .iter()
            .position(|entry| entry.node() == Some(node))
    }

    pub fn contains(&self, node: NodeId) -> bool {
        self.position(node).is_some()
    }

    pub fn remove(&mut self, node: NodeId) {
        if let Some(index) = self.position(node) {
            self.entries.remove(index);
        }
    }

    pub fn get(&self, index: usize) -> Option<&Entry> {
        self.entries.get(index)
    }

    pub fn token_for(&self, node: NodeId) -> Option<&FormattingToken> {
        self.entries.iter().find_map(|entry| match entry {
            Entry::Element {
                node: candidate,
                token,
            } if *candidate == node => Some(token),
            _ => None,
        })
    }

    /// Point the entry for `old` at `new`, keeping its token (reconstruction and
    /// the adoption agency replace an entry in place).
    pub fn replace_node(&mut self, old: NodeId, new: NodeId) {
        for entry in &mut self.entries {
            if let Entry::Element { node, .. } = entry {
                if *node == old {
                    *node = new;
                }
            }
        }
    }

    pub fn insert(&mut self, index: usize, entry: Entry) {
        let index = index.min(self.entries.len());
        self.entries.insert(index, entry);
    }

    /// The last element (after the last marker) whose tag name is `name`, the
    /// adoption agency's "formatting element".
    pub fn last_named_after_marker(&self, arena: &Arena, name: &str) -> Option<NodeId> {
        for entry in self.entries.iter().rev() {
            match entry {
                Entry::Marker => return None,
                Entry::Element { node, .. } if arena.is_html(*node, name) => return Some(*node),
                Entry::Element { .. } => {}
            }
        }
        None
    }

    /// "Clear the list of active formatting elements up to the last marker".
    pub fn clear_to_last_marker(&mut self) {
        while let Some(entry) = self.entries.pop() {
            if entry == Entry::Marker {
                break;
            }
        }
    }
}

/// Attributes compare as a set: the same names with the same values, in any
/// order (the specification's "same attributes").
fn same_attributes(left: &[Attribute], right: &[Attribute]) -> bool {
    left.len() == right.len()
        && left.iter().all(|attribute| {
            right
                .iter()
                .any(|other| other.name == attribute.name && other.value == attribute.value)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::Namespace;

    fn token(name: &str, attributes: &[(&str, &str)]) -> FormattingToken {
        FormattingToken {
            name: name.into(),
            attributes: attributes
                .iter()
                .map(|(name, value)| Attribute {
                    name: (*name).into(),
                    value: (*value).into(),
                })
                .collect(),
        }
    }

    #[test]
    fn noahs_ark_keeps_at_most_three_identical_entries() {
        let mut arena = Arena::new();
        let mut list = ActiveFormatting::default();
        let nodes: Vec<_> = (0..4)
            .map(|_| arena.create_element(Namespace::Html, "b", Vec::new()))
            .collect();
        for &node in &nodes {
            list.push(node, token("b", &[]));
        }
        assert_eq!(list.len(), 3);
        assert!(!list.contains(nodes[0]));
        assert!(list.contains(nodes[3]));
    }

    #[test]
    fn noahs_ark_compares_attributes_as_a_set_and_stops_at_markers() {
        let mut arena = Arena::new();
        let mut list = ActiveFormatting::default();
        let mut push = |list: &mut ActiveFormatting, attributes: &[(&str, &str)]| {
            let node = arena.create_element(Namespace::Html, "font", Vec::new());
            list.push(node, token("font", attributes));
            node
        };
        let first = push(&mut list, &[("a", "1"), ("b", "2")]);
        list.push_marker();
        push(&mut list, &[("b", "2"), ("a", "1")]);
        push(&mut list, &[("a", "1"), ("b", "2")]);
        push(&mut list, &[("a", "1"), ("b", "2")]);
        push(&mut list, &[("a", "1"), ("b", "2")]);
        // The one before the marker survives; one of the four after it went.
        assert!(list.contains(first));
        assert_eq!(list.len(), 5);
    }

    #[test]
    fn clear_to_last_marker_and_lookup() {
        let mut arena = Arena::new();
        let mut list = ActiveFormatting::default();
        let outer = arena.create_element(Namespace::Html, "a", Vec::new());
        let inner = arena.create_element(Namespace::Html, "a", Vec::new());
        list.push(outer, token("a", &[]));
        list.push_marker();
        assert_eq!(list.last_named_after_marker(&arena, "a"), None);
        list.push(inner, token("a", &[]));
        assert_eq!(list.last_named_after_marker(&arena, "a"), Some(inner));
        list.clear_to_last_marker();
        assert_eq!(
            list.entries(),
            &[Entry::Element {
                node: outer,
                token: token("a", &[])
            }]
        );
    }
}
