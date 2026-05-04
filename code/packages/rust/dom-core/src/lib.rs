//! Small DOM tree model for Venture browser packages.
//!
//! This crate is intentionally lower-level than `document-ast`: it preserves
//! HTML element names, attributes, comments, and doctypes so browser-facing
//! packages can later layer CSS, layout, and scripting semantics on top.

/// A parsed DOM document.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Document {
    pub children: Vec<Node>,
}

impl Document {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_child(&mut self, node: Node) {
        self.children.push(node);
    }
}

/// A DOM node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    DocumentType(DocumentType),
    Element(Element),
    Text(Text),
    Comment(Comment),
}

impl Node {
    pub fn element(name: impl Into<String>, attributes: Vec<Attribute>) -> Self {
        Self::Element(Element {
            name: name.into(),
            attributes,
            children: Vec::new(),
        })
    }

    pub fn text(data: impl Into<String>) -> Self {
        Self::Text(Text { data: data.into() })
    }

    pub fn comment(data: impl Into<String>) -> Self {
        Self::Comment(Comment { data: data.into() })
    }

    pub fn children(&self) -> Option<&[Node]> {
        match self {
            Self::Element(element) => Some(&element.children),
            _ => None,
        }
    }

    pub fn children_mut(&mut self) -> Option<&mut Vec<Node>> {
        match self {
            Self::Element(element) => Some(&mut element.children),
            _ => None,
        }
    }
}

/// A document type declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentType {
    pub name: Option<String>,
    pub public_identifier: Option<String>,
    pub system_identifier: Option<String>,
    pub force_quirks: bool,
}

/// An element node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    pub name: String,
    pub attributes: Vec<Attribute>,
    pub children: Vec<Node>,
}

impl Element {
    pub fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|attribute| attribute.name == name)
            .map(|attribute| attribute.value.as_str())
    }
}

/// An element attribute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    pub name: String,
    pub value: String,
}

/// A text node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Text {
    pub data: String,
}

/// A comment node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment {
    pub data: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_can_hold_element_text_and_comment_nodes() {
        let mut document = Document::new();
        document.push_child(Node::element(
            "p",
            vec![Attribute {
                name: "class".to_string(),
                value: "intro".to_string(),
            }],
        ));
        document.push_child(Node::text("hello"));
        document.push_child(Node::comment("note"));

        assert_eq!(document.children.len(), 3);
        match &document.children[0] {
            Node::Element(element) => {
                assert_eq!(element.name, "p");
                assert_eq!(element.attribute("class"), Some("intro"));
            }
            other => panic!("expected element, got {other:?}"),
        }
    }
}
