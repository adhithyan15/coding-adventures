// Comprehensive tests for the GenericCompiler — the pluggable AST-to-bytecode
// compiler framework.
//
// These tests verify every aspect of the GenericCompiler:
//
//  1. Plugin registration and dispatch — handlers are called for their
//     registered rule names.
//  2. Pass-through — single-child nodes without handlers are transparently
//     forwarded.
//  3. Error handling — multi-child nodes without handlers panic.
//  4. Instruction emission — Emit, EmitJump, PatchJump, CurrentOffset.
//  5. Pool management — constant and name deduplication.
//  6. Scope management — enter, exit, params, nesting.
//  7. Nested compilation — CompileNested saves/restores state.
//  8. Top-level compile — appends HALT, returns CodeObject.
//  9. Integration — a realistic "compile addition" scenario.
package bytecodecompiler

import (
	"fmt"
	"reflect"
	"strings"
	"testing"

	vm "github.com/adhithyan15/coding-adventures/code/packages/go/virtual-machine"
)

// =========================================================================
// Helpers — factory functions for building test ASTs
// =========================================================================

// mkAST creates an ASTNode with the given rule name and children.
func mkAST(ruleName string, children ...interface{}) *ASTNode {
	if children == nil {
		children = []interface{}{}
	}
	return &ASTNode{RuleName: ruleName, Children: children}
}

// mkToken creates a TokenNode (leaf) with the given type and value.
func mkToken(tokenType, value string) *TokenNode {
	return &TokenNode{Type: tokenType, Value: value}
}

// assertPanic is a helper that expects the given function to panic with a
// message containing the given substring.
func assertPanic(t *testing.T, f func(), substr string) {
	t.Helper()
	defer func() {
		r := recover()
		if r == nil {
			t.Fatal("expected panic but did not get one")
		}
		msg, ok := r.(string)
		if !ok {
			t.Fatalf("expected panic message to be a string, got %T: %v", r, r)
		}
		if !strings.Contains(msg, substr) {
			t.Fatalf("panic message %q does not contain %q", msg, substr)
		}
	}()
	f()
}

// opcodes extracts just the opcodes from a slice of instructions, for
// easy comparison in tests.
func opcodes(instrs []vm.Instruction) []vm.OpCode {
	result := make([]vm.OpCode, len(instrs))
	for i, instr := range instrs {
		result[i] = instr.Opcode
	}
	return result
}

// =========================================================================
// Plugin registration and dispatch
// =========================================================================

func TestPluginRegistrationAndDispatch(t *testing.T) {
	t.Run("calls the registered handler for a matching ruleName", func(t *testing.T) {
		// The most basic test: register a handler, compile a node with that
		// rule name, and verify the handler was called.
		compiler := NewGenericCompiler()
		called := false

		compiler.RegisterRule("my_rule", func(c *GenericCompiler, node *ASTNode) {
			_ = c
			_ = node
			called = true
		})

		compiler.CompileNode(mkAST("my_rule"))
		if !called {
			t.Fatal("expected handler to be called")
		}
	})

	t.Run("passes the compiler and node to the handler", func(t *testing.T) {
		// Handlers receive both the compiler instance (for emitting instructions)
		// and the AST node (for reading children). Verify both are correct.
		compiler := NewGenericCompiler()
		testNode := mkAST("check_args", mkToken("NUM", "42"))

		compiler.RegisterRule("check_args", func(c *GenericCompiler, node *ASTNode) {
			if c != compiler {
				t.Error("handler received wrong compiler")
			}
			if node != testNode {
				t.Error("handler received wrong node")
			}
		})

		compiler.CompileNode(testNode)
	})

	t.Run("dispatches different rules to different handlers", func(t *testing.T) {
		// Multiple rules can be registered, and each gets its own handler.
		compiler := NewGenericCompiler()
		var log []string

		compiler.RegisterRule("rule_a", func(c *GenericCompiler, node *ASTNode) {
			log = append(log, "a")
		})
		compiler.RegisterRule("rule_b", func(c *GenericCompiler, node *ASTNode) {
			log = append(log, "b")
		})

		compiler.CompileNode(mkAST("rule_a"))
		compiler.CompileNode(mkAST("rule_b"))

		if !reflect.DeepEqual(log, []string{"a", "b"}) {
			t.Fatalf("expected [a b], got %v", log)
		}
	})

	t.Run("later registration overwrites earlier for the same ruleName", func(t *testing.T) {
		// If a handler is registered twice for the same rule, the second wins.
		compiler := NewGenericCompiler()
		result := ""

		compiler.RegisterRule("overridable", func(c *GenericCompiler, node *ASTNode) {
			result = "first"
		})
		compiler.RegisterRule("overridable", func(c *GenericCompiler, node *ASTNode) {
			result = "second"
		})

		compiler.CompileNode(mkAST("overridable"))
		if result != "second" {
			t.Fatalf("expected 'second', got %q", result)
		}
	})
}

// =========================================================================
// Pass-through single child nodes
// =========================================================================

func TestPassThroughSingleChild(t *testing.T) {
	t.Run("passes through a single-child ASTNode to its child", func(t *testing.T) {
		// A node with one child and no handler should delegate to its child.
		compiler := NewGenericCompiler()
		called := false

		compiler.RegisterRule("inner", func(c *GenericCompiler, node *ASTNode) {
			called = true
		})

		// "wrapper" has no handler, but it has one child with a handler.
		compiler.CompileNode(mkAST("wrapper", mkAST("inner")))
		if !called {
			t.Fatal("expected inner handler to be called via pass-through")
		}
	})

	t.Run("chains through multiple levels of single-child wrappers", func(t *testing.T) {
		// Multiple layers of wrapper rules should all pass through.
		compiler := NewGenericCompiler()
		called := false

		compiler.RegisterRule("leaf", func(c *GenericCompiler, node *ASTNode) {
			called = true
		})

		tree := mkAST("level1",
			mkAST("level2",
				mkAST("level3",
					mkAST("leaf"))))

		compiler.CompileNode(tree)
		if !called {
			t.Fatal("expected leaf handler to be called via chained pass-through")
		}
	})
}

// =========================================================================
// Unhandled multi-child raises error
// =========================================================================

func TestUnhandledMultiChildPanics(t *testing.T) {
	t.Run("panics for multi-child node without handler", func(t *testing.T) {
		// If a node has multiple children and no registered handler, the
		// compiler can't guess what to do — it panics.
		compiler := NewGenericCompiler()
		node := mkAST("unknown_rule", mkToken("A", "a"), mkToken("B", "b"))

		assertPanic(t, func() {
			compiler.CompileNode(node)
		}, "UnhandledRuleError")
	})

	t.Run("error message includes the rule name", func(t *testing.T) {
		// The error message should tell the developer which rule is missing.
		compiler := NewGenericCompiler()
		node := mkAST("missing_handler", mkToken("X", "x"), mkToken("Y", "y"))

		assertPanic(t, func() {
			compiler.CompileNode(node)
		}, "missing_handler")
	})
}

// =========================================================================
// Token pass-through (no-op)
// =========================================================================

func TestTokenPassThrough(t *testing.T) {
	t.Run("compileToken is a no-op by default", func(t *testing.T) {
		// When CompileNode encounters a TokenNode, it calls CompileToken,
		// which does nothing by default. No instructions should be emitted.
		compiler := NewGenericCompiler()
		before := len(compiler.Instructions)

		compiler.CompileNode(mkToken("NUMBER", "42"))

		if len(compiler.Instructions) != before {
			t.Fatal("expected no instructions to be emitted for token node")
		}
	})

	t.Run("single-child wrapper around token passes through silently", func(t *testing.T) {
		// A wrapper node whose only child is a token should pass through
		// to the token, which is a no-op.
		compiler := NewGenericCompiler()
		node := mkAST("wrapper", mkToken("IDENT", "x"))

		compiler.CompileNode(node) // should not panic
		if len(compiler.Instructions) != 0 {
			t.Fatal("expected no instructions")
		}
	})
}

// =========================================================================
// Instruction emission
// =========================================================================

func TestInstructionEmission(t *testing.T) {
	t.Run("emit appends an instruction with opcode only", func(t *testing.T) {
		// Instructions like ADD, POP, HALT don't need an operand.
		compiler := NewGenericCompiler()
		compiler.Emit(vm.OpAdd)

		if len(compiler.Instructions) != 1 {
			t.Fatalf("expected 1 instruction, got %d", len(compiler.Instructions))
		}
		if compiler.Instructions[0].Opcode != vm.OpAdd {
			t.Fatalf("expected OpAdd, got %v", compiler.Instructions[0].Opcode)
		}
		if compiler.Instructions[0].Operand != nil {
			t.Fatalf("expected nil operand, got %v", compiler.Instructions[0].Operand)
		}
	})

	t.Run("emit appends an instruction with opcode and operand", func(t *testing.T) {
		compiler := NewGenericCompiler()
		compiler.Emit(vm.OpLoadConst, 0)

		if len(compiler.Instructions) != 1 {
			t.Fatalf("expected 1 instruction, got %d", len(compiler.Instructions))
		}
		if compiler.Instructions[0].Opcode != vm.OpLoadConst {
			t.Fatalf("expected OpLoadConst, got %v", compiler.Instructions[0].Opcode)
		}
		if compiler.Instructions[0].Operand != 0 {
			t.Fatalf("expected operand 0, got %v", compiler.Instructions[0].Operand)
		}
	})

	t.Run("emit returns sequential indices", func(t *testing.T) {
		compiler := NewGenericCompiler()

		idx0 := compiler.Emit(vm.OpLoadConst, 0)
		idx1 := compiler.Emit(vm.OpLoadConst, 1)
		idx2 := compiler.Emit(vm.OpAdd)

		if idx0 != 0 || idx1 != 1 || idx2 != 2 {
			t.Fatalf("expected indices 0, 1, 2; got %d, %d, %d", idx0, idx1, idx2)
		}
	})

	t.Run("currentOffset reflects the number of emitted instructions", func(t *testing.T) {
		compiler := NewGenericCompiler()

		if compiler.CurrentOffset() != 0 {
			t.Fatalf("expected offset 0, got %d", compiler.CurrentOffset())
		}
		compiler.Emit(vm.OpAdd)
		if compiler.CurrentOffset() != 1 {
			t.Fatalf("expected offset 1, got %d", compiler.CurrentOffset())
		}
		compiler.Emit(vm.OpSub)
		if compiler.CurrentOffset() != 2 {
			t.Fatalf("expected offset 2, got %d", compiler.CurrentOffset())
		}
	})

	t.Run("emit supports string operand", func(t *testing.T) {
		compiler := NewGenericCompiler()
		compiler.Emit(vm.OpLoadConst, "hello")

		if compiler.Instructions[0].Operand != "hello" {
			t.Fatalf("expected 'hello', got %v", compiler.Instructions[0].Operand)
		}
	})

	t.Run("emit supports nil operand", func(t *testing.T) {
		// Passing nil explicitly as an operand.
		compiler := NewGenericCompiler()
		compiler.Emit(vm.OpLoadConst, nil)

		if compiler.Instructions[0].Operand != nil {
			t.Fatalf("expected nil, got %v", compiler.Instructions[0].Operand)
		}
	})
}

// =========================================================================
// Jump patching
// =========================================================================

func TestJumpPatching(t *testing.T) {
	t.Run("emitJump emits a placeholder with operand 0", func(t *testing.T) {
		compiler := NewGenericCompiler()
		idx := compiler.EmitJump(vm.OpJumpIfFalse)

		if compiler.Instructions[idx].Opcode != vm.OpJumpIfFalse {
			t.Fatalf("expected OpJumpIfFalse, got %v", compiler.Instructions[idx].Opcode)
		}
		if compiler.Instructions[idx].Operand != 0 {
			t.Fatalf("expected operand 0, got %v", compiler.Instructions[idx].Operand)
		}
	})

	t.Run("patchJump with explicit target", func(t *testing.T) {
		compiler := NewGenericCompiler()
		jumpIdx := compiler.EmitJump(vm.OpJump)
		compiler.Emit(vm.OpAdd) // index 1
		compiler.Emit(vm.OpSub) // index 2

		compiler.PatchJump(jumpIdx, 2)

		if compiler.Instructions[jumpIdx].Operand != 2 {
			t.Fatalf("expected operand 2, got %v", compiler.Instructions[jumpIdx].Operand)
		}
	})

	t.Run("patchJump defaults to currentOffset", func(t *testing.T) {
		compiler := NewGenericCompiler()
		jumpIdx := compiler.EmitJump(vm.OpJumpIfFalse)
		compiler.Emit(vm.OpAdd) // index 1
		compiler.Emit(vm.OpSub) // index 2

		// currentOffset is now 3.
		compiler.PatchJump(jumpIdx)

		if compiler.Instructions[jumpIdx].Operand != 3 {
			t.Fatalf("expected operand 3, got %v", compiler.Instructions[jumpIdx].Operand)
		}
	})

	t.Run("patchJump preserves the original opcode", func(t *testing.T) {
		compiler := NewGenericCompiler()
		jumpIdx := compiler.EmitJump(vm.OpJumpIfFalse)

		compiler.PatchJump(jumpIdx, 10)

		if compiler.Instructions[jumpIdx].Opcode != vm.OpJumpIfFalse {
			t.Fatalf("expected OpJumpIfFalse, got %v", compiler.Instructions[jumpIdx].Opcode)
		}
		if compiler.Instructions[jumpIdx].Operand != 10 {
			t.Fatalf("expected operand 10, got %v", compiler.Instructions[jumpIdx].Operand)
		}
	})

	t.Run("emitJump returns the instruction index for later patching", func(t *testing.T) {
		compiler := NewGenericCompiler()
		compiler.Emit(vm.OpLoadConst, 0) // index 0
		jumpIdx := compiler.EmitJump(vm.OpJump) // index 1

		if jumpIdx != 1 {
			t.Fatalf("expected index 1, got %d", jumpIdx)
		}
	})
}

// =========================================================================
// Constant pool
// =========================================================================

func TestConstantPool(t *testing.T) {
	t.Run("addConstant adds a new value and returns its index", func(t *testing.T) {
		compiler := NewGenericCompiler()

		idx := compiler.AddConstant(42)

		if idx != 0 {
			t.Fatalf("expected index 0, got %d", idx)
		}
		if !reflect.DeepEqual(compiler.Constants, []interface{}{42}) {
			t.Fatalf("expected [42], got %v", compiler.Constants)
		}
	})

	t.Run("addConstant deduplicates identical values", func(t *testing.T) {
		compiler := NewGenericCompiler()

		idx1 := compiler.AddConstant(42)
		idx2 := compiler.AddConstant(42)

		if idx1 != 0 || idx2 != 0 {
			t.Fatalf("expected both indices to be 0, got %d and %d", idx1, idx2)
		}
		if !reflect.DeepEqual(compiler.Constants, []interface{}{42}) {
			t.Fatalf("expected [42], got %v", compiler.Constants)
		}
	})

	t.Run("addConstant handles multiple distinct values", func(t *testing.T) {
		compiler := NewGenericCompiler()

		i0 := compiler.AddConstant(1)
		i1 := compiler.AddConstant("hello")
		i2 := compiler.AddConstant(nil)
		i3 := compiler.AddConstant(2)

		if i0 != 0 || i1 != 1 || i2 != 2 || i3 != 3 {
			t.Fatalf("expected indices 0,1,2,3; got %d,%d,%d,%d", i0, i1, i2, i3)
		}
		expected := []interface{}{1, "hello", nil, 2}
		if !reflect.DeepEqual(compiler.Constants, expected) {
			t.Fatalf("expected %v, got %v", expected, compiler.Constants)
		}
	})

	t.Run("addConstant distinguishes numbers from strings", func(t *testing.T) {
		// The integer 0 and the string "0" are different constants.
		compiler := NewGenericCompiler()

		i0 := compiler.AddConstant(0)
		i1 := compiler.AddConstant("0")

		if i0 != 0 || i1 != 1 {
			t.Fatalf("expected indices 0 and 1, got %d and %d", i0, i1)
		}
	})
}

// =========================================================================
// Name pool
// =========================================================================

func TestNamePool(t *testing.T) {
	t.Run("addName adds a new name and returns its index", func(t *testing.T) {
		compiler := NewGenericCompiler()

		idx := compiler.AddName("x")

		if idx != 0 {
			t.Fatalf("expected index 0, got %d", idx)
		}
		if !reflect.DeepEqual(compiler.Names, []string{"x"}) {
			t.Fatalf("expected [x], got %v", compiler.Names)
		}
	})

	t.Run("addName deduplicates identical names", func(t *testing.T) {
		compiler := NewGenericCompiler()

		idx1 := compiler.AddName("x")
		idx2 := compiler.AddName("x")

		if idx1 != 0 || idx2 != 0 {
			t.Fatalf("expected both indices to be 0, got %d and %d", idx1, idx2)
		}
		if !reflect.DeepEqual(compiler.Names, []string{"x"}) {
			t.Fatalf("expected [x], got %v", compiler.Names)
		}
	})

	t.Run("addName handles multiple distinct names", func(t *testing.T) {
		compiler := NewGenericCompiler()

		i0 := compiler.AddName("x")
		i1 := compiler.AddName("y")
		i2 := compiler.AddName("z")

		if i0 != 0 || i1 != 1 || i2 != 2 {
			t.Fatalf("expected indices 0,1,2; got %d,%d,%d", i0, i1, i2)
		}
		if !reflect.DeepEqual(compiler.Names, []string{"x", "y", "z"}) {
			t.Fatalf("expected [x y z], got %v", compiler.Names)
		}
	})
}

// =========================================================================
// Scope management
// =========================================================================

func TestScopeManagement(t *testing.T) {
	t.Run("enterScope creates a new scope and sets it as current", func(t *testing.T) {
		compiler := NewGenericCompiler()
		if compiler.Scope != nil {
			t.Fatal("expected nil scope initially")
		}

		scope := compiler.EnterScope()

		if compiler.Scope != scope {
			t.Fatal("expected compiler.Scope to be the new scope")
		}
		if scope.Parent != nil {
			t.Fatal("expected parent to be nil for first scope")
		}
	})

	t.Run("enterScope with params pre-assigns local slots", func(t *testing.T) {
		compiler := NewGenericCompiler()

		scope := compiler.EnterScope("x", "y", "z")

		x, xOk := scope.GetLocal("x")
		y, yOk := scope.GetLocal("y")
		z, zOk := scope.GetLocal("z")
		if !xOk || x != 0 {
			t.Fatalf("expected x=0, got %d (ok=%v)", x, xOk)
		}
		if !yOk || y != 1 {
			t.Fatalf("expected y=1, got %d (ok=%v)", y, yOk)
		}
		if !zOk || z != 2 {
			t.Fatalf("expected z=2, got %d (ok=%v)", z, zOk)
		}
		if scope.NumLocals() != 3 {
			t.Fatalf("expected 3 locals, got %d", scope.NumLocals())
		}
	})

	t.Run("exitScope restores the parent scope", func(t *testing.T) {
		compiler := NewGenericCompiler()
		compiler.EnterScope()
		inner := compiler.EnterScope()

		exited := compiler.ExitScope()

		if exited != inner {
			t.Fatal("expected exited scope to be the inner scope")
		}
		if compiler.Scope == nil {
			t.Fatal("expected compiler.Scope to be the outer scope, not nil")
		}
		if compiler.Scope.Parent != nil {
			t.Fatal("expected outer scope parent to be nil")
		}
	})

	t.Run("nested scopes link via parent pointers", func(t *testing.T) {
		compiler := NewGenericCompiler()
		outer := compiler.EnterScope("a")
		inner := compiler.EnterScope("b")

		if inner.Parent != outer {
			t.Fatal("expected inner.Parent to be outer")
		}
		if outer.Parent != nil {
			t.Fatal("expected outer.Parent to be nil")
		}
	})

	t.Run("exitScope panics when not in a scope", func(t *testing.T) {
		compiler := NewGenericCompiler()

		assertPanic(t, func() {
			compiler.ExitScope()
		}, "CompilerError")
	})

	t.Run("exitScope returns the exited scope for inspection", func(t *testing.T) {
		compiler := NewGenericCompiler()
		scope := compiler.EnterScope("x", "y")
		scope.AddLocal("temp")

		exited := compiler.ExitScope()

		if exited.NumLocals() != 3 {
			t.Fatalf("expected 3 locals, got %d", exited.NumLocals())
		}
		x, _ := exited.GetLocal("x")
		temp, _ := exited.GetLocal("temp")
		if x != 0 {
			t.Fatalf("expected x=0, got %d", x)
		}
		if temp != 2 {
			t.Fatalf("expected temp=2, got %d", temp)
		}
	})
}

// =========================================================================
// CompilerScope (standalone tests)
// =========================================================================

func TestCompilerScope(t *testing.T) {
	t.Run("addLocal assigns consecutive slot indices", func(t *testing.T) {
		scope := NewCompilerScope(nil)

		a := scope.AddLocal("a")
		b := scope.AddLocal("b")
		c := scope.AddLocal("c")

		if a != 0 || b != 1 || c != 2 {
			t.Fatalf("expected 0,1,2; got %d,%d,%d", a, b, c)
		}
	})

	t.Run("addLocal deduplicates — same name returns same slot", func(t *testing.T) {
		scope := NewCompilerScope(nil)

		i1 := scope.AddLocal("x")
		i2 := scope.AddLocal("x")

		if i1 != 0 || i2 != 0 {
			t.Fatalf("expected both 0, got %d and %d", i1, i2)
		}
		if scope.NumLocals() != 1 {
			t.Fatalf("expected 1 local, got %d", scope.NumLocals())
		}
	})

	t.Run("getLocal returns the slot index for known variables", func(t *testing.T) {
		scope := NewCompilerScope(nil)
		scope.AddLocal("param")
		scope.AddLocal("local")

		param, paramOk := scope.GetLocal("param")
		local, localOk := scope.GetLocal("local")

		if !paramOk || param != 0 {
			t.Fatalf("expected param=0, got %d (ok=%v)", param, paramOk)
		}
		if !localOk || local != 1 {
			t.Fatalf("expected local=1, got %d (ok=%v)", local, localOk)
		}
	})

	t.Run("getLocal returns false for unknown variables", func(t *testing.T) {
		scope := NewCompilerScope(nil)

		_, ok := scope.GetLocal("nonexistent")
		if ok {
			t.Fatal("expected ok=false for unknown variable")
		}
	})

	t.Run("numLocals reflects the total count", func(t *testing.T) {
		scope := NewCompilerScope(nil)
		scope.AddLocal("a")
		scope.AddLocal("b")
		scope.AddLocal("c")

		if scope.NumLocals() != 3 {
			t.Fatalf("expected 3, got %d", scope.NumLocals())
		}
	})

	t.Run("numLocals starts at 0 for empty scope", func(t *testing.T) {
		scope := NewCompilerScope(nil)
		if scope.NumLocals() != 0 {
			t.Fatalf("expected 0, got %d", scope.NumLocals())
		}
	})

	t.Run("params are pre-assigned before addLocal", func(t *testing.T) {
		// Simulate what EnterScope does: create scope and add params.
		scope := NewCompilerScope(nil)
		scope.AddLocal("x")
		scope.AddLocal("y")
		tempSlot := scope.AddLocal("temp")

		x, _ := scope.GetLocal("x")
		y, _ := scope.GetLocal("y")

		if x != 0 || y != 1 || tempSlot != 2 {
			t.Fatalf("expected x=0, y=1, temp=2; got %d, %d, %d", x, y, tempSlot)
		}
	})
}

// =========================================================================
// Nested code object compilation
// =========================================================================

func TestNestedCompilation(t *testing.T) {
	t.Run("compileNested returns a separate CodeObject", func(t *testing.T) {
		compiler := NewGenericCompiler()

		compiler.RegisterRule("body", func(c *GenericCompiler, node *ASTNode) {
			idx := c.AddConstant(99)
			c.Emit(vm.OpLoadConst, idx)
			c.AddName("local_var")
		})

		nested := compiler.CompileNested(mkAST("body"))

		if len(nested.Instructions) != 1 {
			t.Fatalf("expected 1 instruction, got %d", len(nested.Instructions))
		}
		if nested.Instructions[0].Opcode != vm.OpLoadConst {
			t.Fatalf("expected OpLoadConst, got %v", nested.Instructions[0].Opcode)
		}
		if !reflect.DeepEqual(nested.Constants, []interface{}{99}) {
			t.Fatalf("expected [99], got %v", nested.Constants)
		}
		if !reflect.DeepEqual(nested.Names, []string{"local_var"}) {
			t.Fatalf("expected [local_var], got %v", nested.Names)
		}
	})

	t.Run("compileNested restores outer state", func(t *testing.T) {
		compiler := NewGenericCompiler()

		// Set up some outer state first.
		compiler.Emit(vm.OpLoadConst, compiler.AddConstant(1))
		compiler.AddName("outer_var")

		outerInstrCount := len(compiler.Instructions)
		outerConstCount := len(compiler.Constants)
		outerNameCount := len(compiler.Names)

		compiler.RegisterRule("inner_body", func(c *GenericCompiler, node *ASTNode) {
			c.Emit(vm.OpAdd)
			c.AddConstant(999)
			c.AddName("inner_var")
		})

		compiler.CompileNested(mkAST("inner_body"))

		// Outer state should be restored.
		if len(compiler.Instructions) != outerInstrCount {
			t.Fatalf("expected %d instructions, got %d", outerInstrCount, len(compiler.Instructions))
		}
		if len(compiler.Constants) != outerConstCount {
			t.Fatalf("expected %d constants, got %d", outerConstCount, len(compiler.Constants))
		}
		if len(compiler.Names) != outerNameCount {
			t.Fatalf("expected %d names, got %d", outerNameCount, len(compiler.Names))
		}
		if !reflect.DeepEqual(compiler.Constants, []interface{}{1}) {
			t.Fatalf("expected [1], got %v", compiler.Constants)
		}
		if !reflect.DeepEqual(compiler.Names, []string{"outer_var"}) {
			t.Fatalf("expected [outer_var], got %v", compiler.Names)
		}
	})

	t.Run("compileNested does not pollute outer instructions", func(t *testing.T) {
		compiler := NewGenericCompiler()
		compiler.Emit(vm.OpLoadConst, 0) // outer instruction

		compiler.RegisterRule("nested", func(c *GenericCompiler, node *ASTNode) {
			c.Emit(vm.OpAdd)
			c.Emit(vm.OpSub)
			c.Emit(vm.OpMul)
		})

		compiler.CompileNested(mkAST("nested"))

		// Outer should still have just the one instruction.
		if len(compiler.Instructions) != 1 {
			t.Fatalf("expected 1 instruction, got %d", len(compiler.Instructions))
		}
		if compiler.Instructions[0].Opcode != vm.OpLoadConst {
			t.Fatalf("expected OpLoadConst, got %v", compiler.Instructions[0].Opcode)
		}
	})
}

// =========================================================================
// Top-level compile
// =========================================================================

func TestTopLevelCompile(t *testing.T) {
	t.Run("appends HALT instruction at the end", func(t *testing.T) {
		compiler := NewGenericCompiler()
		compiler.RegisterRule("program", func(c *GenericCompiler, node *ASTNode) {
			c.Emit(vm.OpLoadConst, c.AddConstant(42))
		})

		code := compiler.Compile(mkAST("program"))

		last := code.Instructions[len(code.Instructions)-1]
		if last.Opcode != vm.OpHalt {
			t.Fatalf("expected OpHalt, got %v", last.Opcode)
		}
	})

	t.Run("supports custom halt opcode", func(t *testing.T) {
		compiler := NewGenericCompiler()
		compiler.RegisterRule("prog", func(c *GenericCompiler, node *ASTNode) {})

		customHalt := vm.OpCode(0xFE)
		code := compiler.Compile(mkAST("prog"), customHalt)

		last := code.Instructions[len(code.Instructions)-1]
		if last.Opcode != customHalt {
			t.Fatalf("expected 0xFE, got %v", last.Opcode)
		}
	})

	t.Run("returns a CodeObject with instructions constants and names", func(t *testing.T) {
		compiler := NewGenericCompiler()
		compiler.RegisterRule("root", func(c *GenericCompiler, node *ASTNode) {
			c.Emit(vm.OpLoadConst, c.AddConstant(10))
			c.Emit(vm.OpStoreName, c.AddName("x"))
		})

		code := compiler.Compile(mkAST("root"))

		// LOAD_CONST, STORE_NAME, HALT = 3 instructions
		if len(code.Instructions) != 3 {
			t.Fatalf("expected 3 instructions, got %d", len(code.Instructions))
		}
		if !reflect.DeepEqual(code.Constants, []interface{}{10}) {
			t.Fatalf("expected [10], got %v", code.Constants)
		}
		if !reflect.DeepEqual(code.Names, []string{"x"}) {
			t.Fatalf("expected [x], got %v", code.Names)
		}
	})

	t.Run("empty program produces just HALT", func(t *testing.T) {
		compiler := NewGenericCompiler()
		compiler.RegisterRule("empty", func(c *GenericCompiler, node *ASTNode) {})

		code := compiler.Compile(mkAST("empty"))

		if len(code.Instructions) != 1 {
			t.Fatalf("expected 1 instruction, got %d", len(code.Instructions))
		}
		if code.Instructions[0].Opcode != vm.OpHalt {
			t.Fatalf("expected OpHalt, got %v", code.Instructions[0].Opcode)
		}
	})
}

// =========================================================================
// Integration test: compile addition expression
// =========================================================================

func TestIntegrationCompileAddition(t *testing.T) {
	t.Run("compiles 1 + 2 to LOAD_CONST LOAD_CONST ADD HALT", func(t *testing.T) {
		compiler := NewGenericCompiler()

		// Handler for number literals: extract the value from the token child.
		compiler.RegisterRule("number", func(c *GenericCompiler, node *ASTNode) {
			token := node.Children[0].(*TokenNode)
			// Parse the number. For simplicity in tests, use integer.
			var value int
			fmt.Sscanf(token.Value, "%d", &value)
			idx := c.AddConstant(value)
			c.Emit(vm.OpLoadConst, idx)
		})

		// Handler for addition: compile left, compile right, emit ADD.
		compiler.RegisterRule("addition", func(c *GenericCompiler, node *ASTNode) {
			c.CompileNode(node.Children[0]) // left operand
			c.CompileNode(node.Children[2]) // right operand (skip PLUS token)
			c.Emit(vm.OpAdd)
		})

		// Build the AST for "1 + 2".
		ast := mkAST("expression",
			mkAST("addition",
				mkAST("number", mkToken("NUMBER", "1")),
				mkToken("PLUS", "+"),
				mkAST("number", mkToken("NUMBER", "2")),
			),
		)

		code := compiler.Compile(ast)

		// Verify instruction sequence.
		expectedOpcodes := []vm.OpCode{
			vm.OpLoadConst, // push 1
			vm.OpLoadConst, // push 2
			vm.OpAdd,       // pop both, push 3
			vm.OpHalt,      // stop
		}
		if !reflect.DeepEqual(opcodes(code.Instructions), expectedOpcodes) {
			t.Fatalf("expected opcodes %v, got %v", expectedOpcodes, opcodes(code.Instructions))
		}

		// Verify constant pool.
		if !reflect.DeepEqual(code.Constants, []interface{}{1, 2}) {
			t.Fatalf("expected [1 2], got %v", code.Constants)
		}

		// Verify operands.
		if code.Instructions[0].Operand != 0 {
			t.Fatalf("expected operand 0 for first LOAD_CONST, got %v", code.Instructions[0].Operand)
		}
		if code.Instructions[1].Operand != 1 {
			t.Fatalf("expected operand 1 for second LOAD_CONST, got %v", code.Instructions[1].Operand)
		}
	})

	t.Run("compiles nested 1 + 2 + 3 with left-associative grouping", func(t *testing.T) {
		// "1 + 2 + 3" is parsed as "(1 + 2) + 3".
		compiler := NewGenericCompiler()

		compiler.RegisterRule("number", func(c *GenericCompiler, node *ASTNode) {
			token := node.Children[0].(*TokenNode)
			var value int
			fmt.Sscanf(token.Value, "%d", &value)
			c.Emit(vm.OpLoadConst, c.AddConstant(value))
		})

		compiler.RegisterRule("addition", func(c *GenericCompiler, node *ASTNode) {
			c.CompileNode(node.Children[0])
			c.CompileNode(node.Children[2])
			c.Emit(vm.OpAdd)
		})

		ast := mkAST("addition",
			mkAST("addition",
				mkAST("number", mkToken("NUMBER", "1")),
				mkToken("PLUS", "+"),
				mkAST("number", mkToken("NUMBER", "2")),
			),
			mkToken("PLUS", "+"),
			mkAST("number", mkToken("NUMBER", "3")),
		)

		code := compiler.Compile(ast)

		expectedOpcodes := []vm.OpCode{
			vm.OpLoadConst, // 1
			vm.OpLoadConst, // 2
			vm.OpAdd,       // 1 + 2
			vm.OpLoadConst, // 3
			vm.OpAdd,       // (1+2) + 3
			vm.OpHalt,
		}
		if !reflect.DeepEqual(opcodes(code.Instructions), expectedOpcodes) {
			t.Fatalf("expected opcodes %v, got %v", expectedOpcodes, opcodes(code.Instructions))
		}
		if !reflect.DeepEqual(code.Constants, []interface{}{1, 2, 3}) {
			t.Fatalf("expected [1 2 3], got %v", code.Constants)
		}
	})

	t.Run("compiles variable assignment and lookup", func(t *testing.T) {
		// "x = 42" followed by "x" — tests STORE_NAME and LOAD_NAME.
		compiler := NewGenericCompiler()

		compiler.RegisterRule("number", func(c *GenericCompiler, node *ASTNode) {
			token := node.Children[0].(*TokenNode)
			var value int
			fmt.Sscanf(token.Value, "%d", &value)
			c.Emit(vm.OpLoadConst, c.AddConstant(value))
		})

		compiler.RegisterRule("assignment", func(c *GenericCompiler, node *ASTNode) {
			nameToken := node.Children[0].(*TokenNode)
			c.CompileNode(node.Children[2]) // compile the value
			c.Emit(vm.OpStoreName, c.AddName(nameToken.Value))
		})

		compiler.RegisterRule("name_ref", func(c *GenericCompiler, node *ASTNode) {
			token := node.Children[0].(*TokenNode)
			c.Emit(vm.OpLoadName, c.AddName(token.Value))
		})

		compiler.RegisterRule("program", func(c *GenericCompiler, node *ASTNode) {
			for _, child := range node.Children {
				c.CompileNode(child)
			}
		})

		ast := mkAST("program",
			mkAST("assignment",
				mkToken("IDENT", "x"),
				mkToken("EQUALS", "="),
				mkAST("number", mkToken("NUMBER", "42")),
			),
			mkAST("name_ref",
				mkToken("IDENT", "x"),
			),
		)

		code := compiler.Compile(ast)

		expectedOpcodes := []vm.OpCode{
			vm.OpLoadConst,  // 42
			vm.OpStoreName,  // x
			vm.OpLoadName,   // x
			vm.OpHalt,
		}
		if !reflect.DeepEqual(opcodes(code.Instructions), expectedOpcodes) {
			t.Fatalf("expected opcodes %v, got %v", expectedOpcodes, opcodes(code.Instructions))
		}
		if !reflect.DeepEqual(code.Constants, []interface{}{42}) {
			t.Fatalf("expected [42], got %v", code.Constants)
		}
		if !reflect.DeepEqual(code.Names, []string{"x"}) {
			t.Fatalf("expected [x], got %v", code.Names)
		}
	})
}
