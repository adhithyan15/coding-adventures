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
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strings"
	"unicode/utf8"

	directedgraph "github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

const metadataInvalidUTF8 = "METADATA_INVALID_UTF8"

// MetadataEncodingError reports text metadata that cannot be decoded under the
// repository's strict UTF-8 contract. Manifest is always repository-relative so
// diagnostics do not disclose checkout or temporary-directory paths.
type MetadataEncodingError struct {
	Code     string
	Package  string
	Manifest string
	Encoding string
}

func (err *MetadataEncodingError) Error() string {
	return fmt.Sprintf(
		"%s: package=%s manifest=%s encoding=%s",
		err.Code,
		err.Package,
		err.Manifest,
		err.Encoding,
	)
}

func repositoryManifestPath(path string) string {
	parts := strings.Split(filepath.ToSlash(filepath.Clean(path)), "/")
	canonicalStart := -1
	for index := 0; index+1 < len(parts); index++ {
		if parts[index] == "code" &&
			(parts[index+1] == "packages" || parts[index+1] == "programs") {
			canonicalStart = index
		}
	}
	if canonicalStart >= 0 {
		return strings.Join(parts[canonicalStart:], "/")
	}
	return filepath.ToSlash(filepath.Base(path))
}

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
// Ruby gemspecs declare runtime dependencies with either of these synonyms:
//
//	spec.add_dependency "coding_adventures_logic_gates"
//	spec.add_runtime_dependency("coding_adventures_logic_gates")
//
// Only calls on the Gem::Specification block receiver are authoritative.
// Development dependencies and other metadata do not affect build ordering.
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
	receiver := rubySpecificationReceiver(text)
	if receiver == "" {
		return nil
	}
	var internalDeps []string

	for _, line := range strings.Split(text, "\n") {
		gemName := rubyDependencyName(line, receiver)
		if gemName == "" {
			continue
		}
		if pkgName, ok := knownNames[gemName]; ok {
			internalDeps = append(internalDeps, pkgName)
		}
	}

	return internalDeps
}

var rubySpecificationReceiverPattern = regexp.MustCompile(
	`^\s*Gem::Specification\.new\s+do\s+\|([A-Za-z_][A-Za-z0-9_]*)\|`,
)

func rubySpecificationReceiver(text string) string {
	for _, line := range strings.Split(text, "\n") {
		match := rubySpecificationReceiverPattern.FindStringSubmatch(line)
		if len(match) == 2 {
			return match[1]
		}
	}
	return ""
}

func rubyDependencyName(line, receiver string) string {
	stripped := strings.TrimSpace(line)
	for _, method := range []string{"add_dependency", "add_runtime_dependency"} {
		prefix := receiver + "." + method
		if !strings.HasPrefix(stripped, prefix) {
			continue
		}
		remainder := stripped[len(prefix):]
		if remainder == "" || (!isRubySpace(remainder[0]) && remainder[0] != '(') {
			continue
		}
		remainder = strings.TrimSpace(remainder)
		if strings.HasPrefix(remainder, "(") {
			remainder = strings.TrimSpace(remainder[1:])
		}
		if gemName := firstRubyQuotedString(remainder); gemName != "" {
			return strings.ToLower(strings.TrimSpace(gemName))
		}
	}
	return ""
}

func isRubySpace(character byte) bool {
	return character == ' ' || character == '\t'
}

func firstRubyQuotedString(value string) string {
	value = strings.TrimSpace(value)
	if len(value) < 2 || (value[0] != '\'' && value[0] != '"') {
		return ""
	}
	quote := value[0]
	closing := strings.IndexByte(value[1:], quote)
	if closing < 0 {
		return ""
	}
	return value[1 : closing+1]
}

func rubySpecificationNames(packagePath string) []string {
	entries, err := os.ReadDir(packagePath)
	if err != nil {
		return nil
	}
	var names []string
	for _, entry := range entries {
		if entry.IsDir() || !strings.HasSuffix(entry.Name(), ".gemspec") {
			continue
		}
		data, err := os.ReadFile(filepath.Join(packagePath, entry.Name()))
		if err != nil {
			continue
		}
		text := string(data)
		receiver := rubySpecificationReceiver(text)
		if receiver == "" {
			continue
		}
		prefix := receiver + ".name"
		for _, line := range strings.Split(text, "\n") {
			stripped := strings.TrimSpace(line)
			if !strings.HasPrefix(stripped, prefix) {
				continue
			}
			remainder := strings.TrimSpace(stripped[len(prefix):])
			if !strings.HasPrefix(remainder, "=") {
				continue
			}
			if name := firstRubyQuotedString(remainder[1:]); name != "" {
				names = append(names, strings.ToLower(name))
				break
			}
		}
	}
	return names
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
// Only direct keys of the root dependencies and devDependencies objects are
// authoritative. Version specifiers and file:/workspace: references are
// ignored -- we only care about the package name.
func parseTypescriptDeps(pkg discovery.Package, knownNames map[string]string) []string {
	packageJSON := filepath.Join(pkg.Path, "package.json")
	manifest, ok := readPackageJSON(packageJSON)
	if !ok {
		return nil
	}

	var internalDeps []string
	for _, field := range []string{"dependencies", "devDependencies"} {
		for _, depName := range packageJSONDependencyNames(manifest[field]) {
			if pkgName, ok := knownNames[depName]; ok {
				internalDeps = append(internalDeps, pkgName)
			}
		}
	}

	return internalDeps
}

func readPackageJSON(path string) (map[string]json.RawMessage, bool) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, false
	}

	var manifest map[string]json.RawMessage
	if err := json.Unmarshal(data, &manifest); err != nil || manifest == nil {
		return nil, false
	}
	return manifest, true
}

func packageJSONName(manifest map[string]json.RawMessage) string {
	var name string
	if err := json.Unmarshal(manifest["name"], &name); err != nil {
		return ""
	}
	return strings.ToLower(strings.TrimSpace(name))
}

func packageJSONDependencyNames(raw json.RawMessage) []string {
	var dependencies map[string]json.RawMessage
	if err := json.Unmarshal(raw, &dependencies); err != nil || dependencies == nil {
		return nil
	}

	names := make([]string, 0, len(dependencies))
	for name := range dependencies {
		names = append(names, strings.ToLower(strings.TrimSpace(name)))
	}
	sort.Strings(names)
	return names
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
	inDependencyBlock := false
	directEntryIndent := -1

	for _, line := range strings.Split(string(data), "\n") {
		trimmed := strings.TrimSpace(line)
		if trimmed == "" || strings.HasPrefix(trimmed, "#") {
			continue
		}

		indent := len(line) - len(strings.TrimLeft(line, " "))
		if indent == 0 {
			inDependencyBlock = trimmed == "dependencies:" || trimmed == "dev_dependencies:"
			directEntryIndent = -1
			continue
		}

		if !inDependencyBlock {
			continue
		}

		if directEntryIndent < 0 {
			directEntryIndent = indent
		}
		if indent != directEntryIndent {
			continue
		}

		depName, _, found := strings.Cut(trimmed, ":")
		depName = strings.ToLower(strings.TrimSpace(depName))
		if !found || !isDartPackageIdentifier(depName) {
			continue
		}

		if pkgName, ok := knownNames[depName]; ok && pkgName != pkg.Name {
			internalDeps = append(internalDeps, pkgName)
		}
	}

	return internalDeps
}

func isDartPackageIdentifier(value string) bool {
	if value == "" || value[0] < 'a' || value[0] > 'z' {
		return false
	}
	for _, char := range value[1:] {
		if (char < 'a' || char > 'z') && (char < '0' || char > '9') && char != '_' {
			return false
		}
	}
	return true
}

// parseRustDeps extracts internal dependencies from a Rust Cargo.toml file.
//
// Rust Cargo.toml declares workspace-local dependencies with path references:
//
//	[dependencies]
//	logic-gates = { path = "../logic-gates" }
//
// We look for inline-table entries in the [dependencies] section with a
// quoted `path` field and extract the crate name (the key before the first
// `=`). We then look up that name in the known names mapping.
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
		if strings.Contains(trimmed, "=") {
			// Extract the crate name (everything before the first '=')
			parts := strings.SplitN(trimmed, "=", 2)
			if len(parts) < 2 {
				continue
			}
			if cargoInlineStringValue(parts[1], "path") == "" {
				continue
			}
			crateName := strings.TrimSpace(strings.ToLower(parts[0]))
			if packageName := cargoInlineStringValue(parts[1], "package"); packageName != "" {
				crateName = strings.ToLower(packageName)
			}
			if pkgName, ok := knownNames[crateName]; ok {
				internalDeps = append(internalDeps, pkgName)
			}
		}
	}

	return internalDeps
}

// cargoInlineStringValue returns a quoted string field from a Cargo inline
// table. Commas inside quoted strings stay within their field, so a package
// rename cannot be confused with neighboring version or path metadata.
func cargoInlineStringValue(value, target string) string {
	for _, field := range splitCargoInlineFields(value) {
		field = strings.TrimLeft(strings.TrimSpace(field), "{")
		parts := strings.SplitN(field, "=", 2)
		if len(parts) != 2 || strings.TrimSpace(strings.ToLower(parts[0])) != target {
			continue
		}
		quoted := strings.TrimSpace(strings.TrimRight(strings.TrimSpace(parts[1]), "}"))
		if len(quoted) >= 2 && ((quoted[0] == '"' && quoted[len(quoted)-1] == '"') ||
			(quoted[0] == '\'' && quoted[len(quoted)-1] == '\'')) {
			return quoted[1 : len(quoted)-1]
		}
	}
	return ""
}

func splitCargoInlineFields(value string) []string {
	fields := make([]string, 0, 3)
	start := 0
	var quote byte
	for index := 0; index < len(value); index++ {
		character := value[index]
		if quote != 0 {
			if quote == '"' && character == '\\' {
				index++
				continue
			}
			if character == quote {
				quote = 0
			}
			continue
		}
		if character == '"' || character == '\'' {
			quote = character
			continue
		}
		if character == ',' {
			fields = append(fields, value[start:index])
			start = index + 1
		}
	}
	return append(fields, value[start:])
}

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

// parseOCamlDeps unions the process-free local dependency declarations from
// one unambiguous root opam manifest and the fixed Dune files owned by OCAML04.
// It never follows pins or referenced paths and never executes opam or Dune.
func parseOCamlDeps(pkg discovery.Package, knownNames map[string]string) []string {
	seen := make(map[string]bool)
	add := func(candidate string) {
		candidate = strings.ToLower(strings.TrimSpace(candidate))
		if dependency, ok := knownNames[candidate]; ok && dependency != pkg.Name {
			seen[dependency] = true
		}
	}

	if manifest := findOCamlOpamFile(pkg.Path); manifest != "" {
		data, err := os.ReadFile(manifest)
		if err == nil {
			if body, ok := opamListField(string(data), "depends"); ok {
				for _, candidate := range opamDependencyNames(body) {
					add(candidate)
				}
			}
		}
	}

	for _, relative := range []string{"dune", "src/dune", "bin/dune", "test/dune"} {
		manifestPath := filepath.Join(pkg.Path, filepath.FromSlash(relative))
		info, infoErr := os.Lstat(manifestPath)
		if infoErr != nil || !info.Mode().IsRegular() {
			continue
		}
		data, readErr := os.ReadFile(manifestPath)
		if readErr != nil {
			continue
		}
		for _, candidate := range duneLibraryCandidates(string(data)) {
			add(candidate)
		}
	}

	deps := make([]string, 0, len(seen))
	for dependency := range seen {
		deps = append(deps, dependency)
	}
	sort.Strings(deps)
	return deps
}

func findOCamlOpamFile(pkgPath string) string {
	entries, err := os.ReadDir(pkgPath)
	if err != nil {
		return ""
	}
	var manifest string
	for _, entry := range entries {
		info, infoErr := entry.Info()
		if infoErr != nil || !info.Mode().IsRegular() || !strings.HasSuffix(strings.ToLower(entry.Name()), ".opam") {
			continue
		}
		if manifest != "" {
			return ""
		}
		manifest = filepath.Join(pkgPath, entry.Name())
	}
	return manifest
}

func readOCamlPackageNames(pkgPath string) []string {
	manifest := findOCamlOpamFile(pkgPath)
	if manifest == "" {
		return nil
	}
	names := []string{strings.ToLower(strings.TrimSuffix(filepath.Base(manifest), filepath.Ext(manifest)))}
	data, err := os.ReadFile(manifest)
	if err != nil {
		return names
	}
	visible := stripOpamComments(string(data))
	re := regexp.MustCompile(`(?m)^name[ \t]*:[ \t]*"([^"\r\n]+)"[ \t]*$`)
	if match := re.FindStringSubmatch(visible); len(match) == 2 {
		names = append(names, strings.ToLower(match[1]))
	}
	return names
}

func stripOpamComments(source string) string {
	var result strings.Builder
	inString := false
	escaped := false
	inComment := false
	for index := 0; index < len(source); index++ {
		character := source[index]
		if inComment {
			if character == '\n' {
				inComment = false
				result.WriteByte(character)
			}
			continue
		}
		if inString {
			result.WriteByte(character)
			if escaped {
				escaped = false
			} else if character == '\\' {
				escaped = true
			} else if character == '"' {
				inString = false
			}
			continue
		}
		switch character {
		case '#':
			inComment = true
		case '"':
			inString = true
			result.WriteByte(character)
		default:
			result.WriteByte(character)
		}
	}
	return result.String()
}

func opamListField(source, field string) (string, bool) {
	visible := stripOpamComments(source)
	re := regexp.MustCompile(`(?m)^` + regexp.QuoteMeta(field) + `[ \t]*:[ \t]*\[`)
	location := re.FindStringIndex(visible)
	if location == nil {
		return "", false
	}
	open := strings.LastIndex(visible[location[0]:location[1]], "[") + location[0]
	depth := 0
	inString := false
	escaped := false
	for index := open; index < len(visible); index++ {
		character := visible[index]
		if inString {
			if escaped {
				escaped = false
			} else if character == '\\' {
				escaped = true
			} else if character == '"' {
				inString = false
			}
			continue
		}
		if character == '"' {
			inString = true
			continue
		}
		switch character {
		case '[':
			depth++
		case ']':
			depth--
			if depth == 0 {
				return visible[open+1 : index], true
			}
		}
	}
	return "", false
}

// opamDependencyNames returns quoted package atoms while excluding quoted
// strings inside dependency filters and version constraints. The caller has
// already isolated the top-level depends list.
func opamDependencyNames(source string) []string {
	var values []string
	braceDepth := 0
	for index := 0; index < len(source); index++ {
		switch source[index] {
		case '{':
			braceDepth++
			continue
		case '}':
			if braceDepth > 0 {
				braceDepth--
			}
			continue
		}
		if source[index] != '"' {
			continue
		}
		var value strings.Builder
		for index++; index < len(source); index++ {
			if source[index] == '\\' && index+1 < len(source) {
				index++
				value.WriteByte(source[index])
				continue
			}
			if source[index] == '"' {
				if braceDepth == 0 {
					values = append(values, value.String())
				}
				break
			}
			value.WriteByte(source[index])
		}
	}
	return values
}

type duneToken struct {
	value string
	paren byte
}

func tokenizeDune(source string) []duneToken {
	var tokens []duneToken
	for index := 0; index < len(source); {
		switch source[index] {
		case ' ', '\t', '\r', '\n':
			index++
		case ';':
			for index < len(source) && source[index] != '\n' {
				index++
			}
		case '(', ')':
			tokens = append(tokens, duneToken{paren: source[index]})
			index++
		case '"':
			index++
			var value strings.Builder
			for index < len(source) {
				if source[index] == '\\' && index+1 < len(source) {
					index++
					value.WriteByte(source[index])
					index++
					continue
				}
				if source[index] == '"' {
					index++
					break
				}
				value.WriteByte(source[index])
				index++
			}
			tokens = append(tokens, duneToken{value: value.String()})
		default:
			start := index
			for index < len(source) && !strings.ContainsRune("() \t\r\n;", rune(source[index])) {
				index++
			}
			tokens = append(tokens, duneToken{value: source[start:index]})
		}
	}
	return tokens
}

func duneLibraryCandidates(source string) []string {
	tokens := tokenizeDune(source)
	var candidates []string
	for index := 0; index+1 < len(tokens); index++ {
		if tokens[index].paren != '(' || tokens[index+1].value != "libraries" {
			continue
		}
		depth := 1
		for cursor := index + 2; cursor < len(tokens) && depth > 0; cursor++ {
			token := tokens[cursor]
			switch token.paren {
			case '(':
				depth++
			case ')':
				depth--
			default:
				if depth == 1 && token.value != "" && !strings.HasPrefix(token.value, ":") && !strings.HasPrefix(token.value, "%{") {
					candidates = append(candidates, token.value)
				}
			}
		}
	}
	return candidates
}

// parseElixirDeps extracts internal dependencies from an Elixir mix.exs file.
//
// Elixir mix.exs declares internal path dependencies usually like:
//
//	{:coding_adventures_logic_gates, path: "../logic-gates"}
//	{:csv_parser, path: "../csv_parser"}
//
// We capture the dependency atom for path-based local deps, regardless of
// whether it uses the conventional `coding_adventures_` prefix.
func parseElixirDeps(pkg discovery.Package, knownNames map[string]string) []string {
	mixExs := filepath.Join(pkg.Path, "mix.exs")
	data, err := os.ReadFile(mixExs)
	if err != nil {
		return nil
	}

	text := stripElixirLineComments(string(data))
	var internalDeps []string

	re := regexp.MustCompile(`(?s)\{\s*:([a-z0-9_]+)\s*,[^{}]*\bpath:\s*"[^"]+"[^{}]*\}`)
	for _, body := range elixirDependencyBodies(text) {
		for _, match := range re.FindAllStringSubmatch(body, -1) {
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

func elixirDependencyBodies(text string) []string {
	bodies := elixirDirectDepsBodies(text)
	if functionBody := elixirDepsBody(text); strings.TrimSpace(functionBody) != "" {
		bodies = append(bodies, functionBody)
	}
	return bodies
}

func elixirDirectDepsBodies(text string) []string {
	var bodies []string
	var current []string
	depth := 0
	for _, line := range strings.Split(text, "\n") {
		if depth == 0 {
			marker := indexElixirOutsideString(line, "deps:")
			if marker < 0 {
				continue
			}
			value := strings.TrimSpace(line[marker+len("deps:"):])
			if !strings.HasPrefix(value, "[") {
				continue
			}
			current = []string{value}
			depth = elixirBracketDelta(value)
			if depth <= 0 {
				bodies = append(bodies, strings.Join(current, "\n"))
				current = nil
				depth = 0
			}
			continue
		}

		current = append(current, line)
		depth += elixirBracketDelta(line)
		if depth <= 0 {
			bodies = append(bodies, strings.Join(current, "\n"))
			current = nil
			depth = 0
		}
	}
	return bodies
}

func indexElixirOutsideString(text, target string) int {
	inString := false
	escaped := false
	for index := 0; index < len(text); index++ {
		character := text[index]
		if escaped {
			escaped = false
			continue
		}
		if inString && character == '\\' {
			escaped = true
			continue
		}
		if character == '"' {
			inString = !inString
			continue
		}
		if !inString && strings.HasPrefix(text[index:], target) {
			return index
		}
	}
	return -1
}

func elixirBracketDelta(text string) int {
	delta := 0
	inString := false
	escaped := false
	for _, character := range text {
		if escaped {
			escaped = false
			continue
		}
		if inString && character == '\\' {
			escaped = true
			continue
		}
		if character == '"' {
			inString = !inString
			continue
		}
		if !inString && character == '[' {
			delta++
		}
		if !inString && character == ']' {
			delta--
		}
	}
	return delta
}

func stripElixirLineComments(text string) string {
	lines := strings.Split(text, "\n")
	for lineIndex, line := range lines {
		inString := false
		escaped := false
		for index, character := range line {
			if escaped {
				escaped = false
				continue
			}
			if inString && character == '\\' {
				escaped = true
				continue
			}
			if character == '"' {
				inString = !inString
				continue
			}
			if character == '#' && !inString {
				lines[lineIndex] = line[:index]
				break
			}
		}
	}
	return strings.Join(lines, "\n")
}

func elixirDepsBody(text string) string {
	lines := strings.Split(text, "\n")
	insideBlock := false
	var body []string
	for _, line := range lines {
		trimmed := strings.TrimSpace(line)
		if !insideBlock {
			if strings.HasPrefix(trimmed, "defp deps,") || strings.HasPrefix(trimmed, "def deps,") {
				if marker := strings.Index(trimmed, "do:"); marker >= 0 {
					return trimmed[marker+len("do:"):]
				}
			}
			if trimmed == "defp deps do" || trimmed == "def deps do" {
				insideBlock = true
			}
			continue
		}
		if trimmed == "end" {
			break
		}
		body = append(body, line)
	}
	return strings.Join(body, "\n")
}

// parseLuaDeps extracts internal dependencies from a Lua .rockspec file.
//
// LuaRocks rockspec files declare dependencies in a Lua table:
//
//	dependencies = {
//	    "lua >= 5.4",
//	    "coding-adventures-logic-gates >= 0.1.0",
//	}
//
// We scan for quoted strings inside the dependencies block that start with
// "coding-adventures-" and map them to internal package names. Version
// specifiers are stripped. The rockspec format uses hyphens in package names,
// matching the Python/PyPI convention.
func parseLuaDeps(
	pkg discovery.Package,
	knownNames map[string]string,
) ([]string, error) {
	// Find .rockspec files in the package directory.
	entries, err := os.ReadDir(pkg.Path)
	if err != nil {
		return nil, nil
	}

	var rockspecPath string
	for _, entry := range entries {
		if !entry.IsDir() && strings.HasSuffix(entry.Name(), ".rockspec") {
			rockspecPath = filepath.Join(pkg.Path, entry.Name())
			break
		}
	}
	if rockspecPath == "" {
		return nil, nil
	}

	data, err := os.ReadFile(rockspecPath)
	if err != nil {
		return nil, nil
	}
	if !utf8.Valid(data) {
		return nil, &MetadataEncodingError{
			Code:     metadataInvalidUTF8,
			Package:  pkg.Name,
			Manifest: repositoryManifestPath(rockspecPath),
			Encoding: "UTF-8",
		}
	}

	text := string(data)
	var internalDeps []string

	// Strategy: find the dependencies = { ... } block and extract quoted strings.
	// We look for lines containing quoted strings inside the dependencies block.
	inDeps := false
	re := regexp.MustCompile(`"([^"]+)"`)
	for _, line := range strings.Split(text, "\n") {
		trimmed := strings.TrimSpace(line)

		if !inDeps {
			// Look for: dependencies = {
			if strings.Contains(trimmed, "dependencies") && strings.Contains(trimmed, "=") && strings.Contains(trimmed, "{") {
				inDeps = true
				// Check if it's a single-line block: dependencies = { "foo", "bar" }
				if strings.Contains(trimmed, "}") {
					for _, match := range re.FindAllStringSubmatch(trimmed, -1) {
						if len(match) >= 2 {
							mapLuaDep(match[1], knownNames, &internalDeps)
						}
					}
					break
				}
			}
			continue
		}

		// We're inside the dependencies block.
		if strings.Contains(trimmed, "}") {
			// Extract any deps on the closing line too.
			for _, match := range re.FindAllStringSubmatch(trimmed, -1) {
				if len(match) >= 2 {
					mapLuaDep(match[1], knownNames, &internalDeps)
				}
			}
			break
		}

		for _, match := range re.FindAllStringSubmatch(trimmed, -1) {
			if len(match) >= 2 {
				mapLuaDep(match[1], knownNames, &internalDeps)
			}
		}
	}

	return internalDeps, nil
}

// mapLuaDep strips version specifiers from a Lua dependency string and maps it
// to an internal package name if it matches a known dependency.
//
// Input examples:
//
//	"coding-adventures-logic-gates >= 0.1.0"  →  "lua/logic_gates"
//	"lua >= 5.4"                              →  (skipped, not in knownNames)
func mapLuaDep(depStr string, knownNames map[string]string, deps *[]string) {
	// Strip version specifiers: split on >=, <=, >, <, ==, ~=, spaces
	depName := regexp.MustCompile(`[>=<!~\s]`).Split(depStr, 2)[0]
	depName = strings.TrimSpace(strings.ToLower(depName))
	if pkgName, ok := knownNames[depName]; ok {
		*deps = append(*deps, pkgName)
	}
}

// parsePerlDeps extracts internal dependencies from a Perl cpanfile.
//
// A cpanfile is Perl's declarative dependency file (like Gemfile or
// package.json). It uses a Perl DSL with one `requires` per line:
//
//	requires 'coding-adventures-logic-gates';
//	requires 'coding-adventures-bitset', '>= 0.01';
//
//	on 'test' => sub {
//	    requires 'Test2::V0';
//	};
//
// Only top-level runtime declarations are authoritative. Requirements inside
// `on ... => sub { ... }` phase blocks are ignored, as are comments and all
// Makefile.PL dependency tables. External dependencies are silently skipped.
func parsePerlDeps(pkg discovery.Package, knownNames map[string]string) []string {
	cpanfile := filepath.Join(pkg.Path, "cpanfile")
	data, err := os.ReadFile(cpanfile)
	if err != nil {
		return nil
	}

	var internalDeps []string
	blockDepth := 0
	for _, line := range strings.Split(string(data), "\n") {
		uncommented := stripPerlComment(line)
		trimmed := strings.TrimSpace(uncommented)

		if blockDepth == 0 {
			if matches := perlRequiresPattern.FindStringSubmatch(trimmed); len(matches) == 2 {
				depName := strings.ToLower(strings.TrimSpace(matches[1]))
				if pkgName, ok := knownNames[depName]; ok {
					internalDeps = append(internalDeps, pkgName)
				}
			}
		}

		structure := hidePerlStringContents(uncommented)
		blockDepth += strings.Count(structure, "{") - strings.Count(structure, "}")
		if blockDepth < 0 {
			blockDepth = 0
		}
	}

	return internalDeps
}

var perlRequiresPattern = regexp.MustCompile(`^requires\s+['"]([^'"]+)['"]`)

func stripPerlComment(line string) string {
	quote := byte(0)
	escaped := false
	for index := 0; index < len(line); index++ {
		character := line[index]
		if escaped {
			escaped = false
			continue
		}
		if quote != 0 && character == '\\' {
			escaped = true
			continue
		}
		if character == '\'' || character == '"' {
			if quote == 0 {
				quote = character
			} else if quote == character {
				quote = 0
			}
			continue
		}
		if quote == 0 && character == '#' {
			return line[:index]
		}
	}
	return line
}

func hidePerlStringContents(line string) string {
	visible := []byte(line)
	quote := byte(0)
	escaped := false
	for index, character := range visible {
		if escaped {
			visible[index] = ' '
			escaped = false
			continue
		}
		if quote != 0 && character == '\\' {
			visible[index] = ' '
			escaped = true
			continue
		}
		if character == '\'' || character == '"' {
			visible[index] = ' '
			if quote == 0 {
				quote = character
			} else if quote == character {
				quote = 0
			}
			continue
		}
		if quote != 0 {
			visible[index] = ' '
		}
	}
	return string(visible)
}

var perlNamePattern = regexp.MustCompile(`\bNAME\s*=>\s*['"]([^'"]+)['"]`)

func perlPackageNames(packagePath string) []string {
	data, err := os.ReadFile(filepath.Join(packagePath, "Makefile.PL"))
	if err != nil {
		return nil
	}
	for _, line := range strings.Split(string(data), "\n") {
		match := perlNamePattern.FindStringSubmatch(stripPerlComment(line))
		if len(match) == 2 {
			return []string{strings.ToLower(strings.TrimSpace(match[1]))}
		}
	}
	return nil
}

// parseSwiftDeps extracts internal dependencies from a Swift Package.swift file.
//
// Swift Package Manager uses path references for local (monorepo)
// dependencies. The declaration always appears on a single line in
// scaffold-generated files:
//
//	.package(path: "../logic-gates"),
//	.package(path: "../../../packages/swift/md5"),
//
// We scan for this pattern, take the final path component, and map that
// directory name back to our internal package name. External dependencies
// (declared with `url:`) are silently skipped because they don't match this
// `path:` form.
func parseSwiftDeps(pkg discovery.Package, knownNames map[string]string) []string {
	manifest := filepath.Join(pkg.Path, "Package.swift")
	data, err := os.ReadFile(manifest)
	if err != nil {
		return nil
	}

	var internalDeps []string
	for _, path := range swiftLocalPackagePaths(string(data)) {
		if strings.HasSuffix(path, "/") || strings.HasSuffix(path, "\\") {
			continue
		}
		cleaned := filepath.Clean(filepath.FromSlash(path))
		if swiftPathIsAbsolute(path) || filepath.IsAbs(cleaned) {
			continue
		}
		depDir := strings.ToLower(filepath.Base(cleaned))
		if depDir == "" || depDir == "." || depDir == ".." {
			continue
		}
		if pkgName, ok := knownNames[depDir]; ok {
			internalDeps = append(internalDeps, pkgName)
		}
	}

	return internalDeps
}

// swiftLocalPackagePaths returns only path values from actual
// .package(path: "...") calls. Comments and unrelated string literals are
// ignored before the field-aware scan.
func swiftLocalPackagePaths(source string) []string {
	visible := stripSwiftComments(source)
	var paths []string
	for index := 0; index < len(visible); {
		if visible[index] == '"' {
			index = skipSwiftString(visible, index)
			continue
		}
		if strings.HasPrefix(visible[index:], ".package") {
			if path, next, ok := parseSwiftPackagePath(visible, index+len(".package")); ok {
				paths = append(paths, path)
				index = next
				continue
			}
		}
		index++
	}
	return paths
}

func parseSwiftPackagePath(source string, index int) (string, int, bool) {
	index = skipSwiftWhitespace(source, index)
	if index >= len(source) || source[index] != '(' {
		return "", index, false
	}
	index = skipSwiftWhitespace(source, index+1)
	if !strings.HasPrefix(source[index:], "path") {
		return "", index, false
	}
	index += len("path")
	if index < len(source) && (source[index] == '_' || source[index] == '-' ||
		(source[index] >= '0' && source[index] <= '9') ||
		(source[index] >= 'A' && source[index] <= 'Z') ||
		(source[index] >= 'a' && source[index] <= 'z')) {
		return "", index, false
	}
	index = skipSwiftWhitespace(source, index)
	if index >= len(source) || source[index] != ':' {
		return "", index, false
	}
	index = skipSwiftWhitespace(source, index+1)
	if index >= len(source) || source[index] != '"' {
		return "", index, false
	}

	start := index + 1
	for index = start; index < len(source); index++ {
		if source[index] == '\\' {
			index++
			continue
		}
		if source[index] == '"' {
			return source[start:index], index + 1, true
		}
	}
	return "", index, false
}

func skipSwiftWhitespace(source string, index int) int {
	for index < len(source) {
		switch source[index] {
		case ' ', '\t', '\r', '\n':
			index++
		default:
			return index
		}
	}
	return index
}

func skipSwiftString(source string, index int) int {
	index++
	for index < len(source) {
		if source[index] == '\\' {
			index += 2
			continue
		}
		index++
		if source[index-1] == '"' {
			break
		}
	}
	return index
}

func stripSwiftComments(source string) string {
	visible := []byte(source)
	blockDepth := 0
	for index := 0; index < len(visible); {
		if blockDepth > 0 {
			if index+1 < len(visible) && visible[index] == '/' && visible[index+1] == '*' {
				visible[index], visible[index+1] = ' ', ' '
				blockDepth++
				index += 2
				continue
			}
			if index+1 < len(visible) && visible[index] == '*' && visible[index+1] == '/' {
				visible[index], visible[index+1] = ' ', ' '
				blockDepth--
				index += 2
				continue
			}
			if visible[index] != '\n' && visible[index] != '\r' {
				visible[index] = ' '
			}
			index++
			continue
		}

		if visible[index] == '"' {
			index = skipSwiftStringBytes(visible, index)
			continue
		}
		if index+1 < len(visible) && visible[index] == '/' && visible[index+1] == '/' {
			for index < len(visible) && visible[index] != '\n' && visible[index] != '\r' {
				visible[index] = ' '
				index++
			}
			continue
		}
		if index+1 < len(visible) && visible[index] == '/' && visible[index+1] == '*' {
			visible[index], visible[index+1] = ' ', ' '
			blockDepth = 1
			index += 2
			continue
		}
		index++
	}
	return string(visible)
}

func skipSwiftStringBytes(source []byte, index int) int {
	index++
	for index < len(source) {
		if source[index] == '\\' {
			index += 2
			continue
		}
		index++
		if source[index-1] == '"' {
			break
		}
	}
	return index
}

func swiftPathIsAbsolute(path string) bool {
	if path == "" {
		return false
	}
	if path[0] == '/' || path[0] == '\\' {
		return true
	}
	return len(path) >= 2 && path[1] == ':' &&
		((path[0] >= 'A' && path[0] <= 'Z') || (path[0] >= 'a' && path[0] <= 'z'))
}

// parseHaskellDeps extracts internal dependencies from every build-depends
// field in a Haskell .cabal file.
//
// Most repository Cabal packages use plain names, while a few older packages
// retain the coding-adventures-* prefix. Both aliases are registered in
// buildKnownNamesForLanguage, so this parser maps only names belonging to
// discovered Haskell packages.
func parseHaskellDeps(pkg discovery.Package, knownNames map[string]string) []string {
	cabalFile := findCabalFile(pkg.Path)
	if cabalFile == "" {
		return nil
	}

	data, err := os.ReadFile(cabalFile)
	if err != nil {
		return nil
	}

	nameRe := regexp.MustCompile(`^([a-zA-Z0-9][a-zA-Z0-9-]*)`)
	fieldRe := regexp.MustCompile(`^[a-zA-Z][a-zA-Z0-9-]*\s*:`)
	seen := make(map[string]bool)
	var internalDeps []string
	inBuildDepends := false
	for _, rawLine := range strings.Split(string(data), "\n") {
		line := strings.TrimSpace(strings.SplitN(rawLine, "--", 2)[0])
		lowerLine := strings.ToLower(line)
		if strings.HasPrefix(lowerLine, "build-depends:") {
			inBuildDepends = true
			line = strings.TrimSpace(line[len("build-depends:"):])
		} else if inBuildDepends &&
			(line == "" || fieldRe.MatchString(line) ||
				(len(rawLine) > 0 && rawLine[0] != ' ' && rawLine[0] != '\t')) {
			inBuildDepends = false
		}
		if !inBuildDepends {
			continue
		}

		for _, piece := range strings.Split(line, ",") {
			match := nameRe.FindStringSubmatch(strings.TrimSpace(piece))
			if len(match) != 2 {
				continue
			}
			depName := strings.ToLower(match[1])
			if pkgName, ok := knownNames[depName]; ok &&
				pkgName != pkg.Name && !seen[pkgName] {
				seen[pkgName] = true
				internalDeps = append(internalDeps, pkgName)
			}
		}
	}

	return internalDeps
}

// parseDotnetDeps reads only literal ProjectReference Include attributes from
// .csproj and .fsproj files directly inside the package root. Referenced paths
// are normalized lexically and matched against already discovered root project
// files in the shared .NET scope; the targets are never opened or followed.
func parseDotnetDeps(pkg discovery.Package, knownProjectPaths map[string]string) []string {
	projectFiles := rootDotnetProjectFiles(pkg.Path)
	seen := make(map[string]bool)
	for _, projectFile := range projectFiles {
		data, err := os.ReadFile(projectFile)
		if err != nil {
			continue
		}
		for _, include := range dotnetProjectReferenceIncludes(string(data)) {
			targetPath, ok := dotnetProjectReferencePath(projectFile, include)
			if !ok {
				continue
			}
			if packageName, found := knownProjectPaths[targetPath]; found && packageName != pkg.Name {
				seen[packageName] = true
			}
		}
	}

	deps := make([]string, 0, len(seen))
	for dep := range seen {
		deps = append(deps, dep)
	}
	sort.Strings(deps)
	return deps
}

func rootDotnetProjectFiles(root string) []string {
	entries, err := os.ReadDir(root)
	if err != nil {
		return nil
	}
	var projectFiles []string
	for _, entry := range entries {
		if entry.IsDir() {
			continue
		}
		name := entry.Name()
		lowerName := strings.ToLower(name)
		if strings.HasSuffix(lowerName, ".csproj") || strings.HasSuffix(lowerName, ".fsproj") {
			projectFiles = append(projectFiles, filepath.Join(root, name))
		}
	}
	sort.Strings(projectFiles)
	return projectFiles
}

func dotnetProjectReferenceIncludes(source string) []string {
	var includes []string
	for index := 0; index < len(source); {
		relative := strings.IndexByte(source[index:], '<')
		if relative < 0 {
			break
		}
		index += relative
		switch {
		case strings.HasPrefix(source[index:], "<!--"):
			index = skipXMLMarkup(source, index+4, "-->")
		case strings.HasPrefix(source[index:], "<![CDATA["):
			index = skipXMLMarkup(source, index+9, "]]>")
		case strings.HasPrefix(source[index:], "<?"):
			index = skipXMLMarkup(source, index+2, "?>")
		case strings.HasPrefix(source[index:], "<!"):
			index = skipXMLMarkup(source, index+2, ">")
		default:
			name, attributes, next, ok := parseXMLStartTag(source, index)
			if !ok {
				index++
				continue
			}
			index = next
			if name != "ProjectReference" {
				continue
			}
			if include, found := xmlLiteralAttribute(attributes, "Include"); found {
				includes = append(includes, include)
			}
		}
	}
	return includes
}

func skipXMLMarkup(source string, index int, terminator string) int {
	relative := strings.Index(source[index:], terminator)
	if relative < 0 {
		return len(source)
	}
	return index + relative + len(terminator)
}

func parseXMLStartTag(source string, index int) (string, string, int, bool) {
	if index >= len(source) || source[index] != '<' || index+1 >= len(source) || source[index+1] == '/' {
		return "", "", index, false
	}
	nameStart := index + 1
	nameEnd := nameStart
	for nameEnd < len(source) && isXMLNameByte(source[nameEnd]) {
		nameEnd++
	}
	if nameEnd == nameStart {
		return "", "", index, false
	}

	quote := byte(0)
	for end := nameEnd; end < len(source); end++ {
		character := source[end]
		if quote != 0 {
			if character == quote {
				quote = 0
			}
			continue
		}
		if character == '\'' || character == '"' {
			quote = character
			continue
		}
		if character == '>' {
			return source[nameStart:nameEnd], source[nameEnd:end], end + 1, true
		}
	}
	return "", "", len(source), false
}

func isXMLNameByte(character byte) bool {
	return character == ':' || character == '_' || character == '-' || character == '.' ||
		(character >= '0' && character <= '9') ||
		(character >= 'A' && character <= 'Z') ||
		(character >= 'a' && character <= 'z')
}

func xmlLiteralAttribute(attributes string, wanted string) (string, bool) {
	for index := 0; index < len(attributes); {
		index = skipSwiftWhitespace(attributes, index)
		if index >= len(attributes) || attributes[index] == '/' {
			return "", false
		}
		nameStart := index
		for index < len(attributes) && isXMLNameByte(attributes[index]) {
			index++
		}
		if index == nameStart {
			index++
			continue
		}
		name := attributes[nameStart:index]
		index = skipSwiftWhitespace(attributes, index)
		if index >= len(attributes) || attributes[index] != '=' {
			continue
		}
		index = skipSwiftWhitespace(attributes, index+1)
		if index >= len(attributes) || (attributes[index] != '\'' && attributes[index] != '"') {
			continue
		}
		quote := attributes[index]
		valueStart := index + 1
		index = valueStart
		for index < len(attributes) && attributes[index] != quote {
			index++
		}
		if index >= len(attributes) {
			return "", false
		}
		value := attributes[valueStart:index]
		index++
		if name == wanted {
			return value, true
		}
	}
	return "", false
}

func dotnetProjectReferencePath(projectFile, include string) (string, bool) {
	if include == "" || strings.ContainsAny(include, "*?#&") ||
		strings.Contains(include, "$(") || swiftPathIsAbsolute(include) {
		return "", false
	}
	portable := strings.Map(func(character rune) rune {
		if character == '/' || character == '\\' {
			return filepath.Separator
		}
		return character
	}, include)
	return normalizedDotnetProjectPath(filepath.Join(filepath.Dir(projectFile), portable)), true
}

func normalizedDotnetProjectPath(path string) string {
	return strings.ToLower(filepath.Clean(path))
}

var buildToolDepsRe = regexp.MustCompile(`(?m)#\s*build-tool:\s*deps\s*=\s*(.+)$`)

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

// parseGradleDeps extracts internal dependencies from a Gradle
// settings.gradle.kts file. This parser works for both Java and Kotlin
// packages since both use Gradle composite builds. It scans actual
// includeBuild("...") calls outside comments and unrelated strings. Relative
// paths are normalized lexically and matched to discovered package roots in
// the same language scope. The reader never follows or reads a referenced
// path.
func parseGradleDeps(pkg discovery.Package, knownPaths map[string]string) []string {
	settingsFile := filepath.Join(pkg.Path, "settings.gradle.kts")
	data, err := os.ReadFile(settingsFile)
	if err != nil {
		return nil
	}

	seen := make(map[string]bool)
	for _, relativePath := range gradleIncludeBuildPaths(string(data)) {
		if relativePath == "" || strings.Contains(relativePath, "\\") ||
			swiftPathIsAbsolute(relativePath) {
			continue
		}
		targetPath := normalizedGradlePackagePath(
			filepath.Join(pkg.Path, filepath.FromSlash(relativePath)),
		)
		if pkgName, ok := knownPaths[targetPath]; ok {
			seen[pkgName] = true
		}
	}

	deps := make([]string, 0, len(seen))
	for dep := range seen {
		deps = append(deps, dep)
	}
	sort.Strings(deps)
	return deps
}

func gradleIncludeBuildPaths(source string) []string {
	visible := stripSwiftComments(source)
	var paths []string
	for index := 0; index < len(visible); {
		if visible[index] == '"' {
			index = skipSwiftString(visible, index)
			continue
		}
		if hasGradleIdentifierAt(visible, index, "includeBuild") {
			if path, next, ok := parseGradleIncludeBuild(visible, index+len("includeBuild")); ok {
				paths = append(paths, path)
				index = next
				continue
			}
		}
		index++
	}
	return paths
}

func hasGradleIdentifierAt(source string, index int, identifier string) bool {
	if !strings.HasPrefix(source[index:], identifier) {
		return false
	}
	if index > 0 && isGradleIdentifierByte(source[index-1]) {
		return false
	}
	end := index + len(identifier)
	return end == len(source) || !isGradleIdentifierByte(source[end])
}

func isGradleIdentifierByte(value byte) bool {
	return value == '_' || (value >= '0' && value <= '9') ||
		(value >= 'A' && value <= 'Z') || (value >= 'a' && value <= 'z')
}

func parseGradleIncludeBuild(source string, index int) (string, int, bool) {
	index = skipSwiftWhitespace(source, index)
	if index >= len(source) || source[index] != '(' {
		return "", index, false
	}
	index = skipSwiftWhitespace(source, index+1)
	if index >= len(source) || source[index] != '"' {
		return "", index, false
	}

	start := index + 1
	for index = start; index < len(source); index++ {
		if source[index] == '\\' {
			index++
			continue
		}
		if source[index] != '"' {
			continue
		}
		path := source[start:index]
		next := skipSwiftWhitespace(source, index+1)
		if next >= len(source) || source[next] != ')' {
			return "", next, false
		}
		return path, next + 1, true
	}
	return "", index, false
}

func normalizedGradlePackagePath(path string) string {
	return strings.ToLower(filepath.Clean(path))
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

func buildKnownGradlePathsForLanguage(packages []discovery.Package, language string) map[string]string {
	known := make(map[string]string)
	scope := dependencyScope(language)
	for _, pkg := range packages {
		if !inDependencyScope(pkg.Language, scope) {
			continue
		}
		known[normalizedGradlePackagePath(pkg.Path)] = pkg.Name
	}
	return known
}

func buildKnownDotnetProjectPaths(packages []discovery.Package) map[string]string {
	known := make(map[string]string)
	for _, pkg := range packages {
		if !inDependencyScope(pkg.Language, "dotnet") {
			continue
		}
		for _, projectFile := range rootDotnetProjectFiles(pkg.Path) {
			known[normalizedDotnetProjectPath(projectFile)] = pkg.Name
		}
	}
	return known
}

func buildKnownNamesForLanguage(packages []discovery.Package, language string) map[string]string {
	known := make(map[string]string)
	knownLanguage := make(map[string]string)
	ambiguous := make(map[string]bool)
	scope := dependencyScope(language)

	// setKnown inserts key→value, letting library packages overwrite programs,
	// rejecting same-priority Dart ambiguity, and never letting programs
	// overwrite library packages. Within shared
	// toolchain families, collisions are resolved deterministically so wrapper
	// ecosystems do not shadow the canonical implementation names:
	//   - WASM scope prefers Rust crate names for bare crate identifiers.
	//   - .NET scope prefers the caller's exact language (C#, F#, or dotnet).
	setKnown := func(key, value, pkgPath, pkgLanguage string) {
		if ambiguous[key] {
			return
		}
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
		if (scope == "dart" || scope == "ocaml") && existing != value {
			delete(known, key)
			delete(knownLanguage, key)
			ambiguous[key] = true
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
			for _, declaredName := range rubySpecificationNames(pkg.Path) {
				setKnown(declaredName, pkg.Name, pkg.Path, pkg.Language)
			}

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
			if manifest, ok := readPackageJSON(packageJSON); ok {
				if declaredName := packageJSONName(manifest); declaredName != "" {
					setKnown(declaredName, pkg.Name, pkg.Path, pkg.Language)
				}
			}

		case "rust":
			// Rust crate names use the directory name directly (kebab-case).
			// "logic-gates" → "logic-gates"
			crateName := strings.ToLower(filepath.Base(pkg.Path))
			setKnown(crateName, pkg.Name, pkg.Path, pkg.Language)
			if cargoName := readCargoPackageName(pkg.Path); cargoName != "" {
				setKnown(cargoName, pkg.Name, pkg.Path, pkg.Language)
			}

		case "wasm":
			// WASM wrappers should resolve through their explicit Cargo package
			// names (e.g. "graph-wasm"), not by bare directory names like
			// "graph". The bare crate names belong to the canonical Rust crates
			// they wrap; reusing them here can create self-loops when the Rust
			// package is absent from discovery.
			if cargoName := readCargoPackageName(pkg.Path); cargoName != "" {
				setKnown(cargoName, pkg.Name, pkg.Path, pkg.Language)
			}

		case "elixir":
			// Elixir mix names replace hyphens with underscores: "logic-gates" → "coding_adventures_logic_gates"
			baseName := strings.ReplaceAll(strings.ToLower(filepath.Base(pkg.Path)), "-", "_")
			appName := "coding_adventures_" + baseName
			setKnown(appName, pkg.Name, pkg.Path, pkg.Language)
			setKnown(baseName, pkg.Name, pkg.Path, pkg.Language)

			mixExs := filepath.Join(pkg.Path, "mix.exs")
			data, err := os.ReadFile(mixExs)
			if err == nil {
				re := regexp.MustCompile(`app:\s*:([a-z0-9_]+)`)
				if match := re.FindStringSubmatch(string(data)); len(match) == 2 {
					setKnown(strings.ToLower(strings.TrimSpace(match[1])), pkg.Name, pkg.Path, pkg.Language)
				}
			}

		case "dart":
			baseName := strings.ReplaceAll(strings.ToLower(filepath.Base(pkg.Path)), "-", "_")
			pubName := "coding_adventures_" + baseName
			setKnown(pubName, pkg.Name, pkg.Path, pkg.Language)
			setKnown(baseName, pkg.Name, pkg.Path, pkg.Language)

			pubspec := filepath.Join(pkg.Path, "pubspec.yaml")
			data, err := os.ReadFile(pubspec)
			if err == nil {
				re := regexp.MustCompile(`(?m)^name\s*:\s*([a-z0-9_]+)\s*$`)
				if match := re.FindStringSubmatch(string(data)); len(match) == 2 {
					setKnown(strings.ToLower(strings.TrimSpace(match[1])), pkg.Name, pkg.Path, pkg.Language)
				}
			}

		case "lua":
			// Lua rockspec names use hyphens: "logic_gates" → "coding-adventures-logic-gates"
			// Note: Lua directory names use underscores, rockspec names use hyphens.
			rockspecName := "coding-adventures-" + strings.ReplaceAll(
				strings.ToLower(filepath.Base(pkg.Path)), "_", "-")
			setKnown(rockspecName, pkg.Name, pkg.Path, pkg.Language)

		case "perl":
			baseName := strings.ToLower(filepath.Base(pkg.Path))
			kebabName := strings.ReplaceAll(baseName, "_", "-")
			snakeName := strings.ReplaceAll(baseName, "-", "_")
			for _, alias := range []string{
				baseName,
				kebabName,
				snakeName,
				"coding-adventures-" + baseName,
				"coding-adventures-" + kebabName,
				"coding_adventures_" + snakeName,
			} {
				setKnown(alias, pkg.Name, pkg.Path, pkg.Language)
			}
			for _, declaredName := range perlPackageNames(pkg.Path) {
				setKnown(declaredName, pkg.Name, pkg.Path, pkg.Language)
			}

		case "swift":
			// Swift SPM package names are the kebab-case directory name, matching
			// the `name:` field in Package.swift. .package(path: "../logic-gates")
			// references the directory name "logic-gates" directly.
			dirBase := strings.ToLower(filepath.Base(pkg.Path))
			setKnown(dirBase, pkg.Name, pkg.Path, pkg.Language)

		case "haskell":
			// Modern repository Cabal packages use the plain directory name.
			// Keep the legacy prefix and the manifest's declared name as aliases.
			dirBase := strings.ToLower(filepath.Base(pkg.Path))
			setKnown(dirBase, pkg.Name, pkg.Path, pkg.Language)
			setKnown("coding-adventures-"+dirBase, pkg.Name, pkg.Path, pkg.Language)
			if cabalName := readCabalPackageName(pkg.Path); cabalName != "" {
				setKnown(cabalName, pkg.Name, pkg.Path, pkg.Language)
			}

		case "ocaml":
			// OCaml local packages may be referenced by their directory, their
			// conventional repository package name, or the sole root opam file.
			dirBase := strings.ToLower(filepath.Base(pkg.Path))
			setKnown(dirBase, pkg.Name, pkg.Path, pkg.Language)
			setKnown("coding-adventures-"+dirBase, pkg.Name, pkg.Path, pkg.Language)
			setKnown("coding_adventures_"+strings.ReplaceAll(dirBase, "-", "_"), pkg.Name, pkg.Path, pkg.Language)
			for _, declaredName := range readOCamlPackageNames(pkg.Path) {
				setKnown(declaredName, pkg.Name, pkg.Path, pkg.Language)
			}

		case "java", "kotlin":
			// Java and Kotlin packages use Gradle composite builds. Dependencies
			// are referenced by directory name in settings.gradle.kts via
			// includeBuild("../dep-name"). The directory name maps directly to
			// the internal package name, same as Swift and Rust.
			dirBase := strings.ToLower(filepath.Base(pkg.Path))
			setKnown(dirBase, pkg.Name, pkg.Path, pkg.Language)

		case "dotnet", "csharp", "fsharp":
			// .NET (C#/F#) packages use MSBuild ProjectReference elements, which
			// reference sibling directories by name. The directory name maps
			// directly to the NuGet package name by convention in this repo.
			dirBase := strings.ToLower(filepath.Base(pkg.Path))
			setKnown(dirBase, pkg.Name, pkg.Path, pkg.Language)
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

// findCabalFile returns the sole Cabal manifest in a package directory.
// Multiple manifests are ambiguous, so reject them instead of allowing
// directory enumeration order to change package identity or dependencies.
func findCabalFile(pkgPath string) string {
	entries, err := os.ReadDir(pkgPath)
	if err != nil {
		return ""
	}
	var cabalFile string
	for _, entry := range entries {
		if !entry.IsDir() && strings.HasSuffix(strings.ToLower(entry.Name()), ".cabal") {
			if cabalFile != "" {
				return ""
			}
			cabalFile = filepath.Join(pkgPath, entry.Name())
		}
	}
	return cabalFile
}

func readCabalPackageName(pkgPath string) string {
	cabalFile := findCabalFile(pkgPath)
	if cabalFile == "" {
		return ""
	}
	data, err := os.ReadFile(cabalFile)
	if err != nil {
		return ""
	}
	re := regexp.MustCompile(`(?m)^\s*name\s*:\s*([a-zA-Z0-9][a-zA-Z0-9-]*)\s*$`)
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
func ResolveDependencies(packages []discovery.Package) (*directedgraph.Graph, error) {
	graph := directedgraph.New()

	// First, add all packages as nodes. Even packages with no dependencies
	// need to be in the graph so they appear in independent_groups().
	for _, pkg := range packages {
		graph.AddNode(pkg.Name)
	}

	// Build the ecosystem-specific name mapping table.
	knownNamesByLanguage := make(map[string]map[string]string)
	knownGradlePathsByLanguage := make(map[string]map[string]string)
	knownDotnetProjectPaths := buildKnownDotnetProjectPaths(packages)
	knownPackageNames := make(map[string]bool, len(packages))
	for _, pkg := range packages {
		knownPackageNames[pkg.Name] = true
		if _, ok := knownNamesByLanguage[pkg.Language]; !ok {
			knownNamesByLanguage[pkg.Language] = buildKnownNamesForLanguage(packages, pkg.Language)
		}
		if (pkg.Language == "java" || pkg.Language == "kotlin") && knownGradlePathsByLanguage[pkg.Language] == nil {
			knownGradlePathsByLanguage[pkg.Language] = buildKnownGradlePathsForLanguage(packages, pkg.Language)
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
		case "rust":
			deps = parseRustDeps(pkg, knownNames)
		case "wasm":
			deps = parseRustDeps(pkg, knownNames)
		case "elixir":
			deps = parseElixirDeps(pkg, knownNames)
		case "lua":
			var err error
			deps, err = parseLuaDeps(pkg, knownNames)
			if err != nil {
				return nil, err
			}
		case "perl":
			deps = parsePerlDeps(pkg, knownNames)
		case "swift":
			deps = parseSwiftDeps(pkg, knownNames)
		case "haskell":
			deps = parseHaskellDeps(pkg, knownNames)
		case "ocaml":
			deps = parseOCamlDeps(pkg, knownNames)
		case "java", "kotlin":
			deps = parseGradleDeps(pkg, knownGradlePathsByLanguage[pkg.Language])
		case "dotnet", "csharp", "fsharp":
			deps = parseDotnetDeps(pkg, knownDotnetProjectPaths)
		}
		deps = append(deps, parseBuildToolDeps(pkg, knownPackageNames)...)

		for _, depName := range deps {
			// Edge direction: dep → pkg means "dep must be built before pkg".
			// This convention makes IndependentGroups() produce the correct
			// build order: nodes with zero in-degree (no deps) come first.
			graph.AddEdge(depName, pkg.Name)
		}
	}

	return graph, nil
}

// BuildKnownNames is exported for testing. It delegates to buildKnownNames.
func BuildKnownNames(packages []discovery.Package) map[string]string {
	return buildKnownNames(packages)
}
