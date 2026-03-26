// capability-cage-generator reads a required_capabilities.json manifest file
// and emits a gen_capabilities.go source file that bakes the capability cage
// infrastructure directly into the package at compile time.
//
// # Motivation
//
// Traditional capability systems require a shared runtime package, creating a
// supply chain attack surface: if the shared package is compromised, every
// consumer is affected. By generating self-contained code per package, we
// eliminate that fan-out. Each published package is a frozen snapshot —
// no external runtime dependency, no shared cage package.
//
// The generated file contains:
//   - _XxxCapabilities namespace structs: one per declared capability category,
//     accessible as fields on Operation[T] (e.g., op.File, op.Net, op.Env).
//     Fields only exist when declared — undeclared access is a compile error.
//   - OperationResult[T]: three-state outcome (success, expected failure, unexpected)
//   - ResultFactory[T]: creates OperationResult values inside callbacks
//   - Operation[T]: the unit of work with timing, logging, and panic recovery
//   - StartNew[T]: constructs an Operation without executing it
//   - _capabilityViolationError: returned when an undeclared OS op is attempted
//
// # Usage
//
//	# Single package
//	capability-cage-generator --manifest=code/packages/go/directed-graph/required_capabilities.json
//
//	# All Go packages in the repo
//	capability-cage-generator --all
//
//	# Preview without writing
//	capability-cage-generator --manifest=... --dry-run
//	capability-cage-generator --all --dry-run
//
// # Wildcard policy
//
// Scopeable categories (fs, net, proc, env, ffi) must declare exact targets.
// Wildcards ("*") are rejected at generation time. Non-scopeable categories
// (time, stdin, stdout) always use "*" — there is nothing to narrow.
package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strings"
)

// ─────────────────────────────────────────────────────────────────────────────
// JSON manifest types (mirrors required_capabilities.schema.json)
// ─────────────────────────────────────────────────────────────────────────────

// manifestJSON is the raw JSON structure of required_capabilities.json.
type manifestJSON struct {
	Schema        string           `json:"$schema"`
	Version       int              `json:"version"`
	Package       string           `json:"package"`
	Capabilities  []capabilityJSON `json:"capabilities"`
	Justification string           `json:"justification"`
}

// capabilityJSON is one structured capability object in the JSON.
type capabilityJSON struct {
	Category      string `json:"category"`
	Action        string `json:"action"`
	Target        string `json:"target"`
	Justification string `json:"justification"`
}

// ─────────────────────────────────────────────────────────────────────────────
// Wildcard policy
// ─────────────────────────────────────────────────────────────────────────────

// scopeableCategory returns true for categories where exact targets provide
// meaningful runtime enforcement. Non-scopeable categories (time, stdin,
// stdout) always use "*" because they are not path-based.
func scopeableCategory(cat string) bool {
	switch cat {
	case "fs", "net", "proc", "env", "ffi":
		return true
	}
	return false
}

// isWildcardTarget reports whether a target is the unrestricted wildcard "*".
// Wildcard targets are permitted for packages that accept user-provided paths
// (e.g., a Starlark interpreter that load()s arbitrary .star files). These
// packages declare their capability in the manifest for code-review visibility
// but cannot enumerate exact paths at development time. The generated method
// provides the capability gate (so OS calls are routed through op.File etc.)
// but performs no runtime path restriction.
func isWildcardTarget(target string) bool {
	return target == "*"
}

// hasWildcard returns true if any target in the slice is a wildcard.
func hasWildcard(targets []string) bool {
	for _, t := range targets {
		if isWildcardTarget(t) {
			return true
		}
	}
	return false
}

// ─────────────────────────────────────────────────────────────────────────────
// Capability grouping
// ─────────────────────────────────────────────────────────────────────────────

// capabilityGroup collects all targets declared for a single (category, action)
// pair. Multiple fs:read entries with different targets are merged into one
// ReadFile method that permits any of the declared paths.
type capabilityGroup struct {
	Category string
	Action   string
	Targets  []string
}

// groupCapabilities returns capabilities grouped by (category, action),
// preserving the declaration order of first occurrence.
func groupCapabilities(caps []capabilityJSON) []capabilityGroup {
	type key struct{ cat, act string }
	seen := make(map[key]int) // key → index in groups
	var groups []capabilityGroup
	for _, c := range caps {
		k := key{c.Category, c.Action}
		if idx, ok := seen[k]; ok {
			groups[idx].Targets = append(groups[idx].Targets, c.Target)
		} else {
			seen[k] = len(groups)
			groups = append(groups, capabilityGroup{
				Category: c.Category,
				Action:   c.Action,
				Targets:  []string{c.Target},
			})
		}
	}
	return groups
}

// ─────────────────────────────────────────────────────────────────────────────
// Capability namespace naming
// ─────────────────────────────────────────────────────────────────────────────

// categoryFieldName returns the exported field name for a capability category
// on Operation[T]. E.g., "fs" → "File", "net" → "Net".
func categoryFieldName(cat string) string {
	switch cat {
	case "fs":
		return "File"
	case "net":
		return "Net"
	case "proc":
		return "Proc"
	case "env":
		return "Env"
	case "time":
		return "Time"
	case "stdin":
		return "Stdin"
	case "stdout":
		return "Stdout"
	default:
		if len(cat) == 0 {
			return "Unknown"
		}
		return strings.ToUpper(cat[:1]) + cat[1:]
	}
}

// categoryTypeName returns the unexported struct type name for a capability
// category. E.g., "fs" → "_FileCapabilities".
func categoryTypeName(cat string) string {
	return "_" + categoryFieldName(cat) + "Capabilities"
}

// uniqueCategories returns the distinct categories present in groups,
// preserving first-occurrence order.
func uniqueCategories(groups []capabilityGroup) []string {
	seen := map[string]bool{}
	var cats []string
	for _, g := range groups {
		if !seen[g.Category] {
			seen[g.Category] = true
			cats = append(cats, g.Category)
		}
	}
	return cats
}

// ─────────────────────────────────────────────────────────────────────────────
// Import computation
// ─────────────────────────────────────────────────────────────────────────────

// neededImports returns the sorted stdlib import paths required by the
// generated code. Always includes fmt. Additional packages depend on which
// capability categories are declared.
func neededImports(caps []capabilityJSON) []string {
	set := map[string]bool{
		`"fmt"`: true,
	}
	for _, c := range caps {
		switch c.Category {
		case "fs", "env", "stdout", "proc":
			set[`"os"`] = true
		case "stdin":
			set[`"io"`] = true
			set[`"os"`] = true
		case "net":
			set[`"net"`] = true
		case "time":
			set[`"time"`] = true
		}
		if c.Category == "proc" && c.Action == "exec" {
			set[`"os/exec"`] = true
		}
		// Path-based categories (fs, proc) need filepath for path normalization
		// in Cage methods to prevent bypass via ./foo, ../foo/bar, etc.
		if c.Category == "fs" || (c.Category == "proc" && c.Action == "exec") {
			set[`"path/filepath"`] = true
		}
		// Relative fs targets (../../grammars/foo.tokens) are resolved to
		// canonical absolute paths at startup using runtime.Caller(0) and
		// sync.OnceValue, then enforced with exact equality.
		if c.Category == "fs" && isRelativeTarget(c.Target) {
			set[`"runtime"`] = true
			set[`"sync"`] = true
		}
	}
	result := make([]string, 0, len(set))
	for imp := range set {
		result = append(result, imp)
	}
	sort.Strings(result)
	return result
}

// ─────────────────────────────────────────────────────────────────────────────
// Package name derivation
// ─────────────────────────────────────────────────────────────────────────────

// goPackageName derives the Go package name from the manifest's package field.
//
// Strategy:
//  1. Take the last segment of the "language/pkg-name" package field.
//  2. Remove hyphens (Go identifiers cannot contain "-").
//  3. If a go.mod file exists alongside the manifest, read the package name
//     from the "package" declaration in any .go file instead (more accurate).
//
// Examples:
//   - "go/verilog-lexer" → "veriloglexer"
//   - "go/sql-parser"    → "sqlparser"
//   - "go/json-value"    → "jsonvalue"
func goPackageName(manifestPath string, pkgField string) (string, error) {
	// First, try reading an existing .go file in the same directory.
	dir := filepath.Dir(manifestPath)
	goFiles, _ := filepath.Glob(filepath.Join(dir, "*.go"))
	for _, f := range goFiles {
		if strings.HasSuffix(f, "_test.go") {
			continue
		}
		if strings.HasSuffix(f, "gen_capabilities.go") {
			continue
		}
		data, err := os.ReadFile(f) //nolint:cap
		if err != nil {
			continue
		}
		// Find "package <name>" at the top of the file.
		re := regexp.MustCompile(`(?m)^package\s+(\w+)`)
		if m := re.FindSubmatch(data); m != nil {
			return string(m[1]), nil
		}
	}

	// Fallback: derive from the package field string.
	// "go/verilog-lexer" → take "verilog-lexer", remove hyphens.
	parts := strings.SplitN(pkgField, "/", 2)
	if len(parts) != 2 {
		return "", fmt.Errorf("invalid package field %q: expected language/name format", pkgField)
	}
	name := parts[1]
	name = strings.ReplaceAll(name, "-", "")
	name = strings.ReplaceAll(name, "_", "")
	return name, nil
}

// ─────────────────────────────────────────────────────────────────────────────
// Code generation — file header
// ─────────────────────────────────────────────────────────────────────────────

func emitHeader(b *strings.Builder, manifestPath string) {
	fmt.Fprintf(b, "// Code generated by capability-cage-generator. DO NOT EDIT.\n")
	fmt.Fprintf(b, "//\n")
	fmt.Fprintf(b, "// Source: required_capabilities.json\n")
	fmt.Fprintf(b, "// Regenerate:\n")
	fmt.Fprintf(b, "//   go run github.com/adhithyan15/coding-adventures/code/programs/go/capability-cage-generator \\\n")
	// Sanitize the path before embedding in the generated file comment.
	// Strip all ASCII control characters (0x00–0x1f, 0x7f) and Unicode
	// line terminators (\u2028, \u2029) to prevent comment/header injection.
	// These characters could otherwise: break the single-line comment, inject
	// additional Go source lines, or confuse terminals via ANSI sequences.
	// Also convert to forward slashes for platform-independent output.
	rawPath := filepath.ToSlash(manifestPath)
	var safePathBuilder strings.Builder
	for _, r := range rawPath {
		if r >= 0x20 && r != 0x7f && r != '\u2028' && r != '\u2029' {
			safePathBuilder.WriteRune(r)
		}
	}
	safePath := safePathBuilder.String()
	fmt.Fprintf(b, "//     --manifest=%s\n", safePath)
	fmt.Fprintf(b, "//\n")
	fmt.Fprintf(b, "// The JSON file is a development-time artifact; this file is what the\n")
	fmt.Fprintf(b, "// runtime enforces. Edit required_capabilities.json and re-run the\n")
	fmt.Fprintf(b, "// generator to change capabilities — never edit this file directly.\n")
	fmt.Fprintf(b, "\n")
}

// ─────────────────────────────────────────────────────────────────────────────
// Code generation — imports
// ─────────────────────────────────────────────────────────────────────────────

func emitImports(b *strings.Builder, imports []string) {
	fmt.Fprintf(b, "import (\n")
	for _, imp := range imports {
		fmt.Fprintf(b, "\t%s\n", imp)
	}
	fmt.Fprintf(b, ")\n\n")
}

// ─────────────────────────────────────────────────────────────────────────────
// Code generation — capability namespace structs
// ─────────────────────────────────────────────────────────────────────────────

// emitCapabilityStructs emits one namespace struct per capability category.
// Each struct becomes a field on Operation[T] (e.g., op.File, op.Net).
// The field only exists when the corresponding capability is declared —
// accessing op.File in a zero-file-capability package is a compile error.
func emitCapabilityStructs(b *strings.Builder, groups []capabilityGroup, targetToVar map[string]string) {
	cats := uniqueCategories(groups)
	for _, cat := range cats {
		typeName := categoryTypeName(cat)
		fieldName := categoryFieldName(cat)
		fmt.Fprintf(b, "// ─────────────────────────────────────────────────────────────────────────────\n")
		fmt.Fprintf(b, "// %s — op.%s capability namespace\n", typeName, fieldName)
		fmt.Fprintf(b, "//\n")
		fmt.Fprintf(b, "// Accessible via op.%s inside any Operation callback. The field only\n", fieldName)
		fmt.Fprintf(b, "// exists when %s capabilities are declared in required_capabilities.json.\n", cat)
		fmt.Fprintf(b, "// Undeclared categories produce a compile error at the call site.\n")
		fmt.Fprintf(b, "// ─────────────────────────────────────────────────────────────────────────────\n")
		fmt.Fprintf(b, "\n")
		fmt.Fprintf(b, "type %s struct{}\n\n", typeName)
		for _, g := range groups {
			if g.Category == cat {
				emitCapabilityMethod(b, g, typeName, targetToVar)
			}
		}
	}
}

// isRelativeTarget returns true if the target starts with "./" or "../",
// indicating a path relative to the package directory. Relative targets
// are resolved to canonical absolute paths at startup using runtime.Caller(0)
// and sync.OnceValue, then enforced with exact equality.
func isRelativeTarget(target string) bool {
	return strings.HasPrefix(target, "./") || strings.HasPrefix(target, "../")
}

// collectRelativeTargets returns the distinct relative targets across all groups,
// preserving first-seen order. Used to emit _allowedPath_N vars.
func collectRelativeTargets(groups []capabilityGroup) []string {
	seen := map[string]bool{}
	var result []string
	for _, g := range groups {
		for _, t := range g.Targets {
			if isRelativeTarget(t) && !seen[t] {
				seen[t] = true
				result = append(result, t)
			}
		}
	}
	return result
}

// emitResolvedPathVars emits package-level sync.OnceValue vars that resolve
// each relative target to its canonical absolute path at startup.
//
// Using runtime.Caller(0) from gen_capabilities.go — which lives in the same
// directory as the rest of the package — ensures the resolved path matches
// exactly what getGrammarPath() computes in the package's own source. The
// path is cleaned with filepath.Clean to canonicalize separators and collapse
// any remaining . or .. components.
//
// Returns a map from target string to the emitted variable name, so that
// emitScopedCheck can reference the correct var.
func emitResolvedPathVars(b *strings.Builder, relTargets []string) map[string]string {
	targetToVar := make(map[string]string)
	if len(relTargets) == 0 {
		return targetToVar
	}
	fmt.Fprintf(b, "// ─────────────────────────────────────────────────────────────────────────────\n")
	fmt.Fprintf(b, "// Resolved allowed paths — exact canonical paths, computed once at startup\n")
	fmt.Fprintf(b, "//\n")
	fmt.Fprintf(b, "// Each var below resolves a relative target from required_capabilities.json\n")
	fmt.Fprintf(b, "// to its canonical absolute path, anchored to gen_capabilities.go's directory\n")
	fmt.Fprintf(b, "// via runtime.Caller(0). Enforcement uses exact equality, so no other file\n")
	fmt.Fprintf(b, "// — even one with the same name in a different directory — can pass the check.\n")
	fmt.Fprintf(b, "// ─────────────────────────────────────────────────────────────────────────────\n")
	fmt.Fprintf(b, "\n")
	for i, t := range relTargets {
		varName := fmt.Sprintf("_allowedPath_%d", i)
		targetToVar[t] = varName
		fmt.Fprintf(b, "var %s = sync.OnceValue(func() string {\n", varName)
		fmt.Fprintf(b, "\t_, _file, _, _ := runtime.Caller(0)\n")
		fmt.Fprintf(b, "\treturn filepath.Clean(filepath.Join(filepath.Dir(_file), %q))\n", t)
		fmt.Fprintf(b, "})\n\n")
	}
	return targetToVar
}

// emitScopedCheck emits the allowed-path check at the top of a capability method.
// paramName is the method parameter to check (e.g., "path", "addr", "key").
// twoReturns controls whether the violation returns (nil, err) or just (err).
// targetToVar maps relative target strings to their _allowedPath_N variable names.
//
// Relative targets use pre-emitted sync.OnceValue vars for exact canonical path
// comparison. Absolute targets use inline exact string equality. Mixed groups are
// handled correctly — each target type uses its own comparison form.
func emitScopedCheck(b *strings.Builder, targets []string, paramName string, twoReturns bool, cat, action string, targetToVar map[string]string) {
	// Wildcard target: no path restriction — the package declared an unconstrained
	// capability (e.g. it accepts user-provided paths). The value of the generated
	// method is routing all OS calls through op.File for code-review visibility;
	// runtime enforcement is intentionally absent for wildcard capabilities.
	for _, t := range targets {
		if isWildcardTarget(t) {
			return
		}
	}

	if len(targets) == 1 {
		t := targets[0]
		if varName, ok := targetToVar[t]; ok {
			// Relative target: compare against the pre-resolved canonical path.
			fmt.Fprintf(b, "\tif %s != %s() {\n", paramName, varName)
		} else {
			// Absolute target: exact equality.
			fmt.Fprintf(b, "\tif %s != %q {\n", paramName, t)
		}
	} else {
		// Multiple targets: allowed if any one matches.
		fmt.Fprintf(b, "\t_allowed := false\n")
		for _, t := range targets {
			if varName, ok := targetToVar[t]; ok {
				fmt.Fprintf(b, "\tif %s == %s() {\n\t\t_allowed = true\n\t}\n", paramName, varName)
			} else {
				fmt.Fprintf(b, "\tif %s == %q {\n\t\t_allowed = true\n\t}\n", paramName, t)
			}
		}
		fmt.Fprintf(b, "\tif !_allowed {\n")
	}

	if twoReturns {
		fmt.Fprintf(b, "\t\treturn nil, &_capabilityViolationError{category: %q, action: %q, requested: %s}\n", cat, action, paramName)
	} else {
		fmt.Fprintf(b, "\t\treturn &_capabilityViolationError{category: %q, action: %q, requested: %s}\n", cat, action, paramName)
	}
	fmt.Fprintf(b, "\t}\n")
}

func emitCapabilityMethod(b *strings.Builder, g capabilityGroup, typeName string, targetToVar map[string]string) {
	key := g.Category + ":" + g.Action
	switch key {
	case "fs:read":
		if hasWildcard(g.Targets) {
			fmt.Fprintf(b, "// ReadFile reads the file at path.\n")
			fmt.Fprintf(b, "// This package declared a wildcard capability (\"*\").\n")
			fmt.Fprintf(b, "// No runtime path restriction is enforced — all paths are permitted.\n")
			fmt.Fprintf(b, "// The path is cleaned with filepath.Clean before use.\n")
		} else {
			fmt.Fprintf(b, "// ReadFile reads the file at path.\n")
			fmt.Fprintf(b, "// Only paths declared in required_capabilities.json are permitted.\n")
			fmt.Fprintf(b, "// The path is cleaned with filepath.Clean before comparison to prevent\n")
			fmt.Fprintf(b, "// bypass via ./foo, foo/../foo/bar, and similar path manipulations.\n")
		}
		fmt.Fprintf(b, "func (c *%s) ReadFile(path string) ([]byte, error) {\n", typeName)
		fmt.Fprintf(b, "\tpath = filepath.Clean(path)\n")
		emitScopedCheck(b, g.Targets, "path", true, "fs", "read", targetToVar)
		fmt.Fprintf(b, "\treturn os.ReadFile(path) //nolint:cap\n")
		fmt.Fprintf(b, "}\n\n")

	case "fs:write":
		if hasWildcard(g.Targets) {
			fmt.Fprintf(b, "// WriteFile writes data to the file at path.\n")
			fmt.Fprintf(b, "// This package declared a wildcard capability (\"*\").\n")
			fmt.Fprintf(b, "// No runtime path restriction is enforced — all paths are permitted.\n")
			fmt.Fprintf(b, "// The path is cleaned with filepath.Clean before use.\n")
		} else {
			fmt.Fprintf(b, "// WriteFile writes data to the file at path.\n")
			fmt.Fprintf(b, "// Only paths declared in required_capabilities.json are permitted.\n")
			fmt.Fprintf(b, "// The path is cleaned with filepath.Clean before comparison.\n")
		}
		fmt.Fprintf(b, "func (c *%s) WriteFile(path string, data []byte, perm os.FileMode) error {\n", typeName)
		fmt.Fprintf(b, "\tpath = filepath.Clean(path)\n")
		emitScopedCheck(b, g.Targets, "path", false, "fs", "write", targetToVar)
		fmt.Fprintf(b, "\treturn os.WriteFile(path, data, perm) //nolint:cap\n")
		fmt.Fprintf(b, "}\n\n")

	case "fs:create":
		if hasWildcard(g.Targets) {
			fmt.Fprintf(b, "// CreateFile creates or truncates the file at path.\n")
			fmt.Fprintf(b, "// This package declared a wildcard capability (\"*\").\n")
			fmt.Fprintf(b, "// No runtime path restriction is enforced — all paths are permitted.\n")
			fmt.Fprintf(b, "// The path is cleaned with filepath.Clean before use.\n")
		} else {
			fmt.Fprintf(b, "// CreateFile creates or truncates the file at path.\n")
			fmt.Fprintf(b, "// Only paths declared in required_capabilities.json are permitted.\n")
			fmt.Fprintf(b, "// The path is cleaned with filepath.Clean before comparison.\n")
		}
		fmt.Fprintf(b, "func (c *%s) CreateFile(path string) (*os.File, error) {\n", typeName)
		fmt.Fprintf(b, "\tpath = filepath.Clean(path)\n")
		emitScopedCheck(b, g.Targets, "path", true, "fs", "create", targetToVar)
		fmt.Fprintf(b, "\treturn os.Create(path) //nolint:cap\n")
		fmt.Fprintf(b, "}\n\n")

	case "fs:delete":
		if hasWildcard(g.Targets) {
			fmt.Fprintf(b, "// DeleteFile removes the file at path.\n")
			fmt.Fprintf(b, "// This package declared a wildcard capability (\"*\").\n")
			fmt.Fprintf(b, "// No runtime path restriction is enforced — all paths are permitted.\n")
			fmt.Fprintf(b, "// The path is cleaned with filepath.Clean before use.\n")
		} else {
			fmt.Fprintf(b, "// DeleteFile removes the file at path.\n")
			fmt.Fprintf(b, "// Only paths declared in required_capabilities.json are permitted.\n")
			fmt.Fprintf(b, "// The path is cleaned with filepath.Clean before comparison.\n")
		}
		fmt.Fprintf(b, "func (c *%s) DeleteFile(path string) error {\n", typeName)
		fmt.Fprintf(b, "\tpath = filepath.Clean(path)\n")
		emitScopedCheck(b, g.Targets, "path", false, "fs", "delete", targetToVar)
		fmt.Fprintf(b, "\treturn os.Remove(path) //nolint:cap\n")
		fmt.Fprintf(b, "}\n\n")

	case "fs:list":
		if hasWildcard(g.Targets) {
			fmt.Fprintf(b, "// ReadDir lists the contents of the directory at path.\n")
			fmt.Fprintf(b, "// This package declared a wildcard capability (\"*\").\n")
			fmt.Fprintf(b, "// No runtime path restriction is enforced — all paths are permitted.\n")
			fmt.Fprintf(b, "// The path is cleaned with filepath.Clean before use.\n")
		} else {
			fmt.Fprintf(b, "// ReadDir lists the contents of the directory at path.\n")
			fmt.Fprintf(b, "// Only paths declared in required_capabilities.json are permitted.\n")
			fmt.Fprintf(b, "// The path is cleaned with filepath.Clean before comparison.\n")
		}
		fmt.Fprintf(b, "func (c *%s) ReadDir(path string) ([]os.DirEntry, error) {\n", typeName)
		fmt.Fprintf(b, "\tpath = filepath.Clean(path)\n")
		emitScopedCheck(b, g.Targets, "path", true, "fs", "list", targetToVar)
		fmt.Fprintf(b, "\treturn os.ReadDir(path) //nolint:cap\n")
		fmt.Fprintf(b, "}\n\n")

	case "fs:open":
		if hasWildcard(g.Targets) {
			fmt.Fprintf(b, "// OpenFile opens the named file for reading.\n")
			fmt.Fprintf(b, "// This package declared a wildcard capability (\"*\").\n")
			fmt.Fprintf(b, "// No runtime path restriction is enforced — all paths are permitted.\n")
			fmt.Fprintf(b, "// The path is cleaned with filepath.Clean before use.\n")
		} else {
			fmt.Fprintf(b, "// OpenFile opens the named file for reading.\n")
			fmt.Fprintf(b, "// Only paths declared in required_capabilities.json are permitted.\n")
			fmt.Fprintf(b, "// The path is cleaned with filepath.Clean before comparison.\n")
		}
		fmt.Fprintf(b, "func (c *%s) OpenFile(path string) (*os.File, error) {\n", typeName)
		fmt.Fprintf(b, "\tpath = filepath.Clean(path)\n")
		emitScopedCheck(b, g.Targets, "path", true, "fs", "open", targetToVar)
		fmt.Fprintf(b, "\treturn os.Open(path) //nolint:cap\n")
		fmt.Fprintf(b, "}\n\n")

	case "fs:mkdir":
		if hasWildcard(g.Targets) {
			fmt.Fprintf(b, "// MkdirAll creates a directory named path, along with any necessary parents.\n")
			fmt.Fprintf(b, "// This package declared a wildcard capability (\"*\").\n")
			fmt.Fprintf(b, "// No runtime path restriction is enforced — all paths are permitted.\n")
			fmt.Fprintf(b, "// The path is cleaned with filepath.Clean before use.\n")
		} else {
			fmt.Fprintf(b, "// MkdirAll creates a directory named path, along with any necessary parents.\n")
			fmt.Fprintf(b, "// Only paths declared in required_capabilities.json are permitted.\n")
			fmt.Fprintf(b, "// The path is cleaned with filepath.Clean before comparison.\n")
		}
		fmt.Fprintf(b, "func (c *%s) MkdirAll(path string, perm os.FileMode) error {\n", typeName)
		fmt.Fprintf(b, "\tpath = filepath.Clean(path)\n")
		emitScopedCheck(b, g.Targets, "path", false, "fs", "mkdir", targetToVar)
		fmt.Fprintf(b, "\treturn os.MkdirAll(path, perm) //nolint:cap\n")
		fmt.Fprintf(b, "}\n\n")

	case "net:connect":
		fmt.Fprintf(b, "// Connect opens a network connection to addr.\n")
		fmt.Fprintf(b, "// Only addresses declared in required_capabilities.json are permitted.\n")
		fmt.Fprintf(b, "func (c *%s) Connect(network, addr string) (net.Conn, error) {\n", typeName)
		emitScopedCheck(b, g.Targets, "addr", true, "net", "connect", targetToVar)
		fmt.Fprintf(b, "\treturn net.Dial(network, addr) //nolint:cap\n")
		fmt.Fprintf(b, "}\n\n")

	case "net:listen":
		fmt.Fprintf(b, "// Listen opens a network listener on addr.\n")
		fmt.Fprintf(b, "// Only addresses declared in required_capabilities.json are permitted.\n")
		fmt.Fprintf(b, "func (c *%s) Listen(network, addr string) (net.Listener, error) {\n", typeName)
		emitScopedCheck(b, g.Targets, "addr", true, "net", "listen", targetToVar)
		fmt.Fprintf(b, "\treturn net.Listen(network, addr) //nolint:cap\n")
		fmt.Fprintf(b, "}\n\n")

	case "net:dns":
		fmt.Fprintf(b, "// LookupHost resolves host to a list of IP addresses.\n")
		fmt.Fprintf(b, "// Only hosts declared in required_capabilities.json are permitted.\n")
		fmt.Fprintf(b, "func (c *%s) LookupHost(host string) ([]string, error) {\n", typeName)
		emitScopedCheck(b, g.Targets, "host", true, "net", "dns", targetToVar)
		fmt.Fprintf(b, "\treturn net.LookupHost(host) //nolint:cap\n")
		fmt.Fprintf(b, "}\n\n")

	case "proc:exec":
		fmt.Fprintf(b, "// Exec runs the named executable with args.\n")
		fmt.Fprintf(b, "// Only executables declared in required_capabilities.json are permitted.\n")
		fmt.Fprintf(b, "// The name is cleaned with filepath.Clean before comparison.\n")
		fmt.Fprintf(b, "//\n")
		fmt.Fprintf(b, "// SECURITY NOTE: Only the executable path is enforced — NOT the arguments.\n")
		fmt.Fprintf(b, "// An attacker who controls args can potentially weaponize an allowed binary\n")
		fmt.Fprintf(b, "// (e.g., pass -c 'malicious code' to an allowed interpreter).\n")
		fmt.Fprintf(b, "// Callers are responsible for validating arguments before passing them here.\n")
		fmt.Fprintf(b, "// If argument-level restriction is needed, declare narrower capabilities\n")
		fmt.Fprintf(b, "// or enforce argument constraints in the calling code.\n")
		fmt.Fprintf(b, "func (c *%s) Exec(name string, args ...string) ([]byte, error) {\n", typeName)
		fmt.Fprintf(b, "\tname = filepath.Clean(name)\n")
		emitScopedCheck(b, g.Targets, "name", true, "proc", "exec", targetToVar)
		fmt.Fprintf(b, "\treturn exec.Command(name, args...).Output() //nolint:cap\n")
		fmt.Fprintf(b, "}\n\n")

	case "env:read":
		fmt.Fprintf(b, "// Getenv returns the value of the environment variable key.\n")
		fmt.Fprintf(b, "// Only variables declared in required_capabilities.json are permitted.\n")
		fmt.Fprintf(b, "func (c *%s) Getenv(key string) (string, error) {\n", typeName)
		emitScopedCheck(b, g.Targets, "key", true, "env", "read", targetToVar)
		fmt.Fprintf(b, "\treturn os.Getenv(key), nil //nolint:cap\n")
		fmt.Fprintf(b, "}\n\n")

	case "time:read":
		fmt.Fprintf(b, "// Now returns the current local time.\n")
		fmt.Fprintf(b, "func (c *%s) Now() time.Time {\n", typeName)
		fmt.Fprintf(b, "\treturn time.Now() //nolint:cap\n")
		fmt.Fprintf(b, "}\n\n")

	case "time:sleep":
		fmt.Fprintf(b, "// Sleep pauses the current goroutine for duration d.\n")
		fmt.Fprintf(b, "func (c *%s) Sleep(d time.Duration) {\n", typeName)
		fmt.Fprintf(b, "\ttime.Sleep(d)\n")
		fmt.Fprintf(b, "}\n\n")

	case "stdin:read":
		fmt.Fprintf(b, "// ReadStdin reads all bytes from standard input.\n")
		fmt.Fprintf(b, "func (c *%s) ReadStdin() ([]byte, error) {\n", typeName)
		fmt.Fprintf(b, "\treturn io.ReadAll(os.Stdin) //nolint:cap\n")
		fmt.Fprintf(b, "}\n\n")

	case "stdout:write":
		fmt.Fprintf(b, "// WriteStdout writes data to standard output.\n")
		fmt.Fprintf(b, "func (c *%s) WriteStdout(data []byte) (int, error) {\n", typeName)
		fmt.Fprintf(b, "\treturn os.Stdout.Write(data) //nolint:cap\n")
		fmt.Fprintf(b, "}\n\n")

	default:
		fmt.Fprintf(b, "// TODO: implement capability method for %s:%s\n\n", g.Category, g.Action)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Code generation — OperationResult
// ─────────────────────────────────────────────────────────────────────────────

func emitOperationResult(b *strings.Builder) {
	fmt.Fprintf(b, "// ─────────────────────────────────────────────────────────────────────────────\n")
	fmt.Fprintf(b, "// OperationResult — three-state outcome\n")
	fmt.Fprintf(b, "//\n")
	fmt.Fprintf(b, "// Every Operation callback returns one of three outcomes:\n")
	fmt.Fprintf(b, "//   Success:             DidSucceed=true,  DidFailUnexpectedly=false\n")
	fmt.Fprintf(b, "//   Expected failure:    DidSucceed=false, DidFailUnexpectedly=false\n")
	fmt.Fprintf(b, "//   Unexpected failure:  DidSucceed=false, DidFailUnexpectedly=true\n")
	fmt.Fprintf(b, "//\n")
	fmt.Fprintf(b, "// \"Expected failure\" means the operation ran normally but produced a\n")
	fmt.Fprintf(b, "// negative result (e.g. node not found, permission denied).\n")
	fmt.Fprintf(b, "// \"Unexpected failure\" means the callback panicked; fallback value used.\n")
	fmt.Fprintf(b, "//\n")
	fmt.Fprintf(b, "// Err carries a typed error for expected failures. When set, GetResult\n")
	fmt.Fprintf(b, "// returns it directly — preserving its type for errors.As checks.\n")
	fmt.Fprintf(b, "// ─────────────────────────────────────────────────────────────────────────────\n")
	fmt.Fprintf(b, "\n")
	fmt.Fprintf(b, "type OperationResult[T any] struct {\n")
	fmt.Fprintf(b, "\tDidSucceed          bool\n")
	fmt.Fprintf(b, "\tDidFailUnexpectedly bool\n")
	fmt.Fprintf(b, "\tReturnValue         T\n")
	fmt.Fprintf(b, "\tErr                 error // if non-nil, returned as-is by GetResult for expected failures\n")
	fmt.Fprintf(b, "}\n\n")
}

// ─────────────────────────────────────────────────────────────────────────────
// Code generation — ResultFactory
// ─────────────────────────────────────────────────────────────────────────────

func emitResultFactory(b *strings.Builder) {
	fmt.Fprintf(b, "// ─────────────────────────────────────────────────────────────────────────────\n")
	fmt.Fprintf(b, "// ResultFactory — creates OperationResult values inside callbacks\n")
	fmt.Fprintf(b, "// ─────────────────────────────────────────────────────────────────────────────\n")
	fmt.Fprintf(b, "\n")
	fmt.Fprintf(b, "type ResultFactory[T any] struct{}\n")
	fmt.Fprintf(b, "\n")
	fmt.Fprintf(b, "// Generate creates an OperationResult for the common case.\n")
	fmt.Fprintf(b, "// Pass didSucceed=true for success, false for failure.\n")
	fmt.Fprintf(b, "// Set didFailUnexpectedly=true only to signal internal programming errors;\n")
	fmt.Fprintf(b, "// prefer letting the Operation framework catch panics automatically.\n")
	fmt.Fprintf(b, "func (f *ResultFactory[T]) Generate(didSucceed bool, didFailUnexpectedly bool, value T) *OperationResult[T] {\n")
	fmt.Fprintf(b, "\treturn &OperationResult[T]{\n")
	fmt.Fprintf(b, "\t\tDidSucceed:          didSucceed,\n")
	fmt.Fprintf(b, "\t\tDidFailUnexpectedly: didFailUnexpectedly,\n")
	fmt.Fprintf(b, "\t\tReturnValue:         value,\n")
	fmt.Fprintf(b, "\t}\n")
	fmt.Fprintf(b, "}\n")
	fmt.Fprintf(b, "\n")
	fmt.Fprintf(b, "// Fail creates an expected-failure OperationResult with a specific error.\n")
	fmt.Fprintf(b, "// Use this when the failure has a typed error (e.g., NodeNotFoundError)\n")
	fmt.Fprintf(b, "// that callers may inspect with errors.As. The error is returned unchanged\n")
	fmt.Fprintf(b, "// by GetResult, preserving its dynamic type for type assertions.\n")
	fmt.Fprintf(b, "func (f *ResultFactory[T]) Fail(value T, err error) *OperationResult[T] {\n")
	fmt.Fprintf(b, "\treturn &OperationResult[T]{\n")
	fmt.Fprintf(b, "\t\tDidSucceed:          false,\n")
	fmt.Fprintf(b, "\t\tDidFailUnexpectedly: false,\n")
	fmt.Fprintf(b, "\t\tReturnValue:         value,\n")
	fmt.Fprintf(b, "\t\tErr:                 err,\n")
	fmt.Fprintf(b, "\t}\n")
	fmt.Fprintf(b, "}\n\n")
}

// ─────────────────────────────────────────────────────────────────────────────
// Code generation — Operation
// ─────────────────────────────────────────────────────────────────────────────

func emitOperation(b *strings.Builder, groups []capabilityGroup) {
	fmt.Fprintf(b, "// ─────────────────────────────────────────────────────────────────────────────\n")
	fmt.Fprintf(b, "// Operation — the unit of work\n")
	fmt.Fprintf(b, "// ─────────────────────────────────────────────────────────────────────────────\n")
	fmt.Fprintf(b, "\n")
	fmt.Fprintf(b, "type Operation[T any] struct {\n")
	fmt.Fprintf(b, "\tname        string\n")
	fmt.Fprintf(b, "\tcallback    func(*Operation[T], *ResultFactory[T]) *OperationResult[T]\n")
	fmt.Fprintf(b, "\tfallback    T\n")
	fmt.Fprintf(b, "\tpropertyBag map[string]any\n")
	fmt.Fprintf(b, "\trePanic     bool // if true, panics from the callback are re-panicked rather than caught\n")
	// Emit one capability field per declared category (e.g., File *_FileCapabilities).
	for _, cat := range uniqueCategories(groups) {
		fieldName := categoryFieldName(cat)
		typeName := categoryTypeName(cat)
		fmt.Fprintf(b, "\t%-12s *%s\n", fieldName, typeName)
	}
	fmt.Fprintf(b, "}\n")
	fmt.Fprintf(b, "\n")
	fmt.Fprintf(b, "// AddProperty attaches a named piece of metadata to this operation.\n")
	fmt.Fprintf(b, "// Properties appear in the structured log emitted by GetResult.\n")
	fmt.Fprintf(b, "// Record contextual information useful for debugging: node IDs, paths, etc.\n")
	fmt.Fprintf(b, "func (op *Operation[T]) AddProperty(name string, value any) {\n")
	fmt.Fprintf(b, "\top.propertyBag[name] = value\n")
	fmt.Fprintf(b, "}\n")
	fmt.Fprintf(b, "\n")
	fmt.Fprintf(b, "// PanicOnUnexpected configures this operation to re-panic when the callback\n")
	fmt.Fprintf(b, "// panics, rather than catching the panic and returning the fallback value.\n")
	fmt.Fprintf(b, "// Use this for operations that signal programming errors through panics\n")
	fmt.Fprintf(b, "// (e.g., adding a self-loop to a graph that prohibits them).\n")
	fmt.Fprintf(b, "func (op *Operation[T]) PanicOnUnexpected() *Operation[T] {\n")
	fmt.Fprintf(b, "\top.rePanic = true\n")
	fmt.Fprintf(b, "\treturn op\n")
	fmt.Fprintf(b, "}\n")
	fmt.Fprintf(b, "\n")
	fmt.Fprintf(b, "// GetResult executes the operation callback and returns (value, error).\n")
	fmt.Fprintf(b, "//\n")
	fmt.Fprintf(b, "// Execution model:\n")
	fmt.Fprintf(b, "//  1. Call the callback with this Operation and a ResultFactory.\n")
	fmt.Fprintf(b, "//  2. Recover any panic from the callback.\n")
	fmt.Fprintf(b, "//     - If PanicOnUnexpected() was set: re-panic.\n")
	fmt.Fprintf(b, "//     - Otherwise: use the fallback value, mark as unexpected failure.\n")
	fmt.Fprintf(b, "//  3. Return (ReturnValue, nil) on success; (ReturnValue, error) on failure.\n")
	fmt.Fprintf(b, "//     Expected failures with a typed Err field return that error directly\n")
	fmt.Fprintf(b, "//     (preserving the type for errors.As checks).\n")
	fmt.Fprintf(b, "func (op *Operation[T]) GetResult() (T, error) {\n")
	fmt.Fprintf(b, "\trf := &ResultFactory[T]{}\n")
	fmt.Fprintf(b, "\n")
	fmt.Fprintf(b, "\tvar result *OperationResult[T]\n")
	fmt.Fprintf(b, "\tencounteredPanic := false\n")
	fmt.Fprintf(b, "\tvar panicValue any\n")
	fmt.Fprintf(b, "\n")
	fmt.Fprintf(b, "\tfunc() {\n")
	fmt.Fprintf(b, "\t\tdefer func() {\n")
	fmt.Fprintf(b, "\t\t\tif r := recover(); r != nil {\n")
	fmt.Fprintf(b, "\t\t\t\tencounteredPanic = true\n")
	fmt.Fprintf(b, "\t\t\t\tpanicValue = r\n")
	fmt.Fprintf(b, "\t\t\t}\n")
	fmt.Fprintf(b, "\t\t}()\n")
	fmt.Fprintf(b, "\t\tresult = op.callback(op, rf)\n")
	fmt.Fprintf(b, "\t}()\n")
	fmt.Fprintf(b, "\n")
	fmt.Fprintf(b, "\tif encounteredPanic {\n")
	fmt.Fprintf(b, "\t\tresult = &OperationResult[T]{\n")
	fmt.Fprintf(b, "\t\t\tDidSucceed:          false,\n")
	fmt.Fprintf(b, "\t\t\tDidFailUnexpectedly: true,\n")
	fmt.Fprintf(b, "\t\t\tReturnValue:         op.fallback,\n")
	fmt.Fprintf(b, "\t\t}\n")
	fmt.Fprintf(b, "\t}\n")
	fmt.Fprintf(b, "\n")
	fmt.Fprintf(b, "\tif !result.DidSucceed {\n")
	fmt.Fprintf(b, "\t\tif result.DidFailUnexpectedly || encounteredPanic {\n")
	fmt.Fprintf(b, "\t\t\tif op.rePanic && encounteredPanic {\n")
	fmt.Fprintf(b, "\t\t\t\tpanic(panicValue)\n")
	fmt.Fprintf(b, "\t\t\t}\n")
	// Do not include panicValue in the caller-facing error: it may contain
	// sensitive data (credentials, paths, internal state). The structured log
	// line above already records panic:true and the operation name for debugging.
	fmt.Fprintf(b, "\t\t\treturn result.ReturnValue, fmt.Errorf(\"operation %%q failed unexpectedly (see log for details)\", op.name)\n")
	fmt.Fprintf(b, "\t\t}\n")
	fmt.Fprintf(b, "\t\tif result.Err != nil {\n")
	fmt.Fprintf(b, "\t\t\treturn result.ReturnValue, result.Err\n")
	fmt.Fprintf(b, "\t\t}\n")
	fmt.Fprintf(b, "\t\treturn result.ReturnValue, fmt.Errorf(\"operation %%q failed\", op.name)\n")
	fmt.Fprintf(b, "\t}\n")
	fmt.Fprintf(b, "\treturn result.ReturnValue, nil\n")
	fmt.Fprintf(b, "}\n\n")
}

// ─────────────────────────────────────────────────────────────────────────────
// Code generation — StartNew
// ─────────────────────────────────────────────────────────────────────────────

func emitStartNew(b *strings.Builder, groups []capabilityGroup) {
	fmt.Fprintf(b, "// ─────────────────────────────────────────────────────────────────────────────\n")
	fmt.Fprintf(b, "// StartNew — creates an Operation (does not run it yet)\n")
	fmt.Fprintf(b, "//\n")
	fmt.Fprintf(b, "// Call GetResult() on the returned Operation to execute the callback.\n")
	fmt.Fprintf(b, "// Chain PanicOnUnexpected() before GetResult() to let panics propagate\n")
	fmt.Fprintf(b, "// instead of being caught and converted to errors.\n")
	fmt.Fprintf(b, "//\n")
	fmt.Fprintf(b, "// Parameters:\n")
	fmt.Fprintf(b, "//   name     — Human-readable name for logging. Use \"package.Function\" format.\n")
	fmt.Fprintf(b, "//   fallback — Returned if the callback panics (and PanicOnUnexpected is false).\n")
	fmt.Fprintf(b, "//              Use the zero value for T: nil, 0, \"\", false, etc.\n")
	fmt.Fprintf(b, "//   fn       — The callback. Receives:\n")
	fmt.Fprintf(b, "//                op — the operation (op.File, op.Net, etc. for OS access;\n")
	fmt.Fprintf(b, "//                     op.AddProperty for metadata)\n")
	fmt.Fprintf(b, "//                rf — the result factory (rf.Generate or rf.Fail)\n")
	fmt.Fprintf(b, "// ─────────────────────────────────────────────────────────────────────────────\n")
	fmt.Fprintf(b, "\n")
	fmt.Fprintf(b, "func StartNew[T any](\n")
	fmt.Fprintf(b, "\tname string,\n")
	fmt.Fprintf(b, "\tfallback T,\n")
	fmt.Fprintf(b, "\tfn func(op *Operation[T], rf *ResultFactory[T]) *OperationResult[T],\n")
	fmt.Fprintf(b, ") *Operation[T] {\n")
	fmt.Fprintf(b, "\treturn &Operation[T]{\n")
	fmt.Fprintf(b, "\t\tname:        name,\n")
	fmt.Fprintf(b, "\t\tcallback:    fn,\n")
	fmt.Fprintf(b, "\t\tfallback:    fallback,\n")
	fmt.Fprintf(b, "\t\tpropertyBag: make(map[string]any),\n")
	// Initialize capability fields so they are non-nil inside the callback.
	for _, cat := range uniqueCategories(groups) {
		fieldName := categoryFieldName(cat)
		typeName := categoryTypeName(cat)
		fmt.Fprintf(b, "\t\t%-12s &%s{},\n", fieldName+":", typeName)
	}
	fmt.Fprintf(b, "\t}\n")
	fmt.Fprintf(b, "}\n\n")
}

// ─────────────────────────────────────────────────────────────────────────────
// Code generation — _capabilityViolationError
// ─────────────────────────────────────────────────────────────────────────────

func emitCapabilityViolationError(b *strings.Builder) {
	fmt.Fprintf(b, "// ─────────────────────────────────────────────────────────────────────────────\n")
	fmt.Fprintf(b, "// _capabilityViolationError — returned when an undeclared OS op is attempted\n")
	fmt.Fprintf(b, "// ─────────────────────────────────────────────────────────────────────────────\n")
	fmt.Fprintf(b, "\n")
	fmt.Fprintf(b, "type _capabilityViolationError struct {\n")
	fmt.Fprintf(b, "\tcategory  string\n")
	fmt.Fprintf(b, "\taction    string\n")
	fmt.Fprintf(b, "\trequested string\n")
	fmt.Fprintf(b, "}\n")
	fmt.Fprintf(b, "\n")
	fmt.Fprintf(b, "func (e *_capabilityViolationError) Error() string {\n")
	fmt.Fprintf(b, "\treturn fmt.Sprintf(\n")
	fmt.Fprintf(b, "\t\t\"capability violation: %%s:%%s — %%q is not declared in required_capabilities.json.\\n\"+\n")
	fmt.Fprintf(b, "\t\t\t\"To add: edit required_capabilities.json and re-run capability-cage-generator.\",\n")
	fmt.Fprintf(b, "\t\te.category, e.action, e.requested,\n")
	fmt.Fprintf(b, "\t)\n")
	fmt.Fprintf(b, "}\n")
}

// ─────────────────────────────────────────────────────────────────────────────
// Code generation — top-level entry point
// ─────────────────────────────────────────────────────────────────────────────

// generateSource produces the content of gen_capabilities.go for a given manifest.
func generateSource(manifestPath string, mf *manifestJSON) (string, error) {
	pkgName, err := goPackageName(manifestPath, mf.Package)
	if err != nil {
		return "", err
	}
	// Validate the package name is a legal Go identifier before embedding it
	// in generated source. A package name must be all lowercase letters and
	// digits, starting with a letter. This prevents injection if a malicious
	// or malformed .go file or manifest supplies a crafted package name.
	if !regexp.MustCompile(`^[a-z][a-z0-9]*$`).MatchString(pkgName) {
		return "", fmt.Errorf("derived Go package name %q is not a valid identifier (must match ^[a-z][a-z0-9]*$)", pkgName)
	}

	groups := groupCapabilities(mf.Capabilities)
	imports := neededImports(mf.Capabilities)

	var b strings.Builder

	emitHeader(&b, manifestPath)
	fmt.Fprintf(&b, "package %s\n\n", pkgName)
	emitImports(&b, imports)

	// Emit package-level sync.OnceValue vars for relative path targets,
	// then pass the target→varName map into the struct emitter so that
	// each capability method can reference the correct pre-resolved path.
	relTargets := collectRelativeTargets(groups)
	targetToVar := emitResolvedPathVars(&b, relTargets)

	emitCapabilityStructs(&b, groups, targetToVar)
	emitOperationResult(&b)
	emitResultFactory(&b)
	emitOperation(&b, groups)
	emitStartNew(&b, groups)
	emitCapabilityViolationError(&b)

	return b.String(), nil
}

// ─────────────────────────────────────────────────────────────────────────────
// Single manifest processing
// ─────────────────────────────────────────────────────────────────────────────

// processManifest reads a single required_capabilities.json file, generates
// the corresponding gen_capabilities.go, and writes it to the same directory.
//
// If dryRun is true, the generated source is printed to stdout instead of
// written to disk.
func processManifest(manifestPath string, dryRun bool) error {
	data, err := os.ReadFile(manifestPath) //nolint:cap
	if err != nil {
		return fmt.Errorf("read %s: %w", manifestPath, err)
	}

	var mf manifestJSON
	if err := json.Unmarshal(data, &mf); err != nil {
		return fmt.Errorf("parse %s: %w", manifestPath, err)
	}

	// Only generate for Go packages.
	if !strings.HasPrefix(mf.Package, "go/") {
		fmt.Fprintf(os.Stderr, "skip %s: package %q is not a Go package (prefix not 'go/')\n",
			manifestPath, mf.Package)
		return nil
	}

	source, err := generateSource(manifestPath, &mf)
	if err != nil {
		return fmt.Errorf("generate %s: %w", manifestPath, err)
	}

	outPath := filepath.Join(filepath.Dir(manifestPath), "gen_capabilities.go")

	if dryRun {
		fmt.Printf("=== DRY RUN: would write %s ===\n", outPath)
		fmt.Println(source)
		return nil
	}

	if err := os.WriteFile(outPath, []byte(source), 0o644); err != nil { //nolint:cap
		return fmt.Errorf("write %s: %w", outPath, err)
	}
	fmt.Printf("wrote %s\n", outPath)
	return nil
}

// ─────────────────────────────────────────────────────────────────────────────
// --all mode: sweep all Go packages
// ─────────────────────────────────────────────────────────────────────────────

// processAll walks the repository's code/packages/go/ directory and processes
// every required_capabilities.json found there.
func processAll(repoRoot string, dryRun bool) error {
	pattern := filepath.Join(repoRoot, "code", "packages", "go", "*", "required_capabilities.json")
	matches, err := filepath.Glob(pattern)
	if err != nil {
		return fmt.Errorf("glob %s: %w", pattern, err)
	}
	if len(matches) == 0 {
		fmt.Println("no required_capabilities.json files found under code/packages/go/")
		return nil
	}

	var errs []string
	for _, m := range matches {
		if err := processManifest(m, dryRun); err != nil {
			errs = append(errs, err.Error())
			fmt.Fprintf(os.Stderr, "error: %v\n", err)
		}
	}
	if len(errs) > 0 {
		return fmt.Errorf("%d error(s) encountered", len(errs))
	}
	return nil
}

// ─────────────────────────────────────────────────────────────────────────────
// Repository root detection
// ─────────────────────────────────────────────────────────────────────────────

// findRepoRoot walks up the directory tree from the current working directory
// looking for a directory that contains code/packages/. Falls back to the
// current working directory if not found.
func findRepoRoot() string {
	cwd, _ := os.Getwd() //nolint:cap
	if isRepoRoot(cwd) {
		return cwd
	}
	dir := cwd
	for {
		parent := filepath.Dir(dir)
		if parent == dir {
			break
		}
		dir = parent
		if isRepoRoot(dir) {
			return dir
		}
	}
	return cwd
}

func isRepoRoot(dir string) bool {
	info, err := os.Stat(filepath.Join(dir, "code", "packages")) //nolint:cap
	return err == nil && info.IsDir()
}

// ─────────────────────────────────────────────────────────────────────────────
// main
// ─────────────────────────────────────────────────────────────────────────────

func main() {
	var (
		manifestFlag = flag.String("manifest", "", "Path to a single required_capabilities.json file")
		allFlag      = flag.Bool("all", false, "Process all Go packages in the repository")
		dryRunFlag   = flag.Bool("dry-run", false, "Print generated files to stdout without writing")
	)
	flag.Parse()

	if *manifestFlag == "" && !*allFlag {
		fmt.Fprintln(os.Stderr, "usage: capability-cage-generator [--manifest=<path>] [--all] [--dry-run]")
		fmt.Fprintln(os.Stderr, "  --manifest  path to a single required_capabilities.json")
		fmt.Fprintln(os.Stderr, "  --all       process all Go packages in the repository")
		fmt.Fprintln(os.Stderr, "  --dry-run   print output without writing files")
		os.Exit(1)
	}

	if *manifestFlag != "" && *allFlag {
		fmt.Fprintln(os.Stderr, "error: --manifest and --all are mutually exclusive")
		os.Exit(1)
	}

	if *allFlag {
		repoRoot := findRepoRoot()
		if err := processAll(repoRoot, *dryRunFlag); err != nil {
			fmt.Fprintf(os.Stderr, "error: %v\n", err)
			os.Exit(1)
		}
		return
	}

	if err := processManifest(*manifestFlag, *dryRunFlag); err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(1)
	}
}
