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
| XAML | a custom automation peer with the SelectionItem pattern | a WinUI `Button`'s own peer has only Invoke; see §4.2 |

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

**Except on Apple platforms.** `AccessibilityTraits` has no negative, so
`selected : false` adds no trait on SwiftUI and an unselected option is
indistinguishable from a push button there. Every other backend carries the
false state (`aria-pressed="false"`, `Accessible.checked: false`,
`semantics { selected = false }`, `Semantics(selected: false)`, and WinUI's
`IsSelected` false). This is a platform limitation, not a lowering choice.

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
  **All three are required on every backend.** The toolkit sets a literal on
  each branch of its option `If` (§8.4), but expressions are what a host
  writes: `selected : ( i == selectedIndex )` inside `For`. A backend that
  accepted only slot references would lower neither. (This is the
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
| HTML | literal: `aria-pressed="true\|false"`; dynamic: `data-mosaic&pressed="…"`, which the project runtime replaces with `aria-pressed="true\|false"` | see §4.1.1 |
| WebComponent | `aria-pressed="${…}"` | through `escapeHtmlAttribute`, like the name |
| SwiftUI | `.accessibilityAddTraits(_mosaicSelectedTraits(expr))` | added after `.accessibilityLabel`, so both apply; the typed helper keeps a `? .isSelected : []` ternary out of the result builder, and is emitted only when used |
| Compose | `this.selected = _mosaicTruthy(expr)` inside the button's `Modifier.semantics { … }`, after any `contentDescription` | `this.` because a slot named `selected` would shadow the property; the import is added only when used |
| Flutter | `selected: _mosaicTruthy(expr)` on the button's `Semantics` node, after `enabled` | a button with no authored name gets the same node, named by its visible label |
| Qt | `property bool mosaicSelected: Boolean(expr)`, pushed to `Accessible.checkable: true` / `Accessible.checked` | the attached properties only; the `Button`'s own `checkable` stays false so a click does not toggle it; see §4.2.1 |
| XAML | `<local:{Component}MosaicSelectableButton MosaicSelected="…">`: literal `True`/`False`, or `{x:Bind …, Mode=OneWay}` | see §4.2; `i == selectedIndex` in `For` binds the row's `IsSelected` |

#### 4.1.1 HTML: a condition, not a placeholder

*(Changed while implementing slice 2. The first draft said `aria-pressed="{{…}}"`,
with a reported degradation for expressions that are not data paths.)*

A `{{placeholder}}` substitutes a *value*. For `selected : ( i == selectedIndex )`
there is no path to substitute, and for `selected : slot: s` the value would be
written as-is, not as a state. So nothing dynamic becomes a placeholder.
Instead, the emitter writes the condition as `data-mosaic&pressed="…"`
(attribute-escaped, and brace-escaped against the mustache pass).
The project runtime `main.js` then:

- evaluates the condition with `evaluateCondition`, the same evaluator that
  decides `<!-- mosaic-if when="…" -->`, after loops have bound their row;
- replaces the attribute with `aria-pressed="true"` or `"false"`.

As a result:

- **Every call-site shape works.** `i == selectedIndex` inside `For` works,
  so a degradation is never needed on this backend.
- **Only a boolean reaches the page.**
- **The condition is never executed as script.** The evaluator understands
  paths, literals, `==`, `!=` and `!`, and nothing else.

Static HTML served without the runtime keeps the `data-` attribute and has no
state, like every other dynamic binding on that backend.

### 4.2 XAML: a Button with a selectable automation peer

*(Revised while implementing #15463. The first draft proposed a
`ToggleButton` whose click handler restored `IsChecked`.)*

WinUI's `Button` automation peer implements only the Invoke pattern, and no
attached property adds a selected state to it. A `ToggleButton` was rejected
for two reasons:

- **Styling.** Its default template restyles the Checked states with the
  accent brushes, overriding the authored part style, which §5 rules out.
- **Self-toggling.** It changes its own `IsChecked` on click, and a screen
  reader's Toggle action may bypass `Click` entirely.

Instead, a button with `selected:` is a generated
`{Component}MosaicSelectableButton : Button`.

- **Unchanged:** the template, styling, visual states, `Click` and Invoke
  pattern are all the same as a plain `Button`.
- **The peer:** `OnCreateAutomationPeer` returns a `ButtonAutomationPeer`
  subclass that also implements **SelectionItem**:

| UIA member | behaviour |
| --- | --- |
| `IsSelected` | the `MosaicSelected` dependency property, bound `OneWay` |
| `Select` | the same as activating the button: `Click` is raised and the host decides |
| `AddToSelection` | as `Select`, when not already selected |
| `RemoveFromSelection` | nothing: deselection is the application's decision |
| `SelectionContainer` | none (no group node yet; see §6) |

A change to `MosaicSelected` raises the peer's `IsSelected` property-changed
event, and raises `ElementSelected` when the value becomes true.

**Elevation.** `ElevationZ` carries a part's elevation as on the drag source,
because WinUI's XAML compiler fails on `Translation` plus `<X.Shadow>` written
on a custom subclass.

**Value lowering.** Values follow `If ( when: … )`:
- A literal becomes `True` or `False`.
- A bool slot, loop binding or expression becomes `{x:Bind …, Mode=OneWay}`.
- `i == selectedIndex` inside `For` binds the row view model's `IsSelected`,
  once any outer parentheses are stripped.
- `x:Bind` does not coerce, so a non-bool slot, an unknown bare name, or an
  expression `x:Bind` cannot take is an emit error, never a silent drop.

**Verified on WinUI (Windows App SDK, .NET 9).**
- **Build:** the fixture `mosaic-emit-xaml/fixtures/host-selected-button`,
  which covers every value shape plus an elevated button, compiles, and CI
  builds it.
- **Runtime:** `code/scripts/xaml-selected-button-smoke.ps1`, run through
  UI Automation against a build with props set in place of an engine, found:
  - a literal `true` reports `IsSelected` true;
  - a false slot reports false;
  - a button without `selected` has no SelectionItem pattern;
  - loop rows report their own values;
  - invoking a row moves the selection;
  - so does `SelectionItem.Select`.

### 4.3 Degradation

`ignored_native_property` gains a `("HostButton", "selected")` arm. Following
the `HostCheckbox` `indeterminate` precedent, it asks each native emitter's own
predicate (`host_button_selected_is_native(node)`), the same function the
lowering uses, so the report cannot drift from what was emitted. A backend or
value shape that is not lowered produces
`accessibility.button-selected-unsupported` and makes the build not
native-complete, in line with UI84 §3 ("recorded by the lowering itself").

Web backends are not native-complete targets, but their emitter tests must
assert all three value shapes. A value no web backend can lower (a string or
number literal, an empty expression, an unsafe binding name) is an emit
error, never a silent drop.

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
- XAML is checked by a UI Automation run proving that the selected state is
  application-owned: `IsSelected` changes only when the host changes it, and
  `Select` dispatches like a click (§4.2). This replaces the draft's
  `ToggleButton` click test.
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
- **XAML keeps `Button`**, with a SelectionItem automation peer (§4.2). The draft's `ToggleButton` was rejected.
- **Styling and group roving stay out** (§5, §6), with follow-ups filed.
