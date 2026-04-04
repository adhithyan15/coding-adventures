package asciidocparser

// inline_parser.go — Phase 2: AsciiDoc Inline Content
//
// This file implements the inline parser that converts raw AsciiDoc inline
// content strings into inline node trees.
//
// # AsciiDoc Inline Priority Order
//
// Inline constructs are checked in this priority order to ensure that longer
// markers (** before *) take precedence over shorter ones:
//
//  1. Two trailing spaces + newline  → hard break
//  2. backslash + newline (\n)       → hard break
//  3. plain newline                  → soft break
//  4. backtick (`)                   → code span (verbatim, no markup inside)
//  5. ** ... **                      → strong (unconstrained)
//  6. __ ... __                      → emphasis (unconstrained)
//  7. * ... *                        → strong (NOTE: AsciiDoc * = strong!)
//  8. _ ... _                        → emphasis
//  9. link:url[text]                 → link
// 10. image:url[alt]                 → image
// 11. <<anchor,text>> or <<anchor>>  → cross-reference link
// 12. https:// or http:// with [text] → link; bare → autolink
// 13. anything else                  → text
//
// # AsciiDoc vs Markdown Bold/Italic
//
// In Markdown: *text* = emphasis, **text** = strong.
// In AsciiDoc: *text* = strong, **text** = strong (unconstrained).
//              _text_ = emphasis, __text__ = emphasis (unconstrained).
// This is a major semantic difference that is easy to get wrong.
//
// Constrained vs unconstrained:
//   Constrained (* _): must be surrounded by word boundaries (spaces/punctuation)
//   Unconstrained (** __): can appear mid-word

import (
	"strings"

	documentast "github.com/adhithyan15/coding-adventures/code/packages/go/document-ast"
)

// parseInlines converts a raw AsciiDoc inline string into a slice of
// InlineNode values. This is Phase 2 of the parser.
func parseInlines(raw string) []documentast.InlineNode {
	if raw == "" {
		return nil
	}
	p := &inlineParser{src: raw, pos: 0}
	return p.parse()
}

// inlineParser holds the parser state for inline content.
type inlineParser struct {
	src string
	pos int
}

// parse runs the inline parsing loop and returns all parsed inline nodes.
func (p *inlineParser) parse() []documentast.InlineNode {
	var nodes []documentast.InlineNode
	var textBuf strings.Builder

	flushText := func() {
		if textBuf.Len() > 0 {
			nodes = append(nodes, &documentast.TextNode{Value: textBuf.String()})
			textBuf.Reset()
		}
	}

	for p.pos < len(p.src) {
		// ── Hard break: two trailing spaces before \n ──────────────────────
		if p.src[p.pos] == ' ' && p.pos+2 < len(p.src) && p.src[p.pos+1] == ' ' && p.src[p.pos+2] == '\n' {
			flushText()
			nodes = append(nodes, &documentast.HardBreakNode{})
			p.pos += 3
			continue
		}

		// ── Hard break: backslash before \n ───────────────────────────────
		if p.src[p.pos] == '\\' && p.pos+1 < len(p.src) && p.src[p.pos+1] == '\n' {
			flushText()
			nodes = append(nodes, &documentast.HardBreakNode{})
			p.pos += 2
			continue
		}

		// ── Soft break: plain newline ─────────────────────────────────────
		if p.src[p.pos] == '\n' {
			flushText()
			nodes = append(nodes, &documentast.SoftBreakNode{})
			p.pos++
			continue
		}

		// ── Code span: `...` ─────────────────────────────────────────────
		if p.src[p.pos] == '`' {
			if node, advance := p.tryCodeSpan(); node != nil {
				flushText()
				nodes = append(nodes, node)
				p.pos += advance
				continue
			}
		}

		// ── Strong unconstrained: **...** ────────────────────────────────
		if p.pos+1 < len(p.src) && p.src[p.pos] == '*' && p.src[p.pos+1] == '*' {
			if node, advance := p.tryMarker("**", func(inner []documentast.InlineNode) documentast.InlineNode {
				return &documentast.StrongNode{Children: inner}
			}); node != nil {
				flushText()
				nodes = append(nodes, node)
				p.pos += advance
				continue
			}
		}

		// ── Emphasis unconstrained: __...__ ──────────────────────────────
		if p.pos+1 < len(p.src) && p.src[p.pos] == '_' && p.src[p.pos+1] == '_' {
			if node, advance := p.tryMarker("__", func(inner []documentast.InlineNode) documentast.InlineNode {
				return &documentast.EmphasisNode{Children: inner}
			}); node != nil {
				flushText()
				nodes = append(nodes, node)
				p.pos += advance
				continue
			}
		}

		// ── Strong constrained: *...* (AsciiDoc: * = strong!) ────────────
		if p.src[p.pos] == '*' {
			if node, advance := p.tryConstrainedMarker('*', func(inner []documentast.InlineNode) documentast.InlineNode {
				return &documentast.StrongNode{Children: inner}
			}); node != nil {
				flushText()
				nodes = append(nodes, node)
				p.pos += advance
				continue
			}
		}

		// ── Emphasis constrained: _..._ ──────────────────────────────────
		if p.src[p.pos] == '_' {
			if node, advance := p.tryConstrainedMarker('_', func(inner []documentast.InlineNode) documentast.InlineNode {
				return &documentast.EmphasisNode{Children: inner}
			}); node != nil {
				flushText()
				nodes = append(nodes, node)
				p.pos += advance
				continue
			}
		}

		// ── link:url[text] ────────────────────────────────────────────────
		if strings.HasPrefix(p.src[p.pos:], "link:") {
			if node, advance := p.tryLinkMacro(); node != nil {
				flushText()
				nodes = append(nodes, node)
				p.pos += advance
				continue
			}
		}

		// ── image:url[alt] ────────────────────────────────────────────────
		if strings.HasPrefix(p.src[p.pos:], "image:") {
			if node, advance := p.tryImageMacro(); node != nil {
				flushText()
				nodes = append(nodes, node)
				p.pos += advance
				continue
			}
		}

		// ── Cross-reference: <<anchor,text>> or <<anchor>> ────────────────
		if p.pos+1 < len(p.src) && p.src[p.pos] == '<' && p.src[p.pos+1] == '<' {
			if node, advance := p.tryCrossRef(); node != nil {
				flushText()
				nodes = append(nodes, node)
				p.pos += advance
				continue
			}
		}

		// ── URL with bracket text: https://....[text] or bare URL ─────────
		if strings.HasPrefix(p.src[p.pos:], "https://") || strings.HasPrefix(p.src[p.pos:], "http://") {
			if node, advance := p.tryURL(); node != nil {
				flushText()
				nodes = append(nodes, node)
				p.pos += advance
				continue
			}
		}

		// ── Default: consume one byte into the text buffer ────────────────
		textBuf.WriteByte(p.src[p.pos])
		p.pos++
	}

	flushText()
	return nodes
}

// ─── Inline construct parsers ─────────────────────────────────────────────────

// tryCodeSpan tries to parse a backtick code span starting at p.pos.
// Returns (node, advance) or (nil, 0).
func (p *inlineParser) tryCodeSpan() (documentast.InlineNode, int) {
	start := p.pos
	if start >= len(p.src) || p.src[start] != '`' {
		return nil, 0
	}
	// Count opening backticks
	numTicks := 0
	for start+numTicks < len(p.src) && p.src[start+numTicks] == '`' {
		numTicks++
	}
	// Find closing sequence of same length
	search := strings.Repeat("`", numTicks)
	rest := p.src[start+numTicks:]
	idx := strings.Index(rest, search)
	if idx < 0 {
		return nil, 0
	}
	content := rest[:idx]
	// Strip single leading/trailing space if present on both sides
	if len(content) >= 2 && content[0] == ' ' && content[len(content)-1] == ' ' {
		content = content[1 : len(content)-1]
	}
	advance := numTicks + idx + numTicks
	return &documentast.CodeSpanNode{Value: content}, advance
}

// tryMarker tries to parse an unconstrained inline span with marker m (e.g. "**", "__").
// Returns (node, advance) or (nil, 0).
func (p *inlineParser) tryMarker(marker string, wrap func([]documentast.InlineNode) documentast.InlineNode) (documentast.InlineNode, int) {
	start := p.pos
	mlen := len(marker)
	if start+mlen > len(p.src) {
		return nil, 0
	}
	if p.src[start:start+mlen] != marker {
		return nil, 0
	}
	// Find closing marker
	rest := p.src[start+mlen:]
	idx := strings.Index(rest, marker)
	if idx < 0 {
		return nil, 0
	}
	inner := rest[:idx]
	innerNodes := parseInlines(inner)
	advance := mlen + idx + mlen
	return wrap(innerNodes), advance
}

// tryConstrainedMarker tries to parse a constrained inline span with single char ch.
// Constrained means: the opener must be preceded by a non-word char (or start of string),
// and the closer must be followed by a non-word char (or end of string).
func (p *inlineParser) tryConstrainedMarker(ch byte, wrap func([]documentast.InlineNode) documentast.InlineNode) (documentast.InlineNode, int) {
	start := p.pos
	if start >= len(p.src) || p.src[start] != ch {
		return nil, 0
	}
	// Check that the next char is not the same (to avoid ** being consumed here)
	if start+1 < len(p.src) && p.src[start+1] == ch {
		return nil, 0
	}
	// Check left boundary: must be at start or preceded by non-word char
	if start > 0 && isWordChar(p.src[start-1]) {
		return nil, 0
	}
	// Find closing marker
	end := start + 1
	for end < len(p.src) {
		if p.src[end] == ch {
			// Check it's not a doubled marker
			if end+1 < len(p.src) && p.src[end+1] == ch {
				end++
				continue
			}
			// Check right boundary: must be at end or followed by non-word char
			if end+1 < len(p.src) && isWordChar(p.src[end+1]) {
				end++
				continue
			}
			inner := p.src[start+1 : end]
			if inner == "" {
				return nil, 0
			}
			innerNodes := parseInlines(inner)
			advance := end - start + 1
			return wrap(innerNodes), advance
		}
		end++
	}
	return nil, 0
}

// tryLinkMacro tries to parse link:url[text] starting at p.pos.
func (p *inlineParser) tryLinkMacro() (documentast.InlineNode, int) {
	rest := p.src[p.pos:]
	if !strings.HasPrefix(rest, "link:") {
		return nil, 0
	}
	after := rest[5:] // after "link:"
	// Find the [ that starts the label
	bracketIdx := strings.Index(after, "[")
	if bracketIdx < 0 {
		return nil, 0
	}
	url := after[:bracketIdx]
	if url == "" {
		return nil, 0
	}
	after2 := after[bracketIdx+1:]
	closingIdx := strings.Index(after2, "]")
	if closingIdx < 0 {
		return nil, 0
	}
	label := after2[:closingIdx]
	advance := 5 + bracketIdx + 1 + closingIdx + 1
	var children []documentast.InlineNode
	if label == "" {
		children = []documentast.InlineNode{&documentast.TextNode{Value: url}}
	} else {
		children = parseInlines(label)
	}
	return &documentast.LinkNode{
		Destination: url,
		Children:    children,
	}, advance
}

// tryImageMacro tries to parse image:url[alt] starting at p.pos.
func (p *inlineParser) tryImageMacro() (documentast.InlineNode, int) {
	rest := p.src[p.pos:]
	if !strings.HasPrefix(rest, "image:") {
		return nil, 0
	}
	after := rest[6:] // after "image:"
	bracketIdx := strings.Index(after, "[")
	if bracketIdx < 0 {
		return nil, 0
	}
	url := after[:bracketIdx]
	if url == "" {
		return nil, 0
	}
	after2 := after[bracketIdx+1:]
	closingIdx := strings.Index(after2, "]")
	if closingIdx < 0 {
		return nil, 0
	}
	alt := after2[:closingIdx]
	advance := 6 + bracketIdx + 1 + closingIdx + 1
	return &documentast.ImageNode{
		Destination: url,
		Alt:         alt,
	}, advance
}

// tryCrossRef tries to parse <<anchor,text>> or <<anchor>> starting at p.pos.
func (p *inlineParser) tryCrossRef() (documentast.InlineNode, int) {
	rest := p.src[p.pos:]
	if !strings.HasPrefix(rest, "<<") {
		return nil, 0
	}
	after := rest[2:]
	closingIdx := strings.Index(after, ">>")
	if closingIdx < 0 {
		return nil, 0
	}
	inner := after[:closingIdx]
	advance := 2 + closingIdx + 2

	// Inner may be "anchor,text" or just "anchor"
	parts := strings.SplitN(inner, ",", 2)
	anchor := "#" + strings.TrimSpace(parts[0])
	var label string
	if len(parts) == 2 {
		label = strings.TrimSpace(parts[1])
	} else {
		label = strings.TrimSpace(parts[0])
	}
	return &documentast.LinkNode{
		Destination: anchor,
		Children:    []documentast.InlineNode{&documentast.TextNode{Value: label}},
	}, advance
}

// tryURL tries to parse a URL (http:// or https://) with optional [text] suffix.
// With [text]: renders as a link node.
// Without: renders as an autolink node.
func (p *inlineParser) tryURL() (documentast.InlineNode, int) {
	rest := p.src[p.pos:]
	// Find end of URL: space, newline, or [ starts a bracket label
	end := 0
	for end < len(rest) && rest[end] != ' ' && rest[end] != '\n' && rest[end] != '[' {
		end++
	}
	url := rest[:end]
	if url == "" {
		return nil, 0
	}

	// Check for [text] suffix
	if end < len(rest) && rest[end] == '[' {
		after := rest[end+1:]
		closingIdx := strings.Index(after, "]")
		if closingIdx >= 0 {
			label := after[:closingIdx]
			advance := end + 1 + closingIdx + 1
			var children []documentast.InlineNode
			if label == "" {
				children = []documentast.InlineNode{&documentast.TextNode{Value: url}}
			} else {
				children = parseInlines(label)
			}
			return &documentast.LinkNode{
				Destination: url,
				Children:    children,
			}, advance
		}
	}

	// Bare URL → autolink
	return &documentast.AutolinkNode{Destination: url, IsEmail: false}, end
}

// ─── Character classification helpers ────────────────────────────────────────

// isWordChar returns true for ASCII letters, digits, and underscore.
// Used for constrained marker boundary checks.
func isWordChar(c byte) bool {
	return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '_'
}
