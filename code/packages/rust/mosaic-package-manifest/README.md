# mosaic-package-manifest

Parser and validator for `mosaic-package.toml` manifests as defined in
**UI29 §4.1 / §4.2** (Mosaic Primitive Kernel — package manifest shape).

A `mosaic-package.toml` manifest answers exactly one question:

> *What does a Mosaic package declare to the kernel and to other packages?*

It answers with core sections plus optional package resources:

| Section          | What it carries                                                         |
|------------------|-------------------------------------------------------------------------|
| `[package]`      | name (kebab-case), strict SemVer version, free-text description, SPDX license |
| `[components]`   | `exports = [...]` — PascalCase component names this package publishes   |
| `[dependencies]` | map from kebab-case package name to a version requirement (see below)   |
| `[styles]`       | optional package-relative schema-v1 token palette                       |
| `[app]`          | optional paired initial desktop-window width and height                 |
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

[styles]
token_palette = "tokens/grid.json"

[app]
initial-window-width = 1280
initial-window-height = 900

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
| `package.version`        | strict SemVer 2.0.0, no build metadata (`Version::parse`)     |
| `package.description`    | non-empty string                                              |
| `package.license`        | non-empty string (SPDX id recommended)                        |
| `components.exports[]`   | `^[A-Z][a-zA-Z0-9]*$` (PascalCase)                            |
| `dependencies.<name>`    | name = kebab-case, value = `X.Y.Z` (caret) or `=X.Y.Z` (exact) |
| `styles.token_palette`    | optional portable package-relative path ending in `.json`      |
| `app.initial-window-*`    | optional paired integers from 1 through 2,147,483,647 logical pixels |
| `host_assets.files[]`    | optional `{ backend, source, target }` strings                 |
| `kernel.version`         | exactly `"1"` (kernel ABI v1)                                 |

## Version requirements

A `[dependencies]` value is a requirement the resolver checks against the
dependency's `[package].version` (mosaic-package-publishing-v1 §7.4). v1
has exactly two forms, both Cargo-compatible:

| Written     | Kind  | Means               | Matches          |
|-------------|-------|---------------------|------------------|
| `"1.2.3"`   | caret | `>=1.2.3, <2.0.0`   | 1.2.3, 1.9.0     |
| `"0.16.0"`  | caret | `>=0.16.0, <0.17.0` | 0.16.0, 0.16.3   |
| `"0.0.3"`   | caret | `>=0.0.3, <0.0.4`   | 0.0.3 only       |
| `"=0.16.0"` | exact | `=0.16.0`           | 0.16.0 only      |

`~`, `^`, `>=`, `<`, `*`, `1.x`, partial versions (`1.2`) and ranges are
refused at parse with `UnsupportedVersionRequirement`. A pre-release
(`1.0.0-rc.1`) matches only a requirement that names a pre-release of the
same `major.minor.patch`.

```rust
use mosaic_package_manifest::{Version, VersionReq};

let req = VersionReq::parse("0.16.0").unwrap();
assert!(req.matches(&Version::parse("0.16.3").unwrap()));
assert!(!req.matches(&Version::parse("0.17.0").unwrap()));
assert_eq!(req.range(), ">=0.16.0, <0.17.0");
```

`MosaicPackage::dependencies` still maps each name to the requirement
*text* as written; `VersionReq::parse` on a value from a parsed manifest
cannot fail.

## Error model

```rust
pub enum ManifestError {
    TomlSyntax(String),
    MissingField { section: String, field: String },
    InvalidPackageName(String),
    InvalidComponentName(String),
    InvalidKernelVersion(String),
    InvalidSemverString(String),
    UnsupportedVersionRequirement { dependency: String, requirement: String },
    InvalidStylePath(String),
    // … plus the [styles], [app] and [host_*] variants listed in lib.rs
}
```

The error model is deliberately *structural*, not interpretive:
the parser does not check that the package name is meaningful,
only that it is well-formed; it does not check that the license
is a real SPDX identifier, only that it is non-empty.  Semantic
validation (does this dependency exist? does the kernel ABI v1
actually support these primitives?) belongs to the kernel
loader, not to the manifest parser.  The one exception is a
dependency's version requirement: the parser accepts only the forms
the resolver can evaluate, so an unevaluable requirement fails here
instead of being silently ignored later.

## Public API

```rust
pub fn parse(toml_source: &str) -> Result<MosaicPackage, ManifestError>;
pub fn parse_path(path: &Path)   -> Result<MosaicPackage, ManifestError>;

pub struct Version { major, minor, patch, pre }   // Version::parse, Ord
pub enum VersionReq { Caret(Version), Exact(Version) }
impl VersionReq { fn parse(&str); fn matches(&Version); fn range() }
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
