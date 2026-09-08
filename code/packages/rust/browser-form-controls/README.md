# browser-form-controls

Owns browser form-control focus, value, selection, checked state, and semantic
keyboard/pointer transitions without depending on a window toolkit or paint
backend. It projects state into `BrowserRenderTree` for shared reflow.

The model also retains each control's initial value, checked state, selected
options, and form association. Form engines can query those bindings and reset
one form without coupling submission policy to layout or a platform host.

Text controls retain character-indexed anchor/focus selection, composition
text, and validation feedback. Range replacement and deletion are UTF-8 safe,
textarea Home/End navigation respects line boundaries, and password values are
only exposed to layout through their masked `display_value`. Invalid forms can
focus the first failing control and synchronize `aria-invalid` plus an
accessible description without putting validation policy in native adapters.

Specs: [`UI53-layout-controls`](../../../specs/UI53-layout-controls.md) and
[`UI55-form-editing`](../../../specs/UI55-form-editing.md).
