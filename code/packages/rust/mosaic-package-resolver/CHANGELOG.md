# Changelog

## Unreleased

### Added
- Registered `HostSurface` as a kernel primitive so package resolution
  preserves typed host-owned `node` mount points instead of treating them as
  missing userland component dependencies.
- Added `LayoutPackageResolver`, a shared `pkg::P::C` layout inliner that
  compiles referenced component layouts, rewrites slot/event bindings, detects
  package-reference cycles, and rejects source symlink escapes.
- Added `first_qualified_tag()` for post-resolution sanity checks.

All notable changes to this crate are documented here.  The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the crate follows
SemVer.

## [0.1.0] — 2026-05-19

### Added
- Initial release implementing **UI29-R2** (component-reference resolver).
- `KERNEL_PRIMITIVES` constant listing the UI29 §2.1 kernel set
  (`Box`, `Row`, `Column`, `Stack`, `Text`, `Image`, `Spacer`, `Divider`,
  `Icon`, `If`, `Else`, `For`, `HostInput`, `HostButton`, `HostTable`,
  `HostScroll`).
- `Resolver` type with `resolve(tag) -> Option<&Resolution>` and
  `knows(tag) -> bool`.
- `Resolution::{Kernel, Component}` enum.
- `build(package_root, search_paths)` builder.  Reads the user's
  `mosaic-package.toml` via `mosaic-package-manifest`, walks
  `[dependencies]`, locates each dep in the search paths (tries
  `mosaic-pkg-{name}` then literal `{name}`), reads its manifest, and
  registers each `[components].exports` entry into the resolution table.
- `ResolveError::{DependencyNotFound, BadDependencyManifest,
  DuplicateExport, Io}` for build-time failures.
- 12 unit tests covering empty packages, deps with one or many exports,
  collisions, missing deps, malformed dep manifests, kernel coverage,
  and `package_path` absoluteness.
