//! Hostile shapes: the builder must finish, in bounded time and memory, on
//! inputs built to stress the parts of tree construction that loop or nest.
//! Several come from the step-1 security review.

use coding_adventures_html_tree_builder::arena::MAX_TREE_DEPTH;
use coding_adventures_html_tree_builder::{html5lib, parse_document, TreeBuilderOptions};
use dom_core::Node;
use std::time::{Duration, Instant};

fn parse(source: &str) -> coding_adventures_html_tree_builder::ParseOutput {
    let started = Instant::now();
    let output = parse_document(source, TreeBuilderOptions::default()).expect("parses");
    assert!(
        started.elapsed() < Duration::from_secs(60),
        "took {:?}",
        started.elapsed()
    );
    output
}

fn depth(nodes: &[Node]) -> usize {
    let mut deepest = 0;
    let mut work: Vec<(&Node, usize)> = nodes.iter().map(|node| (node, 1)).collect();
    while let Some((node, level)) = work.pop() {
        deepest = deepest.max(level);
        if let Node::Element(element) = node {
            work.extend(element.children.iter().map(|child| (child, level + 1)));
        }
    }
    deepest
}

fn count_elements(nodes: &[Node]) -> usize {
    let mut count = 0;
    let mut work: Vec<&Node> = nodes.iter().collect();
    while let Some(node) = work.pop() {
        if let Node::Element(element) = node {
            count += 1;
            work.extend(element.children.iter());
        }
    }
    count
}

#[test]
fn deep_nesting_is_capped_and_the_result_drops_safely() {
    // The result is dropped normally: dom_core's Drop recurses, and the
    // depth cap is what keeps that safe.
    let output = parse(&"<div>".repeat(20_000));
    assert!(depth(&output.document.children) <= MAX_TREE_DEPTH);
    assert_eq!(count_elements(&output.document.children), 3 + 20_000);
    assert!(output
        .tree_diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "tree-builder-open-elements-limit"));
}

#[test]
fn deep_template_nesting_is_capped_too() {
    let output = parse(&"<template>".repeat(3_000));
    assert!(depth(&output.document.children) <= MAX_TREE_DEPTH);
}

#[test]
fn distinct_formatting_elements_cannot_amplify() {
    // Security review finding 1: distinct attributes defeat the Noah's Ark
    // clause, and each later text run reconstructs the whole list.
    let n = 3_000;
    let mut source = String::from("<p>");
    for index in 0..n {
        source.push_str(&format!("<b id={index}>"));
    }
    source.push_str("</p>");
    source.push_str(&"<p>x</p>".repeat(n));
    let output = parse(&source);
    let elements = count_elements(&output.document.children);
    assert!(
        elements < 400_000,
        "{elements} elements from {} bytes",
        source.len()
    );
}

#[test]
fn one_character_after_many_formatting_elements_is_cheap() {
    // Security review finding 2.
    let mut source = String::from("<p>");
    for index in 0..20_000 {
        source.push_str(&format!("<b id={index}>"));
    }
    source.push_str("</p>x");
    parse(&source);
}

#[test]
fn identical_formatting_elements_are_capped_by_noahs_ark() {
    let source = format!("{}<p>x", "<b>".repeat(2_000));
    let lines = html5lib::document_lines(&parse(&source).document);
    let reopened = lines
        .iter()
        .rev()
        .take_while(|line| !line.contains("<p>"))
        .count();
    assert!(reopened <= 4, "{reopened} lines after <p>");
}

#[test]
fn misnested_formatting_runs_the_adoption_agency_repeatedly() {
    parse(&("<a><b><i><u><s><div>".repeat(250) + &"</a>x".repeat(250)));
}

#[test]
fn stray_end_tags_and_frameset_recovery() {
    parse(
        &("</p></div></body></html>".repeat(1_000)
            + "<frameset><frame>"
            + &"</frameset>".repeat(100)),
    );
}

#[test]
fn many_templates_closed_at_end_of_file_all_close() {
    let output = parse(&"<template>".repeat(100));
    assert!(!output
        .tree_diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "tree-builder-reprocess-limit"));
}

// Round 2 of the security review.

#[test]
fn the_adoption_agency_cannot_renest_past_the_depth_limit() {
    let source = "<div>".repeat(600) + &"<b><div></b>".repeat(5_000);
    let output = parse(&source);
    assert!(depth(&output.document.children) <= MAX_TREE_DEPTH);
}

#[test]
fn an_open_p_out_of_scope_does_not_make_every_div_walk_the_stack() {
    parse(&("<p><button>".to_string() + &"<div>".repeat(40_000)));
    parse(&("<p><object>".to_string() + &"<div>".repeat(40_000)));
}

#[test]
fn unmatched_end_tags_under_deep_inline_nesting() {
    parse(&("<span>".repeat(20_000) + &"</x>".repeat(20_000)));
}

#[test]
fn many_short_formatting_elements_under_deep_nesting() {
    parse(&("<div>".repeat(20_000) + &"<b></b>".repeat(20_000)));
}

#[test]
fn marker_segmented_formatting_lists() {
    let mut source = String::new();
    for _ in 0..200 {
        source.push_str("<object>");
        for index in 0..64 {
            source.push_str(&format!("<b id={index}>"));
        }
    }
    source.push_str(&"<b></b>".repeat(20_000));
    parse(&source);
}
