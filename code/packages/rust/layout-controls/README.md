# layout-controls

Backend-neutral form-control state, intrinsic sizing, and Layout IR metadata.
The package owns no HTML parsing, CSS cascade, paint backend, or host toolkit.

`ControlKind` distinguishes text, email, URL, password, number, boolean,
choice, and multiline families and publishes which kinds support public text
selection and live `maxlength` enforcement.

Specs: [`UI53-layout-controls`](../../../specs/UI53-layout-controls.md) and
[`UI63-typed-input-value-semantics`](../../../specs/UI63-typed-input-value-semantics.md).
