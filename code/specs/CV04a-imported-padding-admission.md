# CV04a — imported padding admission and final common-origin controls

Independent review of e32c4728 found that the d301 padding producer reaches
floating clamp before try_layout_swimlane validates this new caller-owned
field. A NaN lower bound can panic inside the fallible public API. This is a
static counterexample until the direct native red control actually executes.

Extend the existing finite inclusive 0..=65536 spacing admission list with
flowchart padding. Return UnsupportedSpacing { field: "padding" } before
layout arithmetic for NaN, either infinity, negative values, and cap+epsilon.
Do not change valid layout arithmetic, upstream padding semantics, parser
admission, limits, wrapper behavior or any existing control. Test all four
directions, preserve input bits, and assert finite accepted zero/default/cap
layouts. Keep the failed control receipt and rerun the full affected cone.

The common-origin candidate module already passes six new tests and the
original two tests containing all32 varied-support cases at e32c4728. Close
review coverage gaps with direct permanent-invalidation/re-entry controls and
integrated endpoint-equality / maximum-coordinate / minimal-dx queries against
the independent original directed predicate. Require selected candidates to
equal the independent exact strict-interior inversion/end-bound oracle, and
returned raw signatures to equal the original predicate traversal. Both axes
and independent directions apply. Numeric equality of unreduced rational
extrema is the invariant; raw representation is not a public contract.

Strict Clippy found a test-only complex tuple annotation. Introduce a named
test oracle group type without changing grouping or assertions. No lint allow,
capacity increase, ignored tests, unsafe or new dependency.

Current full affected diagram suites at e32c4728:843PASS/0FAIL/0ignore/32targets;
strict failed on that annotation. This is scoped source evidence, not final
security or whole-goal acceptance. Full Closure parity/provenance/general
floating work/hosted merge remain OPEN. Miri DEFERRED/PENDING; frozen run,
checkout, inputs, target and receipts remain untouched.

Further independent review identifies a second incoming padding gap: LR/RL
lanes retain a fixed150 cross extent while accepted node padding can make
node height exceed it (padding100 gives226). Centering then places nodes above
the lane/canvas and can clip output. This static counterexample also needs an
executed red control. Resolve the horizontal cross extent symmetrically with
the existing vertical width rule: max(150,nodeHeight+18), then add the existing
60-per-group-depth margin. Default dimensions stay exact because54+18<150.
No other cursor, ranking, node-size, spacing or rendering formula changes.

Add complete independently computed two-lane golden layouts for all four
directions at default, zero,14,100 and65536 padding, including exact canvas,
lane/node positions/dimensions, preserved input/config/metadata and lane/canvas
enclosure. Keep the native red receipt, then rerun the full affected suites,
strict lint and independent review. Accepted padding is a bounded functional
feature; finite dimensions alone do not establish that nodes fit the canvas.
