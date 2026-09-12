# UI70: Form State Restoration And Autofill

## Status

Implemented for Venture's shared native/web control pipeline.

## State Contract

`browser-form-controls` retains immutable defaults separately from each live
value and checked state. User, accessibility, choice, typed-value, and file
mutations set the corresponding dirty flag; form reset restores the authored
baseline and clears those flags.

History snapshots are bounded by control count and encoded bytes. Public mode
omits password values, and file controls are always excluded so host paths and
payload bytes cannot enter session history. Credential mode is an explicit
caller capability. Back and Forward save the departing public snapshot only
after destination loading succeeds, then restore a matching final URL before
reflow. Restoration does not synthesize input or change events.

Attached form-associated custom elements contribute their already-bounded text
restoration state; file-backed state is excluded from history. A restored page
queues that state until internals attach, then emits `FormAssociated` before
`FormStateRestore` with Restore mode.

## Autofill Contract

Each eligible native control exposes a document-ordered descriptor containing
its autocomplete section, shipping/billing address group, contact group,
purpose, enabled state, and sensitivity. Explicit tokens win; conservative
type/name inference covers common fields when tokens are absent. Form-level or
control-level `autocomplete=off`, disabled, readonly, file, and button controls
are not fillable.

Hosts submit only bounded section/purpose/value records. Public transactions
skip passwords, one-time codes, and payment credentials. Credential mode must
be selected explicitly. Accepted text, numeric, temporal, color, range, and
select values use the same normalization and limits as direct interaction.
Each changed control emits `input` followed by `change` in document order;
custom-element autocomplete restoration uses the existing lifecycle callback.

Generated SwiftUI, WinUI, Qt, Flutter, Compose, HTML, Web Component, React, and
Electron hosts translate descriptors and events but do not own persistence,
credential policy, normalization, grouping, dirty flags, or restoration order.
