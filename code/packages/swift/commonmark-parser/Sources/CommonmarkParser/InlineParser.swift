// ============================================================================
// InlineParser.swift — Phase 2: CommonMark Inline Content Parser
// ============================================================================
//
// This file implements the inline parsing phase of the CommonMark parser.
// It converts raw inline strings (produced by BlockParser) into arrays of
// `InlineNode` values.
//
// # What is Parsed
//
// - `**bold**` and `__bold__` → StrongNode
// - `*italic*` and `_italic_` → EmphasisNode
// - `` `code` `` → CodeSpanNode
// - `[text](url "title")` → LinkNode
// - `![alt](url "title")` → ImageNode
// - `<https://url>` → AutolinkNode (isEmail: false)
// - `<user@email>` → AutolinkNode (isEmail: true)
// - Two trailing spaces + newline → HardBreakNode
// - Newline → SoftBreakNode
// - Everything else → TextNode
//
// # Algorithm
//
// The parser uses a character-by-character scan with lookahead. It uses a
// simple recursive descent approach rather than the full CommonMark delimiter
// stack algorithm. This covers the vast majority of CommonMark inline content
// and is much simpler to implement and understand.
//
// For emphasis/strong, the parser tries to match the longest possible run:
//   - `**` before `*` (strong takes priority if both could match)
//   - `__` before `_`
//
// # Limitations vs. Full CommonMark
//
// - Nested emphasis edge cases (e.g. `*foo **bar** baz*`) may differ slightly
//   from the full spec implementation
// - HTML entities are not decoded (they're left as-is)
// - Link reference definitions are not supported
//

import DocumentAst

// ── InlineParser ──────────────────────────────────────────────────────────────

/// Phase 2 inline content parser.
///
/// The parser scans a raw inline string character by character, recognizing
/// inline constructs and producing `InlineNode` values.
enum InlineParser {

    // ── Public entry point ────────────────────────────────────────────────

    /// Parse a raw inline string into an array of inline nodes.
    ///
    /// - Parameter raw: The raw inline content string (e.g., the content of
    ///   a paragraph or heading, as produced by BlockParser).
    /// - Returns: An array of `InlineNode` values.
    static func parse(_ raw: String) -> [InlineNode] {
        var scanner = InlineScanner(raw)
        return scanner.parseAll()
    }
}

// ── InlineScanner ─────────────────────────────────────────────────────────────

/// A character-by-character scanner for inline Markdown content.
///
/// The scanner maintains a current position and accumulates a text buffer
/// for plain text runs. Special characters trigger attempts to parse
/// inline constructs.
struct InlineScanner {
    /// The raw input string as an array of characters for random access.
    private let chars: [Character]
    /// Current position in `chars`.
    private var pos: Int
    /// Buffer for accumulating plain text characters.
    private var textBuffer: String

    init(_ input: String) {
        // Normalize line endings in inline content
        let normalized = input
            .replacingOccurrences(of: "\r\n", with: "\n")
            .replacingOccurrences(of: "\r", with: "\n")
        self.chars = Array(normalized)
        self.pos = 0
        self.textBuffer = ""
    }

    // ── Main parse loop ───────────────────────────────────────────────────

    /// Parse all inline content and return the resulting nodes.
    mutating func parseAll() -> [InlineNode] {
        var nodes: [InlineNode] = []

        while pos < chars.count {
            let ch = chars[pos]

            switch ch {
            case "`":
                // Code span: `code` or ``code``
                flushText(into: &nodes)
                if let node = tryCodeSpan() {
                    nodes.append(node)
                } else {
                    textBuffer.append(ch)
                    pos += 1
                }

            case "*", "_":
                // Emphasis or strong: *text*, **text**, _text_, __text__
                flushText(into: &nodes)
                if let node = tryEmphasisOrStrong() {
                    nodes.append(contentsOf: node)
                } else {
                    textBuffer.append(ch)
                    pos += 1
                }

            case "~":
                // Strikethrough (GFM): ~~text~~
                flushText(into: &nodes)
                if let node = tryStrikethrough() {
                    nodes.append(node)
                } else {
                    textBuffer.append(ch)
                    pos += 1
                }

            case "[":
                // Link: [text](url) or Image: ![alt](url)
                flushText(into: &nodes)
                if let node = tryLink() {
                    nodes.append(node)
                } else {
                    textBuffer.append(ch)
                    pos += 1
                }

            case "!":
                // Image: ![alt](url) — only special when followed by `[`.
                // If `!` is not followed by `[` we treat it as plain text without
                // flushing, so that "Hello!" stays as a single TextNode.
                if pos + 1 < chars.count && chars[pos + 1] == "[" {
                    flushText(into: &nodes)
                    if let node = tryImage() {
                        nodes.append(node)
                    } else {
                        textBuffer.append(ch)
                        pos += 1
                    }
                } else {
                    textBuffer.append(ch)
                    pos += 1
                }

            case "<":
                // Autolink: <https://url> or <user@email>
                flushText(into: &nodes)
                if let node = tryAutolink() {
                    nodes.append(node)
                } else {
                    textBuffer.append(ch)
                    pos += 1
                }

            case "\\":
                // Backslash escape: \* → literal *
                if pos + 1 < chars.count {
                    let next = chars[pos + 1]
                    if isAsciiPunctuation(next) {
                        textBuffer.append(next)
                        pos += 2
                    } else if next == "\n" {
                        // Backslash before newline → hard break
                        flushText(into: &nodes)
                        nodes.append(.hardBreak)
                        pos += 2
                    } else {
                        textBuffer.append(ch)
                        pos += 1
                    }
                } else {
                    textBuffer.append(ch)
                    pos += 1
                }

            case "\n":
                // Soft or hard break
                flushText(into: &nodes)
                // Two trailing spaces before \n → hard break
                // Check the last characters of the buffer (already flushed)
                // We detect trailing spaces in the text buffer before flushing
                // Here we just emit a soft break; hard breaks are handled above.
                nodes.append(.softBreak)
                pos += 1

            default:
                textBuffer.append(ch)
                pos += 1
            }
        }

        flushText(into: &nodes)
        return nodes
    }

    // ── Flush text buffer ─────────────────────────────────────────────────

    /// Flush the accumulated text buffer into a TextNode and append it.
    ///
    /// Two trailing spaces before a newline signal a hard break. We check
    /// for this pattern and emit a HardBreakNode instead of a SoftBreakNode.
    private mutating func flushText(into nodes: inout [InlineNode]) {
        guard !textBuffer.isEmpty else { return }

        // Check for trailing "  " before flush — signals hard break
        // This handles the case where we just flushed before a \n
        if textBuffer.hasSuffix("  ") {
            let trimmed = String(textBuffer.dropLast(2))
            if !trimmed.isEmpty {
                nodes.append(.text(TextNode(value: trimmed)))
            }
            nodes.append(.hardBreak)
        } else {
            nodes.append(.text(TextNode(value: textBuffer)))
        }
        textBuffer = ""
    }

    // ── Code Span ─────────────────────────────────────────────────────────

    /// Try to parse a code span starting at the current position.
    ///
    /// Code spans use backtick strings as delimiters:
    ///   `` `code` `` — single backtick
    ///   ``` ``code`` ``` — double backtick (allows single backticks inside)
    ///
    /// Rules:
    /// - Count opening backticks
    /// - Find matching closing backtick string of the same length
    /// - Strip one leading and one trailing space if both present
    mutating func tryCodeSpan() -> InlineNode? {
        let start = pos
        var openCount = 0

        // Count opening backticks
        while pos < chars.count && chars[pos] == "`" {
            openCount += 1
            pos += 1
        }

        // Scan for closing backticks of the same length
        var content = ""
        while pos < chars.count {
            if chars[pos] == "`" {
                // Count closing backticks
                var closeCount = 0
                let closeStart = pos
                while pos < chars.count && chars[pos] == "`" {
                    closeCount += 1
                    pos += 1
                }
                if closeCount == openCount {
                    // Normalize: strip one leading and trailing space if present
                    var normalized = content
                    if normalized.hasPrefix(" ") && normalized.hasSuffix(" ") && normalized.count > 2 {
                        normalized = String(normalized.dropFirst().dropLast())
                    }
                    return .codeSpan(CodeSpanNode(value: normalized))
                } else {
                    // Not a match — add the backticks to content
                    content += String(repeating: "`", count: closeCount)
                }
            } else if chars[pos] == "\n" {
                content += " " // Newlines in code spans become spaces
                pos += 1
            } else {
                content.append(chars[pos])
                pos += 1
            }
        }

        // No closing backticks found — restore position and fail
        pos = start
        return nil
    }

    // ── Emphasis and Strong ───────────────────────────────────────────────

    /// Try to parse emphasis (`*text*`, `_text_`) or strong (`**text**`, `__text__`).
    ///
    /// Returns an array because we might return multiple nodes (e.g., if
    /// we consume `**` but only close `*`, we return a strong + content).
    ///
    /// Algorithm:
    /// 1. Count opening delimiter characters (`*` or `_`)
    /// 2. If 2+, try to match as strong first (looking for `**` close)
    /// 3. If 1, try to match as emphasis (looking for `*` close)
    /// 4. Fall back to literal text if no match
    mutating func tryEmphasisOrStrong() -> [InlineNode]? {
        let delimChar = chars[pos]
        let start = pos
        var delimCount = 0

        while pos < chars.count && chars[pos] == delimChar {
            delimCount += 1
            pos += 1
        }

        // Check left-flanking requirements:
        // After the delimiter, must not be whitespace (for `_`, also must not be preceded by alphanumeric)
        if pos >= chars.count || chars[pos] == " " || chars[pos] == "\n" {
            pos = start
            return nil
        }

        // Try to match strong (** or __) first
        if delimCount >= 2 {
            let innerStart = pos
            if let (innerNodes, closePos) = findEmphasisClose(
                from: pos,
                delimChar: delimChar,
                delimCount: 2
            ) {
                pos = closePos
                let inner = innerNodes
                let node = InlineNode.strong(StrongNode(children: inner))
                if delimCount > 2 {
                    // Extra delimiters become literal text
                    let extra = String(repeating: String(delimChar), count: delimCount - 2)
                    return [.text(TextNode(value: extra)), node]
                }
                return [node]
            }
            pos = innerStart
        }

        // Try to match emphasis (single * or _)
        if delimCount >= 1 {
            let innerStart = pos
            if let (innerNodes, closePos) = findEmphasisClose(
                from: pos,
                delimChar: delimChar,
                delimCount: 1
            ) {
                pos = closePos
                let node = InlineNode.emphasis(EmphasisNode(children: innerNodes))
                if delimCount > 1 {
                    let extra = String(repeating: String(delimChar), count: delimCount - 1)
                    return [.text(TextNode(value: extra)), node]
                }
                return [node]
            }
            pos = innerStart
        }

        // No match found
        pos = start
        return nil
    }

    /// Find the closing delimiter and parse the inner content.
    ///
    /// Recursively parses the inner content (allowing nested emphasis).
    /// Returns `(innerNodes, posAfterClose)` or `nil` if no close found.
    private func findEmphasisClose(
        from startPos: Int,
        delimChar: Character,
        delimCount: Int
    ) -> ([InlineNode], Int)? {
        var scanPos = startPos
        var innerText = ""

        while scanPos < chars.count {
            let ch = chars[scanPos]

            // Check for closing delimiter
            if ch == delimChar {
                var closeCount = 0
                let closeStart = scanPos
                while scanPos < chars.count && chars[scanPos] == delimChar {
                    closeCount += 1
                    scanPos += 1
                }

                if closeCount >= delimCount {
                    // Found closing delimiter
                    // The inner text is parsed recursively
                    var innerScanner = InlineScanner(innerText)
                    let innerNodes = innerScanner.parseAll()
                    // If more delimiters than needed, back up
                    let extra = closeCount - delimCount
                    let finalPos = closeStart + delimCount
                    _ = extra // extra delimiters handled by caller if needed
                    return (innerNodes, closeStart + delimCount)
                } else {
                    // Not enough — treat as literal
                    innerText += String(repeating: String(delimChar), count: closeCount)
                }
            } else if ch == "`" {
                // Skip over code spans
                var btCount = 0
                while scanPos < chars.count && chars[scanPos] == "`" {
                    btCount += 1
                    scanPos += 1
                }
                innerText += String(repeating: "`", count: btCount)
            } else if ch == "\\" && scanPos + 1 < chars.count {
                let next = chars[scanPos + 1]
                if isAsciiPunctuation(next) {
                    innerText.append(next)
                    scanPos += 2
                } else {
                    innerText.append(ch)
                    scanPos += 1
                }
            } else {
                innerText.append(ch)
                scanPos += 1
            }
        }

        return nil // No closing delimiter found
    }

    // ── Strikethrough ─────────────────────────────────────────────────────

    /// Try to parse GFM strikethrough `~~text~~`.
    mutating func tryStrikethrough() -> InlineNode? {
        guard pos + 1 < chars.count && chars[pos] == "~" && chars[pos + 1] == "~" else {
            return nil
        }
        let start = pos
        pos += 2 // Skip ~~

        var content = ""
        while pos < chars.count {
            if chars[pos] == "~" && pos + 1 < chars.count && chars[pos + 1] == "~" {
                pos += 2
                var innerScanner = InlineScanner(content)
                let innerNodes = innerScanner.parseAll()
                return .strikethrough(StrikethroughNode(children: innerNodes))
            }
            content.append(chars[pos])
            pos += 1
        }

        // No closing ~~ found
        pos = start
        return nil
    }

    // ── Links ─────────────────────────────────────────────────────────────

    /// Try to parse a link `[text](url "title")` starting at `[`.
    mutating func tryLink() -> InlineNode? {
        let start = pos
        guard chars[pos] == "[" else { return nil }
        pos += 1

        guard let (text, afterText) = scanBracketContent(from: pos) else {
            pos = start
            return nil
        }
        pos = afterText

        // Expect `(`
        guard pos < chars.count && chars[pos] == "(" else {
            pos = start
            return nil
        }
        pos += 1

        guard let (dest, title, afterParen) = scanLinkDestinationAndTitle(from: pos) else {
            pos = start
            return nil
        }
        pos = afterParen

        // Parse text as inline nodes
        var textScanner = InlineScanner(text)
        let textNodes = textScanner.parseAll()

        return .link(LinkNode(destination: dest, title: title, children: textNodes))
    }

    /// Try to parse an image `![alt](url "title")` starting at `!`.
    mutating func tryImage() -> InlineNode? {
        let start = pos
        guard pos + 1 < chars.count && chars[pos] == "!" && chars[pos + 1] == "[" else {
            return nil
        }
        pos += 1 // Skip `!`

        guard chars[pos] == "[" else {
            pos = start
            return nil
        }
        pos += 1

        guard let (alt, afterAlt) = scanBracketContent(from: pos) else {
            pos = start
            return nil
        }
        pos = afterAlt

        guard pos < chars.count && chars[pos] == "(" else {
            pos = start
            return nil
        }
        pos += 1

        guard let (dest, title, afterParen) = scanLinkDestinationAndTitle(from: pos) else {
            pos = start
            return nil
        }
        pos = afterParen

        return .image(ImageNode(destination: dest, title: title, alt: alt))
    }

    /// Scan the content between `[` and `]`.
    ///
    /// Handles nested brackets. Returns `(content, posAfterBracket)` or `nil`.
    private func scanBracketContent(from startPos: Int) -> (String, Int)? {
        var scanPos = startPos
        var content = ""
        var depth = 0

        while scanPos < chars.count {
            let ch = chars[scanPos]
            if ch == "[" {
                depth += 1
                content.append(ch)
                scanPos += 1
            } else if ch == "]" {
                if depth == 0 {
                    scanPos += 1
                    return (content, scanPos)
                }
                depth -= 1
                content.append(ch)
                scanPos += 1
            } else {
                content.append(ch)
                scanPos += 1
            }
        }

        return nil
    }

    /// Scan a link destination and optional title between `(` and `)`.
    ///
    /// Formats:
    ///   `(url)`
    ///   `(url "title")`
    ///   `(url 'title')`
    ///   `(<url> "title")`
    ///
    /// Returns `(destination, title?, posAfterParen)` or `nil`.
    private func scanLinkDestinationAndTitle(from startPos: Int) -> (String, String?, Int)? {
        var scanPos = startPos

        // Skip leading whitespace
        while scanPos < chars.count && chars[scanPos] == " " { scanPos += 1 }

        // Parse destination
        var dest = ""
        if scanPos < chars.count && chars[scanPos] == "<" {
            // Angle-bracket destination
            scanPos += 1
            while scanPos < chars.count && chars[scanPos] != ">" {
                dest.append(chars[scanPos])
                scanPos += 1
            }
            guard scanPos < chars.count else { return nil }
            scanPos += 1 // Skip >
        } else {
            // Regular destination: scan until space, ), or end
            var parenDepth = 0
            while scanPos < chars.count {
                let ch = chars[scanPos]
                if ch == "(" { parenDepth += 1; dest.append(ch); scanPos += 1 }
                else if ch == ")" {
                    if parenDepth == 0 { break }
                    parenDepth -= 1; dest.append(ch); scanPos += 1
                }
                else if ch == " " || ch == "\n" { break }
                else { dest.append(ch); scanPos += 1 }
            }
        }

        // Skip whitespace before optional title
        while scanPos < chars.count && (chars[scanPos] == " " || chars[scanPos] == "\t") {
            scanPos += 1
        }

        // Parse optional title
        var title: String? = nil
        if scanPos < chars.count && (chars[scanPos] == "\"" || chars[scanPos] == "'" || chars[scanPos] == "(") {
            let openQuote = chars[scanPos]
            let closeQuote: Character = openQuote == "(" ? ")" : openQuote
            scanPos += 1
            var titleContent = ""
            while scanPos < chars.count && chars[scanPos] != closeQuote {
                titleContent.append(chars[scanPos])
                scanPos += 1
            }
            if scanPos < chars.count { scanPos += 1 } // Skip closing quote
            title = titleContent
        }

        // Skip whitespace before `)`
        while scanPos < chars.count && chars[scanPos] == " " { scanPos += 1 }

        // Expect closing `)`
        guard scanPos < chars.count && chars[scanPos] == ")" else { return nil }
        scanPos += 1

        return (dest, title, scanPos)
    }

    // ── Autolinks ─────────────────────────────────────────────────────────

    /// Try to parse an autolink `<url>` or `<email>`.
    ///
    /// URL autolinks: `<scheme://...>` — any non-space, non-> characters after `<`.
    /// Email autolinks: `<local@domain>` — contains exactly one `@`.
    mutating func tryAutolink() -> InlineNode? {
        let start = pos
        guard chars[pos] == "<" else { return nil }
        pos += 1

        var content = ""
        while pos < chars.count && chars[pos] != ">" && chars[pos] != " " && chars[pos] != "\n" {
            content.append(chars[pos])
            pos += 1
        }

        guard pos < chars.count && chars[pos] == ">" else {
            pos = start
            return nil
        }
        pos += 1 // Skip >

        // Must have content
        guard !content.isEmpty else {
            pos = start
            return nil
        }

        // Check if it's an email (exactly one @, no spaces)
        let atCount = content.filter { $0 == "@" }.count
        if atCount == 1 && !content.hasPrefix("@") && !content.hasSuffix("@") {
            // Looks like an email
            return .autolink(AutolinkNode(destination: content, isEmail: true))
        }

        // Check if it's a URL (has a scheme: letters + colon)
        if let colonIdx = content.firstIndex(of: ":") {
            let scheme = String(content[..<colonIdx])
            if !scheme.isEmpty && scheme.allSatisfy({ $0.isLetter }) {
                return .autolink(AutolinkNode(destination: content, isEmail: false))
            }
        }

        // Not a valid autolink
        pos = start
        return nil
    }

    // ── Utilities ─────────────────────────────────────────────────────────

    /// Returns `true` if the character is an ASCII punctuation character.
    ///
    /// These characters can be backslash-escaped in CommonMark:
    /// `! " # $ % & ' ( ) * + , - . / : ; < = > ? @ [ \ ] ^ _ ` { | } ~`
    private func isAsciiPunctuation(_ ch: Character) -> Bool {
        let punctuation = "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~"
        return punctuation.contains(ch)
    }
}
