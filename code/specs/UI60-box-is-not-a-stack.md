# UI60 — `Box` is a flow container, not a z-axis stack

Issue: [#14828](https://github.com/adhithyan15/coding-adventures/issues/14828)

**Status: implemented.** Option (1) was taken. Two things the spec did not
anticipate, both recorded in §8 below.

Extends [UI29](UI29-primitive-kernel.md)'s primitive kernel. `mosaic-emit-compose`
gives `Box` the overlay behaviour the kernel assigns to `Stack`, so a `Box` with
two or more children draws them on top of one another.

## 1. The defect

Engram's deck-stat chips render the count over the label — `0` drawn on top of
`Total`, `New`, `Note types`. Measured on the rendered app:

```
NODE [0]      bounds=Rect.fromLTRB(49.0, 209.0, 60.0, 233.0)
NODE [Total]  bounds=Rect.fromLTRB(49.0, 209.0, 89.0, 233.0)
```

Identical origin. The source is ordinary — a `Box` with two `Text` children:

```
Box [ deck-stat-total ] {
  Text [ deck-total-value ] ( content : slot: total-value )
  Text [ deck-total-label ] ( content : slot: total-label )
}
```

## 2. Why it is a kernel-conformance bug

The emitter lowers both primitives identically:

```rust
"Box" | "Stack" => emit_container(…)   // → Compose's Box, which overlays
```

UI29's own table disagrees:

| primitive | meaning | html | SwiftUI | Qt |
| --- | --- | --- | --- | --- |
| `Box` | **generic opaque container** | `<div>` | `Group { }` | `Item { }` |
| `Stack` | **z-axis / absolute container** | `position: relative` | `ZStack { }` | `Item { anchors.fill }` |

Confirmed with a minimal two-child `Box` probe:

| backend | emits | children |
| --- | --- | --- |
| **compose** | `Box(` | **overlaid** |
| swiftui | `Group {` | normal flow |
| html | `<div>` | block flow |
| qt | `Item {` | normal flow |

Compose is the only backend giving `Box` `Stack` semantics — and the same
emitter's comments elsewhere state that `Stack` "is *not* a synonym" for the
vertical container, so the distinction is understood in the file that gets it
wrong.

## 3. The blast radius is much larger than the visible defect

This is the reason the change needs specifying rather than patching.

Measured across the products:

| product | `Box` nodes | with 2+ children | shared text origins in the render |
| --- | --- | --- | --- |
| Engram | 22 | 16 | **yes** — the stat chips |
| Trestle | 3 | 3 | **none** (24 texts, 0 shared origins) |
| VisiCalc | 0 | 0 | none |
| Venture | 0 | 0 | none |

Trestle has multi-child `Box`es and renders correctly, because its children are
conditionals — only one renders at a time. So changing `Box`'s lowering touches
every `Box` in every package while fixing a defect that is *visible* in one.
A regression here would be silent in exactly the places that look fine today.

## 4. The interaction that rules out a naive swap

`Box` currently emits `contentAlignment`, derived from `text-align`:

```kotlin
Box(modifier = …, contentAlignment = Alignment.CenterEnd) { … }
```

`contentAlignment` is a **`Box`-only** argument in Compose. Swapping the
composable to `Column` without also mapping that argument produces Kotlin that
does not compile — and `text-align` on a `Box` is authored in real packages
(`mosaic-pkg-grid`'s body cells, among others).

So any fix must decide what `text-align` means on a flow container:
`horizontalAlignment`, a `wrapContentWidth` alignment, or something per-child.

## 5. Options

1. **`Box` → `Column`, mapping `contentAlignment`.** Correct per UI29 and
   matches `<div>` block flow. Requires translating `text-align` to
   `horizontalAlignment` and deciding vertical behaviour.
2. **Keep Compose `Box` for 0–1 children, `Column` for 2+.** Minimal visible
   risk — a single-child `Box` keeps today's behaviour exactly, including
   `contentAlignment`. But the same primitive then behaves differently by child
   count, which is not a semantics anyone can reason about.
3. **Leave `Box` and fix the authoring.** Change Engram's chips to `Column`.
   Smallest change, and wrong: it treats a kernel divergence as a per-package
   problem, and the next multi-child `Box` reintroduces it.

(1) is the answer. (2) is worth naming only to reject it explicitly.

## 6. Acceptance

- A two-child `Box` lays its children out in flow, asserted on **bounds**:
  the second child's top must be greater than or equal to the first child's
  bottom, not equal to its top.
- `text-align` on a `Box` still positions its content, with a test naming the
  emitted argument.
- Before/after renders of **Engram** through the screenshot harness (#14799) —
  the stat chips are the visible case.
- Before/after renders of **Trestle**, whose `Box`es render correctly today: a
  change to every `Box` must be shown not to disturb them.
- `Stack` continues to overlay, with a test that distinguishes it from `Box`.
  The two lowering to the same thing is the bug; a fix that leaves them
  identical has not fixed it.

## 7. Out of scope

Other backends. swiftui, html and qt already lay `Box` children out in flow, as
measured above. No change is required there, and making one without a
measurement would be guessing.

## 8. What implementation found that the spec did not

### The composable is chosen in TWO places

`emit_node`'s `match node.tag` is the obvious one. `root_container_context`
is the other, and it decides the composable for a component's ROOT node. A fix
to only the first would have left a root `Box` overlaying while every nested
one flowed — from one source tree, with no test able to see the difference
from a single-node fixture.

### A root `Stack` was not overlaying at all

Found by the same audit, and a separate bug: `root_container_context` never
named `Stack`, so it fell through to `_ => ("Column", ..)`. A root-level
`Stack` laid its children out in flow — the exact **opposite** of a nested
`Stack`, in the same emitter. The defect this spec describes and its own
mirror image were both live, on the same primitive pair.

### `text-align` was already emitting Kotlin that did not compile

§4 predicted that swapping the composable would invalidate `contentAlignment`.
It was worse than that: `contentAlignment` was applied with no check on the
composable at all, so `text-align` on a `Row` already emitted
`Row(contentAlignment = ..)`, which kotlinc rejects. Latent only because every
shipped package authored `text-align` on a `Box`. Fixed first, as #14839.

## 9. Acceptance, as measured

| requirement | result |
| --- | --- |
| two-child `Box` flows, asserted on bounds | Engram `[0]` bottom `271.0`, `[Total]` top `271.0` — was an identical origin of `247.0` for both |
| `text-align` still positions content, naming the argument | `horizontalAlignment = Alignment.End` on the grid body cell (`text-align: right`) |
| before/after renders of Engram | all eleven chips read as `0` above their label instead of over it |
| before/after renders of Trestle | all three renders **byte-identical**; 27 `Box`→`Column` swaps, zero pixel change — its multi-child `Box`es render one child each, as §3 predicted |
| `Stack` still overlays, distinguished from `Box` | asserted as a pair; identical output for the two is itself the failure |
