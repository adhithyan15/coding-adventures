# UI29-2 — `HostCheckbox` and `HostRadio` kernel primitives

> **Status.** Specification (draft). Follows the second cross-backend
> kernel-growth pass, after UI29-1 added `HostDialog`.
>
> **Parent.** UI29 — Primitive Kernel + Userland Component Packages
> (`code/specs/UI29-primitive-kernel.md`).
>
> **Scope.** Adds two native form-control primitives. Same pattern as
> UI29-1: every backend lowers each to its platform-native widget,
> with the accessibility role and keyboard behaviour the host
> platform ships by default.

---

## 1. Why these belong in the kernel

The first cross-backend audit of `mosaic-pkg-toolkit` (the userland
Bootstrap-shaped component library) revealed that its `Checkbox` and
`Radio` components are **literally `HostButton` underneath**:

```moslayout
// mosaic-pkg-toolkit v0.2 — Checkbox.mll (pre-UI29-2)
Row [root] {
  HostButton [box] (label: "", onTap: emit: onToggle)   // ← FAKE
  Text (content: slot: label)
}
```

That's exactly the trap UI29-1 §1 documented for `HostDialog`. Native
checkbox / radio primitives provide:

1. **Checked-state visual + interaction**. Browsers/Cocoa/Qt all
   render an actual check or filled-circle glyph; clicking the
   primitive toggles it. A `HostButton` shows neither and toggles
   nothing.
2. **Accessibility role**. Screen readers announce a real checkbox
   as **"checkbox, checked"** / **"checkbox, not checked"** and a
   real radio as **"radio button, selected"**. A `HostButton` is
   announced as **"button"**. ARIA cannot fully patch this; the
   `role="checkbox"` attribute alone misses focus-ring shape,
   activation key (Space toggles checkbox, Enter activates button),
   and group-navigation semantics.
3. **Keyboard semantics**. Space toggles a real checkbox without
   firing a button-click event. Arrow keys navigate between radios
   in the same group. Buttons don't have any of that.
4. **Group coordination (radio only)**. Native radio buttons that
   share a group identifier (DOM `name=`, XAML `GroupName`, Qt
   `ButtonGroup`, etc.) auto-deselect siblings when one is selected.
   Re-implementing this on top of buttons requires every consumer
   to wire up multi-button mutex state.

Every host platform Mosaic targets ships these primitives. Per
UI29 §2.2's three inclusion criteria:

1. ✓ Every host has a native equivalent (full table in §3).
2. ✓ No reasonable composition exists — the accessibility role +
   keyboard semantics are non-composable.
3. ✓ Semantically irreducible — screen-reader UX *requires* the
   role to be present as a checkbox / radio, not as a button.

Adding both brings the kernel to 18 primitives. UI29 §2.4 explicitly
permits slow growth via numbered amendments; this is the second such
growth after UI29-1.

---

## 2. `HostCheckbox`

### 2.1 Slot / emit surface

| moslayout prop | Kind     | Required | Meaning                                                         |
|---|---|---|---|
| `checked`      | slot ref | yes      | bool — drives the checked state                                 |
| `disabled`     | slot ref | no       | bool — when true, the checkbox is non-interactive               |
| `indeterminate`| slot ref | no       | bool — tri-state display (only honoured where the platform supports it; otherwise treated as `checked=false`) |
| `label`        | slot ref / string | no | inline label rendered alongside the box; when absent, the userland host wraps the primitive in its own layout for the label text |
| `onToggle`     | emit ref | no       | fires when the user toggles the box (mouse click or Space key); payload `checked: bool` carries the new state |

### 2.2 Children

`HostCheckbox` is a **leaf** — it has no children. Composition (label
positioning, validation hints, etc.) lives in userland wrappers.

### 2.3 Sub-parts

| Sub-part        | Targets                                              |
|---|---|
| `checkbox`      | The outer interactive element                        |
| `checkbox:checked` | While `checked: true`                             |
| `checkbox:disabled` | While `disabled: true`                            |
| `checkbox:indeterminate` | While `indeterminate: true`                  |

---

## 3. `HostRadio`

### 3.1 Slot / emit surface

| moslayout prop | Kind     | Required | Meaning                                                          |
|---|---|---|---|
| `checked`      | slot ref | yes      | bool — drives the selected state for this radio                  |
| `group`        | slot ref / string | no | identifier shared across sibling radios; backends translate to the platform's group concept (see §4). When absent, each radio is its own group of one. |
| `value`        | slot ref / string | no | opaque payload carried on `onSelect` so the host can disambiguate which radio fired (e.g. `"option-a"`) |
| `disabled`     | slot ref | no       | bool — non-interactive when true                                 |
| `label`        | slot ref / string | no | inline label                                                |
| `onSelect`     | emit ref | no       | fires when the user selects this radio; payload `value: text` (the radio's `value` slot, falling back to an emitter-assigned auto-id when absent) |

### 3.2 Group coordination (v1 simplification)

Native radio groups auto-deselect siblings. UI29-2 v1 keeps the
implementation simple: each `HostRadio` has its own `checked: bool`
slot, and the host reducer is responsible for clearing siblings when
one fires `onSelect`. Backends that have first-class group widgets
(XAML `GroupName`, DOM `name=`) still pass the group identifier down
so the platform's native mutex still works without host help.

A future UI29-2.1 amendment can add a `RadioGroup` composition
primitive that wraps `HostRadio` siblings and does mutex coordination
in the kernel.

### 3.3 Children / sub-parts

`HostRadio` is a leaf. Sub-parts mirror `HostCheckbox` but with
`radio` / `radio:checked` / `radio:disabled` / `radio:focused` names.

---

## 4. Per-backend lowering

### 4.1 React (`mosaic-emit-react`)

```tsx
<input
  type="checkbox"
  checked={checked}
  disabled={disabled}
  onChange={e => dispatch({ type: "<toggle-event>", checked: e.target.checked })}
/>
```

```tsx
<input
  type="radio"
  name={group}
  value={value}
  checked={checked}
  disabled={disabled}
  onChange={e => dispatch({ type: "<select-event>", value: value })}
/>
```

* `indeterminate` requires a `ref` + `useEffect` (DOM checkboxes can
  only be set indeterminate from JS): same pattern HostDialog uses.
* `label`, when present, is rendered in a wrapping `<label>` element
  so clicking the text toggles the box for free.

### 4.2 SwiftUI (`mosaic-emit-swiftui`)

```swift
Toggle(isOn: .constant(checked), label: { Text(label) })
    .toggleStyle(.checkbox)              // macOS only; iOS uses default switch style
    .disabled(disabled)
```

```swift
Toggle(isOn: .constant(checked), label: { Text(label) })
    .toggleStyle(.checkbox)              // v1 — see §3.2
    .disabled(disabled)
```

* SwiftUI has no first-class single `RadioButton` view; v1 lowers
  `HostRadio` to the same `Toggle` shape with a documented note.
  Real `Picker(.radioGroup)` integration is a future UI29-2.1.
* `indeterminate` is not natively supported on SwiftUI's `Toggle`;
  the emitter currently renders an unfilled box and documents the gap.

### 4.3 Qt/QML (`mosaic-emit-qt`)

```qml
import QtQuick.Controls 2.15

CheckBox {
    checked: checked
    enabled: !disabled
    text: label
    onCheckedChanged: dispatch_toggle(checked)
}
```

```qml
RadioButton {
    checked: checked
    enabled: !disabled
    text: label
    ButtonGroup.group: <group-button-group-instance>
    onCheckedChanged: if (checked) dispatch_select(value)
}
```

* Both gate the existing `QtQuick.Controls 2.15` conditional import.
* When a `group` is bound, the emitter synthesises a top-level
  `ButtonGroup { id: <group-id> }` and binds each `HostRadio`'s
  `ButtonGroup.group` to it; this gives auto-mutex for free.

### 4.4 HTML (`mosaic-emit-html`)

```html
<input type="checkbox" {{#if checked}}checked{{/if}} {{#if disabled}}disabled{{/if}}>
<input type="radio" name="{{group}}" value="{{value}}" {{#if checked}}checked{{/if}}>
```

* Static HTML: the browser handles native checkbox / radio interaction
  for free including group coordination via `name=`.
* `onToggle` / `onSelect` emits are dropped with a comment per the
  HTML backend's no-JS-runtime convention.

### 4.5 WebComponent (`mosaic-emit-webcomponent`)

Same shape as HTML but inside the shadow DOM, with `this.dispatch()`
wired to the input's `change` event via `CustomEvent` (bubbles+composed).
Group coordination still works because `name=` matches across light
DOM AND shadow DOM (per the HTML spec).

### 4.6 XAML / WinUI (`mosaic-emit-xaml`)

```xml
<CheckBox IsChecked="{Binding checked, Mode=TwoWay}"
          IsEnabled="{Binding disabled, Converter={StaticResource InvertBoolConverter}}"
          Content="{Binding label}"
          Click="OnHostCheckboxClick_N" />
```

```xml
<RadioButton IsChecked="{Binding checked, Mode=TwoWay}"
             GroupName="{Binding group}"
             IsEnabled="{Binding disabled, Converter={StaticResource InvertBoolConverter}}"
             Content="{Binding label}"
             Click="OnHostRadioClick_N" />
```

* Follows the same code-behind dispatch pattern XAML uses for
  `HostButton` and `HostDialog` — generated `.xaml.cs` file gets one
  `OnHostCheckboxClick_N` / `OnHostRadioClick_N` handler per primitive
  instance that calls `Dispatch(...)`.
* `GroupName` gives auto-mutex within the same XAML scope for free.

### 4.7 Paint-VM (`mosaic-emit-paint`)

Future. `PaintHostCheckboxInstruction` / `PaintHostRadioInstruction`
get added to the IR with the slot values inlined; the runtime layer
renders them with the platform-canonical glyph.

---

## 5. Inclusion criteria revisit (UI29 §2.2)

| Criterion | HostCheckbox | HostRadio |
|---|---|---|
| Every host has a native equivalent | ✓ (table in §4) | ✓ (table in §4) |
| No reasonable composition exists | ✓ (role + Space-key + checked glyph) | ✓ (role + arrow-nav + group mutex) |
| Semantically irreducible | ✓ (screen reader role) | ✓ (screen reader role) |

Kernel surface after UI29-2:

```
Box  Row  Column  Stack
Text  Image  Spacer  Divider  Icon
If  Else  For
HostInput  HostButton  HostTable  HostScroll  HostDialog
HostCheckbox  HostRadio
```

18 primitives total. Same growth pattern UI29 §2.4 anticipated.

---

## 6. Migration: `mosaic-pkg-toolkit` v0.2 → v0.3

The userland `Checkbox` and `Radio` (currently `HostButton`-fakes) are
rewritten:

```moslayout
// v0.3 Checkbox.mll
component Checkbox {
  slot checked   : bool ;
  slot disabled  : bool ;
  slot label     : text ;
  emit onToggle ( checked : bool ) ;
}

layout Checkbox {
  Row [root] {
    HostCheckbox [box] (
      checked  : slot: checked ,
      disabled : slot: disabled ,
      onToggle : emit: onToggle ,
    )
    Text (content: slot: label)
  }
}
```

```moslayout
// v0.3 Radio.mll
component Radio {
  slot checked   : bool ;
  slot disabled  : bool ;
  slot group     : text ;
  slot value     : text ;
  slot label     : text ;
  emit onSelect ( value : text ) ;
}

layout Radio {
  Row [root] {
    HostRadio [dot] (
      checked  : slot: checked ,
      disabled : slot: disabled ,
      group    : slot: group ,
      value    : slot: value ,
      onSelect : emit: onSelect ,
    )
    Text (content: slot: label)
  }
}
```

**Breaking changes vs v0.2:**

* `Checkbox.onClick` → `Checkbox.onToggle(checked: bool)` (carries the
  new state, not just a click signal)
* `Radio.onClick` → `Radio.onSelect(value: text)` (carries the value
  the host needs to update its selection state)
* New `checked: bool` slot is required on both (was implicit / unused
  in the fake-button v0.2)

The toolkit major-versions to 0.3.0 for these breaking signatures.
Downstream code that consumed the fake `onClick` needs to switch to
the new emit; the host reducer for radio mutex also moves out of
userland (each backend's native group widget handles it now).

---

## 7. Implementation roadmap

| ID | Work | Depends on |
|---|---|---|
| U29-2-0 | This spec | — |
| U29-2-G | `moslayout-compiler`: add `"HostCheckbox"`, `"HostRadio"` to PRIMITIVES; `mosaic-package-resolver`: same to KERNEL_PRIMITIVES | U29-2-0 |
| U29-2-K-react | `mosaic-emit-react`: lower both | U29-2-G |
| U29-2-K-swiftui | `mosaic-emit-swiftui`: lower both | U29-2-G |
| U29-2-K-qt | `mosaic-emit-qt`: lower both + ButtonGroup synthesis | U29-2-G |
| U29-2-K-html | `mosaic-emit-html`: lower both | U29-2-G |
| U29-2-K-webcomp | `mosaic-emit-webcomponent`: lower both | U29-2-G |
| U29-2-K-xaml | `mosaic-emit-xaml`: lower both + code-behind glue | U29-2-G |
| U29-2-P | `mosaic-pkg-toolkit` v0.3: rewrite Checkbox + Radio on the new primitives | U29-2-K-react + U29-2-K-swiftui + U29-2-K-qt (at minimum, to validate the v0.3 manifest cross-backend) |

All `K-*` PRs run in parallel; `P` fans in.

---

## 8. Open questions (resolved in implementation PRs)

1. **SwiftUI radio-style picker.** v1 lowers `HostRadio` to a
   styled `Toggle`. The native idiom is `Picker(.radioGroup)`, which
   is a *single* multi-selection widget — not a per-radio view. A
   future UI29-2.1 can introduce `RadioGroup` as a composition
   primitive that wraps `HostRadio` siblings and lowers to one
   `Picker` on SwiftUI / one fieldset on DOM / one `RadioButtons`
   on WinUI.
2. **`indeterminate` parity.** Native checkboxes vary in how they
   surface tri-state: DOM via JS-set `.indeterminate`, XAML via
   `IsThreeState`, Qt via `tristate: true`, SwiftUI via no native
   support. Each K-PR documents its parity; the kernel surface
   stays as defined in §2.1.
3. **Group: text vs Group: ref.** v1 keeps `group` as a free-form
   text identifier. A typed group reference would require new
   grammar; deferred.

---

## 9. What this spec does *not* do

* Doesn't add `HostSlider`, `HostProgressBar`, `HostSelect`,
  `HostTextArea`, or `HostLink`. Those are recognised as candidates
  in the same audit but get their own numbered amendments to keep
  each spec focused.
* Doesn't redesign UI29's kernel-version policy. UI29-2 is the
  second numbered amendment after UI29-1; the kernel grows from
  16 → 18.
* Doesn't deprecate v0.2 `Checkbox` / `Radio` source-compat — the
  v0.2 fake-button shape still compiles (no kernel grammar removes
  HostButton). v0.3 toolkit supersedes it; consumers update at
  their pace.
