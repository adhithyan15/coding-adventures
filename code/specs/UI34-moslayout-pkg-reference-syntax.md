# UI34 — `pkg::package-name::Component` reference syntax for moslayout

**Status:** Specification (draft)
**Layer:** moslayout language (`.mll` grammar, AST, compiler resolver)
**Depends on:** UI14 (moslayout), UI28-1 (`mosaic-pkg-grid` userland
package), UI29 (kernel primitives), UI33 (rewrite-unified-architecture).
**Unblocks:** removing every hand-written Grid widget from the VisiCalc
demos by letting each demo's `.mll` import the canonical Grid composition
from `mosaic-pkg-grid` instead of inlining it.

---

## 1. Motivation

UI28-1 ships `mosaic-pkg-grid` v0.2.0 as the canonical userland Grid.
The package exports `Grid`, `Cell`, and `Column` built from UI29 kernel
primitives — exactly the composition every backend's emitter already
lowers.  The package compiles clean, has integration tests, and is the
single source of truth for what a Grid is.

In spite of that, every VisiCalc demo today either:

1. Inlines a hand-copied version of `mosaic-pkg-grid`'s Grid layout
   into its own `.mll` file (the React demo) — drifting the moment the
   package changes; or
2. Hand-writes the grid widget directly in the host language
   (`Grid.swift`, `Grid.kt`, `grid.dart`, `<table>` literals in
   `index.html`, inline `QtQuick` in `main.qml`) — duplicating the
   composition N times with no consistency guarantees.

The leading comment of `code/programs/mosaic/visicalc/Grid.desktop.mll`
documents the missing piece in plain English:

> The VisiCalc demo cannot yet `import` Grid from that package
> cross-repo because mosaic-compile's package-resolver isn't wired
> through the demo's build script — so the composition is inlined here.
> When the package-resolver lands, this whole file collapses to:
>
>     layout Grid {
>       pkg::mosaic-pkg-grid::Grid (
>         viewport-rows:  slot: viewport-rows ,
>         column-headers: slot: column-headers ,
>         …
>       )
>     }

UI34 specifies the `pkg::…::…` syntax referenced there — the grammar
extension, the AST shape, the resolver semantics, and the per-backend
contract — without yet committing to any one backend's implementation.

---

## 2. Surface syntax

A layout node tag may take one of two forms:

```
node = qualified_name [ part_name ] [ "(" prop_list ")" ] [ "{" node* "}" ]
qualified_name = NAME
               | "pkg" "::" NAME "::" NAME
```

The unqualified form (just `NAME`) is unchanged from UI14 — it names
either a kernel primitive (`Box`, `HostTable`, …) or a same-file local
component reference.

The qualified form **`pkg::P::C`** names component `C` exported by
package `P`.  `P` is the package's `name` field in `mosaic-package.toml`
(kebab-case, e.g. `mosaic-pkg-grid`).  `C` is one of the entries in
`[components].exports` (PascalCase, e.g. `Grid`).

### 2.1 Example

```mll
layout Sheet {
  Column [ root ] {
    pkg::mosaic-pkg-grid::Grid (
      viewport-rows:  slot: viewport-rows ,
      column-headers: slot: column-headers ,
      column-widths:  slot: column-widths ,
      selected-row:   slot: selected-row ,
      selected-col:   slot: selected-col ,
      edit-row:       slot: edit-row ,
      edit-col:       slot: edit-col ,
      edit-content:   slot: edit-content ,
      onNavigate:     emit: onNavigate ,
      onEditCommit:   emit: onEditCommit ,
      onEditCancel:   emit: onEditCancel
    )
  }
}
```

### 2.2 What `pkg::` deliberately does NOT do

- **No version specifier.**  Package versions live in
  `mosaic-package.toml`'s `[dependencies]` table, not inline in the
  reference.  This matches how every other ecosystem (Cargo, npm,
  Maven) keeps source code free of versions.
- **No path / module subscripts.**  A package's exports are flat
  (UI28-1 §3): every export is named once in `[components].exports`
  and is reachable via `pkg::P::Name`.  Three-level paths like
  `pkg::P::layout::Grid` are deliberately not part of the grammar.
- **No `pkg::self::…` form.**  Within a package's own source files,
  the unqualified form `Grid` already resolves to the sibling
  component (this is what the XAML registry calls *self-reference*).
  Adding `pkg::self::Grid` would be redundant.

---

## 3. Grammar changes

### 3.1 `moslayout.tokens`

Add one literal token and one keyword:

```
DOUBLE_COLON = "::"   # listed BEFORE COLON for maximal munch

keywords:
  layout
  slot
  emit
  pkg            # new — fourth structural keyword
```

The `pkg` keyword is unambiguous in expression position because
expressions never start a node, and unambiguous in node position
because nodes never start with `slot` / `emit` either.  Authors who
were previously using `pkg` as a slot or part name (none, as far as
the repo audit shows) would need to rename; the grammar test suite
should fail loudly if any in-repo `.mll` regresses.

### 3.2 `moslayout.grammar`

```
node            = qualified_name [ part_name ] [ LPAREN prop_list RPAREN ]
                                              [ LBRACE { node } RBRACE ] ;
qualified_name  = NAME
                | PKG DOUBLE_COLON NAME DOUBLE_COLON NAME ;
```

`PKG` is the new keyword token from §3.1.  The rule is LL(1): the
parser knows from the first token whether to take the unqualified
or qualified branch.

The two NAMEs in the qualified branch are validated by the compiler,
not the grammar — kebab-case package name first, PascalCase
component name second.

---

## 4. AST changes

### 4.1 `LayoutNode` — encoded-tag form

`LayoutNode` is **unchanged**.  Qualified references are stored
inside the existing `tag: String` field, encoded with the same
`pkg::P::C` shape the source language uses:

```rust
pub struct LayoutNode {
    /// Either an unqualified component name (`"Grid"`, `"HostTable"`)
    /// or a qualified reference (`"pkg::mosaic-pkg-grid::Grid"`).
    pub tag: String,
    pub part_name: Option<String>,
    pub props: Vec<LayoutProp>,
    pub children: Vec<LayoutNode>,
}
```

Two small helper methods expose the structure without forcing
callers to split the string:

```rust
impl LayoutNode {
    /// `Some((package, component))` for qualified tags.
    /// `None` for unqualified references.
    pub fn package_ref(&self) -> Option<(&str, &str)>;
    /// The unqualified component name — strips the `pkg::P::`
    /// prefix when present.
    pub fn component(&self) -> &str;
}
```

This is a deliberate revision of the earlier draft that proposed a
new `package: Option<String>` struct field.  The encoded-tag form
wins on:

- **Source compatibility.**  ≈ 350 in-repo `LayoutNode { … }` literal
  constructions in test code keep compiling unchanged.  A new struct
  field would require touching every one of them.
- **Wire compatibility.**  The serde JSON shape that downstream
  tooling (`mosaic-driver`, language-server, debugger) consumes is
  byte-identical for every existing layout.
- **Zero cost when unused.**  The unqualified path stays a single
  string comparison; `package_ref()` returns `None` after one
  `starts_with("pkg::")` check.
- **Round-trip is exact.**  Re-emitting the AST yields the same
  source text the author wrote.

The struct-field form (the earlier draft) is preserved as a
follow-up amendment if downstream tooling ever wants cheap
field-projected access to the package name.

### 4.2 `analyze()` and `validate()`

`analyze()` writes the qualified reference into `tag` using the
canonical `pkg::P::C` form.  Unqualified nodes are unchanged.
`validate()` gains two new checks:

- **PK1** — qualified tag's package name must be kebab-case
  (`[a-z][a-z0-9]*(-[a-z][a-z0-9]*)*`).  This is the same shape
  `mosaic-package-manifest` already enforces on `package.name`.
- **PK2** — qualified tag's component name must be PascalCase.

Cross-package resolution (does the package exist? does it export
this component?) is a *resolver-layer* concern, not a validator one;
see §5.

---

## 5. Resolution semantics

Resolution happens in **`mosaic-compile`**, not in the per-backend
emitter.  The emitter sees only kernel primitives and resolved local
components; it never sees a `pkg::` reference.

This is a deliberate choice — Path A in the design space — for three
reasons:

1. **Universal across backends.**  Every backend benefits from the
   same one resolver implementation.  No per-emitter package code
   path, no per-emitter import vocabulary.
2. **No package-publish prerequisite.**  Packages ship `.mil`/`.mll`/
   `.msl` source; the resolver compiles them in-context.  We avoid
   the npm-style per-target pre-compilation step for now.
3. **Preserves the kernel-primitive moratorium.**  The set of tags
   any emitter must handle is exactly the UI29 kernel.

The XAML registry (Path B — import-and-call) stays available for
backends with first-class component-reference systems (XAML's xmlns,
future React's `import` statement); UI34 does not deprecate it.
Backends are free to opt into Path B in a later spec if their target
ecosystem makes it cheaper.

### 5.1 Resolution algorithm

Inputs to `mosaic-compile`:

- The consumer's `--interface` / `--layout` / `--style` triple.
- `--package-manifest <path>` (already exists) — optional, points at
  the consumer's *own* `mosaic-package.toml`.
- A new optional `--package-search-path <colon-separated-list>` —
  directories to search for sibling packages by name.  Defaults to
  the consumer's own package directory plus a small set of well-known
  workspace roots (`code/packages/`).

Algorithm — invoked once per `pkg::P::C` reference found while
analyzing the consumer's `.mll`:

1. Locate package `P` by searching `--package-search-path` for a
   directory containing a `mosaic-package.toml` whose
   `package.name == P`.  Error `PackageNotFound { package: P }` if no
   match.
2. Read the manifest; assert `C ∈ [components].exports`.  Error
   `ComponentNotExported { package: P, component: C }` otherwise.
3. Locate the three component source files relative to the package
   root (`src/C.mil`, `src/C.mll`, `src/C.dark.msl` — UI28-1 §3.1
   layout).  Compile them through the same `mosmodel_compiler` /
   `moslayout_compiler` / `mosstyle_compiler` pipeline the consumer
   uses.
4. **Inline** the resolved `LayoutDef.root` into the consumer's tree
   at the `pkg::P::C` reference site.  Inlining rules:
   - Props passed at the call site (`viewport-rows: slot: x`) bind to
     the called component's same-named slots — this is how the
     existing same-file component-reference path already works
     (UI14 §6).
   - The called component's part names are prefixed with
     `<P>__<C>__` to avoid colliding with the consumer's part names
     (`sheet` → `mosaic-pkg-grid__Grid__sheet`).  The mosstyle
     compiler picks up the rewritten names through the same part-map
     JSON the unqualified path produces today.
   - The called component's slot-binding sites that reference
     SlotRefs use the consumer's slot names directly — the resolver
     has already substituted them at step 4.1.
5. Recurse — the resolved subtree may itself contain `pkg::` nodes
   (e.g. `mosaic-pkg-grid::Grid` references `mosaic-pkg-grid::Cell`).
   The resolver tracks the in-flight set of `(P, C)` pairs and
   errors on a cycle as `CircularPackageReference`.

The resolver is purely additive — when `--package-search-path` is
not set and no `pkg::` references are present, the compile path is
unchanged from today.

### 5.2 Cache & build-graph integration

Each resolved package compilation is keyed by
`(package-root-absolute-path, package-version, component-name)` and
the result is memoised inside a single `mosaic-compile` invocation.
The build-tool (Go implementation at `code/programs/go/build-tool/`)
already tracks file modifications; resolved packages register as
input dependencies of the consuming component so a touch on
`mosaic-pkg-grid/src/Grid.mll` invalidates every consumer.

---

## 6. Per-backend contract

Emitters need **zero** new code paths to support UI34.  By the time
the resolver hands them a `LayoutNode` tree:

- Every `pkg::` reference has been substituted with the resolved
  subtree.
- Every remaining `tag` is either a kernel primitive (UI29) or a
  same-file local component (UI14 §6 — unchanged path).
- No `tag` starts with `pkg::` after resolution
  (`debug_assert!(!node.tag.starts_with("pkg::"))`).

A diagnostic-only check (`debug_assert!`) in each emitter's entry
point can confirm this invariant during development; release builds
omit the check.

Backends that *want* to render a `pkg::` reference as a real
component-system call site (XAML `<grid:Grid>`, future React
`import` + `<Grid />`) can opt into a Path B emit by reading the
unresolved-tree representation through a new `mosaic-compile`
flag (`--keep-pkg-refs`).  UI34 does not specify that path; it is
left for a future amendment.

---

## 7. Error model

The resolver surfaces precise, fixable errors:

| Variant | Meaning | Suggested fix |
|---|---|---|
| `PackageNotFound { package }` | No `mosaic-package.toml` matched | Add the package directory to `--package-search-path` or check the kebab-case name. |
| `ComponentNotExported { package, component }` | Manifest's `[components].exports` does not list the component | Add the component to the manifest, or re-check the PascalCase name. |
| `ComponentSourceMissing { package, component, file }` | `src/<C>.mil` (or `.mll`/`.msl`) is missing | Add the missing file to the package. |
| `CircularPackageReference { cycle }` | `pkg::A::X → pkg::B::Y → pkg::A::X` | Refactor the cycle out; package graphs must be DAGs. |
| `SlotBindingMissing { package, component, slot }` | Call site did not supply a required slot | Bind the slot at the call site or mark it optional in the component's `.mil`. |

Each error carries the source span of the offending `pkg::` reference
so editors and language-server clients can underline it.

---

## 8. Migration plan

UI34 lands as a sequence of small PRs.  None are individually scary:

1. **PR-1 (this spec).**  Doc only.  No code changes.
2. **PR-2 — grammar & AST.**  Adds `DOUBLE_COLON` token, `pkg`
   keyword, `qualified_name` rule, `LayoutNode.package` field.
   Existing `.mll` files compile unchanged.  Tests cover the parser
   accepting / rejecting qualified tags; no resolver yet, so a
   qualified tag fails analysis with `UnresolvedPackageReference`.
3. **PR-3 — resolver.**  Implements §5 inside `mosaic-compile`.
   First exercise target: a synthetic test consumer that
   references `pkg::mosaic-pkg-grid::Grid`.
4. **PR-4 — demo rewire.**  Collapses
   `code/programs/mosaic/visicalc/Grid.desktop.mll` from its inlined-grid
   shape to the `pkg::mosaic-pkg-grid::Grid (...)` reference.  No
   visual change in the running app; the React-emitted Grid.tsx is
   byte-identical to today because the resolved subtree is the same.
5. **PR-5..N — other demos.**  Each non-React demo (`visicalc-html`,
   `visicalc-webcomp`, `visicalc-swiftui`, `visicalc-qt`,
   `visicalc-flutter`, `visicalc-compose`, `visicalc-android`) gets:
   - A consumer `.mll` written against the kernel primitives (or
     reusing `pkg::mosaic-pkg-grid::Grid` directly).
   - Its hand-written grid file deleted.
   - Its build script updated to invoke `mosaic-compile` with the
     right backend.

Each PR is independently green and reviewable.  PR-4 is the proof
the cross-package compile loop works end-to-end on the React
backend; PR-5..N validate that no per-backend resolver code was
needed.

---

## 9. Open questions

- **Package discovery in monorepo vs polyrepo.**  Today every package
  lives under `code/packages/`.  When packages start coming from
  outside the repo (vendored via Cargo / npm / RubyGems), the
  `--package-search-path` default needs to expand.  Not in scope for
  UI34 — explicit `--package-search-path` is enough until the polyrepo
  story lands.
- **Diamond resolution.**  Two packages A and B both depending on a
  third package C is fine because resolution is purely structural
  (we re-resolve C from source every time).  Once we add a build cache
  the cache key includes C's package root, so identical C resolves to
  identical AST — no diamond hazard.
- **Cross-package style overrides.**  Today the consumer's `.msl`
  can target the resolved sub-tree's parts via the prefixed part name
  (`mosaic-pkg-grid__Grid__cell`).  Whether to also allow a friendlier
  re-export syntax is left for UI35.

---

## 10. Backwards compatibility

UI34 is purely additive.  The only break is the new `pkg` keyword,
which the grammar audit confirms is unused as a slot / part / prop
name anywhere in the in-repo `.mll` corpus.  The grammar test suite
catches any regression.
