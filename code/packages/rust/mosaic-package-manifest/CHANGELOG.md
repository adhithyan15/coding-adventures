# Changelog — mosaic-package-manifest

## [Unreleased]

### Added

- Added optional `[host_assets]` support with `files = [{ backend, source, target }]`
  declarations so app packages can describe backend-specific host adapter files
  inside `mosaic-package.toml`.

## [0.1.0] — 2026-05-19

### Added

- Initial implementation of the `mosaic-package.toml` manifest parser per
  UI29 §4.1 / §4.2.
- `parse(&str) -> Result<MosaicPackage, ManifestError>` — parses a TOML
  manifest from a string and validates every field.
- `parse_path(&Path) -> Result<MosaicPackage, ManifestError>` — reads a
  manifest file from the filesystem and parses it.
- `MosaicPackage` IR — typed representation of the four required sections:
  `package`, `components`, `dependencies`, `kernel`.
- `PackageMeta` — name + version + description + license.
- `ComponentsSection` — `exports: Vec<String>` (PascalCase component names).
- `KernelSection` — `version: String` (currently only `"1"` accepted).
- `ManifestError` — six structured error kinds: `TomlSyntax`,
  `MissingField`, `InvalidPackageName`, `InvalidComponentName`,
  `InvalidKernelVersion`, `InvalidSemverString`.
- Validation regexes:
  - kebab-case package names: `^[a-z][a-z0-9]*(-[a-z][a-z0-9]*)*$`
  - semver-like versions: `^\d+\.\d+\.\d+(-[A-Za-z0-9.-]+)?$`
  - PascalCase component names: `^[A-Z][a-zA-Z0-9]*$`
- 13 unit tests covering the happy-path manifest, every error variant,
  empty exports, dependency parsing, kernel version rejection, TOML
  syntax error reporting, and `parse_path` filesystem read.
- 1 doctest for the `parse()` entry point.
