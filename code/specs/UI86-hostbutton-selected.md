# UI86 — `HostButton` gains a `selected` prop

Issue: [#15420](https://github.com/adhithyan15/coding-adventures/issues/15420)

**Status:** Specification (draft). Implementation follows in the slices listed
in §8.

Extends [UI29](UI29-primitive-kernel.md)'s primitive kernel. `HostButton` has an
accessible **name** (`a11y-label`, UI29 §2.1) but no selected **state**, so a
component cannot tell assistive technology which of its options is the current
one. Every native surface that could carry that state exists, and no emitter
writes to any of them.

## 1. The question this settles

Four toolkit components draw a set of options, one of which is current:

| component | primitive | how the current option is marked today |
| --- | --- | --- |
| `SegmentedControl` | `HostButton` | a distinct part, **plus** the state written into the accessible name by the host (`"Board, selected"`) |
| `Tabs` | `HostButton` | a distinct part only |
| `ListGroup` | `HostButton` | a distinct part only |
| `Nav` | `HostLink` | a distinct part only |

A distinct part is purely visual. A screen reader user hears every option the
same way, except in `SegmentedControl`, where they hear the *word* "selected"
as part of the name rather than the platform's selected trait. That stand-in is
better than silence and worse than native: the word is not localised with the
platform, it is read where the trait would be announced as state, and it forces
every host to build the name (Engram's `switcher_rows`, Trestle's
`switcher_views`) because the layout language has no string concatenation.

This spec gives `HostButton` a portable selected state. `HostLink` is a
different semantic ("current page") and is tracked separately (§7).

## 2. Measured: nothing is emitted today

A search of all eight emitters for `aria-pressed`, `aria-selected`,
`aria-current`, `isSelected`, `Accessible.checked` and `Semantics(selected`
finds no `HostButton` lowering on any backend. The artifact builder's
`ignored_native_property` has no `HostButton` arm, so a `selected` prop authored
today is dropped silently rather than reported.

The native surfaces that exist:

| backend | surface | notes |
| --- | --- | --- |
| React / HTML / WebComponent | `aria-pressed="true\|false"` | valid on the native `button` role |
| SwiftUI | `.accessibilityAddTraits(.isSelected)` | a trait, added or not |
| Compose | `Modifier.semantics { selected = … }` | the button already has a `semantics` block for its name |
| Flutter | `Semantics(selected: …)` | the button is already wrapped in `Semantics` for its name |
| Qt | `Accessible.checkable` / `Accessible.checked` | attached properties; independent of the control's own `checkable` |
| XAML | `ToggleButton.IsChecked` | a WinUI `Button` has no selection or toggle pattern |

## 3. What "selected" means here

**`selected` means "this button is the current option of the set it belongs
to."** It is a state the *application* owns: the button reports it, and never
changes it itself. Clicking a button still only fires `onClick`; the host
decides what becomes selected, and the next props say so.

Each platform announces that state in its own vocabulary, and the spec accepts
the nearest one rather than inventing a role:

| platform | announced as |
| --- | --- |
| web | "toggle button, pressed" / "not pressed" |
| Qt, WinUI | "checked" / "not checked" |
| SwiftUI, Compose, Flutter | "selected" |

### 3.1 Why `aria-pressed`, not `aria-selected` or `aria-current`

- `aria-selected` is **not permitted** on the `button` role (it belongs to
  `tab`, `option`, `gridcell`, `row`), and UI29 §2.1 requires `HostButton` to
  keep the platform button role. Changing the role is the job of a group
  primitive (§6), which is where `role="tab"` and `role="radio"` belong.
- `aria-current` means "the current item within a set of *locations*" and is the
  right surface for `HostLink` (§7), not for a button that switches a view.
- `aria-pressed` is the WAI-ARIA toggle-button state and is valid on
  `button`; the APG's toolbar example uses it for its toggle buttons.

### 3.2 Absent is not the same as false

A `HostButton` **without** `selected` is an ordinary push button: nothing is
emitted, and no platform announces a state.

A `HostButton` **with** `selected` is a selectable option, and its state is
emitted whether it is true or false: `aria-pressed="false"`, `selected = false`,
`Accessible.checkable: true` with `checked: false`. That is what tells a screen
reader "this is one of a set, and it is not the current one." Emitting nothing
for false would make the unselected options indistinguishable from push buttons.

## 4. Decision

```
HostButton [ part ] (
  label : …,
  selected : <slot ref | true | false | expression>,
  onClick : emit: …
)
```

- **Type:** bool. Non-bool values are a compile error in the layout compiler
  where the type is knowable (a slot declared with a non-bool type), and
  truthiness applies to expressions, as for `If`.
- **Value shapes:** slot reference, keyword (`true`/`false`), and expression.
  **All three are required on every backend.** The toolkit's use is
  `selected : ( i == selectedIndex )` inside `For`, so a backend that accepts
  only slot references would lower none of the real call sites. (This is the
  gap `checked` has today: React and SwiftUI accept only slots, and Qt turns an
  expression into `checked: false` without reporting it.)
- **Liveness:** the value is re-evaluated on every render, including inside
  repeated rows, under the same rule UI29 §2.1 sets for `a11y-label`.
- **Only booleans reach the output, and never as raw source.** On every
  backend the emitted state is a normalised boolean (`true`/`false`, or the
  platform's spelling). An expression is lowered only through the emitter's
  existing expression and data-path machinery, the same functions `checked` and
  `a11y-label` use, and is escaped for its attribute context. Expression text is
  never spliced into generated markup or code: `<` and `&` would need escaping
  in XAML attributes, and `x:Bind` does not accept `==`, so a spliced
  `( i == selectedIndex )` would be broken at best and injectable at worst.
- **No behaviour:** `selected` never changes what a click does, never toggles
  local state, and never disables the button. It does not affect styling; see
  §5.

### 4.1 Lowering

| backend | `selected` present | notes |
| --- | --- | --- |
| React | `aria-pressed={Boolean(expr)}` | |
| HTML | `aria-pressed="{{…}}"` rendering `true`/`false` | expression values go through the same data-path rule as `a11y-label`; a non-path expression is a **reported** degradation, not a silent drop |
| WebComponent | `aria-pressed="${…}"` | through `escapeHtmlAttribute`, like the name |
| SwiftUI | `.accessibilityAddTraits((expr) ? .isSelected : [])` | added after `.accessibilityLabel`, so both apply |
| Compose | `selected = (expr)` inside the button's existing `Modifier.semantics { … }` | booleans pass through `_mosaicTruthy` as for `checked` |
| Flutter | `selected: (expr)` on the existing `Semantics` wrapper | via `bool_prop_expression` |
| Qt | `property bool mosaicSelected: Boolean(expr)`, pushed to `Accessible.checkable: true` / `Accessible.checked` | the attached properties only; the `Button`'s own `checkable` stays false so a click does not toggle it; see §4.2.1 |
| XAML | **not yet lowered**: reported as a degradation (§4.2, #15463) | the proposed lowering is a `ToggleButton` with `IsChecked="{x:Bind …, Mode=OneWay}"` |

### 4.2 XAML: a toggle that does not toggle itself

WinUI's `Button` automation peer implements only the Invoke pattern, so there is
no attached property that adds a selected or toggle state to it. `ToggleButton`
implements the Toggle pattern and is announced as checked or not checked, which
is what is needed.

A `ToggleButton` toggles `IsChecked` on click, which would violate §3 (the
application owns the state) and would break a `OneWay` binding. The generated
click handler therefore dispatches `onClick` and then **restores**
`IsChecked` from the bound value. The next props update corrects it anyway if
the host changed the selection. The restoration is part of the lowering and must
be covered by a test that clicks an unselected option whose host does *not*
select it and asserts that it stays unchecked.

**Status (slice 3):** the fallback below is in effect, and the `ToggleButton`
lowering is tracked in
[#15463](https://github.com/adhithyan15/coding-adventures/issues/15463).
Two further problems turned up that this section did not anticipate, and
neither can be checked without running WinUI:

- **Checked styling.** The default `ToggleButton` template's Checked states
  apply the accent brushes, overriding the authored part style. §5 rules that
  out.
- **Toggle without Click.** A screen reader's Toggle action reaches
  `OnToggle`, which may not raise `Click`. That would flip the state without
  dispatching, and without running the restore.

If the implementation finds this cannot be made reliable, the fallback is a
reported degradation (§4.3) on XAML, not a silent `Button`.

### 4.3 Degradation

`ignored_native_property` gains a `("HostButton", "selected")` arm. Following
the `HostCheckbox` `indeterminate` precedent, it asks each native emitter's own
predicate (`host_button_selected_is_native(node)`), the same function the
lowering uses, so the report cannot drift from what was emitted. A backend or
value shape that is not lowered produces
`accessibility.button-selected-unsupported` and makes the build not
native-complete, in line with UI84 §3 ("recorded by the lowering itself").

Web backends are not native-complete targets, but their emitter tests must
assert all three value shapes, and HTML's non-path-expression case must be an
error or a reported drop, never a silent one.

### 4.2.1 Qt: the state is pushed, not only bound

*(Found while implementing slice 3.)*

**The problem.** When accessibility becomes active,
`QQuickAbstractButton::accessibilityActiveChanged` writes the Button's own
`checked` and `checkable` onto its `Accessible` attached object from C++. Both
are false here, because the Button's `checkable` must stay false. A constant
`Accessible.checkable: true` is never evaluated again, so it would be lost for
good. `Accessible.checked` would be wrong until the selection next changed.

**The lowering.** The state therefore lives in a property of its own, and is
pushed onto the attached object:

```qml
property bool mosaicSelected: Boolean(expr)
Accessible.checkable: true
Accessible.checked: mosaicSelected
onMosaicSelectedChanged: Accessible.checked = mosaicSelected
Accessible.onCheckableChanged: if (!Accessible.checkable) Accessible.checkable = true
Accessible.onCheckedChanged: if (Accessible.checked !== mosaicSelected) Accessible.checked = mosaicSelected
```

**Verification.** This is checked with `qml` offscreen, using a harder
overwrite than Qt's own: a QML write, which also removes the binding. After
the overwrite, both values come back, `checked` keeps following the
selection, and a click changes nothing.

## 5. Styling is out of scope, deliberately

The toolkit marks the current option with a separate part
(`segmented-option-selected`, `tabs-tab-active`, `list-group-item-selected`) and
an `If`/`Else` inside `For`. It would be natural for `selected` to drive a
`state selected` style instead (mosstyle already knows the `selected` state, and
UI57 §4.5 binds it for bool slots).

That is not done here. A primitive prop driving a style state is a new rule for
every backend's state lowering, separate from accessibility, and nothing in this
issue needs it. Components keep their two parts and set `selected` on both
branches (`true` on the selected part, `false` on the other). A follow-up can
collapse the parts once the style rule exists.

## 6. Arrow-key traversal within a group: deferred, with reasons

The issue asks for arrow-key traversal within a group (the WAI-ARIA toolbar and
radio-group pattern: one Tab stop for the group, arrows move within it), or an
explicit decision that it is out of scope. **It is out of scope for this spec**,
and tracked in [#15457](https://github.com/adhithyan15/coding-adventures/issues/15457):

- **It needs a group node.** A focus scope belongs to a container, so it is a
  new primitive or container prop, not a `HostButton` prop. This spec adds no
  primitive; UI29 §2.4 requires its own spec for that.
- **The platforms differ in kind, not only in syntax.**
  - The web has nothing by default: it needs a roving `tabindex` and a key
    handler.
  - WinUI has `XYFocusKeyboardNavigation`.
  - Qt has `KeyNavigation` / `ButtonGroup`.
  - Compose has `focusGroup` / `selectableGroup`.
  - Flutter has `FocusTraversalGroup`.
  - On macOS, SwiftUI's full keyboard access moves by Tab, so arrow roving there
    would be *non*-native.

  A spec has to decide, platform by platform, when the right lowering is to do
  nothing.
- **The existing focus specs do not reach it.** UI81 and UI82 cover Tab and
  Shift+Tab in Venture's browser pipeline only. UI29-2 §3.2 already defers a
  `RadioGroup` composition primitive, which is the same shape of problem.
- **The two halves are independent.** A selected trait is useful with Tab
  navigation, and group roving is useful without a selected trait (toolbars).
  Shipping the first does not constrain the second, provided the group spec
  keeps `selected` as the state its roles report (`aria-checked` or
  `aria-selected` once the group changes the children's role).

## 7. `HostLink`: a different state, a separate change

`Nav` marks its active link with a part only. The native notion for a link is
"current page" (`aria-current="page"` on the web), not "selected", and applying
this spec's `aria-pressed` to a link would be wrong. It is tracked in
[#15458](https://github.com/adhithyan15/coding-adventures/issues/15458) as a
`HostLink ( current : … )` prop, with the same value shapes and degradation rule
as §4.

## 8. Migration and implementation slices

1. **Kernel:** this spec, and a UI29 §2.1 paragraph pointing here.
2. **Web emitters** (React, HTML, WebComponent): lowering and tests for all three
   value shapes, including inside `For`.
3. **Native emitters** (SwiftUI, Compose, Flutter, Qt, XAML): lowering, the
   `host_button_selected_is_native` predicates, the degradation arm, and a
   cross-backend native-complete test in the pattern of
   `portable_text_accessibility_is_native_complete_on_all_backends`.
4. **Toolkit adoption.**
   - `SegmentedControl`, `Tabs` and `ListGroup` set `selected` on both
     branches of their option `If`.
   - `SegmentedControl`'s `options` becomes `list<text>`. This is a breaking
     change to its slot type, so it needs a toolkit minor version bump.
   - The option labels are the accessible names again, unless a component
     still needs a distinct name for another reason.
5. **Host clean-up.**
   - Engram's `switcher_rows` and Trestle's `switcher_views` stop writing
     ", selected" into names, and emit plain labels.
   - Their app layouts follow the new `options` type.

Slices 2 and 3 can land in either order. Slice 4 must follow both, because the
toolkit's native-complete gate would otherwise report the new prop as a
degradation on every native backend.

## 9. Acceptance criteria

- Every emitter lowers `selected` for slot, keyword and expression values,
  including inside `For`, as in §4.1.
- On native backends, any shape that is not lowered is reported as
  `accessibility.button-selected-unsupported`, through the emitter's own
  predicate.
- A cross-backend test asserts native-complete for a `HostButton` with each
  value shape on Compose, Flutter, Qt, SwiftUI and XAML. It is mutation-checked
  by removing each backend's lowering in turn.
- XAML has a click test proving that a `ToggleButton` does not change its own
  checked state (§4.2).
- A test on each backend passes a non-bool and a markup-bearing value and
  asserts that only a boolean state is emitted (§4).
- Absent `selected` emits nothing on every backend, and existing snapshot and
  needle tests for plain buttons are unchanged.
- The toolkit components in §8.4 adopt the prop, and the host name workaround
  is removed (§8.5).

## 10. Non-goals

- Changing any button's role (`tab`, `radio`, `option`), which belongs to the
  group spec (#15457).
- Group keyboard traversal (§6, #15457).
- `HostLink` current state (§7, #15458).
- Driving style state from the prop (§5).
- A pressed state that the button toggles itself. `selected` is
  application-owned; a self-toggling control is `HostCheckbox` / `HostSwitch`
  territory.

## 11. Decisions made while writing this spec

- **Name.** `selected`, not `pressed` or `checked`. The portable meaning is
  "current option of a set" (§3). `pressed` names the web announcement, and
  `checked` collides with `HostCheckbox`, whose control owns its state.
- **Absent versus false** (§3.2): only an authored prop emits state.
- **All value shapes on all backends** (§4): the real call sites are
  expressions inside `For`.
- **XAML becomes `ToggleButton`** (§4.2), with the fallback recorded.
- **Styling and group roving stay out** (§5, §6), with follow-ups filed.
