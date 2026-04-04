// ============================================================================
// Renderer.swift — Document AST → HTML Renderer
// ============================================================================
//
// Converts a Document AST node tree (from the `DocumentAst` package) into
// an HTML string. This is the standard CommonMark HTML back-end.
//
// # Design
//
// - Block nodes render with a trailing newline (`\n`).
// - Inline nodes render without a trailing newline.
// - `rawBlock` / `rawInline` nodes with `format: "html"` are emitted verbatim.
// - `rawBlock` / `rawInline` nodes with an unknown format are silently skipped.
// - Tight lists suppress `<p>` wrappers around single-paragraph list items.
// - `codeBlock` content is HTML-escaped and always ends with a newline.
// - Heading levels are expected to be 1–6 (guaranteed by the parser).
//
// # HTML Escaping
//
// The characters `&`, `<`, `>`, and `"` are escaped in text content and
// attribute values:
//
//   &  →  &amp;
//   <  →  &lt;
//   >  →  &gt;
//   "  →  &quot;
//
// URLs in `href` and `src` attributes are NOT re-percent-encoded here
// because the parser already normalizes them. The `&` in URLs is still
// escaped to `&amp;` for HTML attribute validity.
//

import DocumentAst

// ── Public API ────────────────────────────────────────────────────────────────

/// Render a `BlockNode` (typically `.document(...)`) to an HTML string.
///
/// Any node type is accepted, but the function is designed for the document
/// root. Rendering a non-document node renders only that node's HTML fragment.
///
///     import DocumentAst
///     import DocumentAstToHtml
///
///     let doc = BlockNode.document(DocumentNode(children: [
///         .paragraph(ParagraphNode(children: [.text(TextNode(value: "Hello"))]))
///     ]))
///     render(doc)  // → "<p>Hello</p>\n"
///
/// - Parameter node: The block node to render.
/// - Returns: An HTML string. Block nodes include a trailing newline.
public func render(_ node: BlockNode) -> String {
    renderBlock(node, tight: false)
}

/// Escape HTML special characters in a string.
///
/// Replaces `&`, `<`, `>`, and `"` with their HTML entity equivalents.
/// This is exposed publicly so that consumers can escape text for use in
/// custom HTML templates.
///
///     htmlEscape("<script>alert('xss')</script>")
///     // → "&lt;script&gt;alert(&#39;xss&#39;)&lt;/script&gt;"
///
/// - Parameter s: The raw string to escape.
/// - Returns: The HTML-safe string.
public func htmlEscape(_ s: String) -> String {
    var result = ""
    result.reserveCapacity(s.count)
    for ch in s {
        switch ch {
        case "&":  result.append("&amp;")
        case "<":  result.append("&lt;")
        case ">":  result.append("&gt;")
        case "\"": result.append("&quot;")
        default:   result.append(ch)
        }
    }
    return result
}

// ── Block Rendering ───────────────────────────────────────────────────────────

/// Render a block node to HTML, threading the `tight` flag for list rendering.
///
/// The `tight` flag suppresses `<p>` wrappers inside tight list items.
/// When a list has `tight: true`, its item paragraphs render as plain text
/// rather than `<p>text</p>`.
///
/// - Parameters:
///   - node: The block node to render.
///   - tight: Whether we're inside a tight list.
/// - Returns: HTML string with a trailing newline.
private func renderBlock(_ node: BlockNode, tight: Bool) -> String {
    switch node {

    case .document(let doc):
        // The document root renders as the concatenation of all its children.
        // No wrapper element is added.
        return doc.children.map { renderBlock($0, tight: tight) }.joined()

    case .heading(let h):
        // ATX headings: # through ######
        // Each heading level maps directly to <h1>–<h6>.
        let tag = "h\(h.level)"
        let inner = renderInlines(h.children)
        return "<\(tag)>\(inner)</\(tag)>\n"

    case .paragraph(let p):
        // In a tight list, paragraph content is rendered without <p> wrappers.
        // This is the CommonMark "tight list" rule: items separated by blank
        // lines get <p> wrappers; items without blank lines do not.
        let inner = renderInlines(p.children)
        if tight {
            return inner + "\n"
        } else {
            return "<p>\(inner)</p>\n"
        }

    case .codeBlock(let cb):
        // Fenced code blocks: ```lang\ncode\n```
        // The language info string becomes a CSS class for syntax highlighting.
        // Take only the first word of the info string (CommonMark spec rule).
        let langAttr: String
        if let lang = cb.language, !lang.trimmingCharacters(in: .whitespaces).isEmpty {
            let firstWord = lang.components(separatedBy: .whitespaces).first ?? lang
            let escapedLang = htmlEscape(firstWord)
            langAttr = " class=\"language-\(escapedLang)\""
        } else {
            langAttr = ""
        }
        let escapedValue = htmlEscape(cb.value)
        return "<pre><code\(langAttr)>\(escapedValue)</code></pre>\n"

    case .blockquote(let bq):
        // Blockquotes nest arbitrarily deep. Inner blocks render with
        // tight: false — the tightness state does not propagate through quotes.
        let inner = bq.children.map { renderBlock($0, tight: false) }.joined()
        return "<blockquote>\n\(inner)</blockquote>\n"

    case .list(let list):
        // Ordered lists use <ol>, unordered use <ul>.
        // The `start` attribute is only emitted when start != 1 (CommonMark spec).
        let tag = list.ordered ? "ol" : "ul"
        let startAttr: String
        if list.ordered, let start = list.start, start != 1 {
            startAttr = " start=\"\(start)\""
        } else {
            startAttr = ""
        }
        // Pass the list's tight flag down to children for paragraph wrapping.
        let inner = list.children.map { renderListItem($0, tight: list.tight) }.joined()
        return "<\(tag)\(startAttr)>\n\(inner)</\(tag)>\n"

    case .listItem(let item):
        // Standalone list_item nodes (outside of a list context) render with
        // tight: false by default.
        return renderListItem(item, tight: false)

    case .taskItem(let task):
        return renderTaskItem(task, tight: false)

    case .thematicBreak:
        // Horizontal rule — the self-closing form is canonical CommonMark HTML.
        return "<hr />\n"

    case .rawBlock(let rb):
        // Only HTML raw blocks are emitted. All other formats are silently
        // skipped. This allows LaTeX renderers to emit LaTeX raw blocks while
        // HTML renderers ignore them.
        if rb.format == "html" {
            return rb.value
        }
        return ""

    case .table(let t):
        return renderTable(t)

    case .tableRow(let row):
        // Standalone table_row (outside a table context): no alignments known.
        let alignments = [TableAlignment?](repeating: nil, count: row.children.count)
        return renderTableRow(row, alignments: alignments)

    case .tableCell(let cell):
        // Standalone table_cell: render as <td>.
        return "<td>\(renderInlines(cell.children))</td>\n"
    }
}

// ── List Item Rendering ───────────────────────────────────────────────────────

/// Render a single `ListItemNode` to HTML.
///
/// The `tight` flag controls whether a single-paragraph item is rendered
/// without a `<p>` wrapper (CommonMark tight list rule).
///
/// Rendering rules:
///   1. Empty item: `<li></li>`
///   2. Tight, single paragraph: `<li>text</li>` (no `<p>` wrapper)
///   3. Tight, paragraph + more: `<li>text\n<block>...</li>`
///   4. Tight, first child not a paragraph: `<li>\n<block>...</li>`
///   5. Loose: `<li>\n<block>...\n</li>`
private func renderListItem(_ item: ListItemNode, tight: Bool) -> String {
    let children = item.children
    if children.isEmpty {
        return "<li></li>\n"
    }
    if tight {
        return renderTightListItem(children)
    } else {
        let inner = children.map { renderBlock($0, tight: false) }.joined()
        return "<li>\n\(inner)</li>\n"
    }
}

/// Render a task item (GFM checkbox) to HTML.
///
/// The checkbox is rendered as `<input type="checkbox" disabled>` with an
/// optional `checked` attribute.
private func renderTaskItem(_ task: TaskItemNode, tight: Bool) -> String {
    let checkbox = task.checked
        ? "<input type=\"checkbox\" disabled=\"\" checked=\"\" />"
        : "<input type=\"checkbox\" disabled=\"\" />"

    let children = task.children
    if children.isEmpty {
        return "<li>\(checkbox)</li>\n"
    }

    // If tight and first child is a paragraph, inline its content.
    if tight, case .paragraph(let p) = children.first {
        let firstContent = renderInlines(p.children)
        let content = firstContent.isEmpty ? checkbox : "\(checkbox) \(firstContent)"
        if children.count == 1 {
            return "<li>\(content)</li>\n"
        } else {
            let rest = children.dropFirst().map { renderBlock($0, tight: tight) }.joined()
            return "<li>\(content)\n\(rest)</li>\n"
        }
    }

    let inner = children.map { renderBlock($0, tight: tight) }.joined()
    return "<li>\(checkbox)\n\(inner)</li>\n"
}

/// Render the children of a tight list item following CommonMark rules.
///
/// The tricky part: if the first child is a paragraph, its content is inlined
/// (no `<p>` wrapper). If the last child is a paragraph (tight), its trailing
/// newline is stripped. Any other blocks are rendered normally.
///
/// This matches the CommonMark spec examples for tight lists exactly.
private func renderTightListItem(_ children: [BlockNode]) -> String {
    if children.isEmpty {
        return "<li></li>\n"
    }

    // Single paragraph item — no <p> wrapper (canonical tight list item).
    if children.count == 1, case .paragraph(let p) = children[0] {
        return "<li>\(renderInlines(p.children))</li>\n"
    }

    // First child is a paragraph, more children follow.
    if case .paragraph(let p) = children[0] {
        let paraText = renderInlines(p.children)
        let restHtml = children.dropFirst().map { renderBlock($0, tight: true) }.joined()
        return "<li>\(paraText)\n\(restHtml)</li>\n"
    }

    // First child is a non-paragraph block (heading, code block, nested list, etc.).
    // All-but-last children render normally; the last child strips its trailing
    // newline if it's a tight paragraph.
    let initChildren = children.dropLast()
    let lastChild = children.last!

    let initHtml = initChildren.map { renderBlock($0, tight: true) }.joined()
    let lastHtml: String
    if case .paragraph(let p) = lastChild {
        // Tight paragraph at end: no trailing newline before </li>.
        lastHtml = renderInlines(p.children)
    } else {
        lastHtml = renderBlock(lastChild, tight: true)
    }
    return "<li>\n\(initHtml)\(lastHtml)</li>\n"
}

// ── Table Rendering ───────────────────────────────────────────────────────────

/// Render a `TableNode` to HTML.
///
/// Tables are split into `<thead>` (rows with `isHeader: true`) and
/// `<tbody>` (remaining rows). Each section is only emitted if it has rows.
/// Alignment is expressed as `align="left"` etc. attributes on `<th>`/`<td>`.
private func renderTable(_ table: TableNode) -> String {
    let headerRows = table.children.filter { $0.isHeader }
    let bodyRows = table.children.filter { !$0.isHeader }
    let alignments = table.align

    var parts: [String] = ["<table>\n"]

    if !headerRows.isEmpty {
        parts.append("<thead>\n")
        parts.append(contentsOf: headerRows.map { renderTableRow($0, alignments: alignments) })
        parts.append("</thead>\n")
    }

    if !bodyRows.isEmpty {
        parts.append("<tbody>\n")
        parts.append(contentsOf: bodyRows.map { renderTableRow($0, alignments: alignments) })
        parts.append("</tbody>\n")
    }

    parts.append("</table>\n")
    return parts.joined()
}

/// Render one table row (`<tr>`), threading column alignments.
private func renderTableRow(_ row: TableRowNode, alignments: [TableAlignment?]) -> String {
    let cellsHtml = row.children.enumerated().map { (index, cell) -> String in
        let alignment = index < alignments.count ? alignments[index] : nil
        return renderTableCell(cell, isHeader: row.isHeader, alignment: alignment)
    }.joined()
    return "<tr>\n\(cellsHtml)</tr>\n"
}

/// Render one table cell (`<td>` or `<th>`) with optional alignment.
///
/// Header cells use `<th>`, body cells use `<td>`.
/// The alignment is expressed as `align="left"` etc. (CommonMark HTML spec).
private func renderTableCell(_ cell: TableCellNode, isHeader: Bool, alignment: TableAlignment?) -> String {
    let tag = isHeader ? "th" : "td"
    let alignAttr: String
    if let a = alignment {
        switch a {
        case .left:   alignAttr = " align=\"left\""
        case .center: alignAttr = " align=\"center\""
        case .right:  alignAttr = " align=\"right\""
        }
    } else {
        alignAttr = ""
    }
    let inner = renderInlines(cell.children)
    return "<\(tag)\(alignAttr)>\(inner)</\(tag)>\n"
}

// ── Inline Rendering ──────────────────────────────────────────────────────────

/// Render a sequence of inline nodes to HTML, joined without separators.
private func renderInlines(_ nodes: [InlineNode]) -> String {
    nodes.map { renderInline($0) }.joined()
}

/// Render a single inline node to HTML.
///
/// Inline nodes do NOT include a trailing newline (unlike block nodes).
private func renderInline(_ node: InlineNode) -> String {
    switch node {

    case .text(let t):
        // Plain text is HTML-escaped to prevent XSS.
        return htmlEscape(t.value)

    case .emphasis(let e):
        // *text* or _text_ → <em>text</em>
        return "<em>\(renderInlines(e.children))</em>"

    case .strong(let s):
        // **text** or __text__ → <strong>text</strong>
        return "<strong>\(renderInlines(s.children))</strong>"

    case .strikethrough(let s):
        // ~~text~~ → <del>text</del> (GFM extension)
        return "<del>\(renderInlines(s.children))</del>"

    case .codeSpan(let cs):
        // `code` → <code>code</code> (content is HTML-escaped)
        return "<code>\(htmlEscape(cs.value))</code>"

    case .link(let link):
        // [text](url "title") → <a href="url" title="title">text</a>
        let titleAttr = link.title.map { " title=\"\(htmlEscape($0))\"" } ?? ""
        let href = escapeUrl(link.destination)
        return "<a href=\"\(href)\"\(titleAttr)>\(renderInlines(link.children))</a>"

    case .image(let img):
        // ![alt](url "title") → <img src="url" alt="alt" title="title" />
        let titleAttr = img.title.map { " title=\"\(htmlEscape($0))\"" } ?? ""
        let src = escapeUrl(img.destination)
        let alt = htmlEscape(img.alt)
        return "<img src=\"\(src)\" alt=\"\(alt)\"\(titleAttr) />"

    case .autolink(let al):
        // <https://url> or <user@email> — angle-bracket autolinks.
        // URL autolinks: href = raw url; display = raw url.
        // Email autolinks: href = mailto:email; display = email.
        let display = htmlEscape(al.destination)
        if al.isEmail {
            return "<a href=\"mailto:\(display)\">\(display)</a>"
        } else {
            let href = normalizeUrlForHtml(al.destination)
            return "<a href=\"\(href)\">\(display)</a>"
        }

    case .rawInline(let ri):
        // Raw HTML inline: emit verbatim. Unknown formats are silently skipped.
        if ri.format == "html" {
            return ri.value
        }
        return ""

    case .hardBreak:
        // Two trailing spaces or backslash before newline → <br />\n
        return "<br />\n"

    case .softBreak:
        // Single newline inside a paragraph → \n (browsers collapse to space)
        return "\n"
    }
}

// ── URL Handling ──────────────────────────────────────────────────────────────

/// Escape `&` in a pre-normalized URL for use in an HTML attribute.
///
/// Pre-normalized URLs (from inline links `[text](url)`) have already been
/// percent-encoded by the parser. We only need to escape `&` → `&amp;` to
/// keep the HTML attribute well-formed.
///
/// - Parameter url: A pre-normalized URL string.
/// - Returns: An HTML-attribute-safe URL string.
private func escapeUrl(_ url: String) -> String {
    url.replacingOccurrences(of: "&", with: "&amp;")
}

/// Percent-encode unsafe characters in an autolink URL and escape `&`.
///
/// Autolink URLs (`<https://example.com>`) are stored as raw strings
/// without prior percent-encoding. This function encodes any character
/// that is unsafe in an HTML `href` attribute.
///
/// Safe characters (not encoded): `A-Z a-z 0-9 - . _ ~ : / ? # @ ! $ & ' ( ) * + , ; = %`
///
/// - Parameter url: A raw autolink URL string.
/// - Returns: A percent-encoded, HTML-attribute-safe URL string.
private func normalizeUrlForHtml(_ url: String) -> String {
    var result = ""
    result.reserveCapacity(url.count)
    for ch in url.unicodeScalars {
        if ch.value == 38 { // '&'
            result.append("&amp;")
        } else if isSafeUrlChar(ch) {
            result.append(Character(ch))
        } else {
            // Percent-encode the UTF-8 bytes of this character
            let scalar = Character(ch)
            let utf8 = String(scalar).utf8
            for byte in utf8 {
                result.append(String(format: "%%%02X", byte))
            }
        }
    }
    return result
}

/// Returns `true` if the given Unicode scalar is safe in a URL (RFC 3986 +
/// common URL characters). These characters do not need percent-encoding.
private func isSafeUrlChar(_ scalar: Unicode.Scalar) -> Bool {
    let v = scalar.value
    // ASCII alphanumeric
    if (v >= 65 && v <= 90) || (v >= 97 && v <= 122) || (v >= 48 && v <= 57) {
        return true
    }
    // Safe punctuation: - . _ ~ : / ? # @ ! $ & ' ( ) * + , ; = %
    let safePunct: [UInt32] = [45, 46, 95, 126, 58, 47, 63, 35, 64, 33, 36, 38, 39, 40, 41, 42, 43, 44, 59, 61, 37]
    return safePunct.contains(v)
}
