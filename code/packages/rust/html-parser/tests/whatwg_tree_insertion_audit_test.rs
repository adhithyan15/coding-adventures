use coding_adventures_html_lexer::HtmlScriptingMode;
use coding_adventures_html_parser::{
    parse_html_fragment_for_context_with_options, parse_html_with_options, HtmlParseOptions,
};
use dom_core::{Document, DocumentType, Element, Node};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

const TREE_CONSTRUCTION_SMOKE: &str = include_str!("fixtures/html5lib-tree-construction-smoke.dat");
const WHATWG_TREE_INSERTION_AUDIT: &str = include_str!("fixtures/whatwg-tree-insertion-audit.json");

#[derive(Debug, Deserialize)]
struct TreeInsertionSuite {
    format: String,
    description: String,
    source_fixture: String,
    case_count: usize,
    counts_by_axis: BTreeMap<String, usize>,
    cases: Vec<TreeInsertionCase>,
}

#[derive(Debug, Deserialize)]
struct TreeInsertionCase {
    id: String,
    source: String,
    axis: String,
    reason: String,
}

#[derive(Debug)]
struct TreeConstructionCase {
    source: String,
    data: String,
    scripting: HtmlScriptingMode,
    fragment_context: Option<String>,
    document: Vec<String>,
}

#[test]
fn whatwg_tree_insertion_audit_fixture_parses() {
    let suite = load_suite();

    assert_eq!(suite.format, "whatwg-html-tree-insertion-audit/v1");
    assert_eq!(suite.source_fixture, "html5lib-tree-construction-smoke.dat");
    assert!(!suite.description.is_empty());
    assert_eq!(suite.case_count, suite.cases.len());
    assert!(suite.case_count >= 250);
    assert_axis_count(&suite, "adoption-agency", 19);
    assert_axis_count(&suite, "table-insertion", 15);
    assert_axis_count(&suite, "template-insertion", 100);
    assert_axis_count(&suite, "foreign-fragment", 60);
    assert_axis_count(&suite, "html-fragment", 75);
}

#[test]
fn whatwg_tree_insertion_audit_cases_match_parser_dom_dump() {
    let suite = load_suite();
    let smoke_cases = parse_tree_construction_cases(TREE_CONSTRUCTION_SMOKE)
        .into_iter()
        .map(|case| (case.source.clone(), case))
        .collect::<HashMap<_, _>>();

    for case in &suite.cases {
        assert!(
            !case.id.is_empty() && !case.axis.is_empty() && !case.reason.is_empty(),
            "case `{}` should carry audit metadata",
            case.source
        );
        let source_case = smoke_cases
            .get(&case.source)
            .unwrap_or_else(|| panic!("case `{}` should exist in smoke fixture", case.source));
        let options = HtmlParseOptions {
            scripting: source_case.scripting,
            ..HtmlParseOptions::default()
        };
        let actual = if let Some(fragment_context) = &source_case.fragment_context {
            let nodes = parse_html_fragment_for_context_with_options(
                &source_case.data,
                fragment_context,
                options,
            )
            .unwrap_or_else(|error| {
                panic!(
                    "case `{}` ({}) fragment parse failed: {error:?}",
                    case.id, case.axis
                )
            });
            dump_nodes(&nodes)
        } else {
            let document =
                parse_html_with_options(&source_case.data, options).unwrap_or_else(|error| {
                    panic!("case `{}` ({}) parse failed: {error:?}", case.id, case.axis)
                });
            dump_document(&document)
        };

        assert_eq!(
            actual, source_case.document,
            "case `{}` ({}) failed for input {:?}",
            case.id, case.axis, source_case.data
        );
    }
}

fn load_suite() -> TreeInsertionSuite {
    serde_json::from_str(WHATWG_TREE_INSERTION_AUDIT)
        .expect("WHATWG tree insertion audit fixture should parse")
}

fn assert_axis_count(suite: &TreeInsertionSuite, axis: &str, minimum: usize) {
    let count = suite.counts_by_axis.get(axis).copied().unwrap_or_default();
    assert!(
        count >= minimum,
        "expected at least {minimum} `{axis}` cases, found {count}"
    );
}

fn parse_tree_construction_cases(raw: &str) -> Vec<TreeConstructionCase> {
    let mut cases = Vec::new();
    let mut lines = raw.lines().peekable();

    let mut source = String::new();

    while let Some(line) = lines.next() {
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("#source ") {
            source = rest.to_string();
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

        let mut scripting = HtmlScriptingMode::Enabled;
        let mut fragment_context = None;
        while let Some(line) = lines.next() {
            if line == "#document" {
                break;
            }
            if line == "#document-fragment" {
                fragment_context = Some(
                    lines
                        .next()
                        .expect("document-fragment marker should name a context element")
                        .to_string(),
                );
                continue;
            }
            if line == "#script-off" {
                scripting = HtmlScriptingMode::Disabled;
            } else if line == "#script-on" {
                scripting = HtmlScriptingMode::Enabled;
            }
        }

        let mut document = Vec::new();
        while let Some(line) = lines.peek() {
            if *line == "#data" || line.starts_with("#source ") {
                break;
            }
            document.push(lines.next().expect("peeked line should exist").to_string());
        }
        while document.last().is_some_and(|line| line.is_empty()) {
            document.pop();
        }

        cases.push(TreeConstructionCase {
            source: std::mem::take(&mut source),
            data: data.join("\n"),
            scripting,
            fragment_context,
            document,
        });
    }

    cases
}

fn dump_document(document: &Document) -> Vec<String> {
    dump_nodes(&document.children)
}

fn dump_nodes(nodes: &[Node]) -> Vec<String> {
    let mut lines = Vec::new();
    for node in nodes {
        dump_node(node, 0, &mut lines);
    }
    lines
}

fn dump_node(node: &Node, depth: usize, lines: &mut Vec<String>) {
    match node {
        Node::DocumentType(doctype) => dump_doctype(doctype, depth, lines),
        Node::Element(element) => dump_element(element, depth, lines),
        Node::Text(text) => dump_text(&text.data, depth, lines),
        Node::Comment(comment) => dump_comment(&comment.data, depth, lines),
    }
}

fn dump_comment(comment: &str, depth: usize, lines: &mut Vec<String>) {
    let parts = comment.split('\n').collect::<Vec<_>>();
    if parts.len() == 1 {
        lines.push(format!("{}<!-- {} -->", prefix(depth), comment));
        return;
    }

    let last_index = parts.len() - 1;
    for (index, part) in parts.iter().enumerate() {
        match index {
            0 => lines.push(format!("{}<!-- {}", prefix(depth), part)),
            index if index == last_index => lines.push(format!("{part} -->")),
            _ => lines.push((*part).to_string()),
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
    if let Some(namespace) = &element.namespace {
        lines.push(format!("{}<{} {}>", prefix(depth), namespace, element.name));
    } else {
        lines.push(format!("{}<{}>", prefix(depth), element.name));
    }

    let mut attributes = element.attributes.iter().collect::<Vec<_>>();
    attributes.sort_by(|left, right| left.name.cmp(&right.name));
    for attribute in attributes {
        if element.namespace.is_some() {
            if let Some(local_name) = attribute.name.strip_prefix("xlink ") {
                lines.push(format!(
                    "{}xlink {}=\"{}\"",
                    prefix(depth + 1),
                    local_name,
                    attribute.value
                ));
                continue;
            }
            if let Some(local_name) = attribute.name.strip_prefix("xml ") {
                lines.push(format!(
                    "{}xml {}=\"{}\"",
                    prefix(depth + 1),
                    local_name,
                    attribute.value
                ));
                continue;
            }
        }
        lines.push(format!(
            "{}{}=\"{}\"",
            prefix(depth + 1),
            attribute.name,
            attribute.value
        ));
    }

    if element.name == "template" && element.namespace.is_none() {
        lines.push(format!("{}content", prefix(depth + 1)));
        for child in &element.children {
            dump_node(child, depth + 2, lines);
        }
    } else {
        for child in &element.children {
            dump_node(child, depth + 1, lines);
        }
    }
}

fn prefix(depth: usize) -> String {
    format!("| {}", "  ".repeat(depth))
}
