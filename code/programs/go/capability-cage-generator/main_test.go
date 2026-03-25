// Tests for the capability-cage-generator program.
//
// These tests cover:
//   - generateSource: correct Go code emitted for various manifests
//   - goPackageName: correct derivation from package field and .go files
//   - processManifest: reads JSON, writes gen_capabilities.go
//   - categoryConst / actionConst: correct constant name mapping
//   - --all mode: processes multiple packages
//   - --dry-run mode: no files written
//   - Error handling: invalid JSON, non-Go packages, missing files
package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// ─────────────────────────────────────────────────────────────────────────────
// categoryConst and actionConst
// ─────────────────────────────────────────────────────────────────────────────

func TestCategoryConst_KnownValues(t *testing.T) {
	cases := []struct{ input, expected string }{
		{"fs", "cage.CategoryFS"},
		{"net", "cage.CategoryNet"},
		{"proc", "cage.CategoryProc"},
		{"env", "cage.CategoryEnv"},
		{"ffi", "cage.CategoryFFI"},
		{"time", "cage.CategoryTime"},
		{"stdin", "cage.CategoryStdin"},
		{"stdout", "cage.CategoryStdout"},
	}
	for _, c := range cases {
		got := categoryConst(c.input)
		if got != c.expected {
			t.Errorf("categoryConst(%q) = %q, want %q", c.input, got, c.expected)
		}
	}
}

func TestCategoryConst_UnknownValue(t *testing.T) {
	got := categoryConst("unknown")
	if !strings.Contains(got, "unknown") {
		t.Errorf("expected unknown category to appear in output, got %q", got)
	}
}

func TestActionConst_KnownValues(t *testing.T) {
	cases := []struct{ input, expected string }{
		{"read", "cage.ActionRead"},
		{"write", "cage.ActionWrite"},
		{"create", "cage.ActionCreate"},
		{"delete", "cage.ActionDelete"},
		{"list", "cage.ActionList"},
		{"connect", "cage.ActionConnect"},
		{"listen", "cage.ActionListen"},
		{"dns", "cage.ActionDNS"},
		{"exec", "cage.ActionExec"},
		{"fork", "cage.ActionFork"},
		{"signal", "cage.ActionSignal"},
		{"call", "cage.ActionCall"},
		{"load", "cage.ActionLoad"},
		{"sleep", "cage.ActionSleep"},
	}
	for _, c := range cases {
		got := actionConst(c.input)
		if got != c.expected {
			t.Errorf("actionConst(%q) = %q, want %q", c.input, got, c.expected)
		}
	}
}

func TestActionConst_UnknownValue(t *testing.T) {
	got := actionConst("unknown")
	if !strings.Contains(got, "unknown") {
		t.Errorf("expected unknown action to appear in output, got %q", got)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// goPackageName
// ─────────────────────────────────────────────────────────────────────────────

func TestGoPackageName_FromPackageField(t *testing.T) {
	tmp := t.TempDir()
	manifestPath := filepath.Join(tmp, "required_capabilities.json")

	cases := []struct{ pkgField, expected string }{
		{"go/verilog-lexer", "veriloglexer"},
		{"go/sql-parser", "sqlparser"},
		{"go/json-value", "jsonvalue"},
		{"go/capability-cage", "capabilitycage"},
	}
	for _, c := range cases {
		got, err := goPackageName(manifestPath, c.pkgField)
		if err != nil {
			t.Errorf("goPackageName(%q): unexpected error: %v", c.pkgField, err)
			continue
		}
		if got != c.expected {
			t.Errorf("goPackageName(%q) = %q, want %q", c.pkgField, got, c.expected)
		}
	}
}

func TestGoPackageName_FromGoFile(t *testing.T) {
	tmp := t.TempDir()
	// Write a .go file with a package declaration.
	goFile := filepath.Join(tmp, "lexer.go")
	_ = os.WriteFile(goFile, []byte("package veriloglexer\n\nfunc Foo() {}\n"), 0o644) //nolint:cap
	manifestPath := filepath.Join(tmp, "required_capabilities.json")

	got, err := goPackageName(manifestPath, "go/verilog-lexer")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if got != "veriloglexer" {
		t.Errorf("expected 'veriloglexer', got %q", got)
	}
}

func TestGoPackageName_SkipsTestFiles(t *testing.T) {
	tmp := t.TempDir()
	// Only a test file — should fall back to package field.
	testFile := filepath.Join(tmp, "lexer_test.go")
	_ = os.WriteFile(testFile, []byte("package veriloglexer_test\n"), 0o644) //nolint:cap
	manifestPath := filepath.Join(tmp, "required_capabilities.json")

	got, err := goPackageName(manifestPath, "go/verilog-lexer")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if got != "veriloglexer" {
		t.Errorf("expected 'veriloglexer', got %q", got)
	}
}

func TestGoPackageName_InvalidPackageField(t *testing.T) {
	tmp := t.TempDir()
	manifestPath := filepath.Join(tmp, "required_capabilities.json")

	_, err := goPackageName(manifestPath, "no-slash-here")
	if err == nil {
		t.Error("expected error for invalid package field")
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// generateSource — empty capabilities
// ─────────────────────────────────────────────────────────────────────────────

func TestGenerateSource_EmptyCapabilities(t *testing.T) {
	tmp := t.TempDir()
	manifestPath := filepath.Join(tmp, "required_capabilities.json")

	mf := &manifestJSON{
		Package:      "go/test-pkg",
		Capabilities: []capabilityJSON{},
	}
	src, err := generateSource(manifestPath, mf)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	// Must contain the auto-generated header.
	if !strings.Contains(src, "DO NOT EDIT") {
		t.Error("expected 'DO NOT EDIT' in header")
	}
	// Must import capability-cage.
	if !strings.Contains(src, `cage "github.com/adhithyan15/coding-adventures/code/packages/go/capability-cage"`) {
		t.Error("expected capability-cage import")
	}
	// Must use EmptyManifest for zero-capability packages.
	if !strings.Contains(src, "cage.EmptyManifest") {
		t.Error("expected EmptyManifest for empty capabilities")
	}
	// Must NOT use NewManifest for empty capabilities.
	if strings.Contains(src, "NewManifest") {
		t.Error("did not expect NewManifest for empty capabilities")
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// generateSource — non-empty capabilities
// ─────────────────────────────────────────────────────────────────────────────

func TestGenerateSource_SingleCapability(t *testing.T) {
	tmp := t.TempDir()
	manifestPath := filepath.Join(tmp, "required_capabilities.json")

	mf := &manifestJSON{
		Package: "go/verilog-lexer",
		Capabilities: []capabilityJSON{
			{
				Category:      "fs",
				Action:        "read",
				Target:        "*",
				Justification: "Reads grammar files at startup.",
			},
		},
	}

	// Write a go file so package name can be read.
	_ = os.WriteFile(filepath.Join(tmp, "lexer.go"),
		[]byte("package veriloglexer\n"), 0o644) //nolint:cap

	src, err := generateSource(manifestPath, mf)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	// Package declaration.
	if !strings.Contains(src, "package veriloglexer") {
		t.Error("expected package veriloglexer")
	}
	// Import.
	if !strings.Contains(src, "cage \"github.com/adhithyan15") {
		t.Error("expected cage import")
	}
	// NewManifest call.
	if !strings.Contains(src, "cage.NewManifest") {
		t.Error("expected cage.NewManifest")
	}
	// Category constant.
	if !strings.Contains(src, "cage.CategoryFS") {
		t.Error("expected cage.CategoryFS")
	}
	// Action constant.
	if !strings.Contains(src, "cage.ActionRead") {
		t.Error("expected cage.ActionRead")
	}
	// Target.
	if !strings.Contains(src, `"*"`) {
		t.Error("expected target *")
	}
	// Justification.
	if !strings.Contains(src, "Reads grammar files at startup.") {
		t.Error("expected justification text")
	}
}

func TestGenerateSource_MultipleCapabilities(t *testing.T) {
	tmp := t.TempDir()
	manifestPath := filepath.Join(tmp, "required_capabilities.json")

	mf := &manifestJSON{
		Package: "go/brainfuck",
		Capabilities: []capabilityJSON{
			{Category: "stdin", Action: "read", Target: "*",
				Justification: "Reads user input."},
			{Category: "stdout", Action: "write", Target: "*",
				Justification: "Writes output."},
		},
	}

	src, err := generateSource(manifestPath, mf)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if !strings.Contains(src, "cage.CategoryStdin") {
		t.Error("expected cage.CategoryStdin")
	}
	if !strings.Contains(src, "cage.CategoryStdout") {
		t.Error("expected cage.CategoryStdout")
	}
	if !strings.Contains(src, "cage.ActionRead") {
		t.Error("expected cage.ActionRead")
	}
	if !strings.Contains(src, "cage.ActionWrite") {
		t.Error("expected cage.ActionWrite")
	}
}

func TestGenerateSource_JustificationEscaping(t *testing.T) {
	tmp := t.TempDir()
	manifestPath := filepath.Join(tmp, "required_capabilities.json")

	mf := &manifestJSON{
		Package: "go/test-pkg",
		Capabilities: []capabilityJSON{
			{Category: "fs", Action: "read", Target: "*",
				Justification: `Reads "grammar" files via os.ReadFile.`},
		},
	}

	src, err := generateSource(manifestPath, mf)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	// The justification with quotes should be properly escaped in the Go source.
	if !strings.Contains(src, `Reads \"grammar\" files via os.ReadFile.`) {
		t.Error("expected escaped quotes in justification")
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// processManifest — file I/O
// ─────────────────────────────────────────────────────────────────────────────

func TestProcessManifest_WritesGenCapabilities(t *testing.T) {
	tmp := t.TempDir()
	manifestPath := filepath.Join(tmp, "required_capabilities.json")
	goFilePath := filepath.Join(tmp, "lexer.go")

	// Write a .go file so the package name can be read.
	_ = os.WriteFile(goFilePath, []byte("package veriloglexer\n"), 0o644) //nolint:cap

	manifest := `{
		"$schema": "https://example.com/schema",
		"version": 1,
		"package": "go/verilog-lexer",
		"capabilities": [
			{
				"category": "fs",
				"action": "read",
				"target": "*",
				"justification": "Reads token grammar files."
			}
		],
		"justification": "Reads grammar files."
	}`
	_ = os.WriteFile(manifestPath, []byte(manifest), 0o644) //nolint:cap

	err := processManifest(manifestPath, false)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	// gen_capabilities.go should exist.
	outPath := filepath.Join(tmp, "gen_capabilities.go")
	data, err := os.ReadFile(outPath) //nolint:cap
	if err != nil {
		t.Fatalf("gen_capabilities.go not written: %v", err)
	}

	src := string(data)
	if !strings.Contains(src, "DO NOT EDIT") {
		t.Error("expected DO NOT EDIT header")
	}
	if !strings.Contains(src, "package veriloglexer") {
		t.Error("expected package declaration")
	}
	if !strings.Contains(src, "cage.CategoryFS") {
		t.Error("expected cage.CategoryFS")
	}
}

func TestProcessManifest_DryRunDoesNotWriteFile(t *testing.T) {
	tmp := t.TempDir()
	manifestPath := filepath.Join(tmp, "required_capabilities.json")

	manifest := `{
		"version": 1,
		"package": "go/test-pkg",
		"capabilities": [],
		"justification": "Pure computation."
	}`
	_ = os.WriteFile(manifestPath, []byte(manifest), 0o644) //nolint:cap

	err := processManifest(manifestPath, true /* dryRun */)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	// gen_capabilities.go should NOT be written in dry-run mode.
	outPath := filepath.Join(tmp, "gen_capabilities.go")
	if _, err := os.Stat(outPath); !os.IsNotExist(err) { //nolint:cap
		t.Error("dry-run should not write gen_capabilities.go")
	}
}

func TestProcessManifest_SkipsNonGoPackages(t *testing.T) {
	tmp := t.TempDir()
	manifestPath := filepath.Join(tmp, "required_capabilities.json")

	manifest := `{
		"version": 1,
		"package": "python/verilog-lexer",
		"capabilities": [],
		"justification": "Python package."
	}`
	_ = os.WriteFile(manifestPath, []byte(manifest), 0o644) //nolint:cap

	err := processManifest(manifestPath, false)
	if err != nil {
		t.Fatalf("unexpected error for non-Go package: %v", err)
	}

	// gen_capabilities.go should NOT be written for non-Go packages.
	outPath := filepath.Join(tmp, "gen_capabilities.go")
	if _, err := os.Stat(outPath); !os.IsNotExist(err) { //nolint:cap
		t.Error("should not write gen_capabilities.go for non-Go package")
	}
}

func TestProcessManifest_InvalidJSON(t *testing.T) {
	tmp := t.TempDir()
	manifestPath := filepath.Join(tmp, "required_capabilities.json")
	_ = os.WriteFile(manifestPath, []byte("not json {{{"), 0o644) //nolint:cap

	err := processManifest(manifestPath, false)
	if err == nil {
		t.Error("expected error for invalid JSON")
	}
}

func TestProcessManifest_MissingFile(t *testing.T) {
	err := processManifest("/nonexistent/path/required_capabilities.json", false)
	if err == nil {
		t.Error("expected error for missing file")
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// processAll
// ─────────────────────────────────────────────────────────────────────────────

func TestProcessAll_ProcessesGoPackages(t *testing.T) {
	// Create a fake repo structure.
	tmp := t.TempDir()
	pkgDir := filepath.Join(tmp, "code", "packages", "go", "test-pkg")
	_ = os.MkdirAll(pkgDir, 0o755) //nolint:cap

	// Write a .go file for package name.
	_ = os.WriteFile(filepath.Join(pkgDir, "main.go"), []byte("package testpkg\n"), 0o644) //nolint:cap

	manifest := `{
		"version": 1,
		"package": "go/test-pkg",
		"capabilities": [],
		"justification": "Pure computation."
	}`
	manifestPath := filepath.Join(pkgDir, "required_capabilities.json")
	_ = os.WriteFile(manifestPath, []byte(manifest), 0o644) //nolint:cap

	err := processAll(tmp, false)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	outPath := filepath.Join(pkgDir, "gen_capabilities.go")
	if _, err := os.Stat(outPath); os.IsNotExist(err) { //nolint:cap
		t.Error("expected gen_capabilities.go to be written")
	}
}

func TestProcessAll_ReturnsErrorOnBadManifest(t *testing.T) {
	// Create a fake repo structure with an invalid manifest.
	tmp := t.TempDir()
	pkgDir := filepath.Join(tmp, "code", "packages", "go", "bad-pkg")
	_ = os.MkdirAll(pkgDir, 0o755) //nolint:cap
	manifestPath := filepath.Join(pkgDir, "required_capabilities.json")
	_ = os.WriteFile(manifestPath, []byte("not valid json"), 0o644) //nolint:cap

	err := processAll(tmp, false)
	if err == nil {
		t.Error("expected error when manifest is invalid JSON")
	}
}

func TestProcessAll_EmptyDirectory(t *testing.T) {
	tmp := t.TempDir()
	_ = os.MkdirAll(filepath.Join(tmp, "code", "packages", "go"), 0o755) //nolint:cap

	// No packages — should not error.
	err := processAll(tmp, false)
	if err != nil {
		t.Fatalf("unexpected error for empty directory: %v", err)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// isRepoRoot
// ─────────────────────────────────────────────────────────────────────────────

func TestIsRepoRoot_TrueForValidRoot(t *testing.T) {
	tmp := t.TempDir()
	_ = os.MkdirAll(filepath.Join(tmp, "code", "packages"), 0o755) //nolint:cap

	if !isRepoRoot(tmp) {
		t.Error("expected isRepoRoot to return true for directory with code/packages/")
	}
}

func TestIsRepoRoot_FalseForRandom(t *testing.T) {
	tmp := t.TempDir()
	if isRepoRoot(tmp) {
		t.Error("expected isRepoRoot to return false for directory without code/packages/")
	}
}
