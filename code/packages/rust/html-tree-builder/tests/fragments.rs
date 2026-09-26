//! Fragment parsing (§13.4), beyond what the corpus checks.

use coding_adventures_html_tree_builder::arena::Namespace;
use coding_adventures_html_tree_builder::{
    html5lib, parse_fragment, FragmentContext, TreeBuilderOptions,
};
use std::time::{Duration, Instant};

fn fragment(source: &str, context: &FragmentContext) -> Vec<String> {
    html5lib::node_lines(
        &parse_fragment(source, context, TreeBuilderOptions::default())
            .expect("parses")
            .nodes,
    )
}

#[test]
fn a_cell_fragment_holds_only_its_content() {
    assert_eq!(
        fragment("<b>bold</b> text", &FragmentContext::html("td")),
        ["| <b>", "|   \"bold\"", "| \" text\""]
    );
}

#[test]
fn a_textarea_fragment_has_no_appropriate_end_tag() {
    // The spec: no end tag ends the RCDATA of a fragment, so `</textarea>`
    // is text.
    assert_eq!(
        fragment("a</textarea>b", &FragmentContext::html("textarea")),
        ["| \"a</textarea>b\""]
    );
}

#[test]
fn a_foreign_context_parses_foreign_content() {
    let context = FragmentContext {
        namespace: Namespace::Svg,
        name: "svg".into(),
        attributes: Vec::new(),
    };
    assert_eq!(fragment("<path/>", &context), ["| <svg path>"]);
}

#[test]
fn hostile_fragments_finish() {
    let started = Instant::now();
    fragment(&"<tr><td>".repeat(20_000), &FragmentContext::html("table"));
    fragment(&"</form><form>".repeat(20_000), &FragmentContext::html("template"));
    fragment(&"<div>".repeat(20_000), &FragmentContext::html("select"));
    assert!(started.elapsed() < Duration::from_secs(60), "{:?}", started.elapsed());
}
