package main

// analyzer.go is the core of ca-capability-analyzer. It orchestrates the
// full analysis pipeline:
//
//  1. Walk a directory for .go files (skip gen_capabilities.go).
//  2. Parse each file with go/parser (ParseComments mode required for //nolint:cap).
//  3. Run capability detection and banned-construct detection.
//  4. Load required_capabilities.json.
//  5. Cross-reference detected vs declared → produce violations.
//  6. Apply explicit banned-construct exceptions for supported FFI cases.
//  7. Return AnalysisResult.
//
// The two exported functions are:
//
//   - DetectCapabilities: pure AST analysis, no filesystem access. Used by
//     unit tests that parse source strings in memory.
//   - AnalyzeDir: full pipeline including filesystem access and manifest loading.
//     Used by the CLI and integration tests.

import (
	"fmt"
	"go/ast"
	"go/parser"
	"go/token"
	"os"
	"path/filepath"
	"strings"
)

// DetectCapabilities inspects a set of parsed AST files and returns all
// detected raw OS capability usage.
//
// Exemptions applied automatically:
//   - Files named "gen_capabilities.go" (the caller should filter these before
//     passing, but if any slip through they are skipped here).
//   - Calls via op.File.*, op.Net.*, op.Time.* — Operations system.
//   - Calls via cage.ReadFile, cage.WriteFile, etc. — capability-cage wrappers.
//   - Lines annotated with //nolint:cap.
//
// Parameters:
//
//	fset  — the FileSet used when parsing files. Required for position lookups.
//	files — map from filename to parsed *ast.File. Use filename "gen_capabilities.go"
//	        to trigger the generated-file exemption.
//
// Returns a list of DetectedCapability values, one per detected raw OS call.
// Multiple calls on different lines produce separate entries. The list is NOT
// deduplicated by capability — use buildCapabilitySet for that.
func DetectCapabilities(fset *token.FileSet, files map[string]*ast.File) []DetectedCapability {
	var detected []DetectedCapability

	for filename, f := range files {
		// Skip generated files entirely. They contain intentional raw OS calls
		// (marked //nolint:cap) and must never be flagged.
		if filepath.Base(filename) == "gen_capabilities.go" {
			continue
		}

		detected = append(detected, detectInFile(fset, f, filename)...)
	}

	return detected
}

// detectInFile runs capability detection on a single parsed file.
// Called by DetectCapabilities for each non-generated file.
func detectInFile(fset *token.FileSet, f *ast.File, filename string) []DetectedCapability {
	// Build a comment index: line number → comments on that line.
	// This enables O(1) per-call nolint checking during the AST walk.
	commentsByLine := buildCommentIndex(fset, f)

	// Build import map: import path → local identifier.
	// e.g., "os" → "os", "net/http" → "http", or alias if present.
	imports := activeImports(f)

	var detected []DetectedCapability

	// ── Phase 1: Import-level rules ───────────────────────────────────────
	// Some imports imply a capability regardless of which functions are called.
	for _, rule := range ImportRules {
		if localName, present := imports[rule.ImportPath]; present {
			// The import is present. Check if any actual usage of this import
			// exists (avoid flagging unused imports that the compiler would reject
			// anyway). For import-level rules we do flag the import itself.
			_ = localName // local name used in phase 2; here we flag the import line
			importLine := importLine(fset, f, rule.ImportPath)
			if isNolintLine(commentsByLine, importLine) {
				continue
			}
			detected = append(detected, DetectedCapability{
				File:       filename,
				Line:       importLine,
				Capability: rule.Capability,
				Evidence:   rule.Evidence,
			})
		}
	}

	// ── Phase 2: Call-level rules ─────────────────────────────────────────
	// Walk all call expressions and match against CallRules.
	ast.Inspect(f, func(n ast.Node) bool {
		call, ok := n.(*ast.CallExpr)
		if !ok {
			return true
		}

		// Skip calls routed through the Operations system or cage wrappers.
		if isOperationsCall(call) {
			return true
		}

		// We expect a simple selector expression: pkg.FunctionName(...)
		sel, ok := call.Fun.(*ast.SelectorExpr)
		if !ok {
			// Could be a method call like v.Read() — handled in Phase 3.
			return true
		}

		// The receiver must be a simple identifier (the package name).
		pkgIdent, ok := sel.X.(*ast.Ident)
		if !ok {
			return true
		}

		funcName := sel.Sel.Name
		pkgLocalName := pkgIdent.Name

		for _, rule := range CallRules {
			localName, present := imports[rule.ImportPath]
			if !present {
				continue
			}
			if localName != pkgLocalName {
				continue
			}
			if rule.FunctionName != funcName {
				continue
			}

			callLine := fset.Position(call.Pos()).Line
			if isNolintLine(commentsByLine, callLine) {
				continue
			}

			detected = append(detected, DetectedCapability{
				File:       filename,
				Line:       callLine,
				Capability: rule.Capability,
				Evidence:   rule.Evidence,
			})
		}
		return true
	})

	// ── Phase 3: Special multi-level selectors ────────────────────────────
	// Handles os.Stdout.Write, os.Stdin.Read, and fmt.Fprintf(os.Stdout, ...).
	detected = append(detected, detectSpecialCalls(fset, f, filename, imports, commentsByLine)...)

	return detected
}

// detectSpecialCalls handles capability detection for patterns that don't fit
// the simple pkg.Func(...) model used by CallRules:
//
//   - os.Stdout.Write(p)     → stdout:write:*
//   - os.Stdin.Read(p)       → stdin:read:*
//   - fmt.Fprintf(os.Stdout, ...) → stdout:write:*
func detectSpecialCalls(
	fset *token.FileSet,
	f *ast.File,
	filename string,
	imports map[string]string,
	commentsByLine map[int][]*ast.CommentGroup,
) []DetectedCapability {
	osLocal, hasOS := imports["os"]
	fmtLocal, hasFmt := imports["fmt"]

	var detected []DetectedCapability

	ast.Inspect(f, func(n ast.Node) bool {
		call, ok := n.(*ast.CallExpr)
		if !ok {
			return true
		}
		if isOperationsCall(call) {
			return true
		}

		callLine := fset.Position(call.Pos()).Line
		if isNolintLine(commentsByLine, callLine) {
			return true
		}

		// ── os.Stdout.Write(...) and os.Stdin.Read(...) ───────────────
		// AST: SelectorExpr{ X: SelectorExpr{ X: Ident{osLocal}, Sel: "Stdout"/"Stdin" }, Sel: "Write"/"Read" }
		if hasOS {
			if outer, ok := call.Fun.(*ast.SelectorExpr); ok {
				if inner, ok := outer.X.(*ast.SelectorExpr); ok {
					if ident, ok := inner.X.(*ast.Ident); ok && ident.Name == osLocal {
						switch inner.Sel.Name {
						case "Stdout":
							if outer.Sel.Name == "Write" || outer.Sel.Name == "WriteString" {
								detected = append(detected, DetectedCapability{
									File:       filename,
									Line:       callLine,
									Capability: "stdout:write:*",
									Evidence:   "os.Stdout." + outer.Sel.Name + " call",
								})
							}
						case "Stderr":
							if outer.Sel.Name == "Write" || outer.Sel.Name == "WriteString" {
								detected = append(detected, DetectedCapability{
									File:       filename,
									Line:       callLine,
									Capability: "stdout:write:*",
									Evidence:   "os.Stderr." + outer.Sel.Name + " call",
								})
							}
						case "Stdin":
							if outer.Sel.Name == "Read" {
								detected = append(detected, DetectedCapability{
									File:       filename,
									Line:       callLine,
									Capability: "stdin:read:*",
									Evidence:   "os.Stdin.Read call",
								})
							}
						}
					}
				}
			}
		}

		// ── fmt.Fprintf(os.Stdout, ...) ──────────────────────────────
		// AST: CallExpr{ Fun: SelectorExpr{ X: Ident{fmtLocal}, Sel: "Fprintf" },
		//               Args[0]: SelectorExpr{ X: Ident{osLocal}, Sel: "Stdout"/"Stderr" } }
		if hasFmt && hasOS {
			if sel, ok := call.Fun.(*ast.SelectorExpr); ok {
				if ident, ok := sel.X.(*ast.Ident); ok && ident.Name == fmtLocal {
					if sel.Sel.Name == "Fprintf" && len(call.Args) > 0 {
						if argSel, ok := call.Args[0].(*ast.SelectorExpr); ok {
							if argIdent, ok := argSel.X.(*ast.Ident); ok && argIdent.Name == osLocal {
								if argSel.Sel.Name == "Stdout" || argSel.Sel.Name == "Stderr" {
									detected = append(detected, DetectedCapability{
										File:       filename,
										Line:       callLine,
										Capability: "stdout:write:*",
										Evidence:   "fmt.Fprintf(" + osLocal + "." + argSel.Sel.Name + ", ...) call",
									})
								}
							}
						}
					}
				}
			}
		}

		return true
	})

	return detected
}

// AnalyzeDir performs a complete analysis of the Go package at dir.
//
// Steps:
//  1. Walk dir for .go files (skipping gen_capabilities.go and test helpers).
//  2. Parse each file with go/parser in ParseComments mode.
//  3. Run DetectCapabilities and DetectBanned over all parsed files.
//  4. Load required_capabilities.json via LoadManifestData.
//  5. Cross-reference: detected ⊄ declared → CAP001 violations.
//  6. Every remaining disallowed restricted construct → CAP002 violation.
//  7. Return AnalysisResult.
func AnalyzeDir(dir string) (*AnalysisResult, error) {
	absDir, err := filepath.Abs(dir)
	if err != nil {
		return nil, err
	}

	result := &AnalysisResult{Dir: absDir}

	// ── Step 1+2: Walk and parse ──────────────────────────────────────────
	fset := token.NewFileSet()
	parsedFiles := make(map[string]*ast.File)

	entries, err := os.ReadDir(absDir) //nolint:cap
	if err != nil {
		return nil, err
	}

	for _, entry := range entries {
		if entry.IsDir() {
			continue
		}
		name := entry.Name()
		if !strings.HasSuffix(name, ".go") {
			continue
		}
		fullPath := filepath.Join(absDir, name)
		f, err := parser.ParseFile(fset, fullPath, nil, parser.ParseComments)
		if err != nil {
			// Record unparseable files (e.g., build-tag-restricted files or
			// files with syntax errors) as parse warnings rather than silently
			// skipping them. A silently skipped file means violations within it
			// go undetected, which could produce a false-clean exit code 0.
			// The CLI prints ParseErrors to stderr so developers see them.
			result.ParseErrors = append(result.ParseErrors,
				fmt.Sprintf("%s: parse error: %v", name, err))
			continue
		}
		// Use the relative name as the key so that violation messages show
		// short paths rather than absolute paths.
		parsedFiles[name] = f
	}

	// ── Step 3: Detect capabilities and banned constructs ─────────────────
	result.Detected = DetectCapabilities(fset, parsedFiles)

	for filename, f := range parsedFiles {
		if filepath.Base(filename) == "gen_capabilities.go" {
			continue
		}
		result.Banned = append(result.Banned, DetectBanned(fset, f, filename)...)
	}

	// ── Step 4: Load manifest ─────────────────────────────────────────────
	manifest, err := LoadManifestData(absDir)
	if err != nil {
		return nil, err
	}
	declared := manifest.Declared
	for k := range declared {
		result.Declared = append(result.Declared, k)
	}

	// ── Step 5: Cross-reference detected vs declared ──────────────────────
	// Build the unique set of capability strings detected. We report one
	// violation per unique capability (not per call site) to avoid flooding
	// the output. Verbose mode shows all call sites separately.
	detectedSet := buildCapabilitySet(result.Detected)
	for cap := range detectedSet {
		if !declared[cap] {
			// Find one representative DetectedCapability for this cap string
			// to get the file/line for the violation message.
			for _, d := range result.Detected {
				if d.Capability == cap {
					result.Violations = append(result.Violations, newViolationCAP001(d))
					break
				}
			}
		}
	}

	// ── Step 6: Restricted constructs → violations ────────────────────────
	filteredBanned := make([]BannedConstruct, 0, len(result.Banned))
	for _, b := range result.Banned {
		allowed, hint := allowBannedConstruct(manifest, b)
		if allowed {
			continue
		}
		filteredBanned = append(filteredBanned, b)
		result.Violations = append(result.Violations, newViolationCAP002Hint(b, hint))
	}
	result.Banned = filteredBanned

	return result, nil
}

// allowBannedConstruct returns whether the restricted construct is explicitly
// authorized for this package. Today this is intentionally narrow: only the
// two FFI-style bridge constructs may opt in, and they must declare both a
// banned_construct_exceptions entry and the matching ffi capability.
func allowBannedConstruct(manifest *ManifestData, banned BannedConstruct) (bool, string) {
	switch banned.Construct {
	case bannedConstructImportC:
		return allowExplicitFFIConstruct(manifest, banned, "ffi:call:*")
	case bannedConstructPluginOpen:
		return allowExplicitFFIConstruct(manifest, banned, "ffi:load:*")
	default:
		return false, ""
	}
}

func allowExplicitFFIConstruct(
	manifest *ManifestData,
	banned BannedConstruct,
	required CapabilityString,
) (bool, string) {
	exceptionKey := canonicalBannedConstructExceptionKey("go", banned.Construct)
	hasException := manifest.BannedConstructExceptions[exceptionKey]
	hasCapability := manifest.Declared[required]

	if hasException && hasCapability {
		return true, ""
	}

	switch {
	case !hasException && !hasCapability:
		return false, fmt.Sprintf(
			"add banned_construct_exceptions for %q and declare %s",
			banned.Construct, required,
		)
	case !hasException:
		return false, fmt.Sprintf(
			"add banned_construct_exceptions for %q",
			banned.Construct,
		)
	default:
		return false, fmt.Sprintf(
			"declare %s to accompany the %q exception",
			required, banned.Construct,
		)
	}
}

// ── Internal helpers ──────────────────────────────────────────────────────────

// activeImports returns a map from import path to local identifier for all
// imports in f.
//
// Examples:
//   - `import "os"` → {"os": "os"}
//   - `import myfmt "fmt"` → {"fmt": "myfmt"}
//   - `import _ "net/http"` → not included (blank imports don't contribute names)
//   - `import . "fmt"` → not included (dot imports scatter symbols into the namespace)
func activeImports(f *ast.File) map[string]string {
	result := make(map[string]string)
	for _, spec := range f.Imports {
		// spec.Path.Value includes surrounding quotes; strip them.
		path := strings.Trim(spec.Path.Value, `"`)

		if spec.Name != nil {
			switch spec.Name.Name {
			case "_":
				// Blank import: imported for side effects only. No exported name.
				continue
			case ".":
				// Dot import: all exported names scattered into the current package.
				// We don't track these (they would require symbol resolution).
				continue
			default:
				result[path] = spec.Name.Name
			}
		} else {
			// Default: last segment of the import path.
			parts := strings.Split(path, "/")
			result[path] = parts[len(parts)-1]
		}
	}
	return result
}

// buildCommentIndex returns a map from 1-based line number to the comment
// groups that start on that line. Used by isNolintLine for O(1) lookup.
func buildCommentIndex(fset *token.FileSet, f *ast.File) map[int][]*ast.CommentGroup {
	idx := make(map[int][]*ast.CommentGroup)
	for _, cg := range f.Comments {
		line := fset.Position(cg.Pos()).Line
		idx[line] = append(idx[line], cg)
	}
	return idx
}

// isNolintLine reports whether the given 1-based line number carries a
// //nolint:cap annotation. When it does, any capability detection on that
// line is suppressed.
//
// The //nolint:cap convention mirrors golangci-lint's nolint directive format.
// Using the same format means developers can use one comment for both tools.
//
// Matching rules (to prevent substring abuse like "nolint:capfoo"):
//   - Strip the leading "//" from the comment text.
//   - Strip optional leading whitespace (allows "// nolint:cap" and "//nolint:cap").
//   - Require the remainder to start with "nolint:".
//   - After "nolint:", split on commas and check that one token equals "cap" exactly.
//
// This means "//nolint:cap,errcheck" and "// nolint:cap" both match, but
// "//nolint:capfoo" and "// some text mentioning nolint:cap" do not.
func isNolintLine(commentsByLine map[int][]*ast.CommentGroup, line int) bool {
	for _, cg := range commentsByLine[line] {
		for _, c := range cg.List {
			text := c.Text
			// Strip the "//" prefix that all line comments start with.
			if len(text) >= 2 && text[:2] == "//" {
				text = text[2:]
			}
			// Strip optional leading whitespace (handles "// nolint:cap").
			text = strings.TrimLeft(text, " \t")
			// Must be a nolint directive.
			if !strings.HasPrefix(text, "nolint:") {
				continue
			}
			// Extract the linter list and check for "cap" as an exact token.
			linters := text[len("nolint:"):]
			for _, linter := range strings.Split(linters, ",") {
				if strings.TrimSpace(linter) == "cap" {
					return true
				}
			}
		}
	}
	return false
}

// isOperationsCall reports whether call is routed through the Operations system
// or capability-cage wrappers.
//
// Patterns recognized:
//   - op.File.ReadFile(...)  → Operations system (3-level: op.File.ReadFile)
//   - op.Net.Connect(...)    → Operations system
//   - op.Time.Now(...)       → Operations system
//   - cage.ReadFile(...)     → capability-cage secure wrapper
//
// # Security limitation (known best-effort heuristic)
//
// This function exempts calls by checking whether the outermost receiver is
// a variable NAMED "op" or "cage". It does NOT perform type resolution —
// full type inference would require golang.org/x/tools/go/types, which is
// excluded from this zero-dependency package.
//
// Consequence: a developer could write the following to bypass detection:
//
//	op := os.Open   // "op" is now os.Open, not *Operation
//	op("secret")    // NOT flagged — false negative
//
// This is an accepted limitation for a zero-dependency static analyzer.
// The Operations system itself enforces capability checks at runtime, so
// a bypass here only means the static linter misses it — CI and runtime
// enforcement remain intact. The heuristic is reliable for well-typed
// codebases following the monorepo conventions.
//
// False negatives are preferred to false positives: flagging correct
// Operations system calls as violations would make the tool unusable.
func isOperationsCall(call *ast.CallExpr) bool {
	// Check for 3-level: op.File.ReadFile(...)
	// AST: SelectorExpr{ X: SelectorExpr{ X: Ident{"op"}, Sel: _ }, Sel: _ }
	if sel, ok := call.Fun.(*ast.SelectorExpr); ok {
		if inner, ok := sel.X.(*ast.SelectorExpr); ok {
			if ident, ok := inner.X.(*ast.Ident); ok {
				if ident.Name == "op" {
					return true
				}
			}
		}
		// Check for 2-level: cage.ReadFile(...)
		// AST: SelectorExpr{ X: Ident{"cage"}, Sel: _ }
		if ident, ok := sel.X.(*ast.Ident); ok {
			if ident.Name == "cage" {
				return true
			}
		}
	}
	return false
}

// importLine returns the 1-based line number of the import statement for
// the given import path in file f.
//
// If the import is not found, returns 0 (the caller should check hasImport first).
func importLine(fset *token.FileSet, f *ast.File, importPath string) int {
	for _, spec := range f.Imports {
		if strings.Trim(spec.Path.Value, `"`) == importPath {
			return fset.Position(spec.Pos()).Line
		}
	}
	return 0
}

// buildCapabilitySet converts a slice of DetectedCapability to a set of
// unique CapabilityString values. Used to report one violation per unique
// capability rather than one per call site.
func buildCapabilitySet(detected []DetectedCapability) map[CapabilityString]bool {
	set := make(map[CapabilityString]bool)
	for _, d := range detected {
		set[d.Capability] = true
	}
	return set
}
