package asciidocparser

// block_parser.go — Phase 1: AsciiDoc Block Structure
//
// This file implements the block-level state machine that converts AsciiDoc
// source text into a tree of DocumentAST block nodes.
//
// # State Machine
//
// The parser maintains a current state that determines how each line is
// interpreted. The states are:
//
//	normal        — between blocks, looking for the start of a new block
//	paragraph     — accumulating paragraph lines
//	code_block    — inside a ---- delimited code block
//	literal_block — inside a .... delimited literal block (treated as code)
//	passthrough   — inside a ++++ block (raw HTML passthrough)
//	quote_block   — inside a ____ block (blockquote; content re-parsed)
//	unordered_list — accumulating unordered list items
//	ordered_list  — accumulating ordered list items
//
// # Line Dispatch in Normal State
//
// Each line in normal state is tested in priority order:
//
//	blank line         → stay in normal (flush any pending block)
//	// comment         → skip
//	[source,lang]      → set pending_language for next code block
//	= text             → heading level 1
//	== text            → heading level 2  (up to ======)
//	''' (≥3 quotes)    → thematic break
//	---- (≥4 dashes)   → enter code_block mode
//	.... (≥4 dots)     → enter literal_block mode
//	++++ (≥4 pluses)   → enter passthrough_block mode
//	____ (≥4 undersc.) → enter quote_block mode
//	* text / ** text   → unordered list item (level = count of *)
//	. text / .. text   → ordered list item   (level = count of .)
//	other text         → paragraph mode
//
// # List Nesting
//
// List items carry their nesting level (count of leading * or . characters).
// When a list item is encountered, we close or open list nesting levels as
// needed to build the tree. AsciiDoc uses continuation (+) to continue a
// list item across paragraphs, but we parse the simple case here.
//
// # Quote Block Recursion
//
// The content inside a ____ block is recursively re-parsed with Parse(),
// so nested AsciiDoc syntax works inside blockquotes.

import (
	"strings"

	documentast "github.com/adhithyan15/coding-adventures/code/packages/go/document-ast"
)

// ─── Internal list item accumulator ──────────────────────────────────────────

type listItemAccum struct {
	level    int    // nesting level (1 = top, 2 = nested, etc.)
	ordered  bool   // true = ., false = *
	rawLines []string // content lines for inline parsing
}

// ─── parseBlocks ─────────────────────────────────────────────────────────────

// parseBlocks converts AsciiDoc text into a slice of DocumentAST block nodes.
// This is Phase 1 of parsing — it identifies structural blocks and leaves
// inline content as raw strings for Phase 2 (inline_parser.go).
func parseBlocks(text string) []documentast.BlockNode {
	lines := splitLines(text)
	var result []documentast.BlockNode

	// Parser state
	type state int
	const (
		stateNormal state = iota
		stateParagraph
		stateCodeBlock
		stateLiteralBlock
		statePassthrough
		stateQuoteBlock
	)

	cur := stateNormal
	var pendingLanguage string
	var accumLines []string  // lines being accumulated in a block
	var listItems []listItemAccum  // pending list items

	// flushParagraph emits a ParagraphNode from accumulated lines.
	flushParagraph := func() {
		if len(accumLines) == 0 {
			return
		}
		raw := strings.Join(accumLines, "\n")
		inlines := parseInlines(raw)
		result = append(result, &documentast.ParagraphNode{Children: inlines})
		accumLines = nil
	}

	// flushList emits ListNode(s) from accumulated list items.
	flushList := func() {
		if len(listItems) == 0 {
			return
		}
		nodes := buildListNodes(listItems)
		result = append(result, nodes...)
		listItems = nil
	}

	// isHeadingLine checks if a line starts with one or more = followed by space.
	// Returns (level, content) if it is a heading line, otherwise (0, "").
	isHeadingLine := func(line string) (int, string) {
		i := 0
		for i < len(line) && line[i] == '=' {
			i++
		}
		if i == 0 || i > 6 {
			return 0, ""
		}
		if i < len(line) && line[i] == ' ' {
			return i, strings.TrimSpace(line[i+1:])
		}
		return 0, ""
	}

	// isListLine checks if a line is an unordered (*) or ordered (.) list item.
	// Returns (level, ordered, content) or (0, false, "").
	isListLine := func(line string) (int, bool, string) {
		// Unordered: * or ** or ***
		if len(line) > 0 && line[0] == '*' {
			i := 0
			for i < len(line) && line[i] == '*' {
				i++
			}
			if i < len(line) && line[i] == ' ' {
				return i, false, strings.TrimSpace(line[i+1:])
			}
		}
		// Ordered: . or .. or ...
		if len(line) > 0 && line[0] == '.' {
			i := 0
			for i < len(line) && line[i] == '.' {
				i++
			}
			if i < len(line) && line[i] == ' ' {
				return i, true, strings.TrimSpace(line[i+1:])
			}
		}
		return 0, false, ""
	}

	for _, line := range lines {
		switch cur {

		case stateCodeBlock:
			// Inside ---- fenced code block
			if isDelim(line, '-', 4) {
				// End of code block
				val := strings.Join(accumLines, "\n")
				if val != "" && !strings.HasSuffix(val, "\n") {
					val += "\n"
				}
				lang := pendingLanguage
				pendingLanguage = ""
				result = append(result, &documentast.CodeBlockNode{Language: lang, Value: val})
				accumLines = nil
				cur = stateNormal
			} else {
				accumLines = append(accumLines, line)
			}

		case stateLiteralBlock:
			// Inside .... literal block (treated as code, no language)
			if isDelim(line, '.', 4) {
				val := strings.Join(accumLines, "\n")
				if val != "" && !strings.HasSuffix(val, "\n") {
					val += "\n"
				}
				result = append(result, &documentast.CodeBlockNode{Language: "", Value: val})
				accumLines = nil
				cur = stateNormal
			} else {
				accumLines = append(accumLines, line)
			}

		case statePassthrough:
			// Inside ++++ passthrough block (raw HTML)
			if isDelim(line, '+', 4) {
				val := strings.Join(accumLines, "\n")
				result = append(result, &documentast.RawBlockNode{Format: "html", Value: val})
				accumLines = nil
				cur = stateNormal
			} else {
				accumLines = append(accumLines, line)
			}

		case stateQuoteBlock:
			// Inside ____ quote block — collect lines until closing delimiter
			if isDelim(line, '_', 4) {
				// Recursively parse the quoted content
				inner := strings.Join(accumLines, "\n")
				innerBlocks := parseBlocks(inner)
				result = append(result, &documentast.BlockquoteNode{Children: innerBlocks})
				accumLines = nil
				cur = stateNormal
			} else {
				accumLines = append(accumLines, line)
			}

		case stateParagraph:
			if strings.TrimSpace(line) == "" {
				// Blank line ends the paragraph
				flushParagraph()
				cur = stateNormal
			} else {
				// Check for block-starting patterns that end a paragraph
				if level, content := isHeadingLine(line); level > 0 {
					flushParagraph()
					inlines := parseInlines(content)
					result = append(result, &documentast.HeadingNode{Level: level, Children: inlines})
					cur = stateNormal
				} else if itemLevel, itemOrdered, itemContent := isListLine(line); itemLevel > 0 {
					flushParagraph()
					listItems = append(listItems, listItemAccum{
						level:    itemLevel,
						ordered:  itemOrdered,
						rawLines: []string{itemContent},
					})
					cur = stateNormal
				} else {
					accumLines = append(accumLines, line)
				}
			}

		case stateNormal:
			trimmed := strings.TrimSpace(line)

			// Blank line: no-op (flush any pending list)
			if trimmed == "" {
				flushList()
				continue
			}

			// Single-line comment: //
			if strings.HasPrefix(line, "//") && !strings.HasPrefix(line, "///") {
				continue
			}

			// Block attribute line: [source,lang] or [source] or [literal] etc.
			if strings.HasPrefix(line, "[") && strings.HasSuffix(strings.TrimSpace(line), "]") {
				flushList()
				attr := strings.TrimSpace(line[1 : len(strings.TrimSpace(line))-1])
				if strings.HasPrefix(strings.ToLower(attr), "source") {
					// Extract language: [source,ruby] → "ruby", [source] → ""
					parts := strings.SplitN(attr, ",", 2)
					if len(parts) == 2 {
						pendingLanguage = strings.TrimSpace(parts[1])
					} else {
						pendingLanguage = ""
					}
				}
				continue
			}

			// Heading: = ... ====== (level 1–6)
			if level, content := isHeadingLine(line); level > 0 {
				flushList()
				inlines := parseInlines(content)
				result = append(result, &documentast.HeadingNode{Level: level, Children: inlines})
				continue
			}

			// Thematic break: ''' (three or more single-quotes on their own line)
			if isThematicBreak(line) {
				flushList()
				result = append(result, &documentast.ThematicBreakNode{})
				continue
			}

			// Code block delimiter: ----
			if isDelim(line, '-', 4) {
				flushList()
				accumLines = nil
				cur = stateCodeBlock
				continue
			}

			// Literal block delimiter: ....
			if isDelim(line, '.', 4) {
				flushList()
				accumLines = nil
				cur = stateLiteralBlock
				continue
			}

			// Passthrough block delimiter: ++++
			if isDelim(line, '+', 4) {
				flushList()
				accumLines = nil
				cur = statePassthrough
				continue
			}

			// Quote block delimiter: ____
			if isDelim(line, '_', 4) {
				flushList()
				accumLines = nil
				cur = stateQuoteBlock
				continue
			}

			// Unordered or ordered list item
			if itemLevel, itemOrdered, itemContent := isListLine(line); itemLevel > 0 {
				// If we're already accumulating a list, check type compatibility
				if len(listItems) > 0 && listItems[0].ordered != itemOrdered {
					// Different list type: flush and start new
					flushList()
				}
				listItems = append(listItems, listItemAccum{
					level:    itemLevel,
					ordered:  itemOrdered,
					rawLines: []string{itemContent},
				})
				continue
			}

			// List continuation marker: +
			if line == "+" && len(listItems) > 0 {
				// continuation; just skip for now
				continue
			}

			// Regular text: start a paragraph
			flushList()
			accumLines = []string{line}
			cur = stateParagraph
		}
	}

	// Flush any remaining state
	switch cur {
	case stateParagraph:
		flushParagraph()
	case stateCodeBlock, stateLiteralBlock:
		// Unclosed code block — emit what we have
		val := strings.Join(accumLines, "\n")
		if val != "" && !strings.HasSuffix(val, "\n") {
			val += "\n"
		}
		lang := pendingLanguage
		result = append(result, &documentast.CodeBlockNode{Language: lang, Value: val})
	case statePassthrough:
		val := strings.Join(accumLines, "\n")
		result = append(result, &documentast.RawBlockNode{Format: "html", Value: val})
	case stateQuoteBlock:
		inner := strings.Join(accumLines, "\n")
		innerBlocks := parseBlocks(inner)
		result = append(result, &documentast.BlockquoteNode{Children: innerBlocks})
	}
	flushList()

	return result
}

// ─── Helper functions ─────────────────────────────────────────────────────────

// splitLines splits text on \n, preserving empty lines. Trailing newline does
// not create an extra empty element.
func splitLines(text string) []string {
	if text == "" {
		return nil
	}
	lines := strings.Split(text, "\n")
	// Remove trailing empty line from trailing newline
	if len(lines) > 0 && lines[len(lines)-1] == "" {
		lines = lines[:len(lines)-1]
	}
	return lines
}

// isDelim returns true if `line` consists entirely of `ch` repeated at least
// `minLen` times (with optional trailing whitespace).
func isDelim(line string, ch byte, minLen int) bool {
	trimmed := strings.TrimRight(line, " \t")
	if len(trimmed) < minLen {
		return false
	}
	for i := 0; i < len(trimmed); i++ {
		if trimmed[i] != ch {
			return false
		}
	}
	return true
}

// isThematicBreak returns true if the line is ''' or more single-quotes.
func isThematicBreak(line string) bool {
	trimmed := strings.TrimRight(line, " \t")
	if len(trimmed) < 3 {
		return false
	}
	for i := 0; i < len(trimmed); i++ {
		if trimmed[i] != '\'' {
			return false
		}
	}
	return true
}

// ─── List building ────────────────────────────────────────────────────────────

// buildListNodes converts a flat slice of listItemAccum into one or more
// ListNode block nodes, respecting nesting levels.
//
// AsciiDoc list nesting works by counting leading markers:
//   * level 1
//   ** level 2
//   *** level 3
//
// Items at the same level become siblings. An item at a deeper level becomes
// a nested list inside the previous item.
func buildListNodes(items []listItemAccum) []documentast.BlockNode {
	if len(items) == 0 {
		return nil
	}

	// Determine if this is an ordered or unordered list
	ordered := items[0].ordered

	// Build a flat slice of (level, content) and produce nested structure
	root := buildNestedList(items, ordered, 1)
	if root == nil {
		return nil
	}
	return []documentast.BlockNode{root}
}

// buildNestedList recursively builds a ListNode from items starting at `level`.
func buildNestedList(items []listItemAccum, ordered bool, level int) *documentast.ListNode {
	list := &documentast.ListNode{
		Ordered: ordered,
		Start:   1,
		Tight:   true,
	}

	i := 0
	for i < len(items) {
		item := items[i]
		if item.level < level {
			// This item belongs to an outer level; stop
			break
		}
		if item.level == level {
			// Create a list item with its inline content
			raw := strings.Join(item.rawLines, "\n")
			inlines := parseInlines(raw)
			li := &documentast.ListItemNode{
				Children: []documentast.BlockNode{
					&documentast.ParagraphNode{Children: inlines},
				},
			}

			// Look ahead for nested items
			j := i + 1
			var nestedItems []listItemAccum
			for j < len(items) && items[j].level > level {
				nestedItems = append(nestedItems, items[j])
				j++
			}
			if len(nestedItems) > 0 {
				nested := buildNestedList(nestedItems, nestedItems[0].ordered, level+1)
				if nested != nil {
					li.Children = append(li.Children, nested)
				}
			}

			list.Children = append(list.Children, li)
			i = j
		} else {
			// item.level > level: skip (already consumed by nested call)
			i++
		}
	}

	if len(list.Children) == 0 {
		return nil
	}
	return list
}
