package lexer

// Frozen copy of the pre-compilation lexer engine (test oracle)
// ==============================================================
//
// This file is a mechanical copy of the grammar-driven lexer as it was before
// Compile / first-byte dispatch / literal fast path were introduced: one regexp
// per rule, compiled with a bare "^" prefix, tried one after another at every
// position. The only edits are renames (oracleLexer, ...) and the removal of
// callbacks and hooks, which the compiled path does not offer either.
//
// It exists for one purpose: the differential tests in compiled_diff_test.go
// check that the optimised engine produces exactly the tokens this simple,
// obviously-correct interpreter does. Do not "improve" it. If the real engine
// changes behaviour on purpose, change this copy deliberately and say so.

import (
	"fmt"
	"regexp"
	"strings"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
)

// ---------------------------------------------------------------------------
// Compiled Pattern — A regex paired with its token name and optional alias
// ---------------------------------------------------------------------------

type oraclePattern struct {
	Name    string
	Pattern *regexp.Regexp
	Alias   string // Optional type alias
}

// ---------------------------------------------------------------------------
// The Grammar-Driven Lexer
// ---------------------------------------------------------------------------

// oracleLexer tokenizes source code using grammar-defined token patterns.
//
// Instead of hardcoded character-matching logic, this lexer:
//
//  1. Compiles each token definition's pattern into a regex
//  2. At each position, tries each regex in definition order (first match wins)
//  3. Emits a Token with the matched type and value
//
// Supports skip patterns, type aliases, reserved keywords, indentation mode,
// pattern groups with stackable transitions, and on-token callbacks.
type oracleLexer struct {
	source       string
	grammar      *grammartools.TokenGrammar
	pos          int
	line         int
	column       int
	keywordSet   map[string]struct{}
	reservedSet  map[string]struct{}
	patterns     []oraclePattern
	skipPatterns []*regexp.Regexp

	// hasSkipPatterns tracks whether the grammar defines skip patterns.
	// When true, skip patterns take over whitespace handling entirely.
	// When false, the lexer falls back to hardcoded space/tab/CR skipping.
	hasSkipPatterns bool

	// Indentation mode state
	indentMode   bool
	layoutMode   bool
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

	// lastEmittedToken is the most recently emitted token, for lookbehind
	// in callbacks. Updated after each token push (including callback-emitted
	// tokens). Reset to nil on each Tokenize() call.
	lastEmittedToken *Token

	// contextKeywordSet is the pre-computed set of context-sensitive keywords
	// for O(1) lookup. Words in this set are emitted as NAME tokens with
	// the TokenContextKeyword flag.
	contextKeywordSet map[string]struct{}
	layoutKeywordSet  map[string]struct{}

	// --- Pattern groups ---
	// groupPatterns maps group names to their compiled patterns. The
	// "default" group uses the top-level definitions. Named groups use
	// their own definitions from the grammar's "group:" sections.
	groupPatterns map[string][]oraclePattern

	// groupStack is the stack of active pattern groups. The bottom is
	// always "default". The top determines which patterns are tried
	// during token matching.
	groupStack []string

	// skipEnabled controls whether skip patterns are processed. The
	// on-token callback can toggle this for groups where whitespace is
	// significant (e.g., CDATA sections).
	skipEnabled bool

	// aliasMap maps token definition names to their aliases. Used during
	// group pattern compilation to register aliases from group definitions.
	aliasMap map[string]string

	// F10: declarative mode transitions (from the transitions: section).
	// Empty when the grammar has no transitions table (pre-F10 behaviour).
	transitions []grammartools.ModeTransition

	// F10: set of group names that inherit the default group's patterns.
	// A group inherits when it is a set_mode target but not a push target.
	// Pre-computed from transitions at construction time.
	inheritingModes map[string]bool

	// F10: the group the lexer starts in. Set from the grammar's start_mode:
	// directive; defaults to "default" when unset.
	startMode string

	// caseInsensitive is true when the grammar was loaded with
	// # @case_insensitive true. In this mode:
	//   - The keyword set stores uppercase keyword values.
	//   - Keyword lookup uses strings.ToUpper(value) before checking.
	//   - Emitted KEYWORD tokens have their value normalized to uppercase.
	// This allows SQL grammars to accept SELECT/select/Select etc. and
	// still match grammar literals like "SELECT".
	caseInsensitive bool

	// originalSource stores the raw, unmodified input string.
	//
	// In case-insensitive mode the working source (l.source) is lowercased
	// to make keyword pattern matching case-insensitive.  Without this field
	// STRING token values would also be lowercased — 'Alice' would become
	// 'alice'.  By keeping originalSource we can extract the body of every
	// STRING token from the original text at the same byte offset, preserving
	// whatever case the user wrote.
	originalSource string
}

// newOracleLexer creates a new grammar-driven lexer.
//
// It compiles all token definitions, skip patterns, and group patterns into
// regexes. The compiled patterns are anchored to the start of the remaining
// source (using ^) so that only matches at the current position are found.
func newOracleLexer(source string, grammar *grammartools.TokenGrammar) *oracleLexer {
	result, _ := StartNew[*oracleLexer]("lexer.newOracleLexer", nil,
		func(op *Operation[*oracleLexer], rf *ResultFactory[*oracleLexer]) *OperationResult[*oracleLexer] {
			return rf.Generate(true, false, newOracleLexerImpl(source, grammar))
		}).GetResult()
	return result
}

func newOracleLexerImpl(source string, grammar *grammartools.TokenGrammar) *oracleLexer {
	// Case-insensitive mode: lowercase the entire source before tokenization.
	// This ensures keyword matching works because keywords in the .tokens file
	// are already lowercase, and now the source text will be too.
	src := source
	if !grammar.CaseSensitive {
		src = strings.ToLower(source)
	}

	// Build keyword set. When case-insensitive mode is active, keywords
	// are stored as uppercase so lookup can use strings.ToUpper(value).
	keywordSet := make(map[string]struct{})
	for _, kw := range grammar.Keywords {
		if grammar.CaseInsensitive {
			keywordSet[strings.ToUpper(kw)] = struct{}{}
		} else {
			keywordSet[kw] = struct{}{}
		}
	}

	// Build reserved keyword set (same case treatment as keyword set).
	reservedSet := make(map[string]struct{})
	for _, rk := range grammar.ReservedKeywords {
		if grammar.CaseInsensitive {
			reservedSet[strings.ToUpper(rk)] = struct{}{}
		} else {
			reservedSet[rk] = struct{}{}
		}
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
	var patterns []oraclePattern
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
		patterns = append(patterns, oraclePattern{Name: defn.Name, Pattern: pat, Alias: defn.Alias})
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
	groupPatterns := map[string][]oraclePattern{
		"default": append([]oraclePattern{}, patterns...),
	}
	for groupName, group := range grammar.Groups {
		var compiled []oraclePattern
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
			compiled = append(compiled, oraclePattern{Name: defn.Name, Pattern: pat, Alias: defn.Alias})
			// Register aliases from group definitions.
			if defn.Alias != "" {
				aliasMap[defn.Name] = defn.Alias
			}
		}
		groupPatterns[groupName] = compiled
	}

	// Build context keyword set for O(1) lookup.
	contextKeywordSet := make(map[string]struct{})
	for _, ck := range grammar.ContextKeywords {
		contextKeywordSet[ck] = struct{}{}
	}
	layoutKeywordSet := make(map[string]struct{})
	for _, kw := range grammar.LayoutKeywords {
		layoutKeywordSet[kw] = struct{}{}
	}

	// F10: Compute flat-mode inheritance and resolve the start mode.
	// When start_mode: is unset in the grammar, startMode defaults to "default".
	startMode := grammar.StartMode
	if startMode == "" {
		startMode = "default"
	}
	inheritingModes := computeInheritingModes(grammar.Transitions)

	return &oracleLexer{
		source:            src,
		originalSource:    source,
		grammar:           grammar,
		pos:               0,
		line:              1,
		column:            1,
		keywordSet:        keywordSet,
		reservedSet:       reservedSet,
		patterns:          patterns,
		skipPatterns:      skipPatterns,
		hasSkipPatterns:   len(grammar.SkipDefinitions) > 0,
		indentMode:        grammar.Mode == "indentation",
		layoutMode:        grammar.Mode == "layout",
		indentStack:       []int{0},
		bracketDepth:      0,
		groupPatterns:     groupPatterns,
		groupStack:        []string{startMode},
		skipEnabled:       true,
		aliasMap:          aliasMap,
		caseInsensitive:   grammar.CaseInsensitive,
		contextKeywordSet: contextKeywordSet,
		layoutKeywordSet:  layoutKeywordSet,
		transitions:       grammar.Transitions,
		inheritingModes:   inheritingModes,
		startMode:         startMode,
	}
}

// BracketDepth returns the current nesting depth for a specific bracket type.
// Pass "paren" for (), "bracket" for [], "brace" for {}, or "" for the total
// depth across all three types.
func (l *oracleLexer) BracketDepth(kind string) int {
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
func (l *oracleLexer) updateBracketDepth(value string) {
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

func (l *oracleLexer) advance() {
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
func (l *oracleLexer) resolveTokenType(tokenName string, value string, alias string) (TokenType, string) {
	// Reserved keyword check. In case-insensitive mode, compare using the
	// uppercase form of the value (the reserved set stores uppercase keys).
	if tokenName == "NAME" {
		lookupValue := value
		if l.caseInsensitive {
			lookupValue = strings.ToUpper(value)
		}
		if _, ok := l.reservedSet[lookupValue]; ok {
			panic(fmt.Sprintf("LexerError at %d:%d: Reserved keyword %q cannot be used as an identifier", l.line, l.column, value))
		}
	}

	// Regular keyword check. In case-insensitive mode, the keyword set
	// stores uppercase values. We compare strings.ToUpper(value) against
	// the set and emit the keyword with its value normalized to uppercase.
	// This means grammar rules can use "SELECT" and it matches select/SELECT/Select.
	if tokenName == "NAME" {
		lookupValue := value
		if l.caseInsensitive {
			lookupValue = strings.ToUpper(value)
		}
		if _, ok := l.keywordSet[lookupValue]; ok {
			if l.caseInsensitive {
				// Normalize the emitted keyword value to uppercase so grammar
				// literals like "SELECT" match regardless of input case.
				return TokenKeyword, "KEYWORD"
			}
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
func (l *oracleLexer) trySkip() bool {
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
func (l *oracleLexer) tryMatchToken() *Token {
	return l.tryMatchTokenInGroup("default")
}

// tryMatchTokenInGroup attempts to match a token at the current position
// using the patterns from a specific group.
//
// It tries each compiled pattern in the named group in priority order (first
// match wins). Handles keyword detection, reserved word checking, aliases,
// and string escape processing.
func (l *oracleLexer) tryMatchTokenInGroup(groupName string) *Token {
	remaining := l.source[l.pos:]

	// Look up the group's patterns. Fall back to default if unknown.
	patterns, ok := l.groupPatterns[groupName]
	if !ok {
		patterns = l.patterns
	}

	// F10: flat-mode inheritance. Groups targeted by set_mode (but not push)
	// inherit the default group's patterns as a fallthrough. This means
	// group-specific patterns take priority, then the default patterns are
	// tried for tokens that the group does not override.
	if groupName != "default" && l.inheritingModes[groupName] {
		defaultPatterns := l.groupPatterns["default"]
		merged := make([]oraclePattern, 0, len(patterns)+len(defaultPatterns))
		merged = append(merged, patterns...)
		merged = append(merged, defaultPatterns...)
		patterns = merged
	}

	for _, p := range patterns {
		loc := p.Pattern.FindStringIndex(remaining)
		if loc != nil && loc[0] == 0 {
			value := remaining[:loc[1]]
			startLine := l.line
			startCol := l.column

			tType, typeName := l.resolveTokenType(p.Name, value, p.Alias)

			// In case-insensitive mode, normalize KEYWORD token values to
			// uppercase. This ensures grammar literals like "SELECT" match
			// regardless of how the user typed the keyword (select/SELECT/Select).
			if l.caseInsensitive && tType == TokenKeyword {
				value = strings.ToUpper(value)
			}

			// Handle STRING tokens: strip quotes and optionally process escapes.
			// When EscapeMode is "none", we strip the quotes but leave escape
			// sequences as raw text. This is used by grammars like TOML and CSS
			// where different string types have different escape semantics — the
			// semantic layer handles escape processing instead of the lexer.
			//
			// Case-preservation: in case-insensitive mode, l.source is the
			// lowercased working copy, so `value` is already lowercase.  For
			// STRING tokens we want to preserve the original case the user typed
			// (e.g. 'Alice' must not become 'alice').  We re-read the same byte
			// range from l.originalSource (which is never lowercased) and use
			// that for the string body instead of `value`.
			if strings.Contains(p.Name, "STRING") || (p.Alias != "" && strings.Contains(p.Alias, "STRING")) {
				tokenLen := loc[1]
				originalValue := value
				if l.caseInsensitive && tokenLen > 0 && l.pos+tokenLen <= len(l.originalSource) {
					originalValue = l.originalSource[l.pos : l.pos+tokenLen]
				}
				if len(originalValue) >= 2 {
					quote := originalValue[0]
					if quote == '"' || quote == '\'' {
						// Check for triple-quoted strings (""" or ''')
						if len(originalValue) >= 6 && originalValue[0] == originalValue[1] && originalValue[1] == originalValue[2] {
							inner := originalValue[3 : len(originalValue)-3]
							if l.grammar.EscapeMode != "none" {
								inner = oracleProcessEscapes(inner)
							}
							value = inner
						} else {
							inner := originalValue[1 : len(originalValue)-1]
							if l.grammar.EscapeMode != "none" {
								inner = oracleProcessEscapes(inner)
							}
							value = inner
						}
					}
				}
			}

			// Check if this NAME token is a context keyword — a word that
			// is sometimes a keyword and sometimes an identifier depending
			// on syntactic position. Context keywords are emitted as NAME
			// with the TokenContextKeyword flag, leaving the final decision
			// to the language-specific parser or callback.
			var flags int
			if typeName == "NAME" && len(l.contextKeywordSet) > 0 {
				if _, isCtx := l.contextKeywordSet[value]; isCtx {
					flags = TokenContextKeyword
				}
			}

			tok := Token{Type: tType, Value: value, Line: startLine, Column: startCol, TypeName: typeName, Flags: flags}

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

// oracleProcessEscapes handles escape sequences in string literals.
func oracleProcessEscapes(s string) string {
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
func (l *oracleLexer) Tokenize() []Token {
	result, _ := StartNew[[]Token]("lexer.oracleLexer.Tokenize", nil,
		func(op *Operation[[]Token], rf *ResultFactory[[]Token]) *OperationResult[[]Token] {
			// Reset extension state for reuse between Tokenize() calls.
			l.lastEmittedToken = nil
			l.bracketDepths.paren = 0
			l.bracketDepths.bracket = 0
			l.bracketDepths.brace = 0

			// Stage 2: Core tokenization.
			var tokens []Token
			if l.indentMode {
				tokens = l.tokenizeIndentation()
			} else if l.layoutMode {
				tokens = l.tokenizeLayout()
			} else {
				tokens = l.tokenizeStandard()
			}

			return rf.Generate(true, false, tokens)
		}).PanicOnUnexpected().GetResult()
	return result
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
func (l *oracleLexer) tokenizeStandard() []Token {
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
			newlineTok := Token{Type: TokenNewline, Value: "\\n", Line: l.line, Column: l.column, TypeName: "NEWLINE"}
			tokens = append(tokens, newlineTok)
			l.lastEmittedToken = &newlineTok
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
			// Update per-type bracket depth tracking.
			l.updateBracketDepth(tok.Value)

			tokens = append(tokens, *tok)
			l.lastEmittedToken = tok
			// F10: Apply declarative transitions.
			l.applyTransitions(*tok)
			continue
		}

		panic(fmt.Sprintf("LexerError at %d:%d: Unexpected character %q", l.line, l.column, char))
	}

	tokens = append(tokens, Token{Type: TokenEOF, Value: "", Line: l.line, Column: l.column, TypeName: "EOF"})

	// Reset group stack and skip state for reuse. This ensures the lexer
	// can be called multiple times without group state leaking between
	// tokenize() calls.
	l.groupStack = []string{l.startMode}
	l.skipEnabled = true

	return tokens
}

// ---------------------------------------------------------------------------
// Indentation Mode Tokenization
// ---------------------------------------------------------------------------

func (l *oracleLexer) tokenizeIndentation() []Token {
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
			// Track bracket depth (legacy single counter for indentation logic)
			switch tok.Value {
			case "(", "[", "{":
				l.bracketDepth++
			case ")", "]", "}":
				l.bracketDepth--
			}
			// Track per-type bracket depth (shared for callback access)
			l.updateBracketDepth(tok.Value)
			tokens = append(tokens, *tok)
			l.lastEmittedToken = tok
			// F10: Apply declarative transitions after each regular token.
			l.applyTransitions(*tok)
			continue
		}

		panic(fmt.Sprintf("LexerError at %d:%d: Unexpected character %q", l.line, l.column, char))
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
	return tokens
}

func (l *oracleLexer) tokenizeLayout() []Token {
	return l.applyLayout(l.tokenizeStandard())
}

func (l *oracleLexer) applyLayout(tokens []Token) []Token {
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

func (l *oracleLexer) nextLayoutToken(tokens []Token, startIndex int) *Token {
	for idx := startIndex; idx < len(tokens); idx++ {
		if tokens[idx].TypeName != "NEWLINE" {
			return &tokens[idx]
		}
	}
	return nil
}

func (l *oracleLexer) virtualLayoutToken(typeName, value string, anchor Token) Token {
	return Token{
		Type:     TokenName,
		Value:    value,
		Line:     anchor.Line,
		Column:   anchor.Column,
		TypeName: typeName,
	}
}

func (l *oracleLexer) isLayoutKeyword(token Token) bool {
	if len(l.layoutKeywordSet) == 0 {
		return false
	}
	if _, ok := l.layoutKeywordSet[token.Value]; ok {
		return true
	}
	_, ok := l.layoutKeywordSet[strings.ToLower(token.Value)]
	return ok
}

// processLineStart handles indentation at the start of a logical line.
// Returns (indentTokens, skipLine).
func (l *oracleLexer) processLineStart() ([]Token, bool) {
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

func (l *oracleLexer) applyTransitions(tok Token) {
	if len(l.transitions) == 0 {
		return
	}

	// The active group is the top of the group stack.
	active := l.groupStack[len(l.groupStack)-1]

	for _, rule := range l.transitions {
		// 1. Token-type guard: does the rule trigger on this token's type?
		typeMatched := false
		for _, onType := range rule.OnTokens {
			if onType == tok.TypeName {
				typeMatched = true
				break
			}
		}
		if !typeMatched {
			continue
		}

		// 2. In-mode guard: rule must be for the current active group
		//    (or have no in-mode constraint).
		if rule.InMode != "" && rule.InMode != active {
			continue
		}

		// 3. Value guard: rule must match the token's value
		//    (or have no value constraint).
		if rule.OnValue != "" && rule.OnValue != tok.Value {
			continue
		}

		// First matching rule fires: apply all actions in order.
		for _, action := range rule.Actions {
			switch action.Kind {
			case "set_mode":
				// Flat toggle: replace the active group in place.
				l.groupStack[len(l.groupStack)-1] = action.Target

			case "push":
				// Nested region: push a new exclusive group onto the stack.
				l.groupStack = append(l.groupStack, action.Target)

			case "pop":
				// Close a nested region. Clamp to keep at least one entry
				// so the stack never becomes completely empty.
				if len(l.groupStack) > 1 {
					l.groupStack = l.groupStack[:len(l.groupStack)-1]
				}

			case "enable_skip":
				l.skipEnabled = true

			case "disable_skip":
				l.skipEnabled = false
			}
		}

		// First-match-wins: stop after the first matching rule.
		break
	}
}
