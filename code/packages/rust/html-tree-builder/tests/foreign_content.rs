//! Foreign content (BR03 step 3): cases the security review found, pinned.

use coding_adventures_html_tree_builder::{html5lib, parse_document, TreeBuilderOptions};
use std::time::{Duration, Instant};

fn tree(source: &str) -> Vec<String> {
    html5lib::document_lines(
        &parse_document(source, TreeBuilderOptions::default())
            .expect("parses")
            .document,
    )
}

fn has_live_img(lines: &[String]) -> bool {
    lines
        .iter()
        .any(|line| line.trim_start_matches('|').trim() == "<img>")
}

/// Only a real `<![CDATA[` is a CDATA section. A comment or a bogus end tag
/// whose text starts `[CDATA[` must stay what every spec parser makes of it;
/// reading it as CDATA switched the tokenizer and turned inert text into a
/// live `<img onerror>` (a sanitizer bypass).
#[test]
fn look_alike_cdata_is_never_a_cdata_section() {
    for source in [
        "<svg><!--[CDATA[--></svg><style>]]><img src=x onerror=alert(1)></style>",
        "<svg></[CDATA[x></svg><style>]]><img src=x onerror=alert(1)></style>",
        "<svg><!--[CDATA[--><desc title=\"]]><img src=x onerror=alert(1)>\">",
    ] {
        let lines = tree(source);
        assert!(!has_live_img(&lines), "{source}\n{}", lines.join("\n"));
    }
    let lines = tree("<svg><!--[CDATA[x]]-->y");
    assert!(
        lines
            .iter()
            .any(|line| line.contains("<!-- [CDATA[x]] -->")),
        "{}",
        lines.join("\n")
    );
    assert!(
        lines.iter().any(|line| line.ends_with("\"y\"")),
        "{}",
        lines.join("\n")
    );
}

#[test]
fn real_cdata_sections_still_read_as_text() {
    let lines = tree("<svg><![CDATA[a>b]]>c</svg>");
    assert!(
        lines.iter().any(|line| line.ends_with("\"a>bc\"")),
        "{}",
        lines.join("\n")
    );
    let lines = tree("<svg><![CDATA[x]]]></svg>");
    assert!(
        lines.iter().any(|line| line.ends_with("\"x]\"")),
        "{}",
        lines.join("\n")
    );
    // In HTML content it is a bogus comment, as the specification says.
    let lines = tree("<p><![CDATA[x]]>");
    assert!(
        lines
            .iter()
            .any(|line| line.contains("<!-- [CDATA[x]] -->")),
        "{}",
        lines.join("\n")
    );
}

/// The foreign end-tag walk compares names without allocating: long element
/// names and many unmatched end tags must not multiply.
#[test]
fn long_foreign_names_do_not_make_end_tags_expensive() {
    let name = "g".repeat(200);
    let source = "<svg>".to_string() + &format!("<{name}>").repeat(500) + &"</x>".repeat(20_000);
    let started = Instant::now();
    tree(&source);
    assert!(
        started.elapsed() < Duration::from_secs(60),
        "{:?}",
        started.elapsed()
    );
}
