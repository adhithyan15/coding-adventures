//! §13.2.4.2 The stack of open elements.
//!
//! The stack grows downwards: the first element pushed (`<html>`) is the
//! topmost, and the *current node* is the bottommost, the one pushed last.
//! This file uses "above" and "below" the way the specification does:
//!
//! ```text
//!   index 0   <html>      topmost      ("above" everything)
//!   index 1   <body>
//!   index 2   <div>
//!   index 3   <p>         current node ("below" everything)
//! ```
//!
//! The scope predicates answer "is there an `X` open that an end tag could
//! reach?" by walking from the current node upwards and stopping at the first
//! element that bounds the scope. `<p>` inside a `<button>` is out of reach of
//! a `</p>` that follows the button's own content, for example, because
//! *button scope* stops at `button`.

use crate::arena::{Arena, Namespace, NodeId};
use crate::elements::bounds_default_scope;
use std::collections::{HashMap, HashSet};

/// Which of the specification's four scopes a lookup uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// "has an element in scope"
    Default,
    /// "in list item scope": default plus `ol`, `ul`.
    ListItem,
    /// "in button scope": default plus `button`.
    Button,
    /// "in table scope": only `html`, `table`, `template`.
    Table,
}

impl Scope {
    fn bounded_by(self, namespace: Namespace, name: &str) -> bool {
        let html = namespace == Namespace::Html;
        match self {
            Scope::Default => bounds_default_scope(namespace, name),
            Scope::ListItem => {
                bounds_default_scope(namespace, name) || (html && matches!(name, "ol" | "ul"))
            }
            Scope::Button => bounds_default_scope(namespace, name) || (html && name == "button"),
            Scope::Table => html && matches!(name, "html" | "table" | "template"),
        }
    }
}

/// The stack, plus a count of the HTML elements on it by tag name.
///
/// The count is what keeps scope checks cheap on hostile input. Every
/// `<div>` start tag asks "is a `p` open in button scope?"; with no `p` open,
/// the walk would visit the whole stack, so 100,000 nested `<div>`s cost
/// 5·10⁹ steps. When the count for a name is zero the answer is "no" without
/// walking.
#[derive(Debug, Clone, Default)]
pub struct OpenElements {
    stack: Vec<NodeId>,
    members: HashSet<NodeId>,
    html_counts: HashMap<String, usize>,
}

impl OpenElements {
    fn html_name(arena: &Arena, node: NodeId) -> Option<&str> {
        arena
            .element(node)
            .filter(|element| element.namespace == Namespace::Html)
            .map(|element| element.name.as_str())
    }

    fn count_in(&mut self, arena: &Arena, node: NodeId) {
        self.members.insert(node);
        if let Some(name) = Self::html_name(arena, node) {
            *self.html_counts.entry(name.to_string()).or_default() += 1;
        }
    }

    fn count_out(&mut self, arena: &Arena, node: NodeId) {
        self.members.remove(&node);
        if let Some(name) = Self::html_name(arena, node) {
            if let Some(count) = self.html_counts.get_mut(name) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    self.html_counts.remove(name);
                }
            }
        }
    }

    fn open_count(&self, name: &str) -> usize {
        self.html_counts.get(name).copied().unwrap_or(0)
    }

    pub fn push(&mut self, arena: &Arena, node: NodeId) {
        self.count_in(arena, node);
        self.stack.push(node);
    }

    pub fn pop(&mut self, arena: &Arena) -> Option<NodeId> {
        let node = self.stack.pop()?;
        self.count_out(arena, node);
        Some(node)
    }

    /// The bottommost node.
    pub fn current(&self) -> Option<NodeId> {
        self.stack.last().copied()
    }

    /// The topmost node, the `<html>` element once one exists.
    pub fn top(&self) -> Option<NodeId> {
        self.stack.first().copied()
    }

    pub fn len(&self) -> usize {
        self.stack.len()
    }

    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<NodeId> {
        self.stack.get(index).copied()
    }

    pub fn as_slice(&self) -> &[NodeId] {
        &self.stack
    }

    /// O(1): the stack is asked this for every entry reconstruction visits.
    pub fn contains(&self, node: NodeId) -> bool {
        self.members.contains(&node)
    }

    /// Searches from the current node upwards: the node asked about is almost
    /// always near the bottom of the stack.
    pub fn position(&self, node: NodeId) -> Option<usize> {
        if !self.contains(node) {
            return None;
        }
        self.stack.iter().rposition(|&candidate| candidate == node)
    }

    pub fn remove(&mut self, arena: &Arena, node: NodeId) {
        if let Some(index) = self.position(node) {
            self.stack.remove(index);
            self.count_out(arena, node);
        }
    }

    pub fn insert(&mut self, arena: &Arena, index: usize, node: NodeId) {
        self.count_in(arena, node);
        self.stack.insert(index, node);
    }

    pub fn replace(&mut self, arena: &Arena, old: NodeId, new: NodeId) {
        if let Some(index) = self.position(old) {
            self.count_out(arena, old);
            self.count_in(arena, new);
            self.stack[index] = new;
        }
    }

    /// Whether an HTML element named `name` is on the stack at all.
    pub fn contains_html(&self, name: &str) -> bool {
        self.open_count(name) > 0
    }

    /// "Has an element in the specific scope": walk up from the current node;
    /// an HTML element named `name` answers yes, and a scope boundary answers
    /// no. The `html` element bounds every scope, so the walk always ends.
    pub fn has_in_scope(&self, arena: &Arena, name: &str, scope: Scope) -> bool {
        self.has_any_in_scope(arena, &[name], scope)
    }

    /// The same walk for "any of these names" (`h1`…`h6` share one rule).
    pub fn has_any_in_scope(&self, arena: &Arena, names: &[&str], scope: Scope) -> bool {
        if names.iter().all(|name| self.open_count(name) == 0) {
            return false;
        }
        for &node in self.stack.iter().rev() {
            let Some(element) = arena.element(node) else {
                continue;
            };
            if element.namespace == Namespace::Html && names.contains(&element.name.as_str()) {
                return true;
            }
            if scope.bounded_by(element.namespace, &element.name) {
                return false;
            }
        }
        false
    }

    /// The same walk for one particular node (`</form>` and the adoption agency
    /// ask about a node, not a name).
    pub fn has_node_in_scope(&self, arena: &Arena, target: NodeId, scope: Scope) -> bool {
        if Self::html_name(arena, target).is_some_and(|name| self.open_count(name) == 0) {
            return false;
        }
        for &node in self.stack.iter().rev() {
            if node == target {
                return true;
            }
            if let Some(element) = arena.element(node) {
                if scope.bounded_by(element.namespace, &element.name) {
                    return false;
                }
            }
        }
        false
    }

    /// Pop until an HTML element named `name` has been popped.
    pub fn pop_until_html(&mut self, arena: &Arena, name: &str) {
        self.pop_until_any_html(arena, &[name]);
    }

    pub fn pop_until_any_html(&mut self, arena: &Arena, names: &[&str]) {
        if names.iter().all(|name| self.open_count(name) == 0) {
            return;
        }
        while let Some(node) = self.pop(arena) {
            if Self::html_name(arena, node).is_some_and(|name| names.contains(&name)) {
                break;
            }
        }
    }

    /// Pop until `target` itself has been popped.
    pub fn pop_until_node(&mut self, arena: &Arena, target: NodeId) {
        if !self.contains(target) {
            return;
        }
        while let Some(node) = self.pop(arena) {
            if node == target {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stack_of(arena: &mut Arena, names: &[&str]) -> OpenElements {
        let mut stack = OpenElements::default();
        for name in names {
            let node = arena.create_element(Namespace::Html, *name, Vec::new());
            stack.push(arena, node);
        }
        stack
    }

    #[test]
    fn button_scope_stops_at_button_but_default_scope_does_not() {
        let mut arena = Arena::new();
        let stack = stack_of(&mut arena, &["html", "body", "p", "button", "span"]);
        assert!(!stack.has_in_scope(&arena, "p", Scope::Button));
        assert!(stack.has_in_scope(&arena, "p", Scope::Default));
    }

    #[test]
    fn list_item_scope_stops_at_lists() {
        let mut arena = Arena::new();
        let stack = stack_of(&mut arena, &["html", "body", "li", "ul"]);
        assert!(!stack.has_in_scope(&arena, "li", Scope::ListItem));
        assert!(stack.has_in_scope(&arena, "li", Scope::Default));
    }

    #[test]
    fn table_scope_only_stops_at_table_html_template() {
        let mut arena = Arena::new();
        let stack = stack_of(
            &mut arena,
            &["html", "body", "table", "tbody", "tr", "td", "div"],
        );
        assert!(stack.has_in_scope(&arena, "tr", Scope::Table));
        assert!(!stack.has_in_scope(&arena, "body", Scope::Table));
    }

    #[test]
    fn select_bounds_default_scope() {
        let mut arena = Arena::new();
        let stack = stack_of(&mut arena, &["html", "body", "p", "select", "option"]);
        assert!(stack.has_in_scope(&arena, "select", Scope::Default));
        assert!(!stack.has_in_scope(&arena, "p", Scope::Default));
    }

    #[test]
    fn pop_until_and_node_scope() {
        let mut arena = Arena::new();
        let mut stack = stack_of(&mut arena, &["html", "body", "div", "p", "b"]);
        let div = stack.get(2).unwrap();
        assert!(stack.has_node_in_scope(&arena, div, Scope::Default));
        stack.pop_until_html(&arena, "p");
        assert_eq!(stack.current(), Some(div));
        stack.pop_until_node(&arena, div);
        assert_eq!(stack.len(), 2);
        assert!(stack.contains_html("body"));
        assert!(!stack.contains_html("p"));
    }
}
