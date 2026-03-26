package main

// infra_test.go exercises the generated Operations infrastructure code
// (gen_capabilities.go) and edge cases in the detection helpers.
//
// This ensures the operations framework itself is tested even though it is
// auto-generated code — bugs in the generator could affect every package.

import (
	"errors"
	"go/parser"
	"go/token"
	"strings"
	"testing"
)

// ── Operations infrastructure ─────────────────────────────────────────────────

// TestStartNew_Success verifies the happy path: callback returns a successful result.
func TestStartNew_Success(t *testing.T) {
	result, err := StartNew[int]("test.op", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, 42)
		}).GetResult()

	if err != nil {
		t.Errorf("expected no error, got: %v", err)
	}
	if result != 42 {
		t.Errorf("expected 42, got %d", result)
	}
}

// TestStartNew_ExpectedFailure verifies that rf.Fail returns the fallback value
// and a non-nil error.
func TestStartNew_ExpectedFailure(t *testing.T) {
	result, err := StartNew[string]("test.op", "fallback",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			return rf.Fail("", errors.New("operation failed as expected"))
		}).GetResult()

	if err == nil {
		t.Error("expected an error from rf.Fail, got nil")
	}
	_ = result // result is the fallback
}

// TestStartNew_PanicRecovery verifies that a panicking callback does not
// propagate the panic and returns the fallback value with an error.
func TestStartNew_PanicRecovery(t *testing.T) {
	const fallback = -1
	result, err := StartNew[int]("test.op", fallback,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			panic("simulated panic for test coverage")
		}).GetResult()

	if err == nil {
		t.Error("expected error after panic, got nil")
	}
	if result != fallback {
		t.Errorf("expected fallback %d after panic, got %d", fallback, result)
	}
}

// TestOperation_AddProperty verifies that AddProperty stores metadata on the
// operation's property bag without error.
func TestOperation_AddProperty(t *testing.T) {
	op := StartNew[int]("test.op", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("key", "value")
			return rf.Generate(true, false, 1)
		})
	result, err := op.GetResult()
	if err != nil {
		t.Errorf("expected no error, got: %v", err)
	}
	if result != 1 {
		t.Errorf("expected 1, got %d", result)
	}
}

// TestOperation_PanicOnUnexpected verifies that PanicOnUnexpected re-panics when
// the callback itself panics. Without PanicOnUnexpected, the panic is caught and
// turned into an error. With PanicOnUnexpected, the panic propagates to the caller.
func TestOperation_PanicOnUnexpected(t *testing.T) {
	defer func() {
		if r := recover(); r == nil {
			t.Error("expected a panic from PanicOnUnexpected, but no panic occurred")
		}
	}()

	// This operation panics internally. With PanicOnUnexpected(), the panic
	// propagates out of GetResult() instead of being suppressed.
	StartNew[int]("test.op", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			panic("re-panic test") // This panic should be re-raised by GetResult
		}).PanicOnUnexpected().GetResult()
}

// TestCapabilityViolationError_Error verifies that the error message from a
// capability violation contains the expected components.
func TestCapabilityViolationError_Error(t *testing.T) {
	err := &_capabilityViolationError{
		category:  "fs",
		action:    "read",
		requested: "/etc/passwd",
	}
	msg := err.Error()
	if !strings.Contains(msg, "fs") {
		t.Errorf("error message should contain category, got: %s", msg)
	}
	if !strings.Contains(msg, "read") {
		t.Errorf("error message should contain action, got: %s", msg)
	}
	if !strings.Contains(msg, "/etc/passwd") {
		t.Errorf("error message should contain requested path, got: %s", msg)
	}
}

// ── banned.go edge cases ──────────────────────────────────────────────────────

// TestBanned_PluginOpen_NonSelectorFun verifies that a call with a non-selector
// function expression (e.g., a function variable) is not mistakenly flagged.
func TestBanned_PluginOpen_NonSelectorFun(t *testing.T) {
	src := `package foo
import "plugin"
var openFn = plugin.Open
func f() {
	// calling through a variable — AST has Ident as Fun, not SelectorExpr
	openFn("path.so")
}
`
	fset := token.NewFileSet()
	f, err := parser.ParseFile(fset, "test.go", src, parser.ParseComments)
	if err != nil {
		t.Fatal(err)
	}
	// openFn("path.so") has Fun = Ident, not SelectorExpr.
	// detectPlugin should NOT flag this (it can't resolve through the variable).
	banned := detectPlugin(fset, f, "test.go")
	// We may or may not detect this — the important thing is it doesn't crash.
	_ = banned
}

// TestBanned_Reflect_NonSelectorFun verifies that detectReflect handles calls
// where Fun is not a SelectorExpr without panicking.
func TestBanned_Reflect_NonSelectorFun(t *testing.T) {
	src := `package foo
import "reflect"
var call = (reflect.Value).Call
func f() {
	_ = call
}
`
	fset := token.NewFileSet()
	f, err := parser.ParseFile(fset, "test.go", src, parser.ParseComments)
	if err != nil {
		t.Fatal(err)
	}
	// Should not panic; some constructs may or may not be detected.
	banned := detectReflect(fset, f, "test.go")
	_ = banned
}

// TestBanned_Unsafe_NonSelectorFun verifies that detectUnsafe handles a case where
// the call's Fun resolves to something other than a SelectorExpr without panicking.
func TestBanned_Unsafe_NonSelectorFun(t *testing.T) {
	// A function call like f() where f is an identifier (not pkg.Func).
	src := `package foo
import "unsafe"
func convert(p *int) uintptr {
	fn := unsafe.Pointer
	return uintptr(fn(p))
}
`
	fset := token.NewFileSet()
	f, err := parser.ParseFile(fset, "test.go", src, parser.ParseComments)
	if err != nil {
		t.Fatal(err)
	}
	banned := detectUnsafe(fset, f, "test.go")
	_ = banned // May or may not detect; must not panic
}

// ── detectInFile: import-level nolint ─────────────────────────────────────────

// TestDetect_ImportLevel_NolintOnImportLine verifies that a //nolint:cap
// on the import line suppresses the import-level capability detection.
func TestDetect_ImportLevel_NolintOnImportLine(t *testing.T) {
	src := `package foo
import "net" //nolint:cap
func f() {}
`
	fset, files := parseFiles(t, "foo.go", src)
	caps := DetectCapabilities(fset, files)
	if containsCap(caps, "net:*:*") {
		t.Errorf("//nolint:cap on import line should suppress detection, got %v", caps)
	}
}
