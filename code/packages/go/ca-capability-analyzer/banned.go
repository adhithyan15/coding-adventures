package main

// banned.go detects restricted code constructs in a Go AST file.
//
// Most of these constructs remain hard CAP002 violations. The only exceptions
// the Go analyzer currently allows are the FFI-style bridges (`import "C"` and
// `plugin.Open`) when a package explicitly opts in through both
// banned_construct_exceptions and matching ffi capabilities.
//
// # The Five Banned Categories
//
// 1. unsafe.Pointer arithmetic
//    Bypasses Go's type system and memory safety. Can forge interface values,
//    read/write arbitrary memory, and defeat garbage collector invariants.
//    Mere import of "unsafe" is not banned (it has legitimate uses like
//    unsafe.Sizeof for struct sizing). Only unsafe.Pointer(expr) conversions
//    are flagged.
//
// 2. import "C" (CGo)
//    Any file that imports "C" can call arbitrary C code, which bypasses the
//    entire Go capability system. The C code can open files, make network
//    connections, or read environment variables with no manifest declaration.
//
// 3. plugin.Open
//    Loads an arbitrary shared library at runtime, executing its init()
//    functions. Equivalent to dynamic code execution — the loaded plugin can
//    do anything, with no capability tracking possible.
//
// 4. reflect.Value dynamic dispatch
//    reflect.Value.Call, CallSlice, and MethodByName allow calling arbitrary
//    functions by name at runtime. An attacker could call os.ReadFile through
//    reflection without any visible os.ReadFile in the AST.
//
// 5. //go:linkname directives
//    Allows a package to reach into another package's unexported symbols,
//    defeating encapsulation. Could be used to bypass capability checks by
//    calling internal enforcement functions directly.

import (
	"go/ast"
	"go/token"
	"strings"
)

const (
	bannedConstructUnsafePointer       = "unsafe.Pointer"
	bannedConstructImportC             = `import "C"`
	bannedConstructPluginOpen          = "plugin.Open"
	bannedConstructReflectCall         = "reflect.Value.Call"
	bannedConstructReflectCallSlice    = "reflect.Value.CallSlice"
	bannedConstructReflectMethodByName = "reflect.Value.MethodByName"
	bannedConstructLinkname            = "//go:linkname"
)

// DetectBanned returns all restricted constructs found in f.
//
// The caller is responsible for not passing gen_capabilities.go to this
// function — generated files are exempt from all checks.
//
// The filename parameter is used in BannedConstruct.File; it should be
// the path relative to the analyzed directory.
func DetectBanned(fset *token.FileSet, f *ast.File, filename string) []BannedConstruct {
	var found []BannedConstruct

	found = append(found, detectCgo(fset, f, filename)...)
	found = append(found, detectUnsafe(fset, f, filename)...)
	found = append(found, detectPlugin(fset, f, filename)...)
	found = append(found, detectReflect(fset, f, filename)...)
	found = append(found, detectLinkname(fset, f, filename)...)

	return found
}

// detectCgo looks for import "C" in the file's import declarations.
//
// The "C" pseudo-package is the CGo bridge. Any file that imports it can
// call arbitrary C code via the cgo tool. Detection is simple: scan the
// import specs for a path literal equal to "C".
//
// Note: CGo imports typically appear at the top of the file, immediately
// after a comment block containing C declarations.
func detectCgo(fset *token.FileSet, f *ast.File, filename string) []BannedConstruct {
	var found []BannedConstruct
	for _, spec := range f.Imports {
		if spec.Path.Value == `"C"` {
			pos := fset.Position(spec.Pos())
			found = append(found, BannedConstruct{
				File:      filename,
				Line:      pos.Line,
				Construct: bannedConstructImportC,
				Kind:      `import "C" (CGo)`,
			})
		}
	}
	return found
}

// detectUnsafe looks for unsafe.Pointer(...) conversion expressions.
//
// The "unsafe" package is not banned outright because unsafe.Sizeof,
// unsafe.Alignof, and unsafe.Offsetof are legitimate compile-time operations
// with no runtime memory danger. Only unsafe.Pointer(expr) conversions are
// flagged, as these allow arbitrary memory reinterpretation at runtime.
//
// AST pattern: ast.CallExpr where Fun is ast.SelectorExpr{X: "unsafe", Sel: "Pointer"}.
func detectUnsafe(fset *token.FileSet, f *ast.File, filename string) []BannedConstruct {
	// First check: is "unsafe" imported at all? If not, no unsafe.Pointer can exist.
	if !hasImport(f, "unsafe") {
		return nil
	}

	// Find the local name for "unsafe" (handles aliased imports like `import u "unsafe"`).
	localName := importLocalName(f, "unsafe")

	var found []BannedConstruct
	ast.Inspect(f, func(n ast.Node) bool {
		call, ok := n.(*ast.CallExpr)
		if !ok {
			return true
		}
		sel, ok := call.Fun.(*ast.SelectorExpr)
		if !ok {
			return true
		}
		ident, ok := sel.X.(*ast.Ident)
		if !ok {
			return true
		}
		if ident.Name == localName && sel.Sel.Name == "Pointer" {
			pos := fset.Position(call.Pos())
			found = append(found, BannedConstruct{
				File:      filename,
				Line:      pos.Line,
				Construct: bannedConstructUnsafePointer,
				Kind:      "unsafe.Pointer conversion",
			})
		}
		return true
	})
	return found
}

// detectPlugin looks for plugin.Open(...) calls.
//
// plugin.Open loads a shared library (.so / .dylib) at runtime, executing
// its init() functions and making its symbols available. The loaded code can
// perform any OS operation with no capability tracking. This is equivalent to
// dynamic code execution and is banned unconditionally.
//
// AST pattern: ast.CallExpr where Fun is ast.SelectorExpr{X: "plugin", Sel: "Open"}.
func detectPlugin(fset *token.FileSet, f *ast.File, filename string) []BannedConstruct {
	if !hasImport(f, "plugin") {
		return nil
	}
	localName := importLocalName(f, "plugin")

	var found []BannedConstruct
	ast.Inspect(f, func(n ast.Node) bool {
		call, ok := n.(*ast.CallExpr)
		if !ok {
			return true
		}
		sel, ok := call.Fun.(*ast.SelectorExpr)
		if !ok {
			return true
		}
		ident, ok := sel.X.(*ast.Ident)
		if !ok {
			return true
		}
		if ident.Name == localName && sel.Sel.Name == "Open" {
			pos := fset.Position(call.Pos())
			found = append(found, BannedConstruct{
				File:      filename,
				Line:      pos.Line,
				Construct: bannedConstructPluginOpen,
				Kind:      "plugin.Open (dynamic library loading)",
			})
		}
		return true
	})
	return found
}

// detectReflect looks for reflect.Value method calls that enable dynamic dispatch:
// Call, CallSlice, and MethodByName.
//
// These three methods allow invoking arbitrary functions by name at runtime.
// An attacker could write reflect.ValueOf(os.ReadFile).Call(...) to call
// os.ReadFile without any visible os.ReadFile in the AST, defeating static
// capability analysis.
//
// Note: reflect.TypeOf, reflect.ValueOf, and value.Field are NOT banned because
// they enable legitimate generic programming without dynamic dispatch.
//
// # Detection approach and known false-positive risk
//
// AST-only analysis cannot resolve the type of a receiver without
// golang.org/x/tools/go/types, which is excluded from this zero-dependency
// package. Instead, detection works in two steps:
//
//  1. Confirm "reflect" is imported (guard: avoids any false positives when
//     the import is absent).
//  2. Flag any call whose method name is "Call", "CallSlice", or "MethodByName".
//
// Step 2 may produce false positives: if a file imports "reflect" AND defines
// its own type with a method named "Call", that method call will be flagged.
// This is an intentional tradeoff — the security cost of a false negative
// (missing actual reflect.Value dynamic dispatch) is far higher than the
// nuisance cost of a false positive (manually suppressing with //nolint:cap).
//
// In practice, user-defined types with these exact method names are rare in
// the monorepo; the comment documents the risk for future maintainers.
func detectReflect(fset *token.FileSet, f *ast.File, filename string) []BannedConstruct {
	if !hasImport(f, "reflect") {
		return nil
	}

	bannedMethods := map[string]bool{
		"Call":         true,
		"CallSlice":    true,
		"MethodByName": true,
	}

	var found []BannedConstruct
	ast.Inspect(f, func(n ast.Node) bool {
		call, ok := n.(*ast.CallExpr)
		if !ok {
			return true
		}
		sel, ok := call.Fun.(*ast.SelectorExpr)
		if !ok {
			return true
		}
		if bannedMethods[sel.Sel.Name] {
			pos := fset.Position(call.Pos())
			construct := bannedConstructReflectCall
			switch sel.Sel.Name {
			case "CallSlice":
				construct = bannedConstructReflectCallSlice
			case "MethodByName":
				construct = bannedConstructReflectMethodByName
			}
			found = append(found, BannedConstruct{
				File:      filename,
				Line:      pos.Line,
				Construct: construct,
				Kind:      "reflect." + sel.Sel.Name + " (dynamic dispatch)",
			})
		}
		return true
	})
	return found
}

// detectLinkname scans the file's comment groups for //go:linkname directives.
//
// go:linkname is a compiler directive that allows a package to bind to another
// package's unexported symbol by name. It defeats encapsulation completely.
// An attacker could use it to directly call internal enforcement functions or
// access private state.
//
// Detection: scan all comment groups for lines containing "go:linkname".
// The directive must appear in a comment (//go:linkname ...) but the
// go/parser places these in the comment list regardless of their exact form.
func detectLinkname(fset *token.FileSet, f *ast.File, filename string) []BannedConstruct {
	var found []BannedConstruct
	for _, cg := range f.Comments {
		for _, c := range cg.List {
			if strings.Contains(c.Text, "go:linkname") {
				pos := fset.Position(c.Pos())
				found = append(found, BannedConstruct{
					File:      filename,
					Line:      pos.Line,
					Construct: bannedConstructLinkname,
					Kind:      "//go:linkname directive",
				})
			}
		}
	}
	return found
}

// normalizeBannedConstructName canonicalizes manifest and detection identifiers
// so a package can write either "plugin.Open" or "plugin.Open()" and still
// match the same restricted construct.
func normalizeBannedConstructName(construct string) string {
	construct = strings.TrimSpace(construct)
	switch {
	case strings.HasPrefix(construct, bannedConstructImportC):
		return bannedConstructImportC
	case strings.HasPrefix(construct, bannedConstructPluginOpen):
		return bannedConstructPluginOpen
	case strings.HasPrefix(construct, bannedConstructUnsafePointer):
		return bannedConstructUnsafePointer
	case strings.HasPrefix(construct, bannedConstructReflectCallSlice):
		return bannedConstructReflectCallSlice
	case strings.HasPrefix(construct, bannedConstructReflectCall):
		return bannedConstructReflectCall
	case strings.HasPrefix(construct, bannedConstructReflectMethodByName):
		return bannedConstructReflectMethodByName
	case strings.Contains(construct, "go:linkname"):
		return bannedConstructLinkname
	default:
		return strings.TrimSuffix(construct, "()")
	}
}

// ── Helpers ──────────────────────────────────────────────────────────────────

// hasImport reports whether f imports the given package path.
func hasImport(f *ast.File, importPath string) bool {
	for _, spec := range f.Imports {
		// spec.Path.Value includes surrounding quotes, e.g., `"os"`.
		if spec.Path.Value == `"`+importPath+`"` {
			return true
		}
	}
	return false
}

// importLocalName returns the local identifier used for a given import path.
// If the import has an explicit alias, that alias is returned. Otherwise, the
// last path segment is returned (e.g., "net/http" → "http").
//
// If the import is not present, returns the last segment of importPath anyway
// (the caller should check hasImport first).
func importLocalName(f *ast.File, importPath string) string {
	for _, spec := range f.Imports {
		if spec.Path.Value == `"`+importPath+`"` {
			if spec.Name != nil && spec.Name.Name != "_" && spec.Name.Name != "." {
				return spec.Name.Name
			}
			// Default: last segment of the import path.
			parts := strings.Split(importPath, "/")
			return parts[len(parts)-1]
		}
	}
	// Not found — return last segment as fallback.
	parts := strings.Split(importPath, "/")
	return parts[len(parts)-1]
}
