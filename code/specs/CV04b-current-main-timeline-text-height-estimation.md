# CV04b — Timeline period-card scalar height estimation

This bounded slice follows published carrier
`7f9e541c850b7a874d8c143cb9caa2509c2d352c`. The complete 46-path endpoint
inventory against main `1ff5c89aa3773f96990dfa0db4395dae700ce24d` has SHA256
`0cda3f7f8471703efc9e19801f05db025ca8d82a77195b2e6b00efe34d2f2f94`.
This is preparatory evidence, not acceptance of incoming main.

The preparatory spec at dbc3553937d included period and event estimates.
Before tests/implementation, fixture audit showed default16 events would grow
from30 to31.2, changing many complete Scene/arrow and f64-association controls.
Narrow this first implementation to period-card estimates. Event estimates
and cumulative stacks require a separate, explicit expected-product migration.

This estimates vertical space; it does not shape text, rewrite authored
strings, insert breaks, guarantee containment, or prove backend/raster fit.
The painter's fixed period-label height22 remains a separate limitation.
Keep title/section/event minima, event height/pitch/indexing, owned widths
(including TD event minimum160), palette ordinals, order, metadata and fonts.
Larger cards feed only the current midpoint/LR-origin/row-height formulas.

Miri is DEFERRED/PENDING, never passed. Preserve the frozen run, checkout,
inputs, receipts and target untouched and unpolled. Use a new isolated Cargo
target. Full Closure parity, stable identities/spans, output-byte/source
joins, every requested transformation/pass-sweep lineage, whole-main
reconciliation, hosted checks, actual merge and final-source Miri remain OPEN.

## Checked scalar estimator

Retain raw error order Canvas, Padding, LeftMargin, FontSize and all four
eager planned role products from the size-minima spec, before allocation and
section predicate/visit work, including empty input. New arithmetic refusal
uses existing TimelineGeometry. No new dependency, API, public type,
serialized-boundary change or magnitude/string/count cap is introduced.

For each actual period label let w be TD180 or LR190, s the admitted positive
finite configured size and m the admitted card minimum max(s*3,48):

1. Capacity is floor((w-20)/(s*0.44)).max(1), converted to usize with Rust's
   defined saturating float-to-integer conversion. Certify w and w-20 finite
   and positive, and s*0.44 finite. Subnormal underflow to zero is allowed:
   positive infinity in the quotient saturates to usize::MAX capacity. An
   otherwise overflowing positive quotient also saturates. This explicit
   scalar-count heuristic does not admit infinite layout coordinates.
2. Iterate str::lines(), retaining Rust CRLF and trailing-terminator semantics.
   For each line count Unicode scalars, replace zero with one, and apply
   integer div_ceil(capacity). Sum with checked usize addition; replace an
   empty sum by one. Spaces, combining marks and wide characters count per
   scalar, with no glyph/grapheme inference or allocated wrapped string.
3. Evaluate (lines as f64 * s) * 1.2 + 12 in that order, checking both
   multiplications and the addition as finite BEFORE max(m). Count conversion
   uses Rust's normal f64 rounding. Refuse overflow with TimelineGeometry.

Short default cards remain48; short size24 cards remain72. Three estimated
lines at size16 produce69.6; at24 they produce98.39999999999999 with these
represented operations. Tiny positive sizes retain the literal minimum.
The private helper is called after ingress; direct tests also cover its
finite/positive width contract.

Only replace the two actual TD/LR card-height assignments. Preserve the TD
max(card,event stack), indexed event coordinates and row/section advances.
Preserve LR row max and the exact grouped event expression
card_y + (card_height + 42) + event_index * event_pitch. Keep final actual
rectangle and rounded-arrow certification, panic API and error/work controls.

## Test-first acceptance

Commit this narrowed spec before tests or production changes. All existing
period fixtures P, Q, P0..P6 and year labels fit the relevant capacities;
the size65536 fixture P still stays under its role minimum. Retain every old
authored source/IR, complete literal layout/Scene/query, typed error and
work-counter control unchanged. Forged multiline layouts sent directly to
the painter retain their old contract; never route them through the estimator.
Any newly discovered affected expected product must be inventoried explicitly,
with its complete old shape retained and a declared complete new expectation.
Never normalize actual outputs.

Add independent complete TD/LR products with short and growing periods,
unequal card heights, multiple indexed events, named/unnamed and empty rows.
Cover hard lines, CRLF, trailing terminators, empty strings, spaces, Unicode
scalar-versus-grapheme counts, capacity transitions22/23 and24/25 at16,
three-line growth, adjacent f64 derived multiplication refusal, subnormal
admission, raw precedence and eager zero-section-work refusal. Full products
must verify card frames, link centers, LR event origins, next-period and
section advances while event height/indexing remains exact.

Compare representative complete authored source/IR/layout/Scene/font-query
products with long period labels. FakeShaper output and the scalar-height
heuristic are different contracts; taller cards do not prove text wrapping.
Run compiled unfiltered semantic red before production edits, retaining
actual exits and executable identities separately from runner acceptance.
Recompute the affected graph/target census and run native/downstream suites,
doctests and strict all-target checks in the fresh target. Obtain independent
exact-source and actual-terminal security review. Update README/changelog,
backlog and owned issue checkpoints before reviewed normal progress push to
the sole active PR. Event estimates/cumulative stacks and incoming main remain
separate unaccepted work. Auto-merge requires current conflict-free,
review-clean exact-head terminal acceptable required checks and actual main
reachability. No merge approval or full completion is implied by this slice.
