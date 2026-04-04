// ============================================================================
// BlockParser.swift — AsciiDoc Phase 1: Block Structure Parser
// ============================================================================
//
// Processes AsciiDoc source text line-by-line using a state machine. Produces
// fully resolved `BlockNode` values (Phase 2 inline parsing is invoked inline
// as blocks are flushed).
//
// # AsciiDoc Block Constructs Implemented
//
// - Section headings:  `= Title` (h1), `== Section` (h2), … `====== h6`
// - Thematic breaks:   `'''` (three or more single quotes)
// - Comments:          `//` line prefix — silently dropped
// - Attribute list:    `[source,lang]` — sets language for the next code block
// - Listing block:     `----` delimiter fence (4+ dashes) → CodeBlockNode
// - Literal block:     `....` delimiter fence (4+ dots) → CodeBlockNode(language:nil)
// - Passthrough block: `++++` → RawBlockNode(format:"html")
// - Quote block:       `____` delimiter fence (4+ underscores) → BlockquoteNode
// - Unordered list:    `* item`, `** item` (up to 6 levels)
// - Ordered list:      `. item`, `.. item` (up to 6 levels)
// - Paragraph:         any other non-blank text
// - Blank lines:       separate blocks
//
// # State Machine
//
// The parser tracks a `ParseMode` enum to know what kind of block it is
// currently accumulating. When the mode changes (blank line after paragraph,
// closing delimiter found, etc.), `flush()` converts accumulated state into a
// final `BlockNode` and appends it to the output.
//
// # Inline Resolution
//
// Block content that contains inline markup (paragraphs, headings) is parsed
// by `InlineParser.parse(_:)` during flush. Code blocks and raw blocks emit
// their content verbatim (no inline parsing).
//

import DocumentAst

// ── Parse Mode ────────────────────────────────────────────────────────────────

/// The current state of the block-level parser.
///
/// Each mode corresponds to a kind of AsciiDoc block being accumulated.
/// The parser transitions between modes as it processes lines.
private enum ParseMode: Equatable {
    /// Not inside any block — looking for the next block start.
    case normal
    /// Accumulating paragraph lines (inline content).
    case paragraph
    /// Inside a `----` listing (code) block fence.
    case codeBlock
    /// Inside a `....` literal block fence.
    case literalBlock
    /// Inside a `++++` passthrough block fence.
    case passthroughBlock
    /// Inside a `____` quote block fence.
    case quoteBlock
    /// Accumulating unordered list items.
    case unorderedList
    /// Accumulating ordered list items.
    case orderedList
}

// ── Parse State ───────────────────────────────────────────────────────────────

/// Mutable state carried through the line-by-line parser loop.
///
/// Each field accumulates data for the block currently being parsed.
/// When a block boundary is reached, `flush()` consumes the state and
/// produces a `BlockNode`.
private struct ParseState {
    /// Current parse mode (what kind of block we're in).
    var mode: ParseMode = .normal

    /// For code blocks: the language tag set by `[source,lang]`, or nil.
    var pendingLanguage: String? = nil

    /// Lines accumulated for the current block (paragraph, code, etc.).
    var currentLines: [String] = []

    /// For list modes: the raw items collected so far.
    /// Each tuple is `(level, rawText)` where level 1 = `*` or `.`.
    var listItems: [(level: Int, text: String)] = []

    /// Finished blocks produced so far.
    var blocks: [BlockNode] = []
}

// ── BlockParser ───────────────────────────────────────────────────────────────

/// Phase 1 AsciiDoc block structure parser.
///
/// Use `BlockParser.parseBlocks(_:)` to parse a full AsciiDoc document.
/// The parser handles all AsciiDoc block constructs defined in the spec and
/// invokes `InlineParser.parse(_:)` to resolve inline content.
enum BlockParser {

    // ── Public Entry Point ────────────────────────────────────────────────

    /// Parse an AsciiDoc string into an array of `BlockNode` values.
    ///
    /// - Parameter text: Raw AsciiDoc input.
    /// - Returns: Array of block nodes representing the document structure.
    static func parseBlocks(_ text: String) -> [BlockNode] {
        // Normalize line endings so we only deal with \n
        let normalized = text
            .replacingOccurrences(of: "\r\n", with: "\n")
            .replacingOccurrences(of: "\r", with: "\n")

        let lines = normalized.components(separatedBy: "\n")
        var state = ParseState()

        for line in lines {
            state = processLine(line, state: state)
        }

        // Flush any remaining accumulated block at EOF
        state = flush(state)
        return state.blocks
    }

    // ── Line Processing ───────────────────────────────────────────────────

    /// Process one line of input, advancing the state machine.
    ///
    /// This is the heart of the parser. Each line is classified and handled
    /// according to the current parse mode.
    ///
    /// - Parameters:
    ///   - line: The current line (without trailing newline).
    ///   - state: The current parse state.
    /// - Returns: The updated parse state.
    private static func processLine(_ line: String, state: ParseState) -> ParseState {
        var s = state

        // ── Inside a code listing block ────────────────────────────────────
        if s.mode == .codeBlock {
            if isListingDelimiter(line) {
                // Closing `----` fence — flush the code block
                s = flush(s)
            } else {
                s.currentLines.append(line)
            }
            return s
        }

        // ── Inside a literal block ─────────────────────────────────────────
        if s.mode == .literalBlock {
            if isLiteralDelimiter(line) {
                s = flush(s)
            } else {
                s.currentLines.append(line)
            }
            return s
        }

        // ── Inside a passthrough block ─────────────────────────────────────
        if s.mode == .passthroughBlock {
            if isPassthroughDelimiter(line) {
                s = flush(s)
            } else {
                s.currentLines.append(line)
            }
            return s
        }

        // ── Inside a quote block ───────────────────────────────────────────
        if s.mode == .quoteBlock {
            if isQuoteDelimiter(line) {
                s = flush(s)
            } else {
                s.currentLines.append(line)
            }
            return s
        }

        // ── Skip comment lines ─────────────────────────────────────────────
        // AsciiDoc single-line comments start with `//`.
        // They are silently discarded.
        if isCommentLine(line) {
            return s
        }

        // ── Blank line ─────────────────────────────────────────────────────
        // A blank line terminates a paragraph or list, and is ignored otherwise.
        if isBlank(line) {
            if s.mode == .paragraph || s.mode == .unorderedList || s.mode == .orderedList {
                s = flush(s)
            }
            return s
        }

        // ── Section heading ────────────────────────────────────────────────
        // Pattern: `= text` (h1) through `====== text` (h6).
        if let (level, text) = parseHeadingLine(line) {
            s = flush(s)
            let children = InlineParser.parse(text)
            s.blocks.append(.heading(HeadingNode(level: level, children: children)))
            return s
        }

        // ── Thematic break ─────────────────────────────────────────────────
        // Pattern: `'''` (3+ single quotes on their own line).
        if isThematicBreak(line) {
            s = flush(s)
            s.blocks.append(.thematicBreak)
            return s
        }

        // ── Attribute list ─────────────────────────────────────────────────
        // Pattern: `[source,lang]` — captures the language for the next block.
        // Only recognized when not already inside a block.
        if s.mode == .normal, let lang = parseAttrList(line) {
            s.pendingLanguage = lang
            return s
        }

        // ── Code listing delimiter ─────────────────────────────────────────
        // Pattern: 4+ consecutive dashes `----`.
        if isListingDelimiter(line) {
            s = flush(s)
            s.mode = .codeBlock
            // pendingLanguage set by [source,lang] above carries over
            return s
        }

        // ── Literal block delimiter ────────────────────────────────────────
        // Pattern: 4+ consecutive dots `....`.
        if isLiteralDelimiter(line) {
            s = flush(s)
            s.pendingLanguage = nil  // Literal blocks have no language
            s.mode = .literalBlock
            return s
        }

        // ── Passthrough block delimiter ────────────────────────────────────
        // Pattern: exactly `++++`.
        if isPassthroughDelimiter(line) {
            s = flush(s)
            s.mode = .passthroughBlock
            return s
        }

        // ── Quote block delimiter ──────────────────────────────────────────
        // Pattern: 4+ consecutive underscores `____`.
        if isQuoteDelimiter(line) {
            s = flush(s)
            s.mode = .quoteBlock
            return s
        }

        // ── Unordered list item ────────────────────────────────────────────
        // Pattern: `* text` (level 1), `** text` (level 2), …
        if let item = parseUnorderedItem(line) {
            if s.mode == .orderedList {
                // Switching from ordered to unordered — flush first
                s = flush(s)
                s.mode = .unorderedList
            } else if s.mode == .paragraph {
                s = flush(s)
                s.mode = .unorderedList
            } else if s.mode != .unorderedList {
                s.mode = .unorderedList
            }
            s.listItems.append(item)
            return s
        }

        // ── Ordered list item ──────────────────────────────────────────────
        // Pattern: `. text` (level 1), `.. text` (level 2), …
        if let item = parseOrderedItem(line) {
            if s.mode == .unorderedList {
                s = flush(s)
                s.mode = .orderedList
            } else if s.mode == .paragraph {
                s = flush(s)
                s.mode = .orderedList
            } else if s.mode != .orderedList {
                s.mode = .orderedList
            }
            s.listItems.append(item)
            return s
        }

        // ── Paragraph ──────────────────────────────────────────────────────
        // Any other non-blank line continues or starts a paragraph.
        if s.mode == .unorderedList || s.mode == .orderedList {
            // A non-list line after a list ends the list
            s = flush(s)
        }
        s.mode = .paragraph
        s.currentLines.append(line)
        return s
    }

    // ── Flush ──────────────────────────────────────────────────────────────

    /// Convert the current accumulated state into a `BlockNode` and append it.
    ///
    /// Called whenever a block boundary is detected: blank line after paragraph,
    /// closing delimiter for fenced blocks, end of input, etc.
    ///
    /// After flushing, `mode` is reset to `.normal` and all accumulators cleared.
    ///
    /// - Parameter state: The current parse state to flush.
    /// - Returns: Updated state with the new block appended and accumulators cleared.
    private static func flush(_ state: ParseState) -> ParseState {
        var s = state

        switch s.mode {

        case .normal:
            // Nothing to flush — clear any pending language attribute
            s.pendingLanguage = nil

        case .paragraph:
            // Join paragraph lines with newlines and parse inline content.
            // AsciiDoc allows soft breaks (single \n) within a paragraph.
            if !s.currentLines.isEmpty {
                let raw = s.currentLines.joined(separator: "\n")
                let children = InlineParser.parse(raw)
                s.blocks.append(.paragraph(ParagraphNode(children: children)))
            }
            s.currentLines = []
            s.pendingLanguage = nil

        case .codeBlock:
            // Join code lines with newlines and add a trailing newline.
            // The language was set by [source,lang] or is nil.
            let value = s.currentLines.joined(separator: "\n")
            let finalValue = value.isEmpty ? "\n" : value + "\n"
            s.blocks.append(.codeBlock(CodeBlockNode(language: s.pendingLanguage, value: finalValue)))
            s.currentLines = []
            s.pendingLanguage = nil

        case .literalBlock:
            // Literal blocks are verbatim, language is always nil.
            let value = s.currentLines.joined(separator: "\n")
            let finalValue = value.isEmpty ? "\n" : value + "\n"
            s.blocks.append(.codeBlock(CodeBlockNode(language: nil, value: finalValue)))
            s.currentLines = []
            s.pendingLanguage = nil

        case .passthroughBlock:
            // Passthrough blocks are emitted as raw HTML.
            // Content is verbatim — no inline parsing.
            let value = s.currentLines.joined(separator: "\n")
            let finalValue = value.isEmpty ? "" : value + "\n"
            s.blocks.append(.rawBlock(RawBlockNode(format: "html", value: finalValue)))
            s.currentLines = []
            s.pendingLanguage = nil

        case .quoteBlock:
            // Quote block content is recursively parsed as AsciiDoc.
            // This allows nested headings, lists, code blocks inside quotes.
            let inner = s.currentLines.joined(separator: "\n")
            let children = parseBlocks(inner)
            s.blocks.append(.blockquote(BlockquoteNode(children: children)))
            s.currentLines = []
            s.pendingLanguage = nil

        case .unorderedList:
            if !s.listItems.isEmpty {
                let node = buildNestedList(s.listItems, ordered: false)
                s.blocks.append(node)
            }
            s.listItems = []
            s.pendingLanguage = nil

        case .orderedList:
            if !s.listItems.isEmpty {
                let node = buildNestedList(s.listItems, ordered: true)
                s.blocks.append(node)
            }
            s.listItems = []
            s.pendingLanguage = nil
        }

        s.mode = .normal
        return s
    }

    // ── Line Classifiers ──────────────────────────────────────────────────

    /// Returns true if the line is blank (empty or all whitespace).
    ///
    ///     isBlank("")       → true
    ///     isBlank("  \t")   → true
    ///     isBlank("hello")  → false
    private static func isBlank(_ line: String) -> Bool {
        line.trimmingCharacters(in: .whitespaces).isEmpty
    }

    /// Returns true if the line is an AsciiDoc single-line comment (`//`).
    ///
    /// AsciiDoc comments start with `//`. Block comments (`////`) are treated
    /// as multiple single-line comments for simplicity.
    ///
    ///     isCommentLine("// a note")  → true
    ///     isCommentLine("//")         → true
    ///     isCommentLine("/foo")       → false
    private static func isCommentLine(_ line: String) -> Bool {
        line.hasPrefix("//")
    }

    /// Parse an AsciiDoc section heading line.
    ///
    /// AsciiDoc headings use `=` characters at the start of a line:
    ///
    ///   `= Title`         → level 1 (document title)
    ///   `== Section`      → level 2
    ///   `=== Subsection`  → level 3
    ///   …up to 6 levels
    ///
    /// The `=` characters must be followed by exactly one space, then text.
    ///
    /// - Parameter line: The line to parse.
    /// - Returns: `(level, text)` or `nil` if not a heading.
    private static func parseHeadingLine(_ line: String) -> (level: Int, text: String)? {
        var index = line.startIndex
        var level = 0

        // Count leading `=` characters (max 6)
        while index < line.endIndex && line[index] == "=" && level < 6 {
            level += 1
            index = line.index(after: index)
        }

        guard level > 0 else { return nil }

        // Must be followed by a space
        guard index < line.endIndex && line[index] == " " else { return nil }

        // The rest (after the space) is the heading text
        index = line.index(after: index)
        let text = String(line[index...]).trimmingCharacters(in: .whitespaces)
        return (level, text)
    }

    /// Returns true if the line is a listing (code) block delimiter (4+ dashes).
    ///
    ///   `----`     → true
    ///   `--------` → true
    ///   `---`      → false (only 3)
    ///   `--x-`     → false (other chars)
    private static func isListingDelimiter(_ line: String) -> Bool {
        let trimmed = line.trimmingCharacters(in: .whitespaces)
        guard trimmed.count >= 4 else { return false }
        return trimmed.allSatisfy { $0 == "-" }
    }

    /// Returns true if the line is a literal block delimiter (4+ dots).
    ///
    ///   `....`     → true
    ///   `........` → true
    ///   `...`      → false
    private static func isLiteralDelimiter(_ line: String) -> Bool {
        let trimmed = line.trimmingCharacters(in: .whitespaces)
        guard trimmed.count >= 4 else { return false }
        return trimmed.allSatisfy { $0 == "." }
    }

    /// Returns true if the line is a passthrough block delimiter (exactly `++++`).
    ///
    /// AsciiDoc passthrough blocks emit their content verbatim (raw HTML).
    ///
    ///   `++++`      → true
    ///   `+++++`     → true (5 is also valid)
    ///   `+++`       → false
    private static func isPassthroughDelimiter(_ line: String) -> Bool {
        let trimmed = line.trimmingCharacters(in: .whitespaces)
        guard trimmed.count >= 4 else { return false }
        return trimmed.allSatisfy { $0 == "+" }
    }

    /// Returns true if the line is a quote block delimiter (4+ underscores).
    ///
    ///   `____`      → true
    ///   `________`  → true
    ///   `___`       → false
    private static func isQuoteDelimiter(_ line: String) -> Bool {
        let trimmed = line.trimmingCharacters(in: .whitespaces)
        guard trimmed.count >= 4 else { return false }
        return trimmed.allSatisfy { $0 == "_" }
    }

    /// Returns true if the line is an AsciiDoc thematic break (`'''`).
    ///
    /// Three or more single-quote characters on their own line.
    ///
    ///   `'''`       → true
    ///   `''''`      → true
    ///   `''`        → false (only 2)
    ///   `'a'`       → false (other chars)
    private static func isThematicBreak(_ line: String) -> Bool {
        let trimmed = line.trimmingCharacters(in: .whitespaces)
        guard trimmed.count >= 3 else { return false }
        return trimmed.allSatisfy { $0 == "'" }
    }

    /// Parse an attribute list line and extract the language tag.
    ///
    /// AsciiDoc attribute lists appear on their own line before a block:
    ///
    ///   `[source,swift]`   → "swift"
    ///   `[source,python]`  → "python"
    ///   `[source]`         → nil (no language)
    ///   `[NOTE]`           → nil (not a source block)
    ///
    /// - Parameter line: The line to parse.
    /// - Returns: The language string, or `nil` if not a source attr list.
    private static func parseAttrList(_ line: String) -> String? {
        let trimmed = line.trimmingCharacters(in: .whitespaces)
        guard trimmed.hasPrefix("[") && trimmed.hasSuffix("]") else { return nil }
        let inner = String(trimmed.dropFirst().dropLast())
        let parts = inner.split(separator: ",", maxSplits: 2).map { String($0).trimmingCharacters(in: .whitespaces) }
        guard parts.count >= 2, parts[0].lowercased() == "source" else { return nil }
        let lang = parts[1].trimmingCharacters(in: .whitespaces)
        return lang.isEmpty ? nil : lang
    }

    /// Parse an unordered list item line.
    ///
    /// AsciiDoc unordered list items use `*` characters:
    ///
    ///   `* item`    → level 1, "item"
    ///   `** item`   → level 2, "item"
    ///   `*** item`  → level 3, "item"
    ///
    /// The `*` characters must be followed by a single space, then the item text.
    ///
    /// - Parameter line: The line to parse.
    /// - Returns: `(level, text)` or `nil` if not an unordered list item.
    private static func parseUnorderedItem(_ line: String) -> (level: Int, text: String)? {
        let trimmed = line.trimmingCharacters(in: .whitespaces)
        var index = trimmed.startIndex
        var level = 0

        // Count leading `*` characters
        while index < trimmed.endIndex && trimmed[index] == "*" {
            level += 1
            index = trimmed.index(after: index)
        }

        guard level > 0 && level <= 6 else { return nil }

        // Must be followed by a space
        guard index < trimmed.endIndex && trimmed[index] == " " else { return nil }

        index = trimmed.index(after: index)
        let text = String(trimmed[index...])
        return (level, text)
    }

    /// Parse an ordered list item line.
    ///
    /// AsciiDoc ordered list items use `.` characters:
    ///
    ///   `. item`    → level 1, "item"
    ///   `.. item`   → level 2, "item"
    ///   `... item`  → level 3, "item"
    ///
    /// - Parameter line: The line to parse.
    /// - Returns: `(level, text)` or `nil` if not an ordered list item.
    private static func parseOrderedItem(_ line: String) -> (level: Int, text: String)? {
        let trimmed = line.trimmingCharacters(in: .whitespaces)
        var index = trimmed.startIndex
        var level = 0

        // Count leading `.` characters
        while index < trimmed.endIndex && trimmed[index] == "." {
            level += 1
            index = trimmed.index(after: index)
        }

        guard level > 0 && level <= 6 else { return nil }

        // Must be followed by a space
        guard index < trimmed.endIndex && trimmed[index] == " " else { return nil }

        index = trimmed.index(after: index)
        let text = String(trimmed[index...])
        return (level, text)
    }

    // ── Nested List Builder ────────────────────────────────────────────────

    /// Build a nested `BlockNode` list from a flat array of `(level, text)` items.
    ///
    /// AsciiDoc supports multi-level lists where deeper levels are sub-lists
    /// inside list items. This function recursively groups items into the correct
    /// nesting structure.
    ///
    /// # Algorithm
    ///
    /// The function groups the flat item array into "runs" where each run starts
    /// with the minimum level found in the slice. Items at a deeper level become
    /// a nested sub-list inside the preceding item.
    ///
    /// # Example
    ///
    ///     items = [(1,"a"), (2,"b"), (2,"c"), (1,"d")]
    ///
    ///     Result:
    ///       list [
    ///         listItem [paragraph("a"), list [listItem [para "b"], listItem [para "c"]]],
    ///         listItem [paragraph("d")]
    ///       ]
    ///
    /// - Parameters:
    ///   - items: Flat array of `(level, text)` tuples.
    ///   - ordered: `true` for ordered (`<ol>`), `false` for unordered (`<ul>`).
    /// - Returns: A `.list(ListNode(...))` block node.
    static func buildNestedList(_ items: [(level: Int, text: String)], ordered: Bool) -> BlockNode {
        guard !items.isEmpty else {
            return .list(ListNode(ordered: ordered, start: ordered ? 1 : nil, tight: true, children: []))
        }

        let minLevel = items.map { $0.level }.min()!
        var listItems: [ListItemNode] = []
        var i = 0

        while i < items.count {
            let item = items[i]

            if item.level == minLevel {
                // This is a top-level item in this slice.
                // Collect any following items that are deeper (they become sub-items).
                var subItems: [(level: Int, text: String)] = []
                i += 1

                while i < items.count && items[i].level > minLevel {
                    subItems.append(items[i])
                    i += 1
                }

                // Build the item's children: always starts with a paragraph
                let children = InlineParser.parse(item.text)
                var itemChildren: [BlockNode] = [
                    .paragraph(ParagraphNode(children: children))
                ]

                // If there are sub-items, recursively build a nested list
                if !subItems.isEmpty {
                    // Determine if the sub-list is ordered or unordered
                    // by checking if the first sub-item was from the ordered side.
                    // Since we use a single ordered flag per call, pass it through.
                    itemChildren.append(buildNestedList(subItems, ordered: ordered))
                }

                listItems.append(ListItemNode(children: itemChildren))
            } else {
                // Item is deeper than minLevel but appears without a preceding
                // same-level item. Treat it as a top-level item for robustness.
                let children = InlineParser.parse(item.text)
                listItems.append(ListItemNode(children: [
                    .paragraph(ParagraphNode(children: children))
                ]))
                i += 1
            }
        }

        return .list(ListNode(
            ordered: ordered,
            start: ordered ? 1 : nil,
            tight: true,
            children: listItems
        ))
    }
}
