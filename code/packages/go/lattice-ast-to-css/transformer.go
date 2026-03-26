package latticeasttocss

// transformer.go — transforms a Lattice AST into a clean CSS AST.
//
// This is the core of the Lattice-to-CSS compiler. It takes a Lattice AST
// (containing both CSS and Lattice nodes) and produces a CSS-only AST by
// expanding all Lattice constructs.
//
// # Three-Pass Architecture
//
// Pass 1 — Symbol Collection:
//   Walk the top-level AST and collect:
//   - Variable declarations → global variable registry (ScopeChain)
//   - Mixin definitions → mixins map
//   - Function definitions → functions map
//   - @use directives → skip (module resolution not implemented yet)
//   All definition nodes are removed from the AST (they produce no CSS output).
//
// Pass 2 — Expansion:
//   Recursively walk remaining nodes with a scope chain:
//   - Replace VARIABLE tokens with their resolved values
//   - Expand @include directives by cloning mixin bodies
//   - Evaluate @if / @else control flow (keep matching branch, discard rest)
//   - Unroll @for and @each loops
//   - Call @function definitions and replace call sites with return values
//   After this pass, the AST contains only pure CSS nodes.
//
// Pass 3 — Cleanup:
//   Remove any empty blocks or rules that resulted from transformation.
//
// # Why Three Passes?
//
// Mixins and functions can be defined AFTER they are used:
//
//   .btn { @include button(red); }   ← used first
//   @mixin button($bg) { ... }       ← defined later
//
// Pass 1 collects all definitions up-front, so Pass 2 can resolve them
// regardless of source order — just like how JavaScript hoists function
// declarations to the top of their scope.
//
// # Cycle Detection
//
// Mixin and function expansion tracks a call stack (a slice of names).
// If a name appears twice in the stack, a CircularReferenceError is raised:
//
//   @mixin a { @include b; }
//   @mixin b { @include a; }    ← Circular mixin: a → b → a

import (
	"fmt"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

// MaxWhileIterations is the default maximum number of @while loop iterations.
// Prevents infinite loops from consuming all CPU at compile time.
const MaxWhileIterations = 1000

// ============================================================================
// Mixin and Function Definition Records
// ============================================================================

// MixinDef stores the compiled definition of a @mixin.
//
// After Pass 1, every @mixin in the source is represented by one MixinDef.
// During Pass 2, @include calls clone the Body and evaluate it in a new scope.
type MixinDef struct {
	Name     string
	Params   []string               // parameter names in declaration order
	Defaults map[string]interface{} // AST nodes for params with default values
	Body     *parser.ASTNode        // the { ... } block node
}

// FunctionDef stores the compiled definition of a @function.
//
// Same structure as MixinDef, but functions return values via @return rather
// than emitting CSS declarations.
type FunctionDef struct {
	Name     string
	Params   []string
	Defaults map[string]interface{}
	Body     *parser.ASTNode // the function_body node
}

// ============================================================================
// ReturnSignal — internal control flow for @return
// ============================================================================

// returnSignal is a panic value used to implement @return in functions.
//
// Go has no first-class exception handling, but we can simulate
// non-local return using panic/recover. When the evaluator hits a @return
// directive, it panics with a returnSignal. The function evaluator recovers
// the signal and uses its value as the function's return value.
//
// This is an internal implementation detail — it never escapes the package.
// External callers always see normal Go errors, never raw panics.
type returnSignal struct {
	value LatticeValue
}

// ============================================================================
// CSS Built-in Functions
// ============================================================================

// cssFunctions is the set of CSS built-in function names that should NOT be
// resolved as Lattice functions.
//
// When a function_call node uses one of these names, it is passed through
// unchanged (arguments are still expanded for variable substitution).
// Any function name NOT in this set that is @function-defined in the Lattice
// source will be called and replaced with its return value.
var cssFunctions = map[string]bool{
	// Color functions
	"rgb": true, "rgba": true, "hsl": true, "hsla": true, "hwb": true,
	"lab": true, "lch": true, "oklch": true, "oklab": true,
	"color": true, "color-mix": true,
	// Math functions
	"calc": true, "min": true, "max": true, "clamp": true,
	"abs": true, "sign": true, "round": true, "mod": true, "rem": true,
	"sin": true, "cos": true, "tan": true, "asin": true, "acos": true,
	"atan": true, "atan2": true, "pow": true, "sqrt": true,
	"hypot": true, "log": true, "exp": true,
	// CSS variable functions
	"var": true, "env": true,
	// URL/format
	"url": true, "format": true, "local": true,
	// Gradient functions
	"linear-gradient": true, "radial-gradient": true, "conic-gradient": true,
	"repeating-linear-gradient": true, "repeating-radial-gradient": true,
	"repeating-conic-gradient": true,
	// Misc
	"counter": true, "counters": true, "attr": true, "element": true,
	// Transform functions
	"translate": true, "translateX": true, "translateY": true, "translateZ": true,
	"rotate": true, "rotateX": true, "rotateY": true, "rotateZ": true,
	"scale": true, "scaleX": true, "scaleY": true, "scaleZ": true,
	"skew": true, "skewX": true, "skewY": true,
	"matrix": true, "matrix3d": true, "perspective": true,
	// Easing
	"cubic-bezier": true, "steps": true,
	// Shape
	"path": true, "polygon": true, "circle": true, "ellipse": true, "inset": true,
	// Image
	"image-set": true, "cross-fade": true,
	// Grid
	"fit-content": true, "minmax": true, "repeat": true,
	// Filter
	"blur": true, "brightness": true, "contrast": true, "drop-shadow": true,
	"grayscale": true, "hue-rotate": true, "invert": true, "opacity": true,
	"saturate": true, "sepia": true,
}

// isCSSFunction checks if a FUNCTION token name is a CSS built-in.
// FUNCTION tokens include the opening paren: "rgb(" → strip the paren → "rgb".
func isCSSFunction(name string) bool {
	clean := name
	if len(clean) > 0 && clean[len(clean)-1] == '(' {
		clean = clean[:len(clean)-1]
	}
	return cssFunctions[clean]
}

func levenshteinDistance(left, right string) int {
	if left == right {
		return 0
	}
	if left == "" {
		return len(right)
	}
	if right == "" {
		return len(left)
	}

	previous := make([]int, len(right)+1)
	for i := range previous {
		previous[i] = i
	}

	for i, leftRune := range left {
		current := make([]int, len(right)+1)
		current[0] = i + 1
		for j, rightRune := range right {
			insertion := current[j] + 1
			deletion := previous[j+1] + 1
			substitution := previous[j]
			if leftRune != rightRune {
				substitution++
			}
			current[j+1] = minInt(insertion, deletion, substitution)
		}
		previous = current
	}

	return previous[len(previous)-1]
}

func closestName(name string, candidates []string) string {
	best := ""
	bestDistance := -1

	for _, candidate := range candidates {
		distance := levenshteinDistance(name, candidate)
		if bestDistance == -1 || distance < bestDistance {
			best = candidate
			bestDistance = distance
		}
	}

	if bestDistance == -1 {
		return ""
	}

	threshold := len(name) / 3
	if threshold < 2 {
		threshold = 2
	}
	if bestDistance > threshold {
		return ""
	}
	return best
}

func minInt(values ...int) int {
	best := values[0]
	for _, value := range values[1:] {
		if value < best {
			best = value
		}
	}
	return best
}

// ============================================================================
// Transformer
// ============================================================================

// LatticeTransformer transforms a Lattice AST into a clean CSS AST.
//
// Usage:
//
//	t := NewLatticeTransformer()
//	cssAST, err := t.Transform(latticeAST)
//	// cssAST now contains only CSS nodes; pass to CSSEmitter.Emit()
type LatticeTransformer struct {
	variables     *ScopeChain            // global variable scope
	mixins        map[string]MixinDef    // collected mixin definitions
	functions     map[string]FunctionDef // collected function definitions
	mixinStack    []string               // current mixin call chain (for cycle detection)
	functionStack []string               // current function call chain

	// Lattice v2 fields
	maxWhileIterations int                    // max @while iterations (default: 1000)
	extendMap          map[string][]string    // @extend target → extending selectors
	atRootRules        []interface{}          // hoisted @at-root rules
	contentBlockStack  []*parser.ASTNode      // @content block stack for @include
	contentScopeStack  []*ScopeChain          // caller scope stack for @content evaluation
}

// NewLatticeTransformer creates a new transformer with empty registries.
func NewLatticeTransformer() *LatticeTransformer {
	return &LatticeTransformer{
		variables:          NewScopeChain(nil),
		mixins:             make(map[string]MixinDef),
		functions:          make(map[string]FunctionDef),
		maxWhileIterations: MaxWhileIterations,
		extendMap:          make(map[string][]string),
	}
}

// Transform runs the three-pass pipeline on a Lattice AST.
//
// Input:  root ASTNode from lattice-parser (mix of CSS and Lattice nodes)
// Output: root ASTNode containing only CSS nodes
//
// The input AST is modified in-place. Do not reuse it after calling Transform.
func (t *LatticeTransformer) Transform(ast *parser.ASTNode) (*parser.ASTNode, error) {
	// Pass 1: Collect symbols
	t.collectSymbols(ast)

	// Pass 2: Expand all Lattice constructs
	result := t.expandNode(ast, t.variables)
	resultNode, ok := result.(*parser.ASTNode)
	if !ok {
		return ast, nil
	}

	// Pass 3: Cleanup — remove empty blocks/rules
	t.cleanup(resultNode)

	return resultNode, nil
}

// ============================================================================
// Pass 1: Symbol Collection
// ============================================================================

// collectSymbols walks top-level AST rules and registers definitions.
//
// Variable declarations, mixin definitions, function definitions, and @use
// directives are removed from the AST children (they produce no CSS output).
// Everything else is kept for Pass 2 expansion.
func (t *LatticeTransformer) collectSymbols(ast *parser.ASTNode) {
	if ast == nil {
		return
	}

	newChildren := make([]interface{}, 0, len(ast.Children))

	for _, child := range ast.Children {
		astChild, ok := child.(*parser.ASTNode)
		if !ok {
			newChildren = append(newChildren, child)
			continue
		}

		if astChild.RuleName != "rule" {
			newChildren = append(newChildren, child)
			continue
		}

		// rule = lattice_rule | at_rule | qualified_rule
		if len(astChild.Children) == 0 {
			newChildren = append(newChildren, child)
			continue
		}

		inner, ok := astChild.Children[0].(*parser.ASTNode)
		if !ok {
			newChildren = append(newChildren, child)
			continue
		}

		// Path 1: ideal grammar match — lattice_rule
		if inner.RuleName == "lattice_rule" {
			if len(inner.Children) == 0 {
				continue
			}
			latticeChild, ok := inner.Children[0].(*parser.ASTNode)
			if !ok {
				continue
			}
			switch latticeChild.RuleName {
			case "variable_declaration":
				t.collectVariable(latticeChild)
			case "mixin_definition":
				t.collectMixin(latticeChild)
			case "function_definition":
				t.collectFunction(latticeChild)
			case "use_directive":
				// skip @use
			default:
				newChildren = append(newChildren, child)
			}
			continue
		}

		// Path 2: fallback — Lattice constructs parsed as at_rule
		// This happens when the packrat parser resolves ambiguity differently.
		// Detect @mixin, @function, and $variable declarations.
		if inner.RuleName == "at_rule" {
			keyword := t.atRuleKeyword(inner)
			switch keyword {
			case "@mixin":
				t.collectMixinFromAtRule(inner)
				// Remove from output — mixin definitions produce no CSS
				continue
			case "@function":
				t.collectFunctionFromAtRule(inner)
				// Remove from output
				continue
			case "@use":
				// Skip @use
				continue
			}
		}

		newChildren = append(newChildren, child)
	}

	ast.Children = newChildren
}

// collectVariable extracts a variable name and its value_list from a
// variable_declaration node and stores it in the global scope.
//
// Grammar: variable_declaration = VARIABLE COLON value_list { variable_flag } SEMICOLON ;
//
// Lattice v2 adds !default and !global flags:
//   - !default: only set if not already defined anywhere in scope chain
//   - !global: set in the root (global) scope
//   - Both can appear together: check global scope; if not defined, set globally
func (t *LatticeTransformer) collectVariable(node *parser.ASTNode) {
	name, valueNode, isDefault, isGlobal := parseVariableDeclaration(node)

	if name != "" && valueNode != nil {
		t.setVariableWithFlags(t.variables, name, valueNode, isDefault, isGlobal)
	}
}

// parseVariableDeclaration extracts name, value, and flags from a variable_declaration.
func parseVariableDeclaration(node *parser.ASTNode) (string, *parser.ASTNode, bool, bool) {
	var name string
	var valueNode *parser.ASTNode
	isDefault := false
	isGlobal := false

	for _, child := range node.Children {
		switch c := child.(type) {
		case lexer.Token:
			tn := tokenTypeName(c)
			if tn == "VARIABLE" {
				name = c.Value
			} else if tn == "BANG_DEFAULT" {
				isDefault = true
			} else if tn == "BANG_GLOBAL" {
				isGlobal = true
			}
		case *parser.ASTNode:
			if c.RuleName == "value_list" {
				valueNode = c
			} else if c.RuleName == "variable_flag" {
				for _, fc := range c.Children {
					if tok, ok := fc.(lexer.Token); ok {
						ft := tokenTypeName(tok)
						if ft == "BANG_DEFAULT" {
							isDefault = true
						} else if ft == "BANG_GLOBAL" {
							isGlobal = true
						}
					}
				}
			}
		}
	}
	return name, valueNode, isDefault, isGlobal
}

// setVariableWithFlags sets a variable respecting !default and !global flags.
func (t *LatticeTransformer) setVariableWithFlags(scope *ScopeChain, name string, value interface{}, isDefault, isGlobal bool) {
	if isDefault && isGlobal {
		// Check global scope only; if not defined there, set it globally
		root := scope
		for root.parent != nil {
			root = root.parent
		}
		if _, ok := root.Get(name); !ok {
			scope.SetGlobal(name, value)
		}
	} else if isDefault {
		// Only set if not already defined anywhere
		if !scope.Has(name) {
			scope.Set(name, value)
		}
	} else if isGlobal {
		// Always set in global scope
		scope.SetGlobal(name, value)
	} else {
		scope.Set(name, value)
	}
}

// collectMixin extracts a mixin's name, parameters, defaults, and body.
//
// Grammar: mixin_definition = "@mixin" FUNCTION [ mixin_params ] RPAREN block ;
//
// Note: the FUNCTION token for "button(" includes the opening paren.
// We strip it with strings.TrimSuffix to get the bare name "button".
func (t *LatticeTransformer) collectMixin(node *parser.ASTNode) {
	var name string
	var params []string
	defaults := make(map[string]interface{})
	var body *parser.ASTNode

	for _, child := range node.Children {
		switch c := child.(type) {
		case lexer.Token:
			if tokenTypeName(c) == "FUNCTION" {
				// Strip the trailing "(" from the FUNCTION token
				name = trimTrailingParen(c.Value)
			} else if tokenTypeName(c) == "IDENT" && name == "" {
				name = c.Value
			}
		case *parser.ASTNode:
			switch c.RuleName {
			case "mixin_params":
				params, defaults = extractParams(c)
			case "block":
				body = c
			}
		}
	}

	if name != "" && body != nil {
		t.mixins[name] = MixinDef{
			Name:     name,
			Params:   params,
			Defaults: defaults,
			Body:     body,
		}
	}
}

// collectFunction extracts a function's name, parameters, defaults, and body.
//
// Grammar: function_definition = "@function" FUNCTION [ mixin_params ] RPAREN function_body ;
func (t *LatticeTransformer) collectFunction(node *parser.ASTNode) {
	var name string
	var params []string
	defaults := make(map[string]interface{})
	var body *parser.ASTNode

	for _, child := range node.Children {
		switch c := child.(type) {
		case lexer.Token:
			if tokenTypeName(c) == "FUNCTION" {
				name = trimTrailingParen(c.Value)
			} else if tokenTypeName(c) == "IDENT" && name == "" {
				name = c.Value
			}
		case *parser.ASTNode:
			switch c.RuleName {
			case "mixin_params":
				params, defaults = extractParams(c)
			case "function_body":
				body = c
			}
		}
	}

	if name != "" && body != nil {
		t.functions[name] = FunctionDef{
			Name:     name,
			Params:   params,
			Defaults: defaults,
			Body:     body,
		}
	}
}

// extractParams parses a mixin_params node into a list of parameter names
// and a map of default values.
//
// Grammar:
//
//	mixin_params = mixin_param { COMMA mixin_param } ;
//	mixin_param = VARIABLE [ COLON value_list ] ;
func extractParams(node *parser.ASTNode) ([]string, map[string]interface{}) {
	params := []string{}
	defaults := make(map[string]interface{})

	for _, child := range node.Children {
		param, ok := child.(*parser.ASTNode)
		if !ok || param.RuleName != "mixin_param" {
			continue
		}

		var paramName string
		var defaultVal *parser.ASTNode

		for _, pc := range param.Children {
			switch c := pc.(type) {
			case lexer.Token:
				if tokenTypeName(c) == "VARIABLE" {
					paramName = c.Value
				}
			case *parser.ASTNode:
				if c.RuleName == "value_list" || c.RuleName == "mixin_value_list" {
					defaultVal = c
				}
			}
		}

		if paramName != "" {
			params = append(params, paramName)
			if defaultVal != nil {
				defaults[paramName] = defaultVal
			}
		}
	}

	return params, defaults
}

// atRuleKeyword returns the AT_KEYWORD value from an at_rule node, or "" if none.
func (t *LatticeTransformer) atRuleKeyword(node *parser.ASTNode) string {
	for _, child := range node.Children {
		if tok, ok := child.(lexer.Token); ok && tokenTypeName(tok) == "AT_KEYWORD" {
			return tok.Value
		}
	}
	return ""
}

// collectMixinFromAtRule collects a @mixin that was parsed as a generic at_rule.
//
// This happens when the mixin has multiple parameters (the grammar's packrat
// parser resolves the ambiguity differently for complex parameter lists).
//
// The at_rule structure for "@mixin bordered($w: 1px, $s: solid) { ... }" is:
//
//	at_rule
//	  AT_KEYWORD="@mixin"
//	  at_prelude
//	    at_prelude_token
//	      function_in_prelude
//	        FUNCTION="bordered("
//	        at_prelude_tokens   ← flat list: VARIABLE COLON VALUE COMMA ...
//	        RPAREN=")"
//	  block
func (t *LatticeTransformer) collectMixinFromAtRule(node *parser.ASTNode) {
	name, params, defaults, body := t.parseDefinitionFromAtRule(node)
	if name != "" && body != nil {
		t.mixins[name] = MixinDef{
			Name:     name,
			Params:   params,
			Defaults: defaults,
			Body:     body,
		}
	}
}

// collectFunctionFromAtRule collects a @function that was parsed as a generic at_rule.
func (t *LatticeTransformer) collectFunctionFromAtRule(node *parser.ASTNode) {
	name, params, defaults, body := t.parseDefinitionFromAtRule(node)
	if name != "" && body != nil {
		t.functions[name] = FunctionDef{
			Name:     name,
			Params:   params,
			Defaults: defaults,
			Body:     body,
		}
	}
}

// parseDefinitionFromAtRule extracts name, params, defaults, and body from
// a @mixin or @function that was parsed as a generic at_rule.
//
// The name comes from the FUNCTION token in the at_prelude (e.g., "bordered(").
// The params come from the at_prelude_tokens inside the function_in_prelude.
// The body is the block node.
func (t *LatticeTransformer) parseDefinitionFromAtRule(node *parser.ASTNode) (string, []string, map[string]interface{}, *parser.ASTNode) {
	var name string
	var params []string
	defaults := make(map[string]interface{})
	var body *parser.ASTNode

	for _, child := range node.Children {
		switch c := child.(type) {
		case *parser.ASTNode:
			switch c.RuleName {
			case "at_prelude":
				name, params, defaults = t.extractDefinitionPrelude(c)
			case "block":
				body = c
			case "function_body":
				// @function uses function_body instead of block
				body = c
			}
		}
	}

	return name, params, defaults, body
}

// extractDefinitionPrelude parses the at_prelude of a @mixin/@function at_rule.
//
// Walks the prelude tree to find:
//   - FUNCTION token → the definition name (e.g., "bordered(")
//   - at_prelude_tokens → flat token list containing params and defaults
//
// The param tokens follow the pattern:
//
//	VARIABLE COLON value_token [ COMMA VARIABLE COLON value_token ... ]
func (t *LatticeTransformer) extractDefinitionPrelude(node *parser.ASTNode) (string, []string, map[string]interface{}) {
	var name string
	var rawTokens []lexer.Token

	t.walkDefinitionPrelude(node, &name, &rawTokens)

	if name == "" {
		return "", nil, nil
	}

	params, defaults := t.parseParamTokens(rawTokens)
	return name, params, defaults
}

// walkDefinitionPrelude recursively walks an at_prelude to extract the function
// name and raw param tokens.
func (t *LatticeTransformer) walkDefinitionPrelude(node *parser.ASTNode, name *string, rawTokens *[]lexer.Token) {
	for _, child := range node.Children {
		switch c := child.(type) {
		case lexer.Token:
			typeName := tokenTypeName(c)
			if typeName == "FUNCTION" {
				*name = trimTrailingParen(c.Value)
			} else if typeName != "RPAREN" && typeName != "AT_KEYWORD" {
				*rawTokens = append(*rawTokens, c)
			}
		case *parser.ASTNode:
			t.walkDefinitionPrelude(c, name, rawTokens)
		}
	}
}

// parseParamTokens converts a flat list of tokens (from at_prelude_tokens)
// into an ordered params list and defaults map.
//
// Tokens follow this pattern:
//
//	VARIABLE COLON value_token [ COMMA VARIABLE COLON value_token ... ]
//
// For params without defaults:
//
//	VARIABLE [ COMMA VARIABLE ... ]
func (t *LatticeTransformer) parseParamTokens(tokens []lexer.Token) ([]string, map[string]interface{}) {
	params := []string{}
	defaults := make(map[string]interface{})

	i := 0
	for i < len(tokens) {
		tok := tokens[i]
		typeName := tokenTypeName(tok)

		if typeName == "COMMA" {
			i++
			continue
		}

		if typeName == "VARIABLE" {
			paramName := tok.Value
			// Check if next token is COLON (has default)
			if i+1 < len(tokens) && tokenTypeName(tokens[i+1]) == "COLON" {
				// Default value follows the colon
				if i+2 < len(tokens) {
					defaultTok := tokens[i+2]
					// Build a value_list node wrapping the default token
					defaultNode := &parser.ASTNode{
						RuleName: "value_list",
						Children: []interface{}{
							&parser.ASTNode{
								RuleName: "value",
								Children: []interface{}{defaultTok},
							},
						},
					}
					params = append(params, paramName)
					defaults[paramName] = defaultNode
					i += 3
					continue
				}
			}
			params = append(params, paramName)
			i++
			continue
		}

		i++
	}

	return params, defaults
}

// ============================================================================
// Pass 2: Expansion
// ============================================================================

// expandNode recursively expands a single AST node.
//
// For CSS nodes (qualified_rule, declaration, etc.), it recursively expands
// all children. For Lattice nodes (block_item, value, function_call), it
// performs the appropriate substitution or expansion.
func (t *LatticeTransformer) expandNode(node interface{}, scope *ScopeChain) interface{} {
	switch n := node.(type) {
	case lexer.Token:
		// Tokens are leaf nodes. Only VARIABLE tokens need substitution.
		if tokenTypeName(n) == "VARIABLE" {
			return t.substituteVariable(n, scope)
		}
		return node

	case *parser.ASTNode:
		if n == nil {
			return nil
		}

		switch n.RuleName {
		case "block":
			return t.expandBlock(n, scope)
		case "block_contents":
			return t.expandBlockContents(n, scope)
		case "block_item":
			return t.expandBlockItem(n, scope)
		case "value_list":
			return t.expandValueList(n, scope)
		case "value":
			return t.expandValue(n, scope)
		case "function_call":
			return t.expandFunctionCall(n, scope)
		case "function_args":
			return t.expandChildren(n, scope)
		case "function_arg":
			return t.expandChildren(n, scope)
		// Lattice v2: resolve variables in selector positions
		case "compound_selector", "simple_selector", "class_selector":
			return t.expandSelectorWithVars(n, scope)
		}

		// Default: expand all children
		return t.expandChildren(n, scope)
	}

	return node
}

// expandChildren expands all children of a node in-place.
func (t *LatticeTransformer) expandChildren(node *parser.ASTNode, scope *ScopeChain) *parser.ASTNode {
	if node == nil {
		return nil
	}

	newChildren := make([]interface{}, 0, len(node.Children))
	for _, child := range node.Children {
		expanded := t.expandNode(child, scope)
		if expanded != nil {
			newChildren = append(newChildren, expanded)
		}
	}
	node.Children = newChildren
	return node
}

// substituteVariable replaces a VARIABLE token with its value from scope.
//
// If the variable holds a value_list AST node, we deep-clone and expand it.
// If it holds a LatticeValue, we create a synthetic token with the CSS text.
// If it's not found, we raise UndefinedVariableError.
func (t *LatticeTransformer) substituteVariable(tok lexer.Token, scope *ScopeChain) interface{} {
	name := tok.Value
	val, ok := scope.Get(name)
	if !ok {
		panic(NewUndefinedVariableError(name, tok.Line, tok.Column))
	}

	// If the value is an AST node (e.g., value_list), deep-clone and expand it
	if ast, ok := val.(*parser.ASTNode); ok {
		cloned := deepCloneAST(ast)
		return t.expandNode(cloned, scope)
	}

	// If it's a LatticeValue, convert to a synthetic IDENT token
	if lv, ok := val.(LatticeValue); ok {
		cssText := valueToCSSText(lv)
		return makeSyntheticToken(cssText, tok)
	}

	// Raw token — return unchanged
	if rawTok, ok := val.(lexer.Token); ok {
		return rawTok
	}

	return tok
}

// expandBlock expands a block, creating a new child scope.
//
// In Lattice, every { } block is a new scope. Variables declared inside
// the block are local to it and shadow any outer variables.
func (t *LatticeTransformer) expandBlock(node *parser.ASTNode, scope *ScopeChain) *parser.ASTNode {
	childScope := scope.Child()
	return t.expandChildren(node, childScope)
}

// expandBlockContents handles block_contents, splicing control-flow results.
//
// block_contents = { block_item } ;
//
// Control flow (@if, @for, @each) and @include can return multiple items
// that need to be spliced into the parent children list. We handle this by
// collecting expanded items and flattening them.
func (t *LatticeTransformer) expandBlockContents(node *parser.ASTNode, scope *ScopeChain) *parser.ASTNode {
	newChildren := make([]interface{}, 0, len(node.Children))

	for _, child := range node.Children {
		expanded := t.expandBlockItemInner(child, scope)
		switch e := expanded.(type) {
		case []interface{}:
			newChildren = append(newChildren, e...)
		case nil:
			// dropped (e.g., variable_declaration)
		default:
			if expanded != nil {
				newChildren = append(newChildren, expanded)
			}
		}
	}

	node.Children = newChildren
	return node
}

// expandBlockItemInner processes a single child of block_contents.
//
// Returns one of:
//   - nil: item was dropped (e.g., variable_declaration)
//   - []interface{}: items to splice in (from @include or control flow)
//   - interface{}: a single expanded item
//
// Lattice constructs (@include, @if, @for, @each, variable declarations) are
// always parsed by the grammar as lattice_block_item children of block_item.
// Other children (declarations, nested rules, CSS at-rules) are expanded
// recursively.
func (t *LatticeTransformer) expandBlockItemInner(child interface{}, scope *ScopeChain) interface{} {
	ast, ok := child.(*parser.ASTNode)
	if !ok {
		return child
	}

	if ast.RuleName != "block_item" {
		return t.expandChildren(ast, scope)
	}

	// block_item = lattice_block_item | at_rule | declaration_or_nested
	if len(ast.Children) == 0 {
		return ast
	}

	inner, ok := ast.Children[0].(*parser.ASTNode)
	if !ok {
		return t.expandChildren(ast, scope)
	}

	// Case 1: explicit lattice_block_item (ideal grammar path)
	if inner.RuleName == "lattice_block_item" {
		result := t.expandLatticeBlockItem(inner, scope)
		switch r := result.(type) {
		case nil:
			return nil
		case []interface{}:
			return r
		default:
			ast.Children = []interface{}{inner}
			inner.Children = []interface{}{result}
			return ast
		}
	}

	return t.expandChildren(ast, scope)
}

// expandBlockItem expands a single block_item node.
func (t *LatticeTransformer) expandBlockItem(node *parser.ASTNode, scope *ScopeChain) interface{} {
	if len(node.Children) == 0 {
		return node
	}

	inner, ok := node.Children[0].(*parser.ASTNode)
	if !ok {
		return t.expandChildren(node, scope)
	}

	if inner.RuleName == "lattice_block_item" {
		result := t.expandLatticeBlockItem(inner, scope)
		switch r := result.(type) {
		case nil:
			return nil
		case []interface{}:
			return r
		default:
			node.Children = []interface{}{inner}
			inner.Children = []interface{}{result}
			return node
		}
	}

	return t.expandChildren(node, scope)
}

// expandLatticeBlockItem handles lattice_block_item constructs.
//
// lattice_block_item = variable_declaration | include_directive | lattice_control
//                    | content_directive | at_root_directive | extend_directive ;
//
// Lattice v2 adds: content_directive, at_root_directive, extend_directive.
func (t *LatticeTransformer) expandLatticeBlockItem(node *parser.ASTNode, scope *ScopeChain) interface{} {
	if len(node.Children) == 0 {
		return node
	}

	inner, ok := node.Children[0].(*parser.ASTNode)
	if !ok {
		return node
	}

	switch inner.RuleName {
	case "variable_declaration":
		t.expandVariableDeclaration(inner, scope)
		return nil // removed from output

	case "include_directive":
		return t.expandInclude(inner, scope)

	case "lattice_control":
		return t.expandControl(inner, scope)

	case "content_directive":
		return t.expandContent(inner, scope)

	case "at_root_directive":
		return t.expandAtRoot(inner, scope)

	case "extend_directive":
		t.collectExtend(inner, scope)
		return nil // removed from output
	}

	return t.expandChildren(node, scope)
}

// expandVariableDeclaration processes a variable_declaration inside a block.
//
// Sets the variable in the current scope. The declaration node is removed
// from CSS output (it has no CSS equivalent).
//
// Lattice v2: handles !default and !global flags.
func (t *LatticeTransformer) expandVariableDeclaration(node *parser.ASTNode, scope *ScopeChain) {
	name, valueNode, isDefault, isGlobal := parseVariableDeclaration(node)

	if name != "" && valueNode != nil {
		// Expand the value first (it might reference other variables)
		expanded := t.expandNode(deepCloneAST(valueNode), scope)

		// Try to evaluate as an expression (e.g. $i + 1 → LatticeNumber(2)).
		// This is critical for @while loops: without it, $i: $i + 1
		// stores unevaluated tokens instead of the computed number, causing
		// the loop condition to never change and looping forever.
		func() {
			defer func() { recover() }() // silently ignore evaluation failures
			evaluator := NewExpressionEvaluator(scope)
			evaluated := evaluator.Evaluate(expanded)
			if evaluated != nil {
				// Store the LatticeValue directly so substituteVariable can
				// convert it via the LatticeValue type assertion branch.
				expanded = evaluated
			}
		}()

		t.setVariableWithFlags(scope, name, expanded, isDefault, isGlobal)
	}
}

// expandValueList expands variables within a value_list.
//
// value_list = value { value } ;
//
// If expanding a value returns another value_list (from variable substitution),
// we splice its children in rather than nesting value_lists.
func (t *LatticeTransformer) expandValueList(node *parser.ASTNode, scope *ScopeChain) *parser.ASTNode {
	newChildren := make([]interface{}, 0, len(node.Children))

	for _, child := range node.Children {
		expanded := t.expandNode(child, scope)
		if expanded == nil {
			continue
		}
		// If a variable expands to a value_list, splice it in
		if ast, ok := expanded.(*parser.ASTNode); ok && ast.RuleName == "value_list" {
			newChildren = append(newChildren, ast.Children...)
		} else {
			newChildren = append(newChildren, expanded)
		}
	}

	node.Children = newChildren
	return node
}

// expandValue expands a single value node, substituting variables.
func (t *LatticeTransformer) expandValue(node *parser.ASTNode, scope *ScopeChain) interface{} {
	if len(node.Children) == 0 {
		return node
	}

	// If it's a single VARIABLE token, substitute it
	if len(node.Children) == 1 {
		if tok, ok := node.Children[0].(lexer.Token); ok && tokenTypeName(tok) == "VARIABLE" {
			result := t.substituteVariable(tok, scope)
			if ast, ok := result.(*parser.ASTNode); ok {
				return ast // return the value_list directly
			}
			node.Children = []interface{}{result}
			return node
		}
	}

	return t.expandChildren(node, scope)
}

// expandFunctionCall expands a function_call node.
//
// CSS built-in functions (rgb, calc, etc.) are passed through with their
// arguments expanded. Lattice functions are evaluated and replaced with
// their return values. Unknown functions are passed through.
func (t *LatticeTransformer) expandFunctionCall(node *parser.ASTNode, scope *ScopeChain) interface{} {
	// Find the FUNCTION token to get the function name
	var funcName string
	for _, child := range node.Children {
		if tok, ok := child.(lexer.Token); ok && tokenTypeName(tok) == "FUNCTION" {
			funcName = trimTrailingParen(tok.Value)
			break
		}
	}

	// URL_TOKEN — single child, pass through
	if funcName == "" {
		return t.expandChildren(node, scope)
	}

	// User-defined @function takes highest priority (Sass behavior).
	// A user-defined @function named "scale" overrides the CSS transform "scale".
	if _, ok := t.functions[funcName]; ok {
		return t.evaluateFunctionCall(funcName, node, scope)
	}

	// CSS built-in that is NOT also a Lattice built-in — pass through
	if isCSSFunction(funcName) && !IsBuiltinFunction(funcName) {
		return t.expandChildren(node, scope)
	}

	// Lattice v2 built-in function
	if IsBuiltinFunction(funcName) {
		return t.evaluateBuiltinFunctionCall(funcName, node, scope)
	}

	// CSS built-in that overlaps with Lattice built-in names
	if isCSSFunction(funcName) {
		return t.expandChildren(node, scope)
	}

	// Unknown function — pass through (might be a CSS function we don't know)
	return t.expandChildren(node, scope)
}

// ============================================================================
// @include Expansion
// ============================================================================

// expandInclude expands an @include directive.
//
// Grammar:
//
//	include_directive = "@include" FUNCTION include_args RPAREN ( SEMICOLON | block )
//	                  | "@include" IDENT ( SEMICOLON | block ) ;
//
// Returns a []interface{} of the expanded block_contents items from the mixin body.
// These items are spliced into the parent block_contents.
func (t *LatticeTransformer) expandInclude(node *parser.ASTNode, scope *ScopeChain) []interface{} {
	var mixinName string
	var mixinToken lexer.Token
	var hasMixinToken bool
	var argsNode *parser.ASTNode
	var contentBlock *parser.ASTNode

	for _, child := range node.Children {
		switch c := child.(type) {
		case lexer.Token:
			switch tokenTypeName(c) {
			case "FUNCTION":
				mixinName = trimTrailingParen(c.Value)
				mixinToken = c
				hasMixinToken = true
			case "IDENT":
				if mixinName == "" {
					mixinName = c.Value
					mixinToken = c
					hasMixinToken = true
				}
			}
		case *parser.ASTNode:
			if c.RuleName == "include_args" {
				argsNode = c
			} else if c.RuleName == "block" {
				// Lattice v2: trailing block is the @content block
				contentBlock = c
			}
		}
	}

	if mixinName == "" {
		return nil
	}

	mixinDef, ok := t.mixins[mixinName]
	if !ok {
		line, col := 0, 0
		if hasMixinToken {
			line = mixinToken.Line
			col = mixinToken.Column
		}
		candidates := make([]string, 0, len(t.mixins))
		for name := range t.mixins {
			candidates = append(candidates, name)
		}
		panic(NewUndefinedMixinError(mixinName, line, col, closestName(mixinName, candidates)))
	}

	// Cycle detection: if mixinName is already on the stack, we have a cycle
	for _, name := range t.mixinStack {
		if name == mixinName {
			chain := append(t.mixinStack, mixinName)
			panic(NewCircularReferenceError("mixin", chain, 0, 0))
		}
	}

	// Parse call arguments (positional and named)
	var positional []interface{}
	named := map[string]interface{}{}
	if argsNode != nil {
		positional, named = t.parseIncludeArgs(argsNode)
	}

	// Pre-evaluate each arg in the CALLER's scope before binding to mixin scope.
	// This prevents infinite recursion when a mixin param name matches a caller variable.
	// e.g., @include gap($gap: $gap) — without pre-eval, the mixin scope sees itself.
	evaluateArg := func(argNode interface{}) interface{} {
		if ast, ok := argNode.(*parser.ASTNode); ok {
			cloned := deepCloneAST(ast)
			return t.expandNode(cloned, scope) // caller's scope
		}
		return argNode
	}

	// Check arity against positional + named counts
	totalProvided := len(positional) + len(named)
	required := len(mixinDef.Params) - len(mixinDef.Defaults)
	if totalProvided < required || totalProvided > len(mixinDef.Params) {
		panic(NewWrongArityError("Mixin", mixinName, len(mixinDef.Params), totalProvided, 0, 0))
	}

	// Create a child scope for the mixin expansion, bind parameters
	// Named args take priority; positional args fill remaining slots in order.
	mixinScope := scope.Child()
	posIdx := 0
	for _, paramName := range mixinDef.Params {
		if val, ok := named[paramName]; ok {
			mixinScope.Set(paramName, evaluateArg(val))
		} else if posIdx < len(positional) {
			mixinScope.Set(paramName, evaluateArg(positional[posIdx]))
			posIdx++
		} else if defVal, ok := mixinDef.Defaults[paramName]; ok {
			mixinScope.Set(paramName, deepCloneRaw(defVal))
		}
	}

	// Lattice v2: push content block and caller scope for @content
	t.contentBlockStack = append(t.contentBlockStack, contentBlock)
	t.contentScopeStack = append(t.contentScopeStack, scope)

	// Clone and expand the mixin body, tracking the call stack
	t.mixinStack = append(t.mixinStack, mixinName)
	defer func() {
		t.mixinStack = t.mixinStack[:len(t.mixinStack)-1]
		t.contentBlockStack = t.contentBlockStack[:len(t.contentBlockStack)-1]
		t.contentScopeStack = t.contentScopeStack[:len(t.contentScopeStack)-1]
	}()

	bodyClone := deepCloneAST(mixinDef.Body)
	expanded := t.expandNode(bodyClone, mixinScope)

	// Extract block_contents children from the expanded block
	if expAST, ok := expanded.(*parser.ASTNode); ok {
		for _, child := range expAST.Children {
			if ast, ok := child.(*parser.ASTNode); ok && ast.RuleName == "block_contents" {
				result := make([]interface{}, 0, len(ast.Children))
				for _, c := range ast.Children {
					if c != nil {
						result = append(result, c)
					}
				}
				return result
			}
		}
	}

	return nil
}

// parseIncludeArgs parses include_args into positional and named argument slices.
//
// The grammar was updated so include_args = include_arg { COMMA include_arg }
// where include_arg = VARIABLE COLON value_list | value_list.
//
// Named args (e.g., $gap: 8px) are returned separately from positional args.
// Legacy form: a single value_list with internal commas is split on those commas.
//
// Returns (positional []interface{}, named map[string]interface{}).
func (t *LatticeTransformer) parseIncludeArgs(node *parser.ASTNode) ([]interface{}, map[string]interface{}) {
	positional := []interface{}{}
	named := map[string]interface{}{}

	// Check for include_arg children (new grammar form)
	hasIncludeArgs := false
	for _, child := range node.Children {
		if ast, ok := child.(*parser.ASTNode); ok && ast.RuleName == "include_arg" {
			hasIncludeArgs = true
			break
		}
	}

	if hasIncludeArgs {
		for _, child := range node.Children {
			ast, ok := child.(*parser.ASTNode)
			if !ok || ast.RuleName != "include_arg" {
				continue
			}
			// include_arg = VARIABLE COLON value_list  (named)
			//             | value_list                  (positional)
			children := ast.Children
			if len(children) >= 3 {
				// Check if first child is VARIABLE and second is COLON
				if tok, ok := children[0].(lexer.Token); ok && tokenTypeName(tok) == "VARIABLE" {
					if colon, ok := children[1].(lexer.Token); ok && tokenTypeName(colon) == "COLON" {
						if vl, ok := children[2].(*parser.ASTNode); ok {
							named[tok.Value] = vl
							continue
						}
					}
				}
			}
			// Positional: find value_list child and split on commas.
			// value_list greedily consumes COMMA tokens, so button(blue, white)
			// arrives as one include_arg wrapping value_list([blue, ,, white]).
			// splitValueListOnCommas splits it into [value_list(blue), value_list(white)].
			for _, c := range children {
				if vl, ok := c.(*parser.ASTNode); ok && (vl.RuleName == "value_list" || vl.RuleName == "mixin_value_list") {
					parts := splitValueListOnCommas(vl)
					positional = append(positional, parts...)
					break
				}
			}
		}
		return positional, named
	}

	// Legacy form: collect all value_list children and split on commas
	var valueLists []*parser.ASTNode
	for _, child := range node.Children {
		if ast, ok := child.(*parser.ASTNode); ok && ast.RuleName == "value_list" {
			valueLists = append(valueLists, ast)
		}
	}

	if len(valueLists) == 0 {
		return positional, named
	}

	var raw []interface{}
	if len(valueLists) == 1 {
		raw = splitValueListOnCommas(valueLists[0])
	} else {
		raw = make([]interface{}, len(valueLists))
		for i, vl := range valueLists {
			raw[i] = vl
		}
	}
	positional = append(positional, raw...)
	return positional, named
}

// splitValueListOnCommas splits a value_list into multiple lists at COMMA boundaries.
//
// The grammar allows COMMA as a value token, so:
//   button(red, white)
// parses as a single value_list: [value(red), value(COMMA), value(white)].
// We need to split this into [value_list(red), value_list(white)].
func splitValueListOnCommas(node *parser.ASTNode) []interface{} {
	// Check if any value child wraps a COMMA token
	hasComma := false
	for _, child := range node.Children {
		if ast, ok := child.(*parser.ASTNode); ok && ast.RuleName == "value" {
			for _, vc := range ast.Children {
				if tok, ok := vc.(lexer.Token); ok && tokenTypeName(tok) == "COMMA" {
					hasComma = true
					break
				}
			}
		}
	}

	if !hasComma {
		return []interface{}{node}
	}

	// Split on comma value nodes
	var groups [][]interface{}
	groups = append(groups, []interface{}{})

	for _, child := range node.Children {
		if ast, ok := child.(*parser.ASTNode); ok && ast.RuleName == "value" {
			// Is this value just a COMMA?
			if len(ast.Children) == 1 {
				if tok, ok := ast.Children[0].(lexer.Token); ok && tokenTypeName(tok) == "COMMA" {
					groups = append(groups, []interface{}{})
					continue
				}
			}
		}
		groups[len(groups)-1] = append(groups[len(groups)-1], child)
	}

	// Build new value_list nodes for each group
	result := make([]interface{}, 0, len(groups))
	for _, group := range groups {
		if len(group) > 0 {
			vl := &parser.ASTNode{
				RuleName: "value_list",
				Children: group,
			}
			result = append(result, vl)
		}
	}
	return result
}

// ============================================================================
// Control Flow
// ============================================================================

// expandControl dispatches to the appropriate control-flow handler.
//
// lattice_control = if_directive | for_directive | each_directive | while_directive ;
//
// Lattice v2 adds while_directive.
func (t *LatticeTransformer) expandControl(node *parser.ASTNode, scope *ScopeChain) []interface{} {
	if len(node.Children) == 0 {
		return nil
	}

	inner, ok := node.Children[0].(*parser.ASTNode)
	if !ok {
		return nil
	}

	switch inner.RuleName {
	case "if_directive":
		return t.expandIf(inner, scope)
	case "for_directive":
		return t.expandFor(inner, scope)
	case "each_directive":
		return t.expandEach(inner, scope)
	case "while_directive":
		return t.expandWhile(inner, scope)
	}

	return nil
}

// expandIf evaluates @if / @else if / @else directives.
//
// Grammar:
//
//	if_directive = "@if" lattice_expression block
//	               { "@else" "if" lattice_expression block }
//	               [ "@else" block ] ;
//
// We walk the children to extract (condition, block) pairs, evaluate each
// condition in order, and expand the first matching branch.
func (t *LatticeTransformer) expandIf(node *parser.ASTNode, scope *ScopeChain) []interface{} {
	type branch struct {
		condition interface{} // nil for @else
		block     *parser.ASTNode
	}

	var branches []branch

	children := node.Children
	i := 0
	for i < len(children) {
		child := children[i]
		tok, isTok := child.(lexer.Token)

		if isTok && tok.Value == "@if" {
			// @if expr block
			if i+2 < len(children) {
				branches = append(branches, branch{children[i+1], mustASTNode(children[i+2])})
				i += 3
			} else {
				i++
			}
			continue
		}

		if isTok && tok.Value == "@else" {
			// Check if next token is "if"
			if i+1 < len(children) {
				nextTok, nextIsTok := children[i+1].(lexer.Token)
				if nextIsTok && nextTok.Value == "if" {
					// @else if expr block
					if i+3 < len(children) {
						branches = append(branches, branch{children[i+2], mustASTNode(children[i+3])})
						i += 4
					} else {
						i++
					}
					continue
				}
			}
			// @else block
			if i+1 < len(children) {
				branches = append(branches, branch{nil, mustASTNode(children[i+1])})
				i += 2
			} else {
				i++
			}
			continue
		}

		i++
	}

	// Evaluate branches and expand the first matching one
	eval := NewExpressionEvaluator(scope)
	for _, b := range branches {
		if b.condition == nil {
			// @else — always matches
			return t.expandBlockToItems(b.block, scope)
		}
		result := eval.Evaluate(b.condition)
		if result.Truthy() {
			return t.expandBlockToItems(b.block, scope)
		}
	}

	return nil
}

// expandFor unrolls a @for loop into its iterations.
//
// Grammar:
//
//	for_directive = "@for" VARIABLE "from" lattice_expression
//	                ( "through" | "to" ) lattice_expression block ;
//
// "through" is inclusive (1 through 3 → 1, 2, 3).
// "to" is exclusive (1 to 3 → 1, 2).
func (t *LatticeTransformer) expandFor(node *parser.ASTNode, scope *ScopeChain) []interface{} {
	var varName string
	var fromExpr interface{}
	var toExpr interface{}
	isThrough := false
	var block *parser.ASTNode

	children := node.Children
	i := 0
	for i < len(children) {
		child := children[i]
		switch c := child.(type) {
		case lexer.Token:
			switch {
			case tokenTypeName(c) == "VARIABLE":
				varName = c.Value
			case c.Value == "from":
				if i+1 < len(children) {
					fromExpr = children[i+1]
					i++
				}
			case c.Value == "through":
				isThrough = true
				if i+1 < len(children) {
					toExpr = children[i+1]
					i++
				}
			case c.Value == "to":
				isThrough = false
				if i+1 < len(children) {
					toExpr = children[i+1]
					i++
				}
			}
		case *parser.ASTNode:
			if c.RuleName == "block" {
				block = c
			}
		}
		i++
	}

	if varName == "" || fromExpr == nil || toExpr == nil || block == nil {
		return nil
	}

	eval := NewExpressionEvaluator(scope)
	fromVal := eval.Evaluate(fromExpr)
	toVal := eval.Evaluate(toExpr)

	fromNum := int(toFloat64(fromVal))
	toNum := int(toFloat64(toVal))

	end := toNum
	if isThrough {
		end = toNum + 1 // inclusive
	}

	var result []interface{}
	for idx := fromNum; idx < end; idx++ {
		loopScope := scope.Child()
		loopScope.Set(varName, LatticeNumber{Value: float64(idx)})
		items := t.expandBlockToItems(deepCloneAST(block), loopScope)
		result = append(result, items...)
	}

	return result
}

// expandEach unrolls a @each loop over a list.
//
// Grammar:
//
//	each_directive = "@each" VARIABLE { COMMA VARIABLE } "in" each_list block ;
//	each_list = value { COMMA value } ;
func (t *LatticeTransformer) expandEach(node *parser.ASTNode, scope *ScopeChain) []interface{} {
	var varNames []string
	var eachListNode *parser.ASTNode
	var block *parser.ASTNode

	for _, child := range node.Children {
		switch c := child.(type) {
		case lexer.Token:
			if tokenTypeName(c) == "VARIABLE" {
				varNames = append(varNames, c.Value)
			}
		case *parser.ASTNode:
			switch c.RuleName {
			case "each_list":
				eachListNode = c
			case "block":
				block = c
			}
		}
	}

	if len(varNames) == 0 || eachListNode == nil || block == nil {
		return nil
	}

	// Lattice v2: check if the each_list contains a variable that resolves to a map or list
	resolvedCollection := t.resolveEachList(eachListNode, scope)
	if resolvedCollection != nil {
		return t.expandEachOverResolved(varNames, resolvedCollection, block, scope)
	}

	// Extract items from each_list = value { COMMA value }
	var items []interface{}
	for _, child := range eachListNode.Children {
		if ast, ok := child.(*parser.ASTNode); ok && ast.RuleName == "value" {
			items = append(items, ast)
		}
	}

	var result []interface{}
	for _, item := range items {
		loopScope := scope.Child()
		if len(varNames) > 0 {
			// Extract the token value from the value node
			itemVal := extractValueToken(item)
			loopScope.Set(varNames[0], itemVal)
		}
		expanded := t.expandBlockToItems(deepCloneAST(block), loopScope)
		result = append(result, expanded...)
	}

	return result
}

// resolveEachList checks if an each_list contains a single variable that
// resolves to a LatticeMap or LatticeList.
//
// Lattice v2: if the variable's scope value is an AST node wrapping a
// map_literal, we convert it to a LatticeMap on the fly.
func (t *LatticeTransformer) resolveEachList(eachList *parser.ASTNode, scope *ScopeChain) LatticeValue {
	var varTokens []lexer.Token
	for _, child := range eachList.Children {
		if ast, ok := child.(*parser.ASTNode); ok && ast.RuleName == "value" {
			for _, vc := range ast.Children {
				if tok, ok := vc.(lexer.Token); ok && tokenTypeName(tok) == "VARIABLE" {
					varTokens = append(varTokens, tok)
				}
			}
		}
	}
	if len(varTokens) == 1 {
		val, ok := scope.Get(varTokens[0].Value)
		if ok {
			switch v := val.(type) {
			case LatticeMap:
				return v
			case LatticeList:
				return v
			case *parser.ASTNode:
				// The variable holds an unevaluated AST node — check if it wraps a map_literal
				mapLit := findMapLiteralInAST(v)
				if mapLit != nil {
					return convertMapLiteralToLatticeMap(mapLit, scope)
				}
			}
		}
	}
	return nil
}

// findMapLiteralInAST recursively searches an AST subtree for a node with
// RuleName == "map_literal".
func findMapLiteralInAST(node *parser.ASTNode) *parser.ASTNode {
	if node == nil {
		return nil
	}
	if node.RuleName == "map_literal" {
		return node
	}
	for _, child := range node.Children {
		if ast, ok := child.(*parser.ASTNode); ok {
			if found := findMapLiteralInAST(ast); found != nil {
				return found
			}
		}
	}
	return nil
}

// convertMapLiteralToLatticeMap converts a map_literal AST node to a LatticeMap.
//
// Grammar: map_literal = LPAREN map_entry { COMMA map_entry } [ COMMA ] RPAREN ;
//          map_entry   = ( IDENT | STRING ) COLON lattice_expression ;
func convertMapLiteralToLatticeMap(mapLit *parser.ASTNode, scope *ScopeChain) LatticeMap {
	var items []MapEntry
	eval := NewExpressionEvaluator(scope)

	for _, child := range mapLit.Children {
		ast, ok := child.(*parser.ASTNode)
		if !ok || ast.RuleName != "map_entry" {
			continue
		}
		var key string
		var valueExpr interface{}
		seenColon := false
		for _, ec := range ast.Children {
			switch c := ec.(type) {
			case lexer.Token:
				tn := tokenTypeName(c)
				if !seenColon && (tn == "IDENT" || tn == "STRING") {
					key = strings.Trim(c.Value, "\"'")
				} else if tn == "COLON" {
					seenColon = true
				}
			case *parser.ASTNode:
				if seenColon && valueExpr == nil {
					valueExpr = c
				}
			}
		}
		if key != "" && valueExpr != nil {
			items = append(items, MapEntry{Key: key, Value: eval.Evaluate(valueExpr)})
		}
	}
	return LatticeMap{Items: items}
}

// expandEachOverResolved handles @each iteration over a resolved map or list.
func (t *LatticeTransformer) expandEachOverResolved(varNames []string, collection LatticeValue, block *parser.ASTNode, scope *ScopeChain) []interface{} {
	var result []interface{}

	switch coll := collection.(type) {
	case LatticeMap:
		for _, entry := range coll.Items {
			loopScope := scope.Child()
			loopScope.Set(varNames[0], LatticeIdent{Value: entry.Key})
			if len(varNames) >= 2 {
				loopScope.Set(varNames[1], entry.Value)
			}
			expanded := t.expandBlockToItems(deepCloneAST(block), loopScope)
			result = append(result, expanded...)
		}
	case LatticeList:
		for _, item := range coll.Items {
			loopScope := scope.Child()
			loopScope.Set(varNames[0], item)
			expanded := t.expandBlockToItems(deepCloneAST(block), loopScope)
			result = append(result, expanded...)
		}
	}

	return result
}

// expandBlockToItems expands a block and returns its block_contents children.
//
// Used by @if, @for, @each to get the list of CSS items from a branch body.
func (t *LatticeTransformer) expandBlockToItems(block *parser.ASTNode, scope *ScopeChain) []interface{} {
	expanded := t.expandNode(block, scope)
	if expAST, ok := expanded.(*parser.ASTNode); ok {
		for _, child := range expAST.Children {
			if ast, ok := child.(*parser.ASTNode); ok && ast.RuleName == "block_contents" {
				result := make([]interface{}, 0, len(ast.Children))
				for _, c := range ast.Children {
					if c != nil {
						result = append(result, c)
					}
				}
				return result
			}
		}
	}
	return nil
}

// ============================================================================
// Function Evaluation
// ============================================================================

// evaluateFunctionCall evaluates a Lattice function call and returns the result
// as a synthetic CSS token (or value node).
//
// The function body is evaluated in an isolated scope (parent = globals only),
// not the caller's scope. @return is implemented via panic/recover.
func (t *LatticeTransformer) evaluateFunctionCall(funcName string, node *parser.ASTNode, scope *ScopeChain) interface{} {
	funcDef := t.functions[funcName]

	// Parse arguments from function_args
	var args []interface{}
	for _, child := range node.Children {
		if ast, ok := child.(*parser.ASTNode); ok && ast.RuleName == "function_args" {
			args = t.parseFunctionCallArgs(ast)
			break
		}
	}

	// Check arity
	required := len(funcDef.Params) - len(funcDef.Defaults)
	if len(args) < required || len(args) > len(funcDef.Params) {
		panic(NewWrongArityError("Function", funcName, len(funcDef.Params), len(args), 0, 0))
	}

	// Cycle detection
	for _, name := range t.functionStack {
		if name == funcName {
			chain := append(t.functionStack, funcName)
			panic(NewCircularReferenceError("function", chain, 0, 0))
		}
	}

	// Create an isolated function scope (parent = global scope only)
	funcScope := t.variables.Child()
	for i, paramName := range funcDef.Params {
		if i < len(args) {
			funcScope.Set(paramName, args[i])
		} else if defVal, ok := funcDef.Defaults[paramName]; ok {
			funcScope.Set(paramName, deepCloneRaw(defVal))
		}
	}

	// Evaluate the function body, catching @return via panic/recover
	t.functionStack = append(t.functionStack, funcName)
	defer func() { t.functionStack = t.functionStack[:len(t.functionStack)-1] }()

	var returnValue LatticeValue
	func() {
		defer func() {
			if r := recover(); r != nil {
				if sig, ok := r.(returnSignal); ok {
					returnValue = sig.value
				} else {
					panic(r) // re-panic for real errors
				}
			}
		}()
		bodyClone := deepCloneAST(funcDef.Body)
		t.evaluateFunctionBody(bodyClone, funcScope)
	}()

	if returnValue == nil {
		panic(NewMissingReturnError(funcName, 0, 0))
	}

	// Convert the return value to a CSS value node
	cssText := valueToCSSText(returnValue)
	return makeValueNode(cssText, node)
}

// evaluateFunctionBody evaluates statements in a function body.
//
// Grammar:
//
//	function_body = LBRACE { function_body_item } RBRACE ;
//	function_body_item = variable_declaration | return_directive | lattice_control ;
//
// @return is signaled via panic(returnSignal{value}).
func (t *LatticeTransformer) evaluateFunctionBody(body *parser.ASTNode, scope *ScopeChain) {
	if body == nil {
		return
	}

	for _, child := range body.Children {
		ast, ok := child.(*parser.ASTNode)
		if !ok {
			continue
		}

		if ast.RuleName == "function_body_item" {
			if len(ast.Children) == 0 {
				continue
			}
			inner, ok := ast.Children[0].(*parser.ASTNode)
			if !ok {
				continue
			}

			switch inner.RuleName {
			case "variable_declaration":
				t.expandVariableDeclaration(inner, scope)
			case "return_directive":
				t.evaluateReturn(inner, scope)
			case "lattice_control":
				t.evaluateControlInFunction(inner, scope)
			}
		}
	}
}

// evaluateReturn evaluates a @return directive and signals the return value.
//
// Grammar: return_directive = "@return" lattice_expression SEMICOLON ;
func (t *LatticeTransformer) evaluateReturn(node *parser.ASTNode, scope *ScopeChain) {
	for _, child := range node.Children {
		if ast, ok := child.(*parser.ASTNode); ok && ast.RuleName == "lattice_expression" {
			eval := NewExpressionEvaluator(scope)
			result := eval.Evaluate(ast)
			panic(returnSignal{value: result})
		}
	}
}

// evaluateControlInFunction evaluates control flow inside a function body.
//
// Only @if is currently supported inside functions (as a way to conditionally
// return different values).
func (t *LatticeTransformer) evaluateControlInFunction(node *parser.ASTNode, scope *ScopeChain) {
	if len(node.Children) == 0 {
		return
	}
	inner, ok := node.Children[0].(*parser.ASTNode)
	if !ok {
		return
	}
	if inner.RuleName == "if_directive" {
		t.evaluateIfInFunction(inner, scope)
	}
}

// evaluateIfInFunction evaluates @if inside a function body.
func (t *LatticeTransformer) evaluateIfInFunction(node *parser.ASTNode, scope *ScopeChain) {
	type branch struct {
		condition interface{}
		block     *parser.ASTNode
	}

	var branches []branch
	children := node.Children
	i := 0
	for i < len(children) {
		child := children[i]
		tok, isTok := child.(lexer.Token)
		if isTok && tok.Value == "@if" {
			if i+2 < len(children) {
				branches = append(branches, branch{children[i+1], mustASTNode(children[i+2])})
				i += 3
			} else {
				i++
			}
			continue
		}
		if isTok && tok.Value == "@else" {
			if i+1 < len(children) {
				nextTok, nextIsTok := children[i+1].(lexer.Token)
				if nextIsTok && nextTok.Value == "if" {
					if i+3 < len(children) {
						branches = append(branches, branch{children[i+2], mustASTNode(children[i+3])})
						i += 4
					} else {
						i++
					}
					continue
				}
			}
			if i+1 < len(children) {
				branches = append(branches, branch{nil, mustASTNode(children[i+1])})
				i += 2
			} else {
				i++
			}
			continue
		}
		i++
	}

	eval := NewExpressionEvaluator(scope)
	for _, b := range branches {
		if b.condition == nil || eval.Evaluate(b.condition).Truthy() {
			t.evaluateBlockInFunction(b.block, scope)
			return
		}
	}
}

// evaluateBlockInFunction looks for @return statements in a block that is
// inside a function body.
//
// Inside @if blocks within @function bodies, @return appears as an at_rule
// node in the AST (because the grammar's function_body_item rule is only
// active at the function body's top level). We detect "@return" at-rules here.
func (t *LatticeTransformer) evaluateBlockInFunction(block *parser.ASTNode, scope *ScopeChain) {
	if block == nil {
		return
	}

	for _, child := range block.Children {
		ast, ok := child.(*parser.ASTNode)
		if !ok {
			continue
		}

		switch ast.RuleName {
		case "block_contents":
			t.evaluateBlockInFunction(ast, scope)
		case "block_item":
			if len(ast.Children) == 0 {
				continue
			}
			inner, ok := ast.Children[0].(*parser.ASTNode)
			if !ok {
				continue
			}
			switch inner.RuleName {
			case "at_rule":
				t.maybeEvaluateReturnAtRule(inner, scope)
			case "lattice_block_item":
				for _, lbc := range inner.Children {
					if lbcAST, ok := lbc.(*parser.ASTNode); ok {
						if lbcAST.RuleName == "variable_declaration" {
							t.expandVariableDeclaration(lbcAST, scope)
						}
					}
				}
			}
		}
	}
}

// maybeEvaluateReturnAtRule checks if an at_rule is actually @return.
//
// Grammar: at_rule = AT_KEYWORD at_prelude ( SEMICOLON | block ) ;
//
// Inside an @if block within a @function body, @return is parsed as an at_rule.
// We detect this by checking if the AT_KEYWORD token's value is "@return".
func (t *LatticeTransformer) maybeEvaluateReturnAtRule(node *parser.ASTNode, scope *ScopeChain) {
	for _, child := range node.Children {
		tok, ok := child.(lexer.Token)
		if !ok {
			continue
		}
		if tokenTypeName(tok) == "AT_KEYWORD" && tok.Value == "@return" {
			// Find the at_prelude and evaluate it as an expression
			for _, inner := range node.Children {
				if ast, ok := inner.(*parser.ASTNode); ok && ast.RuleName == "at_prelude" {
					eval := NewExpressionEvaluator(scope)
					result := eval.Evaluate(ast)
					panic(returnSignal{value: result})
				}
			}
		}
	}
}

// parseFunctionCallArgs parses function_args into a list of argument values.
//
// Grammar: function_args = { function_arg } ;
//
// Each function_arg may contain a VARIABLE or other value token.
func (t *LatticeTransformer) parseFunctionCallArgs(node *parser.ASTNode) []interface{} {
	// Group function_arg children into comma-separated argument groups
	var args []interface{}
	var currentGroup []interface{}

	for _, child := range node.Children {
		ast, ok := child.(*parser.ASTNode)
		if !ok {
			continue
		}
		if ast.RuleName == "function_arg" {
			if t.isFunctionArgComma(ast) {
				if len(currentGroup) > 0 {
					args = append(args, t.makeValueListFromGroup(currentGroup))
					currentGroup = nil
				}
			} else {
				currentGroup = append(currentGroup, ast)
			}
		}
	}

	if len(currentGroup) > 0 {
		args = append(args, t.makeValueListFromGroup(currentGroup))
	}

	return args
}

// isFunctionArgComma reports whether a function_arg contains only a COMMA.
func (t *LatticeTransformer) isFunctionArgComma(node *parser.ASTNode) bool {
	if len(node.Children) == 1 {
		if tok, ok := node.Children[0].(lexer.Token); ok {
			return tokenTypeName(tok) == "COMMA"
		}
	}
	return false
}

// makeValueListFromGroup creates a value_list-like node from a group of function_args.
func (t *LatticeTransformer) makeValueListFromGroup(group []interface{}) *parser.ASTNode {
	children := make([]interface{}, 0, len(group))
	for _, item := range group {
		if ast, ok := item.(*parser.ASTNode); ok {
			// Wrap each function_arg as a value node
			val := &parser.ASTNode{
				RuleName: "value",
				Children: ast.Children,
			}
			children = append(children, val)
		}
	}
	return &parser.ASTNode{
		RuleName: "value_list",
		Children: children,
	}
}

// ============================================================================
// Lattice v2: @while Loops
// ============================================================================

// expandWhile expands a @while loop.
//
// Grammar: while_directive = "@while" lattice_expression block ;
//
// Evaluates condition repeatedly, expanding block body each iteration.
// Uses the enclosing scope directly (not a child scope) so variable
// mutations persist across iterations. Max-iteration guard prevents
// infinite loops.
func (t *LatticeTransformer) expandWhile(node *parser.ASTNode, scope *ScopeChain) []interface{} {
	var condition *parser.ASTNode
	var block *parser.ASTNode

	for _, child := range node.Children {
		if ast, ok := child.(*parser.ASTNode); ok {
			switch ast.RuleName {
			case "lattice_expression":
				condition = ast
			case "block":
				block = ast
			}
		}
	}

	if condition == nil || block == nil {
		return nil
	}

	var result []interface{}
	iteration := 0

	for {
		eval := NewExpressionEvaluator(scope)
		condValue := eval.Evaluate(deepCloneAST(condition))

		if !condValue.Truthy() {
			break
		}

		iteration++
		if iteration > t.maxWhileIterations {
			panic(NewMaxIterationError(t.maxWhileIterations, 0, 0))
		}

		expanded := t.expandBlockToItems(deepCloneAST(block), scope)
		result = append(result, expanded...)
	}

	return result
}

// ============================================================================
// Lattice v2: $var in Selectors
// ============================================================================

// expandSelectorWithVars resolves VARIABLE tokens in selector positions.
//
// When a VARIABLE token appears in a compound_selector, simple_selector,
// or class_selector, resolve it to its string value and create a
// synthetic IDENT token.
func (t *LatticeTransformer) expandSelectorWithVars(node *parser.ASTNode, scope *ScopeChain) *parser.ASTNode {
	newChildren := make([]interface{}, 0, len(node.Children))

	for _, child := range node.Children {
		switch c := child.(type) {
		case lexer.Token:
			if tokenTypeName(c) == "VARIABLE" {
				val, ok := scope.Get(c.Value)
				if !ok {
					panic(NewUndefinedVariableError(c.Value, c.Line, c.Column))
				}
				var cssText string
				if lv, ok := val.(LatticeValue); ok {
					cssText = valueToCSSText(lv)
				} else if ast, ok := val.(*parser.ASTNode); ok {
					eval := NewExpressionEvaluator(scope)
					v := eval.extractValueFromAST(ast)
					cssText = valueToCSSText(v)
				} else {
					cssText = fmt.Sprintf("%v", val)
				}
				// Strip quotes from strings in selector context
				cssText = strings.Trim(cssText, "\"'")
				newChildren = append(newChildren, makeSyntheticToken(cssText, c))
			} else {
				newChildren = append(newChildren, child)
			}
		case *parser.ASTNode:
			newChildren = append(newChildren, t.expandNode(c, scope))
		default:
			newChildren = append(newChildren, child)
		}
	}

	node.Children = newChildren
	return node
}

// ============================================================================
// Lattice v2: @content Blocks
// ============================================================================

// expandContent expands a @content directive inside a mixin body.
//
// Replaces @content; with the content block from the current @include call.
// The content block is evaluated in the caller's scope, not the mixin's scope.
// If no content block was passed, produces an empty list.
func (t *LatticeTransformer) expandContent(node *parser.ASTNode, scope *ScopeChain) []interface{} {
	if len(t.contentBlockStack) == 0 {
		return nil
	}

	contentBlock := t.contentBlockStack[len(t.contentBlockStack)-1]
	if contentBlock == nil {
		return nil
	}

	// Evaluate in the caller's scope
	callerScope := scope
	if len(t.contentScopeStack) > 0 {
		callerScope = t.contentScopeStack[len(t.contentScopeStack)-1]
	}

	return t.expandBlockToItems(deepCloneAST(contentBlock), callerScope)
}

// ============================================================================
// Lattice v2: @at-root
// ============================================================================

// expandAtRoot expands an @at-root directive.
//
// Rules inside @at-root are collected and hoisted to the stylesheet root
// level during Pass 3. They are removed from the current nesting context.
func (t *LatticeTransformer) expandAtRoot(node *parser.ASTNode, scope *ScopeChain) interface{} {
	var block *parser.ASTNode
	var selectorList *parser.ASTNode

	for _, child := range node.Children {
		if ast, ok := child.(*parser.ASTNode); ok {
			switch ast.RuleName {
			case "block":
				block = ast
			case "selector_list":
				selectorList = ast
			}
		}
	}

	if block == nil {
		return nil
	}

	if selectorList != nil {
		// Inline form: @at-root .selector { ... }
		expandedSel := t.expandNode(deepCloneAST(selectorList), scope)
		expandedBlock := t.expandNode(deepCloneAST(block), scope)
		qr := makeQualifiedRule(expandedSel, expandedBlock)
		t.atRootRules = append(t.atRootRules, qr)
	} else {
		// Block form: @at-root { ... multiple rules ... }
		expanded := t.expandBlockToItems(deepCloneAST(block), scope)
		t.atRootRules = append(t.atRootRules, expanded...)
	}

	return nil // Remove from current position
}

// makeQualifiedRule creates a qualified_rule AST node from a selector and block.
func makeQualifiedRule(selector, block interface{}) *parser.ASTNode {
	return &parser.ASTNode{
		RuleName: "qualified_rule",
		Children: []interface{}{selector, block},
	}
}

// ============================================================================
// Lattice v2: @extend and %placeholder
// ============================================================================

// collectExtend records an @extend directive for later selector merging.
func (t *LatticeTransformer) collectExtend(node *parser.ASTNode, scope *ScopeChain) {
	var target string

	for _, child := range node.Children {
		if ast, ok := child.(*parser.ASTNode); ok && ast.RuleName == "extend_target" {
			target = extractExtendTarget(ast)
		}
	}

	if target != "" {
		if _, exists := t.extendMap[target]; !exists {
			t.extendMap[target] = []string{}
		}
	}
}

// extractExtendTarget gets the target selector string from an extend_target node.
func extractExtendTarget(node *parser.ASTNode) string {
	var parts []string
	for _, child := range node.Children {
		if tok, ok := child.(lexer.Token); ok {
			parts = append(parts, tok.Value)
		}
	}
	return strings.Join(parts, "")
}

// ============================================================================
// Lattice v2: Built-in Function Evaluation
// ============================================================================

// evaluateBuiltinFunctionCall evaluates a Lattice v2 built-in function call.
//
// Uses the ExpressionEvaluator to resolve arguments, then calls the
// registered built-in function handler. The result is converted back
// to an AST node for emission.
func (t *LatticeTransformer) evaluateBuiltinFunctionCall(funcName string, node *parser.ASTNode, scope *ScopeChain) interface{} {
	var args []LatticeValue

	// Collect and evaluate arguments
	for _, child := range node.Children {
		if ast, ok := child.(*parser.ASTNode); ok && ast.RuleName == "function_args" {
			eval := NewExpressionEvaluator(scope)
			args = eval.collectFunctionArgs(ast)
			break
		}
	}

	result := CallBuiltinFunction(funcName, args, scope)

	if _, isNull := result.(LatticeNull); isNull {
		// Null result — pass through as CSS function
		return t.expandChildren(node, scope)
	}

	cssText := valueToCSSText(result)
	return makeValueNode(cssText, node)
}

// collectFunctionArgs evaluates function arguments for built-in function calls.
func (e *ExpressionEvaluator) collectFunctionArgs(node *parser.ASTNode) []LatticeValue {
	var args []LatticeValue
	var currentTokens []interface{}

	for _, child := range node.Children {
		if ast, ok := child.(*parser.ASTNode); ok && ast.RuleName == "function_arg" {
			for _, ic := range ast.Children {
				if tok, ok := ic.(lexer.Token); ok {
					if tokenTypeName(tok) == "COMMA" {
						if len(currentTokens) > 0 {
							args = append(args, e.evalArgTokens(currentTokens))
							currentTokens = nil
						}
						continue
					}
					currentTokens = append(currentTokens, ic)
				} else if astChild, ok := ic.(*parser.ASTNode); ok {
					// AST node (expression) — evaluate directly
					args = append(args, e.Evaluate(astChild))
					currentTokens = nil
				}
			}
		}
	}

	if len(currentTokens) > 0 {
		args = append(args, e.evalArgTokens(currentTokens))
	}

	return args
}

// evalArgTokens evaluates a sequence of tokens as a single argument value.
func (e *ExpressionEvaluator) evalArgTokens(tokens []interface{}) LatticeValue {
	if len(tokens) == 1 {
		if tok, ok := tokens[0].(lexer.Token); ok {
			if tokenTypeName(tok) == "VARIABLE" {
				val, ok := e.scope.Get(tok.Value)
				if ok {
					if lv, ok := val.(LatticeValue); ok {
						return lv
					}
					if ast, ok := val.(*parser.ASTNode); ok {
						return e.extractValueFromAST(ast)
					}
				}
			}
			return tokenToValue(tok)
		}
	}
	if len(tokens) > 0 {
		if tok, ok := tokens[0].(lexer.Token); ok {
			return tokenToValue(tok)
		}
	}
	return LatticeNull{}
}

// ============================================================================
// Pass 3: Cleanup
// ============================================================================

// cleanup removes empty blocks and nodes that resulted from expansion.
//
// After expansion, @if branches that evaluated to false, @for loops with
// zero iterations, etc. may leave behind empty block_contents nodes.
// This pass removes them to produce clean CSS output.
func (t *LatticeTransformer) cleanup(node *parser.ASTNode) {
	if node == nil {
		return
	}

	newChildren := make([]interface{}, 0, len(node.Children))
	for _, child := range node.Children {
		if ast, ok := child.(*parser.ASTNode); ok {
			t.cleanup(ast)
			// Remove empty rules
			if ast.RuleName == "rule" && len(ast.Children) == 0 {
				continue
			}
		}
		newChildren = append(newChildren, child)
	}
	node.Children = newChildren
}

// ============================================================================
// Helpers
// ============================================================================

// trimTrailingParen removes a trailing "(" from a FUNCTION token value.
// "button(" → "button"
func trimTrailingParen(s string) string {
	if len(s) > 0 && s[len(s)-1] == '(' {
		return s[:len(s)-1]
	}
	return s
}

// mustASTNode asserts that an interface{} is *parser.ASTNode, returning nil otherwise.
func mustASTNode(v interface{}) *parser.ASTNode {
	ast, _ := v.(*parser.ASTNode)
	return ast
}

// toFloat64 extracts the numeric value from a LatticeValue.
func toFloat64(val LatticeValue) float64 {
	switch v := val.(type) {
	case LatticeNumber:
		return v.Value
	case LatticeDimension:
		return v.Value
	case LatticePercentage:
		return v.Value
	}
	return 0
}

// extractValueToken extracts the meaningful LatticeValue from a value AST node.
func extractValueToken(node interface{}) interface{} {
	ast, ok := node.(*parser.ASTNode)
	if !ok {
		return node
	}
	if len(ast.Children) == 1 {
		child := ast.Children[0]
		if tok, ok := child.(lexer.Token); ok {
			return tokenToValue(tok)
		}
		return child
	}
	return node
}

// makeSyntheticToken creates a new IDENT token with the given text value,
// inheriting position from an existing token.
func makeSyntheticToken(text string, from lexer.Token) lexer.Token {
	return lexer.Token{
		TypeName: "IDENT",
		Value:    text,
		Line:     from.Line,
		Column:   from.Column,
	}
}

// makeValueNode creates a value AST node wrapping a synthetic IDENT token.
// Used when a function call returns a value that needs to be placed in CSS output.
func makeValueNode(text string, from *parser.ASTNode) *parser.ASTNode {
	// Create a synthetic token for the CSS text
	var synTok lexer.Token
	if from != nil {
		synTok = lexer.Token{TypeName: "IDENT", Value: text}
	} else {
		synTok = lexer.Token{TypeName: "IDENT", Value: text}
	}
	return &parser.ASTNode{
		RuleName: "value",
		Children: []interface{}{synTok},
	}
}

// deepCloneAST performs a deep copy of an AST node tree.
//
// This is necessary for mixin/function expansion — we clone the body
// before expanding so the same mixin can be expanded multiple times
// without interference.
//
// We use JSON marshaling as a simple deep-copy mechanism. This works
// because ASTNode and lexer.Token are plain data structs with no
// function fields or circular references.
func deepCloneAST(node *parser.ASTNode) *parser.ASTNode {
	if node == nil {
		return nil
	}
	cloned := &parser.ASTNode{
		RuleName: node.RuleName,
		Children: make([]interface{}, len(node.Children)),
	}
	for i, child := range node.Children {
		switch c := child.(type) {
		case *parser.ASTNode:
			cloned.Children[i] = deepCloneAST(c)
		case lexer.Token:
			cloned.Children[i] = c // Token is a value type, copy is fine
		default:
			cloned.Children[i] = child
		}
	}
	return cloned
}

// deepCloneRaw clones any value that might be an AST node or primitive.
func deepCloneRaw(val interface{}) interface{} {
	if ast, ok := val.(*parser.ASTNode); ok {
		return deepCloneAST(ast)
	}
	return val
}
