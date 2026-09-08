# browser-form-submission

Plans HTML form activation without depending on a window toolkit, network
transport, layout engine, or paint backend. It validates retained control
state, collects successful controls in document order, serializes UTF-8
`application/x-www-form-urlencoded` payloads, and produces GET or POST
navigation requests for Venture hosts.

The planner bounds entries, payload bytes, and diagnostics. Reset activation
is returned as an explicit effect so the shared control model can restore its
initial state before ordinary retained-page reflow.

Spec: [`UI54-form-submission`](../../../specs/UI54-form-submission.md).
