package main

// coverage_test.go contains additional tests to exercise uncovered code paths.
// These tests target specific branches in analyzer.go, banned.go, and manifest.go
// that the main test files did not reach.

import (
	"go/ast"
	"go/parser"
	"go/token"
	"os"
	"path/filepath"
	"testing"
)

// ── activeImports edge cases ──────────────────────────────────────────────────

// TestActiveImports_BlankImport verifies that blank imports (_) are excluded
// from the import map, since they contribute no usable identifier.
func TestActiveImports_BlankImport(t *testing.T) {
	src := `package foo
import _ "net/http"
`
	fset := token.NewFileSet()
	f, err := parser.ParseFile(fset, "foo.go", src, parser.ParseComments)
	if err != nil {
		t.Fatal(err)
	}
	imports := activeImports(f)
	if _, ok := imports["net/http"]; ok {
		t.Error("blank import should not appear in activeImports result")
	}
}

// TestActiveImports_DotImport verifies that dot imports (.) are excluded
// from the import map, since their symbols are scattered into the namespace.
func TestActiveImports_DotImport(t *testing.T) {
	src := `package foo
import . "fmt"
`
	fset := token.NewFileSet()
	f, err := parser.ParseFile(fset, "foo.go", src, parser.ParseComments)
	if err != nil {
		t.Fatal(err)
	}
	imports := activeImports(f)
	if _, ok := imports["fmt"]; ok {
		t.Error("dot import should not appear in activeImports result")
	}
}

// TestActiveImports_ExplicitAlias verifies that explicit import aliases are
// returned as the local name (e.g., import myfmt "fmt" → "fmt":"myfmt").
func TestActiveImports_ExplicitAlias(t *testing.T) {
	src := `package foo
import myfmt "fmt"
`
	fset := token.NewFileSet()
	f, err := parser.ParseFile(fset, "foo.go", src, parser.ParseComments)
	if err != nil {
		t.Fatal(err)
	}
	imports := activeImports(f)
	if imports["fmt"] != "myfmt" {
		t.Errorf("expected alias 'myfmt', got %q", imports["fmt"])
	}
}

// ── importLocalName edge cases ────────────────────────────────────────────────

// TestImportLocalName_AliasedImport verifies that importLocalName returns the
// explicit alias when one is present.
func TestImportLocalName_AliasedImport(t *testing.T) {
	src := `package foo
import myos "os"
`
	fset := token.NewFileSet()
	f, err := parser.ParseFile(fset, "foo.go", src, parser.ParseComments)
	if err != nil {
		t.Fatal(err)
	}
	got := importLocalName(f, "os")
	if got != "myos" {
		t.Errorf("expected alias 'myos', got %q", got)
	}
}

// TestImportLocalName_NotPresent verifies that importLocalName returns the
// last path segment when the import is not found.
func TestImportLocalName_NotPresent(t *testing.T) {
	src := `package foo
`
	fset := token.NewFileSet()
	f, err := parser.ParseFile(fset, "foo.go", src, parser.ParseComments)
	if err != nil {
		t.Fatal(err)
	}
	// "net/http" is not imported; fallback should return "http"
	got := importLocalName(f, "net/http")
	if got != "http" {
		t.Errorf("expected last segment 'http', got %q", got)
	}
}

// ── importLine edge cases ─────────────────────────────────────────────────────

// TestImportLine_NotFound verifies that importLine returns 0 when the import
// is not present in the file.
func TestImportLine_NotFound(t *testing.T) {
	src := `package foo
import "fmt"
`
	fset := token.NewFileSet()
	f, err := parser.ParseFile(fset, "foo.go", src, parser.ParseComments)
	if err != nil {
		t.Fatal(err)
	}
	line := importLine(fset, f, "os") // "os" not imported
	if line != 0 {
		t.Errorf("expected 0 for missing import, got %d", line)
	}
}

// ── DeclaredCapabilityList ────────────────────────────────────────────────────

// TestDeclaredCapabilityList verifies that DeclaredCapabilityList returns the
// expected capability strings from a manifest.
func TestDeclaredCapabilityList(t *testing.T) {
	dir := t.TempDir()
	if err := os.WriteFile(filepath.Join(dir, "required_capabilities.json"), []byte(`{
		"version": 1,
		"package": "go/test",
		"capabilities": [
			{"category": "fs", "action": "read", "target": "*", "justification": "reads files"}
		],
		"justification": "reads files"
	}`), 0o600); err != nil {
		t.Fatal(err)
	}

	list, err := DeclaredCapabilityList(dir)
	if err != nil {
		t.Fatalf("DeclaredCapabilityList: %v", err)
	}
	if len(list) == 0 {
		t.Error("expected at least one capability in list")
	}
	found := false
	for _, cap := range list {
		if cap == "fs:read:*" {
			found = true
		}
	}
	if !found {
		t.Errorf("expected fs:read:* in list, got %v", list)
	}
}

// TestDeclaredCapabilityList_NoManifest verifies that no error is returned when
// there is no manifest file (returns empty list).
func TestDeclaredCapabilityList_NoManifest(t *testing.T) {
	dir := t.TempDir()
	list, err := DeclaredCapabilityList(dir)
	if err != nil {
		t.Fatalf("DeclaredCapabilityList: %v", err)
	}
	if len(list) != 0 {
		t.Errorf("expected empty list for missing manifest, got %v", list)
	}
}


// ── detectSpecialCalls extra paths ────────────────────────────────────────────

// TestDetect_Stderr_Write verifies that os.Stderr.Write is detected as stdout:write:*.
// (Stderr writes are tracked under stdout:write:* per the spec.)
func TestDetect_Stderr_Write(t *testing.T) {
	fset, files := parseFiles(t,
		"foo.go", `package foo
import "os"
func f() { os.Stderr.Write([]byte("error")) }
`)
	caps := DetectCapabilities(fset, files)
	if !containsCap(caps, "stdout:write:*") {
		t.Errorf("expected stdout:write:* for os.Stderr.Write, got %v", caps)
	}
}

// TestDetect_OsStdoutWriteString verifies that os.Stdout.WriteString is detected.
func TestDetect_OsStdoutWriteString(t *testing.T) {
	fset, files := parseFiles(t,
		"foo.go", `package foo
import "os"
func f() { os.Stdout.WriteString("hello") }
`)
	caps := DetectCapabilities(fset, files)
	if !containsCap(caps, "stdout:write:*") {
		t.Errorf("expected stdout:write:* for os.Stdout.WriteString, got %v", caps)
	}
}

// TestDetect_FmtFprintfOsStderr verifies that fmt.Fprintf(os.Stderr, ...) is detected.
func TestDetect_FmtFprintfOsStderr(t *testing.T) {
	fset, files := parseFiles(t,
		"foo.go", `package foo
import (
	"fmt"
	"os"
)
func f() { fmt.Fprintf(os.Stderr, "error: %v", "oops") }
`)
	caps := DetectCapabilities(fset, files)
	if !containsCap(caps, "stdout:write:*") {
		t.Errorf("expected stdout:write:* for fmt.Fprintf(os.Stderr,...), got %v", caps)
	}
}

// TestDetect_FSWrite_OsRename verifies that os.Rename is detected as fs:write:*.
func TestDetect_FSWrite_OsRename(t *testing.T) {
	fset, files := parseFiles(t,
		"foo.go", `package foo
import "os"
func f() { os.Rename("old", "new") }
`)
	caps := DetectCapabilities(fset, files)
	if !containsCap(caps, "fs:write:*") {
		t.Errorf("expected fs:write:*, got %v", caps)
	}
}

// TestDetect_FSList_OsLstat verifies that os.Lstat is detected as fs:list:*.
func TestDetect_FSList_OsLstat(t *testing.T) {
	fset, files := parseFiles(t,
		"foo.go", `package foo
import "os"
func f() { os.Lstat("x") }
`)
	caps := DetectCapabilities(fset, files)
	if !containsCap(caps, "fs:list:*") {
		t.Errorf("expected fs:list:*, got %v", caps)
	}
}

// ── AnalyzeDir with parse error ───────────────────────────────────────────────

// TestAnalyzeDir_UnparseableFileSkipped verifies that a .go file with a
// syntax error is skipped gracefully rather than aborting the analysis.
func TestAnalyzeDir_UnparseableFileSkipped(t *testing.T) {
	dir := t.TempDir()
	// Write a valid file and an invalid one.
	writeGoFile(t, dir, "valid.go", `package foo
func Add(a, b int) int { return a + b }
`)
	writeGoFile(t, dir, "invalid.go", `package foo
this is not valid Go {{{
`)

	result, err := AnalyzeDir(dir)
	if err != nil {
		t.Fatalf("AnalyzeDir should not error on parse failures, got: %v", err)
	}
	// The valid file has no OS calls so should pass.
	if !result.Passed() {
		t.Errorf("expected pass with only parse-error file skipped, got %v", result.Violations)
	}
}

// TestAnalyzeDir_BothCapAndBanned verifies that a file with both an undeclared
// capability and a banned construct produces both CAP001 and CAP002 violations.
func TestAnalyzeDir_BothCapAndBanned(t *testing.T) {
	dir := t.TempDir()
	writeGoFile(t, dir, "foo.go", `package foo
import "C"
import "os"
func f() { os.ReadFile("x") }
`)
	// No manifest → os.ReadFile is undeclared (CAP001)
	// import "C" is banned (CAP002)

	result, err := AnalyzeDir(dir)
	if err != nil {
		t.Fatalf("AnalyzeDir: %v", err)
	}
	hasCAP001, hasCAP002 := false, false
	for _, v := range result.Violations {
		if v.Code == "CAP001" {
			hasCAP001 = true
		}
		if v.Code == "CAP002" {
			hasCAP002 = true
		}
	}
	if !hasCAP001 {
		t.Error("expected CAP001 violation")
	}
	if !hasCAP002 {
		t.Error("expected CAP002 violation")
	}
}

// TestAnalyzeDir_ResultDir verifies that AnalysisResult.Dir is the absolute path.
func TestAnalyzeDir_ResultDir(t *testing.T) {
	dir := t.TempDir()
	result, err := AnalyzeDir(dir)
	if err != nil {
		t.Fatalf("AnalyzeDir: %v", err)
	}
	if result.Dir == "" {
		t.Error("expected non-empty Dir in result")
	}
}

// ── buildCapabilitySet ────────────────────────────────────────────────────────

// TestBuildCapabilitySet verifies that the helper deduplicates capability strings.
func TestBuildCapabilitySet(t *testing.T) {
	input := []DetectedCapability{
		{Capability: "fs:read:*"},
		{Capability: "fs:read:*"}, // duplicate
		{Capability: "time:read:*"},
	}
	set := buildCapabilitySet(input)
	if len(set) != 2 {
		t.Errorf("expected 2 unique capabilities, got %d: %v", len(set), set)
	}
	if !set["fs:read:*"] {
		t.Error("expected fs:read:* in set")
	}
	if !set["time:read:*"] {
		t.Error("expected time:read:* in set")
	}
}

// ── isNolintLine ──────────────────────────────────────────────────────────────

// TestIsNolintLine_NoComment verifies that a line with no comments returns false.
func TestIsNolintLine_NoComment(t *testing.T) {
	emptyIndex := make(map[int][]*ast.CommentGroup)
	if isNolintLine(emptyIndex, 5) {
		t.Error("empty comment index should not nolint any line")
	}
}

// ── isOperationsCall ──────────────────────────────────────────────────────────

// TestIsOperationsCall_OpThreeLevel verifies detection of op.File.ReadFile pattern.
func TestIsOperationsCall_OpThreeLevel(t *testing.T) {
	src := `package foo
func f(op *Op) { op.File.ReadFile("x") }
`
	fset := token.NewFileSet()
	f, err := parser.ParseFile(fset, "foo.go", src, parser.ParseComments)
	if err != nil {
		t.Fatal(err)
	}
	// The function should detect no capabilities because the call goes through op.
	files := map[string]*ast.File{"foo.go": f}
	caps := DetectCapabilities(fset, files)
	if containsCap(caps, "fs:read:*") {
		t.Error("op.File.ReadFile should be exempt from detection")
	}
}
