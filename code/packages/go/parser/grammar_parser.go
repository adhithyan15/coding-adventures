package parser

import (
	"fmt"
	"sync"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// ASTNode is a generic AST node produced by grammar-driven parsing.
//
// Position fields (StartLine, StartColumn, EndLine, EndColumn) are computed
// from the first and last leaf tokens in the children tree. A value of 0
// means unset (e.g., an empty repetition node with no tokens).
type ASTNode struct {
	RuleName    string
	Children    []interface{} // Can be *ASTNode or lexer.Token
	StartLine   int           // 1-based line of first token (0 = unset)
	StartColumn int           // 1-based column of first token (0 = unset)
	EndLine     int           // 1-based line of last token (0 = unset)
	EndColumn   int           // 1-based column of last token (0 = unset)
}

// IsLeaf returns true if this node wraps a single token.
func (n *ASTNode) IsLeaf() bool {
	result, _ := StartNew[bool]("parser.ASTNode.IsLeaf", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			if len(n.Children) == 1 {
				_, ok := n.Children[0].(lexer.Token)
				return rf.Generate(true, false, ok)
			}
			return rf.Generate(true, false, false)
		}).GetResult()
	return result
}

// Token returns the leaf token if IsLeaf(), nil otherwise.
func (n *ASTNode) Token() *lexer.Token {
	result, _ := StartNew[*lexer.Token]("parser.ASTNode.Token", nil,
		func(op *Operation[*lexer.Token], rf *ResultFactory[*lexer.Token]) *OperationResult[*lexer.Token] {
			if n.IsLeaf() {
				tok := n.Children[0].(lexer.Token)
				return rf.Generate(true, false, &tok)
			}
			return rf.Generate(true, false, nil)
		}).GetResult()
	return result
}

// GrammarParseError is returned when grammar-driven parsing fails.
//
// The parser has NO error recovery. The first failure ends the parse: there is
// no partial tree, no "best effort" AST, and no resynchronisation. Callers can
// therefore treat any non-nil error as "the input is not in the language" and
// fail closed. (See also the package documentation in compiled.go.)
//
// Err, when set, is a machine-readable cause that errors.Is / errors.As can
// match (for example ErrMaxDepth). Message and Tok stay as before.
type GrammarParseError struct {
	Message string
	Tok     lexer.Token
	Err     error
}

func (e *GrammarParseError) Error() string {
	return fmt.Sprintf("Parse error at %d:%d: %s", e.Tok.Line, e.Tok.Column, e.Message)
}

// Unwrap exposes the machine-readable cause (nil for ordinary syntax errors).
func (e *GrammarParseError) Unwrap() error { return e.Err }

// ---------------------------------------------------------------------------
// Hook Types — Pre/Post Parse Transforms
// ---------------------------------------------------------------------------

// PreParseHook transforms the token list before parsing.
// Multiple hooks compose left-to-right.
type PreParseHook func(tokens []lexer.Token) []lexer.Token

// PostParseHook transforms the AST after parsing.
// Multiple hooks compose left-to-right.
type PostParseHook func(ast *ASTNode) *ASTNode

// GrammarParser interprets grammar rules at runtime with packrat memoization.
//
// It is a thin, backwards-compatible wrapper: the grammar is compiled once by
// CompileGrammar and the engine lives in compiled.go. New code that parses
// many inputs against one grammar should hold a *Compiled and call its Parse
// method directly, which skips the per-parser compile and reuses memo storage.
//
// Like everything in this package, a GrammarParser performs no error
// recovery: Parse returns either a complete tree or an error.
type GrammarParser struct {
	tokens     []lexer.Token
	compiled   *Compiled
	compileErr error
	opts       Options

	// preParseHooks holds functions that transform the token list before
	// parsing. Multiple hooks compose left-to-right.
	preParseHooks []PreParseHook

	// postParseHooks holds functions that transform the AST after parsing.
	// Multiple hooks compose left-to-right.
	postParseHooks []PostParseHook
}

// NewGrammarParser creates a new grammar-driven parser with memoization and
// the default recursion-depth limit (DefaultMaxDepth).
func NewGrammarParser(tokens []lexer.Token, grammar *grammartools.ParserGrammar) *GrammarParser {
	return NewGrammarParserWithOptions(tokens, grammar, Options{})
}

// NewGrammarParserWithTrace creates a grammar-driven parser with optional
// trace output. When trace=true, each rule attempt is printed to stderr with
// the format:
//
//	[TRACE] rule '<name>' at token <index> (<TYPE> "<value>") → match
//	[TRACE] rule '<name>' at token <index> (<TYPE> "<value>") → fail
//
// Trace mode is useful for diagnosing parse failures: it shows exactly which
// rules were tried at each position and whether they matched. Because the
// parser uses packrat memoization, each (rule, position) pair is traced at
// most once.
func NewGrammarParserWithTrace(tokens []lexer.Token, grammar *grammartools.ParserGrammar, trace bool) *GrammarParser {
	return NewGrammarParserWithOptions(tokens, grammar, Options{Trace: trace})
}

// NewGrammarParserWithOptions creates a parser with explicit Options
// (MaxDepth, StartRule, Trace). The zero Options value behaves exactly like
// NewGrammarParser.
//
// A grammar that cannot be compiled (nil) is reported by Parse, not by the
// constructor, so the constructor signature stays error-free like its
// predecessors.
//
// The compiled form of a grammar is cached per *ParserGrammar pointer, so
// building thousands of parsers against one grammar value compiles it once.
// Each entry carries a cheap fingerprint of the grammar (rule count, address
// of the first rule, a hash of the rule names and the top-level shape of each
// body); if it no longer matches, the grammar is recompiled, so appending,
// removing, renaming or replacing rules is noticed. An in-place edit deeper
// inside a rule body is NOT noticed: call ForgetGrammar after such an edit,
// or better, do not mutate a grammar once it has been used. (Generated
// grammar data is package-level and immutable, which is the intended use.)
func NewGrammarParserWithOptions(tokens []lexer.Token, grammar *grammartools.ParserGrammar, opts Options) *GrammarParser {
	c, err := compileCached(grammar)
	return &GrammarParser{tokens: tokens, compiled: c, compileErr: err, opts: opts}
}

// cacheEntry is a compiled grammar plus the fingerprint it was compiled from.
type cacheEntry struct {
	c  *Compiled
	fp grammarFingerprint
}

// grammarFingerprint is a cheap (a few microseconds for a 180-rule grammar)
// summary used to notice most mutations of a cached grammar.
type grammarFingerprint struct {
	n     int
	first *grammartools.GrammarRule
	hash  uint64
}

func fingerprintOf(g *grammartools.ParserGrammar) grammarFingerprint {
	fp := grammarFingerprint{n: len(g.Rules)}
	if fp.n > 0 {
		fp.first = &g.Rules[0]
	}
	const prime = 1099511628211
	h := uint64(14695981039346656037)
	for i := range g.Rules {
		r := &g.Rules[i]
		for j := 0; j < len(r.Name); j++ {
			h = (h ^ uint64(r.Name[j])) * prime
		}
		h = (h ^ bodyShape(r.Body)) * prime
	}
	fp.hash = h
	return fp
}

// bodyShape is the element kind plus its immediate arity.
func bodyShape(el grammartools.GrammarElement) uint64 {
	switch e := el.(type) {
	case grammartools.Sequence:
		return 1 | uint64(len(e.Elements))<<8
	case grammartools.Alternation:
		return 2 | uint64(len(e.Choices))<<8
	case grammartools.Repetition:
		return 3
	case grammartools.Optional:
		return 4
	case grammartools.Group:
		return 5
	case grammartools.RuleReference:
		return 6 | uint64(len(e.Name))<<8
	case grammartools.Literal:
		return 7 | uint64(len(e.Value))<<8
	case grammartools.PositiveLookahead:
		return 8
	case grammartools.NegativeLookahead:
		return 9
	case grammartools.OneOrMoreRepetition:
		return 10
	case grammartools.SeparatedRepetition:
		return 11
	}
	return 0
}

// compiledCache maps *ParserGrammar to its *cacheEntry. Entries pin their
// grammar, so the cache is bounded: when maxCachedGrammars distinct grammars
// are cached, the whole map is cleared (a generational reset) and filling
// starts over. Hot grammars recompile once after a reset and are cached
// again, so the cache never degrades into "compile on every call".
var (
	compiledCache     sync.Map
	compiledCacheMu   sync.Mutex // guards compiledCacheSize and inserts
	compiledCacheSize int
)

const maxCachedGrammars = 512

func compileCached(grammar *grammartools.ParserGrammar) (*Compiled, error) {
	if grammar == nil {
		return CompileGrammar(nil)
	}
	fp := fingerprintOf(grammar)
	if v, ok := compiledCache.Load(grammar); ok {
		if e := v.(*cacheEntry); e.fp == fp {
			return e.c, nil
		}
	}
	c, err := CompileGrammar(grammar)
	if err != nil {
		return nil, err
	}
	compiledCacheMu.Lock()
	defer compiledCacheMu.Unlock()
	if _, replacing := compiledCache.Load(grammar); !replacing {
		if compiledCacheSize >= maxCachedGrammars {
			compiledCache.Clear()
			compiledCacheSize = 0
		}
		compiledCacheSize++
	}
	compiledCache.Store(grammar, &cacheEntry{c: c, fp: fp})
	return c, nil
}

// ForgetGrammar drops a grammar's cached compiled form. Call it after
// mutating a rule body in place (changes to the rule list itself are noticed
// automatically), before the grammar is next used.
func ForgetGrammar(grammar *grammartools.ParserGrammar) {
	compiledCacheMu.Lock()
	defer compiledCacheMu.Unlock()
	if _, loaded := compiledCache.LoadAndDelete(grammar); loaded {
		compiledCacheSize--
	}
}

// AddPreParse registers a token transform to run before parsing.
// The hook receives the token list and returns a (possibly modified) token
// list. Multiple hooks compose left-to-right.
func (p *GrammarParser) AddPreParse(hook PreParseHook) {
	p.preParseHooks = append(p.preParseHooks, hook)
}

// AddPostParse registers an AST transform to run after parsing.
// The hook receives the root AST node and returns a (possibly modified)
// AST node. Multiple hooks compose left-to-right.
func (p *GrammarParser) AddPostParse(hook PostParseHook) {
	p.postParseHooks = append(p.postParseHooks, hook)
}

// NewlinesSignificant returns whether newlines are significant in this grammar.
func (p *GrammarParser) NewlinesSignificant() bool {
	if p.compiled == nil {
		return false
	}
	return p.compiled.newlinesSignificant
}

// Parse parses the token stream, starting from Options.StartRule (default:
// the first grammar rule). It returns a complete AST or an error; there is no
// error recovery.
func (p *GrammarParser) Parse() (ast *ASTNode, err error) {
	if p.compileErr != nil {
		return nil, p.compileErr
	}
	// A panicking pre-parse hook is reported as an error, like panics in
	// post-parse hooks (those are recovered inside Compiled.Parse).
	defer func() {
		if r := recover(); r != nil {
			ast, err = nil, fmt.Errorf("parser: internal error: %v", r)
		}
	}()
	// Historical behaviour: a pre-parse hook replaces the parser's own token
	// list, and a second call to Parse sees the transformed list.
	tokens := p.tokens
	for _, hook := range p.preParseHooks {
		tokens = hook(tokens)
	}
	p.tokens = tokens

	opts := p.opts
	opts.PostParse = append(append([]PostParseHook(nil), opts.PostParse...), p.postParseHooks...)
	return p.compiled.Parse(tokens, opts)
}

// tokenTypeName extracts the effective type name from a token.
func tokenTypeName(tok lexer.Token) string {
	if tok.TypeName != "" {
		return tok.TypeName
	}
	// Fall back to the enum-based names
	switch tok.Type {
	case lexer.TokenName:
		return "NAME"
	case lexer.TokenNumber:
		return "NUMBER"
	case lexer.TokenString:
		return "STRING"
	case lexer.TokenKeyword:
		return "KEYWORD"
	case lexer.TokenPlus:
		return "PLUS"
	case lexer.TokenMinus:
		return "MINUS"
	case lexer.TokenStar:
		return "STAR"
	case lexer.TokenSlash:
		return "SLASH"
	case lexer.TokenEquals:
		return "EQUALS"
	case lexer.TokenEqualsEquals:
		return "EQUALS_EQUALS"
	case lexer.TokenLParen:
		return "LPAREN"
	case lexer.TokenRParen:
		return "RPAREN"
	case lexer.TokenComma:
		return "COMMA"
	case lexer.TokenColon:
		return "COLON"
	case lexer.TokenNewline:
		return "NEWLINE"
	case lexer.TokenEOF:
		return "EOF"
	case lexer.TokenSemicolon:
		return "SEMICOLON"
	case lexer.TokenLBrace:
		return "LBRACE"
	case lexer.TokenRBrace:
		return "RBRACE"
	case lexer.TokenLBracket:
		return "LBRACKET"
	case lexer.TokenRBracket:
		return "RBRACKET"
	case lexer.TokenDot:
		return "DOT"
	case lexer.TokenBang:
		return "BANG"
	default:
		return "UNKNOWN"
	}
}

// ===========================================================================
// AST POSITION COMPUTATION
// ===========================================================================

// computeNodePosition fills in the StartLine/StartColumn/EndLine/EndColumn
// fields on an ASTNode from its first and last leaf tokens. If the children
// contain no tokens (e.g., empty repetition), the position fields remain zero
// (unset).
//
// A child that already has a position (every node built by a first parse
// does) answers for its whole subtree in O(1), so building a left-recursive
// chain is linear, not quadratic. Nodes without one (memo-hit copies) are
// searched. Everything here is ITERATIVE: tree height is bounded by MaxDepth
// during a parse, but these helpers also run on trees of any shape a caller
// builds, and a Go stack overflow is fatal.
func computeNodePosition(node *ASTNode) {
	var firstLine, firstCol, lastLine, lastCol int
	found := false
	for _, child := range node.Children {
		if tok, ok := firstOf(child); ok {
			firstLine, firstCol, found = tok.line, tok.col, true
			break
		}
	}
	if !found {
		return
	}
	for i := len(node.Children) - 1; i >= 0; i-- {
		if tok, ok := lastOf(node.Children[i]); ok {
			lastLine, lastCol = tok.line, tok.col
			break
		}
	}
	node.StartLine, node.StartColumn = firstLine, firstCol
	node.EndLine, node.EndColumn = lastLine, lastCol
}

type linecol struct{ line, col int }

// firstOf returns the position of the first leaf token under one child.
func firstOf(child interface{}) (linecol, bool) {
	switch c := child.(type) {
	case lexer.Token:
		return linecol{c.Line, c.Column}, true
	case *ASTNode:
		if c.StartLine != 0 {
			return linecol{c.StartLine, c.StartColumn}, true
		}
		if t := findFirstToken(c.Children); t != nil {
			return linecol{t.Line, t.Column}, true
		}
	}
	return linecol{}, false
}

// lastOf returns the position of the last leaf token under one child.
func lastOf(child interface{}) (linecol, bool) {
	switch c := child.(type) {
	case lexer.Token:
		return linecol{c.Line, c.Column}, true
	case *ASTNode:
		if c.EndLine != 0 {
			return linecol{c.EndLine, c.EndColumn}, true
		}
		if t := findLastToken(c.Children); t != nil {
			return linecol{t.Line, t.Column}, true
		}
	}
	return linecol{}, false
}

// childCursor is one level of an explicit traversal stack.
type childCursor struct {
	children []interface{}
	i        int
}

// findFirstToken returns the first leaf token in a children slice by
// depth-first traversal, or nil if there are none. Iterative.
func findFirstToken(children []interface{}) *lexer.Token {
	stack := []childCursor{{children: children}}
	for len(stack) > 0 {
		top := &stack[len(stack)-1]
		if top.i >= len(top.children) {
			stack = stack[:len(stack)-1]
			continue
		}
		child := top.children[top.i]
		top.i++
		switch c := child.(type) {
		case *ASTNode:
			stack = append(stack, childCursor{children: c.Children})
		case lexer.Token:
			return &c
		}
	}
	return nil
}

// findLastToken returns the last leaf token in a children slice by reverse
// depth-first traversal, or nil if there are none. Iterative.
func findLastToken(children []interface{}) *lexer.Token {
	stack := []childCursor{{children: children, i: len(children) - 1}}
	for len(stack) > 0 {
		top := &stack[len(stack)-1]
		if top.i < 0 {
			stack = stack[:len(stack)-1]
			continue
		}
		child := top.children[top.i]
		top.i--
		switch c := child.(type) {
		case *ASTNode:
			stack = append(stack, childCursor{children: c.Children, i: len(c.Children) - 1})
		case lexer.Token:
			return &c
		}
	}
	return nil
}

// ===========================================================================
// AST WALKING UTILITIES
// ===========================================================================

// ASTVisitor defines callbacks for walking an AST tree. Both Enter and Leave
// are optional. Each receives the current node and its parent (nil for the
// root). Returning a non-nil *ASTNode replaces the visited node; returning
// nil keeps the original.
type ASTVisitor struct {
	// Enter is called before visiting children. Return a replacement node
	// or nil to keep the original.
	Enter func(node *ASTNode, parent *ASTNode) *ASTNode

	// Leave is called after visiting children. Return a replacement node
	// or nil to keep the original.
	Leave func(node *ASTNode, parent *ASTNode) *ASTNode
}

// WalkAST performs a depth-first walk of an AST tree with enter/leave visitor
// callbacks. Visitor callbacks can return a replacement node or nil (keep
// original). Token children are not visited — only ASTNode children are
// walked.
//
// The walk is iterative, so it is safe on trees of any depth (a left-recursive
// chain of a million nodes does not overflow the stack). Visitor callbacks
// run in the same order as a recursive walk would run them.
//
// This is the generic traversal primitive. Language packages use it for
// cover grammar rewriting, desugaring, and semantic analysis.
func WalkAST(node *ASTNode, visitor ASTVisitor) *ASTNode {
	return walkNode(node, nil, visitor)
}

// walkFrame is the state of one node mid-walk: the recursive version's local
// variables, made explicit.
type walkFrame struct {
	orig        *ASTNode      // the node as the parent saw it
	current     *ASTNode      // after Enter's optional replacement
	children    []interface{} // current.Children as of entering (a snapshot)
	parent      *ASTNode
	idx         int // next child to visit
	newChildren []interface{}
	changed     bool
}

func walkNode(node *ASTNode, parent *ASTNode, visitor ASTVisitor) *ASTNode {
	var stack []walkFrame

	// enter runs the Enter phase and pushes the node's frame.
	enter := func(n, parent *ASTNode) {
		current := n
		if visitor.Enter != nil {
			if replacement := visitor.Enter(current, parent); replacement != nil {
				current = replacement
			}
		}
		stack = append(stack, walkFrame{
			orig:        n,
			current:     current,
			children:    current.Children,
			parent:      parent,
			newChildren: make([]interface{}, len(current.Children)),
		})
	}

	enter(node, parent)
	for {
		f := &stack[len(stack)-1]

		// Visit children until one is a node (descend into it) or we run out.
		descended := false
		for f.idx < len(f.children) {
			child := f.children[f.idx]
			if astChild, ok := child.(*ASTNode); ok {
				enter(astChild, f.current) // may reallocate stack: f is stale now
				descended = true
				break
			}
			f.newChildren[f.idx] = child
			f.idx++
		}
		if descended {
			continue
		}

		// All children done: rebuild if any changed, then the Leave phase.
		current := f.current
		if f.changed {
			current = &ASTNode{
				RuleName:    current.RuleName,
				Children:    f.newChildren,
				StartLine:   current.StartLine,
				StartColumn: current.StartColumn,
				EndLine:     current.EndLine,
				EndColumn:   current.EndColumn,
			}
		}
		if visitor.Leave != nil {
			if replacement := visitor.Leave(current, f.parent); replacement != nil {
				current = replacement
			}
		}

		orig := f.orig
		stack = stack[:len(stack)-1]
		if len(stack) == 0 {
			return current
		}
		pf := &stack[len(stack)-1]
		pf.newChildren[pf.idx] = current
		if current != orig {
			pf.changed = true
		}
		pf.idx++
	}
}

// FindNodes returns all nodes matching a rule name in depth-first order.
func FindNodes(node *ASTNode, ruleName string) []*ASTNode {
	var results []*ASTNode
	WalkAST(node, ASTVisitor{
		Enter: func(n *ASTNode, parent *ASTNode) *ASTNode {
			if n.RuleName == ruleName {
				results = append(results, n)
			}
			return nil
		},
	})
	return results
}

// CollectTokens collects all tokens in depth-first order. If tokenType is
// non-empty, only tokens whose effective type name matches are included.
// Pass "" to collect all tokens. Iterative.
func CollectTokens(node *ASTNode, tokenType string) []lexer.Token {
	var results []lexer.Token
	stack := []childCursor{{children: node.Children}}
	for len(stack) > 0 {
		top := &stack[len(stack)-1]
		if top.i >= len(top.children) {
			stack = stack[:len(stack)-1]
			continue
		}
		child := top.children[top.i]
		top.i++
		switch c := child.(type) {
		case *ASTNode:
			stack = append(stack, childCursor{children: c.Children})
		case lexer.Token:
			if tokenType == "" || tokenTypeName(c) == tokenType {
				results = append(results, c)
			}
		}
	}
	return results
}

// stringToTokenType maps grammar token names to TokenType constants.
func stringToTokenType(id string) lexer.TokenType {
	switch id {
	case "NAME":
		return lexer.TokenName
	case "NUMBER":
		return lexer.TokenNumber
	case "STRING":
		return lexer.TokenString
	case "KEYWORD":
		return lexer.TokenKeyword
	case "PLUS":
		return lexer.TokenPlus
	case "MINUS":
		return lexer.TokenMinus
	case "STAR":
		return lexer.TokenStar
	case "SLASH":
		return lexer.TokenSlash
	case "EQUALS":
		return lexer.TokenEquals
	case "EQUALS_EQUALS":
		return lexer.TokenEqualsEquals
	case "LPAREN":
		return lexer.TokenLParen
	case "RPAREN":
		return lexer.TokenRParen
	case "COMMA":
		return lexer.TokenComma
	case "COLON":
		return lexer.TokenColon
	case "NEWLINE":
		return lexer.TokenNewline
	case "EOF":
		return lexer.TokenEOF
	case "SEMICOLON":
		return lexer.TokenSemicolon
	case "LBRACE":
		return lexer.TokenLBrace
	case "RBRACE":
		return lexer.TokenRBrace
	case "LBRACKET":
		return lexer.TokenLBracket
	case "RBRACKET":
		return lexer.TokenRBracket
	case "DOT":
		return lexer.TokenDot
	case "BANG":
		return lexer.TokenBang
	default:
		return lexer.TokenName
	}
}
