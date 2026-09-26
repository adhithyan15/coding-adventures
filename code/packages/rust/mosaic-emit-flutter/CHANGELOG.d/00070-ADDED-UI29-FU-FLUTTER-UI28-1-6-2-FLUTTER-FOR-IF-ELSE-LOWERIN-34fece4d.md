### Added — UI29-FU-flutter / UI28-1 §6.2 — Flutter `For` / `If` / `Else` lowering

Replaces the v0.1 placeholder (`/* TODO: For not yet wired in the Flutter
emitter */ const SizedBox.shrink()`) with real Dart lowering for all
three control-flow primitives. Required by `mosaic-pkg-grid` v0.2.0 per
[UI28-1 §6.2](../../../specs/UI28-1-grid-v3-userland-revised.md).

- **`For`** lowers to a `Column(children: ...)` whose list is built
  by `.map(...).toList()` over the iterated collection:
  - `For ( each: slot: X , as: y )` → `Column(children: x.map((y) => <body>).toList())`
  - `For ( each: slot: X , as: y , index: i )` → `Column(children:
    x.asMap().entries.map((entry) { final i = entry.key; final y =
    entry.value; return KeyedSubtree(key: ValueKey(i), child:
    <body>); }).toList())` — the `KeyedSubtree(ValueKey(i))` wrapper
    gives Flutter's element tree the stable identity Flutter's diff
    needs when rows reorder (matches React `key={i}` and SwiftUI
    `id: \.offset`; UI28-1 §5 performance property).
  - `each:` accepting `SlotRef` or `Expr` — Expr text passes through
    verbatim (author-controlled, matches React/SwiftUI/Qt behaviour).
  - `as:` / `index:` kebab-case names lower via `to_camel_case_first_lower`
    so the emitted Dart bindings are valid identifiers.
- **`If`** lowers to a Dart ternary returning a Widget:
  - `If { then }` → `((cond) ? <then> : const SizedBox.shrink())`
  - `If { then } Else { e }` → `((cond) ? <then> : <else>)`
  - `when:` accepting `SlotRef` (lowered via camelCase) or `Expr`
    (verbatim). Empty branches collapse to `const SizedBox.shrink()`
    so the ternary always returns a concrete Widget.
- **Sibling pairing** — `emit_paired_children` walks each container's
  children with peek-ahead so an `If` followed by `Else` fuses into a
  single ternary. Orphan `Else` (analyzer should reject) renders a
  documenting comment instead of crashing. Pattern mirrors the SwiftUI
  backend's `emit_children`. `HostScroll` and `HostTooltip`'s
  multi-child paths also route through the paired walker.

8 new tests cover the lowering shapes: unindexed `For`, indexed `For`
with `KeyedSubtree`, `Expr`-as-each pass-through, kebab-case binding
camel-casing, standalone `If` with empty-else SizedBox fallback,
paired `If`/`Else` inside a Box (Cell.mll shape), `Expr` `when:`
(verifies the Cell.mll predicate `cellRow == editRow && cellCol ==
editCol` lowers without UI29 §3.4 since slots are in scope), empty
`For` body, and nested `For` with `Expr` inner each (the v0.2.0 Grid
composition shape). Total tests: 57 (was 49, +8).

