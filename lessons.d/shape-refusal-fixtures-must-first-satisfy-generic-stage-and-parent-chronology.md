---
category: Testing & coverage
---

# Shape refusal fixtures must first satisfy generic stage and parent chronology

Independent D3a test review caught two preconditions before implementation:
clearing pass_order after mutating one input left another input's inventory event
undeclared, and reparenting an early piece to a later input violated compact
parent chronology. Both would fail generic import before exercising source shape.
Recompute the exact surviving event-source census and reparent a later piece to
an earlier wrong input. Every shape cell must explicitly pass checked generic
import before asserting a shape refusal. A backward-coordinate control must
actually decrease from a positive start rather than leave a zero start unchanged.

The later private compiler transaction fixture confused checks_only (a separate
boolean) with the five-level enum. Its ChecksOnly variant caused E0599 before
tests ran. The actual fifth level is TranspileOnly. Preserve that compile-failed
receipt separately and require a compiling successor across all five real levels;
neither the failure nor a substituted mode is runtime coverage.
