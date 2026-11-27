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
use coding_adventures_html_tree_builder::arena::Namespace;
use coding_adventures_html_tree_builder::{
    html5lib, parse_document, parse_fragment, FragmentContext, ParseError, TreeBuilderOptions,
    TreeDiagnostic,
};
use std::collections::BTreeSet;

const CORPUS: &str =
    include_str!("../../html-parser/tests/fixtures/html5lib-tree-construction-smoke.dat");
const EXPECTED_FAILURES_PATH: &str = "tests/fixtures/expected-failures.txt";
const EXPECTED_FAILURES: &str = include_str!("fixtures/expected-failures.txt");

struct Case {
    source: String,
    data: String,
    scripting: Option<HtmlScriptingMode>,
    /// `#document-fragment` context: `td`, or `svg desc` / `math mi`.
    fragment: Option<String>,
    document: Vec<String>,
    new_errors: Vec<&'static str>,
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
        let mut fragment = None;
        let mut new_errors = Vec::new();
        while let Some(line) = lines.next() {
            match line {
                "#document" => break,
                "#new-errors" => {
                    while let Some(error) = lines.next_if(|line| !line.starts_with('#')) {
                        new_errors.push(error);
                    }
                }
                "#document-fragment" => {
                    fragment = lines.next().map(str::to_string);
                }
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
            new_errors,
        });
    }
    cases
}

/// A `#document-fragment` line as a context element.
fn fragment_context(line: &str) -> FragmentContext {
    let (namespace, name) = match line.split_once(' ') {
        Some(("svg", name)) => (Namespace::Svg, name),
        Some(("math", name)) => (Namespace::MathMl, name),
        _ => (Namespace::Html, line),
    };
    FragmentContext {
        namespace,
        name: name.to_string(),
        attributes: Vec::new(),
    }
}

fn parse_case(
    case: &Case,
    scripting: HtmlScriptingMode,
) -> Result<(Vec<String>, Vec<TreeDiagnostic>), ParseError> {
    let options = TreeBuilderOptions { scripting };
    match &case.fragment {
        Some(context) => {
            let output = parse_fragment(&case.data, &fragment_context(context), options)?;
            Ok((html5lib::node_lines(&output.nodes), output.tree_diagnostics))
        }
        None => {
            let output = parse_document(&case.data, options)?;
            Ok((
                html5lib::document_lines(&output.document),
                output.tree_diagnostics,
            ))
        }
    }
}

fn tree_for(case: &Case, scripting: HtmlScriptingMode) -> Option<Vec<String>> {
    parse_case(case, scripting)
        .ok()
        .map(|(lines, _)| normalized(lines))
}

fn scripting_modes(case: &Case) -> &[HtmlScriptingMode] {
    match &case.scripting {
        Some(mode) => std::slice::from_ref(mode),
        None => &[HtmlScriptingMode::Enabled, HtmlScriptingMode::Disabled],
    }
}

fn passes(case: &Case) -> bool {
    scripting_modes(case)
        .iter()
        .copied()
        .all(|scripting| tree_for(case, scripting).is_some_and(|tree| tree == case.document))
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
fn corpus_solidus_errors_match_the_upstream_code_and_emission_position() {
    // This is the named tree-construction error in #new-errors. The legacy
    // #errors use a mix of historical codes and position conventions.
    const CODE: &str = "non-void-html-element-start-tag-with-trailing-solidus";
    let mut checked_cases = 0;
    for case in cases() {
        let expected: Vec<_> = case
            .new_errors
            .iter()
            .filter_map(|error| {
                let (range, code) = error.split_once(") ")?;
                if code != CODE {
                    return None;
                }
                let (_, end) = range.split_once('-').expect("solidus errors span a tag");
                let (line, column) = end.split_once(':').expect("line and column");
                // Upstream ends the range after `>`. The builder reports
                // the point at which `>` emitted the token.
                Some((
                    code,
                    line.parse::<usize>().expect("line number"),
                    column.parse::<usize>().expect("column number") - 1,
                ))
            })
            .collect();
        if expected.is_empty() {
            continue;
        }
        checked_cases += 1;
        for &scripting in scripting_modes(&case) {
            let (_, diagnostics) = parse_case(&case, scripting)
                .unwrap_or_else(|error| panic!("{} with {scripting:?}: {error}", case.source));
            let actual: Vec<_> = diagnostics
                .iter()
                .filter(|error| error.code == CODE)
                .map(|error| (error.code, error.position.line, error.position.column))
                .collect();
            assert_eq!(actual, expected, "{} with {scripting:?}", case.source);
        }
    }
    assert!(
        checked_cases >= 6,
        "only checked {checked_cases} solidus cases"
    );
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
        let actual = tree_for(case, scripting)
            .map(|lines| lines.join("\n"))
            .unwrap_or_else(|| "(parse error)".to_string());
        println!(
            "== {} {:?}\n-- expected\n{}\n-- actual\n{}",
            case.source,
            case.data,
            case.document.join("\n"),
            actual
        );
    }
}
