# browser-form-submission

Plans HTML form activation without depending on a window toolkit, network
transport, layout engine, or paint backend. It validates retained control
state, collects successful controls in document order, serializes UTF-8
`application/x-www-form-urlencoded` payloads, and produces GET or POST
navigation requests for Venture hosts.

The planner bounds entries, payload bytes, and diagnostics. Reset activation
is returned as an explicit effect so the shared control model can restore its
initial state before ordinary retained-page reflow.

Constraint planning consumes `browser-form-controls::ControlValueState`, so
live editor feedback and submit-time email, URL, length, numeric range, and
step diagnostics cannot drift into separate host-specific policies. The same
path validates temporal bounds/alignment and serializes normalized temporal and
simple-color values, so submission never depends on a native picker format.
Live select serialization likewise reads the shared selected-index state and
omits disabled options, including options disabled through an optgroup.
For multipart POST, ordered file parts consume path-free retained selections,
use sanitized disposition parameters and normalized media types, and enforce a
complete-body byte bound. Boundaries are deterministic and retried if they
occur in any submitted value or file payload.
Image submitters expand at their document position into bounded integer `x`
and `y` entries: pointer activation uses control-local coordinates while
keyboard and accessibility activation use `(0, 0)`. Text/search/textarea
`dirname` entries immediately follow their owning control and use the shared
live Unicode directionality result.
Attached form-associated custom elements contribute bounded string, file, or
entry-list values at their retained document ordinal. Their shared validity
messages block navigation through the same diagnostic path as native controls;
disabled or detached internals are never successful.

Specs: [`UI54-form-submission`](../../../specs/UI54-form-submission.md) and
[`UI63-typed-input-value-semantics`](../../../specs/UI63-typed-input-value-semantics.md),
[`UI64-choice-range-controls`](../../../specs/UI64-choice-range-controls.md), and
[`UI65-temporal-color-controls`](../../../specs/UI65-temporal-color-controls.md), and
[`UI66-file-values-multipart`](../../../specs/UI66-file-values-multipart.md), and
[`UI67-image-submit-dirname`](../../../specs/UI67-image-submit-dirname.md), and
[`UI68-form-associated-custom-elements`](../../../specs/UI68-form-associated-custom-elements.md).
