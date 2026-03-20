package analyzer

import (
	"strings"
	"testing"
)

// ── Helper functions for banned tests ────────────────────────────────

// mustDetectBanned parses source and returns banned construct violations,
// failing the test if parsing fails.
func mustDetectBanned(t *testing.T, source string) []BannedConstructViolation {
	t.Helper()
	violations, err := DetectBannedSource("test.go", source)
	if err != nil {
		t.Fatalf("failed to parse source: %v", err)
	}
	return violations
}

// assertBannedFound checks that at least one violation matches the given
// construct name.
func assertBannedFound(t *testing.T, violations []BannedConstructViolation, construct string) {
	t.Helper()
	for _, v := range violations {
		if v.Construct == construct {
			return
		}
	}
	t.Errorf("expected banned construct %q but not found in %v", construct, violations)
}

// assertNoBanned checks that no banned constructs were detected.
func assertNoBanned(t *testing.T, violations []BannedConstructViolation) {
	t.Helper()
	if len(violations) != 0 {
		t.Errorf("expected no banned constructs but found %d: %v", len(violations), violations)
	}
}

// ── reflect.Value.Call tests ────────────────────────────────────────
//
// reflect.Value.Call() is banned because it enables dynamic method
// invocation. An attacker can use it to call any method on any object
// without the call appearing in the source code statically.

func TestReflectValueCall(t *testing.T) {
	violations := mustDetectBanned(t, `package main
import "reflect"
func main() {
	v := reflect.ValueOf(42)
	reflect.ValueOf(v).Call(nil)
}
`)
	assertBannedFound(t, violations, "reflect.Value.Call")
}

func TestReflectValueCallChained(t *testing.T) {
	// Chained: reflect.ValueOf(x).Call(args)
	violations := mustDetectBanned(t, `package main
import "reflect"
func main() {
	reflect.ValueOf(42).Call(nil)
}
`)
	assertBannedFound(t, violations, "reflect.Value.Call")
}

// ── reflect.MethodByName tests ──────────────────────────────────────
//
// reflect.ValueOf(x).MethodByName("name") enables dynamic dispatch —
// looking up methods by string name at runtime. This completely bypasses
// static analysis.

func TestReflectMethodByName(t *testing.T) {
	violations := mustDetectBanned(t, `package main
import "reflect"
func main() {
	reflect.ValueOf(42).MethodByName("String")
}
`)
	assertBannedFound(t, violations, "reflect.MethodByName")
}

// ── plugin.Open tests ───────────────────────────────────────────────
//
// plugin.Open() loads a Go plugin (shared library) at runtime. The loaded
// code is arbitrary and cannot be analyzed statically.

func TestPluginOpen(t *testing.T) {
	violations := mustDetectBanned(t, `package main
import "plugin"
func main() {
	plugin.Open("myplugin.so")
}
`)
	assertBannedFound(t, violations, "plugin.Open")
}

// ── //go:linkname tests ─────────────────────────────────────────────
//
// //go:linkname is a compiler directive that lets one package access
// unexported symbols from another package. It completely breaks Go's
// encapsulation model.

func TestGoLinkname(t *testing.T) {
	violations := mustDetectBanned(t, `package main

import _ "unsafe"

//go:linkname runtimeNano runtime.nanotime
func runtimeNano() int64

func main() { _ = runtimeNano() }
`)
	assertBannedFound(t, violations, "go:linkname")
}

func TestGoLinknameWithArgs(t *testing.T) {
	// //go:linkname with the full two-argument form
	violations := mustDetectBanned(t, `package main

import _ "unsafe"

//go:linkname localFunc remote/package.remoteFunc
func localFunc()

func main() {}
`)
	assertBannedFound(t, violations, "go:linkname")
}

// ── unsafe.Pointer tests ────────────────────────────────────────────
//
// unsafe.Pointer breaks Go's type system. It can be used for pointer
// arithmetic, type punning, and memory corruption.

func TestUnsafePointer(t *testing.T) {
	violations := mustDetectBanned(t, `package main
import "unsafe"
func main() {
	x := 42
	p := unsafe.Pointer(&x)
	_ = p
}
`)
	assertBannedFound(t, violations, "unsafe.Pointer")
}

func TestUnsafePointerConversion(t *testing.T) {
	// Using unsafe.Pointer in a type conversion
	violations := mustDetectBanned(t, `package main
import "unsafe"
func main() {
	var x int64 = 42
	p := unsafe.Pointer(&x)
	_ = *(*float64)(p)
}
`)
	assertBannedFound(t, violations, "unsafe.Pointer")
}

// ── import "C" (cgo) tests ──────────────────────────────────────────
//
// import "C" enables calling arbitrary C functions. No static analysis
// can determine what C code does.

func TestImportCgoBanned(t *testing.T) {
	violations := mustDetectBanned(t, `package main
import "C"
func main() {}
`)
	assertBannedFound(t, violations, "cgo")
}

// ── Clean code tests (no banned constructs) ─────────────────────────
//
// These tests verify that normal Go code does NOT trigger false positives.

func TestCleanCode(t *testing.T) {
	violations := mustDetectBanned(t, `package main
import "fmt"
func main() { fmt.Println("hello") }
`)
	assertNoBanned(t, violations)
}

func TestCleanCodeWithOS(t *testing.T) {
	// os package is a capability, not a banned construct
	violations := mustDetectBanned(t, `package main
import "os"
func main() { os.Open("file.txt") }
`)
	assertNoBanned(t, violations)
}

func TestCleanCodeWithReflectTypeOf(t *testing.T) {
	// reflect.TypeOf is NOT banned — it's just type inspection, not
	// dynamic invocation. Only Call() and MethodByName() are banned.
	violations := mustDetectBanned(t, `package main
import "reflect"
func main() { _ = reflect.TypeOf(42) }
`)
	assertNoBanned(t, violations)
}

func TestCleanCodeWithMath(t *testing.T) {
	violations := mustDetectBanned(t, `package main
import "math"
func main() { _ = math.Sqrt(4.0) }
`)
	assertNoBanned(t, violations)
}

func TestCleanCodeWithStrings(t *testing.T) {
	violations := mustDetectBanned(t, `package main
import "strings"
func main() { _ = strings.ToUpper("hi") }
`)
	assertNoBanned(t, violations)
}

func TestNonReflectCallMethod(t *testing.T) {
	// A method named "Call" on a non-reflect type should NOT be flagged.
	// However, our heuristic checks the receiver — since myObj is not
	// from the reflect package, it won't match.
	violations := mustDetectBanned(t, `package main

type MyType struct{}
func (m MyType) Call(args []int) {}

func main() {
	m := MyType{}
	m.Call(nil)
}
`)
	assertNoBanned(t, violations)
}

// ── Multiple violations in one file ─────────────────────────────────

func TestMultipleBannedConstructs(t *testing.T) {
	violations := mustDetectBanned(t, `package main
import (
	"reflect"
	"unsafe"
)

func main() {
	// Two banned constructs in one file
	reflect.ValueOf(42).Call(nil)
	p := unsafe.Pointer(nil)
	_ = p
}
`)
	if len(violations) < 2 {
		t.Errorf("expected at least 2 violations, got %d", len(violations))
	}
	assertBannedFound(t, violations, "reflect.Value.Call")
	assertBannedFound(t, violations, "unsafe.Pointer")
}

// ── BannedConstructViolation String test ────────────────────────────

func TestBannedConstructViolationString(t *testing.T) {
	v := BannedConstructViolation{
		Construct: "reflect.Value.Call",
		File:      "test.go",
		Line:      5,
		Evidence:  "reflect.Value.Call(...)",
	}
	s := v.String()
	if !strings.Contains(s, "BANNED") {
		t.Errorf("expected String() to contain 'BANNED', got %q", s)
	}
	if !strings.Contains(s, "reflect.Value.Call") {
		t.Errorf("expected String() to contain construct name, got %q", s)
	}
}

// ── Parse error test ────────────────────────────────────────────────

func TestDetectBannedSourceParseError(t *testing.T) {
	_, err := DetectBannedSource("bad.go", "this is not valid go }{")
	if err == nil {
		t.Error("expected parse error for invalid Go source")
	}
}

// ── Line number and evidence tests ──────────────────────────────────

func TestBannedLineNumbers(t *testing.T) {
	violations := mustDetectBanned(t, `package main
import "unsafe"
func main() {
	p := unsafe.Pointer(nil)
	_ = p
}
`)
	for _, v := range violations {
		if v.Line <= 0 {
			t.Errorf("expected positive line number, got %d for %s", v.Line, v.Construct)
		}
	}
}

func TestBannedEvidence(t *testing.T) {
	violations := mustDetectBanned(t, `package main

//go:linkname myFunc runtime.myFunc
func myFunc()

func main() {}
`)
	for _, v := range violations {
		if v.Construct == "go:linkname" {
			if !strings.Contains(v.Evidence, "go:linkname") {
				t.Errorf("expected evidence to contain 'go:linkname', got %q", v.Evidence)
			}
			return
		}
	}
	t.Error("go:linkname violation not found")
}
