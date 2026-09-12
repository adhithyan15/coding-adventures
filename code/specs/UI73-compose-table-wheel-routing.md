# UI73 — Compose routes the table wheel

Issue: [#14843](https://github.com/adhithyan15/coding-adventures/issues/14843)

VisiCalc's **last** Compose degradation. Landing this makes it native-complete
on Compose, which would be the first time all three desktop products are.

```
interaction.table-wheel-shift-unimplemented
  measured table wheel routing is currently implemented only by React
```

Unlike the other four entries in that report — which were false positives,
fixed in #14954 — this one is accurate. `grep onViewportShift` in
`mosaic-emit-compose` returns **zero**. Nothing is emitted, so the wheel does
nothing over a virtualised table.

## 1. What is authored

`mosaic-pkg-grid`'s `RowHeaderGrid.mll` declares the whole contract on the
table:

```
HostTable [sheet] (
  ..., onViewportRows: emit: onViewportRows,
  viewport-offset: slot: viewport-offset,
  total-rows: slot: total-rows,
  onViewportShift: emit: onViewportShift )
```

`onViewportShift ( rows : number )` — one numeric parameter, a **signed row
delta**, not an absolute offset.

## 2. What React does, measured

From `mosaic-emit-react/src/table_capacity.ts`, the behaviour to match:

| rule | React |
| --- | --- |
| axis | ignores the event when the horizontal delta dominates (`abs(deltaX) >= abs(deltaY)`) |
| modifiers | ignores ctrl / meta / shift (zoom and pan, not scroll) |
| clamp | `limit = max(0, total - renderedRows)`; ignores wheel-up at `offset <= 0` and wheel-down at `offset >= limit`, and **resets the accumulator** when it does |
| units | converts pixels to rows by dividing by the measured row pitch |
| accumulation | keeps the fractional remainder across events, and **discards it on a direction change** (`sign(previous) === sign(amount) ? previous : 0`) |
| dispatch | only when the truncated row count is non-zero |

The accumulator is the part worth copying deliberately: without it a trackpad
delivering sub-row deltas would either scroll nothing or jump a whole row per
event.

## 3. What Compose can do — verified against the toolchain

This repo pins `org.jetbrains.compose` **1.6.11**. Before designing around it,
the API was compiled in a real generated VisiCalc project:

```kotlin
@file:OptIn(androidx.compose.ui.ExperimentalComposeUiApi::class)
Modifier.onPointerEvent(PointerEventType.Scroll) { event ->
    val delta = event.changes.first().scrollDelta.y
    ...
}
```

`gradle compileKotlin` — **clean**, no deprecation. So
`androidx.compose.ui.input.pointer.onPointerEvent` and `scrollDelta` are
available at the pinned version.

The opt-in goes into the file's **single merged** `@file:OptIn(..)` list
(#14964), not a second annotation — `@file:OptIn` is not repeatable.

## 4. The difference that matters

**Compose's `scrollDelta.y` is already in line units, not pixels.** A wheel
notch is ±1.0. React divides `deltaY` by a measured row pitch precisely because
the DOM reports pixels; Compose needs no pitch and no measurement pass.

That removes the most fragile part of the React implementation. It also means
the Compose version cannot reuse React's `deltaMode` branch, which exists only
to normalise the three DOM delta modes.

Trackpads still deliver fractional line deltas, so the **accumulator stays**.

## 5. Proposed lowering

On a `HostTable` that carries `onViewportShift` together with `viewport-offset`
and `total-rows`:

```kotlin
var wheelRows by remember { mutableStateOf(0f) }
Modifier.onPointerEvent(PointerEventType.Scroll) { event ->
    val change = event.changes.first()
    val amount = change.scrollDelta.y
    if (amount != 0f && kotlin.math.abs(change.scrollDelta.x) < kotlin.math.abs(amount)) {
        val limit = maxOf(0, totalRows - renderedRows)
        if (!((amount < 0f && viewportOffset <= 0) || (amount > 0f && viewportOffset >= limit))) {
            val accumulated =
                (if (wheelRows.sign == amount.sign) wheelRows else 0f) + amount
            val rows = accumulated.toInt()
            wheelRows = accumulated - rows
            if (rows != 0) dispatch(Event.ViewportShift(rows))
        } else {
            wheelRows = 0f
        }
    }
}
```

`renderedRows` is the size of the collection the table's `For` iterates, which
the emitter already knows the expression for — it emits `onViewportRows` from
the same measurement today.

## 6. Acceptance

- A `HostTable` with the three props emits an `onPointerEvent(PointerEventType.Scroll)`
  handler dispatching the `onViewportShift` event; one **without** them emits no
  handler at all, asserted as a pair so a predicate that never fires cannot pass.
- The emitted Kotlin **compiles**: verified by building the generated VisiCalc
  project, not by grepping it (#14964's lesson).
- `interaction.table-wheel-shift-unimplemented` disappears from VisiCalc's
  Compose degradation report, and the report is checked to be **co-total** with
  what the emitter emits rather than edited to match.
- VisiCalc's render script pin moves from 1 to **0**, and the script switches
  from pinning a count to asserting `nativeComplete` — the pin exists only
  because there was something to pin.
- Trestle and Engram still generate `native-complete` with 0, since the
  predicate must not fire on tables that lack the props.

## 7. Out of scope

The **scroll-position** half of React's helper — `scrollFrame.scrollTop`, the
spacer rows before and after, and the `requestedOffset` round-trip that keeps a
real scrollbar in sync with a virtual window. Compose's table is not inside a
scrollable frame today, so there is no scrollbar to reconcile. Wheel routing is
useful on its own and is what the degradation names; conflating the two would
make this change unlandable.

Also out of scope: horizontal wheel routing. Nothing authors it, and React
explicitly ignores the horizontal axis.
