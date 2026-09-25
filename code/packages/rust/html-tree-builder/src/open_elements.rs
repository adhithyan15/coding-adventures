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

#[derive(Debug, Clone, Default)]
pub struct OpenElements {
    stack: Vec<NodeId>,
}

impl OpenElements {
    pub fn push(&mut self, node: NodeId) {
        self.stack.push(node);
    }

    pub fn pop(&mut self) -> Option<NodeId> {
        self.stack.pop()
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

    pub fn contains(&self, node: NodeId) -> bool {
        self.stack.contains(&node)
    }

    pub fn position(&self, node: NodeId) -> Option<usize> {
        self.stack.iter().position(|&candidate| candidate == node)
    }

    pub fn remove(&mut self, node: NodeId) {
        if let Some(index) = self.position(node) {
            self.stack.remove(index);
        }
    }

    pub fn insert(&mut self, index: usize, node: NodeId) {
        self.stack.insert(index, node);
    }

    pub fn replace(&mut self, old: NodeId, new: NodeId) {
        if let Some(index) = self.position(old) {
            self.stack[index] = new;
        }
    }

    /// Whether an HTML element named `name` is on the stack at all.
    pub fn contains_html(&self, arena: &Arena, name: &str) -> bool {
        self.stack.iter().any(|&node| arena.is_html(node, name))
    }

    /// "Has an element in the specific scope": walk up from the current node;
    /// an HTML element named `name` answers yes, and a scope boundary answers
    /// no. The `html` element bounds every scope, so the walk always ends.
    pub fn has_in_scope(&self, arena: &Arena, name: &str, scope: Scope) -> bool {
        self.has_any_in_scope(arena, &[name], scope)
    }

    /// The same walk for "any of these names" (`h1`…`h6` share one rule).
    pub fn has_any_in_scope(&self, arena: &Arena, names: &[&str], scope: Scope) -> bool {
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
        while let Some(node) = self.stack.pop() {
            if arena.element(node).is_some_and(|element| {
                element.namespace == Namespace::Html && names.contains(&element.name.as_str())
            }) {
                break;
            }
        }
    }

    /// Pop until `target` itself has been popped.
    pub fn pop_until_node(&mut self, target: NodeId) {
        while let Some(node) = self.stack.pop() {
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
            stack.push(node);
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
        stack.pop_until_node(div);
        assert_eq!(stack.len(), 2);
        assert!(stack.contains_html(&arena, "body"));
    }
}
