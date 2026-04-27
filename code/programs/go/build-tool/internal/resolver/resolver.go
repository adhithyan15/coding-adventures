// Package resolver reads package metadata files (pyproject.toml, .gemspec,
// go.mod, package.json, .rockspec, pubspec.yaml) and extracts internal dependencies,
// building a directed graph.
//
// # Why dependency resolution matters
//
// In a monorepo, packages often depend on each other. If package B depends
// on package A, we must build A before B. The resolver reads each package's
// metadata file to discover these relationships, then encodes them as edges
// in a directed graph.
//
// # Dependency naming conventions
//
// Each language ecosystem uses a different naming convention for packages:
//
//   - Python: pyproject.toml uses "coding-adventures-" prefix with hyphens.
//     "coding-adventures-logic-gates" maps to "python/logic-gates".
//
//   - Ruby: .gemspec uses "coding_adventures_" prefix with underscores.
//     "coding_adventures_logic_gates" maps to "ruby/logic_gates".
//
//   - Go: go.mod uses full module paths. We map based on the last path
//     component: "go/directed-graph".
//
//   - TypeScript: package.json uses "@coding-adventures/" scoped npm names.
//     "@coding-adventures/logic-gates" maps to "typescript/logic-gates".
//
//   - Dart: pubspec.yaml uses snake_case package names.
//     "coding_adventures_logic_gates" maps to "dart/logic-gates".
//
// External dependencies (those not matching the monorepo prefix) are
// silently skipped — we only care about internal build ordering.
//
// # The directed graph
//
// We use the directed-graph package from this repo. Edges go FROM
// dependency TO dependent: if B depends on A, the edge is A → B.
// This convention means "A must be built before B", and
// IndependentGroups() naturally produces the correct build order.
package resolver

import (
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strings"

	directedgraph "github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

// parsePythonDeps extracts internal dependencies from a Python pyproject.toml.
//
// We use Go's built-in tomllib equivalent approach: since we only need the
// [project] dependencies list, we parse it with simple string scanning.
// This avoids adding a TOML library dependency for what amounts to reading
// a single array of strings.
//
// The parsing strategy:
//  1. Find the "dependencies = [" line
//  2. Collect lines until we hit "]"
//  3. Extract quoted strings and strip version specifiers
func parsePythonDeps(pkg discovery.Package, knownNames map[string]string) []string {
	pyproject := filepath.Join(pkg.Path, "pyproject.toml")
	data, err := os.ReadFile(pyproject)
	if err != nil {
		return nil
	}

	text := string(data)
	var internalDeps []string

	// Strategy: find dependencies = [...] and extract the entries.
	// We handle both single-line: dependencies = ["foo", "bar"]
	// and multi-line:
	//   dependencies = [
	//       "foo",
	//       "bar",
	//   ]
	inDeps := false
	for _, line := range strings.Split(text, "\n") {
		trimmed := strings.TrimSpace(line)

		if !inDeps {
			// Look for the start of the dependencies array.
			if strings.HasPrefix(trimmed, "dependencies") && strings.Contains(trimmed, "=") {
				// Extract everything after the '='
				afterEq := strings.SplitN(trimmed, "=", 2)[1]
				afterEq = strings.TrimSpace(afterEq)

				if strings.HasPrefix(afterEq, "[") {
					// Could be single-line: dependencies = ["foo", "bar"]
					if strings.Contains(afterEq, "]") {
						// Single-line array
						extractDeps(afterEq, knownNames, &internalDeps)
						continue
					}
					// Multi-line array starts here
					inDeps = true
					extractDeps(afterEq, knownNames, &internalDeps)
				}
			}
			continue
		}

		// We're inside a multi-line dependencies array.
		if strings.Contains(trimmed, "]") {
			extractDeps(trimmed, knownNames, &internalDeps)
			inDeps = false
			continue
		}
		extractDeps(trimmed, knownNames, &internalDeps)
	}

	return internalDeps
}

// extractDeps finds quoted dependency names in a line and maps them to
// internal package names. Version specifiers (>=, <, etc.) are stripped.
func extractDeps(line string, knownNames map[string]string, deps *[]string) {
	// Match quoted strings: "something" or 'something'
	re := regexp.MustCompile(`["']([^"']+)["']`)
	matches := re.FindAllStringSubmatch(line, -1)
	for _, match := range matches {
		if len(match) < 2 {
			continue
		}
		// Strip version specifiers: split on >=, <=, >, <, ==, !=, ~=, ;, spaces
		depName := regexp.MustCompile(`[>=<!~\s;]`).Split(match[1], 2)[0]
		depName = strings.TrimSpace(strings.ToLower(depName))
		if pkgName, ok := knownNames[depName]; ok {
			*deps = append(*deps, pkgName)
		}
	}
}

// parseRubyDeps extracts internal dependencies from a Ruby .gemspec file.
//
// Ruby gemspecs declare dependencies with:
//
//	spec.add_dependency "coding_adventures_logic_gates"
//
// We use a regex to find these lines and map the gem names to our
// internal package names.
func parseRubyDeps(pkg discovery.Package, knownNames map[string]string) []string {
	// Find .gemspec files in the package directory
	entries, err := os.ReadDir(pkg.Path)
	if err != nil {
		return nil
	}

	var gemspecPath string
	for _, entry := range entries {
		if !entry.IsDir() && strings.HasSuffix(entry.Name(), ".gemspec") {
			gemspecPath = filepath.Join(pkg.Path, entry.Name())
			break
		}
	}
	if gemspecPath == "" {
		return nil
	}

	data, err := os.ReadFile(gemspecPath)
	if err != nil {
		return nil
	}

	text := string(data)
	var internalDeps []string

	// Match: spec.add_dependency "coding_adventures_something"
	re := regexp.MustCompile(`spec\.add_dependency\s+"([^"]+)"`)
	for _, match := range re.FindAllStringSubmatch(text, -1) {
		if len(match) < 2 {
			continue
		}
		gemName := strings.TrimSpace(strings.ToLower(match[1]))
		if pkgName, ok := knownNames[gemName]; ok {
			internalDeps = append(internalDeps, pkgName)
		}
	}

	return internalDeps
}

// parseGoDeps extracts internal dependencies from a Go go.mod file.
//
// Go modules declare dependencies in go.mod with:
//
//	require github.com/user/repo/pkg v1.0.0
//
// or in a block:
//
//	require (
//	    github.com/user/repo/pkg v1.0.0
//	)
//
// We parse both forms and map module paths to our internal package names.
func parseGoDeps(pkg discovery.Package, knownNames map[string]string) []string {
	goMod := filepath.Join(pkg.Path, "go.mod")
	data, err := os.ReadFile(goMod)
	if err != nil {
		return nil
	}

	text := string(data)
	var internalDeps []string

	inRequireBlock := false
	for _, line := range strings.Split(text, "\n") {
		stripped := strings.TrimSpace(line)

		if stripped == "require (" {
			inRequireBlock = true
			continue
		}
		if stripped == ")" {
			inRequireBlock = false
			continue
		}

		if inRequireBlock || strings.HasPrefix(stripped, "require ") {
			// Extract the module path (first whitespace-separated token).
			clean := strings.TrimPrefix(stripped, "require ")
			clean = strings.TrimSpace(clean)
			parts := strings.Fields(clean)
			if len(parts) > 0 {
				modulePath := strings.ToLower(parts[0])
				if pkgName, ok := knownNames[modulePath]; ok {
					internalDeps = append(internalDeps, pkgName)
				}
			}
		}
	}

	return internalDeps
}

// parseTypescriptDeps extracts internal dependencies from a TypeScript
// package.json file.
//
// TypeScript packages declare dependencies in package.json:
//
//	"dependencies": {
//	    "@coding-adventures/logic-gates": "file:../logic-gates"
//	}
//
// We scan for lines matching the @coding-adventures/ prefix and map them
// to our internal package names. Version specifiers and file: references
// are ignored — we only care about the package name.
func parseTypescriptDeps(pkg discovery.Package, knownNames map[string]string) []string {
	packageJSON := filepath.Join(pkg.Path, "package.json")
	data, err := os.ReadFile(packageJSON)
	if err != nil {
		return nil
	}

	text := string(data)
	var internalDeps []string

	// We look inside both "dependencies" and "devDependencies" blocks so the
	// graph reflects local build/test prerequisites too.
	inDeps := false
	re := regexp.MustCompile(`"([^"]+)"\s*:`)
	for _, line := range strings.Split(text, "\n") {
		trimmed := strings.TrimSpace(line)

		if !inDeps {
			if (strings.Contains(trimmed, `"dependencies"`) ||
				strings.Contains(trimmed, `"devDependencies"`)) &&
				strings.Contains(trimmed, "{") {
				inDeps = true
			}
			continue
		}

		// Inside dependencies block.
		if strings.Contains(trimmed, "}") {
			inDeps = false
			continue
		}

		for _, match := range re.FindAllStringSubmatch(trimmed, -1) {
			if len(match) < 2 {
				continue
			}
			depName := strings.ToLower(strings.TrimSpace(match[1]))
			if pkgName, ok := knownNames[depName]; ok {
				internalDeps = append(internalDeps, pkgName)
			}
		}
	}

	return internalDeps
}

// parseDartDeps extracts internal dependencies from a Dart pubspec.yaml file.
//
// Dart packages declare dependencies in `dependencies:` and
// `dev_dependencies:` blocks. Local monorepo dependencies still use the
// package name key even when the value is a path map:
//
//	dependencies:
//	  coding_adventures_logic_gates:
//	    path: ../logic-gates
//
// We only need the dependency keys, so a small line-oriented parser is
// sufficient here.
func parseDartDeps(pkg discovery.Package, knownNames map[string]string) []string {
	pubspec := filepath.Join(pkg.Path, "pubspec.yaml")
	data, err := os.ReadFile(pubspec)
	if err != nil {
		return nil
	}

	var internalDeps []string
	currentBlock := ""

	for _, line := range strings.Split(string(data), "\n") {
		trimmed := strings.TrimSpace(line)
		if trimmed == "" || strings.HasPrefix(trimmed, "#") {
			continue
		}

		if !strings.HasPrefix(line, " ") && strings.HasSuffix(trimmed, ":") {
			switch strings.TrimSuffix(trimmed, ":") {
			case "dependencies", "dev_dependencies":
				currentBlock = strings.TrimSuffix(trimmed, ":")
			default:
				currentBlock = ""
			}
			continue
		}

		if currentBlock == "" {
			continue
		}

		if len(line)-len(strings.TrimLeft(line, " ")) < 2 {
			continue
		}

		if strings.HasPrefix(trimmed, "#") || strings.HasPrefix(trimmed, "sdk:") || strings.HasPrefix(trimmed, "path:") {
			continue
		}

		if !strings.Contains(trimmed, ":") {
			continue
		}

		depName := strings.TrimSpace(strings.SplitN(trimmed, ":", 2)[0])
		depName = strings.ToLower(depName)
		if pkgName, ok := knownNames[depName]; ok && pkgName != pkg.Name {
			internalDeps = append(internalDeps, pkgName)
		}
	}

	return internalDeps
}

// parseRustDeps extracts internal dependencies from a Rust Cargo.toml file.
//
// Rust Cargo.toml declares workspace-local dependencies with path references:
//
//	[dependencies]
//	logic-gates = { path = "../logic-gates" }
//
// We look for lines in the [dependencies] section that contain `path =` and
// extract the crate name (the key before the `=`). We then look up that name
// in the known names mapping.
func parseRustDeps(pkg discovery.Package, knownNames map[string]string) []string {
	cargoToml := filepath.Join(pkg.Path, "Cargo.toml")
	data, err := os.ReadFile(cargoToml)
	if err != nil {
		return nil
	}

	text := string(data)
	var internalDeps []string

	// Scan for [dependencies] section and extract path-based deps.
	inDeps := false
	for _, line := range strings.Split(text, "\n") {
		trimmed := strings.TrimSpace(line)

		// Detect section headers like [dependencies] or [dev-dependencies]
		if strings.HasPrefix(trimmed, "[") {
			inDeps = trimmed == "[dependencies]"
			continue
		}

		if !inDeps {
			continue
		}

		// Look for lines like: logic-gates = { path = "../logic-gates" }
		if strings.Contains(trimmed, "path") && strings.Contains(trimmed, "=") {
			// Extract the crate name (everything before the first '=')
			parts := strings.SplitN(trimmed, "=", 2)
			if len(parts) < 2 {
				continue
			}
			crateName := strings.TrimSpace(strings.ToLower(parts[0]))
			if pkgName, ok := knownNames[crateName]; ok {
				internalDeps = append(internalDeps, pkgName)
			}
		}
	}

	return internalDeps
}

// parseElixirDeps extracts internal dependencies from an Elixir mix.exs file.
//
// Elixir mix.exs declares internal path dependencies usually like:
//
//	{:coding_adventures_logic_gates, path: "../logic-gates"}
//
// We use a regex to capture the atom name starting with `coding_adventures_`.
func parseElixirDeps(pkg discovery.Package, knownNames map[string]string) []string {
	mixExs := filepath.Join(pkg.Path, "mix.exs")
	data, err := os.ReadFile(mixExs)
	if err != nil {
		return nil
	}

	text := string(data)
	var internalDeps []string

	re := regexp.MustCompile(`\{:(coding_adventures_[a-z0-9_]+)`)
	for _, line := range strings.Split(text, "\n") {
		trimmed := strings.TrimSpace(line)
		for _, match := range re.FindAllStringSubmatch(trimmed, -1) {
			if len(match) < 2 {
				continue
			}
			appName := strings.ToLower(match[1])
			if pkgName, ok := knownNames[appName]; ok {
				internalDeps = append(internalDeps, pkgName)
			}
		}
	}

	return internalDeps
}

// parsePerlDeps extracts internal dependencies from a Perl cpanfile.
//
// Perl cpanfiles declare dependencies like:
//
//	requires 'CodingAdventures::LogicGates';
//
// We look for lines referencing the CodingAdventures:: namespace and map
// them to internal package names via the knownNames table.
func parsePerlDeps(pkg discovery.Package, knownNames map[string]string) []string {
	cpanfile := filepath.Join(pkg.Path, "cpanfile")
	data, err := os.ReadFile(cpanfile)
	if err != nil {
		return nil
	}

	text := string(data)
	seen := make(map[string]bool)
	var internalDeps []string

	addDep := func(pkgName string) {
		if !seen[pkgName] {
			seen[pkgName] = true
			internalDeps = append(internalDeps, pkgName)
		}
	}

	// Match: requires 'CodingAdventures::SomeName' or requires "CodingAdventures::SomeName"
	reCamel := regexp.MustCompile(`requires\s+['"]CodingAdventures::([A-Za-z0-9:]+)['"]`)
	for _, line := range strings.Split(text, "\n") {
		trimmed := strings.TrimSpace(line)
		for _, match := range reCamel.FindAllStringSubmatch(trimmed, -1) {
			if len(match) < 2 {
				continue
			}
			// Convert CodingAdventures::ModuleName to lowercase lookup key
			// e.g. "LogicGates" → "codingadventures::logicgates"
			moduleName := strings.ToLower("codingadventures::" + match[1])
			if pkgName, ok := knownNames[moduleName]; ok {
				addDep(pkgName)
			}
		}
	}

	// Also match dist-name format: requires 'coding-adventures-logic-gates'
	// Allows underscores too: requires 'coding-adventures-paint_vm_metal_native'
	reKebab := regexp.MustCompile(`requires\s+['"]coding-adventures-([a-z0-9_-]+)['"]`)
	for _, line := range strings.Split(text, "\n") {
		trimmed := strings.TrimSpace(line)
		for _, match := range reKebab.FindAllStringSubmatch(trimmed, -1) {
			if len(match) < 2 {
				continue
			}
			distName := "coding-adventures-" + match[1]
			if pkgName, ok := knownNames[distName]; ok {
				addDep(pkgName)
			}
		}
	}

	return internalDeps
}

// parseSwiftDeps extracts internal dependencies from a Swift Package.swift file.
//
// Package.swift declares dependencies like:
//
//	dependencies: [
//	    .package(path: "../PaintInstructions"),
//	]
//
// We resolve each relative path to an absolute path and look it up in knownNames,
// which maps absolute package paths to package names for Swift packages.
func parseSwiftDeps(pkg discovery.Package, knownNames map[string]string) []string {
	data, err := os.ReadFile(filepath.Join(pkg.Path, "Package.swift"))
	if err != nil {
		return nil
	}

	re := regexp.MustCompile(`\.package\s*\(\s*path\s*:\s*"([^"]+)"`)
	seen := make(map[string]bool)
	var internalDeps []string

	for _, match := range re.FindAllStringSubmatch(string(data), -1) {
		relPath := match[1]
		abs := filepath.Clean(filepath.Join(pkg.Path, relPath))
		if name, ok := knownNames[abs]; ok && name != pkg.Name && !seen[name] {
			seen[name] = true
			internalDeps = append(internalDeps, name)
		}
	}

	return internalDeps
}

// parseLuaDeps extracts internal dependencies from a Lua rockspec file.
//
// Rockspecs declare dependencies like:
//
//	dependencies = {
//	    "coding-adventures-grammar-tools >= 0.1.0",
//	}
//
// We scan all *.rockspec files in the package dir for "coding-adventures-*" strings.
func parseLuaDeps(pkg discovery.Package, knownNames map[string]string) []string {
	entries, err := os.ReadDir(pkg.Path)
	if err != nil {
		return nil
	}

	re := regexp.MustCompile(`"(coding-adventures-[a-z0-9-]+)`)
	seen := make(map[string]bool)
	var internalDeps []string

	for _, entry := range entries {
		if !strings.HasSuffix(entry.Name(), ".rockspec") {
			continue
		}
		data, err := os.ReadFile(filepath.Join(pkg.Path, entry.Name()))
		if err != nil {
			continue
		}
		for _, match := range re.FindAllStringSubmatch(string(data), -1) {
			if len(match) < 2 {
				continue
			}
			depName := strings.ToLower(match[1])
			if pkgName, ok := knownNames[depName]; ok && pkgName != pkg.Name && !seen[pkgName] {
				seen[pkgName] = true
				internalDeps = append(internalDeps, pkgName)
			}
		}
	}
	return internalDeps
}

// buildKnownNames creates a mapping from ecosystem-specific dependency names
// to our internal package names.
//
// This mapping is the "Rosetta Stone" of our build system. Each language
// ecosystem uses its own naming convention for packages:
//
//   - Python: "coding-adventures-logic-gates" → "python/logic-gates"
//   - Ruby:   "coding_adventures_logic_gates" → "ruby/logic_gates"
//   - Go:     full module path → "go/module-name"
//   - Perl:   "coding-adventures-logic-gates" → "perl/logic-gates"
//
// By building this mapping upfront, we can resolve dependencies across
// languages without hard-coding specific package names.
//
// When a library package and a program share the same external dep name
// (e.g., the grammar-tools library and the grammar-tools program both use
// Cargo crate name "grammar-tools"), the LIBRARY always takes priority. This
// prevents a program that depends on its own library from resolving the dep
// to itself and creating a self-loop.
func buildKnownNames(packages []discovery.Package) map[string]string {
	return buildKnownNamesForLanguage(packages, "")
}

// dependencyScope maps a language to the scope it uses for cross-language
// dependency resolution. Languages in the same scope can depend on each other.
func dependencyScope(language string) string {
	switch language {
	case "csharp", "fsharp", "dotnet":
		return "dotnet"
	case "wasm":
		return "wasm"
	default:
		return language
	}
}

// inDependencyScope reports whether packageLanguage belongs to the given scope.
func inDependencyScope(packageLanguage, scope string) bool {
	switch scope {
	case "dotnet":
		return packageLanguage == "csharp" || packageLanguage == "fsharp" || packageLanguage == "dotnet"
	case "wasm":
		return packageLanguage == "wasm" || packageLanguage == "rust"
	default:
		return packageLanguage == scope
	}
}

// buildToolDepsRe matches "# build-tool: deps = pkg1, pkg2" comments in BUILD files.
var buildToolDepsRe = regexp.MustCompile(`(?m)#\s*build-tool:\s*deps\s*=\s*(.+)$`)

// parseBuildToolDeps extracts explicit dependencies declared via
// "# build-tool: deps = <pkg1>, <pkg2>" comments in a BUILD file.
// This is the escape hatch for packages whose deps cannot be auto-detected
// from their language-specific config files.
func parseBuildToolDeps(pkg discovery.Package, knownPackageNames map[string]bool) []string {
	if pkg.BuildContent == "" {
		return nil
	}

	seen := make(map[string]bool)
	for _, match := range buildToolDepsRe.FindAllStringSubmatch(pkg.BuildContent, -1) {
		if len(match) < 2 {
			continue
		}
		for _, raw := range strings.FieldsFunc(match[1], func(r rune) bool {
			return r == ',' || r == ' ' || r == '\t'
		}) {
			dep := strings.TrimSpace(raw)
			if dep == "" || dep == pkg.Name || !knownPackageNames[dep] {
				continue
			}
			seen[dep] = true
		}
	}

	if len(seen) == 0 {
		return nil
	}
	deps := make([]string, 0, len(seen))
	for dep := range seen {
		deps = append(deps, dep)
	}
	sort.Strings(deps)
	return deps
}

func buildKnownNamesForLanguage(packages []discovery.Package, language string) map[string]string {
	known := make(map[string]string)
	knownLanguage := make(map[string]string)
	scope := dependencyScope(language)

	// setKnown inserts key→value, letting library packages overwrite programs
	// but never letting programs overwrite library packages. Within shared
	// toolchain families, collisions are resolved deterministically so wrapper
	// ecosystems do not shadow the canonical implementation names:
	//   - WASM scope prefers Rust crate names for bare crate identifiers.
	//   - .NET scope prefers the caller's exact language (C#, F#, or dotnet).
	setKnown := func(key, value, pkgPath, pkgLanguage string) {
		existing, exists := known[key]
		if !exists {
			known[key] = value
			knownLanguage[key] = pkgLanguage
			return
		}

		existingLanguage := knownLanguage[key]
		existingIsProgram := strings.Contains(filepath.ToSlash(existing), "/programs/")
		currentIsProgram := strings.Contains(filepath.ToSlash(pkgPath), "/programs/")

		switch {
		case existingIsProgram && !currentIsProgram:
			known[key] = value
			knownLanguage[key] = pkgLanguage
			return
		case !existingIsProgram && currentIsProgram:
			return
		}

		switch scope {
		case "wasm":
			if existingLanguage == "rust" {
				return
			}
			if pkgLanguage == "rust" {
				known[key] = value
				knownLanguage[key] = pkgLanguage
				return
			}
		case "dotnet":
			if existingLanguage == language {
				return
			}
			if pkgLanguage == language {
				known[key] = value
				knownLanguage[key] = pkgLanguage
				return
			}
		}

		// Key already set. Allow the overwrite only if the current pkg is
		// a library (not a program) — that is, when the existing entry came
		// from a program and we now have the definitive library entry.
		_ = existing
		if !currentIsProgram {
			known[key] = value
			knownLanguage[key] = pkgLanguage
		}
	}

	for _, pkg := range packages {
		if language != "" && !inDependencyScope(pkg.Language, scope) {
			continue
		}
		switch pkg.Language {
		case "python":
			// Convert dir name to PyPI name: "logic-gates" → "coding-adventures-logic-gates"
			pypiName := "coding-adventures-" + strings.ToLower(filepath.Base(pkg.Path))
			setKnown(pypiName, pkg.Name, pkg.Path, pkg.Language)

		case "ruby":
			// Convert dir name to gem name: "logic_gates" → "coding_adventures_logic_gates"
			gemName := "coding_adventures_" + strings.ToLower(filepath.Base(pkg.Path))
			setKnown(gemName, pkg.Name, pkg.Path, pkg.Language)

		case "go":
			// For Go, read the module path from go.mod.  Go module paths are
			// unique across packages and programs (they include the full path),
			// so the standard map write is safe here.
			goMod := filepath.Join(pkg.Path, "go.mod")
			data, err := os.ReadFile(goMod)
			if err != nil {
				continue
			}
			for _, line := range strings.Split(string(data), "\n") {
				if strings.HasPrefix(line, "module ") {
					modulePath := strings.TrimSpace(strings.TrimPrefix(line, "module "))
					known[strings.ToLower(modulePath)] = pkg.Name
					knownLanguage[strings.ToLower(modulePath)] = pkg.Language
					break
				}
			}

		case "typescript":
			// Convert dir name to npm scoped name: "logic-gates" → "@coding-adventures/logic-gates"
			npmName := "@coding-adventures/" + strings.ToLower(filepath.Base(pkg.Path))
			setKnown(npmName, pkg.Name, pkg.Path, pkg.Language)
			setKnown(strings.ToLower(filepath.Base(pkg.Path)), pkg.Name, pkg.Path, pkg.Language)

			packageJSON := filepath.Join(pkg.Path, "package.json")
			data, err := os.ReadFile(packageJSON)
			if err == nil {
				re := regexp.MustCompile(`"name"\s*:\s*"([^"]+)"`)
				if match := re.FindStringSubmatch(string(data)); len(match) == 2 {
					setKnown(strings.ToLower(strings.TrimSpace(match[1])), pkg.Name, pkg.Path, pkg.Language)
				}
			}

		case "rust":
			// Rust crate names use the directory name directly (kebab-case).
			// "logic-gates" → "logic-gates"
			crateName := strings.ToLower(filepath.Base(pkg.Path))
			setKnown(crateName, pkg.Name, pkg.Path, pkg.Language)

		case "elixir":
			// Elixir mix names replace hyphens with underscores: "logic-gates" → "coding_adventures_logic_gates"
			appName := "coding_adventures_" + strings.ReplaceAll(strings.ToLower(filepath.Base(pkg.Path)), "-", "_")
			setKnown(appName, pkg.Name, pkg.Path, pkg.Language)

		case "dart":
			// Dart pubspec package names use underscore-separated lowercase with prefix:
			// "logic-gates" dir → "coding_adventures_logic_gates" pubspec dep name
			dartName := "coding_adventures_" + strings.ReplaceAll(strings.ToLower(filepath.Base(pkg.Path)), "-", "_")
			setKnown(dartName, pkg.Name, pkg.Path, pkg.Language)

		case "lua":
			// Lua rockspec package names use hyphen-separated lowercase with prefix:
			// "logic_gates" dir → "coding-adventures-logic-gates" rockspec dep name
			luaName := "coding-adventures-" + strings.ReplaceAll(strings.ToLower(filepath.Base(pkg.Path)), "_", "-")
			setKnown(luaName, pkg.Name, pkg.Path, pkg.Language)

		case "swift":
			// Swift packages are referenced by filesystem path in Package.swift.
			// Map the absolute package path to the package name so parseSwiftDeps
			// can resolve .package(path: "../SomePackage") entries.
			setKnown(filepath.Clean(pkg.Path), pkg.Name, pkg.Path, pkg.Language)

		case "perl":
			// Perl module names use CamelCase: "logic-gates" → "CodingAdventures::LogicGates"
			// We map the kebab-case directory name to the Perl namespace.
			dirName := strings.ToLower(filepath.Base(pkg.Path))
			// Build the CodingAdventures::PascalCase module name from kebab-case dir.
			parts := strings.Split(dirName, "-")
			for i, part := range parts {
				if len(part) > 0 {
					parts[i] = strings.ToUpper(part[:1]) + part[1:]
				}
			}
			perlName := "codingadventures::" + strings.ToLower(strings.Join(parts, ""))
			setKnown(perlName, pkg.Name, pkg.Path, pkg.Language)
			// Also add the CPAN dist-name format with hyphens: "coding-adventures-logic-gates"
			distName := "coding-adventures-" + strings.ReplaceAll(dirName, "_", "-")
			setKnown(distName, pkg.Name, pkg.Path, pkg.Language)
			// Also add with underscores preserved (native packages use this format):
			// "coding-adventures-paint_vm_metal_native"
			if strings.Contains(dirName, "_") {
				distNameUnder := "coding-adventures-" + dirName
				setKnown(distNameUnder, pkg.Name, pkg.Path, pkg.Language)
			}
		}
	}

	return known
}

func readCargoPackageName(pkgPath string) string {
	data, err := os.ReadFile(filepath.Join(pkgPath, "Cargo.toml"))
	if err != nil {
		return ""
	}

	re := regexp.MustCompile(`(?m)^\s*name\s*=\s*"([^"]+)"`)
	match := re.FindSubmatch(data)
	if len(match) != 2 {
		return ""
	}

	return strings.ToLower(strings.TrimSpace(string(match[1])))
}

// ResolveDependencies parses package metadata to discover dependencies
// and builds a directed graph.
//
// The graph contains all discovered packages as nodes. Edges represent
// build ordering: an edge from A to B means "A must be built before B"
// (because B depends on A). External dependencies — those not found
// among the discovered packages — are silently skipped.
//
// This function is the main entry point for dependency resolution.
func ResolveDependencies(packages []discovery.Package) *directedgraph.Graph {
	graph := directedgraph.New()

	// First, add all packages as nodes. Even packages with no dependencies
	// need to be in the graph so they appear in independent_groups().
	for _, pkg := range packages {
		graph.AddNode(pkg.Name)
	}

	// Build the ecosystem-specific name mapping table.
	knownNamesByLanguage := make(map[string]map[string]string)
	knownPackageNames := make(map[string]bool, len(packages))
	for _, pkg := range packages {
		knownPackageNames[pkg.Name] = true
		if _, ok := knownNamesByLanguage[pkg.Language]; !ok {
			knownNamesByLanguage[pkg.Language] = buildKnownNamesForLanguage(packages, pkg.Language)
		}
	}

	// Parse dependencies for each package and add edges.
	for _, pkg := range packages {
		var deps []string
		knownNames := knownNamesByLanguage[pkg.Language]
		switch pkg.Language {
		case "python":
			deps = parsePythonDeps(pkg, knownNames)
		case "ruby":
			deps = parseRubyDeps(pkg, knownNames)
		case "go":
			deps = parseGoDeps(pkg, knownNames)
		case "typescript":
			deps = parseTypescriptDeps(pkg, knownNames)
		case "dart":
			deps = parseDartDeps(pkg, knownNames)
		case "swift":
			deps = parseSwiftDeps(pkg, knownNames)
		case "lua":
			deps = parseLuaDeps(pkg, knownNames)
		case "rust":
			deps = parseRustDeps(pkg, knownNames)
		case "elixir":
			deps = parseElixirDeps(pkg, knownNames)
		case "perl":
			deps = parsePerlDeps(pkg, knownNames)
		}
		deps = append(deps, parseBuildToolDeps(pkg, knownPackageNames)...)

		for _, depName := range deps {
			// Edge direction: dep → pkg means "dep must be built before pkg".
			// This convention makes IndependentGroups() produce the correct
			// build order: nodes with zero in-degree (no deps) come first.
			graph.AddEdge(depName, pkg.Name)
		}
	}

	return graph
}

// BuildKnownNames is exported for testing. It delegates to buildKnownNames.
func BuildKnownNames(packages []discovery.Package) map[string]string {
	return buildKnownNames(packages)
}
