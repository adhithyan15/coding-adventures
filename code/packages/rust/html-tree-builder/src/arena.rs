//! The tree the builder grows: an arena of nodes with parent links.
//!
//! Tree construction keeps moving nodes around after it has made them. The
//! adoption agency algorithm (§13.2.6.4.7) lifts a run of children out of one
//! element and into a new one; foster parenting (§13.2.6.1) inserts in front of
//! a table rather than inside it; `<frameset>` removes the `<body>` it replaces.
//! Each of those needs to ask "who is this node's parent?" and to move a node
//! without copying its subtree.
//!
//! An owned tree (`Vec<Node>` inside each element, as `dom_core` has) can do
//! neither cheaply, so the builder keeps every node in one `Vec` and refers to
//! them by index:
//!
//! ```text
//!   nodes[0]  Document            children: [1, 2]
//!   nodes[1]  <!DOCTYPE html>     parent: 0
//!   nodes[2]  <html>              parent: 0   children: [3, 4]
//!   nodes[3]  <head>              parent: 2
//!   nodes[4]  <body>              parent: 2   children: [5]
//!   nodes[5]  "hello"             parent: 4
//! ```
//!
//! A node that is removed from the tree keeps its slot (nothing is ever freed
//! during a parse); it just has no parent. When the parse ends, [`Arena::to_document`]
//! walks from the Document node and builds today's `dom_core::Document`, so a
//! caller never sees the arena. BR02 P4 later makes this arena the real DOM.

use dom_core::{Attribute, Document, DocumentType, Element, Node};

/// The deepest element a converted tree contains (the Document is depth 0,
/// `<html>` depth 1). Blink uses the same figure.
///
/// Tree construction can nest without limit: `<div>` × 100,000 does, and the
/// adoption agency can re-nest elements after the fact. Rather than police
/// every place that moves nodes, the limit is applied once, where the tree
/// leaves the arena: an element at the limit is emitted without children, and
/// its children follow it as siblings. So no consumer — `dom_core`'s
/// recursive `Drop`, layout, a serializer — ever receives a deeper tree.
pub const MAX_TREE_DEPTH: usize = 512;

/// How far [`Arena::is_inclusive_ancestor`] walks before giving up.
pub const MAX_ANCESTOR_WALK: usize = 8 * MAX_TREE_DEPTH;

/// A node's index in the arena. Cheap to copy and compare, which is what the
/// stack of open elements and the list of active formatting elements hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub(crate) usize);

/// The three namespaces HTML tree construction creates elements in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Namespace {
    Html,
    Svg,
    MathMl,
}

impl Namespace {
    /// The name the html5lib tree format prints before a foreign element
    /// (`<svg svg>`, `<math math>`); HTML elements print none.
    fn dom_name(self) -> Option<String> {
        match self {
            Namespace::Html => None,
            Namespace::Svg => Some("svg".to_string()),
            Namespace::MathMl => Some("math".to_string()),
        }
    }
}

/// An element's own data. `template_contents` is the separate
/// `DocumentFragment` a `<template>` keeps its children in (§4.12.3): the
/// parser inserts into it rather than into the element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementData {
    pub namespace: Namespace,
    pub name: String,
    pub attributes: Vec<Attribute>,
    pub template_contents: Option<NodeId>,
}

impl ElementData {
    pub fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|attribute| attribute.name == name)
            .map(|attribute| attribute.value.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    Document,
    DocumentFragment,
    Doctype {
        name: Option<String>,
        public_identifier: Option<String>,
        system_identifier: Option<String>,
    },
    Element(ElementData),
    Text(String),
    Comment(String),
    ProcessingInstruction {
        target: String,
        data: String,
    },
}

#[derive(Debug, Clone)]
struct NodeRecord {
    kind: NodeKind,
    parent: Option<NodeId>,
    children: Vec<NodeId>,
    /// For a template's contents fragment: the `<template>` it belongs to.
    /// Not a parent (the fragment is not a child of anything), but depth
    /// is measured through it, because the contents print — and are laid
    /// out — as the template's children.
    host: Option<NodeId>,
}

#[derive(Debug, Clone)]
pub struct Arena {
    nodes: Vec<NodeRecord>,
}

impl Default for Arena {
    fn default() -> Self {
        Self::new()
    }
}

impl Arena {
    /// The Document node is always `NodeId(0)`.
    pub const DOCUMENT: NodeId = NodeId(0);

    pub fn new() -> Self {
        Self {
            nodes: vec![NodeRecord {
                kind: NodeKind::Document,
                parent: None,
                children: Vec::new(),
                host: None,
            }],
        }
    }

    fn push(&mut self, kind: NodeKind) -> NodeId {
        self.nodes.push(NodeRecord {
            kind,
            parent: None,
            children: Vec::new(),
            host: None,
        });
        NodeId(self.nodes.len() - 1)
    }

    /// "Create an element for a token" (§13.2.6.1), minus the parts that belong
    /// to script execution and custom elements. A `<template>` in the HTML
    /// namespace gets its contents fragment at the same time.
    pub fn create_element(
        &mut self,
        namespace: Namespace,
        name: impl Into<String>,
        attributes: Vec<Attribute>,
    ) -> NodeId {
        let name = name.into();
        let template_contents = (namespace == Namespace::Html && name == "template")
            .then(|| self.push(NodeKind::DocumentFragment));
        let element = self.push(NodeKind::Element(ElementData {
            namespace,
            name,
            attributes,
            template_contents,
        }));
        if let Some(contents) = template_contents {
            self.nodes[contents.0].host = Some(element);
        }
        element
    }

    pub fn create(&mut self, kind: NodeKind) -> NodeId {
        self.push(kind)
    }

    pub fn kind(&self, id: NodeId) -> &NodeKind {
        &self.nodes[id.0].kind
    }

    pub fn kind_mut(&mut self, id: NodeId) -> &mut NodeKind {
        &mut self.nodes[id.0].kind
    }

    pub fn element(&self, id: NodeId) -> Option<&ElementData> {
        match &self.nodes[id.0].kind {
            NodeKind::Element(data) => Some(data),
            _ => None,
        }
    }

    pub fn element_mut(&mut self, id: NodeId) -> Option<&mut ElementData> {
        match &mut self.nodes[id.0].kind {
            NodeKind::Element(data) => Some(data),
            _ => None,
        }
    }

    /// Whether `id` is an element in `namespace` whose local name is `name`.
    pub fn is(&self, id: NodeId, namespace: Namespace, name: &str) -> bool {
        self.element(id)
            .is_some_and(|data| data.namespace == namespace && data.name == name)
    }

    /// Shorthand for the common case: an HTML element named `name`.
    pub fn is_html(&self, id: NodeId, name: &str) -> bool {
        self.is(id, Namespace::Html, name)
    }

    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.nodes[id.0].parent
    }

    pub fn children(&self, id: NodeId) -> &[NodeId] {
        &self.nodes[id.0].children
    }

    /// Take `child` out of its parent's child list, if it has a parent.
    pub fn detach(&mut self, child: NodeId) {
        if let Some(parent) = self.nodes[child.0].parent.take() {
            self.nodes[parent.0].children.retain(|&node| node != child);
        }
    }

    /// Append `child` as `parent`'s last child, moving it if it already had a
    /// parent (as the DOM's `appendChild` does).
    pub fn append(&mut self, parent: NodeId, child: NodeId) {
        self.detach(child);
        self.nodes[child.0].parent = Some(parent);
        self.nodes[parent.0].children.push(child);
    }

    /// Insert `child` into `parent` immediately before `reference`, which must
    /// be one of `parent`'s children; otherwise append.
    pub fn insert_before(&mut self, parent: NodeId, child: NodeId, reference: NodeId) {
        self.detach(child);
        self.nodes[child.0].parent = Some(parent);
        let children = &mut self.nodes[parent.0].children;
        match children.iter().position(|&node| node == reference) {
            Some(index) => children.insert(index, child),
            None => children.push(child),
        }
    }

    /// The node `id` sits inside: its parent, or for a template's contents
    /// fragment, the template.
    pub fn container(&self, id: NodeId) -> Option<NodeId> {
        let record = &self.nodes[id.0];
        record.parent.or(record.host)
    }

    /// Whether `ancestor` is `node` or one of its ancestors, looking through
    /// template contents to their template. The walk is bounded by
    /// [`MAX_ANCESTOR_WALK`]; past it the answer is a conservative `true`, so
    /// a caller guarding against cycles skips the move rather than risking one.
    pub fn is_inclusive_ancestor(&self, ancestor: NodeId, node: NodeId) -> bool {
        let mut cursor = Some(node);
        for _ in 0..MAX_ANCESTOR_WALK {
            let Some(current) = cursor else {
                return false;
            };
            if current == ancestor {
                return true;
            }
            cursor = self.container(current);
        }
        true
    }

    /// Move every child of `from` to the end of `to`, in order (adoption agency
    /// step 4.17).
    pub fn reparent_children(&mut self, from: NodeId, to: NodeId) {
        let children = std::mem::take(&mut self.nodes[from.0].children);
        for child in &children {
            self.nodes[child.0].parent = Some(to);
        }
        self.nodes[to.0].children.extend(children);
    }

    /// The child immediately before `reference` in `parent`, or `parent`'s last
    /// child when there is no reference. Inserting a character looks here to
    /// decide whether to extend an existing Text node.
    pub fn child_before(&self, parent: NodeId, reference: Option<NodeId>) -> Option<NodeId> {
        let children = &self.nodes[parent.0].children;
        match reference {
            None => children.last().copied(),
            Some(reference) => {
                let index = children.iter().position(|&node| node == reference)?;
                index.checked_sub(1).map(|before| children[before])
            }
        }
    }

    /// Build the owned `dom_core` tree the rest of Venture uses. A template's
    /// contents become its children, which is how `dom_core` and the html5lib
    /// format show them.
    pub fn to_document(&self) -> Document {
        self.to_document_capped().0
    }

    /// [`Arena::to_document`], also saying whether [`MAX_TREE_DEPTH`] had to
    /// flatten anything.
    pub fn to_document_capped(&self) -> (Document, bool) {
        let mut document = Document::new();
        let mut flattened = false;
        for &child in self.children(Self::DOCUMENT) {
            if let Some(node) = self.to_node(child, &mut flattened) {
                document.push_child(node);
            }
        }
        (document, flattened)
    }

    /// The children of `parent` as `dom_core` nodes. Fragment parsing returns
    /// the children of its root `html` element this way.
    pub fn to_nodes(&self, parent: NodeId) -> Vec<Node> {
        self.children(parent)
            .iter()
            .filter_map(|&child| self.to_node(child, &mut false))
            .collect()
    }

    fn to_node(&self, id: NodeId, flattened: &mut bool) -> Option<Node> {
        // An explicit stack rather than recursion: the arena can nest
        // arbitrarily deep, and the builder itself never recurses on depth.
        enum Frame {
            Open(NodeId, usize),
            Close,
        }
        let mut finished: Vec<Vec<Node>> = vec![Vec::new()];
        let mut pending: Vec<Element> = Vec::new();
        let mut work = vec![Frame::Open(id, 1)];
        while let Some(frame) = work.pop() {
            match frame {
                Frame::Open(node, level) => match &self.nodes[node.0].kind {
                    NodeKind::Document | NodeKind::DocumentFragment => {}
                    NodeKind::Doctype {
                        name,
                        public_identifier,
                        system_identifier,
                    } => finished.last_mut()?.push(Node::DocumentType(DocumentType {
                        name: name.clone(),
                        public_identifier: public_identifier.clone(),
                        system_identifier: system_identifier.clone(),
                        force_quirks: false,
                    })),
                    NodeKind::Text(data) => finished.last_mut()?.push(Node::text(data.clone())),
                    NodeKind::Comment(data) => {
                        finished.last_mut()?.push(Node::comment(data.clone()))
                    }
                    NodeKind::ProcessingInstruction { target, data } => finished
                        .last_mut()?
                        .push(Node::processing_instruction(target.clone(), data.clone())),
                    NodeKind::Element(data) => {
                        let element = Element {
                            namespace: data.namespace.dom_name(),
                            name: data.name.clone(),
                            attributes: data.attributes.clone(),
                            children: Vec::new(),
                        };
                        let children = match data.template_contents {
                            Some(contents) => self.children(contents),
                            None => self.children(node),
                        };
                        if level >= MAX_TREE_DEPTH {
                            // At the limit: the element stays a leaf, and its
                            // children follow it at the same level.
                            *flattened |= !children.is_empty();
                            finished.last_mut()?.push(Node::Element(element));
                            for &child in children.iter().rev() {
                                work.push(Frame::Open(child, level));
                            }
                        } else {
                            pending.push(element);
                            finished.push(Vec::new());
                            work.push(Frame::Close);
                            for &child in children.iter().rev() {
                                work.push(Frame::Open(child, level + 1));
                            }
                        }
                    }
                },
                Frame::Close => {
                    let children = finished.pop()?;
                    let mut element = pending.pop()?;
                    element.children = children;
                    finished.last_mut()?.push(Node::Element(element));
                }
            }
        }
        finished.pop()?.pop()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_moves_a_node_between_parents() {
        let mut arena = Arena::new();
        let a = arena.create_element(Namespace::Html, "a", Vec::new());
        let b = arena.create_element(Namespace::Html, "b", Vec::new());
        let text = arena.create(NodeKind::Text("x".into()));
        arena.append(a, text);
        arena.append(b, text);
        assert!(arena.children(a).is_empty());
        assert_eq!(arena.children(b), &[text]);
        assert_eq!(arena.parent(text), Some(b));
    }

    #[test]
    fn insert_before_and_child_before() {
        let mut arena = Arena::new();
        let parent = arena.create_element(Namespace::Html, "div", Vec::new());
        let last = arena.create(NodeKind::Comment("last".into()));
        let first = arena.create(NodeKind::Comment("first".into()));
        arena.append(parent, last);
        arena.insert_before(parent, first, last);
        assert_eq!(arena.children(parent), &[first, last]);
        assert_eq!(arena.child_before(parent, Some(last)), Some(first));
        assert_eq!(arena.child_before(parent, Some(first)), None);
        assert_eq!(arena.child_before(parent, None), Some(last));
    }

    #[test]
    fn template_children_live_in_their_contents_and_print_as_children() {
        let mut arena = Arena::new();
        let template = arena.create_element(Namespace::Html, "template", Vec::new());
        let contents = arena.element(template).unwrap().template_contents.unwrap();
        let text = arena.create(NodeKind::Text("inside".into()));
        arena.append(contents, text);
        arena.append(Arena::DOCUMENT, template);
        let document = arena.to_document();
        let Node::Element(element) = &document.children[0] else {
            panic!("template should convert to an element")
        };
        assert_eq!(element.children, vec![Node::text("inside")]);
    }

    #[test]
    fn deep_trees_convert_without_recursion() {
        let mut arena = Arena::new();
        let mut parent = Arena::DOCUMENT;
        for _ in 0..200_000 {
            let child = arena.create_element(Namespace::Html, "div", Vec::new());
            arena.append(parent, child);
            parent = child;
        }
        let (document, flattened) = arena.to_document_capped();
        assert!(flattened);
        assert_eq!(document.children.len(), 1);
        // Walk the leftmost chain: it stops at the limit.
        let mut depth = 0;
        let mut level = &document.children;
        while let Some(Node::Element(element)) = level.first() {
            depth += 1;
            level = &element.children;
        }
        assert_eq!(depth, MAX_TREE_DEPTH);
        // Dropped normally: the capped tree is shallow enough for Drop.
    }

    #[test]
    fn reparent_children_moves_all_in_order() {
        let mut arena = Arena::new();
        let from = arena.create_element(Namespace::Html, "b", Vec::new());
        let to = arena.create_element(Namespace::Html, "i", Vec::new());
        let one = arena.create(NodeKind::Text("1".into()));
        let two = arena.create(NodeKind::Text("2".into()));
        arena.append(from, one);
        arena.append(from, two);
        arena.reparent_children(from, to);
        assert!(arena.children(from).is_empty());
        assert_eq!(arena.children(to), &[one, two]);
        assert_eq!(arena.parent(two), Some(to));
    }
}
