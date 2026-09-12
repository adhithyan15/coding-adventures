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

## 8. Measured before implementing — the acceptance in §6 cannot be met

Three mechanisms were built and measured against the real generated Trestle
app. All three fixed the chip. None produced the layout §6 asks for, and the
reason is not the mechanism.

### The three attempts

| | `summary` | `On track` chip |
| --- | --- | --- |
| today | `185 x 48` (2 lines) | **`0 x 168`** |
| `weight(1f, fill = false)` on every Row-child `Text` | `92 x 144` (**6 lines**) | `56 x 24` |
| `weight(1f)` on the first Row-child `Text` only | **`0 x 744`** | `156 x 24` |

The second looked like the answer and is not: Compose does not redistribute
what a `fill = false` child leaves unused, so each `Text` is capped at an equal
share whatever its content. The third moves the starvation onto the summary
instead of the chip. Every mechanism chooses *who* starves.

### Why — the topbar is over-subscribed by ~587px

The same app measured at two viewports, all children of the topbar row:

| | 1280 x 900 | 1900 x 900 |
| --- | --- | --- |
| `Tasks — auto-scheduled` | `185 x 48` | `196 x 24` |
| summary | `185 x 48` | `302 x 24` |
| chip container | `0 x 168` | `67 x 24` |
| rightmost element ends at | `1247` | **`1867`** |

At its natural size the topbar needs **1867px**. At 1280 it is **587px**
short. §6 asks for `summary` on one line *and* a non-zero chip — that is the
1900 layout, and it does not exist at 1280 while the row also carries a title,
a progress readout, a theme toggle, a Board button and a 474px five-button
segment control.

### What this changes

1. **The rule in §4 stands.** A zero-width child is never right, and Compose
   starving the last-measured sibling is a real divergence from the CSS
   default the `.mll` was authored against.
2. **The acceptance in §6 does not.** "Its text renders on one line" is a
   claim about available space, not about shrink semantics, and no shrink rule
   can satisfy it at 1280. It should become: *no child measures zero, and the
   row's content is legible at 1280* — which the `fill = false` variant nearly
   meets, at six lines of summary.
3. **The 587px is its own defect** ([#14847](https://github.com/adhithyan15/coding-adventures/issues/14847)).
   The topbar has to wrap, scroll or shed content at narrow widths. Until it
   does, UI59 is choosing which child absorbs a deficit that should not exist
   — worth doing, because "everything visible and cramped" beats "one thing
   invisible", but it is mitigation.

Implementation is therefore sequenced **after** #14847, so the shrink rule is
chosen against a row that fits rather than one that cannot.

### Method note

None of this was visible from the emitted source, and none of it was visible
from a screenshot: the chip renders as *nothing at all* once
`Modifier.clip(..)` is in play. Each row above is `SemanticsNode.size` and
`positionInRoot` from a Compose test rendering the real app against the real
Rust runtime. The `fill = false` variant in particular measured "correct" on
the chip and would have shipped as a fix had the summary not also been
measured.
