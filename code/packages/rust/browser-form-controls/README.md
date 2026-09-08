# browser-form-controls

Owns browser form-control focus, value, selection, checked state, and semantic
keyboard/pointer transitions without depending on a window toolkit or paint
backend. It projects state into `BrowserRenderTree` for shared reflow.

The model also retains each control's initial value, checked state, selected
options, and form association. Form engines can query those bindings and reset
one form without coupling submission policy to layout or a platform host.

Spec: [`UI53-layout-controls`](../../../specs/UI53-layout-controls.md).
