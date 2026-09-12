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
step diagnostics cannot drift into separate host-specific policies.
Live select serialization likewise reads the shared selected-index state and
omits disabled options, including options disabled through an optgroup.

Specs: [`UI54-form-submission`](../../../specs/UI54-form-submission.md) and
[`UI63-typed-input-value-semantics`](../../../specs/UI63-typed-input-value-semantics.md), and
[`UI64-choice-range-controls`](../../../specs/UI64-choice-range-controls.md).
