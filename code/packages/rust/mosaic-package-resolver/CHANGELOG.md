# Changelog

## Unreleased

### Added — `SlotBindingMissing`: a required slot must be bound (UI34 §7, E6 F1)

UI13 §1 says a slot without a default is required, and that the compiler
rejects a call site that omits one. Nothing checked that. An unbound
`slot: x` was left in the inlined tree, where it named a slot of the
*consumer*. Depending on the consumer, that was:

- a misleading `UnknownSlot` naming the consumer (probe P1);
- a silent capture of an unrelated consumer slot with the same name (P3);
- an undeclared identifier in the generated Swift and TypeScript, when the
  slot was read only inside an expression (P12).

See `mosaic-unbound-slot-semantics-v1.md` §1.3 for the probes.

What changed:

- **The check.** `resolve` and `resolve_with_renames` now check every
  `pkg::P::C` call site before substituting anything, against the callee's
  interface (its `.mil`), not against what its layout reads.
  - Every slot with no default must be bound under its declared name.
  - Only the slot named by the layout's child mount is exempt, because the
    caller's children fill it. Every other `node` or `list<node>` slot is
    read as a prop, so it is required (security review, finding 1).
  - Defaulted slots keep their default, as before.
  - The walk covers the consumer's tree and, once each, every package
    component layout it reaches, including sibling references written
    without `pkg::`. A miss in a package's own layout is reported against
    that component.
- **New variant `LayoutResolveError::SlotBindingMissing`**, with fields
  `consumer`, `package`, `component`, `slot`, `call_site` and
  `defaultable`.
  - The layout IR has no source spans, so `call_site` is a path such as
    `Column[app-root] > Row#2 > pkg::mosaic-pkg-lib::Card`.
  - The message names the package and component and gives the fix. It
    offers "give it a default" only for `text`, `number` and `bool`.
- **New variant `LayoutResolveError::Multiple`.** Every miss, plus any
  package that failed to load along the way, is reported in one error.
  A single error is still reported bare.
- **Callee scope check (security review, findings 1 and 2).** When a
  package component is loaded, every free name in its expressions must be a
  declared slot, a `For` `as:`/`index:` binding in scope, or
  `true`/`false`. The child-mount slot may be read only at the mount. The
  new variants are `UndeclaredExpressionIdentifier` and
  `ChildMountSlotReferenced`.
  - The layout validator treats expressions as opaque text, so a package
    could otherwise name a consumer value it was never given, by accident
    or on purpose.
  - The binding rewrite and this check now share one expression tokenizer,
    `expression_pieces`. It reads a kebab-case NAME as one name, where the
    old rewrite split it at the `-`; the grammar has no minus operator.
- **Bindings come only from declared slots and emits (finding 3).** A
  call-site prop that names neither is no longer substituted into the
  callee. Before, `row: slot: x` turned a loop's `row.title` into
  `x.title`. Inside a `For`, a bare name that matches the loop's binding is
  never rewritten. Reporting such props is left to F2
  (`UnknownSlotBinding`).
- **Bounded error list (finding 4).** At most `MAX_REPORTED_ERRORS` (100)
  errors are kept, and `Multiple { errors, omitted }` counts the rest:
  "… and N more". Load errors are deduplicated through a set keyed by
  (variant, package, component), not a linear scan of every error.
- **A binding counts only under the slot's kebab-case name.** A
  `bodyValue:` binding never filled a `slot: body-value` prop, so it does
  not satisfy the check either.

**Tests:**

- One test per probe: P1, P3, P5b and P12 now fail, while P2, P8, P9b and
  P11 still pass.
- Several misses reported together; the exact message; a required slot
  the layout never reads; nested and caller-authored call sites; a
  component mounted twice; load errors reported beside misses.
- `every_repository_layout_binds_every_required_slot` runs the binding
  check on every exported layout in `code/` (93 layouts, 57 call sites),
  and the callee scope check on all 89 exports. It found no consumer
  relying on an unbound slot, and no export with a scope error.
  - Removing JournalApp's `selected-key` binding from its `RecordList`
    makes it fail. Before this check, that omission would have built:
    JournalApp declares its own `selected-key`, which would have been
    captured.
  - Renaming `selectedKey` to a free name in `RecordList.mll` makes it
    fail too.
- Security-review tests:
  - a `node` slot read as a prop, and an unmounted `list<node>` slot, must
    be bound;
  - the mount slot read as a prop, and in an expression, is rejected;
  - a free name is rejected, with an exact-message test;
  - the names an expression may use;
  - `slot:` to an undeclared slot inside an expression is rejected;
  - an undeclared call-site prop is not substituted;
  - `For` shadowing;
  - the kebab-aware tokenizer;
  - the cap on the error list.

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
