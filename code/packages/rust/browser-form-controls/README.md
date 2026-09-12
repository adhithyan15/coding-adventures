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

Specs: [`UI53-layout-controls`](../../../specs/UI53-layout-controls.md) and
[`UI55-form-editing`](../../../specs/UI55-form-editing.md), and
[`UI56-editor-presentation`](../../../specs/UI56-editor-presentation.md), and
[`UI62-advanced-editing-transactions`](../../../specs/UI62-advanced-editing-transactions.md), and
[`UI63-typed-input-value-semantics`](../../../specs/UI63-typed-input-value-semantics.md).
