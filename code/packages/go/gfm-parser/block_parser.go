package commonmarkparser

import (
	"regexp"
	"strconv"
	"strings"

	documentast "github.com/adhithyan15/coding-adventures/code/packages/go/document-ast"
)

// ─── Internal Mutable Block Types ────────────────────────────────────────────
//
// During Phase 1 (block parsing), we build a mutable intermediate tree using
// these internal types. They are then converted to the immutable DocumentAST
// types in convertToAST().
//
// Why mutable intermediates? Because the block parser modifies blocks as it
// processes more lines — e.g. paragraphs accumulate lines, lists track tightness.
// The final AST conversion freezes everything into the DocumentAST types.

type blockKind int

const (
	kindDocument blockKind = iota
	kindBlockquote
	kindList
	kindListItem
	kindParagraph
	kindFencedCode
	kindIndentedCode
	kindHtmlBlock
	kindHeading
	kindThematicBreak
	kindLinkDef
)

// mutableBlock is the base interface for all mutable intermediate blocks.
type mutableBlock interface {
	kind() blockKind
}

type mutableDocument struct {
	children []mutableBlock
}

func (b *mutableDocument) kind() blockKind { return kindDocument }

type mutableBlockquote struct {
	children []mutableBlock
}

func (b *mutableBlockquote) kind() blockKind { return kindBlockquote }

type mutableList struct {
	ordered      bool
	marker       string // - * + . )
	start        int
	tight        bool
	items        []*mutableListItem
	hadBlankLine bool
}

func (b *mutableList) kind() blockKind { return kindList }

type mutableListItem struct {
	marker        string
	markerIndent  int
	contentIndent int
	children      []mutableBlock
	hadBlankLine  bool
}

func (b *mutableListItem) kind() blockKind { return kindListItem }

type mutableParagraph struct {
	lines []string
}

func (b *mutableParagraph) kind() blockKind { return kindParagraph }

type mutableFencedCode struct {
	fence      string // fence opener chars (``` or ~~~)
	fenceLen   int
	baseIndent int
	infoString string
	lines      []string
	closed     bool
}

func (b *mutableFencedCode) kind() blockKind { return kindFencedCode }

type mutableIndentedCode struct {
	lines []string
}

func (b *mutableIndentedCode) kind() blockKind { return kindIndentedCode }

type mutableHtmlBlock struct {
	htmlType int // 1–7
	lines    []string
	closed   bool
}

func (b *mutableHtmlBlock) kind() blockKind { return kindHtmlBlock }

type mutableHeading struct {
	level   int // 1–6
	content string
}

func (b *mutableHeading) kind() blockKind { return kindHeading }

type mutableThematicBreak struct{}

func (b *mutableThematicBreak) kind() blockKind { return kindThematicBreak }

type mutableLinkDef struct {
	label       string
	destination string
	title       string
	hasTitle    bool
}

func (b *mutableLinkDef) kind() blockKind { return kindLinkDef }

// ─── HTML Block Patterns ──────────────────────────────────────────────────────

var (
	htmlBlock1Open  = regexp.MustCompile(`(?i)^<(?:script|pre|textarea|style)(?:\s|>|$)`)
	htmlBlock1Close = regexp.MustCompile(`(?i)<\/(?:script|pre|textarea|style)>`)
	htmlBlock2Open  = regexp.MustCompile(`^<!--`)
	htmlBlock2Close = regexp.MustCompile(`--!?>`)
	htmlBlock3Open  = regexp.MustCompile(`^<\?`)
	htmlBlock3Close = regexp.MustCompile(`\?>`)
	htmlBlock4Open  = regexp.MustCompile(`^<![A-Z]`)
	htmlBlock4Close = regexp.MustCompile(`>`)
	htmlBlock5Open  = regexp.MustCompile(`^<!\[CDATA\[`)
	htmlBlock5Close = regexp.MustCompile(`\]\]>`)
)

// HTML block type 6: block-level HTML element open/close tag
var htmlBlock6Tags = map[string]bool{
	"address": true, "article": true, "aside": true, "base": true,
	"basefont": true, "blockquote": true, "body": true, "caption": true,
	"center": true, "col": true, "colgroup": true, "dd": true,
	"details": true, "dialog": true, "dir": true, "div": true,
	"dl": true, "dt": true, "fieldset": true, "figcaption": true,
	"figure": true, "footer": true, "form": true, "frame": true,
	"frameset": true, "h1": true, "h2": true, "h3": true, "h4": true,
	"h5": true, "h6": true, "head": true, "header": true, "hr": true,
	"html": true, "iframe": true, "legend": true, "li": true, "link": true,
	"main": true, "menu": true, "menuitem": true, "meta": true, "nav": true,
	"noframes": true, "ol": true, "optgroup": true, "option": true,
	"p": true, "param": true, "search": true, "section": true,
	"summary": true, "table": true, "tbody": true, "td": true,
	"tfoot": true, "th": true, "thead": true, "title": true, "tr": true,
	"track": true, "ul": true,
}

// htmlBlock6Pattern matches an open or close block-level HTML tag.
// Built lazily from the tag set.
var htmlBlock6Pattern = buildHtmlBlock6Pattern()

func buildHtmlBlock6Pattern() *regexp.Regexp {
	tags := make([]string, 0, len(htmlBlock6Tags))
	for tag := range htmlBlock6Tags {
		tags = append(tags, tag)
	}
	// Sort for determinism (not required for correctness but aids debugging)
	return regexp.MustCompile(`(?i)^</?(?:` + strings.Join(tags, "|") + `)(?:\s|>|/>|$)`)
}

// HTML block type 7: complete open/close tag not in type 6 list
var (
	html7AttrPart = `(?:\s+[a-zA-Z_:][a-zA-Z0-9_:.\-]*(?:\s*=\s*(?:[^\s"'=<>` + "`" + `]+|'[^'\n]*'|"[^"\n]*"))?)`
	html7OpenTag  = regexp.MustCompile(`^<[A-Za-z][A-Za-z0-9\-]*(` + html7AttrPart + `)*\s*/?>$`)
	html7CloseTag = regexp.MustCompile(`^<\/[A-Za-z][A-Za-z0-9\-]*\s*>$`)
)

// detectHtmlBlockType detects which type (1–7) of HTML block a line starts,
// or returns 0 if not an HTML block opener.
func detectHtmlBlockType(line string) int {
	stripped := strings.TrimLeft(line, " \t")
	if htmlBlock1Open.MatchString(stripped) {
		return 1
	}
	if htmlBlock2Open.MatchString(stripped) {
		return 2
	}
	if htmlBlock3Open.MatchString(stripped) {
		return 3
	}
	if htmlBlock4Open.MatchString(stripped) {
		return 4
	}
	if htmlBlock5Open.MatchString(stripped) {
		return 5
	}
	if htmlBlock6Pattern.MatchString(stripped) {
		return 6
	}
	if html7OpenTag.MatchString(stripped) || html7CloseTag.MatchString(stripped) {
		return 7
	}
	return 0
}

// htmlBlockEnds returns true if the line signals the end of the given HTML block type.
func htmlBlockEnds(line string, htmlType int) bool {
	switch htmlType {
	case 1:
		return htmlBlock1Close.MatchString(line)
	case 2:
		return htmlBlock2Close.MatchString(line)
	case 3:
		return htmlBlock3Close.MatchString(line)
	case 4:
		return htmlBlock4Close.MatchString(line)
	case 5:
		return htmlBlock5Close.MatchString(line)
	case 6, 7:
		return isBlank(line)
	}
	return false
}

// ─── Line Classification Helpers ─────────────────────────────────────────────

// isBlank returns true if the line is empty or contains only whitespace.
func isBlank(line string) bool {
	for i := 0; i < len(line); i++ {
		ch := line[i]
		if ch != ' ' && ch != '\t' && ch != '\r' && ch != '\f' {
			return false
		}
	}
	return true
}

// indentOf counts the virtual leading spaces of line, expanding tabs to
// 4-column tab stops. baseCol is the virtual column of line[0] in the
// original document.
func indentOf(line string, baseCol int) int {
	col := baseCol
	for _, ch := range line {
		if ch == ' ' {
			col++
		} else if ch == '\t' {
			col += 4 - (col % 4)
		} else {
			break
		}
	}
	return col - baseCol
}

// stripIndent strips exactly n virtual spaces of leading indentation from line,
// expanding tabs correctly relative to baseCol.
//
// Returns [strippedLine, nextBaseCol].
// nextBaseCol is the virtual column of strippedLine[0].
//
// When a tab spans the strip boundary (would expand past n virtual spaces),
// the tab is consumed and leftover virtual spaces are prepended to the result.
func stripIndent(line string, n int, baseCol int) (string, int) {
	remaining := n
	col := baseCol
	i := 0
	for remaining > 0 && i < len(line) {
		ch := line[i]
		if ch == ' ' {
			i++
			remaining--
			col++
		} else if ch == '\t' {
			w := 4 - (col % 4)
			if w <= remaining {
				i++
				remaining -= w
				col += w
			} else {
				// Partial tab: consume it, prepend leftover spaces
				leftover := w - remaining
				return strings.Repeat(" ", leftover) + line[i+1:], col + remaining
			}
		} else {
			break
		}
	}
	return line[i:], col
}

// virtualColAfter computes the virtual column after consuming charCount bytes
// from line, starting at virtual column startCol.
func virtualColAfter(line string, charCount int, startCol int) int {
	col := startCol
	for i := 0; i < charCount && i < len(line); i++ {
		if line[i] == '\t' {
			col += 4 - (col % 4)
		} else {
			col++
		}
	}
	return col
}

// applyBackslashEscapes applies backslash escapes — only for ASCII punctuation.
func applyBackslashEscapes(s string) string {
	if !strings.Contains(s, "\\") {
		return s
	}
	var b strings.Builder
	b.Grow(len(s))
	i := 0
	for i < len(s) {
		if s[i] == '\\' && i+1 < len(s) {
			next := rune(s[i+1])
			if IsAsciiPunctuation(next) {
				b.WriteByte(s[i+1])
				i += 2
				continue
			}
		}
		b.WriteByte(s[i])
		i++
	}
	return b.String()
}

// extractInfoString extracts the language identifier from a fenced code opener.
func extractInfoString(line string) string {
	m := infoStringRe.FindStringSubmatch(line)
	if m == nil {
		return ""
	}
	raw := strings.TrimSpace(m[1])
	// Only the first word
	parts := strings.Fields(raw)
	if len(parts) == 0 {
		return ""
	}
	return DecodeEntities(applyBackslashEscapes(parts[0]))
}

var infoStringRe = regexp.MustCompile("^[`~]+\\s*(.*)")

// ─── ATX Heading Parsing ──────────────────────────────────────────────────────

type atxHeading struct {
	level   int
	content string
}

var atxRe = regexp.MustCompile(`^ {0,3}(#{1,6})([ \t]|$)(.*)`)
var closingHashRe = regexp.MustCompile(`[ \t]+#+[ \t]*$`)
var onlyHashRe = regexp.MustCompile(`^#+[ \t]*$`)

// parseAtxHeading tries to parse the line as an ATX heading.
// Returns nil if it's not an ATX heading.
func parseAtxHeading(line string) *atxHeading {
	m := atxRe.FindStringSubmatch(line)
	if m == nil {
		return nil
	}
	hashes := m[1]
	content := strings.TrimRight(m[3], "")
	content = strings.TrimRight(content, " \t")
	// Remove trailing hash sequence
	content = closingHashRe.ReplaceAllString(content, "")
	if onlyHashRe.MatchString(content) {
		content = ""
	}
	return &atxHeading{
		level:   len(hashes),
		content: strings.TrimSpace(content),
	}
}

// ─── Thematic Break ───────────────────────────────────────────────────────────

var thematicBreakRe = regexp.MustCompile(`^ {0,3}((?:\*[ \t]*){3,}|(?:-[ \t]*){3,}|(?:_[ \t]*){3,})\s*$`)

// isThematicBreak returns true if the line is a thematic break.
func isThematicBreak(line string) bool {
	return thematicBreakRe.MatchString(line)
}

// ─── Setext Heading Underlines ────────────────────────────────────────────────

// isSetextUnderline returns 1 if the line is a level-1 setext underline (===+),
// 2 if level-2 (---+), or 0 if neither.
func isSetextUnderline(line string) int {
	if regexp.MustCompile(`^ {0,3}=+\s*$`).MatchString(line) {
		return 1
	}
	if regexp.MustCompile(`^ {0,3}-+\s*$`).MatchString(line) {
		return 2
	}
	return 0
}

// ─── List Marker Parsing ──────────────────────────────────────────────────────

type listMarker struct {
	ordered    bool
	start      int
	marker     string // - * + . )
	markerLen  int
	spaceAfter int
	indent     int
}

var unorderedListRe = regexp.MustCompile(`^( {0,3})([-*+])( +|\t|$)`)
var orderedListRe = regexp.MustCompile(`^( {0,3})(\d{1,9})([.)])( +|\t|$)`)

// parseListMarker attempts to parse a list marker from the start of line.
func parseListMarker(line string) *listMarker {
	if m := unorderedListRe.FindStringSubmatch(line); m != nil {
		indent := len(m[1])
		marker := m[2]
		space := m[3]
		return &listMarker{
			ordered:    false,
			start:      1,
			marker:     marker,
			markerLen:  indent + 1 + len(space),
			spaceAfter: len(space),
			indent:     indent,
		}
	}
	if m := orderedListRe.FindStringSubmatch(line); m != nil {
		indent := len(m[1])
		numStr := m[2]
		delim := m[3]
		space := m[4]
		num, _ := strconv.Atoi(numStr)
		markerWidth := len(numStr) + 1 // digits + delimiter
		return &listMarker{
			ordered:    true,
			start:      num,
			marker:     delim,
			markerLen:  indent + markerWidth + len(space),
			spaceAfter: len(space),
			indent:     indent,
		}
	}
	return nil
}

// ─── Link Reference Definition Parsing ───────────────────────────────────────

type parsedLinkDef struct {
	label         string
	destination   string
	title         string
	hasTitle      bool
	charsConsumed int
}

var linkLabelRe = regexp.MustCompile(`^ {0,3}\[([^\]\\\[]*(?:\\.[^\]\\\[]*)*)\]:`)

// parseLinkDefinition attempts to parse a link reference definition from text.
// Returns nil if the text does not start with a valid link definition.
func parseLinkDefinition(text string) *parsedLinkDef {
	labelMatch := linkLabelRe.FindStringSubmatch(text)
	if labelMatch == nil {
		return nil
	}
	rawLabel := labelMatch[1]
	if strings.TrimSpace(rawLabel) == "" {
		return nil
	}
	label := NormalizeLinkLabel(rawLabel)
	pos := len(labelMatch[0])

	// Skip whitespace (including one newline)
	wsEnd := pos
	for wsEnd < len(text) && (text[wsEnd] == ' ' || text[wsEnd] == '\t') {
		wsEnd++
	}
	if wsEnd < len(text) && text[wsEnd] == '\n' {
		wsEnd++
		for wsEnd < len(text) && (text[wsEnd] == ' ' || text[wsEnd] == '\t') {
			wsEnd++
		}
	}
	pos = wsEnd

	// Destination
	var destination string
	if pos < len(text) && text[pos] == '<' {
		// Angle-bracket destination
		angleEnd := pos + 1
		var destBuf strings.Builder
		for angleEnd < len(text) {
			ch := text[angleEnd]
			if ch == '<' || ch == '\n' {
				return nil
			}
			if ch == '\\' && angleEnd+1 < len(text) {
				angleEnd++
				destBuf.WriteByte(text[angleEnd])
				angleEnd++
				continue
			}
			if ch == '>' {
				destination = NormalizeURL(DecodeEntities(applyBackslashEscapes(destBuf.String())))
				angleEnd++
				break
			}
			destBuf.WriteByte(ch)
			angleEnd++
		}
		if text[angleEnd-1] != '>' {
			return nil
		}
		pos = angleEnd
	} else {
		// Bare destination
		depth := 0
		start := pos
		for pos < len(text) {
			ch := text[pos]
			if ch == '(' {
				depth++
				pos++
			} else if ch == ')' {
				if depth == 0 {
					break
				}
				depth--
				pos++
			} else if ch == ' ' || ch == '\t' || ch == '\n' || (ch >= 0 && ch < 0x20) {
				break
			} else if ch == '\\' {
				pos += 2
			} else {
				pos++
			}
		}
		if pos == start {
			return nil
		}
		destination = NormalizeURL(DecodeEntities(applyBackslashEscapes(text[start:pos])))
	}

	// Optional title
	var title string
	hasTitle := false
	beforeTitle := pos

	// Try to parse optional whitespace + title
	spaceStart := pos
	for pos < len(text) && (text[pos] == ' ' || text[pos] == '\t') {
		pos++
	}
	if pos < len(text) && text[pos] == '\n' {
		pos++
		for pos < len(text) && (text[pos] == ' ' || text[pos] == '\t') {
			pos++
		}
	}

	if pos > spaceStart && pos < len(text) {
		titleChar := text[pos]
		var closeChar byte
		if titleChar == '"' {
			closeChar = '"'
		} else if titleChar == '\'' {
			closeChar = '\''
		} else if titleChar == '(' {
			closeChar = ')'
		}

		if closeChar != 0 {
			pos++ // skip open char
			titleStart := pos
			escaped := false
			validTitle := false
			for pos < len(text) {
				ch := text[pos]
				if escaped {
					escaped = false
					pos++
					continue
				}
				if ch == '\\' {
					escaped = true
					pos++
					continue
				}
				if ch == closeChar {
					title = DecodeEntities(applyBackslashEscapes(text[titleStart:pos]))
					hasTitle = true
					pos++
					validTitle = true
					break
				}
				if ch == '\n' && closeChar == ')' {
					break // parens don't allow newlines
				}
				pos++
			}
			if !validTitle {
				// Failed to parse title — restore
				pos = beforeTitle
			}
		} else {
			pos = beforeTitle
		}
	} else {
		pos = beforeTitle
	}

	// Must be followed by whitespace only on rest of line
	eolPos := pos
	for eolPos < len(text) && (text[eolPos] == ' ' || text[eolPos] == '\t') {
		eolPos++
	}
	if eolPos < len(text) && text[eolPos] != '\n' {
		// Extra content — if we had title, try without
		if hasTitle {
			pos = beforeTitle
			hasTitle = false
			title = ""
			for pos < len(text) && (text[pos] == ' ' || text[pos] == '\t') {
				pos++
			}
			if pos < len(text) && text[pos] != '\n' && text[pos] != 0 {
				return nil
			}
			eolPos = pos
		} else {
			return nil
		}
	}
	pos = eolPos
	if pos < len(text) && text[pos] == '\n' {
		pos++
	}

	return &parsedLinkDef{
		label:         label,
		destination:   destination,
		title:         title,
		hasTitle:      hasTitle,
		charsConsumed: pos,
	}
}

// ─── Container Helpers ────────────────────────────────────────────────────────

func lastChild(container mutableBlock) mutableBlock {
	switch c := container.(type) {
	case *mutableDocument:
		if len(c.children) == 0 {
			return nil
		}
		return c.children[len(c.children)-1]
	case *mutableBlockquote:
		if len(c.children) == 0 {
			return nil
		}
		return c.children[len(c.children)-1]
	case *mutableListItem:
		if len(c.children) == 0 {
			return nil
		}
		return c.children[len(c.children)-1]
	}
	return nil
}

func addChild(container mutableBlock, block mutableBlock) {
	switch c := container.(type) {
	case *mutableDocument:
		c.children = append(c.children, block)
	case *mutableBlockquote:
		c.children = append(c.children, block)
	case *mutableListItem:
		c.children = append(c.children, block)
	}
}

func removeLastChild(container mutableBlock) {
	switch c := container.(type) {
	case *mutableDocument:
		if len(c.children) > 0 {
			c.children = c.children[:len(c.children)-1]
		}
	case *mutableBlockquote:
		if len(c.children) > 0 {
			c.children = c.children[:len(c.children)-1]
		}
	case *mutableListItem:
		if len(c.children) > 0 {
			c.children = c.children[:len(c.children)-1]
		}
	}
}

// ─── Block Finalization ───────────────────────────────────────────────────────

func finalizeBlock(block mutableBlock, linkRefs map[string]*linkReference) {
	switch b := block.(type) {
	case *mutableParagraph:
		// Extract link reference definitions from the paragraph
		text := strings.Join(b.lines, "\n")
		for {
			def := parseLinkDefinition(text)
			if def == nil {
				break
			}
			if _, exists := linkRefs[def.label]; !exists {
				linkRefs[def.label] = &linkReference{
					destination: def.destination,
					title:       def.title,
					hasTitle:    def.hasTitle,
				}
			}
			text = text[def.charsConsumed:]
		}
		// Update paragraph lines with remaining text
		if strings.TrimSpace(text) == "" {
			b.lines = nil
		} else {
			b.lines = strings.Split(text, "\n")
			if len(b.lines) > 0 {
				b.lines[len(b.lines)-1] = strings.TrimRight(b.lines[len(b.lines)-1], " \t")
			}
		}

	case *mutableIndentedCode:
		// Trim trailing blank lines
		for len(b.lines) > 0 && b.lines[len(b.lines)-1] == "" {
			b.lines = b.lines[:len(b.lines)-1]
		}
	}
}

func closeParagraph(leaf mutableBlock, container mutableBlock, linkRefs map[string]*linkReference) {
	if leaf == nil {
		return
	}
	switch l := leaf.(type) {
	case *mutableParagraph:
		finalizeBlock(l, linkRefs)
	case *mutableIndentedCode:
		// Trim trailing blank/whitespace lines
		for len(l.lines) > 0 && isBlank(l.lines[len(l.lines)-1]) {
			l.lines = l.lines[:len(l.lines)-1]
		}
	}
	_ = container
}

// ─── Link Reference Map ───────────────────────────────────────────────────────

// linkReference holds a resolved link reference definition.
type linkReference struct {
	destination string
	title       string
	hasTitle    bool
}

// ─── Parse State ─────────────────────────────────────────────────────────────

// parseState tracks current multi-line block parsing state.
type parseState int

const (
	stateNormal    parseState = iota
	stateFenced               // inside fenced code block
	stateHtmlBlock            // inside HTML block
)

// ─── Main Block Parser ────────────────────────────────────────────────────────

// blockParseResult holds the result of Phase 1.
type blockParseResult struct {
	document *mutableDocument
	linkRefs map[string]*linkReference
}

// parseBlocks is Phase 1 of GFM parsing: split input into block-level
// tokens and build the structural skeleton.
//
// Two-phase overview:
//
//	Phase 1 (this file): Block structure
//	  Input text → lines → block tree with raw inline content strings
//
//	Phase 2 (inline_parser.go): Inline content
//	  Each block's raw content → inline nodes (emphasis, links, etc.)
//
// The phases cannot be merged because block structure determines where inline
// content lives. A * that starts a list item is structural; a * inside a
// paragraph may be emphasis.
func parseBlocks(input string) blockParseResult {
	// Normalize line endings to LF, then split into lines
	normalized := strings.ReplaceAll(input, "\r\n", "\n")
	normalized = strings.ReplaceAll(normalized, "\r", "\n")
	rawLines := strings.Split(normalized, "\n")

	// The trailing newline at end of input produces a spurious empty string.
	if len(rawLines) > 0 && rawLines[len(rawLines)-1] == "" {
		rawLines = rawLines[:len(rawLines)-1]
	}

	linkRefs := make(map[string]*linkReference)
	root := &mutableDocument{}

	// Container block stack. The innermost open container is at the end.
	openContainers := []mutableBlock{root}

	// Current open leaf block (paragraph, code block, etc.)
	var currentLeaf mutableBlock

	// Multi-line block state
	state := stateNormal

	// List tightness tracking
	lastLineWasBlank := false
	lastBlankInnerContainer := mutableBlock(root)

	for _, rawLine := range rawLines {
		origBlank := isBlank(rawLine)

		// ── Container continuation ─────────────────────────────────────────────
		lineContent := rawLine
		lineBaseCol := 0
		newContainers := []mutableBlock{root}
		lazyParagraphContinuation := false

		containerIdx := 1
	continuationLoop:
		for containerIdx < len(openContainers) {
			container := openContainers[containerIdx]

			switch c := container.(type) {
			case *mutableBlockquote:
				// Strip blockquote marker `> ` (up to 3 leading spaces)
				bqI := 0
				bqCol := lineBaseCol
				for bqI < 3 && bqI < len(lineContent) && lineContent[bqI] == ' ' {
					bqI++
					bqCol++
				}
				if bqI < len(lineContent) && lineContent[bqI] == '>' {
					bqI++
					bqCol++
					if bqI < len(lineContent) {
						if lineContent[bqI] == ' ' {
							bqI++
							bqCol++
						} else if lineContent[bqI] == '\t' {
							w := 4 - (bqCol % 4)
							bqI++
							if w > 1 {
								lineContent = strings.Repeat(" ", w-1) + lineContent[bqI:]
								lineBaseCol = bqCol + 1
								newContainers = append(newContainers, container)
								containerIdx++
								continue continuationLoop
							}
							bqCol += w
						}
					}
					lineContent = lineContent[bqI:]
					lineBaseCol = bqCol
					newContainers = append(newContainers, container)
					containerIdx++
				} else if p, ok := currentLeaf.(*mutableParagraph); ok && !origBlank &&
					!isThematicBreak(lineContent) &&
					!(indentOf(lineContent, lineBaseCol) < 4 && fenceStartRe.MatchString(strings.TrimLeft(lineContent, " \t"))) &&
					parseAtxHeading(lineContent) == nil {
					// Lazy paragraph continuation
					lm := parseListMarker(lineContent)
					blankStart := lm != nil && isBlank(lineContent[lm.markerLen:])
					if lm == nil || blankStart {
						newContainers = append(newContainers, container)
						containerIdx++
						lazyParagraphContinuation = true
						_ = p
						goto doneContinuation
					}
					break continuationLoop
				} else {
					break continuationLoop
				}

			case *mutableList:
				// Lists themselves pass through
				newContainers = append(newContainers, container)
				containerIdx++
				_ = c

			case *mutableListItem:
				effectiveBlank := origBlank || isBlank(lineContent)
				indent := indentOf(lineContent, lineBaseCol)
				if !effectiveBlank && indent >= c.contentIndent {
					lineContent, lineBaseCol = stripIndent(lineContent, c.contentIndent, lineBaseCol)
					newContainers = append(newContainers, container)
					containerIdx++
				} else if effectiveBlank {
					hasContent := len(c.children) > 0 || (currentLeaf != nil && openContainers[containerIdx] == currentLeaf)
					if hasContent {
						newContainers = append(newContainers, container)
						containerIdx++
					} else {
						break continuationLoop
					}
				} else if _, ok := currentLeaf.(*mutableParagraph); ok && !origBlank &&
					!isThematicBreak(lineContent) &&
					parseListMarker(lineContent) == nil &&
					!(indentOf(lineContent, lineBaseCol) < 4 && fenceStartRe.MatchString(strings.TrimLeft(lineContent, " \t"))) &&
					parseAtxHeading(lineContent) == nil {
					// Lazy continuation
					newContainers = append(newContainers, container)
					containerIdx++
					lazyParagraphContinuation = true
					goto doneContinuation
				} else {
					break continuationLoop
				}

			default:
				break continuationLoop
			}
		}
	doneContinuation:

		prevInnerContainer := openContainers[len(openContainers)-1]
		openContainers = newContainers

		blank := origBlank
		if !blank && isBlank(lineContent) {
			blank = true
		}

		currentInnerAfterContinuation := openContainers[len(openContainers)-1]

		// ── Multi-line block continuation ─────────────────────────────────────

		if state == stateFenced {
			if fc, ok := currentLeaf.(*mutableFencedCode); ok {
				if currentInnerAfterContinuation != prevInnerContainer {
					// Container dropped — force-close
					fc.closed = true
					state = stateNormal
					currentLeaf = nil
					// Fall through to normal block processing
				} else {
					stripped := strings.TrimLeft(lineContent, " \t")
					fenceChar := fc.fence[0]
					closingFenceRe := buildClosingFenceRe(fenceChar, fc.fenceLen)
					if indentOf(lineContent, lineBaseCol) < 4 && closingFenceRe.MatchString(stripped) && (stripped[0] == fenceChar) {
						fc.closed = true
						state = stateNormal
						currentLeaf = nil
					} else {
						fenceLine, _ := stripIndent(lineContent, fc.baseIndent, lineBaseCol)
						fc.lines = append(fc.lines, fenceLine)
					}
					lastLineWasBlank = origBlank
					continue
				}
			}
		}

		if state == stateHtmlBlock {
			if hb, ok := currentLeaf.(*mutableHtmlBlock); ok {
				if currentInnerAfterContinuation != prevInnerContainer {
					hb.closed = true
					state = stateNormal
					currentLeaf = nil
					// Fall through
				} else {
					hb.lines = append(hb.lines, lineContent)
					if htmlBlockEnds(lineContent, hb.htmlType) {
						hb.closed = true
						state = stateNormal
						currentLeaf = nil
					}
					lastLineWasBlank = origBlank
					continue
				}
			}
		}

		// Finalize current leaf if we left its container
		if currentInnerAfterContinuation != prevInnerContainer && currentLeaf != nil && !lazyParagraphContinuation {
			finalizeBlock(currentLeaf, linkRefs)
			currentLeaf = nil
		}

		// ── Lazy paragraph continuation ────────────────────────────────────
		if lazyParagraphContinuation {
			if p, ok := currentLeaf.(*mutableParagraph); ok {
				p.lines = append(p.lines, lineContent)
				lastLineWasBlank = false
				continue
			}
		}

		// Close list if next line won't continue it
		for !blank && len(openContainers) > 1 {
			lastC := openContainers[len(openContainers)-1]
			if l, ok := lastC.(*mutableList); ok {
				marker := parseListMarker(lineContent)
				if marker != nil && l.ordered == marker.ordered && l.marker == marker.marker &&
					!isThematicBreak(lineContent) {
					break
				}
				openContainers = openContainers[:len(openContainers)-1]
			} else {
				break
			}
		}

		innerContainer := openContainers[len(openContainers)-1]

		// ── Blank line handling ─────────────────────────────────────────────
		if blank {
			if p, ok := currentLeaf.(*mutableParagraph); ok {
				finalizeBlock(p, linkRefs)
				currentLeaf = nil
			} else if ic, ok := currentLeaf.(*mutableIndentedCode); ok {
				blankLine, _ := stripIndent(rawLine, 4, 0)
				ic.lines = append(ic.lines, blankLine)
			}

			if li, ok := innerContainer.(*mutableListItem); ok {
				li.hadBlankLine = true
			}
			if l, ok := innerContainer.(*mutableList); ok {
				l.hadBlankLine = true
			}

			lastLineWasBlank = true
			lastBlankInnerContainer = innerContainer
			continue
		}

		// ── New block detection ─────────────────────────────────────────────
		//
		// We use a labeled loop so that blockquote detection can re-dispatch
		// into the same detection logic after stripping the > marker.
	blockDetect:
		for {
			// After a blank line in a list, make it loose
			if lastLineWasBlank {
				if innerList, ok := innerContainer.(*mutableList); ok {
					lbi := lastBlankInnerContainer
					if lbi != nil {
						_, lbiIsList := lbi.(*mutableList)
						_, lbiIsListItem := lbi.(*mutableListItem)
						if lbiIsList || lbiIsListItem {
							innerList.tight = false
						}
					}
				}
			}

			if lastLineWasBlank {
				if li, ok := innerContainer.(*mutableListItem); ok {
					li.hadBlankLine = true
				}
			}

			indent := indentOf(lineContent, lineBaseCol)

			// 1. Fenced code block
			stripped := strings.TrimLeft(lineContent, " \t")
			fenceM := fenceStartRe.FindString(stripped)
			if fenceM != "" && indent < 4 {
				fenceChar := fenceM[0]
				fenceLen := len(fenceM)
				infoString := extractInfoString(stripped)
				// Backtick fences cannot have backticks in info string
				if fenceChar == '`' && strings.Contains(stripped[fenceLen:], "`") {
					// fall through
				} else {
					closeParagraph(currentLeaf, innerContainer, linkRefs)
					currentLeaf = nil
					fc := &mutableFencedCode{
						fence:      string(fenceChar),
						fenceLen:   fenceLen,
						baseIndent: indent,
						infoString: infoString,
						closed:     false,
					}
					addChild(innerContainer, fc)
					currentLeaf = fc
					state = stateFenced
					lastLineWasBlank = false
					break blockDetect
				}
			}

			// 2. ATX heading
			if indent < 4 {
				if h := parseAtxHeading(lineContent); h != nil {
					closeParagraph(currentLeaf, innerContainer, linkRefs)
					currentLeaf = nil
					addChild(innerContainer, &mutableHeading{level: h.level, content: h.content})
					lastLineWasBlank = false
					break blockDetect
				}
			}

			// 3. Thematic break / setext heading
			if indent < 4 && isThematicBreak(lineContent) {
				if _, ok := currentLeaf.(*mutableParagraph); ok {
					level := isSetextUnderline(lineContent)
					if level != 0 {
						para := currentLeaf.(*mutableParagraph)
						finalizeBlock(para, linkRefs)
						if len(para.lines) > 0 {
							h := &mutableHeading{
								level:   level,
								content: strings.TrimSpace(strings.Join(para.lines, "\n")),
							}
							removeLastChild(innerContainer)
							addChild(innerContainer, h)
							currentLeaf = nil
							lastLineWasBlank = false
							break blockDetect
						}
						// All link defs — para is empty; fall through to thematic break
						removeLastChild(innerContainer)
						currentLeaf = nil
					}
				}
				closeParagraph(currentLeaf, innerContainer, linkRefs)
				currentLeaf = nil
				addChild(innerContainer, &mutableThematicBreak{})
				lastLineWasBlank = false
				break blockDetect
			}

			// 4. Setext heading underline (when no thematic break matched)
			if indent < 4 {
				if _, ok := currentLeaf.(*mutableParagraph); ok {
					level := isSetextUnderline(lineContent)
					if level != 0 {
						para := currentLeaf.(*mutableParagraph)
						finalizeBlock(para, linkRefs)
						if len(para.lines) > 0 {
							h := &mutableHeading{
								level:   level,
								content: strings.TrimSpace(strings.Join(para.lines, "\n")),
							}
							removeLastChild(innerContainer)
							addChild(innerContainer, h)
							currentLeaf = nil
							lastLineWasBlank = false
							break blockDetect
						}
						removeLastChild(innerContainer)
						currentLeaf = nil
					}
				}
			}

			// 5. HTML block
			if indent < 4 {
				htmlType := detectHtmlBlockType(lineContent)
				if htmlType != 0 {
					// Type 7 cannot interrupt a paragraph
					if htmlType == 7 {
						if _, ok := currentLeaf.(*mutableParagraph); ok {
							htmlType = 0 // don't interrupt
						}
					}
					if htmlType != 0 {
						closeParagraph(currentLeaf, innerContainer, linkRefs)
						currentLeaf = nil
						hb := &mutableHtmlBlock{
							htmlType: htmlType,
							lines:    []string{lineContent},
							closed:   htmlBlockEnds(lineContent, htmlType),
						}
						addChild(innerContainer, hb)
						if !hb.closed {
							currentLeaf = hb
							state = stateHtmlBlock
						}
						lastLineWasBlank = false
						break blockDetect
					}
				}
			}

			// 6. Blockquote
			if indent < 4 && strings.HasPrefix(strings.TrimLeft(lineContent, " \t"), ">") {
				closeParagraph(currentLeaf, innerContainer, linkRefs)
				currentLeaf = nil

				var bq *mutableBlockquote
				// Continue existing blockquote only if no blank line intervened
				if lc := lastChild(innerContainer); lc != nil {
					if existBq, ok := lc.(*mutableBlockquote); ok && !lastLineWasBlank {
						bq = existBq
					}
				}
				if bq == nil {
					bq = &mutableBlockquote{}
					addChild(innerContainer, bq)
				}

				openContainers = append(openContainers, bq)

				// Strip the > marker with tab-aware arithmetic
				bqI := 0
				bqCol := lineBaseCol
				for bqI < len(lineContent) && lineContent[bqI] == ' ' && bqI < 3 {
					bqI++
					bqCol++
				}
				if bqI < len(lineContent) && lineContent[bqI] == '>' {
					bqI++
					bqCol++
					if bqI < len(lineContent) {
						if lineContent[bqI] == ' ' {
							bqI++
							bqCol++
						} else if lineContent[bqI] == '\t' {
							w := 4 - (bqCol % 4)
							bqI++
							if w > 1 {
								lineContent = strings.Repeat(" ", w-1) + lineContent[bqI:]
								lineBaseCol = bqCol + 1
								innerContainer = bq
								if isBlank(lineContent) {
									break blockDetect
								}
								continue blockDetect
							}
							bqCol += w
						}
					}
				}
				lineContent = lineContent[bqI:]
				lineBaseCol = bqCol
				innerContainer = bq

				if isBlank(lineContent) {
					lastLineWasBlank = false
					break blockDetect
				}
				continue blockDetect
			}

			// 7. List item
			if indent < 4 {
				marker := parseListMarker(lineContent)
				if marker != nil {
					var list *mutableList

					// Check if continuing an existing list
					if l, ok := innerContainer.(*mutableList); ok {
						if l.ordered == marker.ordered && l.marker == marker.marker {
							list = l
						}
					}
					if list == nil {
						if lc := lastChild(innerContainer); lc != nil {
							if l, ok := lc.(*mutableList); ok {
								if l.ordered == marker.ordered && l.marker == marker.marker {
									list = l
								}
							}
						}
					}

					newLineBaseCol := virtualColAfter(lineContent, marker.markerLen, lineBaseCol)
					itemContent := lineContent[marker.markerLen:]

					// Handle tab separator
					if marker.spaceAfter == 1 {
						sepChar := lineContent[marker.markerLen-1]
						if sepChar == '\t' {
							sepCol := virtualColAfter(lineContent, marker.markerLen-1, lineBaseCol)
							w := 4 - (sepCol % 4)
							if w > 1 {
								itemContent = strings.Repeat(" ", w-1) + itemContent
								newLineBaseCol = sepCol + 1
							}
						}
					}

					blankStart := isBlank(itemContent)

					// Can this interrupt a paragraph?
					paraInCurrentContainer := false
					if p, ok := currentLeaf.(*mutableParagraph); ok {
						if lc := lastChild(innerContainer); lc == p {
							paraInCurrentContainer = true
						}
					}
					canInterruptPara := (!marker.ordered || marker.start == 1 || list != nil) &&
						(!blankStart || !paraInCurrentContainer)

					_, isCurrentPara := currentLeaf.(*mutableParagraph)
					if !isCurrentPara || canInterruptPara {
						if list == nil {
							closeParagraph(currentLeaf, innerContainer, linkRefs)
							currentLeaf = nil
							list = &mutableList{
								ordered: marker.ordered,
								marker:  marker.marker,
								start:   marker.start,
								tight:   true,
							}
							addChild(innerContainer, list)
						} else {
							closeParagraph(currentLeaf, innerContainer, linkRefs)
							currentLeaf = nil
							if list.hadBlankLine || (lastLineWasBlank &&
								(func() bool {
									_, ok1 := lastBlankInnerContainer.(*mutableList)
									_, ok2 := lastBlankInnerContainer.(*mutableListItem)
									return ok1 || ok2
								})()) {
								list.tight = false
							}
							list.hadBlankLine = false
						}

						normalIndent := marker.markerLen
						reducedIndent := marker.markerLen - marker.spaceAfter + 1
						contentIndent := normalIndent
						if blankStart || marker.spaceAfter >= 5 {
							contentIndent = reducedIndent
						}

						item := &mutableListItem{
							marker:        marker.marker,
							markerIndent:  marker.indent,
							contentIndent: contentIndent,
						}
						list.items = append(list.items, item)

						if innerContainer != list {
							openContainers = append(openContainers, list)
						}
						openContainers = append(openContainers, item)

						if !blankStart {
							innerContainer = item
							if marker.spaceAfter >= 5 {
								lineBaseCol = virtualColAfter(lineContent, marker.markerLen-marker.spaceAfter+1, lineBaseCol)
								lineContent = strings.Repeat(" ", marker.spaceAfter-1) + itemContent
							} else {
								lineBaseCol = newLineBaseCol
								lineContent = itemContent
							}
							continue blockDetect
						}
						currentLeaf = nil
						lastLineWasBlank = false
						break blockDetect
					}
				}
			}

			// 8. Indented code block (4+ spaces, NOT inside a paragraph)
			if indent >= 4 {
				if _, ok := currentLeaf.(*mutableParagraph); !ok {
					stripped2, _ := stripIndent(lineContent, 4, lineBaseCol)
					if ic, ok2 := currentLeaf.(*mutableIndentedCode); ok2 {
						ic.lines = append(ic.lines, stripped2)
					} else {
						closeParagraph(currentLeaf, innerContainer, linkRefs)
						ic := &mutableIndentedCode{lines: []string{stripped2}}
						addChild(innerContainer, ic)
						currentLeaf = ic
					}
					lastLineWasBlank = false
					break blockDetect
				}
			}

			// 9. Paragraph continuation or new paragraph
			if p, ok := currentLeaf.(*mutableParagraph); ok {
				p.lines = append(p.lines, lineContent)
			} else {
				closeParagraph(currentLeaf, innerContainer, linkRefs)
				p := &mutableParagraph{lines: []string{lineContent}}
				addChild(innerContainer, p)
				currentLeaf = p
			}
			lastLineWasBlank = false
			break blockDetect
		} // end blockDetect
	}

	// Finalize any remaining open leaf block
	if currentLeaf != nil {
		finalizeBlock(currentLeaf, linkRefs)
	}

	return blockParseResult{document: root, linkRefs: linkRefs}
}

var fenceStartRe = regexp.MustCompile("^(`{3,}|~{3,})")

// buildClosingFenceRe builds a regex that matches a closing fence of at least
// fenceLen characters of fenceChar, with optional trailing whitespace.
var closingFenceCache = make(map[[2]interface{}]*regexp.Regexp)

func buildClosingFenceRe(fenceChar byte, fenceLen int) *regexp.Regexp {
	key := [2]interface{}{fenceChar, fenceLen}
	if re, ok := closingFenceCache[key]; ok {
		return re
	}
	var charStr string
	if fenceChar == '`' {
		charStr = "`"
	} else {
		charStr = "~"
	}
	re := regexp.MustCompile(`^` + regexp.QuoteMeta(charStr) + `{` + strconv.Itoa(fenceLen) + `,}\s*$`)
	closingFenceCache[key] = re
	return re
}

// ─── AST Conversion ──────────────────────────────────────────────────────────

// convertResult holds the final AST along with raw inline content strings
// for Phase 2 processing.
type convertResult struct {
	document         *documentast.DocumentNode
	rawInlineContent map[int]string // id -> raw string
	nextID           int
}

// convertToAST converts the mutable intermediate document into the final AST.
// Inline content is NOT yet parsed — raw strings are stored in rawInlineContent.
func convertToAST(mutableDoc *mutableDocument, linkRefs map[string]*linkReference) convertResult {
	result := convertResult{
		rawInlineContent: make(map[int]string),
	}

	var convertBlock func(block mutableBlock) documentast.BlockNode
	convertBlock = func(block mutableBlock) documentast.BlockNode {
		switch b := block.(type) {
		case *mutableDocument:
			doc := &documentast.DocumentNode{}
			for _, child := range b.children {
				if n := convertBlock(child); n != nil {
					doc.Children = append(doc.Children, n)
				}
			}
			return doc

		case *mutableHeading:
			id := result.nextID
			result.nextID++
			result.rawInlineContent[id] = b.content
			h := &headingNodeWithID{
				HeadingNode: documentast.HeadingNode{Level: b.level},
				rawID:       id,
			}
			return h

		case *mutableParagraph:
			if len(b.lines) == 0 {
				return nil
			}
			// Strip leading whitespace from each line
			processedLines := make([]string, len(b.lines))
			for i, l := range b.lines {
				processedLines[i] = strings.TrimLeft(l, " \t")
			}
			content := strings.Join(processedLines, "\n")
			id := result.nextID
			result.nextID++
			result.rawInlineContent[id] = content
			return &paragraphNodeWithID{
				ParagraphNode: documentast.ParagraphNode{},
				rawID:         id,
			}

		case *mutableFencedCode:
			value := strings.Join(b.lines, "\n")
			if len(b.lines) > 0 {
				value += "\n"
			}
			return &documentast.CodeBlockNode{
				Language: b.infoString,
				Value:    value,
			}

		case *mutableIndentedCode:
			value := strings.Join(b.lines, "\n") + "\n"
			return &documentast.CodeBlockNode{
				Language: "",
				Value:    value,
			}

		case *mutableBlockquote:
			bq := &documentast.BlockquoteNode{}
			for _, child := range b.children {
				if n := convertBlock(child); n != nil {
					bq.Children = append(bq.Children, n)
				}
			}
			return bq

		case *mutableList:
			// A list is loose if blank lines appeared between items OR
			// blank lines appeared between blocks within an item with >1 block.
			isTight := b.tight && !b.hadBlankLine
			if isTight {
				for _, item := range b.items {
					if item.hadBlankLine && len(item.children) > 1 {
						isTight = false
						break
					}
				}
			}

			list := &documentast.ListNode{
				Ordered: b.ordered,
				Tight:   isTight,
			}
			if b.ordered {
				list.Start = b.start
			}
			for _, item := range b.items {
				if n := convertBlock(item); n != nil {
					list.Children = append(list.Children, n.(*documentast.ListItemNode))
				}
			}
			return list

		case *mutableListItem:
			item := &documentast.ListItemNode{}
			for _, child := range b.children {
				if n := convertBlock(child); n != nil {
					item.Children = append(item.Children, n)
				}
			}
			return item

		case *mutableThematicBreak:
			return &documentast.ThematicBreakNode{}

		case *mutableHtmlBlock:
			// For type 6/7 blocks, trim trailing blank lines
			lines := make([]string, len(b.lines))
			copy(lines, b.lines)
			for len(lines) > 0 && strings.TrimSpace(lines[len(lines)-1]) == "" {
				lines = lines[:len(lines)-1]
			}
			return &documentast.RawBlockNode{
				Format: "html",
				Value:  strings.Join(lines, "\n") + "\n",
			}

		case *mutableLinkDef:
			// Link definitions are resolved into linkRefs; not emitted in the AST
			return nil

		default:
			return nil
		}
	}

	doc := convertBlock(mutableDoc).(*documentast.DocumentNode)
	result.document = doc
	return result
}

// headingNodeWithID extends HeadingNode with an ID for Phase 2 inline parsing.
// This is an internal type; after Phase 2, it's replaced by a proper HeadingNode.
type headingNodeWithID struct {
	documentast.HeadingNode
	rawID int
}

func (n *headingNodeWithID) NodeType() string { return "heading" }
func (n *headingNodeWithID) blockNode()       {}

// paragraphNodeWithID extends ParagraphNode with an ID for Phase 2 inline parsing.
type paragraphNodeWithID struct {
	documentast.ParagraphNode
	rawID int
}

func (n *paragraphNodeWithID) NodeType() string { return "paragraph" }
func (n *paragraphNodeWithID) blockNode()       {}

var taskItemPrefixRe = regexp.MustCompile(`^\[( |x|X)\](?:[ \t]+|$)`)
var tableDelimiterRe = regexp.MustCompile(`^:?-{3,}:?$`)

func applyGfmBlockExtensions(doc *documentast.DocumentNode, rawInlineContent map[int]string, linkRefs map[string]*linkReference) {
	var transformBlocks func([]documentast.BlockNode) []documentast.BlockNode
	var transformBlock func(documentast.BlockNode) documentast.BlockNode

	transformBlocks = func(blocks []documentast.BlockNode) []documentast.BlockNode {
		out := make([]documentast.BlockNode, 0, len(blocks))
		for _, block := range blocks {
			out = append(out, transformBlock(block))
		}
		return out
	}

	transformBlock = func(block documentast.BlockNode) documentast.BlockNode {
		switch b := block.(type) {
		case *documentast.DocumentNode:
			return &documentast.DocumentNode{Children: transformBlocks(b.Children)}
		case *documentast.BlockquoteNode:
			return &documentast.BlockquoteNode{Children: transformBlocks(b.Children)}
		case *documentast.ListNode:
			children := make([]documentast.ListChildNode, 0, len(b.Children))
			for _, item := range b.Children {
				children = append(children, transformListChild(item, rawInlineContent, transformBlocks))
			}
			return &documentast.ListNode{
				Ordered:  b.Ordered,
				Start:    b.Start,
				Tight:    b.Tight,
				Children: children,
			}
		case *paragraphNodeWithID:
			if table := maybeTransformParagraphToTable(b, rawInlineContent, linkRefs); table != nil {
				return table
			}
			return b
		case *documentast.ListItemNode:
			return transformListChild(b, rawInlineContent, transformBlocks).(documentast.BlockNode)
		case *documentast.TaskItemNode:
			return transformListChild(b, rawInlineContent, transformBlocks).(documentast.BlockNode)
		default:
			return block
		}
	}

	doc.Children = transformBlocks(doc.Children)
}

func transformListChild(
	item documentast.ListChildNode,
	rawInlineContent map[int]string,
	transformBlocks func([]documentast.BlockNode) []documentast.BlockNode,
) documentast.ListChildNode {
	switch it := item.(type) {
	case *documentast.ListItemNode:
		children := transformBlocks(it.Children)
		nextItem := &documentast.ListItemNode{Children: children}
		if len(children) == 0 {
			return nextItem
		}
		first, ok := children[0].(*paragraphNodeWithID)
		if !ok {
			return nextItem
		}
		raw, ok := rawInlineContent[first.rawID]
		if !ok {
			return nextItem
		}
		match := taskItemPrefixRe.FindStringSubmatch(raw)
		if match == nil {
			return nextItem
		}
		rawInlineContent[first.rawID] = raw[len(match[0]):]
		return &documentast.TaskItemNode{Checked: strings.EqualFold(match[1], "x"), Children: children}
	case *documentast.TaskItemNode:
		return &documentast.TaskItemNode{Checked: it.Checked, Children: transformBlocks(it.Children)}
	default:
		return item
	}
}

func maybeTransformParagraphToTable(
	block *paragraphNodeWithID,
	rawInlineContent map[int]string,
	linkRefs map[string]*linkReference,
) *documentast.TableNode {
	raw, ok := rawInlineContent[block.rawID]
	if !ok {
		return nil
	}
	align, header, rows, ok := tryParseTable(raw)
	if !ok {
		return nil
	}

	makeCell := func(content string) *documentast.TableCellNode {
		return &documentast.TableCellNode{Children: ParseInline(content, linkRefs)}
	}
	makeRow := func(cells []string, isHeader bool) *documentast.TableRowNode {
		row := &documentast.TableRowNode{IsHeader: isHeader}
		for _, cell := range cells {
			row.Children = append(row.Children, makeCell(cell))
		}
		return row
	}

	delete(rawInlineContent, block.rawID)
	table := &documentast.TableNode{Align: align}
	table.Children = append(table.Children, makeRow(header, true))
	for _, row := range rows {
		table.Children = append(table.Children, makeRow(row, false))
	}
	return table
}

func tryParseTable(raw string) ([]documentast.TableAlignment, []string, [][]string, bool) {
	lines := strings.Split(raw, "\n")
	if len(lines) < 2 {
		return nil, nil, nil, false
	}
	headerCells, ok := splitTableRow(lines[0])
	if !ok {
		return nil, nil, nil, false
	}
	delimiterCells, ok := splitTableRow(lines[1])
	if !ok || len(headerCells) == 0 || len(headerCells) != len(delimiterCells) {
		return nil, nil, nil, false
	}
	align := make([]documentast.TableAlignment, 0, len(delimiterCells))
	for _, cell := range delimiterCells {
		trimmed := strings.TrimSpace(cell)
		if !tableDelimiterRe.MatchString(trimmed) {
			return nil, nil, nil, false
		}
		left := strings.HasPrefix(trimmed, ":")
		right := strings.HasSuffix(trimmed, ":")
		switch {
		case left && right:
			align = append(align, documentast.TableAlignCenter)
		case left:
			align = append(align, documentast.TableAlignLeft)
		case right:
			align = append(align, documentast.TableAlignRight)
		default:
			align = append(align, documentast.TableAlignNone)
		}
	}
	rows := make([][]string, 0, len(lines)-2)
	for _, line := range lines[2:] {
		if strings.TrimSpace(line) == "" {
			return nil, nil, nil, false
		}
		cells, ok := splitTableRow(line)
		if !ok {
			return nil, nil, nil, false
		}
		rows = append(rows, normalizeTableRow(cells, len(headerCells)))
	}
	return align, normalizeTableRow(headerCells, len(delimiterCells)), rows, true
}

func splitTableRow(line string) ([]string, bool) {
	if !strings.Contains(line, "|") {
		return nil, false
	}
	trimmed := strings.TrimSpace(line)
	hadOuterPipe := strings.HasPrefix(trimmed, "|") || strings.HasSuffix(trimmed, "|")
	start := 0
	end := len(line)
	for start < end && (line[start] == ' ' || line[start] == '\t') {
		start++
	}
	for end > start && (line[end-1] == ' ' || line[end-1] == '\t') {
		end--
	}
	if start < end && line[start] == '|' {
		start++
	}
	if end > start && line[end-1] == '|' {
		end--
	}

	cells := []string{}
	var current strings.Builder
	escaped := false
	pipeCount := 0
	for i := start; i < end; i++ {
		ch := line[i]
		if escaped {
			current.WriteByte(ch)
			escaped = false
			continue
		}
		if ch == '\\' {
			current.WriteByte(ch)
			escaped = true
			continue
		}
		if ch == '|' {
			pipeCount++
			cells = append(cells, strings.TrimSpace(current.String()))
			current.Reset()
			continue
		}
		current.WriteByte(ch)
	}
	cells = append(cells, strings.TrimSpace(current.String()))
	return cells, pipeCount > 0 || hadOuterPipe
}

func normalizeTableRow(cells []string, width int) []string {
	out := append([]string(nil), cells...)
	if len(out) > width {
		out = out[:width]
	}
	for len(out) < width {
		out = append(out, "")
	}
	return out
}
