# UI37 — generic-container payload dispatch

**Status:** design + React backend implementation.
**Kernel surface:** any generic container (`Box`, `Row`, `Column`, `Stack`) that
declares an `onClick`-shaped emit-ref prop, when the target emit has declared params.

---

## 1. The gap

`mosaic-emit-react`'s connects-wiring (UI24) has two tiers:

- **Dedicated primitives** (`HostButton`, `HostInput`, `HostLink`) get bespoke
  handlers that can attach a payload — `HostButton` synthesizes an `index` field from
  the nearest enclosing `For`'s index binding; `HostInput`'s `onChange` attaches
  `e.target.value`; `onCommit` attaches `e.currentTarget.value` when the target emit
  declares a param (fixed alongside this spec — see `mosaic-emit-react`'s CHANGELOG).
- **Everything else** (`Box`, `Row`, `Column`, `Stack`, and any future generic
  container) goes through `build_emit_handlers`, which dispatches `{ type: "..." }`
  and nothing else — no matter what the target emit declares.

This second tier is not a corner case. `mosaic-pkg-grid`'s `Cell` is a `Box` (chosen
deliberately — a table cell is not semantically a button), and `Grid.mil` declares
`onNavigate(row: number, col: number)`. Every consumer of `Grid` inherits a contract
that **cannot be fulfilled**: a click fires (as of the `Cell.mll` wiring fix, also
alongside this spec), but always void. `mosaic-pkg-sheet`'s v1 shipped read-only
specifically because of this gap (see its CHANGELOG/README).

The underlying reason dedicated primitives *can* attach a payload and `Box` can't is
architectural, not incidental: `for_payload: Option<ForPayloadScope<'_>>` threaded
through `emit_jsx_tree` tracks only the **single innermost** enclosing `For`'s
`item`/`index` bindings. `HostButton`'s index synthesis reads that one scope. `Grid`'s
`onNavigate` needs **two** nested scopes at once (`row` from the outer `For`, `col`
from the inner) — which `ForPayloadScope` cannot represent even for the primitives
that already get dedicated treatment. Generalizing loop-scope tracking into a real
stack would fix this at the root, but is a substantially larger, higher-risk change
(every `for_payload` call site in the recursive emit chain), deferred — see §5.

## 2. The fix: payload from named, author-supplied props

UI35's drag family already solved an equivalent problem a different way:
`HostDropTarget`'s `drop-key` isn't synthesized from loop context at all — it's an
**author-supplied prop**, resolved by `drag_value_expr` (literal / slot-ref /
expression, the last being exactly `( col[1] )` from inside a `For`), and the
resolved expression is embedded directly in the dispatch. No loop-scope tracking is
needed because the author already named which loop-bound value to use.

UI37 generalizes that pattern to any generic container's `onClick`:

```mll
Box [ cell ] (
  onClick : emit: onClick ,
  row      : ( r ) ,
  col      : ( c ) ,
)
```

When a generic container's emit-ref prop (`onClick`, or any future analog) targets an
emit whose declared params are non-empty, the emitter looks for a **prop on the same
node named after each declared param** (`row`, `col`, …). Each is resolved the same
way `drag_value_expr` already resolves `drop-key` — literal, slot-ref, or expression —
and included in the dispatch object under that param's camelCased name:

```jsx
onClick={() => dispatch({ type: "navigate", row: r, col: c })}
```

A declared param with **no matching prop on the node** is a hard compile error
(`PipelineEmitError`), not a silent `undefined` — the same "accepted-but-ignored is
the worst outcome" principle UI36 established for size props. A **void** emit (no
declared params) is unaffected: `build_emit_handlers`'s existing behavior is exactly
right for it already.

## 3. Applying the fix to `Grid`/`Cell`

- `Cell.mil`: add `slot row : number` and `slot col : number` alongside the existing
  `is-editing`/`is-selected` coordinate-adjacent slots. `emit onClick` gains the
  `(row: number, col: number)` payload it always should have had.
- `Cell.mll`: thread the new slots onto `Box[cell]` as literal props (`row: slot:
  row`, `col: slot: col`) alongside the existing `onClick: emit: onClick`.
- `Grid.mll`: at the `Cell(...)` call site, supply `row: ( r )`, `col: ( c )` — the
  exact same expression-in-slot-binding shape already used for `is-editing`/
  `is-selected` two lines above.
- `Sheet.mil`/`Sheet.mll`/`TaskApp.mil`/`TaskApp.mll`: `onNavigate` regains its
  `(row, col)` payload the whole way up the forwarding chain; the "declared void
  because nothing could deliver it" workaround is reverted.
- `main.tsx`: the sheet's `onNavigate` handler is re-wired for real (enter edit mode
  on an editable column, resolve the clicked task id via the ordered row list — the
  design was already built and stripped back to a no-op for the v1 read-only ship,
  see `SHEET_FIELDS`' `editable`/`write` fields).

## 4. Backend status

Implemented on **React** only, matching UI35's rollout shape (react + html
implement the interaction; other backends degrade). Backends that don't implement
this get the pre-existing behavior unchanged: `Box`'s generic `onClick` dispatches
void there too today, so there is nothing to regress — this spec only adds
capability, on the one backend task-app's web host actually uses.

## 5. What this does NOT fix (deferred)

- **The general N-level-nested-loop case for `HostButton`-style index synthesis.**
  UI37's named-prop mechanism sidesteps needing a loop-scope stack for `Box`, but
  `HostButton` inside doubly-nested `For`s still can't synthesize two index values —
  only ever the innermost. If a future primitive needs that, the `Option<
  ForPayloadScope>` → `Vec<ForPayloadScope>` generalization is still the real fix;
  UI37 does not attempt it.
- **Non-React backends.** HTML, SwiftUI, Qt, Flutter, Compose, WebComponent, XAML all
  keep today's void-dispatch behavior for generic containers.
- **A general `connects: onX(p) -> emit onY(p: p)` mapping syntax** in moslayout
  (noted as a known limitation in `emit_input_jsx`'s own doc-comment, predating
  this spec) — UI37's named-prop convention is narrower and purpose-built for the
  loop-coordinate case, not a replacement for that broader idea.
