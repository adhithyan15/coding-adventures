// Package resolver reads package metadata files (pyproject.toml, .gemspec,
// go.mod) and extracts internal dependencies, building a directed graph.
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
						break
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
			break
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

// buildKnownNames creates a mapping from ecosystem-specific dependency names
// to our internal package names.
//
// This mapping is the "Rosetta Stone" of our build system. Each language
// ecosystem uses its own naming convention for packages:
//
//   - Python: "coding-adventures-logic-gates" → "python/logic-gates"
//   - Ruby:   "coding_adventures_logic_gates" → "ruby/logic_gates"
//   - Go:     full module path → "go/module-name"
//
// By building this mapping upfront, we can resolve dependencies across
// languages without hard-coding specific package names.
func buildKnownNames(packages []discovery.Package) map[string]string {
	known := make(map[string]string)

	for _, pkg := range packages {
		switch pkg.Language {
		case "python":
			// Convert dir name to PyPI name: "logic-gates" → "coding-adventures-logic-gates"
			pypiName := "coding-adventures-" + strings.ToLower(filepath.Base(pkg.Path))
			known[pypiName] = pkg.Name

		case "ruby":
			// Convert dir name to gem name: "logic_gates" → "coding_adventures_logic_gates"
			gemName := "coding_adventures_" + strings.ToLower(filepath.Base(pkg.Path))
			known[gemName] = pkg.Name

		case "go":
			// For Go, read the module path from go.mod.
			goMod := filepath.Join(pkg.Path, "go.mod")
			data, err := os.ReadFile(goMod)
			if err != nil {
				continue
			}
			for _, line := range strings.Split(string(data), "\n") {
				if strings.HasPrefix(line, "module ") {
					modulePath := strings.TrimSpace(strings.TrimPrefix(line, "module "))
					known[strings.ToLower(modulePath)] = pkg.Name
					break
				}
			}
		}
	}

	return known
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
	knownNames := buildKnownNames(packages)

	// Parse dependencies for each package and add edges.
	for _, pkg := range packages {
		var deps []string
		switch pkg.Language {
		case "python":
			deps = parsePythonDeps(pkg, knownNames)
		case "ruby":
			deps = parseRubyDeps(pkg, knownNames)
		case "go":
			deps = parseGoDeps(pkg, knownNames)
		}

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
