# browser-form-controls

Owns browser form-control focus, value, selection, checked state, and semantic
keyboard/pointer transitions without depending on a window toolkit or paint
backend. It projects state into `BrowserRenderTree` for shared reflow.

The model also retains each control's initial value, checked state, selected
options, and form association. Form engines can query those bindings and reset
one form without coupling submission policy to layout or a platform host.

Text controls retain character-indexed anchor/focus selection, composition
text, and validation feedback. Grapheme-safe deletion and movement reuse the
generated Unicode 17 `text-flow` segmenter; word movement, double-click word
selection, and triple-click line selection share one reducer. Per-control,
100-snapshot undo/redo history restores values and selections together, while
drag autoscroll remains a bounded function of explicit viewport metrics.
Password values are only exposed to layout through their masked
`display_value`. Invalid forms can
focus the first failing control and synchronize `aria-invalid` plus an
accessible description without putting validation policy in native adapters.

`ControlEditorPresentation` converts that retained state into reusable logical
rectangles for carets, selections, composition underlines, validation, and IME
candidate placement. Pointer placement and drag selection use explicit text
metrics, control viewports scroll to reveal the caret, and blink timing advances
only from host-supplied elapsed time. Clipboard copy/cut/paste remains in the
shared reducer; payloads negotiate plain and escaped HTML flavors, and password
controls never return clipboard payloads. Platform accessibility adapters use
the same movement, selection, replacement, and transaction actions.

Typed inputs also share one value reducer. Unicode-scalar `maxlength` is
enforced during replacement, email and absolute URL syntax produce reusable
diagnostics, and finite number constraints drive min/max/step validation plus
Arrow Up/Down stepping. `ControlValueState` exposes current value,
`inputmode`, selection capability, numeric range metadata, and typed validity
diagnostics; accessibility SetValue/Increment/Decrement actions enter the same
transaction path. Number controls deliberately do not expose public selection
or clipboard ranges.

Choice and range controls use the same reducer. Multi-select retains ordered
selected indices, skips disabled options and optgroups during keyboard or
accessibility selection, and exposes listbox option state. Checkbox activation
clears an indeterminate state, radio arrows wrap within the enabled members of
their form-owned group, and range values are finite, clamped, step-aligned, and
driven by semantic arrows, Home/End, and accessibility value actions.
`ControlChoiceState` publishes role, checked/mixed state, active option,
per-option availability/selection, and slider value metadata without a host
widget type.

Picker-backed values remain host neutral too. Date, month, ISO week, time,
datetime-local, and simple color inputs normalize through shared parsers before
entering retained state. Temporal controls use canonical day/month/week or
millisecond scalars for min/max/step diagnostics and semantic stepping;
`ControlValueState::value_text` gives accessibility adapters the normalized
human-readable value. Color values are strict lowercase `#rrggbb`. Hosts may
present native pickers later, but SetValue, Increment/Decrement, arrows, and
Home/End always return through this reducer.

File inputs emit `ControlFilePickerRequest` rather than reading a host path.
Hosts return opaque identifiers, sanitized display names, optional media types,
and bounded bytes. The reducer owns accept/multiple filtering and reset state;
`ControlFileState` exposes a payload-free file list and accessible value text.
Per-file and aggregate retained-byte limits are enforced before state changes.

Text/search/textarea `dirname` metadata stays in the control binding. The
shared model resolves explicit and inherited direction, and recomputes
`dir=auto` from the live value with the Unicode bidi analyzer. Accessibility
activation also enters the same semantic path as pointer and keyboard input.

Autonomous custom elements are retained as attachable ElementInternals-style
candidates with stable document ordinals. Attached internals own bounded
string/file/entry-list values and restoration state, explicit or containing
form association, validity messages and anchors, disabled/reset/restore
lifecycle effects, label-derived accessibility metadata, and shared numeric
value actions. This state remains separate from native `ControlState` because
custom elements decide their own rendering while sharing form policy.

Form state and autofill remain in this reducer as well. Each native control
exposes its immutable default and live dirty flags; bounded snapshots omit
passwords unless credential access is explicit and never retain file bytes.
Autocomplete section, address, contact, and purpose tokens produce ordered
descriptors. A bounded autofill transaction normalizes values through the same
typed/choice paths, blocks credential fields by default, and emits `input`
before `change` for each updated control. History restoration is event-silent,
restores dirty flags, and queues custom-element restore callbacks until their
internals attach.

Datalist-backed controls resolve authored options before entering the reducer.
Shared queries cap source options, query bytes, and returned results; skip
disabled, duplicate, and type-invalid candidates; and match values, labels, or
fallback text without exposing the full source list to hosts. Arrow keys and
accessibility actions move one active option, Escape cancels, and commit marks
the value dirty and enters edit history as one input/change transaction.
Native pickers consume only the bounded `ControlSuggestionState` JSON
projection and return semantic actions.

Specs: [`UI53-layout-controls`](../../../specs/UI53-layout-controls.md) and
[`UI55-form-editing`](../../../specs/UI55-form-editing.md), and
[`UI56-editor-presentation`](../../../specs/UI56-editor-presentation.md), and
[`UI62-advanced-editing-transactions`](../../../specs/UI62-advanced-editing-transactions.md), and
[`UI63-typed-input-value-semantics`](../../../specs/UI63-typed-input-value-semantics.md),
[`UI64-choice-range-controls`](../../../specs/UI64-choice-range-controls.md), and
[`UI65-temporal-color-controls`](../../../specs/UI65-temporal-color-controls.md), and
[`UI66-file-values-multipart`](../../../specs/UI66-file-values-multipart.md), and
[`UI67-image-submit-dirname`](../../../specs/UI67-image-submit-dirname.md), and
[`UI68-form-associated-custom-elements`](../../../specs/UI68-form-associated-custom-elements.md).
State restoration and autofill are specified by
[`UI70-form-state-autofill`](../../../specs/UI70-form-state-autofill.md).
Datalist behavior is specified by
[`UI71-datalist-picker-mediation`](../../../specs/UI71-datalist-picker-mediation.md).
