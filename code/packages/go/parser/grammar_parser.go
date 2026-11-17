package parser

import (
	"fmt"

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
func NewGrammarParserWithOptions(tokens []lexer.Token, grammar *grammartools.ParserGrammar, opts Options) *GrammarParser {
	c, err := CompileGrammar(grammar)
	return &GrammarParser{tokens: tokens, compiled: c, compileErr: err, opts: opts}
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
func (p *GrammarParser) Parse() (*ASTNode, error) {
	if p.compileErr != nil {
		return nil, p.compileErr
	}
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
