// interpreter.go -- Full Starlark pipeline: source -> lexer -> parser -> compiler -> VM -> result.
//
// ============================================================================
// OVERVIEW
// ============================================================================
//
// This package chains together every layer of the Starlark compilation and
// execution pipeline into a single, easy-to-use API.  It adds one critical
// feature that the lower layers don't provide: load() support.
//
// The pipeline looks like this:
//
//   Source Code (string)
//       |
//       v
//   starlark-lexer: tokenize
//       |
//       v
//   starlark-parser: parse into AST
//       |
//       v
//   starlark-ast-to-bytecode-compiler: compile to bytecode
//       |
//       v
//   starlark-vm: execute bytecode
//       |
//       v
//   StarlarkResult { Variables, Output, Traces }
//
// The starlark-vm package already provides ExecuteStarlark() which does steps
// 1-4 in one call.  So why do we need an interpreter package?
//
// The answer is load().  Starlark's load() statement imports symbols from
// another file:
//
//   load("helpers.star", "double", "triple")
//   result = double(21)  # 42
//
// When the VM encounters a LOAD_MODULE opcode, it needs to:
//   1. Resolve the file label to actual source code.
//   2. Compile and execute that source code (recursively).
//   3. Extract the exported symbols.
//   4. Make them available to the importing program.
//
// The starlark-vm's default LOAD_MODULE handler just pushes an empty dict
// (a no-op stub).  This interpreter overrides that handler to actually
// resolve, compile, and execute loaded files.
//
// ============================================================================
// ARCHITECTURE
// ============================================================================
//
//   +---------------------------------------------------------------+
//   |                    StarlarkInterpreter                        |
//   |                                                               |
//   |  FileResolver: how to find source code for load() labels      |
//   |  loadCache:    avoid re-executing the same file twice         |
//   |  MaxRecursionDepth: safety limit on call stack                |
//   |                                                               |
//   |  Interpret(source) ----+                                      |
//   |                        |                                      |
//   |                        v                                      |
//   |  +---------------------------------------------------+       |
//   |  | starlark-ast-to-bytecode-compiler.CompileStarlark  |       |
//   |  +---------------------------------------------------+       |
//   |                        |                                      |
//   |                        v                                      |
//   |  +---------------------------------------------------+       |
//   |  | starlark-vm.CreateStarlarkVM (GenericVM)           |       |
//   |  |   - 59 opcode handlers pre-registered              |       |
//   |  |   - 23 builtins pre-registered                     |       |
//   |  |   - LOAD_MODULE handler OVERRIDDEN by interpreter  |       |
//   |  +---------------------------------------------------+       |
//   |                        |                                      |
//   |                        v                                      |
//   |  +---------------------------------------------------+       |
//   |  | GenericVM.Execute(code)                            |       |
//   |  |   If LOAD_MODULE is encountered:                   |       |
//   |  |     -> FileResolver finds the source               |       |
//   |  |     -> Recursive Interpret() call                  |       |
//   |  |     -> Result cached in loadCache                  |       |
//   |  |     -> Module variables pushed onto stack           |       |
//   |  +---------------------------------------------------+       |
//   |                        |                                      |
//   |                        v                                      |
//   |              StarlarkResult                                   |
//   +---------------------------------------------------------------+
//
// ============================================================================
// LOAD() CACHING
// ============================================================================
//
// If the same file is loaded by multiple modules, we don't want to execute
// it multiple times.  The loadCache maps file labels to their resulting
// variable maps.  On first load, the file is compiled and executed; on
// subsequent loads, the cached variables are returned immediately.
//
// This is exactly how Python's import system works: each module is executed
// once and cached in sys.modules.
//
//   First load("math.star", "PI"):
//     1. Check cache -> miss
//     2. Resolve "math.star" -> source code
//     3. Compile and execute -> {PI: 3.14159, ...}
//     4. Store in cache: loadCache["math.star"] = {PI: 3.14159, ...}
//     5. Push the dict onto the stack
//
//   Second load("math.star", "E"):
//     1. Check cache -> hit! -> {PI: 3.14159, ...}
//     2. Push the cached dict onto the stack (no re-execution)
//
// ============================================================================
// USAGE EXAMPLES
// ============================================================================
//
// Simple execution (no load() needed):
//
//   result, err := starlarkinterpreter.Interpret("x = 1 + 2\n")
//   fmt.Println(result.Variables["x"])  // 3
//
// Execution with load() support:
//
//   files := map[string]string{
//       "helpers.star": "def double(n):\n    return n * 2\n",
//   }
//   resolver := starlarkinterpreter.DictResolver(files)
//   result, err := starlarkinterpreter.Interpret(
//       `load("helpers.star", "double")` + "\n" +
//       "x = double(21)\n",
//       resolver,
//   )
//   fmt.Println(result.Variables["x"])  // 42
//
// Using the struct API for more control:
//
//   interp := starlarkinterpreter.NewInterpreter(
//       starlarkinterpreter.WithFileResolver(resolver),
//       starlarkinterpreter.WithMaxRecursionDepth(500),
//   )
//   result, err := interp.Interpret(source)
//
package starlarkinterpreter

import (
	"fmt"
	"os"
	"strings"

	starlarkcompiler "github.com/adhithyan15/coding-adventures/code/packages/go/starlark-ast-to-bytecode-compiler"
	starlarkvm "github.com/adhithyan15/coding-adventures/code/packages/go/starlark-vm"
	vm "github.com/adhithyan15/coding-adventures/code/packages/go/virtual-machine"
)

// ============================================================================
// FILE RESOLVER
// ============================================================================
//
// A FileResolver maps a load() label (like "helpers.star" or "//pkg:defs.bzl")
// to the actual source code of that file.  This is the interpreter's "file
// system" -- it decides where code lives.
//
// Why a function type instead of always reading from disk?
//
//   1. Testability: in tests, you can use DictResolver with a map of
//      filename -> source code.  No temp files needed.
//
//   2. Flexibility: a BUILD system might store files in memory, fetch
//      them from a network, or generate them on the fly.
//
//   3. Security: by controlling the resolver, you control what files
//      a Starlark program can access.  An empty resolver = no load()
//      allowed.

// FileResolver takes a load() label (e.g., "helpers.star") and returns
// the source code for that file.  If the file cannot be found, it returns
// an error.
type FileResolver func(label string) (string, error)

// DictResolver creates a FileResolver backed by a static map of
// label -> source code.  This is ideal for testing and for cases where
// all loaded files are known in advance.
//
// Example:
//
//   resolver := DictResolver(map[string]string{
//       "math.star":    "PI = 3\n",
//       "helpers.star":  "def double(n):\n    return n * 2\n",
//   })
//
//   // Now load("math.star", "PI") will find PI = 3
//   // And load("helpers.star", "double") will find the function
//
// If the label is not in the map, the resolver returns an error:
//
//   _, err := resolver("missing.star")
//   // err: "load(): file not found: missing.star"
func DictResolver(files map[string]string) FileResolver {
	return func(label string) (string, error) {
		content, ok := files[label]
		if !ok {
			return "", fmt.Errorf("load(): file not found: %s", label)
		}
		return content, nil
	}
}

// ============================================================================
// INTERPRETER OPTIONS (FUNCTIONAL OPTIONS PATTERN)
// ============================================================================
//
// Go's "functional options" pattern lets you configure a struct with
// a clean API:
//
//   interp := NewInterpreter(
//       WithFileResolver(resolver),
//       WithMaxRecursionDepth(500),
//   )
//
// Each option is a function that modifies the interpreter.  This avoids
// the "config struct with 20 fields" anti-pattern and makes it easy to
// add new options without breaking existing code.

// InterpreterOption is a function that configures a StarlarkInterpreter.
type InterpreterOption func(*StarlarkInterpreter)

// WithFileResolver sets the file resolver used for load() statements.
// Without a resolver, any load() call will panic.
func WithFileResolver(resolver FileResolver) InterpreterOption {
	return func(i *StarlarkInterpreter) {
		i.FileResolver = resolver
	}
}

// WithMaxRecursionDepth sets the maximum call stack depth.
// The default is 200, which is generous for most Starlark programs.
// Increase this if you have deeply recursive functions or deeply
// nested load() chains.
func WithMaxRecursionDepth(depth int) InterpreterOption {
	return func(i *StarlarkInterpreter) {
		i.MaxRecursionDepth = depth
	}
}

// WithGlobals sets pre-seeded variables that are injected into every VM
// instance created by this interpreter. This is the mechanism for making
// build context (like _ctx) available to all Starlark code.
//
// Example: injecting a build context dict into every Starlark scope:
//
//	interp := NewInterpreter(
//	    WithGlobals(map[string]interface{}{
//	        "_ctx": map[string]interface{}{
//	            "version": 1,
//	            "os":      "darwin",
//	        },
//	    }),
//	)
func WithGlobals(globals map[string]interface{}) InterpreterOption {
	return func(i *StarlarkInterpreter) {
		i.Globals = globals
	}
}

// ============================================================================
// STARLARK INTERPRETER
// ============================================================================

// StarlarkInterpreter wraps the full Starlark pipeline with load() support
// and module caching.
//
// Fields:
//
//   FileResolver      -- How to find source code for load() labels.
//                        nil means load() is not supported (will panic).
//
//   MaxRecursionDepth -- Maximum call stack depth for the VM.
//                        Default is 200.
//
//   loadCache         -- Maps file labels to their executed variable maps.
//                        Prevents re-executing the same file twice.
//                        This is analogous to Python's sys.modules.
//
//   Globals           -- Pre-seeded variables injected into every VM
//                        instance before execution begins.
//                        Use this for build context like _ctx.
//                        nil means no globals are injected.
type StarlarkInterpreter struct {
	FileResolver      FileResolver
	MaxRecursionDepth int
	Globals           map[string]interface{}
	loadCache         map[string]map[string]interface{}
}

// NewInterpreter creates a new StarlarkInterpreter with optional
// configuration.
//
// With no options, you get a basic interpreter that supports all Starlark
// features except load() (since no file resolver is configured):
//
//   interp := NewInterpreter()
//   result, err := interp.Interpret("x = 42\n")
//
// With options:
//
//   interp := NewInterpreter(
//       WithFileResolver(DictResolver(files)),
//       WithMaxRecursionDepth(500),
//   )
func NewInterpreter(opts ...InterpreterOption) *StarlarkInterpreter {
	interp := &StarlarkInterpreter{
		MaxRecursionDepth: 200,
		loadCache:         make(map[string]map[string]interface{}),
	}
	for _, opt := range opts {
		opt(interp)
	}
	return interp
}

// ============================================================================
// INTERPRET -- Execute Starlark source code
// ============================================================================

// Interpret compiles and executes Starlark source code, returning the
// result (variables, output, traces) or an error if compilation fails.
//
// The execution pipeline:
//
//   1. COMPILE: source -> lexer -> parser -> AST -> bytecode (CodeObject)
//   2. CREATE VM: a fresh GenericVM with all 59 opcode handlers + 23 builtins
//   3. OVERRIDE: replace the default LOAD_MODULE handler with our resolver-aware version
//   4. EXECUTE: run the bytecode on the VM
//   5. RETURN: package up Variables, Output, and Traces into StarlarkResult
//
// If compilation fails (syntax error, etc.), the error is returned
// immediately and no VM is created.
//
// Runtime errors (division by zero, undefined variable, etc.) are
// reported as panics by the VM.  The caller can use recover() to
// catch them, or let them propagate.
func (interp *StarlarkInterpreter) Interpret(source string) (*starlarkvm.StarlarkResult, error) {
	// Step 1: Compile source to bytecode.
	// This runs the full front-end pipeline: lexer -> parser -> compiler.
	// If the source has a syntax error, we get an error here.
	code, err := starlarkcompiler.CompileStarlark(source)
	if err != nil {
		return nil, err
	}

	// Step 2: Create a fresh Starlark VM.
	// CreateStarlarkVM registers all 59 opcode handlers and 23 builtins.
	v := starlarkvm.CreateStarlarkVM(interp.MaxRecursionDepth)

	// Step 3: Override the LOAD_MODULE handler to support load().
	// The default handler (in starlark-vm/handlers.go) just pushes an
	// empty dict.  Our override actually resolves and executes the file.
	interp.registerLoadHandler(v)

	// Step 3.5: Inject pre-seeded globals (e.g., _ctx) into the VM before
	// execution so they are available as regular variables from the first
	// instruction.
	for k, val := range interp.Globals {
		v.Variables[k] = val
	}

	// Step 4: Execute the bytecode.
	traces := v.Execute(code)

	// Step 5: Package and return results.
	return &starlarkvm.StarlarkResult{
		Variables: v.Variables,
		Output:    v.Output,
		Traces:    traces,
	}, nil
}

// ============================================================================
// INTERPRET FILE -- Read and execute a .star file from disk
// ============================================================================

// InterpretFile reads a file from disk and executes it as Starlark code.
//
// This is a convenience wrapper around Interpret() for when your Starlark
// source lives in a file rather than a string.
//
// The path can be absolute or relative to the current working directory.
// The file must be valid UTF-8 text.
//
// If the file doesn't end with a newline, one is appended automatically.
// This is because the Starlark grammar requires a trailing newline
// (NEWLINE token) after the last statement.
func (interp *StarlarkInterpreter) InterpretFile(path string) (*starlarkvm.StarlarkResult, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}
	source := string(data)

	// Ensure trailing newline.  The lexer expects every line to end
	// with a newline, including the last one.  Without this, the last
	// statement might not be recognized.
	if !strings.HasSuffix(source, "\n") {
		source += "\n"
	}

	return interp.Interpret(source)
}

// ============================================================================
// LOAD HANDLER -- The heart of load() support
// ============================================================================
//
// When the compiler encounters a load() statement like:
//
//   load("helpers.star", "double", "triple")
//
// It emits these instructions:
//
//   LOAD_MODULE  <index-into-constants>   (constants[index] = "helpers.star")
//   DUP                                    (duplicate module dict on stack)
//   IMPORT_FROM  <name-index>              (names[index] = "double")
//   STORE_NAME   <name-index>              (store "double" in variables)
//   DUP
//   IMPORT_FROM  <name-index>              (names[index] = "triple")
//   STORE_NAME   <name-index>              (store "triple" in variables)
//   POP                                    (discard the module dict)
//
// The IMPORT_FROM handler is already implemented in starlark-vm/handlers.go.
// It peeks at the dict on top of stack and extracts the named symbol.
//
// We only need to override LOAD_MODULE to:
//   1. Read the module path from constants (not names!).
//   2. Check the cache.
//   3. If not cached, resolve the file and recursively interpret it.
//   4. Push the module's variables as a dict onto the stack.

// registerLoadHandler replaces the default LOAD_MODULE handler with one
// that actually resolves and executes loaded files.
func (interp *StarlarkInterpreter) registerLoadHandler(v *vm.GenericVM) {
	v.RegisterOpcode(starlarkcompiler.OpLoadModule, func(v *vm.GenericVM, instr vm.Instruction, code vm.CodeObject) *string {
		// The operand is an index into the CONSTANTS array (not names).
		// The compiler stores the module path as a constant string.
		idx := instr.Operand.(int)
		label := code.Constants[idx].(string)

		// Check the cache first.  If we've already executed this file,
		// reuse the cached variables.  This prevents infinite loops
		// (A loads B loads A) and avoids redundant work.
		if _, cached := interp.loadCache[label]; !cached {
			// No cache entry -- we need to resolve and execute the file.

			// Guard: if no file resolver is configured, we can't load anything.
			if interp.FileResolver == nil {
				panic(fmt.Sprintf("load() called but no file resolver configured: %s", label))
			}

			// Resolve the label to source code.
			contents, resolveErr := interp.FileResolver(label)
			if resolveErr != nil {
				panic(resolveErr.Error())
			}

			// Ensure trailing newline (same as InterpretFile).
			if !strings.HasSuffix(contents, "\n") {
				contents += "\n"
			}

			// Recursively interpret the loaded file.
			// This creates a NEW VM instance for the loaded file,
			// so it gets its own variable scope.  The loaded file's
			// variables become the "module" that IMPORT_FROM extracts from.
			result, interpErr := interp.Interpret(contents)
			if interpErr != nil {
				panic(fmt.Sprintf("error loading %s: %v", label, interpErr))
			}

			// Cache the result for future loads of the same file.
			interp.loadCache[label] = result.Variables
		}

		// Push a COPY of the cached module variables onto the stack.
		// We copy to prevent the importing program from mutating the
		// cached module state (which would affect other importers).
		moduleCopy := make(map[string]interface{})
		for k, val := range interp.loadCache[label] {
			moduleCopy[k] = val
		}
		v.Push(moduleCopy)

		// Advance past the LOAD_MODULE instruction.
		v.AdvancePC()
		return nil
	})
}

// ============================================================================
// MODULE-LEVEL CONVENIENCE FUNCTIONS
// ============================================================================
//
// These functions provide a simple API for callers who don't need the
// full StarlarkInterpreter struct.  They create a temporary interpreter,
// run the code, and return the result.
//
// For repeated execution (e.g., running many files with the same resolver),
// prefer creating an interpreter once and reusing it -- the loadCache
// will be shared across calls, improving performance.

// Interpret executes Starlark source code with an optional file resolver.
//
// This is the simplest way to run Starlark code:
//
//   result, err := starlarkinterpreter.Interpret("x = 42\n")
//
// With a resolver for load() support:
//
//   resolver := starlarkinterpreter.DictResolver(files)
//   result, err := starlarkinterpreter.Interpret(source, resolver)
func Interpret(source string, resolver ...FileResolver) (*starlarkvm.StarlarkResult, error) {
	opts := []InterpreterOption{}
	if len(resolver) > 0 && resolver[0] != nil {
		opts = append(opts, WithFileResolver(resolver[0]))
	}
	interp := NewInterpreter(opts...)
	return interp.Interpret(source)
}

// InterpretFile reads and executes a Starlark file from disk.
//
//   result, err := starlarkinterpreter.InterpretFile("build.star")
//
// With a resolver for load() support:
//
//   resolver := starlarkinterpreter.DictResolver(files)
//   result, err := starlarkinterpreter.InterpretFile("build.star", resolver)
func InterpretFile(path string, resolver ...FileResolver) (*starlarkvm.StarlarkResult, error) {
	opts := []InterpreterOption{}
	if len(resolver) > 0 && resolver[0] != nil {
		opts = append(opts, WithFileResolver(resolver[0]))
	}
	interp := NewInterpreter(opts...)
	return interp.InterpretFile(path)
}
