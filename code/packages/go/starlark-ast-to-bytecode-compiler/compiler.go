// compiler.go — Compiles Starlark AST to bytecode instructions.
//
// ════════════════════════════════════════════════════════════════════════
// OVERVIEW
// ════════════════════════════════════════════════════════════════════════
//
// This file is the heart of the Starlark compiler. It takes an Abstract
// Syntax Tree (AST) produced by the starlark-parser and emits a sequence
// of bytecode instructions that the virtual machine can execute.
//
// The compilation pipeline looks like this:
//
//	Source Code (string)
//	    │
//	    ▼
//	starlark-lexer: TokenizeStarlark(source)
//	    │
//	    ▼
//	starlark-parser: ParseStarlark(source)
//	    │
//	    ▼
//	AST (parser.ASTNode tree)
//	    │
//	    ▼
//	THIS FILE: compile AST to bytecode
//	    │
//	    ▼
//	CodeObject (instructions + constants + names)
//
// ════════════════════════════════════════════════════════════════════════
// HOW THE COMPILER WORKS
// ════════════════════════════════════════════════════════════════════════
//
// The compiler walks the AST recursively. For each node type (identified
// by its RuleName), it emits the appropriate bytecode instructions.
//
// For example, compiling "x = 1 + 2":
//
//  1. Visit assign_stmt node
//  2. Visit the RHS expression first (1 + 2)
//     a. Visit arith node
//     b. Visit left atom (1) -> emit LOAD_CONST 0  (adds 1 to constants)
//     c. Visit right atom (2) -> emit LOAD_CONST 1  (adds 2 to constants)
//     d. See "+" operator -> emit ADD
//  3. See "=" operator and LHS target "x"
//  4. Emit STORE_NAME 0  (adds "x" to names)
//
// Result: instructions=[LOAD_CONST 0, LOAD_CONST 1, ADD, STORE_NAME 0, HALT]
//
//	constants=[1, 2]
//	names=["x"]
//
// ════════════════════════════════════════════════════════════════════════
// AST STRUCTURE
// ════════════════════════════════════════════════════════════════════════
//
// The parser produces parser.ASTNode trees where:
//   - node.RuleName is the grammar rule name (e.g., "file", "assign_stmt")
//   - node.Children is []interface{} containing:
//   - *parser.ASTNode for nested grammar rules
//   - lexer.Token for terminal tokens (identifiers, literals, operators)
//
// Children are in source order. For "x = 1 + 2", assign_stmt's children are:
//
//	[expression_list("x"), Token("="), expression_list("1 + 2")]
package starlarkcompiler

import (
	"fmt"
	"reflect"
	"strconv"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
	starlarkparser "github.com/adhithyan15/coding-adventures/code/packages/go/starlark-parser"
	vm "github.com/adhithyan15/coding-adventures/code/packages/go/virtual-machine"
)

// ════════════════════════════════════════════════════════════════════════
// COMPILER STATE
// ════════════════════════════════════════════════════════════════════════

// StarlarkCompiler holds the mutable state accumulated during compilation.
//
// As the compiler walks the AST, it builds up three parallel arrays:
//   - instructions: the bytecode program
//   - constants: literal values referenced by LOAD_CONST instructions
//   - names: variable/function/attribute names referenced by STORE_NAME etc.
//
// The scopeDepth tracks whether we're inside a function definition.
// At depth 0, variables use STORE_NAME/LOAD_NAME (global scope).
// At depth > 0, they use STORE_LOCAL/LOAD_LOCAL (function scope).
type StarlarkCompiler struct {
	instructions []vm.Instruction
	constants    []interface{}
	names        []string
	scopeDepth   int
}

// NewStarlarkCompiler creates a fresh compiler with empty state.
func NewStarlarkCompiler() *StarlarkCompiler {
	result, _ := StartNew[*StarlarkCompiler]("starlark-ast-to-bytecode-compiler.NewStarlarkCompiler", nil,
		func(op *Operation[*StarlarkCompiler], rf *ResultFactory[*StarlarkCompiler]) *OperationResult[*StarlarkCompiler] {
			return rf.Generate(true, false, &StarlarkCompiler{
				instructions: []vm.Instruction{},
				constants:    []interface{}{},
				names:        []string{},
				scopeDepth:   0,
			})
		}).GetResult()
	return result
}

// ════════════════════════════════════════════════════════════════════════
// HELPER METHODS — Emit instructions and manage constants/names
// ════════════════════════════════════════════════════════════════════════

// emit appends a single instruction to the bytecode stream.
func (c *StarlarkCompiler) emit(opcode vm.OpCode, operand ...interface{}) {
	instr := vm.Instruction{Opcode: opcode}
	if len(operand) > 0 {
		instr.Operand = operand[0]
	}
	c.instructions = append(c.instructions, instr)
}

// addConstant adds a value to the constants pool and returns its index.
// If the value already exists, returns the existing index (deduplication).
//
// Values that are not comparable with == (maps, slices — such as CodeObject
// from def statements) are always appended without deduplication. Each
// function definition needs its own constant slot even if two functions
// have identical structure. The == operator panics on uncomparable types,
// so we check comparability via reflect first.
func (c *StarlarkCompiler) addConstant(value interface{}) int {
	// Only deduplicate comparable types (int, string, bool, float64, etc.).
	// CodeObject, maps, and slices are uncomparable and must always get
	// fresh slots.
	if reflect.TypeOf(value) != nil && reflect.TypeOf(value).Comparable() {
		for i, v := range c.constants {
			if v == value {
				return i
			}
		}
	}
	c.constants = append(c.constants, value)
	return len(c.constants) - 1
}

func checkedIntLiteral(value int64, raw string) int {
	if value < math.MinInt || value > math.MaxInt {
		panic(fmt.Sprintf("integer literal out of range: %s", raw))
	}
	return int(value)
}

// addName adds a name to the names table and returns its index.
// If the name already exists, returns the existing index.
func (c *StarlarkCompiler) addName(name string) int {
	for i, n := range c.names {
		if n == name {
			return i
		}
	}
	c.names = append(c.names, name)
	return len(c.names) - 1
}

// currentOffset returns the index where the next instruction will be placed.
// Used for jump target calculations.
func (c *StarlarkCompiler) currentOffset() int {
	return len(c.instructions)
}

// emitJump emits a jump instruction with a placeholder operand (0) and
// returns the index of the jump instruction so it can be patched later.
//
// Why placeholders? When we compile "if x: ... else: ...", we emit
// JUMP_IF_FALSE before compiling the else branch, but we don't know
// the target address yet. So we emit a placeholder and patch it later.
func (c *StarlarkCompiler) emitJump(opcode vm.OpCode) int {
	idx := c.currentOffset()
	c.emit(opcode, 0)
	return idx
}

// patchJump fills in the operand of a previously-emitted jump instruction
// with the current offset (i.e., the instruction AFTER the jump target).
func (c *StarlarkCompiler) patchJump(jumpIdx int) {
	c.instructions[jumpIdx].Operand = c.currentOffset()
}

// ════════════════════════════════════════════════════════════════════════
// AST TRAVERSAL HELPERS
// ════════════════════════════════════════════════════════════════════════
//
// The parser produces heterogeneous children (ASTNode or lexer.Token).
// These helpers extract typed children for pattern matching.

// extractNodes returns all ASTNode children of a node.
func extractNodes(node *parser.ASTNode) []*parser.ASTNode {
	var nodes []*parser.ASTNode
	for _, child := range node.Children {
		if n, ok := child.(*parser.ASTNode); ok {
			nodes = append(nodes, n)
		}
	}
	return nodes
}

// extractTokens returns all lexer.Token children of a node.
func extractTokens(node *parser.ASTNode) []lexer.Token {
	var tokens []lexer.Token
	for _, child := range node.Children {
		if t, ok := child.(lexer.Token); ok {
			tokens = append(tokens, t)
		}
	}
	return tokens
}

// hasToken checks if any token child has a specific value.
// Used to detect operators like "=", "+", "if", etc.
func hasToken(node *parser.ASTNode, value string) bool {
	for _, child := range node.Children {
		if t, ok := child.(lexer.Token); ok {
			if t.Value == value {
				return true
			}
		}
	}
	return false
}

// tokenTypeName returns the effective type name for a token.
// Grammar-driven lexers set TypeName (e.g., "INT", "FLOAT", "STRING").
// The base lexer uses the Type enum.
func tokenTypeName(tok lexer.Token) string {
	if tok.TypeName != "" {
		return tok.TypeName
	}
	switch tok.Type {
	case lexer.TokenName:
		return "NAME"
	case lexer.TokenNumber:
		return "NUMBER"
	case lexer.TokenString:
		return "STRING"
	case lexer.TokenKeyword:
		return "KEYWORD"
	default:
		return tok.TypeName
	}
}

// extractSimpleName extracts a variable name from an expression AST.
// For a simple name reference like "x", this traverses the expression
// tree down to the atom containing the NAME token.
//
// AST path: expression_list -> expression -> or_expr -> and_expr ->
//
//	not_expr -> comparison -> bitwise_or -> bitwise_xor ->
//	bitwise_and -> shift -> arith -> term -> factor ->
//	power -> primary -> atom -> NAME token
//
// Returns empty string if the expression is not a simple name.
func extractSimpleName(node *parser.ASTNode) string {
	// Walk down single-child paths until we find a NAME token
	current := node
	for current != nil {
		// Check if this node directly contains a NAME token
		for _, child := range current.Children {
			if tok, ok := child.(lexer.Token); ok {
				tn := tokenTypeName(tok)
				if tn == "NAME" {
					return tok.Value
				}
			}
		}
		// If there's exactly one child node, descend into it
		nodes := extractNodes(current)
		if len(nodes) == 1 {
			current = nodes[0]
		} else {
			break
		}
	}
	return ""
}

// parseStringLiteral handles string values from the lexer.
//
// The Starlark grammar-driven lexer already strips quotes and processes
// escape sequences for simple strings (those without prefixes like r"" or b"").
// For prefixed strings, the lexer leaves the value as-is (e.g., r"hello\n").
//
// This function handles both cases:
//   - If the value starts with a quote or prefix character, strip prefix/quotes
//   - If the value is already bare content (no quotes), return as-is
//
// Starlark strings can be:
//   - Single-quoted:  'hello'
//   - Double-quoted:  "hello"
//   - Triple-quoted:  ”'hello”' or """hello"""
//   - Raw prefixed:   r"hello\n"  (backslashes are literal)
//   - Byte prefixed:  b"hello"
func parseStringLiteral(s string) string {
	if len(s) == 0 {
		return s
	}

	// Check if this string still has its quotes (lexer didn't strip them).
	// The grammar lexer strips quotes when the first character is a quote char.
	// Prefixed strings (r"...", b"...", rb"...") still have their prefix + quotes.
	//
	// IMPORTANT: we must check for quotes AFTER any potential prefix characters.
	// A string like "build" (already stripped by lexer) starts with 'b' but has
	// no quotes — it's a bare value, not a b"uild" byte-string prefix.
	// Previously this function would incorrectly strip 'b'/'r' from values
	// like "build", "run", "rake", "bundle" etc.
	firstChar := s[0]
	hasQuotes := firstChar == '"' || firstChar == '\''

	if hasQuotes {
		// String still has quotes — strip them below.
	} else {
		// Check if it's a prefixed string (e.g., r"hello" or b"data").
		// A prefix is only valid if it's followed by a quote character.
		stripped := s
		for len(stripped) > 0 && (stripped[0] == 'r' || stripped[0] == 'R' || stripped[0] == 'b' || stripped[0] == 'B') {
			stripped = stripped[1:]
		}
		if len(stripped) == 0 || (stripped[0] != '"' && stripped[0] != '\'') {
			// No quotes after the prefix characters — the lexer already
			// stripped quotes. The leading b/r/B/R is part of the value.
			return s
		}
		// Fall through to prefix+quote stripping below.
	}

	if hasQuotes {
		// String still has quotes — strip them below.
	} else {
		// Check if it's a prefixed string (e.g., r"hello" or b"data").
		// A prefix is only valid if it's followed by a quote character.
		stripped := s
		for len(stripped) > 0 && (stripped[0] == 'r' || stripped[0] == 'R' || stripped[0] == 'b' || stripped[0] == 'B') {
			stripped = stripped[1:]
		}
		if len(stripped) == 0 || (stripped[0] != '"' && stripped[0] != '\'') {
			// No quotes after the prefix characters — the lexer already
			// stripped quotes. The leading b/r/B/R is part of the value.
			return s
		}
		// Fall through to prefix+quote stripping below.
	}

	// Strip optional prefix (r, b, rb, br, R, B, etc.)
	raw := false
	for len(s) > 0 && (s[0] == 'r' || s[0] == 'R' || s[0] == 'b' || s[0] == 'B') {
		if s[0] == 'r' || s[0] == 'R' {
			raw = true
		}
		s = s[1:]
	}

	// Strip triple or single quotes
	if strings.HasPrefix(s, `"""`) && strings.HasSuffix(s, `"""`) {
		s = s[3 : len(s)-3]
	} else if strings.HasPrefix(s, `'''`) && strings.HasSuffix(s, `'''`) {
		s = s[3 : len(s)-3]
	} else if len(s) >= 2 && (s[0] == '"' || s[0] == '\'') {
		s = s[1 : len(s)-1]
	}

	if raw {
		return s
	}

	// Process escape sequences
	var result strings.Builder
	i := 0
	for i < len(s) {
		if s[i] == '\\' && i+1 < len(s) {
			switch s[i+1] {
			case 'n':
				result.WriteByte('\n')
			case 't':
				result.WriteByte('\t')
			case 'r':
				result.WriteByte('\r')
			case '\\':
				result.WriteByte('\\')
			case '\'':
				result.WriteByte('\'')
			case '"':
				result.WriteByte('"')
			case '0':
				result.WriteByte(0)
			default:
				result.WriteByte('\\')
				result.WriteByte(s[i+1])
			}
			i += 2
		} else {
			result.WriteByte(s[i])
			i++
		}
	}
	return result.String()
}

// ════════════════════════════════════════════════════════════════════════
// BINARY AND COMPARISON OPERATOR MAPPINGS
// ════════════════════════════════════════════════════════════════════════
//
// These maps connect the operator token value to the appropriate opcode.
// When the compiler sees a "+" token in an arith node, it looks up "+"
// in binaryOpMap to find OpAdd.

var binaryOpMap = map[string]vm.OpCode{
	"+":  OpAdd,
	"-":  OpSub,
	"*":  OpMul,
	"/":  OpDiv,
	"//": OpFloorDiv,
	"%":  OpMod,
	"**": OpPower,
	"<<": OpLShift,
	">>": OpRShift,
	"&":  OpBitAnd,
	"|":  OpBitOr,
	"^":  OpBitXor,
}

var compareOpMap = map[string]vm.OpCode{
	"==":     OpCmpEq,
	"!=":     OpCmpNe,
	"<":      OpCmpLt,
	">":      OpCmpGt,
	"<=":     OpCmpLe,
	">=":     OpCmpGe,
	"in":     OpCmpIn,
	"not in": OpCmpNotIn,
}

// augmentedAssignOpMap maps augmented assignment operators (+=, -=, etc.)
// to their corresponding binary operation opcode.
var augmentedAssignOpMap = map[string]vm.OpCode{
	"+=":  OpAdd,
	"-=":  OpSub,
	"*=":  OpMul,
	"/=":  OpDiv,
	"//=": OpFloorDiv,
	"%=":  OpMod,
	"**=": OpPower,
	"<<=": OpLShift,
	">>=": OpRShift,
	"&=":  OpBitAnd,
	"|=":  OpBitOr,
	"^=":  OpBitXor,
}

// ════════════════════════════════════════════════════════════════════════
// PUBLIC API
// ════════════════════════════════════════════════════════════════════════

// CompileStarlark is the one-shot entry point: source code in, CodeObject out.
//
// It performs the complete pipeline:
//  1. Lex the source into tokens (via starlark-lexer)
//  2. Parse the tokens into an AST (via starlark-parser)
//  3. Compile the AST into bytecode (this package)
//
// Returns an error if lexing, parsing, or compilation fails.
//
// Usage:
//
//	code, err := CompileStarlark("x = 1 + 2")
//	// code.Instructions = [LOAD_CONST 0, LOAD_CONST 1, ADD, STORE_NAME 0, HALT]
//	// code.Constants = [1, 2]
//	// code.Names = ["x"]
func CompileStarlark(source string) (vm.CodeObject, error) {
	return StartNew[vm.CodeObject]("starlark-ast-to-bytecode-compiler.CompileStarlark", vm.CodeObject{},
		func(op *Operation[vm.CodeObject], rf *ResultFactory[vm.CodeObject]) *OperationResult[vm.CodeObject] {
			op.AddProperty("sourceLen", len(source))
			ast, err := starlarkparser.ParseStarlark(source)
			if err != nil {
				return rf.Fail(vm.CodeObject{}, fmt.Errorf("parse error: %w", err))
			}

			compiler := NewStarlarkCompiler()
			compiler.compileNode(ast)
			compiler.emit(OpHalt)

			return rf.Generate(true, false, vm.CodeObject{
				Instructions: compiler.instructions,
				Constants:    compiler.constants,
				Names:        compiler.names,
			})
		}).GetResult()
}

// CompileAST compiles a pre-parsed AST into a CodeObject.
// Useful when you already have an AST and want to skip re-parsing.
func CompileAST(ast *parser.ASTNode) vm.CodeObject {
	result, _ := StartNew[vm.CodeObject]("starlark-ast-to-bytecode-compiler.CompileAST", vm.CodeObject{},
		func(op *Operation[vm.CodeObject], rf *ResultFactory[vm.CodeObject]) *OperationResult[vm.CodeObject] {
			compiler := NewStarlarkCompiler()
			compiler.compileNode(ast)
			compiler.emit(OpHalt)
			return rf.Generate(true, false, vm.CodeObject{
				Instructions: compiler.instructions,
				Constants:    compiler.constants,
				Names:        compiler.names,
			})
		}).GetResult()
	return result
}

// ════════════════════════════════════════════════════════════════════════
// NODE DISPATCH — The main recursive compilation function
// ════════════════════════════════════════════════════════════════════════

// compileNode dispatches compilation based on the AST node's rule name.
//
// This is the central switch statement of the compiler. Each case
// corresponds to a grammar rule from starlark.grammar. The cases
// are organized in the same order as the grammar file for easy
// cross-referencing.
func (c *StarlarkCompiler) compileNode(node *parser.ASTNode) {
	switch node.RuleName {

	// ── Top-level and statement containers ─────────────────────────
	case "file":
		c.compileFile(node)
	case "statement":
		c.compileStatement(node)
	case "simple_stmt":
		c.compileSimpleStmt(node)
	case "small_stmt":
		c.compileSmallStmt(node)

	// ── Simple statements ──────────────────────────────────────────
	case "assign_stmt":
		c.compileAssignStmt(node)
	case "return_stmt":
		c.compileReturnStmt(node)
	case "break_stmt":
		c.emit(OpBreak)
	case "continue_stmt":
		c.emit(OpContinue)
	case "pass_stmt":
		// pass is a no-op — emit nothing
	case "load_stmt":
		c.compileLoadStmt(node)

	// ── Compound statements ────────────────────────────────────────
	case "if_stmt":
		c.compileIfStmt(node)
	case "for_stmt":
		c.compileForStmt(node)
	case "def_stmt":
		c.compileDefStmt(node)
	case "suite":
		c.compileSuite(node)

	// ── Expressions ────────────────────────────────────────────────
	case "expression":
		c.compileExpression(node)
	case "expression_list":
		c.compileExpressionList(node)
	case "or_expr":
		c.compileOrExpr(node)
	case "and_expr":
		c.compileAndExpr(node)
	case "not_expr":
		c.compileNotExpr(node)
	case "comparison":
		c.compileComparison(node)
	case "bitwise_or":
		c.compileBinaryChain(node, "|")
	case "bitwise_xor":
		c.compileBinaryChain(node, "^")
	case "bitwise_and":
		c.compileBinaryChain(node, "&")
	case "shift":
		c.compileBinaryChainMultiOp(node)
	case "arith":
		c.compileBinaryChainMultiOp(node)
	case "term":
		c.compileBinaryChainMultiOp(node)
	case "factor":
		c.compileFactor(node)
	case "power":
		c.compilePower(node)
	case "primary":
		c.compilePrimary(node)
	case "atom":
		c.compileAtom(node)

	// ── Collection literals ────────────────────────────────────────
	case "list_expr":
		c.compileListExpr(node)
	case "list_body":
		c.compileListBody(node)
	case "dict_expr":
		c.compileDictExpr(node)
	case "dict_body":
		c.compileDictBody(node)
	case "dict_entry":
		c.compileDictEntry(node)
	case "paren_expr":
		c.compileParenExpr(node)
	case "paren_body":
		c.compileParenBody(node)

	// ── Lambda ─────────────────────────────────────────────────────
	case "lambda_expr":
		c.compileLambdaExpr(node)

	// ── Fallthrough for wrapper rules ──────────────────────────────
	// Some grammar rules are "pass-through" — they exist for precedence
	// or readability but simply contain a single child. We compile that child.
	default:
		nodes := extractNodes(node)
		if len(nodes) == 1 {
			c.compileNode(nodes[0])
		}
	}
}

// ════════════════════════════════════════════════════════════════════════
// STATEMENT COMPILATION
// ════════════════════════════════════════════════════════════════════════

// compileFile compiles the top-level "file" rule.
// A file is a sequence of statements separated by newlines.
// Grammar: file = { NEWLINE | statement } ;
func (c *StarlarkCompiler) compileFile(node *parser.ASTNode) {
	for _, child := range node.Children {
		if n, ok := child.(*parser.ASTNode); ok {
			c.compileNode(n)
		}
		// NEWLINE tokens are skipped
	}
}

// compileStatement dispatches between simple_stmt and compound_stmt.
// Grammar: statement = compound_stmt | simple_stmt ;
func (c *StarlarkCompiler) compileStatement(node *parser.ASTNode) {
	for _, child := range node.Children {
		if n, ok := child.(*parser.ASTNode); ok {
			c.compileNode(n)
		}
	}
}

// compileSimpleStmt compiles one or more small statements on a line.
// Grammar: simple_stmt = small_stmt { SEMICOLON small_stmt } NEWLINE ;
func (c *StarlarkCompiler) compileSimpleStmt(node *parser.ASTNode) {
	for _, child := range node.Children {
		if n, ok := child.(*parser.ASTNode); ok {
			c.compileNode(n)
		}
	}
}

// compileSmallStmt dispatches to the specific simple statement type.
// Grammar: small_stmt = return_stmt | break_stmt | continue_stmt | pass_stmt | load_stmt | assign_stmt ;
func (c *StarlarkCompiler) compileSmallStmt(node *parser.ASTNode) {
	for _, child := range node.Children {
		if n, ok := child.(*parser.ASTNode); ok {
			c.compileNode(n)
		}
	}
}

// compileAssignStmt handles assignment, augmented assignment, and expression statements.
//
// Grammar: assign_stmt = expression_list [ ( assign_op | augmented_assign_op ) expression_list ] ;
//
// Three cases:
//  1. x = expr           (simple assignment)
//  2. x += expr          (augmented assignment)
//  3. expr               (expression statement — result discarded)
//
// For case 1:
//
//	Compile RHS, then emit STORE_NAME/STORE_LOCAL for the LHS target.
//
// For case 2 (e.g., x += 1):
//
//	Compile: LOAD_NAME x, LOAD_CONST 1, ADD, STORE_NAME x
//
// For case 3:
//
//	Compile the expression, then emit POP to discard the result.
//	(Expression statements are used for side effects like function calls.)
func (c *StarlarkCompiler) compileAssignStmt(node *parser.ASTNode) {
	nodes := extractNodes(node)
	tokens := extractTokens(node)

	if len(nodes) == 1 && len(tokens) == 0 {
		// Case 3: bare expression statement
		c.compileNode(nodes[0])
		c.emit(OpPop)
		return
	}

	if len(nodes) >= 2 {
		// Find the operator token
		var opToken *lexer.Token
		for _, child := range node.Children {
			if n, ok := child.(*parser.ASTNode); ok {
				if n.RuleName == "assign_op" || n.RuleName == "augmented_assign_op" {
					opTokens := extractTokens(n)
					if len(opTokens) > 0 {
						opToken = &opTokens[0]
					}
				}
			}
		}

		if opToken != nil && opToken.Value == "=" {
			// Case 1: simple assignment (x = expr)
			// Compile RHS first (last expression_list node)
			c.compileNode(nodes[len(nodes)-1])
			// Store to LHS target
			c.compileStoreTarget(nodes[0])
			return
		}

		if opToken != nil {
			// Case 2: augmented assignment (x += expr)
			binOp, ok := augmentedAssignOpMap[opToken.Value]
			if ok {
				// Load current value
				name := extractSimpleName(nodes[0])
				if name != "" {
					nameIdx := c.addName(name)
					if c.scopeDepth > 0 {
						c.emit(OpLoadLocal, nameIdx)
					} else {
						c.emit(OpLoadName, nameIdx)
					}
				}
				// Compile RHS
				c.compileNode(nodes[len(nodes)-1])
				// Apply operation
				c.emit(binOp)
				// Store back
				c.compileStoreTarget(nodes[0])
				return
			}
		}
	}

	// Fallback: if we have a single node with tokens, try to handle it
	if len(nodes) == 1 {
		c.compileNode(nodes[0])
		c.emit(OpPop)
	}
}

// compileStoreTarget emits a STORE instruction for an assignment target.
//
// The target can be:
//   - A simple name:     x = ...       -> STORE_NAME
//   - A dotted name:     obj.x = ...   -> STORE_ATTR
//   - A subscript:       lst[i] = ...  -> STORE_SUBSCRIPT
//
// For simple names, we use STORE_NAME (global) or STORE_LOCAL (in function).
func (c *StarlarkCompiler) compileStoreTarget(target *parser.ASTNode) {
	name := extractSimpleName(target)
	if name != "" {
		nameIdx := c.addName(name)
		if c.scopeDepth > 0 {
			c.emit(OpStoreLocal, nameIdx)
		} else {
			c.emit(OpStoreName, nameIdx)
		}
	}
}

// compileReturnStmt compiles a return statement.
// Grammar: return_stmt = "return" [ expression ] ;
//
// If there's no expression, we return None. In Starlark, every function
// implicitly returns None if it falls through without a return statement,
// and "return" without a value also returns None.
func (c *StarlarkCompiler) compileReturnStmt(node *parser.ASTNode) {
	nodes := extractNodes(node)
	if len(nodes) > 0 {
		c.compileNode(nodes[0])
	} else {
		c.emit(OpLoadNone)
	}
	c.emit(OpReturnValue)
}

// compileLoadStmt compiles a load() statement.
// Grammar: load_stmt = "load" LPAREN STRING { COMMA load_arg } [ COMMA ] RPAREN ;
//
// Starlark's load() imports symbols from another module:
//
//	load("module.star", "symbol1", alias = "symbol2")
//
// Compilation:
//  1. LOAD_MODULE (push the module path as a constant)
//  2. For each imported symbol:
//     a. DUP the module object
//     b. IMPORT_FROM (extract the symbol)
//     c. STORE_NAME (bind to local/alias name)
//  3. POP the module object
func (c *StarlarkCompiler) compileLoadStmt(node *parser.ASTNode) {
	tokens := extractTokens(node)
	loadArgNodes := []*parser.ASTNode{}
	for _, child := range node.Children {
		if n, ok := child.(*parser.ASTNode); ok {
			if n.RuleName == "load_arg" {
				loadArgNodes = append(loadArgNodes, n)
			}
		}
	}

	// First STRING token is the module path
	var modulePath string
	for _, tok := range tokens {
		tn := tokenTypeName(tok)
		if tn == "STRING" {
			modulePath = parseStringLiteral(tok.Value)
			break
		}
	}

	// Emit LOAD_MODULE
	moduleIdx := c.addConstant(modulePath)
	c.emit(OpLoadModule, moduleIdx)

	// Process each load_arg
	for _, arg := range loadArgNodes {
		argTokens := extractTokens(arg)
		c.emit(OpDup)

		if len(argTokens) >= 3 {
			// Alias form: local_name = "remote_name"
			localName := argTokens[0].Value
			remoteName := parseStringLiteral(argTokens[2].Value)
			remoteIdx := c.addName(remoteName)
			c.emit(OpImportFrom, remoteIdx)
			localIdx := c.addName(localName)
			c.emit(OpStoreName, localIdx)
		} else if len(argTokens) >= 1 {
			// Simple form: "symbol_name"
			tn := tokenTypeName(argTokens[0])
			if tn == "STRING" {
				symbolName := parseStringLiteral(argTokens[0].Value)
				symIdx := c.addName(symbolName)
				c.emit(OpImportFrom, symIdx)
				c.emit(OpStoreName, symIdx)
			}
		}
	}

	// Pop the module object
	c.emit(OpPop)
}

// ════════════════════════════════════════════════════════════════════════
// COMPOUND STATEMENT COMPILATION
// ════════════════════════════════════════════════════════════════════════

// compileIfStmt compiles if/elif/else chains.
// Grammar: if_stmt = "if" expression COLON suite { "elif" expression COLON suite } [ "else" COLON suite ] ;
//
// The bytecode pattern for if/elif/else:
//
//	compile condition1
//	JUMP_IF_FALSE -> elif1 (or else, or end)
//	compile body1
//	JUMP -> end
//	elif1:
//	compile condition2
//	JUMP_IF_FALSE -> else (or end)
//	compile body2
//	JUMP -> end
//	else:
//	compile else_body
//	end:
//
// This is a forward-branching pattern. We use placeholder jumps that
// get patched once we know the target addresses.
func (c *StarlarkCompiler) compileIfStmt(node *parser.ASTNode) {
	// Collect the structure: pairs of (condition, body) plus optional else body.
	// The children look like: "if" expr COLON suite {"elif" expr COLON suite} ["else" COLON suite]
	type branch struct {
		condition *parser.ASTNode // nil for else branch
		body      *parser.ASTNode
	}

	var branches []branch
	var currentCondition *parser.ASTNode
	expectingCondition := false
	expectingBody := false

	for _, child := range node.Children {
		if tok, ok := child.(lexer.Token); ok {
			if tok.Value == "if" || tok.Value == "elif" {
				expectingCondition = true
				continue
			}
			if tok.Value == "else" {
				currentCondition = nil
				expectingBody = true
				continue
			}
			if tok.Value == ":" {
				if expectingCondition {
					expectingCondition = false
				}
				expectingBody = true
				continue
			}
		}
		if n, ok := child.(*parser.ASTNode); ok {
			if expectingCondition || (currentCondition == nil && !expectingBody && n.RuleName != "suite") {
				currentCondition = n
				expectingCondition = false
				continue
			}
			if expectingBody || n.RuleName == "suite" {
				branches = append(branches, branch{condition: currentCondition, body: n})
				currentCondition = nil
				expectingBody = false
				continue
			}
		}
	}

	// Compile the branches
	var endJumps []int

	for i, br := range branches {
		if br.condition != nil {
			// Compile condition
			c.compileNode(br.condition)
			// Jump past this branch if condition is false
			falseJump := c.emitJump(OpJumpIfFalse)

			// Compile body
			c.compileNode(br.body)

			// Jump to end (skip remaining elif/else branches)
			if i < len(branches)-1 {
				endJump := c.emitJump(OpJump)
				endJumps = append(endJumps, endJump)
			}

			// Patch the false jump to point here
			c.patchJump(falseJump)
		} else {
			// else branch — no condition
			c.compileNode(br.body)
		}
	}

	// Patch all end jumps to point here
	for _, j := range endJumps {
		c.patchJump(j)
	}
}

// compileForStmt compiles a for loop.
// Grammar: for_stmt = "for" loop_vars "in" expression COLON suite ;
//
// Bytecode pattern:
//
//	compile iterable_expression
//	GET_ITER
//	loop_top:
//	FOR_ITER -> loop_exit      (jump if iterator exhausted)
//	STORE_NAME loop_var        (or UNPACK_SEQUENCE for multiple vars)
//	compile loop_body
//	JUMP -> loop_top
//	loop_exit:
//
// The FOR_ITER instruction checks if the iterator has more values.
// If yes, it pushes the next value and continues. If exhausted, it
// jumps to the exit address (removing the iterator from the stack).
func (c *StarlarkCompiler) compileForStmt(node *parser.ASTNode) {
	// Extract: loop_vars node, expression node, suite node
	var loopVarsNode, exprNode, suiteNode *parser.ASTNode
	expectingVars := false
	expectingExpr := false
	expectingSuite := false

	for _, child := range node.Children {
		if tok, ok := child.(lexer.Token); ok {
			if tok.Value == "for" {
				expectingVars = true
				continue
			}
			if tok.Value == "in" {
				expectingVars = false
				expectingExpr = true
				continue
			}
			if tok.Value == ":" {
				expectingExpr = false
				expectingSuite = true
				continue
			}
		}
		if n, ok := child.(*parser.ASTNode); ok {
			if expectingVars && loopVarsNode == nil {
				loopVarsNode = n
				continue
			}
			if expectingExpr && exprNode == nil {
				exprNode = n
				continue
			}
			if expectingSuite && suiteNode == nil {
				suiteNode = n
				continue
			}
		}
	}

	if exprNode == nil || suiteNode == nil {
		return
	}

	// Compile the iterable expression
	c.compileNode(exprNode)
	c.emit(OpGetIter)

	// Loop header: FOR_ITER with placeholder exit address
	loopTop := c.currentOffset()
	exitJump := c.emitJump(OpForIter)

	// Store loop variable(s)
	if loopVarsNode != nil {
		varTokens := extractTokens(loopVarsNode)
		if len(varTokens) > 1 {
			// Multiple loop variables: unpack
			// Count only NAME tokens
			nameCount := 0
			for _, tok := range varTokens {
				tn := tokenTypeName(tok)
				if tn == "NAME" {
					nameCount++
				}
			}
			c.emit(OpUnpackSequence, nameCount)
			for _, tok := range varTokens {
				tn := tokenTypeName(tok)
				if tn == "NAME" {
					nameIdx := c.addName(tok.Value)
					if c.scopeDepth > 0 {
						c.emit(OpStoreLocal, nameIdx)
					} else {
						c.emit(OpStoreName, nameIdx)
					}
				}
			}
		} else if len(varTokens) == 1 {
			nameIdx := c.addName(varTokens[0].Value)
			if c.scopeDepth > 0 {
				c.emit(OpStoreLocal, nameIdx)
			} else {
				c.emit(OpStoreName, nameIdx)
			}
		}
	}

	// Compile loop body
	c.compileNode(suiteNode)

	// Jump back to loop header
	c.emit(OpJump, loopTop)

	// Patch the exit jump
	c.patchJump(exitJump)
}

// compileDefStmt compiles a function definition.
// Grammar: def_stmt = "def" NAME LPAREN [ parameters ] RPAREN COLON suite ;
//
// Function definitions are compiled into TWO parts:
//
//  1. The function body is compiled into a separate CodeObject
//     (a nested compilation with its own instructions/constants/names).
//
//  2. In the outer scope, we emit:
//     - Any default argument values (pushed as constants)
//     - LOAD_CONST (the function's CodeObject)
//     - MAKE_FUNCTION (creates a function object from CodeObject + defaults)
//     - STORE_NAME (binds the function to its name)
//
// This mirrors how Python compiles function definitions: the def statement
// is an executable statement that creates a function object and assigns it
// to a variable.
func (c *StarlarkCompiler) compileDefStmt(node *parser.ASTNode) {
	// Extract function name, parameters, and body
	var funcName string
	var paramsNode *parser.ASTNode
	var suiteNode *parser.ASTNode

	for _, child := range node.Children {
		if tok, ok := child.(lexer.Token); ok {
			tn := tokenTypeName(tok)
			if tn == "NAME" && funcName == "" {
				funcName = tok.Value
			}
		}
		if n, ok := child.(*parser.ASTNode); ok {
			if n.RuleName == "parameters" {
				paramsNode = n
			} else if n.RuleName == "suite" {
				suiteNode = n
			}
		}
	}

	if suiteNode == nil {
		return
	}

	// Extract parameter names and default values
	var paramNames []string
	var defaultCount int
	if paramsNode != nil {
		for _, child := range paramsNode.Children {
			if paramNode, ok := child.(*parser.ASTNode); ok {
				if paramNode.RuleName == "parameter" {
					paramTokens := extractTokens(paramNode)
					paramSubnodes := extractNodes(paramNode)
					for _, pt := range paramTokens {
						ptn := tokenTypeName(pt)
						if ptn == "NAME" {
							paramNames = append(paramNames, pt.Value)
							break
						}
					}
					// Check for default value
					if hasToken(paramNode, "=") && len(paramSubnodes) > 0 {
						c.compileNode(paramSubnodes[0])
						defaultCount++
					}
				}
			}
		}
	}

	// Compile the function body into a nested CodeObject
	bodyCompiler := NewStarlarkCompiler()
	bodyCompiler.scopeDepth = c.scopeDepth + 1
	bodyCompiler.compileNode(suiteNode)
	// Ensure the function returns None if no explicit return
	bodyCompiler.emit(OpLoadNone)
	bodyCompiler.emit(OpReturnValue)

	bodyCode := vm.CodeObject{
		Instructions: bodyCompiler.instructions,
		Constants:    bodyCompiler.constants,
		Names:        bodyCompiler.names,
	}

	// Store parameter info with the CodeObject
	funcInfo := map[string]interface{}{
		"code":          bodyCode,
		"params":        paramNames,
		"default_count": defaultCount,
	}

	// Emit LOAD_CONST for the function info, then MAKE_FUNCTION
	funcIdx := c.addConstant(funcInfo)
	c.emit(OpMakeFunction, funcIdx)

	// Bind the function to its name
	nameIdx := c.addName(funcName)
	if c.scopeDepth > 0 {
		c.emit(OpStoreLocal, nameIdx)
	} else {
		c.emit(OpStoreName, nameIdx)
	}
}

// compileSuite compiles the body of a compound statement.
// Grammar: suite = simple_stmt | NEWLINE INDENT { statement } DEDENT ;
//
// A suite can be either:
//   - A single simple statement on the same line: if True: pass
//   - An indented block of statements:
//     if True:
//     x = 1
//     y = 2
func (c *StarlarkCompiler) compileSuite(node *parser.ASTNode) {
	for _, child := range node.Children {
		if n, ok := child.(*parser.ASTNode); ok {
			c.compileNode(n)
		}
	}
}

// ════════════════════════════════════════════════════════════════════════
// EXPRESSION COMPILATION
// ════════════════════════════════════════════════════════════════════════

// compileExpression compiles a top-level expression.
// Grammar: expression = lambda_expr | or_expr [ "if" or_expr "else" expression ] ;
//
// The "if" form is the ternary conditional expression:
//
//	value = x if condition else y
//
// Bytecode for ternary:
//
//	compile condition (the middle expression)
//	JUMP_IF_FALSE -> else_branch
//	compile true_value (the left expression)
//	JUMP -> end
//	else_branch:
//	compile false_value (the right expression)
//	end:
func (c *StarlarkCompiler) compileExpression(node *parser.ASTNode) {
	nodes := extractNodes(node)

	// Check for lambda
	if len(nodes) > 0 && nodes[0].RuleName == "lambda_expr" {
		c.compileNode(nodes[0])
		return
	}

	// Check for ternary: or_expr "if" or_expr "else" expression
	if len(nodes) >= 3 && hasToken(node, "if") && hasToken(node, "else") {
		// nodes[0] = true value, nodes[1] = condition, nodes[2] = false value
		c.compileNode(nodes[1]) // condition
		falseJump := c.emitJump(OpJumpIfFalse)
		c.compileNode(nodes[0]) // true value
		endJump := c.emitJump(OpJump)
		c.patchJump(falseJump)
		c.compileNode(nodes[2]) // false value
		c.patchJump(endJump)
		return
	}

	// Simple expression — compile the single child
	if len(nodes) >= 1 {
		c.compileNode(nodes[0])
	}
}

// compileExpressionList compiles an expression list (for tuples and multi-assignment).
// Grammar: expression_list = expression { COMMA expression } [ COMMA ] ;
//
// If there's only one expression (no commas), compile it directly.
// If there are multiple expressions, compile each and emit BUILD_TUPLE.
func (c *StarlarkCompiler) compileExpressionList(node *parser.ASTNode) {
	nodes := extractNodes(node)
	if len(nodes) == 1 {
		c.compileNode(nodes[0])
		return
	}
	// Multiple expressions -> build a tuple
	for _, n := range nodes {
		c.compileNode(n)
	}
	if len(nodes) > 1 {
		c.emit(OpBuildTuple, len(nodes))
	}
}

// compileOrExpr compiles boolean OR with short-circuit evaluation.
// Grammar: or_expr = and_expr { "or" and_expr } ;
//
// Short-circuit semantics: "a or b" evaluates a, and if truthy,
// returns a without evaluating b.
//
// Bytecode pattern:
//
//	compile a
//	JUMP_IF_TRUE_OR_POP -> end   (if a is truthy, keep it and skip b)
//	compile b
//	end:
//
// For chained: a or b or c
//
//	compile a
//	JUMP_IF_TRUE_OR_POP -> end
//	compile b
//	JUMP_IF_TRUE_OR_POP -> end
//	compile c
//	end:
func (c *StarlarkCompiler) compileOrExpr(node *parser.ASTNode) {
	nodes := extractNodes(node)
	if len(nodes) == 1 {
		c.compileNode(nodes[0])
		return
	}

	var jumps []int
	for i, n := range nodes {
		c.compileNode(n)
		if i < len(nodes)-1 {
			jump := c.emitJump(OpJumpIfTrueOrPop)
			jumps = append(jumps, jump)
		}
	}
	for _, j := range jumps {
		c.patchJump(j)
	}
}

// compileAndExpr compiles boolean AND with short-circuit evaluation.
// Grammar: and_expr = not_expr { "and" not_expr } ;
//
// Short-circuit: "a and b" evaluates a, and if falsy, returns a
// without evaluating b.
//
// Bytecode pattern:
//
//	compile a
//	JUMP_IF_FALSE_OR_POP -> end   (if a is falsy, keep it and skip b)
//	compile b
//	end:
func (c *StarlarkCompiler) compileAndExpr(node *parser.ASTNode) {
	nodes := extractNodes(node)
	if len(nodes) == 1 {
		c.compileNode(nodes[0])
		return
	}

	var jumps []int
	for i, n := range nodes {
		c.compileNode(n)
		if i < len(nodes)-1 {
			jump := c.emitJump(OpJumpIfFalseOrPop)
			jumps = append(jumps, jump)
		}
	}
	for _, j := range jumps {
		c.patchJump(j)
	}
}

// compileNotExpr compiles logical NOT.
// Grammar: not_expr = "not" not_expr | comparison ;
//
// If the node starts with "not", compile the operand and emit NOT.
// Otherwise, it's a comparison — pass through.
func (c *StarlarkCompiler) compileNotExpr(node *parser.ASTNode) {
	if hasToken(node, "not") {
		nodes := extractNodes(node)
		if len(nodes) > 0 {
			c.compileNode(nodes[0])
			c.emit(OpNot)
		}
		return
	}
	nodes := extractNodes(node)
	if len(nodes) > 0 {
		c.compileNode(nodes[0])
	}
}

// compileComparison compiles comparison operators.
// Grammar: comparison = bitwise_or { comp_op bitwise_or } ;
//
// For a single comparison (a < b):
//
//	compile a
//	compile b
//	CMP_LT
//
// For chained comparisons (a < b < c), Starlark forbids them,
// but we handle the grammar structure: compile pairs left-to-right.
func (c *StarlarkCompiler) compileComparison(node *parser.ASTNode) {
	// Interleaved: bitwise_or, comp_op, bitwise_or, comp_op, bitwise_or, ...
	var operands []*parser.ASTNode
	var operators []string

	for _, child := range node.Children {
		if n, ok := child.(*parser.ASTNode); ok {
			if n.RuleName == "comp_op" {
				// Extract operator string
				op := c.extractCompOp(n)
				operators = append(operators, op)
			} else {
				operands = append(operands, n)
			}
		}
	}

	if len(operators) == 0 {
		// No comparison — just a single bitwise_or expression
		if len(operands) > 0 {
			c.compileNode(operands[0])
		}
		return
	}

	// Compile first operand
	c.compileNode(operands[0])

	// For each operator, compile the next operand and emit the comparison
	for i, op := range operators {
		c.compileNode(operands[i+1])
		if opcode, ok := compareOpMap[op]; ok {
			c.emit(opcode)
		}
	}
}

// extractCompOp extracts the comparison operator string from a comp_op node.
// Grammar: comp_op = EQUALS_EQUALS | NOT_EQUALS | LESS_THAN | GREATER_THAN
//
//	| LESS_EQUALS | GREATER_EQUALS | "in" | "not" "in" ;
//
// Special case: "not" "in" is a two-token operator that we combine into "not in".
func (c *StarlarkCompiler) extractCompOp(node *parser.ASTNode) string {
	tokens := extractTokens(node)
	if len(tokens) == 2 && tokens[0].Value == "not" && tokens[1].Value == "in" {
		return "not in"
	}
	if len(tokens) >= 1 {
		return tokens[0].Value
	}
	return ""
}

// compileBinaryChain compiles a left-associative binary operator chain
// with a single operator type.
// Used for: bitwise_or (|), bitwise_xor (^), bitwise_and (&)
//
// Grammar pattern: rule = subrule { OP subrule } ;
//
// Bytecode:
//
//	compile first_operand
//	for each additional operand:
//	  compile operand
//	  emit OP
func (c *StarlarkCompiler) compileBinaryChain(node *parser.ASTNode, op string) {
	nodes := extractNodes(node)
	if len(nodes) == 0 {
		return
	}

	c.compileNode(nodes[0])
	opcode := binaryOpMap[op]
	for i := 1; i < len(nodes); i++ {
		c.compileNode(nodes[i])
		c.emit(opcode)
	}
}

// compileBinaryChainMultiOp compiles a left-associative binary operator chain
// where different operators may appear (e.g., arith uses both + and -).
//
// Grammar pattern: rule = subrule { ( OP1 | OP2 ) subrule } ;
//
// Children are interleaved: [node, token, node, token, node, ...]
// We compile the first operand, then for each (token, node) pair,
// compile the operand and emit the operator.
func (c *StarlarkCompiler) compileBinaryChainMultiOp(node *parser.ASTNode) {
	var operands []*parser.ASTNode
	var operators []string

	for _, child := range node.Children {
		if n, ok := child.(*parser.ASTNode); ok {
			operands = append(operands, n)
		}
		if tok, ok := child.(lexer.Token); ok {
			// Only consider operator tokens, skip NEWLINE etc.
			if _, exists := binaryOpMap[tok.Value]; exists {
				operators = append(operators, tok.Value)
			}
		}
	}

	if len(operands) == 0 {
		return
	}

	c.compileNode(operands[0])
	for i, op := range operators {
		if i+1 < len(operands) {
			c.compileNode(operands[i+1])
			if opcode, exists := binaryOpMap[op]; exists {
				c.emit(opcode)
			}
		}
	}
}

// compileFactor compiles unary prefix operators.
// Grammar: factor = ( PLUS | MINUS | TILDE ) factor | power ;
//
// Unary minus (-x) emits NEGATE.
// Unary plus (+x) is a no-op (but we still compile the operand).
// Bitwise not (~x) emits BIT_NOT.
func (c *StarlarkCompiler) compileFactor(node *parser.ASTNode) {
	nodes := extractNodes(node)
	tokens := extractTokens(node)

	// Check for unary operator
	if len(tokens) > 0 && len(nodes) > 0 {
		op := tokens[0].Value
		c.compileNode(nodes[0])
		switch op {
		case "-":
			c.emit(OpNegate)
		case "~":
			c.emit(OpBitNot)
		case "+":
			// unary plus is a no-op
		}
		return
	}

	// No unary operator — pass through to power
	if len(nodes) > 0 {
		c.compileNode(nodes[0])
	}
}

// compilePower compiles exponentiation.
// Grammar: power = primary [ DOUBLE_STAR factor ] ;
//
// If there's a ** operator:
//
//	compile base (primary)
//	compile exponent (factor)
//	POWER
func (c *StarlarkCompiler) compilePower(node *parser.ASTNode) {
	nodes := extractNodes(node)
	if len(nodes) == 0 {
		return
	}

	c.compileNode(nodes[0])
	if len(nodes) > 1 && hasToken(node, "**") {
		c.compileNode(nodes[1])
		c.emit(OpPower)
	}
}

// compilePrimary compiles a primary expression with optional suffixes.
// Grammar: primary = atom { suffix } ;
// suffix = DOT NAME | LBRACKET subscript RBRACKET | LPAREN [ arguments ] RPAREN ;
//
// The atom is compiled first, then each suffix modifies the result:
//   - .attr    -> LOAD_ATTR
//   - [index]  -> LOAD_SUBSCRIPT
//   - (args)   -> CALL_FUNCTION
func (c *StarlarkCompiler) compilePrimary(node *parser.ASTNode) {
	// Separate atom from suffix nodes
	var atomNode *parser.ASTNode
	var suffixNodes []*parser.ASTNode

	for _, child := range node.Children {
		if n, ok := child.(*parser.ASTNode); ok {
			if atomNode == nil && n.RuleName != "suffix" {
				atomNode = n
			} else {
				suffixNodes = append(suffixNodes, n)
			}
		}
	}

	if atomNode != nil {
		c.compileNode(atomNode)
	}

	// Compile each suffix
	for _, suffix := range suffixNodes {
		c.compileSuffix(suffix)
	}
}

// compileSuffix compiles a single suffix (dot, subscript, or call).
func (c *StarlarkCompiler) compileSuffix(node *parser.ASTNode) {
	tokens := extractTokens(node)
	nodes := extractNodes(node)

	if hasToken(node, ".") {
		// Attribute access: .NAME
		for _, tok := range tokens {
			tn := tokenTypeName(tok)
			if tn == "NAME" {
				nameIdx := c.addName(tok.Value)
				c.emit(OpLoadAttr, nameIdx)
				return
			}
		}
		return
	}

	if hasToken(node, "[") {
		// Subscript: [expr] or [slice]
		if len(nodes) > 0 {
			subscriptNode := nodes[0]
			if subscriptNode.RuleName == "subscript" {
				c.compileSubscript(subscriptNode)
			} else {
				c.compileNode(subscriptNode)
				c.emit(OpLoadSubscript)
			}
		}
		return
	}

	if hasToken(node, "(") {
		// Function call: (args)
		if len(nodes) > 0 {
			c.compileArguments(nodes[0])
		} else {
			// No arguments: f()
			c.emit(OpCallFunction, 0)
		}
		return
	}
}

// compileSubscript compiles a subscript expression (index or slice).
// Grammar: subscript = expression | [ expression ] COLON [ expression ] [ COLON [ expression ] ] ;
func (c *StarlarkCompiler) compileSubscript(node *parser.ASTNode) {
	if hasToken(node, ":") {
		// Slice syntax — for now, emit as LoadSlice
		nodes := extractNodes(node)
		sliceArgs := 0
		for _, n := range nodes {
			c.compileNode(n)
			sliceArgs++
		}
		// Fill missing args with None
		for sliceArgs < 2 {
			c.emit(OpLoadNone)
			sliceArgs++
		}
		c.emit(OpLoadSlice, sliceArgs)
	} else {
		// Simple index
		nodes := extractNodes(node)
		if len(nodes) > 0 {
			c.compileNode(nodes[0])
		}
		c.emit(OpLoadSubscript)
	}
}

// compileArguments compiles function call arguments.
// Grammar: arguments = argument { COMMA argument } [ COMMA ] ;
//
// We need to count positional and keyword arguments separately.
// Positional args are pushed onto the stack, then CALL_FUNCTION.
// Keyword args trigger CALL_FUNCTION_KW instead.
func (c *StarlarkCompiler) compileArguments(node *parser.ASTNode) {
	argNodes := []*parser.ASTNode{}
	for _, child := range node.Children {
		if n, ok := child.(*parser.ASTNode); ok {
			if n.RuleName == "argument" {
				argNodes = append(argNodes, n)
			}
		}
	}

	positionalCount := 0
	kwCount := 0

	for _, arg := range argNodes {
		c.compileArgument(arg, &positionalCount, &kwCount)
	}

	if kwCount > 0 {
		c.emit(OpCallFunctionKW, positionalCount+kwCount)
	} else {
		c.emit(OpCallFunction, positionalCount)
	}
}

// compileArgument compiles a single function call argument.
// Grammar: argument = DOUBLE_STAR expression | STAR expression | NAME EQUALS expression | expression ;
func (c *StarlarkCompiler) compileArgument(node *parser.ASTNode, positionalCount *int, kwCount *int) {
	tokens := extractTokens(node)
	nodes := extractNodes(node)

	// Check for keyword argument: NAME = expression
	if hasToken(node, "=") && len(tokens) >= 2 && len(nodes) >= 1 {
		// First NAME token is the keyword name
		for _, tok := range tokens {
			tn := tokenTypeName(tok)
			if tn == "NAME" {
				nameIdx := c.addConstant(tok.Value)
				c.emit(OpLoadConst, nameIdx)
				break
			}
		}
		c.compileNode(nodes[0])
		*kwCount++
		return
	}

	// Positional argument
	if len(nodes) > 0 {
		c.compileNode(nodes[0])
		*positionalCount++
	}
}

// ════════════════════════════════════════════════════════════════════════
// ATOM COMPILATION — Leaf values of the expression tree
// ════════════════════════════════════════════════════════════════════════

// compileAtom compiles atomic expressions (literals, names, collections).
// Grammar: atom = INT | FLOAT | STRING { STRING } | NAME
//
//	| "True" | "False" | "None"
//	| list_expr | dict_expr | paren_expr ;
//
// This is where literal values enter the bytecode. Each literal is added
// to the constants pool and referenced by a LOAD_CONST instruction.
func (c *StarlarkCompiler) compileAtom(node *parser.ASTNode) {
	// Check for child AST nodes first (list_expr, dict_expr, paren_expr)
	nodes := extractNodes(node)
	if len(nodes) > 0 {
		c.compileNode(nodes[0])
		return
	}

	// Process token children
	tokens := extractTokens(node)
	if len(tokens) == 0 {
		return
	}

	// Handle adjacent string concatenation: "a" "b" -> "ab"
	allStrings := true
	for _, tok := range tokens {
		tn := tokenTypeName(tok)
		if tn != "STRING" {
			allStrings = false
			break
		}
	}
	if allStrings && len(tokens) > 1 {
		var sb strings.Builder
		for _, tok := range tokens {
			sb.WriteString(parseStringLiteral(tok.Value))
		}
		idx := c.addConstant(sb.String())
		c.emit(OpLoadConst, idx)
		return
	}

	tok := tokens[0]
	tn := tokenTypeName(tok)

	switch {
	case tn == "INT":
		// Parse integer literal (decimal, hex, or octal)
		val, err := strconv.ParseInt(tok.Value, 0, 64)
		if err != nil {
			// Fallback to decimal
			val, _ = strconv.ParseInt(tok.Value, 10, 64)
		}
		idx := c.addConstant(checkedIntLiteral(val, tok.Value))
		c.emit(OpLoadConst, idx)

	case tn == "FLOAT":
		val, _ := strconv.ParseFloat(tok.Value, 64)
		idx := c.addConstant(val)
		c.emit(OpLoadConst, idx)

	case tn == "STRING":
		val := parseStringLiteral(tok.Value)
		idx := c.addConstant(val)
		c.emit(OpLoadConst, idx)

	case tn == "NAME":
		nameIdx := c.addName(tok.Value)
		if c.scopeDepth > 0 {
			c.emit(OpLoadLocal, nameIdx)
		} else {
			c.emit(OpLoadName, nameIdx)
		}

	case tn == "KEYWORD" && tok.Value == "True":
		c.emit(OpLoadTrue)

	case tn == "KEYWORD" && tok.Value == "False":
		c.emit(OpLoadFalse)

	case tn == "KEYWORD" && tok.Value == "None":
		c.emit(OpLoadNone)

	case tn == "NUMBER":
		// Generic NUMBER from base lexer — try int first, then float
		if strings.Contains(tok.Value, ".") {
			val, _ := strconv.ParseFloat(tok.Value, 64)
			idx := c.addConstant(val)
			c.emit(OpLoadConst, idx)
		} else {
			val, _ := strconv.ParseInt(tok.Value, 10, 64)
			idx := c.addConstant(checkedIntLiteral(val, tok.Value))
			c.emit(OpLoadConst, idx)
		}
	}
}

// ════════════════════════════════════════════════════════════════════════
// COLLECTION LITERAL COMPILATION
// ════════════════════════════════════════════════════════════════════════

// compileListExpr compiles a list literal or list comprehension.
// Grammar: list_expr = LBRACKET [ list_body ] RBRACKET ;
func (c *StarlarkCompiler) compileListExpr(node *parser.ASTNode) {
	nodes := extractNodes(node)
	if len(nodes) == 0 {
		// Empty list: []
		c.emit(OpBuildList, 0)
		return
	}
	c.compileNode(nodes[0])
}

// compileListBody compiles the contents of a list literal.
// Grammar: list_body = expression comp_clause | expression { COMMA expression } [ COMMA ] ;
func (c *StarlarkCompiler) compileListBody(node *parser.ASTNode) {
	nodes := extractNodes(node)

	// Check for comprehension
	for _, n := range nodes {
		if n.RuleName == "comp_clause" || n.RuleName == "comp_for" {
			// List comprehension — simplified: emit BUILD_LIST 0 as placeholder
			c.emit(OpBuildList, 0)
			return
		}
	}

	// Regular list literal
	count := 0
	for _, n := range nodes {
		c.compileNode(n)
		count++
	}
	c.emit(OpBuildList, count)
}

// compileDictExpr compiles a dict literal or dict comprehension.
// Grammar: dict_expr = LBRACE [ dict_body ] RBRACE ;
func (c *StarlarkCompiler) compileDictExpr(node *parser.ASTNode) {
	nodes := extractNodes(node)
	if len(nodes) == 0 {
		// Empty dict: {}
		c.emit(OpBuildDict, 0)
		return
	}
	c.compileNode(nodes[0])
}

// compileDictBody compiles the contents of a dict literal.
// Grammar: dict_body = dict_entry comp_clause | dict_entry { COMMA dict_entry } [ COMMA ] ;
func (c *StarlarkCompiler) compileDictBody(node *parser.ASTNode) {
	nodes := extractNodes(node)
	count := 0
	for _, n := range nodes {
		if n.RuleName == "dict_entry" {
			c.compileNode(n)
			count++
		}
	}
	c.emit(OpBuildDict, count)
}

// compileDictEntry compiles a single key: value pair.
// Grammar: dict_entry = expression COLON expression ;
func (c *StarlarkCompiler) compileDictEntry(node *parser.ASTNode) {
	nodes := extractNodes(node)
	if len(nodes) >= 2 {
		c.compileNode(nodes[0]) // key
		c.compileNode(nodes[1]) // value
	}
}

// compileParenExpr compiles a parenthesized expression or tuple.
// Grammar: paren_expr = LPAREN [ paren_body ] RPAREN ;
func (c *StarlarkCompiler) compileParenExpr(node *parser.ASTNode) {
	nodes := extractNodes(node)
	if len(nodes) == 0 {
		// Empty tuple: ()
		c.emit(OpBuildTuple, 0)
		return
	}
	c.compileNode(nodes[0])
}

// compileParenBody compiles the contents of parentheses.
// Grammar: paren_body = expression comp_clause
//
//	| expression COMMA [ expression { COMMA expression } [ COMMA ] ]
//	| expression ;
func (c *StarlarkCompiler) compileParenBody(node *parser.ASTNode) {
	nodes := extractNodes(node)

	if hasToken(node, ",") {
		// Tuple: (a, b, c)
		count := 0
		for _, n := range nodes {
			c.compileNode(n)
			count++
		}
		c.emit(OpBuildTuple, count)
		return
	}

	// Single expression in parens: (expr)
	if len(nodes) > 0 {
		c.compileNode(nodes[0])
	}
}

// ════════════════════════════════════════════════════════════════════════
// LAMBDA COMPILATION
// ════════════════════════════════════════════════════════════════════════

// compileLambdaExpr compiles a lambda expression.
// Grammar: lambda_expr = "lambda" [ lambda_params ] COLON expression ;
//
// A lambda is compiled similarly to a def statement but:
//   - It has no name (anonymous)
//   - Its body is a single expression (not a suite)
//   - It implicitly returns the expression's value
func (c *StarlarkCompiler) compileLambdaExpr(node *parser.ASTNode) {
	nodes := extractNodes(node)

	// Find the expression (last child node)
	var bodyExpr *parser.ASTNode
	var paramsNode *parser.ASTNode

	for _, n := range nodes {
		if n.RuleName == "lambda_params" {
			paramsNode = n
		} else {
			bodyExpr = n
		}
	}

	// Extract parameter names
	var paramNames []string
	if paramsNode != nil {
		for _, child := range paramsNode.Children {
			if paramNode, ok := child.(*parser.ASTNode); ok {
				if paramNode.RuleName == "lambda_param" {
					for _, tok := range extractTokens(paramNode) {
						tn := tokenTypeName(tok)
						if tn == "NAME" {
							paramNames = append(paramNames, tok.Value)
							break
						}
					}
				}
			}
		}
	}

	// Compile the lambda body
	bodyCompiler := NewStarlarkCompiler()
	bodyCompiler.scopeDepth = c.scopeDepth + 1
	if bodyExpr != nil {
		bodyCompiler.compileNode(bodyExpr)
	}
	bodyCompiler.emit(OpReturnValue)

	bodyCode := vm.CodeObject{
		Instructions: bodyCompiler.instructions,
		Constants:    bodyCompiler.constants,
		Names:        bodyCompiler.names,
	}

	funcInfo := map[string]interface{}{
		"code":          bodyCode,
		"params":        paramNames,
		"default_count": 0,
	}

	funcIdx := c.addConstant(funcInfo)
	c.emit(OpMakeFunction, funcIdx)
}

// Disassemble returns a human-readable string representation of a CodeObject.
// Useful for debugging and testing.
func Disassemble(code vm.CodeObject) string {
	result, _ := StartNew[string]("starlark-ast-to-bytecode-compiler.Disassemble", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			var sb strings.Builder
			for i, instr := range code.Instructions {
				name := OpcodeName[instr.Opcode]
				if name == "" {
					name = fmt.Sprintf("UNKNOWN(0x%02x)", int(instr.Opcode))
				}
				if instr.Operand != nil {
					fmt.Fprintf(&sb, "%04d  %-24s %v\n", i, name, instr.Operand)
				} else {
					fmt.Fprintf(&sb, "%04d  %s\n", i, name)
				}
			}
			return rf.Generate(true, false, sb.String())
		}).GetResult()
	return result
}
