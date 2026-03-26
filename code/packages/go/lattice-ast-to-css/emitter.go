package latticeasttocss

// emitter.go — CSSEmitter walks a clean CSS AST and produces formatted CSS text.
//
// # What Is the Emitter?
//
// After the LatticeTransformer reduces a Lattice AST to pure CSS nodes, the
// emitter serializes those nodes into a CSS text string. Think of it as a
// "pretty printer" for the CSS AST.
//
// The emitter is the last stage of the pipeline:
//
//	Lattice source
//	  → lexer (tokens)
//	  → parser (AST)
//	  → transformer (CSS-only AST)
//	  → emitter (CSS text)   ← this file
//
// # Two Output Modes
//
// Pretty mode (default):
//
//	.button {
//	  color: red;
//	  background: blue;
//	}
//
// Minified mode:
//
//	.button{color:red;background:blue;}
//
// # Actual AST Structure
//
// The CSS AST from the parser uses these rule names (observed from actual output):
//
//	stylesheet
//	  rule
//	    qualified_rule
//	      selector_list          ← the selector (e.g., ".button")
//	        complex_selector
//	          compound_selector
//	            subclass_selector
//	              class_selector
//	                DOT + IDENT
//	      block
//	        LBRACE
//	        block_contents
//	          block_item
//	            declaration_or_nested
//	              declaration     ← property: value;
//	                property
//	                  IDENT
//	                COLON
//	                value_list
//	                  value
//	                    IDENT
//	                SEMICOLON
//	        RBRACE
//	    at_rule
//	      AT_KEYWORD
//	      at_prelude
//	      block | SEMICOLON
//
// # Token Handling
//
// Tokens that have been stripped of whitespace/comments by the lexer are
// reconstructed with appropriate spacing. The emitter does NOT depend on the
// original token positions — it derives spacing from the AST structure.

import (
	"strings"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

// ============================================================================
// CSSEmitter
// ============================================================================

// CSSEmitter converts a CSS AST into formatted CSS text.
//
// The same emitter instance can emit multiple ASTs; it resets its state
// for each Emit() call. Use NewCSSEmitter to create one.
//
// Usage:
//
//	emitter := NewCSSEmitter(false, "  ")
//	css := emitter.Emit(cssAST)
type CSSEmitter struct {
	minify bool   // if true, emit compact CSS with no extra whitespace
	indent string // indentation string per level (e.g., "  " or "\t")
}

// NewCSSEmitter creates a new CSSEmitter.
//
//	minify: true for minified output, false for pretty-printed output
//	indent: indentation string per level (ignored when minify is true)
//	        common values: "  " (2 spaces), "    " (4 spaces), "\t" (tab)
func NewCSSEmitter(minify bool, indent string) *CSSEmitter {
	if indent == "" {
		indent = "  " // default: 2-space indentation
	}
	return &CSSEmitter{
		minify: minify,
		indent: indent,
	}
}

// Emit serializes a CSS AST to a string.
//
// If the AST is nil, returns an empty string. The output ends with a newline
// in pretty mode, but not in minified mode.
func (e *CSSEmitter) Emit(node *parser.ASTNode) string {
	if node == nil {
		return ""
	}
	var sb strings.Builder
	e.emitStylesheet(node, &sb, 0)
	result := strings.TrimSpace(sb.String())
	if result == "" {
		return ""
	}
	if !e.minify {
		return result + "\n"
	}
	return result
}

// ============================================================================
// Stylesheet
// ============================================================================

// emitStylesheet emits all top-level rules, separated by blank lines in pretty mode.
func (e *CSSEmitter) emitStylesheet(node *parser.ASTNode, sb *strings.Builder, depth int) {
	first := true
	for _, child := range node.Children {
		ast, ok := child.(*parser.ASTNode)
		if !ok {
			continue
		}
		// Emit based on rule name (stylesheet contains rule wrappers or direct rules)
		switch ast.RuleName {
		case "rule":
			// Each rule may be qualified_rule, at_rule, etc.
			if e.isEmptyNode(ast) {
				continue
			}
			if !first && !e.minify {
				sb.WriteString("\n")
			}
			e.emitRule(ast, sb, depth)
			first = false
		default:
			// Direct emission for non-rule children (uncommon but possible)
			if !e.isEmptyNode(ast) {
				if !first && !e.minify {
					sb.WriteString("\n")
				}
				e.emitAny(ast, sb, depth)
				first = false
			}
		}
	}
}

// isEmptyNode returns true if a node has no meaningful CSS content.
//
// Used to skip empty rules produced by @if branches that evaluated to false
// or @for loops with zero iterations.
func (e *CSSEmitter) isEmptyNode(node *parser.ASTNode) bool {
	if node == nil || len(node.Children) == 0 {
		return true
	}
	return false
}

// ============================================================================
// Rule Dispatch
// ============================================================================

// emitRule emits a rule node (which wraps either a qualified_rule or an at_rule).
//
//	rule = qualified_rule | at_rule | lattice_rule (already removed) ;
func (e *CSSEmitter) emitRule(node *parser.ASTNode, sb *strings.Builder, depth int) {
	for _, child := range node.Children {
		if ast, ok := child.(*parser.ASTNode); ok {
			e.emitAny(ast, sb, depth)
		}
	}
}

// emitAny dispatches to the appropriate emitter based on rule name.
func (e *CSSEmitter) emitAny(node *parser.ASTNode, sb *strings.Builder, depth int) {
	if node == nil {
		return
	}
	switch node.RuleName {
	case "qualified_rule":
		e.emitQualifiedRule(node, sb, depth)
	case "at_rule":
		e.emitAtRule(node, sb, depth)
	case "block":
		e.emitBlock(node, sb, depth)
	case "block_contents":
		e.emitBlockContents(node, sb, depth)
	case "block_item":
		e.emitBlockItem(node, sb, depth)
	case "declaration_or_nested":
		e.emitDeclarationOrNested(node, sb, depth)
	case "declaration":
		e.emitDeclaration(node, sb, depth)
	default:
		// Generic fallback: emit all children recursively
		e.emitChildrenAny(node, sb, depth)
	}
}

// ============================================================================
// Qualified Rule (selector + block)
// ============================================================================

// emitQualifiedRule emits: selector { declarations... }
//
// Actual AST structure:
//
//	qualified_rule
//	  selector_list   ← the CSS selector
//	  block
//	    LBRACE
//	    block_contents
//	    RBRACE
//
// Pretty:
//
//	.selector {
//	  property: value;
//	}
//
// Minified:
//
//	.selector{property:value;}
func (e *CSSEmitter) emitQualifiedRule(node *parser.ASTNode, sb *strings.Builder, depth int) {
	// Write indentation for the selector
	e.writeIndent(sb, depth)

	var selectorNode *parser.ASTNode
	var blockNode *parser.ASTNode

	for _, child := range node.Children {
		ast, ok := child.(*parser.ASTNode)
		if !ok {
			continue
		}
		switch ast.RuleName {
		case "selector_list", "prelude":
			selectorNode = ast
		case "block":
			blockNode = ast
		}
	}

	// Emit selector
	if selectorNode != nil {
		e.emitSelectorTokens(selectorNode, sb)
		if !e.minify {
			sb.WriteString(" ")
		}
	}

	// Emit block
	if blockNode != nil {
		e.emitBlock(blockNode, sb, depth)
	} else {
		sb.WriteString("{}")
	}

	if !e.minify {
		sb.WriteString("\n")
	}
}

// emitSelectorTokens collects and emits all tokens in a selector subtree.
//
// Selectors are made of many nested AST rules (selector_list → complex_selector →
// compound_selector → subclass_selector → class_selector, etc.). Rather than
// implementing each rule specifically, we flatten all tokens into a string
// and join them without spaces (CSS selectors have no spaces between their
// structural tokens, except for combinators like " ", ">", "~", "+").
func (e *CSSEmitter) emitSelectorTokens(node *parser.ASTNode, sb *strings.Builder) {
	tokens := e.collectSelectorTokens(node)
	sb.WriteString(strings.Join(tokens, ""))
}

// collectSelectorTokens recursively collects all relevant tokens in a selector subtree.
//
// Most tokens are emitted directly (DOT, IDENT, LBRACKET, etc.).
// WHITESPACE tokens become a single space (combinator).
// LBRACE, RBRACE, SEMICOLON are skipped.
func (e *CSSEmitter) collectSelectorTokens(node *parser.ASTNode) []string {
	var parts []string
	for _, child := range node.Children {
		switch c := child.(type) {
		case lexer.Token:
			typeName := tokenTypeName(c)
			switch typeName {
			case "LBRACE", "RBRACE", "SEMICOLON":
				// skip block tokens
			case "WHITESPACE":
				// Add space only if there are adjacent selector parts
				if len(parts) > 0 {
					parts = append(parts, " ")
				}
			default:
				text := c.Value
				if text != "" {
					parts = append(parts, text)
				}
			}
		case *parser.ASTNode:
			sub := e.collectSelectorTokens(c)
			parts = append(parts, sub...)
		}
	}
	return parts
}

// ============================================================================
// At-Rule (@media, @keyframes, @import, etc.)
// ============================================================================

// emitAtRule emits an at-rule: @keyword prelude { block } or @keyword prelude ;
//
// Grammar:
//
//	at_rule = AT_KEYWORD at_prelude ( block | SEMICOLON ) ;
//
// Pretty:
//
//	@media (max-width: 768px) {
//	  .btn {
//	    font-size: 14px;
//	  }
//	}
//
// Minified:
//
//	@media (max-width:768px){.btn{font-size:14px;}}
func (e *CSSEmitter) emitAtRule(node *parser.ASTNode, sb *strings.Builder, depth int) {
	e.writeIndent(sb, depth)

	hasSemicolon := false
	for _, child := range node.Children {
		switch c := child.(type) {
		case lexer.Token:
			typeName := tokenTypeName(c)
			switch typeName {
			case "AT_KEYWORD":
				sb.WriteString(c.Value)
				sb.WriteString(" ")
			case "SEMICOLON":
				hasSemicolon = true
			}
		case *parser.ASTNode:
			switch c.RuleName {
			case "at_prelude":
				e.emitAtPrelude(c, sb)
				if !e.minify {
					sb.WriteString(" ")
				}
			case "block":
				e.emitBlock(c, sb, depth)
				if !e.minify {
					sb.WriteString("\n")
				}
			}
		}
	}

	if hasSemicolon {
		sb.WriteString(";")
		if !e.minify {
			sb.WriteString("\n")
		}
	}
}

// emitAtPrelude emits the "prelude" tokens after an @-keyword.
//
// For @media rules: (max-width: 768px)
// For @import:      "styles.css"
func (e *CSSEmitter) emitAtPrelude(node *parser.ASTNode, sb *strings.Builder) {
	parts := e.collectAtPreludeTokens(node)
	// Join with spaces, but collapse multiple spaces
	sb.WriteString(strings.Join(parts, " "))
}

// collectAtPreludeTokens collects tokens from an at_prelude subtree.
func (e *CSSEmitter) collectAtPreludeTokens(node *parser.ASTNode) []string {
	var parts []string
	for _, child := range node.Children {
		switch c := child.(type) {
		case lexer.Token:
			typeName := tokenTypeName(c)
			switch typeName {
			case "WHITESPACE", "COMMENT":
				// skip
			case "STRING":
				parts = append(parts, "\""+c.Value+"\"")
			case "COLON":
				// Colon in at_prelude (e.g., @media (min-width: 768px))
				// append to previous token rather than space-separating
				if len(parts) > 0 {
					parts[len(parts)-1] += ":"
				} else {
					parts = append(parts, ":")
				}
			default:
				if c.Value != "" {
					parts = append(parts, c.Value)
				}
			}
		case *parser.ASTNode:
			sub := e.collectAtPreludeTokens(c)
			parts = append(parts, sub...)
		}
	}
	return parts
}

// ============================================================================
// Block
// ============================================================================

// emitBlock emits a { block_contents } pair with proper indentation.
//
// The LBRACE and RBRACE tokens are already in the AST; we skip them and
// emit our own structured braces.
//
// Pretty:
//
//	{
//	  property: value;
//	}
//
// Minified:
//
//	{property:value;}
func (e *CSSEmitter) emitBlock(node *parser.ASTNode, sb *strings.Builder, depth int) {
	sb.WriteString("{")
	if !e.minify {
		sb.WriteString("\n")
	}

	for _, child := range node.Children {
		switch c := child.(type) {
		case lexer.Token:
			// Skip LBRACE/RBRACE — we emit them manually above/below
		case *parser.ASTNode:
			switch c.RuleName {
			case "block_contents":
				e.emitBlockContents(c, sb, depth+1)
			default:
				e.emitAny(c, sb, depth+1)
			}
		}
	}

	e.writeIndent(sb, depth)
	sb.WriteString("}")
}

// ============================================================================
// Block Contents
// ============================================================================

// emitBlockContents emits the items inside a block.
//
// Grammar:
//
//	block_contents = { block_item } ;
//	block_item = declaration_or_nested | lattice_block_item | at_rule
func (e *CSSEmitter) emitBlockContents(node *parser.ASTNode, sb *strings.Builder, depth int) {
	for _, child := range node.Children {
		ast, ok := child.(*parser.ASTNode)
		if !ok {
			continue
		}
		e.emitBlockItem(ast, sb, depth)
	}
}

// emitBlockItem emits a single item inside a block.
func (e *CSSEmitter) emitBlockItem(node *parser.ASTNode, sb *strings.Builder, depth int) {
	// block_item wraps the actual content
	for _, child := range node.Children {
		ast, ok := child.(*parser.ASTNode)
		if !ok {
			continue
		}
		switch ast.RuleName {
		case "declaration_or_nested":
			e.emitDeclarationOrNested(ast, sb, depth)
		case "declaration":
			e.writeIndent(sb, depth)
			e.emitDeclaration(ast, sb, depth)
		case "qualified_rule":
			e.emitQualifiedRule(ast, sb, depth)
		case "at_rule":
			e.emitAtRule(ast, sb, depth)
		default:
			e.emitAny(ast, sb, depth)
		}
	}
}

// emitDeclarationOrNested handles the declaration_or_nested wrapper.
//
// The parser wraps declarations in a declaration_or_nested node. This function
// unpacks that wrapper and delegates to the appropriate emitter.
func (e *CSSEmitter) emitDeclarationOrNested(node *parser.ASTNode, sb *strings.Builder, depth int) {
	for _, child := range node.Children {
		ast, ok := child.(*parser.ASTNode)
		if !ok {
			continue
		}
		switch ast.RuleName {
		case "declaration":
			e.writeIndent(sb, depth)
			e.emitDeclaration(ast, sb, depth)
		case "qualified_rule":
			e.emitQualifiedRule(ast, sb, depth)
		default:
			e.emitAny(ast, sb, depth)
		}
	}
}

// ============================================================================
// Declaration
// ============================================================================

// emitDeclaration emits a CSS property declaration.
//
// Actual AST structure:
//
//	declaration
//	  property
//	    IDENT("color")
//	  COLON(":")
//	  value_list
//	    value
//	      IDENT("red")
//	  SEMICOLON(";")
//
// Pretty:   color: red;
// Minified: color:red;
func (e *CSSEmitter) emitDeclaration(node *parser.ASTNode, sb *strings.Builder, depth int) {
	var propName string
	var valueList *parser.ASTNode

	for _, child := range node.Children {
		switch c := child.(type) {
		case lexer.Token:
			// Property name as a direct IDENT token (fallback for non-wrapped properties)
			typeName := tokenTypeName(c)
			if typeName == "IDENT" && propName == "" {
				propName = c.Value
			}
		case *parser.ASTNode:
			switch c.RuleName {
			case "property":
				// property wraps the IDENT token
				propName = e.extractFirstIdent(c)
			case "value_list":
				valueList = c
			}
		}
	}

	if propName == "" {
		return
	}

	sb.WriteString(propName)
	sb.WriteString(":")
	if !e.minify {
		sb.WriteString(" ")
	}

	if valueList != nil {
		e.emitValueList(valueList, sb)
	}
	sb.WriteString(";")
	if !e.minify {
		sb.WriteString("\n")
	}
}

// extractFirstIdent extracts the first IDENT token value from a node subtree.
func (e *CSSEmitter) extractFirstIdent(node *parser.ASTNode) string {
	for _, child := range node.Children {
		switch c := child.(type) {
		case lexer.Token:
			if tokenTypeName(c) == "IDENT" {
				return c.Value
			}
		case *parser.ASTNode:
			if s := e.extractFirstIdent(c); s != "" {
				return s
			}
		}
	}
	return ""
}

// ============================================================================
// Value List
// ============================================================================

// emitValueList emits a value_list node — the right-hand side of a declaration.
//
// Grammar:
//
//	value_list = value { value } ;
//
// Values are emitted space-separated. Commas are preserved from COMMA tokens.
//
// Actual AST structure:
//
//	value_list
//	  value
//	    IDENT("red")   ← or HASH, NUMBER, DIMENSION, etc.
func (e *CSSEmitter) emitValueList(node *parser.ASTNode, sb *strings.Builder) {
	parts := e.collectValueParts(node)
	sb.WriteString(strings.Join(parts, " "))
}

// collectValueParts recursively collects the CSS text from a value_list subtree.
//
// Returns a slice of strings to be space-joined. Commas are kept with the
// preceding value (no space before comma, one space after).
func (e *CSSEmitter) collectValueParts(node *parser.ASTNode) []string {
	var parts []string
	for _, child := range node.Children {
		switch c := child.(type) {
		case lexer.Token:
			text := e.valueTokenText(c)
			if text == "," {
				// Attach comma to the previous part (no leading space)
				if len(parts) > 0 {
					parts[len(parts)-1] += ","
				} else {
					parts = append(parts, ",")
				}
			} else if text != "" {
				parts = append(parts, text)
			}
		case *parser.ASTNode:
			switch c.RuleName {
			case "value":
				sub := e.collectValueParts(c)
				// A value node's children are joined without internal spaces
				// only when the value contains a function call (handled below)
				if len(sub) > 0 {
					parts = append(parts, strings.Join(sub, ""))
				}
			case "function_call":
				var funcSB strings.Builder
				e.emitFunctionCall(c, &funcSB)
				if s := funcSB.String(); s != "" {
					parts = append(parts, s)
				}
			default:
				sub := e.collectValueParts(c)
				parts = append(parts, sub...)
			}
		}
	}
	return parts
}

// ============================================================================
// Function Call
// ============================================================================

// emitFunctionCall emits a function call: name(args...)
//
// Grammar:
//
//	function_call = FUNCTION function_args RPAREN ;
//
// The FUNCTION token already includes the opening paren: "rgb(" → emits "rgb("
// then the args, then ")".
func (e *CSSEmitter) emitFunctionCall(node *parser.ASTNode, sb *strings.Builder) {
	for _, child := range node.Children {
		switch c := child.(type) {
		case lexer.Token:
			typeName := tokenTypeName(c)
			switch typeName {
			case "FUNCTION":
				sb.WriteString(c.Value) // includes the "("
			case "RPAREN":
				sb.WriteString(")")
			}
		case *parser.ASTNode:
			if c.RuleName == "function_args" {
				e.emitFunctionArgs(c, sb)
			}
		}
	}
}

// emitFunctionArgs emits the arguments inside a function call.
//
// Grammar: function_args = { function_arg } ;
//
// Arguments are emitted comma-separated. Each argument may be a simple value
// token or a nested function call (FUNCTION function_args RPAREN). Nested
// function calls are joined with "" (no spaces) to produce e.g. "rgb(0,0,0)".
func (e *CSSEmitter) emitFunctionArgs(node *parser.ASTNode, sb *strings.Builder) {
	var groups []string
	var current []string

	flushGroup := func() {
		if len(current) > 0 {
			groups = append(groups, strings.Join(current, " "))
			current = nil
		}
	}

	for _, child := range node.Children {
		ast, ok := child.(*parser.ASTNode)
		if !ok {
			continue
		}
		if ast.RuleName == "function_arg" {
			if e.isFunctionArgComma(ast) {
				flushGroup()
			} else {
				// Check if this function_arg contains a nested function call
				// (structure: FUNCTION token, function_args node, RPAREN token)
				nestedText := e.emitFunctionArgNested(ast)
				if nestedText != "" {
					current = append(current, nestedText)
				} else {
					for _, argChild := range ast.Children {
						if tok, ok := argChild.(lexer.Token); ok {
							text := e.valueTokenText(tok)
							if text != "" {
								current = append(current, text)
							}
						}
					}
				}
			}
		}
	}
	flushGroup()

	sep := ", "
	if e.minify {
		sep = ","
	}
	sb.WriteString(strings.Join(groups, sep))
}

// emitFunctionArgNested checks if a function_arg node represents a nested
// function call (FUNCTION function_args RPAREN) and, if so, returns the
// serialized text with parts joined by "" (no spaces).
//
// Returns "" if the arg is not a nested function call.
func (e *CSSEmitter) emitFunctionArgNested(node *parser.ASTNode) string {
	hasFunctionToken := false
	hasFunctionArgs := false
	hasRParen := false

	for _, child := range node.Children {
		switch c := child.(type) {
		case lexer.Token:
			tn := tokenTypeName(c)
			if tn == "FUNCTION" {
				hasFunctionToken = true
			} else if tn == "RPAREN" {
				hasRParen = true
			}
		case *parser.ASTNode:
			if c.RuleName == "function_args" {
				hasFunctionArgs = true
			}
		}
	}

	if !hasFunctionToken || !hasFunctionArgs || !hasRParen {
		return ""
	}

	// It is a nested function call — serialize without spaces between parts
	var parts []string
	for _, child := range node.Children {
		switch c := child.(type) {
		case lexer.Token:
			tn := tokenTypeName(c)
			if tn == "FUNCTION" {
				parts = append(parts, c.Value) // includes "("
			} else if tn == "RPAREN" {
				parts = append(parts, ")")
			}
		case *parser.ASTNode:
			if c.RuleName == "function_args" {
				var argSB strings.Builder
				e.emitFunctionArgs(c, &argSB)
				parts = append(parts, argSB.String())
			}
		}
	}
	return strings.Join(parts, "")
}

// isFunctionArgComma reports whether a function_arg contains only a COMMA token.
func (e *CSSEmitter) isFunctionArgComma(node *parser.ASTNode) bool {
	if len(node.Children) == 1 {
		if tok, ok := node.Children[0].(lexer.Token); ok {
			return tokenTypeName(tok) == "COMMA"
		}
	}
	return false
}

// ============================================================================
// Token Text Helpers
// ============================================================================

// valueTokenText returns the CSS text for a value token.
//
// Most tokens emit their Value directly. COMMA emits ",".
// Structural tokens (braces, semicolons) are skipped.
// STRING tokens get re-quoted.
func (e *CSSEmitter) valueTokenText(tok lexer.Token) string {
	typeName := tokenTypeName(tok)
	switch typeName {
	case "WHITESPACE", "COMMENT", "CDO", "CDC":
		return "" // skip
	case "LBRACE", "RBRACE":
		return "" // skip block delimiters
	case "SEMICOLON":
		return "" // skip — declarations emit their own semicolons
	case "COLON":
		return ":" // colon inside selectors or calc()
	case "COMMA":
		return ","
	case "STRING":
		return "\"" + tok.Value + "\""
	}
	return tok.Value
}

// ============================================================================
// Generic Fallback
// ============================================================================

// emitChildrenAny emits all children of a node recursively.
// Used as a fallback for rules we don't have specific handlers for.
func (e *CSSEmitter) emitChildrenAny(node *parser.ASTNode, sb *strings.Builder, depth int) {
	for _, child := range node.Children {
		switch c := child.(type) {
		case *parser.ASTNode:
			e.emitAny(c, sb, depth)
		case lexer.Token:
			text := e.valueTokenText(c)
			if text != "" {
				sb.WriteString(text)
			}
		}
	}
}

// ============================================================================
// Indentation
// ============================================================================

// writeIndent writes depth * indent spaces to the builder.
// No-op when minifying.
func (e *CSSEmitter) writeIndent(sb *strings.Builder, depth int) {
	if e.minify {
		return
	}
	for i := 0; i < depth; i++ {
		sb.WriteString(e.indent)
	}
}
