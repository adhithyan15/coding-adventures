// =========================================================================
// scaffold-generator — Tests
// =========================================================================
//
// These tests cover:
// 1. Name normalization (kebab → snake, camel, joined)
// 2. CLI parsing via cli-builder integration
// 3. Dependency resolution (transitive closure, topological sort)
// 4. File generation for supported scaffold languages
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
		{"my-package", "java", "my-package"},
		{"my-package", "kotlin", "my-package"},
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
	code := run(specPath(t), []string{"scaffold-generator", "test-pkg", "--language", "fortran"}, &stdout, &stderr)
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
	buildContent := `pip install -e ../logic-gates -e ../arithmetic -e .[dev] --quiet
python -m pytest tests/ -v
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

func TestReadJavaDeps(t *testing.T) {
	tmpDir := t.TempDir()
	buildGradle := `dependencies {
    api("com.codingadventures:grammar-tools")
    api("com.codingadventures:lexer")
    testImplementation("org.junit.jupiter:junit-jupiter:5.11.4")
}
`
	os.WriteFile(filepath.Join(tmpDir, "build.gradle.kts"), []byte(buildGradle), 0o644)

	deps, err := readJavaDeps(tmpDir)
	if err != nil {
		t.Fatalf("readJavaDeps: %v", err)
	}
	if len(deps) != 2 {
		t.Fatalf("expected 2 deps, got %d: %v", len(deps), deps)
	}
	if deps[0] != "grammar-tools" || deps[1] != "lexer" {
		t.Errorf("deps = %v, want [grammar-tools lexer]", deps)
	}
}

func TestReadKotlinDeps(t *testing.T) {
	tmpDir := t.TempDir()
	buildGradle := `dependencies {
    api("com.codingadventures:parser")
    api("com.codingadventures:json-lexer")
    testImplementation(kotlin("test"))
}
`
	os.WriteFile(filepath.Join(tmpDir, "build.gradle.kts"), []byte(buildGradle), 0o644)

	deps, err := readKotlinDeps(tmpDir)
	if err != nil {
		t.Fatalf("readKotlinDeps: %v", err)
	}
	if len(deps) != 2 {
		t.Fatalf("expected 2 deps, got %d: %v", len(deps), deps)
	}
	if deps[0] != "parser" || deps[1] != "json-lexer" {
		t.Errorf("deps = %v, want [parser json-lexer]", deps)
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
	os.WriteFile(filepath.Join(aDir, "BUILD"), []byte("python -m pip install -e ../b -e .[dev] --quiet\n"), 0o644)
	// B depends on C
	os.WriteFile(filepath.Join(bDir, "BUILD"), []byte("python -m pip install -e ../c -e .[dev] --quiet\n"), 0o644)
	// C has no deps
	os.WriteFile(filepath.Join(cDir, "BUILD"), []byte("python -m pip install -e .[dev] --quiet\n"), 0o644)

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
	os.WriteFile(filepath.Join(tmpDir, "a", "BUILD"), []byte("python -m pip install -e ../b -e ../d -e .[dev] --quiet\n"), 0o644)
	os.WriteFile(filepath.Join(tmpDir, "b", "BUILD"), []byte("python -m pip install -e ../c -e .[dev] --quiet\n"), 0o644)
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
// File generation tests — Perl
// =========================================================================

func TestReadPerlDeps(t *testing.T) {
	tmpDir := t.TempDir()
	cpanfile := "requires 'coding-adventures-logic-gates';\nrequires 'coding-adventures-arithmetic';\n\non 'test' => sub {\n    requires 'Test2::V0';\n};\n"
	os.WriteFile(filepath.Join(tmpDir, "cpanfile"), []byte(cpanfile), 0o644)

	deps, err := readPerlDeps(tmpDir)
	if err != nil {
		t.Fatalf("readPerlDeps: %v", err)
	}
	if len(deps) != 2 {
		t.Fatalf("expected 2 deps, got %d: %v", len(deps), deps)
	}
}

func TestReadPerlDepsMissingFile(t *testing.T) {
	tmpDir := t.TempDir()
	deps, err := readPerlDeps(tmpDir)
	if err != nil {
		t.Fatalf("expected no error for missing cpanfile, got: %v", err)
	}
	if len(deps) != 0 {
		t.Errorf("expected 0 deps for missing cpanfile, got: %v", deps)
	}
}

func TestGeneratePerl(t *testing.T) {
	tmpDir := t.TempDir()
	err := generatePerl(tmpDir, "test-pkg", "A test package", "", []string{"logic-gates"}, []string{"logic-gates"})
	if err != nil {
		t.Fatalf("generatePerl: %v", err)
	}

	// Makefile.PL has correct module name and dep
	mpl, err := os.ReadFile(filepath.Join(tmpDir, "Makefile.PL"))
	if err != nil {
		t.Fatalf("cannot read Makefile.PL: %v", err)
	}
	if !strings.Contains(string(mpl), "CodingAdventures::TestPkg") {
		t.Error("Makefile.PL missing module name")
	}
	if !strings.Contains(string(mpl), "CodingAdventures::LogicGates") {
		t.Error("Makefile.PL missing dep in PREREQ_PM")
	}

	// cpanfile has runtime dep
	cpanfile, err := os.ReadFile(filepath.Join(tmpDir, "cpanfile"))
	if err != nil {
		t.Fatalf("cannot read cpanfile: %v", err)
	}
	if !strings.Contains(string(cpanfile), "coding-adventures-logic-gates") {
		t.Error("cpanfile missing runtime dep")
	}
	if !strings.Contains(string(cpanfile), "Test2::V0") {
		t.Error("cpanfile missing test dep")
	}

	// Source module has package declaration, use strict, and ends with 1;
	pm, err := os.ReadFile(filepath.Join(tmpDir, "lib", "CodingAdventures", "TestPkg.pm"))
	if err != nil {
		t.Fatalf("cannot read TestPkg.pm: %v", err)
	}
	if !strings.Contains(string(pm), "package CodingAdventures::TestPkg;") {
		t.Error("module missing package declaration")
	}
	if !strings.Contains(string(pm), "use strict;") {
		t.Error("module missing use strict")
	}
	if !strings.Contains(string(pm), "\n1;\n") {
		t.Error("module missing trailing 1;")
	}
	if !strings.Contains(string(pm), "use CodingAdventures::LogicGates;") {
		t.Error("module missing dep import")
	}

	// t/00-load.t uses eval{require} (Test2::V0 does not export use_ok)
	loadT, err := os.ReadFile(filepath.Join(tmpDir, "t", "00-load.t"))
	if err != nil {
		t.Fatalf("cannot read t/00-load.t: %v", err)
	}
	if !strings.Contains(string(loadT), "require CodingAdventures::TestPkg") {
		t.Error("00-load.t missing module require")
	}
	if !strings.Contains(string(loadT), "done_testing") {
		t.Error("00-load.t missing done_testing")
	}

	// t/01-basic.t has done_testing
	basicT, err := os.ReadFile(filepath.Join(tmpDir, "t", "01-basic.t"))
	if err != nil {
		t.Fatalf("cannot read t/01-basic.t: %v", err)
	}
	if !strings.Contains(string(basicT), "done_testing") {
		t.Error("01-basic.t missing done_testing")
	}

	// BUILD installs dep before current package
	build, err := os.ReadFile(filepath.Join(tmpDir, "BUILD"))
	if err != nil {
		t.Fatalf("cannot read BUILD: %v", err)
	}
	buildStr := string(build)
	if !strings.Contains(buildStr, "../logic-gates") {
		t.Error("BUILD missing dep install")
	}
	if !strings.Contains(buildStr, "prove -l -v t/") {
		t.Error("BUILD missing prove command")
	}
	// Dep line must come before prove line
	depIdx := strings.Index(buildStr, "../logic-gates")
	proveIdx := strings.Index(buildStr, "prove")
	if depIdx > proveIdx {
		t.Error("BUILD: dep install should come before prove")
	}
}

func TestGeneratePerlNoDeps(t *testing.T) {
	tmpDir := t.TempDir()
	err := generatePerl(tmpDir, "my-pkg", "My package", "", nil, nil)
	if err != nil {
		t.Fatalf("generatePerl: %v", err)
	}

	mpl, _ := os.ReadFile(filepath.Join(tmpDir, "Makefile.PL"))
	// PREREQ_PM block should be empty when no deps given.
	// Entries look like: 'CodingAdventures::Foo' => 0,
	// Extract just the PREREQ_PM block and check it has no such entries.
	mplStr := string(mpl)
	prereqStart := strings.Index(mplStr, "PREREQ_PM")
	testReqStart := strings.Index(mplStr, "TEST_REQUIRES")
	if prereqStart >= 0 && testReqStart > prereqStart {
		prereqBlock := mplStr[prereqStart:testReqStart]
		// In the PREREQ_PM block, dep entries have format 'Foo::Bar' => 0,
		// The block itself is: PREREQ_PM => {\n    },
		// Count occurrences of "=> 0," which only appear as dep entries
		if strings.Contains(prereqBlock, "=> 0,") {
			t.Error("Makefile.PL PREREQ_PM should be empty when no deps given")
		}
	}

	build, _ := os.ReadFile(filepath.Join(tmpDir, "BUILD"))
	buildStr := string(build)
	if !strings.Contains(buildStr, "cpanm --installdeps --quiet .") {
		t.Error("BUILD missing cpanm install command")
	}
	if !strings.Contains(buildStr, "prove -l -v t/") {
		t.Error("BUILD missing prove command")
	}
	// No dep install lines
	if strings.Contains(buildStr, "cd ../") {
		t.Error("BUILD should not have dep install lines when no deps")
	}
}

func TestGenerateHaskellUsesRepositoryConventions(t *testing.T) {
	tmpDir := t.TempDir()
	if err := generateHaskell(
		tmpDir,
		"my-pkg",
		"100% compatible package",
		"Layer context",
		[]string{"graph"},
		[]string{"bitset", "graph"},
	); err != nil {
		t.Fatalf("generateHaskell: %v", err)
	}

	cabal, err := os.ReadFile(filepath.Join(tmpDir, "my-pkg.cabal"))
	if err != nil {
		t.Fatalf("cannot read cabal file: %v", err)
	}

	cabalText := string(cabal)
	for _, want := range []string{
		"name:          my-pkg",
		"synopsis:      100% compatible package",
		"description:   100% compatible package.",
		"category:      Development",
		"exposed-modules:  CodingAdventures.MyPkg",
		", graph >=0.1 && <0.2",
		"other-modules:    MyPkgSpec",
		", hspec ==2.*",
		"ghc-options:      -Wall",
	} {
		if !strings.Contains(cabalText, want) {
			t.Errorf("cabal file missing %q", want)
		}
	}
	if strings.Contains(cabalText, "coding-adventures-") {
		t.Error("cabal file should use the repository's plain package names")
	}
	if strings.Contains(cabalText, ", bitset") {
		t.Error("cabal build-depends should contain direct dependencies only")
	}

	project, err := os.ReadFile(filepath.Join(tmpDir, "cabal.project"))
	if err != nil {
		t.Fatalf("cannot read cabal.project: %v", err)
	}
	projectText := string(project)
	if !strings.Contains(projectText, "../bitset") || !strings.Contains(projectText, "../graph") {
		t.Errorf("cabal.project should contain the transitive local closure, got %q", projectText)
	}

	for _, path := range []string{
		"BUILD",
		"BUILD_windows",
		"required_capabilities.json",
		filepath.Join("src", "CodingAdventures", "MyPkg.hs"),
		filepath.Join("test", "MyPkgSpec.hs"),
		filepath.Join("test", "Spec.hs"),
	} {
		if _, err := os.Stat(filepath.Join(tmpDir, path)); err != nil {
			t.Errorf("generated Haskell file %s missing: %v", path, err)
		}
	}

	build, err := os.ReadFile(filepath.Join(tmpDir, "BUILD"))
	if err != nil {
		t.Fatalf("cannot read BUILD: %v", err)
	}
	if string(build) != "cabal test\n" {
		t.Errorf("BUILD = %q, want plain cabal test", build)
	}

	capabilityPath := filepath.Join(tmpDir, "required_capabilities.json")
	capabilities, err := os.ReadFile(capabilityPath)
	if err != nil {
		t.Fatalf("cannot read required_capabilities.json: %v", err)
	}
	golden, err := os.ReadFile(
		filepath.Join("..", "..", "..", "specs", "fixtures", "scaffold-generator", "haskell_library_required_capabilities.json"),
	)
	if err != nil {
		t.Fatalf("cannot read Haskell capability golden file: %v", err)
	}
	if string(capabilities) != string(golden) {
		t.Errorf(
			"required_capabilities.json did not match golden output\n--- got ---\n%s--- want ---\n%s",
			capabilities,
			golden,
		)
	}
}

func TestDescriptionSafety(t *testing.T) {
	for _, description := range []string{
		"safe\nnext-field",
		"safe\ttab",
		"safe\x00nul",
		"safe\u0085next-field",
		"safe\u2028next-line",
		"safe */ injected",
		"safe *) injected",
		"safe %{workspace_root} injected",
	} {
		if isSafeDescription(description) {
			t.Errorf("isSafeDescription(%q) = true, want false", description)
		}
	}
	if !isSafeDescription("A printable single-line description.") {
		t.Error("isSafeDescription rejected a printable single-line description")
	}
}

func TestGenerateOcamlMatchesGoldenTrees(t *testing.T) {
	for _, pkgType := range []string{"library", "program"} {
		t.Run(pkgType, func(t *testing.T) {
			target := t.TempDir()
			if err := generateOcaml(
				target,
				"my-pkg",
				pkgType,
				"A test package",
				"",
				nil,
				nil,
				nil,
			); err != nil {
				t.Fatalf("generateOcaml: %v", err)
			}

			golden := filepath.Join(
				"..", "..", "..", "specs", "fixtures", "scaffold-generator", "ocaml-"+pkgType,
			)
			assertTreesEqual(t, target, golden)
		})
	}
}

func TestGenerateOcamlEncodesDependenciesAndDescriptions(t *testing.T) {
	target := t.TempDir()
	description := `Quotes " backslash \ hash # parens () ; backticks ` + "`" + " and $(shell) plus\u00a0space"
	if err := generateOcaml(
		target,
		"my-pkg",
		"library",
		description,
		"Layer 4 in the computing stack.",
		[]string{"graph"},
		[]string{"bitset", "graph"},
		nil,
	); err != nil {
		t.Fatalf("generateOcaml: %v", err)
	}

	opam, err := os.ReadFile(filepath.Join(target, "coding-adventures-my-pkg.opam"))
	if err != nil {
		t.Fatal(err)
	}
	opamText := string(opam)
	for _, want := range []string{
		`"coding-adventures-graph" {= "0.1.0"}`,
		"synopsis: \"Quotes \\\" backslash \\\\ hash # parens () ; backticks ` and $(shell) plus\u00a0space\"",
	} {
		if !strings.Contains(opamText, want) {
			t.Errorf("opam metadata missing encoded %q", want)
		}
	}

	build, err := os.ReadFile(filepath.Join(target, "BUILD"))
	if err != nil {
		t.Fatal(err)
	}
	buildText := string(build)
	for _, want := range []string{
		"opam pin add --no-action -y coding-adventures-bitset ../bitset",
		"opam pin add --no-action -y coding-adventures-graph ../graph",
		"opam exec -- dune build @fmt",
		"opam exec -- dune runtest --force",
		"opam exec -- bisect-ppx-report summary --per-file --expect src/coding_adventures_my_pkg.ml bisect*.coverage",
	} {
		if !strings.Contains(buildText, want) {
			t.Errorf("BUILD missing %q", want)
		}
	}
	if strings.Contains(buildText, description) {
		t.Error("BUILD must not contain user-controlled description text")
	}
	if strings.Contains(opamText, `\u00a0`) {
		t.Error("opam metadata must preserve accepted Unicode as UTF-8, not Go escapes")
	}
	duneProject, err := os.ReadFile(filepath.Join(target, "dune-project"))
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(string(duneProject), "plus\u00a0space") {
		t.Error("Dune metadata did not preserve accepted Unicode")
	}
}

func TestReadOcamlDepsUsesCheckedInOpamMetadata(t *testing.T) {
	pkgDir := filepath.Join(t.TempDir(), "my-pkg")
	if err := os.MkdirAll(pkgDir, 0o755); err != nil {
		t.Fatal(err)
	}
	metadata := `opam-version: "2.0"
name: "coding-adventures-my-pkg"
synopsis: "depends: [ \"coding-adventures-synopsis-decoy\" ]"
depends: [
  "ocaml" {= "5.2.1"}
  "coding-adventures-graph" {= "0.1.0"}
  "coding-adventures-state-machine" {= "0.1.0"}
]
build: [ "echo coding-adventures-build-decoy" ]
`
	if err := os.WriteFile(filepath.Join(pkgDir, "coding-adventures-my-pkg.opam"), []byte(metadata), 0o644); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(
		filepath.Join(pkgDir, "aaa-decoy.opam"),
		[]byte("depends: [ \"coding-adventures-wrong-file\" ]\n"),
		0o644,
	); err != nil {
		t.Fatal(err)
	}
	deps, err := readOcamlDeps(pkgDir)
	if err != nil {
		t.Fatalf("readOcamlDeps: %v", err)
	}
	if got := strings.Join(deps, ","); got != "graph,state-machine" {
		t.Fatalf("readOcamlDeps = %v", deps)
	}
}

func TestGeneratedOcamlMetadataHasNoSelfDependency(t *testing.T) {
	baseDir := t.TempDir()
	pkgDir := filepath.Join(baseDir, "my-pkg")
	if err := os.MkdirAll(pkgDir, 0o755); err != nil {
		t.Fatal(err)
	}
	if err := generateOcaml(
		pkgDir,
		"my-pkg",
		"library",
		"A test package",
		"",
		nil,
		nil,
		nil,
	); err != nil {
		t.Fatal(err)
	}

	deps, err := readOcamlDeps(pkgDir)
	if err != nil {
		t.Fatal(err)
	}
	if len(deps) != 0 {
		t.Fatalf("generated dependency metadata includes false deps: %v", deps)
	}
	closure, err := transitiveClosure([]string{"my-pkg"}, "ocaml", baseDir)
	if err != nil {
		t.Fatal(err)
	}
	order, err := topologicalSort(closure, "ocaml", baseDir)
	if err != nil {
		t.Fatal(err)
	}
	if got := strings.Join(order, ","); got != "my-pkg" {
		t.Fatalf("topological order = %q, want my-pkg", got)
	}
}

func TestScaffoldOcamlProgramResolvesLibraryDependencies(t *testing.T) {
	repoRoot := t.TempDir()
	packagesDir := filepath.Join(repoRoot, "code", "packages", "ocaml")
	leafDir := filepath.Join(packagesDir, "leaf")
	baseDir := filepath.Join(packagesDir, "base")
	for _, dir := range []string{leafDir, baseDir} {
		if err := os.MkdirAll(dir, 0o755); err != nil {
			t.Fatal(err)
		}
	}
	if err := generateOcaml(leafDir, "leaf", "library", "Leaf", "", nil, nil, nil); err != nil {
		t.Fatal(err)
	}
	if err := generateOcaml(
		baseDir,
		"base",
		"library",
		"Base",
		"",
		[]string{"leaf"},
		[]string{"leaf"},
		map[string]string{"leaf": leafDir},
	); err != nil {
		t.Fatal(err)
	}

	var stdout, stderr bytes.Buffer
	err := scaffold(scaffoldConfig{
		packageName: "tool",
		pkgType:     "program",
		directDeps:  []string{"base"},
		description: "An OCaml tool",
		repoRoot:    repoRoot,
	}, "ocaml", &stdout, &stderr)
	if err != nil {
		t.Fatalf("scaffold: %v", err)
	}

	buildPath := filepath.Join(repoRoot, "code", "programs", "ocaml", "tool", "BUILD")
	build, err := os.ReadFile(buildPath)
	if err != nil {
		t.Fatal(err)
	}
	buildText := string(build)
	leafPin := "coding-adventures-leaf ../../../packages/ocaml/leaf"
	basePin := "coding-adventures-base ../../../packages/ocaml/base"
	if !strings.Contains(buildText, leafPin) || !strings.Contains(buildText, basePin) {
		t.Fatalf("program BUILD has incorrect package pin paths:\n%s", buildText)
	}
	if strings.Index(buildText, leafPin) > strings.Index(buildText, basePin) {
		t.Fatalf("program BUILD is not leaf-first:\n%s", buildText)
	}
}

func TestScaffoldRejectsSymlinkDependency(t *testing.T) {
	repoRoot := t.TempDir()
	baseDir := filepath.Join(repoRoot, "code", "packages", "ocaml")
	if err := os.MkdirAll(baseDir, 0o755); err != nil {
		t.Fatal(err)
	}
	outside := filepath.Join(t.TempDir(), "outside")
	if err := os.MkdirAll(outside, 0o755); err != nil {
		t.Fatal(err)
	}
	depDir := filepath.Join(baseDir, "linked-dep")
	if err := os.Symlink(outside, depDir); err != nil {
		t.Skipf("symlink creation is unavailable on this platform: %v", err)
	}

	var stdout, stderr bytes.Buffer
	err := scaffold(scaffoldConfig{
		packageName: "consumer",
		pkgType:     "library",
		directDeps:  []string{"linked-dep"},
		description: "A consumer",
		dryRun:      true,
		repoRoot:    repoRoot,
	}, "ocaml", &stdout, &stderr)
	if err == nil || !strings.Contains(err.Error(), "symlink") {
		t.Fatalf("expected symlink dependency rejection, got %v", err)
	}
}

func assertTreesEqual(t *testing.T, gotRoot, wantRoot string) {
	t.Helper()
	wantFiles := map[string]string{}
	err := filepath.Walk(wantRoot, func(path string, info os.FileInfo, walkErr error) error {
		if walkErr != nil {
			return walkErr
		}
		if info.IsDir() {
			return nil
		}
		rel, relErr := filepath.Rel(wantRoot, path)
		if relErr != nil {
			return relErr
		}
		data, readErr := os.ReadFile(path)
		if readErr != nil {
			return readErr
		}
		wantFiles[filepath.ToSlash(rel)] = string(data)
		return nil
	})
	if err != nil {
		t.Fatalf("walking golden tree: %v", err)
	}

	gotFiles := map[string]string{}
	err = filepath.Walk(gotRoot, func(path string, info os.FileInfo, walkErr error) error {
		if walkErr != nil {
			return walkErr
		}
		if info.IsDir() {
			return nil
		}
		rel, relErr := filepath.Rel(gotRoot, path)
		if relErr != nil {
			return relErr
		}
		data, readErr := os.ReadFile(path)
		if readErr != nil {
			return readErr
		}
		gotFiles[filepath.ToSlash(rel)] = string(data)
		return nil
	})
	if err != nil {
		t.Fatalf("walking generated tree: %v", err)
	}

	if len(gotFiles) != len(wantFiles) {
		t.Fatalf("generated %d files, golden has %d\ngot: %v\nwant: %v", len(gotFiles), len(wantFiles), gotFiles, wantFiles)
	}
	for path, want := range wantFiles {
		if got, ok := gotFiles[path]; !ok {
			t.Errorf("generated tree missing %s", path)
		} else if got != want {
			t.Errorf("%s differs from golden\n--- got ---\n%s--- want ---\n%s", path, got, want)
		}
	}
	for path := range gotFiles {
		if _, ok := wantFiles[path]; !ok {
			t.Errorf("generated tree has unexpected %s", path)
		}
	}
}

func TestReadHaskellDepsUsesCabalProjectSiblingPaths(t *testing.T) {
	root := t.TempDir()
	pkgDir := filepath.Join(root, "http1")
	if err := os.MkdirAll(pkgDir, 0o755); err != nil {
		t.Fatal(err)
	}
	for _, sibling := range []string{"http-core", "parser"} {
		if err := os.MkdirAll(filepath.Join(root, sibling), 0o755); err != nil {
			t.Fatal(err)
		}
	}
	project := `packages: . ../http-core
          ../parser
          ../missing
          -- ../commented-out
repository: ../not-a-package
`
	if err := os.WriteFile(
		filepath.Join(pkgDir, "cabal.project"),
		[]byte(project),
		0o644,
	); err != nil {
		t.Fatal(err)
	}

	deps, err := readHaskellDeps(pkgDir)
	if err != nil {
		t.Fatalf("readHaskellDeps: %v", err)
	}
	if strings.Join(deps, ",") != "http-core,parser" {
		t.Fatalf("readHaskellDeps = %v, want [http-core parser]", deps)
	}
}

func TestGenerateJava(t *testing.T) {
	tmpDir := t.TempDir()
	err := generateJava(tmpDir, "test-pkg", "A test package", "", []string{"logic-gates"})
	if err != nil {
		t.Fatalf("generateJava: %v", err)
	}

	buildGradle, err := os.ReadFile(filepath.Join(tmpDir, "build.gradle.kts"))
	if err != nil {
		t.Fatalf("cannot read build.gradle.kts: %v", err)
	}
	if !strings.Contains(string(buildGradle), "api(\"com.codingadventures:logic-gates\")") {
		t.Error("build.gradle.kts missing direct dependency")
	}

	settingsGradle, err := os.ReadFile(filepath.Join(tmpDir, "settings.gradle.kts"))
	if err != nil {
		t.Fatalf("cannot read settings.gradle.kts: %v", err)
	}
	if !strings.Contains(string(settingsGradle), "includeBuild(\"../logic-gates\")") {
		t.Error("settings.gradle.kts missing composite build include")
	}

	sourcePath := filepath.Join(tmpDir, "src", "main", "java", "com", "codingadventures", "testpkg", "TestPkg.java")
	if _, err := os.Stat(sourcePath); err != nil {
		t.Fatalf("generated Java source missing: %v", err)
	}

	testPath := filepath.Join(tmpDir, "src", "test", "java", "com", "codingadventures", "testpkg", "TestPkgTest.java")
	if _, err := os.Stat(testPath); err != nil {
		t.Fatalf("generated Java test missing: %v", err)
	}

	capabilities, err := os.ReadFile(filepath.Join(tmpDir, "required_capabilities.json"))
	if err != nil {
		t.Fatalf("cannot read required_capabilities.json: %v", err)
	}
	if !strings.Contains(string(capabilities), "\"package\": \"java/test-pkg\"") {
		t.Error("required_capabilities.json missing package identifier")
	}
}

func TestGenerateKotlin(t *testing.T) {
	tmpDir := t.TempDir()
	err := generateKotlin(tmpDir, "test-pkg", "A test package", "", []string{"logic-gates"})
	if err != nil {
		t.Fatalf("generateKotlin: %v", err)
	}

	buildGradle, err := os.ReadFile(filepath.Join(tmpDir, "build.gradle.kts"))
	if err != nil {
		t.Fatalf("cannot read build.gradle.kts: %v", err)
	}
	if !strings.Contains(string(buildGradle), "kotlin(\"jvm\") version \"2.1.20\"") {
		t.Error("build.gradle.kts missing Kotlin JVM plugin")
	}
	if !strings.Contains(string(buildGradle), "api(\"com.codingadventures:logic-gates\")") {
		t.Error("build.gradle.kts missing direct dependency")
	}

	settingsGradle, err := os.ReadFile(filepath.Join(tmpDir, "settings.gradle.kts"))
	if err != nil {
		t.Fatalf("cannot read settings.gradle.kts: %v", err)
	}
	if !strings.Contains(string(settingsGradle), "includeBuild(\"../logic-gates\")") {
		t.Error("settings.gradle.kts missing composite build include")
	}

	sourcePath := filepath.Join(tmpDir, "src", "main", "kotlin", "com", "codingadventures", "testpkg", "TestPkg.kt")
	if _, err := os.Stat(sourcePath); err != nil {
		t.Fatalf("generated Kotlin source missing: %v", err)
	}

	testPath := filepath.Join(tmpDir, "src", "test", "kotlin", "com", "codingadventures", "testpkg", "TestPkgTest.kt")
	if _, err := os.Stat(testPath); err != nil {
		t.Fatalf("generated Kotlin test missing: %v", err)
	}

	capabilities, err := os.ReadFile(filepath.Join(tmpDir, "required_capabilities.json"))
	if err != nil {
		t.Fatalf("cannot read required_capabilities.json: %v", err)
	}
	if !strings.Contains(string(capabilities), "\"package\": \"kotlin/test-pkg\"") {
		t.Error("required_capabilities.json missing package identifier")
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

// =========================================================================
// File generation tests — C and C++ (pure ISO, iso-harness)
// =========================================================================

func TestRefusesDanglingSymlinkTarget(t *testing.T) {
	repoRoot := t.TempDir()
	baseDir := filepath.Join(repoRoot, "code", "packages", "ocaml")
	if err := os.MkdirAll(baseDir, 0o755); err != nil {
		t.Fatalf("create OCaml package directory: %v", err)
	}

	targetDir := filepath.Join(baseDir, "linked-pkg")
	if err := os.Symlink(filepath.Join(repoRoot, "missing-target"), targetDir); err != nil {
		t.Skipf("symlink creation is unavailable on this platform: %v", err)
	}

	var stdout, stderr bytes.Buffer
	err := scaffold(scaffoldConfig{
		packageName: "linked-pkg",
		pkgType:     "library",
		description: "A linked package",
		repoRoot:    repoRoot,
	}, "ocaml", &stdout, &stderr)
	if err == nil {
		t.Fatal("expected dangling symlink target to be refused")
	}
	if !strings.Contains(err.Error(), "directory already exists") {
		t.Fatalf("expected existing-directory error, got %v", err)
	}
}

func TestGenerateC(t *testing.T) {
	tmpDir := t.TempDir()
	if err := generateC(tmpDir, "ring-buf", "A ring buffer", ""); err != nil {
		t.Fatalf("generateC: %v", err)
	}

	// Header, source, and test use snake_case filenames/symbols derived from the
	// kebab package name.
	header, err := os.ReadFile(filepath.Join(tmpDir, "include", "ring_buf.h"))
	if err != nil {
		t.Fatalf("cannot read header: %v", err)
	}
	if !strings.Contains(string(header), "#ifndef RING_BUF_H") {
		t.Error("header missing include guard")
	}
	if !strings.Contains(string(header), "int ring_buf_answer(void);") {
		t.Error("header missing API declaration")
	}
	if _, err := os.Stat(filepath.Join(tmpDir, "src", "ring_buf.c")); err != nil {
		t.Error("source file missing")
	}
	test, err := os.ReadFile(filepath.Join(tmpDir, "tests", "ring_buf_test.c"))
	if err != nil {
		t.Fatalf("cannot read test: %v", err)
	}
	if !strings.Contains(string(test), "#include \"iso_test.h\"") {
		t.Error("test does not use the iso_test.h harness")
	}

	// BUILD declares the iso-harness toolchain dep and runs the POSIX script.
	build, err := os.ReadFile(filepath.Join(tmpDir, "BUILD"))
	if err != nil {
		t.Fatalf("cannot read BUILD: %v", err)
	}
	if !strings.Contains(string(build), "# build-tool: deps=c/iso-harness") {
		t.Error("BUILD missing iso-harness dependency comment")
	}
	if !strings.Contains(string(build), "sh tools/run.sh") {
		t.Error("BUILD does not invoke tools/run.sh")
	}

	// BUILD_windows drives the MSVC path via PowerShell.
	buildWin, err := os.ReadFile(filepath.Join(tmpDir, "BUILD_windows"))
	if err != nil {
		t.Fatalf("cannot read BUILD_windows: %v", err)
	}
	if !strings.Contains(string(buildWin), "tools\\run.ps1") {
		t.Error("BUILD_windows does not invoke tools/run.ps1")
	}

	// run.sh locates the harness by walking up and compiles the C sources.
	runSh, err := os.ReadFile(filepath.Join(tmpDir, "tools", "run.sh"))
	if err != nil {
		t.Fatalf("cannot read run.sh: %v", err)
	}
	if !strings.Contains(string(runSh), "code/packages/c/iso-harness") {
		t.Error("run.sh does not reference the iso-harness location")
	}
	if !strings.Contains(string(runSh), "iso_build_and_run c ring_buf-tests tests/ring_buf_test.c src/ring_buf.c") {
		t.Error("run.sh does not invoke iso_build_and_run with the expected C sources")
	}

	// _build/ is ignored so compiled artifacts never get committed.
	gitignore, err := os.ReadFile(filepath.Join(tmpDir, ".gitignore"))
	if err != nil {
		t.Fatalf("cannot read .gitignore: %v", err)
	}
	if !strings.Contains(string(gitignore), "_build/") {
		t.Error(".gitignore does not ignore _build/")
	}
}

func TestGenerateCpp(t *testing.T) {
	tmpDir := t.TempDir()
	if err := generateCpp(tmpDir, "static-vector", "A fixed-capacity vector", ""); err != nil {
		t.Fatalf("generateCpp: %v", err)
	}

	// Header-only: a .hpp under include/ with a namespaced inline API, and NO src/.
	header, err := os.ReadFile(filepath.Join(tmpDir, "include", "static_vector.hpp"))
	if err != nil {
		t.Fatalf("cannot read header: %v", err)
	}
	if !strings.Contains(string(header), "#ifndef STATIC_VECTOR_HPP") {
		t.Error("header missing include guard")
	}
	if !strings.Contains(string(header), "namespace static_vector") {
		t.Error("header missing namespace")
	}
	if _, err := os.Stat(filepath.Join(tmpDir, "src")); !os.IsNotExist(err) {
		t.Error("C++ header-only package should not create a src/ directory")
	}

	test, err := os.ReadFile(filepath.Join(tmpDir, "tests", "static_vector_test.cpp"))
	if err != nil {
		t.Fatalf("cannot read test: %v", err)
	}
	if !strings.Contains(string(test), "static_vector::answer()") {
		t.Error("test does not call the generated API")
	}

	runSh, err := os.ReadFile(filepath.Join(tmpDir, "tools", "run.sh"))
	if err != nil {
		t.Fatalf("cannot read run.sh: %v", err)
	}
	if !strings.Contains(string(runSh), "iso_build_and_run cpp static_vector-tests tests/static_vector_test.cpp") {
		t.Error("run.sh does not invoke iso_build_and_run with the expected C++ source")
	}
}

// C and C++ must be accepted by the language validator and produce packages
// under the c/ and cpp/ buckets.
func TestCFamilyLanguagesAreValid(t *testing.T) {
	for _, lang := range []string{"c", "cpp"} {
		found := false
		for _, vl := range validLanguages {
			if vl == lang {
				found = true
			}
		}
		if !found {
			t.Errorf("%q missing from validLanguages", lang)
		}
	}
}

func TestOcamlLanguageIsValid(t *testing.T) {
	for _, lang := range validLanguages {
		if lang == "ocaml" {
			return
		}
	}
	t.Fatal(`"ocaml" missing from validLanguages`)
}

// readDeps must not error for C/C++ (they have no manifest; deps live in the
// BUILD comment). It returns an empty dependency set.
func TestReadDepsCFamilyIsEmpty(t *testing.T) {
	for _, lang := range []string{"c", "cpp"} {
		deps, err := readDeps(t.TempDir(), lang)
		if err != nil {
			t.Errorf("readDeps(%q) errored: %v", lang, err)
		}
		if len(deps) != 0 {
			t.Errorf("readDeps(%q) = %v, want empty", lang, deps)
		}
	}
}
