# UI25 — moslayout-input: The Input Primitive

**Status:** Specification (draft)
**Layer:** UI / moslayout language
**Depends on:** UI13 (mosmodel), UI14 (moslayout), UI15 (mosstyle), UI24 (emit→dispatch)

---

## 1. Purpose

The moslayout primitive vocabulary defined in UI14 has eleven leaf and
container primitives: `Box`, `Row`, `Column`, `Text`, `Image`, `Spacer`,
`Scroll`, `Divider`, `Stack`, `Icon`, and `Grid`. Notably absent is any
form of **editable text entry**. There is no way to express "the user
types a value here" inside moslayout today.

This gap blocks composing FormulaBar-class components — anything that
needs the user to type a string and emit `onChange` / `onCommit` /
`onCancel` events back to the host.

UI25 adds the **`Input` primitive** to moslayout. Input is a single,
focused, scope-minimal primitive: it represents one text entry field
and nothing more. It does not include:

- An attached label (use a sibling `Text` primitive).
- A submit button (use a sibling `Box` with the appropriate click handler).
- Validation icons (decoration is the host's job).
- A dropdown or autocomplete affordance (out of scope; future primitive
  `Combobox` will handle this).

The single-responsibility rule keeps every Mosaic primitive cleanly
mappable across every backend.

---

## 2. Grammar additions

The following additions extend the moslayout grammar from UI14.

### Token additions (`moslayout.tokens`)

```
KW_INPUT           = "Input"

KW_VALUE           = "value"
KW_PLACEHOLDER     = "placeholder"
KW_READ_ONLY       = "read-only"
KW_MULTILINE       = "multiline"
KW_MAX_LENGTH      = "max-length"

KW_ON_CHANGE       = "onChange"
KW_ON_COMMIT       = "onCommit"
KW_ON_CANCEL       = "onCancel"
```

### Grammar rule additions (`moslayout.grammar`)

```
primitive ::= ... existing alternatives ...
            | input_primitive ;

input_primitive ::= KW_INPUT LBRACE { input_member } RBRACE ;

input_member ::= input_prop
               | part_label
               | connects_clause ;

input_prop ::= KW_VALUE       COLON slot_ref_or_string ";"
             | KW_PLACEHOLDER COLON STRING            ";"
             | KW_READ_ONLY   COLON slot_ref_or_bool  ";"
             | KW_MULTILINE   COLON BOOL_LIT          ";"
             | KW_MAX_LENGTH  COLON NUMBER            ";"
             ;

slot_ref_or_string ::= AT KEBAB_IDENT
                     | STRING ;

slot_ref_or_bool   ::= AT KEBAB_IDENT
                     | BOOL_LIT ;
```

The `slot_ref_or_X` helpers are reusable across other primitives that may
accept either a slot reference (`@formula`) or a literal value.

---

## 3. Input properties

| Property      | Required | Type       | Notes |
|---|---|---|---|
| `value`       | yes      | text       | The current text in the field. Almost always a slot ref so the host can drive it; literal strings are valid for read-only display. |
| `placeholder` | no       | text (literal only) | Ghost text shown when `value` is empty. Literal only because placeholder is a layout decision, not data. |
| `read-only`   | no       | bool       | Default `false`. If `true`, the field renders but rejects edits. May be a slot ref so the host can toggle it. |
| `multiline`   | no       | bool (literal only) | Default `false`. If `true`, renders as a multi-line text area instead of a single-line input. Literal-only because the choice affects which HTML/native widget is used. |
| `max-length`  | no       | number (literal only) | Maximum character count. Literal-only for the same widget-decision reason. |

### Why some properties are literal-only

`value` and `read-only` are *data* — they change at runtime in response to
host state, so they must be slot-refable. `multiline` and `max-length`
determine *which widget* is constructed (`<input>` vs. `<textarea>`, or
the native equivalent) and cannot change after the widget is mounted.
Putting them on slots would force every backend to handle widget-swap on
slot change — a complexity not justified by any real use case in this v1.

---

## 4. Native events and `connects` clauses

The Input primitive fires three native events. These are wired to mosmodel
emits with `connects` clauses (UI14 §6):

| Native event | Payload  | When it fires |
|---|---|---|
| `onChange`   | `text`   | On every keystroke / IME composition step — the new value. |
| `onCommit`   | (none)   | When the user presses Enter (single-line) or Ctrl/Cmd+Enter (multi-line), or the field loses focus from an explicit submit gesture. |
| `onCancel`   | (none)   | When the user presses Escape. |

Connects syntax (the right-hand side names a mosmodel emit):

```moslayout
Input {
  value:     @formula;
  read-only: @read-only;
  [formula-field]

  connects: onChange(value: text) -> emit onFormulaChange(formula: value);
  connects: onCommit              -> emit onCommit;
  connects: onCancel              -> emit onCancel;
}
```

### Connects validation

The moslayout compiler enforces:

1. The right-hand emit name must exist in the component's `.mil` interface.
2. The right-hand emit's parameter types must structurally match the
   left-hand event's payload.
3. The `onChange` event has a single `text` payload — the connects clause
   must declare a matching `value: text` (the parameter name is
   author-chosen; only the type is checked).
4. `onCommit` and `onCancel` are void events — the connects clause may not
   reference any payload field.

---

## 5. Parts and styling (mosstyle interaction)

An `Input` primitive exports one part — the field itself — via the
existing `[part-name]` syntax. The mosstyle stylesheet may select that
part:

```mosstyle
.formula-field {
  font-family: monospace;
  font-size: 13px;
  border: 1px solid $color-border;
  padding: 4px 8px;
  color: $color-text-primary;
  background: $color-surface;

  state focused {
    border-color: $color-accent;
    outline: none;
  }
  state disabled {
    opacity: 0.5;
    cursor: not-allowed;
    color: $color-text-secondary;
  }
}
```

### State mapping

| moslayout condition | mosstyle state |
|---|---|
| keyboard focus on the field | `focused` |
| `read-only: true`           | `disabled` |
| `value` is empty            | `:empty` (planned, mosstyle v1 may omit) |

The `read-only -> disabled` mapping is intentional: visually, both states
mean "you cannot type here." Authors get a single state to style for the
"cannot edit" appearance.

---

## 6. Validation rules (compiler errors)

The moslayout compiler must reject the following with explicit error
messages (file path, line, column included):

| # | Condition | Example message |
|---|---|---|
| 1 | `value` property missing | `Input at FormulaBar.mll:12: 'value' property is required` |
| 2 | `value` slot ref points to a non-`text` slot | `Input at line 14: slot '@count' has type 'number' but Input.value requires 'text'` |
| 3 | `read-only` slot ref points to a non-`bool` slot | `Input at line 16: slot '@count' has type 'number' but Input.read-only requires 'bool'` |
| 4 | `multiline` value is a slot ref | `Input at line 18: 'multiline' must be a literal — slot references are not allowed (changing widget type at runtime is unsupported)` |
| 5 | `max-length` value is a slot ref | `Input at line 20: 'max-length' must be a literal number` |
| 6 | A `connects` clause references an emit that does not exist in the .mil | `Input at line 22: emit 'onFormulaChanged' not found in interface 'FormulaBar' (did you mean 'onFormulaChange'?)` |
| 7 | A `connects: onChange` clause has wrong payload type | `Input at line 24: onChange payload must be of type 'text', got 'number'` |
| 8 | A void event (`onCommit`/`onCancel`) connects clause has parameters | `Input at line 26: 'onCommit' is a void event — payload parameters are not allowed` |

---

## 7. Backend mapping table

Each Mosaic backend lowers `Input` to its idiomatic widget. The behaviour
contract — fire change events on every keystroke, fire commit on Enter,
fire cancel on Escape — is identical everywhere.

| Backend | Single-line | Multi-line | onChange | onCommit | onCancel |
|---|---|---|---|---|---|
| **React (TSX)** | `<input type="text" value={value} onChange={…} onKeyDown={…} />` | `<textarea value={value} onChange={…} onKeyDown={…} />` | React `onChange` synthetic event | `onKeyDown` Enter (Ctrl/Cmd+Enter for multi-line) | `onKeyDown` Escape |
| **WebComponent (JS)** | `<input type="text">` in shadow root | `<textarea>` in shadow root | DOM `input` event | DOM `keydown` Enter | DOM `keydown` Escape |
| **HTML (static)** | `<input type="text" value="…" readonly?>` | `<textarea readonly?>…</textarea>` | n/a (static) | n/a | n/a |
| **paint-vm** | `PaintTextInput { multiline: false, … }` widget descriptor | `PaintTextInput { multiline: true, … }` widget descriptor | Resolved by Widget Runtime layer (future UI-WR1) | Resolved by Widget Runtime | Resolved by Widget Runtime |
| **Qt (future)** | `QLineEdit` | `QTextEdit` | `textChanged` signal | `returnPressed` signal | `eventFilter` for `Qt::Key_Escape` |
| **SwiftUI (future)** | `TextField` | `TextEditor` | `onChange(of:)` modifier | `onSubmit { … }` | `onKeyPress(.escape)` |
| **Jetpack Compose (future)** | `TextField` | `TextField(maxLines = N)` | `onValueChange` lambda | `KeyboardActions.onDone` | `Modifier.onKeyEvent` |
| **XAML / WinUI3 (future)** | `TextBox` | `TextBox AcceptsReturn="True"` | `TextChanged` event | `KeyDown` Enter | `KeyDown` Escape |
| **AppKit (future)** | `NSTextField` | `NSTextView` (in `NSScrollView`) | `controlTextDidChange:` | action method on `enterAction` | key-down handler for Escape |

### The `PaintTextInput` widget descriptor

The paint-vm backend cannot natively handle text input — text input
requires focus management, IME composition, caret rendering, selection
tracking, and clipboard plumbing, none of which belong in a renderer
called "paint-vm." Instead, the paint backend emits a typed widget
descriptor:

```rust
pub struct PaintTextInput {
    pub bounds:       Rect,
    pub value:        String,
    pub placeholder:  String,
    pub read_only:    bool,
    pub multiline:    bool,
    pub max_length:   Option<usize>,
    pub on_change_id: u32,    // index into the dispatch table
    pub on_commit_id: u32,
    pub on_cancel_id: u32,
    pub style:        TextInputStyle,
}
```

This descriptor is consumed by the **Widget Runtime layer** (future spec
UI-WR1), which lives one layer above paint-vm and below the application.
Until UI-WR1 ships, the paint backend may render the input as a static
rectangle with the placeholder text — visible but not interactive. This
is acceptable because the paint backend's primary use case today is
preview thumbnails (UI22), not live UIs.

The `PaintTextInput` struct shape is the agreed contract between this
spec and UI-WR1; UI-WR1 will document how each field is consumed.

---

## 8. React generated output

For the FormulaBar layout in §10 below, the React emitter (with the
UI24 dispatch pattern) produces:

```tsx
// Auto-generated by mosaic-emit-react. Do not edit.
import React from "react";

type FormulaBarEvent =
  | { type: "formulaChange"; formula: string }
  | { type: "commit" }
  | { type: "cancel" };

interface FormulaBarProps {
  cellAddress: string;
  formula:     string;
  readOnly:    boolean;
  dispatch:    (event: FormulaBarEvent) => void;
}

export function FormulaBar({
  cellAddress,
  formula,
  readOnly,
  dispatch,
}: FormulaBarProps) {
  return (
    <div style={{ display: "flex", flexDirection: "row" }}>
      <span>{cellAddress}</span>
      <input
        type="text"
        value={formula}
        readOnly={readOnly}
        onChange={e => dispatch({ type: "formulaChange", formula: e.target.value })}
        onKeyDown={e => {
          if (e.key === "Enter")  dispatch({ type: "commit" });
          if (e.key === "Escape") dispatch({ type: "cancel" });
        }}
      />
    </div>
  );
}
```

Notes:

- The `<input>` is **controlled** — `value` always comes from the prop,
  and changes propagate up via `dispatch` rather than living in component
  state. This is the React idiom that matches the `useReducer` host model.
- `readOnly` is the camelCase form of the kebab-case `read-only` slot,
  per UI20 §3.2.
- The `onKeyDown` handler emits `commit` on Enter and `cancel` on Escape.
  Multi-line input would gate `commit` on `(e.ctrlKey || e.metaKey)`.

---

## 9. Worked example — FormulaBar

The complete three-file FormulaBar component:

### FormulaBar.mil

```mosmodel
component FormulaBar version 1.0 {
  slot cell-address : text ;
  slot formula      : text ;
  slot read-only    : bool = false ;

  emit onFormulaChange ( formula : text ) ;
  emit onCommit ;
  emit onCancel ;
}
```

### FormulaBar.desktop.mll

```moslayout
layout FormulaBar.desktop version 1.0 implements FormulaBar 1.x {
  Row {
    Text {
      content: @cell-address;
      [address-label]
    }
    Input {
      value:     @formula;
      read-only: @read-only;
      [formula-field]

      connects: onChange(value: text) -> emit onFormulaChange(formula: value);
      connects: onCommit              -> emit onCommit;
      connects: onCancel              -> emit onCancel;
    }
  }
}
```

### FormulaBar.dark.msl

```mosstyle
style FormulaBar.dark version 1.0 for FormulaBar 1.x {
  .address-label {
    font-family: monospace;
    font-size:   12px;
    color:       $color-text-secondary;
    min-width:   48px;
    padding:     4px 8px;
  }

  .formula-field {
    font-family:    monospace;
    font-size:      13px;
    flex:           1;
    border:         none;
    border-bottom:  1px solid $color-border;
    background:     transparent;
    color:          $color-text-primary;
    padding:        4px 8px;

    state focused {
      border-bottom-color: $color-accent;
    }
    state disabled {
      color: $color-text-secondary;
    }
  }
}
```

---

## 10. Test additions for `mosaic-emit-react`

The React backend's implementation of Input must be covered by:

| Test | What it covers |
|---|---|
| `test_input_emits_input_tag` | An `Input` primitive in moslayout lowers to a `<input type="text" …>` element in JSX. |
| `test_input_multiline_emits_textarea` | `multiline: true` lowers to `<textarea>` instead. |
| `test_input_value_bound_to_prop` | The `value={…}` attribute uses the camelCase slot ref name. |
| `test_input_readonly_attribute` | `read-only` slot lowers to a `readOnly={…}` attribute (note casing). |
| `test_input_onchange_dispatch_call` | The `onChange` handler calls `dispatch({ type: "...", … })` with the connected mosmodel emit name. |
| `test_input_commit_on_enter` | The `onKeyDown` handler dispatches the commit event on `e.key === "Enter"`. |
| `test_input_cancel_on_escape` | The `onKeyDown` handler dispatches the cancel event on `e.key === "Escape"`. |
| `test_input_placeholder_attribute` | The `placeholder="..."` attribute appears verbatim. |
| `test_input_max_length_attribute` | `max-length: 100` lowers to `maxLength={100}`. |
| `test_input_missing_value_errors` | A moslayout `Input` with no `value` property causes a compile error. |
| `test_input_value_wrong_type_errors` | `value: @number-slot` where the slot is `number` errors. |
| `test_input_connect_to_unknown_emit_errors` | `connects: onChange(...) -> emit onFoo(...)` where `onFoo` does not exist errors. |
| `test_input_connect_void_event_with_params_errors` | `connects: onCommit(x: text) -> ...` errors (onCommit is void). |
| `test_input_renders_inside_row` | An `Input` nested under a `Row` produces the expected JSX nesting. |

Test additions for the moslayout compiler should mirror §6's validation
table, one test per error condition.

---

## 11. Out of scope

The following are deferred and **not** part of UI25:

- **Combobox / autocomplete / dropdown** — future primitive.
- **Password fields** (`<input type="password">`) — future primitive
  `PasswordInput` so the security difference is loud at the source level.
- **Number / date / file inputs** — future typed-input primitives.
- **Rich-text editing** — never part of moslayout; belongs in a separate
  document-editor toolkit.
- **IME composition events** — backends must handle IME correctly when
  emitting `onChange`, but the moslayout source does not expose IME state.
- **Placeholder colour customisation in mosstyle v1** — most platforms use
  a vendor-specific pseudo-element (`::placeholder` in CSS,
  `placeholderTextColor` in SwiftUI, etc.). Mosstyle v2 will add a
  `placeholder` state to abstract over these.
- **Widget Runtime resolution** of `PaintTextInput` — covered by future
  UI-WR1.
- **Drag-and-drop into inputs** — future spec.

---

## 12. Relationship to other specs

- **UI13 mosmodel** — declares the emits that this primitive's connects
  clauses reference.
- **UI14 moslayout** — primitive vocabulary. UI25 adds `Input` to that
  vocabulary; everything else in UI14 (Box/Row/Column/Text/Image/Spacer/
  Scroll/Divider/Stack/Icon/Grid, `connects` syntax, part labels) is
  unchanged.
- **UI15 mosstyle** — part styling and state. UI25 maps `read-only` to
  the existing `disabled` state and adds a single new exported part name.
- **UI20 mosaic-emit-react** — JSX lowering rules; UI25 adds one entry to
  the primitive → JSX mapping table.
- **UI24 mosaic-emit-dispatch** — defines the `dispatch(event: …)` prop
  shape that this primitive's React lowering uses for change/commit/cancel
  events.
- **UI23 mosaic-pipeline** — adding `Input` to moslayout is a non-breaking
  *layout* change. Any layout using `Input` should be at minor version
  `1.x` of its layout name. Components using such layouts do not require
  any version bump.
