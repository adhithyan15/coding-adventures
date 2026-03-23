// =========================================================================
// scaffold-generator — Tests
// =========================================================================
//
// These tests cover:
// 1. Name normalization (kebab → snake, camel, joined)
// 2. CLI parsing via cli-builder integration
// 3. Dependency resolution (transitive closure, topological sort)
// 4. File generation for all six languages
// 5. Input validation (bad names, missing deps)
// 6. End-to-end scaffolding with BUILD file verification

package main

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// =========================================================================
// Name normalization tests
// =========================================================================

func TestToSnakeCase(t *testing.T) {
	tests := []struct{ input, want string }{
		{"my-package", "my_package"},
		{"logic-gates", "logic_gates"},
		{"simple", "simple"},
		{"a-b-c", "a_b_c"},
		{"cpu-simulator", "cpu_simulator"},
	}
	for _, tt := range tests {
		got := toSnakeCase(tt.input)
		if got != tt.want {
			t.Errorf("toSnakeCase(%q) = %q, want %q", tt.input, got, tt.want)
		}
	}
}

func TestToCamelCase(t *testing.T) {
	tests := []struct{ input, want string }{
		{"my-package", "MyPackage"},
		{"logic-gates", "LogicGates"},
		{"simple", "Simple"},
		{"a-b-c", "ABC"},
		{"cpu-simulator", "CpuSimulator"},
	}
	for _, tt := range tests {
		got := toCamelCase(tt.input)
		if got != tt.want {
			t.Errorf("toCamelCase(%q) = %q, want %q", tt.input, got, tt.want)
		}
	}
}

func TestToJoinedLower(t *testing.T) {
	tests := []struct{ input, want string }{
		{"my-package", "mypackage"},
		{"logic-gates", "logicgates"},
		{"simple", "simple"},
	}
	for _, tt := range tests {
		got := toJoinedLower(tt.input)
		if got != tt.want {
			t.Errorf("toJoinedLower(%q) = %q, want %q", tt.input, got, tt.want)
		}
	}
}

func TestDirName(t *testing.T) {
	tests := []struct {
		input, lang, want string
	}{
		{"my-package", "python", "my-package"},
		{"my-package", "go", "my-package"},
		{"my-package", "typescript", "my-package"},
		{"my-package", "rust", "my-package"},
		{"my-package", "ruby", "my_package"},
		{"my-package", "elixir", "my_package"},
	}
	for _, tt := range tests {
		got := dirName(tt.input, tt.lang)
		if got != tt.want {
			t.Errorf("dirName(%q, %q) = %q, want %q", tt.input, tt.lang, got, tt.want)
		}
	}
}

// =========================================================================
// Input validation tests
// =========================================================================

func TestKebabCaseValidation(t *testing.T) {
	valid := []string{"my-package", "logic-gates", "a", "a1", "cpu-sim-v2", "x86"}
	for _, name := range valid {
		if !kebabCaseRe.MatchString(name) {
			t.Errorf("expected %q to be valid kebab-case", name)
		}
	}

	invalid := []string{"MyPackage", "my_package", "-leading", "trailing-", "double--hyphen", "UPPER", "has space", "123start"}
	for _, name := range invalid {
		if kebabCaseRe.MatchString(name) {
			t.Errorf("expected %q to be invalid kebab-case", name)
		}
	}
}

// =========================================================================
// CLI Builder integration tests
// =========================================================================

func specPath(t *testing.T) string {
	t.Helper()
	// The spec file is at code/programs/scaffold-generator.json
	// We're at code/programs/go/scaffold-generator/
	abs, err := filepath.Abs("../../scaffold-generator.json")
	if err != nil {
		t.Fatalf("cannot resolve spec path: %v", err)
	}
	return abs
}

func TestSpecLoads(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := run(specPath(t), []string{"scaffold-generator", "--help"}, &stdout, &stderr)
	if code != 0 {
		t.Errorf("--help returned exit code %d, stderr: %s", code, stderr.String())
	}
	if stdout.Len() == 0 {
		t.Error("--help produced no output")
	}
}

func TestVersionFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := run(specPath(t), []string{"scaffold-generator", "--version"}, &stdout, &stderr)
	if code != 0 {
		t.Errorf("--version returned exit code %d", code)
	}
	if strings.TrimSpace(stdout.String()) != "1.0.0" {
		t.Errorf("--version output = %q, want 1.0.0", stdout.String())
	}
}

func TestMissingPackageName(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := run(specPath(t), []string{"scaffold-generator"}, &stdout, &stderr)
	if code == 0 {
		t.Error("expected non-zero exit code when package name is missing")
	}
}

func TestInvalidPackageName(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := run(specPath(t), []string{"scaffold-generator", "INVALID_NAME"}, &stdout, &stderr)
	if code == 0 {
		t.Error("expected non-zero exit code for invalid package name")
	}
	if !strings.Contains(stderr.String(), "invalid package name") {
		t.Errorf("stderr should mention invalid package name, got: %s", stderr.String())
	}
}

func TestUnknownLanguage(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := run(specPath(t), []string{"scaffold-generator", "test-pkg", "--language", "java"}, &stdout, &stderr)
	if code == 0 {
		t.Error("expected non-zero exit code for unknown language")
	}
	if !strings.Contains(stderr.String(), "unknown language") {
		t.Errorf("stderr should mention unknown language, got: %s", stderr.String())
	}
}

// =========================================================================
// Dependency reading tests
// =========================================================================

func TestReadPythonDeps(t *testing.T) {
	// Create a temp dir with a mock BUILD file
	tmpDir := t.TempDir()
	buildContent := `uv venv .venv --quiet --no-project
uv pip install --python .venv -e ../logic-gates --quiet
uv pip install --python .venv -e ../arithmetic --quiet
uv pip install --python .venv -e .[dev] --quiet
uv run --no-project python -m pytest tests/ -v
`
	os.WriteFile(filepath.Join(tmpDir, "BUILD"), []byte(buildContent), 0o644)

	deps, err := readPythonDeps(tmpDir)
	if err != nil {
		t.Fatalf("readPythonDeps: %v", err)
	}
	if len(deps) != 2 {
		t.Fatalf("expected 2 deps, got %d: %v", len(deps), deps)
	}
	if deps[0] != "logic-gates" || deps[1] != "arithmetic" {
		t.Errorf("deps = %v, want [logic-gates arithmetic]", deps)
	}
}

func TestReadTypeScriptDeps(t *testing.T) {
	tmpDir := t.TempDir()
	pkgJSON := `{
  "name": "@coding-adventures/test",
  "dependencies": {
    "@coding-adventures/logic-gates": "file:../logic-gates",
    "@coding-adventures/arithmetic": "file:../arithmetic"
  }
}`
	os.WriteFile(filepath.Join(tmpDir, "package.json"), []byte(pkgJSON), 0o644)

	deps, err := readTypeScriptDeps(tmpDir)
	if err != nil {
		t.Fatalf("readTypeScriptDeps: %v", err)
	}
	if len(deps) != 2 {
		t.Fatalf("expected 2 deps, got %d: %v", len(deps), deps)
	}
}

func TestReadGoDeps(t *testing.T) {
	tmpDir := t.TempDir()
	goMod := `module github.com/test/pkg

go 1.26

replace (
	github.com/test/logic-gates => ../logic-gates
	github.com/test/arithmetic => ../arithmetic
)
`
	os.WriteFile(filepath.Join(tmpDir, "go.mod"), []byte(goMod), 0o644)

	deps, err := readGoDeps(tmpDir)
	if err != nil {
		t.Fatalf("readGoDeps: %v", err)
	}
	if len(deps) != 2 {
		t.Fatalf("expected 2 deps, got %d: %v", len(deps), deps)
	}
}

func TestReadRustDeps(t *testing.T) {
	tmpDir := t.TempDir()
	cargoToml := `[package]
name = "test"
version = "0.1.0"

[dependencies]
logic-gates = { path = "../logic-gates" }
arithmetic = { path = "../arithmetic" }
`
	os.WriteFile(filepath.Join(tmpDir, "Cargo.toml"), []byte(cargoToml), 0o644)

	deps, err := readRustDeps(tmpDir)
	if err != nil {
		t.Fatalf("readRustDeps: %v", err)
	}
	if len(deps) != 2 {
		t.Fatalf("expected 2 deps, got %d: %v", len(deps), deps)
	}
}

func TestReadRubyDeps(t *testing.T) {
	tmpDir := t.TempDir()
	gemfile := `source "https://rubygems.org"
gemspec
gem "coding_adventures_logic_gates", path: "../logic_gates"
gem "coding_adventures_arithmetic", path: "../arithmetic"
`
	os.WriteFile(filepath.Join(tmpDir, "Gemfile"), []byte(gemfile), 0o644)

	deps, err := readRubyDeps(tmpDir)
	if err != nil {
		t.Fatalf("readRubyDeps: %v", err)
	}
	if len(deps) != 2 {
		t.Fatalf("expected 2 deps, got %d: %v", len(deps), deps)
	}
}

func TestReadElixirDeps(t *testing.T) {
	tmpDir := t.TempDir()
	mixExs := `defmodule Test.MixProject do
  defp deps do
    [
      {:coding_adventures_logic_gates, path: "../logic_gates"},
      {:coding_adventures_arithmetic, path: "../arithmetic"}
    ]
  end
end
`
	os.WriteFile(filepath.Join(tmpDir, "mix.exs"), []byte(mixExs), 0o644)

	deps, err := readElixirDeps(tmpDir)
	if err != nil {
		t.Fatalf("readElixirDeps: %v", err)
	}
	if len(deps) != 2 {
		t.Fatalf("expected 2 deps, got %d: %v", len(deps), deps)
	}
}

// =========================================================================
// Transitive closure and topological sort tests
// =========================================================================

func TestTransitiveClosure(t *testing.T) {
	// Set up a mock dependency graph:
	// A depends on B, B depends on C
	tmpDir := t.TempDir()
	aDir := filepath.Join(tmpDir, "a")
	bDir := filepath.Join(tmpDir, "b")
	cDir := filepath.Join(tmpDir, "c")
	os.MkdirAll(aDir, 0o755)
	os.MkdirAll(bDir, 0o755)
	os.MkdirAll(cDir, 0o755)

	// A depends on B (Python BUILD style)
	os.WriteFile(filepath.Join(aDir, "BUILD"), []byte("uv pip install --python .venv -e ../b --quiet\n"), 0o644)
	// B depends on C
	os.WriteFile(filepath.Join(bDir, "BUILD"), []byte("uv pip install --python .venv -e ../c --quiet\n"), 0o644)
	// C has no deps
	os.WriteFile(filepath.Join(cDir, "BUILD"), []byte("uv pip install --python .venv -e .[dev] --quiet\n"), 0o644)

	deps, err := transitiveClosure([]string{"b"}, "python", tmpDir)
	if err != nil {
		t.Fatalf("transitiveClosure: %v", err)
	}

	// Should find b and c
	depSet := make(map[string]bool)
	for _, d := range deps {
		depSet[d] = true
	}
	if !depSet["b"] {
		t.Error("expected b in transitive closure")
	}
	if !depSet["c"] {
		t.Error("expected c in transitive closure")
	}
}

func TestTopologicalSort(t *testing.T) {
	// Set up: A→B→C, A→D (no deps)
	tmpDir := t.TempDir()
	for _, name := range []string{"a", "b", "c", "d"} {
		os.MkdirAll(filepath.Join(tmpDir, name), 0o755)
	}
	os.WriteFile(filepath.Join(tmpDir, "a", "BUILD"), []byte("uv pip install --python .venv -e ../b --quiet\nuv pip install --python .venv -e ../d --quiet\n"), 0o644)
	os.WriteFile(filepath.Join(tmpDir, "b", "BUILD"), []byte("uv pip install --python .venv -e ../c --quiet\n"), 0o644)
	os.WriteFile(filepath.Join(tmpDir, "c", "BUILD"), []byte(""), 0o644)
	os.WriteFile(filepath.Join(tmpDir, "d", "BUILD"), []byte(""), 0o644)

	allDeps := []string{"b", "c", "d"}
	order, err := topologicalSort(allDeps, "python", tmpDir)
	if err != nil {
		t.Fatalf("topologicalSort: %v", err)
	}

	// c and d should come before b (they are leaves)
	posOf := map[string]int{}
	for i, dep := range order {
		posOf[dep] = i
	}

	if posOf["c"] >= posOf["b"] {
		t.Errorf("c (pos %d) should come before b (pos %d)", posOf["c"], posOf["b"])
	}
	// d has no deps within set, so it can be anywhere
}

// =========================================================================
// File generation tests — Python
// =========================================================================

func TestGeneratePython(t *testing.T) {
	tmpDir := t.TempDir()
	err := generatePython(tmpDir, "test-pkg", "A test package", "", []string{"logic-gates"}, []string{"logic-gates"})
	if err != nil {
		t.Fatalf("generatePython: %v", err)
	}

	// Check pyproject.toml exists and has correct content
	pyproject, err := os.ReadFile(filepath.Join(tmpDir, "pyproject.toml"))
	if err != nil {
		t.Fatalf("cannot read pyproject.toml: %v", err)
	}
	if !strings.Contains(string(pyproject), "coding-adventures-test-pkg") {
		t.Error("pyproject.toml missing package name")
	}
	if !strings.Contains(string(pyproject), "hatchling") {
		t.Error("pyproject.toml missing hatchling build system")
	}
	if !strings.Contains(string(pyproject), "ruff") {
		t.Error("pyproject.toml missing ruff in dev deps")
	}

	// Check BUILD file has transitive deps
	build, _ := os.ReadFile(filepath.Join(tmpDir, "BUILD"))
	if !strings.Contains(string(build), "../logic-gates") {
		t.Error("BUILD missing dependency install")
	}

	// Check test file exists
	if _, err := os.Stat(filepath.Join(tmpDir, "tests", "test_test_pkg.py")); err != nil {
		t.Error("test file missing")
	}
}

// =========================================================================
// File generation tests — Go
// =========================================================================

func TestGenerateGo(t *testing.T) {
	tmpDir := t.TempDir()
	err := generateGo(tmpDir, "test-pkg", "A test package", "", []string{"logic-gates"}, []string{"logic-gates"})
	if err != nil {
		t.Fatalf("generateGo: %v", err)
	}

	// Check go.mod
	goMod, err := os.ReadFile(filepath.Join(tmpDir, "go.mod"))
	if err != nil {
		t.Fatalf("cannot read go.mod: %v", err)
	}
	if !strings.Contains(string(goMod), "go/test-pkg") {
		t.Error("go.mod missing module path")
	}
	if !strings.Contains(string(goMod), "../logic-gates") {
		t.Error("go.mod missing replace directive")
	}

	// Check source file
	if _, err := os.Stat(filepath.Join(tmpDir, "test_pkg.go")); err != nil {
		t.Error("source file missing")
	}
}

// =========================================================================
// File generation tests — TypeScript
// =========================================================================

func TestGenerateTypeScript(t *testing.T) {
	tmpDir := t.TempDir()
	err := generateTypeScript(tmpDir, "test-pkg", "A test package", "", []string{"logic-gates"}, []string{"logic-gates"})
	if err != nil {
		t.Fatalf("generateTypeScript: %v", err)
	}

	// Check package.json — critical fields
	pkgJSON, err := os.ReadFile(filepath.Join(tmpDir, "package.json"))
	if err != nil {
		t.Fatalf("cannot read package.json: %v", err)
	}

	var pkg map[string]any
	json.Unmarshal(pkgJSON, &pkg)

	// CRITICAL: main must be src/index.ts (NOT dist/index.js)
	main, _ := pkg["main"].(string)
	if main != "src/index.ts" {
		t.Errorf("package.json main = %q, MUST be \"src/index.ts\"", main)
	}

	// CRITICAL: type must be "module"
	moduleType, _ := pkg["type"].(string)
	if moduleType != "module" {
		t.Errorf("package.json type = %q, MUST be \"module\"", moduleType)
	}

	// CRITICAL: @vitest/coverage-v8 must be in devDependencies
	devDeps, _ := pkg["devDependencies"].(map[string]any)
	if _, ok := devDeps["@vitest/coverage-v8"]; !ok {
		t.Error("package.json devDependencies missing @vitest/coverage-v8")
	}

	// Check BUILD has npm ci (no more chain installs, npm ci resolves file: deps)
	build, _ := os.ReadFile(filepath.Join(tmpDir, "BUILD"))
	if !strings.Contains(string(build), "npm ci --quiet") {
		t.Error("BUILD missing npm ci")
	}
}

// =========================================================================
// File generation tests — Ruby
// =========================================================================

func TestGenerateRuby(t *testing.T) {
	tmpDir := t.TempDir()
	err := generateRuby(tmpDir, "test-pkg", "A test package", "", []string{"logic-gates"}, []string{"logic-gates"})
	if err != nil {
		t.Fatalf("generateRuby: %v", err)
	}

	// Check entry point requires deps FIRST
	entryPoint, err := os.ReadFile(filepath.Join(tmpDir, "lib", "coding_adventures_test_pkg.rb"))
	if err != nil {
		t.Fatalf("cannot read entry point: %v", err)
	}

	content := string(entryPoint)
	requireIdx := strings.Index(content, "require \"coding_adventures_logic_gates\"")
	relativeIdx := strings.Index(content, "require_relative")
	if requireIdx < 0 {
		t.Error("entry point missing require for dependency")
	}
	if relativeIdx < 0 {
		t.Error("entry point missing require_relative")
	}
	if requireIdx >= 0 && relativeIdx >= 0 && requireIdx > relativeIdx {
		t.Error("CRITICAL: dependency require must come BEFORE require_relative")
	}

	// Check Gemfile has transitive deps
	gemfile, _ := os.ReadFile(filepath.Join(tmpDir, "Gemfile"))
	if !strings.Contains(string(gemfile), "coding_adventures_logic_gates") {
		t.Error("Gemfile missing transitive dependency")
	}
}

// =========================================================================
// File generation tests — Rust
// =========================================================================

func TestGenerateRust(t *testing.T) {
	tmpDir := t.TempDir()
	err := generateRust(tmpDir, "test-pkg", "A test package", "", []string{"logic-gates"})
	if err != nil {
		t.Fatalf("generateRust: %v", err)
	}

	cargo, err := os.ReadFile(filepath.Join(tmpDir, "Cargo.toml"))
	if err != nil {
		t.Fatalf("cannot read Cargo.toml: %v", err)
	}
	if !strings.Contains(string(cargo), "logic-gates") {
		t.Error("Cargo.toml missing dependency")
	}

	// Check BUILD uses -p flag
	build, _ := os.ReadFile(filepath.Join(tmpDir, "BUILD"))
	if !strings.Contains(string(build), "-p test-pkg") {
		t.Error("BUILD missing -p flag for package name")
	}
}

// =========================================================================
// File generation tests — Elixir
// =========================================================================

func TestGenerateElixir(t *testing.T) {
	tmpDir := t.TempDir()
	err := generateElixir(tmpDir, "test-pkg", "A test package", "", []string{"logic-gates"}, []string{"logic-gates"})
	if err != nil {
		t.Fatalf("generateElixir: %v", err)
	}

	mixExs, err := os.ReadFile(filepath.Join(tmpDir, "mix.exs"))
	if err != nil {
		t.Fatalf("cannot read mix.exs: %v", err)
	}
	if !strings.Contains(string(mixExs), "coding_adventures_logic_gates") {
		t.Error("mix.exs missing dependency")
	}
	if !strings.Contains(string(mixExs), "coding_adventures_test_pkg") {
		t.Error("mix.exs missing app name")
	}

	// Check BUILD has chain install for deps
	build, _ := os.ReadFile(filepath.Join(tmpDir, "BUILD"))
	if !strings.Contains(string(build), "../logic_gates") {
		t.Error("BUILD missing transitive dep chain install")
	}
}

// =========================================================================
// Common files tests
// =========================================================================

func TestGenerateCommonFiles(t *testing.T) {
	tmpDir := t.TempDir()
	err := generateCommonFiles(tmpDir, "test-pkg", "A test package", "python", 5, []string{"logic-gates"})
	if err != nil {
		t.Fatalf("generateCommonFiles: %v", err)
	}

	// README exists and has content
	readme, err := os.ReadFile(filepath.Join(tmpDir, "README.md"))
	if err != nil {
		t.Fatalf("cannot read README.md: %v", err)
	}
	if !strings.Contains(string(readme), "test-pkg") {
		t.Error("README missing package name")
	}
	if !strings.Contains(string(readme), "Layer 5") {
		t.Error("README missing layer info")
	}

	// CHANGELOG exists
	changelog, err := os.ReadFile(filepath.Join(tmpDir, "CHANGELOG.md"))
	if err != nil {
		t.Fatalf("cannot read CHANGELOG.md: %v", err)
	}
	if !strings.Contains(string(changelog), "0.1.0") {
		t.Error("CHANGELOG missing version")
	}
}

// =========================================================================
// Dry run test
// =========================================================================

func TestDryRun(t *testing.T) {
	var stdout, stderr bytes.Buffer

	// Use the actual repo root — we test dry-run which creates no files
	code := run(specPath(t), []string{"scaffold-generator", "test-dry-run-pkg", "--language", "python", "--dry-run"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("dry-run returned exit code %d, stderr: %s", code, stderr.String())
	}
	if !strings.Contains(stdout.String(), "[dry-run]") {
		t.Error("dry-run output should contain [dry-run] prefix")
	}
}

// =========================================================================
// Directory already exists test
// =========================================================================

func TestRefusesToOverwrite(t *testing.T) {
	// This test creates a temp package dir, then tries to scaffold over it.
	// We need to be in a git repo context for findRepoRoot to work.
	// Since we're running inside the actual repo, we can test with a real path.

	var stdout, stderr bytes.Buffer
	// "logic-gates" already exists in Python, so scaffolding it should fail
	code := run(specPath(t), []string{"scaffold-generator", "logic-gates", "--language", "python"}, &stdout, &stderr)
	if code == 0 {
		t.Error("expected non-zero exit code when target directory exists")
	}
	if !strings.Contains(stderr.String(), "already exists") {
		t.Errorf("stderr should mention directory already exists, got: %s", stderr.String())
	}
}
