package commonmarkparser

import (
	"regexp"
	"strings"

	documentast "github.com/adhithyan15/coding-adventures/code/packages/go/document-ast"
)

// ParseInline is Phase 2 of CommonMark parsing: scan raw inline content
// strings (produced by the block parser) and emit inline AST nodes —
// emphasis, links, code spans, etc.
//
// # Overview of Inline Constructs
//
// CommonMark recognises ten inline constructs, processed left-to-right:
//
//  1. Backslash escapes       `\*`    → literal `*`
//  2. HTML character refs     `&amp;` → `&`
//  3. Code spans              `` `code` ``
//  4. HTML inline             `<em>`, `<!-- -->`, `<?...?>`
//  5. Autolinks               `<https://example.com>`, `<me@example.com>`
//  6. Hard line breaks        two trailing spaces + newline, or `\` + newline
//  7. Soft line breaks        single newline within a paragraph
//  8. Emphasis / strong       `*em*`, `**strong**`, `_em_`, `__strong__`
//  9. Links                   `[text](url)`, `[text][label]`, `[text][]`
//  10. Images                 `![alt](url)`, `![alt][label]`
//
// # The Delimiter Stack Algorithm
//
// Emphasis is the hardest part of CommonMark inline parsing. The rules are
// context-sensitive: whether `*` or `_` can open or close emphasis depends
// on what precedes and follows the run. CommonMark Appendix A defines the
// canonical "delimiter stack" algorithm.
//
// The algorithm has two phases:
//
//	A. SCAN — read left-to-right, building a flat list of "tokens":
//	   ordinary text, delimiter runs (* ** _ __), code spans, links, etc.
//	   Each delimiter run is tagged as can_open, can_close, or both.
//
//	B. RESOLVE — walk the token list, matching openers with the nearest
//	   valid closers. For each matched pair, wrap the tokens between them
//	   in an emphasis or strong node.
func ParseInline(raw string, linkRefs map[string]*linkReference) []documentast.InlineNode {
	sc := NewScanner(raw)
	var tokens []inlineToken

	// bracketStack holds indices into tokens of each open bracket.
	var bracketStack []int

	// Text accumulation buffer — flushed into a NodeToken when a non-text
	// construct is encountered.
	var textBuf strings.Builder

	flushText := func() {
		if textBuf.Len() > 0 {
			tokens = append(tokens, &nodeToken{node: &documentast.TextNode{Value: textBuf.String()}})
			textBuf.Reset()
		}
	}

	// ─── Scan Phase ──────────────────────────────────────────────────────────

	for !sc.Done() {
		ch := sc.PeekByte()

		// ── 1. Backslash escape ─────────────────────────────────────────────
		if ch == '\\' {
			next := sc.PeekByteAt(1)
			if next != 0 && IsAsciiPunctuation(rune(next)) {
				sc.Skip(2)
				textBuf.WriteByte(next)
				continue
			}
			if next == '\n' {
				sc.Skip(2)
				flushText()
				tokens = append(tokens, &nodeToken{node: &documentast.HardBreakNode{}})
				continue
			}
			sc.Skip(1)
			textBuf.WriteByte('\\')
			continue
		}

		// ── 2. HTML character reference ─────────────────────────────────────
		if ch == '&' {
			m := matchAtPos(entityInlineRe, sc.Source, sc.Pos)
			if m != "" {
				sc.Pos += len(m)
				textBuf.WriteString(DecodeEntity(m))
				continue
			}
			sc.Skip(1)
			textBuf.WriteByte('&')
			continue
		}

		// ── 3. Code span ────────────────────────────────────────────────────
		if ch == '`' {
			span := tryCodeSpan(sc)
			if span != nil {
				flushText()
				tokens = append(tokens, &nodeToken{node: span})
				continue
			}
			ticks := sc.ConsumeWhile(func(b byte) bool { return b == '`' })
			textBuf.WriteString(ticks)
			continue
		}

		// ── 4 & 5. HTML inline and autolinks (both start with `<`) ──────────
		if ch == '<' {
			autolink := tryAutolink(sc)
			if autolink != nil {
				flushText()
				tokens = append(tokens, &nodeToken{node: autolink})
				continue
			}
			htmlInline := tryHtmlInline(sc)
			if htmlInline != nil {
				flushText()
				tokens = append(tokens, &nodeToken{node: htmlInline})
				continue
			}
			sc.Skip(1)
			textBuf.WriteByte('<')
			continue
		}

		// ── Image opener `![` ───────────────────────────────────────────────
		if ch == '!' && sc.PeekByteAt(1) == '[' {
			flushText()
			bracketStack = append(bracketStack, len(tokens))
			sc.Skip(2)
			tokens = append(tokens, &bracketToken{isImage: true, active: true, sourcePos: sc.Pos})
			continue
		}

		// ── Link opener `[` ─────────────────────────────────────────────────
		if ch == '[' {
			flushText()
			bracketStack = append(bracketStack, len(tokens))
			sc.Skip(1)
			tokens = append(tokens, &bracketToken{isImage: false, active: true, sourcePos: sc.Pos})
			continue
		}

		// ── Link/image closer `]` ───────────────────────────────────────────
		if ch == ']' {
			sc.Skip(1)

			// Handle deactivated non-image bracket opener
			if len(bracketStack) > 0 {
				topIdx := bracketStack[len(bracketStack)-1]
				if topTok, ok := tokens[topIdx].(*bracketToken); ok && !topTok.active && !topTok.isImage {
					bracketStack = bracketStack[:len(bracketStack)-1]
					textBuf.WriteByte(']')
					continue
				}
			}

			openerStackIdx := findActiveBracketOpener(bracketStack, tokens)

			if openerStackIdx == -1 {
				textBuf.WriteByte(']')
				continue
			}

			openerTokenIdx := bracketStack[openerStackIdx]
			opener := tokens[openerTokenIdx].(*bracketToken)

			flushText()

			closerPos := sc.Pos - 1
			innerTextForLabel := sc.Source[opener.sourcePos:closerPos]

			linkResult := tryLinkAfterClose(sc, linkRefs, innerTextForLabel)

			if linkResult == nil {
				// No valid link — deactivate opener
				opener.active = false
				bracketStack = append(bracketStack[:openerStackIdx], bracketStack[openerStackIdx+1:]...)
				textBuf.WriteByte(']')
				continue
			}

			flushText()

			// Collect inner tokens
			innerTokens := make([]inlineToken, len(tokens[openerTokenIdx+1:]))
			copy(innerTokens, tokens[openerTokenIdx+1:])
			tokens = tokens[:openerTokenIdx]
			bracketStack = append(bracketStack[:openerStackIdx], bracketStack[openerStackIdx+1:]...)

			innerNodes := resolveEmphasis(innerTokens)

			if opener.isImage {
				altText := extractPlainText(innerNodes)
				img := &documentast.ImageNode{
					Destination: linkResult.destination,
					Alt:         altText,
				}
				if linkResult.hasTitle {
					img.Title = linkResult.title
					img.HasTitle = true
				}
				tokens = append(tokens, &nodeToken{node: img})
			} else {
				link := &documentast.LinkNode{
					Destination: linkResult.destination,
					Children:    innerNodes,
				}
				if linkResult.hasTitle {
					link.Title = linkResult.title
					link.HasTitle = true
				}
				tokens = append(tokens, &nodeToken{node: link})
				// Deactivate all preceding non-image link openers
				for k := len(bracketStack) - 1; k >= 0; k-- {
					idx := bracketStack[k]
					if t, ok := tokens[idx].(*bracketToken); ok && !t.isImage {
						t.active = false
					}
				}
			}
			continue
		}

		// ── 8. Emphasis / strong delimiter run ──────────────────────────────
		if ch == '*' || ch == '_' {
			flushText()
			delim := scanDelimiterRun(sc)
			tokens = append(tokens, delim)
			continue
		}

		// ── 6 & 7. Line breaks ──────────────────────────────────────────────
		if ch == '\n' {
			sc.Skip(1)
			buf := textBuf.String()
			if strings.HasSuffix(buf, "  ") || trailingSpaceTabRe.MatchString(buf) {
				trimmed := strings.TrimRight(buf, " \t")
				textBuf.Reset()
				textBuf.WriteString(trimmed)
				flushText()
				tokens = append(tokens, &nodeToken{node: &documentast.HardBreakNode{}})
			} else {
				trimmed := strings.TrimRight(buf, " \t")
				textBuf.Reset()
				textBuf.WriteString(trimmed)
				flushText()
				tokens = append(tokens, &nodeToken{node: &documentast.SoftBreakNode{}})
			}
			continue
		}

		// ── Regular character ────────────────────────────────────────────────
		r := sc.Advance()
		textBuf.WriteRune(r)
	}

	flushText()

	// ─── Resolve Phase ────────────────────────────────────────────────────────
	return resolveEmphasis(tokens)
}

var entityInlineRe = regexp.MustCompile(`^&(?:#[xX][0-9a-fA-F]{1,6}|#[0-9]{1,7}|[a-zA-Z][a-zA-Z0-9]{0,31});`)
var trailingSpaceTabRe = regexp.MustCompile(`[ \t]{2,}$`)

// ─── Inline Token Types ───────────────────────────────────────────────────────

// inlineToken is the union type for tokens during the scan phase.
type inlineToken interface {
	tokenKind() string
}

// nodeToken wraps a fully-resolved inline node.
type nodeToken struct {
	node documentast.InlineNode
}

func (t *nodeToken) tokenKind() string { return "node" }

// delimiterToken represents a run of * or _.
type delimiterToken struct {
	char     byte // '*' or '_'
	count    int
	canOpen  bool
	canClose bool
	active   bool
}

func (t *delimiterToken) tokenKind() string { return "delimiter" }

// bracketToken represents an open [ or ![.
type bracketToken struct {
	isImage   bool
	active    bool
	sourcePos int
}

func (t *bracketToken) tokenKind() string { return "bracket" }

// ─── Delimiter Run Scanning ───────────────────────────────────────────────────

// scanDelimiterRun scans a run of * or _ and returns a delimiterToken with
// flanking classification.
//
// # Flanking Rules (CommonMark spec §6.2)
//
// A delimiter run of `*` is LEFT-FLANKING (can open) if:
//   (a) not followed by Unicode whitespace, AND
//   (b) either not followed by Unicode punctuation,
//       OR preceded by Unicode whitespace or Unicode punctuation.
//
// A delimiter run of `*` is RIGHT-FLANKING (can close) if:
//   (a) not preceded by Unicode whitespace, AND
//   (b) either not preceded by Unicode punctuation,
//       OR followed by Unicode whitespace or Unicode punctuation.
//
// For `_`, the rules are stricter to prevent intra-word emphasis.
func scanDelimiterRun(sc *Scanner) *delimiterToken {
	source := sc.Source
	runStart := sc.Pos
	char := source[runStart]

	// Get the character before the run (for flanking detection)
	var preChar rune
	if runStart > 0 {
		preChar, _ = prevRune(source, runStart)
	}

	run := sc.ConsumeWhile(func(b byte) bool { return b == char })
	count := len(run)

	var postChar rune
	if sc.Pos < len(source) {
		postChar, _ = nextRune(source, sc.Pos)
	}

	afterWhitespace := postChar == 0 || IsUnicodeWhitespace(postChar)
	afterPunctuation := postChar != 0 && IsUnicodePunctuation(postChar)
	beforeWhitespace := preChar == 0 || IsUnicodeWhitespace(preChar)
	beforePunctuation := preChar != 0 && IsUnicodePunctuation(preChar)

	leftFlanking := !afterWhitespace && (!afterPunctuation || beforeWhitespace || beforePunctuation)
	rightFlanking := !beforeWhitespace && (!beforePunctuation || afterWhitespace || afterPunctuation)

	var canOpen, canClose bool
	if char == '*' {
		canOpen = leftFlanking
		canClose = rightFlanking
	} else { // '_'
		canOpen = leftFlanking && (!rightFlanking || beforePunctuation)
		canClose = rightFlanking && (!leftFlanking || afterPunctuation)
	}

	return &delimiterToken{
		char:     char,
		count:    count,
		canOpen:  canOpen,
		canClose: canClose,
		active:   true,
	}
}

// prevRune returns the rune immediately before position pos in s.
func prevRune(s string, pos int) (rune, int) {
	if pos <= 0 {
		return 0, 0
	}
	// Walk backwards to find the start of the previous rune
	end := pos
	for end > 0 {
		end--
		if isRuneStart(s[end]) {
			r, size := decodeRune(s[end:pos])
			_ = size
			return r, end
		}
	}
	return 0, 0
}

// nextRune returns the rune starting at position pos in s.
func nextRune(s string, pos int) (rune, int) {
	if pos >= len(s) {
		return 0, 0
	}
	r, size := decodeRune(s[pos:])
	return r, size
}

// isRuneStart returns true if b is the start byte of a UTF-8 sequence.
func isRuneStart(b byte) bool {
	return b&0xC0 != 0x80
}

func decodeRune(s string) (rune, int) {
	if len(s) == 0 {
		return 0, 0
	}
	b := s[0]
	if b < 0x80 {
		return rune(b), 1
	}
	if b < 0xC0 {
		return 0xFFFD, 1
	}
	if b < 0xE0 {
		if len(s) < 2 {
			return 0xFFFD, 1
		}
		return rune(b&0x1F)<<6 | rune(s[1]&0x3F), 2
	}
	if b < 0xF0 {
		if len(s) < 3 {
			return 0xFFFD, 1
		}
		return rune(b&0x0F)<<12 | rune(s[1]&0x3F)<<6 | rune(s[2]&0x3F), 3
	}
	if len(s) < 4 {
		return 0xFFFD, 1
	}
	return rune(b&0x07)<<18 | rune(s[1]&0x3F)<<12 | rune(s[2]&0x3F)<<6 | rune(s[3]&0x3F), 4
}

// ─── Emphasis Resolution ──────────────────────────────────────────────────────
//
// Implements the CommonMark Appendix A delimiter stack algorithm.
//
// Walk the token list left-to-right looking for closers. For each closer,
// search backwards for the nearest compatible opener (same character, can open).
// When a pair is found, wrap the tokens between them in an emphasis or strong
// node and continue scanning.
//
// Key rules from the spec:
//
//  1. Opener and closer must use the same character (* or _).
//  2. We prefer strong (length 2) over emphasis (length 1) when both sides
//     have enough characters.
//  3. Mod-3 rule: if the sum of opener+closer lengths is divisible by 3,
//     and either side can BOTH open and close, the pair is invalid — UNLESS
//     both lengths are individually divisible by 3.
//  4. After matching, remaining delimiter characters stay as new delimiters.

func resolveEmphasis(tokens []inlineToken) []documentast.InlineNode {
	i := 0
	for i < len(tokens) {
		tok := tokens[i]
		closer, ok := tok.(*delimiterToken)
		if !ok || !closer.canClose || !closer.active {
			i++
			continue
		}

		// Search backwards for an opener
		openerIdx := -1
		for j := i - 1; j >= 0; j-- {
			t, ok2 := tokens[j].(*delimiterToken)
			if !ok2 || !t.canOpen || !t.active || t.char != closer.char {
				continue
			}
			// Mod-3 rule
			if (t.canOpen && t.canClose) || (closer.canOpen && closer.canClose) {
				if (t.count+closer.count)%3 == 0 && t.count%3 != 0 {
					continue
				}
			}
			openerIdx = j
			break
		}

		if openerIdx == -1 {
			i++
			continue
		}

		opener := tokens[openerIdx].(*delimiterToken)

		useLen := 1
		if opener.count >= 2 && closer.count >= 2 {
			useLen = 2
		}
		isStrong := useLen == 2

		// Collect inner tokens
		innerSlice := make([]inlineToken, i-openerIdx-1)
		copy(innerSlice, tokens[openerIdx+1:i])
		innerNodes := resolveEmphasis(innerSlice)

		var emphNode documentast.InlineNode
		if isStrong {
			emphNode = &documentast.StrongNode{Children: innerNodes}
		} else {
			emphNode = &documentast.EmphasisNode{Children: innerNodes}
		}

		// Replace inner tokens with the emphasis node
		newTokens := make([]inlineToken, 0, len(tokens)-(i-openerIdx-1)+1)
		newTokens = append(newTokens, tokens[:openerIdx+1]...)
		newTokens = append(newTokens, &nodeToken{node: emphNode})
		newTokens = append(newTokens, tokens[i:]...)
		tokens = newTokens
		// After splice, closer is now at openerIdx + 2

		opener.count -= useLen
		closer.count -= useLen

		if opener.count == 0 {
			tokens = append(tokens[:openerIdx], tokens[openerIdx+1:]...)
			i = openerIdx + 1
		} else {
			i = openerIdx + 2
		}

		if closer.count == 0 {
			tokens = append(tokens[:i], tokens[i+1:]...)
		}
	}

	// Convert remaining tokens to InlineNodes
	result := make([]documentast.InlineNode, 0, len(tokens))
	for _, tok := range tokens {
		switch t := tok.(type) {
		case *nodeToken:
			result = append(result, t.node)
		case *bracketToken:
			var literal string
			if t.isImage {
				literal = "!["
			} else {
				literal = "["
			}
			result = append(result, &documentast.TextNode{Value: literal})
		case *delimiterToken:
			result = append(result, &documentast.TextNode{
				Value: strings.Repeat(string(rune(t.char)), t.count),
			})
		}
	}
	return result
}

// ─── Code Span ───────────────────────────────────────────────────────────────

// tryCodeSpan attempts to parse a code span starting at the scanner position.
//
// A code span opens with N backticks and closes with the next run of exactly
// N backticks. The content normalisation per spec §6.1:
//  1. CR/LF/newline → space
//  2. If content has a non-space char AND starts and ends with exactly one
//     space, strip those surrounding spaces.
func tryCodeSpan(sc *Scanner) *documentast.CodeSpanNode {
	savedPos := sc.Pos
	openTicks := sc.ConsumeWhile(func(b byte) bool { return b == '`' })
	tickLen := len(openTicks)

	var content strings.Builder
	for !sc.Done() {
		if sc.PeekByte() == '`' {
			closeTicks := sc.ConsumeWhile(func(b byte) bool { return b == '`' })
			if len(closeTicks) == tickLen {
				// Matching close found — normalise
				c := content.String()
				c = strings.ReplaceAll(c, "\r\n", " ")
				c = strings.ReplaceAll(c, "\r", " ")
				c = strings.ReplaceAll(c, "\n", " ")
				if len(c) >= 2 && c[0] == ' ' && c[len(c)-1] == ' ' && strings.TrimSpace(c) != "" {
					c = c[1 : len(c)-1]
				}
				return &documentast.CodeSpanNode{Value: c}
			}
			content.WriteString(closeTicks)
		} else {
			content.WriteRune(sc.Advance())
		}
	}

	sc.Pos = savedPos
	return nil
}

// ─── HTML Inline ─────────────────────────────────────────────────────────────

// tryHtmlInline attempts to parse an inline HTML construct starting at `<`.
//
// CommonMark spec §6.6 defines six inline HTML forms:
//  1. Open tag:           `<tagname attr="val">`
//  2. Closing tag:        `</tagname>`
//  3. HTML comment:       `<!-- content -->`
//  4. Processing instr:   `<?content?>`
//  5. Declaration:        `<!UPPER content>`
//  6. CDATA section:      `<![CDATA[content]]>`
func tryHtmlInline(sc *Scanner) *documentast.RawInlineNode {
	if sc.PeekByte() != '<' {
		return nil
	}
	savedPos := sc.Pos
	sc.Skip(1) // consume `<`

	// HTML comment: <!-- ... -->
	if sc.Match("!--") {
		contentStart := sc.Pos
		// Comment content must not start with `>` or `->`
		if sc.PeekByte() == '>' || sc.PeekSlice(2) == "->" {
			invalid := ">"
			if sc.PeekSlice(2) == "->" {
				invalid = "->"
			}
			sc.Skip(len(invalid))
			return &documentast.RawInlineNode{Format: "html", Value: sc.Source[savedPos:sc.Pos]}
		}
		for !sc.Done() {
			if sc.Match("-->") {
				content := sc.Source[contentStart : sc.Pos-3]
				if strings.HasSuffix(content, "-") {
					sc.Pos = savedPos
					return nil
				}
				return &documentast.RawInlineNode{Format: "html", Value: sc.Source[savedPos:sc.Pos]}
			}
			sc.Skip(1)
		}
		sc.Pos = savedPos
		return nil
	}

	// Processing instruction: <? ... ?>
	if sc.Match("?") {
		for !sc.Done() {
			if sc.Match("?>") {
				return &documentast.RawInlineNode{Format: "html", Value: sc.Source[savedPos:sc.Pos]}
			}
			sc.Skip(1)
		}
		sc.Pos = savedPos
		return nil
	}

	// CDATA section: <![CDATA[ ... ]]>
	if sc.Match("![CDATA[") {
		for !sc.Done() {
			if sc.Match("]]>") {
				return &documentast.RawInlineNode{Format: "html", Value: sc.Source[savedPos:sc.Pos]}
			}
			sc.Skip(1)
		}
		sc.Pos = savedPos
		return nil
	}

	// Declaration: <!UPPER...>
	if sc.Match("!") {
		if ch := sc.PeekByte(); ch >= 'A' && ch <= 'Z' {
			sc.ConsumeWhile(func(b byte) bool { return b != '>' })
			if sc.Match(">") {
				return &documentast.RawInlineNode{Format: "html", Value: sc.Source[savedPos:sc.Pos]}
			}
		}
		sc.Pos = savedPos
		return nil
	}

	// Closing tag: </tagname>
	if sc.PeekByte() == '/' {
		sc.Skip(1)
		tag := sc.ConsumeWhile(func(b byte) bool {
			c := rune(b)
			return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '-'
		})
		if len(tag) == 0 {
			sc.Pos = savedPos
			return nil
		}
		sc.SkipSpaces()
		if !sc.Match(">") {
			sc.Pos = savedPos
			return nil
		}
		return &documentast.RawInlineNode{Format: "html", Value: sc.Source[savedPos:sc.Pos]}
	}

	// Open tag: <tagname attr...> or <tagname attr.../>
	ch := sc.PeekByte()
	if (ch >= 'a' && ch <= 'z') || (ch >= 'A' && ch <= 'Z') {
		tagName := sc.ConsumeWhile(func(b byte) bool {
			c := rune(b)
			return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '-'
		})
		if len(tagName) == 0 {
			sc.Pos = savedPos
			return nil
		}

		newlinesInTag := 0

		for {
			spaceLen := sc.SkipSpaces()
			// Allow at most one newline in the attribute area
			if newlinesInTag == 0 && sc.PeekByte() == '\n' {
				newlinesInTag++
				sc.Skip(1)
				spaceLen += 1 + sc.SkipSpaces()
			}
			next := sc.PeekByte()
			if next == '>' || next == '/' || next == 0 {
				break
			}
			// Second newline → invalid
			if next == '\n' {
				sc.Pos = savedPos
				return nil
			}
			// Each attribute must be preceded by whitespace
			if spaceLen == 0 {
				sc.Pos = savedPos
				return nil
			}
			// Attribute name: must start with ASCII alpha, _, or :
			if !isAttrNameStart(next) {
				sc.Pos = savedPos
				return nil
			}
			sc.ConsumeWhile(func(b byte) bool {
				return isAttrNameContinue(b)
			})

			// Optional `= value`
			posBeforeEqSpaces := sc.Pos
			sc.SkipSpaces()
			if sc.PeekByte() == '=' {
				sc.Skip(1) // consume `=`
				sc.SkipSpaces()
				q := sc.PeekByte()
				if q == '"' || q == '\'' {
					sc.Skip(1)
					closed := false
					for !sc.Done() {
						vc := sc.Source[sc.Pos]
						if vc == q {
							sc.Skip(1)
							closed = true
							break
						}
						if vc == '\n' {
							if newlinesInTag >= 1 {
								sc.Pos = savedPos
								return nil
							}
							newlinesInTag++
						}
						sc.Skip(1)
					}
					if !closed {
						sc.Pos = savedPos
						return nil
					}
				} else {
					// Unquoted value
					unquoted := sc.ConsumeWhile(func(b byte) bool {
						return b != ' ' && b != '\t' && b != '\n' && b != '"' && b != '\'' &&
							b != '=' && b != '<' && b != '>' && b != '`'
					})
					if len(unquoted) == 0 {
						sc.Pos = savedPos
						return nil
					}
				}
			} else {
				sc.Pos = posBeforeEqSpaces
			}
		}

		if sc.Match("/>") || sc.Match(">") {
			return &documentast.RawInlineNode{Format: "html", Value: sc.Source[savedPos:sc.Pos]}
		}
		sc.Pos = savedPos
		return nil
	}

	sc.Pos = savedPos
	return nil
}

func isAttrNameStart(b byte) bool {
	return (b >= 'a' && b <= 'z') || (b >= 'A' && b <= 'Z') || b == '_' || b == ':'
}

func isAttrNameContinue(b byte) bool {
	return (b >= 'a' && b <= 'z') || (b >= 'A' && b <= 'Z') || (b >= '0' && b <= '9') ||
		b == '_' || b == ':' || b == '.' || b == '-'
}

// ─── Autolink ────────────────────────────────────────────────────────────────

// tryAutolink attempts to parse an autolink: `<URI>` or `<email>`.
func tryAutolink(sc *Scanner) *documentast.AutolinkNode {
	if sc.PeekByte() != '<' {
		return nil
	}
	savedPos := sc.Pos
	sc.Skip(1)

	start := sc.Pos

	// Try email autolink: local@domain
	localPart := sc.ConsumeWhile(func(b byte) bool {
		return b != ' ' && b != '<' && b != '>' && b != '@' && b != '\n'
	})
	if len(localPart) > 0 && sc.PeekByte() == '@' {
		sc.Skip(1)
		domainPart := sc.ConsumeWhile(func(b byte) bool {
			return b != ' ' && b != '<' && b != '>' && b != '\n'
		})
		if len(domainPart) > 0 && sc.Match(">") {
			if isValidEmailLocalPart(localPart) && isValidEmailDomain(domainPart) {
				return &documentast.AutolinkNode{
					Destination: localPart + "@" + domainPart,
					IsEmail:     true,
				}
			}
		}
	}

	// Retry as URL autolink
	sc.Pos = start
	scheme := sc.ConsumeWhile(func(b byte) bool {
		return (b >= 'a' && b <= 'z') || (b >= 'A' && b <= 'Z') || (b >= '0' && b <= '9') ||
			b == '+' || b == '-' || b == '.'
	})
	if len(scheme) >= 2 && len(scheme) <= 32 && sc.Match(":") {
		path := sc.ConsumeWhile(func(b byte) bool {
			return b != ' ' && b != '<' && b != '>' && b != '\n'
		})
		if sc.Match(">") {
			return &documentast.AutolinkNode{
				Destination: scheme + ":" + path,
				IsEmail:     false,
			}
		}
	}

	sc.Pos = savedPos
	return nil
}

// isValidEmailLocalPart checks the email local part per CommonMark spec.
var emailLocalPartRe = regexp.MustCompile(`^[a-zA-Z0-9.!#$%&'*+/=?^_{|}~\-]+$`)
var emailDomainRe = regexp.MustCompile(`^[a-zA-Z0-9]([a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?(\.[a-zA-Z0-9]([a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?)*$`)

func isValidEmailLocalPart(s string) bool {
	return emailLocalPartRe.MatchString(s)
}

func isValidEmailDomain(s string) bool {
	return emailDomainRe.MatchString(s)
}

// ─── Link / Image Destination Parsing ────────────────────────────────────────

type linkResult struct {
	destination string
	title       string
	hasTitle    bool
}

// tryLinkAfterClose attempts to parse an inline link `(dest "title")`, a full
// reference `[label]`, a collapsed reference `[]`, or a shortcut reference.
func tryLinkAfterClose(
	sc *Scanner,
	linkRefs map[string]*linkReference,
	innerText string,
) *linkResult {
	savedPos := sc.Pos

	// ── Inline link: ( destination "title" ) ────────────────────────────────
	if sc.PeekByte() == '(' {
		result := tryInlineLinkDest(sc, savedPos)
		if result != nil {
			return result
		}
		sc.Pos = savedPos
	}

	// ── Full reference: [label] or Collapsed reference: [] ──────────────────
	if sc.PeekByte() == '[' {
		sc.Skip(1)
		var labelBuf strings.Builder
		validLabel := true
		for !sc.Done() {
			c := sc.PeekByte()
			if c == ']' {
				sc.Skip(1)
				break
			}
			if c == '\n' || c == '[' {
				validLabel = false
				break
			}
			if c == '\\' {
				sc.Skip(1)
				if !sc.Done() {
					labelBuf.WriteByte('\\')
					labelBuf.WriteByte(sc.Source[sc.Pos])
					sc.Skip(1)
				}
			} else {
				labelBuf.WriteByte(c)
				sc.Skip(1)
			}
		}
		if validLabel {
			rawLabel := labelBuf.String()
			if strings.TrimSpace(rawLabel) != "" {
				label := NormalizeLinkLabel(rawLabel)
				if ref, ok := linkRefs[label]; ok {
					return &linkResult{destination: ref.destination, title: ref.title, hasTitle: ref.hasTitle}
				}
			} else {
				// Collapsed reference: [] — use inner text as label
				label := NormalizeLinkLabel(innerText)
				if ref, ok := linkRefs[label]; ok {
					return &linkResult{destination: ref.destination, title: ref.title, hasTitle: ref.hasTitle}
				}
			}
		}
		sc.Pos = savedPos
		return nil
	}

	// ── Shortcut reference: no `(` or `[` follows ─────────────────────────
	label := NormalizeLinkLabel(innerText)
	if ref, ok := linkRefs[label]; ok {
		return &linkResult{destination: ref.destination, title: ref.title, hasTitle: ref.hasTitle}
	}

	return nil
}

// tryInlineLinkDest attempts to parse an inline link destination and optional title.
func tryInlineLinkDest(sc *Scanner, savedPos int) *linkResult {
	sc.Skip(1) // consume `(`
	skipOptionalSpacesAndNewline(sc)

	var destination string

	if sc.PeekByte() == '<' {
		// Angle-bracket destination
		sc.Skip(1)
		var destBuf strings.Builder
		for !sc.Done() {
			c := sc.PeekByte()
			if c == '\n' || c == '\r' {
				return nil
			}
			if c == '\\' {
				sc.Skip(1)
				next := sc.Source[sc.Pos]
				if IsAsciiPunctuation(rune(next)) {
					destBuf.WriteByte(next)
				} else {
					destBuf.WriteByte('\\')
					destBuf.WriteByte(next)
				}
				sc.Skip(1)
			} else if c == '>' {
				sc.Skip(1)
				break
			} else if c == '<' {
				return nil
			} else {
				destBuf.WriteByte(c)
				sc.Skip(1)
			}
		}
		destination = NormalizeURL(DecodeEntities(destBuf.String()))
	} else {
		// Bare destination
		depth := 0
		destStart := sc.Pos
		for !sc.Done() {
			c := sc.PeekByte()
			if c == '(' {
				depth++
				sc.Skip(1)
			} else if c == ')' {
				if depth == 0 {
					break
				}
				depth--
				sc.Skip(1)
			} else if c == '\\' {
				sc.Skip(2)
			} else if IsAsciiWhitespace(rune(c)) {
				break
			} else {
				sc.Skip(1)
			}
		}
		destRaw := sc.Source[destStart:sc.Pos]
		destination = NormalizeURL(DecodeEntities(applyBackslashEscapes(destRaw)))
	}

	skipOptionalSpacesAndNewline(sc)

	// Optional title
	var title string
	hasTitle := false
	q := sc.PeekByte()
	if q == '"' || q == '\'' || q == '(' {
		closeQ := q
		if q == '(' {
			closeQ = ')'
		}
		sc.Skip(1)
		var titleBuf strings.Builder
		for !sc.Done() {
			c := sc.PeekByte()
			if c == '\\' {
				sc.Skip(1)
				if sc.Done() {
					break
				}
				next := sc.Source[sc.Pos]
				if IsAsciiPunctuation(rune(next)) {
					titleBuf.WriteByte(next)
				} else {
					titleBuf.WriteByte('\\')
					titleBuf.WriteByte(next)
				}
				sc.Skip(1)
			} else if c == closeQ {
				sc.Skip(1)
				title = DecodeEntities(titleBuf.String())
				hasTitle = true
				break
			} else if c == '\n' && q == '(' {
				break // parens title cannot span lines
			} else {
				titleBuf.WriteByte(c)
				sc.Skip(1)
			}
		}
	}

	sc.SkipSpaces()
	if !sc.Match(")") {
		return nil
	}
	return &linkResult{destination: destination, title: title, hasTitle: hasTitle}
}

// skipOptionalSpacesAndNewline skips ASCII spaces/tabs and at most one newline.
func skipOptionalSpacesAndNewline(sc *Scanner) {
	sc.SkipSpaces()
	if sc.PeekByte() == '\n' {
		sc.Skip(1)
		sc.SkipSpaces()
	} else if sc.PeekByte() == '\r' && sc.PeekByteAt(1) == '\n' {
		sc.Skip(2)
		sc.SkipSpaces()
	}
}

// findActiveBracketOpener finds the most recent active bracket opener.
// Returns the index into bracketStack, or -1 if none.
func findActiveBracketOpener(bracketStack []int, tokens []inlineToken) int {
	for i := len(bracketStack) - 1; i >= 0; i-- {
		idx := bracketStack[i]
		if t, ok := tokens[idx].(*bracketToken); ok && t.active {
			return i
		}
	}
	return -1
}

// extractPlainText recursively extracts plain text from inline nodes.
// Used for image alt attributes and link label fallback.
func extractPlainText(nodes []documentast.InlineNode) string {
	var b strings.Builder
	for _, node := range nodes {
		switch n := node.(type) {
		case *documentast.TextNode:
			b.WriteString(n.Value)
		case *documentast.CodeSpanNode:
			b.WriteString(n.Value)
		case *documentast.HardBreakNode:
			b.WriteByte('\n')
		case *documentast.SoftBreakNode:
			b.WriteByte(' ')
		case *documentast.EmphasisNode:
			b.WriteString(extractPlainText(n.Children))
		case *documentast.StrongNode:
			b.WriteString(extractPlainText(n.Children))
		case *documentast.LinkNode:
			b.WriteString(extractPlainText(n.Children))
		case *documentast.ImageNode:
			b.WriteString(n.Alt)
		case *documentast.AutolinkNode:
			b.WriteString(n.Destination)
		}
	}
	return b.String()
}

// ─── Document-Level Inline Resolution ────────────────────────────────────────

// resolveInlineContent walks the block AST and fills in inline content for
// headings and paragraphs. The block parser stored raw inline strings keyed
// by integer IDs; this function looks up those strings, parses them, and
// writes the resulting InlineNode slices back into the nodes.
func resolveInlineContent(
	doc *documentast.DocumentNode,
	rawInlineContent map[int]string,
	linkRefs map[string]*linkReference,
) {
	var walk func(block documentast.BlockNode)
	walk = func(block documentast.BlockNode) {
		switch b := block.(type) {
		case *headingNodeWithID:
			raw, ok := rawInlineContent[b.rawID]
			if ok {
				b.HeadingNode.Children = ParseInline(raw, linkRefs)
			}
		case *paragraphNodeWithID:
			raw, ok := rawInlineContent[b.rawID]
			if ok {
				b.ParagraphNode.Children = ParseInline(raw, linkRefs)
			}
		case *documentast.DocumentNode:
			for _, child := range b.Children {
				walk(child)
			}
		case *documentast.BlockquoteNode:
			for _, child := range b.Children {
				walk(child)
			}
		case *documentast.ListNode:
			for _, item := range b.Children {
				walk(item)
			}
		case *documentast.ListItemNode:
			for _, child := range b.Children {
				walk(child)
			}
		}
	}

	walk(doc)
}

// matchAtPos tries to match a regexp anchored at position pos in s.
// Returns the matched string or "".
func matchAtPos(re *regexp.Regexp, s string, pos int) string {
	if pos > len(s) {
		return ""
	}
	loc := re.FindStringIndex(s[pos:])
	if loc == nil || loc[0] != 0 {
		return ""
	}
	return s[pos : pos+loc[1]]
}
