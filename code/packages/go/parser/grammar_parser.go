package parser

import (
	"fmt"
	"os"
	"strings"

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

// GrammarParseError is raised when grammar-driven parsing fails.
type GrammarParseError struct {
	Message string
	Tok     lexer.Token
}

func (e *GrammarParseError) Error() string {
	return fmt.Sprintf("Parse error at %d:%d: %s", e.Tok.Line, e.Tok.Column, e.Message)
}

// memoEntry caches a parse result for packrat memoization.
type memoEntry struct {
	children []interface{} // nil means parse failed
	endPos   int
	ok       bool
}

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
type GrammarParser struct {
	tokens              []lexer.Token
	grammar             *grammartools.ParserGrammar
	pos                 int
	rules               map[string]grammartools.GrammarRule
	newlinesSignificant bool
	memo                map[[2]int]*memoEntry // key: [ruleIndex, position]
	ruleIndex           map[string]int        // rule name -> index for memo key
	furthestPos         int
	furthestExpected    []string
	trace               bool // When true, print rule attempts to stderr

	// preParseHooks holds functions that transform the token list before
	// parsing. Multiple hooks compose left-to-right.
	preParseHooks []PreParseHook

	// postParseHooks holds functions that transform the AST after parsing.
	// Multiple hooks compose left-to-right.
	postParseHooks []PostParseHook
}

// NewGrammarParser creates a new grammar-driven parser with memoization.
func NewGrammarParser(tokens []lexer.Token, grammar *grammartools.ParserGrammar) *GrammarParser {
	result, _ := StartNew[*GrammarParser]("parser.NewGrammarParser", nil,
		func(op *Operation[*GrammarParser], rf *ResultFactory[*GrammarParser]) *OperationResult[*GrammarParser] {
			return rf.Generate(true, false, NewGrammarParserWithTrace(tokens, grammar, false))
		}).GetResult()
	return result
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
	result, _ := StartNew[*GrammarParser]("parser.NewGrammarParserWithTrace", nil,
		func(op *Operation[*GrammarParser], rf *ResultFactory[*GrammarParser]) *OperationResult[*GrammarParser] {
			op.AddProperty("trace", trace)
			rules := make(map[string]grammartools.GrammarRule)
			ruleIndex := make(map[string]int)
			for i, rule := range grammar.Rules {
				rules[rule.Name] = rule
				ruleIndex[rule.Name] = i
			}
			p := &GrammarParser{
				tokens:    tokens,
				grammar:   grammar,
				pos:       0,
				rules:     rules,
				memo:      make(map[[2]int]*memoEntry),
				ruleIndex: ruleIndex,
				trace:     trace,
			}
			p.newlinesSignificant = p.grammarReferencesNewline()
			return rf.Generate(true, false, p)
		}).GetResult()
	return result
}

// AddPreParse registers a token transform to run before parsing.
// The hook receives the token list and returns a (possibly modified) token
// list. Multiple hooks compose left-to-right.
func (p *GrammarParser) AddPreParse(hook PreParseHook) {
	_, _ = StartNew[struct{}]("parser.AddPreParse", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			p.preParseHooks = append(p.preParseHooks, hook)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// AddPostParse registers an AST transform to run after parsing.
// The hook receives the root AST node and returns a (possibly modified)
// AST node. Multiple hooks compose left-to-right.
func (p *GrammarParser) AddPostParse(hook PostParseHook) {
	_, _ = StartNew[struct{}]("parser.AddPostParse", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			p.postParseHooks = append(p.postParseHooks, hook)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// NewlinesSignificant returns whether newlines are significant in this grammar.
func (p *GrammarParser) NewlinesSignificant() bool {
	result, _ := StartNew[bool]("parser.NewlinesSignificant", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			return rf.Generate(true, false, p.newlinesSignificant)
		}).GetResult()
	return result
}

func (p *GrammarParser) current() lexer.Token {
	if p.pos < len(p.tokens) {
		return p.tokens[p.pos]
	}
	return p.tokens[len(p.tokens)-1]
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

func (p *GrammarParser) recordFailure(expected string) {
	if p.pos > p.furthestPos {
		p.furthestPos = p.pos
		p.furthestExpected = []string{expected}
	} else if p.pos == p.furthestPos {
		for _, e := range p.furthestExpected {
			if e == expected {
				return
			}
		}
		p.furthestExpected = append(p.furthestExpected, expected)
	}
}

func (p *GrammarParser) grammarReferencesNewline() bool {
	for _, rule := range p.grammar.Rules {
		if p.elementReferencesNewline(rule.Body) {
			return true
		}
	}
	return false
}

func (p *GrammarParser) elementReferencesNewline(element grammartools.GrammarElement) bool {
	switch e := element.(type) {
	case grammartools.RuleReference:
		return e.IsToken && e.Name == "NEWLINE"
	case grammartools.Sequence:
		for _, sub := range e.Elements {
			if p.elementReferencesNewline(sub) {
				return true
			}
		}
	case grammartools.Alternation:
		for _, choice := range e.Choices {
			if p.elementReferencesNewline(choice) {
				return true
			}
		}
	case grammartools.Repetition:
		return p.elementReferencesNewline(e.Element)
	case grammartools.Optional:
		return p.elementReferencesNewline(e.Element)
	case grammartools.Group:
		return p.elementReferencesNewline(e.Element)
	case grammartools.PositiveLookahead:
		return p.elementReferencesNewline(e.Element)
	case grammartools.NegativeLookahead:
		return p.elementReferencesNewline(e.Element)
	case grammartools.OneOrMoreRepetition:
		return p.elementReferencesNewline(e.Element)
	case grammartools.SeparatedRepetition:
		return p.elementReferencesNewline(e.Element) || p.elementReferencesNewline(e.Separator)
	}
	return false
}

// Parse parses the token stream using the first grammar rule as entry point.
func (p *GrammarParser) Parse() (*ASTNode, error) {
	return StartNew[*ASTNode]("parser.GrammarParser.Parse", nil,
		func(op *Operation[*ASTNode], rf *ResultFactory[*ASTNode]) *OperationResult[*ASTNode] {
			// Stage 1: Pre-parse hooks transform the token list.
			// Each hook receives the output of the previous hook, composing
			// left-to-right. This enables token-level transforms like filtering,
			// rewriting, or injecting synthetic tokens before parsing begins.
			if len(p.preParseHooks) > 0 {
				tokens := p.tokens
				for _, hook := range p.preParseHooks {
					tokens = hook(tokens)
				}
				p.tokens = tokens
			}

			if len(p.grammar.Rules) == 0 {
				return rf.Fail(nil, fmt.Errorf("Grammar has no rules"))
			}

			entryRule := p.grammar.Rules[0]
			parseResult := p.parseRule(entryRule.Name)

			if parseResult == nil {
				tok := p.current()
				if len(p.furthestExpected) > 0 {
					expected := strings.Join(p.furthestExpected, " or ")
					return rf.Fail(nil, &GrammarParseError{
						Message: fmt.Sprintf("Expected %s, got %q", expected, tok.Value),
						Tok:     tok,
					})
				}
				return rf.Fail(nil, &GrammarParseError{
					Message: "Failed to parse",
					Tok:     tok,
				})
			}

			// Skip trailing newlines
			for p.pos < len(p.tokens) && tokenTypeName(p.current()) == "NEWLINE" {
				p.pos++
			}

			if p.pos < len(p.tokens) && tokenTypeName(p.current()) != "EOF" {
				tok := p.current()
				if len(p.furthestExpected) > 0 && p.furthestPos > p.pos {
					expected := strings.Join(p.furthestExpected, " or ")
					furthestTok := tok
					if p.furthestPos < len(p.tokens) {
						furthestTok = p.tokens[p.furthestPos]
					}
					return rf.Fail(nil, &GrammarParseError{
						Message: fmt.Sprintf("Expected %s, got %q", expected, furthestTok.Value),
						Tok:     furthestTok,
					})
				}
				return rf.Fail(nil, &GrammarParseError{
					Message: fmt.Sprintf("Unexpected token: %q", tok.Value),
					Tok:     tok,
				})
			}

			// Stage 3: Post-parse hooks transform the AST.
			// Each hook receives the root AST node and returns a (possibly modified)
			// AST node. Hooks compose left-to-right.
			for _, hook := range p.postParseHooks {
				parseResult = hook(parseResult)
			}

			return rf.Generate(true, false, parseResult)
		}).GetResult()
}

func (p *GrammarParser) parseRule(ruleName string) *ASTNode {
	rule, exists := p.rules[ruleName]
	if !exists {
		return nil
	}

	// Check memo cache
	idx, hasIdx := p.ruleIndex[ruleName]
	if hasIdx {
		key := [2]int{idx, p.pos}
		if entry, ok := p.memo[key]; ok {
			p.pos = entry.endPos
			if !entry.ok {
				return nil
			}
			return &ASTNode{RuleName: ruleName, Children: entry.children}
		}
	}

	// Emit trace line before attempting the rule.
	if p.trace {
		tok := p.current()
		fmt.Fprintf(os.Stderr, "[TRACE] rule '%s' at token %d (%s %q)",
			ruleName, p.pos, tokenTypeName(tok), tok.Value)
	}

	startPos := p.pos

	// Left-recursion guard: seed the memo with a failure entry BEFORE parsing
	// the rule body. If the rule references itself (directly or indirectly)
	// at the same position, the memo check above will find this failure entry
	// and return nil, breaking the infinite recursion cycle.
	//
	// After the initial parse, if it succeeded, we iteratively try to grow
	// the match. This is the standard technique for handling left recursion
	// in packrat parsers (see Warth et al., "Packrat Parsers Can Support
	// Left Recursion", 2008).
	if hasIdx {
		key := [2]int{idx, startPos}
		p.memo[key] = &memoEntry{children: nil, endPos: startPos, ok: false}
	}

	children, ok := p.matchElement(rule.Body)

	// Cache result
	if hasIdx {
		key := [2]int{idx, startPos}
		p.memo[key] = &memoEntry{children: children, endPos: p.pos, ok: ok}

		// If the initial parse succeeded and this rule might be left-recursive,
		// iteratively try to grow the match. Each iteration re-parses the rule
		// body with the previous successful result cached, allowing the
		// left-recursive alternative to consume more input.
		if ok {
			for {
				prevEnd := p.pos
				p.pos = startPos
				p.memo[key] = &memoEntry{children: children, endPos: prevEnd, ok: true}
				newChildren, newOk := p.matchElement(rule.Body)
				if !newOk || p.pos <= prevEnd {
					// Could not grow the match — restore the best result.
					p.pos = prevEnd
					p.memo[key] = &memoEntry{children: children, endPos: prevEnd, ok: true}
					break
				}
				children = newChildren
			}
		}
	}

	if !ok {
		if p.trace {
			fmt.Fprintln(os.Stderr, " → fail")
		}
		p.pos = startPos
		p.recordFailure(ruleName)
		return nil
	}

	if p.trace {
		fmt.Fprintln(os.Stderr, " → match")
	}

	// When a rule body consists entirely of repetitions and optionals
	// (like TOML's array_values rule), children may be nil even on success.
	// Normalize nil to an empty slice so the ASTNode is well-formed.
	if children == nil {
		children = []interface{}{}
	}

	// Compute position info from child tokens.
	node := &ASTNode{RuleName: ruleName, Children: children}
	computeNodePosition(node)
	return node
}

func (p *GrammarParser) matchElement(element grammartools.GrammarElement) ([]interface{}, bool) {
	savePos := p.pos

	switch e := element.(type) {
	case grammartools.Sequence:
		var children []interface{}
		for _, sub := range e.Elements {
			res, ok := p.matchElement(sub)
			if !ok {
				p.pos = savePos
				return nil, false
			}
			children = append(children, res...)
		}
		return children, true

	case grammartools.Alternation:
		for _, choice := range e.Choices {
			p.pos = savePos
			res, ok := p.matchElement(choice)
			if ok {
				return res, true
			}
		}
		p.pos = savePos
		return nil, false

	case grammartools.Repetition:
		var children []interface{}
		for {
			saveRep := p.pos
			res, ok := p.matchElement(e.Element)
			if !ok {
				p.pos = saveRep
				break
			}
			children = append(children, res...)
		}
		return children, true

	case grammartools.Optional:
		res, ok := p.matchElement(e.Element)
		if !ok {
			return []interface{}{}, true
		}
		return res, true

	case grammartools.Group:
		return p.matchElement(e.Element)

	case grammartools.RuleReference:
		if e.IsToken {
			return p.matchTokenReference(e)
		}
		node := p.parseRule(e.Name)
		if node != nil {
			return []interface{}{node}, true
		}
		p.pos = savePos
		return nil, false

	case grammartools.Literal:
		token := p.current()
		// Skip insignificant newlines before literal matching
		if !p.newlinesSignificant {
			for tokenTypeName(token) == "NEWLINE" {
				p.pos++
				token = p.current()
			}
		}
		if token.Value == e.Value {
			p.pos++
			return []interface{}{token}, true
		}
		p.recordFailure(fmt.Sprintf("%q", e.Value))
		return nil, false

	// ---------------------------------------------------------------
	// Extension: Syntactic predicates (lookahead without consuming)
	// ---------------------------------------------------------------

	case grammartools.PositiveLookahead:
		// Succeed if inner element matches, but consume no input.
		_, ok := p.matchElement(e.Element)
		p.pos = savePos
		if ok {
			return []interface{}{}, true
		}
		return nil, false

	case grammartools.NegativeLookahead:
		// Succeed if inner element does NOT match, consume no input.
		_, ok := p.matchElement(e.Element)
		p.pos = savePos
		if !ok {
			return []interface{}{}, true
		}
		return nil, false

	// ---------------------------------------------------------------
	// Extension: One-or-more repetition
	// ---------------------------------------------------------------

	case grammartools.OneOrMoreRepetition:
		// Match one required, then zero or more additional.
		first, ok := p.matchElement(e.Element)
		if !ok {
			p.pos = savePos
			return nil, false
		}
		children := append([]interface{}{}, first...)
		for {
			saveRep := p.pos
			res, ok := p.matchElement(e.Element)
			if !ok {
				p.pos = saveRep
				break
			}
			children = append(children, res...)
		}
		return children, true

	// ---------------------------------------------------------------
	// Extension: Separated repetition
	// ---------------------------------------------------------------

	case grammartools.SeparatedRepetition:
		// Match: element { separator element }
		// Or with AtLeastOne=false: [ element { separator element } ]
		first, ok := p.matchElement(e.Element)
		if !ok {
			p.pos = savePos
			if e.AtLeastOne {
				return nil, false
			}
			return []interface{}{}, true // zero occurrences is valid
		}
		children := append([]interface{}{}, first...)
		for {
			saveSep := p.pos
			sep, ok := p.matchElement(e.Separator)
			if !ok {
				p.pos = saveSep
				break
			}
			next, ok := p.matchElement(e.Element)
			if !ok {
				p.pos = saveSep
				break
			}
			children = append(children, sep...)
			children = append(children, next...)
		}
		return children, true
	}

	return nil, false
}

func (p *GrammarParser) matchTokenReference(e grammartools.RuleReference) ([]interface{}, bool) {
	token := p.current()

	// Skip newlines when matching non-NEWLINE tokens
	if !p.newlinesSignificant && e.Name != "NEWLINE" {
		for tokenTypeName(token) == "NEWLINE" {
			p.pos++
			token = p.current()
		}
	}

	typeName := tokenTypeName(token)

	// Direct string comparison (works for both enum and string types)
	if typeName == e.Name {
		p.pos++
		return []interface{}{token}, true
	}

	// Backward compatibility: try enum-based matching
	expectedType := stringToTokenType(e.Name)
	if token.Type == expectedType && expectedType != lexer.TokenName {
		p.pos++
		return []interface{}{token}, true
	}

	p.recordFailure(e.Name)
	return nil, false
}

// ===========================================================================
// AST POSITION COMPUTATION
// ===========================================================================

// computeNodePosition fills in the StartLine/StartColumn/EndLine/EndColumn
// fields on an ASTNode by walking its children to find the first and last
// leaf tokens. If the children contain no tokens (e.g., empty repetition),
// the position fields remain zero (unset).
func computeNodePosition(node *ASTNode) {
	first := findFirstToken(node.Children)
	last := findLastToken(node.Children)
	if first != nil && last != nil {
		node.StartLine = first.Line
		node.StartColumn = first.Column
		node.EndLine = last.Line
		node.EndColumn = last.Column
	}
}

// findFirstToken returns the first leaf token in a children slice by
// depth-first traversal. Returns nil if no tokens are found.
func findFirstToken(children []interface{}) *lexer.Token {
	for _, child := range children {
		switch c := child.(type) {
		case *ASTNode:
			tok := findFirstToken(c.Children)
			if tok != nil {
				return tok
			}
		case lexer.Token:
			return &c
		}
	}
	return nil
}

// findLastToken returns the last leaf token in a children slice by
// reverse depth-first traversal. Returns nil if no tokens are found.
func findLastToken(children []interface{}) *lexer.Token {
	for i := len(children) - 1; i >= 0; i-- {
		switch c := children[i].(type) {
		case *ASTNode:
			tok := findLastToken(c.Children)
			if tok != nil {
				return tok
			}
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
// This is the generic traversal primitive. Language packages use it for
// cover grammar rewriting, desugaring, and semantic analysis.
func WalkAST(node *ASTNode, visitor ASTVisitor) *ASTNode {
	return walkNode(node, nil, visitor)
}

func walkNode(node *ASTNode, parent *ASTNode, visitor ASTVisitor) *ASTNode {
	// Enter phase — visitor may replace the node.
	current := node
	if visitor.Enter != nil {
		if replacement := visitor.Enter(current, parent); replacement != nil {
			current = replacement
		}
	}

	// Walk children recursively.
	childrenChanged := false
	newChildren := make([]interface{}, len(current.Children))
	for i, child := range current.Children {
		if astChild, ok := child.(*ASTNode); ok {
			walked := walkNode(astChild, current, visitor)
			if walked != astChild {
				childrenChanged = true
			}
			newChildren[i] = walked
		} else {
			newChildren[i] = child
		}
	}

	// If children changed, create a new node with updated children.
	if childrenChanged {
		current = &ASTNode{
			RuleName:    current.RuleName,
			Children:    newChildren,
			StartLine:   current.StartLine,
			StartColumn: current.StartColumn,
			EndLine:     current.EndLine,
			EndColumn:   current.EndColumn,
		}
	}

	// Leave phase — visitor may replace the node.
	if visitor.Leave != nil {
		if replacement := visitor.Leave(current, parent); replacement != nil {
			current = replacement
		}
	}

	return current
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
// Pass "" to collect all tokens.
func CollectTokens(node *ASTNode, tokenType string) []lexer.Token {
	var results []lexer.Token
	var walk func(n *ASTNode)
	walk = func(n *ASTNode) {
		for _, child := range n.Children {
			switch c := child.(type) {
			case *ASTNode:
				walk(c)
			case lexer.Token:
				if tokenType == "" || tokenTypeName(c) == tokenType {
					results = append(results, c)
				}
			}
		}
	}
	walk(node)
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
