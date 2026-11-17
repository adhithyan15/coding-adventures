package parser

// ===========================================================================
// THE PACKRAT ENGINE: compiled grammars, bounded recursion, cheap memoization
// ===========================================================================
//
// This file holds the engine behind GrammarParser. It is organised around
// three ideas, each of which fixes a concrete problem with the original
// "interpret the grammar tree on every parse" design.
//
// 1. BOUNDED RECURSION (security)
//
//    A PEG parser is recursive: to parse `[[[[1]]]]` the rule `value` calls
//    `array`, which calls `value`, which calls `array`, ... once per bracket.
//    Go grows goroutine stacks on demand, but only up to a hard cap (1 GB on
//    64-bit). Past it the runtime does not panic: it prints
//    `fatal error: stack overflow` and kills the WHOLE PROCESS. `recover()`
//    cannot catch it. So a few hundred thousand opening brackets in
//    attacker-controlled input is a one-packet denial of service against any
//    program that parses untrusted text with this package.
//
//    The cure is to count rule-recursion depth ourselves and refuse to go
//    past Options.MaxDepth (DefaultMaxDepth when unset), returning
//    ErrMaxDepth. The counter, not the runtime, is what stops the descent, so
//    the process never gets near its stack limit.
//
//    One subtlety (the same trap documented for the Rust GrammarParser): the
//    refusal happens deep inside the recursion, where an enclosing optional or
//    repetition will happily "backtrack past" the failed rule and succeed with
//    LESS input. If we only reported the depth error when the top-level rule
//    failed, hostile input would surface as an unrelated syntax error, or,
//    worse, as a successful parse of a truncated prefix. So the refusal is
//    STICKY: it sets a flag, every in-flight matcher fails immediately once
//    the flag is up, and Parse checks the flag before looking at anything
//    else. Depth exceeded always means error. It is never silently swallowed.
//
// 2. COMPILE ONCE, PARSE MANY (reuse)
//
//    CompileGrammar turns a grammartools.ParserGrammar into an immutable
//    Compiled: rule names resolved to integer ids, literals pre-quoted,
//    token names pre-mapped to token types. A Compiled can be shared by any
//    number of goroutines and parse any number of inputs.
//
// 3. CHEAP MEMOIZATION (speed)
//
//    Packrat parsing caches "rule R at token position P gave result X" so no
//    (rule, position) pair is ever parsed twice. The original cache was a
//    map[[2]int]*memoEntry: a hash lookup plus a heap allocation for every
//    entry, plus another allocation on every left-recursion growth step.
//
//    This engine keeps the cache in two flat slices:
//
//        head[pos]  -> index of the newest slot memoised at token position pos
//        slots[i]   -> {rule, result, endPos, ok, next}
//
//    Slots at one position form a short linked list (a position typically
//    sees a handful of rules: the precedence chain of an expression grammar,
//    say), so a lookup is a few integer comparisons with no hashing and no
//    allocation beyond amortised slice growth. Memory is proportional to the
//    entries actually created, NOT to tokens x rules, so a 500-rule grammar
//    over a million tokens does not allocate a dense table. The slices are
//    pooled and reused between parses of the same Compiled.
//
// NO ERROR RECOVERY (a deliberate, load-bearing property)
//
//    This parser has none, and must keep having none. A parse either returns
//    a complete AST for the whole input or an error; it never skips tokens,
//    never inserts synthetic ones, never returns a partial tree alongside an
//    error. Downstream consumers (config loaders, the build system, anything
//    gating a decision on "did this parse?") rely on that to fail closed.
//    Backtracking inside PEG alternatives is not recovery: it only chooses
//    between grammar-legal readings of the SAME input.

import (
	"errors"
	"fmt"
	"os"
	"strings"
	"sync"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// DefaultMaxDepth is the rule-recursion limit used when Options.MaxDepth is
// zero (which includes every pre-existing caller).
//
// Why 10,000:
//
//   - Far below the crash point. One level of rule recursion costs a few
//     Go stack frames (parseRule plus a handful of nested match frames).
//     Measured on arm64 with go1.26: a 10,000-deep parse of a bracket grammar
//     and of a six-rule expression grammar left a 16 MB stack in use (Go
//     doubles stacks, so the true use is between 8 and 16 MB, about 0.8 to
//     1.6 KB per level). Go's cap is 1 GB on 64-bit and 250 MB on 32-bit, so
//     the default sits 60x or more below the 64-bit crash point. (Go's
//     initial stack is tiny; growing to 16 MB costs a few copies and is fine.)
//
//   - Far above any real input. Depth counts RULES, not tokens. A grammar
//     with a 20-rule precedence chain spends about 20 levels per nested
//     parenthesis, so 10,000 levels admits roughly 500 nested parentheses
//     in the deepest expression grammars here, and about 5,000 nested JSON
//     arrays (two rules per level). Hand-written code and data essentially
//     never exceed a few dozen levels of nesting.
//
// Callers who legitimately need more can raise Options.MaxDepth, up to
// HardMaxDepth.
const DefaultMaxDepth = 10000

// HardMaxDepth is the ceiling Options.MaxDepth is clamped to. It exists so
// that "never overflow the stack" holds even when a caller passes a huge
// limit (say math.MaxInt) believing it means "unlimited". 50,000 levels is
// about 64 MB of stack at the measured frame sizes (a 100,000-deep parse
// pushed the stack to 128 to 256 MB, which is why the ceiling is half that):
// well under the 1 GB 64-bit cap. There is deliberately no way to disable
// the limit. 32-bit platforms have a 250 MB cap and smaller frames, so they
// fit too, but were not measured.
const HardMaxDepth = 50000

// ErrMaxDepth is the cause (see errors.Is) of the *GrammarParseError returned
// when input nests deeper than the configured limit.
var ErrMaxDepth = errors.New("input nests deeper than the supported limit")

// ErrUnknownStartRule is the cause of the error returned when
// Options.StartRule names a rule the grammar does not define.
var ErrUnknownStartRule = errors.New("unknown start rule")

// Options configures a parse. The zero value reproduces the historical
// behaviour of NewGrammarParser, plus the default depth limit.
type Options struct {
	// MaxDepth bounds rule recursion. Zero (or negative) means
	// DefaultMaxDepth; values above HardMaxDepth are clamped to it.
	MaxDepth int

	// StartRule names the rule to parse from. Empty means the first rule of
	// the grammar. An unknown name is an error (ErrUnknownStartRule).
	StartRule string

	// Trace prints each rule attempt to stderr (see NewGrammarParserWithTrace).
	Trace bool

	// PreParse hooks transform the token list before parsing, left to right.
	// (Compiled.Parse only; GrammarParser keeps its own AddPreParse.)
	PreParse []PreParseHook

	// PostParse hooks transform the finished AST, left to right.
	PostParse []PostParseHook
}

func (o Options) maxDepth() int {
	switch {
	case o.MaxDepth <= 0:
		return DefaultMaxDepth
	case o.MaxDepth > HardMaxDepth:
		return HardMaxDepth
	}
	return o.MaxDepth
}

// ---------------------------------------------------------------------------
// Compiled grammar
// ---------------------------------------------------------------------------

type nodeKind uint8

const (
	kFail nodeKind = iota // unknown element type: never matches (as before)
	kSeq
	kAlt
	kRep
	kOpt
	kGroup
	kRule
	kTok
	kLit
	kPosLook
	kNegLook
	kOneOrMore
	kSepRep
)

// node is one element of the compiled grammar. It is a flat union: each kind
// uses only the fields it needs.
type node struct {
	kind       nodeKind
	subs       []*node // kSeq, kAlt
	sub        *node   // kRep, kOpt, kGroup, kPosLook, kNegLook, kOneOrMore, kSepRep
	sep        *node   // kSepRep
	atLeastOne bool

	rule int32 // kRule: rule id, or -1 if the name is undefined

	name    string          // kTok: token name; kRule: rule name
	tokType lexer.TokenType // kTok: legacy enum fallback
	skipNL  bool            // kTok, kLit: skip insignificant newlines first
	value   string          // kLit: the literal text
	quoted  string          // kLit: %q form, for failure messages
}

type compiledRule struct {
	name string
	body *node
}

// Compiled is an immutable, goroutine-safe form of a parser grammar. Build it
// once with CompileGrammar and call Parse for each input.
type Compiled struct {
	rules               []compiledRule
	byName              map[string]int32
	newlinesSignificant bool
	pool                sync.Pool // of *parseState
}

// CompileGrammar resolves rule names to ids and pre-computes everything the
// hot path would otherwise recompute per token. The grammar is not retained
// or modified; later changes to it do not affect the result.
//
// Duplicate rule names behave as before: the last definition wins, for the
// start rule and for references alike.
func CompileGrammar(grammar *grammartools.ParserGrammar) (*Compiled, error) {
	if grammar == nil {
		return nil, errors.New("parser: nil grammar")
	}
	c := &Compiled{byName: make(map[string]int32, len(grammar.Rules))}

	// Newline significance is a property of the whole grammar and decides
	// how every literal and token reference compiles, so find it first.
	for _, r := range grammar.Rules {
		if elementReferencesNewline(r.Body) {
			c.newlinesSignificant = true
			break
		}
	}

	// Assign ids. A repeated name reuses its id and the later body replaces
	// the earlier one (matching the old map-overwrite behaviour).
	names := make([]string, 0, len(grammar.Rules))
	bodies := make([]grammartools.GrammarElement, 0, len(grammar.Rules))
	for _, r := range grammar.Rules {
		if id, dup := c.byName[r.Name]; dup {
			bodies[id] = r.Body
			continue
		}
		c.byName[r.Name] = int32(len(names))
		names = append(names, r.Name)
		bodies = append(bodies, r.Body)
	}
	c.rules = make([]compiledRule, len(names))
	for i := range names {
		c.rules[i] = compiledRule{name: names[i], body: c.compileElement(bodies[i])}
	}

	// The start rule is the first rule *as named*, resolved after dedup.
	c.pool.New = func() any { return &parseState{c: c} }
	return c, nil
}

func (c *Compiled) compileElement(el grammartools.GrammarElement) *node {
	switch e := el.(type) {
	case grammartools.Sequence:
		n := &node{kind: kSeq, subs: make([]*node, len(e.Elements))}
		for i, s := range e.Elements {
			n.subs[i] = c.compileElement(s)
		}
		return n
	case grammartools.Alternation:
		n := &node{kind: kAlt, subs: make([]*node, len(e.Choices))}
		for i, s := range e.Choices {
			n.subs[i] = c.compileElement(s)
		}
		return n
	case grammartools.Repetition:
		return &node{kind: kRep, sub: c.compileElement(e.Element)}
	case grammartools.Optional:
		return &node{kind: kOpt, sub: c.compileElement(e.Element)}
	case grammartools.Group:
		return &node{kind: kGroup, sub: c.compileElement(e.Element)}
	case grammartools.PositiveLookahead:
		return &node{kind: kPosLook, sub: c.compileElement(e.Element)}
	case grammartools.NegativeLookahead:
		return &node{kind: kNegLook, sub: c.compileElement(e.Element)}
	case grammartools.OneOrMoreRepetition:
		return &node{kind: kOneOrMore, sub: c.compileElement(e.Element)}
	case grammartools.SeparatedRepetition:
		return &node{kind: kSepRep, sub: c.compileElement(e.Element),
			sep: c.compileElement(e.Separator), atLeastOne: e.AtLeastOne}
	case grammartools.RuleReference:
		if e.IsToken {
			return &node{kind: kTok, name: e.Name, tokType: stringToTokenType(e.Name),
				skipNL: !c.newlinesSignificant && e.Name != "NEWLINE"}
		}
		id, ok := c.byName[e.Name]
		if !ok {
			id = -1
		}
		return &node{kind: kRule, rule: id, name: e.Name}
	case grammartools.Literal:
		return &node{kind: kLit, value: e.Value, quoted: fmt.Sprintf("%q", e.Value),
			skipNL: !c.newlinesSignificant}
	}
	return &node{kind: kFail}
}

// elementReferencesNewline reports whether a NEWLINE token is mentioned
// anywhere in the element. Such grammars treat newlines as significant.
func elementReferencesNewline(element grammartools.GrammarElement) bool {
	switch e := element.(type) {
	case grammartools.RuleReference:
		return e.IsToken && e.Name == "NEWLINE"
	case grammartools.Sequence:
		for _, sub := range e.Elements {
			if elementReferencesNewline(sub) {
				return true
			}
		}
	case grammartools.Alternation:
		for _, choice := range e.Choices {
			if elementReferencesNewline(choice) {
				return true
			}
		}
	case grammartools.Repetition:
		return elementReferencesNewline(e.Element)
	case grammartools.Optional:
		return elementReferencesNewline(e.Element)
	case grammartools.Group:
		return elementReferencesNewline(e.Element)
	case grammartools.PositiveLookahead:
		return elementReferencesNewline(e.Element)
	case grammartools.NegativeLookahead:
		return elementReferencesNewline(e.Element)
	case grammartools.OneOrMoreRepetition:
		return elementReferencesNewline(e.Element)
	case grammartools.SeparatedRepetition:
		return elementReferencesNewline(e.Element) || elementReferencesNewline(e.Separator)
	}
	return false
}

// ---------------------------------------------------------------------------
// Parse
// ---------------------------------------------------------------------------

// Parse parses tokens against the compiled grammar.
//
// It never recovers from errors: the result is a complete AST for the whole
// input (trailing NEWLINEs aside) or a non-nil error and a nil tree. Possible
// errors, all *GrammarParseError unless noted:
//
//   - a syntax error, positioned at the furthest token the parser reached;
//   - input nested deeper than Options.MaxDepth (errors.Is(err, ErrMaxDepth));
//   - an unknown Options.StartRule (plain error, errors.Is ErrUnknownStartRule);
//   - an empty grammar ("Grammar has no rules").
//
// Parse is safe to call concurrently on one Compiled. The token slice is read
// but never modified.
func (c *Compiled) Parse(tokens []lexer.Token, opts Options) (ast *ASTNode, err error) {
	// Hooks and hostile inputs must not take the process down by panic.
	// (A stack overflow is not a panic: that is what MaxDepth prevents.)
	defer func() {
		if r := recover(); r != nil {
			ast, err = nil, fmt.Errorf("parser: internal error: %v", r)
		}
	}()

	// Stage 1: pre-parse hooks transform the token list, left to right.
	for _, hook := range opts.PreParse {
		tokens = hook(tokens)
	}

	if len(c.rules) == 0 {
		return nil, fmt.Errorf("Grammar has no rules")
	}

	start := int32(0)
	if opts.StartRule != "" {
		id, ok := c.byName[opts.StartRule]
		if !ok {
			return nil, fmt.Errorf("%w: %q", ErrUnknownStartRule, opts.StartRule)
		}
		start = id
	}

	s := c.pool.Get().(*parseState)
	s.reset(tokens, opts.maxDepth(), opts.Trace)
	ast, err = s.run(start)
	s.release()
	if err != nil {
		return nil, err
	}

	// Stage 3: post-parse hooks transform the AST, left to right.
	for _, hook := range opts.PostParse {
		ast = hook(ast)
	}
	return ast, nil
}

// ---------------------------------------------------------------------------
// Parse state
// ---------------------------------------------------------------------------

// memoSlot caches one (rule, position) result. See the file comment.
type memoSlot struct {
	children []interface{} // nil when failed
	endPos   int
	rule     int32
	next     int32 // next older slot at the same position, or -1
	ok       bool
}

type parseState struct {
	c      *Compiled
	tokens []lexer.Token
	pos    int

	head  []int32
	slots []memoSlot

	depth    int
	maxDepth int
	// exceeded is the sticky "depth limit hit" flag; see the file comment.
	exceeded    bool
	exceededPos int

	furthestPos      int
	furthestExpected []string
	trace            bool

	eof lexer.Token // returned by tok() for an empty token list
}

// maxPooledSlots bounds how much memo storage is kept for reuse, so one
// enormous parse does not pin its memory for the life of the process.
const maxPooledSlots = 1 << 20

func (s *parseState) reset(tokens []lexer.Token, maxDepth int, trace bool) {
	s.tokens = tokens
	s.pos = 0
	s.depth, s.maxDepth, s.exceeded, s.exceededPos = 0, maxDepth, false, 0
	s.furthestPos, s.furthestExpected = 0, s.furthestExpected[:0]
	s.trace = trace
	s.eof = lexer.Token{Type: lexer.TokenEOF, TypeName: "EOF"}
	if n := len(tokens); n > 0 {
		s.eof.Line, s.eof.Column = tokens[n-1].Line, tokens[n-1].Column
	}

	// One head entry per position 0..len(tokens) (the position after the
	// last token is a valid place to memoise a rule that matches nothing).
	// advance() never moves beyond it.
	n := len(tokens) + 1
	if cap(s.head) < n {
		s.head = make([]int32, n)
	}
	s.head = s.head[:n]
	for i := range s.head {
		s.head[i] = -1
	}
	s.slots = s.slots[:0]
}

// release hands the state back to the pool, dropping references to the
// caller's tokens and the produced AST so pooled memory does not retain them.
func (s *parseState) release() {
	s.tokens = nil
	clear(s.slots)
	if cap(s.slots) > maxPooledSlots {
		s.slots = nil
	} else {
		s.slots = s.slots[:0]
	}
	if cap(s.head) > maxPooledSlots {
		s.head = nil
	}
	s.c.pool.Put(s)
}

// tok returns the token at the current position. Reading past the end of the
// list yields a synthetic EOF token (carrying the last token's line and
// column for error messages), never a repeat of the last real token. The old
// behaviour of repeating it let a token list with no trailing EOF recurse
// forever on a grammar like `atom = LPAREN expr RPAREN`.
func (s *parseState) tok() *lexer.Token {
	if s.pos < len(s.tokens) {
		return &s.tokens[s.pos]
	}
	return &s.eof
}

// advance consumes the current token. It never moves past the end of the
// list, so the synthetic EOF can be "matched" repeatedly without the
// position running away (which would also defeat the repetition guards).
func (s *parseState) advance() {
	if s.pos < len(s.tokens) {
		s.pos++
	}
}

func isNewline(t *lexer.Token) bool {
	if t.TypeName != "" {
		return t.TypeName == "NEWLINE"
	}
	return t.Type == lexer.TokenNewline
}

// skipNewlines advances over NEWLINE tokens. Bounded by the token list so a
// trailing NEWLINE with no EOF cannot spin forever.
func (s *parseState) skipNewlines() {
	for s.pos < len(s.tokens) && isNewline(&s.tokens[s.pos]) {
		s.pos++
	}
}

func (s *parseState) recordFailure(expected string) {
	if s.pos > s.furthestPos {
		s.furthestPos = s.pos
		s.furthestExpected = append(s.furthestExpected[:0], expected)
	} else if s.pos == s.furthestPos {
		for _, e := range s.furthestExpected {
			if e == expected {
				return
			}
		}
		s.furthestExpected = append(s.furthestExpected, expected)
	}
}

func (s *parseState) tokenAt(i int) lexer.Token {
	if i < len(s.tokens) {
		return s.tokens[i]
	}
	return s.eof
}

// run parses from the given rule and produces the final result or error,
// following the original Parse logic exactly.
func (s *parseState) run(start int32) (*ASTNode, error) {
	result := s.parseRule(start)

	// A depth refusal always wins, even if the top rule "succeeded" by
	// backtracking past it. Fail closed.
	if s.exceeded {
		return nil, &GrammarParseError{
			Message: fmt.Sprintf("%s (%d)", ErrMaxDepth.Error(), s.maxDepth),
			Tok:     s.tokenAt(s.exceededPos),
			Err:     ErrMaxDepth,
		}
	}

	if result == nil {
		tok := *s.tok()
		if len(s.furthestExpected) > 0 {
			expected := strings.Join(s.furthestExpected, " or ")
			return nil, &GrammarParseError{
				Message: fmt.Sprintf("Expected %s, got %q", expected, tok.Value),
				Tok:     tok,
			}
		}
		return nil, &GrammarParseError{Message: "Failed to parse", Tok: tok}
	}

	// Skip trailing newlines.
	s.skipNewlines()

	if s.pos < len(s.tokens) && tokenTypeName(*s.tok()) != "EOF" {
		tok := *s.tok()
		if len(s.furthestExpected) > 0 && s.furthestPos > s.pos {
			expected := strings.Join(s.furthestExpected, " or ")
			furthestTok := tok
			if s.furthestPos < len(s.tokens) {
				furthestTok = s.tokens[s.furthestPos]
			}
			return nil, &GrammarParseError{
				Message: fmt.Sprintf("Expected %s, got %q", expected, furthestTok.Value),
				Tok:     furthestTok,
			}
		}
		return nil, &GrammarParseError{
			Message: fmt.Sprintf("Unexpected token: %q", tok.Value),
			Tok:     tok,
		}
	}
	return result, nil
}

// ---------------------------------------------------------------------------
// Memo table
// ---------------------------------------------------------------------------

// memoFind returns the slot index for (rule, current position), or -1.
func (s *parseState) memoFind(pos int, rule int32) int32 {
	for i := s.head[pos]; i >= 0; i = s.slots[i].next {
		if s.slots[i].rule == rule {
			return i
		}
	}
	return -1
}

// memoInsert adds a slot at pos and returns its index. Indices stay valid for
// the whole parse (slots only ever grow), unlike pointers into the slice.
func (s *parseState) memoInsert(pos int, rule int32, m memoSlot) int32 {
	m.rule = rule
	m.next = s.head[pos]
	s.slots = append(s.slots, m)
	idx := int32(len(s.slots) - 1)
	s.head[pos] = idx
	return idx
}

// ---------------------------------------------------------------------------
// Rule and element matching
// ---------------------------------------------------------------------------

func (s *parseState) parseRule(id int32) *ASTNode {
	if id < 0 || s.exceeded {
		return nil
	}
	rule := &s.c.rules[id]

	// Memo hit: replay the cached result without recursing.
	if si := s.memoFind(s.pos, id); si >= 0 {
		m := &s.slots[si]
		s.pos = m.endPos
		if !m.ok {
			return nil
		}
		return &ASTNode{RuleName: rule.name, Children: m.children}
	}

	// The depth guard. Everything below this line recurses.
	if s.depth >= s.maxDepth {
		s.exceeded = true
		s.exceededPos = s.pos
		return nil
	}
	s.depth++
	node := s.parseRuleBody(id, rule)
	s.depth--
	return node
}

func (s *parseState) parseRuleBody(id int32, rule *compiledRule) *ASTNode {
	if s.trace {
		t := s.tok()
		fmt.Fprintf(os.Stderr, "[TRACE] rule '%s' at token %d (%s %q)",
			rule.name, s.pos, tokenTypeName(*t), t.Value)
	}

	startPos := s.pos

	// Left-recursion guard: seed the memo with a failure entry BEFORE parsing
	// the rule body. If the rule references itself (directly or indirectly)
	// at the same position, the memo check in parseRule finds this failure
	// entry and returns nil, breaking the infinite recursion cycle.
	//
	// After the initial parse, if it succeeded, we iteratively try to grow
	// the match (Warth et al., "Packrat Parsers Can Support Left Recursion",
	// 2008). Slot indices, not pointers, are held across the nested calls
	// because the slots slice may be reallocated underneath us.
	si := s.memoInsert(startPos, id, memoSlot{endPos: startPos})

	children, ok := s.match(rule.body, nil)

	s.slots[si].children, s.slots[si].endPos, s.slots[si].ok = children, s.pos, ok

	if ok {
		for {
			prevEnd := s.pos
			s.pos = startPos
			s.slots[si].children, s.slots[si].endPos, s.slots[si].ok = children, prevEnd, true
			newChildren, newOk := s.match(rule.body, nil)
			if !newOk || s.pos <= prevEnd {
				// Could not grow the match: restore the best result.
				s.pos = prevEnd
				s.slots[si].children, s.slots[si].endPos, s.slots[si].ok = children, prevEnd, true
				break
			}
			children = newChildren
		}
	}

	if !ok {
		if s.trace {
			fmt.Fprintln(os.Stderr, " → fail")
		}
		s.pos = startPos
		s.recordFailure(rule.name)
		return nil
	}

	if s.trace {
		fmt.Fprintln(os.Stderr, " → match")
	}

	// A rule body made only of repetitions and optionals (like TOML's
	// array_values) may match nothing; normalise nil to an empty slice so the
	// ASTNode is well formed.
	if children == nil {
		children = []interface{}{}
	}

	n := &ASTNode{RuleName: rule.name, Children: children}
	computeNodePosition(n)
	return n
}

// match tries element n at the current position, APPENDING what it matched to
// out and returning the extended slice. On failure the returned slice is
// truncated back to its entry length (so its contents are as if nothing was
// matched) and the caller decides about s.pos, exactly as the original
// recursive-return implementation did.
//
// Appending into one caller-owned slice, rather than returning a fresh slice
// per element and concatenating, is what removes most of the per-token
// allocations: a rule body builds exactly one children slice.
func (s *parseState) match(n *node, out []interface{}) ([]interface{}, bool) {
	if s.exceeded {
		return out, false
	}
	save := s.pos
	start := len(out)

	switch n.kind {
	case kSeq:
		for _, sub := range n.subs {
			var ok bool
			out, ok = s.match(sub, out)
			if !ok {
				s.pos = save
				return out[:start], false
			}
		}
		return out, true

	case kAlt:
		for _, choice := range n.subs {
			s.pos = save
			var ok bool
			out, ok = s.match(choice, out)
			if ok {
				return out, true
			}
			out = out[:start]
		}
		s.pos = save
		return out, false

	case kRep:
		for {
			saveRep := s.pos
			mark := len(out)
			var ok bool
			out, ok = s.match(n.sub, out)
			if !ok {
				s.pos = saveRep
				out = out[:mark]
				break
			}
			if s.pos == saveRep {
				break // matched nothing: looping again could never finish
			}
		}
		return out, true

	case kOpt:
		var ok bool
		out, ok = s.match(n.sub, out)
		if !ok {
			// NB: s.pos is deliberately not restored here (historical
			// behaviour: a failed literal may have consumed newlines).
			return out[:start], true
		}
		return out, true

	case kGroup:
		return s.match(n.sub, out)

	case kRule:
		node := s.parseRule(n.rule)
		if node != nil {
			return append(out, node), true
		}
		s.pos = save
		return out, false

	case kTok:
		return s.matchToken(n, out)

	case kLit:
		if n.skipNL {
			s.skipNewlines()
		}
		t := s.tok()
		if t.Value == n.value {
			s.advance()
			return append(out, *t), true
		}
		s.recordFailure(n.quoted)
		return out, false

	// Syntactic predicates: look without consuming.
	case kPosLook:
		var ok bool
		out, ok = s.match(n.sub, out)
		s.pos = save
		return out[:start], ok

	case kNegLook:
		var ok bool
		out, ok = s.match(n.sub, out)
		s.pos = save
		return out[:start], !ok

	case kOneOrMore:
		var ok bool
		out, ok = s.match(n.sub, out)
		if !ok {
			s.pos = save
			return out[:start], false
		}
		for {
			saveRep := s.pos
			mark := len(out)
			out, ok = s.match(n.sub, out)
			if !ok {
				s.pos = saveRep
				out = out[:mark]
				break
			}
			if s.pos == saveRep {
				break
			}
		}
		return out, true

	case kSepRep:
		// element { separator element }; zero occurrences is valid unless
		// atLeastOne.
		var ok bool
		out, ok = s.match(n.sub, out)
		if !ok {
			s.pos = save
			if n.atLeastOne {
				return out[:start], false
			}
			return out[:start], true
		}
		for {
			saveSep := s.pos
			mark := len(out)
			out, ok = s.match(n.sep, out)
			if !ok {
				s.pos = saveSep
				out = out[:mark]
				break
			}
			out, ok = s.match(n.sub, out)
			if !ok {
				s.pos = saveSep
				out = out[:mark]
				break
			}
			if s.pos == saveSep {
				break
			}
		}
		return out, true
	}

	return out, false // kFail
}

func (s *parseState) matchToken(n *node, out []interface{}) ([]interface{}, bool) {
	// Skip newlines when matching non-NEWLINE tokens.
	if n.skipNL {
		s.skipNewlines()
	}
	t := s.tok()

	// Direct string comparison (works for both enum and string types).
	if tokenTypeName(*t) == n.name {
		s.advance()
		return append(out, *t), true
	}

	// Backward compatibility: enum-based matching.
	if t.Type == n.tokType && n.tokType != lexer.TokenName {
		s.advance()
		return append(out, *t), true
	}

	s.recordFailure(n.name)
	return out, false
}
