---
category: Testing & coverage
---

# A report that hardcodes its own evidence passes an assurance gate that only checks shape

`ca-assurance-report`'s `generate_ct_compare_report()` for `rust/ct-compare`
wrote every number as a literal: `property.cases: 10000`, `fuzz.iterations:
100000`, `has_pbt_coverage: true`, and the same `drat:sha256:e2a4c386892...`
certificate (a commit hash with a literal `...`) on all four claims. `--check`
passed it, because the checker only validates shape and internal consistency:
every claim has a "pass" status, the rungs are legal, the matrix is complete.
Nothing compared a claim with a test that actually ran. Evidence IDs such as
`ct-compare::tests::bench_instruction_counts::equal-retired-instructions`
matched no real test.

The audit that found it also found that the real evidence was thinner than the
report: the PBT "claim" test compared `a == b` with `a == b`; the generated PBT
harness was only ever string-matched, never compiled; random 0..32-byte pairs
almost never exercise the equal branch; `ct_eq` was proved only through its
equal-length helper on 4-byte buffers while the report said `proved`.

**Do instead:** a report generator may not contain an evidence number. Every
count, label, seed and certificate hash must be read from a manifest written by
the run that produced it, and `--check` must fail closed when an ID is absent
from the manifest, a count or label disagrees, a hash does not match a file on
disk, or a spec claim has no evidence. Then write a falsification test for each
of those failure modes. Also: a generated test is only evidence once the
generated file has been compiled and run against the real public API, with a
sabotage run proving it can fail; and a proof label must name the bound that
was actually discharged (`proved` only when unbounded).

