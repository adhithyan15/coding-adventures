// =========================================================================
// scaffold-generator — Generate CI-ready package scaffolding
// =========================================================================
//
// This program generates correctly-structured, CI-ready package directories
// for the coding-adventures monorepo. It supports the repo's current
// scaffoldable language set across package ecosystems.
//
// # Why this tool exists
//
// The lessons.md file documents 12+ recurring categories of CI failures
// caused by agents hand-crafting packages inconsistently:
//
//   - Missing BUILD files
//   - TypeScript "main" pointing to dist/ instead of src/
//   - Missing transitive dependency installs in BUILD files
//   - Ruby require ordering (deps before own modules)
//   - Rust workspace Cargo.toml not updated
//   - Missing README.md or CHANGELOG.md
//
// This tool eliminates those failures. Run it, get a package that compiles,
// lints, and passes tests. Then fill in the business logic.
//
// # Architecture
//
//   scaffold-generator.json (spec)     main.go (this file)
//   ┌─────────────────────────────┐    ┌──────────────────────────────────┐
//   │ flags: -t, -l, -d, etc.    │    │ 1. Parse argv via cli-builder    │
//   │ argument: PACKAGE_NAME     │───>│ 2. Resolve dependencies          │
//   │ help, version, validation  │    │ 3. Generate files per language   │
//   └─────────────────────────────┘    └──────────────────────────────────┘
//       CLI Builder handles this            Your code handles this

package main

import (
	"encoding/json"
	"fmt"
	"io"
	"math"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strings"
	"time"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// Constants
// =========================================================================

// validLanguages lists all supported target languages.
var validLanguages = []string{"python", "go", "ruby", "typescript", "rust", "elixir", "perl", "lua", "swift", "haskell", "java", "kotlin"}

// kebabCaseRe validates that a package name is kebab-case:
// lowercase letters and digits, segments separated by single hyphens.
var kebabCaseRe = regexp.MustCompile(`^[a-z][a-z0-9]*(-[a-z0-9]+)*$`)

func intFromFloatFlag(value float64) (int, error) {
	if math.IsNaN(value) || math.IsInf(value, 0) {
		return 0, fmt.Errorf("%v is not a finite integer", value)
	}

	truncated := math.Trunc(value)
	if truncated != value {
		return 0, fmt.Errorf("%v is not an integer", value)
	}

	asInt64 := int64(truncated)
	if float64(asInt64) != truncated {
		return 0, fmt.Errorf("%v is outside the supported integer range", value)
	}

	converted := int(asInt64)
	if int64(converted) != asInt64 {
		return 0, fmt.Errorf("%v is outside the supported integer range", value)
	}

	return converted, nil
}

// =========================================================================
// Name normalization
// =========================================================================
//
// The input package name is always kebab-case (e.g., "my-package"). Each
// language has different naming conventions. These functions convert between
// them.

// toSnakeCase converts "my-package" to "my_package".
func toSnakeCase(kebab string) string {
	return strings.ReplaceAll(kebab, "-", "_")
}

// toCamelCase converts "my-package" to "MyPackage".
func toCamelCase(kebab string) string {
	parts := strings.Split(kebab, "-")
	for i, p := range parts {
		if len(p) > 0 {
			parts[i] = strings.ToUpper(p[:1]) + p[1:]
		}
	}
	return strings.Join(parts, "")
}

// toJoinedLower converts "my-package" to "mypackage" (Go package name convention).
func toJoinedLower(kebab string) string {
	return strings.ReplaceAll(kebab, "-", "")
}

// dirName returns the directory name for a package in a given language.
// Ruby, Elixir, and Lua use snake_case directories; others use kebab-case.
func dirName(kebab, lang string) string {
	switch lang {
	case "ruby", "elixir", "lua":
		return toSnakeCase(kebab)
	default:
		return kebab
	}
}

// =========================================================================
// Dependency resolution
// =========================================================================
//
// The scaffold generator reads existing packages' metadata to discover their
// dependencies, then computes the transitive closure and topological sort.
// This is the most critical feature — missing transitive deps in BUILD files
// is the #1 CI failure category.

// readDeps reads the direct local dependencies of a package by parsing its
// metadata files. Returns dependency names in kebab-case.
func readDeps(pkgDir, lang string) ([]string, error) {
	switch lang {
	case "python":
		return readPythonDeps(pkgDir)
	case "go":
		return readGoDeps(pkgDir)
	case "ruby":
		return readRubyDeps(pkgDir)
	case "typescript":
		return readTypeScriptDeps(pkgDir)
	case "rust":
		return readRustDeps(pkgDir)
	case "elixir":
		return readElixirDeps(pkgDir)
	case "perl":
		return readPerlDeps(pkgDir)
	case "lua":
		return readLuaDeps(pkgDir)
	case "swift":
		return readSwiftDeps(pkgDir)
	case "haskell":
		return readHaskellDeps(pkgDir)
	case "java":
		return readJavaDeps(pkgDir)
	case "kotlin":
		return readKotlinDeps(pkgDir)
	default:
		return nil, fmt.Errorf("unknown language: %s", lang)
	}
}

// readPythonDeps reads BUILD file for `-e ../` entries.
func readPythonDeps(pkgDir string) ([]string, error) {
	buildPath := filepath.Join(pkgDir, "BUILD")
	data, err := os.ReadFile(buildPath)
	if err != nil {
		return nil, nil // no BUILD = no deps
	}
	var deps []string
	for _, line := range strings.Split(string(data), "\n") {
		// Find ALL occurrences of -e ../ on each line (new format puts them all on one line)
		remaining := line
		for {
			idx := strings.Index(remaining, "-e ../")
			if idx < 0 {
				idx = strings.Index(remaining, "-e \"../")
			}
			if idx < 0 {
				break
			}
			// Extract the path after ../
			rest := remaining[idx:]
			if strings.HasPrefix(rest, "-e \"../") {
				rest = rest[7:] // skip `-e "../`
			} else {
				rest = rest[6:] // skip `-e ../`
			}
			// Take until space, quote, or end
			dep := ""
			for _, c := range rest {
				if c == ' ' || c == '"' || c == '\'' {
					break
				}
				dep += string(c)
			}
			if dep != "" && dep != "." {
				deps = append(deps, dep)
			}
			// Advance past this match
			remaining = remaining[idx+6:]
		}
	}
	return deps, nil
}

// readGoDeps reads go.mod replace directives for ../dep paths.
func readGoDeps(pkgDir string) ([]string, error) {
	modPath := filepath.Join(pkgDir, "go.mod")
	data, err := os.ReadFile(modPath)
	if err != nil {
		return nil, nil
	}
	var deps []string
	for _, line := range strings.Split(string(data), "\n") {
		line = strings.TrimSpace(line)
		if strings.Contains(line, "=> ../") {
			// Extract the directory name after ../
			idx := strings.Index(line, "=> ../")
			rest := line[idx+6:]
			rest = strings.TrimSpace(rest)
			// Take until whitespace or end
			dep := strings.Fields(rest)[0]
			if dep != "" {
				deps = append(deps, dep)
			}
		}
	}
	return deps, nil
}

// readRubyDeps reads the Gemfile for path dependency entries.
func readRubyDeps(pkgDir string) ([]string, error) {
	gemfilePath := filepath.Join(pkgDir, "Gemfile")
	data, err := os.ReadFile(gemfilePath)
	if err != nil {
		return nil, nil
	}
	var deps []string
	for _, line := range strings.Split(string(data), "\n") {
		// gem "coding_adventures_logic_gates", path: "../logic_gates"
		if strings.Contains(line, "path:") && strings.Contains(line, "\"../") {
			idx := strings.Index(line, "\"../")
			if idx >= 0 {
				rest := line[idx+3:]
				dep := ""
				for _, c := range rest {
					if c == '"' {
						break
					}
					dep += string(c)
				}
				// Convert snake_case dir back to kebab-case
				dep = strings.ReplaceAll(dep, "_", "-")
				if dep != "" {
					deps = append(deps, dep)
				}
			}
		}
	}
	return deps, nil
}

// readTypeScriptDeps reads package.json dependencies with "file:../" values.
func readTypeScriptDeps(pkgDir string) ([]string, error) {
	pkgJSONPath := filepath.Join(pkgDir, "package.json")
	data, err := os.ReadFile(pkgJSONPath)
	if err != nil {
		return nil, nil
	}
	var pkg map[string]any
	if err := json.Unmarshal(data, &pkg); err != nil {
		return nil, nil
	}
	depsObj, _ := pkg["dependencies"].(map[string]any)
	var deps []string
	for _, v := range depsObj {
		val, _ := v.(string)
		if strings.HasPrefix(val, "file:../") {
			dep := strings.TrimPrefix(val, "file:../")
			if dep != "" {
				deps = append(deps, dep)
			}
		}
	}
	return deps, nil
}

// readRustDeps reads Cargo.toml for path = "../dep" entries.
func readRustDeps(pkgDir string) ([]string, error) {
	cargoPath := filepath.Join(pkgDir, "Cargo.toml")
	data, err := os.ReadFile(cargoPath)
	if err != nil {
		return nil, nil
	}
	var deps []string
	for _, line := range strings.Split(string(data), "\n") {
		// pattern: dep-name = { path = "../dep-name" }
		if strings.Contains(line, "path = \"../") {
			idx := strings.Index(line, "path = \"../")
			rest := line[idx+11:]
			dep := ""
			for _, c := range rest {
				if c == '"' {
					break
				}
				dep += string(c)
			}
			if dep != "" {
				deps = append(deps, dep)
			}
		}
	}
	return deps, nil
}

// readElixirDeps reads mix.exs for path: "../dep" entries.
func readElixirDeps(pkgDir string) ([]string, error) {
	mixPath := filepath.Join(pkgDir, "mix.exs")
	data, err := os.ReadFile(mixPath)
	if err != nil {
		return nil, nil
	}
	var deps []string
	for _, line := range strings.Split(string(data), "\n") {
		// {:coding_adventures_logic_gates, path: "../logic_gates"}
		if strings.Contains(line, "path: \"../") {
			idx := strings.Index(line, "path: \"../")
			rest := line[idx+10:]
			dep := ""
			for _, c := range rest {
				if c == '"' {
					break
				}
				dep += string(c)
			}
			// Convert snake_case dir back to kebab-case
			dep = strings.ReplaceAll(dep, "_", "-")
			if dep != "" {
				deps = append(deps, dep)
			}
		}
	}
	return deps, nil
}

// readPerlDeps reads cpanfile for requires 'coding-adventures-X' entries.
func readPerlDeps(pkgDir string) ([]string, error) {
	cpanfilePath := filepath.Join(pkgDir, "cpanfile")
	data, err := os.ReadFile(cpanfilePath)
	if err != nil {
		return nil, nil // no cpanfile = no deps
	}
	re := regexp.MustCompile(`requires\s+['"]coding-adventures-([^'"]+)['"]`)
	var deps []string
	for _, line := range strings.Split(string(data), "\n") {
		m := re.FindStringSubmatch(line)
		if len(m) == 2 {
			deps = append(deps, m[1])
		}
	}
	return deps, nil
}

// readLuaDeps reads the rockspec for coding-adventures-* dependency entries.
func readLuaDeps(pkgDir string) ([]string, error) {
	// Find the rockspec file (coding-adventures-{name}-0.1.0-1.rockspec)
	entries, err := os.ReadDir(pkgDir)
	if err != nil {
		return nil, nil
	}
	var rockspecData []byte
	for _, e := range entries {
		if strings.HasSuffix(e.Name(), ".rockspec") {
			rockspecData, err = os.ReadFile(filepath.Join(pkgDir, e.Name()))
			if err != nil {
				return nil, nil
			}
			break
		}
	}
	if rockspecData == nil {
		return nil, nil
	}
	// Parse lines like: "coding-adventures-logic-gates >= 0.1.0",
	re := regexp.MustCompile(`"coding-adventures-([a-z0-9-]+)\s*>=`)
	var deps []string
	for _, line := range strings.Split(string(rockspecData), "\n") {
		m := re.FindStringSubmatch(line)
		if len(m) == 2 {
			deps = append(deps, m[1])
		}
	}
	return deps, nil
}

// swiftDepRe matches .package(path: "../dep-name") in Package.swift.
// Compiled once at package level to avoid repeated regex compilation.
var swiftDepRe = regexp.MustCompile(`\.package\s*\(\s*path\s*:\s*"\.\./([^"]+)"`)

// readSwiftDeps reads Package.swift for .package(path: "../dep-name") entries.
func readSwiftDeps(pkgDir string) ([]string, error) {
	manifestPath := filepath.Join(pkgDir, "Package.swift")
	data, err := os.ReadFile(manifestPath)
	if err != nil {
		return nil, nil // no Package.swift = no deps
	}
	var deps []string
	for _, line := range strings.Split(string(data), "\n") {
		m := swiftDepRe.FindStringSubmatch(line)
		if len(m) == 2 {
			depDir := m[1]
			// Guard against path traversal: reject any segment containing
			// a path separator or additional ".." components.
			if strings.ContainsAny(depDir, "/\\") || depDir == ".." {
				continue
			}
			deps = append(deps, depDir)
		}
	}
	return deps, nil
}

// readHaskellDeps reads cabal file for build-depends entries targeting coding-adventures-* packages.
func readHaskellDeps(pkgDir string) ([]string, error) {
	cabalPath := filepath.Join(pkgDir, "coding-adventures-"+filepath.Base(pkgDir)+".cabal")
	data, err := os.ReadFile(cabalPath)
	if err != nil {
		return nil, nil // no cabal file = no deps
	}
	re := regexp.MustCompile(`coding-adventures-([a-zA-Z0-9-]+)`)
	selfName := filepath.Base(pkgDir)
	var deps []string
	for _, line := range strings.Split(string(data), "\n") {
		m := re.FindStringSubmatch(line)
		if len(m) == 2 {
			// Ignore metadata lines and the package's own test-suite self-reference.
			if strings.Contains(line, "name:") || strings.Contains(line, "executable") || strings.Contains(line, "library") || m[1] == selfName {
				continue
			}
			deps = append(deps, m[1])
		}
	}
	return deps, nil
}

// jvmDepRe matches local composite-build dependency coordinates in Gradle.
var jvmDepRe = regexp.MustCompile(`com\.codingadventures:([a-z0-9-]+)`)

func readJavaDeps(pkgDir string) ([]string, error) {
	return readJVMDeps(pkgDir)
}

func readKotlinDeps(pkgDir string) ([]string, error) {
	return readJVMDeps(pkgDir)
}

func readJVMDeps(pkgDir string) ([]string, error) {
	buildPath := filepath.Join(pkgDir, "build.gradle.kts")
	data, err := os.ReadFile(buildPath)
	if err != nil {
		return nil, nil
	}

	seen := make(map[string]bool)
	var deps []string
	for _, line := range strings.Split(string(data), "\n") {
		matches := jvmDepRe.FindAllStringSubmatch(line, -1)
		for _, match := range matches {
			if len(match) != 2 {
				continue
			}
			dep := match[1]
			if !seen[dep] {
				seen[dep] = true
				deps = append(deps, dep)
			}
		}
	}
	return deps, nil
}

// transitiveClosure computes all transitive dependencies starting from
// the given direct dependencies. Returns the full set (not including the
// package itself).
func transitiveClosure(directDeps []string, lang, baseDir string) ([]string, error) {
	visited := make(map[string]bool)
	queue := make([]string, len(directDeps))
	copy(queue, directDeps)

	for len(queue) > 0 {
		dep := queue[0]
		queue = queue[1:]
		if visited[dep] {
			continue
		}
		visited[dep] = true

		depDir := filepath.Join(baseDir, dirName(dep, lang))
		depDeps, err := readDeps(depDir, lang)
		if err != nil {
			return nil, fmt.Errorf("reading deps of %s: %w", dep, err)
		}
		for _, dd := range depDeps {
			if !visited[dd] {
				queue = append(queue, dd)
			}
		}
	}

	result := make([]string, 0, len(visited))
	for dep := range visited {
		result = append(result, dep)
	}
	sort.Strings(result)
	return result, nil
}

// topologicalSort returns dependencies in leaf-first order (dependencies
// that have no dependencies of their own come first). This is the install
// order needed for BUILD files.
func topologicalSort(allDeps []string, lang, baseDir string) ([]string, error) {
	// Build adjacency: dep -> its deps (within the allDeps set)
	depSet := make(map[string]bool)
	for _, d := range allDeps {
		depSet[d] = true
	}

	graph := make(map[string][]string)
	inDegree := make(map[string]int)
	for _, dep := range allDeps {
		inDegree[dep] = 0
		graph[dep] = nil
	}

	for _, dep := range allDeps {
		depDir := filepath.Join(baseDir, dirName(dep, lang))
		depDeps, _ := readDeps(depDir, lang)
		for _, dd := range depDeps {
			if depSet[dd] {
				graph[dep] = append(graph[dep], dd)
				// dd is a dependency OF dep, so dep depends on dd.
				// In our sort, dd should come before dep.
			}
		}
	}

	// Kahn's algorithm: nodes with no dependencies (within set) come first
	// We want: inDegree = how many deps within the set point TO this node
	// Reset and recount
	for _, dep := range allDeps {
		inDegree[dep] = 0
	}
	for _, dep := range allDeps {
		for range graph[dep] {
			// Each entry in graph[dep] is something dep depends on,
			// so dep's in-degree increases by 1 for each dependency.
			inDegree[dep]++
		}
	}

	// Queue starts with nodes that have 0 in-degree (leaves)
	var queue []string
	for _, dep := range allDeps {
		if inDegree[dep] == 0 {
			queue = append(queue, dep)
		}
	}
	sort.Strings(queue) // deterministic output

	var result []string
	for len(queue) > 0 {
		node := queue[0]
		queue = queue[1:]
		result = append(result, node)

		// Find nodes that depend on this node and decrease their in-degree
		for _, dep := range allDeps {
			for _, dd := range graph[dep] {
				if dd == node {
					inDegree[dep]--
					if inDegree[dep] == 0 {
						queue = append(queue, dep)
						sort.Strings(queue)
					}
				}
			}
		}
	}

	if len(result) != len(allDeps) {
		return nil, fmt.Errorf("circular dependency detected: resolved %d of %d deps", len(result), len(allDeps))
	}

	return result, nil
}

// =========================================================================
// File generation — Python
// =========================================================================

func generatePython(targetDir, pkgName, description, layerCtx string, directDeps, orderedDeps []string) error {
	snake := toSnakeCase(pkgName)

	// pyproject.toml
	pyproject := fmt.Sprintf(`[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[project]
name = "coding-adventures-%s"
version = "0.1.0"
description = "%s"
requires-python = ">=3.12"
license = "MIT"
authors = [{ name = "Adhithya Rajasekaran" }]
readme = "README.md"

[project.optional-dependencies]
dev = ["pytest>=8.0", "pytest-cov>=5.0", "ruff>=0.4", "mypy>=1.10"]

[tool.hatch.build.targets.wheel]
packages = ["src/%s"]

[tool.ruff]
target-version = "py312"
line-length = 88

[tool.ruff.lint]
select = ["E", "W", "F", "I", "UP", "B", "SIM", "ANN"]

[tool.pytest.ini_options]
testpaths = ["tests"]
addopts = "--cov=%s --cov-report=term-missing --cov-fail-under=80"

[tool.coverage.run]
source = ["src/%s"]

[tool.coverage.report]
fail_under = 80
show_missing = true
`, pkgName, description, snake, snake, snake)

	// src/__init__.py
	initPy := fmt.Sprintf(`"""%s — %s

This package is part of the coding-adventures monorepo, a ground-up
implementation of the computing stack from transistors to operating systems.
%s"""

__version__ = "0.1.0"
`, pkgName, description, layerCtx)

	// tests/test_*.py
	testPy := fmt.Sprintf(`"""Tests for %s."""

from %s import __version__


class TestVersion:
    """Verify the package is importable and has a version."""

    def test_version_exists(self) -> None:
        assert __version__ == "0.1.0"
`, pkgName, snake)

	// BUILD
	var buildLines []string
	installParts := []string{"python -m pip install"}
	for _, dep := range orderedDeps {
		installParts = append(installParts, fmt.Sprintf("-e ../%s", dep))
	}
	installParts = append(installParts, "-e .[dev]", "--quiet")
	buildLines = append(buildLines, strings.Join(installParts, " "))
	buildLines = append(buildLines, "python -m pytest tests/ -v")
	build := strings.Join(buildLines, "\n") + "\n"

	// BUILD_windows
	var buildWinLines []string
	buildWinLines = append(buildWinLines, "uv venv --quiet --clear")
	if len(orderedDeps) > 0 {
		winInstallParts := []string{"uv pip install"}
		for _, dep := range orderedDeps {
			winInstallParts = append(winInstallParts, fmt.Sprintf("-e ../%s", dep))
		}
		winInstallParts = append(winInstallParts, "--quiet")
		buildWinLines = append(buildWinLines, strings.Join(winInstallParts, " "))
	}
	buildWinLines = append(buildWinLines, "uv pip install --no-deps -e .[dev] --quiet")
	buildWinLines = append(buildWinLines, "uv pip install pytest pytest-cov ruff mypy --quiet")
	buildWinLines = append(buildWinLines, "uv run --no-project python -m pytest tests/ -v")
	buildWindows := strings.Join(buildWinLines, "\n") + "\n"

	// Create directories
	srcDir := filepath.Join(targetDir, "src", snake)
	testDir := filepath.Join(targetDir, "tests")
	if err := os.MkdirAll(srcDir, 0o755); err != nil {
		return err
	}
	if err := os.MkdirAll(testDir, 0o755); err != nil {
		return err
	}

	files := map[string]string{
		"pyproject.toml": pyproject,
		filepath.Join("src", snake, "__init__.py"):  initPy,
		filepath.Join("tests", "__init__.py"):       "",
		filepath.Join("tests", "test_"+snake+".py"): testPy,
		"BUILD":         build,
		"BUILD_windows": buildWindows,
	}
	for path, content := range files {
		if err := os.WriteFile(filepath.Join(targetDir, path), []byte(content), 0o644); err != nil {
			return err
		}
	}
	return nil
}

// =========================================================================
// File generation — Go
// =========================================================================

func generateGo(targetDir, pkgName, description, layerCtx string, directDeps, allTransitiveDeps []string) error {
	goPkg := toJoinedLower(pkgName)

	// go.mod
	var goMod strings.Builder
	fmt.Fprintf(&goMod, "module github.com/adhithyan15/coding-adventures/code/packages/go/%s\n\n", pkgName)
	goMod.WriteString("go 1.26\n")

	if len(directDeps) > 0 {
		goMod.WriteString("\nrequire (\n")
		for _, dep := range directDeps {
			fmt.Fprintf(&goMod, "\tgithub.com/adhithyan15/coding-adventures/code/packages/go/%s v0.0.0\n", dep)
		}
		goMod.WriteString(")\n")

		goMod.WriteString("\nreplace (\n")
		for _, dep := range allTransitiveDeps {
			fmt.Fprintf(&goMod, "\tgithub.com/adhithyan15/coding-adventures/code/packages/go/%s => ../%s\n", dep, dep)
		}
		goMod.WriteString(")\n")
	}

	// source file
	snake := toSnakeCase(pkgName)
	srcFile := fmt.Sprintf(`// Package %s provides %s.
//
// This package is part of the coding-adventures monorepo, a ground-up
// implementation of the computing stack from transistors to operating systems.
// %s
package %s
`, goPkg, description, layerCtx, goPkg)

	// test file
	testFile := fmt.Sprintf(`package %s

import "testing"

func TestPackageLoads(t *testing.T) {
	t.Log("%s package loaded successfully")
}
`, goPkg, pkgName)

	build := "go test ./... -v -cover\n"

	files := map[string]string{
		"go.mod":           goMod.String(),
		snake + ".go":      srcFile,
		snake + "_test.go": testFile,
		"BUILD":            build,
	}
	for path, content := range files {
		if err := os.WriteFile(filepath.Join(targetDir, path), []byte(content), 0o644); err != nil {
			return err
		}
	}
	return nil
}

// =========================================================================
// File generation — Ruby
// =========================================================================

func generateRuby(targetDir, pkgName, description, layerCtx string, directDeps, allTransitiveDeps []string) error {
	snake := toSnakeCase(pkgName)
	camel := toCamelCase(pkgName)

	// gemspec
	gemspec := fmt.Sprintf(`# frozen_string_literal: true

require_relative "lib/coding_adventures/%s/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_%s"
  spec.version       = CodingAdventures::%s::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "%s"
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.3.0"

  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]

  spec.metadata = {
    "source_code_uri"        => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required"  => "true"
  }

`, snake, snake, camel, description)

	for _, dep := range directDeps {
		depSnake := toSnakeCase(dep)
		gemspec += fmt.Sprintf("  spec.add_dependency \"coding_adventures_%s\", \"~> 0.1\"\n", depSnake)
	}
	gemspec += `  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
`

	// Gemfile
	var gemfile strings.Builder
	gemfile.WriteString("# frozen_string_literal: true\n\nsource \"https://rubygems.org\"\ngemspec\n")
	if len(allTransitiveDeps) > 0 {
		gemfile.WriteString("\n# All transitive path dependencies must be listed here.\n")
		gemfile.WriteString("# Bundler needs to know where to find each gem locally.\n")
		for _, dep := range allTransitiveDeps {
			depSnake := toSnakeCase(dep)
			fmt.Fprintf(&gemfile, "gem \"coding_adventures_%s\", path: \"../%s\"\n", depSnake, depSnake)
		}
	}

	// Rakefile
	rakefile := `# frozen_string_literal: true

require "rake/testtask"

Rake::TestTask.new(:test) do |t|
  t.libs << "test"
  t.libs << "lib"
  t.test_files = FileList["test/**/test_*.rb"]
end

task default: :test
`

	// Entry point — require deps FIRST, then own modules
	var entryPoint strings.Builder
	entryPoint.WriteString("# frozen_string_literal: true\n\n")
	if len(directDeps) > 0 {
		entryPoint.WriteString("# IMPORTANT: Require dependencies FIRST, before own modules.\n")
		entryPoint.WriteString("# Ruby loads files in require order. If our modules reference\n")
		entryPoint.WriteString("# constants from dependencies, those gems must be loaded first.\n")
		for _, dep := range directDeps {
			depSnake := toSnakeCase(dep)
			fmt.Fprintf(&entryPoint, "require \"coding_adventures_%s\"\n", depSnake)
		}
		entryPoint.WriteString("\n")
	}
	fmt.Fprintf(&entryPoint, "require_relative \"coding_adventures/%s/version\"\n\n", snake)
	fmt.Fprintf(&entryPoint, "module CodingAdventures\n  # %s\n  module %s\n  end\nend\n", description, camel)

	// version.rb
	versionRb := fmt.Sprintf(`# frozen_string_literal: true

module CodingAdventures
  module %s
    VERSION = "0.1.0"
  end
end
`, camel)

	// test file
	testRb := fmt.Sprintf(`# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_%s"

class Test%s < Minitest::Test
  def test_version_exists
    refute_nil CodingAdventures::%s::VERSION
  end
end
`, snake, camel, camel)

	build := "bundle install --quiet\nbundle exec rake test\n"

	// Create directories
	libDir := filepath.Join(targetDir, "lib", "coding_adventures", snake)
	testDir := filepath.Join(targetDir, "test")
	if err := os.MkdirAll(libDir, 0o755); err != nil {
		return err
	}
	if err := os.MkdirAll(testDir, 0o755); err != nil {
		return err
	}

	files := map[string]string{
		fmt.Sprintf("coding_adventures_%s.gemspec", snake): gemspec,
		"Gemfile":  gemfile.String(),
		"Rakefile": rakefile,
		fmt.Sprintf("lib/coding_adventures_%s.rb", snake):         entryPoint.String(),
		fmt.Sprintf("lib/coding_adventures/%s/version.rb", snake): versionRb,
		filepath.Join("test", fmt.Sprintf("test_%s.rb", snake)):   testRb,
		"BUILD": build,
	}
	for path, content := range files {
		if err := os.WriteFile(filepath.Join(targetDir, path), []byte(content), 0o644); err != nil {
			return err
		}
	}
	return nil
}

// =========================================================================
// File generation — TypeScript
// =========================================================================

func generateTypeScript(targetDir, pkgName, description, layerCtx string, directDeps, orderedDeps []string) error {
	// package.json
	depsJSON := ""
	if len(directDeps) > 0 {
		var depEntries []string
		for _, dep := range directDeps {
			depEntries = append(depEntries, fmt.Sprintf("    \"@coding-adventures/%s\": \"file:../%s\"", dep, dep))
		}
		depsJSON = strings.Join(depEntries, ",\n")
	}

	packageJSON := fmt.Sprintf(`{
  "name": "@coding-adventures/%s",
  "version": "0.1.0",
  "description": "%s",
  "type": "module",
  "main": "src/index.ts",
  "scripts": {
    "build": "tsc",
    "test": "vitest run",
    "test:coverage": "vitest run --coverage"
  },
  "author": "Adhithya Rajasekaran",
  "license": "MIT",
  "dependencies": {
%s
  },
  "devDependencies": {
    "typescript": "^5.0.0",
    "vitest": "^3.0.0",
    "@vitest/coverage-v8": "^3.0.0"
  }
}
`, pkgName, description, depsJSON)

	tsconfig := `{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "outDir": "dist",
    "rootDir": "src",
    "declaration": true
  },
  "include": ["src"]
}
`

	vitestConfig := `import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    coverage: {
      provider: "v8",
      thresholds: {
        lines: 80,
      },
    },
  },
});
`

	indexTs := fmt.Sprintf(`/**
 * @coding-adventures/%s
 *
 * %s
 *
 * This package is part of the coding-adventures monorepo, a ground-up
 * implementation of the computing stack from transistors to operating systems.
 * %s
 */

export const VERSION = "0.1.0";
`, pkgName, description, layerCtx)

	testTs := fmt.Sprintf(`import { describe, it, expect } from "vitest";
import { VERSION } from "../src/index.js";

describe("%s", () => {
  it("has a version", () => {
    expect(VERSION).toBe("0.1.0");
  });
});
`, pkgName)

	// BUILD — npm ci resolves file: deps transitively
	build := "npm ci --quiet\nnpx vitest run --coverage\n"

	// Create directories
	srcDir := filepath.Join(targetDir, "src")
	testsDir := filepath.Join(targetDir, "tests")
	if err := os.MkdirAll(srcDir, 0o755); err != nil {
		return err
	}
	if err := os.MkdirAll(testsDir, 0o755); err != nil {
		return err
	}

	files := map[string]string{
		"package.json":                             packageJSON,
		"tsconfig.json":                            tsconfig,
		"vitest.config.ts":                         vitestConfig,
		filepath.Join("src", "index.ts"):           indexTs,
		filepath.Join("tests", pkgName+".test.ts"): testTs,
		"BUILD": build,
	}
	for path, content := range files {
		if err := os.WriteFile(filepath.Join(targetDir, path), []byte(content), 0o644); err != nil {
			return err
		}
	}
	return nil
}

// =========================================================================
// File generation — Rust
// =========================================================================

func generateRust(targetDir, pkgName, description, layerCtx string, directDeps []string) error {
	// Cargo.toml
	var cargo strings.Builder
	fmt.Fprintf(&cargo, `[package]
name = "%s"
version = "0.1.0"
edition = "2021"
description = "%s"

[dependencies]
`, pkgName, description)

	for _, dep := range directDeps {
		fmt.Fprintf(&cargo, "%s = { path = \"../%s\" }\n", dep, dep)
	}

	// src/lib.rs
	libRs := fmt.Sprintf(`//! # %s
//!
//! %s
//!
//! This crate is part of the coding-adventures monorepo, a ground-up
//! implementation of the computing stack from transistors to operating systems.
//! %s

#[cfg(test)]
mod tests {
    #[test]
    fn it_loads() {
        assert!(true, "%s crate loaded successfully");
    }
}
`, pkgName, description, layerCtx, pkgName)

	build := fmt.Sprintf("cargo test -p %s -- --nocapture\n", pkgName)

	// Create directories
	srcDir := filepath.Join(targetDir, "src")
	if err := os.MkdirAll(srcDir, 0o755); err != nil {
		return err
	}

	files := map[string]string{
		"Cargo.toml":                   cargo.String(),
		filepath.Join("src", "lib.rs"): libRs,
		"BUILD":                        build,
	}
	for path, content := range files {
		if err := os.WriteFile(filepath.Join(targetDir, path), []byte(content), 0o644); err != nil {
			return err
		}
	}
	return nil
}

// =========================================================================
// File generation — Elixir
// =========================================================================

func generateElixir(targetDir, pkgName, description, layerCtx string, directDeps, orderedDeps []string) error {
	snake := toSnakeCase(pkgName)
	camel := toCamelCase(pkgName)

	// mix.exs
	var mixExs strings.Builder
	fmt.Fprintf(&mixExs, `defmodule CodingAdventures.%s.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_%s,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      test_coverage: [
        summary: [threshold: 80]
      ]
    ]
  end

  def application do
    [
      extra_applications: [:logger]
    ]
  end

  defp deps do
    [
`, camel, snake)

	for i, dep := range directDeps {
		depSnake := toSnakeCase(dep)
		comma := ","
		if i == len(directDeps)-1 {
			comma = ""
		}
		fmt.Fprintf(&mixExs, "      {:coding_adventures_%s, path: \"../%s\"}%s\n", depSnake, depSnake, comma)
	}
	mixExs.WriteString("    ]\n  end\nend\n")

	// lib module
	libEx := fmt.Sprintf(`defmodule CodingAdventures.%s do
  @moduledoc """
  %s

  This module is part of the coding-adventures monorepo, a ground-up
  implementation of the computing stack from transistors to operating systems.
  %s
  """
end
`, camel, description, layerCtx)

	// test
	testExs := fmt.Sprintf(`defmodule CodingAdventures.%sTest do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.%s)
  end
end
`, camel, camel)

	testHelper := "ExUnit.start()\n"

	// BUILD — chain install transitive deps
	var build string
	if len(orderedDeps) > 0 {
		var parts []string
		for _, dep := range orderedDeps {
			depSnake := toSnakeCase(dep)
			parts = append(parts, fmt.Sprintf("cd ../%s && mix deps.get --quiet && mix compile --quiet", depSnake))
		}
		parts = append(parts, fmt.Sprintf("cd ../%s && mix deps.get --quiet && mix test --cover", snake))
		build = strings.Join(parts, " && \\\n") + "\n"
	} else {
		build = "mix deps.get --quiet && mix test --cover\n"
	}

	// Create directories
	libDir := filepath.Join(targetDir, "lib", "coding_adventures")
	testDir := filepath.Join(targetDir, "test")
	if err := os.MkdirAll(libDir, 0o755); err != nil {
		return err
	}
	if err := os.MkdirAll(testDir, 0o755); err != nil {
		return err
	}

	files := map[string]string{
		"mix.exs": mixExs.String(),
		filepath.Join("lib", "coding_adventures", snake+".ex"): libEx,
		filepath.Join("test", snake+"_test.exs"):               testExs,
		filepath.Join("test", "test_helper.exs"):               testHelper,
		"BUILD":                                                build,
	}
	for path, content := range files {
		if err := os.WriteFile(filepath.Join(targetDir, path), []byte(content), 0o644); err != nil {
			return err
		}
	}
	return nil
}

// =========================================================================
// File generation — Perl
// =========================================================================

func generatePerl(targetDir, pkgName, description, layerCtx string, directDeps, orderedDeps []string) error {
	camel := toCamelCase(pkgName)

	// Makefile.PL
	var mpl strings.Builder
	fmt.Fprintf(&mpl, "use strict;\nuse warnings;\nuse ExtUtils::MakeMaker;\n\nWriteMakefile(\n")
	fmt.Fprintf(&mpl, "    NAME             => 'CodingAdventures::%s',\n", camel)
	fmt.Fprintf(&mpl, "    VERSION_FROM     => 'lib/CodingAdventures/%s.pm',\n", camel)
	fmt.Fprintf(&mpl, "    ABSTRACT         => '%s',\n", description)
	mpl.WriteString("    AUTHOR           => 'coding-adventures',\n")
	mpl.WriteString("    LICENSE          => 'mit',\n")
	mpl.WriteString("    MIN_PERL_VERSION => '5.026000',\n")
	mpl.WriteString("    PREREQ_PM        => {\n")
	for _, dep := range directDeps {
		depCamel := toCamelCase(dep)
		fmt.Fprintf(&mpl, "        'CodingAdventures::%s' => 0,\n", depCamel)
	}
	mpl.WriteString("    },\n")
	mpl.WriteString("    TEST_REQUIRES    => {\n        'Test2::V0' => 0,\n    },\n")
	mpl.WriteString("    META_MERGE       => {\n        'meta-spec' => { version => 2 },\n")
	mpl.WriteString("        resources   => {\n            repository => {\n")
	mpl.WriteString("                type => 'git',\n")
	mpl.WriteString("                url  => 'https://github.com/adhithyan15/coding-adventures.git',\n")
	mpl.WriteString("                web  => 'https://github.com/adhithyan15/coding-adventures',\n")
	mpl.WriteString("            },\n        },\n    },\n);\n")

	// cpanfile
	var cpanfile strings.Builder
	if len(directDeps) > 0 {
		cpanfile.WriteString("# Runtime dependencies\n")
		for _, dep := range directDeps {
			fmt.Fprintf(&cpanfile, "requires 'coding-adventures-%s';\n", dep)
		}
		cpanfile.WriteString("\n")
	}
	cpanfile.WriteString("# Test dependencies\non 'test' => sub {\n    requires 'Test2::V0';\n};\n")

	// Source module
	layerLine := ""
	if layerCtx != "" {
		layerLine = fmt.Sprintf("#\n# %s\n", layerCtx)
	}
	var depImports strings.Builder
	for _, dep := range directDeps {
		fmt.Fprintf(&depImports, "use CodingAdventures::%s;\n", toCamelCase(dep))
	}
	module := fmt.Sprintf(`package CodingAdventures::%s;

# ============================================================================
# CodingAdventures::%s — %s
# ============================================================================
#
# This module is part of the coding-adventures project, an educational
# computing stack built from logic gates up through interpreters and
# compilers.
#%s#
# Usage:
#
#   use CodingAdventures::%s;
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

%s
# TODO: Implement %s

1;

__END__

=head1 NAME

CodingAdventures::%s - %s

=head1 SYNOPSIS

    use CodingAdventures::%s;

=head1 DESCRIPTION

%s

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
`, camel, camel, description, layerLine, camel, depImports.String(), camel, camel, description, camel, description)

	// t/00-load.t
	// Note: Test2::V0 does NOT export use_ok (that is a Test::More function).
	// Use eval { require ... } instead, which works with Test2::V0.
	loadT := fmt.Sprintf(`use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::%s; 1 }, 'CodingAdventures::%s loads' );

# Verify the module exports a version number.
ok(CodingAdventures::%s->VERSION, 'has a VERSION');

done_testing;
`, camel, camel, camel)

	// t/01-basic.t
	basicT := fmt.Sprintf(`use strict;
use warnings;
use Test2::V0;

use CodingAdventures::%s;

# TODO: Replace this placeholder with real tests.
ok(1, '%s module loaded successfully');

done_testing;
`, camel, camel)

	// BUILD — install transitive deps leaf-first, then this package, then test
	// Note: cpanm on Strawberry Perl (Windows) does NOT support --with-test.
	// Use plain --installdeps which reads Makefile.PL and cpanfile sections.
	var build strings.Builder
	for _, dep := range orderedDeps {
		fmt.Fprintf(&build, "cd ../%s && cpanm --installdeps --quiet .\n", dep)
	}
	build.WriteString("cpanm --installdeps --quiet .\n")
	build.WriteString("prove -l -v t/\n")

	// BUILD_windows — Perl is not tested on Windows CI (setup step skips it).
	// A no-op BUILD_windows prevents the build-tool from falling back to BUILD
	// and failing with "Unknown option: --with-test" on Strawberry Perl.
	buildWindows := "echo Perl testing is not supported on Windows - skipping\n"

	// Create directories
	libDir := filepath.Join(targetDir, "lib", "CodingAdventures")
	testDir := filepath.Join(targetDir, "t")
	if err := os.MkdirAll(libDir, 0o755); err != nil {
		return err
	}
	if err := os.MkdirAll(testDir, 0o755); err != nil {
		return err
	}

	files := map[string]string{
		"Makefile.PL": mpl.String(),
		"cpanfile":    cpanfile.String(),
		filepath.Join("lib", "CodingAdventures", camel+".pm"): module,
		filepath.Join("t", "00-load.t"):                       loadT,
		filepath.Join("t", "01-basic.t"):                      basicT,
		"BUILD":                                               build.String(),
		"BUILD_windows":                                       buildWindows,
	}
	for path, content := range files {
		if err := os.WriteFile(filepath.Join(targetDir, path), []byte(content), 0o644); err != nil {
			return err
		}
	}
	return nil
}

// =========================================================================
// File generation — Lua
// =========================================================================
//
// Lua packages follow this layout:
//
//   {snake_name}/
//     coding-adventures-{kebab}-0.1.0-1.rockspec  — LuaRocks package metadata
//     BUILD                                        — busted test runner command
//     BUILD_windows                                — same (busted works on Windows)
//     required_capabilities.json                   — capability declarations
//     src/coding_adventures/{snake}/init.lua       — main module
//     tests/test_{snake}.lua                       — test suite
//
// The BUILD file only needs one command:
//   cd tests && busted . --verbose --pattern=test_
//
// LuaRocks dep install is handled by the CI workflow, not BUILD.

func generateLua(targetDir, pkgName, description, layerCtx string, directDeps, orderedDeps []string) error {
	snake := toSnakeCase(pkgName)

	// rockspec
	var rockspec strings.Builder
	fmt.Fprintf(&rockspec, "package = %q\n", "coding-adventures-"+pkgName)
	fmt.Fprintf(&rockspec, "version = \"0.1.0-1\"\n")
	rockspec.WriteString("source = {\n    url = \"git://github.com/adhithyan15/coding-adventures.git\",\n}\n")
	rockspec.WriteString("description = {\n")
	fmt.Fprintf(&rockspec, "    summary = %q,\n", description)
	rockspec.WriteString("    license = \"MIT\",\n}\n")
	rockspec.WriteString("dependencies = {\n    \"lua >= 5.4\",\n")
	for _, dep := range directDeps {
		fmt.Fprintf(&rockspec, "    \"coding-adventures-%s >= 0.1.0\",\n", dep)
	}
	rockspec.WriteString("}\n")
	rockspec.WriteString("build = {\n    type = \"builtin\",\n    modules = {\n")
	fmt.Fprintf(&rockspec, "        [\"coding_adventures.%s\"] = \"src/coding_adventures/%s/init.lua\",\n", snake, snake)
	rockspec.WriteString("    },\n}\n")

	// BUILD — just the busted command
	build := "cd tests && busted . --verbose --pattern=test_\n"

	// required_capabilities.json
	capJSON := fmt.Sprintf(`{
  "$schema": "https://raw.githubusercontent.com/adhithyan15/coding-adventures/main/code/specs/schemas/required_capabilities.schema.json",
  "version": 1,
  "package": "lua/%s",
  "capabilities": [],
  "justification": "Pure computation. No filesystem, network, process, or environment access needed."
}
`, snake)

	// src/coding_adventures/{snake}/init.lua
	layerComment := ""
	if layerCtx != "" {
		layerComment = fmt.Sprintf("-- %s\n", layerCtx)
	}
	var depRequires strings.Builder
	for _, dep := range directDeps {
		depSnake := toSnakeCase(dep)
		fmt.Fprintf(&depRequires, "local %s = require(\"coding_adventures.%s\")\n", depSnake, depSnake)
	}
	initLua := fmt.Sprintf(`-- %s — %s
--
-- This module is part of the coding-adventures project, an educational
-- computing stack built from logic gates up through interpreters.
--%s--
-- Usage:
--
--   local m = require("coding_adventures.%s")
--
-- ============================================================================

%s
local M = {}

M.VERSION = "0.1.0"

-- TODO: Implement %s

return M
`, pkgName, description, func() string {
		if layerComment != "" {
			return "\n" + layerComment
		}
		return ""
	}(), snake, depRequires.String(), pkgName)

	// tests/test_{snake}.lua
	testLua := fmt.Sprintf(`-- Tests for %s

local m = require("coding_adventures.%s")

describe("%s", function()
    it("has a VERSION", function()
        assert.is_not_nil(m.VERSION)
        assert.equals("0.1.0", m.VERSION)
    end)

    -- TODO: Add real tests
end)
`, pkgName, snake, pkgName)

	// Create directories
	srcDir := filepath.Join(targetDir, "src", "coding_adventures", snake)
	testDir := filepath.Join(targetDir, "tests")
	if err := os.MkdirAll(srcDir, 0o755); err != nil {
		return err
	}
	if err := os.MkdirAll(testDir, 0o755); err != nil {
		return err
	}

	rockspecFilename := fmt.Sprintf("coding-adventures-%s-0.1.0-1.rockspec", pkgName)
	files := map[string]string{
		rockspecFilename:             build, // placeholder, overwritten below
		"BUILD":                      build,
		"BUILD_windows":              build,
		"required_capabilities.json": capJSON,
		filepath.Join("src", "coding_adventures", snake, "init.lua"): initLua,
		filepath.Join("tests", "test_"+snake+".lua"):                 testLua,
	}
	// Fix: rockspec content is not "build", write it separately
	files[rockspecFilename] = rockspec.String()

	for path, content := range files {
		if err := os.WriteFile(filepath.Join(targetDir, path), []byte(content), 0o644); err != nil {
			return err
		}
	}
	return nil
}

// =========================================================================
// Swift
// =========================================================================
//
// Swift Package Manager (SPM) is the official build tool shipped with Swift.
// Package structure:
//
//   my-package/
//   ├── BUILD                         # swift test --enable-code-coverage --verbose
//   ├── Package.swift                 # SPM manifest
//   ├── Sources/MyPackage/MyPackage.swift
//   └── Tests/MyPackageTests/MyPackageTests.swift
//
// Naming: directory = kebab-case; target = PascalCase; no namespace prefix.
// The same BUILD command works on macOS, Linux, and Windows.

func generateSwift(targetDir, pkgName, description, layerCtx string, directDeps []string) error {
	pascal := toCamelCase(pkgName)

	// -----------------------------------------------------------------------
	// BUILD — identical regardless of dependency count.
	// SPM resolves local path deps automatically from Package.swift.
	// -----------------------------------------------------------------------
	build := "swift test --enable-code-coverage --verbose\n"

	// -----------------------------------------------------------------------
	// Package.swift
	// -----------------------------------------------------------------------
	var pkg strings.Builder
	pkg.WriteString("// swift-tools-version: 5.9\n")
	pkg.WriteString("// ============================================================================\n")
	fmt.Fprintf(&pkg, "// Package.swift — %s\n", description)
	pkg.WriteString("// ============================================================================\n")
	pkg.WriteString("//\n")
	pkg.WriteString("// This is the Swift Package Manager manifest for this package.\n")
	pkg.WriteString("// It is part of the coding-adventures project, an educational computing stack\n")
	pkg.WriteString("// built from logic gates up through interpreters and compilers.\n")
	pkg.WriteString("//\n")
	pkg.WriteString("// Local monorepo dependencies are declared via relative path references so\n")
	pkg.WriteString("// that SPM resolves them from the local filesystem.\n")
	pkg.WriteString("//\n")
	pkg.WriteString("import PackageDescription\n\n")
	pkg.WriteString("let package = Package(\n")
	fmt.Fprintf(&pkg, "    name: %q,\n", pkgName)
	pkg.WriteString("    products: [\n")
	fmt.Fprintf(&pkg, "        .library(name: %q, targets: [%q]),\n", pascal, pascal)
	pkg.WriteString("    ],\n")

	if len(directDeps) > 0 {
		pkg.WriteString("    dependencies: [\n")
		for _, dep := range directDeps {
			fmt.Fprintf(&pkg, "        .package(path: \"../%s\"),\n", dep)
		}
		pkg.WriteString("    ],\n")
	}

	pkg.WriteString("    targets: [\n")
	pkg.WriteString("        .target(\n")
	fmt.Fprintf(&pkg, "            name: %q", pascal)
	if len(directDeps) > 0 {
		pkg.WriteString(",\n            dependencies: [\n")
		for _, dep := range directDeps {
			depPascal := toCamelCase(dep)
			fmt.Fprintf(&pkg, "                .product(name: %q, package: %q),\n", depPascal, dep)
		}
		pkg.WriteString("            ]")
	}
	pkg.WriteString("\n        ),\n")
	pkg.WriteString("        .testTarget(\n")
	fmt.Fprintf(&pkg, "            name: %q,\n", pascal+"Tests")
	fmt.Fprintf(&pkg, "            dependencies: [%q]\n", pascal)
	pkg.WriteString("        ),\n")
	pkg.WriteString("    ]\n")
	pkg.WriteString(")\n")

	// -----------------------------------------------------------------------
	// Sources/<Pascal>/<Pascal>.swift
	// -----------------------------------------------------------------------
	layerComment := ""
	if layerCtx != "" {
		layerComment = fmt.Sprintf("// %s\n//\n", layerCtx)
	}
	var depImports strings.Builder
	for _, dep := range directDeps {
		fmt.Fprintf(&depImports, "import %s\n", toCamelCase(dep))
	}
	sourceSuffix := ""
	if depImports.Len() > 0 {
		sourceSuffix = depImports.String() + "\n"
	}
	sourceSwift := fmt.Sprintf(`// %s.swift
// Part of coding-adventures — an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// %s — %s
// ============================================================================
//
%s// Usage:
//
//   import %s
//
// ============================================================================

%s/// %s is the primary type exported by this module.
///
/// TODO: Replace this stub with the real implementation.
public struct %s {
    /// Creates a new %s instance.
    public init() {}
}
`, pascal, pascal, description, layerComment, pascal, sourceSuffix, pascal, pascal, pascal)

	// -----------------------------------------------------------------------
	// Tests/<Pascal>Tests/<Pascal>Tests.swift
	// -----------------------------------------------------------------------
	testSwift := fmt.Sprintf(`import XCTest
@testable import %s

/// %sTests — unit tests for the %s module.
///
/// These are stub tests generated by the scaffold generator.
/// Replace them with real tests that exercise the actual implementation.
final class %sTests: XCTestCase {

    /// Verifies the module loads and its primary type can be instantiated.
    func testModuleLoads() {
        // This test confirms that the module compiles and its public API
        // is accessible. Replace it with meaningful tests.
        let _ = %s()
        XCTAssertTrue(true, "%s instantiated successfully")
    }
}
`, pascal, pascal, pascal, pascal, pascal, pascal)

	// -----------------------------------------------------------------------
	// required_capabilities.json
	// -----------------------------------------------------------------------
	capJSON := fmt.Sprintf(`{
  "$schema": "https://raw.githubusercontent.com/adhithyan15/coding-adventures/main/code/specs/schemas/required_capabilities.schema.json",
  "version": 1,
  "package": "swift/%s",
  "capabilities": [],
  "justification": "Pure computation. No filesystem, network, process, or environment access needed."
}
`, pkgName)

	// -----------------------------------------------------------------------
	// Write all files
	// -----------------------------------------------------------------------
	sourcePath := filepath.Join("Sources", pascal, pascal+".swift")
	testPath := filepath.Join("Tests", pascal+"Tests", pascal+"Tests.swift")

	files := map[string]string{
		"BUILD":                      build,
		"Package.swift":              pkg.String(),
		sourcePath:                   sourceSwift,
		testPath:                     testSwift,
		"required_capabilities.json": capJSON,
	}

	for path, content := range files {
		fullPath := filepath.Join(targetDir, path)
		if err := os.MkdirAll(filepath.Dir(fullPath), 0o755); err != nil {
			return err
		}
		if err := os.WriteFile(fullPath, []byte(content), 0o644); err != nil {
			return err
		}
	}
	return nil
}

// =========================================================================
// File generation — Haskell
// =========================================================================

func generateHaskell(targetDir, pkgName, description, layerCtx string, directDeps, orderedDeps []string) error {
	pkgNameHaskell := "coding-adventures-" + pkgName
	moduleName := toCamelCase(pkgName)

	cabal := fmt.Sprintf(`cabal-version: 3.0
name:          %s
version:       0.1.0
synopsis:      %s
license:       MIT
author:        Adhithya Rajasekaran
maintainer:    Adhithya Rajasekaran
build-type:    Simple

library
    exposed-modules:  %s
    build-depends:    base >=4.14
`, pkgNameHaskell, description, moduleName)

	for _, dep := range orderedDeps {
		cabal += fmt.Sprintf("                      , coding-adventures-%s\n", dep)
	}
	cabal += `    hs-source-dirs:   src
    default-language: Haskell2010

test-suite spec
    type:             exitcode-stdio-1.0
    main-is:          Spec.hs
    build-depends:    base >=4.14
                    , %s
`
	for _, dep := range orderedDeps {
		cabal += fmt.Sprintf("                    , coding-adventures-%s\n", dep)
	}
	cabal = fmt.Sprintf(cabal, pkgNameHaskell, pkgNameHaskell)
	cabal += `    hs-source-dirs:   test
    default-language: Haskell2010
`

	libHs := fmt.Sprintf(`module %s where

-- | %s
-- %s
someFunc :: IO ()
someFunc = putStrLn "someFunc"
`, moduleName, description, layerCtx)

	specHs := fmt.Sprintf(`import %s

main :: IO ()
main = do
    putStrLn "Test suite not yet implemented."
`, moduleName)

	cabalProject := "packages: ."
	for _, dep := range orderedDeps {
		cabalProject += fmt.Sprintf("\n          ../%s", dep)
	}
	cabalProject += "\n"

	// BUILD
	build := "cabal test all\n"

	files := map[string]string{
		fmt.Sprintf("%s.cabal", pkgNameHaskell): cabal,
		"cabal.project":                         cabalProject,
		"src/" + moduleName + ".hs":             libHs,
		"test/Spec.hs":                          specHs,
		"BUILD":                                 build,
	}

	for path, content := range files {
		dir := filepath.Dir(filepath.Join(targetDir, path))
		if err := os.MkdirAll(dir, 0o755); err != nil {
			return err
		}
		if err := os.WriteFile(filepath.Join(targetDir, path), []byte(content), 0o644); err != nil {
			return err
		}
	}
	return nil
}

// =========================================================================
// File generation — Java
// =========================================================================

func generateJava(targetDir, pkgName, description, layerCtx string, directDeps []string) error {
	camel := toCamelCase(pkgName)
	joined := toJoinedLower(pkgName)
	sourcePath := filepath.Join("src", "main", "java", "com", "codingadventures", joined, camel+".java")
	testPath := filepath.Join("src", "test", "java", "com", "codingadventures", joined, camel+"Test.java")

	var build strings.Builder
	build.WriteString("layout.buildDirectory = file(\"gradle-build\")\n\n")
	build.WriteString("plugins {\n")
	build.WriteString("    java\n")
	build.WriteString("    `java-library`\n")
	build.WriteString("}\n\n")
	build.WriteString("group = \"com.codingadventures\"\n")
	build.WriteString("version = \"0.1.0\"\n\n")
	build.WriteString("repositories {\n")
	build.WriteString("    mavenCentral()\n")
	build.WriteString("}\n\n")
	build.WriteString("tasks.withType<JavaCompile> {\n")
	build.WriteString("    sourceCompatibility = \"21\"\n")
	build.WriteString("    targetCompatibility = \"21\"\n")
	build.WriteString("    options.release.set(21)\n")
	build.WriteString("}\n\n")
	build.WriteString("dependencies {\n")
	for _, dep := range directDeps {
		fmt.Fprintf(&build, "    api(\"com.codingadventures:%s\")\n", dep)
	}
	build.WriteString("    testImplementation(\"org.junit.jupiter:junit-jupiter:5.11.4\")\n")
	build.WriteString("    testRuntimeOnly(\"org.junit.platform:junit-platform-launcher\")\n")
	build.WriteString("}\n\n")
	build.WriteString("tasks.test {\n")
	build.WriteString("    useJUnitPlatform()\n")
	build.WriteString("}\n")

	var settings strings.Builder
	fmt.Fprintf(&settings, "rootProject.name = %q\n", pkgName)
	for _, dep := range directDeps {
		fmt.Fprintf(&settings, "\nincludeBuild(\"../%s\")\n", dep)
	}

	layerDoc := ""
	if layerCtx != "" {
		layerDoc = fmt.Sprintf(" * <p>%s</p>\n", layerCtx)
	}
	source := fmt.Sprintf(`package com.codingadventures.%s;

/**
 * %s — %s
 *
%s */
public final class %s {
    public String ping() {
        return "%s";
    }
}
`, joined, camel, description, layerDoc, camel, pkgName)

	testSource := fmt.Sprintf(`package com.codingadventures.%s;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;

class %sTest {
    @Test
    void pingReturnsPackageName() {
        assertEquals("%s", new %s().ping());
    }
}
`, joined, camel, pkgName, camel)

	capJSON := fmt.Sprintf(`{
  "$schema": "https://raw.githubusercontent.com/adhithyan15/coding-adventures/main/code/specs/schemas/required_capabilities.schema.json",
  "version": 1,
  "package": "java/%s",
  "capabilities": [],
  "justification": "Pure in-memory library and tests. No filesystem, process, environment, or network access needed."
}
`, pkgName)

	files := map[string]string{
		".gitignore":                 "/.gradle/\n/gradle-build/\n/build/\n/out/\n",
		"BUILD":                      "gradle test\n",
		"BUILD_windows":              "gradle test\n",
		"build.gradle.kts":           build.String(),
		"settings.gradle.kts":        settings.String(),
		"required_capabilities.json": capJSON,
		sourcePath:                   source,
		testPath:                     testSource,
	}

	for path, content := range files {
		fullPath := filepath.Join(targetDir, path)
		if err := os.MkdirAll(filepath.Dir(fullPath), 0o755); err != nil {
			return err
		}
		if err := os.WriteFile(fullPath, []byte(content), 0o644); err != nil {
			return err
		}
	}
	return nil
}

// =========================================================================
// File generation — Kotlin
// =========================================================================

func generateKotlin(targetDir, pkgName, description, layerCtx string, directDeps []string) error {
	camel := toCamelCase(pkgName)
	joined := toJoinedLower(pkgName)
	sourcePath := filepath.Join("src", "main", "kotlin", "com", "codingadventures", joined, camel+".kt")
	testPath := filepath.Join("src", "test", "kotlin", "com", "codingadventures", joined, camel+"Test.kt")

	var build strings.Builder
	build.WriteString("layout.buildDirectory = file(\"gradle-build\")\n\n")
	build.WriteString("plugins {\n")
	build.WriteString("    kotlin(\"jvm\") version \"2.1.20\"\n")
	build.WriteString("    `java-library`\n")
	build.WriteString("}\n\n")
	build.WriteString("group = \"com.codingadventures\"\n")
	build.WriteString("version = \"0.1.0\"\n\n")
	build.WriteString("repositories {\n")
	build.WriteString("    mavenCentral()\n")
	build.WriteString("}\n\n")
	build.WriteString("tasks.withType<org.jetbrains.kotlin.gradle.tasks.KotlinCompile> {\n")
	build.WriteString("    compilerOptions {\n")
	build.WriteString("        jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_21)\n")
	build.WriteString("    }\n")
	build.WriteString("}\n\n")
	build.WriteString("tasks.withType<JavaCompile> {\n")
	build.WriteString("    sourceCompatibility = \"21\"\n")
	build.WriteString("    targetCompatibility = \"21\"\n")
	build.WriteString("    options.release.set(21)\n")
	build.WriteString("}\n\n")
	build.WriteString("dependencies {\n")
	for _, dep := range directDeps {
		fmt.Fprintf(&build, "    api(\"com.codingadventures:%s\")\n", dep)
	}
	build.WriteString("    testImplementation(kotlin(\"test\"))\n")
	build.WriteString("    testImplementation(\"org.junit.jupiter:junit-jupiter:5.11.4\")\n")
	build.WriteString("    testRuntimeOnly(\"org.junit.platform:junit-platform-launcher\")\n")
	build.WriteString("}\n\n")
	build.WriteString("tasks.test {\n")
	build.WriteString("    useJUnitPlatform()\n")
	build.WriteString("}\n")

	var settings strings.Builder
	fmt.Fprintf(&settings, "rootProject.name = %q\n", pkgName)
	for _, dep := range directDeps {
		fmt.Fprintf(&settings, "\nincludeBuild(\"../%s\")\n", dep)
	}

	layerDoc := ""
	if layerCtx != "" {
		layerDoc = fmt.Sprintf(" * %s\n", layerCtx)
	}
	source := fmt.Sprintf(`package com.codingadventures.%s

/**
 * %s — %s
 *
%s */
class %s {
    fun ping(): String = "%s"
}
`, joined, camel, description, layerDoc, camel, pkgName)

	testSource := fmt.Sprintf(`package com.codingadventures.%s

import kotlin.test.Test
import kotlin.test.assertEquals

class %sTest {
    @Test
    fun pingReturnsPackageName() {
        assertEquals("%s", %s().ping())
    }
}
`, joined, camel, pkgName, camel)

	capJSON := fmt.Sprintf(`{
  "$schema": "https://raw.githubusercontent.com/adhithyan15/coding-adventures/main/code/specs/schemas/required_capabilities.schema.json",
  "version": 1,
  "package": "kotlin/%s",
  "capabilities": [],
  "justification": "Pure in-memory library and tests. No filesystem, process, environment, or network access needed."
}
`, pkgName)

	files := map[string]string{
		".gitignore":                 "/.gradle/\n/gradle-build/\n/build/\n/out/\n",
		"BUILD":                      "gradle test\n",
		"BUILD_windows":              "gradle test\n",
		"build.gradle.kts":           build.String(),
		"settings.gradle.kts":        settings.String(),
		"required_capabilities.json": capJSON,
		sourcePath:                   source,
		testPath:                     testSource,
	}

	for path, content := range files {
		fullPath := filepath.Join(targetDir, path)
		if err := os.MkdirAll(filepath.Dir(fullPath), 0o755); err != nil {
			return err
		}
		if err := os.WriteFile(fullPath, []byte(content), 0o644); err != nil {
			return err
		}
	}
	return nil
}

// =========================================================================
// Common files (README, CHANGELOG)
// =========================================================================

func generateCommonFiles(targetDir, pkgName, description, lang string, layer int, directDeps []string) error {
	today := time.Now().Format("2006-01-02")

	// CHANGELOG.md
	changelog := fmt.Sprintf(`# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - %s

### Added

- Initial package scaffolding generated by scaffold-generator
`, today)

	// README.md
	var readme strings.Builder
	fmt.Fprintf(&readme, "# %s\n\n%s\n", pkgName, description)
	if layer > 0 {
		fmt.Fprintf(&readme, "\n## Layer %d\n\nThis package is part of Layer %d of the coding-adventures computing stack.\n", layer, layer)
	}
	if len(directDeps) > 0 {
		readme.WriteString("\n## Dependencies\n\n")
		for _, dep := range directDeps {
			fmt.Fprintf(&readme, "- %s\n", dep)
		}
	}

	// Language-specific install/dev instructions
	readme.WriteString("\n## Development\n\n```bash\n# Run tests\nbash BUILD\n```\n")

	files := map[string]string{
		"README.md":    readme.String(),
		"CHANGELOG.md": changelog,
	}
	for path, content := range files {
		if err := os.WriteFile(filepath.Join(targetDir, path), []byte(content), 0o644); err != nil {
			return err
		}
	}
	return nil
}

// =========================================================================
// Rust workspace integration
// =========================================================================

// updateRustWorkspace adds a new crate to the workspace Cargo.toml members list.
func updateRustWorkspace(repoRoot, pkgName string) error {
	workspacePath := filepath.Join(repoRoot, "code", "packages", "rust", "Cargo.toml")
	data, err := os.ReadFile(workspacePath)
	if err != nil {
		return fmt.Errorf("cannot read workspace Cargo.toml: %w", err)
	}

	content := string(data)

	// Check if already present
	if strings.Contains(content, fmt.Sprintf("\"%s\"", pkgName)) {
		return nil // already listed
	}

	// Find the members = [...] array and add the new crate
	// We look for the last entry before the closing ]
	membersIdx := strings.Index(content, "members = [")
	if membersIdx < 0 {
		return fmt.Errorf("cannot find members = [ in workspace Cargo.toml")
	}

	// Find the closing ] for the members array
	closingIdx := strings.Index(content[membersIdx:], "]")
	if closingIdx < 0 {
		return fmt.Errorf("cannot find closing ] for members array")
	}
	closingIdx += membersIdx

	// Insert the new member before the closing ]
	newEntry := fmt.Sprintf("  \"%s\",\n", pkgName)
	newContent := content[:closingIdx] + newEntry + content[closingIdx:]

	return os.WriteFile(workspacePath, []byte(newContent), 0o644)
}

// =========================================================================
// Main scaffolding logic
// =========================================================================

// scaffoldConfig holds the parsed and validated configuration for scaffolding.
type scaffoldConfig struct {
	packageName string
	pkgType     string // "library" or "program"
	languages   []string
	directDeps  []string
	layer       int
	description string
	dryRun      bool
	repoRoot    string
}

// findRepoRoot walks up from the current directory to find the git root.
func findRepoRoot() (string, error) {
	dir, err := os.Getwd()
	if err != nil {
		return "", err
	}
	for {
		if _, err := os.Stat(filepath.Join(dir, ".git")); err == nil {
			return dir, nil
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			return "", fmt.Errorf("not inside a git repository")
		}
		dir = parent
	}
}

// scaffold generates the package for a single language.
func scaffold(cfg scaffoldConfig, lang string, stdout, stderr io.Writer) error {
	// Determine base directory
	var baseCategory string
	if cfg.pkgType == "library" {
		baseCategory = "packages"
	} else {
		baseCategory = "programs"
	}
	baseDir := filepath.Join(cfg.repoRoot, "code", baseCategory, lang)
	dName := dirName(cfg.packageName, lang)
	targetDir := filepath.Join(baseDir, dName)

	// Check target doesn't exist
	if _, err := os.Stat(targetDir); err == nil {
		return fmt.Errorf("directory already exists: %s", targetDir)
	}

	// Validate dependencies exist
	for _, dep := range cfg.directDeps {
		depDir := filepath.Join(baseDir, dirName(dep, lang))
		if _, err := os.Stat(depDir); os.IsNotExist(err) {
			return fmt.Errorf("dependency %q not found for %s at %s", dep, lang, depDir)
		}
	}

	// Compute transitive closure and topological sort
	allDeps, err := transitiveClosure(cfg.directDeps, lang, baseDir)
	if err != nil {
		return fmt.Errorf("resolving transitive deps for %s: %w", lang, err)
	}

	orderedDeps, err := topologicalSort(allDeps, lang, baseDir)
	if err != nil {
		return fmt.Errorf("topological sort for %s: %w", lang, err)
	}

	layerCtx := ""
	if cfg.layer > 0 {
		layerCtx = fmt.Sprintf("Layer %d in the computing stack.", cfg.layer)
	}

	if cfg.dryRun {
		fmt.Fprintf(stdout, "[dry-run] Would create %s package at: %s\n", lang, targetDir)
		fmt.Fprintf(stdout, "  Direct deps: %v\n", cfg.directDeps)
		fmt.Fprintf(stdout, "  All transitive deps: %v\n", allDeps)
		fmt.Fprintf(stdout, "  Install order: %v\n", orderedDeps)
		return nil
	}

	// Create target directory
	if err := os.MkdirAll(targetDir, 0o755); err != nil {
		return err
	}

	// Generate language-specific files
	switch lang {
	case "python":
		if err := generatePython(targetDir, cfg.packageName, cfg.description, layerCtx, cfg.directDeps, orderedDeps); err != nil {
			return err
		}
	case "go":
		if err := generateGo(targetDir, cfg.packageName, cfg.description, layerCtx, cfg.directDeps, allDeps); err != nil {
			return err
		}
	case "ruby":
		if err := generateRuby(targetDir, cfg.packageName, cfg.description, layerCtx, cfg.directDeps, allDeps); err != nil {
			return err
		}
	case "typescript":
		if err := generateTypeScript(targetDir, cfg.packageName, cfg.description, layerCtx, cfg.directDeps, orderedDeps); err != nil {
			return err
		}
	case "rust":
		if err := generateRust(targetDir, cfg.packageName, cfg.description, layerCtx, cfg.directDeps); err != nil {
			return err
		}
	case "elixir":
		if err := generateElixir(targetDir, cfg.packageName, cfg.description, layerCtx, cfg.directDeps, orderedDeps); err != nil {
			return err
		}
	case "perl":
		if err := generatePerl(targetDir, cfg.packageName, cfg.description, layerCtx, cfg.directDeps, orderedDeps); err != nil {
			return err
		}
	case "lua":
		if err := generateLua(targetDir, cfg.packageName, cfg.description, layerCtx, cfg.directDeps, orderedDeps); err != nil {
			return err
		}
	case "swift":
		if err := generateSwift(targetDir, cfg.packageName, cfg.description, layerCtx, cfg.directDeps); err != nil {
			return err
		}
	case "haskell":
		if err := generateHaskell(targetDir, cfg.packageName, cfg.description, layerCtx, cfg.directDeps, orderedDeps); err != nil {
			return err
		}
	case "java":
		if err := generateJava(targetDir, cfg.packageName, cfg.description, layerCtx, cfg.directDeps); err != nil {
			return err
		}
	case "kotlin":
		if err := generateKotlin(targetDir, cfg.packageName, cfg.description, layerCtx, cfg.directDeps); err != nil {
			return err
		}
	}

	// Generate common files (README, CHANGELOG)
	if err := generateCommonFiles(targetDir, cfg.packageName, cfg.description, lang, cfg.layer, cfg.directDeps); err != nil {
		return err
	}

	// Language-specific post-generation
	fmt.Fprintf(stdout, "Created %s package at: %s\n", lang, targetDir)

	switch lang {
	case "rust":
		if err := updateRustWorkspace(cfg.repoRoot, cfg.packageName); err != nil {
			fmt.Fprintf(stderr, "  WARNING: Could not update Rust workspace: %v\n", err)
			fmt.Fprintf(stderr, "  You must manually add \"%s\" to code/packages/rust/Cargo.toml members\n", cfg.packageName)
		} else {
			fmt.Fprintf(stdout, "  Updated code/packages/rust/Cargo.toml workspace members\n")
		}
		fmt.Fprintf(stdout, "  Run: cargo build --workspace (to verify)\n")
	case "typescript":
		fmt.Fprintf(stdout, "  Run: cd %s && npm install (to generate package-lock.json)\n", targetDir)
	case "go":
		fmt.Fprintf(stdout, "  Run: cd %s && go mod tidy\n", targetDir)
		fmt.Fprintf(stdout, "  After other packages depend on this, run go mod tidy in those too\n")
	case "java", "kotlin":
		fmt.Fprintf(stdout, "  Run: cd %s && gradle test\n", targetDir)
	}

	return nil
}

// =========================================================================
// run — the testable core
// =========================================================================

func run(specPath string, argv []string, stdout, stderr io.Writer) int {
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "scaffold-generator: %s\n", err)
		return 1
	}

	result, err := parser.Parse()
	if err != nil {
		fmt.Fprintf(stderr, "%s\n", err)
		return 1
	}

	switch r := result.(type) {
	case *clibuilder.HelpResult:
		fmt.Fprintln(stdout, r.Text)
		return 0

	case *clibuilder.VersionResult:
		fmt.Fprintln(stdout, r.Version)
		return 0

	case *clibuilder.ParseResult:
		// Extract arguments and flags
		pkgName, _ := r.Arguments["package-name"].(string)
		pkgType, _ := r.Flags["type"].(string)
		if pkgType == "" {
			pkgType = "library"
		}
		langStr, _ := r.Flags["language"].(string)
		if langStr == "" {
			langStr = "all"
		}
		depsStr, _ := r.Flags["depends-on"].(string)
		layerVal, _ := r.Flags["layer"].(float64)
		description, _ := r.Flags["description"].(string)
		dryRun, _ := r.Flags["dry-run"].(bool)

		// Validate package name
		if !kebabCaseRe.MatchString(pkgName) {
			fmt.Fprintf(stderr, "scaffold-generator: invalid package name %q (must be kebab-case: lowercase, digits, hyphens)\n", pkgName)
			return 1
		}

		// Validate description: no newlines or control characters that could
		// corrupt generated source files (e.g. Swift block-comment terminators
		// or JSON escape sequences).
		if strings.ContainsAny(description, "\n\r") {
			fmt.Fprintf(stderr, "scaffold-generator: description must not contain newline characters\n")
			return 1
		}

		// Parse languages
		var languages []string
		if langStr == "all" {
			languages = validLanguages
		} else {
			for _, l := range strings.Split(langStr, ",") {
				l = strings.TrimSpace(l)
				valid := false
				for _, vl := range validLanguages {
					if l == vl {
						valid = true
						break
					}
				}
				if !valid {
					fmt.Fprintf(stderr, "scaffold-generator: unknown language %q (valid: %s)\n", l, strings.Join(validLanguages, ", "))
					return 1
				}
				languages = append(languages, l)
			}
		}

		// Parse dependencies
		var directDeps []string
		if depsStr != "" {
			for _, d := range strings.Split(depsStr, ",") {
				d = strings.TrimSpace(d)
				if d != "" {
					if !kebabCaseRe.MatchString(d) {
						fmt.Fprintf(stderr, "scaffold-generator: invalid dependency name %q (must be kebab-case)\n", d)
						return 1
					}
					directDeps = append(directDeps, d)
				}
			}
		}

		// Find repo root
		repoRoot, err := findRepoRoot()
		if err != nil {
			fmt.Fprintf(stderr, "scaffold-generator: %s\n", err)
			return 1
		}

		layer, err := intFromFloatFlag(layerVal)
		if err != nil {
			fmt.Fprintf(stderr, "scaffold-generator: invalid layer value: %s\n", err)
			return 1
		}

		cfg := scaffoldConfig{
			packageName: pkgName,
			pkgType:     pkgType,
			languages:   languages,
			directDeps:  directDeps,
			layer:       layer,
			description: description,
			dryRun:      dryRun,
			repoRoot:    repoRoot,
		}

		// Scaffold for each language
		hadError := false
		for _, lang := range languages {
			if err := scaffold(cfg, lang, stdout, stderr); err != nil {
				fmt.Fprintf(stderr, "scaffold-generator [%s]: %s\n", lang, err)
				hadError = true
			}
		}

		if hadError {
			return 1
		}
		return 0

	default:
		fmt.Fprintf(stderr, "scaffold-generator: unexpected result type: %T\n", result)
		return 1
	}
}

// =========================================================================
// Main
// =========================================================================

func main() {
	// Resolve spec path relative to the executable
	execPath, err := os.Executable()
	if err != nil {
		fmt.Fprintf(os.Stderr, "scaffold-generator: cannot determine executable path: %s\n", err)
		os.Exit(1)
	}
	execDir := filepath.Dir(execPath)
	specPath := filepath.Join(execDir, "scaffold-generator.json")

	// Fallback: try relative to working directory (for development)
	if _, err := os.Stat(specPath); os.IsNotExist(err) {
		specPath = filepath.Join(".", "scaffold-generator.json")
		if _, err := os.Stat(specPath); os.IsNotExist(err) {
			// Try going up to the programs directory
			wd, _ := os.Getwd()
			specPath = filepath.Join(wd, "..", "..", "scaffold-generator.json")
		}
	}

	os.Exit(run(specPath, os.Args, os.Stdout, os.Stderr))
}
