//! Hostile shapes: the builder must finish, in bounded time, on inputs built
//! to stress the parts of tree construction that loop.

use coding_adventures_html_tree_builder::{html5lib, parse_document, TreeBuilderOptions};
use std::time::{Duration, Instant};

fn parses_quickly(source: &str) -> Vec<String> {
    let started = Instant::now();
    let output = parse_document(source, TreeBuilderOptions::default()).expect("parses");
    let lines = html5lib::document_lines(&output.document);
    assert!(
        started.elapsed() < Duration::from_secs(60),
        "took {:?}",
        started.elapsed()
    );
    // dom_core's owned tree drops recursively; don't let a deep test tree
    // measure (or overflow) that instead of the builder.
    std::mem::forget(output);
    lines
}

#[test]
fn deeply_nested_elements() {
    let lines = parses_quickly(&"<div>".repeat(10_000));
    assert_eq!(lines.len(), 3 + 10_000);
}

#[test]
fn many_distinct_formatting_elements_then_text_in_new_blocks() {
    let mut source = String::new();
    for index in 0..1_000 {
        source.push_str(&format!("<b id={index}>"));
    }
    source.push_str(&"<p>x".repeat(50));
    parses_quickly(&source);
}

#[test]
fn identical_formatting_elements_are_capped_by_noahs_ark() {
    let source = format!("{}<p>x", "<b>".repeat(2_000));
    let lines = parses_quickly(&source);
    // The <p> reopens only the three <b> the list kept.
    let reopened = lines
        .iter()
        .rev()
        .take_while(|line| !line.contains("<p>"))
        .count();
    assert!(reopened <= 4, "{reopened} lines after <p>");
}

#[test]
fn misnested_formatting_runs_the_adoption_agency_repeatedly() {
    let source = "<a><b><i><u><s><div>".repeat(250) + &"</a>x".repeat(250);
    parses_quickly(&source);
}

#[test]
fn stray_end_tags_and_frameset_recovery() {
    let source =
        "</p></div></body></html>".repeat(1_000) + "<frameset><frame>" + &"</frameset>".repeat(100);
    parses_quickly(&source);
}
