## Unreleased (proposal only)

### Added
- `catalogue-v2.proposed.json` and `buckets-v2.proposed.json`: a **proposed**, unsigned
  registration of the 13 ids G-M1 opens (`kernel.checker.correct`,
  `kernel.logic.consistent`, `spec.goal.faithful`, `pif.reader.correct`,
  `pif.prelude.faithful`, `tcb.pif.gate-binding`, `tcb.pif.printer`,
  `gates.model.faithful`, `gates.model.code-link`, `warrant.model.elaboration-faithful`,
  `adl.prelude.faithful`, `adl.wir-abi.faithful`, `tcb.vcgen.claim-faithful`), with a
  proposed bucket for each (PIF-016a, ADL-006a). Requires the owner's signature on
  `buckets-v1.json` revision 2 before it can take effect.
- `PROPOSED-gm1-registration.md`: the diff summary, what the owner signs and the
  validation results.

### Unchanged
- `catalogue-v1.json`, `buckets-v1.json`, `evidence-kinds.json` and `non-claims.json` are
  byte-identical. No bucket moves. No report's catalogue pin changes.
