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
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

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
	// `<?` pushes the "pi" group, which offers only PI_END and PI_TARGET.
	// Skip is disabled so whitespace in the PI content is preserved.
	case "PI_START":
		ctx.PushGroup("pi")
		ctx.SetSkipEnabled(false)

	// PI_TARGET is only ever the first token in a PI. Swap (pop, then
	// push — not a nested push) from "pi" to "pi_body", which offers
	// PI_END, PI_TEXT, and PI_QMARK instead. Without this swap, PI_TARGET's
	// own pattern would still be on offer for the rest of the body and
	// could wrongly re-match a run of letters following a lone "?" as a
	// second PI_TARGET instead of PI_TEXT content — see xml.tokens' pi/
	// pi_body groups. PI_END's single PopGroup() below still returns
	// straight past this swap to the default group.
	case "PI_TARGET":
		ctx.PopGroup()
		ctx.PushGroup("pi_body")

	// `?>` pops the "pi"/"pi_body" group and re-enables skip.
	case "PI_END":
		ctx.PopGroup()
		ctx.SetSkipEnabled(true)
	}
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

// CreateXmlLexer returns a GrammarLexer configured with the XML token grammar,
// ready to tokenize the given XML text.
//
// The grammar is embedded at compile time as native Go in grammar_data.go
// (TokenGrammarData) — nothing is read from disk at run time, and every
// pattern in it is already Go-`regexp`-compatible (no lookaround; see
// xml.tokens' portability note for how the comment/cdata/pi groups avoid it).
// The XmlOnToken callback is registered so the lexer switches pattern groups
// as tag delimiters, comments, CDATA sections, and processing instructions
// are encountered.
//
// The lexer works unchanged when the package is built standalone and needs no
// filesystem capability. The error result is retained for API compatibility
// and is always nil.
//
// Example:
//
//	lex, err := xmllexer.CreateXmlLexer(`<div>hello</div>`)
//	if err != nil {
//	    panic(err)
//	}
//	tokens := lex.Tokenize()
func CreateXmlLexer(source string) (*lexer.GrammarLexer, error) {
	// Create the grammar-driven lexer. The GrammarLexer constructor compiles
	// all regex patterns and initializes skip pattern matching and pattern
	// group support.
	xmlLexer := lexer.NewGrammarLexer(source, TokenGrammarData)

	// Register the on-token callback. This callback fires after each token
	// match and switches pattern groups based on the token type. Without
	// this callback, the lexer would stay in the default group forever and
	// never recognize tag-internal patterns like attribute names and values.
	xmlLexer.SetOnToken(XmlOnToken)

	return xmlLexer, nil
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
// The error result is retained for API compatibility and is always nil.
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
