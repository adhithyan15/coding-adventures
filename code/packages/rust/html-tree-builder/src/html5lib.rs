//! The html5lib-tests tree format (`| <html>`, `|   "text"`), which the
//! tree-construction corpus states its expected trees in. It lives in the
//! crate rather than in a test so that the differential tests (BR03 §4), the
//! corpus harness and any future debugging tool print trees one way.

use dom_core::{Document, DocumentType, Element, Node};

/// Every line of `document` in html5lib's format.
pub fn document_lines(document: &Document) -> Vec<String> {
    node_lines(&document.children)
}

/// Every line of a list of sibling nodes (a fragment's result) in
/// html5lib's format.
pub fn node_lines(nodes: &[Node]) -> Vec<String> {
    let mut lines = Vec::new();
    // An explicit stack: documents nest deeper than the call stack allows.
    let mut work: Vec<(&Node, usize)> = nodes.iter().rev().map(|node| (node, 0)).collect();
    while let Some((node, depth)) = work.pop() {
        match node {
            Node::DocumentType(doctype) => lines.push(doctype_line(doctype, depth)),
            Node::Text(text) => push_multiline(&mut lines, depth, "\"", &text.data, "\""),
            Node::Comment(comment) => {
                push_multiline(&mut lines, depth, "<!-- ", &comment.data, " -->")
            }
            Node::ProcessingInstruction(pi) => {
                lines.push(format!("{}<?{} {}?>", prefix(depth), pi.target, pi.data))
            }
            Node::Element(element) => {
                element_lines(element, depth, &mut lines);
                let child_depth = if element.name == "template" && element.namespace.is_none() {
                    lines.push(format!("{}content", prefix(depth + 1)));
                    depth + 2
                } else {
                    depth + 1
                };
                for child in element.children.iter().rev() {
                    work.push((child, child_depth));
                }
            }
        }
    }
    lines
}

fn push_multiline(lines: &mut Vec<String>, depth: usize, open: &str, data: &str, close: &str) {
    lines.push(format!("{}{open}{data}{close}", prefix(depth)));
    // A value with newlines spans several lines, only the first prefixed.
    let joined = lines.pop().expect("just pushed");
    lines.extend(joined.split('\n').map(str::to_string));
}

fn doctype_line(doctype: &DocumentType, depth: usize) -> String {
    let name = doctype.name.as_deref().unwrap_or("");
    match (
        doctype.public_identifier.as_deref(),
        doctype.system_identifier.as_deref(),
    ) {
        (None, None) => format!("{}<!DOCTYPE {name}>", prefix(depth)),
        (public, system) => format!(
            "{}<!DOCTYPE {name} \"{}\" \"{}\">",
            prefix(depth),
            public.unwrap_or(""),
            system.unwrap_or("")
        ),
    }
}

fn element_lines(element: &Element, depth: usize, lines: &mut Vec<String>) {
    match &element.namespace {
        Some(namespace) => lines.push(format!("{}<{namespace} {}>", prefix(depth), element.name)),
        None => lines.push(format!("{}<{}>", prefix(depth), element.name)),
    }
    let mut attributes = element.attributes.iter().collect::<Vec<_>>();
    attributes.sort_by(|left, right| left.name.cmp(&right.name));
    for attribute in attributes {
        lines.push(format!(
            "{}{}=\"{}\"",
            prefix(depth + 1),
            attribute.name,
            attribute.value
        ));
    }
}

fn prefix(depth: usize) -> String {
    format!("| {}", "  ".repeat(depth))
}

#[cfg(test)]
mod tests {
    use super::*;
    use dom_core::Attribute;

    #[test]
    fn prints_nested_elements_text_and_comments() {
        let mut document = Document::new();
        document.push_child(Node::DocumentType(DocumentType {
            name: Some("html".into()),
            public_identifier: None,
            system_identifier: None,
            force_quirks: false,
        }));
        document.push_child(Node::Element(Element {
            namespace: None,
            name: "p".into(),
            attributes: vec![
                Attribute {
                    name: "id".into(),
                    value: "x".into(),
                },
                Attribute {
                    name: "class".into(),
                    value: "y".into(),
                },
            ],
            children: vec![Node::text("a\nb"), Node::comment("c")],
        }));
        assert_eq!(
            document_lines(&document),
            vec![
                "| <!DOCTYPE html>",
                "| <p>",
                "|   class=\"y\"",
                "|   id=\"x\"",
                "|   \"a",
                "b\"",
                "|   <!-- c -->",
            ]
        );
    }

    #[test]
    fn prints_template_content_and_foreign_namespaces() {
        let template = Node::Element(Element {
            namespace: None,
            name: "template".into(),
            attributes: Vec::new(),
            children: vec![Node::namespaced_element("svg", "svg", Vec::new())],
        });
        assert_eq!(
            node_lines(&[template]),
            vec!["| <template>", "|   content", "|     <svg svg>"]
        );
    }

    #[test]
    fn prints_doctype_identifiers() {
        let doctype = DocumentType {
            name: Some("html".into()),
            public_identifier: Some("p".into()),
            system_identifier: None,
            force_quirks: false,
        };
        assert_eq!(doctype_line(&doctype, 0), "| <!DOCTYPE html \"p\" \"\">");
    }
}
