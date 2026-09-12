# UI63: Typed Input Value Semantics

## Status

Implemented by `layout-controls`, `browser-form-controls`,
`browser-form-submission`, and `venture-browser-core`.

## Ownership

`ControlKind` identifies URL and numeric controls without importing HTML or a
toolkit. `BrowserControlModel` owns value mutation, constraints, keyboard
stepping, and reusable diagnostics. The submission planner consumes those
diagnostics instead of implementing a second validity model. Hosts translate
text, semantic keys, and accessibility actions, then repaint after the shared
session reflows.

## Text Values

- A valid nonnegative `maxlength` limits every live replacement by Unicode
  scalar count. Selected text is removed before the remaining capacity is
  calculated, and excess replacement text is deterministically truncated.
- `maxlength` does not apply to number controls. Programmatic initial values
  that already exceed a text limit remain observable as `too-long` rather
  than being silently rewritten.
- Email controls validate one address, or a comma-separated list when
  `multiple` is present. URL controls require a parseable absolute URL.
- Number controls do not expose public selection or clipboard ranges. Other
  editable types preserve the shared scalar-indexed selection contract.

## Numeric Values

Finite decimal values expose parsed current, minimum, maximum, and step
metadata. Missing or invalid positive steps default to `1`; `step="any"`
disables step-mismatch validation while retaining unit keyboard stepping. The
step base is `min` when present and zero otherwise.

Arrow Up and accessibility Increment move to the next aligned value; Arrow
Down and Decrement move to the previous aligned value. A mismatched current
value first snaps in the requested direction, bounds clamp the result, and an
empty or malformed value starts from the relevant bound. Every mutation uses
the ordinary bounded undo/redo transaction path.

## Value State

`ControlValueState` is the reusable validation and accessibility projection.
It carries the current string, `inputmode`, selection capability, numeric
value/range/step metadata, and ordered typed diagnostics. Diagnostic codes are
`too-short`, `too-long`, `type-mismatch`, `bad-input`, `range-underflow`,
`range-overflow`, and `step-mismatch`. `BrowserSession` exposes the projection
for a named or focused control, while `ControlAccessibilityAction` adds
SetValue, Increment, and Decrement.

## Acceptance

Deterministic tests cover Unicode live limits, replacement capacity, URL and
email syntax, finite number parsing, step alignment and clamping, undo,
selection restrictions, accessibility value actions, submission blocking,
and retained-session reflow. Native and web hosts share the existing semantic
Arrow Up/Down and accessibility action seams; no toolkit owns value policy.
