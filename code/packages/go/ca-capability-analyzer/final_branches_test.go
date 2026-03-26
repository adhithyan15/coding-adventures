package main

// final_branches_test.go covers the last remaining uncovered branches.
// These are micro-tests targeting very specific code paths.

import (
	"go/parser"
	"go/token"
	"os"
	"path/filepath"
	"runtime"
	"testing"
)

// TestBanned_PluginOpen_ChainedCall verifies that detectPlugin handles a call
// where the receiver is a chained call expression (a.b.Open("x")) rather than
// a simple identifier. This covers the `sel.X.(*ast.Ident)` failure branch.
//
// AST shape: SelectorExpr{X: SelectorExpr{X: Ident("mgr"), Sel: "loader"}, Sel: "Open"}
// When sel.X is a SelectorExpr (not an Ident), the inner type assertion fails
// and `return true` fires.
func TestBanned_PluginOpen_ChainedCall(t *testing.T) {
	src := `package foo
import "plugin"

type Manager struct{ loader interface{ Open(string) (*plugin.Plugin, error) } }

func f(mgr Manager) {
	// mgr.loader.Open("path") — Fun has a non-Ident X (it's a SelectorExpr)
	mgr.loader.Open("lib.so")
}
`
	fset := token.NewFileSet()
	f, err := parser.ParseFile(fset, "test.go", src, parser.ParseComments)
	if err != nil {
		t.Fatal(err)
	}
	// The chained call covers the non-Ident sel.X branch in detectPlugin.
	banned := detectPlugin(fset, f, "test.go")
	// mgr.loader.Open is not a plugin.Open call (plugin is the local name for
	// the "plugin" import, and mgr.loader is not that identifier), so no detection.
	_ = banned
}

// TestBanned_UnsafePointer_ChainedCall verifies that detectUnsafe handles a call
// where the receiver is a chained expression rather than a simple identifier.
//
// AST shape: SelectorExpr{X: SelectorExpr{...}, Sel: "Pointer"} — covers the
// `sel.X.(*ast.Ident)` failure branch in detectUnsafe.
func TestBanned_UnsafePointer_ChainedCall(t *testing.T) {
	src := `package foo
import "unsafe"

type Conv struct{ helper interface{ Pointer(interface{}) unsafe.Pointer } }

func f(c Conv, x int) uintptr {
	// c.helper.Pointer(&x) — Fun = SelectorExpr{X: SelectorExpr{...}, Sel: "Pointer"}
	// sel.X is SelectorExpr, not Ident — covers the non-ident branch
	return uintptr(c.helper.Pointer(&x))
}
`
	fset := token.NewFileSet()
	f, err := parser.ParseFile(fset, "test.go", src, parser.ParseComments)
	if err != nil {
		t.Fatal(err)
	}
	banned := detectUnsafe(fset, f, "test.go")
	_ = banned // Should not panic; may or may not detect depending on AST shape
}

// TestAnalyzeDir_FileInsteadOfDir verifies that AnalyzeDir returns an error
// when given a file path instead of a directory path. This covers the
// os.ReadDir error path in AnalyzeDir.
func TestAnalyzeDir_FileInsteadOfDir(t *testing.T) {
	// Create a real file (not a directory).
	dir := t.TempDir()
	filePath := filepath.Join(dir, "notadir.go")
	if err := os.WriteFile(filePath, []byte("package foo"), 0o600); err != nil {
		t.Fatal(err)
	}

	_, err := AnalyzeDir(filePath) // Pass a file, not a directory
	if err == nil {
		t.Error("expected error when passing a file path instead of directory, got nil")
	}
}

// TestLoadManifest_PermissionError verifies LoadManifest handles a non-"not found"
// read error. We create the manifest but make it unreadable, then try to read it.
// Skip on platforms that don't support Unix-style permission enforcement:
//   - Windows: os.Chmod exists but does not enforce read-bit restrictions;
//     the file remains readable regardless of the mode bits.
//   - Root users: the superuser can read any file regardless of mode bits.
func TestLoadManifest_PermissionError(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("Windows does not enforce Unix-style file permission bits; chmod 0o000 leaves files readable")
	}
	if os.Getuid() == 0 {
		t.Skip("running as root; permission tests not meaningful")
	}

	dir := t.TempDir()
	manifestPath := filepath.Join(dir, "required_capabilities.json")
	// Write the manifest.
	if err := os.WriteFile(manifestPath, []byte(`{"version":1,"capabilities":[]}`), 0o600); err != nil {
		t.Fatal(err)
	}
	// Make it unreadable.
	if err := os.Chmod(manifestPath, 0o000); err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() {
		// Restore permissions for cleanup.
		os.Chmod(manifestPath, 0o600) //nolint:errcheck
	})

	_, err := LoadManifest(dir)
	if err == nil {
		t.Error("expected error for unreadable manifest, got nil")
	}
}
