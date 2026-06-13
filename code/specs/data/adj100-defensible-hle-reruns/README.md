# ADJ100 - Defensible HLE reruns

Goal: re-run two 10-item HLE pilots with the clarified target that the framework should produce
**defensible and auditable work**, not necessarily a large raw-accuracy win.

The framework is evaluated as a parent-enforced acceptance pipeline:

- blind baseline: answer from the raw question only;
- framework proposal: decompose input bytes into typed IR, propose sources, and write programs;
- parent verifier: store input/source/program bytes in CAS, fetch and check source quote bytes, execute
  programs, and decide whether an answer is strict-accepted;
- gold scoring: compare only after answers and acceptance decisions are fixed.

## Artifacts

- `FINDINGS.md` - headline conclusions and item-level texture.
- `PROTOCOL.md` - the stricter acceptance rule used for the reruns.
- `run1_audit.json` - first 10-item rerun, items 1-10 from the frozen ADJ99 HLE set.
- `run2_audit.json` - second 10-item rerun, items 11-20 from the frozen ADJ99 HLE set.

The large temporary CAS/source/program directories were intentionally not committed. These summaries
preserve the run outcomes, acceptance decisions, and notable provenance failures while keeping the PR
small enough to review.
