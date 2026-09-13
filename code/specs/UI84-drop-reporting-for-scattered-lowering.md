# UI84 — drop reporting for scattered lowering

Issue: [#12022](https://github.com/adhithyan15/coding-adventures/issues/12022)

Three backends report the style properties they cannot lower. Two do not, and
the reason is structural rather than neglect. This spec says how a backend whose
lowering is **scattered across many call sites** reports its drops without the
parallel list the repository has already ruled out.

## 1. Measured, not assumed

`mosaic-package-artifact-builder`'s degradation analyzer, at the `match
opts.backend` that populates `styleDegradations`:

| backend | reports drops | lowering shape |
| --- | --- | --- |
| XAML | yes | one `match` over property names |
| SwiftUI | yes | one `match` (`swiftui_modifier_chain_with_drops`) |
| Compose | yes | one `match` |
| **Flutter** | **no** | 48 scattered lookups |
| **Qt** | **no** | 81 scattered lookups |

The fallthrough arm says so in its own words:

> The remaining backends do not report their drops yet, so an empty
> `styleDegradations` means "nobody looked" there rather than "nothing was
> lost".

This matters beyond tidiness. Engram ships artifacts on Qt and Flutter, and for
both an empty `styleDegradations` is evidence of nothing. UI79's table records
the same two backends as the ones whose per-edge border drop goes unreported.

## 2. The trap this spec exists to prevent

`mosaic-emit-flutter` has a function that looks like the hook and is not.
`style_prop_to_container_arg(prop) -> Option<String>` is a single `match`
returning `None` for anything it does not know, and its own doc says *"unknown
props produce `None` and are silently dropped"*. It is the obvious place to add
a `_with_drops` variant.

It handles **six** properties — `background-color`, `color`, `height`,
`min-height`, `padding`, `width` — and covers only the `Container`-args path.
`font-size`, `text-align` and `border-radius` are lowered by other functions
entirely.

Scoping this work by extracting that match's keys and diffing against every
property authored in the repository's `.msl` files yields "59 properties
dropped". A reporter built on it would emit roughly **53 false drops**.

That is not hypothetical. PR #15108 fixed exactly this bug on SwiftUI at smaller
scale: 22 of the 40 drops reported for Engram were false, because `gap` is
lowered at view-construction time (`HStack(spacing:)`) where the modifier-chain
scan could not see it. A false drop is not a harmless extra line — it is carried
in `ALLOWED_STYLE_DROPS`, where it reads as a standing licence for a property
that genuinely stops being applied.

## 3. The rule

> A drop is reported when **no code path consumed the property**, recorded by
> the lowering itself. Never by comparing against a list of supported names.

The second sentence is the repository's existing rule, stated when SwiftUI began
reporting: drops are collected from the same `match` that does the lowering,
*"not from a parallel list of supported names. A parallel list goes stale
silently the moment a property stops being lowered, which is the exact failure
mode being reported on."*

A single `match` satisfies that rule by construction. Scattered lowering needs a
mechanism that satisfies it without one.

## 4. Consumption recording

Replace the property accessor, not the lowering.

Qt reads properties through `style_prop(props, "name")`; Flutter through that
plus `props.get("name")`. Each becomes a call that **records the name it was
asked for** against the part and node currently being emitted, in a recorder
threaded through the emit context. The lowering is unchanged; only the accessor
learns to leave a trace.

Drops are then the complement:

```
dropped(part) = authored(part) − consumed(part)
```

where `authored(part)` comes from `style.parts[].base` — the same source the
reporting backends walk.

Three properties of this shape are worth stating, because they are why it is
preferred to a restructure:

1. **It cannot go stale.** A property that stops being lowered stops being
   asked for, so it starts being reported, with no list to update. That is the
   same guarantee the single-`match` form gives.
2. **It needs no restructure.** The change is mechanical at every call site and
   leaves the emitters' shape alone, which matters when there are 129 of them
   across two crates.
3. **It is accurate under context-dependent lowering**, which a single `match`
   is not. Qt lowers the same property differently for different widget kinds;
   a match keyed only on the name cannot express "consumed here, dropped
   there".

## 5. Every occurrence, not merely one

A part name can be bound to more than one node — package resolution substitutes
a `pkg::` reference with the resolved sub-tree, so one composed layout can carry
a part from an inlined package beside a same-named part on a different tag.

> A part's property is dropped unless **every** node bearing that part consumed
> it.

This is not a refinement invented here. `mosaic-emit-compose`'s
`container_argument_covers` already ends `on.iter().all(|c| carried_by(c))`, and
argues it for a part shared between a `Row` and a `Text` — genuinely dropped on
the `Text`. PR #15108's first version of the SwiftUI equivalent used `any`,
which is the too-wide direction, and it was caught in review rather than by a
test, because no layout in the repository triggers it today.

The recorder therefore keys on `(part, node)`, and a part is reported unless the
consumption set covers every node bearing it.

## 6. Verification

A drop reporter is only useful if it is accurate in **both** directions, so each
backend needs both halves and neither is optional:

1. a package authoring a property the backend **does** lower, by a path the
   recorder must see, asserted **not** reported;
2. a package authoring one genuinely dropped, asserted **reported**;
3. a part bound to two nodes where only one consumes, asserted **reported**;
4. all three mutation-tested — a reporter that reports everything and one that
   reports nothing both look plausible in a diff, and the `any`/`all` distinction
   in §5 is invisible without a mutation that isolates it.

## 7. Expect the sweep

When SwiftUI began reporting (#14728), previously invisible losses appeared at
once and every gated package needed `ALLOWED_STYLE_DROPS` entries with reasons.
There are 13 such gate files. The same will happen here, and
`no_pinned_style_drop_has_silently_been_fixed` will then hold those entries
honest — it already found 11 of 21 pins stale on its first run.

**One backend per change.** The sweep alone is large, and landing two backends'
worth of new degradations together makes the allowlist diff unreadable.

## 8. Out of scope

- **Lowering the properties.** This spec makes the losses visible; closing them
  is per-property work, as UI79 is for per-edge borders.
- **The other three backends.** XAML, SwiftUI and Compose already report, and
  converting them to the recorder would be churn against working code. The two
  shapes coexist; the `match` form is right where a `match` is the lowering.
