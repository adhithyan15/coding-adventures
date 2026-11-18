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
	"math"
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
//     1.6 KB per level). Go's nominal cap is 1e9 bytes, but stacks grow by
//     doubling and a 1,024 MB stack would exceed 1e9, so the largest stack a
//     goroutine can actually have is 512 MB: the default sits about 30x below
//     the real crash point. (Go's initial stack is tiny; growing to 16 MB
//     costs a few copies and is fine.)
//
//   - Far above any real input. Depth counts RULES, not tokens. A grammar
//     with a 20-rule precedence chain spends about 20 levels per nested
//     parenthesis, so 10,000 levels admits roughly 500 nested parentheses
//     in the deepest expression grammars here, and about 5,000 nested JSON
//     arrays (two rules per level). Hand-written code and data essentially
//     never exceed a few dozen levels of nesting.
//
// The TREE is bounded as well as the call path. Every memoised result records
// the height of the subtree it built; reusing it (a memo hit) or growing it
// (a left-recursive rule wrapping its previous result one more level, as in
// `e = e PLUS NUMBER | NUMBER`) is refused when depth plus height would pass
// MaxDepth. The invariant: a successful parse returns a tree of at most
// MaxDepth levels (the root is level 1), so a chain of about DefaultMaxDepth
// or more left-associative terms is refused. That keeps every recursive
// consumer of the finished tree safe. (A rule referenced exactly at the
// limit by a speculative attempt can be refused slightly early, so the usable
// height can be one less than MaxDepth.)
//
// Callers who legitimately need more can raise Options.MaxDepth, up to
// HardMaxDepth.
const DefaultMaxDepth = 10000

// HardMaxDepth is the ceiling Options.MaxDepth is clamped to. It exists so
// that "never overflow the stack" holds even when a caller passes a huge
// limit (say math.MaxInt) believing it means "unlimited". There is
// deliberately no way to disable the limit.
//
// Rule levels alone do not bound the stack: between two rule calls the
// matcher recurses through nested groups, optionals, repetitions and
// lookaheads, so a rule whose body is 50 elements deep costs 50 frames per
// level. The engine therefore also charges every rule its element nesting
// (see maxFrames) and refuses when the total would exceed HardMaxFrames,
// whatever MaxDepth says. Consequently HardMaxDepth is only reachable through
// rules of weight 4 or less (a body at most 2 elements deep, such as
// `a = A B`): 200,000 / 4 = 50,000. Deeper rule bodies hit the frame budget
// first, and an explicit large MaxDepth is silently undercut by it (the error
// message then names the frame budget instead of MaxDepth).
const HardMaxDepth = 50000

// HardMaxFrames bounds the total of (element nesting + 2) over the rules on
// the current call path: an upper bound on the number of Go stack frames the
// matcher can have live. Measured on arm64/go1.26 with a 50-deep nested-group
// rule: the worst case at this budget fits a 64 MB stack (about 250 bytes per
// frame, and it overflowed a 32 MB cap), an eighth of the 512 MB usable stack
// (see DefaultMaxDepth).
const HardMaxFrames = 200000

// ErrMaxDepth is the cause (see errors.Is) of the *GrammarParseError returned
// when input nests deeper than the configured limit.
var ErrMaxDepth = errors.New("input nests deeper than the supported limit")

// ErrMaxTokens is the cause of the error returned when the token list is
// longer than Options.MaxTokens.
var ErrMaxTokens = errors.New("input has more tokens than the supported limit")

// ErrMaxMemoSlots is the cause of the error returned when the packrat memo
// table would exceed Options.MaxMemoSlots (or the int32 index space).
var ErrMaxMemoSlots = errors.New("parse needs more memo entries than the supported limit")

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

	// MaxTokens, when positive, rejects token lists longer than this with
	// ErrMaxTokens before any parsing happens. Zero means no limit. Use it
	// (with MaxMemoSlots) to cap the memory an untrusted input can make the
	// parser spend: memo storage grows with tokens times the rules tried per
	// position, and the AST with the input.
	MaxTokens int

	// MaxMemoSlots, when positive, bounds the number of memoised (rule,
	// position) results; exceeding it fails the parse with ErrMaxMemoSlots.
	// Zero means no explicit limit beyond the hard int32 index space
	// (about 2.1 billion slots).
	MaxMemoSlots int

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
	kind          nodeKind
	leadingTokens uint8   // kAlt: cached leading direct-token run, capped at 255
	subs          []*node // kSeq, kAlt
	sub           *node   // kRep, kOpt, kGroup, kPosLook, kNegLook, kOneOrMore, kSepRep
	sep           *node   // kSepRep
	atLeastOne    bool

	rule int32 // kRule: rule id, or -1 if the name is undefined

	name    string          // kTok: token name; kRule: rule name
	tokType lexer.TokenType // kTok: legacy enum fallback
	skipNL  bool            // kTok, kLit: skip insignificant newlines first
	value   string          // kLit: the literal text

	// exp is the interned id of what a failure of this element reports as
	// "expected": the token name (kTok) or the %q-quoted literal (kLit). See
	// Compiled.expID.
	exp int32
}

type compiledRule struct {
	name string
	body *node
	// weight is an upper bound on the Go stack frames one level of this rule
	// can keep live: parseRule and parseRuleBody, plus one match frame per
	// level of element nesting in the body. See HardMaxFrames.
	weight int

	// exp is the interned id of the rule name, recorded as the expected
	// thing when the rule fails.
	exp int32

	// leftRec says whether the rule might, at the same token position, reach
	// itself again. Only such rules need the left-recursion growth loop in
	// parseRuleBody. It is computed by markLeftRecursive and is deliberately
	// an over-approximation: true is always safe, false must be proven.
	leftRec bool
}

// nodeHeight is the longest chain of nested match frames below n (a leaf is
// 1). Rule references do not descend: the callee is charged separately.
func nodeHeight(n *node) int {
	h := 0
	for _, s := range n.subs {
		if sh := nodeHeight(s); sh > h {
			h = sh
		}
	}
	for _, s := range []*node{n.sub, n.sep} {
		if s != nil {
			if sh := nodeHeight(s); sh > h {
				h = sh
			}
		}
	}
	return h + 1
}

// Compiled is an immutable, goroutine-safe form of a parser grammar. Build it
// once with CompileGrammar and call Parse for each input.
type Compiled struct {
	rules               []compiledRule
	byName              map[string]int32
	newlinesSignificant bool
	pool                sync.Pool // of *parseState
	scratch             sync.Pool // of *astScratch, for Tree.ToAST

	// expNames[id] is the text of expected-id id; expIDs is the inverse,
	// used only while compiling (nil afterwards). Ids are interned BY STRING VALUE, so two
	// different elements that print the same text share one id and the
	// "Expected A or B" de-duplication behaves exactly as it did with strings.
	expNames []string
	expIDs   map[string]int32
}

// CompileGrammar resolves rule names to ids and pre-computes everything the
// hot path would otherwise recompute per token. The grammar is not retained
// or modified; later changes to it do not affect the result.
//
// Duplicate rule names behave as before: the last definition wins, for the
// start rule and for references alike.
func CompileGrammar(grammar *grammartools.ParserGrammar) (*Compiled, error) {
	return compileGrammar(grammar, false)
}

// compileGrammar is CompileGrammar with a test knob: forceLeftRec marks every
// rule as possibly left-recursive, which restores the pre-analysis behaviour
// (the growth loop after every successful rule). Tests compare the two
// settings to prove the analysis changes nothing observable.
func compileGrammar(grammar *grammartools.ParserGrammar, forceLeftRec bool) (*Compiled, error) {
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
		body := c.compileElement(bodies[i])
		c.rules[i] = compiledRule{name: names[i], body: body, weight: nodeHeight(body) + 2, exp: c.expID(names[i])}
	}
	c.markLeftRecursive(forceLeftRec)
	c.expIDs = nil // only needed while compiling; Compiled is immutable from here

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
		if runEnd := leadingTokenRunEnd(n.subs); runEnd >= 2 {
			if runEnd > 255 {
				runEnd = 255
			}
			n.leadingTokens = uint8(runEnd)
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
				skipNL: !c.newlinesSignificant && e.Name != "NEWLINE", exp: c.expID(e.Name)}
		}
		id, ok := c.byName[e.Name]
		if !ok {
			id = -1
		}
		return &node{kind: kRule, rule: id, name: e.Name}
	case grammartools.Literal:
		return &node{kind: kLit, value: e.Value,
			skipNL: !c.newlinesSignificant, exp: c.expID(fmt.Sprintf("%q", e.Value))}
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
//   - an empty grammar ("Grammar has no rules");
//   - a token list longer than Options.MaxTokens (ErrMaxTokens), or a parse
//     that needs more than Options.MaxMemoSlots memo entries (ErrMaxMemoSlots).
//
// Things worth knowing about the input and the result:
//
//   - Past the end of the token list the parser sees a synthetic EOF token (it
//     carries the last token's line and column). So `p = A B EOF` accepts the
//     list [A, B] with no explicit EOF, and an empty list is an error only for
//     grammars whose start rule cannot match nothing.
//   - After the start rule matches, trailing NEWLINEs are skipped and the
//     parse is complete if the next token is EOF or the list is exhausted. An
//     EOF token in the MIDDLE of the list therefore ends the parse and
//     everything after it is ignored. Consumers that cannot allow that must
//     reject any EOF token before the last position themselves.
//   - A repetition stops after an iteration that consumed nothing, but that
//     iteration's children are kept (a zero-width match can still contribute
//     nodes, for example an empty nullable rule).
//   - Every rule's Children slice is non-nil, empty if the rule matched
//     nothing, including on packrat memo hits.
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

	tokens, start, err := c.prepare(tokens, opts)
	if err != nil {
		return nil, err
	}

	s := c.pool.Get().(*parseState)
	s.reset(tokens, opts)
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

// prepare is the part of Parse and ParseInto that happens before the engine
// runs: the pre-parse hooks, and every refusal that needs no parse state. It
// returns the (possibly hook-transformed) token list and the start rule id.
// The caller owns the recover(): hooks run here and may panic.
func (c *Compiled) prepare(tokens []lexer.Token, opts Options) ([]lexer.Token, int32, error) {
	// Stage 1: pre-parse hooks transform the token list, left to right.
	for _, hook := range opts.PreParse {
		tokens = hook(tokens)
	}

	if len(c.rules) == 0 {
		return nil, 0, fmt.Errorf("Grammar has no rules")
	}

	start := int32(0)
	if opts.StartRule != "" {
		id, ok := c.byName[opts.StartRule]
		if !ok {
			return nil, 0, fmt.Errorf("%w: %q", ErrUnknownStartRule, opts.StartRule)
		}
		start = id
	}

	if opts.MaxTokens > 0 && len(tokens) > opts.MaxTokens {
		return nil, 0, &GrammarParseError{
			Message: fmt.Sprintf("%s (%d)", ErrMaxTokens.Error(), opts.MaxTokens),
			Tok:     tokens[opts.MaxTokens],
			Err:     ErrMaxTokens,
		}
	}

	// Token positions, memo slots and the flat tree are all int32-indexed.
	// No machine can hold 2 billion 64-byte tokens, but a refusal costs one
	// comparison and means an index can never wrap.
	if len(tokens) >= flatIndexLimit {
		var last lexer.Token
		if len(tokens) > 0 {
			last = tokens[len(tokens)-1]
		}
		return nil, 0, &GrammarParseError{
			Message: fmt.Sprintf("%s (%d)", ErrMaxTokens.Error(), flatIndexLimit),
			Tok:     last,
			Err:     ErrMaxTokens,
		}
	}
	return tokens, start, nil
}

// ---------------------------------------------------------------------------
// Parse state, and the FLAT tree representation
// ---------------------------------------------------------------------------
//
// While parsing, the engine never builds an *ASTNode. A packrat parser tries
// far more than it keeps: every failed alternative, every speculative rule,
// every left-recursion growth step that does not pan out builds a subtree
// that is thrown away. Allocating a 72-byte ASTNode, a children slice and a
// 64-byte boxed token for each of those is where the original engine spent
// most of its time (profiled: about 87 allocations per strace line).
//
// So the engine builds the tree in four flat arrays of int32s and small
// structs, all pooled with the parse state, and Parse turns the part of it
// that SURVIVES into *ASTNodes at the end (toAST, below):
//
//	tokens   the caller's token list. A token is referred to by its INDEX.
//	         Index len(tokens) means the synthetic EOF token the parser sees
//	         past the end of the list (see tok).
//
//	kids     one []int32 for the whole parse. A rule's children are a
//	         contiguous run of it. A value >= 0 is a token index. A value < 0
//	         is a RULE REFERENCE, ^ref, where ref = node<<1 | memo: node is an
//	         index into nodes, and the low bit says whether the reference is
//	         the node itself (0) or a memo hit replaying it (1).
//
//	nodes    []cnode. One entry per rule result that parsed: {rule,
//	         kids[start:end]}. A node is appended when its rule body
//	         finishes. A memo hit appends NOTHING (see below).
//
//	stack    the scratch stack match() pushes onto while a rule body runs. On
//	         success the body's slice of it is copied into kids and popped; on
//	         backtracking it is simply truncated. No allocation either way.
//
// Two facts about the order nodes are appended in carry the whole design:
//
//  1. A node's children are always finished before the node is, so every
//     child (and every child's children) has a SMALLER index than its parent.
//     toAST can therefore mark what is reachable by sweeping indexes
//     downward from the root, and build the tree by sweeping upward, with no
//     recursion at all. (The tree can be 50,000 levels deep; a recursive
//     toAST would put the Go stack back in play, which is exactly what
//     MaxDepth exists to avoid.)
//  2. Memoised results are node indexes, so replaying one costs one stack
//     push and no copying.
//
// MEMO HITS ARE DISTINCT NODES THAT SHARE CHILDREN. This is observable
// behaviour of the original engine that the flat form preserves on purpose.
// When a rule is replayed from the memo, the original returned a NEW
// *ASTNode whose Children was the very same slice as the first parse's
// (the slice is shared, the node is not), with StartLine/EndLine zero
// because computeNodePosition ran only when the rule actually parsed. Code
// downstream can depend on all three (parent positions are computed from
// first and last tokens, and a zero child position makes it look deeper), so
// the flat engine keeps them: a memo hit pushes the tagged reference
// ^(node<<1|1) onto the stack and nothing else; toAST allocates one fresh,
// position-less *ASTNode for that slot of the run and points its Children at
// the source node's run.
//
// Why the hit is a tag in the kid and not a node of its own: a packrat parse
// can replay results enormously more often than it parses (a lookahead
// that re-scans a repetition at every position replays O(n^2) times in n
// tokens while parsing O(n) things). An appended node per hit would make
// memory grow with parse TIME and defeat MaxTokens and MaxMemoSlots as
// memory bounds; a tagged push is truncated away if the surrounding attempt
// fails, and costs 4 bytes in kids only when its run is kept (and a kept run
// is as long as the grammar's own element count says). Node indexes
// therefore need one spare bit, so nodes are limited to 2^30.
//
// Every rule result that is memoised is a node, even a seed that
// left-recursion growth later supersedes, because a memo hit during growth
// replays exactly that intermediate result.

// cnode is one rule result in the flat tree.
type cnode struct {
	rule  int32
	start int32 // children are kids[start:end]
	end   int32
}

// flatIndexLimit is the largest number of kids the flat arrays may hold, and
// twice the largest number of nodes (a node index shares an int32 with the
// memo bit); both are indexed by int32. It is a variable only so a test can lower
// it: no machine has memory for 2 billion entries of a tree, but the check
// must fail closed (ErrMaxMemoSlots), never wrap.
var flatIndexLimit = math.MaxInt32 - 1

// memoSlot caches one (rule, position) result. See the file comment.
type memoSlot struct {
	endPos int32
	rule   int32
	next   int32 // next older slot at the same position, or -1
	res    int32 // node index of the cached result; -1 when the rule failed
	height int32 // height of the subtree this result builds (1 = no node children)
}

type parseState struct {
	c      *Compiled
	tokens []lexer.Token
	pos    int

	head  []int32
	slots []memoSlot

	// The flat tree. See the comment above.
	stack []int32
	kids  []int32
	nodes []cnode
	limit int // see flatIndexLimit

	// Scratch for toAST (see astScratch). Embedded so a reused state can
	// never leak a previous parse's tokens or nodes into a new tree.
	astScratch

	depth    int // rule levels on the call path, plus left-recursion growth steps
	maxDepth int
	frames   int // sum of rule weights on the call path (see HardMaxFrames)

	// curMax is the greatest subtree height among the node children matched
	// so far by the rule body being parsed (restored when matching backtracks).
	curMax int32

	maxSlots int

	// exceeded is the sticky "a resource limit was hit" flag; see the file
	// comment. limitErr says which limit (ErrMaxDepth or ErrMaxMemoSlots).
	exceeded    bool
	limitErr    error
	limitDetail string // overrides the default message detail when set
	exceededPos int

	furthestPos      int
	furthestExpected []int32 // expected-ids (see Compiled.expID), no duplicates
	trace            bool

	eof lexer.Token // returned by tok() for an empty token list
}

// maxPooledSlots bounds how much memo storage (slots and head, about 24 bytes
// per entry) is kept for reuse, so one enormous parse does not pin its memory
// for the life of the process. A buffer that grew past it is dropped on
// release and regrown by the next big parse.
const maxPooledSlots = 1 << 20

// maxPooledFlat bounds the same way each flat-tree buffer (stack, kids, nodes
// and the toAST scratch: about 50 bytes per entry across all of them, so a
// pooled state holds at most about 6 MB of them, on top of at most about 25 MB
// of memo storage). It is lower than maxPooledSlots because these are
// per-node, and one hostile input should not leave tens of megabytes pinned
// per pooled state; an ordinary parse (a source file of a few thousand
// tokens) is well below it and reuses its buffers.
const maxPooledFlat = 1 << 17

func (s *parseState) reset(tokens []lexer.Token, opts Options) {
	s.tokens = tokens
	s.pos = 0
	s.depth, s.frames, s.maxDepth = 0, 0, opts.maxDepth()
	s.maxSlots = opts.MaxMemoSlots
	s.limit = flatIndexLimit
	s.exceeded, s.limitErr, s.limitDetail, s.exceededPos = false, nil, "", 0
	s.curMax = 0
	s.furthestPos, s.furthestExpected = 0, s.furthestExpected[:0]
	s.trace = opts.Trace
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
	s.stack, s.kids, s.nodes = s.stack[:0], s.kids[:0], s.nodes[:0]
}

// release hands the state back to the pool, dropping references to the
// caller's tokens and the produced AST so pooled memory does not retain them.
// It is NOT called when a parse panics: a state that was mid-mutation is
// simply dropped, never reused.
func (s *parseState) release() {
	s.tokens = nil
	s.astScratch.release()

	s.slots = s.slots[:0]
	if cap(s.slots) > maxPooledSlots {
		s.slots = nil
	}
	if cap(s.head) > maxPooledSlots {
		s.head = nil
	}
	s.stack, s.kids, s.nodes = s.stack[:0], s.kids[:0], s.nodes[:0]
	if cap(s.stack) > maxPooledFlat {
		s.stack = nil
	}
	if cap(s.kids) > maxPooledFlat {
		s.kids = nil
	}
	if cap(s.nodes) > maxPooledFlat {
		s.nodes = nil
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

// recordFailure notes that something expected at the current position did not
// match. Only the furthest position's set is kept, without duplicates. It
// takes an interned id, so the hot path compares integers and never builds a
// string; run turns the ids back into text, only when there is an error.
func (s *parseState) recordFailure(expected int32) {
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

// expectedText renders the furthest-failure set as "A or B or C".
func (s *parseState) expectedText() string {
	names := make([]string, len(s.furthestExpected))
	for i, id := range s.furthestExpected {
		names[i] = s.c.expNames[id]
	}
	return strings.Join(names, " or ")
}

// expID interns an expected-thing's text (compile time only).
func (c *Compiled) expID(name string) int32 {
	if id, ok := c.expIDs[name]; ok {
		return id
	}
	if c.expIDs == nil {
		c.expIDs = map[string]int32{}
	}
	id := int32(len(c.expNames))
	c.expNames = append(c.expNames, name)
	c.expIDs[name] = id
	return id
}

// run parses from the given rule and produces the final result or error,
// following the original Parse logic exactly. The parse itself works on the
// flat tree; the *ASTNode tree is built (toAST) only once the parse has
// succeeded, so an error never pays for one.
func (s *parseState) run(start int32) (*ASTNode, error) {
	root, err := s.runFlat(start)
	if err != nil {
		return nil, err
	}
	return s.toAST(root), nil
}

// runFlat is run without the final toAST: it parses and returns a reference to
// the root node in the flat arrays (always a plain node, see toAST), or the
// error. ParseInto stops here; Parse goes on to build the *ASTNode tree.
func (s *parseState) runFlat(start int32) (int32, error) {
	result := s.parseRule(start)

	// A depth refusal always wins, even if the top rule "succeeded" by
	// backtracking past it. Fail closed.
	if s.exceeded {
		limit := fmt.Sprint(s.maxDepth)
		if s.limitErr == ErrMaxMemoSlots {
			limit = fmt.Sprint(s.maxSlots)
		}
		if s.limitDetail != "" {
			limit = s.limitDetail
		}
		return -1, &GrammarParseError{
			Message: fmt.Sprintf("%s (%s)", s.limitErr.Error(), limit),
			Tok:     s.tokenAt(s.exceededPos),
			Err:     s.limitErr,
		}
	}

	if result < 0 {
		tok := *s.tok()
		if len(s.furthestExpected) > 0 {
			expected := s.expectedText()
			return -1, &GrammarParseError{
				Message: fmt.Sprintf("Expected %s, got %q", expected, tok.Value),
				Tok:     tok,
			}
		}
		return -1, &GrammarParseError{Message: "Failed to parse", Tok: tok}
	}

	// Skip trailing newlines.
	s.skipNewlines()

	if s.pos < len(s.tokens) && tokenTypeName(*s.tok()) != "EOF" {
		tok := *s.tok()
		if len(s.furthestExpected) > 0 && s.furthestPos > s.pos {
			expected := s.expectedText()
			furthestTok := tok
			if s.furthestPos < len(s.tokens) {
				furthestTok = s.tokens[s.furthestPos]
			}
			return -1, &GrammarParseError{
				Message: fmt.Sprintf("Expected %s, got %q", expected, furthestTok.Value),
				Tok:     furthestTok,
			}
		}
		return -1, &GrammarParseError{
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
//
// It returns -1, and raises the sticky limit flag, when the table is full:
// at Options.MaxMemoSlots if set, and always before an index would overflow
// int32.
func (s *parseState) memoInsert(pos int, rule int32, m memoSlot) int32 {
	if len(s.slots) >= math.MaxInt32-1 || (s.maxSlots > 0 && len(s.slots) >= s.maxSlots) {
		s.fail(ErrMaxMemoSlots)
		return -1
	}
	m.rule = rule
	m.next = s.head[pos]
	s.slots = append(s.slots, m)
	idx := int32(len(s.slots) - 1)
	s.head[pos] = idx
	return idx
}

// ---------------------------------------------------------------------------
// Flat tree construction
// ---------------------------------------------------------------------------

// seal turns the kids pushed on the stack since base into a finished rule
// result: it copies them into the permanent kids array, pops them, and
// appends the node. It returns the node's index (not a reference: the caller
// shifts it into one), or -1 (with the sticky flag
// raised) if the flat arrays are out of int32 index space.
//
// The copy is what lets backtracking be a plain truncation of the stack: a
// result that is kept is moved out of the way, and one that is not is just
// popped.
func (s *parseState) seal(rule int32, base int) int32 {
	n := len(s.stack) - base
	if len(s.kids)+n > s.limit || len(s.nodes) >= s.limit>>1 {
		s.flatOverflow()
		s.stack = s.stack[:base]
		return -1
	}
	start := int32(len(s.kids))
	s.kids = append(s.kids, s.stack[base:]...)
	s.stack = s.stack[:base]
	idx := int32(len(s.nodes))
	s.nodes = append(s.nodes, cnode{rule: rule, start: start, end: int32(len(s.kids))})
	return idx
}

func (s *parseState) flatOverflow() {
	s.limitDetail = fmt.Sprintf("tree storage index space %d", s.limit)
	s.fail(ErrMaxMemoSlots)
}

// emptyChildren is every childless node's Children: non-nil (a rule that
// matched nothing is `[]`, not null, in JSON-ish consumers) and shared, which
// is safe because it has no capacity: an append on it copies.
var emptyChildren = []interface{}{}

// Bits of parseState.mark; see toAST.
const (
	needNode = 1 // a plain reference to this node is reachable
	needRun  = 2 // this node's children run is needed
)

// toAST materialises the *ASTNode tree for the reference root. It is the only
// place ASTNodes, children slices and boxed tokens are allocated, and it
// allocates only for what is reachable from root: all the nodes of failed or
// superseded attempts stay in the flat arrays and are dropped with them.
//
// Each index i of nodes can be wanted in two ways, and mark records which:
//
//	needNode  a plain reference to i is in a reachable run (or i is the
//	          root): i needs an *ASTNode of its own;
//	needRun   i's children run must be built: because i needs a node, or
//	          because a memo reference in a reachable run replays i.
//
// Pass 1 sweeps node indexes from root DOWN (children always have smaller
// indexes than their parents, see the comment above), expanding each wanted
// run once, so it totals how many nodes and children slots are needed and
// pass 2 can allocate exactly that much. A memo reference costs one node of
// its own (the position-less duplicate) but marks nothing new to build.
//
// Pass 2 sweeps UP, so a node's children exist before it does:
//
//   - children go into ONE []interface{} slab for the whole parse, each run
//     cap-clipped (children[a:b:b]). The clip is what makes this safe: with
//     plain subslices a consumer's append(n.Children, x) would overwrite the
//     next run's first slot. It also lets a node and its memo hits share one
//     run, as the original engine's shared slice;
//   - ASTNodes come from slabs of at most astChunk, so retaining one node
//     keeps at most a chunk alive rather than the whole parse's;
//   - each token position is boxed into an interface (an allocation: a
//     lexer.Token is 64 bytes) at most once, however many runs mention it;
//   - computeNodePosition runs on every parsed node, in index order
//     (children first), exactly as the original ran it when a rule matched,
//     and NOT on memo-hit nodes, which keep zero positions. A run that is
//     reachable is built once, so a hit node lives in exactly one run, and
//     every parent sharing that run shares the same hit node, as before.
const astChunk = 256

func (s *parseState) toAST(root int32) *ASTNode {
	return buildAST(s.c, s.tokens, s.eof, s.nodes, s.kids, &s.astScratch, root)
}

// astScratch is the working storage of buildAST. It is all pointers or
// marks, zero (nil entries) while idle, and bounded by maxPooledFlat when
// kept. A parseState embeds one; Tree.ToAST borrows one from the Compiled.
type astScratch struct {
	boxed []interface{} // token index -> its boxed lexer.Token, built on demand
	ptrs  []*ASTNode    // node index -> its materialised *ASTNode
	mark  []uint8       // node index -> needNode | needRun
	coff  []int32       // node index -> offset of its children in the slab
}

// release clears every pointer (so an idle holder keeps no reference to a
// finished tree or the caller's tokens) and drops buffers over maxPooledFlat.
func (sc *astScratch) release() {
	clear(sc.boxed)
	clear(sc.ptrs)
	sc.boxed, sc.ptrs = sc.boxed[:0], sc.ptrs[:0]
	if cap(sc.boxed) > maxPooledFlat {
		sc.boxed = nil
	}
	if cap(sc.ptrs) > maxPooledFlat {
		sc.ptrs = nil
	}
	if cap(sc.mark) > maxPooledFlat {
		sc.mark = nil
	}
	if cap(sc.coff) > maxPooledFlat {
		sc.coff = nil
	}
}

// buildAST is toAST over an explicit flat tree, so both a parseState and a
// Tree can use it. tokens and eof are the token list the tree indexes.
func buildAST(c *Compiled, tokens []lexer.Token, eof lexer.Token, nodes []cnode, kids []int32, sc *astScratch, root int32) *ASTNode {
	// The root reference must be a plain node, never a memo hit: the
	// top-level parseRule call happens when the memo is empty, so it always
	// parses. If that ever changed, the root would have no node of its own
	// to materialise, so fail closed (Parse turns a panic into an internal
	// error and never returns this state to the pool) rather than build a
	// wrong tree.
	if root < 0 || root&1 != 0 {
		panic("parser: root reference is not a parsed node")
	}
	n := len(nodes)
	if cap(sc.mark) < n {
		sc.mark = make([]uint8, n)
	}
	mark := sc.mark[:n]
	clear(mark)
	if cap(sc.coff) < n {
		sc.coff = make([]int32, n)
	}
	coff := sc.coff[:n]
	if cap(sc.ptrs) < n {
		sc.ptrs = make([]*ASTNode, n)
	}
	sc.ptrs = sc.ptrs[:n]
	ptrs := sc.ptrs
	if t := len(tokens) + 1; cap(sc.boxed) < t {
		sc.boxed = make([]interface{}, t)
	} else {
		sc.boxed = sc.boxed[:t]
	}

	// Pass 1: mark and count.
	rootNode := int(root >> 1)
	mark[rootNode] = needNode
	nAST, nKids := 0, 0
	for i := rootNode; i >= 0; i-- {
		m := mark[i]
		if m == 0 {
			continue
		}
		if m&needNode != 0 {
			nAST++
			mark[i] |= needRun
		}
		sn := &nodes[i]
		if mark[i]&needRun == 0 {
			continue
		}
		nKids += int(sn.end - sn.start)
		for _, k := range kids[sn.start:sn.end] {
			if k >= 0 {
				continue
			}
			ref := ^k
			if ref&1 != 0 {
				nAST++ // the position-less node of this memo hit
				mark[ref>>1] |= needRun
			} else {
				mark[ref>>1] |= needNode
			}
		}
	}

	// Pass 2: build.
	slab := make([]interface{}, nKids)
	var chunk []ASTNode
	newNode := func() *ASTNode {
		if len(chunk) == cap(chunk) {
			chunk = make([]ASTNode, 0, min(astChunk, nAST))
		}
		chunk = chunk[:len(chunk)+1]
		nAST--
		return &chunk[len(chunk)-1]
	}
	// run returns node r's children as the shared, cap-clipped slice.
	run := func(r int32) []interface{} {
		rn := &nodes[r]
		if rn.end == rn.start {
			return emptyChildren
		}
		lo := int(coff[r])
		hi := lo + int(rn.end-rn.start)
		return slab[lo:hi:hi]
	}
	w := 0
	for i := 0; i <= rootNode; i++ {
		m := mark[i]
		if m == 0 {
			continue
		}
		cn := &nodes[i]
		if m&needRun != 0 {
			coff[i] = int32(w)
			for _, k := range kids[cn.start:cn.end] {
				switch {
				case k >= 0:
					slab[w] = sc.box(tokens, eof, k)
				case ^k&1 == 0:
					slab[w] = ptrs[^k>>1]
				default:
					src := ^k >> 1
					h := newNode()
					h.RuleName = c.rules[nodes[src].rule].name
					h.Children = run(src)
					slab[w] = h
				}
				w++
			}
		}
		if m&needNode != 0 {
			a := newNode()
			a.RuleName = c.rules[cn.rule].name
			a.Children = run(int32(i))
			computeNodePosition(a)
			ptrs[i] = a
		}
	}
	return ptrs[rootNode]
}

// box returns token index k as an interface value (a lexer.Token), boxing it
// the first time it is asked for. Index len(tokens) is the synthetic EOF.
func (sc *astScratch) box(tokens []lexer.Token, eof lexer.Token, k int32) interface{} {
	if b := sc.boxed[k]; b != nil {
		return b
	}
	var b interface{}
	if int(k) < len(tokens) {
		b = tokens[k]
	} else {
		b = eof
	}
	sc.boxed[k] = b
	return b
}

// ---------------------------------------------------------------------------
// Rule and element matching
// ---------------------------------------------------------------------------

// parseRule parses (or replays) rule id at the current position and returns a
// reference to the result, node<<1 | memo (see the flat tree comment), or -1
// if it did not match.
func (s *parseState) parseRule(id int32) int32 {
	if id < 0 || s.exceeded {
		return -1
	}
	rule := &s.c.rules[id]

	// Memo hit: replay the cached result without recursing.
	if si := s.memoFind(s.pos, id); si >= 0 {
		m := &s.slots[si]
		s.pos = int(m.endPos)
		if m.res < 0 {
			return -1
		}
		// The cached subtree may have been built higher up the tree than
		// where it is being reused. Reusing it here puts its whole height
		// below the current depth, so the tree-height bound must be checked
		// again (see the file comment and DefaultMaxDepth).
		if int64(s.depth)+int64(m.height) > int64(s.maxDepth) {
			s.fail(ErrMaxDepth)
			return -1
		}
		if m.height > s.curMax {
			s.curMax = m.height
		}
		return m.res<<1 | 1
	}

	// The depth guards. Everything below this line recurses. Two budgets:
	// rule levels (Options.MaxDepth) and stack frames (HardMaxFrames, which
	// charges a rule for its internal element nesting).
	if s.depth >= s.maxDepth {
		s.fail(ErrMaxDepth)
		return -1
	}
	if s.frames+rule.weight > HardMaxFrames {
		s.limitDetail = fmt.Sprintf("stack-frame budget %d exceeded, %d frames in use at rule depth %d",
			HardMaxFrames, s.frames, s.depth)
		s.fail(ErrMaxDepth)
		return -1
	}
	s.depth++
	s.frames += rule.weight
	node := s.parseRuleBody(id, rule)
	s.frames -= rule.weight
	s.depth--
	return node
}

// fail raises the sticky resource-limit flag (first cause wins). Once it is
// up, parseRule and match fail immediately and run reports the error.
func (s *parseState) fail(cause error) {
	if !s.exceeded {
		s.exceeded = true
		s.limitErr = cause
		s.exceededPos = s.pos
	}
}

func (s *parseState) parseRuleBody(id int32, rule *compiledRule) int32 {
	if s.trace {
		t := s.tok()
		fmt.Fprintf(os.Stderr, "[TRACE] rule '%s' at token %d (%s %q)",
			rule.name, s.pos, tokenTypeName(*t), t.Value)
	}

	startPos := s.pos

	// Left-recursion guard: seed the memo with a failure entry BEFORE parsing
	// the rule body. If the rule references itself (directly or indirectly)
	// at the same position, the memo check in parseRule finds this failure
	// entry and returns -1, breaking the infinite recursion cycle.
	//
	// After the initial parse, if it succeeded, we iteratively try to grow
	// the match (Warth et al., "Packrat Parsers Can Support Left Recursion",
	// 2008). Slot indices, not pointers, are held across the nested calls
	// because the slots slice may be reallocated underneath us.
	si := s.memoInsert(startPos, id, memoSlot{endPos: int32(startPos), res: -1})
	if si < 0 {
		return -1 // memo table full; the sticky flag is up
	}

	// Tree height bookkeeping: outerMax is the caller's running maximum,
	// restored on failure; inside the body curMax collects the heights of the
	// node children matched.
	outerMax := s.curMax
	s.curMax = 0
	base := len(s.stack)
	ok := s.match(rule.body)
	height := s.curMax + 1
	res := int32(-1)
	if ok {
		// A rule that matched nothing has an empty (non-nil) Children, so a
		// memo hit and a first parse build identical nodes (JSON `[]`, not
		// `null`): toAST gives every childless node emptyChildren.
		if res = s.seal(id, base); res < 0 {
			return -1 // flat arrays full; the sticky flag is up
		}
		s.slots[si].res, s.slots[si].endPos = res, int32(s.pos)
		s.slots[si].height = height
	} else {
		s.stack = s.stack[:base]
		s.slots[si].res, s.slots[si].endPos = -1, int32(s.pos)
	}

	if ok {
		// Each successful growth step wraps the previous tree in one more
		// level, without any recursion in the parser. The seed's height is
		// stored in its memo slot, so the next step's reference to it is a
		// memo hit that re-checks the bound; and each new height is checked
		// explicitly, so the finished tree is never taller than MaxDepth
		// (counting this rule's own depth), which is what keeps recursive
		// consumers of the tree safe.
		for rule.leftRec {
			prevEnd := s.pos
			s.pos = startPos
			s.slots[si].res, s.slots[si].endPos = res, int32(prevEnd)
			s.slots[si].height = height
			s.curMax = 0
			newOk := s.match(rule.body)
			if !newOk || s.pos <= prevEnd {
				// Could not grow the match: restore the best result.
				s.stack = s.stack[:base]
				s.pos = prevEnd
				s.slots[si].res, s.slots[si].endPos = res, int32(prevEnd)
				s.slots[si].height = height
				break
			}
			// Retention: a growth step is sealed (kept in kids and nodes)
			// only when it consumed MORE tokens than the previous result
			// (s.pos > prevEnd, checked above), so a rule at one position
			// seals at most len(tokens)+1 results, each as long as that
			// rule's body. The original engine allocated the same slices and
			// kept the superseded ones alive only while referenced.
			if res = s.seal(id, base); res < 0 {
				return -1
			}
			height = s.curMax + 1
			if int64(s.depth-1)+int64(height) > int64(s.maxDepth) {
				s.fail(ErrMaxDepth)
				break
			}
		}
	}

	if !ok {
		s.curMax = outerMax
		if s.trace {
			fmt.Fprintln(os.Stderr, " → fail")
		}
		s.pos = startPos
		s.recordFailure(rule.exp)
		return -1
	}
	s.curMax = max(outerMax, height)

	if s.trace {
		fmt.Fprintln(os.Stderr, " → match")
	}

	// The node a rule returns is its final result record (a plain, non-memo
	// reference). Positions are computed for it by toAST, as the original
	// did here with computeNodePosition.
	return res << 1
}

// match tries element n at the current position, PUSHING what it matched onto
// s.stack: token indexes, and ^node for each rule result. On failure the
// stack is truncated back to its entry height (so its contents are as if
// nothing was matched) and the caller decides about s.pos, exactly as the
// original recursive-return implementation did with its slices.
//
// Pushing onto one shared stack, rather than returning a fresh slice per
// element and concatenating, is what removes the per-token allocations: a
// rule body's children are one contiguous run of the stack, copied once into
// kids by seal.
//
// Subtree heights (s.curMax) follow the children: wherever matching discards
// children that were already matched (a failed alternative, a failed optional
// or repetition iteration, a lookahead), curMax is restored with them. A
// failed sequence needs no restore of its own because every caller that
// continues after a failure is one of those places, and a rule that fails
// outright restores its caller's value.
func (s *parseState) match(n *node) bool {
	if s.exceeded {
		return false
	}
	save := s.pos
	start := len(s.stack)

	switch n.kind {
	case kSeq:
		for _, sub := range n.subs {
			if !s.match(sub) {
				s.pos = save
				s.stack = s.stack[:start]
				return false
			}
		}
		return true

	case kAlt:
		cm := s.curMax
		choices := n.subs
		next := 0
		if runEnd := int(n.leadingTokens); runEnd >= 2 {
			if s.matchLeadingTokenRun(choices[:runEnd], save) {
				return true
			}
			next = runEnd
		}
		for _, choice := range choices[next:] {
			s.pos = save
			if s.match(choice) {
				return true
			}
			s.stack = s.stack[:start]
			s.curMax = cm
		}
		s.pos = save
		return false

	case kRep:
		for {
			saveRep := s.pos
			mark := len(s.stack)
			cm := s.curMax
			if !s.match(n.sub) {
				s.pos = saveRep
				s.stack = s.stack[:mark]
				s.curMax = cm
				break
			}
			if s.pos == saveRep {
				break // matched nothing: looping again could never finish
			}
		}
		return true

	case kOpt:
		cm := s.curMax
		if !s.match(n.sub) {
			// NB: s.pos is deliberately not restored here (historical
			// behaviour: a failed literal may have consumed newlines).
			s.curMax = cm
			s.stack = s.stack[:start]
			return true
		}
		return true

	case kGroup:
		return s.match(n.sub)

	case kRule:
		if r := s.parseRule(n.rule); r >= 0 {
			s.stack = append(s.stack, ^r)
			return true
		}
		s.pos = save
		return false

	case kTok:
		return s.matchToken(n)

	case kLit:
		if n.skipNL {
			s.skipNewlines()
		}
		t := s.tok()
		if t.Value == n.value {
			s.stack = append(s.stack, int32(s.pos))
			s.advance()
			return true
		}
		s.recordFailure(n.exp)
		return false

	// Syntactic predicates: look without consuming.
	case kPosLook:
		cm := s.curMax
		ok := s.match(n.sub)
		s.pos = save
		s.curMax = cm // matched children are discarded
		s.stack = s.stack[:start]
		return ok

	case kNegLook:
		cm := s.curMax
		ok := s.match(n.sub)
		s.pos = save
		s.curMax = cm
		s.stack = s.stack[:start]
		return !ok

	case kOneOrMore:
		if !s.match(n.sub) {
			s.pos = save
			s.stack = s.stack[:start]
			return false
		}
		for {
			saveRep := s.pos
			mark := len(s.stack)
			cm := s.curMax
			if !s.match(n.sub) {
				s.pos = saveRep
				s.stack = s.stack[:mark]
				s.curMax = cm
				break
			}
			if s.pos == saveRep {
				break
			}
		}
		return true

	case kSepRep:
		// element { separator element }; zero occurrences is valid unless
		// atLeastOne.
		cm0 := s.curMax
		if !s.match(n.sub) {
			s.pos = save
			s.curMax = cm0
			s.stack = s.stack[:start]
			return !n.atLeastOne
		}
		for {
			saveSep := s.pos
			mark := len(s.stack)
			cm := s.curMax
			if !s.match(n.sep) {
				s.pos = saveSep
				s.stack = s.stack[:mark]
				s.curMax = cm // a partly matched separator leaves nothing behind
				break
			}
			if !s.match(n.sub) {
				s.pos = saveSep
				s.stack = s.stack[:mark]
				s.curMax = cm // the separator's children were dropped too
				break
			}
			if s.pos == saveSep {
				break
			}
		}
		return true
	}

	return false // kFail
}

// leadingTokenRunEnd returns the end of an initial run of direct token
// alternatives that all use the same newline policy. It runs only while
// compiling a grammar. The 255-choice cap fits the compact cached field and
// leaves any longer suffix to the ordinary matcher. Keeping this to the
// leading run means a preceding nonterminal choice can set the sticky
// resource-limit flag before later alternatives are considered, exactly as in
// the generic matcher.
func leadingTokenRunEnd(choices []*node) int {
	if len(choices) < 2 || choices[0].kind != kTok {
		return 0
	}
	skipNL := choices[0].skipNL
	end := 1
	for end < len(choices) && end < 255 && choices[end].kind == kTok && choices[end].skipNL == skipNL {
		end++
	}
	if end < 2 {
		return 0
	}
	return end
}

// matchLeadingTokenRun tries direct token alternatives using one lookahead.
// Each branch still contributes its expected token in the original order when
// it fails; the whole run restores the pre-skip position if none matches.
func (s *parseState) matchLeadingTokenRun(choices []*node, save int) bool {
	s.pos = save
	if choices[0].skipNL {
		s.skipNewlines()
	}
	tokenPos := s.pos
	t := s.tok()
	tn := t.TypeName
	if tn == "" {
		tn = tokenTypeName(*t)
	}

	for _, choice := range choices {
		// The run contains only kTok nodes and cannot itself set a resource
		// limit, but keep the sticky refusal guard explicit at this boundary.
		if s.exceeded {
			s.pos = save
			return false
		}
		if tn == choice.name || (t.Type == choice.tokType && choice.tokType != lexer.TokenName) {
			s.pos = tokenPos
			s.stack = append(s.stack, int32(tokenPos))
			s.advance()
			return true
		}
		s.pos = tokenPos
		s.recordFailure(choice.exp)
	}
	s.pos = save
	return false
}

func (s *parseState) matchToken(n *node) bool {
	// Skip newlines when matching non-NEWLINE tokens.
	if n.skipNL {
		s.skipNewlines()
	}
	t := s.tok()

	// Direct string comparison (works for both enum and string types).
	tn := t.TypeName
	if tn == "" {
		tn = tokenTypeName(*t) // legacy enum-only token
	}
	if tn == n.name {
		s.stack = append(s.stack, int32(s.pos))
		s.advance()
		return true
	}

	// Backward compatibility: enum-based matching.
	if t.Type == n.tokType && n.tokType != lexer.TokenName {
		s.stack = append(s.stack, int32(s.pos))
		s.advance()
		return true
	}

	s.recordFailure(n.exp)
	return false
}
