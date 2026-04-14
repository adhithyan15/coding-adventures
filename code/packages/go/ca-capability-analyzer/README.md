# ca-capability-analyzer

A Go static analysis tool that enforces capability manifest discipline across the monorepo.

## What It Does

Every Go package in this repository declares its OS-level capabilities in a `required_capabilities.json` file. The `capability-cage-generator` bakes those declarations into `gen_capabilities.go`, providing the Operations system (`op.File.ReadFile`, `op.Time.Now`, etc.) as the only sanctioned way to access OS resources.

`ca-capability-analyzer` closes the gap: it walks the AST of every `.go` file in a package and reports any raw stdlib calls that bypass the Operations system.

```
Package source files
       │
       ▼
  ca-capability-analyzer
       │
       ├── Detects raw OS calls → os.ReadFile, time.Now, net.Dial, ...
       │
       ├── Detects banned constructs → unsafe.Pointer, import "C", plugin.Open, ...
       │
       └── Compares against required_capabilities.json
              │
              ├── Detected ⊆ Declared → EXIT 0 (pass)
              └── Detected ⊄ Declared → EXIT 1 (violations)
```

## Where It Fits

This package implements Layers 3 and 4 of the capability security system described in [spec 13](../../../specs/13-capability-security.md) and [D21](../../../specs/D21-capability-cage.md):

| Layer | Role | Tool |
|-------|------|------|
| 3 | Linter (developer feedback) | Run in `BUILD` alongside tests |
| 4 | CI Gate (independent check) | Run in `capability-gate` CI job |

## Usage

### CLI

```bash
# Analyze the current directory
ca-capability-analyzer

# Analyze a specific package directory
ca-capability-analyzer --dir code/packages/go/json-lexer

# Verbose: show detected capabilities even when passing
ca-capability-analyzer --dir code/packages/go/json-lexer --verbose
```

### Exit Codes

| Code | Meaning |
|------|---------|
| `0` | All detected capabilities are declared; no banned constructs |
| `1` | One or more violations found |
| `2` | Tool error (directory not found, parse error, etc.) |

### Output Format

```
lexer.go:42: [CAP001] undeclared capability: fs:read:* (os.ReadFile call) — add to required_capabilities.json and regenerate gen_capabilities.go
main.go:7: [CAP002] banned construct: import "C" (CGo)
```

## Capability Detection Rules

### Import-level rules (fire on presence of import alone)

| Import Path | Detected Capability |
|---|---|
| `"net"` | `net:*:*` |
| `"net/http"` | `net:*:*` |
| `"os/exec"` | `proc:exec:*` |

### Call-level rules

| Package | Functions | Detected Capability |
|---|---|---|
| `"os"` | `Open`, `ReadFile`, `OpenFile` | `fs:read:*` |
| `"os"` | `Create`, `WriteFile`, `Mkdir`, `MkdirAll`, `Rename` | `fs:write:*` |
| `"os"` | `Remove`, `RemoveAll` | `fs:delete:*` |
| `"os"` | `ReadDir`, `Stat`, `Lstat` | `fs:list:*` |
| `"os"` | `Getenv`, `Environ`, `LookupEnv` | `env:read:*` |
| `"time"` | `Now`, `Sleep` | `time:read:*` |
| `"fmt"` | `Print`, `Println`, `Printf` | `stdout:write:*` |

### Special patterns

| Expression | Detected Capability |
|---|---|
| `os.Stdout.Write(...)` | `stdout:write:*` |
| `os.Stderr.Write(...)` | `stdout:write:*` |
| `os.Stdin.Read(...)` | `stdin:read:*` |
| `fmt.Fprintf(os.Stdout, ...)` | `stdout:write:*` |
| `fmt.Fprintf(os.Stderr, ...)` | `stdout:write:*` |

## Banned Constructs

These are blocked by default because they defeat static capability analysis entirely.
Two FFI-style constructs can be explicitly opted into: `import "C"` requires both
`banned_construct_exceptions` for `import "C"` and an `ffi:call:*` capability, and
`plugin.Open` requires both a `banned_construct_exceptions` entry for `plugin.Open`
and an `ffi:load:*` capability. The remaining constructs stay hard-blocked.

| Construct | Why It's Banned |
|---|---|
| `unsafe.Pointer(expr)` | Bypasses Go's type system; can forge interface values, read arbitrary memory |
| `import "C"` | CGo calls arbitrary C code; bypasses the entire capability system |
| `plugin.Open(path)` | Loads arbitrary shared libraries at runtime; no capability tracking possible |
| `reflect.Value.Call(...)` | Calls functions by name dynamically; invisible to the static analyzer |
| `reflect.Value.CallSlice(...)` | Same as above |
| `reflect.Value.MethodByName(...)` | Same as above |
| `//go:linkname` | Reaches into unexported symbols; defeats encapsulation |

### Explicit FFI opt-in

If a package genuinely needs a reviewed native bridge, it must declare both:

1. A matching `ffi` capability in `capabilities`
2. A matching `banned_construct_exceptions` entry

Example for `cgo`:

```json
{
  "capabilities": [
    {
      "category": "ffi",
      "action": "call",
      "target": "libbarcode_renderer",
      "justification": "Calls a reviewed native bridge."
    }
  ],
  "banned_construct_exceptions": [
    {
      "construct": "import \"C\"",
      "language": "go",
      "justification": "Uses cgo to call the reviewed native bridge."
    }
  ]
}
```

## Exemptions

Three categories are never flagged:

1. **`gen_capabilities.go`** — Auto-generated file. Its raw OS calls are intentional and the direct read site is controlled.

2. **`//nolint:cap` annotation** — A line with this comment is explicitly suppressed. Used in generated code and tested workarounds.

3. **Operations system calls** — `op.File.ReadFile(...)`, `op.Net.Connect(...)`, `op.Time.Now(...)` etc. are already routed through the capability-checked Operations system.

## Library Usage

The analyzer can be imported as a library (all exported symbols are in `package main`):

```go
result, err := capaanalyzer.AnalyzeDir("/path/to/package")
if err != nil { log.Fatal(err) }

for _, v := range result.Violations {
    fmt.Println(v.Format())
}

if !result.Passed() {
    os.Exit(1)
}
```

Key exported types and functions:

```go
// Full pipeline: walk directory, parse, detect, cross-reference manifest.
func AnalyzeDir(dir string) (*AnalysisResult, error)

// Pure AST analysis — no filesystem access. Useful for testing.
func DetectCapabilities(fset *token.FileSet, files map[string]*ast.File) []DetectedCapability

// Banned construct detection for a single file.
func DetectBanned(fset *token.FileSet, f *ast.File, filename string) []BannedConstruct

// Manifest loading: reads required_capabilities.json and returns declared set.
func LoadManifest(dir string) (map[CapabilityString]bool, error)

// Rich manifest loading: declared capabilities + banned-construct exceptions.
func LoadManifestData(dir string) (*ManifestData, error)
```

## Test Coverage

- **98.2%** statement coverage
- 70+ test cases across capability detection, banned constructs, manifest loading, and CLI
- All 22 call-level rules and 3 import-level rules tested individually
- All 5 banned construct categories tested individually
- Integration tests via `AnalyzeDir` on real temp directories

## Dependencies

**None.** Uses only the Go standard library:
- `go/ast`, `go/parser`, `go/token` — AST analysis
- `encoding/json` — manifest parsing
- `os`, `path/filepath` — directory walking
- `strings`, `fmt`, `flag` — utility

This zero-dependency design means the analyzer can run in any CI environment with only a Go toolchain.
