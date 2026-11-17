package lexer

// Grammar-Driven Lexer — Tokenization from .tokens Files
// =======================================================
//
// This file implements a lexer that is driven by a TokenGrammar (parsed from
// a .tokens file). Instead of hardcoded character-matching logic, it compiles
// token definitions into regexes and tries them in priority order at each
// position in the source.
//
// Key features:
//   - Skip patterns (whitespace, comments) consumed without tokens
//   - Type aliases (STRING_DQ -> STRING)
//   - Reserved keywords (lex-time errors)
//   - Indentation mode (INDENT/DEDENT/NEWLINE)
//   - Pattern groups with stackable group transitions
//   - On-token callback for context-sensitive lexing
//
// Pattern Groups & Callbacks
// --------------------------
//
// Pattern groups enable context-sensitive lexing. For example, an XML lexer
// defines a "tag" group with patterns for attribute names, equals signs, and
// quoted values. A callback registered via SetOnToken pushes the "tag" group
// when "<" is matched and pops it when ">" is matched.
//
// The callback receives a Token and a *LexerContext. The context provides
// controlled access to the group stack, token emission, suppression, and
// skip control. All context actions are deferred — they take effect after
// the callback returns, not during execution.

import (
	"fmt"
	"strings"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
)

// ---------------------------------------------------------------------------
// On-Token Callback Type
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Hook Types — Pre/Post Tokenization Transforms
// ---------------------------------------------------------------------------

// PreTokenizeHook transforms source text before tokenization.
// Multiple hooks compose left-to-right: source flows through A → B → C.
type PreTokenizeHook func(source string) string

// PostTokenizeHook transforms the token list after tokenization.
// Multiple hooks compose left-to-right.
type PostTokenizeHook func(tokens []Token) []Token

// OnTokenCallback is the function signature for on-token callbacks.
//
// The callback fires after each token match, before the token is added to
// the output list. It receives the matched token and a LexerContext that
// provides controlled access to the group stack, token emission, and
// skip control.
//
// The callback is NOT invoked for:
//   - Skip pattern matches (they produce no tokens)
//   - Tokens emitted via context.Emit() (prevents infinite loops)
//   - The EOF token
type OnTokenCallback func(token Token, ctx *LexerContext)

// ---------------------------------------------------------------------------
// Lexer Context — Callback Interface for Group Transitions
// ---------------------------------------------------------------------------

// LexerContext is the interface that on-token callbacks use to control the
// lexer. When a callback is registered via GrammarLexer.SetOnToken(), it
// receives a *LexerContext on every token match.
//
// Methods that modify state (PushGroup/PopGroup/Emit/Suppress) take effect
// after the callback returns — they do not interrupt the current match.
//
// Example — XML lexer callback:
//
//	func xmlHook(token lexer.Token, ctx *lexer.LexerContext) {
//	    if token.TypeName == "OPEN_TAG_START" {
//	        ctx.PushGroup("tag")
//	    } else if token.TypeName == "TAG_CLOSE" || token.TypeName == "SELF_CLOSE" {
//	        ctx.PopGroup()
//	    }
//	}
type LexerContext struct {
	// lexer is the GrammarLexer that created this context. Used to read
	// group patterns (for validation) and the group stack (for queries).
	lexer *GrammarLexer

	// source is the complete source text being tokenized.
	source string

	// posAfter is the position in source immediately after the current
	// token. Used by Peek and PeekStr to look ahead.
	posAfter int

	// suppressed tracks whether the callback called Suppress(). When true,
	// the current token is not added to the output list.
	suppressed bool

	// emitted collects tokens injected by Emit(). These are appended to
	// the output after the current token (or in place of it if suppressed).
	emitted []Token

	// groupActions collects push/pop actions to apply after the callback.
	// Each entry is (action, groupName) where action is "push" or "pop".
	groupActions []groupAction

	// skipEnabled records whether the callback changed skip processing.
	// nil means no change; non-nil means the callback set a new value.
	skipEnabled *bool

	// previousToken is the most recently emitted token (for lookbehind).
	// nil at the start of input.
	previousToken *Token

	// currentTokenLine is the line of the current token being processed,
	// used for newline detection (precededByNewline).
	currentTokenLine int
}

// groupAction represents a deferred push or pop on the group stack.
type groupAction struct {
	action    string // "push" or "pop"
	groupName string // group name for push; empty for pop
}

// PushGroup pushes a pattern group onto the group stack.
//
// The pushed group becomes active for the next token match. Panics if the
// group name is not defined in the grammar.
func (ctx *LexerContext) PushGroup(groupName string) {
	_, _ = StartNew[struct{}]("lexer.PushGroup", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("groupName", groupName)
			if !ctx.lexer.c.hasGroup(groupName) {
				available := make([]string, 0, len(ctx.lexer.c.tables))
				for name := range ctx.lexer.c.tables {
					available = append(available, name)
				}
				panic(fmt.Sprintf(
					"Unknown pattern group: %q. Available groups: %v",
					groupName, available,
				))
			}
			ctx.groupActions = append(ctx.groupActions, groupAction{"push", groupName})
			return rf.Generate(true, false, struct{}{})
		}).PanicOnUnexpected().GetResult()
}

// PopGroup pops the current group from the stack.
//
// If only the default group remains, this is a no-op. The default group is
// the floor of the stack and cannot be popped — this prevents accidental
// stack underflow when callbacks pop more times than they push.
func (ctx *LexerContext) PopGroup() {
	_, _ = StartNew[struct{}]("lexer.PopGroup", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			ctx.groupActions = append(ctx.groupActions, groupAction{"pop", ""})
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// ActiveGroup returns the name of the currently active group.
//
// The active group is the top of the group stack. When no groups have been
// pushed, this is always "default".
func (ctx *LexerContext) ActiveGroup() string {
	result, _ := StartNew[string]("lexer.ActiveGroup", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			return rf.Generate(true, false, ctx.lexer.groupStack[len(ctx.lexer.groupStack)-1])
		}).GetResult()
	return result
}

// GroupStackDepth returns the depth of the group stack (always >= 1).
//
// A depth of 1 means only the default group is on the stack. Each PushGroup
// call (once applied) increases the depth by 1.
func (ctx *LexerContext) GroupStackDepth() int {
	result, _ := StartNew[int]("lexer.GroupStackDepth", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, len(ctx.lexer.groupStack))
		}).GetResult()
	return result
}

// Emit injects a synthetic token after the current one.
//
// Emitted tokens do NOT trigger the callback (this prevents infinite loops
// where a callback emits a token that triggers itself). Multiple Emit calls
// produce tokens in call order.
func (ctx *LexerContext) Emit(token Token) {
	_, _ = StartNew[struct{}]("lexer.Emit", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			ctx.emitted = append(ctx.emitted, token)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Suppress marks the current token for suppression — it will not be included
// in the output. Emitted tokens (from Emit) are still included even when the
// current token is suppressed. This enables token replacement: suppress the
// original and emit a rewritten version.
func (ctx *LexerContext) Suppress() {
	_, _ = StartNew[struct{}]("lexer.Suppress", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			ctx.suppressed = true
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Peek reads a source character past the current token.
//
// offset=1 means the character immediately after the token. Returns an empty
// string if the position is past EOF.
func (ctx *LexerContext) Peek(offset int) string {
	result, _ := StartNew[string]("lexer.Peek", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			op.AddProperty("offset", offset)
			idx := ctx.posAfter + offset - 1
			if idx >= 0 && idx < len(ctx.source) {
				return rf.Generate(true, false, string(ctx.source[idx]))
			}
			return rf.Generate(true, false, "")
		}).GetResult()
	return result
}

// PeekStr reads the next `length` characters past the current token.
//
// Returns a shorter string if there are fewer than `length` characters
// remaining in the source.
func (ctx *LexerContext) PeekStr(length int) string {
	result, _ := StartNew[string]("lexer.PeekStr", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			op.AddProperty("length", length)
			end := ctx.posAfter + length
			if end > len(ctx.source) {
				end = len(ctx.source)
			}
			return rf.Generate(true, false, ctx.source[ctx.posAfter:end])
		}).GetResult()
	return result
}

// SetSkipEnabled toggles skip pattern processing.
//
// When disabled, skip patterns (whitespace, comments) are not tried. This is
// useful for groups where whitespace is significant (e.g., CDATA sections,
// raw text blocks). The change takes effect after the callback returns.
func (ctx *LexerContext) SetSkipEnabled(enabled bool) {
	_, _ = StartNew[struct{}]("lexer.SetSkipEnabled", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("enabled", enabled)
			ctx.skipEnabled = &enabled
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// ---------------------------------------------------------------------------
// Extension: Token Lookbehind
// ---------------------------------------------------------------------------

// PreviousToken returns the most recently emitted token, or nil at the start
// of input.
//
// "Emitted" means the token actually made it into the output list — suppressed
// tokens are not counted. This provides lookbehind capability for context-
// sensitive decisions.
//
// For example, in JavaScript `/` is a regex literal after `=`, `(` or `,`
// but a division operator after `)`, `]`, identifiers, or numbers. The
// callback can check ctx.PreviousToken().TypeName to decide which
// interpretation to use.
func (ctx *LexerContext) PreviousToken() *Token {
	return ctx.previousToken
}

// ---------------------------------------------------------------------------
// Extension: Bracket Depth Tracking
// ---------------------------------------------------------------------------

// BracketDepth returns the current nesting depth for a specific bracket type.
// Pass "paren" for (), "bracket" for [], "brace" for {}, or "" for the total
// depth across all three types.
//
// Depth starts at 0 and increments on each opener, decrements on each closer.
// The count never goes below 0 — unmatched closers are clamped.
//
// This is essential for template literal interpolation in languages like
// JavaScript, Kotlin, and Ruby, where `}` at brace-depth 0 closes the
// interpolation rather than being part of a nested expression.
func (ctx *LexerContext) BracketDepth(kind string) int {
	return ctx.lexer.BracketDepth(kind)
}

// ---------------------------------------------------------------------------
// Extension: Newline Detection
// ---------------------------------------------------------------------------

// PrecededByNewline returns true if a newline appeared between the previous
// token and the current token (i.e., they are on different lines).
//
// This is used by languages with automatic semicolon insertion (JavaScript, Go)
// to detect line breaks that trigger implicit statement termination. The lexer
// exposes this as a convenience so callbacks and post-tokenize hooks can set
// the TokenPrecededByNewline flag on tokens that need it.
//
// Returns false if there is no previous token (start of input).
func (ctx *LexerContext) PrecededByNewline() bool {
	if ctx.previousToken == nil {
		return false
	}
	return ctx.previousToken.Line < ctx.currentTokenLine
}

// ---------------------------------------------------------------------------
// The Grammar-Driven Lexer
// ---------------------------------------------------------------------------

// GrammarLexer tokenizes source code using grammar-defined token patterns.
//
// Instead of hardcoded character-matching logic, this lexer:
//
//  1. Compiles each token definition's pattern once (see Compile)
//  2. At each position, tries the rules that can start with the next byte, in
//     definition order (first match wins)
//  3. Emits a Token with the matched type and value
//
// Supports skip patterns, type aliases, reserved keywords, indentation mode,
// pattern groups with stackable transitions, and on-token callbacks.
//
// A GrammarLexer is the per-run state (position, group stack, hooks) wrapped
// around an immutable *Compiled. Use Compile / Compiled.Tokenize when the same
// grammar is used repeatedly or when a panic is unacceptable.
type GrammarLexer struct {
	// c is the shared, immutable compiled grammar.
	c *Compiled

	source string
	pos    int
	line   int
	column int

	// Indentation mode state
	indentStack  []int
	bracketDepth int // Legacy: single bracket depth for indentation mode

	// ---------------------------------------------------------------------------
	// Extension: Per-type bracket depth tracking
	// ---------------------------------------------------------------------------
	//
	// Tracks `()`, `[]`, and `{}` independently. Updated after each token match
	// in both standard and indentation modes. Exposed to callbacks via
	// LexerContext.BracketDepth().
	//
	// This enables context-sensitive lexing for template literals, string
	// interpolation, and other constructs where bracket nesting determines
	// how to tokenize subsequent input.
	bracketDepths struct {
		paren   int
		bracket int
		brace   int
	}

	// lastEmitted is the most recently emitted token, for lookbehind in
	// callbacks; hasLast says whether there is one. Reset on each Tokenize().
	lastEmitted Token
	hasLast     bool

	// groupStack is the stack of active pattern groups. The bottom is
	// always the start mode. The top determines which patterns are tried
	// during token matching. It starts out backed by stackBuf so ordinary
	// nesting depths need no allocation.
	groupStack []string
	stackBuf   [8]string

	// onToken is the optional callback that fires on every token match.
	// nil means no callback (zero overhead in the hot loop).
	onToken OnTokenCallback

	// skipEnabled controls whether skip patterns are processed. The
	// on-token callback can toggle this for groups where whitespace is
	// significant (e.g., CDATA sections).
	skipEnabled bool

	// preTokenizeHooks holds functions that transform the source text before
	// tokenization. Multiple hooks compose left-to-right (A → B → C).
	preTokenizeHooks []PreTokenizeHook

	// postTokenizeHooks holds functions that transform the token list after
	// tokenization. Multiple hooks compose left-to-right.
	postTokenizeHooks []PostTokenizeHook

	// originalSource stores the raw, unmodified input string.
	//
	// When the grammar's CaseSensitive field is false the working source
	// (l.source) is lowercased to make keyword pattern matching
	// case-insensitive.  Without this field STRING token values would also be
	// lowercased — 'Alice' would become 'alice'.  By keeping originalSource we
	// can extract the body of every STRING token from the original text at the
	// same byte offset, preserving whatever case the user wrote.
	originalSource string

	// indentBuf backs indentStack for the common shallow case.
	indentBuf [16]int
}

// NewGrammarLexer creates a new grammar-driven lexer.
//
// It compiles all token definitions, skip patterns, and group patterns (via
// Compile). The compiled patterns are anchored to the start of the remaining
// source so that only matches at the current position are found.
//
// Compatibility: a pattern that does not compile still causes this legacy
// constructor to fail (it yields a nil lexer, as it always did). New code
// should call Compile, which reports the error.
func NewGrammarLexer(source string, grammar *grammartools.TokenGrammar) *GrammarLexer {
	result, _ := StartNew[*GrammarLexer]("lexer.NewGrammarLexer", nil,
		func(op *Operation[*GrammarLexer], rf *ResultFactory[*GrammarLexer]) *OperationResult[*GrammarLexer] {
			return rf.Generate(true, false, newGrammarLexerImpl(source, grammar))
		}).GetResult()
	return result
}

func newGrammarLexerImpl(source string, grammar *grammartools.TokenGrammar) *GrammarLexer {
	c, err := compiledFor(grammar)
	if err != nil {
		panic(err.Error())
	}
	return c.newLexer(source)
}

// newLexer creates the per-run state for one source text.
func (c *Compiled) newLexer(source string) *GrammarLexer {
	// When CaseSensitive is false, lowercase the entire source before
	// tokenization. Keywords in the .tokens file are already lowercase, so
	// this makes keyword matching work.
	src := source
	if c.lowerSource {
		src = strings.ToLower(source)
	}
	l := &GrammarLexer{
		c:              c,
		source:         src,
		originalSource: source,
		line:           1,
		column:         1,
		skipEnabled:    true,
	}
	l.indentStack = l.indentBuf[:1]
	l.stackBuf[0] = c.startMode
	l.groupStack = l.stackBuf[:1]
	return l
}

// AddPreTokenize registers a text transform to run before tokenization.
// The hook receives the raw source string and returns a (possibly modified)
// source string. Multiple hooks compose left-to-right.
func (l *GrammarLexer) AddPreTokenize(hook PreTokenizeHook) {
	_, _ = StartNew[struct{}]("lexer.AddPreTokenize", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			l.preTokenizeHooks = append(l.preTokenizeHooks, hook)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// AddPostTokenize registers a token transform to run after tokenization.
// The hook receives the full token list (including EOF) and returns a
// (possibly modified) token list. Multiple hooks compose left-to-right.
func (l *GrammarLexer) AddPostTokenize(hook PostTokenizeHook) {
	_, _ = StartNew[struct{}]("lexer.AddPostTokenize", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			l.postTokenizeHooks = append(l.postTokenizeHooks, hook)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// SetOnToken registers a callback that fires on every token match.
//
// The callback receives the matched token and a *LexerContext. It can use
// the context to push/pop groups, emit extra tokens, suppress the current
// token, or toggle skip processing.
//
// Only one callback can be registered at a time. Pass nil to clear the
// callback.
//
// The callback is NOT invoked for:
//   - Skip pattern matches (they produce no tokens)
//   - Tokens emitted via context.Emit() (prevents infinite loops)
//   - The EOF token
func (l *GrammarLexer) SetOnToken(callback OnTokenCallback) {
	_, _ = StartNew[struct{}]("lexer.SetOnToken", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			l.onToken = callback
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// BracketDepth returns the current nesting depth for a specific bracket type.
// Pass "paren" for (), "bracket" for [], "brace" for {}, or "" for the total
// depth across all three types.
func (l *GrammarLexer) BracketDepth(kind string) int {
	switch kind {
	case "paren":
		return l.bracketDepths.paren
	case "bracket":
		return l.bracketDepths.bracket
	case "brace":
		return l.bracketDepths.brace
	default:
		return l.bracketDepths.paren + l.bracketDepths.bracket + l.bracketDepths.brace
	}
}

// updateBracketDepth updates per-type bracket depth counters based on a
// token's value. Called after each token match in both standard and
// indentation modes. Only single-character values are checked — multi-
// character tokens cannot be brackets.
func (l *GrammarLexer) updateBracketDepth(value string) {
	if len(value) != 1 {
		return
	}
	switch value {
	case "(":
		l.bracketDepths.paren++
	case ")":
		if l.bracketDepths.paren > 0 {
			l.bracketDepths.paren--
		}
	case "[":
		l.bracketDepths.bracket++
	case "]":
		if l.bracketDepths.bracket > 0 {
			l.bracketDepths.bracket--
		}
	case "{":
		l.bracketDepths.brace++
	case "}":
		if l.bracketDepths.brace > 0 {
			l.bracketDepths.brace--
		}
	}
}

// ---------------------------------------------------------------------------
// Character Advancement
// ---------------------------------------------------------------------------

func (l *GrammarLexer) advance() {
	if l.pos < len(l.source) {
		if l.source[l.pos] == '\n' {
			l.line++
			l.column = 1
		} else {
			l.column++
		}
		l.pos++
	}
}

// advanceN consumes n bytes (n never exceeds what is left), keeping line and
// column exact without stepping one byte at a time through the interpreter.
//
// If the consumed text contains no newline the column simply moves by n. If it
// does, the line moves by the number of newlines and the column restarts at 1
// after the last one:
//
//	consumed "ab\ncd" (n=5, last '\n' at index 2)  ->  column = n - 2 = 3
func (l *GrammarLexer) advanceN(n int) {
	seg := l.source[l.pos : l.pos+n]
	if i := strings.LastIndexByte(seg, '\n'); i >= 0 {
		l.line += strings.Count(seg, "\n")
		l.column = n - i
	} else {
		l.column += n
	}
	l.pos += n
}

// ---------------------------------------------------------------------------
// Token Type Resolution
// ---------------------------------------------------------------------------

// classify decides the final (TokenType, TypeName) for a match of rule r.
//
// For most rules the answer was settled when the grammar was compiled. Only a
// rule literally named NAME needs a run-time look at the matched text: it may
// be a reserved word (an error) or a keyword (type KEYWORD).
//
// In case-insensitive mode the sets hold upper-case words, so the matched text
// is upper-cased before the lookup.
func (l *GrammarLexer) classify(r *rule, value string) (TokenType, string, error) {
	if r.isName {
		c := l.c
		if len(c.reservedSet) > 0 || len(c.keywordSet) > 0 {
			lookup := value
			if c.caseInsensitive {
				lookup = strings.ToUpper(value)
			}
			if _, ok := c.reservedSet[lookup]; ok {
				return 0, "", &LexError{l.line, l.column,
					fmt.Sprintf("Reserved keyword %q cannot be used as an identifier", value)}
			}
			if _, ok := c.keywordSet[lookup]; ok {
				return TokenKeyword, "KEYWORD", nil
			}
		}
	}
	return r.tokenType, r.typeName, nil
}

// ---------------------------------------------------------------------------
// Skip Pattern Matching
// ---------------------------------------------------------------------------

// trySkip attempts to match and consume a skip pattern at the current position.
func (l *GrammarLexer) trySkip() (bool, error) {
	remaining := l.source[l.pos:]
	for _, r := range l.c.skip.byByte[remaining[0]] {
		n := r.matchLen(remaining)
		if n < 0 {
			continue
		}
		if n == 0 {
			// The legacy loop would spin forever here; refuse instead.
			return false, &LexError{l.line, l.column,
				fmt.Sprintf("Skip pattern %s matched the empty string", r.name)}
		}
		l.advanceN(n)
		return true, nil
	}
	return false, nil
}

// ---------------------------------------------------------------------------
// Token Matching — Per-Group
// ---------------------------------------------------------------------------

// tableFor returns the dispatch table of the active (top-of-stack) group.
func (l *GrammarLexer) tableFor() *ruleTable {
	return l.c.table(l.groupStack[len(l.groupStack)-1])
}

// matchToken attempts to match a token at the current position using one
// group's rules.
//
// Only the rules that can start with the next byte are tried, in grammar order
// (first match wins). Handles keyword detection, reserved word checking,
// aliases, and string quote stripping. On a match the position is advanced.
//
// Returns (token, true, nil) on a match, (_, false, nil) when nothing matches
// and (_, false, err) for a reserved word or a rule that matched nothing at all.
func (l *GrammarLexer) matchToken(t *ruleTable) (Token, bool, error) {
	remaining := l.source[l.pos:]

	for _, r := range t.byByte[remaining[0]] {
		n := r.matchLen(remaining)
		if n < 0 {
			continue
		}
		if n == 0 {
			// The legacy loop would emit tokens forever here; refuse instead.
			return Token{}, false, &LexError{l.line, l.column,
				fmt.Sprintf("Token rule %s matched the empty string", r.name)}
		}

		value := remaining[:n]
		tType, typeName, err := l.classify(r, value)
		if err != nil {
			return Token{}, false, err
		}

		// In case-insensitive mode, normalize KEYWORD token values to
		// uppercase. This ensures grammar literals like "SELECT" match
		// regardless of how the user typed the keyword (select/SELECT/Select).
		if l.c.caseInsensitive && tType == TokenKeyword {
			value = strings.ToUpper(value)
		}

		if r.stringLike {
			value = l.stripQuotes(value, n)
		}

		// Context keywords are emitted as NAME with the TokenContextKeyword
		// flag, leaving the final decision to the parser or a callback.
		var flags int
		if typeName == "NAME" && len(l.c.contextKeywordSet) > 0 {
			if _, isCtx := l.c.contextKeywordSet[value]; isCtx {
				flags = TokenContextKeyword
			}
		}

		tok := Token{Type: tType, Value: value, Line: l.line, Column: l.column, TypeName: typeName, Flags: flags}
		l.advanceN(n)
		return tok, true, nil
	}
	return Token{}, false, nil
}

// stripQuotes implements the legacy rule for tokens whose rule name (or alias)
// contains "STRING": if the matched text begins with a quote character (" or
// ') and is at least two bytes long, the surrounding quotes are removed and,
// unless the grammar says `escapes: none`, backslash escapes are processed.
// Triple-quoted text (three double or three single quotes, six bytes or more) has three bytes stripped
// from each end.
//
// This is a naming convention rather than a declared feature, and it surprises
// people: a token called QUOTED_STRING or STRING_LITERAL has its quotes
// removed, one called QUOTED does not. It is kept exactly as-is for
// compatibility with every existing grammar; the cure for a grammar that does
// not want it is to avoid "STRING" in the token name.
//
// Case-preservation: when the grammar is case-insensitive the working source
// is a lowercased copy, so the body is re-read from the original text at the
// same byte offset ('Alice' must not become 'alice').
func (l *GrammarLexer) stripQuotes(value string, tokenLen int) string {
	originalValue := value
	if l.c.caseInsensitive && tokenLen > 0 && l.pos+tokenLen <= len(l.originalSource) {
		originalValue = l.originalSource[l.pos : l.pos+tokenLen]
	}
	if len(originalValue) < 2 {
		return value
	}
	quote := originalValue[0]
	if quote != '"' && quote != '\'' {
		return value
	}
	var inner string
	if len(originalValue) >= 6 && originalValue[0] == originalValue[1] && originalValue[1] == originalValue[2] {
		inner = originalValue[3 : len(originalValue)-3]
	} else {
		inner = originalValue[1 : len(originalValue)-1]
	}
	if !l.c.escapeNone {
		inner = processEscapes(inner)
	}
	return inner
}

// ---------------------------------------------------------------------------
// Escape Processing
// ---------------------------------------------------------------------------

// processEscapes handles escape sequences in string literals.
func processEscapes(s string) string {
	// Fast path: no backslash means nothing to process (and no allocation).
	if strings.IndexByte(s, '\\') < 0 {
		return s
	}
	var sb strings.Builder
	i := 0
	for i < len(s) {
		if s[i] == '\\' && i+1 < len(s) {
			next := s[i+1]
			switch next {
			case 'n':
				sb.WriteByte('\n')
			case 't':
				sb.WriteByte('\t')
			case '\\':
				sb.WriteByte('\\')
			case '"':
				sb.WriteByte('"')
			default:
				sb.WriteByte(next)
			}
			i += 2
		} else {
			sb.WriteByte(s[i])
			i++
		}
	}
	return sb.String()
}

// ---------------------------------------------------------------------------
// Main Tokenization Entry Point
// ---------------------------------------------------------------------------

// Tokenize tokenizes the source using the grammar's token definitions.
//
// Dispatches to the appropriate tokenization method based on whether
// indentation mode is active.
//
// Legacy behaviour: unexpected input panics (with the text of the LexError).
// Callers that need an error instead should use Compiled.Tokenize.
func (l *GrammarLexer) Tokenize() []Token {
	result, _ := StartNew[[]Token]("lexer.GrammarLexer.Tokenize", nil,
		func(op *Operation[[]Token], rf *ResultFactory[[]Token]) *OperationResult[[]Token] {
			tokens, err := l.tokenizeAll()
			if err != nil {
				panic(err.Error())
			}
			return rf.Generate(true, false, tokens)
		}).PanicOnUnexpected().GetResult()
	return result
}

// tokenizeAll runs the whole pipeline (pre-hooks, core, post-hooks) and
// returns an error instead of panicking. It is the single engine behind both
// GrammarLexer.Tokenize and Compiled.Tokenize.
func (l *GrammarLexer) tokenizeAll() ([]Token, error) {
	// Stage 1: Pre-tokenize hooks transform the source text.
	// Each hook receives the output of the previous hook, composing
	// left-to-right. This enables source-level transforms like macro
	// expansion or encoding normalization before any pattern matching.
	if len(l.preTokenizeHooks) > 0 {
		source := l.source
		for _, hook := range l.preTokenizeHooks {
			source = hook(source)
		}
		l.source = source
	}

	// Reset extension state for reuse between Tokenize() calls.
	l.hasLast = false
	l.bracketDepths.paren = 0
	l.bracketDepths.bracket = 0
	l.bracketDepths.brace = 0

	// Stage 2: Core tokenization.
	var tokens []Token
	var err error
	if l.c.indentMode {
		tokens, err = l.tokenizeIndentation()
	} else if l.c.layoutMode {
		tokens, err = l.tokenizeLayout()
	} else {
		tokens, err = l.tokenizeStandard()
	}
	if err != nil {
		return nil, err
	}

	// Stage 3: Post-tokenize hooks transform the token list.
	// Each hook receives the full token list (including EOF) and returns
	// a (possibly modified) token list. Hooks compose left-to-right.
	for _, hook := range l.postTokenizeHooks {
		tokens = hook(tokens)
	}
	return tokens, nil
}

// ---------------------------------------------------------------------------
// Standard (Non-Indentation) Tokenization
// ---------------------------------------------------------------------------

// tokenCapacityHint guesses how many tokens a source will produce so the
// output slice is allocated once instead of regrown. Real source averages
// roughly one token per four to six bytes; the cap bounds the up-front cost
// for very large inputs (append still grows beyond it if needed).
func tokenCapacityHint(sourceLen int) int {
	n := sourceLen/3 + 2
	if n > 1<<16 {
		n = 1 << 16
	}
	return n
}

// tokenizeStandard tokenizes without indentation tracking.
//
// The algorithm:
//
//  1. While there are characters left:
//     a. If skip patterns exist and skip is enabled, try them first.
//     b. If no skip patterns, use default whitespace skip (space/tab/CR).
//     c. If the current character is a newline, emit NEWLINE.
//     d. Try the active group's token patterns (first match wins).
//     e. If a callback is registered, invoke it and process actions.
//     f. If nothing matches, return a LexError.
//  2. Append EOF.
//  3. Reset group stack and skip state for reuse.
func (l *GrammarLexer) tokenizeStandard() ([]Token, error) {
	tokens := make([]Token, 0, tokenCapacityHint(len(l.source)))

	for l.pos < len(l.source) {
		char := l.source[l.pos]

		// --- Skip patterns (grammar-defined) ---
		// When the grammar has skip patterns AND skip is enabled, they take
		// over whitespace handling. The callback can disable skip processing
		// for groups where whitespace is significant (e.g., CDATA).
		if l.c.hasSkip {
			if l.skipEnabled {
				skipped, err := l.trySkip()
				if err != nil {
					return nil, err
				}
				if skipped {
					continue
				}
			}
		} else {
			// --- Default whitespace skip ---
			// Without skip patterns, use the hardcoded behavior: skip spaces,
			// tabs, and carriage returns silently.
			if char == ' ' || char == '\t' || char == '\r' {
				l.advance()
				continue
			}
		}

		// --- Newlines become NEWLINE tokens ---
		// Newlines are structural — they mark line boundaries.
		if char == '\n' {
			newlineTok := Token{Type: TokenNewline, Value: "\\n", Line: l.line, Column: l.column, TypeName: "NEWLINE"}
			tokens = append(tokens, newlineTok)
			l.lastEmitted, l.hasLast = newlineTok, true
			l.advance()
			continue
		}

		// --- Try active group's token patterns (first match wins) ---
		// The active group is the top of the group stack. When no groups
		// are defined, this is always "default" (the top-level definitions),
		// preserving backward compatibility.
		tok, matched, err := l.matchToken(l.tableFor())
		if err != nil {
			return nil, err
		}
		if !matched {
			return nil, &LexError{l.line, l.column, fmt.Sprintf("Unexpected character %q", char)}
		}

		// Update per-type bracket depth tracking.
		l.updateBracketDepth(tok.Value)

		if l.onToken == nil {
			tokens = append(tokens, tok)
			l.lastEmitted, l.hasLast = tok, true
			// F10: Apply declarative transitions.
			l.applyTransitions(tok)
			continue
		}

		// --- Invoke on-token callback ---
		// The callback can push/pop groups, emit extra tokens, suppress
		// the current token, or toggle skip processing. Emitted tokens
		// do NOT re-trigger the callback.
		ctx := &LexerContext{
			lexer:            l,
			source:           l.source,
			posAfter:         l.pos,
			currentTokenLine: tok.Line,
		}
		if l.hasLast {
			prev := l.lastEmitted
			ctx.previousToken = &prev
		}
		l.onToken(tok, ctx)

		// Apply suppression: if the callback suppressed this token,
		// don't add it to the output.
		if !ctx.suppressed {
			tokens = append(tokens, tok)
			l.lastEmitted, l.hasLast = tok, true
		}

		// Append any tokens emitted by the callback.
		for _, emitted := range ctx.emitted {
			tokens = append(tokens, emitted)
			l.lastEmitted, l.hasLast = emitted, true
		}

		// Apply group stack actions in order.
		for _, action := range ctx.groupActions {
			if action.action == "push" {
				l.groupStack = append(l.groupStack, action.groupName)
			} else if action.action == "pop" && len(l.groupStack) > 1 {
				l.groupStack = l.groupStack[:len(l.groupStack)-1]
			}
		}

		// Apply skip toggle if the callback changed it.
		if ctx.skipEnabled != nil {
			l.skipEnabled = *ctx.skipEnabled
		}

		// F10: Apply declarative transitions after callback processing.
		// Transitions see the group stack state after the callback has
		// already applied its push/pop actions.
		l.applyTransitions(tok)
	}

	tokens = append(tokens, Token{Type: TokenEOF, Value: "", Line: l.line, Column: l.column, TypeName: "EOF"})

	// Reset group stack and skip state for reuse. This ensures the lexer
	// can be called multiple times without group state leaking between
	// tokenize() calls.
	l.stackBuf[0] = l.c.startMode
	l.groupStack = l.stackBuf[:1]
	l.skipEnabled = true

	return tokens, nil
}

// ---------------------------------------------------------------------------
// Indentation Mode Tokenization
// ---------------------------------------------------------------------------

func (l *GrammarLexer) tokenizeIndentation() ([]Token, error) {
	tokens := make([]Token, 0, tokenCapacityHint(len(l.source)))
	atLineStart := true

	for l.pos < len(l.source) {
		// Process line start
		if atLineStart && l.bracketDepth == 0 {
			indentTokens, skipLine, err := l.processLineStart()
			if err != nil {
				return nil, err
			}
			if skipLine {
				continue
			}
			tokens = append(tokens, indentTokens...)
			atLineStart = false
			if l.pos >= len(l.source) {
				break
			}
		}

		char := l.source[l.pos]

		// Newline handling
		if char == '\n' {
			if l.bracketDepth == 0 {
				tokens = append(tokens, Token{Type: TokenNewline, Value: "\\n", Line: l.line, Column: l.column, TypeName: "NEWLINE"})
			}
			l.advance()
			atLineStart = true
			continue
		}

		// Inside brackets: skip whitespace
		if l.bracketDepth > 0 && (char == ' ' || char == '\t' || char == '\r') {
			l.advance()
			continue
		}

		// Try skip patterns
		skipped, err := l.trySkip()
		if err != nil {
			return nil, err
		}
		if skipped {
			continue
		}

		// Try token patterns (indentation mode does not use groups, so this
		// is always the default group).
		tok, matched, err := l.matchToken(l.c.table("default"))
		if err != nil {
			return nil, err
		}
		if matched {
			// Track bracket depth (legacy single counter for indentation logic)
			switch tok.Value {
			case "(", "[", "{":
				l.bracketDepth++
			case ")", "]", "}":
				l.bracketDepth--
			}
			// Track per-type bracket depth (shared for callback access)
			l.updateBracketDepth(tok.Value)
			tokens = append(tokens, tok)
			l.lastEmitted, l.hasLast = tok, true
			// F10: Apply declarative transitions after each regular token.
			l.applyTransitions(tok)
			continue
		}

		return nil, &LexError{l.line, l.column, fmt.Sprintf("Unexpected character %q", char)}
	}

	// EOF: emit NEWLINE to end the last statement, then DEDENTs to close
	// all open blocks.  The parser expects NEWLINE before DEDENT because
	// NEWLINE terminates the simple_stmt that precedes the block close.
	// Python's tokenizer follows the same order: NEWLINE, DEDENT, ..., EOF.
	if len(tokens) == 0 || tokens[len(tokens)-1].Type != TokenNewline {
		tokens = append(tokens, Token{Type: TokenNewline, Value: "\\n", Line: l.line, Column: l.column, TypeName: "NEWLINE"})
	}

	for len(l.indentStack) > 1 {
		l.indentStack = l.indentStack[:len(l.indentStack)-1]
		tokens = append(tokens, Token{Type: TokenName, Value: "", Line: l.line, Column: l.column, TypeName: "DEDENT"})
	}

	tokens = append(tokens, Token{Type: TokenEOF, Value: "", Line: l.line, Column: l.column, TypeName: "EOF"})
	return tokens, nil
}

func (l *GrammarLexer) tokenizeLayout() ([]Token, error) {
	tokens, err := l.tokenizeStandard()
	if err != nil {
		return nil, err
	}
	return l.applyLayout(tokens), nil
}

func (l *GrammarLexer) applyLayout(tokens []Token) []Token {
	result := make([]Token, 0, len(tokens))
	layoutStack := []int{}
	pendingLayouts := 0
	suppressDepth := 0

	for index, token := range tokens {
		typeName := token.TypeName

		if typeName == "NEWLINE" {
			result = append(result, token)
			nextToken := l.nextLayoutToken(tokens, index+1)
			if suppressDepth == 0 && nextToken != nil {
				for len(layoutStack) > 0 && nextToken.Column < layoutStack[len(layoutStack)-1] {
					result = append(result, l.virtualLayoutToken("VIRTUAL_RBRACE", "}", *nextToken))
					layoutStack = layoutStack[:len(layoutStack)-1]
				}

				if len(layoutStack) > 0 &&
					nextToken.TypeName != "EOF" &&
					nextToken.Value != "}" &&
					nextToken.Column == layoutStack[len(layoutStack)-1] {
					result = append(result, l.virtualLayoutToken("VIRTUAL_SEMICOLON", ";", *nextToken))
				}
			}
			continue
		}

		if typeName == "EOF" {
			for len(layoutStack) > 0 {
				result = append(result, l.virtualLayoutToken("VIRTUAL_RBRACE", "}", token))
				layoutStack = layoutStack[:len(layoutStack)-1]
			}
			result = append(result, token)
			continue
		}

		if pendingLayouts > 0 {
			if token.Value == "{" {
				pendingLayouts--
			} else {
				for count := 0; count < pendingLayouts; count++ {
					layoutStack = append(layoutStack, token.Column)
					result = append(result, l.virtualLayoutToken("VIRTUAL_LBRACE", "{", token))
				}
				pendingLayouts = 0
			}
		}

		result = append(result, token)

		if !strings.HasPrefix(token.TypeName, "VIRTUAL_") {
			switch token.Value {
			case "(", "[", "{":
				suppressDepth++
			case ")", "]", "}":
				if suppressDepth > 0 {
					suppressDepth--
				}
			}
		}

		if l.isLayoutKeyword(token) {
			pendingLayouts++
		}
	}

	return result
}

func (l *GrammarLexer) nextLayoutToken(tokens []Token, startIndex int) *Token {
	for idx := startIndex; idx < len(tokens); idx++ {
		if tokens[idx].TypeName != "NEWLINE" {
			return &tokens[idx]
		}
	}
	return nil
}

func (l *GrammarLexer) virtualLayoutToken(typeName, value string, anchor Token) Token {
	return Token{
		Type:     TokenName,
		Value:    value,
		Line:     anchor.Line,
		Column:   anchor.Column,
		TypeName: typeName,
	}
}

func (l *GrammarLexer) isLayoutKeyword(token Token) bool {
	if len(l.c.layoutKeywordSet) == 0 {
		return false
	}
	if _, ok := l.c.layoutKeywordSet[token.Value]; ok {
		return true
	}
	_, ok := l.c.layoutKeywordSet[strings.ToLower(token.Value)]
	return ok
}

// processLineStart handles indentation at the start of a logical line.
// Returns (indentTokens, skipLine, err).
func (l *GrammarLexer) processLineStart() ([]Token, bool, error) {
	indent := 0
	for l.pos < len(l.source) {
		char := l.source[l.pos]
		if char == ' ' {
			indent++
			l.advance()
		} else if char == '\t' {
			return nil, false, &LexError{l.line, l.column, "Tab character in indentation (use spaces only)"}
		} else {
			break
		}
	}

	// Blank line or EOF
	if l.pos >= len(l.source) {
		return nil, true, nil
	}
	if l.source[l.pos] == '\n' {
		l.advance()
		return nil, true, nil
	}

	// Comment-only line
	remaining := l.source[l.pos:]
	for _, r := range l.c.skip.byByte[remaining[0]] {
		n := r.matchLen(remaining)
		if n < 0 {
			continue
		}
		peekPos := l.pos + n
		if peekPos >= len(l.source) || l.source[peekPos] == '\n' {
			l.advanceN(n)
			if l.pos < len(l.source) && l.source[l.pos] == '\n' {
				l.advance()
			}
			return nil, true, nil
		}
	}

	// Compare indent to current level
	currentIndent := l.indentStack[len(l.indentStack)-1]
	var tokens []Token

	if indent > currentIndent {
		l.indentStack = append(l.indentStack, indent)
		tokens = append(tokens, Token{Type: TokenName, Value: "", Line: l.line, Column: 1, TypeName: "INDENT"})
	} else if indent < currentIndent {
		for len(l.indentStack) > 1 && l.indentStack[len(l.indentStack)-1] > indent {
			l.indentStack = l.indentStack[:len(l.indentStack)-1]
			tokens = append(tokens, Token{Type: TokenName, Value: "", Line: l.line, Column: 1, TypeName: "DEDENT"})
		}
		if l.indentStack[len(l.indentStack)-1] != indent {
			return nil, false, &LexError{l.line, l.column, "Inconsistent dedent"}
		}
	}

	return tokens, false, nil
}
