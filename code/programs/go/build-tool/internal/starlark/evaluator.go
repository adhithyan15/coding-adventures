// Package starlark evaluates Starlark BUILD files using the Go
// starlark-interpreter package.
//
// # Why Starlark BUILD files?
//
// Traditional BUILD files in this monorepo are shell scripts — each line
// is a command executed sequentially. This works but has limitations:
//
//   - No change detection metadata: the build tool guesses which files
//     matter based on file extensions, not explicit declarations.
//   - No dependency declarations: deps are parsed from language-specific
//     config files (pyproject.toml, go.mod, etc.) with heuristic matching.
//   - No validation: a typo in a BUILD file only surfaces at build time.
//
// Starlark BUILD files solve all three. They're real programs that declare
// targets with explicit srcs, deps, and build metadata. The build tool
// evaluates them using its built-in Starlark interpreter and extracts the
// declared targets.
//
// # How evaluation works
//
//  1. Read the BUILD file contents.
//  2. Create a Starlark interpreter with:
//     - A file resolver rooted at the repo root (for load() statements).
//     - A glob() builtin that lists files relative to the package directory.
//  3. Execute the BUILD file through the interpreter pipeline:
//     source → lexer → parser → compiler → VM → result
//  4. Extract the _targets list from the result's variables.
//  5. Convert each target dict to a Target struct.
//
// # Detecting Starlark vs shell BUILD files
//
// We use a simple heuristic: if the BUILD file's first non-comment,
// non-blank line starts with "load(" or matches a known rule call pattern
// (like "py_library("), it's Starlark. Otherwise it's shell.
package starlark

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	interpreter "github.com/adhithyan15/coding-adventures/code/packages/go/starlark-interpreter"
	starlarkvm "github.com/adhithyan15/coding-adventures/code/packages/go/starlark-vm"
)

// CtxSchemaVersion is the version of the _ctx build context schema.
// Bump this when making breaking changes to the _ctx dict structure.
// See spec 15-os-aware-build-rules.md for versioning rules.
const CtxSchemaVersion = 1

// Target represents a single build target declared in a Starlark BUILD file.
// Each call to py_library(), go_library(), etc. produces one Target.
type Target struct {
	Rule        string        // Rule type: "py_library", "go_binary", etc.
	Name        string        // Target name: "starlark-vm", "build-tool", etc.
	Srcs        []string      // Declared source file patterns for change detection
	Deps        []string      // Dependencies as "language/package-name" strings
	TestRunner  string        // Test framework: "pytest", "vitest", "minitest", etc.
	EntryPoint  string        // Binary entry point: "main.py", "src/index.ts", etc.
	Commands    []interface{} // Structured command dicts from cmd.star (nil = use GenerateCommands)
}

// BuildResult holds the targets extracted from evaluating a Starlark BUILD file.
type BuildResult struct {
	Targets []Target
}

// IsStarlarkBuild checks whether a BUILD file contains Starlark code
// (as opposed to shell commands). We look for Starlark-specific patterns
// in the first few significant lines.
//
// Starlark indicators:
//   - load("...") statements
//   - Known rule function calls: py_library(, go_library(, etc.
//   - def statements (function definitions)
//
// If none of these are found, we treat it as a shell BUILD file.
func IsStarlarkBuild(content string) bool {
	knownRules := []string{
		"py_library(", "py_binary(",
		"go_library(", "go_binary(",
		"ruby_library(", "ruby_binary(",
		"ts_library(", "ts_binary(",
		"rust_library(", "rust_binary(",
		"elixir_library(", "elixir_binary(",
		"perl_library(", "perl_binary(",
		"java_library(", "java_binary(",
		"kotlin_library(", "kotlin_binary(",
		"dotnet_library(", "dotnet_binary(",
	}

	for _, line := range strings.Split(content, "\n") {
		trimmed := strings.TrimSpace(line)

		// Skip blank lines and comments.
		if trimmed == "" || strings.HasPrefix(trimmed, "#") {
			continue
		}

		// Check for Starlark patterns.
		if strings.HasPrefix(trimmed, "load(") {
			return true
		}
		if strings.HasPrefix(trimmed, "def ") {
			return true
		}
		for _, rule := range knownRules {
			if strings.HasPrefix(trimmed, rule) {
				return true
			}
		}

		// If we've seen a non-comment, non-blank line that doesn't match
		// any Starlark pattern, it's probably shell. Stop checking.
		break
	}

	return false
}

// EvaluateBuildFile runs the Starlark interpreter on a BUILD file and
// extracts the declared targets. The pkgDir is used to resolve glob()
// patterns, and repoRoot is used to resolve load() file paths.
//
// Returns an error if the file cannot be read, parsed, or evaluated.
func EvaluateBuildFile(buildFilePath, pkgDir, repoRoot string) (*BuildResult, error) {
	content, err := os.ReadFile(buildFilePath)
	if err != nil {
		return nil, fmt.Errorf("reading BUILD file: %w", err)
	}

	source := string(content)

	// Create a file resolver that resolves load() paths relative to repoRoot.
	// load("code/packages/starlark/library-rules/python_library.star", "py_library")
	// resolves to <repoRoot>/code/packages/starlark/library-rules/python_library.star
	fileResolver := func(label string) (string, error) {
		// The label is a real filesystem path relative to the repo root.
		// filepath.FromSlash ensures forward slashes in load() labels
		// work correctly on Windows (where the separator is \).
		fullPath := filepath.Join(repoRoot, filepath.FromSlash(label))
		data, err := os.ReadFile(fullPath)
		if err != nil {
			return "", fmt.Errorf("load(%q): %w", label, err)
		}
		return string(data), nil
	}

	// Create the interpreter with the file resolver.
	interp := interpreter.NewInterpreter(
		interpreter.WithFileResolver(fileResolver),
	)

	// Execute the BUILD file.  The VM may panic on type errors or
	// unsupported operations — recover and return as an error.
	var result *starlarkvm.StarlarkResult
	var evalErr error
	func() {
		defer func() {
			if r := recover(); r != nil {
				evalErr = fmt.Errorf("VM panic: %v", r)
			}
		}()
		result, err = interp.Interpret(source)
		if err != nil {
			evalErr = fmt.Errorf("evaluating BUILD file %s: %w", buildFilePath, err)
		}
	}()
	if evalErr != nil {
		return nil, evalErr
	}

	// Extract _targets from the result's variables.
	targets, err := extractTargets(result.Variables)
	if err != nil {
		return nil, fmt.Errorf("extracting targets from %s: %w", buildFilePath, err)
	}

	return &BuildResult{Targets: targets}, nil
}

// extractTargets converts the _targets list from the Starlark result
// into a slice of Target structs. Each element in _targets should be
// a dict with keys: rule, name, srcs, deps, and optionally test_runner
// and entry_point.
func extractTargets(variables map[string]interface{}) ([]Target, error) {
	rawTargets, ok := variables["_targets"]
	if !ok {
		// No _targets variable — the BUILD file didn't declare any targets.
		// This is valid (e.g., a BUILD file that only defines helper functions).
		return nil, nil
	}

	targetList, ok := rawTargets.([]interface{})
	if !ok {
		return nil, fmt.Errorf("_targets is not a list (got %T)", rawTargets)
	}

	var targets []Target
	for i, raw := range targetList {
		dict, ok := raw.(map[string]interface{})
		if !ok {
			return nil, fmt.Errorf("_targets[%d] is not a dict (got %T)", i, raw)
		}

		t := Target{
			Rule:       getString(dict, "rule"),
			Name:       getString(dict, "name"),
			Srcs:       getStringList(dict, "srcs"),
			Deps:       getStringList(dict, "deps"),
			TestRunner: getString(dict, "test_runner"),
			EntryPoint: getString(dict, "entry_point"),
			Commands:   getInterfaceList(dict, "commands"),
		}
		targets = append(targets, t)
	}

	return targets, nil
}

// getString safely extracts a string value from a dict.
// Returns "" if the key doesn't exist or isn't a string.
func getString(dict map[string]interface{}, key string) string {
	v, ok := dict[key]
	if !ok {
		return ""
	}
	s, ok := v.(string)
	if !ok {
		return ""
	}
	return s
}

// getStringList safely extracts a []string from a dict.
// Returns nil if the key doesn't exist or isn't a list.
func getStringList(dict map[string]interface{}, key string) []string {
	v, ok := dict[key]
	if !ok {
		return nil
	}
	list, ok := v.([]interface{})
	if !ok {
		return nil
	}
	var result []string
	for _, item := range list {
		if s, ok := item.(string); ok {
			result = append(result, s)
		}
	}
	return result
}

// getInterfaceList safely extracts a []interface{} from a dict.
// Returns nil if the key doesn't exist or isn't a list.
// Used for the "commands" field which contains structured command dicts.
func getInterfaceList(dict map[string]interface{}, key string) []interface{} {
	v, ok := dict[key]
	if !ok {
		return nil
	}
	list, ok := v.([]interface{})
	if !ok {
		return nil
	}
	return list
}

// EnhanceInstallCommands replaces generic install commands in a rendered
// command list with auto-discovered versions that include monorepo deps.
//
// When Starlark rules produce commands via cmd(), the install command is
// generic (e.g., "uv pip install --system -e .[dev]").  This function
// detects such commands and replaces them with the full transitive version
// that includes -e flags for every monorepo dep.
//
// This allows Starlark rules to declare *intent* (install deps) while the
// build tool handles the *mechanics* of monorepo dep discovery.
func EnhanceInstallCommands(cmds []string, pkgPath string) []string {
	if pkgPath == "" || len(cmds) == 0 {
		return cmds
	}

	result := make([]string, len(cmds))
	copy(result, cmds)

	for i, cmd := range result {
		// Replace generic Python install with transitive-aware version.
		if strings.HasPrefix(cmd, "uv pip install") && !strings.Contains(cmd, "-e ../") {
			enhanced := pyInstallCmd(pkgPath)
			if enhanced != cmd {
				result[i] = enhanced
			}
		}
		// Replace generic npm install with transitive-aware version.
		if cmd == "npm install --silent" || cmd == "npm install" {
			enhanced := tsInstallCmd(pkgPath)
			if enhanced != cmd {
				result[i] = enhanced
			}
		}
	}

	return result
}

// GenerateCommands produces shell commands for a target, using the package
// path to auto-discover monorepo deps from the language's dep file
// (pyproject.toml, package.json, Gemfile, go.mod, mix.exs, Cargo.toml).
//
// The pkgPath parameter is the absolute path to the package directory.
// When empty, the function falls back to generic commands without dep
// installation (backward compatibility).
func GenerateCommands(t Target, pkgPath string) []string {
	switch t.Rule {
	case "py_library":
		runner := t.TestRunner
		if runner == "" {
			runner = "pytest"
		}
		installCmd := pyInstallCmd(pkgPath)
		if runner == "pytest" {
			return []string{
				installCmd,
				"python -m pytest --cov --cov-report=term-missing",
			}
		}
		return []string{
			installCmd,
			"python -m unittest discover tests/",
		}

	case "py_binary":
		return []string{
			pyInstallCmd(pkgPath),
			"python -m pytest --cov --cov-report=term-missing",
		}

	case "go_library", "go_binary":
		return []string{
			"go build ./...",
			"go test ./... -v -cover",
			"go vet ./...",
		}

	case "ruby_library", "ruby_binary":
		return []string{
			"bundle install --quiet",
			"bundle exec rake test",
		}

	case "ts_library", "ts_binary":
		installCmd := tsInstallCmd(pkgPath)
		return []string{
			installCmd,
			"npx vitest run --coverage",
		}

	case "rust_library", "rust_binary":
		return []string{
			"cargo build",
			"cargo test",
		}

	case "elixir_library", "elixir_binary":
		return []string{
			"mix deps.get",
			"mix test --cover",
		}

	case "perl_library", "perl_binary":
		return []string{
			"cpanm --installdeps --quiet .",
			"prove -l -v t/",
		}

	case "java_library", "java_binary", "kotlin_library", "kotlin_binary":
		return []string{
			"gradle build",
			"gradle test",
		}

	case "dotnet_library", "dotnet_binary":
		// --disable-build-servers prevents the MSBuild background server from
		// persisting between steps, which causes port-binding failures on CI
		// when the same runner executes multiple build invocations.
		return []string{
			"dotnet build --disable-build-servers",
			"dotnet test --disable-build-servers",
		}

	default:
		return []string{fmt.Sprintf("echo 'Unknown rule: %s'", t.Rule)}
	}
}

// pyInstallCmd reads pyproject.toml to discover monorepo deps and builds
// the uv pip install command with all of them as editable installs.
//
// For a package with pyproject.toml containing:
//
//	dependencies = ["coding-adventures-lexer", "coding-adventures-grammar-tools"]
//
// This produces:
//
//	uv pip install --system -e ../lexer -e ../grammar-tools -e ".[dev]"
//
// This auto-discovery replaces the need to manually list deps in BUILD files.
//
// The discovery is transitive: if A depends on B and B depends on C, the
// resulting command installs C, B, then A.  This mirrors what the old shell
// BUILD files did manually with explicit -e flags for every transitive dep.
func pyInstallCmd(pkgPath string) string {
	if pkgPath == "" {
		return `uv pip install --system -e ".[dev]"`
	}

	// Collect all transitive monorepo deps by walking the dep tree.
	parentDir := filepath.Dir(pkgPath)
	allDeps := collectPyTransitiveDeps(pkgPath, parentDir, make(map[string]bool))
	if len(allDeps) == 0 {
		return `uv pip install --system -e ".[dev]"`
	}

	args := "uv pip install --system"
	for _, dep := range allDeps {
		args += " -e ../" + dep
	}
	args += ` -e ".[dev]"`
	return args
}

// collectPyTransitiveDeps walks the monorepo dep tree starting from pkgPath.
// It reads each package's pyproject.toml, finds coding-adventures-* deps,
// and recurses into each.  Returns a flat list in leaf-first order (deps
// before dependents) with no duplicates.
func collectPyTransitiveDeps(pkgPath, parentDir string, visited map[string]bool) []string {
	pyproject := filepath.Join(pkgPath, "pyproject.toml")
	data, err := os.ReadFile(pyproject)
	if err != nil {
		return nil
	}

	directDeps := extractPyMonorepoDeps(string(data))
	var result []string

	for _, dep := range directDeps {
		if visited[dep] {
			continue
		}
		visited[dep] = true

		// Recurse into the dep's own pyproject.toml.
		depPath := filepath.Join(parentDir, dep)
		transitive := collectPyTransitiveDeps(depPath, parentDir, visited)
		result = append(result, transitive...)
		result = append(result, dep)
	}

	return result
}

// extractPyMonorepoDeps parses pyproject.toml and returns relative dep
// directory names for monorepo dependencies.
//
// It finds entries like "coding-adventures-lexer>=0.1.0" in the
// [project] dependencies array and extracts "lexer" as the dir name.
func extractPyMonorepoDeps(text string) []string {
	const prefix = "coding-adventures-"
	var deps []string

	inDeps := false
	for _, line := range strings.Split(text, "\n") {
		trimmed := strings.TrimSpace(line)

		if !inDeps {
			if strings.HasPrefix(trimmed, "dependencies") && strings.Contains(trimmed, "=") {
				afterEq := strings.SplitN(trimmed, "=", 2)[1]
				afterEq = strings.TrimSpace(afterEq)
				if strings.HasPrefix(afterEq, "[") {
					if strings.Contains(afterEq, "]") {
						// Single-line array.
						deps = append(deps, extractMonorepoPyNames(afterEq, prefix)...)
						break
					}
					inDeps = true
					deps = append(deps, extractMonorepoPyNames(afterEq, prefix)...)
				}
			}
			continue
		}

		if strings.Contains(trimmed, "]") {
			deps = append(deps, extractMonorepoPyNames(trimmed, prefix)...)
			break
		}
		deps = append(deps, extractMonorepoPyNames(trimmed, prefix)...)
	}

	return deps
}

// extractMonorepoPyNames finds "coding-adventures-*" dep names in a line
// and returns the package directory names (the part after the prefix).
func extractMonorepoPyNames(line, prefix string) []string {
	var names []string
	// Find quoted strings.
	inQuote := false
	quoteChar := byte(0)
	start := 0
	for i := 0; i < len(line); i++ {
		c := line[i]
		if !inQuote && (c == '"' || c == '\'') {
			inQuote = true
			quoteChar = c
			start = i + 1
		} else if inQuote && c == quoteChar {
			depStr := line[start:i]
			// Strip version specifier: split on any of >=<!~; or space.
			for j := 0; j < len(depStr); j++ {
				if depStr[j] == '>' || depStr[j] == '<' || depStr[j] == '=' ||
					depStr[j] == '!' || depStr[j] == '~' || depStr[j] == ';' ||
					depStr[j] == ' ' {
					depStr = depStr[:j]
					break
				}
			}
			depStr = strings.TrimSpace(strings.ToLower(depStr))
			if strings.HasPrefix(depStr, prefix) {
				dirName := strings.TrimPrefix(depStr, prefix)
				if dirName != "" {
					names = append(names, dirName)
				}
			}
			inQuote = false
		}
	}
	return names
}

// tsInstallCmd reads package.json to find monorepo file: deps and chains
// npm install commands for all transitive deps in leaf-first order.
//
// TypeScript packages use "file:../sibling-pkg" references in package.json.
// Each dep (and its transitive deps) needs npm install before the parent
// can resolve it.  This mirrors what the old shell BUILD files did with
// explicit chained install commands.
func tsInstallCmd(pkgPath string) string {
	if pkgPath == "" {
		return "npm install --silent"
	}

	parentDir := filepath.Dir(pkgPath)
	allDeps := collectTsTransitiveDeps(pkgPath, parentDir, make(map[string]bool))
	if len(allDeps) == 0 {
		return "npm install --silent"
	}

	// Chain: cd into each dep dir (leaf-first) and npm install.
	cmds := ""
	for _, dep := range allDeps {
		cmds += fmt.Sprintf("cd ../%s && npm install --silent && cd - > /dev/null && ", dep)
	}
	cmds += "npm install --silent"
	return cmds
}

// collectTsTransitiveDeps walks the file: dep tree from a package.json.
// Returns dep directory names in leaf-first order with no duplicates.
func collectTsTransitiveDeps(pkgPath, parentDir string, visited map[string]bool) []string {
	directDeps := extractTsFileDeps(pkgPath)
	var result []string

	for _, dep := range directDeps {
		if visited[dep] {
			continue
		}
		visited[dep] = true

		depPath := filepath.Join(parentDir, dep)
		transitive := collectTsTransitiveDeps(depPath, parentDir, visited)
		result = append(result, transitive...)
		result = append(result, dep)
	}

	return result
}

// extractTsFileDeps reads package.json and returns directory names for
// "file:../" references in dependencies and devDependencies.
func extractTsFileDeps(pkgPath string) []string {
	pkgJSON := filepath.Join(pkgPath, "package.json")
	data, err := os.ReadFile(pkgJSON)
	if err != nil {
		return nil
	}

	var depDirs []string
	for _, line := range strings.Split(string(data), "\n") {
		trimmed := strings.TrimSpace(line)
		if strings.Contains(trimmed, `"file:../`) {
			idx := strings.Index(trimmed, `"file:../`)
			if idx >= 0 {
				rest := trimmed[idx+len(`"file:../`):]
				end := strings.Index(rest, `"`)
				if end > 0 {
					depDirs = append(depDirs, rest[:end])
				}
			}
		}
	}

	return depDirs
}
