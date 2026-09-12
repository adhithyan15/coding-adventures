# UI65: Temporal and Color Value Controls

## Status

Implemented by `layout-controls`, `browser-form-controls`,
`browser-form-submission`, and `venture-browser-core`.

## Ownership

`ControlKind` identifies date, month, week, time, datetime-local, and color
inputs in backend-neutral Layout IR. `BrowserControlModel` is the sole owner of
parsing, normalization, constraints, and mutation. A host may display a native
picker, but it must send semantic value actions back to the shared reducer and
must not retain an independent value.

## Canonical Values

- Dates use valid Gregorian `YYYY-MM-DD` values and a day scalar relative to
  1970-01-01. Month values use `YYYY-MM` and a month scalar from January 1970.
- Weeks use valid ISO week-year `YYYY-Www` values, including the 52/53-week
  rule, and a week scalar from 1970-W01.
- Times use `HH:MM`, optional seconds, and up to millisecond precision.
  Datetime-local combines that time grammar with a Gregorian date and has no
  timezone conversion.
- Colors use the HTML simple-color form `#rrggbb`; hexadecimal digits are
  normalized to lowercase and invalid authored values use `#000000`.

Invalid authored temporal values become empty retained values. Canonical
values round-trip deterministically and do not depend on locale, timezone, or
host date libraries.

## Bounds And Stepping

Temporal min and max attributes parse through the same value grammar. Date,
month, and week steps use their respective scalar units. Time and
datetime-local steps are authored in seconds and retained in milliseconds;
their default step is 60 seconds. The other temporal defaults are one unit.
`step=any` disables mismatch diagnostics while semantic stepping retains the
default increment.

Arrow Up/Right and Increment advance one step; Arrow Down/Left and Decrement
retreat one step. A mismatched current value first aligns in the requested
direction. Home and End select valid authored bounds. Every mutation clamps to
valid bounds and returns through ordinary retained reflow.

## Diagnostics And Accessibility

`ControlValueState` exposes the canonical string as `value_text`, the scalar
current/minimum/maximum/step values, selection capability, and bounded
diagnostics. Diagnostics distinguish malformed values, invalid min/max/step
metadata, range underflow/overflow, and step mismatch. SetValue and
Increment/Decrement accessibility actions share the keyboard reducer; temporal
and color controls never expose public text selection.

## Submission And Acceptance

Successful-control planning serializes the normalized retained string, never a
host-formatted picker label. Deterministic unit, submission, retained-session,
and Venture package fixtures cover leap dates, ISO week 53, subsecond time,
datetime-local, color normalization, alignment, bounds, accessibility actions,
and URL encoding. Existing native and web hosts already translate the semantic
keys and accessibility actions consumed by this phase.
