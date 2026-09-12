# UI64: Advanced Choice and Range Controls

## Status

Implemented by `layout-controls`, `browser-form-controls`,
`browser-form-submission`, and `venture-browser-core`.

## Ownership

`ControlState` carries backend-neutral selected indices, disabled-option flags,
checkbox indeterminate state, and range identity. `BrowserControlModel` owns
all mutation policy. Hosts translate semantic keys, pointer activation, and
accessibility actions, then request ordinary retained reflow; they do not keep
a second widget value or selection.

## Choice Controls

- A select retains an active index separately from its ordered selected-index
  set. Single-select commits one enabled option. Multi-select supports replace,
  toggle, and contiguous extension while skipping disabled options.
- An option disabled directly or through an optgroup can remain observable in
  initial document state, but keyboard and accessibility actions cannot select
  it and successful-control serialization omits it.
- Activating a checkbox clears `indeterminate` before toggling `checked`.
- Radio arrow keys wrap through enabled controls with the same name and form
  owner. The reducer moves focus and checked state together.

## Range Controls

Range inputs use finite numeric values with HTML-profile defaults of `0`,
`100`, and `1`. Invalid or absent values start at the step-aligned midpoint;
authored values and accessibility SetValue are clamped and aligned. Arrow keys
and Increment/Decrement move one step, while Home and End choose the bounds.

## Accessibility State

`ControlChoiceState` exposes a toolkit-free checkbox, radio, combobox, listbox,
or slider projection. It includes checked/mixed state, active option, ordered
option values with disabled and selected flags, and finite slider
value/minimum/maximum/step metadata. `SelectOption`, `Toggle`, and
`SetIndeterminate` enter the same reducer as keyboard and pointer events.

## Submission And Acceptance

The submission planner reads live selected indices in document order and emits
one entry per enabled selection. Required selects fail when no enabled value is
selected. Deterministic tests cover disabled optgroups, multi-select extension
and toggle, checkbox mixed transitions, radio wrapping, range normalization,
shared accessibility actions, retained-session reflow, and successful-control
serialization. Existing generated native and web hosts already translate the
semantic arrow, Home, End, Space, and Enter keys consumed by this reducer.
