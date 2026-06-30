# mosaic-package-manifest

Parser and validator for `mosaic-package.toml` manifests as defined in
**UI29 §4.1 / §4.2** (Mosaic Primitive Kernel — package manifest shape).

A `mosaic-package.toml` manifest answers exactly one question:

> *What does a Mosaic package declare to the kernel and to other packages?*

It answers with four sections:

| Section          | What it carries                                                         |
|------------------|-------------------------------------------------------------------------|
| `[package]`      | name (kebab-case), semver version, free-text description, SPDX license  |
| `[components]`   | `exports = [...]` — PascalCase component names this package publishes   |
| `[dependencies]` | map from kebab-case package name to semver string                       |
| `[host_assets]`  | optional per-backend files copied into generated project shells          |
| `[kernel]`       | `version = "1"` — primitive-kernel ABI this package targets             |

Nothing else is permitted at the top level; the parser does not error on
unknown sections (TOML is permissive), but it does require every field
listed above to be present and well-formed.

## Worked example

```toml
[package]
name = "mosaic-pkg-grid"
version = "0.1.0"
description = "Spreadsheet-style data grid built on UI29 kernel primitives"
license = "MIT OR Apache-2.0"

[components]
exports = ["Grid", "Cell", "Column"]

[dependencies]
# empty for this one

[host_assets]
files = [
  { backend = "react", source = "host/web/grid-host.ts", target = "src/grid-host.ts" },
]

[kernel]
version = "1"
```

```rust
use mosaic_package_manifest::parse;

let pkg = parse(SRC).expect("manifest invalid");
assert_eq!(pkg.package.name, "mosaic-pkg-grid");
assert_eq!(pkg.components.exports, ["Grid", "Cell", "Column"]);
assert_eq!(pkg.kernel.version, "1");
```

## Validation rules

| Field                    | Rule                                                         |
|--------------------------|--------------------------------------------------------------|
| `package.name`           | `^[a-z][a-z0-9]*(-[a-z][a-z0-9]*)*$` (kebab-case)            |
| `package.version`        | `^\d+\.\d+\.\d+(-[A-Za-z0-9.-]+)?$` (semver-like)             |
| `package.description`    | non-empty string                                              |
| `package.license`        | non-empty string (SPDX id recommended)                        |
| `components.exports[]`   | `^[A-Z][a-zA-Z0-9]*$` (PascalCase)                            |
| `dependencies.<name>`    | name = kebab-case, value = semver-like                        |
| `host_assets.files[]`    | optional `{ backend, source, target }` strings                 |
| `kernel.version`         | exactly `"1"` (kernel ABI v1)                                 |

## Error model

```rust
pub enum ManifestError {
    TomlSyntax(String),
    MissingField { section: String, field: String },
    InvalidPackageName(String),
    InvalidComponentName(String),
    InvalidKernelVersion(String),
    InvalidSemverString(String),
}
```

The error model is deliberately *structural*, not interpretive:
the parser does not check that the package name is meaningful,
only that it is well-formed; it does not check that the license
is a real SPDX identifier, only that it is non-empty.  Semantic
validation (does this dependency exist? does the kernel ABI v1
actually support these primitives?) belongs to the kernel
loader, not to the manifest parser.

## Public API

```rust
pub fn parse(toml_source: &str) -> Result<MosaicPackage, ManifestError>;
pub fn parse_path(path: &Path)   -> Result<MosaicPackage, ManifestError>;
```

## Where it fits in UI29

```
mosaic-package.toml          ← this crate parses these
       │  parse()
       ▼
MosaicPackage                 (validated manifest IR)
       │  consumed by
       ▼
mosaic primitive-kernel loader     (resolves dependencies, links components)
```

## Design principles

1. **Manifest = contract.** The manifest is what the kernel reads first;
   it must be trivially machine-parseable and never ambiguous.
2. **Structural validation only.** Reject things that are *shaped* wrong,
   not things that are *meant* wrong.
3. **One error per cause.** Each variant of `ManifestError` corresponds to
   exactly one kind of mistake the author made.
4. **No transitive deps.** This crate depends only on `serde`, `toml`,
   and `regex` — nothing in the Mosaic stack reaches into it for anything
   beyond loading manifests.
