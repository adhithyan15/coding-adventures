---
category: Rust
---

# Distinguish precharge refusal from unpublished graph recording failure

Independent D1 review caught an overbroad test-contract sentence implying every
producer failure leaves the graph unchanged. Precharge and non-Lex capture
admission reject before a root is recorded; a later generic graph-node limit
can reject a child after its root exists internally. Both must preserve all
preexisting output files. State the phase distinction explicitly and test the
appropriate boundary; do not imply transactional graph rollback or weaken the
existing output-sentinel checks to reconcile the wording.
