//! GFM 0.31.2 Specification Tests
//!
//! Runs all 652 examples from the GFM 0.31.2 specification against the
//! Rust `gfm` pipeline (parser + HTML renderer). Each example provides:
//!
//!   markdown  — the input Markdown string
//!   html      — the expected HTML output
//!   example   — the example number (1-based, used for reporting)
//!   section   — the spec section name (e.g. "Tabs", "ATX headings")
//!
//! Failing tests report the example number, section name, expected HTML,
//! and actual HTML so failures are easy to diagnose.
//!
//! # Coverage target
//!
//! We aim for ≥95% of the 652 spec examples to pass. Full 100% compliance
//! means our output is byte-for-byte identical to the spec's reference output.
//! Example numbers map to https://spec.commonmark.org/0.31.2/#example-N.

use gfm::markdown_to_html;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct SpecExample {
    markdown: String,
    html: String,
    example: usize,
    section: String,
}

fn load_spec() -> Vec<SpecExample> {
    let spec_json = include_str!("fixtures/spec.json");
    serde_json::from_str(spec_json).expect("failed to parse spec.json")
}

/// Run all 652 GFM spec examples and report pass/fail counts.
///
/// This test does NOT fail on individual example failures — it collects all
/// failures and prints a summary so you can see the full pass rate in one run.
/// Individual `test_example_N` tests below pin specific examples.
#[test]
fn gfm_spec_suite() {
    let examples = load_spec();
    let total = examples.len();
    let mut failures: Vec<(usize, String, String, String)> = Vec::new();

    for ex in &examples {
        let actual = markdown_to_html(&ex.markdown);
        if actual != ex.html {
            failures.push((ex.example, ex.section.clone(), ex.html.clone(), actual));
        }
    }

    let passed = total - failures.len();
    let pct = (passed as f64 / total as f64) * 100.0;

    println!("\n=== GFM 0.31.2 Spec Results ===");
    println!("Passed: {}/{} ({:.1}%)", passed, total, pct);

    if !failures.is_empty() {
        println!("\nFailed examples:");
        for (example, section, expected, actual) in &failures {
            println!(
                "  Example {} ({}):\n    expected: {:?}\n    actual:   {:?}",
                example, section, expected, actual
            );
        }
    }

    // Require ≥95% pass rate (619/652)
    let required_pass_rate = 0.95_f64;
    let actual_pass_rate = passed as f64 / total as f64;
    assert!(
        actual_pass_rate >= required_pass_rate,
        "Pass rate {:.1}% is below required {:.0}% ({} passed, {} failed)",
        pct,
        required_pass_rate * 100.0,
        passed,
        failures.len()
    );
}

// ─── Section-level tests ───────────────────────────────────────────────────────
//
// Each section gets its own test so you can run a specific section with:
//   cargo test -p gfm --test spec_test -- tabs
//
// These tests fail immediately on the first failure in the section so you get
// a clean diff view.

macro_rules! spec_section {
    ($test_name:ident, $section_name:expr) => {
        #[test]
        fn $test_name() {
            let examples = load_spec();
            let section_examples: Vec<_> = examples
                .iter()
                .filter(|e| e.section == $section_name)
                .collect();

            for ex in &section_examples {
                let actual = markdown_to_html(&ex.markdown);
                assert_eq!(
                    actual,
                    ex.html,
                    "Example {} ({})\n  markdown: {:?}",
                    ex.example,
                    ex.section,
                    ex.markdown,
                );
            }
        }
    };
}

spec_section!(spec_tabs, "Tabs");
spec_section!(spec_backslash_escapes, "Backslash escapes");
spec_section!(spec_entity_references, "Entity and numeric character references");
spec_section!(spec_precedence, "Precedence");
spec_section!(spec_thematic_breaks, "Thematic breaks");
spec_section!(spec_atx_headings, "ATX headings");
spec_section!(spec_setext_headings, "Setext headings");
spec_section!(spec_indented_code_blocks, "Indented code blocks");
spec_section!(spec_fenced_code_blocks, "Fenced code blocks");
spec_section!(spec_html_blocks, "HTML blocks");
spec_section!(spec_link_reference_definitions, "Link reference definitions");
spec_section!(spec_paragraphs, "Paragraphs");
spec_section!(spec_blank_lines, "Blank lines");
spec_section!(spec_block_quotes, "Block quotes");
spec_section!(spec_list_items, "List items");
spec_section!(spec_lists, "Lists");
spec_section!(spec_inlines, "Inlines");
spec_section!(spec_code_spans, "Code spans");
spec_section!(spec_emphasis_and_strong_emphasis, "Emphasis and strong emphasis");
spec_section!(spec_links, "Links");
spec_section!(spec_images, "Images");
spec_section!(spec_autolinks, "Autolinks");
spec_section!(spec_raw_html, "Raw HTML");
spec_section!(spec_hard_line_breaks, "Hard line breaks");
spec_section!(spec_soft_line_breaks, "Soft line breaks");
spec_section!(spec_textual_content, "Textual content");
