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


## Bounded execution checkpoint

Period cards now estimate vertical space from Unicode scalar counts, owned
width180/190 and positive configured size s. Capacity is floor((width-20)/
(s*0.44)), at least1, with defined saturating usize conversion for positive
quotient overflow/subnormal denominator underflow. Rust hard-line semantics,
integer div_ceil and checked accumulation precede ordered finite checks of
(lines*s)*1.2+12, then max with the existing max(3s,48) card minimum. Overflow
returns TimelineGeometry; no arbitrary cap or wrapped-string allocation.

Title/section/event minima, raw error order, eager empty admission, event
heights/pitch/indexing, grouped LR origin, widths, palette ordinals, authored
strings/IR, metadata/fonts and represented frame/arrow guards remain. Existing
short-label products stay exact. Taller cards update current midpoint and row
dependencies. The painter still uses fixed period text height22: complete
FakeShaper controls retain an oversized unbroken word and three hard-line
glyph runs; neither shaping fit nor containment/raster output is certified.
Event-height estimates and cumulative event stacks remain a separate slice.

Executed source `a0a5681f4295bd8083403f2cb412b9234f95e4bf`: 289 native passes across3 unfiltered harnesses
(Windows e2e0), no failures/ignores/filters. Two doctest invocations contain
0 tests; two strict all-target checks and metadata passed:8 actual commands0.
Fresh1293manifest/1292node/436registry graph gives affected2/forward50.
All407 previous completion identities plus53 new typed and6 complete
source/IR/layout/Scene/query products pass. Compiled semantic red at
`144b1df697f782aef3c7ccec7d82b03a15c973ea` was283P6F before production; both executables and raw streams
are immutable. Exact-source and actual-terminal security reviews PASS/RELEASED.
The EOF whitespace failure (shell1/nativegit2), separate diagnostic assertion
failure and one-LF repair/nativegit0 remain documented in filled lesson shards;
none is relabeled a compiler failure, semantic red or green acceptance.

PR17288 remains unmerged; whole-main/conflicts/current required checks/review
and actual merge remain OPEN. Miri DEFERRED/PENDING, never passed; frozen
run/checkout/inputs/receipts/target untouched and unpolled. Full Closure parity,
stable identities/spans, output-byte/source joins and fold/rename/inline/motion/
merge/delete/pass-sweep lineage remain OPEN. Genuine final-source Miri follows
implementation delivery. No new compiler-wide or Apple rendering run is claimed.
Green receipt `d559f2382dbb398795cc9bb006fe199f45ce9cc4359741cda47093fb36694afa`; immutable manifest `79cb663f73faae8c2e07c12a6ef42bf53aebfa4ad465a9310c93315e08311b5f`.
