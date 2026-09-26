use coding_adventures_html_lexer::HtmlScriptingMode;
use coding_adventures_html_parser::{
    parse_html_fragment_for_context_with_diagnostics_and_options,
    parse_html_fragment_for_context_with_options, parse_html_with_diagnostics_and_options,
    parse_html_with_options, HtmlParseOptions,
};
use dom_core::{Document, DocumentType, Element, Node};
use std::collections::BTreeMap;

/// Tree-construction cases this parser is known not to pass, as
/// `source -> reason` (`fixtures/tree-construction-expected-failures.txt`).
/// BR02 §3: a case we cannot pass is listed, visibly, and never special-cased
/// in the parser. Every test that runs corpus cases consults this one list.
const EXPECTED_FAILURES: &str = include_str!("../fixtures/tree-construction-expected-failures.txt");

#[allow(dead_code)]
pub fn expected_failures() -> BTreeMap<&'static str, &'static str> {
    EXPECTED_FAILURES
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let (source, reason) = line
                .split_once(" — ")
                .unwrap_or_else(|| panic!("expected-failure line needs `<source> — <reason>`: {line:?}"));
            assert!(!reason.trim().is_empty(), "expected failure {source} has no reason");
            (source.trim(), reason.trim())
        })
        .collect()
}

/// Whether `source` (e.g. `scripted/webkit01.dat:1`) is a declared expected
/// failure. Callers assert such a case still fails, so the list cannot go stale.
#[allow(dead_code)]
pub fn is_expected_failure(source: &str) -> bool {
    expected_failures().contains_key(source)
}

#[derive(Debug)]
pub struct TreeConstructionCase {
    pub source: String,
    pub data: String,
    pub scripting: HtmlScriptingMode,
    /// Whether the case named its scripting mode (`#script-on` /
    /// `#script-off`). An unflagged case must produce the same tree with
    /// scripting on and off (BR02 P1.2).
    #[allow(dead_code)]
    pub scripting_flagged: bool,
    pub fragment_context: Option<String>,
    #[allow(dead_code)]
    pub expected_errors: Vec<String>,
    pub document: Vec<String>,
}

pub fn parse_tree_construction_cases(raw: &str) -> Vec<TreeConstructionCase> {
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

        let mut expected_errors = Vec::new();
        let mut scripting = HtmlScriptingMode::Enabled;
        let mut scripting_flagged = false;
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
                scripting_flagged = true;
            } else if line == "#script-on" {
                scripting = HtmlScriptingMode::Enabled;
                scripting_flagged = true;
            } else if !line.is_empty() {
                expected_errors.push(line.to_string());
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
            scripting_flagged,
            fragment_context,
            expected_errors,
            document,
        });
    }

    cases
}

#[allow(dead_code)]
pub fn actual_diagnostic_codes_for_tree_case(
    case: &TreeConstructionCase,
) -> Result<Vec<String>, String> {
    let options = HtmlParseOptions {
        scripting: case.scripting,
        ..HtmlParseOptions::default()
    };

    let (lexer_diagnostics, parser_diagnostics) =
        if let Some(fragment_context) = &case.fragment_context {
            let output = parse_html_fragment_for_context_with_diagnostics_and_options(
                &case.data,
                fragment_context,
                options,
            )
            .map_err(|error| format!("{error:?}"))?;
            (output.lexer_diagnostics, output.parser_diagnostics)
        } else {
            let output = parse_html_with_diagnostics_and_options(&case.data, options)
                .map_err(|error| format!("{error:?}"))?;
            (output.lexer_diagnostics, output.parser_diagnostics)
        };

    Ok(lexer_diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .chain(
            parser_diagnostics
                .into_iter()
                .map(|diagnostic| diagnostic.code),
        )
        .collect())
}

pub fn actual_dom_dump_for_tree_case(case: &TreeConstructionCase) -> Result<Vec<String>, String> {
    actual_dom_dump_with_scripting(case, case.scripting)
}

/// The tree for a case parsed with an explicit scripting mode, whatever the
/// case's own flag says.
pub fn actual_dom_dump_with_scripting(
    case: &TreeConstructionCase,
    scripting: HtmlScriptingMode,
) -> Result<Vec<String>, String> {
    let options = HtmlParseOptions {
        scripting,
        ..HtmlParseOptions::default()
    };

    if let Some(fragment_context) = &case.fragment_context {
        return parse_html_fragment_for_context_with_options(&case.data, fragment_context, options)
            .map(|nodes| dump_nodes(&nodes))
            .map_err(|error| format!("{error:?}"));
    }

    parse_html_with_options(&case.data, options)
        .map(|document| dump_document(&document))
        .map_err(|error| format!("{error:?}"))
}

pub fn dump_document(document: &Document) -> Vec<String> {
    dump_nodes(&document.children)
}

pub fn dump_nodes(nodes: &[Node]) -> Vec<String> {
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
        Node::ProcessingInstruction(pi) => {
            lines.push(format!("{}<?{} {}?>", prefix(depth), pi.target, pi.data));
        }
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
