# CV04a — checked Swimlane rank arithmetic

Selected after sourcecf1674c7 bounded spacing/transport acceptance. Independent
review identified inherited unchecked rank+distance and max_rank+1 on caller IR.
This is static, unexecuted debt: no observed overflow or UB is claimed. The
bounded spacing checked API currently covers previously supported structural/
numeric IR and is not certified total for arbitrary caller ranks.

Before changing arithmetic, execute and preserve a bounded direct-IR red control
with actual enabled cross-lane ranks and usize::MAX minimum link length. Include
an accumulation overflow path and rank-count overflow path, LR/RL/TB/BT plus
crossing optimization enabled/disabled. Preserve existing maximum-length unranked
controls and every ordering/gap/resource assertion.

Use checked integer addition for rank propagation, optimizer candidates and
rank count. Propagate an own typed rank-overflow refusal through the checked
layout API before geometric arithmetic; legacy wrapper keeps explicit refusal.
No saturation, truncation, float conversion replacing checked rank arithmetic,
or weakened parser8192 limits. Normal ranks, all four directions, node/edge
identity and complete scene/layout/work counters remain unchanged.

Run affected hierarchy/paint/parser suites, reverse consumers and strict lint,
then independent exact-source security review. This spec is selection only;
tests/implementation/acceptance are pending. Current sourcecf remains attributed
to its immutable receipts; later source changes require fresh affected evidence.
Latest audited/fetched main5ff75a40 adds only25 producer curriculum paths and no
Rust source change; its integration remains pending, without curriculum accuracy
or TypeScript execution claims. One PR17288, autoOFF; no publication/merge.

General floating P2/whole456/Unix tcp-reactor/hosted native3OS/exact-head required
CI/conflicts/full Closure parity/provenance remain OPEN. Miri DEFERRED/PENDING,
frozen run/inputs/checkout/target/receipts untouched. Full objective incomplete;
heartbeat ACTIVE. Deliver implementation before genuine final-source Miri.
