// types.go — Starlark runtime types used by the VM during execution.
//
// ════════════════════════════════════════════════════════════════════════
// OVERVIEW
// ════════════════════════════════════════════════════════════════════════
//
// When the Starlark compiler produces bytecode, certain instructions
// create runtime objects that don't exist in the bytecode itself.  For
// example, MAKE_FUNCTION creates a function object that bundles together
// a CodeObject (the function's body) with metadata like the function
// name, parameter count, and default argument values.
//
// This file defines those runtime types.  Think of these as the "shapes"
// of data that live on the VM's stack and in its variables during
// execution.
//
// ════════════════════════════════════════════════════════════════════════
// TYPE CATALOG
// ════════════════════════════════════════════════════════════════════════
//
//   StarlarkFunction  — A compiled function (def or lambda)
//   StarlarkIterator  — Iteration state for FOR_ITER loops
//   StarlarkResult    — The outcome of running a Starlark program
//
package starlarkvm

import (
	vm "github.com/adhithyan15/coding-adventures/code/packages/go/virtual-machine"
)

// ════════════════════════════════════════════════════════════════════════
// StarlarkFunction — COMPILED FUNCTION OBJECT
// ════════════════════════════════════════════════════════════════════════
//
// When you write `def greet(name):` in Starlark, the compiler produces
// a MAKE_FUNCTION instruction.  The VM's handler for that instruction
// creates a StarlarkFunction and pushes it onto the stack.
//
// A StarlarkFunction is NOT executable by itself — it's just data.
// When CALL_FUNCTION encounters a StarlarkFunction on the stack, it:
//   1. Saves the current VM state (PC, locals) onto the call stack.
//   2. Sets up new locals with the function's arguments.
//   3. Runs the function's Code (a CodeObject).
//   4. Restores the caller's state when the function returns.
//
// This is analogous to how a CPU saves registers before a subroutine
// call and restores them when the subroutine returns.
//
// Fields:
//   Code       — The function body as a CodeObject (instructions + constants + names).
//   Defaults   — Default values for optional parameters.  If a function has
//                `def f(a, b=10, c=20)`, Defaults would be [10, 20].
//   Name       — The function's name.  For lambdas, this is "<lambda>".
//   ParamCount — The number of parameters the function expects.
//   ParamNames — The names of each parameter, in order.  Used for
//                keyword argument resolution: `f(c=30, a=1)` maps
//                "c" → slot 2, "a" → slot 0.

type StarlarkFunction struct {
	Code       vm.CodeObject
	Defaults   []interface{}
	Name       string
	ParamCount int
	ParamNames []string
}

// ════════════════════════════════════════════════════════════════════════
// StarlarkIterator — ITERATION STATE
// ════════════════════════════════════════════════════════════════════════
//
// Starlark's `for x in collection:` loop compiles to three opcodes:
//
//   GET_ITER      — Convert the collection to an iterator.
//   FOR_ITER      — Get the next item (or jump to loop exit if done).
//   ... loop body ...
//   JUMP back to FOR_ITER
//
// The iterator needs to remember which item comes next.  That's what
// StarlarkIterator does — it wraps a slice of items and an index that
// advances each time Next() is called.
//
// Analogy: imagine reading a book.  The book (Items) doesn't change,
// but your bookmark (Index) moves forward each time you read a page.
// When the bookmark reaches the end, you're done.
//
// Example iteration over [10, 20, 30]:
//
//   Call   | Index before | Returns    | Index after
//   ───────┼──────────────┼────────────┼────────────
//   Next() | 0            | (10, true) | 1
//   Next() | 1            | (20, true) | 2
//   Next() | 2            | (30, true) | 3
//   Next() | 3            | (nil, false)| 3

type StarlarkIterator struct {
	Items []interface{}
	Index int
}

// Next returns the next item from the iterator and a boolean indicating
// whether an item was available.  When the iterator is exhausted,
// it returns (nil, false).
//
// This follows Go's "comma ok" idiom:
//
//   val, ok := iter.Next()
//   if !ok {
//       // iterator exhausted
//   }
func (it *StarlarkIterator) Next() (interface{}, bool) {
	if it.Index >= len(it.Items) {
		return nil, false
	}
	val := it.Items[it.Index]
	it.Index++
	return val, true
}

// ════════════════════════════════════════════════════════════════════════
// StarlarkResult — EXECUTION OUTCOME
// ════════════════════════════════════════════════════════════════════════
//
// After a Starlark program finishes running, StarlarkResult captures
// everything that happened:
//
//   Variables — All named variables and their final values.
//               This is the program's "output state" — you can inspect
//               what values were computed.
//
//   Output    — All strings produced by print() calls, in order.
//               This is the program's "console output".
//
//   Traces    — A detailed log of every instruction executed, including
//               stack snapshots before and after each step.  Useful for
//               debugging and understanding program behavior.
//
// Example usage:
//
//   result, err := ExecuteStarlark("x = 1 + 2\nprint(x)")
//   // result.Variables["x"] == 3
//   // result.Output == ["3"]

type StarlarkResult struct {
	Variables map[string]interface{}
	Output    []string
	Traces    []vm.VMTrace
}
