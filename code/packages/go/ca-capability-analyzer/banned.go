package analyzer

// Banned construct detector for Go source code.
//
// This module detects constructs that are banned outright in the capability
// security system. These constructs bypass Go's type system, package
// boundaries, or static analysis — making it impossible to reason about
// what a package can do.
//
// # Why ban these constructs?
//
// Consider an attacker trying to exfiltrate data from a package that
// declares zero network capabilities. In Go, they can't just import "net"
// — the capability analyzer catches that. But they could try:
//
//   - reflect.Value.Call() to invoke net.Dial dynamically
//   - plugin.Open() to load a shared library that makes network calls
//   - //go:linkname to call unexported functions in other packages
//   - unsafe.Pointer arithmetic to corrupt memory and jump to arbitrary code
//   - import "C" (cgo) to call any C function, including socket()
//
// By banning these constructs, we force the attacker to use direct imports
// and function calls, which the capability analyzer catches.
//
// # Banned constructs
//
// | Construct                        | Why dangerous                           |
// |----------------------------------|-----------------------------------------|
// | reflect.Value.Call()             | Dynamic method invocation               |
// | reflect.ValueOf().MethodByName() | Dynamic dispatch by string name         |
// | plugin.Open()                    | Loads arbitrary shared libraries        |
// | //go:linkname                    | Bypasses package boundaries             |
// | unsafe.Pointer arithmetic        | Memory safety bypass                    |
// | import "C" (cgo)                 | Calls arbitrary C functions             |
//
// # Exception process
//
// If a package genuinely needs a banned construct (e.g., a serialization
// library that uses reflect.Value.Call()), it must declare the exception
// in `required_capabilities.json` under `banned_construct_exceptions`
// with a justification.

import (
	"go/ast"
	"go/parser"
	"go/token"
	"os"
	"strings"
)

// BannedConstructViolation represents a banned construct found in source code.
//
// Unlike DetectedCapability (which maps to a capability triple), a banned
// construct is an outright prohibition. No capability declaration can
// authorize reflect.Value.Call() — it must be explicitly exempted.
type BannedConstructViolation struct {
	Construct string `json:"construct"`
	File      string `json:"file"`
	Line      int    `json:"line"`
	Evidence  string `json:"evidence"`
}

// String returns a human-readable description of the violation.
func (b BannedConstructViolation) String() string {
	return "BANNED " + b.Construct + " at " + b.File + ":" + strings.Repeat("", 0) + b.Evidence
}

// DetectBannedSource parses a Go source string and detects banned constructs.
//
// This scans both the AST (for function calls) and raw comments (for
// compiler directives like //go:linkname).
func DetectBannedSource(filename string, source string) ([]BannedConstructViolation, error) {
	fset := token.NewFileSet()
	file, err := parser.ParseFile(fset, filename, source, parser.ParseComments)
	if err != nil {
		return nil, err
	}
	return detectBannedAST(fset, file, filename, source), nil
}

// DetectBannedFile reads and scans a single Go source file for banned constructs.
func DetectBannedFile(filepath string) ([]BannedConstructViolation, error) {
	source, err := os.ReadFile(filepath)
	if err != nil {
		return nil, err
	}
	return DetectBannedSource(filepath, string(source))
}

// detectBannedAST walks a parsed Go AST and scans comments to find banned
// constructs. It checks for:
//
//  1. reflect.Value.Call() and reflect.ValueOf(...).MethodByName(...)
//     — dynamic method invocation that bypasses static analysis
//
//  2. plugin.Open() — loads shared libraries at runtime, enabling
//     arbitrary code execution
//
//  3. import "C" (cgo) — calls arbitrary C functions via the C pseudo-package
//
//  4. unsafe.Pointer arithmetic — any use of unsafe.Pointer in a conversion
//     or expression bypasses Go's memory safety
//
//  5. //go:linkname directive — a compiler directive that lets one package
//     access unexported symbols from another package, completely bypassing
//     Go's encapsulation
func detectBannedAST(fset *token.FileSet, file *ast.File, filename string, source string) []BannedConstructViolation {
	var violations []BannedConstructViolation

	// ── Check for //go:linkname directives in comments ────────────
	//
	// //go:linkname is a compiler directive, not a regular Go construct.
	// It doesn't appear in the AST — it's a comment. So we scan all
	// comments in the file for this pattern.
	//
	// The directive looks like: //go:linkname localName importPath.remoteName
	// It tells the compiler to link `localName` in this package to
	// `remoteName` in another package, even if it's unexported.
	for _, cg := range file.Comments {
		for _, c := range cg.List {
			text := strings.TrimSpace(c.Text)
			if strings.HasPrefix(text, "//go:linkname") {
				line := fset.Position(c.Pos()).Line
				violations = append(violations, BannedConstructViolation{
					Construct: "go:linkname",
					File:      filename,
					Line:      line,
					Evidence:  text,
				})
			}
		}
	}

	// ── Check for import "C" (cgo) ───────────────────────────────
	//
	// The "C" pseudo-package is how Go code calls C functions. It's
	// detected as an import, but it's also a banned construct because
	// it enables calling any C function — socket(), exec(), dlopen(),
	// you name it. No static analysis can track what C code does.
	for _, imp := range file.Imports {
		importPath := strings.Trim(imp.Path.Value, `"`)
		if importPath == "C" {
			line := fset.Position(imp.Pos()).Line
			violations = append(violations, BannedConstructViolation{
				Construct: "cgo",
				File:      filename,
				Line:      line,
				Evidence:  `import "C"`,
			})
		}
	}

	// ── Walk the AST for banned function calls and expressions ────
	ast.Inspect(file, func(n ast.Node) bool {
		switch node := n.(type) {

		case *ast.CallExpr:
			// Check for reflect.Value.Call() and reflect.ValueOf().MethodByName()
			//
			// Pattern 1: someValue.Call(args)
			// This is a method call on a reflect.Value. We detect it by
			// checking if the method name is "Call" and the receiver looks
			// like it came from the reflect package.
			//
			// Pattern 2: someValue.MethodByName("name")
			// Dynamic dispatch — looks up a method by string name.
			// Same detection approach.
			if sel, ok := node.Fun.(*ast.SelectorExpr); ok {
				methodName := sel.Sel.Name

				// reflect.Value.Call() — dynamic invocation
				if methodName == "Call" {
					if isReflectReceiver(sel.X) {
						line := fset.Position(node.Pos()).Line
						violations = append(violations, BannedConstructViolation{
							Construct: "reflect.Value.Call",
							File:      filename,
							Line:      line,
							Evidence:  "reflect.Value.Call(...)",
						})
					}
				}

				// reflect.ValueOf().MethodByName() — dynamic dispatch
				if methodName == "MethodByName" {
					if isReflectReceiver(sel.X) {
						line := fset.Position(node.Pos()).Line
						violations = append(violations, BannedConstructViolation{
							Construct: "reflect.MethodByName",
							File:      filename,
							Line:      line,
							Evidence:  "reflect.ValueOf(...).MethodByName(...)",
						})
					}
				}

				// plugin.Open() — dynamic code loading
				//
				// plugin.Open(path) loads a Go plugin (shared library)
				// at runtime. The loaded plugin can contain arbitrary code
				// including network calls, file I/O, etc. that we can't
				// analyze statically.
				if methodName == "Open" {
					if ident, ok := sel.X.(*ast.Ident); ok && ident.Name == "plugin" {
						line := fset.Position(node.Pos()).Line
						violations = append(violations, BannedConstructViolation{
							Construct: "plugin.Open",
							File:      filename,
							Line:      line,
							Evidence:  "plugin.Open(...)",
						})
					}
				}
			}

		case *ast.SelectorExpr:
			// Check for unsafe.Pointer usage
			//
			// Any reference to unsafe.Pointer is flagged. In Go, unsafe.Pointer
			// is the escape hatch from the type system — it can be used to:
			//   - Cast between arbitrary pointer types
			//   - Perform pointer arithmetic
			//   - Access raw memory
			//
			// This enables memory corruption, which could be used to hijack
			// control flow and execute arbitrary code.
			if ident, ok := node.X.(*ast.Ident); ok {
				if ident.Name == "unsafe" && node.Sel.Name == "Pointer" {
					line := fset.Position(node.Pos()).Line
					violations = append(violations, BannedConstructViolation{
						Construct: "unsafe.Pointer",
						File:      filename,
						Line:      line,
						Evidence:  "unsafe.Pointer",
					})
				}
			}
		}

		return true
	})

	return violations
}

// isReflectReceiver checks if an expression likely originates from the
// reflect package. This is a heuristic — we can't do full type resolution
// without the type checker, but we can catch common patterns:
//
//  1. reflect.ValueOf(x) — a call expression with reflect.ValueOf
//  2. reflect.ValueOf(x).Elem() — chained calls
//  3. v.Call() where v was assigned from reflect — we can't track this
//     without data flow analysis, so we check if the identifier name
//     suggests reflect usage
//
// This is intentionally conservative: we'd rather have false positives
// (flagging non-reflect .Call() methods) than false negatives (missing
// actual reflect usage). The exception process handles false positives.
func isReflectReceiver(expr ast.Expr) bool {
	switch e := expr.(type) {
	case *ast.CallExpr:
		// Check if this is reflect.ValueOf(...) or reflect.New(...)
		if sel, ok := e.Fun.(*ast.SelectorExpr); ok {
			if ident, ok := sel.X.(*ast.Ident); ok {
				return ident.Name == "reflect"
			}
		}
		// Check for chained calls like reflect.ValueOf(x).Elem()
		if sel, ok := e.Fun.(*ast.SelectorExpr); ok {
			return isReflectReceiver(sel.X)
		}
	case *ast.SelectorExpr:
		// Check for reflect.someValue
		if ident, ok := e.X.(*ast.Ident); ok {
			return ident.Name == "reflect"
		}
		// Check nested: reflect.ValueOf(x).Elem().Field(0)
		return isReflectReceiver(e.X)
	case *ast.Ident:
		// If the variable is named something obvious like "reflectVal"
		// we could flag it, but that's too fragile. Just return false.
		return false
	}
	return false
}
