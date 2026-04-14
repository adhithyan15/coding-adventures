# Changelog

## [1.0.2] - 2026-04-13

### Security

- **Explicit FFI opt-in for Go**: `import "C"` and `plugin.Open` are no longer
  treated the same as the irreducibly dangerous constructs. They remain blocked
  by default, but packages may now opt in explicitly by declaring both a matching
  `banned_construct_exceptions` entry and the corresponding `ffi` capability
  (`ffi:call:*` for `import "C"`, `ffi:load:*` for `plugin.Open`).

- **`LoadManifestData`**: Added a richer manifest loader that returns both declared
  capabilities and `banned_construct_exceptions`, while keeping `LoadManifest`
  as a compatibility helper for callers that only need declared capabilities.

- **CAP002 guidance**: Violations for partially-declared FFI constructs now explain
  the missing opt-in step, so packages know whether they still need the manifest
  exception, the `ffi` capability, or both.

## [1.0.1] - 2026-03-26

### Security

- **`isNolintLine`**: Tightened `//nolint:cap` matching from substring to exact-token.
  Previously `//nolint:capfoo` would suppress detection; now only `cap` as an exact
  linter token (comma-separated, trimmed) is accepted. Multi-linter form
  `//nolint:cap,errcheck` still works correctly.

- **`LoadManifest`**: Replaced fragile error-string substring matching for
  file-not-found detection with `errors.Is(err, os.ErrNotExist)`, which is
  locale-safe and works correctly through the Operations system error chain.

- **`AnalyzeDir`**: Parse errors are now surfaced as warnings in `result.ParseErrors`
  (printed to stderr by the CLI) instead of silently skipping files. Silent skips
  could produce a false-clean exit code 0 for files containing violations.

- **`isOperationsCall`**: Expanded comment to explicitly document the heuristic
  limitation (name-based, not type-based) and its security implications, so future
  maintainers understand the known bypass vector.

- **`detectReflect`**: Expanded comment to document the false-positive risk (any
  `.Call()` method in a file that imports `reflect`) and justify the design tradeoff
  (false positives preferable to false negatives for a security gate).

- **`contains()` helper removed**: Replaced the hand-rolled substring search in
  `manifest.go` with `strings.Contains` (from stdlib) at all call sites in test files.
  The custom implementation was correct but unnecessary.

## [1.0.0] - 2026-03-26

### Added

- Initial implementation of `ca-capability-analyzer` — a Go static analysis tool
  for enforcing capability manifest discipline (spec 13, Layer 3/4).

- **`DetectCapabilities`**: pure AST-based detection of raw OS capability usage.
  Accepts parsed `map[string]*ast.File` for in-memory testing (no filesystem needed).
  Detects 22 call-level rules and 3 import-level rules across 8 capability categories.

- **`DetectBanned`**: detection of 5 categories of unconditionally banned constructs:
  `unsafe.Pointer` conversions, `import "C"` (CGo), `plugin.Open`, `reflect.Value`
  dynamic dispatch (`Call`, `CallSlice`, `MethodByName`), and `//go:linkname` directives.

- **`LoadManifest`**: loads `required_capabilities.json` from a directory and returns
  the declared capability set as a map. Returns empty map (not error) when no manifest
  exists, which is the correct baseline for pure-computation packages.

- **`AnalyzeDir`**: full analysis pipeline — walk directory, parse `.go` files, run
  detectors, load manifest, cross-reference, produce `AnalysisResult` with violations.

- **`run()` + `main()`**: CLI entry point with `--dir` and `--verbose` flags.
  Exit codes: 0 (pass), 1 (violations), 2 (tool error).

- **Error codes**:
  - `CAP001`: undeclared capability. Fix: add the capability to `required_capabilities.json`
    and regenerate `gen_capabilities.go`.
  - `CAP002`: banned construct. Fix: remove the construct; no manifest entry can authorize it.

- **Exemptions**:
  - `gen_capabilities.go` files are entirely skipped.
  - Lines with `//nolint:cap` annotation are suppressed.
  - Calls via `op.File.*`, `op.Net.*`, `op.Time.*`, `cage.ReadFile`, etc. are exempt
    (already routed through the Operations system).

- **Capability rules** covering all 8 categories in the taxonomy:
  - `fs:read:*` — os.Open, os.ReadFile, os.OpenFile
  - `fs:write:*` — os.Create, os.WriteFile, os.Mkdir, os.MkdirAll, os.Rename
  - `fs:delete:*` — os.Remove, os.RemoveAll
  - `fs:list:*` — os.ReadDir, os.Stat, os.Lstat
  - `net:*:*` — import "net", import "net/http"
  - `proc:exec:*` — import "os/exec"
  - `env:read:*` — os.Getenv, os.Environ, os.LookupEnv
  - `time:read:*` — time.Now, time.Sleep
  - `stdout:write:*` — fmt.Print/Println/Printf, os.Stdout.Write, fmt.Fprintf(os.Stdout,...)
  - `stdin:read:*` — os.Stdin.Read

- **Special-case detection** for multi-level selector patterns:
  - `os.Stdout.Write(...)` and `os.Stderr.Write(...)`
  - `os.Stdin.Read(...)`
  - `fmt.Fprintf(os.Stdout/os.Stderr, ...)`

- **Aliased import support**: detects capabilities through renamed imports
  (`import myfmt "fmt"` + `myfmt.Println()` → `stdout:write:*`).

- **98.2% test coverage** via 70+ test cases. All individual rules, exemptions,
  banned constructs, manifest scenarios, and CLI exit codes tested.

- **Zero external dependencies**: stdlib only (`go/ast`, `go/parser`, `go/token`,
  `encoding/json`, `os`, `path/filepath`, `strings`, `fmt`, `flag`).

- **`required_capabilities.json`**: declares `fs:read:*` — the analyzer reads `.go`
  source files and `required_capabilities.json` manifests from analyzed packages.

- **`gen_capabilities.go`**: auto-generated by `capability-cage-generator`, providing
  the Operations system infrastructure for the analyzer's own filesystem reads.

- `BUILD` and `BUILD_windows`: `go test ./... -v -cover`
- `README.md`: usage guide, capability rule tables, banned construct reference
- `CHANGELOG.md`: this file
