# UI49 — Varying a mosstyle part by a slot value

**Tracked by:** [#14037](https://github.com/adhithyan15/coding-adventures/issues/14037)
**Status:** Specification complete; implementation continues in #14036
**Layer:** UI / mosstyle language + all nine emitters
**Depends on:** UI15 (mosstyle), UI27 §5 (structural states), UI41 (typed properties)
**Unblocks:** #14036 (six toolkit components with inert `variant`/`size`), and
through it #14017 and the component program (#14011)
**Landed foundation:** `one-of` slots in #14210

---

## 1. The question this settles

How does a mosstyle part look different depending on a slot value?

Every variant system needs this — `Button` primary vs danger, `Alert` success vs
warning, `Badge`, `Toast`, `Spinner`, and any future component with a keyword
axis. There is no answer today, which is why six toolkit components declare
`variant`/`size` slots whose values are accepted and discarded (#14036).

---

## 2. What exists today

Verified against `mosstyle-compiler` and the emitters, not read off the specs.

### 2.1 `state` — a closed, allowlisted sub-key

```mosstyle
part seg-list-off {
  color : "#6a625a" ;
  state hover {
    color : "#2b2723" ;
  }
}
```

`VALID_STATES` is closed, and an unknown name is an `UnknownState` error:

- **Interaction states** (UI15 §3.1) — `hover`, `pressed`, `focused`,
  `disabled`, `selected`, `editing`, `error`.
- **Structural states** (UI27 §5) — `even`, `odd`.

### 2.2 `even`/`odd` are already *data-resolved*, not input-driven

This is the load-bearing precedent for everything below. The compiler's own
comment says they are

> "not interaction states; the emitter resolves them at index time
> (`r % 2 === 0`) rather than from user input."

So the state mechanism **already** carries sub-keys chosen from data at render
rather than from an input device. A variant is the same shape: a sub-key chosen
from a slot value.

### 2.3 Every emitter already lowers `state` natively

React to CSS pseudo-classes and selected/hover handling, XAML to
`VisualState`, and Qt, Compose, and SwiftUI to their own equivalents. Whatever
mechanism this spec chooses, reusing `state` inherits nine working lowerings;
a new parallel construct needs nine new ones.

### 2.4 `elevation` is precedent for a typed, enum-restricted value

UI41 made `elevation` "mosstyle's first typed/enum-restricted property", with
`VALID_ELEVATION_VALUES = ["raised", "overlay"]`. Every other property is
freeform. So a closed value set inside mosstyle is established practice.

### 2.5 UI36 binding covers the *non*-enumerable case

TaskApp's `col-bar` background varies per `For` row with a value the stylesheet
cannot enumerate ahead of render, so it uses the UI36 `background` binding. Its
own comment records why `state` was not an option: `state-when-X` "only accepts
a fixed pseudo-state vocabulary, confirmed against the compiler's diagnostic."

**This splits the problem cleanly.** Values the stylesheet *can* enumerate
(variants, sizes) and values it *cannot* (an arbitrary runtime color) are
different problems. UI36 already owns the second. This spec owns only the first.

### 2.6 The current de-facto mechanism is part duplication

With no variant mechanism, authors duplicate the part: `pill-ok`/`pill-warn`,
`theme-toggle-sun`/`theme-toggle-moon`, `board-card`/`board-card-crit`, and — at
its worst — the 36 `seg-*` parts for one segmented control. **This is the
baseline any design must beat**, and it is the direct cause of TaskApp's 166
parts.

---

## 3. Decision

**A variant is a state whose selector is a slot value.** Rather than adding a
parallel construct, make the closed set of a slot's legal values *become* valid
state names for parts inside that component.

Two changes, one in each language.

### 3.1 `.mil` gains an enumerated slot type

```
component Button {
  slot variant : one-of primary secondary success danger warning info light dark ;
  slot size    : one-of sm md lg ;
  slot label   : text ;
}
```

Today `slot variant : text` puts the eight legal values in a **comment**, so
nothing can validate a value, reject a typo, or enumerate the set to check that
a story covers it. `elevation` (§2.4) is precedent that closed sets belong in
the language.

This is worth doing for its own sake. It also lets emitters lower to real
native enums instead of stringly-typed values.

The `one-of` syntax and compiler IR landed in #14210, and model-aware style
validation plus state-to-slot ownership landed in #14300. React activation
landed in #14306, followed by WebComponent runtime activation in #14314.
Compose runtime activation continues in #14320; the remaining lowerings and
toolkit retrofits stay tracked by #14036.

### 3.2 `.msl` state names may be a slot's enum values

**No new mosstyle syntax at all:**

```mosstyle
part button {
  padding      : 8 ;
  background   : "#0d6efd" ;
  state danger  { background : "#dc3545" ; border-color : "#dc3545" ; }
  state success { background : "#198754" ; border-color : "#198754" ; }
  state hover   { background : "#0b5ed7" ; }
}
```

`VALID_STATES` becomes `VALID_STATES ∪ {enum values of the component's
`one-of` slots}`. An unknown name stays an `UnknownState` error, so the
diagnostic that already exists keeps working and gets stricter rather than
looser.

### 3.3 Why this over the alternatives

The toolkit spec (§10) listed three candidates. Recording why each was rejected:

- **Sub-parts, `part button/primary`.** Introduces a second namespacing concept
  alongside `state`, needs new parsing, and needs nine new lowerings — while
  `state` already lowers everywhere. It also reads as a *different part* when it
  is the same part in a different condition.
- **A new `variant { }` block.** Honest but duplicative: it would be a second
  sub-key mechanism sitting beside `state`, with its own precedence rules
  against it. Two concepts where §2.2 shows one already covers both
  input-resolved and data-resolved selection.
- **Branching in the `.mll` with `If`.** Rejected in `Button.mll`'s own comment
  — it produces per-variant trees every backend then de-duplicates via styling
  anyway. UI30 rejected the same shape for layout variants, and TaskApp's
  36-part switcher is what it looks like at scale.

### 3.4 The cost of this choice

One concept carrying two selection sources is a real tension, and this spec
should not pretend otherwise. `state hover` is resolved by an input device;
`state danger` is resolved by a slot value. A reader cannot tell which from the
`.msl` alone — they must look at the `.mil`.

That is accepted because §2.2 shows the mechanism **already** does this with
`even`/`odd`, so the tension is pre-existing rather than introduced, and because
one mechanism with nine working lowerings beats two mechanisms with nine more to
write. The mitigation is diagnostics: an error naming a slot value that is not
declared, and a warning where a variant name shadows a built-in state (§4.1).

---

## 4. Rules

### 4.1 Collisions and ownership

A component may not declare an enum value equal to a built-in state name.
`error` and `selected` are already states, and a slot with a `selected` value
would make `state selected` ambiguous. This is a hard compile error naming both
the slot and the colliding built-in.

Enum values are also unique across all `one-of` slots in one component. For
example, `variant : one-of compact regular` and
`size : one-of compact spacious` is a compile error naming both slots and the
duplicate `compact` value. This keeps the stylesheet syntax flat while giving
every enum state exactly one owning slot.

### 4.2 Activation

An enum state is active when its owning slot equals that state's name. Given
`slot variant : one-of primary danger`, `state danger` means
`variant == "danger"`; it does not require a `state-when-danger` property in
the `.mll`. The compiler records the state-to-slot ownership for emitters.

Built-in interaction and structural states keep their existing activation
sources. They cannot be activated by a slot because §4.1 forbids an enum value
from reusing their names.

### 4.3 Precedence

Most specific wins, and the order is fixed so it is predictable:

1. base part properties
2. variant/enum states, in `.mil` slot declaration order when several apply
3. structural states (`even`/`odd`)
4. interaction states (`hover`, `pressed`, `focused`, `disabled`, …)

Interaction states are last because a pressed danger button must still look
pressed. Multiple enum slots (`variant` **and** `size`) both apply, and they
must not conflict on the same property — a `size` state setting `background` is
legal but suspicious, and is worth a lint rather than an error.

### 4.4 Theme composition

Unchanged. Variants live inside `.light.msl` / `.dark.msl` like everything else,
so variant colors differ per theme without a new axis.

### 4.5 What an unset slot does

An `one-of` slot with no value falls through to the base part properties. It is
not an error — that is exactly the "unset" story a component must render.

---

## 5. Per-backend lowering

Each emitter already has the target mechanism; the work is selecting on a slot
value instead of an input event, which `even`/`odd` already demonstrates.

| Backend | Lowering | Note |
| --- | --- | --- |
| `react` | Class or inline-style merge chosen from the slot value | Already merges state styles |
| `html` | Static: the resolved variant is baked at compile time | Snapshot backend; a fixture picks the value |
| `webcomponent` | Reflected attribute + internal style map | |
| `qt` | Property-based style selection | |
| `flutter` | Style chosen in the build method | |
| `compose` | Style chosen in the composable | |
| `swiftui` | Modifier chosen from the value | |
| `xaml` | `VisualStateManager` group, one state per value | Its natural shape — a group is a closed set |
| `paint` | Resolved at render from the fixture | |

Where a backend cannot express a case, it emits an explicit **degradation**
rather than silently dropping the variant — the failure mode #14036 already is.

---

## 6. Scope

**In:** enum slots in `.mil`; enum values as state names in `.msl`; precedence;
diagnostics; the nine lowerings.

**Out:** non-enumerable data-driven styling, which is UI36's job (§2.5).
Retrofitting the six affected toolkit components, which is #14036. Deciding
whether existing `text` slots migrate to `one-of` — mechanical, and better as
its own pass once the mechanism is proven on one component.

---

## 7. Decisions completed while specifying

1. **Enum state names stay flat.** `state danger` is intentionally preferred to
   `state variant:danger`. Collisions between slots are compile errors (§4.1),
   which preserves a unique state-to-slot mapping without adding syntax.
2. **UI49 does not require native enum types.** Hosts keep their existing
   string representation; TypeScript may retain the union type introduced in
   #14210. Native enums can be added later without changing the styling
   contract or blocking the nine runtime lowerings.
3. **Is `disabled` special?** Checked: **latent, not live.** Nine components
   declare a `disabled` slot — `Button`, `Checkbox`, `Input`, `Field`,
   `InputGroup`, `NumberInput`, `Radio`, `Select`, `Slider` — and **none** of
   them uses `state disabled` in its stylesheet, so no collision exists today.
   §4.1 must still forbid it, because the first component to style a disabled
   state would create one silently.

   The check surfaced something worth stating: `disabled` is a `bool` slot with
   no styling, yet it is **not** inert the way `variant` is. `Button.mll`
   forwards it to `HostButton`, which paints itself disabled natively. That is
   the real distinction — **a slot that maps onto a host primitive's own
   property gets platform behavior for free; a purely stylistic slot has
   nowhere to go and is dropped.** `variant` and `size` are in the second
   category, which is exactly why they vanished and why this spec is needed.
