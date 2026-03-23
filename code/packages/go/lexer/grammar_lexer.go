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
	"regexp"
	"strings"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
)

// ---------------------------------------------------------------------------
// Compiled Pattern — A regex paired with its token name and optional alias
// ---------------------------------------------------------------------------

type compiledPattern struct {
	Name    string
	Pattern *regexp.Regexp
	Alias   string // Optional type alias
}

// ---------------------------------------------------------------------------
// On-Token Callback Type
// ---------------------------------------------------------------------------

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
	if _, ok := ctx.lexer.groupPatterns[groupName]; !ok {
		// Collect available group names for a helpful error message.
		available := make([]string, 0, len(ctx.lexer.groupPatterns))
		for name := range ctx.lexer.groupPatterns {
			available = append(available, name)
		}
		panic(fmt.Sprintf(
			"Unknown pattern group: %q. Available groups: %v",
			groupName, available,
		))
	}
	ctx.groupActions = append(ctx.groupActions, groupAction{"push", groupName})
}

// PopGroup pops the current group from the stack.
//
// If only the default group remains, this is a no-op. The default group is
// the floor of the stack and cannot be popped — this prevents accidental
// stack underflow when callbacks pop more times than they push.
func (ctx *LexerContext) PopGroup() {
	ctx.groupActions = append(ctx.groupActions, groupAction{"pop", ""})
}

// ActiveGroup returns the name of the currently active group.
//
// The active group is the top of the group stack. When no groups have been
// pushed, this is always "default".
func (ctx *LexerContext) ActiveGroup() string {
	return ctx.lexer.groupStack[len(ctx.lexer.groupStack)-1]
}

// GroupStackDepth returns the depth of the group stack (always >= 1).
//
// A depth of 1 means only the default group is on the stack. Each PushGroup
// call (once applied) increases the depth by 1.
func (ctx *LexerContext) GroupStackDepth() int {
	return len(ctx.lexer.groupStack)
}

// Emit injects a synthetic token after the current one.
//
// Emitted tokens do NOT trigger the callback (this prevents infinite loops
// where a callback emits a token that triggers itself). Multiple Emit calls
// produce tokens in call order.
func (ctx *LexerContext) Emit(token Token) {
	ctx.emitted = append(ctx.emitted, token)
}

// Suppress marks the current token for suppression — it will not be included
// in the output. Emitted tokens (from Emit) are still included even when the
// current token is suppressed. This enables token replacement: suppress the
// original and emit a rewritten version.
func (ctx *LexerContext) Suppress() {
	ctx.suppressed = true
}

// Peek reads a source character past the current token.
//
// offset=1 means the character immediately after the token. Returns an empty
// string if the position is past EOF.
func (ctx *LexerContext) Peek(offset int) string {
	idx := ctx.posAfter + offset - 1
	if idx >= 0 && idx < len(ctx.source) {
		return string(ctx.source[idx])
	}
	return ""
}

// PeekStr reads the next `length` characters past the current token.
//
// Returns a shorter string if there are fewer than `length` characters
// remaining in the source.
func (ctx *LexerContext) PeekStr(length int) string {
	end := ctx.posAfter + length
	if end > len(ctx.source) {
		end = len(ctx.source)
	}
	return ctx.source[ctx.posAfter:end]
}

// SetSkipEnabled toggles skip pattern processing.
//
// When disabled, skip patterns (whitespace, comments) are not tried. This is
// useful for groups where whitespace is significant (e.g., CDATA sections,
// raw text blocks). The change takes effect after the callback returns.
func (ctx *LexerContext) SetSkipEnabled(enabled bool) {
	ctx.skipEnabled = &enabled
}

// ---------------------------------------------------------------------------
// The Grammar-Driven Lexer
// ---------------------------------------------------------------------------

// GrammarLexer tokenizes source code using grammar-defined token patterns.
//
// Instead of hardcoded character-matching logic, this lexer:
//
//  1. Compiles each token definition's pattern into a regex
//  2. At each position, tries each regex in definition order (first match wins)
//  3. Emits a Token with the matched type and value
//
// Supports skip patterns, type aliases, reserved keywords, indentation mode,
// pattern groups with stackable transitions, and on-token callbacks.
type GrammarLexer struct {
	source       string
	grammar      *grammartools.TokenGrammar
	pos          int
	line         int
	column       int
	keywordSet   map[string]struct{}
	reservedSet  map[string]struct{}
	patterns     []compiledPattern
	skipPatterns []*regexp.Regexp

	// hasSkipPatterns tracks whether the grammar defines skip patterns.
	// When true, skip patterns take over whitespace handling entirely.
	// When false, the lexer falls back to hardcoded space/tab/CR skipping.
	hasSkipPatterns bool

	// Indentation mode state
	indentMode   bool
	indentStack  []int
	bracketDepth int

	// --- Pattern groups ---
	// groupPatterns maps group names to their compiled patterns. The
	// "default" group uses the top-level definitions. Named groups use
	// their own definitions from the grammar's "group:" sections.
	groupPatterns map[string][]compiledPattern

	// groupStack is the stack of active pattern groups. The bottom is
	// always "default". The top determines which patterns are tried
	// during token matching.
	groupStack []string

	// onToken is the optional callback that fires on every token match.
	// nil means no callback (zero overhead in the hot loop).
	onToken OnTokenCallback

	// skipEnabled controls whether skip patterns are processed. The
	// on-token callback can toggle this for groups where whitespace is
	// significant (e.g., CDATA sections).
	skipEnabled bool

	// aliasMap maps token definition names to their aliases. Used during
	// group pattern compilation to register aliases from group definitions.
	aliasMap map[string]string
}

// NewGrammarLexer creates a new grammar-driven lexer.
//
// It compiles all token definitions, skip patterns, and group patterns into
// regexes. The compiled patterns are anchored to the start of the remaining
// source (using ^) so that only matches at the current position are found.
func NewGrammarLexer(source string, grammar *grammartools.TokenGrammar) *GrammarLexer {
	// Case-insensitive mode: lowercase the entire source before tokenization.
	// This ensures keyword matching works because keywords in the .tokens file
	// are already lowercase, and now the source text will be too.
	src := source
	if !grammar.CaseSensitive {
		src = strings.ToLower(source)
	}

	keywordSet := make(map[string]struct{})
	for _, kw := range grammar.Keywords {
		keywordSet[kw] = struct{}{}
	}

	reservedSet := make(map[string]struct{})
	for _, rk := range grammar.ReservedKeywords {
		reservedSet[rk] = struct{}{}
	}

	// Build alias map: definition name -> alias name.
	// For example, STRING_DQ -> STRING. When we match STRING_DQ, we emit
	// the token type as STRING (the alias).
	aliasMap := make(map[string]string)
	for _, defn := range grammar.Definitions {
		if defn.Alias != "" {
			aliasMap[defn.Name] = defn.Alias
		}
	}

	// Compile top-level token patterns.
	var patterns []compiledPattern
	for _, defn := range grammar.Definitions {
		var patStr string
		if defn.IsRegex {
			patStr = "^" + defn.Pattern
		} else {
			patStr = "^" + regexp.QuoteMeta(defn.Pattern)
		}

		pat, err := regexp.Compile(patStr)
		if err != nil {
			panic(fmt.Sprintf("Failed to compile pattern for token %s: %v", defn.Name, err))
		}
		patterns = append(patterns, compiledPattern{Name: defn.Name, Pattern: pat, Alias: defn.Alias})
	}

	// Compile skip patterns (comments, whitespace, etc.).
	var skipPatterns []*regexp.Regexp
	for _, defn := range grammar.SkipDefinitions {
		var patStr string
		if defn.IsRegex {
			patStr = "^" + defn.Pattern
		} else {
			patStr = "^" + regexp.QuoteMeta(defn.Pattern)
		}
		pat, err := regexp.Compile(patStr)
		if err != nil {
			panic(fmt.Sprintf("Failed to compile skip pattern %s: %v", defn.Name, err))
		}
		skipPatterns = append(skipPatterns, pat)
	}

	// --- Compile pattern groups ---
	// The "default" group uses the top-level definitions. Named groups
	// use their own definitions from the grammar's "group:" sections.
	groupPatterns := map[string][]compiledPattern{
		"default": append([]compiledPattern{}, patterns...),
	}
	for groupName, group := range grammar.Groups {
		var compiled []compiledPattern
		for _, defn := range group.Definitions {
			var patStr string
			if defn.IsRegex {
				patStr = "^" + defn.Pattern
			} else {
				patStr = "^" + regexp.QuoteMeta(defn.Pattern)
			}
			pat, err := regexp.Compile(patStr)
			if err != nil {
				panic(fmt.Sprintf("Failed to compile group pattern %s/%s: %v", groupName, defn.Name, err))
			}
			compiled = append(compiled, compiledPattern{Name: defn.Name, Pattern: pat, Alias: defn.Alias})
			// Register aliases from group definitions.
			if defn.Alias != "" {
				aliasMap[defn.Name] = defn.Alias
			}
		}
		groupPatterns[groupName] = compiled
	}

	return &GrammarLexer{
		source:          src,
		grammar:         grammar,
		pos:             0,
		line:            1,
		column:          1,
		keywordSet:      keywordSet,
		reservedSet:     reservedSet,
		patterns:        patterns,
		skipPatterns:    skipPatterns,
		hasSkipPatterns: len(grammar.SkipDefinitions) > 0,
		indentMode:      grammar.Mode == "indentation",
		indentStack:     []int{0},
		bracketDepth:    0,
		groupPatterns:   groupPatterns,
		groupStack:      []string{"default"},
		onToken:         nil,
		skipEnabled:     true,
		aliasMap:        aliasMap,
	}
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
	l.onToken = callback
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

// ---------------------------------------------------------------------------
// Token Type Resolution
// ---------------------------------------------------------------------------

// resolveTokenType maps a grammar token name to a TokenType and TypeName.
// Returns (tokenType, typeName) where typeName is the effective name for
// the parser (alias if present, otherwise the definition name).
func (l *GrammarLexer) resolveTokenType(tokenName string, value string, alias string) (TokenType, string) {
	// Reserved keyword check
	if tokenName == "NAME" {
		if _, ok := l.reservedSet[value]; ok {
			panic(fmt.Sprintf("LexerError at %d:%d: Reserved keyword %q cannot be used as an identifier", l.line, l.column, value))
		}
	}

	// Regular keyword check
	if tokenName == "NAME" {
		if _, ok := l.keywordSet[value]; ok {
			return TokenKeyword, "KEYWORD"
		}
	}

	// Determine effective name (alias takes precedence)
	effectiveName := tokenName
	if alias != "" {
		effectiveName = alias
	}

	// Try known token types
	switch effectiveName {
	case "NAME":
		return TokenName, "NAME"
	case "NUMBER":
		return TokenNumber, "NUMBER"
	case "STRING":
		return TokenString, "STRING"
	case "PLUS":
		return TokenPlus, "PLUS"
	case "MINUS":
		return TokenMinus, "MINUS"
	case "STAR":
		return TokenStar, "STAR"
	case "SLASH":
		return TokenSlash, "SLASH"
	case "EQUALS":
		return TokenEquals, "EQUALS"
	case "EQUALS_EQUALS":
		return TokenEqualsEquals, "EQUALS_EQUALS"
	case "LPAREN":
		return TokenLParen, "LPAREN"
	case "RPAREN":
		return TokenRParen, "RPAREN"
	case "COMMA":
		return TokenComma, "COMMA"
	case "COLON":
		return TokenColon, "COLON"
	case "SEMICOLON":
		return TokenSemicolon, "SEMICOLON"
	case "LBRACE":
		return TokenLBrace, "LBRACE"
	case "RBRACE":
		return TokenRBrace, "RBRACE"
	case "LBRACKET":
		return TokenLBracket, "LBRACKET"
	case "RBRACKET":
		return TokenRBracket, "RBRACKET"
	case "DOT":
		return TokenDot, "DOT"
	case "BANG":
		return TokenBang, "BANG"
	default:
		// Unknown type -- use TokenName as base but store the string type
		return TokenName, effectiveName
	}
}

// ---------------------------------------------------------------------------
// Skip Pattern Matching
// ---------------------------------------------------------------------------

// trySkip attempts to match and consume a skip pattern at the current position.
func (l *GrammarLexer) trySkip() bool {
	remaining := l.source[l.pos:]
	for _, pat := range l.skipPatterns {
		loc := pat.FindStringIndex(remaining)
		if loc != nil && loc[0] == 0 {
			for i := 0; i < loc[1]; i++ {
				l.advance()
			}
			return true
		}
	}
	return false
}

// ---------------------------------------------------------------------------
// Token Matching — Per-Group
// ---------------------------------------------------------------------------

// tryMatchToken attempts to match a token using the default group's patterns.
// This is used by the indentation tokenizer which does not support groups.
func (l *GrammarLexer) tryMatchToken() *Token {
	return l.tryMatchTokenInGroup("default")
}

// tryMatchTokenInGroup attempts to match a token at the current position
// using the patterns from a specific group.
//
// It tries each compiled pattern in the named group in priority order (first
// match wins). Handles keyword detection, reserved word checking, aliases,
// and string escape processing.
func (l *GrammarLexer) tryMatchTokenInGroup(groupName string) *Token {
	remaining := l.source[l.pos:]

	// Look up the group's patterns. Fall back to default if unknown.
	patterns, ok := l.groupPatterns[groupName]
	if !ok {
		patterns = l.patterns
	}

	for _, p := range patterns {
		loc := p.Pattern.FindStringIndex(remaining)
		if loc != nil && loc[0] == 0 {
			value := remaining[:loc[1]]
			startLine := l.line
			startCol := l.column

			tType, typeName := l.resolveTokenType(p.Name, value, p.Alias)

			// Handle STRING tokens: strip quotes and optionally process escapes.
			// When EscapeMode is "none", we strip the quotes but leave escape
			// sequences as raw text. This is used by grammars like TOML and CSS
			// where different string types have different escape semantics — the
			// semantic layer handles escape processing instead of the lexer.
			if strings.Contains(p.Name, "STRING") || (p.Alias != "" && strings.Contains(p.Alias, "STRING")) {
				if len(value) >= 2 {
					quote := value[0]
					if quote == '"' || quote == '\'' {
						// Check for triple-quoted strings (""" or ''')
						if len(value) >= 6 && value[0] == value[1] && value[1] == value[2] {
							inner := value[3 : len(value)-3]
							if l.grammar.EscapeMode != "none" {
								inner = processEscapes(inner)
							}
							value = inner
						} else {
							inner := value[1 : len(value)-1]
							if l.grammar.EscapeMode != "none" {
								inner = processEscapes(inner)
							}
							value = inner
						}
					}
				}
			}

			tok := Token{Type: tType, Value: value, Line: startLine, Column: startCol, TypeName: typeName}

			for i := 0; i < loc[1]; i++ {
				l.advance()
			}

			return &tok
		}
	}
	return nil
}

// ---------------------------------------------------------------------------
// Escape Processing
// ---------------------------------------------------------------------------

// processEscapes handles escape sequences in string literals.
func processEscapes(s string) string {
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
func (l *GrammarLexer) Tokenize() []Token {
	if l.indentMode {
		return l.tokenizeIndentation()
	}
	return l.tokenizeStandard()
}

// ---------------------------------------------------------------------------
// Standard (Non-Indentation) Tokenization
// ---------------------------------------------------------------------------

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
//     f. If nothing matches, raise a LexerError (panic).
//  2. Append EOF.
//  3. Reset group stack and skip state for reuse.
func (l *GrammarLexer) tokenizeStandard() []Token {
	var tokens []Token

	for l.pos < len(l.source) {
		char := l.source[l.pos]

		// --- Skip patterns (grammar-defined) ---
		// When the grammar has skip patterns AND skip is enabled, they take
		// over whitespace handling. The callback can disable skip processing
		// for groups where whitespace is significant (e.g., CDATA).
		if l.hasSkipPatterns {
			if l.skipEnabled && l.trySkip() {
				continue
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
			tokens = append(tokens, Token{Type: TokenNewline, Value: "\\n", Line: l.line, Column: l.column, TypeName: "NEWLINE"})
			l.advance()
			continue
		}

		// --- Try active group's token patterns (first match wins) ---
		// The active group is the top of the group stack. When no groups
		// are defined, this is always "default" (the top-level definitions),
		// preserving backward compatibility.
		activeGroup := l.groupStack[len(l.groupStack)-1]
		tok := l.tryMatchTokenInGroup(activeGroup)
		if tok != nil {
			// --- Invoke on-token callback ---
			// The callback can push/pop groups, emit extra tokens, suppress
			// the current token, or toggle skip processing. Emitted tokens
			// do NOT re-trigger the callback.
			if l.onToken != nil {
				ctx := &LexerContext{
					lexer:    l,
					source:   l.source,
					posAfter: l.pos,
				}
				l.onToken(*tok, ctx)

				// Apply suppression: if the callback suppressed this token,
				// don't add it to the output.
				if !ctx.suppressed {
					tokens = append(tokens, *tok)
				}

				// Append any tokens emitted by the callback.
				tokens = append(tokens, ctx.emitted...)

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
			} else {
				tokens = append(tokens, *tok)
			}
			continue
		}

		panic(fmt.Sprintf("LexerError at %d:%d: Unexpected character %q", l.line, l.column, char))
	}

	tokens = append(tokens, Token{Type: TokenEOF, Value: "", Line: l.line, Column: l.column, TypeName: "EOF"})

	// Reset group stack and skip state for reuse. This ensures the lexer
	// can be called multiple times without group state leaking between
	// tokenize() calls.
	l.groupStack = []string{"default"}
	l.skipEnabled = true

	return tokens
}

// ---------------------------------------------------------------------------
// Indentation Mode Tokenization
// ---------------------------------------------------------------------------

func (l *GrammarLexer) tokenizeIndentation() []Token {
	var tokens []Token
	atLineStart := true

	for l.pos < len(l.source) {
		// Process line start
		if atLineStart && l.bracketDepth == 0 {
			indentTokens, skipLine := l.processLineStart()
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
		if l.trySkip() {
			continue
		}

		// Try token patterns
		tok := l.tryMatchToken()
		if tok != nil {
			// Track bracket depth
			switch tok.Value {
			case "(", "[", "{":
				l.bracketDepth++
			case ")", "]", "}":
				l.bracketDepth--
			}
			tokens = append(tokens, *tok)
			continue
		}

		panic(fmt.Sprintf("LexerError at %d:%d: Unexpected character %q", l.line, l.column, char))
	}

	// EOF: emit remaining DEDENTs
	for len(l.indentStack) > 1 {
		l.indentStack = l.indentStack[:len(l.indentStack)-1]
		tokens = append(tokens, Token{Type: TokenName, Value: "", Line: l.line, Column: l.column, TypeName: "DEDENT"})
	}

	// Final NEWLINE if needed
	if len(tokens) == 0 || tokens[len(tokens)-1].Type != TokenNewline {
		tokens = append(tokens, Token{Type: TokenNewline, Value: "\\n", Line: l.line, Column: l.column, TypeName: "NEWLINE"})
	}

	tokens = append(tokens, Token{Type: TokenEOF, Value: "", Line: l.line, Column: l.column, TypeName: "EOF"})
	return tokens
}

// processLineStart handles indentation at the start of a logical line.
// Returns (indentTokens, skipLine).
func (l *GrammarLexer) processLineStart() ([]Token, bool) {
	indent := 0
	for l.pos < len(l.source) {
		char := l.source[l.pos]
		if char == ' ' {
			indent++
			l.advance()
		} else if char == '\t' {
			panic(fmt.Sprintf("LexerError at %d:%d: Tab character in indentation (use spaces only)", l.line, l.column))
		} else {
			break
		}
	}

	// Blank line or EOF
	if l.pos >= len(l.source) {
		return nil, true
	}
	if l.source[l.pos] == '\n' {
		l.advance()
		return nil, true
	}

	// Comment-only line
	remaining := l.source[l.pos:]
	for _, pat := range l.skipPatterns {
		loc := pat.FindStringIndex(remaining)
		if loc != nil && loc[0] == 0 {
			peekPos := l.pos + loc[1]
			if peekPos >= len(l.source) || l.source[peekPos] == '\n' {
				for i := 0; i < loc[1]; i++ {
					l.advance()
				}
				if l.pos < len(l.source) && l.source[l.pos] == '\n' {
					l.advance()
				}
				return nil, true
			}
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
			panic(fmt.Sprintf("LexerError at %d:%d: Inconsistent dedent", l.line, l.column))
		}
	}

	return tokens, false
}
