# Scaffold Generator Swift Support — Adding Swift Templates

## 1. Overview

### 1.1 The Problem

The scaffold generator produces correct-by-construction package scaffolding for
Python, Go, Ruby, TypeScript, Rust, Elixir, Lua, and Perl. As the monorepo
begins accumulating Swift packages, the same hand-crafting failure modes that
motivated the scaffold generator in other languages will apply to Swift:

| Risk | How the scaffold generator prevents it |
|------|-----------------------------------------|
| Missing `BUILD` | Always generated with correct `swift test` command |
| `BUILD` missing transitive dep paths | Computes full transitive closure, passes all `--package-path` env context via SPM |
| Wrong `Package.swift` structure | Template always correct — right `swift-tools-version`, products, targets layout |
| Missing test target in `Package.swift` | Template always includes `.testTarget` |
| Missing `1;`-equivalent (Swift: missing `public` on API) | Template always generates `public` visibility |
| Wrong directory name (camelCase instead of kebab-case) | Name normalisation enforces kebab-case directory |
| Missing README.md or CHANGELOG.md | Always generated |
| `.package(path:)` pointing to wrong relative path | Path derived from standardised `../dep-name` convention |

### 1.2 The Solution

Add a `generateSwift()` function to all scaffold generator implementations. The
generated output is a complete, `swift test`-passing package with a
`Package.swift`, source stub, test stub, `BUILD`, `README.md`,
`CHANGELOG.md`, and `required_capabilities.json`.

The implementations to update are:

| Implementation | Location | Language |
|----------------|----------|----------|
| Go (primary)  | `code/programs/go/scaffold-generator/main.go` | Go |
| Python        | `code/programs/python/scaffold-generator/` | Python |
| Ruby          | `code/programs/ruby/scaffold-generator/` | Ruby |
| TypeScript    | `code/programs/typescript/scaffold-generator/` | TypeScript |
| Rust          | `code/programs/rust/scaffold-generator/` | Rust |
| Elixir        | `code/programs/elixir/scaffold-generator/` | Elixir |
| Lua           | `code/programs/lua/scaffold-generator/` | Lua |

Additionally, `"swift"` must be added to the valid language list in the CLI
spec (`code/programs/scaffold-generator.json`) and in each implementation's
validation logic.

---

## 2. Where It Fits

```
scaffold-generator my-package --language swift --depends-on logic-gates
    |
    v
code/packages/swift/my-package/
    ├── BUILD
    ├── Package.swift
    ├── CHANGELOG.md
    ├── README.md
    ├── required_capabilities.json
    ├── Sources/
    │   └── MyPackage/
    │       └── MyPackage.swift
    └── Tests/
        └── MyPackageTests/
            └── MyPackageTests.swift
```

7 files total. The scaffold generator reads existing Swift packages to discover
transitive dependencies, then writes all 7 files with correct content. This
spec defines what each file looks like.

---

## 3. Name Normalization

### 3.1 Conversion Rules

The input `PACKAGE_NAME` is always kebab-case (e.g., `my-package`). Swift needs
two derived forms:

| Form | Function | Example for `my-package` |
|------|----------|--------------------------|
| Kebab (original) | Identity | `my-package` |
| PascalCase | `to_camel_case()` | `MyPackage` |

The existing `toCamelCase()` / `to_camel_case()` function in each
implementation already handles this conversion — no new function is required.

### 3.2 Swift-Specific Names

Given the input `my-package`:

| Context | Value |
|---------|-------|
| **Directory name** | `my-package` (kebab-case) |
| **`Package(name:)`** | `"my-package"` (kebab-case) |
| **Library product name** | `"MyPackage"` (PascalCase) |
| **Target name** | `"MyPackage"` (PascalCase) |
| **Test target name** | `"MyPackageTests"` (PascalCase + "Tests") |
| **Source directory** | `Sources/MyPackage/` |
| **Test directory** | `Tests/MyPackageTests/` |
| **Primary source file** | `Sources/MyPackage/MyPackage.swift` |
| **Primary test file** | `Tests/MyPackageTests/MyPackageTests.swift` |
| **Import statement** | `import MyPackage` |

### 3.3 Comparison with Other Languages

| Context | Python | Ruby | Go | Swift |
|---------|--------|------|----|-------|
| Dir name | `my-package` | `my_package` | `my-package` | `my-package` |
| Module/package name | `my_package` | `CodingAdventures::MyPackage` | `mypackage` | `MyPackage` |
| Source dir | `src/my_package/` | `lib/coding_adventures/my_package/` | (flat) | `Sources/MyPackage/` |
| Import | `from my_package import ...` | `require "coding_adventures_my_package"` | `import mypackage` | `import MyPackage` |
| Namespace prefix | `coding-adventures-` | `CodingAdventures::` | (module path) | (none — module name is sufficient) |

Swift does not use a namespace prefix like `CodingAdventures`. SPM packages
are identified by their directory name (`my-package`) and their target names
(`MyPackage`) are unique within a package. Two packages can have identically
named targets; SPM disambiguates via the `package:` label in `.product(name:
package:)` calls.

### 3.4 Dependency Name Normalization

When `--depends-on logic-gates` is specified:

| Context | How the dependency appears |
|---------|--------------------------|
| `Package.swift` dependencies array | `.package(path: "../logic-gates")` |
| `Package.swift` target dependencies | `.product(name: "LogicGates", package: "logic-gates")` |
| `BUILD` | No explicit install step needed — SPM resolves path deps automatically |
| Source code import | `import LogicGates` |

The product name (`"LogicGates"`) is derived by `to_camel_case("logic-gates")`.
The package label (`"logic-gates"`) is the original kebab-case directory name.

---

## 4. Generated Files

### 4.1 Package.swift

```swift
// swift-tools-version: 5.9
// ============================================================================
// Package.swift — <description>
// ============================================================================
//
// This is the Swift Package Manager manifest for <PascalCase>.
// It is part of the coding-adventures project, an educational computing stack.
//
// Dependencies on other packages in this monorepo are declared via relative
// path references so that SPM resolves them from the local filesystem instead
// of fetching them from a remote registry.
//
import PackageDescription

let package = Package(
    name: "<kebab>",
    products: [
        // The library product exposes <PascalCase> to other packages.
        .library(name: "<PascalCase>", targets: ["<PascalCase>"]),
    ],
    dependencies: [
        <for each direct dep:>
        .package(path: "../<dep-kebab>"),
    ],
    targets: [
        .target(
            name: "<PascalCase>",
            dependencies: [
                <for each direct dep:>
                .product(name: "<DepPascalCase>", package: "<dep-kebab>"),
            ]
        ),
        .testTarget(
            name: "<PascalCase>Tests",
            dependencies: ["<PascalCase>"]
        ),
    ]
)
```

**Template variables:**
- `<kebab>`: the original kebab-case package name
- `<PascalCase>`: `to_camel_case(package_name)`
- `<DepPascalCase>`: `to_camel_case(dep_name)` for each direct dependency
- `<dep-kebab>`: the dependency's original kebab-case name
- `<description>`: from `--description` flag, or `"A coding-adventures package"`

When there are no dependencies, the `dependencies:` array and the inner
`dependencies:` in `.target` are omitted entirely (empty arrays are valid but
noisy).

**No-dep example for `arithmetic`:**

```swift
// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "arithmetic",
    products: [
        .library(name: "Arithmetic", targets: ["Arithmetic"]),
    ],
    targets: [
        .target(name: "Arithmetic"),
        .testTarget(
            name: "ArithmeticTests",
            dependencies: ["Arithmetic"]
        ),
    ]
)
```

**With-dep example for `cpu-simulator` depending on `arithmetic`:**

```swift
// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "cpu-simulator",
    products: [
        .library(name: "CpuSimulator", targets: ["CpuSimulator"]),
    ],
    dependencies: [
        .package(path: "../arithmetic"),
    ],
    targets: [
        .target(
            name: "CpuSimulator",
            dependencies: [
                .product(name: "Arithmetic", package: "arithmetic"),
            ]
        ),
        .testTarget(
            name: "CpuSimulatorTests",
            dependencies: ["CpuSimulator"]
        ),
    ]
)
```

### 4.2 Sources/`<PascalCase>`/`<PascalCase>`.swift

```swift
// <PascalCase>.swift
// Part of coding-adventures — an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// <PascalCase> — <description>
// ============================================================================
//
<if layer:>
// Layer <layer> in the computing stack.
//
<end if>
// Usage:
//
//   import <PascalCase>
//
// ============================================================================

<for each direct dep:>
import <DepPascalCase>

/// <PascalCase> is the primary type exported by this module.
///
/// TODO: Replace this stub with the real implementation.
public struct <PascalCase> {
    /// Creates a new <PascalCase> instance.
    public init() {}
}
```

**Key points:**
- The top-level type is a `public struct` named `<PascalCase>`. Implementers
  replace this with the real types.
- `public` visibility is required — without it the type is invisible to other
  packages that `import` this module.
- Import statements for direct dependencies appear at the top, after the
  header comment.
- Literate-style header comment explains purpose and layer position.

### 4.3 Tests/`<PascalCase>`Tests/`<PascalCase>`Tests.swift

```swift
import XCTest
@testable import <PascalCase>

/// <PascalCase>Tests — unit tests for the <PascalCase> module.
///
/// These are stub tests generated by the scaffold generator.
/// Replace them with real tests that exercise the actual implementation.
final class <PascalCase>Tests: XCTestCase {

    /// Verifies the module loads and its primary type can be instantiated.
    func testModuleLoads() {
        // This test exists to confirm that the module compiles and its
        // public API is accessible. Replace it with meaningful tests.
        let _ = <PascalCase>()
        XCTAssertTrue(true, "<PascalCase> instantiated successfully")
    }
}
```

**Key points:**
- `@testable import` gives test code access to `internal` symbols in addition
  to `public` ones.
- `XCTestCase` is the built-in Swift test base class — no third-party framework
  needed.
- The stub test instantiates the primary type, catching any compilation or
  initialiser errors immediately.
- `final class` is idiomatic for `XCTestCase` subclasses (they are not meant
  to be further subclassed).

### 4.4 BUILD

**Without dependencies:**

```bash
swift test --enable-code-coverage --verbose
```

**With dependencies (direct or transitive):**

```bash
swift test --enable-code-coverage --verbose
```

**The BUILD file is identical regardless of dependency count.** SPM resolves
local path dependencies automatically by reading `Package.swift` — there is no
equivalent of `bundle install` or `uv pip install` needed. SPM finds
`../logic-gates/Package.swift`, compiles it, and links the product. No
explicit install step is required in the BUILD file.

This is a deliberate simplification over Python, Ruby, and Elixir, where the
BUILD file must list transitive install commands in topological order. SPM's
dependency resolver handles this entirely.

### 4.5 README.md

```markdown
# <PascalCase>

<description>

<if layer:>
**Layer <layer>** in the coding-adventures computing stack.
<end if>

## Installation

Add this package to your `Package.swift` dependencies:

```swift
.package(path: "../<kebab>"),
```

Then add `"<PascalCase>"` to your target's dependencies.

## Usage

```swift
import <PascalCase>

let instance = <PascalCase>()
```

## Testing

```bash
swift test --verbose
```

## Dependencies

<if deps:>
<for each direct dep:>
- [`<DepPascalCase>`](../<dep-kebab>/)
<end for>
<else>
None.
<end if>

## License

MIT
```

### 4.6 CHANGELOG.md

```markdown
# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - <today's date>

### Added

- Initial package scaffolding.
- `<PascalCase>` struct with public initialiser stub.
- `<PascalCase>Tests` test target with load verification test.
```

### 4.7 required_capabilities.json

```json
{
  "filesystem": false,
  "network": false,
  "process": false
}
```

This is the standard minimal capabilities declaration used by all packages in
the monorepo. It declares that the package does not require any elevated
capabilities. Packages that need filesystem access, network access, or process
spawning must update this file.

---

## 5. Dependency Resolution

### 5.1 How It Works

The scaffold generator computes the **transitive closure** of dependencies to
verify that all referenced packages exist at generation time. However, unlike
Python, Ruby, or Elixir, **Swift packages do not need their transitive deps
listed explicitly in `BUILD`** — SPM walks the `Package.swift`
dependency graph automatically.

The scaffold generator still reads existing Swift packages' `Package.swift`
files to:
1. Validate that `--depends-on` targets exist in `code/packages/swift/`.
2. Surface a clear error if a requested dependency does not exist yet.

```
function compute_swift_deps(direct_deps):
    for each dep in direct_deps:
        pkg_path = "code/packages/swift/" + dep
        if pkg_path does not exist:
            error("Swift package '{}' does not exist. Create it first.", dep)
    # No BUILD install ordering needed — SPM handles resolution.
    return direct_deps
```

### 5.2 Reading Swift Dependencies

To validate transitive deps and support future `--language all` dependency
cross-referencing, the scaffold generator must be able to read existing Swift
packages' direct dependencies from their `Package.swift`:

```
function read_swift_deps(package_dir):
    path = package_dir + "/Package.swift"
    if path does not exist: return []

    deps = []
    for each line in read_file(path):
        match = line ~ /\.package\s*\(\s*path\s*:\s*"\.\.\/([^"]+)"/
        if match:
            deps.append(match.group(1))
    return deps
```

This mirrors the pattern used in the build tool resolver (see
`swift-build-support.md`).

---

## 6. CLI Spec Changes

### 6.1 scaffold-generator.json

The `language` flag's description and valid value set must be updated to include
Swift:

```json
{
    "id": "language",
    "description": "Target language(s). Comma-separated or 'all' for all 9 languages: python, go, ruby, typescript, rust, elixir, lua, perl, swift",
    ...
}
```

Valid values become: `python`, `ruby`, `go`, `typescript`, `rust`, `elixir`,
`lua`, `perl`, `swift`, `all`.

### 6.2 Validation

When `--language swift` or `--language all` is used:
- The target directory `code/packages/swift/<name>/` must not already exist.
- If `--depends-on` is specified, each dependency must exist at
  `code/packages/swift/<dep-kebab>/`.
- The generated `BUILD` file is cross-platform (macOS, Linux, Windows).

---

## 7. Files Modified

### 7.1 Go (Primary Implementation)

| File | Change |
|------|--------|
| `code/programs/go/scaffold-generator/main.go` | Add `"swift"` to `validLanguages`; add `case "swift":` in language dispatch; add `generateSwift()` (~100 lines); add `readSwiftDeps()` |
| `code/programs/scaffold-generator.json` | Update `language` flag description and valid value count |

### 7.2 All Other Implementations

| Implementation | Key File | Function to Add |
|----------------|----------|-----------------|
| Python | scaffold generator source | `generate_swift()` |
| Ruby | scaffold generator source | `generate_swift()` |
| TypeScript | scaffold generator source | `generateSwift()` |
| Rust | scaffold generator source | `generate_swift()` |
| Elixir | scaffold generator source | `generate_swift()` |
| Lua | `scaffold.lua` | `generate_swift()` |

---

## 8. Test Strategy

### 8.1 Basic Generation (~8 cases)

| # | Test | Input | Expected |
|---|------|-------|----------|
| 1 | Generate Swift library, no deps | `--language swift my-pkg` | 7 files created |
| 2 | Correct directory location | Library mode | `code/packages/swift/my-pkg/` |
| 3 | Correct Package.swift, no deps | `my-pkg` | No `dependencies:` array, correct name/target |
| 4 | Correct source file path | `my-pkg` | `Sources/MyPkg/MyPkg.swift` |
| 5 | Correct test file path | `my-pkg` | `Tests/MyPkgTests/MyPkgTests.swift` |
| 6 | Source file has public struct | Any | `public struct MyPkg` |
| 7 | Test file imports @testable | Any | `@testable import MyPkg` |
| 8 | CHANGELOG has today's date | Any | `[0.1.0] - <today>` |

### 8.2 Dependencies (~5 cases)

| # | Test | Input | Expected |
|---|------|-------|----------|
| 9 | Single dep in Package.swift | `--depends-on logic-gates` | `.package(path: "../logic-gates")` in deps array |
| 10 | Dep product in target | `--depends-on logic-gates` | `.product(name: "LogicGates", package: "logic-gates")` in target deps |
| 11 | Import in source file | `--depends-on logic-gates` | `import LogicGates` in source stub |
| 12 | Multiple direct deps | `--depends-on bitset,matrix` | Both in Package.swift deps array and target deps |
| 13 | BUILD unchanged with deps | Any | `BUILD` content is always `swift test --enable-code-coverage --verbose` |

### 8.3 Name Normalization (~5 cases)

| # | Test | Input | Expected |
|---|------|-------|----------|
| 14 | Single word | `bitset` | Target: `Bitset`, test target: `BitsetTests` |
| 15 | Two words | `logic-gates` | Target: `LogicGates`, test target: `LogicGatesTests` |
| 16 | Three words | `cpu-simulator` | Target: `CpuSimulator` |
| 17 | With numbers | `sha256` | Target: `Sha256` |
| 18 | Long name | `bytecode-compiler` | Target: `BytecodeCompiler` |

### 8.4 Edge Cases (~4 cases)

| # | Test | Input | Expected |
|---|------|-------|----------|
| 19 | Dry-run mode | `--dry-run` | No files written; output shows tree of would-be files |
| 20 | Program type | `--type program` | Output in `code/programs/swift/` |
| 21 | `--language all` includes Swift | `--language all my-pkg` | Swift directory among outputs |
| 22 | Missing dep error | `--depends-on nonexistent-pkg` | Clear error: package does not exist |

**Total: ~22 test cases per implementation, ~154 across all 7.**

---

## 9. Trade-offs

### 9.1 No Explicit Transitive Install in BUILD

| | Explicit transitive install (Python/Ruby style) | SPM auto-resolution (Swift) |
|-|-------------------------------------------------|------------------------------|
| BUILD file complexity | High — must list all deps in topological order | Low — single `swift test` line |
| Failure mode | Missing dep → CI failure | SPM resolves automatically |
| Build tool resolver | Must compute install order | Only needs existence validation |
| **Decision** | — | **SPM auto-resolution** |

SPM resolves local path dependencies by walking the `Package.swift` chain.
There is no need to enumerate transitive deps in `BUILD`. This is
the most significant structural difference from other language scaffolds.

### 9.2 XCTest vs Swift Testing Framework

| | XCTest | Swift Testing (new in Swift 5.9+) |
|-|--------|-----------------------------------|
| Availability | All Swift versions; built-in | Swift 5.9+; still evolving |
| SPM integration | `testTarget` directly | Also supported via `testTarget` |
| Syntax | `XCTestCase` subclass | `@Test` macro-based |
| Toolchain support | Universal | Requires Xcode 15+ / Swift 5.9+ |
| **Decision** | **XCTest** | — |

XCTest is universally supported and stable. The scaffold template uses XCTest.
Packages can migrate to the Swift Testing framework independently once they
require Swift 5.9+ minimum.

### 9.3 swift-tools-version: 5.9

| | 5.9 | 5.7 | 5.5 |
|-|-----|-----|-----|
| Availability | macOS 14 / Ubuntu 22.04+ | Broader | Very broad |
| Macro support | Yes | No | No |
| Concurrency | Full | Partial | Partial |
| **Decision** | **5.9** | — | — |

Swift 5.9 is the current stable toolchain available on macOS 14 (Sonoma),
Ubuntu 22.04+, and Windows (via swift.org installer). All platforms this repo
targets have Swift 5.9 toolchains available. There is no reason to target an
older tools version.

---

## 10. Future Extensions

- **Swift linting:** Add `swift-format --lint .` to `BUILD`
  once `swift-format` is standardised across CI.
- **Coverage threshold enforcement:** Parse `llvm-cov report` output and fail
  the build if line coverage drops below 80%.
- **Swift scaffold generator implementation:** A Swift-language port of the
  scaffold generator at `code/programs/swift/scaffold-generator/`, consistent
  with the pattern of each language having its own scaffold generator port.
- **`@testable` migration:** Some packages may choose to make all APIs `public`
  and drop `@testable` for a stricter black-box test approach.
- **Package.swift v6 manifest:** When `swift-tools-version: 6.0` (strict
  concurrency) becomes standard, update the template accordingly.
