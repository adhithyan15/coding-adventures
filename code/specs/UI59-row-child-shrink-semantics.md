# UI59 — a Row child that can shrink must yield to one that cannot

Issue: [#14815](https://github.com/adhithyan15/coding-adventures/issues/14815)

Extends [UI29](UI29-primitive-kernel.md)'s primitive kernel. Mosaic's `Row` is
authored against CSS flexbox semantics, and one of those semantics — that
children shrink before they starve a sibling — is not reproduced on Compose.

## 1. The defect this settles

TaskApp's schedule-status chip renders as a **zero-width** element:

```
Row [ subline ] {
  Text [ summary ]                  // "1 task(s) · 0 done · projected finish 2026-01-05"
  Row  [ pill-ok ] { dot; Text }    // "On track"
}
```

Measured on the real generated Compose app, after adding one task:

| viewport | `summary` | `On track` chip |
| --- | --- | --- |
| 1280 x 900 | 276 x 48 (wrapped to two lines) | **0 x 168** |
| 1900 x 900 | 385 x 24 (one line) | 53 x 24 |

The chip is correct when there is room and collapses to nothing when there is
not. Note the summary *did* wrap at 1280 and still starved its sibling, so
wrapping alone does not resolve the contention.

## 2. Why it is a semantic gap, not an authoring error

Nothing is authored wrongly. `pill-ok` declares no width and should not have
to: in CSS flexbox both children carry `flex-shrink: 1` by default, so a narrow
row shrinks the long text and both stay visible.

That the web backends really do this was measured, not assumed — a minimal
two-child `Row` emits a genuine flex container on each:

| backend | emitted |
| --- | --- |
| html | `style="display: flex; flex-direction: row; gap: 8px"` |
| webcomponent | `style="display: flex; flex-direction: row;; gap: 8px"` |
| react | `display: "flex", flexDirection: "row"` |

so CSS's default `flex-shrink: 1` applies and neither child is starved. That is
the behaviour the `.mll` was written against.

Compose's `Row` measures children **in order** against the remaining
constraints. The first child takes what it asks for; later siblings get what is
left, which here is nothing.

So the same authored component is correct on four backends and broken on one,
with no per-backend authoring to explain it.

## 3. Scope

This is not one chip. `flex-shrink` is among the properties Compose discards —
25 occurrences in TaskApp ([#14810](https://github.com/adhithyan15/coding-adventures/issues/14810)) —
but the defect does not require anyone to author it: the CSS **default**
differs, so *every* `Row` containing a text that can wrap beside a
fixed-size sibling is exposed. The chip is simply the instance that is visible.

## 4. The rule

> In a `Row` with two or more children, a child whose content can shrink must
> yield space to one whose content cannot, rather than consuming the row and
> leaving siblings at zero.

Compose's mechanism for this is `Modifier.weight(1f)` on the yielding child.
The emitter already maps `flex-grow` to weight (`compose_row_weight`, with a
test), so the machinery exists — what is missing is the default.

## 5. What must be decided before implementing

This changes every emitted `Row`, so it is deliberately not a patch.

1. **Which child yields.** The simplest rule — "a `Text` sibling of a non-`Text`
   yields" — is wrong for a row of two Texts, and wrong when the fixed-size
   sibling is itself the one that should compress. An explicit authored
   `flex-shrink` should win over any heuristic.
2. **Interaction with `weight` already emitted.** A child that already has a
   weight from `flex-grow` must not gain a second.
3. **Interaction with `Arrangement.spacedBy`.** `gap` (#14804) occupies the
   Row's arrangement argument; a yielding child changes measurement, not
   arrangement, so these should compose — but that must be verified, not
   assumed, because both land in the same opener.
4. **Whether `flex-shrink: 0` is honoured.** If the default becomes "shrink",
   then `flex-shrink: 0` becomes the meaningful authored value and must stop
   being discarded.

## 6. Acceptance

- The `On track` chip reports a **non-zero width** at 1280 x 900 with one task.
- Its text renders on one line.
- A test asserts the width is non-zero rather than that the node exists:
  `size = 0 x 168` satisfies every existence and semantics assertion in the
  current suite, which is how this survived undetected.
- Before/after renders through the screenshot harness
  ([#14799](https://github.com/adhithyan15/coding-adventures/issues/14799)), because
  a change to every `Row` cannot be validated by emitted-source diffing.
- No regression in `row_children_use_weight_and_intrinsic_measurement_in_split_sections`,
  which pins the existing intrinsic-width behaviour.

## 7. Explicitly out of scope

Other backends. Qt, SwiftUI, Flutter and XAML each have their own answer to
this and have not been measured — only Compose has been shown to have the
defect. Fixing it generically across the emitters without that measurement
would be guessing.
