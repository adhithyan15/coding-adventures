# Changelog

All notable changes to `mosaic-emit-compose` are documented here.
This project follows [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added — a HostTable's authored focus and accessible name (#14843)

`accessibility.table-focus-unimplemented` was gated on `backend != React`
with **no per-backend question at all** — unlike
`accessibility.table-semantics-missing` next door, which asks one. So it said
nothing about what any other backend could express, and Compose already emits
both mechanisms elsewhere in the same file.

- `a11y-label` joins the table's existing collection-semantics block as
  `contentDescription`. One block, not two: two `semantics { }` modifiers on
  the same node do not merge — the later replaces the earlier — and the
  collection info is the half that would be lost.
- `focusable: true` emits `Modifier.focusable()`. `false` emits nothing, since
  it is the default and the call would change nothing.
- `host_table_has_focus_semantics` is the per-backend predicate the reporter
  now asks, so the report cannot drift from what is emitted.

**The `focusable` import was inside the drag-and-drop block.** A table
authoring `focusable: true` therefore emitted the modifier with no import, and
the generated Kotlin did not compile. The emitter tests were perfectly happy;
only `gradle compileKotlin` caught it — the same shape as the XAML `Not()`
helper (#14793) and `fillMaxSize` (#14798). It is unconditional now, with a
test that asserts the import on a component with no drag-and-drop in it.

VisiCalc's degradations go **7 → 5**, and its render-harness pin moves with
them. Engram and Trestle stay at 0. Falsified: with the predicate disabled the
count returns to 7.

### Fixed — a row-header table got no collection semantics at all (#14843)

`compose_semantic_table_shape` required **exactly one** child per header and
body row. `Grid` has that shape; `RowHeaderGrid` does not — each of its rows
opens with a fixed cell (the corner, and the row-header) before the `For`.

VisiCalc uses `RowHeaderGrid`, and it is the only `HostTable` in the product.
So the table was not recognised at all: no `collectionInfo`, no
`collectionItemInfo`, and `accessibility.table-semantics-missing` in the
degradation report. Every accessibility gate passed the whole time, because
each cell existed and was correctly named — a screen reader simply had no way
to know it was a table.

The predicate now accepts an optional leading `Box` before the `For`, and the
counting follows:

- `columnCount` becomes `columnHeaders.size + 1` — the fixed column is a real
  column, and reporting one fewer than each row has cells is worse than
  reporting nothing.
- Every `For`-produced cell's `columnIndex` shifts by one, since the loop index
  counts *data* columns from zero.
- Both offsets are conditional. A plain `Grid` keeps unoffset indices, pinned
  by a test — widening that silently would move every cell in every plain
  table one column right.

A **ragged** table — a corner cell in the header but not in the body rows, or
the reverse — is rejected rather than indexed. Half its cells would be one
column off, which is worse than reporting no semantics.

VisiCalc's degradations go **8 → 7**, and its render harness pin moves with
them. The remaining seven are `table-focus` (2), `table-wheel-shift` (1) and
`authored-table-cell` (4), which are gated on `backend != React` with no
per-backend predicate at all — a different problem from this one.

### Fixed — `max-width` reached nothing (#14833)

Six of the eight backends already lower it — html, react, webcomponent,
SwiftUI, Qt and XAML — all measured on a minimal two-node probe rather than
assumed. Compose and Flutter (#14851) were the two that did not, and Engram
authors it on **all seven** of its screens (760px–1100px), so its study screen
is a reading column that ran the full width of the window.

```
Modifier
    .widthIn(max = 980.dp)
    .fillMaxWidth()
```

**The order is load-bearing and not symmetric**, which cost a wrong version
first. `.fillMaxWidth()` pins `minWidth = maxWidth` to the incoming max, after
which `.widthIn(max = ..)` cannot lower the max below that min — Compose
coerces and the floor wins. Engram's screens rendered at the full 1208px with
the modifier plainly present in the emitted Kotlin. This way round, `widthIn`
clamps the incoming max and `fillMaxWidth` fills to the clamped value.

The fill is emitted alongside the cap rather than left to the container
default, because that default is *prepended* and would land on the wrong side.
And it must be emitted: without it the container wraps its content, which is
also wrong — a ceiling is not a width.

No alignment is added. The six backends that already lower `max-width` emit no
centring with it, so the capped box sits at its parent's start.

Measured on the rendered app, by painted extent:

| | before | after |
| --- | --- | --- |
| Engram screen content | `1255` | **`1003`** |
| Engram `app-header` (no cap authored) | `1255` | `1255` |

Trestle's `list-wrap` (`max-width: 760`) now caps too — its task column ends at
~1026 instead of ~1250.

### Fixed — `min-height` reached nothing (#14837)

Parsed and discarded. Three products author `min-height: 100vh` on their app
shell to mean *fill the window*, and on Compose it did nothing at all.

Engram's composition root measured `1280 x 776` in a `1280 x 900` window.
Reading the rendered PNG's alpha channel: the last painted row at x=640 is
**775**, and rows 776..899 are `(0, 0, 0, 0)` — genuinely **unpainted**, not
painted in some other colour, so whatever composites behind the surface shows
through. That is why the same defect read as white in one capture and black in
another.

| authored | Compose |
| --- | --- |
| `min-height: 100vh` / `100%` | `.fillMaxHeight()` |
| `min-height: N` / `Npx` | `.heightIn(min = N.dp)` |
| `min-height: 0` | nothing — a zero floor constrains nothing |

A floor, so `heightIn(min = ..)` rather than `height(..)`: it composes with an
authored `height` instead of replacing it.

Compose has no viewport unit, so `fillMaxHeight()` fills the **parent**. On an
app shell the parent is the window and the two agree; nested, they would not.
Named rather than hidden — the alternative is what shipped, which was nothing.

Both new modifiers join the unconditional import block. Emitting a modifier
without its import is Kotlin that does not compile — the shape of the XAML
`Not()` helper in #14793, and why #14798 pins its import too.

TaskApp's `styleDegradations` go 168 → 164, and exactly four modifiers are
emitted (one `fillMaxHeight`, three `heightIn`): the reporter and the lowering
now ask the same question (#14810).

### Fixed — `Box` overlaid its children instead of laying them out (UI60, #14828)

`Box` and `Stack` both lowered to Compose's `Box`, which layers its children at
a shared origin. That is `Stack`'s meaning; UI29 gives `Box` the generic opaque
container — `<div>` on html, `Group { }` on SwiftUI, `Item { }` on Qt, all of
which lay children out in flow. Compose was the only backend that disagreed.

Engram's deck-stat chips drew the count on top of its label: `[0]` and
`[Total]` both at `Rect.fromLTRB(49.0, 247.0, ..)`, eleven chips, every
semantics gate green throughout — a node drawn over another node is still
present, still named, and still "displayed".

`Box` now lowers to `Column`; `Stack` keeps Compose's `Box`.

Two things the spec did not anticipate, both fixed here:

- **The composable is chosen in two places.** `emit_node`'s `match node.tag`
  and `root_container_context`. Fixing one would have left a root `Box`
  overlaying while every nested one flowed.
- **A root `Stack` was not overlaying at all.** `root_container_context` never
  named `Stack`, so it fell through to `_ => Column`. The defect UI60
  describes and its mirror image were both live on the same primitive pair.

Behaviour change worth naming: `gap` on a `Box` was silently dropped before —
a Box had no arrangement — and is now real `verticalArrangement` spacing.

Measured, not inferred: Engram's chips go from a shared origin to `[0]` bottom
`271.0` / `[Total]` top `271.0`, and **all three Trestle renders are
byte-identical** across 27 `Box`→`Column` swaps, because its multi-child Boxes
render one child each.

### Fixed — `text-align` on a Row or Column emitted Kotlin that did not compile (#14839)

`contentAlignment` was applied from `text-align` with no check on which
composable was being emitted, so a `Row` part with `text-align: center`
produced `Row(contentAlignment = Alignment.Center)`. `contentAlignment` is a
**`Box`-only** parameter; `gradle compileKotlin` rejects it with
*"No parameter with name 'contentAlignment' found."*

Latent rather than absent: every shipped package authors `text-align` only on
a `Box`, where the argument is valid, so CI was green. Emitted output for all
five products is unchanged by this — VisiCalc's two `contentAlignment` uses are
both Boxes and stay exactly as they were.

`arrangement_argument` is replaced by `container_alignment_arguments`, which
owns both `text-align` and `gap` for a reason: on a `Row` they both want
`horizontalArrangement`, and emitting it twice is a duplicate named argument
that also does not compile. Compose's two-argument
`Arrangement.spacedBy(space, alignment)` is the resolution — reachable only if
one place decides both.

| composable | `text-align` | `gap` |
| --- | --- | --- |
| `Box` | `contentAlignment` (unchanged) | dropped — a Box stacks, so a gap is meaningless there |
| `Column` | `horizontalAlignment` | `verticalArrangement` — different axes |
| `Row` | `horizontalArrangement = Arrangement.Start/Center/End` | folded into `spacedBy(gap, alignment)` |

Each row of that table was checked against `gradle compileKotlin` on a
generated project before being written down.

This is the first step of UI60 (#14828): swapping `Box` to a flow container
makes every existing `contentAlignment` invalid, so the mapping has to exist
before the swap can happen.

### Changed — the drop reason for `justify-content`/`align-items`/`align` (#14811)

These three shared a match arm with `display`/`flex-direction`/`flex-wrap`,
reported as *"Compose expresses layout through the composable chosen … and its
arrangement arguments, not through a modifier on a built view"*. That sentence
was being used to justify discarding them while describing the mechanism that
would work: they are the `horizontalArrangement`/`verticalAlignment` arguments
of Row and Column — the same argument slot `gap` already reaches as
`Arrangement.spacedBy` (#14804).

Split into two arms so the report says which kind of gap each property is.
A pinned drop that misstates its own cause reads as settled when it is not
(#14834).

### Added — Compose reports the style properties it drops (#14810, #12022)

Compose had no `dropped_style_properties`, so an empty `styleDegradations`
meant "nobody looked" rather than "nothing was lost". Measuring it named **43
distinct properties** in TaskApp alone, while the strict `native-complete`
profile reported zero degradations:

| count | property |
| --- | --- |
| 318 | `border-radius` — every rounded surface renders square |
| 308 | directional `padding-*` (in flight, #14730) |
| 172 | `gap` (fixed in #14805) |
| 77 | `align` |
| 48 | `font-weight` — every bold label renders at regular weight |
| 48 / 46 | `box-shadow` / `elevation` |

Drops are collected **by the builder**, in the `_` arm of its property match,
rather than by diffing against a hand-kept list of "properties Compose
supports". A parallel list is wrong the first time someone adds an arm and
forgets to update it, which is exactly the drift #12022 exists to catch.

Each drop carries an actionable reason rather than generic text — a missing
modifier, a value that must be an argument, and a concept Compose does not have
are genuinely different problems and want different fixes.

Style drops do not gate `nativeComplete`; this makes them visible, not fatal.
### Fixed — `gap` was dropped entirely (#14804)

`gap` reached the lattice IR and died in the property loop's `_ => {}` arm.
178 declarations across 27 stylesheets, discarded in silence — the strict
`native-complete` profile still reported zero degradations, because the
analyzer does not know the property exists.

In the rendered app it showed as adjacent text running together (`Up next1`)
and controls butting against each other. Not a string bug: two `Text` nodes in
a row whose spacing had been thrown away.

It now lowers to `Arrangement.spacedBy(n.dp)`, on the axis the container names:
`verticalArrangement` for a `Column`, `horizontalArrangement` for a `Row`. A
`Box` stacks its children and has no arrangement, so a gap there is meaningless
rather than unsupported and is dropped deliberately.

TaskApp emits 20 arrangements where it previously emitted none. Verified by
rendering: `Up next 1` now has its space, and the header and sidebar have
spacing.

One subtlety the tests caught: `gap` is an **argument**, not a modifier, so it
never sets `has_chain` — a part whose only property is a gap was still taking
the single-line `Column(modifier = …)` form, where an argument has nowhere to
go. It looked like it worked, because every part in TaskApp that authors a gap
also carries another property. The multi-line form is now chosen when a gap is
present.

`row_children_use_weight_and_intrinsic_measurement_in_split_sections` asserted
the exact string `Row(modifier = Modifier) {`. Its `progress` part authors
`gap: 10px`, so it now takes the multi-line form. The claim that test makes is
about **width**, not formatting, so it now asserts the width directly — bare
`Modifier`, no fill — and was re-checked to confirm it still fails if a fill
appears.

Does not fix every case: the storage line still renders joined, so its gap sits
on a container this does not reach. Tracked in #14804 with Flutter and SwiftUI,
which drop `gap` the same way.
### Added — directional padding lowering (#14709)

`padding-top/-bottom/-left/-right` were dropped entirely, so a part asking for
asymmetric insets got none at all — not even the shorthand, if it never
declared one. 391 uses across the repository.

Compose's `padding(start=, top=, end=, bottom=)` overload overrides per edge,
which is what CSS means, so the four edges are resolved at emit time and
emitted as one call. Resolution is by source order, so no precedence table is
needed: the shorthand seeds all four, a directional property overwrites its
own, and `padding-top: 20; padding: 8` correctly yields 8 everywhere.

When all four agree — the common case — emission collapses back to
`.padding(n.dp)`, byte-identical to previous output. Verified across all 23
toolkit components: 23 unchanged, 0 mismatches.

`left`/`right` lower to `start`/`end` rather than fixed sides, so a
right-to-left layout mirrors them the way every other Compose padding does. An
edge with no authored value is omitted rather than passed as `0.dp`: the
overload already defaults it to zero, and naming it would claim the stylesheet
asked for something it did not.
### Fixed — `font-weight` was discarded (#14810)

48 occurrences in TaskApp, 3 in the toolkit, every one thrown away — so every
bold label rendered at regular weight, which is much of why the app read as
flat.

This is **not** a missing modifier. Compose's `Text` takes `fontWeight` as an
**argument**, so it threads through the text style (beside `color`,
`fontFamily` and `fontSize`) rather than the box modifier chain — the same
shape as the `gap` problem in #14804, where an argument had no home in a
chain-shaped lowering.

CSS numbers map to Compose's named constants where they exist
(`500` → `FontWeight.Medium`, `600` → `FontWeight.SemiBold`), because the
generated Kotlin is meant to be read; other legal weights use
`FontWeight(n)`.

`lighter` and `bolder` stay **unmapped** on purpose: both are relative to the
inherited weight, and this lowering has no inherited value to resolve them
against. Guessing `Light`/`Bold` would be wrong for any parent that is not
already normal, so they fall through to the drop report and say so.

TaskApp emits 18 weights where it emitted none. Verified end to end: emitted,
zero degradations, control contract, compiled, launched, rendered — bold labels
now render bold — and the acceptance lifecycle stays green.
### Fixed — `border-radius` was discarded entirely (#14810)

318 occurrences in TaskApp alone, 31 in the toolkit, every one thrown away — so
every rounded surface in Trestle rendered square while the strict
`native-complete` profile reported zero degradations.

The shape now reaches **every** modifier that takes one:

```kotlin
.shadow(4.dp, RoundedCornerShape(8.dp))
.clip(RoundedCornerShape(8.dp))
.background(Color(0xFFEAA63F), RoundedCornerShape(8.dp))
```

Passing it to only some is visibly wrong in a different way each time: a
rounded background inside a square border, a rounded card casting a square
shadow, or rounded chrome with content spilling past its corners. `.clip` sits
after `.shadow` — clipping first would clip the shadow layer away — and bounds
the children, which `background`/`border` do not.

`RoundedCornerShape` and `clip` join the unconditional import block. An unused
Kotlin import is a warning; a missing one does not compile, and that asymmetry
is what went wrong with the XAML `Not()` helper in #14793.

Verified end to end: emitted, zero degradations, control contract, compiled,
launched, rendered. TaskApp emits 305 shapes, and the acceptance lifecycle
stays green. A no-radius part stays byte-identical, with a test for it, so this
cannot quietly round everything.

**One behaviour change worth flagging.** The `On track` chip measures
`0 x 168` — zero width, a defect that predates this. It used to render its
letters stacked one per line down the screen; clipping now hides them instead.
The bug is unchanged, but a loud symptom became a silent one. Filed as #14815.
### Fixed — `opacity` was dropped (#14708)

**A property consumed outside the style match is not a drop.** `elevation` is
read by `part_elevation_tier` straight from the base props, so it never reaches
the match — and the first version of this reporter counted all 16 of TaskApp's
as dropped while the emitter was emitting 16 `.shadow(..)` calls. Exactly the
same number, which is what gave it away.

The value→tier mapping is now one shared function the reporter and the lowering
both call, so there is no second list to drift. Both directions are tested:
`elevation: raised` is not reported, `elevation: floaty` still is, and its
genuinely-unlowered neighbour `box-shadow` stays reported either way.

A drop report that cries wolf is worse than no report, because the real entries
stop being read.

`flex-grow` had the identical problem, found by asking whether `elevation` was
structurally unique — it is not. `compose_row_weight` consumes it, so all 6 of
TaskApp's usable values were reported against 6 `.weight(..)` calls actually
emitted. Same fix: one shared predicate, `flex_grow_weight`.

One known limit, stated rather than hidden: `compose_row_weight` applies only
to Row children, and this reporter is per-part with no node context, so a
usable `flex-grow` on a **non-Row** child is discarded without being reported.
Under-reporting that narrow case is the lesser error against reporting every
usable value as a drop.

TaskApp's report: 532 → **510**, with 22 false entries removed and every
genuine one (`flex-shrink` 9, `box-shadow` 17, …) still present.

`opacity` is what UI57's `state disabled` treatment is built on, so dropping it
meant a disabled control dimmed on five backends and not on this one — it
looked disabled on the web and XAML and SwiftUI, and fully normal on Compose.

It now lowers to `Modifier.alpha(..)`, placed **first** among the drawing
modifiers so it covers the background, the border and the content alike;
applying it later would fade only what follows it in the chain.

The Float suffix goes on each **value**, not on the assembled expression. A
state-layered opacity becomes `(if (..) 0.4f else 1f)`, and Kotlin cannot
suffix a parenthesised expression — `(...)f` does not parse. The first version
did exactly that, every emitter string assertion passed, and it was the Kotlin
compiler that caught it. There is now a test for the state-layered form, which
is the one that matters: the value worth reading is almost always layered.

The toolkit emits 8 alpha expressions where it emitted none, and its generated
Compose project compiles.
### Fixed — any style property cancelled a container's width default (#14795)

`emit_container` and `emit_container_frame` both wrote the width default and
the style chain as mutually exclusive:

```rust
if has_style_chain { "Modifier{chain}" } else { "Modifier.fillMaxWidth()" }
```

`fillMaxWidth()` is what a container gets when nothing says otherwise, not an
alternative to styling one. As an either/or, authoring any unrelated property —
a background, a border, a padding — silently cancelled it and let the container
shrink to its content. The author asked for a colour and lost their layout.

Verified on unmodified `main` with two identical `Row`s, the second adding only
a background: `plain_has_fill=true styled_has_fill=false`.

The default is now prepended when the chain says nothing about width, and goes
**first** so an explicit `width`/`fillMaxWidth` later still wins — Compose
resolves size modifiers in order. `chain_sets_own_width` covers `width`,
`requiredWidth`, `widthIn`, `fillMaxWidth`, `fillMaxSize` and `weight`;
`in_row_scope` children stay intrinsic as before.

Both paths are fixed together. A container only takes the split path once its
section grows past the size threshold, so fixing one alone would hold until a
layout grew and then quietly stop — exactly how the `HostScroll` modifier
behaved in #14736.

Checked visually rather than inferred, using the screenshot harness from
#14798: TaskApp's empty-state panel now spans the full width instead of
stopping short, and the one-task and completed states are pixel-identical to
before. That is the whole visible effect on TaskApp; it was not obvious from
the emitted diff, which changes 22 containers.

### Fixed — a `HostScroll` region now fills its viewport (#14798)

`HostScroll` lowered to `.verticalScroll(rememberScrollState())` and nothing
else, leaving the container wrapping its **content**. That is wrong twice over:
a scroller sized to its own content has nothing to scroll within, and whatever
the content does not cover stays unpainted.

In TaskApp the whole UI rendered into roughly the top 250 px of a 900 px
window, and the remaining two thirds were **white** — not the theme background,
which is why it read as a broken app rather than an empty one. Found by
rendering it (#14799); no semantics assertion could see it.

The lowering now emits `.fillMaxSize()` before `.verticalScroll(...)`: fill the
space, then scroll within it. Verified by rendering the real `native-complete`
project — the themed background covers the window, and the acceptance lifecycle
stays green.

`fillMaxSize` joins the unconditional layout import block beside `fillMaxWidth`.
Emitting a modifier without its import is Kotlin that does not compile, the
same shape as the XAML `Not()` helper in #14793, so the test asserts the import
as well as the modifier order.

### Added — `HostInput.disabled` (#14786)

`HostInput` had `read-only` but no way to say *unavailable*. Compose spells
availability positively, so `disabled` lowers to a negated `enabled`
argument, distinct from `readOnly`.

Spec: `code/specs/UI58-hostinput-disabled.md` (#14786). Landed on all eight
backends in one change — a partly-landed prop would make a disabled input
*less* restricted on whichever backend lagged.

### Fixed — root-section splitting recurses, and is driven by emitted size (#14736)

`should_split_root_sections` split only the **root's direct children** into
section functions and did not recurse. TaskApp's root `Row` has two children,
so one section held the entire main column — 112,912 characters of generated
Kotlin against the other section's 6,468. It compiled only because it sat just
under the JVM's hard 64KB-per-method bytecode limit, and adding a single
`HostScroll` node crossed it:

```
Method too large: TaskAppKt.TaskAppSection1
```

Sections now split recursively until each fits, driven by the size of the
**emitted body** rather than the shape of the IR. A child-count or depth
heuristic is wrong in both directions: a deep-but-small tree would split
needlessly, and a shallow-but-wide one — exactly what TaskApp is — would not
split at all.

The threshold is source length, because bytecode size is knowable only to the
Kotlin compiler. It is set at 40,000 characters, well below the ~113,000 that
actually failed, so the proxy has roughly 2.5x of margin.

Two details this needed that were not obvious:

- **Descent through single-child wrappers.** Requiring more than one child (as
  the root check does) made a scroll viewport a hard stop, with 80,000
  characters still inside it. A wrapper costs one extra function to pass
  through and lets the split reach the wide node underneath.
- **`HostScroll` keeps its modifier when split.** The split path builds its
  frame through `emit_container_frame`, which never applied the
  `.verticalScroll` prefix that `emit_container` adds — so a viewport large
  enough to be split silently stopped scrolling while the generated file still
  carried the import. Both paths now share one helper. Caught by a test
  asserting the viewport keeps its modifier, not by reading the code.

### Added — `HostScroll` lowering (#14732)

Compose was the only backend of eight with no `HostScroll` arm, so a generated
Compose app could not scroll at all: content past the viewport was simply
unreachable, with no error and no degradation entry.

Compose has no scrolling *container* composable — scrolling is a **modifier**
on an ordinary one — so this lowers to a `Column` whose chain starts with
`.verticalScroll(rememberScrollState())`.

Routed through `emit_container` rather than a bespoke emitter, so the
viewport's own part styles keep working. The scroll modifier is prefixed onto
whatever chain the part already has, and deliberately comes first: the viewport
must be able to scroll its content before padding or size constraints are
applied to it.

The two imports are conditional on the layout actually containing a
`HostScroll`, matching how `Path` and drag imports are handled, so components
without one are unchanged.

### Fixed — state-dependent dimensions keep their Compose units

Numeric state expressions are now parenthesized before applying `.dp` or
`.sp`. This keeps every branch typed as `Dp` or `TextUnit` instead of letting
Kotlin infer `Comparable<*>` / `Any`, which prevented toolkit Button packages
with UI49 size states from compiling (#14383).

### Added — UI49 slot-owned style states

`one-of` slot values now activate their matching `.msl` state blocks in
generated Compose code. The owning composable parameter drives the Kotlin
conditional style expression for generic layout nodes and specialized host
controls alike. Multiple enum axes follow `.mil` slot declaration order, while
existing `state-when-*` structural and interaction layers remain more specific.

Tracked by [#14320](https://github.com/adhithyan15/coding-adventures/issues/14320).

### Fixed — components over ~229 slots could not be loaded by the JVM

The emitter gave every slot its own Kotlin parameter. Engram's `EngramApp` has
254 slots, so the emitted composable took 255 parameters, and the JVM caps a
method signature at **255 argument slots**. The class compiled, packaged into a
`.app`, launched, and died:

```
java.lang.ClassFormatError: Too many arguments in method signature
```

Components past the limit now take a single grouped props object instead.
Positional parameters are kept wherever they fit, because Compose skips
recomposition **per parameter** — collapsing every component into one object
would trade a load-time crash for a performance regression across the board.

The threshold is measured, not guessed. A probe that compiled **and loaded**
composables of increasing arity put the boundary at exactly 229 String slots
(254 JVM slots), with 230 failing. `compileKotlin` succeeds on both, so a
compile-only probe finds no boundary at all. The cost model that follows —
parameters, plus `dispatch`, plus the plugin's `$composer` and one `$changed`
bitmask per ten parameters, with `Double`/`Long` counting twice unless nullable
— reproduces that boundary exactly.

Two things this took more than one attempt to get right, both now pinned by
tests:

- **The split-section helpers have the same problem**, and there is one per
  top-level child, so a component that overflows once overflows eight times.
  Fixing only the root left the class unloadable.
- **A constructor is a method signature too.** One flat data class of 254
  properties overflows its own constructor and its generated `copy()`, which
  moved the failure from `EngramAppKt` to `EngramAppProps` rather than removing
  it. The props object is therefore chunked into groups of 64.

Every emitted props class is `@Immutable`. That is load-bearing: Compose can
only skip recomposition for a type it knows to be stable, and an unannotated
class is treated as unstable, so the whole component would recompose on any
change.

Verified end to end: Engram's Compose desktop app emits, compiles, loads all
three classes, packages, and **runs** with the engine loaded and no
`ClassFormatError`.

### Fixed

- `HostInput.a11y-label` now lowers to native Compose content-description
  semantics for literal, slot-backed, and expression-backed names (#13717).

- `HostButton.a11y-label` now lowers to native Compose semantics, including
  expression-bound labels inside repeated rows (#13691).

- Compose Rows now measure ordinary container children intrinsically instead
  of assigning every child `fillMaxWidth()`. A direct child with `flex-grow`
  or `width: 100%` receives scoped `Modifier.weight(...)`, including in
  split `RowScope` section functions. This keeps TaskApp completion progress
  visible after the flexible title without imposing Compose-specific pixel
  widths on Flutter, SwiftUI, or web (#13565).

### Added

- `elevation` native shadow lowering (#12028 item 1, UI41). A part
  declaring `elevation: raised;` or `elevation: overlay;` (mosstyle's new
  typed shadow-intent property, #13358) now gets a real
  `androidx.compose.ui.draw.shadow(N.dp)` modifier instead of the shadow
  silently vanishing. New `ElevationTier` enum (`Raised` → `4.dp`,
  `Overlay` → `16.dp`, matching `mosaic-emit-xaml`'s `ElevationTier`
  numbers so a part reads the same "how far off the surface" intent on
  both backends) + `part_elevation_tier` helper, read directly inside
  `compose_box_style` — the ONE function every styled container/control in
  this crate already funnels through (`emit_container`'s `Box`/`Row`/
  `Column`, `emit_host_button`, `emit_host_draggable`'s delegation to
  `emit_container("Column", ...)`, etc.). Unlike the XAML PR for the same
  feature, which found `Row`/`Column`/`HostDraggable` needed their own
  separate shadow wiring because XAML has per-primitive emitter functions,
  Compose's centralized `compose_box_style` meant implementing `elevation`
  once covered every call site for free — confirmed, not assumed, by
  compiling the real `TaskApp`/`ProjectNav`/`Notes`/`Calendar` packages
  (all 4 real components declaring `elevation` today) and finding every
  one of `TaskApp.light.msl`'s 13 `elevation`-declaring parts produces at
  least one `.shadow(...)` in the generated Kotlin — including a `Box`
  (`brand-mark`), several `Row`/`Column` containers, and a `HostButton`
  (`notes-row-on`), and even confirming `ProjectNav`'s own `elevation`
  part flows through correctly when TaskApp composes `ProjectNav` as a
  child package.

  Modifier order is load-bearing: `.shadow` must be emitted right after
  `.width`/`.height` and BEFORE `.background`/`.border` — otherwise the
  background paints over the shadow layer instead of the shadow appearing
  behind it. Confirmed empirically (not assumed) against a real
  `org.jetbrains.compose` Gradle Desktop probe: `.shadow(...).background(...)`
  compiles and matches Compose's own documented usage pattern;
  `.background(...).shadow(...)` also compiles (both orders are valid
  Kotlin) but visually the shadow layer would be occluded, so the emitter
  always emits `.shadow` first.

  `box-shadow` itself is still silently skipped by `compose_box_style` (as
  it always has been — this crate never read it before this PR either);
  `elevation` is the only native-shadow signal Compose reads, mirroring
  the XAML PR's "no `elevation` declared → no native shadow, regardless of
  `box-shadow`" posture. The `androidx.compose.ui.draw.shadow` import is
  gated behind a whole-`StyleDef` walk (`uses_elevation`) rather than
  `layout_contains_tag`, since `elevation` is a style property, not a
  layout tag.

  Verified against the real toolchain, and a real version correction along
  the way: an initial scratch probe (matching the `org.jetbrains.compose`
  1.6.11 pin cited in this crate's own `HostProgressRing`/`Path` CHANGELOG
  entries) compiled `Modifier.shadow(...)` cleanly, but
  `mosaic-compile pkg --backend compose --emit-project` against the real
  `task-app` package revealed the *actual* generated project now pins
  `org.jetbrains.compose` **1.11.1** / Kotlin **2.3.21** (version drift
  since those earlier PRs landed) — a discrepancy that would have gone
  unnoticed without re-deriving the pin from the real generator instead of
  trusting an older CHANGELOG citation. Re-verified against the real pin:
  `gradle compileKotlin` on the real generated `TaskApp.kt` inside the
  real generated project scaffold (`build.gradle.kts`, `Main.kt`,
  `MosaicRuntimeHost.kt`, all emitted by `--emit-project`) —
  `BUILD SUCCESSFUL`, no new warnings beyond pre-existing, unrelated
  "redundant conversion method" ones. `gradle run` launched the real
  window with no crash or exception from Compose (the only output was the
  expected "native library not found" warning from omitting
  `--runtime-library`, unrelated to this change).

- `HostProgressRing` native lowering (#13176, UI40). `HostProgressRing
  [part] (value: ..., a11y-label: ...)` now lowers to a determinate
  `androidx.compose.material.CircularProgressIndicator(progress =
  (value).toFloat() / 100f, ...)` instead of reporting
  `primitive.progress-ring-unimplemented`. `value` supports the full
  `Number`/`SlotRef`/`Expr` three-way binding via the new
  `required_progress_ring_value` helper (unlike `Path`'s coordinate
  props, live binding is required from day one — the whole point is
  rendering a live percent value). Sizing reuses
  `compose_style_for_node`'s existing `.width().height()` modifier
  chain; `a11y-label` appends a `.semantics { contentDescription =
  ... }` suffix, matching `HostSlider`'s own accessibility pattern.
  Widened the `CircularProgressIndicator` import gate — previously
  scoped only to `uses_icon` (the indeterminate spinner case) — to
  `uses_icon || uses_progress_ring`, since a component using
  `HostProgressRing` without an `Icon` would otherwise reference an
  unresolved Kotlin symbol. Verified against a real
  `org.jetbrains.compose` 1.6.11 Gradle project: `gradle
  compileKotlin` confirmed the plain-`Float` `progress:` overload
  (this pinned Material1 version predates the newer `progress: () ->
  Float` lambda form) with no deprecation warning, then `gradle run`
  confirmed the widget mounts without crashing.

- `Path` drawing primitive lowering (#12028 item 3, UI39). `Path [name]
  (kind: circle|line|curve, ...)` now lowers to real Compose vector
  geometry instead of reporting `primitive.path-unimplemented` on
  every build. `circle` reuses `Modifier.background(color,
  CircleShape)` + `Modifier.border(width, color, CircleShape)` —
  `CircleShape` plus the already-unconditionally-imported
  `.background`/`.border` modifiers match `background`/`border-color`+
  `border-width` 1:1, the same reuse Qt's `Rectangle` and Flutter's
  `BoxDecoration(shape: BoxShape.circle)` lowerings made for the same
  shape. `line`/`curve` lower to a `Canvas` drawing a Compose
  `graphics.Path` built via `moveTo`/`lineTo`/`quadraticBezierTo`,
  using absolute canvas coordinates (mirrors Qt's `ShapePath` and
  Flutter's `CustomPaint`). `arc` is a stretch goal not implemented in
  this PR; it hard-errors with a named "not yet supported" message,
  matching the XAML/Qt/Flutter lowerings' posture for the same gap.

  Positioning `circle`'s authored center is notably simpler than the
  Flutter lowering: Compose's `Modifier.offset(x, y)` shifts a
  composable's painted position relative to wherever normal layout
  would place it and is legal on ANY composable regardless of parent
  type — unlike Flutter's `Positioned`, which only type-checks (and
  only avoids a runtime panic) as a direct `Stack` child. So `circle`
  always emits `Modifier.offset(...)` unconditionally, no
  `direct_stack_child`-equivalent threading needed.

  Import gating is split in two: `CircleShape` fires whenever any
  `Path` is present, while `Canvas`/`graphics.Path`/`drawscope.Stroke`
  fire only when a `line`/`curve`/`arc` kind is actually present (new
  `tree_needs_path_canvas`, mirroring Qt's `tree_needs_shapes_import`)
  — so a circle-only tree (the common case, the crescent moon) doesn't
  pay for the Canvas imports it never uses.

  Coordinate props (`cx`/`cy`/`r`/`x1`/`y1`/`x2`/`y2`) accept a literal
  `Number` only; a `SlotRef`/`Expr`-bound coordinate is a clear compile
  error, not a silent 0 — full data-driven binding is future work, the
  same not-yet-landed gap the XAML, Qt, and Flutter lowerings all note.

  Verified against the real toolchain: a `mosaic-compile pkg --backend
  compose --profile native-complete` build of the crescent-moon shape
  (two overlapping circles, a line, and a curve) produces
  `nativeComplete: true` with zero degradations. The generated Kotlin
  was dropped into a real JetBrains Compose Multiplatform Desktop
  Gradle project (`org.jetbrains.compose` 1.6.11) and compiled cleanly
  via `gradle compileKotlin`, then launched via `gradle run` and stayed
  running with no crash or exception output.

- Native radio-group mutual exclusion (#13007). `group:` was never read
  anywhere in `emit_host_radio`. A container physically holding 2+
  `HostRadio` siblings sharing a literal `group:` value now gets
  `Modifier.selectableGroup()` on its own modifier chain (new
  `container_needs_radio_group_semantics` + `host_radio_literal_group_key`,
  wired into both `emit_container` and the root-splitting
  `emit_container_frame` path) — purely additive a11y semantics; each
  `RadioButton`'s own `selected`/`onClick` stays entirely local to its
  own `checked`/`onSelect` props, unchanged. The
  `androidx.compose.foundation.selection.selectableGroup` import is
  added conditionally via a new whole-tree `layout_has_radio_group`
  walk. New `pub fn radio_groups_with_native_semantics` lets
  `mosaic-package-artifact-builder`'s degradation analyzer stop
  reporting `property.radio-group-ignored` wherever this lowering
  actually applies. Verified against a real regenerated
  `mosaic-pkg-deck-options` project (the real multi-radio usage this
  targets): `gradle compileKotlin` — `BUILD SUCCESSFUL`.

- Native indeterminate checkbox state (#13006). `emit_host_checkbox` had
  no code path for `indeterminate:` at all. When authored as anything
  other than a literal `Keyword("false")`, the emitter now swaps the
  plain `Checkbox` for Compose's own `TriStateCheckbox(state:
  ToggleableState, onClick: () -> Unit)`, with `state` computed from
  `indeterminate`/`checked` (`_mosaicTruthy`-wrapped for `slot:`/`Expr`
  values, matching `bool_prop_expr`'s existing convention) and the
  `TriStateCheckbox.material` and `androidx.compose.ui.state.ToggleableState`
  imports added conditionally (new `layout_has_checkbox_indeterminate`
  walk). `TriStateCheckbox.onClick` takes no argument — unlike
  `Checkbox.onCheckedChange`'s `checked` lambda parameter — so the
  dispatched "new checked" value is computed inline from the same
  `ToggleableState` expression used for `state =`: clicking always
  resolves *out of* Indeterminate, toggling towards `On` unless already
  `On`. New `pub fn host_checkbox_has_native_semantics` lets
  `mosaic-package-artifact-builder`'s degradation analyzer stop
  reporting `property.checkbox-indeterminate-ignored` for Compose
  wherever this lowering actually applies. Verified against a real
  regenerated `mosaic-pkg-toolkit` project: `gradle compileKotlin` —
  `BUILD SUCCESSFUL` — on the whole Compose Desktop package, including
  the real `Checkbox.kt` this change touches.

### Security

- Validate `HostLink.href`'s URI scheme, literal and slot-bound (#13052).
  Follow-up to #12038 (the identical XAML gap). A literal href is now
  rejected at compile time when it carries an explicit, disallowed scheme
  (new `host_link_href_expr` + `has_disallowed_uri_scheme`, reusing the
  existing `UnsupportedHostLink` error variant). A slot-bound href — unknown
  until runtime — is validated inside the shared `_mosaicHostLink`
  composable via a new `_mosaicIsSafeUri` helper: when the scheme is
  disallowed, the link degrades to the same inert `Clickable` shape the
  `external == false` branch already uses (neither `onActivate` nor
  `uriHandler.openUri` fires), matching the "no navigation target" outcome
  XAML's `SafeNavigateUri` fix settled on for a null `Uri`. A relative
  reference with no scheme at all (`"#"`, a route path) is unaffected in
  both paths, since a relative href never reaches `uriHandler.openUri` as
  an external target regardless (only the `external == true` branch is
  gated).
- Two rounds of security review caught two real gaps in the scheme
  detection, both fixed before merge: a leading space or embedded
  tab/CR/LF made the first-character-alphabetic check fail and
  misclassified the string as "no scheme, therefore safe" -- but a real
  consumer strips that whitespace before parsing the scheme, so it's
  really the dangerous scheme it looks like. The first fix trimmed
  leading/trailing whitespace via Kotlin's `trim()`, but a second review
  round found `trim()`'s default `isWhitespace`-based predicate doesn't
  cover the full C0-control range a real consumer strips (control bytes
  like 0x01/0x1B bypassed it) -- `_mosaicIsSafeUri` now uses
  `raw.trim { it.code <= 0x20 }`, matching the Rust-side check exactly.

### Fixed

- MIL slots with authored defaults now emit non-null Kotlin parameters with
  matching default arguments, so reusable package components can consume their
  own defaulted text, number, and boolean values without nullable type errors.
- Preserve literal `HostInput` values and read-only state, and render its
  placeholder through `BasicTextField`'s native decoration slot.

### Added

- `HostSlider` now maps literal and slot-backed `a11y-label` values to the
  native slider semantics node without replacing its adjustable range role.
- Slot-bound or expression-backed slider steps now derive Compose's discrete
  interior-stop count at runtime, including continuous behavior when step is
  non-positive.
- `HostSlider` now lowers to Compose Material's native adjustable `Slider`,
  including controlled numeric values, range and discrete-step mapping,
  disabled state, continuous `onChange`, and release-time `onCommit` events.
  Numeric values convert at the Float-based Compose boundary and return to
  Mosaic's portable number payload as Double. CI compiles a native-complete
  slider package through the generated Compose project shell.
- `Text` now lowers literal or slot-backed accessible names, heading roles,
  and intentional hiding through Compose semantics. Replacement labels clear
  the built-in text semantics so assistive technology does not announce both
  the visible content and its authored accessible name.

- Canonical dynamic `HostTable`/UI31 Grid layouts now expose Compose's native
  collection semantics: total row/column counts on the table, heading metadata
  on header cells, and stable row/column coordinates on every body cell.
  Unsupported table shapes keep their visual fallback and remain explicit
  native-complete degradations.
- `HostDraggable` and `HostDropTarget` now lower to Compose Desktop's native
  drag source/target modifiers. Generated components add an instance-scoped
  target registry, kind filtering, disabled-state enforcement, pointer
  before/into/after hit testing, focus and Space/Enter/arrow/Escape operation,
  RTL-aware horizontal navigation, live-region state, and shared event payload
  construction for pointer and keyboard drops.
- `Icon` now lowers through a dependency-free native font-glyph vocabulary,
  including runtime glyph and accessibility-label slots, MSL color/size/test
  tags, and a visible fallback. The semantic `spinner` glyph becomes Compose's
  indeterminate `CircularProgressIndicator` with a default "Loading"
  description, allowing all 23 toolkit components to emit on this backend.
- `HostDialog` now lowers modal content to Compose's native `Dialog` and the
  contract's non-modal form to `Popup`. Generated overlays honor controlled
  visibility and interactive-dismiss policy, dispatch open/close events, render
  a semantic heading, preserve nested Mosaic content and styles, and provide
  useful Material surface chrome without application-owned dialog glue.
- `HostLink` now lowers to Compose's native annotated-text link API. External
  links open through the platform `UriHandler`; internal links retain link
  semantics while dispatching Mosaic events, including item/index payloads
  inside `For`. Generated links receive theme-aware visible styling and need no
  application-owned URL adapter.
- `Stack` now lowers to Compose's `Box` — the layering container it already
  uses for the `Box` primitive itself, since Compose's `Box` natively stacks
  its children. Found while wiring `task-app`'s icon assets (progress ring,
  crescent moon, bridge-arc brand mark — see `task-app-icon-assets-v1.md`),
  the first place a Mosaic component used `Stack` and hit this backend's
  build. Not yet lowered: a child's static `position: absolute` + `top`/
  `left` into `Modifier.offset(...)` — v1's existing "anything else
  silently skipped" posture for static props means a Stack's children all
  render at the Box's origin today rather than the pixel positions the
  web/Flutter backends place them at.
- `HostTooltip` now lowers to Compose Foundation's cross-platform
  `BasicTooltipBox`, including native overlay placement and dismissal, Material
  surface chrome, literal/slot/expression text, and assistive-technology
  semantics. This restores package-expanded TaskApp generation after its richer
  Gantt introduced per-row tooltips.
- `HostSurface ( content: slot: ... )` now accepts an
  `@Composable () -> Unit` node slot and invokes it at the shared native
  composition boundary.
- Generated Kotlin event classes now expose `mosaicName`, `mosaicPayload`, and
  `mosaicEnvelope`, giving Compose hosts the same target-neutral event map used
  by the HTML, Electron, SwiftUI, XAML, Qt, and Flutter shells.

### Fixed

- Text expressions that index a collection with an enclosing Mosaic `For`
  index now use Compose's internal Kotlin `Int` shadow. This keeps numeric loop
  comparisons type-correct while allowing toolkit patterns such as
  `bodies[i]` to compile as `Text(String)`.
- Mosaic text, number, collection, and nullable values now lower through a
  generated Kotlin truthiness helper anywhere Compose requires a Boolean,
  including `If`, state styles, checked/selected controls, disabled controls,
  and read-only inputs. Package-expanded TaskApp output now passes the Kotlin
  compiler instead of comparing dynamically typed values with `true`.
- `HostInput.onCommit` now supplies the controlled input value when the MIL
  event declares one payload parameter, while preserving data-object dispatch
  for parameterless commits.
- The legacy `Input ( multiline: true )` spelling now lowers to a native
  multiline `BasicTextField` with a useful editor-sized minimum line count.
  This preserves the multiline capability used by the shared Notes package
  without requiring app-owned Compose code.
- Generated optional boolean slot predicates now compile as nullable-safe Kotlin
  conditions, and large root containers split their direct children into private
  composables so generated Compose Desktop projects avoid JVM method-size
  limits.

## [0.1.0] - 2026-06-02

### Added

- New crate: Jetpack Compose / Compose Multiplatform backend for the Mosaic
  three-language pipeline.
- `from_pipeline(component, layout, style) -> PipelineEmitResult` emits a single
  `.kt` file containing a sealed-class event hierarchy plus a
  `@Composable fun <Component>(...)` function.
- Primitive coverage for v0.1.0:
  - `Box`, `Row`, `Column` -> `Box`, `Row`, `Column` composables
  - `Text` -> `Text(text = ...)`
  - `Spacer` -> `Spacer(modifier = Modifier.weight(1f))`
  - `HostInput` -> `BasicTextField` with strict-Flux dispatch wiring (looks up
    the referenced emit's arity to decide whether to pass `v` to the dispatched
    event)
  - `HostButton` -> `Button(onClick = { dispatch(...) }) { Text(...) }`
- Wired into `mosaic-compile` as `--backend compose`. Bare
  `mosaic-compile --backend compose --interface ... --layout ... --style ... -o
  Component.kt` works end-to-end on macOS arm64.
- 8 unit tests covering empty component, parameterless + payload-carrying emits,
  required + optional slot typing, the full FormulaBar shape end-to-end,
  parameterless button emit, and the unknown-primitive error path.

### Not Yet Implemented

- `Grid` built-in primitive (tracked as `grid-emit-compose` in the autonomous
  loop's roadmap).
- `For`, `If`/`Else` meta-primitives (UI29 sections 3.1 and 3.2).
- `HostTable`, `HostDialog`, `HostCheckbox`, `HostRadio`, `HostLink`,
  `HostNumberInput`, `HostScroll`, `HostTooltip` host primitives return
  `UnknownPrimitive` until their lowerings land.
- `mosstyle` style consumption: `.msl` input is accepted but not yet lowered
  into `Modifier` chains.
- `--emit-project` Android / Compose-Desktop app shell (analogous to the
  `mosaic-emit-flutter` and `mosaic-emit-xaml` project shells).
