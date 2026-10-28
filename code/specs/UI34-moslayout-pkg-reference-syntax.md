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
   - **Required slots.** Before substitution, the callee's interface
     is checked: every slot with no default must be bound at the call
     site, or the result is `SlotBindingMissing` (§7). An omitted slot
     with a default takes its default.
   - **Part names.** *As built:* the called component's part names
     are inlined **as authored** (`sheet` stays `sheet`), so the
     consumer, its tests, and every backend's accessibility id,
     test tag or object name see the package's own names. The
     `<P>__<C>__` prefix this section first proposed was never
     implemented. A part name written at the call site replaces the
     component root's.
   - **Mounting a component more than once.** The resolver counts
     mounts per `(P, C)` across the whole resolved tree, including
     mounts nested inside other packages. The **first** mount keeps
     its authored names, so single-mount output is unchanged. The
     n-th mount (n ≥ 2) gets every inlined part name suffixed `-m<n>`
     (`empty-state` → `empty-state-m2`, `empty-state-title-m2`). It
     is `-m2`, not `-2`, because a Mosaic identifier segment cannot
     start with a digit, and a consumer `.msl` must be able to name
     the part.
     Excluded from the suffix: a root name the caller wrote, and
     children the caller splices in, which are the caller's own
     parts. The artifact builder gives each renamed part a copy of
     the original part's style, all states included, so the n-th
     mount renders like the first. A consumer `.msl` may style
     `empty-state-m2` directly, and that style replaces the copy. A
     suffixed name that clashes with a real part is still a
     `DuplicatePart` error. Before this, a second mount always failed
     with `DuplicatePart` (Trestle's Checklists view had to fall back
     to plain text, task-app-checklists-view-v1.md).
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

### 6.1 What the backends actually do — measured (#14886)

The `debug_assert!` above was never added to any emitter. That turned out
not to matter, because the real behaviour is stronger than the assert would
have been: an assert vanishes in release builds, whereas every backend
already refuses a qualified tag with a clean, named error.

Measured by handing each backend a layout containing
`pkg::mosaic-pkg-grid::Cell` with **no resolver pass**, with a bare unknown
`Cell` as the control so “rejects qualified tags” could be told apart from
“rejects unknown tags”:

| backend | qualified tag | bare unknown tag |
| --- | --- | --- |
| compose, html, react, swiftui, qt, webcomponent | `UnknownPrimitive("pkg::mosaic-pkg-grid::Cell")` | `UnknownPrimitive("Cell")` |
| xaml | resolves it — see 6.2 | `UnknownComponent("Cell")` |
| **flutter** | `UnknownPrimitive(..)` | **`Ok`** — see 6.3 |

No backend silently drops the qualifier and resolves the tag structurally,
which was the failure mode worth checking for: it would produce a component
reference to the right *name* from the wrong *package*.

### 6.2 XAML resolves a qualified tag, by design (#14884)

§6 says no `tag` starts with `pkg::` after resolution. That holds for a
full build, but XAML has a second, resolver-less entry point that §6 did not
anticipate: `from_pipeline` called directly with a `ComponentRegistry`, which
is how `mosaic-compile --package-manifest` compiles a single component and
how `tests/pkg_grid_compiles_to_xaml.rs` exercises the registry.

On that path a qualified tag does reach the emitter. `emit_component_reference`
splits it with `LayoutNode::package_ref()` and uses the component half for
both the registry key (the registry is keyed by bare export names) and the
emitted element name — `<grid:pkg::mosaic-pkg-grid::Cell/>` is not
well-formed XML. A tag that names a package explicitly must match the
registration's package, so `pkg::other-pkg::Cell` does not resolve to
`mosaic-pkg-grid`'s `Cell`.

This is narrower than the “Path B” below: it is not a `--keep-pkg-refs`
mode, just the registry path being made correct for input it can genuinely
receive.

### 6.3 Flutter does not fail closed

Flutter is the one backend that returns `Ok` for an unresolved component
reference, emitting a comment and a zero-size widget:

```dart
/* TODO: component reference 'Cell' not yet resolved */ const SizedBox.shrink()
```

Every gate downstream of `from_pipeline` then passes: the build is green and
the component is absent. This is how `mosaic-pkg-grid` appeared to work on
Flutter while six backends refused it — Flutter was not the one that
supported the package, it was the one that could not report the failure.
Tracked in #14892.

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
| `SlotBindingMissing { package, component, slot }` | Call site did not supply a required slot | Bind the slot at the call site, or give it a default in the component's `.mil` (`slot x : text = "" ;`). |
| `UndeclaredPackageReference { consumer, package, component, call_site, manifest_path, found_version }` | `pkg::P::C` names a package that is neither the referencing package nor in its `mosaic-package.toml` `[dependencies]` | Add `P = "<version>"` to `[dependencies]` in the named manifest (mosaic-package-publishing-v1 §10 Phase 1b). |

Each error carries the source span of the offending `pkg::` reference
so editors and language-server clients can underline it.

**Implementation note: `SlotBindingMissing` (2026-10-26,
mosaic-unbound-slot-semantics-v1 §4 R1 / follow-up F1).** Built in
`mosaic-package-resolver` as
`SlotBindingMissing { consumer, package, component, slot, call_site, defaultable }`.
How it behaves:

- **When it runs.** Once per resolve, before any substitution. It walks the
  consumer's tree and, once each, the layout of every package component it
  reaches. A nested reference inside a package is reported against that
  package component, and an unqualified reference to a sibling export
  counts as a call site. Caller-authored children belong to the component
  that wrote them.
- **What it checks.** The callee's *interface*: every slot in its `.mil`
  with no default. It does not look at which slots the layout reads, so a
  required slot the layout never uses must still be bound. That is what
  stops an unbound name surviving into the consumer's scope, where it was
  captured by a same-named consumer slot or leaked into an expression as an
  undeclared identifier.
- **The one exemption.** The slot that the layout's `slot: <name>` child
  mount names is filled by the caller's children, not by a prop, and an
  unfilled mount is removed (UI29-2). Every other `node` or `list<node>`
  slot is read as a prop (`HostSurface ( content: slot: x )`), which
  children cannot fill, so it is required like any other slot.
- **Callee scope.** When a package component is loaded, two more names are
  checked, because substitution cannot rewrite them and they would
  otherwise be resolved in the consumer's scope:
  - Every free name in its expressions must be a declared slot (kebab-case
    or camelCase), a `For` `as:`/`index:` binding in scope, or
    `true`/`false`. Names after `.` and string contents are not free.
    Anything else is `UndeclaredExpressionIdentifier { package, component,
    identifier, expression }`.
  - A bare NAME prop value (`content: title`) is a `Keyword` in the IR, but
    the emitters lower it to a variable in most positions: React writes
    `{title}`, HTML writes `{{title}}`. It is treated like a bare name in an
    expression, except when it is a literal:
    - `true` or `false`, anywhere;
    - a position in `moslayout_compiler::KEYWORD_LITERAL_PROPS`, the
      kernel's enum props (`a11y-role`, `axis`, `collapse`, `kind`, `dir`,
      `target`, `table-cell-role`);
    - a `For`'s `as:`/`index:`;
    - at a call site, a value bound to a `one-of` slot of the component it
      mounts, but only if it is one of that slot's declared values. Any
      other bare NAME there is an identifier. `tone: tone` forwards the
      caller's own `tone`, and `tone: authToken` is
      `UndeclaredExpressionIdentifier`. If a declared value is also the
      name of one of the caller's slots or loop bindings, the declared
      value wins.
    A bare NAME that names one of the component's own slots is rewritten to
    `slot: <name>`, so it takes the call site's binding or the default.
    Before, it showed the consumer's slot of the same name.
    The layout validator applies the same rule to every layout compiled
    with its interface, packages and applications alike (UI29 §3.4, "A
    bare name must be bound"), so outside call sites an unbound bare
    NAME in a package now fails as the layout compile error
    `UnboundName` (wrapped in `NestedCompileError`) before this check
    runs; the check remains for call-site values, which need the
    mounted component's interface.
  - The mount slot may be read only by the mount. A read anywhere else,
    whether as a prop, as a bare NAME, or inside an expression, is
    `ChildMountSlotReferenced { package, component, slot }`.
- **`one-of` values are strings in the callee.** A call-site value that is
  one of a `one-of` slot's declared values (`tone: loud`) is substituted as
  the string `"loud"`. A `KEYWORD_LITERAL_PROPS` position is the
  exception: there it is substituted as the bare enum value. It never
  becomes an identifier. Before, `content: slot: tone` showed React
  `{loud}`, and `( tone == "calm" )` became `( loud == "calm" )`, an
  undeclared identifier (unbound-slot-semantics-v1 P14).
  - After substitution, expressions that have become constant are
    evaluated, because TypeScript rejects a comparison between two
    different string literals (TS2367). This covers `==`/`!=` on two
    literals of one kind, numeric `<`/`<=`/`>`/`>=`, `!` of a boolean, and
    `&&`/`||` with a boolean literal on the left.
- **Bindings.** Only call-site props that name a declared slot or emit
  become bindings. Inside a `For`, a bare name that matches the loop's
  binding is the loop variable and is not rewritten (UI29 §3.4, "For
  wins"). Reporting a call-site prop that names nothing is F2
  (`UnknownSlotBinding`).
- **Every error.** Every missing slot at every call site is reported in one
  `Multiple` error, up to 100 of them, followed by "… and N more". The
  callee-scope errors of one component have the same cap, and an error
  quotes at most 200 characters of an expression. One missing slot is
  reported bare. A package that fails to load is compiled once, and is
  reported once however many call sites name it.
- **Hygiene: a callee's `For` binding never captures a consumer name**
  (UI29 §3.4, backlog 6f1m). Two kinds of name come from outside the
  callee and can land inside one of its loops: a consumer value bound
  into the callee (`title: row`, read by `Text ( content: title )` in
  the callee's `For ( each: slot: rows , as: row )`), and the children
  a caller splices into a child mount that sits inside a callee loop.
  Both are the consumer's names. Before substitution, the inliner looks
  at each callee `For`: if its `as:` or `index:` binding (compared in
  camelCase, the spelling every emitter gives an identifier) is a name
  that a substituted value or a spliced child would bring into its
  body, the binding is renamed to a fresh name, `<name>-in<n>` with the
  smallest `n` that no name in the callee, the bound values or the
  children already uses. Every use of the binding in the body is
  renamed with it: bare prop values in name positions, `each:` of a
  nested `For`, and bare names in expressions; a nested `For` that
  binds the same name again ends the rename for its own body. Literal
  positions (see "Callee scope") are not uses. A binding that captures
  nothing keeps its authored name, so the output of every layout that
  was not miscompiled is unchanged.
- **The message.** It names the package and component, and gives the fix.
  The "give it a default" half is offered only for `text`, `number` and
  `bool`, the types that can have one today.

Two departures from this section:

- There is no "mark it optional" marker. The marker is `= <default>`
  (UI13 §1).
- The layout IR carries no source spans. `call_site` is a path through the
  consumer's tree instead, for example
  `Column[app-root] > Row#2 > pkg::mosaic-pkg-lib::Card[hero]`. A named
  node shows its part name, and an unnamed one shows its 1-based position
  among its siblings.

Until this was built, a missing binding surfaced as the consumer's
misleading `UnknownSlot`, or not at all.

**Implementation note: `UndeclaredPackageReference` (2026-10-26,
mosaic-package-publishing-v1 §10 Phase 1b).** §2.2 says versions live in
`[dependencies]`, but nothing required a referenced package to be listed
there. Any package on the search path could be inlined, with no version
checked (publishing-v1 §7.4) and no build-graph edge. Now:

- **The rule.** Every `pkg::P::C` call site must name the referencing
  package itself or a dependency it declares.
- **Which manifest.** Inside a package component's layout, the declared set
  comes from that package's own manifest. For the layout being resolved, it
  comes from the consumer manifest the caller supplies with
  `LayoutPackageResolver::with_consumer`. Package builds and
  `mosaic-compile` with a package manifest supply it. With no consumer
  manifest, only nested references are checked.
- **When it runs.** In the same pre-substitution walk as
  `SlotBindingMissing`, so both kinds of error come back in one `Multiple`.
- **How often.** Each undeclared package is reported once per referencing
  manifest, at its first call site.

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
- **Cross-package style overrides.**  The consumer's `.msl` can target
  the resolved sub-tree's parts by their inlined names: the authored
  name for the first mount (`cell`), and the suffixed name for later
  ones (`cell-m2`, §5 step 4). Whether to also allow a friendlier
  re-export syntax is left for UI35.

---

## 10. Backwards compatibility

UI34 is purely additive.  The only break is the new `pkg` keyword,
which the grammar audit confirms is unused as a slot / part / prop
name anywhere in the in-repo `.mll` corpus.  The grammar test suite
catches any regression.
