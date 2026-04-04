// ============================================================================
// Node.swift — Document AST Node Types
// ============================================================================
//
// This file defines every node type in the Document AST, the format-agnostic
// intermediate representation (IR) used by all parsers and renderers in this
// project.
//
// # Design Principles
//
// 1. **Semantic, not notational** — nodes carry meaning, not Markdown syntax.
//    A `HeadingNode` represents a section heading; it doesn't care whether
//    the source was `# Foo` (ATX) or `Foo\n===` (setext).
//
// 2. **Resolved, not deferred** — all link references are resolved by the
//    front-end parser before the AST is emitted. The IR never contains
//    unresolved `[label]` references.
//
// 3. **Format-agnostic** — instead of `html_block` / `html_inline`, the IR
//    uses `RawBlockNode` / `RawInlineNode` with a `format` tag. Back-ends
//    skip nodes with an unknown format. This lets a LaTeX back-end ignore
//    HTML passthrough blocks, and vice versa.
//
// 4. **Minimal and stable** — only concepts that exist in essentially all
//    document formats. No Markdown-specific constructs.
//
// # Node Hierarchy
//
//   BlockNode            — structural elements
//     document           — root of the entire document
//     heading            — h1–h6
//     paragraph          — flow of inline content
//     codeBlock          — fenced or indented code
//     blockquote         — nested quoted content
//     list               — ordered or unordered list
//     listItem           — one item in a list
//     taskItem           — GFM-style checkbox list item
//     thematicBreak      — horizontal rule <hr>
//     rawBlock           — format-specific passthrough (e.g. HTML)
//     table              — tabular data
//     tableRow           — one row (header or body)
//     tableCell          — one cell
//
//   InlineNode           — character-level content
//     text               — plain text
//     emphasis           — <em>
//     strong             — <strong>
//     codeSpan           — inline <code>
//     link               — <a href>
//     image              — <img>
//     autolink           — <https://...> or <user@email>
//     rawInline          — format-specific passthrough
//     hardBreak          — forced line break <br>
//     softBreak          — soft line break (whitespace in HTML)
//     strikethrough      — <del> (GFM extension)
//

import Foundation

// ── Table Alignment ─────────────────────────────────────────────────────────

/// The horizontal alignment of a table column.
///
/// Used in `TableNode.align` to specify the alignment for each column.
/// A `nil` alignment (represented by the optional `TableAlignment?` type)
/// means no alignment attribute is set on that column.
///
/// Truth table:
///   | Markdown | Alignment |
///   |----------|-----------|
///   | `:---`   | `.left`   |
///   | `:---:`  | `.center` |
///   | `---:`   | `.right`  |
///   | `---`    | nil       |
public enum TableAlignment: Equatable, Sendable {
    /// Left-align the column (`text-align: left`).
    case left
    /// Center-align the column (`text-align: center`).
    case center
    /// Right-align the column (`text-align: right`).
    case right
}

// ── Block Node Structs ───────────────────────────────────────────────────────

/// The root node of a Document AST.
///
/// Every document is wrapped in a `DocumentNode`. It is the only node type
/// that can never appear as a child of another node.
///
///     let doc = DocumentNode(children: [
///         .paragraph(ParagraphNode(children: [.text(TextNode(value: "Hello"))]))
///     ])
public struct DocumentNode: Equatable, Sendable {
    /// The top-level block content of the document.
    public let children: [BlockNode]

    public init(children: [BlockNode]) {
        self.children = children
    }
}

/// A section heading (h1–h6).
///
/// Corresponds to `<h1>` through `<h6>` in HTML. Markdown produces headings
/// from ATX syntax (`# Heading`) and setext syntax (`Heading\n======`).
///
///     let h1 = HeadingNode(level: 1, children: [.text(TextNode(value: "Introduction"))])
public struct HeadingNode: Equatable, Sendable {
    /// The heading level, 1 (most important) through 6 (least important).
    public let level: Int
    /// The inline content of the heading.
    public let children: [InlineNode]

    public init(level: Int, children: [InlineNode]) {
        self.level = level
        self.children = children
    }
}

/// A paragraph of inline content.
///
/// Paragraphs are the default block type: any non-blank text that doesn't
/// match a more specific pattern becomes a paragraph. Blank lines separate
/// consecutive paragraphs.
///
///     let para = ParagraphNode(children: [.text(TextNode(value: "Hello, world."))])
public struct ParagraphNode: Equatable, Sendable {
    /// The inline content of the paragraph.
    public let children: [InlineNode]

    public init(children: [InlineNode]) {
        self.children = children
    }
}

/// A block of preformatted (monospace) code.
///
/// Produced by fenced code blocks (` ```lang ... ``` `) and indented code
/// blocks (4+ leading spaces). The `value` always ends with a newline.
///
///     let code = CodeBlockNode(language: "swift", value: "let x = 1\n")
public struct CodeBlockNode: Equatable, Sendable {
    /// The info string language tag (e.g. `"swift"`, `"python"`), or `nil` if
    /// no language was specified.
    public let language: String?
    /// The raw source code, including the final newline character.
    public let value: String

    public init(language: String?, value: String) {
        self.language = language
        self.value = value
    }
}

/// A block quotation.
///
/// Produced by `> ` prefixes in Markdown. May contain any block content,
/// including nested blockquotes.
///
///     let bq = BlockquoteNode(children: [
///         .paragraph(ParagraphNode(children: [.text(TextNode(value: "Wise words."))]))
///     ])
public struct BlockquoteNode: Equatable, Sendable {
    /// The block content inside the quotation.
    public let children: [BlockNode]

    public init(children: [BlockNode]) {
        self.children = children
    }
}

/// An ordered or unordered list.
///
/// Unordered lists use bullet markers (`-`, `*`, `+`). Ordered lists use
/// numeric markers (`1.`, `1)`). A "tight" list suppresses `<p>` wrappers
/// around single-paragraph items in HTML output.
///
///     let ul = ListNode(ordered: false, start: nil, tight: true, children: [
///         ListItemNode(children: [
///             .paragraph(ParagraphNode(children: [.text(TextNode(value: "Item A"))]))
///         ])
///     ])
public struct ListNode: Equatable, Sendable {
    /// `true` for ordered lists (numbered), `false` for unordered (bulleted).
    public let ordered: Bool
    /// The starting number for ordered lists. `nil` for unordered lists.
    /// Defaults to 1; values other than 1 produce `<ol start="N">`.
    public let start: Int?
    /// Tight lists suppress `<p>` wrappers on single-paragraph items.
    public let tight: Bool
    /// The list items.
    public let children: [ListItemNode]

    public init(ordered: Bool, start: Int?, tight: Bool, children: [ListItemNode]) {
        self.ordered = ordered
        self.start = start
        self.tight = tight
        self.children = children
    }
}

/// One item in a list.
///
/// May contain any block content (paragraphs, code, nested lists).
public struct ListItemNode: Equatable, Sendable {
    /// The block content of this list item.
    public let children: [BlockNode]

    public init(children: [BlockNode]) {
        self.children = children
    }
}

/// A GFM-style task list item with a checkbox.
///
/// Produced by `- [x] text` (checked) or `- [ ] text` (unchecked).
///
///     let task = TaskItemNode(checked: true, children: [
///         .paragraph(ParagraphNode(children: [.text(TextNode(value: "Done"))]))
///     ])
public struct TaskItemNode: Equatable, Sendable {
    /// `true` if the checkbox is checked (`[x]`), `false` if unchecked (`[ ]`).
    public let checked: Bool
    /// The block content of this task item.
    public let children: [BlockNode]

    public init(checked: Bool, children: [BlockNode]) {
        self.checked = checked
        self.children = children
    }
}

/// A thematic break (horizontal rule).
///
/// Produced by `---`, `***`, or `___` (three or more of the same character)
/// on a line by itself. Renders as `<hr />` in HTML.
public struct ThematicBreakNode: Equatable, Sendable {
    public init() {}
}

/// A raw, format-specific passthrough block.
///
/// Instead of HTML-specific `html_block` nodes, the Document AST uses
/// `raw_block` with a `format` field. Back-ends skip nodes with an unknown
/// format, so a LaTeX renderer ignores `format: "html"` blocks.
///
///     let html = RawBlockNode(format: "html", value: "<div class=\"note\">...</div>\n")
public struct RawBlockNode: Equatable, Sendable {
    /// The target format identifier (e.g. `"html"`, `"latex"`).
    public let format: String
    /// The raw content to pass through verbatim.
    public let value: String

    public init(format: String, value: String) {
        self.format = format
        self.value = value
    }
}

/// A table (GFM extension).
///
/// Tables consist of rows. The first row(s) with `isHeader: true` form the
/// `<thead>`, and remaining rows form the `<tbody>`.
///
///     let table = TableNode(
///         align: [.left, .center, .right],
///         children: [headerRow, bodyRow]
///     )
public struct TableNode: Equatable, Sendable {
    /// Column alignments, one per column. `nil` means no alignment attribute.
    public let align: [TableAlignment?]
    /// The rows of the table (header and body mixed; sorted by `isHeader`).
    public let children: [TableRowNode]

    public init(align: [TableAlignment?], children: [TableRowNode]) {
        self.align = align
        self.children = children
    }
}

/// One row of a table.
///
/// Rows with `isHeader: true` appear inside `<thead>`, other rows in `<tbody>`.
public struct TableRowNode: Equatable, Sendable {
    /// `true` if this row is a header row (uses `<th>` cells).
    public let isHeader: Bool
    /// The cells in this row, left to right.
    public let children: [TableCellNode]

    public init(isHeader: Bool, children: [TableCellNode]) {
        self.isHeader = isHeader
        self.children = children
    }
}

/// One cell of a table row.
///
/// Alignment is stored on the parent `TableNode`, not on the cell.
public struct TableCellNode: Equatable, Sendable {
    /// The inline content of the cell.
    public let children: [InlineNode]

    public init(children: [InlineNode]) {
        self.children = children
    }
}

// ── Inline Node Structs ──────────────────────────────────────────────────────

/// A run of plain text.
///
/// HTML character entities are decoded by the parser before being stored.
/// Adjacent text nodes may be merged during inline parsing.
///
///     let t = TextNode(value: "Hello, world!")
public struct TextNode: Equatable, Sendable {
    /// The decoded text content.
    public let value: String

    public init(value: String) {
        self.value = value
    }
}

/// Emphasis — light stress (italic in most renderers).
///
/// Produced by `*text*` or `_text_`. Renders as `<em>text</em>` in HTML.
public struct EmphasisNode: Equatable, Sendable {
    /// The emphasized inline content.
    public let children: [InlineNode]

    public init(children: [InlineNode]) {
        self.children = children
    }
}

/// Strong emphasis — heavy stress (bold in most renderers).
///
/// Produced by `**text**` or `__text__`. Renders as `<strong>text</strong>`.
public struct StrongNode: Equatable, Sendable {
    /// The strongly emphasized inline content.
    public let children: [InlineNode]

    public init(children: [InlineNode]) {
        self.children = children
    }
}

/// An inline code span.
///
/// Produced by `` `code` ``. Renders as `<code>code</code>` in HTML.
/// Backslash escapes do not apply inside code spans.
public struct CodeSpanNode: Equatable, Sendable {
    /// The raw code content (whitespace-normalized by the parser).
    public let value: String

    public init(value: String) {
        self.value = value
    }
}

/// A hyperlink.
///
/// The `destination` is always a fully resolved, percent-encoded URL.
/// Never contains an unresolved `[label]` reference — the parser resolves
/// all link references before producing the AST.
///
///     let link = LinkNode(
///         destination: "https://example.com",
///         title: "Example",
///         children: [.text(TextNode(value: "click here"))]
///     )
public struct LinkNode: Equatable, Sendable {
    /// The fully resolved URL destination.
    public let destination: String
    /// An optional tooltip/title string, or `nil`.
    public let title: String?
    /// The link text as inline nodes.
    public let children: [InlineNode]

    public init(destination: String, title: String?, children: [InlineNode]) {
        self.destination = destination
        self.title = title
        self.children = children
    }
}

/// An embedded image.
///
/// Similar to `LinkNode` but for images. The `alt` field is a plain-text
/// fallback (all inline markup stripped from the alt text).
///
///     let img = ImageNode(destination: "cat.png", title: nil, alt: "a fluffy cat")
public struct ImageNode: Equatable, Sendable {
    /// The fully resolved URL of the image resource.
    public let destination: String
    /// An optional title/tooltip, or `nil`.
    public let title: String?
    /// Plain-text alternative description (for `alt` attribute).
    public let alt: String

    public init(destination: String, title: String?, alt: String) {
        self.destination = destination
        self.title = title
        self.alt = alt
    }
}

/// An automatic link from an angle-bracket URL or email address.
///
/// Produced by `<https://example.com>` or `<user@example.com>`.
/// The `isEmail` flag distinguishes the two cases for rendering
/// (`href="mailto:user@..."` vs `href="https://..."`).
///
///     let url = AutolinkNode(destination: "https://example.com", isEmail: false)
///     let email = AutolinkNode(destination: "user@example.com", isEmail: true)
public struct AutolinkNode: Equatable, Sendable {
    /// The URL or email address (without angle brackets).
    public let destination: String
    /// `true` for email autolinks, `false` for URL autolinks.
    public let isEmail: Bool

    public init(destination: String, isEmail: Bool) {
        self.destination = destination
        self.isEmail = isEmail
    }
}

/// A raw, format-specific passthrough inline.
///
/// Inline counterpart to `RawBlockNode`. Back-ends skip nodes with an
/// unknown format.
///
///     let raw = RawInlineNode(format: "html", value: "<kbd>Ctrl</kbd>")
public struct RawInlineNode: Equatable, Sendable {
    /// The target format identifier (e.g. `"html"`, `"latex"`).
    public let format: String
    /// The raw content to pass through verbatim.
    public let value: String

    public init(format: String, value: String) {
        self.format = format
        self.value = value
    }
}

/// A hard line break (forced new line).
///
/// Produced in Markdown by two or more trailing spaces before a newline,
/// or a backslash immediately before a newline. Renders as `<br />\n` in HTML.
public struct HardBreakNode: Equatable, Sendable {
    public init() {}
}

/// A soft line break.
///
/// A single newline within a paragraph. In HTML, browsers collapse it to
/// a space, but it renders as a literal newline in plain-text output.
public struct SoftBreakNode: Equatable, Sendable {
    public init() {}
}

/// Strikethrough text (GFM extension).
///
/// Produced by `~~text~~`. Renders as `<del>text</del>` in HTML.
public struct StrikethroughNode: Equatable, Sendable {
    /// The struck-through inline content.
    public let children: [InlineNode]

    public init(children: [InlineNode]) {
        self.children = children
    }
}

// ── Block Enum ───────────────────────────────────────────────────────────────

/// A block-level node in the Document AST.
///
/// Block nodes form the structural skeleton of a document. They stack
/// vertically and may contain either other blocks (e.g. `blockquote`,
/// `list`) or inline content (e.g. `paragraph`, `heading`).
///
/// The `indirect` keyword allows recursive enum cases (e.g. a `blockquote`
/// containing a `blockquote`).
public indirect enum BlockNode: Equatable, Sendable {
    /// The document root.
    case document(DocumentNode)
    /// A heading (h1–h6).
    case heading(HeadingNode)
    /// A paragraph.
    case paragraph(ParagraphNode)
    /// A fenced or indented code block.
    case codeBlock(CodeBlockNode)
    /// A block quotation.
    case blockquote(BlockquoteNode)
    /// An ordered or unordered list.
    case list(ListNode)
    /// One item in a list.
    case listItem(ListItemNode)
    /// A GFM task list item with a checkbox.
    case taskItem(TaskItemNode)
    /// A thematic break (horizontal rule).
    case thematicBreak
    /// A format-specific raw passthrough block.
    case rawBlock(RawBlockNode)
    /// A table.
    case table(TableNode)
    /// A table row.
    case tableRow(TableRowNode)
    /// A table cell.
    case tableCell(TableCellNode)
}

// ── Inline Enum ──────────────────────────────────────────────────────────────

/// An inline-level node in the Document AST.
///
/// Inline nodes flow horizontally within block containers. They represent
/// character-level markup and content.
///
/// The `indirect` keyword allows recursive cases (e.g. `emphasis` containing
/// `strong` containing `text`).
public indirect enum InlineNode: Equatable, Sendable {
    /// Plain text.
    case text(TextNode)
    /// Emphasized (italic) text.
    case emphasis(EmphasisNode)
    /// Strongly emphasized (bold) text.
    case strong(StrongNode)
    /// An inline code span.
    case codeSpan(CodeSpanNode)
    /// A hyperlink.
    case link(LinkNode)
    /// An embedded image.
    case image(ImageNode)
    /// An automatic link (URL or email).
    case autolink(AutolinkNode)
    /// A format-specific raw passthrough inline.
    case rawInline(RawInlineNode)
    /// A hard line break (`<br />`).
    case hardBreak
    /// A soft line break (newline collapsed to space in HTML).
    case softBreak
    /// Strikethrough text (GFM `~~text~~`).
    case strikethrough(StrikethroughNode)
}
