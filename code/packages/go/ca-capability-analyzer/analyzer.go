// Package analyzer provides static analysis of Go source code to detect
// OS-level capability usage — filesystem access, network calls, process
// execution, environment variable access, and foreign function interfaces.
//
// # Why static capability analysis?
//
// In a capability-based security system, every package must declare which
// OS resources it needs. The analyzer's job is to automatically detect
// what a package *actually* uses by examining its source code, so we can
// compare that against what it *declared*.
//
// This is the Go implementation. The Python version uses Python's `ast`
// module; this one uses Go's `go/ast`, `go/parser`, and `go/token`
// packages from the standard library.
//
// # How Go AST walking works
//
// Go's `go/parser` package parses .go source files into an abstract syntax
// tree (AST). The tree is made of typed nodes — `ast.ImportSpec` for imports,
// `ast.CallExpr` for function calls, `ast.SelectorExpr` for `pkg.Func`
// expressions, and so on.
//
// We use `ast.Inspect()` to walk every node in the tree. For each node,
// we check if it matches a pattern that indicates capability usage:
//
//   - An `ast.ImportSpec` with path "os" indicates filesystem access
//   - An `ast.CallExpr` like `os.Open("file.txt")` indicates fs:read:file.txt
//   - An `ast.CallExpr` like `exec.Command("ls")` indicates proc:exec:ls
//
// # Detection categories
//
// | Category | What it covers                    | Example imports          |
// |----------|----------------------------------|--------------------------|
// | fs       | Filesystem read/write/delete     | os, io, io/ioutil        |
// | net      | Network connections, listeners   | net, net/http            |
// | proc     | Process execution, signals       | os/exec, syscall         |
// | env      | Environment variable access      | os.Getenv, os.Setenv     |
// | ffi      | Foreign function interface       | unsafe, plugin, C, reflect |
package analyzer

import (
	"go/ast"
	"go/parser"
	"go/token"
	"os"
	"path/filepath"
	"strings"
)

// DetectedCapability represents a single OS capability detected in source code.
//
// The format follows the capability triple: category:action:target
//
//   - Category: The kind of resource (fs, net, proc, env, ffi)
//   - Action:   The operation (read, write, connect, exec, etc.)
//   - Target:   The specific resource ("file.txt", "KEY", "*" for unknown)
//   - File:     The source file where detection occurred
//   - Line:     The line number in the source file
//   - Evidence: The code pattern that triggered detection (human-readable)
type DetectedCapability struct {
	Category string `json:"category"`
	Action   string `json:"action"`
	Target   string `json:"target"`
	File     string `json:"file"`
	Line     int    `json:"line"`
	Evidence string `json:"evidence"`
}

// String returns the capability triple in category:action:target format.
func (d DetectedCapability) String() string {
	return d.Category + ":" + d.Action + ":" + d.Target
}

// ── Import-to-capability mapping ──────────────────────────────────────
//
// When Go code imports a package, the import path tells us what kind of
// OS capability the code might use. This mapping is conservative: importing
// "os" doesn't mean the code uses the filesystem, but it *could*. We flag
// it and let the manifest comparison decide.
//
// Why these specific mappings?
//
//   - "os" → fs:*:* because the os package provides Open, Create, Remove, etc.
//   - "io" → fs:*:* because io.Reader/Writer are used for file I/O
//   - "io/ioutil" → fs:*:* because ioutil.ReadFile, WriteFile, etc.
//   - "net" → net:*:* because net.Dial, net.Listen, etc.
//   - "net/http" → net:connect:* because http.Get makes outbound connections
//   - "os/exec" → proc:exec:* because exec.Command runs external programs
//   - "syscall" → proc:*:* because syscall gives raw OS access
//   - "unsafe" → ffi:*:* because unsafe.Pointer breaks type safety
//   - "plugin" → ffi:load:* because plugin.Open loads shared libraries
//   - "C" → ffi:*:* because cgo calls arbitrary C functions
//   - "reflect" → ffi:*:* because reflection enables dynamic dispatch,
//     bypassing static analysis (reflect.Value.Call, MethodByName, etc.)

var importCapabilities = map[string][2]string{
	// Filesystem access — these packages provide file I/O operations
	"os":        {"fs", "*"},
	"io":        {"fs", "*"},
	"io/ioutil": {"fs", "*"},

	// Network access — these packages make or accept network connections
	"net":      {"net", "*"},
	"net/http": {"net", "connect"},

	// Process execution — these packages run external commands or make
	// raw system calls
	"os/exec": {"proc", "exec"},
	"syscall": {"proc", "*"},

	// Foreign function interface — these packages bypass Go's type system
	// or load native code
	"unsafe":  {"ffi", "*"},
	"plugin":  {"ffi", "load"},
	"C":       {"ffi", "*"},
	"reflect": {"ffi", "*"},
}

// ── Function-call-to-capability mapping ───────────────────────────────
//
// Beyond imports, specific function calls give us more precise information
// about what capabilities a package uses. For example:
//
//   - os.Open("x") → fs:read:x    (we know the exact file being read)
//   - os.Getenv("KEY") → env:read:KEY (we know the exact env var)
//   - exec.Command("ls") → proc:exec:ls (we know the exact command)
//
// Each entry maps a (package, function) pair to a (category, action) pair.
// The target is extracted from the first argument if it's a string literal,
// otherwise "*" (unknown).

type callRule struct {
	// pkg is the local name of the package (e.g., "os", "exec", "http")
	pkg string
	// funcName is the function or method name (e.g., "Open", "Command")
	funcName string
	// category is the capability category (fs, net, proc, env, ffi)
	category string
	// action is the capability action (read, write, exec, etc.)
	action string
	// extractTarget controls whether we try to extract the first string
	// argument as the target. If false, target is always "*".
	extractTarget bool
}

// callRules defines the function-call detection rules. Each rule maps a
// package.Function pattern to a capability.
//
// Why these specific rules?
//
//   - os.Open/os.ReadFile: These read files. The first argument is the path.
//   - os.Create/os.WriteFile: These create or write files.
//   - os.Remove: Deletes a file or directory.
//   - os.Mkdir/os.MkdirAll: Creates directories.
//   - os.ReadDir: Lists directory contents.
//   - os.Getenv: Reads an environment variable by name.
//   - os.Setenv: Sets an environment variable.
//   - exec.Command: Runs an external program. First arg is the command name.
//   - net.Dial: Opens a network connection. Target is protocol-dependent.
//   - http.Get: Makes an HTTP GET request. First arg is the URL.
//   - os.Exit: Terminates the process — a form of process signaling.
var callRules = []callRule{
	// Filesystem read operations
	{"os", "Open", "fs", "read", true},
	{"os", "ReadFile", "fs", "read", true},

	// Filesystem write operations
	{"os", "Create", "fs", "write", true},
	{"os", "WriteFile", "fs", "write", true},

	// Filesystem delete operations
	{"os", "Remove", "fs", "delete", true},
	{"os", "RemoveAll", "fs", "delete", true},

	// Filesystem create operations (directories)
	{"os", "Mkdir", "fs", "create", true},
	{"os", "MkdirAll", "fs", "create", true},

	// Filesystem list operations
	{"os", "ReadDir", "fs", "list", true},

	// Environment variable access
	{"os", "Getenv", "env", "read", true},
	{"os", "Setenv", "env", "write", true},

	// Process execution
	{"exec", "Command", "proc", "exec", true},

	// Network connections
	{"net", "Dial", "net", "connect", false},
	{"net", "Listen", "net", "listen", false},
	{"http", "Get", "net", "connect", false},
	{"http", "Post", "net", "connect", false},
	{"http", "Head", "net", "connect", false},

	// Process signals
	{"os", "Exit", "proc", "signal", false},
}

// AnalyzeSource parses a Go source string and detects capability usage.
//
// This is the main entry point for analyzing Go code. It:
// 1. Parses the source into an AST using go/parser
// 2. Walks every node using ast.Inspect
// 3. For each ImportSpec, checks the import-to-capability mapping
// 4. For each CallExpr, checks the function-call-to-capability mapping
// 5. Returns all detected capabilities
//
// The filename parameter is used for error reporting — it appears in the
// File field of each DetectedCapability.
func AnalyzeSource(filename string, source string) ([]DetectedCapability, error) {
	fset := token.NewFileSet()
	file, err := parser.ParseFile(fset, filename, source, parser.ParseComments)
	if err != nil {
		return nil, err
	}
	return analyzeAST(fset, file, filename), nil
}

// AnalyzeFile reads and analyzes a single Go source file.
func AnalyzeFile(filepath string) ([]DetectedCapability, error) {
	source, err := os.ReadFile(filepath)
	if err != nil {
		return nil, err
	}
	return AnalyzeSource(filepath, string(source))
}

// AnalyzeDirectory walks a directory tree and analyzes all .go files.
//
// It skips common non-source directories like vendor, .git, and testdata.
// If excludeTests is true, it also skips *_test.go files — test files often
// use capabilities (e.g., os.TempDir) that the package itself doesn't need.
func AnalyzeDirectory(dir string, excludeTests bool) ([]DetectedCapability, error) {
	var allDetected []DetectedCapability

	// Directories to skip when walking the tree
	skipDirs := map[string]bool{
		"vendor":      true,
		".git":        true,
		"node_modules": true,
		"testdata":    true,
	}

	err := filepath.Walk(dir, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}

		// Skip excluded directories
		if info.IsDir() {
			if skipDirs[info.Name()] {
				return filepath.SkipDir
			}
			return nil
		}

		// Only process .go files
		if !strings.HasSuffix(path, ".go") {
			return nil
		}

		// Optionally skip test files
		if excludeTests && strings.HasSuffix(path, "_test.go") {
			return nil
		}

		detected, err := AnalyzeFile(path)
		if err != nil {
			// Skip files that can't be parsed (e.g., generated code
			// with build constraints that don't match)
			return nil
		}

		allDetected = append(allDetected, detected...)
		return nil
	})

	if err != nil {
		return nil, err
	}

	return allDetected, nil
}

// analyzeAST walks a parsed Go AST and collects all detected capabilities.
//
// This function does the actual work. It uses ast.Inspect to visit every
// node in the tree. For each node type we care about, it checks against
// our detection rules.
//
// The two main node types we examine:
//
//   - ast.ImportSpec: An import declaration like `import "os"` or
//     `import "net/http"`. We check the import path against importCapabilities.
//
//   - ast.CallExpr: A function call like `os.Open("file.txt")`. We check
//     if the function is a selector expression (pkg.Func) that matches
//     one of our callRules.
func analyzeAST(fset *token.FileSet, file *ast.File, filename string) []DetectedCapability {
	var detected []DetectedCapability

	// Build a map of import alias → import path so we can resolve
	// calls like `exec.Command("ls")` where exec was imported as
	// `import "os/exec"`. The local name is "exec" but we need to
	// know the full import path to look up the capability.
	importAliases := buildImportAliases(file)

	ast.Inspect(file, func(n ast.Node) bool {
		switch node := n.(type) {

		case *ast.ImportSpec:
			// ── Import detection ──────────────────────────────
			//
			// Check if this import path maps to a known capability.
			// The import path is a string literal like `"os"` or
			// `"net/http"` — we strip the quotes to get the raw path.
			importPath := strings.Trim(node.Path.Value, `"`)
			if cap, ok := importCapabilities[importPath]; ok {
				line := fset.Position(node.Pos()).Line
				detected = append(detected, DetectedCapability{
					Category: cap[0],
					Action:   cap[1],
					Target:   "*",
					File:     filename,
					Line:     line,
					Evidence: `import "` + importPath + `"`,
				})
			}

		case *ast.CallExpr:
			// ── Function call detection ───────────────────────
			//
			// We look for calls of the form `pkg.Func(args...)`.
			// In the AST, this is a CallExpr whose Fun is a
			// SelectorExpr with X=Ident(pkg) and Sel=Ident(Func).
			sel, ok := node.Fun.(*ast.SelectorExpr)
			if !ok {
				return true
			}
			ident, ok := sel.X.(*ast.Ident)
			if !ok {
				return true
			}

			localPkg := ident.Name
			funcName := sel.Sel.Name

			// Check each call rule
			for _, rule := range callRules {
				if rule.funcName != funcName {
					continue
				}

				// The rule's pkg field matches the *local* name of the
				// imported package. For `import "os/exec"`, the local
				// name is "exec". For `import myos "os"`, it's "myos".
				// We need to check both the local name directly and
				// resolve through aliases.
				if !matchesPackage(localPkg, rule.pkg, importAliases) {
					continue
				}

				target := "*"
				if rule.extractTarget && len(node.Args) > 0 {
					target = extractStringLiteral(node.Args[0])
				}

				line := fset.Position(node.Pos()).Line
				detected = append(detected, DetectedCapability{
					Category: rule.category,
					Action:   rule.action,
					Target:   target,
					File:     filename,
					Line:     line,
					Evidence: localPkg + "." + funcName + "(...)",
				})
				break
			}
		}
		return true
	})

	return detected
}

// buildImportAliases builds a map from local package name to full import path.
//
// For example:
//
//	import "os"           → {"os": "os"}
//	import "os/exec"      → {"exec": "os/exec"}
//	import myos "os"      → {"myos": "os"}
//	import . "os"         → (skipped, dot imports are unusual)
//
// This mapping lets us resolve function calls: when we see `exec.Command()`,
// we look up "exec" in this map to find "os/exec", then check if "os/exec"
// is in our capability mappings.
func buildImportAliases(file *ast.File) map[string]string {
	aliases := make(map[string]string)
	for _, imp := range file.Imports {
		importPath := strings.Trim(imp.Path.Value, `"`)

		var localName string
		if imp.Name != nil {
			// Explicit alias: import myos "os"
			localName = imp.Name.Name
		} else {
			// Default: last component of path. "os/exec" → "exec"
			parts := strings.Split(importPath, "/")
			localName = parts[len(parts)-1]
		}

		// Skip dot imports and blank imports
		if localName == "." || localName == "_" {
			continue
		}

		aliases[localName] = importPath
	}
	return aliases
}

// matchesPackage checks if a local package name in the source code matches
// a rule's expected package name.
//
// For example, rule.pkg might be "exec" (meaning the os/exec package).
// The source code might use the local name "exec" (default) or "myexec"
// (aliased). We check:
//
//  1. Direct match: localPkg == rule.pkg
//  2. Alias resolution: look up localPkg in aliases, extract the last
//     path component, and compare with rule.pkg
func matchesPackage(localPkg string, rulePkg string, aliases map[string]string) bool {
	// Direct match — most common case
	if localPkg == rulePkg {
		return true
	}

	// Alias resolution — check if the import path's last component
	// matches the rule's expected package name
	if fullPath, ok := aliases[localPkg]; ok {
		parts := strings.Split(fullPath, "/")
		lastComponent := parts[len(parts)-1]
		if lastComponent == rulePkg {
			return true
		}
	}

	return false
}

// extractStringLiteral extracts the string value from an AST expression
// if it's a string literal (ast.BasicLit with token.STRING). Returns "*"
// if the expression is anything else (variable, function call, etc.)
// because we can't determine the value statically.
//
// Examples:
//
//	os.Open("file.txt")  → "file.txt"
//	os.Open(path)        → "*"
//	os.Open(getPath())   → "*"
func extractStringLiteral(expr ast.Expr) string {
	lit, ok := expr.(*ast.BasicLit)
	if !ok || lit.Kind != token.STRING {
		return "*"
	}
	// Strip quotes. Go string literals are either "..." or `...`
	s := lit.Value
	if len(s) >= 2 {
		if s[0] == '"' && s[len(s)-1] == '"' {
			s = s[1 : len(s)-1]
		} else if s[0] == '`' && s[len(s)-1] == '`' {
			s = s[1 : len(s)-1]
		}
	}
	return s
}
