# CAS-write gate — findings (the adversarial reading earned trust before commit)

The meningitis rulebook ([`meningitis.adj`](meningitis.adj)) was submitted to the **CAS-write
gate** before being committed: for each of the 12 grounded clauses, **3 model-diverse adversarial
readers** (Opus + Sonnet + Haiku) judged — does the verbatim `byte_quote` *entail* the asserted
likelihood ratio (magnitude + direction)? — majority vote. The 2 ungrounded clauses (no byte_quote)
skip the vote and are admitted only at `inferred`. (`cas_write_gate.py` + `.workflow.js`;
verdicts in `../gate/verdicts.json`, full report in `../gate/gate_report.json`.)

## Result: 8 earned their declared tier, 6 flagged and downgraded to `inferred`

| clause | reader votes | gate |
|---|---|---|
| `csf_gram_stain(positive)` LR 85 | E/E/E | ✅ accepted (consensus) |
| `csf_neutrophilic_pleocytosis(high)` LR 15 | E/E/E | ✅ accepted (authoritative) |
| `csf_glucose(low)` LR 18 | E/E/E | ✅ accepted (authoritative) |
| `csf_protein(elevated)` LR 9.33 | E/E/E | ✅ accepted (empirical) |
| `csf_lactate(elevated)` LR 22.9 | E/E/E | ✅ accepted (authoritative) |
| `serum_procalcitonin(elevated)` LR 27.3 | E/E/E | ✅ accepted (authoritative) |
| `seizure(present)` LR 5.84 | E/E/E | ✅ accepted (empirical) |
| `viral csf_glucose(normal)` LR 4.9 | E/L/E | ✅ accepted (authoritative) |
| **`bacterial prior` 0.037** | L/L/E | ⚑ flagged → inferred (a prevalence, not an LR) |
| **`csf_culture(positive)` LR 271** | E/L/L | ⚑ flagged → inferred (271 is over-precise off spec 99.7%) |
| **`viral csf_lactate(normal)` LR 13.7** | L/E/L | ⚑ flagged → inferred (derived by inverting the bacterial test) |
| **`viral csf_lymphocytic_pleocytosis(high)` LR 5.0** | (ungrounded) | ⚑ inferred (no byte_quote) |
| **`viral enteroviral_pcr(positive)` LR 50** | (ungrounded) | ⚑ inferred (no byte_quote) |

## Why this is the gate working, not a failure

The readers **accepted every clause whose quote states the LR (or the sensitivity/specificity that
computes it) directly**, and **flagged exactly the weaker-grounded ones**: the prior (the quote
gives a *prevalence*, not a likelihood ratio), the culture LR=271 (a point estimate hyper-sensitive
to the 99.7% specificity — the corpus note itself warns it ranges 90→∞), and the viral LRs that are
either derived by inverting a bacterial test or not yet byte-grounded. This is precisely the trust
mechanism MYCIN-2026 needs: **what enters the CAS has earned trust at write time**, so warm reuse is
trust-free.

Crucially, a flagged clause is **downgraded, not deleted** — the rulebook stays runnable in full
(the engine still has its prior), but the proof DAG's `trust` tier now tells a reviewer which links
are solid and which to verify. The flagged clauses are the cold-path follow-up: re-ground the
culture LR with a stability range, byte-ground the two viral findings, and treat the prior as a
prevalence rather than an LR. The content-addressed library `cas/objects/286c17aaf48ff32d.json`
records the gate verdicts as provenance.
