package lexer

// Compiled Lexer — compile a grammar once, tokenize many times
// =============================================================
//
// The interpreter in grammar_lexer.go used to rebuild every regular expression
// each time a GrammarLexer was constructed, and then, for every token, tried
// every rule in turn with a full regexp execution.  Both costs are avoidable:
//
//   - the compiled form of a grammar never changes, so it can be built once
//     and shared (Compile / *Compiled);
//   - at any position in the input, only a handful of rules can possibly
//     match, and the very first byte tells us which ones (first-byte dispatch);
//   - most rules in a real grammar are plain literals ("(", "==", "return"),
//     which need a prefix comparison, not a regular-expression engine
//     (literal fast path).
//
// Nothing here changes *what* the lexer accepts.  Rules are still tried in the
// order the grammar lists them and the first one that matches wins.  Dispatch
// only ever skips a rule when it is certain that rule cannot match at the
// current byte.
//
// Errors, not panics
// ------------------
//
// The legacy entry points panic on bad input, because they predate error
// returns.  Compiled.Tokenize is the first-class API for callers that cannot
// afford a panic (a security-sensitive trace parser, a server).  It returns a
// *LexError carrying the line and column instead, and the hot path contains no
// recover(): the code simply has no panics to catch.
//
// The old API is now a thin layer on the same engine: GrammarLexer.Tokenize
// calls the error-returning core and re-panics with the error text, which is
// exactly the message it always produced.

import (
	"fmt"
	"regexp"
	"regexp/syntax"
	"sort"
	"strings"
	"unicode"
	"unicode/utf8"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
)

// ---------------------------------------------------------------------------
// LexError
// ---------------------------------------------------------------------------

// LexError is the error returned by Compiled.Tokenize when the source cannot
// be tokenized: an unexpected character, a reserved word used as an
// identifier, bad indentation, or a rule that matched the empty string.
//
// Line and Column are 1-based; Column counts bytes, exactly like Token.Column.
// Error() produces the same text the legacy panic carried, so
// `LexerError at 1:5: Unexpected character '§'`.
type LexError struct {
	Line    int
	Column  int
	Message string
}

// Error implements the error interface.
func (e *LexError) Error() string {
	return fmt.Sprintf("LexerError at %d:%d: %s", e.Line, e.Column, e.Message)
}

// ---------------------------------------------------------------------------
// Rules
// ---------------------------------------------------------------------------

// rule is one compiled token (or skip) definition.
//
// Exactly one of literal / re is the matcher.  A non-empty literal means the
// rule is a plain string and is matched with strings.HasPrefix.
type rule struct {
	name  string
	alias string

	// literal is the text of a literal rule ("" for regex rules).
	literal string

	// re is the regex matcher for non-literal rules. It is compiled as
	// `^(?:pattern)` so the engine knows the match is pinned to the current
	// position and never scans ahead.
	re *regexp.Regexp

	// m, when non-nil, is a straight-line matcher proven equivalent to re (see
	// matcher.go). It is used instead of re on the hot path.
	m *matcher

	// legacyAnchor is true only for the rare pattern whose `^(?:...)` wrapped
	// form fails to compile although the legacy `^pattern` form compiles (for
	// instance a pattern containing an unterminated \Q).  Such a rule keeps the
	// legacy shape, which needs an explicit "match starts at 0" check.
	legacyAnchor bool

	// first is the set of bytes a match can start with; nil means "any".
	first *byteSet

	// Resolution that does not depend on the matched text, computed once.
	tokenType TokenType
	typeName  string

	// isName: the rule is literally called NAME, so each match must be checked
	// against the keyword and reserved-word sets.
	isName bool

	// stringLike: the rule (or its alias) has "STRING" in its name, so the
	// surrounding quotes are stripped from the value. See stripQuotes.
	stringLike bool
}

// matchLen reports how many bytes of s (which starts at the current position)
// this rule matches, or -1 if it does not match there. A result of 0 means the
// rule matched the empty string, which the engine reports as an error.
func (r *rule) matchLen(s string) int {
	if r.literal != "" {
		if strings.HasPrefix(s, r.literal) {
			return len(r.literal)
		}
		return -1
	}
	if r.m != nil {
		return r.m.match(s)
	}
	loc := r.re.FindStringIndex(s)
	if loc == nil {
		return -1
	}
	if r.legacyAnchor && loc[0] != 0 {
		return -1
	}
	return loc[1]
}

// ruleTable is an ordered rule list pre-indexed by first byte.
//
// byByte[b] holds, in grammar order, exactly the rules that might match when
// the next input byte is b. A rule appears under every byte it could start
// with; a catch-all rule such as /./ appears under all 256.
type ruleTable struct {
	byByte [256][]*rule
}

// ---------------------------------------------------------------------------
// Compiled
// ---------------------------------------------------------------------------

// Compiled is a grammar compiled for repeated use. It is immutable after
// Compile returns and is safe for concurrent use by multiple goroutines.
type Compiled struct {
	grammar *grammartools.TokenGrammar

	// lowerSource: the legacy engine lowercases the source when the grammar's
	// CaseSensitive field is false. This is preserved bit-for-bit (a
	// TokenGrammar built by hand and not through ParseTokenGrammar has the
	// zero value false).
	lowerSource     bool
	caseInsensitive bool
	escapeNone      bool

	keywordSet        map[string]struct{}
	reservedSet       map[string]struct{}
	contextKeywordSet map[string]struct{}
	layoutKeywordSet  map[string]struct{}

	// tables maps a group name to its dispatch table. "default" always exists.
	// Groups that inherit the default patterns (F10 set_mode targets) already
	// have the default rules appended to their own.
	tables map[string]*ruleTable

	skip    *ruleTable
	hasSkip bool

	indentMode  bool
	layoutMode  bool
	startMode   string
	transitions []grammartools.ModeTransition
	inheriting  map[string]bool
}

// Compile compiles every pattern in tg exactly once.
//
// It returns an error (never panics) if any pattern is not a valid regular
// expression. The error text names the offending token, e.g.
// "Failed to compile pattern for token NAME: error parsing regexp: ...".
func Compile(tg *grammartools.TokenGrammar) (*Compiled, error) {
	if tg == nil {
		return nil, fmt.Errorf("lexer.Compile: nil grammar")
	}

	c := &Compiled{
		grammar:           tg,
		lowerSource:       !tg.CaseSensitive,
		caseInsensitive:   tg.CaseInsensitive,
		escapeNone:        tg.EscapeMode == "none",
		keywordSet:        make(map[string]struct{}, len(tg.Keywords)),
		reservedSet:       make(map[string]struct{}, len(tg.ReservedKeywords)),
		contextKeywordSet: make(map[string]struct{}, len(tg.ContextKeywords)),
		layoutKeywordSet:  make(map[string]struct{}, len(tg.LayoutKeywords)),
		hasSkip:           len(tg.SkipDefinitions) > 0,
		indentMode:        tg.Mode == "indentation",
		layoutMode:        tg.Mode == "layout",
		transitions:       tg.Transitions,
		inheriting:        computeInheritingModes(tg.Transitions),
	}

	// Keyword sets. In case-insensitive mode they hold upper-case words and
	// matched text is upper-cased before the lookup.
	for _, kw := range tg.Keywords {
		c.keywordSet[c.foldKeyword(kw)] = struct{}{}
	}
	for _, rk := range tg.ReservedKeywords {
		c.reservedSet[c.foldKeyword(rk)] = struct{}{}
	}
	for _, ck := range tg.ContextKeywords {
		c.contextKeywordSet[ck] = struct{}{}
	}
	for _, lk := range tg.LayoutKeywords {
		c.layoutKeywordSet[lk] = struct{}{}
	}

	c.startMode = tg.StartMode
	if c.startMode == "" {
		c.startMode = "default"
	}

	// Top-level rules form the "default" group.
	defaultRules := make([]*rule, 0, len(tg.Definitions))
	for _, defn := range tg.Definitions {
		r, err := compileRule(defn, "Failed to compile pattern for token "+defn.Name)
		if err != nil {
			return nil, err
		}
		defaultRules = append(defaultRules, r)
	}

	skipRules := make([]*rule, 0, len(tg.SkipDefinitions))
	for _, defn := range tg.SkipDefinitions {
		r, err := compileRule(defn, "Failed to compile skip pattern "+defn.Name)
		if err != nil {
			return nil, err
		}
		skipRules = append(skipRules, r)
	}

	// Named groups. Sorted so that, if several are broken, the reported error
	// does not depend on map iteration order.
	groupRules := map[string][]*rule{"default": defaultRules}
	names := make([]string, 0, len(tg.Groups))
	for name := range tg.Groups {
		names = append(names, name)
	}
	sort.Strings(names)
	for _, name := range names {
		group := tg.Groups[name]
		rules := make([]*rule, 0, len(group.Definitions))
		for _, defn := range group.Definitions {
			r, err := compileRule(defn, fmt.Sprintf("Failed to compile group pattern %s/%s", name, defn.Name))
			if err != nil {
				return nil, err
			}
			rules = append(rules, r)
		}
		groupRules[name] = rules
	}

	// Build dispatch tables. An inheriting group (F10 flat mode) tries its own
	// rules first and then the default rules; the legacy engine rebuilt that
	// merged list for every token, here it is built once.
	c.tables = make(map[string]*ruleTable, len(groupRules))
	for name, rules := range groupRules {
		if name != "default" && c.inheriting[name] {
			merged := make([]*rule, 0, len(rules)+len(defaultRules))
			merged = append(merged, rules...)
			merged = append(merged, defaultRules...)
			rules = merged
		}
		c.tables[name] = newRuleTable(rules)
	}
	c.skip = newRuleTable(skipRules)
	return c, nil
}

// foldKeyword applies the case-insensitive normalisation to a keyword.
func (c *Compiled) foldKeyword(kw string) string {
	if c.caseInsensitive {
		return strings.ToUpper(kw)
	}
	return kw
}

// table returns the dispatch table for a group. An unknown group falls back to
// the default group, as the legacy lexer did.
func (c *Compiled) table(group string) *ruleTable {
	if t, ok := c.tables[group]; ok {
		return t
	}
	return c.tables["default"]
}

// hasGroup reports whether the grammar defines the named group.
func (c *Compiled) hasGroup(name string) bool {
	_, ok := c.tables[name]
	return ok
}

// compileRule compiles one definition. errPrefix names the definition for
// error messages; it is the text the legacy panic used.
func compileRule(defn grammartools.TokenDefinition, errPrefix string) (*rule, error) {
	r := &rule{
		name:  defn.Name,
		alias: defn.Alias,
	}
	effective := defn.Name
	if defn.Alias != "" {
		effective = defn.Alias
	}
	r.tokenType, r.typeName = baseTokenType(effective)
	r.isName = defn.Name == "NAME"
	r.stringLike = strings.Contains(defn.Name, "STRING") ||
		(defn.Alias != "" && strings.Contains(defn.Alias, "STRING"))

	var first *byteSet

	switch {
	case !defn.IsRegex && isPlainLiteral(defn.Pattern):
		// Literal fast path: no regexp at all.
		r.literal = defn.Pattern
		first = &byteSet{}
		first[defn.Pattern[0]] = true
	default:
		pattern := defn.Pattern
		if !defn.IsRegex {
			pattern = regexp.QuoteMeta(pattern)
		}

		// Validate in the legacy shape first: the pattern must be a well-formed
		// regexp when written after a bare ^. This keeps both the accept set
		// and the error text identical to the legacy engine, and stops a
		// malformed pattern such as `)*x|y(` from escaping the (?:...) wrapper
		// used below and silently becoming a different, valid expression.
		if _, perr := syntax.Parse("^"+pattern, syntax.Perl); perr != nil {
			return nil, fmt.Errorf("%s: %v", errPrefix, perr)
		}

		wrapped := "^(?:" + pattern + ")"
		re, err := regexp.Compile(wrapped)
		if err == nil {
			r.re = re
			first = firstByteSet(wrapped)
			if m := buildMatcher(pattern); m != nil {
				if len(m.steps) == 1 && m.steps[0].kind == stepLit {
					// A regex that is really a plain string (e.g. /<unfinished>/).
					r.literal = m.steps[0].lit
				} else {
					r.m = m
				}
			}
		} else {
			// Report the error in the shape the legacy code produced (the
			// pattern anchored with a bare ^). If, unusually, that form
			// compiles, use it.
			legacy, lerr := regexp.Compile("^" + pattern)
			if lerr != nil {
				return nil, fmt.Errorf("%s: %v", errPrefix, lerr)
			}
			r.re = legacy
			r.legacyAnchor = true
			first = nil // any
		}
	}
	r.first = first
	return r, nil
}

// isPlainLiteral reports whether a literal token text can be matched with a
// byte-prefix comparison. The regexp engine reads invalid UTF-8 as U+FFFD, so a
// text containing U+FFFD (or invalid UTF-8, which the engine rejects outright)
// is left to the regexp path to keep behaviour identical.
func isPlainLiteral(text string) bool {
	return text != "" && utf8.ValidString(text) && !strings.ContainsRune(text, utf8.RuneError)
}

// newRuleTable indexes rules by first byte, preserving their order.
func newRuleTable(rules []*rule) *ruleTable {
	t := &ruleTable{}
	for _, r := range rules {
		for b := 0; b < 256; b++ {
			if r.first == nil || r.first[b] {
				t.byByte[b] = append(t.byByte[b], r)
			}
		}
	}
	return t
}

// baseTokenType maps a grammar token name (or alias) to the built-in
// TokenType and the TypeName the parser sees. Names that are not built in
// become TokenName with the grammar's own name as TypeName.
func baseTokenType(effectiveName string) (TokenType, string) {
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
		return TokenName, effectiveName
	}
}

// Tokenize tokenizes source with the compiled grammar.
//
// For any input that the legacy NewGrammarLexer(source, tg).Tokenize() accepts
// the result is identical: same token types, values, lines, columns and flags.
// Where the legacy path panics, Tokenize returns a *LexError instead. It never
// panics and may be called concurrently.
//
// Pre/post-tokenize hooks and on-token callbacks belong to GrammarLexer;
// declarative F10 transitions are part of the grammar and are applied here.
func (c *Compiled) Tokenize(source string) ([]Token, error) {
	l := c.newLexer(source)
	return l.tokenizeAll()
}

// ---------------------------------------------------------------------------
// First-byte analysis
// ---------------------------------------------------------------------------

// byteSet is a set of byte values.
type byteSet [256]bool

func (s *byteSet) addAll() {
	for i := range s {
		s[i] = true
	}
}

func (s *byteSet) union(o *byteSet) {
	for i, v := range o {
		if v {
			s[i] = true
		}
	}
}

func (s *byteSet) addHighBytes() {
	for i := 0x80; i < 256; i++ {
		s[i] = true
	}
}

// addRune records the first byte of r's UTF-8 encoding.
//
// The regexp engine treats each byte of invalid UTF-8 as the replacement
// character U+FFFD, so a pattern that can match U+FFFD can start with any byte
// of 0x80 or above.
func (s *byteSet) addRune(r rune) {
	switch {
	case r < utf8.RuneSelf && r >= 0:
		s[byte(r)] = true
	case r == utf8.RuneError || r < 0 || r > unicode.MaxRune:
		s.addHighBytes()
	default:
		var buf [utf8.UTFMax]byte
		utf8.EncodeRune(buf[:], r)
		s[buf[0]] = true
	}
}

// firstByteSet returns the set of bytes that a match of pattern can start
// with, or nil, meaning "any byte" (the safe answer, used whenever the
// analysis cannot be sure).
//
// The analysis is the textbook FIRST-set computation on the parsed regexp:
//
//	FIRST(literal "ab")   = {a}
//	FIRST(x y)            = FIRST(x), plus FIRST(y) if x can match empty
//	FIRST(x | y)          = FIRST(x) + FIRST(y)
//	FIRST(x*), FIRST(x?)  = FIRST(x), and the whole thing can match empty
//
// A pattern that can match the empty string can "start with" anything, so it
// yields nil.
func firstByteSet(pattern string) *byteSet {
	re, err := syntax.Parse(pattern, syntax.Perl)
	if err != nil {
		return nil
	}
	set, nullable := firstOf(re)
	if nullable {
		return nil
	}
	return set
}

// firstOf returns the FIRST set of re and whether re can match the empty
// string. The returned set is always a conservative superset.
func firstOf(re *syntax.Regexp) (*byteSet, bool) {
	set := &byteSet{}
	switch re.Op {
	case syntax.OpNoMatch:
		return set, false

	case syntax.OpEmptyMatch,
		syntax.OpBeginLine, syntax.OpEndLine,
		syntax.OpBeginText, syntax.OpEndText,
		syntax.OpWordBoundary, syntax.OpNoWordBoundary:
		// Zero-width: consumes nothing, so it contributes no first byte.
		return set, true

	case syntax.OpLiteral:
		if len(re.Rune) == 0 {
			return set, true
		}
		set.addRune(re.Rune[0])
		if re.Flags&syntax.FoldCase != 0 {
			// Case folding: also every rune in the fold orbit.
			for f := unicode.SimpleFold(re.Rune[0]); f != re.Rune[0]; f = unicode.SimpleFold(f) {
				set.addRune(f)
			}
		}
		return set, false

	case syntax.OpCharClass:
		// Rune is a flat list of inclusive [lo, hi] pairs.
		for i := 0; i+1 < len(re.Rune); i += 2 {
			lo, hi := re.Rune[i], re.Rune[i+1]
			for r := lo; r <= hi && r < utf8.RuneSelf; r++ {
				set[byte(r)] = true
			}
			if hi >= utf8.RuneSelf {
				// Any non-ASCII member: every lead byte (and every stray
				// byte of invalid UTF-8) is a candidate. Conservative.
				set.addHighBytes()
			}
		}
		return set, false

	case syntax.OpAnyCharNotNL:
		set.addAll()
		set['\n'] = false
		return set, false

	case syntax.OpAnyChar:
		set.addAll()
		return set, false

	case syntax.OpCapture:
		return firstOf(re.Sub[0])

	case syntax.OpStar, syntax.OpQuest:
		sub, _ := firstOf(re.Sub[0])
		return sub, true

	case syntax.OpPlus:
		return firstOf(re.Sub[0])

	case syntax.OpRepeat:
		sub, subNullable := firstOf(re.Sub[0])
		return sub, subNullable || re.Min == 0

	case syntax.OpConcat:
		nullable := true
		for _, sub := range re.Sub {
			s, n := firstOf(sub)
			set.union(s)
			if !n {
				nullable = false
				break
			}
		}
		return set, nullable

	case syntax.OpAlternate:
		nullable := false
		for _, sub := range re.Sub {
			s, n := firstOf(sub)
			set.union(s)
			nullable = nullable || n
		}
		return set, nullable
	}

	// Unknown operator (a future Go version): say "anything, maybe empty".
	set.addAll()
	return set, true
}
