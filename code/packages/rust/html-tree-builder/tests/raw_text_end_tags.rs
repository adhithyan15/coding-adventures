//! Where a raw-text element ends decides what is markup and what is text, so a
//! disagreement with the WHATWG tokenizer here turns text a spec parser (and a
//! sanitizer built on one) sees as harmless into live elements. Each case below
//! hides an `<img onerror>` inside something a spec parser keeps as text or as
//! an end tag's attribute; none of them may produce an `<img>`.

use coding_adventures_html_tree_builder::{html5lib, parse_document, TreeBuilderOptions};

fn tree(source: &str) -> Vec<String> {
    let output = parse_document(source, TreeBuilderOptions::default()).expect("parses");
    html5lib::document_lines(&output.document)
}

fn assert_no_img(source: &str) {
    let lines = tree(source);
    assert!(
        !lines.iter().any(|line| line.trim_start_matches(['|', ' ']) == "<img>"),
        "{source:?} produced an <img>:\n{}",
        lines.join("\n")
    );
}

#[test]
fn a_non_letter_after_less_than_solidus_is_text() {
    // `</` + `<` is not an end tag, so the first `</style>` still closes.
    for element in ["style", "textarea", "title", "xmp", "script"] {
        let source = format!(
            "<{element}></</{element}><p title=\"</{element}><img src=x onerror=alert(1)>\">"
        );
        assert_no_img(&source);
        let lines = tree(&source);
        assert!(
            lines.iter().any(|line| line.trim_start_matches(['|', ' ']) == "\"</\""),
            "{element}: expected the text \"</\":\n{}",
            lines.join("\n")
        );
    }
}

#[test]
fn a_quoted_greater_than_inside_an_end_tag_does_not_close_it() {
    for element in ["style", "textarea", "title", "script"] {
        assert_no_img(&format!(
            "<{element}></{element} a=\"><img src=x onerror=alert(1)>\">"
        ));
        assert_no_img(&format!(
            "<{element}></{element} a='><img src=x onerror=alert(1)>'>"
        ));
    }
    // A quote only opens a value after `=`: `a"b` is an attribute name.
    assert_no_img("<script></script a\"b=\"><img src=x onerror=alert(1)>\">");
}

#[test]
fn a_data_state_end_tag_keeps_its_quoted_attributes_together() {
    assert_no_img("<p></p a=\"><img src=x onerror=alert(1)>\">");
    assert_no_img("<div></div a='><img src=x onerror=alert(1)>'>");
}

#[test]
fn text_before_a_script_end_tag_with_attributes_stays_before_it() {
    let lines = tree("<script>alpha</script x=1>tail");
    let script = lines.iter().position(|line| line.ends_with("<script>")).unwrap();
    assert_eq!(lines[script + 1].trim_start_matches(['|', ' ']), "\"alpha\"");
    assert!(lines.iter().any(|line| line.trim_start_matches(['|', ' ']) == "\"tail\""));
}
