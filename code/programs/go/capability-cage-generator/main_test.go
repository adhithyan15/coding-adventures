// Tests for the capability-cage-generator program.
//
// These tests cover:
//   - generateSource: correct Go code emitted for various manifests
//   - goPackageName: correct derivation from package field and .go files
//   - processManifest: reads JSON, writes gen_capabilities.go
//   - scopeableCategory / validateNoWildcards: wildcard rejection
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
// scopeableCategory
// ─────────────────────────────────────────────────────────────────────────────

func TestScopeableCategory_Scopeable(t *testing.T) {
	cases := []string{"fs", "net", "proc", "env", "ffi"}
	for _, cat := range cases {
		if !scopeableCategory(cat) {
			t.Errorf("scopeableCategory(%q) = false, want true", cat)
		}
	}
}

func TestScopeableCategory_NonScopeable(t *testing.T) {
	cases := []string{"time", "stdin", "stdout"}
	for _, cat := range cases {
		if scopeableCategory(cat) {
			t.Errorf("scopeableCategory(%q) = true, want false", cat)
		}
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// validateNoWildcards
// ─────────────────────────────────────────────────────────────────────────────

func TestValidateNoWildcards_AllowsExactPaths(t *testing.T) {
	mf := &manifestJSON{
		Package: "go/verilog-lexer",
		Capabilities: []capabilityJSON{
			{Category: "fs", Action: "read", Target: "code/grammars/verilog.tokens"},
		},
	}
	if err := validateNoWildcards(mf); err != nil {
		t.Errorf("expected no error for exact path, got: %v", err)
	}
}

func TestValidateNoWildcards_AllowsNonScopeableWildcard(t *testing.T) {
	mf := &manifestJSON{
		Package: "go/brainfuck",
		Capabilities: []capabilityJSON{
			{Category: "stdin", Action: "read", Target: "*"},
			{Category: "stdout", Action: "write", Target: "*"},
			{Category: "time", Action: "sleep", Target: "*"},
		},
	}
	if err := validateNoWildcards(mf); err != nil {
		t.Errorf("expected no error for non-scopeable wildcard, got: %v", err)
	}
}

func TestValidateNoWildcards_RejectsWildcardInFS(t *testing.T) {
	mf := &manifestJSON{
		Package: "go/verilog-lexer",
		Capabilities: []capabilityJSON{
			{Category: "fs", Action: "read", Target: "*"},
		},
	}
	err := validateNoWildcards(mf)
	if err == nil {
		t.Error("expected error for wildcard fs target, got nil")
	}
	if !strings.Contains(err.Error(), "wildcard") {
		t.Errorf("error should mention 'wildcard', got: %v", err)
	}
}

func TestValidateNoWildcards_RejectsWildcardInNet(t *testing.T) {
	mf := &manifestJSON{
		Package: "go/http-client",
		Capabilities: []capabilityJSON{
			{Category: "net", Action: "connect", Target: "*"},
		},
	}
	if err := validateNoWildcards(mf); err == nil {
		t.Error("expected error for wildcard net target, got nil")
	}
}

func TestValidateNoWildcards_RejectsWildcardInEnv(t *testing.T) {
	mf := &manifestJSON{
		Package: "go/config",
		Capabilities: []capabilityJSON{
			{Category: "env", Action: "read", Target: "*"},
		},
	}
	if err := validateNoWildcards(mf); err == nil {
		t.Error("expected error for wildcard env target, got nil")
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
	// Must NOT import the shared capability-cage package.
	if strings.Contains(src, `coding-adventures/code/packages/go/capability-cage`) {
		t.Error("must not import shared capability-cage package")
	}
	// Must use only stdlib imports.
	if !strings.Contains(src, `"fmt"`) {
		t.Error("expected fmt import")
	}
	// Zero capabilities: no capability namespace structs, no Cage type.
	if strings.Contains(src, "type Cage struct{}") {
		t.Error("must not emit 'type Cage struct{}' — Cage is replaced by namespace fields on Operation[T]")
	}
	if strings.Contains(src, "_FileCapabilities") || strings.Contains(src, "_NetCapabilities") {
		t.Error("zero-cap package must not emit capability namespace structs")
	}
	// Must include Operation infrastructure.
	if !strings.Contains(src, "type OperationResult[T any] struct") {
		t.Error("expected OperationResult type")
	}
	if !strings.Contains(src, "type ResultFactory[T any] struct{}") {
		t.Error("expected ResultFactory type")
	}
	if !strings.Contains(src, "type Operation[T any] struct") {
		t.Error("expected Operation type")
	}
	if !strings.Contains(src, "func StartNew[T any](") {
		t.Error("expected StartNew function")
	}
	if !strings.Contains(src, "func (f *ResultFactory[T]) Generate(") {
		t.Error("expected ResultFactory.Generate method")
	}
	if !strings.Contains(src, "func (f *ResultFactory[T]) Fail(") {
		t.Error("expected ResultFactory.Fail method")
	}
	if !strings.Contains(src, "func (op *Operation[T]) PanicOnUnexpected()") {
		t.Error("expected Operation.PanicOnUnexpected method")
	}
	if !strings.Contains(src, "_capabilityViolationError") {
		t.Error("expected _capabilityViolationError type")
	}
	// Must NOT contain cage.EmptyManifest or cage.NewManifest (old design).
	if strings.Contains(src, "cage.EmptyManifest") {
		t.Error("must not use cage.EmptyManifest")
	}
	if strings.Contains(src, "cage.NewManifest") {
		t.Error("must not use cage.NewManifest")
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// generateSource — non-empty capabilities
// ─────────────────────────────────────────────────────────────────────────────

func TestGenerateSource_WithFSRead(t *testing.T) {
	tmp := t.TempDir()
	manifestPath := filepath.Join(tmp, "required_capabilities.json")

	// Write a go file so package name can be read.
	_ = os.WriteFile(filepath.Join(tmp, "lexer.go"),
		[]byte("package veriloglexer\n"), 0o644) //nolint:cap

	mf := &manifestJSON{
		Package: "go/verilog-lexer",
		Capabilities: []capabilityJSON{
			{
				Category:      "fs",
				Action:        "read",
				Target:        "code/grammars/verilog.tokens",
				Justification: "Reads Verilog token grammar file at startup.",
			},
		},
	}

	src, err := generateSource(manifestPath, mf)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	// Package declaration.
	if !strings.Contains(src, "package veriloglexer") {
		t.Error("expected package veriloglexer")
	}
	// Must include os import for fs operations.
	if !strings.Contains(src, `"os"`) {
		t.Error("expected os import for fs:read capability")
	}
	// Must have ReadFile method on _FileCapabilities namespace struct.
	if !strings.Contains(src, "func (c *_FileCapabilities) ReadFile(path string) ([]byte, error)") {
		t.Error("expected ReadFile method on _FileCapabilities")
	}
	// Must have File field on Operation[T].
	if !strings.Contains(src, "File") || !strings.Contains(src, "_FileCapabilities") {
		t.Error("expected File field of type *_FileCapabilities on Operation[T]")
	}
	// Must check against exact declared path.
	if !strings.Contains(src, `"code/grammars/verilog.tokens"`) {
		t.Error("expected declared path in ReadFile allowed check")
	}
	// Must return capability violation error for unknown paths.
	if !strings.Contains(src, "_capabilityViolationError") {
		t.Error("expected _capabilityViolationError in ReadFile")
	}
}

func TestGenerateSource_WithRelativeFSRead(t *testing.T) {
	tmp := t.TempDir()
	manifestPath := filepath.Join(tmp, "required_capabilities.json")

	_ = os.WriteFile(filepath.Join(tmp, "lexer.go"),
		[]byte("package sqllexer\n"), 0o644) //nolint:cap

	mf := &manifestJSON{
		Package: "go/sql-lexer",
		Capabilities: []capabilityJSON{
			{
				Category:      "fs",
				Action:        "read",
				Target:        "../../grammars/sql.tokens",
				Justification: "Reads SQL token grammar file at startup.",
			},
		},
	}

	src, err := generateSource(manifestPath, mf)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	// Must import runtime and sync for the resolved-path vars.
	if !strings.Contains(src, `"runtime"`) {
		t.Error("expected runtime import for relative fs:read target")
	}
	if !strings.Contains(src, `"sync"`) {
		t.Error("expected sync import for relative fs:read target")
	}
	// Must use sync.OnceValue for canonical path resolution.
	if !strings.Contains(src, "sync.OnceValue") {
		t.Error("expected sync.OnceValue for relative target enforcement")
	}
	// Must use runtime.Caller to anchor the path.
	if !strings.Contains(src, "runtime.Caller(0)") {
		t.Error("expected runtime.Caller(0) in generated path resolution")
	}
	// The relative path should appear in the filepath.Join call (to navigate to the target).
	if !strings.Contains(src, `"../../grammars/sql.tokens"`) {
		t.Error("expected relative path ../../grammars/sql.tokens in filepath.Join call")
	}
	// Must use exact equality (_allowedPath_0()), not suffix matching.
	if strings.Contains(src, "strings.HasSuffix") {
		t.Error("must not emit strings.HasSuffix — exact canonical path comparison required")
	}
	if !strings.Contains(src, "_allowedPath_0") {
		t.Error("expected _allowedPath_0 var for first relative target")
	}
	// Must still have ReadFile method and capability violation error.
	if !strings.Contains(src, "func (c *_FileCapabilities) ReadFile(path string) ([]byte, error)") {
		t.Error("expected ReadFile method")
	}
	if !strings.Contains(src, "_capabilityViolationError") {
		t.Error("expected _capabilityViolationError")
	}
}

func TestGenerateSource_WildcardInScopeableCategory_ReturnsError(t *testing.T) {
	tmp := t.TempDir()
	manifestPath := filepath.Join(tmp, "required_capabilities.json")

	mf := &manifestJSON{
		Package: "go/verilog-lexer",
		Capabilities: []capabilityJSON{
			{Category: "fs", Action: "read", Target: "*"},
		},
	}

	_, err := generateSource(manifestPath, mf)
	if err == nil {
		t.Error("expected error for wildcard fs:read target, got nil")
	}
	if !strings.Contains(err.Error(), "wildcard") {
		t.Errorf("error should mention 'wildcard', got: %v", err)
	}
}

func TestGenerateSource_MultipleTargetsSameAction(t *testing.T) {
	tmp := t.TempDir()
	manifestPath := filepath.Join(tmp, "required_capabilities.json")

	mf := &manifestJSON{
		Package: "go/vhdl-lexer",
		Capabilities: []capabilityJSON{
			{Category: "fs", Action: "read", Target: "code/grammars/vhdl.tokens"},
			{Category: "fs", Action: "read", Target: "code/grammars/vhdl.grammar"},
		},
	}

	src, err := generateSource(manifestPath, mf)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	// Both paths should appear in the allowed map.
	if !strings.Contains(src, `"code/grammars/vhdl.tokens"`) {
		t.Error("expected vhdl.tokens in allowed paths")
	}
	if !strings.Contains(src, `"code/grammars/vhdl.grammar"`) {
		t.Error("expected vhdl.grammar in allowed paths")
	}
	// Should only have one ReadFile method (merged).
	count := strings.Count(src, "func (c *_FileCapabilities) ReadFile(")
	if count != 1 {
		t.Errorf("expected 1 ReadFile method, got %d", count)
	}
}

func TestGenerateSource_WithTimeCapability(t *testing.T) {
	tmp := t.TempDir()
	manifestPath := filepath.Join(tmp, "required_capabilities.json")

	mf := &manifestJSON{
		Package: "go/ticker",
		Capabilities: []capabilityJSON{
			{Category: "time", Action: "sleep", Target: "*"},
		},
	}

	src, err := generateSource(manifestPath, mf)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if !strings.Contains(src, "func (c *_TimeCapabilities) Sleep(d time.Duration)") {
		t.Error("expected Sleep method on _TimeCapabilities")
	}
	// time:sleep with "*" target is allowed (non-scopeable).
	if strings.Contains(src, "_capabilityViolationError{") {
		// The violation error type should be defined, but not used inside Sleep.
		// Check that Sleep itself doesn't have a path check.
		sleepIdx := strings.Index(src, "func (c *_TimeCapabilities) Sleep(")
		endIdx := strings.Index(src[sleepIdx:], "\n}\n")
		sleepBody := src[sleepIdx : sleepIdx+endIdx]
		if strings.Contains(sleepBody, "_capabilityViolationError") {
			t.Error("Sleep method should not contain capability violation check (non-scopeable)")
		}
	}
}

func TestGenerateSource_WithStdinCapability(t *testing.T) {
	tmp := t.TempDir()
	manifestPath := filepath.Join(tmp, "required_capabilities.json")

	mf := &manifestJSON{
		Package: "go/brainfuck",
		Capabilities: []capabilityJSON{
			{Category: "stdin", Action: "read", Target: "*"},
		},
	}

	src, err := generateSource(manifestPath, mf)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if !strings.Contains(src, "func (c *_StdinCapabilities) ReadStdin() ([]byte, error)") {
		t.Error("expected ReadStdin method on _StdinCapabilities")
	}
	if !strings.Contains(src, `"io"`) {
		t.Error("expected io import for stdin:read")
	}
	if !strings.Contains(src, `"os"`) {
		t.Error("expected os import for stdin:read")
	}
}

func TestGenerateSource_WithStdoutCapability(t *testing.T) {
	tmp := t.TempDir()
	manifestPath := filepath.Join(tmp, "required_capabilities.json")

	mf := &manifestJSON{
		Package: "go/brainfuck",
		Capabilities: []capabilityJSON{
			{Category: "stdout", Action: "write", Target: "*"},
		},
	}

	src, err := generateSource(manifestPath, mf)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if !strings.Contains(src, "func (c *_StdoutCapabilities) WriteStdout(data []byte) (int, error)") {
		t.Error("expected WriteStdout method on _StdoutCapabilities")
	}
}

func TestGenerateSource_WithFSWrite(t *testing.T) {
	tmp := t.TempDir()
	manifestPath := filepath.Join(tmp, "required_capabilities.json")

	mf := &manifestJSON{
		Package: "go/config-writer",
		Capabilities: []capabilityJSON{
			{Category: "fs", Action: "write", Target: "config/output.json"},
		},
	}

	src, err := generateSource(manifestPath, mf)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !strings.Contains(src, "func (c *_FileCapabilities) WriteFile(") {
		t.Error("expected WriteFile method on _FileCapabilities")
	}
	if !strings.Contains(src, `"config/output.json"`) {
		t.Error("expected declared path in WriteFile allowed check")
	}
}

func TestGenerateSource_WithFSCreate(t *testing.T) {
	tmp := t.TempDir()
	mf := &manifestJSON{
		Package: "go/file-maker",
		Capabilities: []capabilityJSON{
			{Category: "fs", Action: "create", Target: "output/result.txt"},
		},
	}
	src, err := generateSource(filepath.Join(tmp, "required_capabilities.json"), mf)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !strings.Contains(src, "func (c *_FileCapabilities) CreateFile(") {
		t.Error("expected CreateFile method on _FileCapabilities")
	}
}

func TestGenerateSource_WithFSDelete(t *testing.T) {
	tmp := t.TempDir()
	mf := &manifestJSON{
		Package: "go/file-cleaner",
		Capabilities: []capabilityJSON{
			{Category: "fs", Action: "delete", Target: "tmp/scratch.tmp"},
		},
	}
	src, err := generateSource(filepath.Join(tmp, "required_capabilities.json"), mf)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !strings.Contains(src, "func (c *_FileCapabilities) DeleteFile(") {
		t.Error("expected DeleteFile method on _FileCapabilities")
	}
}

func TestGenerateSource_WithFSList(t *testing.T) {
	tmp := t.TempDir()
	mf := &manifestJSON{
		Package: "go/dir-lister",
		Capabilities: []capabilityJSON{
			{Category: "fs", Action: "list", Target: "code/grammars"},
		},
	}
	src, err := generateSource(filepath.Join(tmp, "required_capabilities.json"), mf)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !strings.Contains(src, "func (c *_FileCapabilities) ReadDir(") {
		t.Error("expected ReadDir method on _FileCapabilities")
	}
}

func TestGenerateSource_WithNetConnect(t *testing.T) {
	tmp := t.TempDir()
	mf := &manifestJSON{
		Package: "go/http-client",
		Capabilities: []capabilityJSON{
			{Category: "net", Action: "connect", Target: "api.example.com:443"},
		},
	}
	src, err := generateSource(filepath.Join(tmp, "required_capabilities.json"), mf)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !strings.Contains(src, "func (c *_NetCapabilities) Connect(") {
		t.Error("expected Connect method on _NetCapabilities")
	}
	if !strings.Contains(src, `"net"`) {
		t.Error("expected net import")
	}
}

func TestGenerateSource_WithNetListen(t *testing.T) {
	tmp := t.TempDir()
	mf := &manifestJSON{
		Package: "go/http-server",
		Capabilities: []capabilityJSON{
			{Category: "net", Action: "listen", Target: ":8080"},
		},
	}
	src, err := generateSource(filepath.Join(tmp, "required_capabilities.json"), mf)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !strings.Contains(src, "func (c *_NetCapabilities) Listen(") {
		t.Error("expected Listen method on _NetCapabilities")
	}
}

func TestGenerateSource_WithNetDNS(t *testing.T) {
	tmp := t.TempDir()
	mf := &manifestJSON{
		Package: "go/dns-resolver",
		Capabilities: []capabilityJSON{
			{Category: "net", Action: "dns", Target: "api.example.com"},
		},
	}
	src, err := generateSource(filepath.Join(tmp, "required_capabilities.json"), mf)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !strings.Contains(src, "func (c *_NetCapabilities) LookupHost(") {
		t.Error("expected LookupHost method on _NetCapabilities")
	}
}

func TestGenerateSource_WithProcExec(t *testing.T) {
	tmp := t.TempDir()
	mf := &manifestJSON{
		Package: "go/runner",
		Capabilities: []capabilityJSON{
			{Category: "proc", Action: "exec", Target: "/usr/bin/git"},
		},
	}
	src, err := generateSource(filepath.Join(tmp, "required_capabilities.json"), mf)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !strings.Contains(src, "func (c *_ProcCapabilities) Exec(") {
		t.Error("expected Exec method on _ProcCapabilities")
	}
	if !strings.Contains(src, `"os/exec"`) {
		t.Error("expected os/exec import")
	}
}

func TestGenerateSource_WithEnvRead(t *testing.T) {
	tmp := t.TempDir()
	mf := &manifestJSON{
		Package: "go/config-reader",
		Capabilities: []capabilityJSON{
			{Category: "env", Action: "read", Target: "APP_CONFIG"},
		},
	}
	src, err := generateSource(filepath.Join(tmp, "required_capabilities.json"), mf)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !strings.Contains(src, "func (c *_EnvCapabilities) Getenv(") {
		t.Error("expected Getenv method on _EnvCapabilities")
	}
	if !strings.Contains(src, `"APP_CONFIG"`) {
		t.Error("expected declared env var in Getenv allowed check")
	}
}

func TestGenerateSource_UnknownCapabilityEmitsTODO(t *testing.T) {
	tmp := t.TempDir()
	mf := &manifestJSON{
		Package: "go/future-pkg",
		Capabilities: []capabilityJSON{
			{Category: "ffi", Action: "call", Target: "libfoo.so"},
		},
	}
	src, err := generateSource(filepath.Join(tmp, "required_capabilities.json"), mf)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	// Unknown category:action should emit a TODO comment.
	if !strings.Contains(src, "TODO") {
		t.Error("expected TODO comment for unknown capability ffi:call")
	}
}

func TestGenerateSource_MultipleTargetsSingleReturn(t *testing.T) {
	// fs:write with two targets → WriteFile uses map check (not single-target if).
	tmp := t.TempDir()
	mf := &manifestJSON{
		Package: "go/dual-writer",
		Capabilities: []capabilityJSON{
			{Category: "fs", Action: "write", Target: "output/a.json"},
			{Category: "fs", Action: "write", Target: "output/b.json"},
		},
	}
	src, err := generateSource(filepath.Join(tmp, "required_capabilities.json"), mf)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !strings.Contains(src, `"output/a.json"`) || !strings.Contains(src, `"output/b.json"`) {
		t.Error("expected both paths in WriteFile allowed map")
	}
}

func TestGenerateSource_JustificationDoesNotAffectOutput(t *testing.T) {
	// Justification is metadata only — it should not appear in the generated code.
	tmp := t.TempDir()
	manifestPath := filepath.Join(tmp, "required_capabilities.json")

	mf := &manifestJSON{
		Package: "go/test-pkg",
		Capabilities: []capabilityJSON{
			{Category: "fs", Action: "read", Target: "code/grammars/test.tokens",
				Justification: `Reads "grammar" files via os.ReadFile.`},
		},
	}

	src, err := generateSource(manifestPath, mf)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	// Justification text is not part of the generated source.
	if strings.Contains(src, "Reads") {
		t.Error("justification text should not appear in generated source")
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// generateSource — Operation infrastructure completeness
// ─────────────────────────────────────────────────────────────────────────────

func TestGenerateSource_OperationInfrastructure(t *testing.T) {
	// Verify all Operation infrastructure is present even with capabilities.
	tmp := t.TempDir()
	manifestPath := filepath.Join(tmp, "required_capabilities.json")

	mf := &manifestJSON{
		Package: "go/test-pkg",
		Capabilities: []capabilityJSON{
			{Category: "fs", Action: "read", Target: "code/grammars/test.tokens"},
		},
	}

	src, err := generateSource(manifestPath, mf)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	checks := []struct {
		name    string
		snippet string
	}{
		{"OperationResult type", "type OperationResult[T any] struct"},
		{"Err field", "Err                 error"},
		{"ResultFactory type", "type ResultFactory[T any] struct{}"},
		{"Generate method", "func (f *ResultFactory[T]) Generate("},
		{"Fail method", "func (f *ResultFactory[T]) Fail("},
		{"Operation type", "type Operation[T any] struct"},
		{"rePanic field", "rePanic     bool"},
		{"AddProperty method", "func (op *Operation[T]) AddProperty("},
		{"PanicOnUnexpected method", "func (op *Operation[T]) PanicOnUnexpected()"},
		{"GetResult method", "func (op *Operation[T]) GetResult() (T, error)"},
		{"StartNew function", "func StartNew[T any]("},
		{"capabilityViolationError", "type _capabilityViolationError struct"},
	}

	for _, c := range checks {
		if !strings.Contains(src, c.snippet) {
			t.Errorf("missing %s: %q not found in generated source", c.name, c.snippet)
		}
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
				"target": "code/grammars/verilog.tokens",
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
	if !strings.Contains(src, "func (c *_FileCapabilities) ReadFile(") {
		t.Error("expected ReadFile method on _FileCapabilities")
	}
	if !strings.Contains(src, "func StartNew[T any](") {
		t.Error("expected StartNew function")
	}
}

func TestProcessManifest_RejectsWildcardTarget(t *testing.T) {
	tmp := t.TempDir()
	manifestPath := filepath.Join(tmp, "required_capabilities.json")

	manifest := `{
		"version": 1,
		"package": "go/test-pkg",
		"capabilities": [
			{
				"category": "fs",
				"action": "read",
				"target": "*",
				"justification": "Reads all files."
			}
		]
	}`
	_ = os.WriteFile(manifestPath, []byte(manifest), 0o644) //nolint:cap

	err := processManifest(manifestPath, false)
	if err == nil {
		t.Error("expected error for wildcard fs target, got nil")
	}
	// gen_capabilities.go should NOT be written.
	outPath := filepath.Join(tmp, "gen_capabilities.go")
	if _, statErr := os.Stat(outPath); !os.IsNotExist(statErr) { //nolint:cap
		t.Error("gen_capabilities.go should not be written when manifest has wildcard")
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
	data, err := os.ReadFile(outPath) //nolint:cap
	if err != nil {
		t.Fatalf("expected gen_capabilities.go to be written: %v", err)
	}
	// Generated file should have Operation infrastructure.
	if !strings.Contains(string(data), "func StartNew[T any](") {
		t.Error("expected StartNew in generated output")
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
