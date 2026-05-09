use coding_adventures_html_parser::parse_html;
use dom_core::{Document, DocumentType, Element, Node};

const TREE_CONSTRUCTION_SMOKE: &str = include_str!("fixtures/html5lib-tree-construction-smoke.dat");

#[derive(Debug)]
struct TreeConstructionCase {
    data: String,
    document: Vec<String>,
}

#[test]
fn html5lib_tree_construction_smoke_cases_match_dom_dump() {
    let cases = parse_tree_construction_cases(TREE_CONSTRUCTION_SMOKE);
    assert!(!cases.is_empty(), "fixture should contain cases");

    for (index, case) in cases.iter().enumerate() {
        let document = parse_html(&case.data).expect("parser should accept any HTML input");
        let actual = dump_document(&document);
        assert_eq!(
            actual,
            case.document,
            "tree-construction smoke case {} failed for input {:?}",
            index + 1,
            case.data
        );
    }
}

fn parse_tree_construction_cases(raw: &str) -> Vec<TreeConstructionCase> {
    let mut cases = Vec::new();
    let mut lines = raw.lines().peekable();

    while let Some(line) = lines.next() {
        if line.is_empty() {
            continue;
        }
        assert_eq!(line, "#data");

        let mut data = Vec::new();
        for line in lines.by_ref() {
            if line == "#errors" {
                break;
            }
            data.push(line);
        }

        for line in lines.by_ref() {
            if line == "#document" {
                break;
            }
        }

        let mut document = Vec::new();
        while let Some(line) = lines.peek() {
            if *line == "#data" {
                break;
            }
            document.push(lines.next().expect("peeked line should exist").to_string());
        }
        while document.last().is_some_and(|line| line.is_empty()) {
            document.pop();
        }

        cases.push(TreeConstructionCase {
            data: data.join("\n"),
            document,
        });
    }

    cases
}

fn dump_document(document: &Document) -> Vec<String> {
    let mut lines = Vec::new();
    for node in &document.children {
        dump_node(node, 0, &mut lines);
    }
    lines
}

fn dump_node(node: &Node, depth: usize, lines: &mut Vec<String>) {
    match node {
        Node::DocumentType(doctype) => dump_doctype(doctype, depth, lines),
        Node::Element(element) => dump_element(element, depth, lines),
        Node::Text(text) => dump_text(&text.data, depth, lines),
        Node::Comment(comment) => {
            lines.push(format!("{}<!-- {} -->", prefix(depth), comment.data));
        }
    }
}

fn dump_text(text: &str, depth: usize, lines: &mut Vec<String>) {
    let parts = text.split('\n').collect::<Vec<_>>();
    if parts.len() == 1 {
        lines.push(format!("{}\"{}\"", prefix(depth), text));
        return;
    }

    let last_index = parts.len() - 1;
    for (index, part) in parts.iter().enumerate() {
        match index {
            0 => lines.push(format!("{}\"{}", prefix(depth), part)),
            index if index == last_index => lines.push(format!("{part}\"")),
            _ => lines.push((*part).to_string()),
        }
    }
}

fn dump_doctype(doctype: &DocumentType, depth: usize, lines: &mut Vec<String>) {
    let name = doctype.name.as_deref().unwrap_or("");
    match (
        doctype.public_identifier.as_deref(),
        doctype.system_identifier.as_deref(),
    ) {
        (Some(public), Some(system)) => {
            lines.push(format!(
                "{}<!DOCTYPE {} \"{}\" \"{}\">",
                prefix(depth),
                name,
                public,
                system
            ));
        }
        (Some(public), None) => {
            lines.push(format!(
                "{}<!DOCTYPE {} \"{}\" \"\">",
                prefix(depth),
                name,
                public
            ));
        }
        (None, Some(system)) => {
            lines.push(format!(
                "{}<!DOCTYPE {} \"\" \"{}\">",
                prefix(depth),
                name,
                system
            ));
        }
        (None, None) => lines.push(format!("{}<!DOCTYPE {}>", prefix(depth), name)),
    }
}

fn dump_element(element: &Element, depth: usize, lines: &mut Vec<String>) {
    lines.push(format!("{}<{}>", prefix(depth), element.name));

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

    for child in &element.children {
        dump_node(child, depth + 1, lines);
    }
}

fn prefix(depth: usize) -> String {
    format!("| {}", "  ".repeat(depth))
}
