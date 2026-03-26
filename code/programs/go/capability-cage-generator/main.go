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
//   - Cage: the OS-capability gate (methods only for declared capabilities)
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
// Wildcard validation
// ─────────────────────────────────────────────────────────────────────────────

// scopeableCategory returns true for categories that must declare exact targets.
// Non-scopeable categories (time, stdin, stdout) accept "*" because they are
// not path-based — there is nothing to narrow them to.
func scopeableCategory(cat string) bool {
	switch cat {
	case "fs", "net", "proc", "env", "ffi":
		return true
	}
	return false
}

// validateNoWildcards returns an error if any scopeable category uses "*" as
// its target. Every fs/net/proc/env/ffi capability must name an exact path
// or address so the cage can enforce it at runtime.
func validateNoWildcards(mf *manifestJSON) error {
	for _, cap := range mf.Capabilities {
		if scopeableCategory(cap.Category) && cap.Target == "*" {
			return fmt.Errorf(
				"capability %q:%q in package %q declares a wildcard target %q — "+
					"use an exact path instead (e.g., \"code/grammars/vhdl.tokens\")",
				cap.Category, cap.Action, mf.Package, cap.Target,
			)
		}
	}
	return nil
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
// Import computation
// ─────────────────────────────────────────────────────────────────────────────

// neededImports returns the sorted stdlib import paths required by the
// generated code. Always includes fmt, log, time. Additional packages
// depend on which capability categories are declared.
func neededImports(caps []capabilityJSON) []string {
	set := map[string]bool{
		`"fmt"`:  true,
		`"log"`:  true,
		`"time"`: true,
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
		}
		if c.Category == "proc" && c.Action == "exec" {
			set[`"os/exec"`] = true
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
	fmt.Fprintf(b, "//     --manifest=%s\n", manifestPath)
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
// Code generation — Cage
// ─────────────────────────────────────────────────────────────────────────────

func emitCage(b *strings.Builder, groups []capabilityGroup) {
	fmt.Fprintf(b, "// ─────────────────────────────────────────────────────────────────────────────\n")
	fmt.Fprintf(b, "// Cage — the OS-capability gate\n")
	fmt.Fprintf(b, "//\n")
	fmt.Fprintf(b, "// Cage is injected into every Operation callback. It is the ONLY way to\n")
	fmt.Fprintf(b, "// perform OS-level operations. Methods exist only for capabilities that are\n")
	fmt.Fprintf(b, "// declared in required_capabilities.json. Any undeclared OS operation must\n")
	fmt.Fprintf(b, "// be added to the manifest — making it visible in code review and\n")
	fmt.Fprintf(b, "// enforced at compile time (the method simply does not exist).\n")
	fmt.Fprintf(b, "// ─────────────────────────────────────────────────────────────────────────────\n")
	fmt.Fprintf(b, "\n")
	fmt.Fprintf(b, "type Cage struct{}\n")
	fmt.Fprintf(b, "\n")
	for _, g := range groups {
		emitCageMethod(b, g)
	}
}

// emitScopedCheck emits the allowed-path check at the top of a Cage method.
// paramName is the method parameter to check (e.g., "path", "addr", "key").
// twoReturns controls whether the violation returns (nil, err) or just (err).
func emitScopedCheck(b *strings.Builder, targets []string, paramName string, twoReturns bool, cat, action string) {
	if len(targets) == 1 {
		fmt.Fprintf(b, "\tif %s != %q {\n", paramName, targets[0])
	} else {
		fmt.Fprintf(b, "\tallowed := map[string]bool{\n")
		for _, t := range targets {
			fmt.Fprintf(b, "\t\t%q: true,\n", t)
		}
		fmt.Fprintf(b, "\t}\n")
		fmt.Fprintf(b, "\tif !allowed[%s] {\n", paramName)
	}
	if twoReturns {
		fmt.Fprintf(b, "\t\treturn nil, &_capabilityViolationError{category: %q, action: %q, requested: %s}\n", cat, action, paramName)
	} else {
		fmt.Fprintf(b, "\t\treturn &_capabilityViolationError{category: %q, action: %q, requested: %s}\n", cat, action, paramName)
	}
	fmt.Fprintf(b, "\t}\n")
}

func emitCageMethod(b *strings.Builder, g capabilityGroup) {
	key := g.Category + ":" + g.Action
	switch key {
	case "fs:read":
		fmt.Fprintf(b, "// ReadFile reads the file at path.\n")
		fmt.Fprintf(b, "// Only paths declared in required_capabilities.json are permitted.\n")
		fmt.Fprintf(b, "func (c *Cage) ReadFile(path string) ([]byte, error) {\n")
		emitScopedCheck(b, g.Targets, "path", true, "fs", "read")
		fmt.Fprintf(b, "\treturn os.ReadFile(path) //nolint:cap\n")
		fmt.Fprintf(b, "}\n\n")

	case "fs:write":
		fmt.Fprintf(b, "// WriteFile writes data to the file at path.\n")
		fmt.Fprintf(b, "// Only paths declared in required_capabilities.json are permitted.\n")
		fmt.Fprintf(b, "func (c *Cage) WriteFile(path string, data []byte, perm os.FileMode) error {\n")
		emitScopedCheck(b, g.Targets, "path", false, "fs", "write")
		fmt.Fprintf(b, "\treturn os.WriteFile(path, data, perm) //nolint:cap\n")
		fmt.Fprintf(b, "}\n\n")

	case "fs:create":
		fmt.Fprintf(b, "// CreateFile creates or truncates the file at path.\n")
		fmt.Fprintf(b, "// Only paths declared in required_capabilities.json are permitted.\n")
		fmt.Fprintf(b, "func (c *Cage) CreateFile(path string) (*os.File, error) {\n")
		emitScopedCheck(b, g.Targets, "path", true, "fs", "create")
		fmt.Fprintf(b, "\treturn os.Create(path) //nolint:cap\n")
		fmt.Fprintf(b, "}\n\n")

	case "fs:delete":
		fmt.Fprintf(b, "// DeleteFile removes the file at path.\n")
		fmt.Fprintf(b, "// Only paths declared in required_capabilities.json are permitted.\n")
		fmt.Fprintf(b, "func (c *Cage) DeleteFile(path string) error {\n")
		emitScopedCheck(b, g.Targets, "path", false, "fs", "delete")
		fmt.Fprintf(b, "\treturn os.Remove(path) //nolint:cap\n")
		fmt.Fprintf(b, "}\n\n")

	case "fs:list":
		fmt.Fprintf(b, "// ReadDir lists the contents of the directory at path.\n")
		fmt.Fprintf(b, "// Only paths declared in required_capabilities.json are permitted.\n")
		fmt.Fprintf(b, "func (c *Cage) ReadDir(path string) ([]os.DirEntry, error) {\n")
		emitScopedCheck(b, g.Targets, "path", true, "fs", "list")
		fmt.Fprintf(b, "\treturn os.ReadDir(path) //nolint:cap\n")
		fmt.Fprintf(b, "}\n\n")

	case "net:connect":
		fmt.Fprintf(b, "// Connect opens a network connection to addr.\n")
		fmt.Fprintf(b, "// Only addresses declared in required_capabilities.json are permitted.\n")
		fmt.Fprintf(b, "func (c *Cage) Connect(network, addr string) (net.Conn, error) {\n")
		emitScopedCheck(b, g.Targets, "addr", true, "net", "connect")
		fmt.Fprintf(b, "\treturn net.Dial(network, addr) //nolint:cap\n")
		fmt.Fprintf(b, "}\n\n")

	case "net:listen":
		fmt.Fprintf(b, "// Listen opens a network listener on addr.\n")
		fmt.Fprintf(b, "// Only addresses declared in required_capabilities.json are permitted.\n")
		fmt.Fprintf(b, "func (c *Cage) Listen(network, addr string) (net.Listener, error) {\n")
		emitScopedCheck(b, g.Targets, "addr", true, "net", "listen")
		fmt.Fprintf(b, "\treturn net.Listen(network, addr) //nolint:cap\n")
		fmt.Fprintf(b, "}\n\n")

	case "net:dns":
		fmt.Fprintf(b, "// LookupHost resolves host to a list of IP addresses.\n")
		fmt.Fprintf(b, "// Only hosts declared in required_capabilities.json are permitted.\n")
		fmt.Fprintf(b, "func (c *Cage) LookupHost(host string) ([]string, error) {\n")
		emitScopedCheck(b, g.Targets, "host", true, "net", "dns")
		fmt.Fprintf(b, "\treturn net.LookupHost(host) //nolint:cap\n")
		fmt.Fprintf(b, "}\n\n")

	case "proc:exec":
		fmt.Fprintf(b, "// Exec runs the named executable with args.\n")
		fmt.Fprintf(b, "// Only executables declared in required_capabilities.json are permitted.\n")
		fmt.Fprintf(b, "func (c *Cage) Exec(name string, args ...string) ([]byte, error) {\n")
		emitScopedCheck(b, g.Targets, "name", true, "proc", "exec")
		fmt.Fprintf(b, "\treturn exec.Command(name, args...).Output() //nolint:cap\n")
		fmt.Fprintf(b, "}\n\n")

	case "env:read":
		fmt.Fprintf(b, "// Getenv returns the value of the environment variable key.\n")
		fmt.Fprintf(b, "// Only variables declared in required_capabilities.json are permitted.\n")
		fmt.Fprintf(b, "func (c *Cage) Getenv(key string) (string, error) {\n")
		emitScopedCheck(b, g.Targets, "key", true, "env", "read")
		fmt.Fprintf(b, "\treturn os.Getenv(key), nil //nolint:cap\n")
		fmt.Fprintf(b, "}\n\n")

	case "time:sleep":
		fmt.Fprintf(b, "// Sleep pauses the current goroutine for duration d.\n")
		fmt.Fprintf(b, "func (c *Cage) Sleep(d time.Duration) {\n")
		fmt.Fprintf(b, "\ttime.Sleep(d)\n")
		fmt.Fprintf(b, "}\n\n")

	case "stdin:read":
		fmt.Fprintf(b, "// ReadStdin reads all bytes from standard input.\n")
		fmt.Fprintf(b, "func (c *Cage) ReadStdin() ([]byte, error) {\n")
		fmt.Fprintf(b, "\treturn io.ReadAll(os.Stdin) //nolint:cap\n")
		fmt.Fprintf(b, "}\n\n")

	case "stdout:write":
		fmt.Fprintf(b, "// WriteStdout writes data to standard output.\n")
		fmt.Fprintf(b, "func (c *Cage) WriteStdout(data []byte) (int, error) {\n")
		fmt.Fprintf(b, "\treturn os.Stdout.Write(data) //nolint:cap\n")
		fmt.Fprintf(b, "}\n\n")

	default:
		fmt.Fprintf(b, "// TODO: implement Cage method for %s:%s\n\n", g.Category, g.Action)
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

func emitOperation(b *strings.Builder) {
	fmt.Fprintf(b, "// ─────────────────────────────────────────────────────────────────────────────\n")
	fmt.Fprintf(b, "// Operation — the unit of work\n")
	fmt.Fprintf(b, "// ─────────────────────────────────────────────────────────────────────────────\n")
	fmt.Fprintf(b, "\n")
	fmt.Fprintf(b, "type Operation[T any] struct {\n")
	fmt.Fprintf(b, "\tname        string\n")
	fmt.Fprintf(b, "\tcallback    func(*Cage, *Operation[T], *ResultFactory[T]) *OperationResult[T]\n")
	fmt.Fprintf(b, "\tfallback    T\n")
	fmt.Fprintf(b, "\tpropertyBag map[string]any\n")
	fmt.Fprintf(b, "\trePanic     bool // if true, panics from the callback are re-panicked rather than caught\n")
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
	fmt.Fprintf(b, "//  1. Record start time.\n")
	fmt.Fprintf(b, "//  2. Call the callback with a fresh Cage, this Operation, and a ResultFactory.\n")
	fmt.Fprintf(b, "//  3. Recover any panic from the callback.\n")
	fmt.Fprintf(b, "//     - If PanicOnUnexpected() was set: re-panic after logging.\n")
	fmt.Fprintf(b, "//     - Otherwise: use the fallback value, mark as unexpected failure.\n")
	fmt.Fprintf(b, "//  4. Record elapsed time and emit a structured log line.\n")
	fmt.Fprintf(b, "//  5. Return (ReturnValue, nil) on success; (ReturnValue, error) on failure.\n")
	fmt.Fprintf(b, "//     Expected failures with a typed Err field return that error directly\n")
	fmt.Fprintf(b, "//     (preserving the type for errors.As checks).\n")
	fmt.Fprintf(b, "func (op *Operation[T]) GetResult() (T, error) {\n")
	fmt.Fprintf(b, "\tstart := time.Now()\n")
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
	fmt.Fprintf(b, "\t\tresult = op.callback(&Cage{}, op, rf)\n")
	fmt.Fprintf(b, "\t}()\n")
	fmt.Fprintf(b, "\n")
	fmt.Fprintf(b, "\telapsed := time.Since(start)\n")
	fmt.Fprintf(b, "\n")
	fmt.Fprintf(b, "\tif encounteredPanic {\n")
	fmt.Fprintf(b, "\t\tresult = &OperationResult[T]{\n")
	fmt.Fprintf(b, "\t\t\tDidSucceed:          false,\n")
	fmt.Fprintf(b, "\t\t\tDidFailUnexpectedly: true,\n")
	fmt.Fprintf(b, "\t\t\tReturnValue:         op.fallback,\n")
	fmt.Fprintf(b, "\t\t}\n")
	fmt.Fprintf(b, "\t}\n")
	fmt.Fprintf(b, "\n")
	fmt.Fprintf(b, "\tlog.Printf(`{\"op\":%%q,\"elapsedMs\":%%d,\"ok\":%%v,\"unexpected\":%%v,\"panic\":%%v,\"props\":%%v}`,\n")
	fmt.Fprintf(b, "\t\top.name, elapsed.Milliseconds(),\n")
	fmt.Fprintf(b, "\t\tresult.DidSucceed, result.DidFailUnexpectedly,\n")
	fmt.Fprintf(b, "\t\tencounteredPanic, op.propertyBag)\n")
	fmt.Fprintf(b, "\n")
	fmt.Fprintf(b, "\tif !result.DidSucceed {\n")
	fmt.Fprintf(b, "\t\tif result.DidFailUnexpectedly || encounteredPanic {\n")
	fmt.Fprintf(b, "\t\t\tif op.rePanic && encounteredPanic {\n")
	fmt.Fprintf(b, "\t\t\t\tpanic(panicValue)\n")
	fmt.Fprintf(b, "\t\t\t}\n")
	fmt.Fprintf(b, "\t\t\treturn result.ReturnValue, fmt.Errorf(\"operation %%q failed unexpectedly: %%v\", op.name, panicValue)\n")
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

func emitStartNew(b *strings.Builder) {
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
	fmt.Fprintf(b, "//                cage  — the capability gate (only declared OS methods)\n")
	fmt.Fprintf(b, "//                op    — the operation (call AddProperty for metadata)\n")
	fmt.Fprintf(b, "//                rf    — the result factory (rf.Generate or rf.Fail)\n")
	fmt.Fprintf(b, "// ─────────────────────────────────────────────────────────────────────────────\n")
	fmt.Fprintf(b, "\n")
	fmt.Fprintf(b, "func StartNew[T any](\n")
	fmt.Fprintf(b, "\tname string,\n")
	fmt.Fprintf(b, "\tfallback T,\n")
	fmt.Fprintf(b, "\tfn func(cage *Cage, op *Operation[T], rf *ResultFactory[T]) *OperationResult[T],\n")
	fmt.Fprintf(b, ") *Operation[T] {\n")
	fmt.Fprintf(b, "\treturn &Operation[T]{\n")
	fmt.Fprintf(b, "\t\tname:        name,\n")
	fmt.Fprintf(b, "\t\tcallback:    fn,\n")
	fmt.Fprintf(b, "\t\tfallback:    fallback,\n")
	fmt.Fprintf(b, "\t\tpropertyBag: make(map[string]any),\n")
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
// Returns an error if any scopeable capability declares a wildcard target.
func generateSource(manifestPath string, mf *manifestJSON) (string, error) {
	if err := validateNoWildcards(mf); err != nil {
		return "", err
	}

	pkgName, err := goPackageName(manifestPath, mf.Package)
	if err != nil {
		return "", err
	}

	groups := groupCapabilities(mf.Capabilities)
	imports := neededImports(mf.Capabilities)

	var b strings.Builder

	emitHeader(&b, manifestPath)
	fmt.Fprintf(&b, "package %s\n\n", pkgName)
	emitImports(&b, imports)
	emitCage(&b, groups)
	emitOperationResult(&b)
	emitResultFactory(&b)
	emitOperation(&b)
	emitStartNew(&b)
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
