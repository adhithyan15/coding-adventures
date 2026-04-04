// ============================================================================
// InlineParser.swift — AsciiDoc Phase 2: Inline Content Scanner
// ============================================================================
//
// Converts raw inline strings (from BlockParser) into arrays of `InlineNode`
// values. Uses a left-to-right character scanner with simple lookahead.
//
// # AsciiDoc Inline Constructs
//
// | Syntax        | Node produced                  | Notes                      |
// |---------------|--------------------------------|----------------------------|
// | `**text**`    | `.strong(StrongNode)`          | Unconstrained strong       |
// | `*text*`      | `.strong(StrongNode)`          | Constrained strong (bold!) |
// | `__text__`    | `.emphasis(EmphasisNode)`      | Unconstrained emphasis     |
// | `_text_`      | `.emphasis(EmphasisNode)`      | Constrained emphasis       |
// | `` `code` ``  | `.codeSpan(CodeSpanNode)`      | Verbatim, no inner markup  |
// | `link:url[t]` | `.link(LinkNode)`              | AsciiDoc link macro        |
// | `image:u[a]`  | `.image(ImageNode)`            | AsciiDoc image macro       |
// | `<<a,text>>`  | `.link(LinkNode)`              | Cross-reference            |
// | `<<anchor>>`  | `.link(LinkNode)`              | Cross-reference (no label) |
// | `https://…`   | `.autolink` or `.link`         | Bare URL (with/without []) |
// | `http://…`    | `.autolink` or `.link`         | Bare URL (with/without []) |
// | `\n`          | `.softBreak`                   | Single newline             |
// | `  \n`        | `.hardBreak`                   | Two trailing spaces + \n   |
// | `\\\n`        | `.hardBreak`                   | Backslash + newline        |
//
// # KEY DIFFERENCE FROM COMMONMARK
//
// In AsciiDoc, `*text*` means **bold** (StrongNode), not italic!
// In CommonMark, `*text*` means *italic* (EmphasisNode).
//
// Mnemonic: In AsciiDoc, `_` = italic, `*` = bold (at all delimiter counts).
//
// # Delimiter Priority
//
// The scanner checks `**` before `*` and `__` before `_` to ensure that
// the two-character delimiters (unconstrained) are matched first.
//
// # Implementation
//
// The scanner maintains a `remaining` string and a `buf` accumulator.
// At each step it checks whether the front of `remaining` matches any
// inline construct pattern in priority order. If nothing matches, the
// first character is consumed into `buf`.
//

import DocumentAst

/// Phase 2 AsciiDoc inline content parser.
///
/// The parser scans raw inline strings and produces `InlineNode` arrays.
/// Use `InlineParser.parse(_:)` as the entry point.
enum InlineParser {

    // ── Public Entry Point ────────────────────────────────────────────────

    /// Parse a raw inline string into an array of inline nodes.
    ///
    /// - Parameter text: The raw inline content (e.g., paragraph or heading text).
    /// - Returns: An array of `InlineNode` values.
    static func parse(_ text: String) -> [InlineNode] {
        var nodes: [InlineNode] = []
        var buf = ""         // Accumulates plain text characters
        var remaining = text // The yet-to-be-processed part of the string

        while !remaining.isEmpty {

            // ── Hard break: two trailing spaces + newline ──────────────────
            if remaining.hasPrefix("  \n") {
                if !buf.isEmpty { nodes.append(.text(TextNode(value: buf))); buf = "" }
                nodes.append(.hardBreak)
                remaining = String(remaining.dropFirst(3))

            // ── Hard break: backslash + newline ────────────────────────────
            } else if remaining.hasPrefix("\\\n") {
                if !buf.isEmpty { nodes.append(.text(TextNode(value: buf))); buf = "" }
                nodes.append(.hardBreak)
                remaining = String(remaining.dropFirst(2))

            // ── Soft break: single newline ─────────────────────────────────
            } else if remaining.hasPrefix("\n") {
                if !buf.isEmpty { nodes.append(.text(TextNode(value: buf))); buf = "" }
                nodes.append(.softBreak)
                remaining = String(remaining.dropFirst(1))

            // ── Code span: `code` ──────────────────────────────────────────
            // Content is verbatim — no inline parsing inside backticks.
            } else if remaining.hasPrefix("`") {
                if let (content, rest) = findClosing(String(remaining.dropFirst()), delimiter: "`") {
                    if !buf.isEmpty { nodes.append(.text(TextNode(value: buf))); buf = "" }
                    nodes.append(.codeSpan(CodeSpanNode(value: content)))
                    remaining = rest
                } else {
                    buf += "`"
                    remaining = String(remaining.dropFirst())
                }

            // ── Strong (unconstrained): **text** ───────────────────────────
            // NOTE: In AsciiDoc, ** = strong (bold), not emphasis!
            // Check ** before * to avoid matching the first char of **
            } else if remaining.hasPrefix("**") {
                if let (content, rest) = findClosing(String(remaining.dropFirst(2)), delimiter: "**") {
                    if !buf.isEmpty { nodes.append(.text(TextNode(value: buf))); buf = "" }
                    let children = parse(content)
                    nodes.append(.strong(StrongNode(children: children)))
                    remaining = rest
                } else {
                    buf += "**"
                    remaining = String(remaining.dropFirst(2))
                }

            // ── Emphasis (unconstrained): __text__ ─────────────────────────
            // Check __ before _ to avoid matching the first char of __
            } else if remaining.hasPrefix("__") {
                if let (content, rest) = findClosing(String(remaining.dropFirst(2)), delimiter: "__") {
                    if !buf.isEmpty { nodes.append(.text(TextNode(value: buf))); buf = "" }
                    let children = parse(content)
                    nodes.append(.emphasis(EmphasisNode(children: children)))
                    remaining = rest
                } else {
                    buf += "__"
                    remaining = String(remaining.dropFirst(2))
                }

            // ── Strong (constrained): *text* ──────────────────────────────
            // AsciiDoc: single * = bold/strong (NOT italic as in CommonMark!)
            } else if remaining.hasPrefix("*") {
                if let (content, rest) = findClosing(String(remaining.dropFirst()), delimiter: "*") {
                    if !buf.isEmpty { nodes.append(.text(TextNode(value: buf))); buf = "" }
                    let children = parse(content)
                    nodes.append(.strong(StrongNode(children: children)))
                    remaining = rest
                } else {
                    buf += "*"
                    remaining = String(remaining.dropFirst())
                }

            // ── Emphasis (constrained): _text_ ────────────────────────────
            } else if remaining.hasPrefix("_") {
                if let (content, rest) = findClosing(String(remaining.dropFirst()), delimiter: "_") {
                    if !buf.isEmpty { nodes.append(.text(TextNode(value: buf))); buf = "" }
                    let children = parse(content)
                    nodes.append(.emphasis(EmphasisNode(children: children)))
                    remaining = rest
                } else {
                    buf += "_"
                    remaining = String(remaining.dropFirst())
                }

            // ── Link macro: link:url[text] ─────────────────────────────────
            // AsciiDoc link macro syntax: `link:DESTINATION[LABEL]`
            // URL is everything between `link:` and `[`.
            // Label is the content of `[...]`.
            } else if remaining.hasPrefix("link:") {
                let afterLink = String(remaining.dropFirst(5)) // drop "link:"
                if let bracketIdx = afterLink.firstIndex(of: "[") {
                    let url = String(afterLink[..<bracketIdx])
                    let afterBracket = String(afterLink[afterLink.index(after: bracketIdx)...])
                    if let closeBracket = afterBracket.firstIndex(of: "]") {
                        let label = String(afterBracket[..<closeBracket])
                        let afterClose = String(afterBracket[afterBracket.index(after: closeBracket)...])
                        if !buf.isEmpty { nodes.append(.text(TextNode(value: buf))); buf = "" }
                        let textNodes: [InlineNode] = label.isEmpty
                            ? [.text(TextNode(value: url))]
                            : [.text(TextNode(value: label))]
                        nodes.append(.link(LinkNode(destination: url, title: nil, children: textNodes)))
                        remaining = afterClose
                    } else {
                        buf += "link:"
                        remaining = String(remaining.dropFirst(5))
                    }
                } else {
                    buf += "link:"
                    remaining = String(remaining.dropFirst(5))
                }

            // ── Image macro: image:url[alt] ────────────────────────────────
            // AsciiDoc inline image: `image:URL[ALT TEXT]`
            } else if remaining.hasPrefix("image:") {
                let afterImage = String(remaining.dropFirst(6)) // drop "image:"
                if let bracketIdx = afterImage.firstIndex(of: "[") {
                    let url = String(afterImage[..<bracketIdx])
                    let afterBracket = String(afterImage[afterImage.index(after: bracketIdx)...])
                    if let closeBracket = afterBracket.firstIndex(of: "]") {
                        let alt = String(afterBracket[..<closeBracket])
                        let afterClose = String(afterBracket[afterBracket.index(after: closeBracket)...])
                        if !buf.isEmpty { nodes.append(.text(TextNode(value: buf))); buf = "" }
                        nodes.append(.image(ImageNode(destination: url, title: nil, alt: alt)))
                        remaining = afterClose
                    } else {
                        buf += "image:"
                        remaining = String(remaining.dropFirst(6))
                    }
                } else {
                    buf += "image:"
                    remaining = String(remaining.dropFirst(6))
                }

            // ── Cross-reference: <<anchor,text>> or <<anchor>> ─────────────
            // AsciiDoc xref: `<<section-id>>` or `<<section-id,Display Text>>`
            // The destination is `#anchor` (hash-prefixed fragment).
            } else if remaining.hasPrefix("<<") {
                let afterOpen = String(remaining.dropFirst(2))
                if let closeRange = afterOpen.range(of: ">>") {
                    let content = String(afterOpen[..<closeRange.lowerBound])
                    let afterClose = String(afterOpen[closeRange.upperBound...])
                    if !buf.isEmpty { nodes.append(.text(TextNode(value: buf))); buf = "" }
                    if let commaIdx = content.firstIndex(of: ",") {
                        // <<anchor,Display Text>>
                        let anchor = String(content[..<commaIdx])
                        let display = String(content[content.index(after: commaIdx)...])
                            .trimmingCharacters(in: .whitespaces)
                        let textNodes: [InlineNode] = [.text(TextNode(value: display))]
                        nodes.append(.link(LinkNode(destination: "#\(anchor)", title: nil, children: textNodes)))
                    } else {
                        // <<anchor>>
                        let textNodes: [InlineNode] = [.text(TextNode(value: content))]
                        nodes.append(.link(LinkNode(destination: "#\(content)", title: nil, children: textNodes)))
                    }
                    remaining = afterClose
                } else {
                    buf += "<<"
                    remaining = String(remaining.dropFirst(2))
                }

            // ── Bare HTTPS URL ─────────────────────────────────────────────
            // If followed by `[text]`, it becomes a LinkNode.
            // Otherwise, it becomes an AutolinkNode.
            } else if remaining.hasPrefix("https://") {
                if !buf.isEmpty { nodes.append(.text(TextNode(value: buf))); buf = "" }
                let (urlNode, rest) = parseBareUrl(remaining, scheme: "https://")
                nodes.append(urlNode)
                remaining = rest

            // ── Bare HTTP URL ──────────────────────────────────────────────
            } else if remaining.hasPrefix("http://") {
                if !buf.isEmpty { nodes.append(.text(TextNode(value: buf))); buf = "" }
                let (urlNode, rest) = parseBareUrl(remaining, scheme: "http://")
                nodes.append(urlNode)
                remaining = rest

            // ── Default: consume one character as plain text ───────────────
            } else {
                buf += String(remaining.first!)
                remaining = String(remaining.dropFirst())
            }
        }

        // Flush any remaining text buffer
        if !buf.isEmpty {
            nodes.append(.text(TextNode(value: buf)))
        }

        return nodes
    }

    // ── Helpers ───────────────────────────────────────────────────────────

    /// Find the first occurrence of `delimiter` in `text` and split there.
    ///
    /// Returns `(content, rest)` where `content` is everything before the
    /// delimiter and `rest` is everything after the delimiter.
    ///
    /// Returns `nil` if the delimiter is not found.
    ///
    ///     findClosing("hello*world", delimiter: "*") → ("hello", "world")
    ///     findClosing("hello world", delimiter: "*") → nil
    private static func findClosing(_ text: String, delimiter: String) -> (String, String)? {
        guard let range = text.range(of: delimiter) else { return nil }
        let content = String(text[text.startIndex..<range.lowerBound])
        let rest = String(text[range.upperBound...])
        return (content, rest)
    }

    /// Parse a bare URL (https:// or http://) that may optionally have `[text]`.
    ///
    /// URL characters: everything up to whitespace, `[`, `]`, or end of string.
    ///
    /// - If the URL is immediately followed by `[text]`, produce a `LinkNode`
    ///   with the bracketed text as the link label.
    /// - Otherwise, produce an `AutolinkNode` (isEmail: false).
    ///
    /// - Parameters:
    ///   - remaining: The string starting with the URL (including scheme).
    ///   - scheme: The scheme prefix ("https://" or "http://") for accounting.
    /// - Returns: `(inlineNode, restOfString)`.
    private static func parseBareUrl(_ remaining: String, scheme: String) -> (InlineNode, String) {
        // Collect URL characters (stop at whitespace, [, ], or common terminators)
        var url = ""
        var rest = remaining
        while !rest.isEmpty {
            let ch = rest.first!
            if ch.isWhitespace || ch == "[" || ch == "]" || ch == "," || ch == ">" {
                break
            }
            url.append(ch)
            rest = String(rest.dropFirst())
        }

        // Check if followed by [text]
        if rest.hasPrefix("["), let closeBracket = rest.firstIndex(of: "]") {
            let labelStart = rest.index(after: rest.startIndex)
            let label = String(rest[labelStart..<closeBracket])
            let afterClose = String(rest[rest.index(after: closeBracket)...])
            let textNodes: [InlineNode] = label.isEmpty
                ? [.text(TextNode(value: url))]
                : [.text(TextNode(value: label))]
            return (.link(LinkNode(destination: url, title: nil, children: textNodes)), afterClose)
        }

        // Bare URL — AutolinkNode
        return (.autolink(AutolinkNode(destination: url, isEmail: false)), rest)
    }
}
