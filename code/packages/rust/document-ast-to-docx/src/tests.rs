//! Tests for `document-ast-to-docx`.
//!
//! Two kinds of proof:
//! 1. **XML-shape** — build a `DocumentNode`, render it, inflate the emitted
//!    package with the independent `opc` reader, and assert `word/document.xml`
//!    carries the right `<w:pStyle>` / `<w:rPr>` shapes.
//! 2. **Round-trip** — reopen the `.docx` bytes with the read-side
//!    `wordprocessingml` reader and assert the visible text + block structure.

use super::*;
use coding_adventures_opc::Package;
use coding_adventures_wordprocessingml::open_docx;
use document_ast::*;

/// The emitted `word/document.xml` as a string, for XML-shape assertions.
fn body_xml(doc: &DocumentNode) -> String {
    let bytes = to_docx_bytes(doc);
    let pkg = Package::open(&bytes).expect("emitted .docx must be a valid OPC package");
    String::from_utf8(pkg.read_part("/word/document.xml").unwrap().to_vec()).unwrap()
}

// ── small constructors to keep the tests readable ──────────────────────────
fn text(s: &str) -> InlineNode {
    InlineNode::Text(TextNode {
        value: s.to_string(),
    })
}
fn para(children: Vec<InlineNode>) -> BlockNode {
    BlockNode::Paragraph(ParagraphNode { children })
}
fn doc(children: Vec<BlockNode>) -> DocumentNode {
    DocumentNode { children }
}

/// An empty AST renders to a valid, empty `.docx`.
#[test]
fn empty_document_is_valid() {
    let bytes = to_docx_bytes(&doc(vec![]));
    assert_eq!(&bytes[..2], b"PK", "a valid OPC/ZIP package");
    let read = open_docx(&bytes).expect("empty doc opens");
    assert_eq!(read.text(), "");
}

/// A heading renders as a `Heading N` styled paragraph.
#[test]
fn heading_maps_to_heading_style() {
    let d = doc(vec![BlockNode::Heading(HeadingNode {
        level: 2,
        children: vec![text("Chapter Two")],
    })]);
    let xml = body_xml(&d);
    assert!(xml.contains("<w:pStyle w:val=\"Heading2\"/>"), "{xml}");
    assert!(xml.contains("Chapter Two"), "{xml}");
}

/// Strong → bold, Emphasis → italic, CodeSpan → monospace, and nesting combines.
#[test]
fn inline_formatting_maps_to_run_properties() {
    let d = doc(vec![para(vec![
        InlineNode::Strong(StrongNode {
            children: vec![text("b")],
        }),
        InlineNode::Emphasis(EmphasisNode {
            children: vec![text("i")],
        }),
        InlineNode::CodeSpan(CodeSpanNode {
            value: "c".to_string(),
        }),
        // Emphasis inside Strong ⇒ a bold+italic run.
        InlineNode::Strong(StrongNode {
            children: vec![InlineNode::Emphasis(EmphasisNode {
                children: vec![text("bi")],
            })],
        }),
    ])]);
    let xml = body_xml(&d);
    assert!(xml.contains("<w:rPr><w:b/></w:rPr>"), "bold: {xml}");
    assert!(xml.contains("<w:rPr><w:i/></w:rPr>"), "italic: {xml}");
    assert!(
        xml.contains("w:ascii=\"Consolas\""),
        "mono code span: {xml}"
    );
    assert!(
        xml.contains("<w:rPr><w:b/><w:i/></w:rPr>"),
        "combined bold+italic: {xml}"
    );
}

/// A fenced code block becomes one monospace `Code` paragraph per source line,
/// with no spurious trailing blank paragraph.
#[test]
fn code_block_becomes_code_paragraphs_per_line() {
    let d = doc(vec![BlockNode::CodeBlock(CodeBlockNode {
        language: Some("rust".to_string()),
        value: "let x = 1;\nlet y = 2;\n".to_string(),
    })]);
    let xml = body_xml(&d);
    assert_eq!(
        xml.matches("<w:pStyle w:val=\"Code\"/>").count(),
        2,
        "two code lines: {xml}"
    );
    assert!(
        xml.contains("let x = 1;") && xml.contains("let y = 2;"),
        "{xml}"
    );
}

/// Ordered and unordered lists become prefixed `ListParagraph`s; ordered lists
/// honour `start` and increment.
#[test]
fn lists_become_prefixed_paragraphs() {
    let item = |s: &str| {
        ListChildNode::ListItem(ListItemNode {
            children: vec![para(vec![text(s)])],
        })
    };

    let unordered = BlockNode::List(ListNode {
        ordered: false,
        start: None,
        tight: true,
        children: vec![item("apple"), item("pear")],
    });
    let ordered = BlockNode::List(ListNode {
        ordered: true,
        start: Some(3),
        tight: true,
        children: vec![item("third"), item("fourth")],
    });
    let d = doc(vec![unordered, ordered]);
    // The prefix and content are separate runs, so assert style on the XML and
    // the rejoined marker+text via the reader (which concatenates runs).
    assert!(
        body_xml(&d).contains("<w:pStyle w:val=\"ListParagraph\"/>"),
        "list style"
    );
    let text = open_docx(&to_docx_bytes(&d)).unwrap().text();
    assert!(
        text.contains("• apple") && text.contains("• pear"),
        "bullets: {text:?}"
    );
    assert!(
        text.contains("3. third") && text.contains("4. fourth"),
        "numbering from start=3: {text:?}"
    );
}

/// GFM task items render with a checked/unchecked box marker.
#[test]
fn task_items_render_checkboxes() {
    let d = doc(vec![BlockNode::List(ListNode {
        ordered: false,
        start: None,
        tight: true,
        children: vec![
            ListChildNode::TaskItem(TaskItemNode {
                checked: true,
                children: vec![para(vec![text("done")])],
            }),
            ListChildNode::TaskItem(TaskItemNode {
                checked: false,
                children: vec![para(vec![text("todo")])],
            }),
        ],
    })]);
    let text = open_docx(&to_docx_bytes(&d)).unwrap().text();
    assert!(text.contains("☑ done"), "checked: {text:?}");
    assert!(text.contains("☐ todo"), "unchecked: {text:?}");
}

/// A blockquote's paragraphs take the `Quote` style.
#[test]
fn blockquote_uses_quote_style() {
    let d = doc(vec![BlockNode::Blockquote(BlockquoteNode {
        children: vec![para(vec![text("quoted")])],
    })]);
    let xml = body_xml(&d);
    assert!(xml.contains("<w:pStyle w:val=\"Quote\"/>"), "{xml}");
    assert!(xml.contains("quoted"), "{xml}");
}

/// A GFM table renders as a `<w:tbl>` with the cells' flattened text.
#[test]
fn table_renders_with_cell_text() {
    let cell = |s: &str| TableCellNode {
        children: vec![text(s)],
    };
    let d = doc(vec![BlockNode::Table(TableNode {
        align: vec![TableAlignment::None, TableAlignment::None],
        children: vec![
            TableRowNode {
                is_header: true,
                children: vec![cell("H1"), cell("H2")],
            },
            TableRowNode {
                is_header: false,
                children: vec![cell("a"), cell("b")],
            },
        ],
    })]);
    let read = open_docx(&to_docx_bytes(&d)).expect("open");
    let tables: Vec<_> = read.tables().collect();
    assert_eq!(tables.len(), 1);
    assert_eq!(tables[0].rows.len(), 2);
    assert_eq!(tables[0].rows[0][0].text, "H1");
    assert_eq!(tables[0].rows[1][1].text, "b");
}

/// Links render as `text (url)`, images as `[alt] (url)`.
#[test]
fn links_and_images_render_text_and_url() {
    let d = doc(vec![para(vec![
        InlineNode::Link(LinkNode {
            destination: "https://example.com".to_string(),
            title: None,
            children: vec![text("site")],
        }),
        InlineNode::Image(ImageNode {
            destination: "cat.png".to_string(),
            title: None,
            alt: "a cat".to_string(),
        }),
    ])]);
    let xml = body_xml(&d);
    assert!(
        xml.contains("site") && xml.contains("(https://example.com)"),
        "link: {xml}"
    );
    assert!(
        xml.contains("[a cat]") && xml.contains("(cat.png)"),
        "image: {xml}"
    );
}

/// Raw HTML blocks and inlines are DROPPED, never injected into the .docx.
#[test]
fn raw_html_is_dropped() {
    let d = doc(vec![
        BlockNode::RawBlock(RawBlockNode {
            format: "html".to_string(),
            value: "<script>alert(1)</script>".to_string(),
        }),
        para(vec![
            text("before "),
            InlineNode::RawInline(RawInlineNode {
                format: "html".to_string(),
                value: "<b>evil</b>".to_string(),
            }),
            text("after"),
        ]),
    ]);
    let xml = body_xml(&d);
    assert!(!xml.contains("script"), "raw block dropped: {xml}");
    assert!(!xml.contains("evil"), "raw inline dropped: {xml}");
    // The surrounding text survives.
    assert!(xml.contains("before ") && xml.contains("after"), "{xml}");
}

/// The headline round-trip: a rich AST → `.docx` → reopened text + structure.
#[test]
fn rich_document_round_trips() {
    let d = doc(vec![
        BlockNode::Heading(HeadingNode {
            level: 1,
            children: vec![text("Title")],
        }),
        para(vec![
            text("A "),
            InlineNode::Strong(StrongNode {
                children: vec![text("bold ")],
            }),
            text("word."),
        ]),
        BlockNode::List(ListNode {
            ordered: false,
            start: None,
            tight: true,
            children: vec![
                ListChildNode::ListItem(ListItemNode {
                    children: vec![para(vec![text("one")])],
                }),
                ListChildNode::ListItem(ListItemNode {
                    children: vec![para(vec![text("two")])],
                }),
            ],
        }),
    ]);
    let bytes = to_docx_bytes(&d);
    let read = open_docx(&bytes).expect("reopen the rendered .docx");
    let text = read.text();
    assert!(text.contains("Title"), "heading text: {text:?}");
    assert!(text.contains("A bold word."), "bold run rejoined: {text:?}");
    assert!(
        text.contains("• one") && text.contains("• two"),
        "list items: {text:?}"
    );
    // Heading, body, and two list items are four distinct paragraphs.
    assert_eq!(read.paragraphs().count(), 4, "para count");
}

// ===========================================================================
// Recursion-depth DoS guard (CWE-674)
// ===========================================================================
// The AST comes from untrusted Markdown, which the upstream parser doesn't cap:
// `>>>>…> x` nests blockquotes arbitrarily, `***…*` emphasis arbitrarily. A
// recursive walker over that would overflow the native stack (an uncatchable
// SIGSEGV). MAX_DEPTH bounds the descent; these tests pin that boundary.

/// Wrap `inner` in `n` nested blockquotes (built inside-out — a loop, no
/// construction recursion).
fn nest_blockquotes(n: usize, inner: Vec<BlockNode>) -> BlockNode {
    let mut node = BlockNode::Blockquote(BlockquoteNode { children: inner });
    for _ in 1..n {
        node = BlockNode::Blockquote(BlockquoteNode {
            children: vec![node],
        });
    }
    node
}

/// Content nested BEYOND `MAX_DEPTH` blockquotes is dropped (the guard fires)
/// rather than recursed into — while shallow content still renders.
#[test]
fn over_deep_blockquote_content_is_dropped() {
    // A "DEEP" paragraph buried ~50 levels past the cap, plus a top-level "TOP".
    let buried = nest_blockquotes(super::MAX_DEPTH + 50, vec![para(vec![text("DEEP")])]);
    let d = doc(vec![para(vec![text("TOP")]), buried]);
    let text = open_docx(&to_docx_bytes(&d)).unwrap().text();
    assert!(text.contains("TOP"), "shallow content rendered: {text:?}");
    assert!(
        !text.contains("DEEP"),
        "over-deep content dropped by the guard: {text:?}"
    );
}

/// Inline content nested BEYOND `MAX_DEPTH` emphasis spans is dropped; a sibling
/// at the top level still renders.
#[test]
fn over_deep_emphasis_content_is_dropped() {
    let mut inner = InlineNode::Text(TextNode {
        value: "INNER".to_string(),
    });
    for _ in 0..super::MAX_DEPTH + 50 {
        inner = InlineNode::Emphasis(EmphasisNode {
            children: vec![inner],
        });
    }
    let d = doc(vec![para(vec![inner, text("AFTER")])]);
    let xml = body_xml(&d);
    assert!(xml.contains("AFTER"), "top-level sibling rendered: {xml}");
    assert!(
        !xml.contains("INNER"),
        "over-deep inline dropped by the guard: {xml}"
    );
}

/// The headline DoS proof: a pathologically deep AST (50k nested blockquotes)
/// converts to a valid `.docx` WITHOUT overflowing the stack. The walker is
/// bounded to `MAX_DEPTH` frames regardless of input depth; the large thread
/// stack here is only so the TEST's own construction/`Drop` of the 50k-deep tree
/// doesn't overflow (that's a property of the test data, not the converter).
/// Without the depth guard, the converter's recursion would follow the input to
/// 50k frames and crash.
#[test]
fn deeply_nested_input_does_not_overflow() {
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(|| {
            let buried = nest_blockquotes(50_000, vec![para(vec![text("x")])]);
            let bytes = to_docx_bytes(&doc(vec![buried]));
            assert_eq!(&bytes[..2], b"PK", "deep input still yields a valid .docx");
        })
        .unwrap()
        .join()
        .expect("converting a 50k-deep AST must not overflow the stack");
}
