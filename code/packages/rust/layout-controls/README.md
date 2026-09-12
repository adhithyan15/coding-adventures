# layout-controls

Backend-neutral form-control state, intrinsic sizing, and Layout IR metadata.
The package owns no HTML parsing, CSS cascade, paint backend, or host toolkit.

`ControlKind` distinguishes text, email, URL, password, number, range,
date, month, week, time, datetime-local, color, boolean, choice, and multiline
families and publishes which kinds support
public text selection and live `maxlength` enforcement. Choice metadata keeps
disabled options, every selected index, and checkbox indeterminate state in
Layout IR so paint and host adapters consume the same retained projection.

Specs: [`UI53-layout-controls`](../../../specs/UI53-layout-controls.md) and
[`UI63-typed-input-value-semantics`](../../../specs/UI63-typed-input-value-semantics.md),
[`UI64-choice-range-controls`](../../../specs/UI64-choice-range-controls.md), and
[`UI65-temporal-color-controls`](../../../specs/UI65-temporal-color-controls.md).
