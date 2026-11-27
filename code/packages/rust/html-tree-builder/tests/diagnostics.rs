use coding_adventures_html_lexer::SourcePosition;
use coding_adventures_html_tree_builder::{parse_document, TreeBuilderOptions, TreeDiagnostic};

#[test]
fn mixed_table_text_reports_one_error_at_the_tag_that_flushes_it() {
    // The lexer normalizes CRLF and resolves &amp; before the builder sees
    // the text. The offsets still refer to the input, at the closing `>`.
    let output = parse_document(
        "<!doctype html>\r\n<table>é &amp; x</table>",
        TreeBuilderOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.tree_diagnostics,
        [TreeDiagnostic {
            code: "non-whitespace-in-table-text",
            position: SourcePosition {
                byte_offset: 41,
                char_offset: 40,
                line: 2,
                column: 24,
            },
        }],
    );
}

#[test]
fn table_text_can_flush_at_eof() {
    let output = parse_document(
        "<!doctype html>\n<table> a b",
        TreeBuilderOptions::default(),
    )
    .unwrap();
    let eof = SourcePosition {
        byte_offset: 27,
        char_offset: 27,
        line: 2,
        column: 12,
    };
    assert_eq!(
        output.tree_diagnostics,
        [
            TreeDiagnostic {
                code: "non-whitespace-in-table-text",
                position: eof,
            },
            TreeDiagnostic {
                code: "expected-closing-tag-but-got-eof",
                position: eof,
            },
        ],
    );
}

#[test]
fn whitespace_table_text_does_not_report_an_error() {
    let output = parse_document(
        "<!doctype html>\n<table> \t\n</table>",
        TreeBuilderOptions::default(),
    )
    .unwrap();
    assert!(output.tree_diagnostics.is_empty());
}

#[test]
fn tags_using_the_in_table_fallback_still_report_at_each_tag() {
    let output = parse_document(
        "<!doctype html>\n<table><div></div></table>",
        TreeBuilderOptions::default(),
    )
    .unwrap();
    let actual: Vec<_> = output
        .tree_diagnostics
        .iter()
        .map(|error| (error.code, error.position.line, error.position.column))
        .collect();
    assert_eq!(
        actual,
        [
            ("unexpected-token-in-table", 2, 12),
            ("unexpected-token-in-table", 2, 18),
        ],
    );
}
