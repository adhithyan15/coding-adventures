// vm.go — Factory and executor for the Starlark virtual machine.
//
// ════════════════════════════════════════════════════════════════════════
// OVERVIEW
// ════════════════════════════════════════════════════════════════════════
//
// This file provides the public API for the starlark-vm package.
// It ties together all the components:
//
//   1. Creates a GenericVM (from the virtual-machine package).
//   2. Registers all 59 Starlark opcode handlers (from handlers.go).
//   3. Registers all 23 built-in functions (from builtins.go).
//   4. Provides convenience functions for compiling and executing Starlark.
//
// ════════════════════════════════════════════════════════════════════════
// ARCHITECTURE
// ════════════════════════════════════════════════════════════════════════
//
//   ┌─────────────────────────────────────────────────────┐
//   │                  User Code                          │
//   │                                                     │
//   │   result, err := starlarkvm.ExecuteStarlark(src)    │
//   └────────────────────────┬────────────────────────────┘
//                            │
//                            ▼
//   ┌─────────────────────────────────────────────────────┐
//   │              starlark-ast-to-bytecode-compiler       │
//   │                                                     │
//   │   Lexer → Parser → AST → Compiler → CodeObject     │
//   └────────────────────────┬────────────────────────────┘
//                            │
//                            ▼
//   ┌─────────────────────────────────────────────────────┐
//   │                   GenericVM                         │
//   │                                                     │
//   │   59 opcode handlers    23 builtins                 │
//   │   Fetch-Decode-Execute loop                         │
//   │   Stack, Variables, Locals, CallStack               │
//   └────────────────────────┬────────────────────────────┘
//                            │
//                            ▼
//   ┌─────────────────────────────────────────────────────┐
//   │              StarlarkResult                         │
//   │                                                     │
//   │   Variables: final state of all named variables     │
//   │   Output: all print() output in order               │
//   │   Traces: detailed execution log                    │
//   └─────────────────────────────────────────────────────┘
//
// ════════════════════════════════════════════════════════════════════════
// USAGE EXAMPLES
// ════════════════════════════════════════════════════════════════════════
//
// Simple execution:
//
//   result, err := starlarkvm.ExecuteStarlark("x = 1 + 2\n")
//   if err != nil {
//       log.Fatal(err)
//   }
//   fmt.Println(result.Variables["x"])  // 3
//
// Custom VM with deeper recursion:
//
//   v := starlarkvm.CreateStarlarkVM(500)
//   code, _ := starlarkcompiler.CompileStarlark(source)
//   traces := v.Execute(code)
//
package starlarkvm

import (
	starlarkcompiler "github.com/adhithyan15/coding-adventures/code/packages/go/starlark-ast-to-bytecode-compiler"
	vm "github.com/adhithyan15/coding-adventures/code/packages/go/virtual-machine"
)

// CreateStarlarkVM creates a GenericVM fully configured with all
// Starlark opcode handlers and built-in functions.
//
// Parameters:
//   maxRecursionDepth (optional) — Maximum call stack depth.
//   If not provided, defaults to 200.  This prevents infinite recursion
//   from crashing the process.
//
// Returns a ready-to-use *GenericVM.  You can execute code immediately:
//
//   v := CreateStarlarkVM()
//   traces := v.Execute(code)
//
// Or configure further before executing:
//
//   v := CreateStarlarkVM(1000)  // allow deeper recursion
//   v.Variables["my_global"] = 42
//   traces := v.Execute(code)
func CreateStarlarkVM(maxRecursionDepth ...int) *vm.GenericVM {
	v := vm.NewGenericVM()

	// Set recursion depth.  Default is 200, which is generous for most
	// Starlark programs (the typical stack depth is under 20).
	depth := 200
	if len(maxRecursionDepth) > 0 {
		depth = maxRecursionDepth[0]
	}
	v.SetMaxRecursionDepth(&depth)

	// Register all 59 opcode handlers.
	registerAllHandlers(v)

	// Register all 23 built-in functions.
	RegisterAllBuiltins(v)

	return v
}

// ExecuteStarlark compiles and executes Starlark source code in one step.
//
// This is the most convenient way to run Starlark code.  It:
//   1. Compiles the source to bytecode (via starlark-ast-to-bytecode-compiler).
//   2. Creates a fresh VM.
//   3. Executes the bytecode.
//   4. Returns the result (variables, output, traces).
//
// If compilation fails, the error is returned immediately and no
// execution occurs.
//
// Example:
//
//   result, err := ExecuteStarlark(`
//   def greet(name):
//       return "Hello, " + name
//   message = greet("World")
//   print(message)
//   `)
//   // result.Variables["message"] == "Hello, World"
//   // result.Output == ["Hello, World"]
func ExecuteStarlark(source string) (*StarlarkResult, error) {
	// Step 1: Compile source to bytecode.
	code, err := starlarkcompiler.CompileStarlark(source)
	if err != nil {
		return nil, err
	}

	// Step 2: Create a fresh VM with all handlers and builtins.
	v := CreateStarlarkVM()

	// Step 3: Execute the bytecode.
	traces := v.Execute(code)

	// Step 4: Package the results.
	return &StarlarkResult{
		Variables: v.Variables,
		Output:    v.Output,
		Traces:    traces,
	}, nil
}
