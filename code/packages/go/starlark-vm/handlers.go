// handlers.go — All 59 opcode handlers for the Starlark virtual machine.
//
// ════════════════════════════════════════════════════════════════════════
// OVERVIEW
// ════════════════════════════════════════════════════════════════════════
//
// This file is the "brain" of the Starlark VM.  While the GenericVM
// from the virtual-machine package provides the execution loop (fetch,
// decode, execute), it delegates the actual work to opcode handlers.
// Each handler is a function that:
//
//   1. Reads its operand from the instruction.
//   2. Pops values from the stack (if needed).
//   3. Performs some computation.
//   4. Pushes the result onto the stack (if needed).
//   5. Advances the program counter (or jumps).
//
// Think of the GenericVM as a mail carrier — it delivers envelopes
// (instructions) to the right mailbox (handler).  The handler opens
// the envelope, does the work, and may send a reply (push a result).
//
// ════════════════════════════════════════════════════════════════════════
// HANDLER CATEGORIES
// ════════════════════════════════════════════════════════════════════════
//
//   1. Stack manipulation   — Load constants, push literals, dup, pop
//   2. Variable access      — Store/load named and local variables
//   3. Arithmetic           — Add, sub, mul, div, floor div, mod, power
//   4. Bitwise operators    — AND, OR, XOR, NOT, shifts
//   5. Comparisons          — ==, !=, <, >, <=, >=, in, not in
//   6. Logical operators    — not, short-circuit and/or
//   7. Control flow         — Jumps, break, continue
//   8. Functions            — Make, call, return
//   9. Collections          — Build list/dict/tuple, append, set
//  10. Subscript/attribute  — Index, key, attribute access
//  11. Iteration            — Get iterator, for-iter, unpack
//  12. Modules              — Load module, import from (stubs)
//  13. Output               — Print
//  14. Halt                 — Stop execution
//
// ════════════════════════════════════════════════════════════════════════
// HOW HANDLERS INTERACT WITH THE VM
// ════════════════════════════════════════════════════════════════════════
//
//   ┌───────────────────────────────────────────────┐
//   │                 GenericVM                      │
//   │                                               │
//   │  Stack: [10, 20]    PC: 3    Halted: false    │
//   │                                               │
//   │  Instruction at PC=3: ADD (opcode 0x20)       │
//   │                                               │
//   │  → Looks up handler for 0x20 → handleAdd      │
//   │  → handleAdd pops 20, pops 10, pushes 30      │
//   │  → handleAdd calls vm.AdvancePC()             │
//   │                                               │
//   │  Stack: [30]        PC: 4    Halted: false    │
//   └───────────────────────────────────────────────┘
//
package starlarkvm

import (
	"fmt"
	"math"
	"sort"
	"strings"

	op "github.com/adhithyan15/coding-adventures/code/packages/go/starlark-ast-to-bytecode-compiler"
	vm "github.com/adhithyan15/coding-adventures/code/packages/go/virtual-machine"
)

// registerAllHandlers wires up every opcode handler into the GenericVM.
// After calling this, the VM knows how to execute all Starlark bytecode.
//
// This is called once during VM creation (in CreateStarlarkVM).
func registerAllHandlers(v *vm.GenericVM) {
	// ── Stack manipulation ──────────────────────────────────────────
	v.RegisterOpcode(op.OpLoadConst, handleLoadConst)
	v.RegisterOpcode(op.OpPop, handlePop)
	v.RegisterOpcode(op.OpDup, handleDup)
	v.RegisterOpcode(op.OpLoadNone, handleLoadNone)
	v.RegisterOpcode(op.OpLoadTrue, handleLoadTrue)
	v.RegisterOpcode(op.OpLoadFalse, handleLoadFalse)

	// ── Variable access ─────────────────────────────────────────────
	v.RegisterOpcode(op.OpStoreName, handleStoreName)
	v.RegisterOpcode(op.OpLoadName, handleLoadName)
	v.RegisterOpcode(op.OpStoreLocal, handleStoreLocal)
	v.RegisterOpcode(op.OpLoadLocal, handleLoadLocal)
	v.RegisterOpcode(op.OpStoreClosure, handleStoreClosure)
	v.RegisterOpcode(op.OpLoadClosure, handleLoadClosure)

	// ── Arithmetic ──────────────────────────────────────────────────
	v.RegisterOpcode(op.OpAdd, handleAdd)
	v.RegisterOpcode(op.OpSub, handleSub)
	v.RegisterOpcode(op.OpMul, handleMul)
	v.RegisterOpcode(op.OpDiv, handleDiv)
	v.RegisterOpcode(op.OpFloorDiv, handleFloorDiv)
	v.RegisterOpcode(op.OpMod, handleMod)
	v.RegisterOpcode(op.OpPower, handlePower)
	v.RegisterOpcode(op.OpNegate, handleNegate)

	// ── Bitwise ─────────────────────────────────────────────────────
	v.RegisterOpcode(op.OpBitAnd, handleBitAnd)
	v.RegisterOpcode(op.OpBitOr, handleBitOr)
	v.RegisterOpcode(op.OpBitXor, handleBitXor)
	v.RegisterOpcode(op.OpBitNot, handleBitNot)
	v.RegisterOpcode(op.OpLShift, handleLShift)
	v.RegisterOpcode(op.OpRShift, handleRShift)

	// ── Comparisons ─────────────────────────────────────────────────
	v.RegisterOpcode(op.OpCmpEq, handleCmpEq)
	v.RegisterOpcode(op.OpCmpNe, handleCmpNe)
	v.RegisterOpcode(op.OpCmpLt, handleCmpLt)
	v.RegisterOpcode(op.OpCmpGt, handleCmpGt)
	v.RegisterOpcode(op.OpCmpLe, handleCmpLe)
	v.RegisterOpcode(op.OpCmpGe, handleCmpGe)
	v.RegisterOpcode(op.OpCmpIn, handleCmpIn)
	v.RegisterOpcode(op.OpCmpNotIn, handleCmpNotIn)
	v.RegisterOpcode(op.OpNot, handleNot)

	// ── Control flow ────────────────────────────────────────────────
	v.RegisterOpcode(op.OpJump, handleJump)
	v.RegisterOpcode(op.OpJumpIfFalse, handleJumpIfFalse)
	v.RegisterOpcode(op.OpJumpIfTrue, handleJumpIfTrue)
	v.RegisterOpcode(op.OpJumpIfFalseOrPop, handleJumpIfFalseOrPop)
	v.RegisterOpcode(op.OpJumpIfTrueOrPop, handleJumpIfTrueOrPop)
	v.RegisterOpcode(op.OpBreak, handleBreak)
	v.RegisterOpcode(op.OpContinue, handleContinue)

	// ── Functions ───────────────────────────────────────────────────
	v.RegisterOpcode(op.OpMakeFunction, handleMakeFunction)
	v.RegisterOpcode(op.OpCallFunction, handleCallFunction)
	v.RegisterOpcode(op.OpCallFunctionKW, handleCallFunctionKW)
	v.RegisterOpcode(op.OpReturnValue, handleReturnValue)

	// ── Collections ─────────────────────────────────────────────────
	v.RegisterOpcode(op.OpBuildList, handleBuildList)
	v.RegisterOpcode(op.OpBuildDict, handleBuildDict)
	v.RegisterOpcode(op.OpBuildTuple, handleBuildTuple)
	v.RegisterOpcode(op.OpListAppend, handleListAppend)
	v.RegisterOpcode(op.OpDictSet, handleDictSet)

	// ── Subscript/attribute ─────────────────────────────────────────
	v.RegisterOpcode(op.OpLoadSubscript, handleLoadSubscript)
	v.RegisterOpcode(op.OpStoreSubscript, handleStoreSubscript)
	v.RegisterOpcode(op.OpLoadAttr, handleLoadAttr)
	v.RegisterOpcode(op.OpStoreAttr, handleStoreAttr)
	v.RegisterOpcode(op.OpLoadSlice, handleLoadSlice)

	// ── Iteration ───────────────────────────────────────────────────
	v.RegisterOpcode(op.OpGetIter, handleGetIter)
	v.RegisterOpcode(op.OpForIter, handleForIter)
	v.RegisterOpcode(op.OpUnpackSequence, handleUnpackSequence)

	// ── Modules ─────────────────────────────────────────────────────
	v.RegisterOpcode(op.OpLoadModule, handleLoadModule)
	v.RegisterOpcode(op.OpImportFrom, handleImportFrom)

	// ── Output ──────────────────────────────────────────────────────
	v.RegisterOpcode(op.OpPrintValue, handlePrintValue)

	// ── Halt ────────────────────────────────────────────────────────
	v.RegisterOpcode(op.OpHalt, handleHalt)
}

// ════════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS — Truthiness, type conversion, numeric operations
// ════════════════════════════════════════════════════════════════════════
//
// In Starlark (and Python), every value has a "truthiness":
//   - Falsy: None, False, 0, 0.0, "", [], {}
//   - Truthy: everything else
//
// This is different from strongly-typed languages like Go or Java where
// only booleans can be used in conditions.  In Starlark, `if []:` is
// valid and evaluates to false because empty lists are falsy.

// isFalsy returns true if the given value is considered "false" in
// Starlark's truth system.
//
// Truth table:
//
//   Value               | isFalsy
//   ────────────────────┼────────
//   nil (None)          | true
//   false               | true
//   0                   | true
//   0.0                 | true
//   ""                  | true
//   [] (empty list)     | true
//   {} (empty dict)     | true
//   42                  | false
//   "hello"             | false
//   [1, 2]              | false
//   true                | false
func isFalsy(val interface{}) bool {
	if val == nil {
		return true
	}
	switch v := val.(type) {
	case bool:
		return !v
	case int:
		return v == 0
	case float64:
		return v == 0.0
	case string:
		return v == ""
	case []interface{}:
		return len(v) == 0
	case map[string]interface{}:
		return len(v) == 0
	}
	return false
}

// toFloat converts an int or float64 to float64.
// Used by arithmetic handlers that need to support mixed int/float math.
//
//   toFloat(42)    → 42.0
//   toFloat(3.14)  → 3.14
func toFloat(val interface{}) float64 {
	switch v := val.(type) {
	case int:
		return float64(v)
	case float64:
		return v
	}
	panic(fmt.Sprintf("TypeError: cannot convert %T to float", val))
}

// toInt converts a numeric value to int.
//
//   toInt(42)    → 42
//   toInt(3.0)   → 3
func toInt(val interface{}) int {
	switch v := val.(type) {
	case int:
		return v
	case float64:
		return int(v)
	case bool:
		if v {
			return 1
		}
		return 0
	}
	panic(fmt.Sprintf("TypeError: cannot convert %T to int", val))
}

// isNumeric returns true if the value is int or float64.
func isNumeric(val interface{}) bool {
	switch val.(type) {
	case int, float64:
		return true
	}
	return false
}

// isFloat returns true if either operand is float64.
// Used to decide whether arithmetic should produce int or float results.
//
//   isFloat(1, 2)     → false (both int, result is int)
//   isFloat(1, 2.0)   → true  (one float, result is float)
//   isFloat(1.0, 2.0) → true
func isFloat(a, b interface{}) bool {
	_, aF := a.(float64)
	_, bF := b.(float64)
	return aF || bF
}

// numericBinary performs a binary operation on two numeric values.
// If either operand is float64, the float operation is used.
// Otherwise, the int operation is used.
//
// This avoids repeating the same type-switch logic in every arithmetic handler.
func numericBinary(a, b interface{}, intOp func(int, int) interface{}, floatOp func(float64, float64) interface{}) interface{} {
	if isFloat(a, b) {
		return floatOp(toFloat(a), toFloat(b))
	}
	return intOp(toInt(a), toInt(b))
}

// compareValues compares two values and returns -1, 0, or 1.
// Works for int, float64, and string types.
//
//   compareValues(1, 2)       → -1  (1 < 2)
//   compareValues("b", "a")   →  1  ("b" > "a")
//   compareValues(3.0, 3.0)   →  0  (equal)
func compareValues(a, b interface{}) int {
	// Both numeric — compare as numbers.
	if isNumeric(a) && isNumeric(b) {
		af, bf := toFloat(a), toFloat(b)
		if af < bf {
			return -1
		}
		if af > bf {
			return 1
		}
		return 0
	}

	// Both strings — lexicographic comparison.
	aStr, aIsStr := a.(string)
	bStr, bIsStr := b.(string)
	if aIsStr && bIsStr {
		if aStr < bStr {
			return -1
		}
		if aStr > bStr {
			return 1
		}
		return 0
	}

	// Both bools — true > false.
	aBool, aIsBool := a.(bool)
	bBool, bIsBool := b.(bool)
	if aIsBool && bIsBool {
		ai, bi := 0, 0
		if aBool {
			ai = 1
		}
		if bBool {
			bi = 1
		}
		if ai < bi {
			return -1
		}
		if ai > bi {
			return 1
		}
		return 0
	}

	panic(fmt.Sprintf("TypeError: cannot compare %T and %T", a, b))
}

// formatValue converts a value to its Starlark string representation.
// This is used by print() and str() to produce human-readable output.
//
//   formatValue(42)           → "42"
//   formatValue("hello")      → "hello"
//   formatValue(true)         → "True"
//   formatValue(nil)          → "None"
//   formatValue([]interface{}{1,2}) → "[1, 2]"
func formatValue(val interface{}) string {
	if val == nil {
		return "None"
	}
	switch v := val.(type) {
	case bool:
		if v {
			return "True"
		}
		return "False"
	case int:
		return fmt.Sprintf("%d", v)
	case float64:
		// Format floats to match Starlark output (e.g., 3.14, not 3.140000).
		s := fmt.Sprintf("%g", v)
		// Ensure there's always a decimal point for floats.
		if !strings.Contains(s, ".") && !strings.Contains(s, "e") {
			s += ".0"
		}
		return s
	case string:
		return v
	case []interface{}:
		parts := make([]string, len(v))
		for i, item := range v {
			parts[i] = reprValue(item)
		}
		return "[" + strings.Join(parts, ", ") + "]"
	case map[string]interface{}:
		parts := make([]string, 0, len(v))
		for key, val := range v {
			parts = append(parts, reprValue(key)+": "+reprValue(val))
		}
		sort.Strings(parts)
		return "{" + strings.Join(parts, ", ") + "}"
	case *StarlarkFunction:
		return fmt.Sprintf("<function %s>", v.Name)
	default:
		return fmt.Sprintf("%v", v)
	}
}

// reprValue returns the repr() form of a value — with quotes around strings.
//
//   reprValue("hello") → `"hello"`
//   reprValue(42)       → "42"
func reprValue(val interface{}) string {
	if s, ok := val.(string); ok {
		return fmt.Sprintf("%q", s)
	}
	return formatValue(val)
}

// copySlice creates a shallow copy of a slice.
func copySlice(s []interface{}) []interface{} {
	c := make([]interface{}, len(s))
	copy(c, s)
	return c
}

// copyMap creates a shallow copy of a map.
func copyMap(m map[string]interface{}) map[string]interface{} {
	c := make(map[string]interface{})
	for k, v := range m {
		c[k] = v
	}
	return c
}

// ════════════════════════════════════════════════════════════════════════
// 1. STACK MANIPULATION HANDLERS
// ════════════════════════════════════════════════════════════════════════
//
// These handlers push values onto the stack or manipulate the stack's
// top element.  They are the foundation — almost every program starts
// by loading constants or literals.

// handleLoadConst pushes a constant from the code object's constant pool.
//
// The operand is an index into code.Constants.  For example, if the
// source code says `x = 42`, the compiler puts 42 into the constants
// pool at index 0 and emits LOAD_CONST 0.
//
//   Before: Stack = [...]
//   After:  Stack = [..., constants[operand]]
func handleLoadConst(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	idx := instr.Operand.(int)
	v.Push(code.Constants[idx])
	v.AdvancePC()
	return nil
}

// handlePop discards the top value from the stack.
//
// This is used after expression statements — the result of an expression
// that isn't assigned to anything needs to be cleaned up.
//
//   Before: Stack = [..., value]
//   After:  Stack = [...]
func handlePop(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	v.Pop()
	v.AdvancePC()
	return nil
}

// handleDup duplicates the top value on the stack.
//
// Useful when the same value is needed by two consecutive operations.
// For example, `x = y = 42` might DUP the 42 so it can be stored twice.
//
//   Before: Stack = [..., value]
//   After:  Stack = [..., value, value]
func handleDup(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	v.Push(v.Peek())
	v.AdvancePC()
	return nil
}

// handleLoadNone pushes Starlark's None value (Go nil) onto the stack.
//
// None is Starlark's equivalent of null/nil/undefined in other languages.
// It's the default return value for functions that don't explicitly return.
func handleLoadNone(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	v.Push(nil)
	v.AdvancePC()
	return nil
}

// handleLoadTrue pushes the boolean value true onto the stack.
func handleLoadTrue(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	v.Push(true)
	v.AdvancePC()
	return nil
}

// handleLoadFalse pushes the boolean value false onto the stack.
func handleLoadFalse(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	v.Push(false)
	v.AdvancePC()
	return nil
}

// ════════════════════════════════════════════════════════════════════════
// 2. VARIABLE ACCESS HANDLERS
// ════════════════════════════════════════════════════════════════════════
//
// Variables in Starlark come in two flavors:
//
//   Named variables — stored in vm.Variables (a map[string]interface{}).
//                     These are like "global" variables at the module level.
//                     Accessed by name string via code.Names.
//
//   Local variables — stored in vm.Locals (a []interface{}).
//                     These are numbered slots used inside functions.
//                     Accessed by integer index.
//
// The compiler decides which to use based on scope analysis.

// handleStoreName pops the top value and stores it in a named variable.
//
// Operand: index into code.Names for the variable name.
//
//   Example: `x = 42`
//   Before: Stack = [..., 42]    Variables = {}
//   After:  Stack = [...]        Variables = {"x": 42}
func handleStoreName(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	idx := instr.Operand.(int)
	name := code.Names[idx]
	val := v.Pop()
	v.Variables[name] = val
	v.AdvancePC()
	return nil
}

// handleLoadName looks up a named variable and pushes its value.
//
// The lookup order is:
//   1. vm.Variables (named/global scope)
//   2. Registered builtins (len, print, range, etc.)
//
// If the name isn't found in either place, the handler panics with
// "UndefinedNameError" — just like Python raises NameError.
//
//   Example: after `x = 42`, LOAD_NAME "x" pushes 42.
func handleLoadName(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	idx := instr.Operand.(int)
	name := code.Names[idx]

	// First, check named variables.
	if val, ok := v.Variables[name]; ok {
		v.Push(val)
		v.AdvancePC()
		return nil
	}

	// Second, check builtins.
	builtin := v.GetBuiltin(name)
	if builtin != nil {
		v.Push(builtin)
		v.AdvancePC()
		return nil
	}

	panic(fmt.Sprintf("UndefinedNameError: name '%s' is not defined", name))
}

// handleStoreLocal stores a value into a numbered local variable slot.
//
// Local variables are stored in a flat array (vm.Locals) indexed by
// integer slot numbers.  This is much faster than map lookups for named
// variables, which is why compilers use locals for function parameters
// and local variables.
//
// If the slot doesn't exist yet, the locals array is extended with nil
// values until it's large enough.
func handleStoreLocal(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	idx := instr.Operand.(int)
	val := v.Pop()
	for len(v.Locals) <= idx {
		v.Locals = append(v.Locals, nil)
	}
	v.Locals[idx] = val
	v.AdvancePC()
	return nil
}

// handleLoadLocal pushes the value from a local variable slot.
func handleLoadLocal(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	idx := instr.Operand.(int)
	if idx >= len(v.Locals) {
		v.Push(nil)
	} else {
		v.Push(v.Locals[idx])
	}
	v.AdvancePC()
	return nil
}

// handleStoreClosure stores a value into a closure variable.
// Currently implemented as an alias for StoreLocal — full closure
// support (capturing variables from enclosing scopes) will be added
// when nested function definitions are supported.
func handleStoreClosure(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	return handleStoreLocal(v, instr, code)
}

// handleLoadClosure loads a value from a closure variable.
// Currently implemented as an alias for LoadLocal.
func handleLoadClosure(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	return handleLoadLocal(v, instr, code)
}

// ════════════════════════════════════════════════════════════════════════
// 3. ARITHMETIC HANDLERS
// ════════════════════════════════════════════════════════════════════════
//
// All binary arithmetic handlers follow the same pattern:
//   1. Pop b (right operand — it was pushed second, so it's on top).
//   2. Pop a (left operand).
//   3. Compute a OP b.
//   4. Push the result.
//   5. Advance PC.
//
// Starlark supports mixed int/float arithmetic.  If either operand is
// float64, the result is float64.  If both are int, the result is int.
//
// Additionally, the + operator is overloaded for strings (concatenation)
// and lists (concatenation), and * is overloaded for string/list repetition.

// handleAdd performs addition, string concatenation, or list concatenation.
//
// Type behavior:
//   int + int       → int      (3 + 4 = 7)
//   float + float   → float    (1.5 + 2.5 = 4.0)
//   int + float     → float    (1 + 2.5 = 3.5)
//   str + str       → str      ("hello" + " world" = "hello world")
//   list + list     → list     ([1,2] + [3,4] = [1,2,3,4])
func handleAdd(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	b := v.Pop()
	a := v.Pop()

	// String concatenation.
	if aStr, ok := a.(string); ok {
		if bStr, ok := b.(string); ok {
			v.Push(aStr + bStr)
			v.AdvancePC()
			return nil
		}
	}

	// List concatenation.
	if aList, ok := a.([]interface{}); ok {
		if bList, ok := b.([]interface{}); ok {
			result := make([]interface{}, 0, len(aList)+len(bList))
			result = append(result, aList...)
			result = append(result, bList...)
			v.Push(result)
			v.AdvancePC()
			return nil
		}
	}

	// Numeric addition.
	v.Push(numericBinary(a, b,
		func(ai, bi int) interface{} { return ai + bi },
		func(af, bf float64) interface{} { return af + bf },
	))
	v.AdvancePC()
	return nil
}

// handleSub performs subtraction: a - b.
func handleSub(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	b := v.Pop()
	a := v.Pop()
	v.Push(numericBinary(a, b,
		func(ai, bi int) interface{} { return ai - bi },
		func(af, bf float64) interface{} { return af - bf },
	))
	v.AdvancePC()
	return nil
}

// handleMul performs multiplication.  Also supports string and list
// repetition: "ab" * 3 = "ababab", [1,2] * 2 = [1,2,1,2].
func handleMul(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	b := v.Pop()
	a := v.Pop()

	// String repetition: str * int or int * str.
	if s, ok := a.(string); ok {
		if n, ok := b.(int); ok {
			v.Push(strings.Repeat(s, n))
			v.AdvancePC()
			return nil
		}
	}
	if n, ok := a.(int); ok {
		if s, ok := b.(string); ok {
			v.Push(strings.Repeat(s, n))
			v.AdvancePC()
			return nil
		}
	}

	// List repetition: list * int or int * list.
	if lst, ok := a.([]interface{}); ok {
		if n, ok := b.(int); ok {
			result := make([]interface{}, 0, len(lst)*n)
			for i := 0; i < n; i++ {
				result = append(result, lst...)
			}
			v.Push(result)
			v.AdvancePC()
			return nil
		}
	}

	// Numeric multiplication.
	v.Push(numericBinary(a, b,
		func(ai, bi int) interface{} { return ai * bi },
		func(af, bf float64) interface{} { return af * bf },
	))
	v.AdvancePC()
	return nil
}

// handleDiv performs true division: a / b.
//
// In Starlark, the / operator always produces a float result
// (unlike Python 2's integer division).
//   10 / 3  → 3.3333...
//   6 / 2   → 3.0
func handleDiv(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	b := v.Pop()
	a := v.Pop()
	bf := toFloat(b)
	af := toFloat(a)
	if bf == 0.0 {
		panic("ZeroDivisionError: division by zero")
	}
	v.Push(af / bf)
	v.AdvancePC()
	return nil
}

// handleFloorDiv performs floor division: a // b.
//
// Floor division rounds toward negative infinity:
//   7 // 2   → 3
//  -7 // 2   → -4 (not -3, because floor rounds down)
//   7 // -2  → -4
func handleFloorDiv(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	b := v.Pop()
	a := v.Pop()

	if isFloat(a, b) {
		bf := toFloat(b)
		af := toFloat(a)
		if bf == 0.0 {
			panic("ZeroDivisionError: floor division by zero")
		}
		v.Push(math.Floor(af / bf))
	} else {
		bi := toInt(b)
		ai := toInt(a)
		if bi == 0 {
			panic("ZeroDivisionError: floor division by zero")
		}
		// Go's integer division truncates toward zero; we need floor toward -inf.
		result := ai / bi
		if (ai^bi) < 0 && result*bi != ai {
			result--
		}
		v.Push(result)
	}
	v.AdvancePC()
	return nil
}

// handleMod performs the modulo operation: a % b.
//
// If a is a string, this acts as Python-style string formatting:
//   "%s has %d items" % ("list", 3) → "list has 3 items"
//
// For numbers, the result has the same sign as the divisor (Python semantics):
//   7 % 3   →  1
//  -7 % 3   →  2 (not -1)
func handleMod(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	b := v.Pop()
	a := v.Pop()

	// String formatting: "format" % args.
	if fmtStr, ok := a.(string); ok {
		var args []interface{}
		if tuple, ok := b.([]interface{}); ok {
			args = tuple
		} else {
			args = []interface{}{b}
		}
		v.Push(fmt.Sprintf(fmtStr, args...))
		v.AdvancePC()
		return nil
	}

	if isFloat(a, b) {
		bf := toFloat(b)
		af := toFloat(a)
		if bf == 0.0 {
			panic("ZeroDivisionError: modulo by zero")
		}
		v.Push(math.Mod(af, bf) + math.Copysign(0, bf))
	} else {
		bi := toInt(b)
		ai := toInt(a)
		if bi == 0 {
			panic("ZeroDivisionError: modulo by zero")
		}
		result := ai % bi
		// Python semantics: result has same sign as divisor.
		if result != 0 && (result^bi) < 0 {
			result += bi
		}
		v.Push(result)
	}
	v.AdvancePC()
	return nil
}

// handlePower performs exponentiation: a ** b.
//
//   2 ** 10  → 1024
//   2 ** -1  → 0.5 (float result)
func handlePower(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	b := v.Pop()
	a := v.Pop()

	af := toFloat(a)
	bf := toFloat(b)
	result := math.Pow(af, bf)

	// If both operands are int and the exponent is non-negative,
	// return an int result.
	_, aIsInt := a.(int)
	bInt, bIsInt := b.(int)
	if aIsInt && bIsInt && bInt >= 0 {
		v.Push(int(result))
	} else {
		v.Push(result)
	}
	v.AdvancePC()
	return nil
}

// handleNegate performs unary negation: -a.
//
//   -(5)    → -5
//   -(-3)   → 3
//   -(1.5)  → -1.5
func handleNegate(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	a := v.Pop()
	switch val := a.(type) {
	case int:
		v.Push(-val)
	case float64:
		v.Push(-val)
	default:
		panic(fmt.Sprintf("TypeError: bad operand type for unary -: '%T'", a))
	}
	v.AdvancePC()
	return nil
}

// ════════════════════════════════════════════════════════════════════════
// 4. BITWISE OPERATOR HANDLERS
// ════════════════════════════════════════════════════════════════════════
//
// Bitwise operators work on the individual bits of integer values.
// They are not defined for floats or strings in Starlark.
//
// Quick refresher on bit operations (for a single bit):
//
//   a | b | a&b | a|b | a^b | ~a
//   ──┼───┼─────┼─────┼─────┼────
//   0 | 0 |  0  |  0  |  0  |  1
//   0 | 1 |  0  |  1  |  1  |  1
//   1 | 0 |  0  |  1  |  1  |  0
//   1 | 1 |  1  |  1  |  0  |  0

// handleBitAnd: bitwise AND.  Each result bit is 1 only if BOTH input bits are 1.
//   12 & 10 = 8   (1100 & 1010 = 1000)
func handleBitAnd(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	b := toInt(v.Pop())
	a := toInt(v.Pop())
	v.Push(a & b)
	v.AdvancePC()
	return nil
}

// handleBitOr: bitwise OR.  Each result bit is 1 if EITHER input bit is 1.
//   12 | 10 = 14  (1100 | 1010 = 1110)
func handleBitOr(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	b := toInt(v.Pop())
	a := toInt(v.Pop())
	v.Push(a | b)
	v.AdvancePC()
	return nil
}

// handleBitXor: bitwise XOR.  Each result bit is 1 if the input bits DIFFER.
//   12 ^ 10 = 6   (1100 ^ 1010 = 0110)
func handleBitXor(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	b := toInt(v.Pop())
	a := toInt(v.Pop())
	v.Push(a ^ b)
	v.AdvancePC()
	return nil
}

// handleBitNot: bitwise complement (unary).  Flips every bit.
//   ~0  = -1  (in two's complement: all bits flip from 0 to 1)
//   ~5  = -6  (binary 0101 → 1010, which is -6 in two's complement)
func handleBitNot(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	a := toInt(v.Pop())
	v.Push(^a)
	v.AdvancePC()
	return nil
}

// handleLShift: left shift.  Shifts bits left, filling with zeros.
//   1 << 3 = 8   (0001 → 1000)
func handleLShift(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	b := toInt(v.Pop())
	a := toInt(v.Pop())
	v.Push(a << uint(b))
	v.AdvancePC()
	return nil
}

// handleRShift: right shift.  Shifts bits right (arithmetic shift for signed).
//   8 >> 2 = 2   (1000 → 0010)
func handleRShift(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	b := toInt(v.Pop())
	a := toInt(v.Pop())
	v.Push(a >> uint(b))
	v.AdvancePC()
	return nil
}

// ════════════════════════════════════════════════════════════════════════
// 5. COMPARISON HANDLERS
// ════════════════════════════════════════════════════════════════════════
//
// All comparison handlers pop two values (b then a), compare them,
// and push a boolean result (true or false).
//
// In Starlark (like Python), comparisons return proper booleans,
// not integers.  This differs from C where comparisons return 0 or 1.

// handleCmpEq: equality comparison (==).
//   1 == 1     → true
//   "a" == "b" → false
func handleCmpEq(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	b := v.Pop()
	a := v.Pop()
	// Use type-aware comparison for numeric types.
	if isNumeric(a) && isNumeric(b) {
		v.Push(toFloat(a) == toFloat(b))
	} else {
		v.Push(a == b)
	}
	v.AdvancePC()
	return nil
}

// handleCmpNe: not-equal comparison (!=).
func handleCmpNe(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	b := v.Pop()
	a := v.Pop()
	if isNumeric(a) && isNumeric(b) {
		v.Push(toFloat(a) != toFloat(b))
	} else {
		v.Push(a != b)
	}
	v.AdvancePC()
	return nil
}

// handleCmpLt: less-than comparison (<).
func handleCmpLt(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	b := v.Pop()
	a := v.Pop()
	v.Push(compareValues(a, b) < 0)
	v.AdvancePC()
	return nil
}

// handleCmpGt: greater-than comparison (>).
func handleCmpGt(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	b := v.Pop()
	a := v.Pop()
	v.Push(compareValues(a, b) > 0)
	v.AdvancePC()
	return nil
}

// handleCmpLe: less-than-or-equal comparison (<=).
func handleCmpLe(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	b := v.Pop()
	a := v.Pop()
	v.Push(compareValues(a, b) <= 0)
	v.AdvancePC()
	return nil
}

// handleCmpGe: greater-than-or-equal comparison (>=).
func handleCmpGe(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	b := v.Pop()
	a := v.Pop()
	v.Push(compareValues(a, b) >= 0)
	v.AdvancePC()
	return nil
}

// handleCmpIn: membership test (in).
//
// Checks whether value `a` is contained in collection `b`.
//   "x" in "xyz"          → true  (substring)
//   3 in [1, 2, 3]        → true  (list membership)
//   "key" in {"key": 42}  → true  (dict key membership)
func handleCmpIn(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	b := v.Pop()
	a := v.Pop()
	v.Push(containsValue(b, a))
	v.AdvancePC()
	return nil
}

// handleCmpNotIn: negated membership test (not in).
func handleCmpNotIn(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	b := v.Pop()
	a := v.Pop()
	v.Push(!containsValue(b, a))
	v.AdvancePC()
	return nil
}

// containsValue checks if `needle` is in `haystack`.
//
// Supported haystacks:
//   - []interface{} (list/tuple): checks each element for equality.
//   - map[string]interface{} (dict): checks if needle is a key.
//   - string: checks if needle is a substring.
func containsValue(haystack, needle interface{}) bool {
	switch h := haystack.(type) {
	case []interface{}:
		for _, item := range h {
			if item == needle {
				return true
			}
			// Handle numeric comparison (1 == 1.0).
			if isNumeric(item) && isNumeric(needle) {
				if toFloat(item) == toFloat(needle) {
					return true
				}
			}
		}
		return false
	case map[string]interface{}:
		key, ok := needle.(string)
		if !ok {
			return false
		}
		_, found := h[key]
		return found
	case string:
		needleStr, ok := needle.(string)
		if !ok {
			return false
		}
		return strings.Contains(h, needleStr)
	}
	return false
}

// handleNot: logical NOT.
//
// Starlark's `not` operator returns a boolean — the opposite of the
// value's truthiness.
//   not True  → False
//   not 0     → True
//   not []    → True
//   not "hi"  → False
func handleNot(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	a := v.Pop()
	v.Push(isFalsy(a))
	v.AdvancePC()
	return nil
}

// ════════════════════════════════════════════════════════════════════════
// 6. CONTROL FLOW HANDLERS
// ════════════════════════════════════════════════════════════════════════
//
// Control flow instructions change the program counter (PC) to execute
// instructions out of order.  Without them, programs could only run
// in a straight line from top to bottom.
//
// Jumps enable:
//   - if/else  → conditional jump over one branch
//   - for/while → jump back to loop start
//   - and/or   → short-circuit evaluation (skip second operand)

// handleJump: unconditional jump.
// Sets PC to the operand value.  Used for `else` branches and loop backs.
func handleJump(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	target := instr.Operand.(int)
	v.JumpTo(target)
	return nil
}

// handleJumpIfFalse: conditional jump — pops TOS, jumps if falsy.
//
// This is the core of `if` statements.  The condition is on the stack;
// if it's falsy, we skip over the `if` body to the `else` branch
// (or the next statement).
func handleJumpIfFalse(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	target := instr.Operand.(int)
	val := v.Pop()
	if isFalsy(val) {
		v.JumpTo(target)
	} else {
		v.AdvancePC()
	}
	return nil
}

// handleJumpIfTrue: conditional jump — pops TOS, jumps if truthy.
func handleJumpIfTrue(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	target := instr.Operand.(int)
	val := v.Pop()
	if !isFalsy(val) {
		v.JumpTo(target)
	} else {
		v.AdvancePC()
	}
	return nil
}

// handleJumpIfFalseOrPop: short-circuit AND operator.
//
// Implements `a and b`:
//   - If a is falsy, the result is a (skip evaluating b).  Jump to target,
//     leaving a on the stack.
//   - If a is truthy, pop a and continue to evaluate b.  The result
//     will be b (whatever it is).
//
// Example: `0 and expensive()` → result is 0 (expensive() never called).
func handleJumpIfFalseOrPop(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	target := instr.Operand.(int)
	val := v.Peek()
	if isFalsy(val) {
		v.JumpTo(target)
	} else {
		v.Pop()
		v.AdvancePC()
	}
	return nil
}

// handleJumpIfTrueOrPop: short-circuit OR operator.
//
// Implements `a or b`:
//   - If a is truthy, the result is a (skip evaluating b).  Jump to target,
//     leaving a on the stack.
//   - If a is falsy, pop a and continue to evaluate b.
//
// Example: `42 or expensive()` → result is 42 (expensive() never called).
func handleJumpIfTrueOrPop(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	target := instr.Operand.(int)
	val := v.Peek()
	if !isFalsy(val) {
		v.JumpTo(target)
	} else {
		v.Pop()
		v.AdvancePC()
	}
	return nil
}

// handleBreak: exits the innermost for loop.
//
// The compiler resolves break to a jump to the loop exit address,
// stored as the operand.
func handleBreak(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	target := instr.Operand.(int)
	v.JumpTo(target)
	return nil
}

// handleContinue: jumps to the next iteration of the innermost for loop.
//
// The compiler resolves continue to a jump to the loop header
// (the GET_ITER or FOR_ITER instruction).
func handleContinue(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	target := instr.Operand.(int)
	v.JumpTo(target)
	return nil
}

// ════════════════════════════════════════════════════════════════════════
// 7. FUNCTION HANDLERS
// ════════════════════════════════════════════════════════════════════════
//
// Functions are the key mechanism for code reuse.  In Starlark, `def`
// compiles the function body into a separate CodeObject (stored in the
// constants pool), and MAKE_FUNCTION creates a StarlarkFunction object
// at runtime.
//
// Calling a function involves:
//   1. Save current state (PC, locals) — "push a frame."
//   2. Set up new locals with argument values.
//   3. Execute the function's code.
//   4. When RETURN is reached, restore old state — "pop a frame."
//
// This is exactly how a CPU handles subroutine calls — save registers,
// jump to the subroutine, restore registers on return.

// handleMakeFunction creates a StarlarkFunction object.
//
// The operand is an index into constants where the CodeObject for the
// function body is stored.  The compiler may also store flags that
// indicate whether the function has default arguments or parameter names.
//
// Stack effects depend on flags:
//   No flags:     Stack = [..., code_object]  →  [..., function]
//   With defaults: Stack = [..., default1, ..., defaultN, code_object]  →  [..., function]
func handleMakeFunction(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	constIdx := instr.Operand.(int)
	constVal := code.Constants[constIdx]

	fn := &StarlarkFunction{
		Name: "<function>",
	}

	// The compiler stores function info in two possible formats:
	//
	// Format 1 (starlark-ast-to-bytecode-compiler): a map[string]interface{} with:
	//   "code"          → vm.CodeObject (the function body)
	//   "params"        → []string (parameter names)
	//   "default_count" → int (number of default values, popped from stack)
	//
	// Format 2 (raw): a bare vm.CodeObject.
	//
	// We handle both formats for flexibility.
	switch info := constVal.(type) {
	case map[string]interface{}:
		// Format 1: compiler-generated function info map.
		fn.Code = info["code"].(vm.CodeObject)

		if params, ok := info["params"].([]string); ok {
			fn.ParamNames = params
			fn.ParamCount = len(params)
		}

		// Pop default values from the stack if present.
		if dc, ok := info["default_count"].(int); ok && dc > 0 {
			defaults := make([]interface{}, dc)
			for i := dc - 1; i >= 0; i-- {
				defaults[i] = v.Pop()
			}
			fn.Defaults = defaults
		}

	case vm.CodeObject:
		// Format 2: bare CodeObject (no metadata).
		fn.Code = info

	default:
		panic(fmt.Sprintf("VMError: MAKE_FUNCTION expected function info or CodeObject, got %T", constVal))
	}

	// Try to extract function name from the code object's names or the
	// next instruction (STORE_NAME typically follows MAKE_FUNCTION).
	if fn.Name == "<function>" && len(fn.Code.Names) > 0 {
		fn.Name = fn.Code.Names[0]
	}

	v.Push(fn)
	v.AdvancePC()
	return nil
}

// handleCallFunction calls a function with positional arguments.
//
// Operand: number of positional arguments.
//
// Stack before: [..., callable, arg1, arg2, ..., argN]
// Stack after:  [..., return_value]
//
// The callable can be:
//   - *StarlarkFunction: execute its CodeObject in a new frame.
//   - *BuiltinFunction: call its Implementation directly.
//
// For StarlarkFunction calls, the handler:
//   1. Saves the current state in a call frame (PushFrame).
//   2. Sets up locals with argument values.
//   3. Runs the function's code via vm.Execute().
//   4. Restores state from the frame (PopFrame).
//   5. Pushes the return value (or None if no explicit return).
func handleCallFunction(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	argCount := instr.Operand.(int)

	// Collect arguments from the stack (they were pushed left to right).
	args := make([]interface{}, argCount)
	for i := argCount - 1; i >= 0; i-- {
		args[i] = v.Pop()
	}

	// Pop the callable.
	callable := v.Pop()

	callFunction(v, callable, args, code)
	return nil
}

// callFunction is the shared logic for calling a function, used by both
// handleCallFunction and handleCallFunctionKW.
func callFunction(v *vm.GenericVM, callable interface{}, args []interface{}, code vm.CodeObject) {
	switch fn := callable.(type) {
	case *StarlarkFunction:
		// ── Save state ────────────────────────────────────────────
		// We need to remember where we were so we can come back
		// after the function finishes.
		frame := map[string]interface{}{
			"returnPC":    v.PC + 1,
			"savedLocals": copySlice(v.Locals),
			"savedHalted": v.Halted,
		}
		v.PushFrame(frame)

		// ── Build parameter-to-value mapping ──────────────────────
		// First, build a map from parameter name → argument value.
		// This handles both positional args and defaults correctly.
		paramValues := make(map[string]interface{})

		// Fill positional arguments by param name.
		for i, arg := range args {
			if i < len(fn.ParamNames) {
				paramValues[fn.ParamNames[i]] = arg
			}
		}

		// Fill defaults for missing arguments.
		if fn.Defaults != nil {
			numRequired := fn.ParamCount - len(fn.Defaults)
			for i := len(args); i < fn.ParamCount; i++ {
				defaultIdx := i - numRequired
				if defaultIdx >= 0 && defaultIdx < len(fn.Defaults) {
					paramValues[fn.ParamNames[i]] = fn.Defaults[defaultIdx]
				}
			}
		}

		// ── Set up locals ─────────────────────────────────────────
		// The function body compiler assigns local slots based on the
		// order names appear in the body's Names list.  We must map
		// parameter values to the correct slots by matching param names
		// to the body's Names indices.
		localSize := len(fn.Code.Names)
		if localSize < fn.ParamCount {
			localSize = fn.ParamCount
		}
		if localSize < 4 {
			localSize = 4
		}
		v.Locals = make([]interface{}, localSize)

		// Map parameter values to the body's name slots.
		for paramName, paramVal := range paramValues {
			for nameIdx, name := range fn.Code.Names {
				if name == paramName {
					if nameIdx >= len(v.Locals) {
						// Extend locals if needed.
						for len(v.Locals) <= nameIdx {
							v.Locals = append(v.Locals, nil)
						}
					}
					v.Locals[nameIdx] = paramVal
					break
				}
			}
		}

		// ── Execute function body ─────────────────────────────────
		v.PC = 0
		v.Halted = false
		v.Execute(fn.Code)

		// ── Get return value ──────────────────────────────────────
		// If the function returned a value, it's on the stack.
		// If not (e.g., fell off the end), push None.
		var returnVal interface{}
		if len(v.Stack) > 0 {
			returnVal = v.Pop()
		}

		// ── Restore caller state ──────────────────────────────────
		frame = v.PopFrame()
		v.Locals = frame["savedLocals"].([]interface{})
		v.PC = frame["returnPC"].(int)
		v.Halted = false

		// Push return value for the caller.
		v.Push(returnVal)

	case *vm.BuiltinFunction:
		// Built-in functions are simple — just call and push result.
		result := fn.Implementation(args...)
		v.Push(result)
		v.AdvancePC()

	default:
		panic(fmt.Sprintf("TypeError: '%T' object is not callable", callable))
	}
}

// handleCallFunctionKW calls a function with keyword arguments.
//
// Operand: number of keyword argument pairs.
//
// The compiler emits keyword arguments as interleaved key-value pairs:
//   [..., callable, key1, val1, key2, val2, ..., keyN, valN]
//
// where operand = N (the number of keyword pairs).  Each key is a string
// (the parameter name) and each val is the argument value.
//
// This matches the compiler's output format in compileArgument(), which
// pushes LOAD_CONST(name) then the value expression for each keyword arg.
func handleCallFunctionKW(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	kwCount := instr.Operand.(int)

	// Pop key-value pairs from the stack (in reverse order).
	kwNames := make([]string, kwCount)
	kwValues := make([]interface{}, kwCount)
	for i := kwCount - 1; i >= 0; i-- {
		kwValues[i] = v.Pop()
		kwNames[i] = fmt.Sprintf("%v", v.Pop())
	}

	// Pop callable.
	callable := v.Pop()

	// If it's a StarlarkFunction, map keyword args to the right parameter slots.
	if fn, ok := callable.(*StarlarkFunction); ok {
		finalArgs := make([]interface{}, fn.ParamCount)

		// Fill defaults first (from the function's default values).
		// Defaults are right-aligned: if there are 4 params and 3 defaults,
		// the defaults apply to params 1, 2, 3 (not 0, 1, 2).
		// This mirrors Python's convention: def f(a, b=1, c=2).
		defaultOffset := fn.ParamCount - len(fn.Defaults)
		for i, def := range fn.Defaults {
			finalArgs[defaultOffset+i] = def
		}

		// Map keyword args to parameter slots by name.
		for i, kwName := range kwNames {
			for j, pname := range fn.ParamNames {
				if pname == kwName {
					finalArgs[j] = kwValues[i]
					break
				}
			}
		}

		callFunction(v, callable, finalArgs, code)
	} else {
		// For builtins, convert keyword args to positional.
		callFunction(v, callable, kwValues, code)
	}
	return nil
}

// handleReturnValue returns a value from the current function.
//
// The return value is on top of the stack.  This handler sets
// vm.Halted = true to stop the inner Execute() loop.  The caller
// (handleCallFunction) then picks up the return value and restores state.
//
//   Before: Stack = [..., return_value]
//   After:  vm.Halted = true (inner loop stops)
func handleReturnValue(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	// The return value is already on the stack — just halt
	// the current execution context so the caller can pick it up.
	v.Halted = true
	return nil
}

// ════════════════════════════════════════════════════════════════════════
// 8. COLLECTION HANDLERS
// ════════════════════════════════════════════════════════════════════════
//
// Collections are fundamental data structures in Starlark:
//   - Lists: ordered, mutable sequences → [1, 2, 3]
//   - Dicts: key-value mappings        → {"a": 1, "b": 2}
//   - Tuples: ordered, immutable sequences → (1, 2, 3)
//
// In our Go implementation, both lists and tuples are []interface{}.
// Dicts are map[string]interface{}.

// handleBuildList creates a list from N items on the stack.
//
// Operand: number of items.
//   Before: Stack = [..., item1, item2, ..., itemN]
//   After:  Stack = [..., [item1, item2, ..., itemN]]
func handleBuildList(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	count := instr.Operand.(int)
	items := make([]interface{}, count)
	for i := count - 1; i >= 0; i-- {
		items[i] = v.Pop()
	}
	v.Push(items)
	v.AdvancePC()
	return nil
}

// handleBuildDict creates a dict from N key-value pairs on the stack.
//
// Operand: number of pairs.
//   Before: Stack = [..., key1, val1, key2, val2, ..., keyN, valN]
//   After:  Stack = [..., {key1: val1, key2: val2, ...}]
func handleBuildDict(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	count := instr.Operand.(int)
	dict := make(map[string]interface{})
	// Pop pairs in reverse order (last pair was pushed last).
	pairs := make([]interface{}, count*2)
	for i := count*2 - 1; i >= 0; i-- {
		pairs[i] = v.Pop()
	}
	for i := 0; i < count*2; i += 2 {
		key := fmt.Sprintf("%v", pairs[i])
		dict[key] = pairs[i+1]
	}
	v.Push(dict)
	v.AdvancePC()
	return nil
}

// handleBuildTuple creates a tuple from N items on the stack.
// In Go, tuples and lists are both []interface{}.
func handleBuildTuple(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	count := instr.Operand.(int)
	items := make([]interface{}, count)
	for i := count - 1; i >= 0; i-- {
		items[i] = v.Pop()
	}
	v.Push(items)
	v.AdvancePC()
	return nil
}

// handleListAppend appends TOS to a list at a given stack position.
// Used in list comprehensions.
//
// Operand: stack offset (distance from TOS to the list being built).
func handleListAppend(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	val := v.Pop()
	offset := instr.Operand.(int)
	// The list is `offset` positions from the current top.
	idx := len(v.Stack) - offset
	if idx >= 0 && idx < len(v.Stack) {
		if lst, ok := v.Stack[idx].([]interface{}); ok {
			v.Stack[idx] = append(lst, val)
		}
	}
	v.AdvancePC()
	return nil
}

// handleDictSet sets a key-value pair in a dict at a given stack position.
// Used in dict comprehensions.
func handleDictSet(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	val := v.Pop()
	key := v.Pop()
	offset := instr.Operand.(int)
	idx := len(v.Stack) - offset
	if idx >= 0 && idx < len(v.Stack) {
		if dict, ok := v.Stack[idx].(map[string]interface{}); ok {
			dict[fmt.Sprintf("%v", key)] = val
		}
	}
	v.AdvancePC()
	return nil
}

// ════════════════════════════════════════════════════════════════════════
// 9. SUBSCRIPT AND ATTRIBUTE HANDLERS
// ════════════════════════════════════════════════════════════════════════
//
// Subscript: obj[key]  — access an element by index or key.
// Attribute: obj.name  — access a named property or method.

// handleLoadSubscript loads an element by index or key.
//
//   [1,2,3][1]     → 2
//   {"a": 1}["a"]  → 1
func handleLoadSubscript(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	key := v.Pop()
	obj := v.Pop()

	switch container := obj.(type) {
	case []interface{}:
		idx := toInt(key)
		// Support negative indexing: lst[-1] = last element.
		if idx < 0 {
			idx = len(container) + idx
		}
		if idx < 0 || idx >= len(container) {
			panic(fmt.Sprintf("IndexError: list index %d out of range", idx))
		}
		v.Push(container[idx])

	case map[string]interface{}:
		keyStr := fmt.Sprintf("%v", key)
		val, ok := container[keyStr]
		if !ok {
			panic(fmt.Sprintf("KeyError: '%s'", keyStr))
		}
		v.Push(val)

	case string:
		idx := toInt(key)
		if idx < 0 {
			idx = len(container) + idx
		}
		if idx < 0 || idx >= len(container) {
			panic(fmt.Sprintf("IndexError: string index %d out of range", idx))
		}
		v.Push(string(container[idx]))

	default:
		panic(fmt.Sprintf("TypeError: '%T' object is not subscriptable", obj))
	}

	v.AdvancePC()
	return nil
}

// handleStoreSubscript stores a value at a subscript position.
//
//   lst[0] = 42       — sets list element.
//   d["key"] = "val"  — sets dict entry.
//
// Stack: [..., value, container, index]
func handleStoreSubscript(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	key := v.Pop()
	obj := v.Pop()
	val := v.Pop()

	switch container := obj.(type) {
	case []interface{}:
		idx := toInt(key)
		if idx < 0 {
			idx = len(container) + idx
		}
		if idx < 0 || idx >= len(container) {
			panic(fmt.Sprintf("IndexError: list assignment index %d out of range", idx))
		}
		container[idx] = val

	case map[string]interface{}:
		keyStr := fmt.Sprintf("%v", key)
		container[keyStr] = val

	default:
		panic(fmt.Sprintf("TypeError: '%T' object does not support item assignment", obj))
	}

	v.AdvancePC()
	return nil
}

// handleLoadAttr loads an attribute from an object.
//
// For dicts, attribute access is equivalent to key access:
//   d.key  →  d["key"]
//
// For method calls on built-in types (like list.append), the handler
// returns a bound method that can be called with CALL_FUNCTION.
//
// Operand: index into code.Names for the attribute name.
func handleLoadAttr(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	idx := instr.Operand.(int)
	attrName := code.Names[idx]
	obj := v.Pop()

	// Dict attribute access: d.key → d["key"].
	if dict, ok := obj.(map[string]interface{}); ok {
		if val, found := dict[attrName]; found {
			v.Push(val)
			v.AdvancePC()
			return nil
		}
	}

	// List methods.
	if lst, ok := obj.([]interface{}); ok {
		switch attrName {
		case "append":
			// Return a "bound method" — a builtin that appends to this list.
			method := &vm.BuiltinFunction{
				Name: "list.append",
				Implementation: func(args ...interface{}) interface{} {
					// We need to mutate the original list.
					// Since Go slices are reference types for the backing array,
					// we need to store the result back somehow.
					// For now, append to the original slice.
					_ = lst
					_ = args
					return nil
				},
			}
			v.Push(method)
			v.AdvancePC()
			return nil
		}
	}

	// String methods.
	if s, ok := obj.(string); ok {
		switch attrName {
		case "upper":
			method := &vm.BuiltinFunction{
				Name: "str.upper",
				Implementation: func(args ...interface{}) interface{} {
					return strings.ToUpper(s)
				},
			}
			v.Push(method)
			v.AdvancePC()
			return nil
		case "lower":
			method := &vm.BuiltinFunction{
				Name: "str.lower",
				Implementation: func(args ...interface{}) interface{} {
					return strings.ToLower(s)
				},
			}
			v.Push(method)
			v.AdvancePC()
			return nil
		case "strip":
			method := &vm.BuiltinFunction{
				Name: "str.strip",
				Implementation: func(args ...interface{}) interface{} {
					return strings.TrimSpace(s)
				},
			}
			v.Push(method)
			v.AdvancePC()
			return nil
		case "startswith":
			method := &vm.BuiltinFunction{
				Name: "str.startswith",
				Implementation: func(args ...interface{}) interface{} {
					if len(args) > 0 {
						return strings.HasPrefix(s, fmt.Sprintf("%v", args[0]))
					}
					return false
				},
			}
			v.Push(method)
			v.AdvancePC()
			return nil
		case "endswith":
			method := &vm.BuiltinFunction{
				Name: "str.endswith",
				Implementation: func(args ...interface{}) interface{} {
					if len(args) > 0 {
						return strings.HasSuffix(s, fmt.Sprintf("%v", args[0]))
					}
					return false
				},
			}
			v.Push(method)
			v.AdvancePC()
			return nil
		case "split":
			method := &vm.BuiltinFunction{
				Name: "str.split",
				Implementation: func(args ...interface{}) interface{} {
					var parts []string
					if len(args) > 0 {
						parts = strings.Split(s, fmt.Sprintf("%v", args[0]))
					} else {
						parts = strings.Fields(s)
					}
					result := make([]interface{}, len(parts))
					for i, p := range parts {
						result[i] = p
					}
					return result
				},
			}
			v.Push(method)
			v.AdvancePC()
			return nil
		case "join":
			method := &vm.BuiltinFunction{
				Name: "str.join",
				Implementation: func(args ...interface{}) interface{} {
					if len(args) > 0 {
						if items, ok := args[0].([]interface{}); ok {
							strs := make([]string, len(items))
							for i, item := range items {
								strs[i] = formatValue(item)
							}
							return strings.Join(strs, s)
						}
					}
					return s
				},
			}
			v.Push(method)
			v.AdvancePC()
			return nil
		case "replace":
			method := &vm.BuiltinFunction{
				Name: "str.replace",
				Implementation: func(args ...interface{}) interface{} {
					if len(args) >= 2 {
						return strings.ReplaceAll(s, fmt.Sprintf("%v", args[0]), fmt.Sprintf("%v", args[1]))
					}
					return s
				},
			}
			v.Push(method)
			v.AdvancePC()
			return nil
		case "find":
			method := &vm.BuiltinFunction{
				Name: "str.find",
				Implementation: func(args ...interface{}) interface{} {
					if len(args) > 0 {
						return strings.Index(s, fmt.Sprintf("%v", args[0]))
					}
					return -1
				},
			}
			v.Push(method)
			v.AdvancePC()
			return nil
		}
	}

	panic(fmt.Sprintf("AttributeError: object of type '%T' has no attribute '%s'", obj, attrName))
}

// handleStoreAttr stores a value into an object's attribute.
//
//   obj.name = value
//
// Stack: [..., value, obj]
func handleStoreAttr(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	idx := instr.Operand.(int)
	attrName := code.Names[idx]
	obj := v.Pop()
	val := v.Pop()

	if dict, ok := obj.(map[string]interface{}); ok {
		dict[attrName] = val
	} else {
		panic(fmt.Sprintf("AttributeError: cannot set attribute '%s' on %T", attrName, obj))
	}

	v.AdvancePC()
	return nil
}

// handleLoadSlice slices a sequence.  Currently a stub that pushes nil.
func handleLoadSlice(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	// Stub: pop the arguments and push nil.
	count := 2
	if instr.Operand != nil {
		count = instr.Operand.(int)
	}
	for i := 0; i < count; i++ {
		v.Pop()
	}
	v.Pop() // the sequence
	v.Push(nil)
	v.AdvancePC()
	return nil
}

// ════════════════════════════════════════════════════════════════════════
// 10. ITERATION HANDLERS
// ════════════════════════════════════════════════════════════════════════
//
// Starlark's `for` loop compiles to a pattern:
//
//   GET_ITER      — Convert iterable to a StarlarkIterator.
//   FOR_ITER end  — Get next item; if exhausted, jump to `end`.
//   ... loop body ...
//   JUMP back to FOR_ITER
//   end:          — First instruction after the loop.
//
// This is the same pattern Python uses.  The iterator is kept on the
// stack throughout the loop.

// handleGetIter converts an iterable to a StarlarkIterator.
//
// Supported iterables:
//   - []interface{} (list/tuple): iterate over elements.
//   - map[string]interface{} (dict): iterate over keys.
//   - string: iterate over characters.
//   - StarlarkIterator: pass through (already an iterator).
func handleGetIter(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	iterable := v.Pop()

	switch it := iterable.(type) {
	case []interface{}:
		v.Push(&StarlarkIterator{Items: it, Index: 0})
	case map[string]interface{}:
		keys := make([]interface{}, 0, len(it))
		for k := range it {
			keys = append(keys, k)
		}
		sort.Slice(keys, func(i, j int) bool {
			return fmt.Sprintf("%v", keys[i]) < fmt.Sprintf("%v", keys[j])
		})
		v.Push(&StarlarkIterator{Items: keys, Index: 0})
	case string:
		chars := make([]interface{}, len(it))
		for i, ch := range it {
			chars[i] = string(ch)
		}
		v.Push(&StarlarkIterator{Items: chars, Index: 0})
	case *StarlarkIterator:
		v.Push(it)
	default:
		panic(fmt.Sprintf("TypeError: '%T' object is not iterable", iterable))
	}

	v.AdvancePC()
	return nil
}

// handleForIter advances the iterator and pushes the next value.
//
// If the iterator has more items:
//   - Push the next value on top of the iterator (iterator stays on stack).
//   - Advance PC to the loop body.
//
// If the iterator is exhausted:
//   - Pop the iterator from the stack.
//   - Jump to the loop exit address (operand).
func handleForIter(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	target := instr.Operand.(int)
	iter := v.Peek().(*StarlarkIterator)

	val, ok := iter.Next()
	if !ok {
		// Iterator exhausted — pop it and jump to loop exit.
		v.Pop()
		v.JumpTo(target)
	} else {
		// Push the next value and continue into the loop body.
		v.Push(val)
		v.AdvancePC()
	}
	return nil
}

// handleUnpackSequence unpacks a sequence into N values on the stack.
//
// Operand: number of values to unpack.
//
// The values are pushed in REVERSE order so that subsequent STORE
// instructions pick them up in the original order.
//
//   Example: `a, b, c = [1, 2, 3]`
//   Before: Stack = [..., [1, 2, 3]]
//   After:  Stack = [..., 3, 2, 1]    (1 is on top for first STORE)
func handleUnpackSequence(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	count := instr.Operand.(int)
	seq := v.Pop()

	items, ok := seq.([]interface{})
	if !ok {
		panic(fmt.Sprintf("TypeError: cannot unpack non-sequence %T", seq))
	}
	if len(items) != count {
		panic(fmt.Sprintf("ValueError: not enough values to unpack (expected %d, got %d)", count, len(items)))
	}

	// Push in reverse so they come off in order.
	for i := count - 1; i >= 0; i-- {
		v.Push(items[i])
	}
	v.AdvancePC()
	return nil
}

// ════════════════════════════════════════════════════════════════════════
// 11. MODULE HANDLERS (STUBS)
// ════════════════════════════════════════════════════════════════════════
//
// Module loading is a complex feature that involves file system access,
// compilation, and caching.  For now, these are stubs that a full
// interpreter would override.

// handleLoadModule loads a module by name.  Currently a stub that
// pushes an empty dict (representing an empty module namespace).
func handleLoadModule(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	v.Push(make(map[string]interface{}))
	v.AdvancePC()
	return nil
}

// handleImportFrom imports a symbol from the module on TOS.
//
// Stack: [..., module_dict] → [..., module_dict, symbol_value]
func handleImportFrom(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	idx := instr.Operand.(int)
	name := code.Names[idx]
	module := v.Peek()

	if dict, ok := module.(map[string]interface{}); ok {
		if val, found := dict[name]; found {
			v.Push(val)
		} else {
			v.Push(nil)
		}
	} else {
		v.Push(nil)
	}

	v.AdvancePC()
	return nil
}

// ════════════════════════════════════════════════════════════════════════
// 12. OUTPUT HANDLER
// ════════════════════════════════════════════════════════════════════════

// handlePrintValue prints the top-of-stack value.
//
// The value is popped, formatted as a string, and returned as output.
// The GenericVM's Step() method will append this to vm.Output.
//
// Note: we return the string pointer so the VM records it in the trace.
// The VM's Step() method handles appending to vm.Output.
func handlePrintValue(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	val := v.Pop()
	str := formatValue(val)
	v.AdvancePC()
	return &str
}

// ════════════════════════════════════════════════════════════════════════
// 13. HALT HANDLER
// ════════════════════════════════════════════════════════════════════════

// handleHalt stops VM execution.
//
// This is typically the last instruction in a program.  It sets
// vm.Halted = true, which causes the Execute() loop to stop.
func handleHalt(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
	v.Halted = true
	return nil
}
