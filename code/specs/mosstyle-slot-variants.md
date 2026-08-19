# mosstyle slot variants, isolation, and style extension

**Status:** Specification
**Scope:** `mosstyle-compiler`, every `mosaic-emit-*` backend, `mosaic-pkg-toolkit`

---

## 1. Motivation

`mosaic-pkg-toolkit`'s `Button` declares a Bootstrap-shaped API — eight
variants (`primary`, `secondary`, `success`, `danger`, `warning`, `info`,
`light`, `dark`) and three sizes (`sm`, `md`, `lg`). None of it does anything.
`Button.light.msl` says so outright:

> the variant slot is accepted by the `.mil` and stays unused at the styling
> layer — every variant renders with the base style. That's intentional: it
> lets us land the API surface without blocking on the mosstyle sub-part
> design.

This spec is that design.

Three goals, in the user's words:

1. **Every slot declared in `.mil` should be usable in `.msl`**, with `if`/`else`
   *and* `switch`/`case` over its values.
2. **Each style should be authorable in complete isolation** — a designer can
   change one variant without reading or risking another.
3. **Styles should be overridable** — an app extends a Mosaic component and
   overrides specific styles from it.

---

## 2. What already exists

Worth stating precisely, because the gap is smaller than it looks.

`.mll` can already bind a **slot** to a styling condition:

```
Box [ card ] ( state-when-selected -> slot: is-selected )
```

```
part card {
  background : "#ffffff" ;
  selected { background : "#0d6efd" ; }
}
```

That is slot-driven conditional styling, working today, lowered by every
backend — WinUI `VisualState`, Compose conditional modifiers, a CSS class.

Two limits:

- **The condition names are a fixed list of nine** (`VALID_STATES` in
  `mosstyle-compiler`): `hover`, `pressed`, `focused`, `disabled`, `selected`,
  `editing`, `error`, `even`, `odd`. There is no way to say `primary`.
- **`.msl` never sees slots.** The `.mll` mediates. A designer editing a
  stylesheet cannot tell which conditions exist without reading the layout.

There is **no inheritance of any kind** in mosstyle today.

---

## 3. Design

### 3.0 Bindings use `->`, not a second colon

Today a bound property stacks two colons that mean different things:

```
state-when-selected : slot: is-selected
label               : slot: label
onClick             : emit: onClick
```

The first colon is assignment. The second is a namespace tag on the *value*
(`slot:`, `emit:`). Reading `label : slot: label` requires knowing that, and
the two colons look identical while doing unrelated jobs.

**Bound properties use an arrow; literal properties keep the colon:**

```
label   -> slot: label        // bound to a slot
onClick -> emit: onClick      // bound to an event
padding :  8                  // set to a literal
```

This is more than cosmetic. The arrow marks *dataflow*, so a reader can scan a
layout and see at a glance which properties are dynamic and which are fixed —
a distinction that currently requires parsing the value. Exactly one colon
remains per line, and it always means assignment.

**Migration cost, stated plainly:** 98 `.mll` files, roughly 1,157 `slot:` and
535 `emit:` references. The source change is mechanical and regex-able, but it
is a grammar change in `moslayout-compiler`, and every backend's golden tests
move with it. That is a real cost for a readability win; it is worth doing
*before* this spec's other work adds more binding sites, not after.

The `slot:` / `emit:` tags stay as-is on the right of the arrow. They
disambiguate *what kind* of thing is being bound, which is still useful.

### 3.0.1 The style name must match the component name

**Normative.** A stylesheet's declared name must equal the `component` name in
the `.mil` it is compiled against. A mismatch is a **compile error**, not a
warning.

```
Button.mil          component Button { … }
Button.light.msl    style Button { … }        ✅
Button.light.msl    style Buton  { … }        ❌ compile error
```

Nothing enforces this today — verified: there is no name comparison anywhere in
`mosaic-compile` or `mosstyle-compiler`, and no mismatch diagnostic exists. A
stylesheet naming the wrong component compiles cleanly and produces an
**unstyled** component, because every part lookup misses. That is the same
silent-failure shape as a dropped style property or a mode-less binding: the
build is green and the output is wrong.

The rule extends to the two new forms in this spec:

- a variant file's `of <Component>` must match the same component name
- an extending style's own name must match *its* `.mil` (§3.4), while the name
  after `extends` must resolve to a real component elsewhere

**This is independently shippable and should land first** — see Phase 0. It is
a small validation with no dependency on the rest of the design, and it closes
a live failure mode.

### 3.1 Slots are readable in `.msl`

A stylesheet may reference any slot its component's `.mil` declares:

```
style Button {
  part button {
    padding       : 8 ;
    border-radius : 4 ;
  }
}
```

```
// Button.primary.msl
variant primary of Button {
  part button {
    background   : "#0d6efd" ;
    color        : "#ffffff" ;
    border-color : "#0d6efd" ;
  }
}
```

This introduces a **new `.msl` → `.mil` dependency**: the stylesheet compiler
must read the interface to know which slots exist and reject references to
slots that do not. That dependency is the point — it is what lets a designer
work from the stylesheet alone.

### 3.2 `switch` / `case` over a slot value

For a slot with several values, a `switch` replaces N boolean bindings:

```
style Button {
  part button { padding : 8 ; }

  switch ( slot: size ) {
    case "sm" { part button { padding : 4 ; font-size : 12 ; } }
    case "md" { part button { padding : 8 ; font-size : 14 ; } }
    case "lg" { part button { padding : 12 ; font-size : 16 ; } }
    default   { part button { padding : 8 ; font-size : 14 ; } }
  }
}
```

`if` / `else` remains available for boolean slots and is defined as sugar over
a two-case `switch`.

**Selection is at runtime.** A slot carries a runtime value, so the emitter
emits every branch and selects among them at run time — exactly as it already
does for states. This is deliberate: compile-time resolution would only work
when the call site passes a literal, and would silently degrade otherwise.

`default` is required when the cases are not exhaustive, so an unexpected
value renders something rather than nothing. This is the failure mode that
made an empty `Button` invisible for weeks.

### 3.3 One file per variant

**Decision: one file per variant.**

```
mosaic-pkg-toolkit/src/
  Button.mil
  Button.mll
  Button.light.msl      base shape, theme-level
  Button.dark.msl
  Button.primary.msl    variant styles
  Button.danger.msl
  Button.success.msl
  …
```

A designer opens `Button.danger.msl`, changes it, and **cannot** affect any
other variant. Isolation is enforced by the filesystem rather than by
convention.

The compiler discovers `<Component>.<variant>.msl` by the same base-name
pairing that already finds `<Component>.light.msl`. `light`/`dark` remain
reserved theme names and are not variants.

Trade-off accepted: more files per component, and the mapping from variant to
file is implied by the filename rather than written out. The `switch` form in
§3.2 stays available for slots where a single file reads better — `size`, with
three tightly-related cases, is the motivating example.

### 3.4 `extends` with property-level merge

**Decision: property-level merge.**

```
// app-side
style CheckoutButton extends toolkit::Button {
  part button {
    border-radius : 999 ;
  }
}
```

`CheckoutButton` is `Button` with exactly one property changed. Padding,
colours, every variant and every state are inherited untouched.

**Precedence**, lowest to highest:

1. base `part` from the extended component
2. base `part` from the extending style
3. variant block (extended, then extending)
4. state block (`hover`, `selected`, …), same order

A property set at a lower level and not restated at a higher one is inherited.
This is the CSS cascade restricted to a fixed, declared set of levels — no
selector specificity, no source-order surprises.

**A base override reaches into variants.** In the example above,
`border-radius: 999` applies to `primary`, `danger`, and every other variant,
because none of them set `border-radius`. A variant that *does* set it keeps
its own. This is what makes "override specific styles" true: you change the
one thing you meant to and inherit the rest.

Extension across packages uses the same `pkg::` reference syntax the layout
layer already has for component references.

---

## 4. Backend lowering

Every backend already lowers state-conditional styling. Variants reuse that
path; the new work is widening it beyond the nine fixed state names.

| Backend | Existing state mechanism | Variant lowering |
|---|---|---|
| XAML | `VisualStateManager` groups | one `VisualState` per case, group per switch |
| Compose | conditional modifiers | `when (variant) { … }` |
| SwiftUI | conditional view modifiers | `switch variant { … }` |
| Qt | QML property bindings | `state` list with `when:` |
| Flutter | conditional widget properties | `switch` in the build method |
| React / HTML / WebComponent | CSS classes | one class per case |

**Every dropped case must be reported as a degradation.** A backend that
cannot express a variant must say so rather than silently rendering the base —
see the hard-fail gate already agreed for dropped style properties.

---

## 5. Phasing

Each phase is independently shippable and independently verifiable.

**Phase 0 — the two cheap correctness wins.** The style/component name check
(§3.0.1) and the `->` binding syntax (§3.0). Neither depends on the rest, both
close or prevent silent failures, and the syntax change gets cheaper the
earlier it lands. Verify: a deliberately mismatched stylesheet fails the build;
every existing component still emits byte-identical output after migration.

**Phase 1 — slots readable in `.msl`.** Wire the `.mil` dependency, allow
`switch` / `case` / `default` over a slot, keep the existing fixed states
working unchanged. Verify: `Button`'s `size` slot changes padding on XAML and
Compose.

**Phase 2 — one file per variant.** Discovery of `<Component>.<variant>.msl`
and the merge into the base. Verify: `Button.danger.msl` renders red while
`Button.primary.msl` renders blue, in the same build.

**Phase 3 — `extends` with property-level merge.** Cross-package references
and the precedence chain in §3.4. Verify: an app-side extension changes one
property and inherits every variant.

**Phase 4 — populate the toolkit.** Write the eight Button variants and three
sizes, then the same treatment for the next components up the complexity
ladder.

Phase 1 is worth landing alone: it makes `size` work, which is the smallest
end-to-end proof that a slot can drive styling on a native backend.

---

## 6. Verification

Unit tests cannot answer "does the danger button look red." Each phase needs a
**visual** check on real native output — MosaicBook rendering the component per
backend, or a captured screenshot.

The minimum bar per phase: build the component on every locally-available
native backend and look at it. That is currently XAML, Compose, Flutter and Qt;
SwiftUI needs macOS and goes through CI.

---

## 7. Open questions

- **Do the eight Bootstrap variant names stay?** They are inherited from
  whoever wrote `Button.mil`, not chosen deliberately. A toolkit that ships no
  opinionated variants — and simply styles well from the app side — is a
  legitimate alternative, and would mean deleting the `variant` slot rather
  than implementing it. This spec assumes they stay; that assumption should be
  confirmed before Phase 4.
- **Should `size` be a switch or separate files?** It is three tightly-related
  cases where seeing all three together helps. Suggest `switch`, with §3.3's
  per-file form reserved for variants proper.
- **Does a variant compose with a theme?** `Button.danger.msl` versus
  `Button.danger.dark.msl`. Deferred until a dark-mode variant actually
  diverges.
