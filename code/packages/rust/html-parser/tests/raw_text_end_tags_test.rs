//! Raw-text end tags must end where the WHATWG tokenizer ends them, or text a
//! spec parser (and a sanitizer built on one) treats as inert becomes live
//! markup here. Each payload hides an `<img onerror>` that a spec parser keeps
//! as text or as an end tag's attribute; none of them may produce an `img`.

use coding_adventures_html_parser::parse_html;

fn assert_no_img(source: &str) {
    let document = parse_html(source).expect("parses");
    let tree = format!("{document:?}");
    assert!(!tree.contains("\"img\""), "{source:?} produced an img: {tree}");
}

#[test]
fn the_check_sees_a_real_img() {
    let tree = format!("{:?}", parse_html("<p><img src=x></p>").expect("parses"));
    assert!(tree.contains("\"img\""), "control: {tree}");
}

#[test]
fn raw_text_payloads_produce_no_img() {
    for element in ["style", "textarea", "title", "xmp", "script"] {
        assert_no_img(&format!(
            "<{element}></</{element}><p title=\"</{element}><img src=x onerror=alert(1)>\">"
        ));
        assert_no_img(&format!("<{element}></{element} a=\"><img src=x onerror=alert(1)>\">"));
        assert_no_img(&format!("<{element}></{element} a='><img src=x onerror=alert(1)>'>"));
    }
    assert_no_img("<script></script a\"b=\"><img src=x onerror=alert(1)>\">");
    assert_no_img("<p></p a=\"><img src=x onerror=alert(1)>\">");
}
