---
category: Testing & coverage
---

# Native stdout evidence must also prove successful termination and actual execution

Independent review found that the shared stdout conformance helper ignores exit
status. It supports programs with intentional return values, so changing it
globally would alter earlier contracts. Scope the new proof to default-zero
programs, require Some(0) plus exact stdout, and require at least one executed
backend per program while keeping the unavailable-backend guard. Name an ALGOL
temporary saved rather than result, whose binding controls main's return.
Reject matched stdout with nonzero or absent exit in adversarial helper tests;
simulated results are not native evidence. Log actual statuses only at real run
calls, preserving original source receipts and review findings. Patch contexts
must match complete lines, and the lesson helper's returned slug is authoritative
because long titles are truncated in filenames.
