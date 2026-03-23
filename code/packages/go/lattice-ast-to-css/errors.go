package latticeasttocss

// errors.go — structured error types for the Lattice AST-to-CSS compiler.
//
// # Error Hierarchy
//
// All errors in the Lattice compiler carry a message and position (line/column)
// pointing to the source location that triggered them. The base type is
// LatticeError; every specific error embeds it.
//
// Errors are grouped by the compiler pass that detects them:
//
//   Pass 1 (Symbol Collection):
//     ModuleNotFoundError      — @use file not found
//     ReturnOutsideFunctionError — @return outside @function
//
//   Pass 2 (Expansion / Evaluation):
//     UndefinedVariableError   — $var used but never declared
//     UndefinedMixinError      — @include unknown-mixin
//     UndefinedFunctionError   — func() where func was never @function-defined
//     WrongArityError          — called with wrong number of arguments
//     CircularReferenceError   — mixin calls itself (directly or through a chain)
//     TypeErrorInExpression    — 10px + red (incompatible types in arithmetic)
//     UnitMismatchError        — 10px + 5s (incompatible CSS units)
//     MissingReturnError       — @function body never executes a @return
//
// # Catching All Lattice Errors
//
// Since all errors implement the LatticeError interface (via embedding),
// callers can catch the whole family:
//
//   css, err := TranspileLatticeFull(source, false, "  ")
//   var latticeErr *LatticeError
//   if errors.As(err, &latticeErr) {
//     fmt.Printf("Lattice error at line %d: %s\n", latticeErr.Line, latticeErr.Message)
//   }

import "fmt"

// ============================================================================
// Base Error Type
// ============================================================================

// LatticeError is the base type for all Lattice compiler errors.
//
// Every subtype embeds LatticeError to inherit the Line and Column fields.
// These positions come from the token that triggered the error — the Lattice
// lexer attaches line/column to every token it produces.
//
// The error message is intentionally human-readable: it should tell the
// developer what went wrong and (via Line/Column) where in the source.
//
// Example:
//
//	Undefined variable '$primary' at line 5, column 10
//	Cannot add '16px' and '2em' at line 8, column 14
type LatticeError struct {
	Message string
	Line    int
	Column  int
}

// Error implements the error interface.
// Format: "message at line N, column M" when position is known.
func (e *LatticeError) Error() string {
	if e.Line > 0 {
		return fmt.Sprintf("%s at line %d, column %d", e.Message, e.Line, e.Column)
	}
	return e.Message
}

// ============================================================================
// Pass 1: Symbol Collection Errors
// ============================================================================

// ModuleNotFoundError is raised when @use references a module that doesn't exist.
//
// Example: @use "nonexistent";   → cannot find file "nonexistent.lattice"
type ModuleNotFoundError struct {
	LatticeError
	ModuleName string
}

// NewModuleNotFoundError creates a ModuleNotFoundError with the given name and position.
func NewModuleNotFoundError(moduleName string, line, col int) *ModuleNotFoundError {
	return &ModuleNotFoundError{
		LatticeError: LatticeError{
			Message: fmt.Sprintf("Module '%s' not found", moduleName),
			Line:    line,
			Column:  col,
		},
		ModuleName: moduleName,
	}
}

// ReturnOutsideFunctionError is raised when @return appears outside a @function body.
//
// Example:
//
//	$x: 10px;
//	@return $x;   ← error: not inside @function
type ReturnOutsideFunctionError struct {
	LatticeError
}

// NewReturnOutsideFunctionError creates a ReturnOutsideFunctionError with position.
func NewReturnOutsideFunctionError(line, col int) *ReturnOutsideFunctionError {
	return &ReturnOutsideFunctionError{
		LatticeError: LatticeError{
			Message: "@return outside @function",
			Line:    line,
			Column:  col,
		},
	}
}

// ============================================================================
// Pass 2: Expansion / Evaluation Errors
// ============================================================================

// UndefinedVariableError is raised when a $variable is referenced but not declared.
//
// Example:
//
//	h1 { color: $nonexistent; }   ← error: $nonexistent never declared
type UndefinedVariableError struct {
	LatticeError
	Name string
}

// NewUndefinedVariableError creates an UndefinedVariableError.
func NewUndefinedVariableError(name string, line, col int) *UndefinedVariableError {
	return &UndefinedVariableError{
		LatticeError: LatticeError{
			Message: fmt.Sprintf("Undefined variable '%s'", name),
			Line:    line,
			Column:  col,
		},
		Name: name,
	}
}

// UndefinedMixinError is raised when @include references an unknown mixin.
//
// Example:
//
//	.btn { @include nonexistent; }   ← error: 'nonexistent' was never @mixin-defined
type UndefinedMixinError struct {
	LatticeError
	Name string
}

// NewUndefinedMixinError creates an UndefinedMixinError.
func NewUndefinedMixinError(name string, line, col int) *UndefinedMixinError {
	return &UndefinedMixinError{
		LatticeError: LatticeError{
			Message: fmt.Sprintf("Undefined mixin '%s'", name),
			Line:    line,
			Column:  col,
		},
		Name: name,
	}
}

// UndefinedFunctionError is raised when a function call references an unknown function.
//
// Note: this error only fires for Lattice functions (@function-defined). CSS
// built-ins like rgb(), calc(), var() are always passed through unchanged.
//
// Example:
//
//	padding: spacing(2);   ← error if 'spacing' was never @function-defined
type UndefinedFunctionError struct {
	LatticeError
	Name string
}

// NewUndefinedFunctionError creates an UndefinedFunctionError.
func NewUndefinedFunctionError(name string, line, col int) *UndefinedFunctionError {
	return &UndefinedFunctionError{
		LatticeError: LatticeError{
			Message: fmt.Sprintf("Undefined function '%s'", name),
			Line:    line,
			Column:  col,
		},
		Name: name,
	}
}

// WrongArityError is raised when a mixin or function is called with wrong arg count.
//
// "Arity" is the formal term for the number of arguments a function takes.
// Parameters with defaults are optional; those without defaults are required.
//
// Example:
//
//	@mixin button($bg, $fg) { ... }
//	@include button(red, blue, green);   ← error: expects 2 args, got 3
type WrongArityError struct {
	LatticeError
	Kind     string // "Mixin" or "Function"
	Name     string
	Expected int
	Got      int
}

// NewWrongArityError creates a WrongArityError.
func NewWrongArityError(kind, name string, expected, got, line, col int) *WrongArityError {
	return &WrongArityError{
		LatticeError: LatticeError{
			Message: fmt.Sprintf("%s '%s' expects %d args, got %d", kind, name, expected, got),
			Line:    line,
			Column:  col,
		},
		Kind:     kind,
		Name:     name,
		Expected: expected,
		Got:      got,
	}
}

// CircularReferenceError is raised when a mixin or function calls itself in a cycle.
//
// The Chain shows the full call path, making it easy to diagnose where the
// cycle starts. Example chain: ["a", "b", "a"] means a → b → a.
//
// Example:
//
//	@mixin a { @include b; }
//	@mixin b { @include a; }   ← Circular mixin: a → b → a
type CircularReferenceError struct {
	LatticeError
	Kind  string   // "mixin" or "function"
	Chain []string // call stack names, e.g. ["a", "b", "a"]
}

// NewCircularReferenceError creates a CircularReferenceError.
func NewCircularReferenceError(kind string, chain []string, line, col int) *CircularReferenceError {
	chainStr := ""
	for i, name := range chain {
		if i > 0 {
			chainStr += " → "
		}
		chainStr += name
	}
	return &CircularReferenceError{
		LatticeError: LatticeError{
			Message: fmt.Sprintf("Circular %s: %s", kind, chainStr),
			Line:    line,
			Column:  col,
		},
		Kind:  kind,
		Chain: chain,
	}
}

// TypeErrorInExpression is raised when arithmetic is attempted on incompatible types.
//
// Type compatibility rules:
//   - Number ± Number → OK
//   - Dimension ± Dimension (same unit) → OK
//   - Percentage ± Percentage → OK
//   - Number × Dimension → OK (scaling)
//   - Anything else → TypeErrorInExpression
//
// Example:
//
//	$n: 10px + red;   ← error: cannot add '10px' and 'red'
type TypeErrorInExpression struct {
	LatticeError
	Op        string // "add", "subtract", "multiply", "negate"
	LeftType  string
	RightType string
}

// NewTypeErrorInExpression creates a TypeErrorInExpression.
func NewTypeErrorInExpression(op, left, right string, line, col int) *TypeErrorInExpression {
	return &TypeErrorInExpression{
		LatticeError: LatticeError{
			Message: fmt.Sprintf("Cannot %s '%s' and '%s'", op, left, right),
			Line:    line,
			Column:  col,
		},
		Op:        op,
		LeftType:  left,
		RightType: right,
	}
}

// UnitMismatchError is raised when arithmetic combines CSS dimensions with
// incompatible units (e.g., 10px + 5s — length plus time).
//
// Compatible units (same unit) can be added directly: 10px + 5px = 15px.
// Incompatible-but-related units should use calc(): 10px + 2em.
// Truly incompatible units (different CSS types) are an error.
//
// Example:
//
//	$n: 10px + 5s;   ← error: cannot add 'px' and 's' units
type UnitMismatchError struct {
	LatticeError
	LeftUnit  string
	RightUnit string
}

// NewUnitMismatchError creates a UnitMismatchError.
func NewUnitMismatchError(leftUnit, rightUnit string, line, col int) *UnitMismatchError {
	return &UnitMismatchError{
		LatticeError: LatticeError{
			Message: fmt.Sprintf("Cannot add '%s' and '%s' units", leftUnit, rightUnit),
			Line:    line,
			Column:  col,
		},
		LeftUnit:  leftUnit,
		RightUnit: rightUnit,
	}
}

// MissingReturnError is raised when a @function body has no @return statement.
//
// Every @function must return a value via @return. A function that only
// declares variables or contains unreachable control flow is an error.
//
// Example:
//
//	@function noop($x) { $y: $x; }   ← error: no @return
type MissingReturnError struct {
	LatticeError
	Name string
}

// NewMissingReturnError creates a MissingReturnError.
func NewMissingReturnError(name string, line, col int) *MissingReturnError {
	return &MissingReturnError{
		LatticeError: LatticeError{
			Message: fmt.Sprintf("Function '%s' has no @return", name),
			Line:    line,
			Column:  col,
		},
		Name: name,
	}
}
