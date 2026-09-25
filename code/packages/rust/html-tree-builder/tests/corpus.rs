//! The html5lib tree-construction corpus, run through the new builder (BR03 §4).
//!
//! The corpus is `html-parser`'s: the same 2,600-odd cases, so the two
//! builders are measured on one scale. Each case passes only if its tree
//! matches, and a case that names no scripting mode must match with
//! scripting both on and off.
//!
//! `fixtures/expected-failures.txt` lists every case this builder does not
//! pass yet. It is the progress bar: the test fails if an unlisted case fails
//! (a regression) *or* if a listed case passes (the list is stale), so each
//! step of BR03 §5 shrinks it in the same commit that earns the shrink.
//! After a deliberate change, rewrite it with
//!
//! ```text
//! HTML_TREE_BUILDER_BLESS=1 cargo test -p coding-adventures-html-tree-builder --test corpus
//! ```
//!
//! and review the diff: lines should only ever disappear.

use coding_adventures_html_lexer::HtmlScriptingMode;
use coding_adventures_html_tree_builder::{html5lib, parse_document, TreeBuilderOptions};
use std::collections::BTreeSet;

const CORPUS: &str =
    include_str!("../../html-parser/tests/fixtures/html5lib-tree-construction-smoke.dat");
const EXPECTED_FAILURES_PATH: &str = "tests/fixtures/expected-failures.txt";
const EXPECTED_FAILURES: &str = include_str!("fixtures/expected-failures.txt");

struct Case {
    source: String,
    data: String,
    scripting: Option<HtmlScriptingMode>,
    fragment: bool,
    document: Vec<String>,
}

fn cases() -> Vec<Case> {
    let mut cases = Vec::new();
    let mut lines = CORPUS.lines().peekable();
    let mut source = String::new();
    while let Some(line) = lines.next() {
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("#source ") {
            source = rest.to_string();
            continue;
        }
        assert_eq!(line, "#data", "corpus is out of step near {source}");
        let mut data = Vec::new();
        for line in lines.by_ref() {
            if line == "#errors" {
                break;
            }
            data.push(line);
        }
        let mut scripting = None;
        let mut fragment = false;
        for line in lines.by_ref() {
            match line {
                "#document" => break,
                "#document-fragment" => fragment = true,
                "#script-on" => scripting = Some(HtmlScriptingMode::Enabled),
                "#script-off" => scripting = Some(HtmlScriptingMode::Disabled),
                _ => {}
            }
        }
        let mut document = Vec::new();
        while let Some(line) = lines.peek() {
            if *line == "#data" || line.starts_with("#source ") {
                break;
            }
            document.push(lines.next().expect("peeked").to_string());
        }
        while document.last().is_some_and(|line| line.is_empty()) {
            document.pop();
        }
        cases.push(Case {
            source: std::mem::take(&mut source),
            data: data.join("\n"),
            scripting,
            fragment,
            document,
        });
    }
    cases
}

fn passes(case: &Case) -> bool {
    // Fragment parsing is BR03 §5 step 4.
    if case.fragment {
        return false;
    }
    let modes = match case.scripting {
        Some(mode) => vec![mode],
        None => vec![HtmlScriptingMode::Enabled, HtmlScriptingMode::Disabled],
    };
    modes.into_iter().all(|scripting| {
        parse_document(&case.data, TreeBuilderOptions { scripting })
            .map(|output| normalized(html5lib::document_lines(&output.document)) == case.document)
            .unwrap_or(false)
    })
}

/// The smoke corpus was imported with its line endings normalised, so a U+000D
/// that a character reference produced (`&#x0D;`) is stored as a line break.
/// Compare the builder's output the same way.
fn normalized(lines: Vec<String>) -> Vec<String> {
    lines
        .join("\n")
        .replace('\r', "\n")
        .split('\n')
        .map(str::to_string)
        .collect()
}

fn listed() -> BTreeSet<String> {
    EXPECTED_FAILURES
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
        .collect()
}

#[test]
fn corpus_failures_match_the_expected_failure_list() {
    let cases = cases();
    assert!(cases.len() > 2_600, "corpus shrank to {}", cases.len());
    let failing: BTreeSet<String> = cases
        .iter()
        .filter(|case| !passes(case))
        .map(|case| case.source.clone())
        .collect();

    if std::env::var_os("HTML_TREE_BUILDER_BLESS").is_some() {
        let header = EXPECTED_FAILURES
            .lines()
            .take_while(|line| line.starts_with('#') || line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        let body = failing.iter().cloned().collect::<Vec<_>>().join("\n");
        std::fs::write(EXPECTED_FAILURES_PATH, format!("{header}\n{body}\n"))
            .expect("write the expected-failure list");
        return;
    }

    let listed = listed();
    let regressions: Vec<_> = failing.difference(&listed).collect();
    let now_passing: Vec<_> = listed.difference(&failing).collect();
    println!(
        "html-tree-builder: {} of {} corpus cases pass",
        cases.len() - failing.len(),
        cases.len()
    );
    assert!(
        regressions.is_empty(),
        "{} unlisted cases fail: {:?}",
        regressions.len(),
        regressions.iter().take(20).collect::<Vec<_>>()
    );
    assert!(
        now_passing.is_empty(),
        "{} listed cases now pass; remove them from {EXPECTED_FAILURES_PATH}: {:?}",
        now_passing.len(),
        now_passing.iter().take(20).collect::<Vec<_>>()
    );
}

/// `HTML_TREE_BUILDER_SHOW=<source id>[,<source id>…]` prints the expected
/// and actual trees of those cases (a debugging aid; it asserts nothing).
#[test]
fn show_selected_cases() {
    let Some(wanted) = std::env::var_os("HTML_TREE_BUILDER_SHOW") else {
        return;
    };
    let wanted = wanted.to_string_lossy().to_string();
    let wanted: BTreeSet<&str> = wanted.split(',').collect();
    for case in cases()
        .iter()
        .filter(|case| wanted.contains(case.source.as_str()))
    {
        let scripting = case.scripting.unwrap_or(HtmlScriptingMode::Enabled);
        let actual = parse_document(&case.data, TreeBuilderOptions { scripting })
            .map(|output| html5lib::document_lines(&output.document).join("\n"))
            .unwrap_or_else(|error| error.to_string());
        println!(
            "== {} {:?}\n-- expected\n{}\n-- actual\n{}",
            case.source,
            case.data,
            case.document.join("\n"),
            actual
        );
    }
}
