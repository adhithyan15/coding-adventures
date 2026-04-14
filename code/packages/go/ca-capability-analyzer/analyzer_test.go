package main

// analyzer_test.go tests DetectCapabilities (unit) and AnalyzeDir (integration).
//
// Unit tests parse inline source strings directly — no real filesystem access.
// Integration tests create temporary directories with real .go files.

import (
	"go/ast"
	"go/parser"
	"go/token"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// ── Helpers ───────────────────────────────────────────────────────────────────

// parseFiles parses one or more (filename, source) pairs and returns
// fset + the map expected by DetectCapabilities.
func parseFiles(t *testing.T, pairs ...string) (*token.FileSet, map[string]*ast.File) {
	t.Helper()
	if len(pairs)%2 != 0 {
		t.Fatal("parseFiles: pairs must be (filename, source) pairs")
	}
	fset := token.NewFileSet()
	files := make(map[string]*ast.File)
	for i := 0; i < len(pairs); i += 2 {
		filename := pairs[i]
		src := pairs[i+1]
		f, err := parser.ParseFile(fset, filename, src, parser.ParseComments)
		if err != nil {
			t.Fatalf("parseFiles: parse error in %q: %v", filename, err)
		}
		files[filename] = f
	}
	return fset, files
}

// containsCap reports whether any DetectedCapability has the given capability string.
func containsCap(caps []DetectedCapability, want CapabilityString) bool {
	for _, c := range caps {
		if c.Capability == want {
			return true
		}
	}
	return false
}

// writeGoFile writes content to filename in dir.
func writeGoFile(t *testing.T, dir, filename, content string) {
	t.Helper()
	if err := os.WriteFile(filepath.Join(dir, filename), []byte(content), 0o600); err != nil {
		t.Fatalf("writeGoFile: %v", err)
	}
}

// writeManifestInDir writes a required_capabilities.json to dir.
func writeManifestInDir(t *testing.T, dir, content string) {
	t.Helper()
	if err := os.WriteFile(filepath.Join(dir, "required_capabilities.json"), []byte(content), 0o600); err != nil {
		t.Fatalf("writeManifestInDir: %v", err)
	}
}

// ── Capability detection unit tests ──────────────────────────────────────────

// TestDetect_FSRead_OsReadFile verifies that os.ReadFile is detected as fs:read:*.
func TestDetect_FSRead_OsReadFile(t *testing.T) {
	fset, files := parseFiles(t,
		"foo.go", `package foo
import "os"
func f() { os.ReadFile("x") }
`)
	caps := DetectCapabilities(fset, files)
	if !containsCap(caps, "fs:read:*") {
		t.Errorf("expected fs:read:*, got %v", caps)
	}
}

// TestDetect_FSRead_OsOpen verifies that os.Open is detected as fs:read:*.
func TestDetect_FSRead_OsOpen(t *testing.T) {
	fset, files := parseFiles(t,
		"foo.go", `package foo
import "os"
func f() { os.Open("x") }
`)
	caps := DetectCapabilities(fset, files)
	if !containsCap(caps, "fs:read:*") {
		t.Errorf("expected fs:read:*, got %v", caps)
	}
}

// TestDetect_FSRead_OsOpenFile verifies that os.OpenFile is detected as fs:read:*.
func TestDetect_FSRead_OsOpenFile(t *testing.T) {
	fset, files := parseFiles(t,
		"foo.go", `package foo
import "os"
func f() { os.OpenFile("x", os.O_RDONLY, 0o644) }
`)
	caps := DetectCapabilities(fset, files)
	if !containsCap(caps, "fs:read:*") {
		t.Errorf("expected fs:read:*, got %v", caps)
	}
}

// TestDetect_FSWrite_OsCreate verifies that os.Create is detected as fs:write:*.
func TestDetect_FSWrite_OsCreate(t *testing.T) {
	fset, files := parseFiles(t,
		"foo.go", `package foo
import "os"
func f() { os.Create("x") }
`)
	caps := DetectCapabilities(fset, files)
	if !containsCap(caps, "fs:write:*") {
		t.Errorf("expected fs:write:*, got %v", caps)
	}
}

// TestDetect_FSWrite_OsWriteFile verifies that os.WriteFile is detected as fs:write:*.
func TestDetect_FSWrite_OsWriteFile(t *testing.T) {
	fset, files := parseFiles(t,
		"foo.go", `package foo
import "os"
func f() { os.WriteFile("x", nil, 0o644) }
`)
	caps := DetectCapabilities(fset, files)
	if !containsCap(caps, "fs:write:*") {
		t.Errorf("expected fs:write:*, got %v", caps)
	}
}

// TestDetect_FSWrite_OsMkdir verifies that os.Mkdir is detected as fs:write:*.
func TestDetect_FSWrite_OsMkdir(t *testing.T) {
	fset, files := parseFiles(t,
		"foo.go", `package foo
import "os"
func f() { os.Mkdir("x", 0o755) }
`)
	caps := DetectCapabilities(fset, files)
	if !containsCap(caps, "fs:write:*") {
		t.Errorf("expected fs:write:*, got %v", caps)
	}
}

// TestDetect_FSWrite_OsMkdirAll verifies that os.MkdirAll is detected as fs:write:*.
func TestDetect_FSWrite_OsMkdirAll(t *testing.T) {
	fset, files := parseFiles(t,
		"foo.go", `package foo
import "os"
func f() { os.MkdirAll("x/y", 0o755) }
`)
	caps := DetectCapabilities(fset, files)
	if !containsCap(caps, "fs:write:*") {
		t.Errorf("expected fs:write:*, got %v", caps)
	}
}

// TestDetect_FSDelete_OsRemove verifies that os.Remove is detected as fs:delete:*.
func TestDetect_FSDelete_OsRemove(t *testing.T) {
	fset, files := parseFiles(t,
		"foo.go", `package foo
import "os"
func f() { os.Remove("x") }
`)
	caps := DetectCapabilities(fset, files)
	if !containsCap(caps, "fs:delete:*") {
		t.Errorf("expected fs:delete:*, got %v", caps)
	}
}

// TestDetect_FSDelete_OsRemoveAll verifies that os.RemoveAll is detected as fs:delete:*.
func TestDetect_FSDelete_OsRemoveAll(t *testing.T) {
	fset, files := parseFiles(t,
		"foo.go", `package foo
import "os"
func f() { os.RemoveAll("x") }
`)
	caps := DetectCapabilities(fset, files)
	if !containsCap(caps, "fs:delete:*") {
		t.Errorf("expected fs:delete:*, got %v", caps)
	}
}

// TestDetect_FSList_OsReadDir verifies that os.ReadDir is detected as fs:list:*.
func TestDetect_FSList_OsReadDir(t *testing.T) {
	fset, files := parseFiles(t,
		"foo.go", `package foo
import "os"
func f() { os.ReadDir(".") }
`)
	caps := DetectCapabilities(fset, files)
	if !containsCap(caps, "fs:list:*") {
		t.Errorf("expected fs:list:*, got %v", caps)
	}
}

// TestDetect_FSList_OsStat verifies that os.Stat is detected as fs:list:*.
func TestDetect_FSList_OsStat(t *testing.T) {
	fset, files := parseFiles(t,
		"foo.go", `package foo
import "os"
func f() { os.Stat("x") }
`)
	caps := DetectCapabilities(fset, files)
	if !containsCap(caps, "fs:list:*") {
		t.Errorf("expected fs:list:*, got %v", caps)
	}
}

// TestDetect_Net_ImportNet verifies that import "net" alone triggers net:*:*.
func TestDetect_Net_ImportNet(t *testing.T) {
	fset, files := parseFiles(t,
		"foo.go", `package foo
import "net"
func f() { net.Dial("tcp", ":8080") }
`)
	caps := DetectCapabilities(fset, files)
	if !containsCap(caps, "net:*:*") {
		t.Errorf("expected net:*:*, got %v", caps)
	}
}

// TestDetect_Net_ImportNetHTTP verifies that import "net/http" triggers net:*:*.
func TestDetect_Net_ImportNetHTTP(t *testing.T) {
	fset, files := parseFiles(t,
		"foo.go", `package foo
import "net/http"
func f() { http.Get("http://example.com") }
`)
	caps := DetectCapabilities(fset, files)
	if !containsCap(caps, "net:*:*") {
		t.Errorf("expected net:*:*, got %v", caps)
	}
}

// TestDetect_Proc_ImportOsExec verifies that import "os/exec" triggers proc:exec:*.
func TestDetect_Proc_ImportOsExec(t *testing.T) {
	fset, files := parseFiles(t,
		"foo.go", `package foo
import "os/exec"
func f() { exec.Command("ls").Run() }
`)
	caps := DetectCapabilities(fset, files)
	if !containsCap(caps, "proc:exec:*") {
		t.Errorf("expected proc:exec:*, got %v", caps)
	}
}

// TestDetect_Env_OsGetenv verifies that os.Getenv is detected as env:read:*.
func TestDetect_Env_OsGetenv(t *testing.T) {
	fset, files := parseFiles(t,
		"foo.go", `package foo
import "os"
func f() string { return os.Getenv("HOME") }
`)
	caps := DetectCapabilities(fset, files)
	if !containsCap(caps, "env:read:*") {
		t.Errorf("expected env:read:*, got %v", caps)
	}
}

// TestDetect_Env_OsEnviron verifies that os.Environ is detected as env:read:*.
func TestDetect_Env_OsEnviron(t *testing.T) {
	fset, files := parseFiles(t,
		"foo.go", `package foo
import "os"
func f() []string { return os.Environ() }
`)
	caps := DetectCapabilities(fset, files)
	if !containsCap(caps, "env:read:*") {
		t.Errorf("expected env:read:*, got %v", caps)
	}
}

// TestDetect_Env_OsLookupEnv verifies that os.LookupEnv is detected as env:read:*.
func TestDetect_Env_OsLookupEnv(t *testing.T) {
	fset, files := parseFiles(t,
		"foo.go", `package foo
import "os"
func f() (string, bool) { return os.LookupEnv("HOME") }
`)
	caps := DetectCapabilities(fset, files)
	if !containsCap(caps, "env:read:*") {
		t.Errorf("expected env:read:*, got %v", caps)
	}
}

// TestDetect_Time_TimeNow verifies that time.Now is detected as time:read:*.
func TestDetect_Time_TimeNow(t *testing.T) {
	fset, files := parseFiles(t,
		"foo.go", `package foo
import "time"
func f() time.Time { return time.Now() }
`)
	caps := DetectCapabilities(fset, files)
	if !containsCap(caps, "time:read:*") {
		t.Errorf("expected time:read:*, got %v", caps)
	}
}

// TestDetect_Time_TimeSleep verifies that time.Sleep is detected as time:read:*.
func TestDetect_Time_TimeSleep(t *testing.T) {
	fset, files := parseFiles(t,
		"foo.go", `package foo
import "time"
func f() { time.Sleep(time.Second) }
`)
	caps := DetectCapabilities(fset, files)
	if !containsCap(caps, "time:read:*") {
		t.Errorf("expected time:read:*, got %v", caps)
	}
}

// TestDetect_Stdout_FmtPrintln verifies that fmt.Println is detected as stdout:write:*.
func TestDetect_Stdout_FmtPrintln(t *testing.T) {
	fset, files := parseFiles(t,
		"foo.go", `package foo
import "fmt"
func f() { fmt.Println("hello") }
`)
	caps := DetectCapabilities(fset, files)
	if !containsCap(caps, "stdout:write:*") {
		t.Errorf("expected stdout:write:*, got %v", caps)
	}
}

// TestDetect_Stdout_FmtPrint verifies that fmt.Print is detected as stdout:write:*.
func TestDetect_Stdout_FmtPrint(t *testing.T) {
	fset, files := parseFiles(t,
		"foo.go", `package foo
import "fmt"
func f() { fmt.Print("hello") }
`)
	caps := DetectCapabilities(fset, files)
	if !containsCap(caps, "stdout:write:*") {
		t.Errorf("expected stdout:write:*, got %v", caps)
	}
}

// TestDetect_Stdout_FmtPrintf verifies that fmt.Printf is detected as stdout:write:*.
func TestDetect_Stdout_FmtPrintf(t *testing.T) {
	fset, files := parseFiles(t,
		"foo.go", `package foo
import "fmt"
func f() { fmt.Printf("%s\n", "hello") }
`)
	caps := DetectCapabilities(fset, files)
	if !containsCap(caps, "stdout:write:*") {
		t.Errorf("expected stdout:write:*, got %v", caps)
	}
}

// TestDetect_Stdout_OsStdoutWrite verifies that os.Stdout.Write is detected.
func TestDetect_Stdout_OsStdoutWrite(t *testing.T) {
	fset, files := parseFiles(t,
		"foo.go", `package foo
import "os"
func f() { os.Stdout.Write([]byte("hello")) }
`)
	caps := DetectCapabilities(fset, files)
	if !containsCap(caps, "stdout:write:*") {
		t.Errorf("expected stdout:write:*, got %v", caps)
	}
}

// TestDetect_Stdin_OsStdinRead verifies that os.Stdin.Read is detected.
func TestDetect_Stdin_OsStdinRead(t *testing.T) {
	fset, files := parseFiles(t,
		"foo.go", `package foo
import "os"
func f() { buf := make([]byte, 10); os.Stdin.Read(buf) }
`)
	caps := DetectCapabilities(fset, files)
	if !containsCap(caps, "stdin:read:*") {
		t.Errorf("expected stdin:read:*, got %v", caps)
	}
}

// TestDetect_Stdout_FmtFprintfOsStdout verifies that fmt.Fprintf(os.Stdout, ...) is detected.
func TestDetect_Stdout_FmtFprintfOsStdout(t *testing.T) {
	fset, files := parseFiles(t,
		"foo.go", `package foo
import (
	"fmt"
	"os"
)
func f() { fmt.Fprintf(os.Stdout, "hello %s", "world") }
`)
	caps := DetectCapabilities(fset, files)
	if !containsCap(caps, "stdout:write:*") {
		t.Errorf("expected stdout:write:*, got %v", caps)
	}
}

// TestDetect_NolintExemption verifies that a line annotated with //nolint:cap
// is NOT flagged, even if it contains a raw OS call.
func TestDetect_NolintExemption(t *testing.T) {
	fset, files := parseFiles(t,
		"foo.go", `package foo
import "os"
func f() { os.ReadFile("x") } //nolint:cap
`)
	caps := DetectCapabilities(fset, files)
	if containsCap(caps, "fs:read:*") {
		t.Errorf("//nolint:cap annotated line should not be flagged, got %v", caps)
	}
}

// TestDetect_GenCapabilitiesSkipped verifies that a file named gen_capabilities.go
// is entirely skipped, even if it contains raw OS calls.
func TestDetect_GenCapabilitiesSkipped(t *testing.T) {
	fset, files := parseFiles(t,
		"gen_capabilities.go", `package foo
import "os"
func f() { os.ReadFile("x") } //nolint:cap
`)
	caps := DetectCapabilities(fset, files)
	if len(caps) != 0 {
		t.Errorf("gen_capabilities.go should be entirely skipped, got %v", caps)
	}
}

// TestDetect_OperationsCallNotFlagged_Op verifies that op.File.ReadFile is not flagged.
func TestDetect_OperationsCallNotFlagged_Op(t *testing.T) {
	fset, files := parseFiles(t,
		"foo.go", `package foo
func f(op *Operation) {
	op.File.ReadFile("x")
}
`)
	caps := DetectCapabilities(fset, files)
	if containsCap(caps, "fs:read:*") {
		t.Errorf("op.File.ReadFile should not be flagged as raw OS call, got %v", caps)
	}
}

// TestDetect_OperationsCallNotFlagged_Cage verifies that cage.ReadFile is not flagged.
func TestDetect_OperationsCallNotFlagged_Cage(t *testing.T) {
	fset, files := parseFiles(t,
		"foo.go", `package foo
func f(cage *Cage) {
	cage.ReadFile("x")
}
`)
	caps := DetectCapabilities(fset, files)
	if containsCap(caps, "fs:read:*") {
		t.Errorf("cage.ReadFile should not be flagged as raw OS call, got %v", caps)
	}
}

// TestDetect_PureComputation verifies that a file with no OS calls produces no detections.
func TestDetect_PureComputation(t *testing.T) {
	fset, files := parseFiles(t,
		"foo.go", `package foo

// Add returns the sum of a and b.
// Pure computation: no OS access, no imports needed.
func Add(a, b int) int {
	return a + b
}
`)
	caps := DetectCapabilities(fset, files)
	if len(caps) != 0 {
		t.Errorf("pure computation file should have no detected caps, got %v", caps)
	}
}

// TestDetect_MultipleCapabilities verifies that a file using both os.ReadFile and
// time.Now produces detections for both capabilities.
func TestDetect_MultipleCapabilities(t *testing.T) {
	fset, files := parseFiles(t,
		"foo.go", `package foo
import (
	"os"
	"time"
)
func f() {
	os.ReadFile("x")
	time.Now()
}
`)
	caps := DetectCapabilities(fset, files)
	if !containsCap(caps, "fs:read:*") {
		t.Errorf("expected fs:read:*, got %v", caps)
	}
	if !containsCap(caps, "time:read:*") {
		t.Errorf("expected time:read:*, got %v", caps)
	}
}

// TestDetect_AliasedImport verifies that aliased imports are correctly resolved.
// import myfmt "fmt" + myfmt.Println → stdout:write:*
func TestDetect_AliasedImport(t *testing.T) {
	fset, files := parseFiles(t,
		"foo.go", `package foo
import myfmt "fmt"
func f() { myfmt.Println("hello") }
`)
	caps := DetectCapabilities(fset, files)
	if !containsCap(caps, "stdout:write:*") {
		t.Errorf("aliased import: expected stdout:write:*, got %v", caps)
	}
}

// TestDetect_MultipleFiles verifies that detections across multiple files are
// all returned.
func TestDetect_MultipleFiles(t *testing.T) {
	fset, files := parseFiles(t,
		"a.go", `package foo
import "os"
func a() { os.ReadFile("x") }
`,
		"b.go", `package foo
import "time"
func b() { time.Now() }
`,
	)
	caps := DetectCapabilities(fset, files)
	if !containsCap(caps, "fs:read:*") {
		t.Errorf("expected fs:read:* from a.go, got %v", caps)
	}
	if !containsCap(caps, "time:read:*") {
		t.Errorf("expected time:read:* from b.go, got %v", caps)
	}
}

// ── AnalyzeDir integration tests ──────────────────────────────────────────────

// TestAnalyzeDir_CleanPackage verifies that a package with a declared capability
// and a matching raw call passes (no violations).
func TestAnalyzeDir_CleanPackage(t *testing.T) {
	dir := t.TempDir()
	writeGoFile(t, dir, "foo.go", `package foo
import "os"
func f() { os.ReadFile("x") }
`)
	writeManifestInDir(t, dir, `{
		"version": 1,
		"package": "go/test-pkg",
		"capabilities": [
			{"category": "fs", "action": "read", "target": "*", "justification": "reads files"}
		],
		"justification": "reads files"
	}`)

	result, err := AnalyzeDir(dir)
	if err != nil {
		t.Fatalf("AnalyzeDir: %v", err)
	}
	if !result.Passed() {
		t.Errorf("expected pass, got violations: %v", result.Violations)
	}
}

// TestAnalyzeDir_UndeclaredCapability verifies that a package with os.ReadFile
// but no manifest produces a CAP001 violation.
func TestAnalyzeDir_UndeclaredCapability(t *testing.T) {
	dir := t.TempDir()
	writeGoFile(t, dir, "foo.go", `package foo
import "os"
func f() { os.ReadFile("x") }
`)
	// No manifest written → zero declared capabilities.

	result, err := AnalyzeDir(dir)
	if err != nil {
		t.Fatalf("AnalyzeDir: %v", err)
	}
	if result.Passed() {
		t.Error("expected CAP001 violation, but analysis passed")
	}
	hasCAP001 := false
	for _, v := range result.Violations {
		if v.Code == "CAP001" {
			hasCAP001 = true
			break
		}
	}
	if !hasCAP001 {
		t.Errorf("expected CAP001 violation, got %v", result.Violations)
	}
}

// TestAnalyzeDir_BannedConstructFails verifies that a restricted construct still
// produces CAP002 unless the package explicitly opts in through both manifest
// exception metadata and matching ffi capabilities.
func TestAnalyzeDir_BannedConstructFails(t *testing.T) {
	dir := t.TempDir()
	writeGoFile(t, dir, "foo.go", `package foo
import "C"
`)

	result, err := AnalyzeDir(dir)
	if err != nil {
		t.Fatalf("AnalyzeDir: %v", err)
	}
	if result.Passed() {
		t.Error("expected CAP002 violation for import C, but analysis passed")
	}
	hasCAP002 := false
	for _, v := range result.Violations {
		if v.Code == "CAP002" {
			hasCAP002 = true
			break
		}
	}
	if !hasCAP002 {
		t.Errorf("expected CAP002 violation, got %v", result.Violations)
	}
}

func TestAnalyzeDir_CGoAllowedWithExplicitOptIn(t *testing.T) {
	dir := t.TempDir()
	writeGoFile(t, dir, "foo.go", `package foo
import "C"
`)
	writeManifest(t, dir, `{
		"version": 1,
		"package": "go/test-pkg",
		"capabilities": [
			{
				"category": "ffi",
				"action": "call",
				"target": "libexample",
				"justification": "Calls reviewed native code."
			}
		],
		"banned_construct_exceptions": [
			{
				"construct": "import \"C\"",
				"language": "go",
				"justification": "Bridge to reviewed native implementation."
			}
		],
		"justification": "Reviewed FFI package."
	}`)

	result, err := AnalyzeDir(dir)
	if err != nil {
		t.Fatalf("AnalyzeDir: %v", err)
	}
	if !result.Passed() {
		t.Fatalf("expected explicit CGo opt-in to pass, got violations: %v", result.Violations)
	}
	if len(result.Banned) != 0 {
		t.Fatalf("expected no remaining banned constructs, got %v", result.Banned)
	}
}

func TestAnalyzeDir_CGoExceptionWithoutFFICapabilityFails(t *testing.T) {
	dir := t.TempDir()
	writeGoFile(t, dir, "foo.go", `package foo
import "C"
`)
	writeManifest(t, dir, `{
		"version": 1,
		"package": "go/test-pkg",
		"capabilities": [],
		"banned_construct_exceptions": [
			{
				"construct": "import \"C\"",
				"language": "go",
				"justification": "Bridge to reviewed native implementation."
			}
		],
		"justification": "Missing ffi capability."
	}`)

	result, err := AnalyzeDir(dir)
	if err != nil {
		t.Fatalf("AnalyzeDir: %v", err)
	}

	if result.Passed() {
		t.Fatal("expected missing ffi capability to fail")
	}
	if len(result.Violations) == 0 || result.Violations[0].Code != "CAP002" {
		t.Fatalf("expected CAP002 violation, got %v", result.Violations)
	}
	if got := result.Violations[0].Message; !strings.Contains(got, "ffi:call:*") {
		t.Fatalf("expected ffi:call hint in %q", got)
	}
}

func TestAnalyzeDir_PluginOpenAllowedWithExplicitOptIn(t *testing.T) {
	dir := t.TempDir()
	writeGoFile(t, dir, "foo.go", `package foo
import "plugin"
func load(path string) {
	_, _ = plugin.Open(path)
}
`)
	writeManifest(t, dir, `{
		"version": 1,
		"package": "go/test-pkg",
		"capabilities": [
			{
				"category": "ffi",
				"action": "load",
				"target": "barcode-renderer",
				"justification": "Loads a reviewed native bridge."
			}
		],
		"banned_construct_exceptions": [
			{
				"construct": "plugin.Open()",
				"language": "go",
				"justification": "Loads a reviewed native bridge."
			}
		],
		"justification": "Reviewed plugin-based FFI package."
	}`)

	result, err := AnalyzeDir(dir)
	if err != nil {
		t.Fatalf("AnalyzeDir: %v", err)
	}
	if !result.Passed() {
		t.Fatalf("expected explicit plugin.Open opt-in to pass, got violations: %v", result.Violations)
	}
}

func TestAnalyzeDir_UnsafePointerStillFailsEvenWithException(t *testing.T) {
	dir := t.TempDir()
	writeGoFile(t, dir, "foo.go", `package foo
import "unsafe"
func f(x int) uintptr {
	return uintptr(unsafe.Pointer(&x))
}
`)
	writeManifest(t, dir, `{
		"version": 1,
		"package": "go/test-pkg",
		"capabilities": [
			{
				"category": "ffi",
				"action": "call",
				"target": "libexample",
				"justification": "Calls reviewed native code."
			}
		],
		"banned_construct_exceptions": [
			{
				"construct": "unsafe.Pointer",
				"language": "go",
				"justification": "Attempted exception should not be honored."
			}
		],
		"justification": "Unsafe should stay blocked."
	}`)

	result, err := AnalyzeDir(dir)
	if err != nil {
		t.Fatalf("AnalyzeDir: %v", err)
	}
	if result.Passed() {
		t.Fatal("expected unsafe.Pointer to remain banned")
	}
	hasCAP002 := false
	for _, v := range result.Violations {
		if v.Code == "CAP002" {
			hasCAP002 = true
			break
		}
	}
	if !hasCAP002 {
		t.Fatalf("expected CAP002 violation, got %v", result.Violations)
	}
}

// TestAnalyzeDir_NoGoFiles verifies that an empty directory produces no violations
// and no error.
func TestAnalyzeDir_NoGoFiles(t *testing.T) {
	dir := t.TempDir()
	result, err := AnalyzeDir(dir)
	if err != nil {
		t.Fatalf("AnalyzeDir: %v", err)
	}
	if !result.Passed() {
		t.Errorf("empty directory should pass, got violations: %v", result.Violations)
	}
}

// TestAnalyzeDir_NonexistentDir verifies that a nonexistent directory returns an error.
func TestAnalyzeDir_NonexistentDir(t *testing.T) {
	_, err := AnalyzeDir("/nonexistent/path/that/does/not/exist")
	if err == nil {
		t.Error("expected error for nonexistent directory, got nil")
	}
}

// TestAnalyzeDir_SkipsGenCapabilities verifies that gen_capabilities.go is
// not analyzed, even if it contains raw OS calls.
func TestAnalyzeDir_SkipsGenCapabilities(t *testing.T) {
	dir := t.TempDir()
	// gen_capabilities.go with a raw os.ReadFile (marked nolint:cap in real generated files,
	// but here we test that the file is skipped entirely regardless).
	writeGoFile(t, dir, "gen_capabilities.go", `package foo
import "os"
func genFunc() { os.ReadFile("x") } //nolint:cap
`)
	// No manifest → if the file were analyzed, we'd get a CAP001 violation.

	result, err := AnalyzeDir(dir)
	if err != nil {
		t.Fatalf("AnalyzeDir: %v", err)
	}
	if !result.Passed() {
		t.Errorf("gen_capabilities.go should be skipped, got violations: %v", result.Violations)
	}
}

// TestAnalyzeDir_VerboseDetected verifies that AnalysisResult.Detected is populated
// even when the analysis passes (for --verbose support).
func TestAnalyzeDir_VerboseDetected(t *testing.T) {
	dir := t.TempDir()
	writeGoFile(t, dir, "foo.go", `package foo
import "os"
func f() { os.ReadFile("x") }
`)
	writeManifestInDir(t, dir, `{
		"version": 1,
		"package": "go/test-pkg",
		"capabilities": [
			{"category": "fs", "action": "read", "target": "*", "justification": "reads files"}
		],
		"justification": "reads files"
	}`)

	result, err := AnalyzeDir(dir)
	if err != nil {
		t.Fatalf("AnalyzeDir: %v", err)
	}
	if result.Passed() && len(result.Detected) == 0 {
		t.Error("expected Detected to be populated even on passing run")
	}
}

// TestAnalyzeDir_EmptyManifest verifies that a manifest with empty capabilities
// causes any OS call to produce a CAP001 violation.
func TestAnalyzeDir_EmptyManifest(t *testing.T) {
	dir := t.TempDir()
	writeGoFile(t, dir, "foo.go", `package foo
import "time"
func f() { time.Now() }
`)
	writeManifestInDir(t, dir, `{
		"version": 1,
		"package": "go/test-pkg",
		"capabilities": [],
		"justification": "Pure computation."
	}`)

	result, err := AnalyzeDir(dir)
	if err != nil {
		t.Fatalf("AnalyzeDir: %v", err)
	}
	if result.Passed() {
		t.Error("expected CAP001 for time.Now with empty manifest, but analysis passed")
	}
}

// TestAnalyzeDir_Passed reports true for a package with no OS calls and no manifest.
func TestAnalyzeDir_Passed_NoCallsNoManifest(t *testing.T) {
	dir := t.TempDir()
	writeGoFile(t, dir, "foo.go", `package foo

func Add(a, b int) int { return a + b }
`)

	result, err := AnalyzeDir(dir)
	if err != nil {
		t.Fatalf("AnalyzeDir: %v", err)
	}
	if !result.Passed() {
		t.Errorf("pure computation should pass with no manifest, got %v", result.Violations)
	}
}

// ── AnalysisResult tests ──────────────────────────────────────────────────────

// TestAnalysisResult_Passed verifies that Passed() returns true iff Violations is empty.
func TestAnalysisResult_Passed(t *testing.T) {
	r := &AnalysisResult{}
	if !r.Passed() {
		t.Error("empty violations should mean Passed() == true")
	}
	r.Violations = append(r.Violations, Violation{Code: "CAP001", Message: "test"})
	if r.Passed() {
		t.Error("non-empty violations should mean Passed() == false")
	}
}

// TestViolation_Format verifies that Violation.Format() returns the message.
func TestViolation_Format(t *testing.T) {
	v := Violation{Code: "CAP001", Message: "foo.go:1: [CAP001] undeclared capability"}
	if v.Format() != v.Message {
		t.Errorf("Format() = %q, want %q", v.Format(), v.Message)
	}
}

// TestNewViolationCAP001 verifies the format of CAP001 violations.
func TestNewViolationCAP001(t *testing.T) {
	d := DetectedCapability{File: "foo.go", Line: 42, Capability: "fs:read:*", Evidence: "os.ReadFile call"}
	v := newViolationCAP001(d)
	if v.Code != "CAP001" {
		t.Errorf("expected code CAP001, got %q", v.Code)
	}
	if !strings.Contains(v.Message, "foo.go:42") {
		t.Errorf("expected file:line in message, got %q", v.Message)
	}
	if !strings.Contains(v.Message, "fs:read:*") {
		t.Errorf("expected capability in message, got %q", v.Message)
	}
}

// TestNewViolationCAP002 verifies the format of CAP002 violations.
func TestNewViolationCAP002(t *testing.T) {
	b := BannedConstruct{File: "bar.go", Line: 7, Kind: "unsafe.Pointer conversion"}
	v := newViolationCAP002(b)
	if v.Code != "CAP002" {
		t.Errorf("expected code CAP002, got %q", v.Code)
	}
	if !strings.Contains(v.Message, "bar.go:7") {
		t.Errorf("expected file:line in message, got %q", v.Message)
	}
	if !strings.Contains(v.Message, "unsafe.Pointer") {
		t.Errorf("expected kind in message, got %q", v.Message)
	}
}

// ── Rules table tests ─────────────────────────────────────────────────────────

// TestImportRulesNotEmpty verifies that ImportRules has entries.
func TestImportRulesNotEmpty(t *testing.T) {
	if len(ImportRules) == 0 {
		t.Error("ImportRules must not be empty")
	}
}

// TestCallRulesNotEmpty verifies that CallRules has entries.
func TestCallRulesNotEmpty(t *testing.T) {
	if len(CallRules) == 0 {
		t.Error("CallRules must not be empty")
	}
}

// TestCallRules_AllHaveEvidence verifies that every CallRule has non-empty Evidence.
func TestCallRules_AllHaveEvidence(t *testing.T) {
	for _, rule := range CallRules {
		if rule.Evidence == "" {
			t.Errorf("CallRule for %s.%s has empty Evidence", rule.ImportPath, rule.FunctionName)
		}
	}
}
