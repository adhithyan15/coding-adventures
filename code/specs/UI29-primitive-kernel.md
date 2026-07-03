# UI29 — Primitive Kernel + Userland Component Packages

**Status:** Specification (draft)
**Layer:** UI / cross-cutting (moslayout grammar + every backend emitter + package convention)
**Depends on:** UI14 (moslayout), UI24 (emit→dispatch)
**Supersedes (partial):** the monolithic Grid in UI14 §11, all of UI28 (the Cell-centric Grid v3 that lived inside each backend)

---

## 1. Why this exists

The first six React-Grid PRs (UI14 → UI28) baked one specific component
— **Grid** — directly into the React emitter as ~700 lines of bespoke
JSX-generation logic (`emit_grid_jsx`, `emit_grid_jsx_v3`, plus all the
sub-part / state-suffix / sticky-header / column-widths plumbing). The
same plumbing was about to be re-written, line-for-line and bug-for-bug,
in `mosaic-emit-swiftui`, `mosaic-emit-qt`, `mosaic-emit-webcomponent`,
and `mosaic-emit-html`.

This is the wrong split. The composition logic of Grid (walk Column
children, loop over rows, loop over columns, conditionally render an
editor for the cell being edited, substitute `row[c]` into the cell
template) is **backend-agnostic**. Only the leaf primitives — what is
a `<div>` vs a `Group` vs an `Item`; what is a `<table>` vs a
SwiftUI.Table vs a `QTableView` — are backend-specific.

The architecture this spec proposes:

```
                              ┌─────────────────────────────────────────┐
                              │  Userland component packages            │
                              │  mosaic-pkg-grid, mosaic-pkg-input,     │
                              │  mosaic-pkg-tabs, mosaic-pkg-tree, ...  │
                              │  (each: .mil + .mll + .{theme}.msl)     │
                              └─────────────────┬───────────────────────┘
                                                │ compose from
                                                ▼
                              ┌─────────────────────────────────────────┐
                              │  Kernel primitives (this spec)          │
                              │  Box Row Column Stack Text Image        │
                              │  Spacer Divider Icon  +  If For         │
                              │  HostInput HostButton HostTable         │
                              └─────────────────┬───────────────────────┘
                                                │ lowered by
                                                ▼
       ┌───────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────┐
       │ React     │  │ WebComponent │  │ SwiftUI      │  │ Qt/QML   │  ...
       │ emitter   │  │ emitter      │  │ emitter      │  │ emitter  │
       └───────────┘  └──────────────┘  └──────────────┘  └──────────┘
```

The kernel is **frozen and small** (15 primitives). Backends implement
the kernel once. Everything else — Grid, Cell, Column-as-metadata, rich
Input, Tabs, Tree, Modal, Toast, etc. — is a **userland package** in
plain moslayout source, the same shape as any user component. Authors
who don't want a Grid don't pay for it; authors who want a richer Grid
fork the package and publish their own.

This is the pattern React, Vue, SwiftUI, Compose, and every other modern
UI framework converged on: a tiny host-primitive surface, everything
else composition.

---

## 2. The kernel

### 2.1 The fifteen kernel primitives

The kernel is the *complete* set of primitives every Mosaic backend
must implement. A backend that lowers all 15 can render any Mosaic
component package.

| Primitive | Purpose | DOM lowering | SwiftUI lowering | Qt/QML lowering |
|---|---|---|---|---|
| `Box` | generic opaque container | `<div>` | `Group { ... }` | `Item { ... }` |
| `Row` | flex container, horizontal | `<div style={display:flex,flexDirection:row}>` | `HStack { ... }` | `RowLayout { ... }` |
| `Column` | flex container, vertical | `<div style={display:flex,flexDirection:column}>` | `VStack { ... }` | `ColumnLayout { ... }` |
| `Stack` | z-axis / absolute container | `<div style={position:relative}>` | `ZStack { ... }` | `Item { children: anchors.fill }` |
| `Text` | text leaf | `<span>` (or `<p>` with `block: true`) | `Text(...)` | `Text { text: ... }` |
| `Image` | image leaf | `<img>` | `Image(...)` | `Image { source: ... }` |
| `Spacer` | flex glue | `<div style={flex:1}>` | `Spacer()` | `Item { Layout.fill...: true }` |
| `Divider` | hairline rule | `<hr>` | `Divider()` | `Rectangle { height: 1, color: "#888" }` |
| `Icon` | icon glyph | `<span className="icon">` | `Image(systemName: ...)` | `Text { font.family: iconFont }` |
| `If` | conditional render *(new)* | `{cond ? <a/> : <b/>}` | `if cond { ... } else { ... }` | `Loader { active: cond; ... }` |
| `For` | iterate over a list *(new)* | `{coll.map((x, i) => <child/>)}` | `ForEach(coll, id: \.self) { x in ... }` | `Repeater { model: coll; delegate: ... }` |
| `HostInput` | native single-line text input *(new)* | `<input type="text">` | `TextField(...)` | `TextInput { ... }` |
| `HostButton` | native push button *(new)* | `<button>` | `Button { ... } label: { ... }` | `Button { ... }` |
| `HostTable` | semantic data table *(new)* | `<table>` (with `<colgroup>`/`<thead>`/`<tbody>` slots) | `SwiftUI.Table { ... }` | `TableView { ... }` |
| `HostScroll` | scrollable viewport *(new)* | `<div style={overflow:auto}>` | `ScrollView { ... }` | `Flickable { ... }` / `ScrollView` |

### 2.2 Inclusion criteria

A primitive belongs in the kernel iff:

1. **Every host platform has a native equivalent.** Not "can be
   simulated"; *natively provides*. `<input>`, `TextField`,
   `TextInput`, etc. all exist as platform primitives because IME,
   selection, focus, accessibility, and screen-reader integration are
   not feasible to re-build at the moslayout level.
2. **No reasonable composition exists.** If `Foo` can be built from
   `Bar` + `Baz`, it doesn't belong in the kernel.
3. **It is semantically irreducible.** `Table` is in the kernel because
   accessibility tools (VoiceOver, NVDA, JAWS) and screen readers
   require `<table role="grid">` semantics to be present *as a table*,
   not synthesised from `Box`/`Row`/`Column`. A "fake table" made of
   divs is a fundamental accessibility regression.

### 2.3 What is *not* in the kernel

Examples of components that look kernel-ish but are not:

- **Grid** (the spreadsheet-style data grid). Composition of
  `HostTable` + `For` + `If` + `Cell`. Lives in `mosaic-pkg-grid`.
- **Cell** (one editable spreadsheet cell). Composition of `Box` + `If`
  + `Text` + `HostInput`. Lives in `mosaic-pkg-grid` (or any other
  package that wants a Cell).
- **TextField with floating label / validation**. Composition of
  `HostInput` + `Text` + `If`. Userland.
- **Modal / Dialog**. Composition of `Stack` + `Box` + portal mechanism
  (which is itself a primitive or a host service; TBD).
- **Tabs**. Composition of `Row` + `For` + `HostButton` + state.
- **Carousel, Tree, Tabs, Toast, Drawer, Accordion, Tooltip**. All
  userland packages.

The litmus test: if you can write the component's `.mll` file using
*only* kernel primitives, it doesn't belong in the kernel.

### 2.4 The kernel is versioned and frozen

Once a kernel primitive ships, its slot/emit surface is **stable**.
Adding a primitive requires a new spec (UI29-1, UI29-2, ...) and a
compatibility check across every backend. The set is intended to grow
*slowly* — perhaps one or two primitives per year — and never shrink.

This is the same discipline React applies to host components and the
JSX runtime: `<div>` doesn't change. Userland code depends on it.

---

## 3. New grammar — `If`, `For`, and expression syntax

The current moslayout grammar (UI14) has no conditionals, no loops,
and no expression syntax. Slot binding is the only mechanism for
runtime data flow. This was acceptable while every "interesting"
component was bespoke-coded in Rust, but for userland components to be
expressive enough to subsume Grid+Cell+Input+etc., the grammar must
gain three additions.

### 3.1 `For (each: <expr>, as: <name>, index: <name>?) { <children> }`

Iterate over a list. Binds two names into the body scope. The bindings
sit inside parentheses — the same shape every other primitive uses for
its props, e.g. `Grid (headers: ..., rows: ...)`. Earlier drafts of this
spec showed a paren-less form, but the implementation in U29-G1 (#3622)
landed paren-required for grammar consistency: a `For` node is still
just a regular `node` per the moslayout grammar, with no special parser
rule.

```moslayout
For (each: slot: viewport-rows, as: row, index: r) {
  Row [data-row] {
    For (each: slot: columns, as: col, index: c) {
      Cell [body] (
        value: row[c],
        editable: col.editable,
        is-editing: r == slot: edit-row && c == slot: edit-col,
      )
    }
  }
}
```

Lowering:

- React: `{coll.map((row, r) => <Row ...>{coll2.map((col, c) => <Cell.../>)}</Row>)}`
- SwiftUI: `ForEach(Array(coll.enumerated()), id: \.offset) { (r, row) in ... }`
- Qt/QML: `Repeater { model: coll; ... }`
- HTML (static): unrolled at compile time when the list is a literal;
  for slot-driven lists, the static HTML output uses an iteration
  template the host runtime expands.

The `index:` binding is optional. When omitted, only `as:` is bound.

### 3.2 `If (when: <expr>) { <then> } Else { <else>? }`

Conditional render. As with `For`, the `when:` binding sits inside
parentheses — paren-required matches U29-G2 (#3623). `Else` is a
sibling primitive that must immediately follow an `If`; the analyzer
enforces this in `validate_node`.

```moslayout
If (when: slot: editable && slot: is-editing) {
  HostInput (value: slot: value, onCommit: emit: onCommit)
}
Else {
  Text (content: slot: value)
}
```

The `Else` block is optional. `If`+`Else` chain by parsing two
consecutive nodes (no special chained parser rule); `Else If` is
rewritten by the parser to a nested `Else { If ... }`.

Lowering:

- React: `{cond ? (<HostInput.../>) : (<Text.../>)}`
- SwiftUI: `if cond { HostInput(...) } else { Text(...) }` (Swift's
  view-builder if/else)
- Qt/QML: `Loader { active: cond; sourceComponent: ... }` with a
  secondary loader for the else branch
- HTML (static): when `cond` is a literal, one branch is emitted; when
  `cond` is slot-driven, the static output includes a placeholder the
  host runtime resolves

### 3.3 Expression grammar

A new `expr` non-terminal supplies the values consumed by `For each:`,
`If when:`, and any slot binding that needs more than a slot reference.

```ebnf
expr        ::= or_expr
or_expr     ::= and_expr ('||' and_expr)*
and_expr    ::= eq_expr ('&&' eq_expr)*
eq_expr     ::= rel_expr (('==' | '!=') rel_expr)?
rel_expr    ::= unary    (('<' | '<=' | '>' | '>=') unary)?
unary       ::= '!' unary | primary
primary     ::= literal | slot_ref | name_ref | field_access | index_access | paren
slot_ref    ::= 'slot:' NAME
name_ref    ::= NAME                          // For-bound name
field_access::= primary '.' NAME              // col.key, row.cells
index_access::= primary '[' expr ']'          // row[c], grid[r][c]
paren       ::= '(' expr ')'
literal     ::= NUMBER | STRING | 'true' | 'false'
```

**Deliberately excluded from v1:** arithmetic (`+`/`-`/`*`/`/`),
string concatenation, function calls, ternary `?:`. We add them when
a userland component actually needs them; speculative grammar is
expensive across five backends.

**Type checking:** the expression grammar is dynamically typed at the
moslayout level — type errors surface in the target language (React's
type checker, Swift's, etc.). A future PR can layer a moslayout-level
type system.

### 3.4 Scoping rules

- `For`'s `as:` and `index:` bindings are visible only inside the
  immediate body. Nested `For`s shadow.
- `If when:`'s expression sees the surrounding scope but does not bind
  new names.
- Slot references (`slot: foo`) always resolve to the host component's
  slot, never to a `For`-bound name.
- A `For`-bound name and a slot of the same name is a **shadow
  warning** in the compiler. (Slots win? `For` wins? — picked by the
  spec PR that lands the resolver.)

### 3.5 Grammar fits the existing parser

These additions are scoped to add three new productions (`for_node`,
`if_node`, `expr`) and an extension to `prop_value` to accept an
`expr` where today it accepts a slot/keyword/string/number/emit. The
existing primitive-tag-as-`NAME` rule is unchanged: `For` and `If` are
just two more primitive names.

---

## 4. Component packages

### 4.1 Package shape

A component package lives at `code/packages/mosaic/mosaic-pkg-{name}/` and
follows the same three-file convention as a normal Mosaic component,
plus a manifest:

```
code/packages/mosaic/mosaic-pkg-grid/
├── mosaic-package.toml        // manifest
├── README.md
├── CHANGELOG.md
├── src/
│   ├── Grid.mil
│   ├── Grid.mll
│   ├── Grid.dark.msl
│   ├── Grid.light.msl
│   ├── Cell.mil
│   ├── Cell.mll
│   ├── Cell.dark.msl
│   ├── Cell.light.msl
│   ├── Column.mil               // metadata-only
│   └── Column.mll
└── tests/
    └── grid_compiles_to_react.rs  // smoke test per backend
```

(All `mosaic-pkg-*` component packages live grouped under `code/packages/mosaic/` —
previously they sat directly under `code/packages/`, alongside the per-language
directories like `code/packages/rust/`; they were moved into their own directory
for the same reason every other same-topic package family gets one.)

### 4.2 Manifest

```toml
# mosaic-package.toml
[package]
name = "mosaic-pkg-grid"
version = "0.1.0"
description = "Spreadsheet-style data grid built on UI29 kernel primitives"
license = "MIT OR Apache-2.0"

[components]
exports = ["Grid", "Cell", "Column"]

[dependencies]
# No other packages — this one is built only from UI29 kernel primitives.
# A future mosaic-pkg-grid-pinned-columns might list `mosaic-pkg-grid = "0.1"`.

[kernel]
version = "1"      // declares which UI29 kernel revision the package targets
```

The manifest is the single source of truth for which components a
package exposes. The compiler resolves `Grid` (in another `.mll`'s
node tag) by looking up which package declares it in `exports`.

### 4.3 Compiling a package

`mosaic-compile pkg <path>` produces, per target backend, a
package-shaped artifact:

- React: a directory of `.tsx` files plus an `index.ts` re-export
  (`mosaic-pkg-grid/dist/react/{Grid.tsx, Cell.tsx, Column.tsx, index.ts}`)
- SwiftUI: a Swift package (`Package.swift` + `Sources/`)
- Qt/QML: a `.qmldir` + per-component `.qml` files
- HTML: a static-HTML snippet bundle
- WebComponent: one file registering all custom elements

Hosts depend on the *compiled artifact*, not the source — packages
ship pre-compiled per-backend output the same way npm/SwiftPM/etc.
ship pre-compiled libraries today. (A source distribution path is also
possible but out of scope for v1.)

### 4.4 Resolving a component reference

When a user's `.mll` contains `Grid (rows: slot: data, ...)`, the
compiler:

1. Looks up `Grid` in the user component's manifest dependencies
2. Verifies `Grid` is in that package's `exports`
3. Reads the package's compiled artifact for the active backend
4. For backends with native composition (React, SwiftUI, QML), emits
   a component-reference (`<Grid rows={data} />` / `Grid(rows: data)`)
5. For backends without (static HTML), inlines the package's lowered tree

The result: backends never see the word "Grid". They see kernel
primitives and component references whose definitions also reduce to
kernel primitives.

### 4.5 Userland packaging conventions

A package may depend on other packages. `mosaic-pkg-data-grid-pro` can
extend `mosaic-pkg-grid` by re-exporting it and adding `PinnedColumn`,
`ColumnGroup`, etc. as new components. Dependency resolution follows
the same shape as Cargo / npm: semver, lockfile, transitive deps.

The deliberate consequence: the Mosaic ecosystem is a normal package
ecosystem. Anyone can publish a component package. The framework
core (this repo) ships only the kernel and a tiny set of
reference-implementation packages — likely `mosaic-pkg-grid` and
`mosaic-pkg-input` — so demos can show the architecture working
without forcing the kernel to absorb them.

---

## 5. Migration: what happens to the existing code

### 5.1 The current React Grid v3 code (PR #3603, merged)

The `emit_grid_jsx`, `emit_grid_jsx_v3`, `emit_input_jsx`,
`emit_cell_jsx_standalone`, `emit_column_jsx_standalone`, and
`build_grid_cell_style_expr` functions in `mosaic-emit-react/pipeline.rs`
become **dead code** once the kernel emitter + component resolution
ship. They are removed in a sweep PR (UI29-cleanup) after the first
userland package proves the architecture end-to-end.

The interim: while both code paths exist, the Grid/Cell/Column/Input
tags in `.mll` files continue to hit the bespoke React lowering. New
demos opt into the userland-package path by:

1. Adding a manifest dependency on `mosaic-pkg-grid`
2. Compiling with `mosaic-compile --kernel-only` (or whatever flag the
   resolver uses to refuse-bespoke-lowerings)

### 5.2 code/programs/typescript/visicalc

The VisiCalc demo migrates as part of proving `mosaic-pkg-grid`. Its
`Grid.mil`/`Grid.mll`/`Grid.dark.msl` files are *deleted* — they are
replaced by an import of `mosaic-pkg-grid`'s `Grid` component, used
by reference in the demo's top-level layout. The demo's host code
(`App.tsx`, the reducer) is unchanged.

### 5.3 mosaic-emit-webcomponent and mosaic-emit-html

These two backends still use the legacy single-file `.mosaic` source
path; they were never ported to the three-language pipeline. Under
UI29 they get a fresh pipeline implementation that targets the kernel
directly. Their old bespoke Grid lowerings (the `case "Grid"` arms in
`lib.rs`) are removed at the same time.

### 5.4 SwiftUI and Qt skeletons (PRs #3607, #3608)

These two crates currently support `Box`, `Row`, `Column`, `Text`,
`Spacer`, `Image`, `Divider`, `Icon`. Under UI29 they grow `Stack`,
`If`, `For`, `HostInput`, `HostButton`, `HostTable`, `HostScroll` —
six more primitives each, one PR per primitive (or one PR per
backend covering several).

Once both skeletons have full kernel coverage, the `--backend swiftui`
and `--backend qt` flags wire into `mosaic-compile` (was WB6;
renumbers to U29-CLI).

---

## 6. Implementation roadmap

| ID | Work | Depends on |
|---|---|---|
| U29-0 | This spec | — |
| U29-G1 | moslayout grammar: `For each: as: index: { ... }` | U29-0 |
| U29-G2 | moslayout grammar: `If when: { } else { }` | U29-0 |
| U29-G3 | moslayout grammar: `expr` non-terminal | U29-0 |
| U29-R1 | mosaic-compile: package manifest + `mosaic-package.toml` parser | U29-0 |
| U29-R2 | mosaic-compile: component-reference resolver | U29-R1, U29-G3 |
| U29-R3 | mosaic-compile: per-backend package-artifact build mode | U29-R1 |
| U29-K-react | mosaic-emit-react: lower `If` / `For` / `HostInput` / `HostButton` / `HostTable` / `HostScroll` | U29-G1, U29-G2, U29-G3 |
| U29-K-swiftui | mosaic-emit-swiftui: same six kernel primitives | same |
| U29-K-qt | mosaic-emit-qt: same six kernel primitives | same |
| U29-K-webcomp | mosaic-emit-webcomponent: pipeline path + same | same |
| U29-K-html | mosaic-emit-html: pipeline path + same | same |
| U29-P1 | First userland package: `mosaic-pkg-grid` (Grid, Cell, Column) | U29-R2, all U29-K-* |
| U29-D1 | Migrate `code/programs/typescript/visicalc` to consume `mosaic-pkg-grid` | U29-P1 |
| U29-X1 | Sweep PR: remove dead Grid/Cell/Column/Input code from each backend | U29-D1 |

**Parallelism:** U29-G1 / U29-G2 / U29-G3 are sequential (each touches
grammar). U29-R1 / U29-R2 are sequential. The U29-K-* family is fully
parallel once grammar is in. U29-P1 fans in from all of them.

---

## 7. Relationship to other specs

- **UI14 moslayout**: §11 (primitive vocabulary) is reframed — the
  vocabulary is now exactly the 15 kernel primitives. Grid, Cell,
  Column, Input become userland.
- **UI24 emit→dispatch**: unchanged. The Flux pattern flows through
  component packages identically.
- **UI25 Input**: the kernel's `HostInput` *is* what UI25 specified;
  the rich `Input` (placeholder, validation, multiline, maxLength) is
  the first candidate after `mosaic-pkg-grid` for promotion to a
  userland package, e.g. `mosaic-pkg-input`.
- **UI26 VisiCalc**: the demo migrates per §5.2.
- **UI27 Grid v2**: the sub-part / state-suffix patterns it introduced
  remain valid — they apply to whichever component (kernel or
  userland) declares the sub-parts. Nothing in UI27's mosstyle work is
  invalidated.
- **UI28 Grid v3 Cell-centric**: largely *deprecated* by this spec. The
  Cell-centric decomposition idea is sound; UI28's mistake was placing
  the decomposition logic inside each backend. UI29 keeps the
  decomposition and moves it to a userland package.

---

## 8. Open questions (resolved in implementation PRs)

1. **For-bound shadowing**: does a `For` binding shadow a slot of the
   same name, or is shadowing an error? Spec-level recommendation:
   shadow with a compiler warning.
2. **Else-If chains**: parser-level rewrite to nested `Else { If ... }`?
   Or first-class `Else If` token? Recommendation: parser rewrite, no
   new token.
3. **Static HTML and slot-driven `For`**: the static-HTML backend can't
   render a `For` over a slot whose value is unknown at compile time.
   Recommendation: emit a server-rendered placeholder and document the
   constraint.
4. **Manifest dependency resolution**: lockfile format? Recommendation:
   `mosaic-package.lock` mirroring Cargo's shape.
5. **`HostTable` row/header semantics**: `HostTable` itself probably
   has named slots for `<thead>` / `<tbody>` / `<colgroup>` content.
   Spec the exact slot list in U29-K-react.
6. **Theming across packages**: if `mosaic-pkg-grid` ships a default
   `.dark.msl`, can a host override individual parts? Recommendation:
   yes, via a CSS-style cascade — host's mosstyle wins on collisions
   with the package's defaults.

These are deliberately deferred to keep this spec focused on the
architecture rather than every detail.

---

## 9. What this spec does *not* do

- It does not implement anything. Every entry in §6 is a follow-up PR.
- It does not freeze the kernel forever. The kernel grows by spec
  (UI29-1, UI29-2, ...) but only when a primitive genuinely meets the
  inclusion criteria in §2.2.
- It does not opine on package-registry hosting (npm vs Cargo vs a
  Mosaic-specific registry). That's a separate spec.
- It does not specify the visual / interaction design of any component
  package. `mosaic-pkg-grid` and its peers get their own specs.

The architecture is the contribution. The code follows.
