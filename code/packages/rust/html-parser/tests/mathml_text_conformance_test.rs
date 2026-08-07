#[allow(dead_code)]
mod common;

use coding_adventures_html_lexer::HtmlScriptingMode;
use common::{actual_dom_dump_for_tree_case, TreeConstructionCase};

#[test]
fn wpt_mathml_and_svg_null_text_cases_match_dom_dump() {
    let cases = [
        tree_case(
            "plain-text-unsafe.dat:34",
            "<math>\0filler\0text",
            None,
            &[
                "| <html>",
                "|   <head>",
                "|   <body>",
                "|     <math math>",
                "|       \"�filler�text\"",
            ],
        ),
        tree_case(
            "plain-text-unsafe.dat:35",
            "<math><![CDATA[\0filler\0text\0]]>",
            None,
            &[
                "| <html>",
                "|   <head>",
                "|   <body>",
                "|     <math math>",
                "|       \"�filler�text�\"",
            ],
        ),
        tree_case(
            "plain-text-unsafe.dat:36",
            "<math><annotation-xml>\0x",
            None,
            &[
                "| <html>",
                "|   <head>",
                "|   <body>",
                "|     <math math>",
                "|       <math annotation-xml>",
                "|         \"�x\"",
            ],
        ),
        tree_case(
            "plain-text-unsafe.dat:37",
            "<math><annotation-xml encoding=\"text/html\">\0x",
            None,
            &[
                "| <html>",
                "|   <head>",
                "|   <body>",
                "|     <math math>",
                "|       <math annotation-xml>",
                "|         encoding=\"text/html\"",
                "|         \"x\"",
            ],
        ),
        tree_case(
            "plain-text-unsafe.dat:38",
            "\0filler\0text",
            Some("svg path"),
            &["| \"�filler�text\""],
        ),
        tree_case(
            "plain-text-unsafe.dat:39",
            "\0filler\0text",
            Some("svg title"),
            &["| \"fillertext\""],
        ),
        tree_case(
            "plain-text-unsafe.dat:40",
            "\0filler\0text",
            Some("math ms"),
            &["| \"fillertext\""],
        ),
        tree_case(
            "plain-text-unsafe.dat:41",
            "\0x",
            Some("math annotation-xml"),
            &["| \"�x\""],
        ),
    ];

    for case in cases {
        let actual = actual_dom_dump_for_tree_case(&case)
            .expect("parser should accept any WPT HTML or HTML fragment input");
        assert_eq!(
            actual, case.document,
            "WPT tree-construction case {} failed for input {:?}",
            case.source, case.data
        );
    }
}

fn tree_case(
    source: &str,
    data: &str,
    fragment_context: Option<&str>,
    document: &[&str],
) -> TreeConstructionCase {
    TreeConstructionCase {
        source: source.to_string(),
        data: data.to_string(),
        scripting: HtmlScriptingMode::Enabled,
        fragment_context: fragment_context.map(str::to_string),
        expected_errors: Vec::new(),
        document: document.iter().map(|line| (*line).to_string()).collect(),
    }
}
