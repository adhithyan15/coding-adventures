# Unreleased — admit node padding before floating layout arithmetic

Include imported Swimlane flowchart padding in the existing finite inclusive
0..=65536 admission guard. The fallible public layout API now returns typed
UnsupportedSpacing for unsupported padding before a NaN clamp can panic.
Supported padding layout semantics and limits remain unchanged.

Twenty direct invalid cases cover NaN, infinities, negative and cap+epsilon
values in all four directions. Their original executed receipt fails20/20,
including four NaN panics; it remains preserved. Accepted default/zero/cap
controls retain finite layout and caller input. Final affected suites and
independent security review remain required. Miri DEFERRED/PENDING.
