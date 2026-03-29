# Swift Build Tool Support — Adding Swift to the Build System

## 1. Overview

### 1.1 The Problem

The build tool currently supports eight languages: Python, Ruby, Go, TypeScript,
Rust, Elixir, Lua, and Perl. As the monorepo begins to grow Swift packages, the
build system must be able to discover, resolve dependencies for, and build them.
Without this, Swift packages are invisible to the incremental build system —
they will never be rebuilt when changed, their transitive dependents will not
be flagged, and CI will silently skip them.

### 1.2 The Solution

Add `swift` as a first-class language to the Go build tool (the primary CI tool)
and its Python, Ruby, and Rust educational ports. This requires:

1. **Language detection** — recognise `swift` as a path component when inferring
   package language from directory path.
2. **Dependency resolution** — parse `Package.swift` to discover inter-package
   dependencies declared via `path:` references.
3. **BUILD command pattern** — document the correct `BUILD_mac_and_linux` content
   for Swift packages (Swift has no native Windows support).
4. **Skip-list entry** — add Swift Package Manager's build artefact directories
   to the discovery skip list so they are never mistaken for packages.

---

## 2. Platform Support

Swift is available on macOS, Linux, and Windows. The Windows toolchain ships
as a first-class download from swift.org and has been available since Swift 5.3.
All packages in this monorepo are pure-logic packages — no UIKit, AppKit, or
other Apple-framework imports. The same source code compiles and runs on all
three platforms without modification.

Swift packages use the standard cross-platform `BUILD` file. The same
`swift test` command works on macOS, Linux, and Windows.

---

## 3. Swift Package Structure

Every Swift package in this monorepo follows the Swift Package Manager (SPM)
layout. SPM is the official build tool shipped with Swift itself — no external
build system is needed.

```
code/packages/swift/my-package/
├── BUILD                     # build + test commands (cross-platform)
├── Package.swift             # SPM manifest (the metadata file)
├── README.md
├── CHANGELOG.md
├── required_capabilities.json
├── Sources/
│   └── MyPackage/
│       └── MyPackage.swift   # primary source file
└── Tests/
    └── MyPackageTests/
        └── MyPackageTests.swift
```

### 3.1 Package.swift Format

`Package.swift` is the authoritative manifest. It declares the package name,
products, dependencies, and targets.

```swift
// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "my-package",
    products: [
        .library(name: "MyPackage", targets: ["MyPackage"]),
    ],
    dependencies: [
        // Local monorepo dependencies use relative path references:
        .package(path: "../logic-gates"),
    ],
    targets: [
        .target(
            name: "MyPackage",
            dependencies: [
                .product(name: "LogicGates", package: "logic-gates"),
            ]
        ),
        .testTarget(
            name: "MyPackageTests",
            dependencies: ["MyPackage"]
        ),
    ]
)
```

Key conventions:
- `name:` in `Package(name:)` is the **kebab-case directory name**
  (e.g., `"my-package"`). This is what `.package(path: "../...")` references.
- Target names are **PascalCase** (e.g., `"MyPackage"`).
- Test target name is always `"{PascalCase}Tests"`.
- The `swift-tools-version` comment must be the very first line.

### 3.2 Naming Conventions

| Context | Form | Example for `my-package` |
|---------|------|--------------------------|
| Directory name | kebab-case | `my-package` |
| `Package(name:)` | kebab-case | `"my-package"` |
| Product name | PascalCase | `"MyPackage"` |
| Target name | PascalCase | `"MyPackage"` |
| Test target name | PascalCase + "Tests" | `"MyPackageTests"` |
| Source directory | `Sources/{PascalCase}/` | `Sources/MyPackage/` |
| Test directory | `Tests/{PascalCase}Tests/` | `Tests/MyPackageTests/` |
| Primary source file | `{PascalCase}.swift` | `MyPackage.swift` |

### 3.3 BUILD

```bash
swift test --enable-code-coverage --verbose
```

A single command. SPM handles fetching dependencies (via `Package.swift`),
compiling, and running the test suite. The `--enable-code-coverage` flag emits
`.profraw` coverage data; coverage reports can be generated post-hoc with
`llvm-cov`.

This command works identically on macOS, Linux, and Windows. For packages
without local monorepo dependencies, this is the complete `BUILD` content.
SPM resolves external dependencies automatically via its `Package.resolved`
lockfile mechanism.

---

## 4. Language Detection

### 4.1 Path-Based Inference

The build tool infers a package's language from its directory path by scanning
path components for known language names. Swift packages live under
`code/packages/swift/` or `code/programs/swift/`.

Add `swift` to the known language component table:

| Path component | Language |
|----------------|----------|
| `python`       | python   |
| `ruby`         | ruby     |
| `go`           | go       |
| `rust`         | rust     |
| `typescript`   | typescript |
| `elixir`       | elixir   |
| `lua`          | lua      |
| `perl`         | perl     |
| **`swift`**    | **swift** |

### 4.2 allLanguages List

The `allLanguages` slice (used for `--language all` filtering) must include
`"swift"`:

```go
// Before
var allLanguages = []string{"python", "ruby", "go", "typescript", "rust", "elixir", "lua", "perl"}

// After
var allLanguages = []string{"python", "ruby", "go", "typescript", "rust", "elixir", "lua", "perl", "swift"}
```

The equivalent lists in the Python, Ruby, and Rust build tool implementations
must be updated identically.

---

## 5. Dependency Resolution

### 5.1 Metadata File

| Language | Metadata file  | Dependency indicator |
|----------|----------------|----------------------|
| Python   | `pyproject.toml` | `coding-adventures-` prefix |
| Go       | `go.mod`         | `=> ../` replace directive |
| Ruby     | `*.gemspec`      | `coding_adventures_` prefix |
| TypeScript | `package.json` | `file:` prefix |
| Rust     | `Cargo.toml`     | `path =` entry |
| Elixir   | `mix.exs`        | `path:` option |
| Perl     | `cpanfile`       | `coding-adventures-` prefix |
| **Swift** | **`Package.swift`** | **`.package(path: "../")` call** |

### 5.2 Parsing Algorithm

Swift local dependencies are declared with `.package(path: "../dep-name")` in
the top-level `Package.swift`. The resolver scans each line of `Package.swift`
for this pattern and extracts the directory name.

```
function read_swift_deps(package_dir):
    path = package_dir + "/Package.swift"
    if path does not exist: return []

    deps = []
    for each line in read_file(path):
        match = line ~ /\.package\s*\(\s*path\s*:\s*"\.\.\/([^"]+)"/
        if match:
            deps.append(match.group(1))   # e.g., "logic-gates"
    return deps
```

This is intentionally line-oriented rather than a full Swift AST parse.
`Package.swift` files in this monorepo are always generated by the scaffold
generator and follow a consistent, single-line-per-dependency format. A line
regex is sufficient and avoids any dependency on a Swift parser.

### 5.3 Dependency Format in Package.swift

The build tool resolver maps the extracted directory name back to the qualified
package name using the standard `{language}/{dirname}` convention:

```
".package(path: \"../logic-gates\")"  →  dep name "logic-gates"
                                      →  qualified: "swift/logic-gates"
```

### 5.4 Go Implementation Location

The resolver logic lives in:

```
code/programs/go/build-tool/internal/resolver/resolver.go
```

Add a `readSwiftDeps(pkgDir string) ([]string, error)` function following the
same pattern as `readElixirDeps`, `readPerlDeps`, etc., and wire it into the
`readDeps` dispatch with `case "swift":`.

---

## 6. Discovery Skip List

SPM places its build artefacts and dependency checkouts in a `.build/` directory
inside the package. Add `.build` to the discovery skip list so the build tool
does not traverse into it.

```
Current skip list (relevant entries):
  target/       # Rust build output
  dist/         # TypeScript build output
  .venv/        # Python virtualenv
  node_modules/ # Node.js deps

Add:
  .build/       # Swift Package Manager build artefacts and checkouts
```

The `.build/` directory is created by SPM the first time `swift build` or
`swift test` is run. It contains compiled object files, cached dependency
source trees, and intermediate products. None of these should be treated as
packages.

---

## 7. Build System Spec Update

The main build system spec (`12-build-system.md`) contains language tables that
must be updated to include Swift:

### 7.1 Language Inference Table

Add the `swift` row to the "Language Inference" table:

| Path component | Language  |
|----------------|-----------|
| `swift`        | swift     |

### 7.2 Dependency Resolution Table

Add the Swift row:

| Language | Metadata file   | Dependency prefix / pattern          |
|----------|-----------------|--------------------------------------|
| Swift    | `Package.swift` | `.package(path: "../")` call         |

### 7.3 Implementations Table

The Go build tool is the only implementation that needs Swift support for CI.
The Python, Ruby, and Rust ports are educational and should mirror the change
for consistency, but they are not on the critical path.

---

## 8. Files Modified

### 8.1 Go Build Tool (Primary)

| File | Change |
|------|--------|
| `code/programs/go/build-tool/main.go` | Add `"swift"` to `allLanguages` |
| `code/programs/go/build-tool/internal/resolver/resolver.go` | Add `readSwiftDeps()` and `case "swift":` in dispatch |
| `code/programs/go/build-tool/internal/discovery/discovery.go` | Add `".build"` to skip list |

### 8.2 Other Build Tool Implementations

| Implementation | File | Change |
|----------------|------|--------|
| Python | `code/programs/python/build-tool/` | Add `"swift"` to language list; add `read_swift_deps()` |
| Ruby | `code/programs/ruby/build-tool/` | Same |
| Rust | `code/programs/rust/build-tool/` | Same |

### 8.3 Spec

| File | Change |
|------|--------|
| `code/specs/12-build-system.md` | Update language inference table, dependency resolution table |

---

## 9. Test Strategy

### 9.1 Unit Tests for Dependency Resolver

| # | Test | Input | Expected |
|---|------|-------|----------|
| 1 | No Package.swift | Directory without Package.swift | Empty dep list |
| 2 | No local deps | Package.swift with no `.package(path:)` lines | Empty dep list |
| 3 | Single local dep | `.package(path: "../logic-gates")` | `["logic-gates"]` |
| 4 | Multiple local deps | Two `.package(path:)` lines | Both dep names returned |
| 5 | External dep ignored | `.package(url: "https://...")` | Empty (not a local dep) |
| 6 | Whitespace variants | `path :  "../logic-gates"` (extra spaces) | `["logic-gates"]` |

### 9.2 Discovery Tests

| # | Test | Setup | Expected |
|---|------|-------|----------|
| 7 | Swift package discovered | Directory with `Package.swift` + `BUILD_mac_and_linux` | Package found |
| 8 | `.build` directory skipped | `.build/` inside Swift package | Not traversed |
| 9 | Language inferred correctly | Path `code/packages/swift/my-pkg` | Language = `swift` |

### 9.3 Integration Tests

| # | Test | Setup | Expected |
|---|------|-------|----------|
| 10 | Swift in `--language all` | Packages spanning multiple languages | Swift packages included |
| 11 | Swift in `--language swift` | Mixed-language repo | Only Swift packages returned |
| 12 | Dep ordering respected | Swift package B depends on Swift package A | A built before B |

---

## 10. Trade-offs

### 10.1 Line Regex vs Full Swift Parser

| | Line regex | Full AST parse |
|-|------------|----------------|
| Dependencies | None | Requires Swift toolchain or third-party parser |
| Correctness | Sufficient for scaffold-generated files | Handles arbitrary Swift expressions |
| Fragility | Breaks on multi-line `.package(path:)` | Robust |
| **Decision** | **Line regex** | — |

The scaffold generator (specified separately) always emits `.package(path:)` on
a single line. The line regex is sufficient for all scaffold-generated packages.
If a package hand-edits `Package.swift` and splits the call across lines, the
resolver will silently miss the dependency — the build will still work, but
incremental rebuilds may be missed. This is the same trade-off made for Python,
Ruby, Elixir, and Perl.

### 10.2 BUILD vs Platform-Specific Files

All packages in this monorepo are pure-logic packages. `swift test
--enable-code-coverage --verbose` behaves identically on macOS, Linux, and
Windows. A single cross-platform `BUILD` file is the only file needed.
Platform-specific `BUILD_mac` / `BUILD_linux` / `BUILD_windows` variants are
out of scope for the Swift packages in this repo.

---

## 11. Future Extensions

- **Coverage reporting:** Generate an HTML coverage report using `llvm-cov` and
  the `.profraw` files produced by `--enable-code-coverage`. This would mirror
  what Elixir's `mix test --cover` does.
- **Swift linting:** Once `swift-format` is widely available as a CI tool,
  add a lint step to `BUILD_mac_and_linux`.
- **Windows CI:** Add the Windows Swift toolchain to CI runners so that Swift
  packages are verified on all three platforms on every PR.
- **Swift scaffold generator:** A Swift-language implementation of the scaffold
  generator itself, at `code/programs/swift/scaffold-generator/`.
