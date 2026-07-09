//! Tests for `markdown-docx` — the headline end-to-end proof of the whole MD02
//! pipeline: a Markdown string in, a real `.docx` out, reopened with the
//! independent `wordprocessingml` reader to confirm the visible text and block
//! structure survived Markdown → Document AST → WordprocessingML → ZIP.
//!
//! (Note on whitespace: the reader keeps a run's *trailing* space but drops a
//! *leading* one — a documented shared-xml-lexer limitation — so these tests
//! assert on the presence of the content tokens and the block structure rather
//! than exact inter-run spacing. The emitted `.docx` itself is correct; Word
//! renders the spaces.)

use super::*;
use coding_adventures_wordprocessingml::{open_docx, Block as RBlock};

/// A `.docx` begins with the ZIP/OPC magic, and the empty string is valid.
#[test]
fn output_is_a_valid_docx() {
    let bytes = markdown_to_docx("# Hi\n\nSome text.\n");
    assert_eq!(&bytes[..2], b"PK", "a .docx is a ZIP/OPC package");
    open_docx(&bytes).expect("reader opens our .docx");

    let empty = markdown_to_docx("");
    assert_eq!(&empty[..2], b"PK");
    assert_eq!(
        open_docx(&empty).unwrap().text(),
        "",
        "empty markdown → empty doc"
    );
}

/// The headline round-trip: a rich Markdown document → `.docx` → reopened text +
/// structure. Heading, a formatted paragraph, and a bullet list all survive.
#[test]
fn rich_markdown_round_trips() {
    let md = "\
# Report

An **important** and *emphasised* point, with some `code`.

- first item
- second item
- third item
";
    let read = open_docx(&markdown_to_docx(md)).expect("reopen");
    let text = read.text();

    // Content tokens survive (heading, the formatted words, the list items).
    for needle in [
        "Report",
        "important",
        "emphasised",
        "code",
        "first item",
        "third item",
    ] {
        assert!(text.contains(needle), "missing {needle:?} in {text:?}");
    }
    // Structure: 1 heading + 1 body paragraph + 3 list-item paragraphs = 5 paragraphs.
    assert_eq!(read.paragraphs().count(), 5, "paragraph count: {text:?}");
    // The list markers are present.
    assert_eq!(text.matches("• ").count(), 3, "three bullets: {text:?}");
}

/// GFM pipe tables round-trip as a real Word table with the cell text intact.
#[test]
fn gfm_table_round_trips() {
    let md = "\
| Name | Qty |
| ---- | --- |
| Apple | 3 |
| Pear  | 7 |
";
    let read = open_docx(&gfm_to_docx(md)).expect("reopen");
    let tables: Vec<_> = read.tables().collect();
    assert_eq!(tables.len(), 1, "one table");
    // Header + two data rows.
    assert_eq!(tables[0].rows.len(), 3, "header + 2 data rows");
    assert_eq!(tables[0].rows[0][0].text, "Name");
    assert_eq!(tables[0].rows[0][1].text, "Qty");
    assert_eq!(tables[0].rows[2][0].text, "Pear");
    assert_eq!(tables[0].rows[2][1].text, "7");
}

/// GFM task lists render with checkbox markers.
#[test]
fn gfm_task_list_round_trips() {
    let read = open_docx(&gfm_to_docx("- [x] done\n- [ ] todo\n")).expect("reopen");
    let text = read.text();
    assert!(text.contains("☑ done"), "checked: {text:?}");
    assert!(text.contains("☐ todo"), "unchecked: {text:?}");
}

/// A fenced code block becomes monospace `Code` paragraphs (one per line).
#[test]
fn code_block_round_trips() {
    let md = "```rust\nfn main() {}\nlet x = 1;\n```\n";
    let read = open_docx(&markdown_to_docx(md)).expect("reopen");
    let text = read.text();
    assert!(text.contains("fn main() {}"), "code line 1: {text:?}");
    assert!(text.contains("let x = 1;"), "code line 2: {text:?}");
}

/// Raw HTML embedded in Markdown is NOT injected into the `.docx` — the pipeline
/// drops it at the Document-AST → docx stage. (Markdown's `<script>…` parses to
/// an HTML block, which `document-ast-to-docx` drops.)
#[test]
fn raw_html_is_not_injected() {
    let md =
        "Before.\n\n<script>alert('xss')</script>\n\n<div onclick=\"evil()\">x</div>\n\nAfter.\n";
    let read = open_docx(&markdown_to_docx(md)).expect("reopen");
    let text = read.text();
    assert!(
        text.contains("Before.") && text.contains("After."),
        "prose kept: {text:?}"
    );
    assert!(!text.contains("alert"), "raw <script> dropped: {text:?}");
    assert!(
        !text.contains("onclick"),
        "raw <div> attributes dropped: {text:?}"
    );
    assert!(!text.contains("evil"), "raw handler dropped: {text:?}");
}

/// A blockquote renders (its text survives); the reader sees it as a paragraph.
#[test]
fn blockquote_round_trips() {
    let read = open_docx(&markdown_to_docx("> quoted wisdom\n")).expect("reopen");
    assert!(
        read.text().contains("quoted wisdom"),
        "quote text: {:?}",
        read.text()
    );
    // The quote is a paragraph block (not a table).
    assert!(matches!(read.blocks.first(), Some(RBlock::Paragraph(_))));
}

/// Adversarially deep Markdown does not crash the pipeline — the depth guard in
/// `document-ast-to-docx` bounds the recursion. A one-line input of 20 000 `>`
/// nests blockquotes 20 000 deep; it must still produce a valid `.docx`.
#[test]
fn deeply_nested_markdown_does_not_crash() {
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024) // for the PARSER's own construction/Drop, not the converter
        .spawn(|| {
            let md = format!("{}x\n", ">".repeat(20_000));
            let bytes = markdown_to_docx(&md);
            assert_eq!(&bytes[..2], b"PK", "deep input still yields a valid .docx");
        })
        .unwrap()
        .join()
        .expect("deeply nested Markdown must not overflow the stack");
}
