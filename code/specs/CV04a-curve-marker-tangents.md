# CV04a: endpoint directions for coincident curve controls

The independent review of candidate `324bf0f2` withheld publication: an ordinary
authored LR `bumpY` edge has controls equal to its distinct endpoints. TB
`bumpX` is symmetric. Passing the adjacent coincident control to the point
marker collapses all three triangle vertices. Aligned StepBefore start and
StepAfter end markers share this defect (with the axes reversed for TB).

Before implementation, add platform-independent parser -> hierarchy layout ->
paint tests using two rectangular nodes in one authored lane. Cover LR and TB,
BumpX/BumpY/Basis/Linear/Step/StepBefore/StepAfter, and both endpoint point
markers. Assert exact route coordinates/control points, metadata, marker tips,
the expected directed triangle vertices and area 50. Preserve current mixed-axis
controls and operation/resource bounds. Run Bump and Step regressions against
the old production code and save actual failing receipts before repair.

For a cubic endpoint, select the first noncoincident control in endpoint-relative
order: first/second/end at the start, second/first/start at the end. For a step
endpoint, select the first noncoincident intermediate in forward/reverse order,
then the opposite endpoint. Keep authored path commands and metadata unchanged.
Use a bounded iterator over at most two intermediates; no new allocation,
dependency, numeric threshold, coordinate normalization or resource exemption.
If every candidate coincides, preserve the existing zero-length behavior rather
than inventing a direction. Add a direct zero-length compatibility control.

Run the complete affected diagram cone, strict all-target lint, documentation
shard reconstruction, lessons and whitespace checks. Retain compiler/shared and
production/native receipts only with exact source-object identity. Require a
fresh independent final-head security review and current-main proof before
publishing to the existing single PR #17288. The rejected Bump candidate and
its 754/32 execution receipt remain historical evidence only.

Full local Miri remains **DEFERRED/PENDING**, with frozen inputs/checkout/target
untouched. Hosted required checks and actual three-platform compiler execution
remain mandatory. No complete compiler parity/provenance claim is made.
