# CV04a — bounded Swimlane spacing

Actual source0389f753 accepted nodeSpacing1e308 and produced LR extent(420,inf).
The failed control/log remain immutable. Finite input alone was an insufficient
admission rule. Adopt an explicit production range0..=65536 for each optional
flowchart spacing field, shared through an own diagram-ir constant. This is a
deliberate bounded-input policy, not a claim of unrestricted upstream parity.
No new crates.io dependency or relaxation of existing resource bounds.

Preserve old negative/nonnumeric/nonfinite refusal messages/locations. Finite
values above65536 are rejected at(1,1) with
`Swimlane flowchart FIELD must not exceed 65536`. Absent/default/zero/ordinary
positive values preserve complete existing IR/layout/PaintScene controls.

Add try_layout_swimlane returning an own typed error for caller-constructed
invalid spacing, before constructing geometry. The existing layout_swimlane
signature stays; it requires supported spacing and valid derived geometry and
panics explicitly on invalid input, via the checked API. It must never return
infinite/negative geometry. After construction, check finite/nonnegative canvas
and rectangle dimensions, finite rectangle positions and edge endpoint
coordinates; unsupported derived geometry returns a distinct error. This does
not claim validation of arbitrary style/paint payloads or cyclic hand-built IR.

Large supported padding can otherwise subtract more than the entire lane
extent. Expand the horizontal minimum canvas width to at least twice padding,
and vertical lane extent to at least padding; preserve existing larger computed
extent and all coordinate/order/default rules. Zero-width/height lanes remain
permitted, consistently with existing zero-spacing semantics. This narrowly
repairs nonnegative dimensions; it does not claim node containment inside lanes.

Extend the original finite safety control, retaining its geometry assertions:
pin exact parsed refusals for1e308/65537, direct-IR typed refusals for negative,
nonfinite and above-bound values across all directions/fields, and accepted
boundary0/65536. Verify finite/nonnegative canvas, all lanes/groups/nodes and
finite edge endpoints, plus full ordinary/default scenes. Pin large-padding
minimum extents separately. Preserve complete old work/classifier controls.

Full suite atb9ff49f6 exposed an existing self-loop defect: the exact ordering
reference control already documented NaN endpoints from a zero displacement
vector. The new checked boundary correctly refused it. Repair coincident-center
endpoint construction before normalization: source right-side midpoint and
target top-side midpoint, using each node's dimensions. Other endpoints remain
bit-identical. This pins finite distinct anchors, not a claim of full loop-path
topology. Keep the complete ordering/reference equality and every endpoint-bit
assertion; replace obsolete NaN normalization with mandatory finite assertions.
Add exact independent self-loop anchor expectations in all four directions,
with ordering/hops variations. Preserve the original failing suite receipt.

Run full affected diagram suites, complete updated reverse cone and standalone
transport benchmark, strict lint/docs plus independent exact-source review.
Prior source-attributed transport scoped acceptance does not accept this repair.
Miri DEFERRED/PENDING; frozen run unchanged. Whole456/general P2/hosted native3OS/
exact-head CI/conflict/full Closure parity and transformation lineage remain
OPEN. No delivery before applicable repairs and review acceptance.
