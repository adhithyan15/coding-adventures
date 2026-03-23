// Package xmllexer tokenizes XML text using a grammar-driven lexer with
// pattern groups and an on-token callback for context-sensitive lexing.
//
// XML is context-sensitive at the lexical level. The same character has
// different meaning depending on position:
//
//   - `=` is an attribute delimiter inside `<tag attr="val">`
//   - `=` is plain text content outside tags: `1 + 1 = 2`
//
// A flat pattern list cannot distinguish these contexts. Pattern groups
// solve this by defining separate sets of patterns for each context, and
// a callback function switches between them at runtime.
//
// # Pattern Groups
//
// The xml.tokens grammar defines 5 pattern groups:
//
//   - **default** (implicit): Text content, entity refs, tag openers
//   - **tag**: Tag names, attributes, equals, quoted values, closers
//   - **comment**: Comment text and `-->` delimiter
//   - **cdata**: Raw text and `]]>` delimiter
//   - **pi**: Processing instruction target, text, and `?>` delimiter
//
// # The Callback
//
// The XmlOnToken callback fires after each token match and drives group
// switching. It follows a simple state machine:
//
//	default ──OPEN_TAG_START──> tag ──TAG_CLOSE──> default
//	        ──CLOSE_TAG_START─> tag ──SELF_CLOSE─> default
//	        ──COMMENT_START───> comment ──COMMENT_END──> default
//	        ──CDATA_START─────> cdata ──CDATA_END──> default
//	        ──PI_START────────> pi ──PI_END──> default
//
// For comment, CDATA, and PI groups, the callback also disables skip
// patterns (so whitespace is preserved as content) and re-enables them
// when leaving the group.
//
// # Usage
//
//	tokens, err := xmllexer.TokenizeXml(`<div class="main">Hello &amp; world</div>`)
//	if err != nil {
//	    panic(err)
//	}
//	for _, tok := range tokens {
//	    fmt.Printf("%s(%q)\n", tok.TypeName, tok.Value)
//	}
package xmllexer

import (
	"os"
	"path/filepath"
	"runtime"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// ---------------------------------------------------------------------------
// Grammar File Location
// ---------------------------------------------------------------------------

// getGrammarPath computes the absolute path to the xml.tokens grammar file.
//
// We use runtime.Caller(0) to find the directory of this Go source file at
// runtime, then navigate up three levels (xml-lexer -> go -> packages ->
// code) to reach the grammars directory. This approach works regardless of
// the working directory, which is important because tests and the build tool
// may run from different locations.
//
// Directory structure:
//
//	code/
//	  grammars/
//	    xml.tokens          <-- this is what we want
//	  packages/
//	    go/
//	      xml-lexer/
//	        xml_lexer.go    <-- we are here (3 levels below code/)
func getGrammarPath() string {
	// runtime.Caller(0) returns the file path of the current source file.
	// The underscore variables are: program counter, line number, and ok bool.
	_, filename, _, _ := runtime.Caller(0)

	// filepath.Dir gives us the directory containing xml_lexer.go
	parent := filepath.Dir(filename)

	// Navigate up 3 levels: xml-lexer -> go -> packages -> code,
	// then down into grammars/
	root := filepath.Join(parent, "..", "..", "..", "grammars")

	return filepath.Join(root, "xml.tokens")
}

// ---------------------------------------------------------------------------
// XML On-Token Callback
// ---------------------------------------------------------------------------
//
// XmlOnToken is the callback that drives group transitions for XML
// tokenization. It is a pure function of the token type -- no external
// state is needed. The LexerContext provides all the control we need
// (push/pop groups, toggle skip).
//
// The pattern is simple:
//   - Opening delimiters push a group
//   - Closing delimiters pop the group
//   - Comment/CDATA/PI groups disable skip (whitespace is content)
//
// Here is a truth table showing the callback's behavior:
//
//	Token Type         | Action
//	-------------------+-------------------------------------------
//	OPEN_TAG_START     | push("tag")
//	CLOSE_TAG_START    | push("tag")
//	TAG_CLOSE          | pop()
//	SELF_CLOSE         | pop()
//	COMMENT_START      | push("comment"), disable skip
//	COMMENT_END        | pop(), enable skip
//	CDATA_START        | push("cdata"), disable skip
//	CDATA_END          | pop(), enable skip
//	PI_START           | push("pi"), disable skip
//	PI_END             | pop(), enable skip
//	(anything else)    | no action
//
// ---------------------------------------------------------------------------

// XmlOnToken is the on-token callback for XML tokenization.
//
// It examines the TypeName of each matched token and pushes or pops
// pattern groups accordingly. For comment, CDATA, and processing
// instruction groups, it also toggles skip pattern processing so that
// whitespace inside those constructs is preserved as content tokens
// rather than being silently consumed.
func XmlOnToken(token lexer.Token, ctx *lexer.LexerContext) {
	switch token.TypeName {

	// --- Tag boundaries ---
	//
	// When we see `<` (OPEN_TAG_START) or `</` (CLOSE_TAG_START), we
	// push the "tag" group. This activates patterns for tag names,
	// attribute names, equals signs, quoted values, and tag closers.
	case "OPEN_TAG_START", "CLOSE_TAG_START":
		ctx.PushGroup("tag")

	// When we see `>` (TAG_CLOSE) or `/>` (SELF_CLOSE), we pop the
	// "tag" group to return to the default group (text content).
	case "TAG_CLOSE", "SELF_CLOSE":
		ctx.PopGroup()

	// --- Comment boundaries ---
	//
	// `<!--` pushes the "comment" group. We disable skip so that
	// whitespace inside the comment is preserved as COMMENT_TEXT.
	case "COMMENT_START":
		ctx.PushGroup("comment")
		ctx.SetSkipEnabled(false)

	// `-->` pops the "comment" group and re-enables skip.
	case "COMMENT_END":
		ctx.PopGroup()
		ctx.SetSkipEnabled(true)

	// --- CDATA boundaries ---
	//
	// `<![CDATA[` pushes the "cdata" group. Skip is disabled so
	// whitespace appears as CDATA_TEXT content.
	case "CDATA_START":
		ctx.PushGroup("cdata")
		ctx.SetSkipEnabled(false)

	// `]]>` pops the "cdata" group and re-enables skip.
	case "CDATA_END":
		ctx.PopGroup()
		ctx.SetSkipEnabled(true)

	// --- Processing instruction boundaries ---
	//
	// `<?` pushes the "pi" group. Skip is disabled so whitespace
	// in the PI content is preserved as PI_TEXT.
	case "PI_START":
		ctx.PushGroup("pi")
		ctx.SetSkipEnabled(false)

	// `?>` pops the "pi" group and re-enables skip.
	case "PI_END":
		ctx.PopGroup()
		ctx.SetSkipEnabled(true)
	}
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

// CreateXmlLexer loads the XML token grammar and returns a configured
// GrammarLexer ready to tokenize the given XML text.
//
// This function reads the xml.tokens file, parses it into a TokenGrammar,
// creates a GrammarLexer, and registers the XmlOnToken callback for
// pattern group switching.
//
// The returned lexer operates with pattern groups. The XmlOnToken callback
// switches between groups as tag delimiters, comments, CDATA sections, and
// processing instructions are encountered.
//
// Returns an error if the grammar file cannot be read or parsed.
//
// Example:
//
//	lex, err := xmllexer.CreateXmlLexer(`<div>hello</div>`)
//	if err != nil {
//	    panic(err)
//	}
//	tokens := lex.Tokenize()
func CreateXmlLexer(source string) (*lexer.GrammarLexer, error) {
	// Read the grammar file from disk. This file defines all token patterns,
	// skip patterns, pattern groups, and literal tokens for XML.
	bytes, err := os.ReadFile(getGrammarPath())
	if err != nil {
		return nil, err
	}

	// Parse the grammar file into a structured TokenGrammar object.
	// This extracts token definitions (with regex patterns), skip
	// definitions, pattern groups, and alias mappings.
	grammar, err := grammartools.ParseTokenGrammar(string(bytes))
	if err != nil {
		return nil, err
	}

	// -----------------------------------------------------------------------
	// Go-Compatible Pattern Rewrites
	// -----------------------------------------------------------------------
	//
	// The xml.tokens grammar uses Perl-style negative lookaheads (?!...)
	// which Go's regexp package does not support. For example:
	//
	//   COMMENT_TEXT = /([^-]|-(?!->))+/
	//
	// means "match everything except the --> sequence". In Python's regex
	// engine, (?!->) is a zero-width assertion that checks the next chars
	// without consuming them. Go has no equivalent.
	//
	// Our workaround: rewrite each problematic pattern into a simpler
	// Go-compatible regex that matches a single "safe unit" — either a run
	// of characters that can't start the end delimiter, or a single instance
	// of the delimiter-start character. We then reorder definitions so the
	// end-delimiter pattern (e.g., COMMENT_END) is tried BEFORE the text
	// pattern. This ensures that when the end delimiter appears, it matches
	// first. When it doesn't, the text pattern matches one safe chunk.
	//
	// The consequence is that the lexer may produce multiple consecutive
	// text tokens (e.g., two COMMENT_TEXT tokens instead of one). The
	// TokenizeXml function merges these adjacent same-type tokens into a
	// single token, preserving the expected output.
	//
	// Pattern rewrites:
	//
	//   Original (Python)                    Go-compatible
	//   ──────────────────────────────────   ────────────────────
	//   COMMENT_TEXT: ([^-]|-(?!->))+        [^-]+|-
	//   CDATA_TEXT:   ([^\]]|\](?!\]>))+     [^\]]+|\]
	//   PI_TEXT:      ([^?]|\?(?!>))+        [^?]+|\?
	//
	// And reorder: end-delimiter before text in each group.
	//
	rewriteGroup(grammar, "comment", "COMMENT_TEXT", `[^-]+|-`, "COMMENT_END")
	rewriteGroup(grammar, "cdata", "CDATA_TEXT", `[^\]]+|\]`, "CDATA_END")
	rewriteGroup(grammar, "pi", "PI_TEXT", `[^?]+|\?`, "PI_END")

	// Create the grammar-driven lexer. The GrammarLexer constructor compiles
	// all regex patterns and initializes skip pattern matching and pattern
	// group support.
	xmlLexer := lexer.NewGrammarLexer(source, grammar)

	// Register the on-token callback. This callback fires after each token
	// match and switches pattern groups based on the token type. Without
	// this callback, the lexer would stay in the default group forever and
	// never recognize tag-internal patterns like attribute names and values.
	xmlLexer.SetOnToken(XmlOnToken)

	return xmlLexer, nil
}

// rewriteGroup replaces a text pattern in a grammar group with a Go-compatible
// regex and reorders definitions so the end-delimiter is tried first.
//
// Parameters:
//   - grammar: the parsed TokenGrammar to modify in-place
//   - groupName: name of the pattern group (e.g., "comment")
//   - textName: name of the text pattern to rewrite (e.g., "COMMENT_TEXT")
//   - newPattern: Go-compatible regex replacement
//   - endName: name of the end-delimiter pattern to move first
//
// After this function, the group's definitions are reordered to:
//
//	[end-delimiter, text-pattern, ...others]
//
// This ensures the end delimiter matches before the text pattern when both
// could match at the same position.
func rewriteGroup(grammar *grammartools.TokenGrammar, groupName, textName, newPattern, endName string) {
	group, ok := grammar.Groups[groupName]
	if !ok {
		return
	}

	// Rewrite the text pattern and separate definitions by role.
	var endDef, textDef *grammartools.TokenDefinition
	var others []grammartools.TokenDefinition

	for i := range group.Definitions {
		d := &group.Definitions[i]
		switch d.Name {
		case textName:
			d.Pattern = newPattern
			d.IsRegex = true
			textDef = d
		case endName:
			endDef = d
		default:
			others = append(others, *d)
		}
	}

	// Rebuild: end-delimiter first, then other patterns (e.g. PI_TARGET),
	// then text pattern last. This order ensures:
	// 1. End delimiters match before text (so we don't consume the delimiter)
	// 2. Specific patterns (like PI_TARGET) match before the greedy text pattern
	// 3. Text pattern is the fallback
	var reordered []grammartools.TokenDefinition
	if endDef != nil {
		reordered = append(reordered, *endDef)
	}
	reordered = append(reordered, others...)
	if textDef != nil {
		reordered = append(reordered, *textDef)
	}
	group.Definitions = reordered
}

// TokenizeXml is a convenience function that tokenizes XML text in a single
// call. It creates a lexer, runs tokenization, and returns the resulting
// token slice.
//
// This is the simplest way to tokenize XML. For repeated tokenization or
// when you need access to the lexer object itself, use CreateXmlLexer instead.
//
// The returned tokens include:
//
// Default group (content between tags):
//   - TEXT: text content (e.g., "Hello world")
//   - ENTITY_REF: entity reference (e.g., "&amp;")
//   - CHAR_REF: character reference (e.g., "&#65;", "&#x41;")
//   - OPEN_TAG_START: "<"
//   - CLOSE_TAG_START: "</"
//   - COMMENT_START: "<!--"
//   - CDATA_START: "<![CDATA["
//   - PI_START: "<?"
//
// Tag group (inside tags):
//   - TAG_NAME: tag or attribute name (e.g., "div", "class")
//   - ATTR_EQUALS: "="
//   - ATTR_VALUE: quoted attribute value (e.g., `"main"`)
//   - TAG_CLOSE: ">"
//   - SELF_CLOSE: "/>"
//
// Comment group:
//   - COMMENT_TEXT: comment content
//   - COMMENT_END: "-->"
//
// CDATA group:
//   - CDATA_TEXT: raw text content
//   - CDATA_END: "]]>"
//
// Processing instruction group:
//   - PI_TARGET: PI target name (e.g., "xml")
//   - PI_TEXT: PI content
//   - PI_END: "?>"
//
// Always present:
//   - EOF: end of input
//
// Returns an error if the grammar file cannot be loaded.
func TokenizeXml(source string) ([]lexer.Token, error) {
	xmlLexer, err := CreateXmlLexer(source)
	if err != nil {
		return nil, err
	}
	tokens := xmlLexer.Tokenize()

	// -----------------------------------------------------------------------
	// Merge Adjacent Same-Type Tokens
	// -----------------------------------------------------------------------
	//
	// Because we rewrote the lookahead-based text patterns into simpler
	// Go-compatible ones (see the comment in CreateXmlLexer), the lexer may
	// produce multiple consecutive tokens of the same type. For example,
	// the comment "<!-- a-b -->" might produce two COMMENT_TEXT tokens:
	// one for " a" and one for "-b ".
	//
	// We merge adjacent tokens with the same TypeName into a single token,
	// concatenating their values. The merged token keeps the line/column of
	// the first token in the run.
	//
	// Token types that benefit from merging:
	//   - COMMENT_TEXT: split on single dashes
	//   - CDATA_TEXT: split on single brackets
	//   - PI_TEXT: split on single question marks
	//
	// We merge ALL adjacent same-type tokens generically, which is safe
	// because no other XML token type should produce consecutive duplicates.
	return mergeAdjacentTokens(tokens), nil
}

// mergeAdjacentTokens combines consecutive tokens with the same TypeName
// into a single token by concatenating their values.
//
// This is needed because our Go-compatible regex patterns for COMMENT_TEXT,
// CDATA_TEXT, and PI_TEXT match one "safe unit" at a time instead of the
// full run (since Go lacks negative lookaheads). The merge step restores
// the expected single-token output.
//
// Example before merge:
//
//	[COMMENT_TEXT(" a"), COMMENT_TEXT("-"), COMMENT_TEXT("b ")]
//
// Example after merge:
//
//	[COMMENT_TEXT(" a-b ")]
// mergeableTypes lists the token types that should be merged when adjacent.
// Only the text patterns that were rewritten from lookahead-based patterns
// need merging. Other token types should never produce consecutive duplicates.
var mergeableTypes = map[string]bool{
	"COMMENT_TEXT": true,
	"CDATA_TEXT":   true,
	"PI_TEXT":      true,
}

func mergeAdjacentTokens(tokens []lexer.Token) []lexer.Token {
	if len(tokens) == 0 {
		return tokens
	}

	merged := make([]lexer.Token, 0, len(tokens))
	current := tokens[0]

	for i := 1; i < len(tokens); i++ {
		if tokens[i].TypeName == current.TypeName && mergeableTypes[current.TypeName] {
			// Same mergeable type — concatenate the value into the current token.
			current.Value += tokens[i].Value
		} else {
			// Different type or non-mergeable — emit and start a new one.
			merged = append(merged, current)
			current = tokens[i]
		}
	}
	merged = append(merged, current)

	return merged
}
