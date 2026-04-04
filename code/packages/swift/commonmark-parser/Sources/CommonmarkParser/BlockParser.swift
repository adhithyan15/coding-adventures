// ============================================================================
// BlockParser.swift — Phase 1: CommonMark Block Structure Parser
// ============================================================================
//
// This file implements the block-level parsing phase of the CommonMark parser.
// It processes Markdown text line-by-line, identifying structural elements
// and producing `IntermediateBlock` values that are later resolved by the
// inline parser.
//
// # Intermediate Representation
//
// During block parsing, inline content is stored as raw strings rather than
// inline nodes. This is necessary because inline parsing (especially emphasis
// with the delimiter stack algorithm) requires seeing the full inline run
// before deciding how to parse it.
//
// # Block Types Implemented
//
// - ATX headings: `# text` through `###### text`
// - Thematic breaks: `---`, `***`, `___` (3+ of the same char)
// - Fenced code blocks: ` ```lang` / ` ``` ` and ` ~~~lang` / ` ~~~ `
// - Indented code blocks: 4+ leading spaces
// - Block HTML: lines starting with `<` in certain patterns
// - Blockquotes: `> text`
// - Unordered lists: `- item`, `* item`, `+ item`
// - Ordered lists: `1. item`, `1) item`
// - Blank lines: separate blocks
// - Paragraphs: everything else
//
// # List Tightness
//
// A list is "tight" if none of its items are separated by blank lines.
// Tight lists suppress `<p>` wrappers in HTML output.
//

/// An intermediate block representation used during parsing.
///
/// This enum is internal to the parser. After block parsing, `resolveInlines`
/// converts `IntermediateBlock` trees into final `BlockNode` trees.
enum IntermediateBlock {
    /// ATX or setext heading with raw inline content.
    case heading(level: Int, raw: String)
    /// Paragraph with raw inline content (may span multiple lines).
    case paragraph(raw: String)
    /// Fenced or indented code block.
    case codeBlock(language: String?, value: String)
    /// Block quotation containing nested intermediate blocks.
    case blockquote(children: [IntermediateBlock])
    /// Ordered or unordered list.
    case list(ordered: Bool, start: Int?, tight: Bool, items: [[IntermediateBlock]])
    /// Thematic break (horizontal rule).
    case thematicBreak
    /// Raw passthrough block (e.g. HTML block).
    case rawBlock(format: String, value: String)
}

// ── BlockParser ───────────────────────────────────────────────────────────────

/// Phase 1 block structure parser.
///
/// The parser processes lines sequentially using a state machine. It
/// recognizes block-level constructs and produces `IntermediateBlock` values.
enum BlockParser {

    // ── Public entry point ────────────────────────────────────────────────

    /// Parse a Markdown string into a list of intermediate block nodes.
    ///
    /// - Parameter markdown: The raw Markdown input string.
    /// - Returns: An array of `IntermediateBlock` values representing the
    ///   document structure.
    static func parseBlocks(_ markdown: String) -> [IntermediateBlock] {
        // Normalize line endings: \r\n → \n, \r → \n
        let normalized = markdown
            .replacingOccurrences(of: "\r\n", with: "\n")
            .replacingOccurrences(of: "\r", with: "\n")

        // Split into lines. Note: a trailing newline produces an empty last line,
        // which we keep so that line-based logic works correctly.
        let lines = normalized.components(separatedBy: "\n")
        return parseLines(lines)
    }

    // ── Core line-by-line parser ──────────────────────────────────────────

    /// Parse an array of lines into intermediate blocks.
    ///
    /// This is the heart of the block parser. It processes lines one-by-one
    /// using a state machine with these states:
    ///   - Idle: between blocks
    ///   - InParagraph: accumulating paragraph lines
    ///   - InFencedCode: inside a fenced code block
    ///   - InHtmlBlock: inside a raw HTML block
    ///
    static func parseLines(_ lines: [String]) -> [IntermediateBlock] {
        var blocks: [IntermediateBlock] = []
        var i = 0

        while i < lines.count {
            let line = lines[i]

            // ── Blank line ────────────────────────────────────────────────
            if line.trimmingCharacters(in: .whitespaces).isEmpty {
                i += 1
                continue
            }

            // ── ATX Heading ───────────────────────────────────────────────
            // Pattern: 1–6 `#` characters, then a space, then content.
            // The trailing `#` characters (if present) are stripped.
            if let heading = parseAtxHeading(line) {
                blocks.append(heading)
                i += 1
                continue
            }

            // ── Thematic Break ────────────────────────────────────────────
            // Pattern: 3+ of the same character (- * _), optionally with spaces.
            if isThematicBreak(line) {
                blocks.append(.thematicBreak)
                i += 1
                continue
            }

            // ── Fenced Code Block ─────────────────────────────────────────
            // Opening fence: 3+ backticks or tildes, optional info string.
            if let (fence, lang, fenceIndent) = parseFenceOpening(line) {
                let (codeLines, consumed) = collectFencedCode(
                    lines: lines,
                    startIndex: i + 1,
                    fence: fence,
                    indent: fenceIndent
                )
                let value = codeLines.map { $0 + "\n" }.joined()
                blocks.append(.codeBlock(language: lang.isEmpty ? nil : lang, value: value))
                i += consumed + 1
                continue
            }

            // ── Indented Code Block ───────────────────────────────────────
            // Pattern: 4+ leading spaces. Continues until a non-indented line.
            if line.hasPrefix("    ") || line.hasPrefix("\t") {
                let (codeLines, consumed) = collectIndentedCode(lines: lines, startIndex: i)
                let value = codeLines.map { stripIndent($0) + "\n" }.joined()
                blocks.append(.codeBlock(language: nil, value: value))
                i += consumed
                continue
            }

            // ── Blockquote ────────────────────────────────────────────────
            // Pattern: lines starting with `> `.
            if line.hasPrefix("> ") || line == ">" {
                let (bqLines, consumed) = collectBlockquoteLines(lines: lines, startIndex: i)
                let stripped = bqLines.map { stripBlockquoteMarker($0) }
                let children = parseLines(stripped)
                blocks.append(.blockquote(children: children))
                i += consumed
                continue
            }

            // ── Unordered List ────────────────────────────────────────────
            // Pattern: `- `, `* `, or `+ ` followed by content.
            if let marker = unorderedListMarker(line) {
                let (items, tight, consumed) = collectListItems(
                    lines: lines, startIndex: i,
                    isOrdered: false, marker: marker
                )
                let parsedItems = items.map { parseLines($0) }
                blocks.append(.list(ordered: false, start: nil, tight: tight, items: parsedItems))
                i += consumed
                continue
            }

            // ── Ordered List ──────────────────────────────────────────────
            // Pattern: number + `.` or `)` followed by a space and content.
            if let (startNum, marker) = orderedListMarker(line) {
                let (items, tight, consumed) = collectListItems(
                    lines: lines, startIndex: i,
                    isOrdered: true, marker: marker
                )
                let parsedItems = items.map { parseLines($0) }
                blocks.append(.list(ordered: true, start: startNum, tight: tight, items: parsedItems))
                i += consumed
                continue
            }

            // ── HTML Block ────────────────────────────────────────────────
            // A line starting with `<` that looks like an HTML tag.
            if line.hasPrefix("<") && looksLikeHtmlBlock(line) {
                let (htmlLines, consumed) = collectHtmlBlock(lines: lines, startIndex: i)
                let value = htmlLines.joined(separator: "\n") + "\n"
                blocks.append(.rawBlock(format: "html", value: value))
                i += consumed
                continue
            }

            // ── Paragraph ─────────────────────────────────────────────────
            // Everything else becomes a paragraph. Paragraphs continue until
            // a blank line or another block-level element interrupts.
            let (paraLines, consumed) = collectParagraphLines(lines: lines, startIndex: i)
            let raw = paraLines.joined(separator: "\n")
            blocks.append(.paragraph(raw: raw))
            i += consumed
        }

        return blocks
    }

    // ── ATX Heading ───────────────────────────────────────────────────────

    /// Parse an ATX heading line like `## Hello`.
    ///
    /// Rules:
    /// - 1–6 `#` characters at the start of the line
    /// - Followed by a space (or the line ends here for an empty heading)
    /// - Content may have trailing `#` characters stripped
    ///
    /// Returns `nil` if the line is not an ATX heading.
    static func parseAtxHeading(_ line: String) -> IntermediateBlock? {
        var index = line.startIndex
        var level = 0

        // Count leading `#` characters (max 6)
        while index < line.endIndex && line[index] == "#" && level < 6 {
            level += 1
            index = line.index(after: index)
        }

        guard level > 0 else { return nil }

        // Must be followed by a space (or end of line for empty heading)
        if index < line.endIndex {
            guard line[index] == " " || line[index] == "\t" else { return nil }
            // Skip the space
            index = line.index(after: index)
        }

        // Remaining text is the heading content
        var content = String(line[index...])

        // Strip optional trailing `#` characters and the space before them
        // e.g. `## Hello ##` → `Hello`
        content = stripTrailingHashes(content)

        return .heading(level: level, raw: content)
    }

    /// Strip trailing hashes from a heading content string.
    ///
    /// According to the CommonMark spec, trailing `#` characters (preceded by
    /// at least one space) are stripped from ATX heading content.
    ///
    ///   "Hello ##"  → "Hello"
    ///   "Hello"     → "Hello"
    ///   "Hello #"   → "Hello"
    private static func stripTrailingHashes(_ s: String) -> String {
        var result = s
        // Trim trailing whitespace first
        result = result.trimmingCharacters(in: .init(charactersIn: " \t"))
        // Check if ends with one or more `#` characters
        if result.hasSuffix("#") {
            // Walk back past hashes
            var end = result.endIndex
            while end > result.startIndex {
                let prev = result.index(before: end)
                if result[prev] == "#" {
                    end = prev
                } else {
                    break
                }
            }
            // If the character before the hashes is a space, strip both
            if end > result.startIndex {
                let prev = result.index(before: end)
                if result[prev] == " " || result[prev] == "\t" {
                    result = String(result[..<prev])
                }
            } else {
                // All hashes — empty heading
                result = ""
            }
        }
        return result
    }

    // ── Thematic Break ────────────────────────────────────────────────────

    /// Determine if a line is a thematic break.
    ///
    /// Rules (CommonMark spec §4.1):
    /// - Three or more of the same character: `-`, `*`, or `_`
    /// - Spaces are allowed between the characters
    /// - No other characters allowed (except leading up to 3 spaces)
    ///
    ///   "---"      → true
    ///   "- - -"    → true
    ///   "***"      → true
    ///   "__ __"    → true
    ///   "--"       → false (only 2)
    ///   "---a"     → false (other character)
    static func isThematicBreak(_ line: String) -> Bool {
        let trimmed = line.trimmingCharacters(in: .init(charactersIn: " \t"))
        guard !trimmed.isEmpty else { return false }

        let first = trimmed.first!
        guard first == "-" || first == "*" || first == "_" else { return false }

        var count = 0
        for ch in trimmed {
            if ch == first { count += 1 }
            else if ch == " " || ch == "\t" { /* allowed */ }
            else { return false }
        }
        return count >= 3
    }

    // ── Fenced Code Block ─────────────────────────────────────────────────

    /// Parse the opening fence of a fenced code block.
    ///
    /// Returns `(fenceChar, infoString, indent)` or `nil` if not a fence.
    ///
    /// The fence is 3+ backtick or tilde characters. The info string (language)
    /// follows on the same line. The indent is the number of leading spaces
    /// (0–3) that must be stripped from subsequent lines.
    static func parseFenceOpening(_ line: String) -> (Character, String, Int)? {
        // Count leading spaces (0-3)
        var indent = 0
        var start = line.startIndex
        while start < line.endIndex && (line[start] == " " || line[start] == "\t") && indent < 4 {
            indent += 1
            start = line.index(after: start)
        }

        guard start < line.endIndex else { return nil }
        let fenceChar = line[start]
        guard fenceChar == "`" || fenceChar == "~" else { return nil }

        // Count fence characters
        var fenceEnd = start
        var fenceLen = 0
        while fenceEnd < line.endIndex && line[fenceEnd] == fenceChar {
            fenceLen += 1
            fenceEnd = line.index(after: fenceEnd)
        }
        guard fenceLen >= 3 else { return nil }

        // Backtick fence: info string must not contain backticks
        let info = String(line[fenceEnd...]).trimmingCharacters(in: .whitespaces)
        if fenceChar == "`" && info.contains("`") { return nil }

        // Take first word of info string as the language
        let lang = info.components(separatedBy: .whitespaces).first ?? ""

        return (fenceChar, lang, indent)
    }

    /// Collect lines inside a fenced code block.
    ///
    /// Reads lines until a closing fence (same character, same or greater length)
    /// is found, or end of input. Returns the code lines and the number of
    /// lines consumed (including the closing fence, if present).
    static func collectFencedCode(
        lines: [String],
        startIndex: Int,
        fence: Character,
        indent: Int
    ) -> ([String], Int) {
        var codeLines: [String] = []
        var i = startIndex

        while i < lines.count {
            let line = lines[i]
            // Check for closing fence: same character, 3+ repetitions
            let trimmed = line.trimmingCharacters(in: .init(charactersIn: " \t"))
            if trimmed.allSatisfy({ $0 == fence }) && trimmed.count >= 3 {
                // Closing fence found; consume it
                return (codeLines, i - startIndex + 1)
            }
            // Strip up to `indent` leading spaces from code lines
            codeLines.append(stripLeadingSpaces(line, count: indent))
            i += 1
        }

        // No closing fence — consume remaining lines as code
        return (codeLines, i - startIndex)
    }

    // ── Indented Code Block ───────────────────────────────────────────────

    /// Collect lines for an indented code block (4-space indent).
    ///
    /// An indented code block continues as long as lines are either:
    /// - Indented by 4+ spaces, or
    /// - Blank lines (which are included only if followed by more indented lines)
    static func collectIndentedCode(lines: [String], startIndex: Int) -> ([String], Int) {
        var codeLines: [String] = []
        var i = startIndex
        var trailingBlanks: [String] = []

        while i < lines.count {
            let line = lines[i]
            if line.trimmingCharacters(in: .whitespaces).isEmpty {
                trailingBlanks.append(line)
                i += 1
            } else if line.hasPrefix("    ") || line.hasPrefix("\t") {
                codeLines.append(contentsOf: trailingBlanks)
                trailingBlanks = []
                codeLines.append(line)
                i += 1
            } else {
                break
            }
        }

        return (codeLines, i - startIndex)
    }

    /// Strip the 4-space indentation from an indented code block line.
    static func stripIndent(_ line: String) -> String {
        if line.hasPrefix("    ") {
            return String(line.dropFirst(4))
        } else if line.hasPrefix("\t") {
            return String(line.dropFirst(1))
        }
        return line
    }

    // ── Blockquote ────────────────────────────────────────────────────────

    /// Collect consecutive blockquote lines (starting with `> `).
    static func collectBlockquoteLines(lines: [String], startIndex: Int) -> ([String], Int) {
        var bqLines: [String] = []
        var i = startIndex

        while i < lines.count {
            let line = lines[i]
            if line.hasPrefix("> ") || line == ">" || line.hasPrefix(">") {
                bqLines.append(line)
                i += 1
            } else if line.trimmingCharacters(in: .whitespaces).isEmpty {
                // Blank line ends the blockquote
                break
            } else {
                // Lazy continuation: non-empty lines can continue a blockquote
                // For simplicity, we stop at a non-quote line
                break
            }
        }

        return (bqLines, i - startIndex)
    }

    /// Remove the `> ` (or `>`) prefix from a blockquote line.
    static func stripBlockquoteMarker(_ line: String) -> String {
        if line.hasPrefix("> ") {
            return String(line.dropFirst(2))
        } else if line.hasPrefix(">") {
            return String(line.dropFirst(1))
        }
        return line
    }

    // ── List Parsing ──────────────────────────────────────────────────────

    /// Extract the unordered list marker from a line, if present.
    ///
    /// Returns the marker character (`-`, `*`, `+`) if the line starts with
    /// one of these followed by a space.
    static func unorderedListMarker(_ line: String) -> Character? {
        let stripped = line.trimmingCharacters(in: .init(charactersIn: " \t"))
        guard stripped.count >= 2 else { return nil }
        let first = stripped.first!
        let second = stripped[stripped.index(after: stripped.startIndex)]
        if (first == "-" || first == "*" || first == "+") && (second == " " || second == "\t") {
            return first
        }
        return nil
    }

    /// Extract the ordered list start number and marker from a line, if present.
    ///
    /// Returns `(startNumber, markerChar)` where markerChar is `.` or `)`.
    static func orderedListMarker(_ line: String) -> (Int, Character)? {
        let stripped = line.trimmingCharacters(in: .init(charactersIn: " \t"))

        // Scan digits
        var digits = ""
        var index = stripped.startIndex
        while index < stripped.endIndex && stripped[index].isNumber {
            digits.append(stripped[index])
            index = stripped.index(after: index)
        }

        guard !digits.isEmpty, digits.count <= 9 else { return nil }
        guard index < stripped.endIndex else { return nil }

        let marker = stripped[index]
        guard marker == "." || marker == ")" else { return nil }

        // Must be followed by a space
        let afterMarker = stripped.index(after: index)
        guard afterMarker < stripped.endIndex else { return nil }
        guard stripped[afterMarker] == " " || stripped[afterMarker] == "\t" else { return nil }

        return (Int(digits) ?? 1, marker)
    }

    /// Collect all items of a list into an array of line arrays.
    ///
    /// Each inner array is the lines belonging to one list item (without the
    /// list marker). Detects tight vs. loose lists based on blank line presence.
    ///
    /// Returns `(items, isTight, linesConsumed)`.
    static func collectListItems(
        lines: [String],
        startIndex: Int,
        isOrdered: Bool,
        marker: Character
    ) -> ([[String]], Bool, Int) {
        var items: [[String]] = []
        var i = startIndex
        var hadBlankLine = false
        var hadBlankBetweenItems = false

        while i < lines.count {
            let line = lines[i]

            // Check if this line starts a new list item with the same marker
            let isNewItem: Bool
            if isOrdered {
                if let (_, m) = orderedListMarker(line), m == marker { isNewItem = true }
                else { isNewItem = false }
            } else {
                if let m = unorderedListMarker(line), m == marker { isNewItem = true }
                else { isNewItem = false }
            }

            if isNewItem {
                if !items.isEmpty && hadBlankLine {
                    hadBlankBetweenItems = true
                }
                hadBlankLine = false
                // Extract the item content (after the marker)
                let content = stripListMarker(line, isOrdered: isOrdered)
                var itemLines: [String] = [content]
                i += 1

                // Collect continuation lines (indented or blank)
                while i < lines.count {
                    let contLine = lines[i]
                    if contLine.trimmingCharacters(in: .whitespaces).isEmpty {
                        hadBlankLine = true
                        itemLines.append("")
                        i += 1
                    } else if contLine.hasPrefix("  ") || contLine.hasPrefix("\t") {
                        // Continuation: strip leading whitespace
                        itemLines.append(stripListContinuation(contLine))
                        i += 1
                    } else {
                        // Not a continuation — check if it's the next list item
                        break
                    }
                }

                items.append(itemLines)
            } else if line.trimmingCharacters(in: .whitespaces).isEmpty {
                hadBlankLine = true
                i += 1
            } else {
                // Non-item, non-blank line: end of list
                break
            }
        }

        let tight = !hadBlankBetweenItems
        return (items, tight, i - startIndex)
    }

    /// Strip the list marker from a list item line.
    ///
    /// For `- item` returns `item`. For `1. item` returns `item`.
    static func stripListMarker(_ line: String, isOrdered: Bool) -> String {
        let stripped = line.trimmingCharacters(in: .init(charactersIn: " \t"))
        if isOrdered {
            // Find the `.` or `)` marker
            if let dotRange = stripped.range(of: ". ") {
                return String(stripped[dotRange.upperBound...])
            }
            if let parenRange = stripped.range(of: ") ") {
                return String(stripped[parenRange.upperBound...])
            }
        } else {
            // `- `, `* `, `+ `
            if stripped.count >= 2 {
                return String(stripped.dropFirst(2))
            }
        }
        return stripped
    }

    /// Strip leading list continuation indentation (2+ spaces or 1 tab).
    static func stripListContinuation(_ line: String) -> String {
        if line.hasPrefix("    ") { return String(line.dropFirst(4)) }
        if line.hasPrefix("  ") { return String(line.dropFirst(2)) }
        if line.hasPrefix("\t") { return String(line.dropFirst(1)) }
        return line
    }

    // ── HTML Block ────────────────────────────────────────────────────────

    /// Determine if a line looks like the start of an HTML block.
    ///
    /// For simplicity, we treat any line starting with `<` followed by a
    /// letter or `/` as an HTML block start.
    static func looksLikeHtmlBlock(_ line: String) -> Bool {
        guard line.hasPrefix("<") else { return false }
        let rest = line.dropFirst()
        guard let first = rest.first else { return false }
        return first.isLetter || first == "/" || first == "!" || first == "?"
    }

    /// Collect lines for an HTML block.
    ///
    /// HTML blocks continue until a blank line (for most block types).
    static func collectHtmlBlock(lines: [String], startIndex: Int) -> ([String], Int) {
        var htmlLines: [String] = []
        var i = startIndex

        while i < lines.count {
            let line = lines[i]
            if line.trimmingCharacters(in: .whitespaces).isEmpty {
                // Blank line ends the HTML block
                break
            }
            htmlLines.append(line)
            i += 1
        }

        return (htmlLines, i - startIndex)
    }

    // ── Paragraph ─────────────────────────────────────────────────────────

    /// Collect consecutive paragraph lines.
    ///
    /// A paragraph continues until:
    /// - A blank line
    /// - An ATX heading
    /// - A thematic break
    /// - A fenced code block opening
    /// - A blockquote marker
    /// - A list marker (unordered or ordered)
    static func collectParagraphLines(lines: [String], startIndex: Int) -> ([String], Int) {
        var paraLines: [String] = []
        var i = startIndex

        while i < lines.count {
            let line = lines[i]

            // Blank line terminates the paragraph
            if line.trimmingCharacters(in: .whitespaces).isEmpty {
                break
            }
            // Another block-level element terminates the paragraph
            if parseAtxHeading(line) != nil { break }
            if isThematicBreak(line) { break }
            if parseFenceOpening(line) != nil { break }
            if line.hasPrefix("> ") || line == ">" { break }
            if unorderedListMarker(line) != nil { break }
            if orderedListMarker(line) != nil { break }

            paraLines.append(line)
            i += 1
        }

        return (paraLines, i - startIndex)
    }

    // ── Utilities ─────────────────────────────────────────────────────────

    /// Strip up to `count` leading spaces from a line.
    static func stripLeadingSpaces(_ line: String, count: Int) -> String {
        var result = line
        var stripped = 0
        while stripped < count && result.hasPrefix(" ") {
            result = String(result.dropFirst(1))
            stripped += 1
        }
        return result
    }
}
