# UI25 — moslayout-input: Input Primitive

**Status:** Specification  
**Layer:** UI  
**Depends on:** UI13 (mosmodel), UI14 (moslayout), UI15 (mosstyle), UI20 (mosaic-emit-react)

---

## Overview

This spec adds the `Input` primitive to the moslayout language (UI14). `Input`
is a text-entry field: the most fundamental interactive primitive missing from
the original moslayout vocabulary. Its primary motivating consumer is the
`FormulaBar` component — the editable formula bar in the spreadsheet workbook —
but it is general-purpose and may appear in any component that needs text input
from the user.

This spec covers:

- The grammar addition to `moslayout.tokens` and `moslayout.grammar`
- The `connects` clause extensions for Input's three native events
- Backend mapping for every current and planned target
- The mosstyle parts and states Input exports
- Compiler validation rules with exact error messages
- A complete three-file worked example (FormulaBar)
- Exact React TSX output after the UI20 dispatcher pattern
- The relationship between Input and the future Widget Runtime layer

---

## §1 Purpose

### §1.1 Why Input is needed

Every component in the existing vocabulary is *read-only from the user's
perspective*: `Text` displays a string, `Image` displays a picture, `Grid`
shows tabular data. The host pushes values in; the user sees them. None of these
primitives accept text from the user.

`Input` fills this gap. It is the primitive through which a user types
characters, sees them appear, and fires events when the value changes or when
they commit or cancel an edit. Without `Input`, an entire class of components —
formula bars, search boxes, rename dialogs, settings fields — cannot be expressed
in moslayout at all and must be hand-coded per backend.

### §1.2 What Input represents

`Input` represents exactly one thing: a rectangular, focusable, text-entry
region. It has a current value (a string), an optional placeholder (ghost text
shown when the value is empty), an optional read-only flag (renders the region
as non-editable), and an optional multiline flag (renders as a textarea rather
than a single-line input).

`Input` is NOT:

- A labeled form field — if a label is needed, put a `Text` primitive beside
  the `Input` in a `Row`. The layout is responsible for arrangement.
- A validated field — validation belongs in the host application. The mosmodel
  interface declares emits; the host decides whether to accept or reject a new
  value.
- A search box with a clear button — compose `Input` + `Box` + `Icon` in the
  layout instead. `Input` has no built-in decorations.

### §1.3 Single-responsibility rule

`Input` renders one thing: the text entry surface. Background color, border,
padding, cursor color, placeholder color — all of these are styling concerns
that belong in `.msl`. Whether the field sits next to a label or stands alone
is a layout concern that belongs in `.mll`. The `Input` primitive in `.mll`
answers only: "there is a text entry field here, it shows this value, it is
or is not editable, it is or is not multiline."

---

## §2 Input Primitive Declaration Syntax

### §2.1 Overview

An `Input` block appears anywhere a node is valid inside a `.mll` file. It
declares property bindings (value, placeholder, read-only, multiline,
max-length), a part name for styling, and zero or more `connects` clauses that
wire the primitive's native events to mosmodel emits.

```
Input {
  value:       @formula;              // slot ref — required
  placeholder: "Enter formula";       // literal string — optional
  read-only:   @read-only;            // slot ref or literal — optional
  multiline:   false;                 // literal bool — optional
  max-length:  4096;                  // literal number — optional
  [formula-field]                     // part name — optional but recommended
  connects: onChange(value: text) -> emit onFormulaChange(formula: value);
  connects: onCommit -> emit onCommit;
  connects: onCancel -> emit onCancel;
}
```

A slot reference is written `@slot-name`. A literal string is written in double
quotes. A literal bool is `true` or `false`. A literal number is an integer or
decimal (no units).

### §2.2 Grammar additions

The following tokens are added to `moslayout.tokens`:

```
# New keyword for Input primitive
KW_INPUT        = "Input"

# New property keywords for Input
KW_VALUE        = "value"
KW_PLACEHOLDER  = "placeholder"
KW_READ_ONLY    = "read-only"
KW_MULTILINE    = "multiline"
KW_MAX_LENGTH   = "max-length"

# New keywords for connects clauses
KW_CONNECTS     = "connects"       # already exists on Box — promote to shared keyword
KW_ON_CHANGE    = "onChange"
KW_ON_COMMIT    = "onCommit"
KW_ON_CANCEL    = "onCancel"
KW_EMIT_KW      = "emit"          # already a keyword in scope

# Slot reference prefix
AT              = "@"

# Arrow in connects clause
ARROW           = "->"
```

The following grammar rules are added to `moslayout.grammar`:

```
# Add input_node to the node alternation
node = box_node | row_node | column_node | text_node | image_node
     | spacer_node | scroll_node | grid_node | input_node | component_node ;

# Input primitive
input_node = KW_INPUT LBRACE { input_item } RBRACE ;

input_item = input_prop | part_name | connects_clause ;

input_prop =
    KW_VALUE        COLON slot_ref_or_literal SEMICOLON
  | KW_PLACEHOLDER  COLON STRING              SEMICOLON
  | KW_READ_ONLY    COLON slot_ref_or_literal SEMICOLON
  | KW_MULTILINE    COLON bool_lit            SEMICOLON
  | KW_MAX_LENGTH   COLON NUMBER              SEMICOLON
  ;

# A slot reference (@slot-name) or a literal value
slot_ref_or_literal =
    AT KEBAB_IDENT   # @slot-name — resolved against the interface descriptor
  | STRING           # "literal text"
  | bool_lit         # true | false (for read-only)
  | NUMBER           # numeric literal (for max-length)
  ;

bool_lit = KW_TRUE | KW_FALSE ;

# Connects clause — wires an Input native event to a mosmodel emit
connects_clause =
    KW_CONNECTS COLON payload_event ARROW KW_EMIT emit_target SEMICOLON
  | KW_CONNECTS COLON void_event    ARROW KW_EMIT void_emit   SEMICOLON
  ;

# onChange with a payload (value: text)
payload_event  = KEBAB_IDENT LPAREN KEBAB_IDENT COLON "text" RPAREN ;
# e.g.  onChange(value: text)

# onCommit / onCancel — no payload
void_event = KEBAB_IDENT ;
# e.g.  onCommit

# Emit target with payload remapping
emit_target = KEBAB_IDENT LPAREN KEBAB_IDENT COLON KEBAB_IDENT RPAREN ;
# e.g.  onFormulaChange(formula: value)
#       emit-name ( param-name : param-value )

# Void emit with no payload
void_emit = KEBAB_IDENT ;
# e.g.  onCommit
```

### §2.3 Property reference

| Property | Required? | Type | Slot ref? | Literal? | Default |
|---|---|---|---|---|---|
| `value` | Yes | `text` | Yes | Yes | — (required) |
| `placeholder` | No | string | No | Yes | `""` |
| `read-only` | No | `bool` | Yes | Yes | `false` |
| `multiline` | No | bool literal | No | Yes | `false` |
| `max-length` | No | number literal | No | Yes | unlimited |

**`value`** is required and almost always a slot reference — it binds the
displayed text to data the host controls. It may be a literal string for a
static non-editable field, but the `read-only` property should be `true` in
that case.

**`placeholder`** is literal-only. It cannot be a slot reference because it is
a compile-time authoring decision ("what should the ghost text say?"), not a
runtime data binding. The placeholder is part of the component's designed
affordance, not a dynamic value.

**`read-only`** may be a slot reference (typically a `bool` slot the host
controls) or a literal. When it is `true`, backends render the field in a
non-editable state: no caret, no keyboard input accepted, no change events
fired. The mosstyle `disabled` state is activated for styling purposes.

**`multiline`** must be a literal — `true` or `false`. It is a layout
decision: the author decides at design time whether this field is single-line or
a textarea. It cannot be controlled by a slot because switching between
single-line and multiline would require a layout change, not merely a data
change.

**`max-length`** must be a literal number. The backend enforces this limit
natively (HTML `maxlength` attribute, Qt `maxLength`, etc.). It cannot be a
slot reference for the same reason as `multiline` — it is a design-time
structural constraint, not a runtime data value.

---

## §3 Connects Clauses for Input

### §3.1 The three native events

`Input` generates exactly three events from the native widget or DOM element
it renders to. These events must be explicitly wired to mosmodel emits via
`connects` clauses; unconnected events are silently discarded (they do not
reach the host).

| Event | When it fires | Payload |
|---|---|---|
| `onChange` | On every character insertion, deletion, or paste | The current full text value after the change |
| `onCommit` | On Enter key press (or equivalent "submit" gesture per platform) | None |
| `onCancel` | On Escape key press (or equivalent "cancel" gesture per platform) | None |

### §3.2 Connects syntax

```
// Wire onChange — the event carries the new text value
connects: onChange(value: text) -> emit onFormulaChange(formula: value);

// Wire onCommit — no payload
connects: onCommit -> emit onCommit;

// Wire onCancel — no payload
connects: onCancel -> emit onCancel;
```

Reading the first line: "when the Input fires its native `onChange` event, and
that event carries a parameter called `value` of type `text`, dispatch the
mosmodel emit `onFormulaChange` with its `formula` parameter set to the value of
`value`." The name `value` on the left is the local binding name for the event's
payload; the name after the `:` on the right side is the mosmodel emit's
parameter name that receives it.

### §3.3 Compiler validation for connects

The compiler validates every `connects` clause:

1. The native event name (`onChange`, `onCommit`, `onCancel`) must be one of the
   three defined Input events. Any other name is a compile error.
2. The emit name on the right side must exist in the mosmodel interface
   descriptor. The compiler checks the interface descriptor imported at the start
   of compilation.
3. If the emit has a payload, the `connects` clause must be in the
   `payload_event` form. If the emit is void, the `connects` clause must be in
   the `void_event` form. Type mismatch between the clause form and the emit
   declaration is a compile error.
4. The payload type of `onChange` is always `text`. A `connects: onChange`
   clause may only wire to a mosmodel emit whose parameter is declared with type
   `text`.

### §3.4 Multiple connects on the same Input

An `Input` block may have zero, one, two, or all three `connects` clauses.
Each of the three native events may appear at most once. Declaring `connects:
onChange` twice on the same `Input` is a compile error (`DuplicateConnects`).

An `Input` without any `connects` clauses is valid — it produces a field whose
value changes are silently discarded. This is useful for static display fields
where `read-only: true` is set and no edit events are expected, or for
prototype layouts where event wiring is stubbed out.

---

## §4 Backend Mapping Table

The table below shows how `Input` maps to each backend. "Future" backends are
included for planning purposes; their implementations are deferred.

| Backend | Rendering element | onChange | onCommit | onCancel |
|---|---|---|---|---|
| **React** | `<input type="text">` (single-line) or `<textarea>` (multiline) | `onChange={e => dispatch({type: …, param: e.target.value})}` | `onKeyDown`: `if (e.key === "Enter") dispatch({type: …})` | `onKeyDown`: `if (e.key === "Escape") dispatch({type: …})` |
| **Web Component** | `<input>` or `<textarea>` in shadow DOM | `addEventListener("input", …)` | `addEventListener("keydown", …)` filter `Enter` | `addEventListener("keydown", …)` filter `Escape` |
| **HTML (static)** | `<input type="text" value="…" readonly>` (no events) | n/a — static render | n/a | n/a |
| **paint-vm** | `TextInput` widget descriptor in PaintScene (see §9) | Widget Runtime callback (future) | Widget Runtime callback (future) | Widget Runtime callback (future) |
| **Qt (future)** | `QLineEdit` (single-line) or `QTextEdit` (multiline) | `textChanged(const QString &)` signal | `returnPressed()` signal | Custom `keyPressEvent` filter for `Qt::Key_Escape` |
| **SwiftUI (future)** | `TextField` (single-line) or `TextEditor` (multiline) | `onChange(of:)` modifier on binding | `onSubmit { }` modifier | Custom `onKeyPress` handler for `.escape` |
| **AppKit (future)** | `NSTextField` (single-line) or `NSTextView` (multiline) | `controlTextDidChange` delegate | `control(_:textView:doCommandBy:)` for `insertNewline` | `control(_:textView:doCommandBy:)` for `cancelOperation` |
| **Android Compose (future)** | `TextField` composable | `onValueChange` lambda | `KeyboardActions(onDone = { … })` | Custom `onKeyEvent` for `Key.Escape` |
| **XAML (future)** | `TextBox` | `TextChanged` event | `KeyDown` with `Key.Enter` | `KeyDown` with `Key.Escape` |

### §4.1 React — single-line vs multiline

When `multiline: false` (the default), the React backend emits `<input
type="text">`. When `multiline: true`, it emits `<textarea>`. The `onChange`
handler differs slightly: for `<input>`, `e.target.value` is the new value; for
`<textarea>`, the same API applies (`e.target.value`) — React normalises these.

### §4.2 HTML static backend

The static HTML backend renders `Input` as `<input type="text" value="…">` for
single-line or `<textarea>…</textarea>` for multiline. No event handlers are
emitted. If `read-only` is `true` or the value is a literal, the `readonly`
attribute is added. The static backend is used for server-rendered snapshots and
documentation previews; interactivity is out of scope.

### §4.3 paint-vm backend

See §9 for the full discussion. The paint backend emits a `TextInput` widget
descriptor into the PaintScene output. The Widget Runtime layer (future spec
UI-WR1) resolves this descriptor into an interactive region with focus
management, caret rendering, and keyboard event routing.

---

## §5 Mosstyle Parts for Input

### §5.1 Exported parts

`Input` exports exactly one part: whichever name the author assigns in the
`[part-name]` declaration inside the block. If no part name is declared, the
node is anonymous and cannot be individually styled. By convention the part name
describes the field's role in the component, e.g. `formula-field`,
`search-field`, `name-field`.

The part map entry emitted by the moslayout compiler for an `Input` node:

```json
{
  "name": "formula-field",
  "primitive": "Input",
  "multiline": false
}
```

The `primitive: "Input"` value tells the mosstyle compiler that this part
accepts the full set of Input-specific style properties (see §5.2).

### §5.2 Style properties valid on Input parts

`Input` parts accept all standard mosstyle properties that are valid on any
rectangular region, plus the typography properties (because the text inside
an input is styled the same way as Text parts).

| Property category | Properties valid on Input |
|---|---|
| Color | `background`, `color`, `border-color`, `outline-color`, `shadow-color` |
| Geometry | `border-radius`, `border-width`, `outline-width`, `padding`, `padding-*`, `shadow-radius`, `shadow-offset-*` |
| Typography | `font-family`, `font-size`, `font-weight`, `line-height`, `letter-spacing` |
| Visibility | `opacity` |

Typography properties are valid on `Input` parts (unlike general `Box` parts)
because the rendered text inside the input field is styled by these properties.
The mosstyle compiler allows font properties on both `Text` and `Input` parts.

### §5.3 Input-specific states

`Input` parts respond to the full built-in state set. The most important:

| State | When active |
|---|---|
| `focused` | The input has keyboard focus (caret is visible) |
| `disabled` | `read-only` is `true` — styled as non-editable |
| `error` | Host sets an error state slot (optional convention) |

The mapping of `read-only: true` → mosstyle `disabled` state is a design
decision: read-only fields should visually communicate non-editability using the
same styling system as other disabled controls. Component authors who want a
distinct look for read-only vs disabled can use mosstyle token overrides.

### §5.4 Example mosstyle block for an Input part

```mosstyle
.formula-field {
  font-family:  monospace;
  font-size:    13px;
  color:        $color-text-primary;
  background:   $color-surface;
  border-width: 1px;
  border-color: $color-border;
  padding:      4px 8px;

  transition border-color $duration-fast $easing-out;
  transition outline-width $duration-fast $easing-out;

  state focused {
    border-color:  $color-accent;
    outline-color: $color-accent;
    outline-width: 2px;
  }

  state disabled {
    opacity: $opacity-disabled;
    color:   $color-text-muted;
  }

  state error {
    border-color: $color-danger;
  }
}
```

The `placeholder` text color is not directly controlled by mosstyle in v1 —
backends use their platform defaults for placeholder ghost text. A
`placeholder-color` property may be added in a future mosstyle revision.

---

## §6 Validation Rules

### §6.1 Compile-time checks by the moslayout compiler

The moslayout compiler validates every `Input` block after parsing and before
emitting layout IR:

| Rule | Check | Error kind |
|---|---|---|
| R1 | `value` property is present | `MissingRequired` |
| R2 | If `value` is a slot ref, the slot must exist in the interface descriptor | `UnknownSlot` |
| R3 | If `value` is a slot ref, the slot's declared type must be `text` | `SlotTypeMismatch` |
| R4 | If `read-only` is a slot ref, the slot's declared type must be `bool` | `SlotTypeMismatch` |
| R5 | `multiline` must be a literal (`true` or `false`), not a slot ref | `SlotRefNotAllowed` |
| R6 | `max-length` must be a literal number, not a slot ref | `SlotRefNotAllowed` |
| R7 | Each `connects` clause's emit name must exist in the interface descriptor | `UnknownEmit` |
| R8 | `onChange` connects clause must target a `text`-typed emit parameter | `EmitTypeMismatch` |
| R9 | The same native event (`onChange`, `onCommit`, `onCancel`) may appear at most once | `DuplicateConnects` |
| R10 | Part names must be unique within the layout (inherited from existing rule) | `DuplicatePart` |

### §6.2 Error messages

The moslayout compiler produces human-readable error messages with file, line,
and column information:

```
// R1
Input at line 12: 'value' property is required

// R2
Input at line 14: slot ref '@formula-text' not found in interface
  Component 'FormulaBar' declares slots: cell-address, formula, read-only

// R3
Input at line 14: slot '@total-rows' has type 'number' but Input.value requires type 'text'

// R4
Input at line 16: slot '@count' has type 'number' but Input.read-only requires type 'bool'

// R5
Input at line 18: 'multiline' must be a literal (true or false), not a slot reference
  Use a platform layout variant (.desktop.mll, .mobile.mll) to select multiline mode per platform.

// R6
Input at line 19: 'max-length' must be a literal number, not a slot reference

// R7
Input at line 21: emit 'onFormulaChanged' not found in interface
  Did you mean 'onFormulaChange'?
  Component 'FormulaBar' declares emits: onFormulaChange, onCommit, onCancel

// R8
Input at line 21: connects onChange wires to emit 'onFormulaChange(formula: number)'
  but onChange payload is type 'text' — emit parameter 'formula' must be type 'text'

// R9
Input at line 23: 'onChange' already has a connects clause at line 21
  Each native event may be wired at most once per Input block.
```

### §6.3 Did-you-mean for emit names

The compiler computes Levenshtein distance between the unknown emit name and
every emit declared in the interface descriptor. If the closest match has
distance ≤ 3, the error message includes a "Did you mean '…'?" suggestion. This
follows the pattern established by the existing `UnknownEmit` error on Box.

---

## §7 Complete Worked Example — FormulaBar

This section shows the complete three-file source for the `FormulaBar`
component: the mosmodel interface, the moslayout file, and a mosstyle dark theme.
`FormulaBar` displays the address of the currently selected cell and an
editable formula bar that the user can type into, commit with Enter, or cancel
with Escape.

### §7.1 FormulaBar.mil — mosmodel interface

```
component FormulaBar {
  // The address of the active cell, e.g. "A1", "B12", "AZ999".
  // The host updates this whenever the selection changes.
  slot cell-address : text ;

  // The raw formula or value string currently in the formula bar.
  // The host updates this on navigation (to show the new cell's content)
  // and on accepted edits (to show the committed value).
  slot formula : text ;

  // When true, the formula bar renders as non-editable (e.g. during
  // a protected-sheet state or modal operation).
  slot read-only : bool = false ;

  // Fired on every keystroke while the user is editing.
  // The host uses this to update a pending-edit state (live preview, etc.)
  // but does NOT commit the value yet.
  emit onFormulaChange ( formula : text ) ;

  // Fired when the user presses Enter or otherwise accepts the edit.
  // The host commits the formula and updates the cell.
  emit onCommit ;

  // Fired when the user presses Escape or otherwise cancels the edit.
  // The host discards the pending edit and restores the original value.
  emit onCancel ;
}
```

### §7.2 FormulaBar.mll — moslayout file

```
layout FormulaBar.desktop version 1.0 implements FormulaBar 1.x {
  //
  // The FormulaBar is a horizontal row with two parts:
  //   [address-label]  — a non-editable display of the active cell address
  //   [formula-field]  — an editable text input for the formula
  //
  // The row uses default flex-start alignment; the formula field
  // will be made to grow via mosstyle (flex: 1) or a Spacer alternative.
  //
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
      connects: onCommit -> emit onCommit;
      connects: onCancel -> emit onCancel;
    }
  }
}
```

Note: `placeholder` is omitted here because a formula bar traditionally shows
either the cell's existing value or an empty field — ghost text is not
conventional for this UI pattern. A search-box variant would include
`placeholder: "Search…"`.

### §7.3 FormulaBar.msl — mosstyle dark theme

```
style FormulaBar.dark version 1.0 for FormulaBar 1.x {

  // The cell address display on the left.
  // Fixed minimum width so the formula field doesn't collapse it.
  part address-label {
    font-family: monospace;
    font-size:   12px;
    color:       $color-text-secondary;
    padding:     4px 8px;
    min-width:   48px;
  }

  // The editable formula input field.
  // flex: 1 is expressed via mosstyle's grow property once it is added
  // to the spec; in v1 the host must size the field via a wrapper Box.
  part formula-field {
    font-family:  monospace;
    font-size:    13px;
    color:        $color-text-primary;
    background:   transparent;
    border-width: 0;
    border-bottom: 1px solid $color-border;
    padding:      4px 8px;

    transition border-bottom-color $duration-fast $easing-out;

    state focused {
      border-bottom-color: $color-accent;
      outline-width:       0;          // suppress system default focus ring
    }

    state disabled {
      color:   $color-text-secondary;
      opacity: $opacity-disabled;
    }
  }

}
```

---

## §8 React Generated Output

This section shows the exact `.tsx` file that `mosaic-emit-react` produces for
`FormulaBar` after the UI24 dispatch-union pattern. In this pattern, the
component receives a single `dispatch` prop typed to a discriminated union of
all possible event types, replacing the per-event callback props used in earlier
React emit patterns.

```tsx
// Auto-generated by mosaic-emit-react. Do not edit.
import React from "react";

// The discriminated event union. Every event the component can fire is a
// member of this type. The host switches on `event.type` to handle events.
//
//   { type: "formulaChange", formula: string }  — onChange payload
//   { type: "commit" }                           — onCommit (void)
//   { type: "cancel" }                           — onCancel (void)
//
type FormulaBarEvent =
  | { type: "formulaChange"; formula: string }
  | { type: "commit" }
  | { type: "cancel" };

// Props interface. Each mosmodel slot becomes a typed prop.
// Required slots have no `?`; optional slots (with defaults) get `?`.
interface FormulaBarProps {
  cellAddress: string;       // slot cell-address : text
  formula:     string;       // slot formula      : text
  readOnly?:   boolean;      // slot read-only    : bool = false
  dispatch:    (event: FormulaBarEvent) => void;
}

export function FormulaBar({
  cellAddress,
  formula,
  readOnly = false,
  dispatch,
}: FormulaBarProps) {
  return (
    // Row → <div style={{ display: "flex", flexDirection: "row" }}>
    <div style={{ display: "flex", flexDirection: "row" }}>

      {/* Text [address-label] — non-editable cell address display */}
      <span>{cellAddress}</span>

      {/* Input [formula-field] — editable formula bar */}
      {/*
       * multiline: false (default) → <input type="text">
       * value bound to the `formula` prop (controlled component pattern)
       * readOnly bound to `readOnly` prop
       * onChange fires the formulaChange event with e.target.value
       * onKeyDown intercepts Enter (commit) and Escape (cancel)
       */}
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

### §8.1 Multiline variant

If `FormulaBar.mll` had declared `multiline: true` on the Input, the React
backend would emit `<textarea>` instead of `<input type="text">`. The event
handling remains identical because `textarea` exposes the same
`e.target.value` and `onKeyDown` React API:

```tsx
<textarea
  value={formula}
  readOnly={readOnly}
  onChange={e => dispatch({ type: "formulaChange", formula: e.target.value })}
  onKeyDown={e => {
    if (e.key === "Enter")  dispatch({ type: "commit" });
    if (e.key === "Escape") dispatch({ type: "cancel" });
  }}
/>
```

Note that for `<textarea>`, Enter normally inserts a newline rather than
submitting — a multiline input field typically uses Ctrl+Enter for commit and
raw Escape for cancel. The exact key mapping for multiline fields is left to
the component author via the `connects` clause on the mosmodel emit. The React
backend honors the mapping as written; the `onCommit` → Enter binding in the
example above is the single-line convention.

### §8.2 Placeholder in generated output

When `placeholder: "Enter formula"` is declared, the React backend adds the
`placeholder` attribute:

```tsx
<input
  type="text"
  value={formula}
  placeholder="Enter formula"
  readOnly={readOnly}
  onChange={e => dispatch({ type: "formulaChange", formula: e.target.value })}
  onKeyDown={e => {
    if (e.key === "Enter")  dispatch({ type: "commit" });
    if (e.key === "Escape") dispatch({ type: "cancel" });
  }}
/>
```

The placeholder value is HTML-escaped before embedding: `<`, `>`, `"`, `&` are
replaced with their entity references.

### §8.3 max-length in generated output

When `max-length: 4096` is declared, the React backend adds the `maxLength`
attribute (camelCase, per React conventions):

```tsx
<input
  type="text"
  value={formula}
  maxLength={4096}
  onChange={e => dispatch({ type: "formulaChange", formula: e.target.value })}
  onKeyDown={…}
/>
```

### §8.4 Controlled component invariant

The React backend always emits Input as a **controlled component**: `value` is
always bound to a prop, never left uncontrolled. This is the correct pattern
for Mosaic because the host owns the state and pushes slot values in. An
uncontrolled input (`defaultValue` with no `value` binding) would allow the DOM
to diverge from the mosmodel state, which breaks the one-directional data flow
the Mosaic system enforces at the interface level.

---

## §9 Relationship to Widget Runtime — paint-vm Backend

### §9.1 The gap

The paint-vm backend renders components as a tree of `PaintScene` instructions:
`PaintRect`, `PaintGlyphRun`, `PaintImage`, and so on. These instructions are
consumed by a renderer (Metal on macOS, Cairo/Skia on Linux/Windows) that
produces pixels. This pipeline is fully stateless and purely visual — there is
no concept of keyboard focus, text cursor position, or character insertion in
the paint instruction set.

`Input` is inherently stateful and interactive. It requires:

1. **Focus management** — knowing which field (if any) has keyboard focus
2. **Caret rendering** — drawing a blinking insertion point at the correct
   glyph position
3. **Text selection** — tracking and rendering selected character ranges
4. **Keyboard event routing** — intercepting key events and dispatching them
   to the focused field
5. **IME support** — handling input method compositions for CJK and other
   scripts

None of these are currently in scope for the paint-vm backend.

### §9.2 What paint-vm emits for Input

Rather than refusing to render `Input` nodes, the paint-vm backend emits a
`TextInput` widget descriptor into the PaintScene. This descriptor records
everything needed for the Widget Runtime to manage the field:

```
PaintTextInput {
  bounds:       Rect { x: 120, y: 8, width: 480, height: 28 },
  value:        "=SUM(A1:A10)",
  placeholder:  "",
  read_only:    false,
  multiline:    false,
  max_length:   None,
  part_name:    "formula-field",
  on_change_id: EmitId(1),   // resolved emit handle for onFormulaChange
  on_commit_id: EmitId(2),   // resolved emit handle for onCommit
  on_cancel_id: EmitId(3),   // resolved emit handle for onCancel
}
```

The pixel renderer ignores `PaintTextInput` and renders a static placeholder
rectangle (using the part's resolved mosstyle properties for background and
border). This gives a visual representation of the field even without
interactivity.

### §9.3 Widget Runtime layer (future)

The Widget Runtime layer (planned spec: **UI-WR1**) sits between the host
application and the paint-vm renderer. It:

1. Receives the `PaintScene` from the paint-vm backend
2. Walks the scene for `PaintTextInput` descriptors
3. Manages focus among all discovered input fields
4. Intercepts keyboard events from the platform window
5. Routes keystrokes to the focused field, updating the value
6. Re-renders the caret via additional paint instructions injected on top of
   the base scene
7. Fires `EmitId` callbacks when `onChange`, `onCommit`, or `onCancel` events
   occur

UI-WR1 is a distinct specification and is not blocked by this spec. The
`PaintTextInput` descriptor format is the agreed contract between this spec and
UI-WR1. When UI-WR1 is implemented, full interactive `Input` support in the
paint backend follows automatically — no changes to the moslayout or mosmodel
layers are needed.

---

## §10 Testing Additions for mosaic-emit-react

The following tests must be added to the `mosaic-emit-react` test suite to
cover the `Input` primitive. Each test follows the existing pattern: parse
a `.mosaic` source string, run `MosaicVM` with `ReactRenderer`, assert on the
resulting output string.

| Test | What it covers |
|---|---|
| `test_input_renders_input_tag` | Single-line Input → `<input type="text" …>` appears in JSX output |
| `test_input_value_bound_to_prop` | `value={propName}` appears with the camelCased slot name |
| `test_input_multiline_renders_textarea` | `multiline: true` → `<textarea>` instead of `<input type="text">` |
| `test_input_readonly_prop` | `read-only` slot ref → `readOnly={propName}` prop on the element |
| `test_input_readonly_literal_true` | `read-only: true` literal → `readOnly={true}` |
| `test_input_placeholder_attribute` | `placeholder: "Enter …"` → `placeholder="Enter …"` in JSX |
| `test_input_maxlength_attribute` | `max-length: 256` → `maxLength={256}` in JSX |
| `test_input_onchange_dispatch` | `connects: onChange(…) -> emit …` → `onChange={e => dispatch({type: …, …: e.target.value})}` |
| `test_input_commit_on_enter` | `connects: onCommit -> emit …` → `onKeyDown` handler dispatches on `e.key === "Enter"` |
| `test_input_cancel_on_escape` | `connects: onCancel -> emit …` → `onKeyDown` handler dispatches on `e.key === "Escape"` |
| `test_input_no_connects_no_handlers` | Input with no connects → element has no `onChange` or `onKeyDown` props |
| `test_input_controlled_component` | `value` is always a controlled prop, not `defaultValue` |
| `test_input_event_union_includes_input_events` | The discriminated union type includes entries for all connected Input emits |
| `test_input_formula_bar_complete` | Full FormulaBar three-event wiring produces the exact TSX shown in §8 |

---

## §11 Grammar Additions Summary

For reference, here is the complete diff to the existing grammar showing what
changes (additions only — no existing rules are modified or removed):

### New tokens (additions to `moslayout.tokens`)

```
KW_INPUT      = "Input"
KW_VALUE      = "value"
KW_PLACEHOLDER = "placeholder"
KW_READ_ONLY  = "read-only"
KW_MULTILINE  = "multiline"
KW_MAX_LENGTH = "max-length"
KW_ON_CHANGE  = "onChange"
KW_ON_COMMIT  = "onCommit"
KW_ON_CANCEL  = "onCancel"
AT            = "@"
ARROW         = "->"
```

### New grammar rules (additions to `moslayout.grammar`)

```
# Updated node alternation (Input added)
node = box_node | row_node | column_node | text_node | image_node
     | spacer_node | scroll_node | grid_node | input_node | component_node ;

input_node = KW_INPUT LBRACE { input_item } RBRACE ;

input_item = input_prop | part_name | connects_clause ;

input_prop =
    KW_VALUE        COLON slot_ref_or_literal SEMICOLON
  | KW_PLACEHOLDER  COLON STRING              SEMICOLON
  | KW_READ_ONLY    COLON slot_ref_or_literal SEMICOLON
  | KW_MULTILINE    COLON bool_lit            SEMICOLON
  | KW_MAX_LENGTH   COLON NUMBER              SEMICOLON
  ;

slot_ref_or_literal =
    AT KEBAB_IDENT
  | STRING
  | bool_lit
  | NUMBER
  ;

bool_lit = KW_TRUE | KW_FALSE ;

connects_clause =
    KW_CONNECTS COLON payload_event ARROW KW_EMIT emit_target SEMICOLON
  | KW_CONNECTS COLON void_event    ARROW KW_EMIT void_emit   SEMICOLON
  ;

payload_event = KEBAB_IDENT LPAREN KEBAB_IDENT COLON "text" RPAREN ;
void_event    = KEBAB_IDENT ;
emit_target   = KEBAB_IDENT LPAREN KEBAB_IDENT COLON KEBAB_IDENT RPAREN ;
void_emit     = KEBAB_IDENT ;
```

### Backend mapping table addition (appended to UI14 §8)

```
Input | TextInput widget (paint-vm) | <input> or <textarea> (web) | QLineEdit/QTextEdit (Qt) | TextField/TextEditor (SwiftUI)
```

### Error message table additions (appended to UI14 §9)

```
MissingRequired   | Input missing value property      | Input at line 12: 'value' property is required
SlotRefNotAllowed | multiline or max-length is a slot  | Input at line 18: 'multiline' must be a literal
DuplicateConnects | Same event wired twice             | Input at line 23: 'onChange' already has a connects clause at line 21
EmitTypeMismatch  | onChange wired to wrong-type emit  | Input at line 21: connects onChange payload is 'text' but emit parameter 'formula' is 'number'
```

---

## §12 Out of Scope

The following are explicitly not part of this spec:

- **Combobox / autocomplete** — a future `Combobox` primitive will extend
  `Input` with a dropdown suggestion list. This is a separate primitive, not a
  property of `Input`.
- **Password fields** — `type="password"` behavior (masked display) is a future
  property addition (`secret: true`), not in this spec.
- **Number inputs** — type="number" behavior with up/down arrows. Use `Input`
  with host-side validation for now; a `NumericInput` primitive may follow.
- **File inputs** — `type="file"` is a completely different primitive with
  picker dialogs. Out of scope for text input.
- **Placeholder color in mosstyle** — a `placeholder-color` style property may
  be added in a future mosstyle revision; it is not in v1.
- **IME composition events** — compositionstart/compositionupdate/compositionend
  are deferred to UI-WR1 (Widget Runtime) and a future `onChange`
  extension.
- **Drag-and-drop text** — similarly deferred.
- **Accessible label association** — ARIA `aria-label` and `aria-labelledby`
  are generated by the backend emitter based on adjacent `Text` parts in the
  layout; no explicit moslayout syntax is needed for v1.
- **Widget Runtime implementation** — the full interactive implementation of
  Input in the paint-vm backend is deferred to spec UI-WR1. This spec only
  defines the `PaintTextInput` descriptor contract.
