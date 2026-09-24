# Changelog

## Unreleased

### Added — a component can be mounted more than once (UI34 §5 step 4)

- The resolver counts mounts per `(package, component)` across the whole
  resolved tree, including mounts nested inside other packages.
  - The **first** mount keeps its authored part names, so single-mount
    output is unchanged.
  - The n-th mount (n ≥ 2) gets every inlined part suffixed `-m<n>`
    (`empty-state` → `empty-state-m2`). It is not `-2`, because a Mosaic
    identifier segment cannot start with a digit, and a consumer `.msl`
    must be able to name the part.
  - A root name the caller wrote and children the caller splices in are
    the caller's own parts, and are never suffixed.
- Before this, a second mount always failed with `DuplicatePart`.
- New `LayoutPackageResolver::resolve_with_renames` returns the renames
  (`PartRename { package, component, from, to }`). `resolve` keeps its
  signature.

### Added — `HostNavigationSplit` in `KERNEL_PRIMITIVES` (UI29-6, #15481)

The 35th kernel primitive: the adaptive navigation container UI48 §5.4 asked
for. A pane beside a detail area composes from two `Column`s — four layouts do
exactly that today — but the platform's own collapse and the pane landmark do
not compose, which is the UI29 §2.2 argument the spec makes in full.

### Fixed — a literal text or number bound into a gated slot is folded too

`fold_constant_conditionals` handled only `true`/`false`. A component that
gates on a text slot (`If ( when: slot: action-label )`) and receives a
literal (`action-label : ""`) was left with a string `when:`, which the
validator rejects. Text now folds as true when non-empty, and numbers as
true when non-zero, the same truthiness every emitter's runtime helper uses.
Found adopting EmptyState in TaskApp (#15440).

### Fixed — a literal bool bound into a component's `If` failed to compile

A component that branches on a bool slot (`If ( when: slot: vertical )`),
used with a literal (`vertical : false`), was left with `when: false` after
binding. The layout validator accepts only a slot or an expression there, so
the consuming app failed with "`If` prop `when:` must be a slot reference or
expression". Found adopting SegmentedControl in Engram (#14063, #15432).

`fold_constant_conditionals` now makes the choice while inlining: the branch
that applies replaces the `If`/`Else` pair, and the other is dropped. This is
safe because `If`/`Else` are not containers. An `If` at the component root is
folded only into a single node.

Rewriting the keyword as an expression was rejected: the static-HTML runtime
reads a bare `true` as a data path, which resolves to undefined and would
invert the branch.

Tests: both literals select the right branch; a slot binding stays a runtime
`If`/`Else`; expression and keyword forms fold; non-literal conditions are
left alone. The first attempt folded a nested `If` before its parent could
pair it with its `Else`. The test caught it, and the root-only rule now
lives in its own step.


- Remove unforwarded dependency event bindings at the composition boundary.
  Omitted child events no longer leak undeclared names into the parent model.

### Added

- Registered `HostProgressRing` (#13176, kernel-contract half) in
  `KERNEL_PRIMITIVES` so `resolve` classifies it as `Resolution::Kernel`
  rather than an unresolved component reference — mirrors `Path`'s own
  two-registry registration below.
- Registered `Path` (the kernel drawing primitive, #12028 item 3) in
  `KERNEL_PRIMITIVES` so `resolve` classifies it as `Resolution::Kernel`
  rather than an unresolved component reference. See
  `code/specs/UI39-mosaic-drawing-primitive.md`.

### Fixed

- Preserve default `text`, `number`, and `bool` MIL slot values while inlining
  package components; explicit call-site bindings continue to take precedence.

### Added
- Registered `HostSwitch` as a kernel primitive so package expansion preserves
  native on/off controls for backend lowering.
- Registered `HostSlider` as a kernel primitive so package expansion preserves
  native range controls for backend lowering.
- Default UI29-2 authored children are spliced into a dependency component's
  typed child mount during package expansion. Caller-owned slot bindings retain
  consumer scope, empty mounts disappear, and passing children to a component
  without a mount fails instead of silently discarding content.
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
