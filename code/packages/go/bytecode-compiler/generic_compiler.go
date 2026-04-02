// GenericCompiler — A Pluggable AST-to-Bytecode Compiler Framework.
//
// ==========================================================================
// Chapter 1: Why a Generic Compiler?
// ==========================================================================
//
// In the previous modules, we built a BytecodeCompiler that knows how to
// compile a specific AST format (the one produced by our parser, with node
// kinds like "NumberLiteral", "BinaryOp", "Assignment"). That compiler is
// tightly coupled to one language's syntax.
//
// But what if we want to compile *different* languages — Python, Ruby, Lua —
// all to the same bytecode? Each language has its own AST structure, its own
// node types, and its own semantic rules. We'd need a different compiler for
// each one.
//
// The GenericCompiler solves this with the **plugin pattern**:
//
//  1. Define a universal AST shape (RuleName + Children).
//  2. Let each language register *handlers* for its specific rule names.
//  3. The framework handles the plumbing (instruction emission, constant
//     pools, scope management, jump patching).
//
// This is exactly how real compiler frameworks work:
//
//   - LLVM has a generic IR that many language front-ends compile to.
//   - GraalVM's Truffle framework lets languages register AST interpreters.
//   - .NET's Roslyn has a common compilation pipeline with language-specific
//     syntax analyzers plugged in.
//
// ==========================================================================
// Chapter 2: The AST Contract
// ==========================================================================
//
// For the generic compiler to walk *any* language's AST, we need a common
// tree shape. We use two node types:
//
// ASTNode — A non-terminal (interior) node. It has:
//   - RuleName: identifies what grammar rule produced this node.
//   - Children: an ordered list of child nodes (ASTNode or TokenNode).
//
// TokenNode — A terminal (leaf) node. It has:
//   - Type: the token category (e.g., "NUMBER", "IDENTIFIER").
//   - Value: the actual text from the source code.
//
// Example: the expression "1 + 2" might parse into:
//
//	ASTNode{RuleName: "addition", Children: []interface{}{
//	    &TokenNode{Type: "NUMBER", Value: "1"},
//	    &TokenNode{Type: "PLUS", Value: "+"},
//	    &TokenNode{Type: "NUMBER", Value: "2"},
//	}}
//
// ==========================================================================
// Chapter 3: The Dispatch Mechanism
// ==========================================================================
//
// When the compiler encounters an ASTNode, it decides what to do based on
// the node's RuleName:
//
//  1. Look up RuleName in the handler registry.
//  2. If a handler exists, call it.
//  3. If no handler exists but the node has exactly one child, "pass
//     through" to that child (handles wrapper rules).
//  4. If no handler exists and there are multiple children, panic with
//     "UnhandledRuleError".
//
// ==========================================================================
// Chapter 4: Scope Management
// ==========================================================================
//
// Languages with functions or block scoping need to track which variables
// are "local" to each scope. The GenericCompiler provides a scope stack:
//
//	EnterScope(params...)  — Push a new scope.
//	ExitScope()            — Pop the current scope and return it.
//
// Each CompilerScope maintains a Locals map from variable names to slot
// indices. Scopes form a linked list via the Parent pointer, enabling
// lexical scoping lookups.
package bytecodecompiler

import (
	"fmt"

	vm "github.com/adhithyan15/coding-adventures/code/packages/go/virtual-machine"
)

// =========================================================================
// Types — The contracts that language plugins implement
// =========================================================================

// CompileHandler handles compilation of a specific grammar rule. It receives
// the compiler (for emitting instructions) and the AST node being compiled.
//
// Handlers are the "language-specific" part. A Python plugin registers
// handlers for "if_statement", "for_loop", etc. A Ruby plugin registers
// handlers for "method_definition", "block", etc.
type CompileHandler func(compiler *GenericCompiler, node *ASTNode)

// ASTNode is a non-terminal (interior) node in the parse tree. Every
// interior node has a RuleName (which grammar rule produced it) and a
// list of Children (the sub-expressions and tokens that make it up).
//
// Children are typed as interface{} because they can be either *ASTNode
// or *TokenNode — Go does not have union types, so we use the empty
// interface and type-switch at runtime.
type ASTNode struct {
	RuleName string
	Children []interface{} // Each element is *ASTNode or *TokenNode
}

// TokenNode is a terminal (leaf) node in the parse tree. It represents
// an actual token from the source code — a number, identifier, operator,
// keyword, etc.
type TokenNode struct {
	Type  string
	Value string
}

// =========================================================================
// CompilerScope — Local variable tracking for nested scopes
// =========================================================================

// CompilerScope tracks local variable slots within a particular region of
// code (a function body, a block, a module). Each local variable is assigned
// a numeric "slot index" — this is what STORE_LOCAL and LOAD_LOCAL
// instructions reference.
//
// Scopes form a linked list: each scope has a Parent pointer to the
// enclosing scope. This enables lexical scoping — if a variable isn't found
// in the current scope, you can walk up the chain to look in enclosing
// scopes.
//
// Real VMs do this too:
//   - The JVM uses a "local variable array" per stack frame, indexed by slot.
//   - CPython uses a co_varnames tuple, indexed by slot.
//   - Our scope's Locals map serves the same purpose.
type CompilerScope struct {
	Locals map[string]int
	Parent *CompilerScope
}

// NewCompilerScope creates a new scope linked to the given parent.
// If parent is nil, this is the outermost (global) scope.
func NewCompilerScope(parent *CompilerScope) *CompilerScope {
	result, _ := StartNew[*CompilerScope]("bytecode-compiler.NewCompilerScope", nil,
		func(_ *Operation[*CompilerScope], rf *ResultFactory[*CompilerScope]) *OperationResult[*CompilerScope] {
			return rf.Generate(true, false, &CompilerScope{
				Locals: make(map[string]int),
				Parent: parent,
			})
		}).GetResult()
	return result
}

// AddLocal registers a new local variable and returns its slot index.
// If the name already exists, returns the existing slot index (deduplication).
//
// This deduplication prevents bugs where AddLocal("x") called twice would
// give different slot indices for the same variable.
func (s *CompilerScope) AddLocal(name string) int {
	result, _ := StartNew[int]("bytecode-compiler.AddLocal", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("name", name)
			if slot, ok := s.Locals[name]; ok {
				return rf.Generate(true, false, slot)
			}
			slot := len(s.Locals)
			s.Locals[name] = slot
			return rf.Generate(true, false, slot)
		}).GetResult()
	return result
}

// GetLocal looks up a variable's slot index by name. Returns the slot and
// true if found, or 0 and false if the variable is not in this scope.
//
// This method does NOT walk up the parent chain — that's intentional.
// Different languages handle scope lookup differently (some have closures,
// some don't), so we leave parent-scope resolution to the language plugin.
func (s *CompilerScope) GetLocal(name string) (int, bool) {
	type getLocalResult struct {
		slot int
		ok   bool
	}
	res, _ := StartNew[getLocalResult]("bytecode-compiler.GetLocal", getLocalResult{},
		func(op *Operation[getLocalResult], rf *ResultFactory[getLocalResult]) *OperationResult[getLocalResult] {
			op.AddProperty("name", name)
			slot, ok := s.Locals[name]
			return rf.Generate(true, false, getLocalResult{slot: slot, ok: ok})
		}).GetResult()
	return res.slot, res.ok
}

// NumLocals returns the total number of local variables registered in this
// scope. This is needed when generating function metadata — the VM needs to
// know how many local slots to allocate when entering a function call.
func (s *CompilerScope) NumLocals() int {
	result, _ := StartNew[int]("bytecode-compiler.NumLocals", 0,
		func(_ *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, len(s.Locals))
		}).GetResult()
	return result
}

// =========================================================================
// GenericCompiler — The pluggable compilation framework
// =========================================================================

// GenericCompiler is a pluggable AST-to-bytecode compiler framework. It
// provides the infrastructure for compilation: instruction emission,
// constant/name pool management, scope tracking, jump patching, and nested
// code object compilation. Language-specific behavior is provided by
// registering CompileHandler functions for each AST rule name.
//
// Think of it like a kitchen (GenericCompiler) with cooking equipment
// (Emit, AddConstant, EnterScope, etc.) — and the chef (language plugin)
// decides what dish to make by registering recipes (handlers).
//
// Usage:
//
//	compiler := NewGenericCompiler()
//
//	compiler.RegisterRule("number", func(c *GenericCompiler, node *ASTNode) {
//	    token := node.Children[0].(*TokenNode)
//	    value, _ := strconv.Atoi(token.Value)
//	    idx := c.AddConstant(value)
//	    c.Emit(vm.OpLoadConst, idx)
//	})
//
//	code := compiler.Compile(ast)
type GenericCompiler struct {
	// Instructions is the growing list of bytecode instructions emitted so far.
	Instructions []vm.Instruction

	// Constants is the constant pool — literal values referenced by LOAD_CONST.
	Constants []interface{}

	// Names is the name pool — variable/function names referenced by index.
	Names []string

	// Scope is the current local variable scope, or nil if not inside a scope.
	Scope *CompilerScope

	// dispatch maps rule names to compile handlers.
	dispatch map[string]CompileHandler

	// codeObjects accumulates code objects from CompileNested calls.
	codeObjects []vm.CodeObject
}

// NewGenericCompiler creates a fresh compiler with empty state and no
// registered handlers. Language plugins call RegisterRule to teach it
// about their syntax.
func NewGenericCompiler() *GenericCompiler {
	result, _ := StartNew[*GenericCompiler]("bytecode-compiler.NewGenericCompiler", nil,
		func(_ *Operation[*GenericCompiler], rf *ResultFactory[*GenericCompiler]) *OperationResult[*GenericCompiler] {
			return rf.Generate(true, false, &GenericCompiler{
				Instructions: []vm.Instruction{},
				Constants:    []interface{}{},
				Names:        []string{},
				Scope:        nil,
				dispatch:     make(map[string]CompileHandler),
				codeObjects:  []vm.CodeObject{},
			})
		}).GetResult()
	return result
}

// =========================================================================
// Plugin registration
// =========================================================================

// RegisterRule registers a compile handler for a specific AST rule name.
// This is how language plugins teach the compiler about their syntax.
//
// If a handler was already registered for the same rule name, it is
// silently replaced. This allows plugins to override default behavior.
func (c *GenericCompiler) RegisterRule(ruleName string, handler CompileHandler) {
	_, _ = StartNew[struct{}]("bytecode-compiler.RegisterRule", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("ruleName", ruleName)
			c.dispatch[ruleName] = handler
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// =========================================================================
// Instruction emission
// =========================================================================

// Emit appends a single bytecode instruction and returns its index.
//
// Call with no operand for instructions like ADD, POP, HALT:
//
//	c.Emit(vm.OpAdd)
//
// Call with one operand for instructions like LOAD_CONST:
//
//	c.Emit(vm.OpLoadConst, 0)
//
// The returned index is useful for jump patching — you might emit a
// JUMP_IF_FALSE now and patch its target later when you know where
// the else-branch starts.
func (c *GenericCompiler) Emit(opcode vm.OpCode, operand ...interface{}) int {
	result, _ := StartNew[int]("bytecode-compiler.Emit", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("opcode", opcode)
			var instr vm.Instruction
			if len(operand) == 0 {
				instr = vm.Instruction{Opcode: opcode, Operand: nil}
			} else {
				instr = vm.Instruction{Opcode: opcode, Operand: operand[0]}
			}
			c.Instructions = append(c.Instructions, instr)
			return rf.Generate(true, false, len(c.Instructions)-1)
		}).GetResult()
	return result
}

// EmitJump emits a jump instruction with a placeholder operand (0).
//
// Jump instructions (JUMP, JUMP_IF_FALSE, JUMP_IF_TRUE) need a target
// address, but at the time we emit the jump, we often don't know the
// target yet (because we haven't compiled the code after the jump).
//
// The solution is a two-step process called **backpatching**:
//
//  1. EmitJump(opcode) — Emit the jump with operand=0 (placeholder).
//  2. PatchJump(index) — Later, fill in the real target.
//
// This is used by every real compiler: JVM's javac, GCC, LLVM.
func (c *GenericCompiler) EmitJump(opcode vm.OpCode) int {
	result, _ := StartNew[int]("bytecode-compiler.EmitJump", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("opcode", opcode)
			return rf.Generate(true, false, c.Emit(opcode, 0))
		}).GetResult()
	return result
}

// PatchJump patches a previously emitted jump instruction with the real
// target. If target is provided, the jump goes to that specific instruction
// index. If omitted, the jump targets CurrentOffset — the next instruction
// that will be emitted.
//
// Example — compiling "if (cond) { then } else { else }":
//
//	compileCondition()
//	jumpToElse := c.EmitJump(vm.OpJumpIfFalse)
//	compileThenBranch()
//	jumpOverElse := c.EmitJump(vm.OpJump)
//	c.PatchJump(jumpToElse)   // else starts here
//	compileElseBranch()
//	c.PatchJump(jumpOverElse) // after else
func (c *GenericCompiler) PatchJump(index int, target ...int) {
	_, _ = StartNew[struct{}]("bytecode-compiler.PatchJump", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("index", index)
			if index < 0 || index >= len(c.Instructions) {
				panic(fmt.Sprintf("CompilerError: Cannot patch jump at index %d: instruction does not exist", index))
			}
			t := c.CurrentOffset()
			if len(target) > 0 {
				t = target[0]
			}
			c.Instructions[index] = vm.Instruction{
				Opcode:  c.Instructions[index].Opcode,
				Operand: t,
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// CurrentOffset returns the index where the *next* emitted instruction
// will be placed. This is used for jump target calculations.
func (c *GenericCompiler) CurrentOffset() int {
	result, _ := StartNew[int]("bytecode-compiler.CurrentOffset", 0,
		func(_ *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, len(c.Instructions))
		}).GetResult()
	return result
}

// =========================================================================
// Pool management — constants and names
// =========================================================================

// AddConstant adds a value to the constant pool and returns its index.
// Constants are **deduplicated**: if the value already exists in the pool,
// the existing index is returned instead of adding a duplicate.
//
// We use == for deduplication, which means:
//   - 42 and 42 are the same (reuses the slot).
//   - "hello" and "hello" are the same.
//   - nil and nil are the same.
//
// Real VMs deduplicate constants too — the JVM's constant pool deduplicates
// strings, and CPython's compiler deduplicates constants.
func (c *GenericCompiler) AddConstant(value interface{}) int {
	result, _ := StartNew[int]("bytecode-compiler.AddConstant", 0,
		func(_ *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			for i, v := range c.Constants {
				if v == value {
					return rf.Generate(true, false, i)
				}
			}
			c.Constants = append(c.Constants, value)
			return rf.Generate(true, false, len(c.Constants)-1)
		}).GetResult()
	return result
}

// AddName adds a name to the name pool and returns its index. Like
// AddConstant, names are deduplicated — the same variable name used in
// multiple places gets the same index.
func (c *GenericCompiler) AddName(name string) int {
	result, _ := StartNew[int]("bytecode-compiler.AddName", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("name", name)
			for i, n := range c.Names {
				if n == name {
					return rf.Generate(true, false, i)
				}
			}
			c.Names = append(c.Names, name)
			return rf.Generate(true, false, len(c.Names)-1)
		}).GetResult()
	return result
}

// =========================================================================
// Scope management
// =========================================================================

// EnterScope pushes a new local variable scope. If params are provided,
// they are pre-assigned to local slots (slot 0 for the first param, slot 1
// for the second, etc.).
//
// Scopes are linked: the new scope's Parent points to the previous scope
// (or nil if there was none). This enables lexical scoping.
//
// Example:
//
//	scope := compiler.EnterScope("x", "y")
//	// scope.GetLocal("x") => 0, true
//	// scope.GetLocal("y") => 1, true
func (c *GenericCompiler) EnterScope(params ...string) *CompilerScope {
	result, _ := StartNew[*CompilerScope]("bytecode-compiler.EnterScope", nil,
		func(_ *Operation[*CompilerScope], rf *ResultFactory[*CompilerScope]) *OperationResult[*CompilerScope] {
			newScope := NewCompilerScope(c.Scope)
			for _, name := range params {
				newScope.AddLocal(name)
			}
			c.Scope = newScope
			return rf.Generate(true, false, newScope)
		}).GetResult()
	return result
}

// ExitScope pops the current scope and restores the parent scope. Returns
// the scope that was just exited, so the caller can inspect its NumLocals
// or other properties.
//
// Panics if not currently inside a scope — this is a programming error.
func (c *GenericCompiler) ExitScope() *CompilerScope {
	result, _ := StartNew[*CompilerScope]("bytecode-compiler.ExitScope", nil,
		func(_ *Operation[*CompilerScope], rf *ResultFactory[*CompilerScope]) *OperationResult[*CompilerScope] {
			if c.Scope == nil {
				panic("CompilerError: Cannot exit scope: not currently inside a scope. Did you call ExitScope() without a matching EnterScope()?")
			}
			exited := c.Scope
			c.Scope = exited.Parent
			return rf.Generate(true, false, exited)
		}).GetResult()
	return result
}

// =========================================================================
// Node compilation — the recursive dispatch engine
// =========================================================================

// CompileNested compiles a nested code object (e.g., a function body).
//
// This saves the compiler's current state (instructions, constants, names),
// compiles the given AST node into a fresh code unit, then restores the
// original state. The nested code object is returned and also stored in
// the codeObjects list.
//
// This is how real compilers handle functions-within-functions:
//   - CPython compiles each function body as a separate code object.
//   - The JVM compiles inner classes and lambdas as separate .class files.
func (c *GenericCompiler) CompileNested(node *ASTNode) vm.CodeObject {
	result, _ := StartNew[vm.CodeObject]("bytecode-compiler.CompileNested", vm.CodeObject{},
		func(_ *Operation[vm.CodeObject], rf *ResultFactory[vm.CodeObject]) *OperationResult[vm.CodeObject] {
			savedInstructions := c.Instructions
			savedConstants := c.Constants
			savedNames := c.Names

			c.Instructions = []vm.Instruction{}
			c.Constants = []interface{}{}
			c.Names = []string{}

			c.CompileNode(node)

			codeObject := vm.CodeObject{
				Instructions: c.Instructions,
				Constants:    c.Constants,
				Names:        c.Names,
			}

			c.codeObjects = append(c.codeObjects, codeObject)

			c.Instructions = savedInstructions
			c.Constants = savedConstants
			c.Names = savedNames

			return rf.Generate(true, false, codeObject)
		}).GetResult()
	return result
}

// CompileNode compiles a single AST node or token node. This is the main
// dispatch method — the recursive heart of the compiler.
//
// The decision tree:
//
//  1. *TokenNode (leaf): Call CompileToken(), which is a no-op by default.
//  2. *ASTNode with a registered handler: Call the handler.
//  3. *ASTNode with one child and no handler: Pass through to the child.
//  4. *ASTNode with multiple children and no handler: Panic with
//     "UnhandledRuleError".
//
// The pass-through behavior (case 3) is important. In a real grammar, many
// rules exist purely for precedence or grouping:
//
//	expression -> comparison -> addition -> multiplication -> primary
//
// When parsing "42", all of these rules fire, each producing a single-child
// node. The pass-through rule means we don't need handlers for these
// "wrapper" rules.
func (c *GenericCompiler) CompileNode(node interface{}) {
	_, _ = StartNew[struct{}]("bytecode-compiler.CompileNode", struct{}{},
		func(_ *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			if token, ok := node.(*TokenNode); ok {
				c.CompileToken(token)
				return rf.Generate(true, false, struct{}{})
			}

			astNode, ok := node.(*ASTNode)
			if !ok {
				panic(fmt.Sprintf("CompilerError: CompileNode received unexpected type %T", node))
			}

			handler, exists := c.dispatch[astNode.RuleName]
			if exists {
				handler(c, astNode)
				return rf.Generate(true, false, struct{}{})
			}

			if len(astNode.Children) == 1 {
				c.CompileNode(astNode.Children[0])
				return rf.Generate(true, false, struct{}{})
			}

			panic(fmt.Sprintf(
				"UnhandledRuleError: No handler registered for rule %q and node has %d children. "+
					"Register a handler with compiler.RegisterRule(%q, handler).",
				astNode.RuleName, len(astNode.Children), astNode.RuleName,
			))
		}).GetResult()
}

// CompileToken compiles a token node. By default, this is a no-op — tokens
// are typically handled by their parent ASTNode's handler, which knows the
// context (is this number a literal? is this identifier a variable
// reference? a function name?).
func (c *GenericCompiler) CompileToken(token *TokenNode) {
	_, _ = StartNew[struct{}]("bytecode-compiler.CompileToken", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			if token != nil {
				op.AddProperty("tokenType", token.Type)
			}
			_ = token
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// =========================================================================
// Top-level compilation
// =========================================================================

// Compile compiles an entire AST into a CodeObject. This is the main entry
// point for compilation. It:
//
//  1. Compiles the root AST node (which recursively compiles all children).
//  2. Appends a HALT instruction (or a custom halt opcode) to ensure the
//     VM stops after executing the program.
//  3. Returns a self-contained CodeObject with instructions, constants,
//     and names — ready for the VM to execute.
//
// The optional haltOpcode parameter lets you specify a different halt
// instruction. Defaults to vm.OpHalt (0xFF).
func (c *GenericCompiler) Compile(ast *ASTNode, haltOpcode ...vm.OpCode) vm.CodeObject {
	result, _ := StartNew[vm.CodeObject]("bytecode-compiler.GenericCompile", vm.CodeObject{},
		func(_ *Operation[vm.CodeObject], rf *ResultFactory[vm.CodeObject]) *OperationResult[vm.CodeObject] {
			c.CompileNode(ast)

			halt := vm.OpHalt
			if len(haltOpcode) > 0 {
				halt = haltOpcode[0]
			}

			c.Emit(halt)

			return rf.Generate(true, false, vm.CodeObject{
				Instructions: c.Instructions,
				Constants:    c.Constants,
				Names:        c.Names,
			})
		}).GetResult()
	return result
}
