# UI57 — Activating a built-in state from a bool slot

Issue: [#14639](https://github.com/adhithyan15/coding-adventures/issues/14639)

Extends [UI49](UI49-mosstyle-slot-states.md), which settled how a *one-of* slot
value selects a mosstyle state. This settles the case UI49 §7.3 flagged and
deferred: a **bool** slot whose name is a built-in state.

## 1. The question this settles

`Button` declares `slot disabled : bool`. `disabled` is already a built-in
mosstyle state. Can

```
part button {
  background : "#0d6efd" ;
  state disabled {
    opacity : $opacity-disabled ;
  }
}
```

mean "when the `disabled` slot is true"? Today it compiles, validates, is
carried in the IR, and is activated by nobody.

## 2. What exists today, measured

Every backend lowers `disabled` to the native control property. This was
measured by emitting `Button` on all nine, not assumed:

| Backend | Lowering |
| --- | --- |
| react | `disabled={disabled}` |
| webcomponent | `${disabled ? " disabled" : ""}` |
| html | `data-disabled="{{disabled}}"`, reflected by `main.js` to `element.disabled` |
| swiftui | `.disabled(disabled)` |
| flutter | `onPressed: _mosaicTruthy(disabled) ? null : ...` |
| qt | `enabled: !disabled` |
| compose | `enabled = !_mosaicTruthy(disabled)` |
| xaml | `IsEnabled="{x:Bind Not(Disabled), Mode=OneWay}"` |
| paint | scene renderer; no interaction model |

So the control is genuinely inert everywhere. UI49 §7.3 called this correctly:
*a slot that maps onto a host primitive's own property gets platform behavior
for free.*

### 2.1 Free platform behavior is not free platform *appearance*

Serving the emitted `Disabled` and `Primary` stories and reading the computed
style in a browser:

| | Primary | Disabled |
| --- | --- | --- |
| `button.disabled` | `false` | **`true`** |
| `background-color` | `rgb(13, 110, 253)` | `rgb(13, 110, 253)` |
| `opacity` | `1` | `1` |

A browser greys a disabled button by changing its *default* colors. Any part
that sets `background` explicitly — which is every styled Mosaic part — wins
over that default, so the DOM backends render an inert button that looks
enabled. The native backends composite a disabled effect *over* the control
rather than changing its defaults, so they still look disabled.

This is the general shape of the problem, not a Button quirk: **platform
disabled painting is defeated by any explicit background**, and Mosaic's whole
premise is explicit styling.

### 2.2 The authored fix does not reach any emitter

`mosstyle-compiler` deliberately gives built-in states no owning slot
(`lib.rs`, `resolve_slot_states`):

```rust
if VALID_STATES.contains(&state.state.as_str()) {
    state.slot = None;
}
```

Every emitter activates only states that have one. `mosaic-emit-react` says so
in a comment: *"Unwired built-in state blocks (`state hover { ... }`, `state
focused { ... }`) still need CSS-class plumbing or a runtime pseudo-class
observer."*

`state disabled` is that case. It is parsed, validated, carried, and dropped.

## 3. Decision

**A built-in state whose name matches a declared `bool` slot is activated by
that slot's truthiness.**

UI49's thesis was *a variant is a state whose selector is a slot value*. UI57
adds: *a built-in state is a state whose selector may be a slot's truth*.

`disabled` needs no runtime observer, because the component already declares
the truth it depends on. That is what separates it from `hover`, `pressed`,
and `focused`, which have no slot and stay out of scope.

### 3.1 Why not fix the emitters instead

The alternative is for each DOM emitter to inject an opacity whenever
`disabled` is truthy. Rejected: the emitter would be inventing a visual
treatment that no stylesheet asked for, it would differ per backend, and it
would not be themeable. UI49's whole point is that appearance is authored in
`.msl`; this keeps it there.

### 3.2 Why not require an explicit opt-in syntax

`state disabled when slot disabled { }` was considered and rejected. There is
exactly one bool slot named `disabled`, the state name and slot name already
agree, and UI49 established that state names are flat and unqualified (§7.1).
Adding syntax to restate an unambiguous match is cost without a decision.

## 4. Rules

### 4.1 Binding

A `state <name>` block binds to a bool slot when **all** hold:

1. `<name>` is in `VALID_STATES`;
2. the component declares a slot named exactly `<name>`; and
3. that slot's type is `bool`.

The compiler records the owning slot, exactly as it does for enum states.

If the component declares no such slot, the state keeps `slot: None` and its
existing (unwired) behavior. This is what keeps the change backward
compatible: no stylesheet that compiles today changes meaning unless it also
declares a matching bool slot.

A slot of a non-bool type sharing a built-in state's name does **not** bind. It
is a compile error naming both, because the author almost certainly meant the
state to apply and silently ignoring it is how #14639 happened.

### 4.2 Activation

Active when the slot is truthy, using each backend's existing truthiness
helper — `_mosaicTruthy` on Flutter and Compose, `truthy()` in the html
runtime, and the native boolean elsewhere. Not string equality: `disabled ==
"disabled"` is what the enum path would have emitted and is always false.

### 4.3 Precedence

UI49 §4.3 is unchanged and already places interaction states last:

1. base part properties
2. enum states, in `.mil` slot declaration order
3. structural states (`even`/`odd`)
4. interaction states (`hover`, `pressed`, `focused`, `disabled`, …)

A disabled danger button is danger-colored *and* dimmed, because `disabled`
sets `opacity` and the variant sets `background`. They compose rather than
conflict. Where a built-in state and an enum state set the *same* property, the
built-in wins — that is what "most specific last" already means.

### 4.4 Interaction with platform painting

Both apply, deliberately. The control keeps its native inertness — focus
skipping, pointer rejection, assistive-technology state — and gains the
authored appearance on top. UI57 does not suppress, replace, or detect the
platform treatment; a backend that already dims will dim slightly further,
which is correct for a state whose whole purpose is to read as unavailable.

### 4.5 Which built-ins this covers

`disabled` today. `error`, `selected`, and `editing` are bool-slot-shaped and
bind under the same rule the moment a component declares a matching bool slot;
no further spec work is needed for them.

`hover`, `pressed`, and `focused` are **out**. They describe transient input
device state, no component declares them as slots, and they need the CSS-class
plumbing or pseudo-class observer the react comment describes. `even`/`odd`
are structural and resolved at index time.

## 5. Per-backend lowering

Each emitter has exactly one slot-state activation site, so each gains one
branch. The condition is the only difference from the enum path:

| Backend | Enum condition (UI49) | Bool condition (UI57) |
| --- | --- | --- |
| react | `variant === "danger"` | `disabled` |
| webcomponent | `variant === "danger"` | `disabled` |
| html | resolved at emit from the fixture | truthy fixture value |
| swiftui | `variant == "danger"` | `disabled` |
| qt | `variant === "danger"` | `disabled` |
| flutter | `variant == "danger"` | `_mosaicTruthy(disabled)` |
| compose | `variant == "danger"` | `_mosaicTruthy(disabled)` |
| xaml | value converter on the DP | `Disabled` DP directly |
| paint | fixture-selected state | truthy fixture value |

html resolves states at emit time from the active fixture rather than emitting
a runtime conditional, so its change is to treat a truthy fixture value as
activating rather than comparing it to the state name.

## 6. Scope

**In:** the binding rule, the compile error for a same-named non-bool slot,
activation on all nine backends, and the `opacity-disabled` token gaining its
first caller.

**Out:** `hover`/`pressed`/`focused` wiring, which needs a runtime observer and
is a separate problem. Authoring the toolkit's disabled treatment across the
eight components that declare the slot, which stays on #14639 — UI57 is the
mechanism, #14639 is the styling that consumes it.

## 7. Decisions completed while specifying

1. **Does this collide with UI49 §4.1?** No. §4.1 forbids an *enum value* equal
   to a built-in state name, so that `state selected` is never ambiguous
   between a one-of slot and the built-in. A *bool* slot named `disabled` is
   not an enum value and creates no ambiguity: there is exactly one candidate
   meaning. §4.1 stands unchanged.

2. **Should the token be the default treatment?** `opacity-disabled: 0.4` has
   existed in the token table with no callers since tokens were introduced.
   UI57 does not make it automatic — a stylesheet must ask for it. Automatic
   application would be §3.1's rejected emitter-invents-appearance path wearing
   a token.

3. **What about a bool slot that is not forwarded to a host primitive?** It
   still binds. Nothing in the rule depends on the slot reaching a host
   property; a purely stylistic bool slot is exactly the "nowhere to go" case
   UI49 §7.3 described, and this gives it somewhere to go.
