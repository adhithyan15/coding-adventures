---
category: Testing & coverage
---

# A source token in a provenance log does not prove a transformed value reaches it

The CLOC27 golden trace test compiled `report("abc".length)` to `report(3)`
and checked that the lexer origin at `1:8` existed anywhere in the sidecar.
It passed even though the fold read the member node's absent identity and
produced a replacement without a parent link. Token presence proved lexing,
not transformation lineage. The old CCR-065 audit also described the leaf
plumbing as missing even though it had already shipped.

CLOC31 starts from the replacement's own fold contribution and follows
`parent_ids` to exact operand origins. Binary folds must reach both inputs;
nested folds must retain their intermediate rewrite; unrelated source tokens
must not be reachable. These stronger tests failed on the unchanged compiler
before the repair and passed afterward.

Read live implementation and test assertions before copying an issue's audit.
For provenance, test connected graph relationships rather than inventory,
counts, or the existence of roots. Keep emitted-byte parity as a separate
assertion so tracing cannot quietly change compiler behavior.
