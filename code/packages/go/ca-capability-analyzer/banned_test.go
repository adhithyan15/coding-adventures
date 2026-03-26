package main

// banned_test.go tests the DetectBanned function and its sub-detectors.
//
// Tests use inline Go source strings parsed with go/parser. No real files
// are needed for banned construct detection.

import (
	"go/ast"
	"go/parser"
	"go/token"
	"strings"
	"testing"
)

// hasBannedKind returns true if any BannedConstruct has the given kind substring.
func hasBannedKind(banned []BannedConstruct, kind string) bool {
	for _, b := range banned {
		if strings.Contains(b.Kind, kind) {
			return true
		}
	}
	return false
}

// parseForBanned is a test helper that parses src and returns (fset, *ast.File).
func parseForBanned(t *testing.T, src string) (*token.FileSet, *ast.File) {
	t.Helper()
	fset := token.NewFileSet()
	f, err := parser.ParseFile(fset, "test.go", src, parser.ParseComments)
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	return fset, f
}

// TestBanned_CGo verifies that import "C" is detected as a banned construct.
func TestBanned_CGo(t *testing.T) {
	src := `package foo

/*
#include <stdio.h>
*/
import "C"

func hello() {
	C.puts(C.CString("hello"))
}
`
	fset, f := parseForBanned(t, src)
	banned := DetectBanned(fset, f, "test.go")
	if !hasBannedKind(banned, "CGo") {
		t.Errorf("expected CGo banned construct, got %v", banned)
	}
}

// TestBanned_UnsafePointer verifies that unsafe.Pointer(...) conversions
// are detected as banned constructs.
func TestBanned_UnsafePointer(t *testing.T) {
	src := `package foo

import "unsafe"

func f(x int) uintptr {
	return uintptr(unsafe.Pointer(&x))
}
`
	fset, f := parseForBanned(t, src)
	banned := DetectBanned(fset, f, "test.go")
	if !hasBannedKind(banned, "unsafe.Pointer") {
		t.Errorf("expected unsafe.Pointer banned construct, got %v", banned)
	}
}

// TestBanned_UnsafeImportNoPointer verifies that merely importing "unsafe"
// without using unsafe.Pointer(...) does NOT trigger the banned-construct rule.
// unsafe.Sizeof, unsafe.Alignof, and unsafe.Offsetof are legitimate.
func TestBanned_UnsafeImportNoPointer(t *testing.T) {
	src := `package foo

import "unsafe"

type S struct{ x int }

func size() uintptr {
	return unsafe.Sizeof(S{})
}
`
	fset, f := parseForBanned(t, src)
	banned := DetectBanned(fset, f, "test.go")
	if hasBannedKind(banned, "unsafe.Pointer") {
		t.Errorf("unsafe.Sizeof should NOT trigger banned construct, got %v", banned)
	}
}

// TestBanned_PluginOpen verifies that plugin.Open calls are detected.
func TestBanned_PluginOpen(t *testing.T) {
	src := `package foo

import "plugin"

func load(path string) {
	p, _ := plugin.Open(path)
	_ = p
}
`
	fset, f := parseForBanned(t, src)
	banned := DetectBanned(fset, f, "test.go")
	if !hasBannedKind(banned, "plugin.Open") {
		t.Errorf("expected plugin.Open banned construct, got %v", banned)
	}
}

// TestBanned_ReflectValueCall verifies that reflect.Value.Call is detected.
func TestBanned_ReflectValueCall(t *testing.T) {
	src := `package foo

import "reflect"

func callIt(fn interface{}, args []reflect.Value) []reflect.Value {
	v := reflect.ValueOf(fn)
	return v.Call(args)
}
`
	fset, f := parseForBanned(t, src)
	banned := DetectBanned(fset, f, "test.go")
	if !hasBannedKind(banned, "Call") {
		t.Errorf("expected reflect.Value.Call banned construct, got %v", banned)
	}
}

// TestBanned_ReflectValueCallSlice verifies that reflect.Value.CallSlice is detected.
func TestBanned_ReflectValueCallSlice(t *testing.T) {
	src := `package foo

import "reflect"

func callSlice(fn interface{}, args []reflect.Value) []reflect.Value {
	v := reflect.ValueOf(fn)
	return v.CallSlice(args)
}
`
	fset, f := parseForBanned(t, src)
	banned := DetectBanned(fset, f, "test.go")
	if !hasBannedKind(banned, "CallSlice") {
		t.Errorf("expected reflect.Value.CallSlice banned construct, got %v", banned)
	}
}

// TestBanned_ReflectValueMethodByName verifies that reflect.Value.MethodByName is detected.
func TestBanned_ReflectValueMethodByName(t *testing.T) {
	src := `package foo

import "reflect"

func callMethod(x interface{}, name string) reflect.Value {
	v := reflect.ValueOf(x)
	return v.MethodByName(name)
}
`
	fset, f := parseForBanned(t, src)
	banned := DetectBanned(fset, f, "test.go")
	if !hasBannedKind(banned, "MethodByName") {
		t.Errorf("expected reflect.Value.MethodByName banned construct, got %v", banned)
	}
}

// TestBanned_GoLinkname verifies that //go:linkname directives are detected.
func TestBanned_GoLinkname(t *testing.T) {
	src := `package foo

import _ "unsafe"

//go:linkname nanotime runtime.nanotime
func nanotime() int64
`
	fset, f := parseForBanned(t, src)
	banned := DetectBanned(fset, f, "test.go")
	if !hasBannedKind(banned, "go:linkname") {
		t.Errorf("expected go:linkname banned construct, got %v", banned)
	}
}

// TestBanned_NoBannedConstructs verifies that a clean file produces no banned constructs.
func TestBanned_NoBannedConstructs(t *testing.T) {
	src := `package foo

import (
	"fmt"
	"strings"
)

func greet(name string) string {
	return fmt.Sprintf("Hello, %s!", strings.TrimSpace(name))
}
`
	fset, f := parseForBanned(t, src)
	banned := DetectBanned(fset, f, "test.go")
	if len(banned) != 0 {
		t.Errorf("expected no banned constructs in clean file, got %v", banned)
	}
}

// TestBanned_ReflectTypeOfNotBanned verifies that reflect.TypeOf and reflect.ValueOf
// are NOT banned — only the dynamic dispatch methods are.
func TestBanned_ReflectTypeOfNotBanned(t *testing.T) {
	src := `package foo

import "reflect"

func typeOf(x interface{}) string {
	return reflect.TypeOf(x).String()
}

func valueOf(x interface{}) reflect.Value {
	return reflect.ValueOf(x)
}
`
	fset, f := parseForBanned(t, src)
	banned := DetectBanned(fset, f, "test.go")
	for _, b := range banned {
		if strings.Contains(b.Kind, "TypeOf") || strings.Contains(b.Kind, "ValueOf") {
			t.Errorf("reflect.TypeOf/ValueOf should NOT be banned, got %v", banned)
		}
	}
}

// TestBanned_LinenumberReported verifies that the line number in a BannedConstruct
// is the line of the banned usage, not line 1.
func TestBanned_LinenumberReported(t *testing.T) {
	src := `package foo

import _ "unsafe"

// line 5 is a comment
//go:linkname nanotime runtime.nanotime
func nanotime() int64
`
	fset, f := parseForBanned(t, src)
	banned := DetectBanned(fset, f, "test.go")
	if len(banned) == 0 {
		t.Fatal("expected a banned construct")
	}
	if banned[0].Line <= 1 {
		t.Errorf("expected line > 1, got %d", banned[0].Line)
	}
}

// TestBanned_FilenameReported verifies that the filename in a BannedConstruct
// matches the filename passed to DetectBanned.
func TestBanned_FilenameReported(t *testing.T) {
	src := `package foo
import "C"
`
	fset, f := parseForBanned(t, src)
	banned := DetectBanned(fset, f, "myfile.go")
	if len(banned) == 0 {
		t.Fatal("expected a banned construct")
	}
	if banned[0].File != "myfile.go" {
		t.Errorf("expected file %q, got %q", "myfile.go", banned[0].File)
	}
}
