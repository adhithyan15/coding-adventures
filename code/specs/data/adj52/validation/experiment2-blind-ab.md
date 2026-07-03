# ADJ52 validation — full blind A/B on experiment-2 (PMC12914605)

A complete blind cross-arm run on the ADJ51 experiment-2 case, with a
**known ground truth** (Primary Mediastinal Large B-cell Lymphoma —
a lymphoma, not an infection; Actinomyces was colonization; the myeloid
leukocytosis was a paraneoplastic leukemoid reaction; correct
disposition = tissue biopsy).

## Arms

- **Framework arm** — the `adj52` runner over the ADJ51 experiment-2
  rulebook: `diagnosis(pulmonary_malignancy)` ~100%,
  `diagnosis(pulmonary_actinomycosis)` ~10% (rejected),
  `next_diagnostic_step(biopsy_or_advanced_workup)` ~99.9%,
  `diagnosis(hematologic_malignancy_myeloid)` ~99.4%,
  `diagnosis(pulmonary_tuberculosis)` ~35%.
- **Plain-Claude arm** — a subagent given the same sanitised prose,
  answering as it normally would (`experiment2-plain-claude.json`).
- **Blind judge** — a subagent given the prose, the ground truth, and
  the two outputs as `OUTPUT A` / `OUTPUT B`, identities hidden, asked
  to score both and pick a winner (`experiment2-judge.json`).

**Keymap (held by the orchestrator, hidden from the judge):**
`A = plain Claude`, `B = framework`.

## Result

**Judge winner: B (framework), narrow margin.**

- Both arms got the load-bearing decisions right: malignancy (not
  infection), Actinomyces = colonizer, biopsy = next step.
- **Plain Claude (A) anchored on CML/AML/myeloid sarcoma** as the lead
  diagnosis — i.e. it fell for the paraneoplastic leukemoid reaction the
  ground truth flags as the trap; lymphoma was only "in the differential."
- **Framework (B) avoided the wrong specific commitment** (stayed at
  "thoracic malignancy"), which is why the judge gave it the edge.
- Neither named PMBCL specifically.

## Findings (this is the value of running the case)

1. **Methodology bug — the judge must receive the audit trail, not a
   summary.** The framework arm was rendered as a prose summary that
   *claimed* "each shift traces to a citation" without showing the fired
   clauses + sources. The judge correctly called this "false rigor … can't
   be traced," and **docked the framework on defensibility — its single
   biggest advantage.** The real runner output lists every fired clause
   with its actual citation (`+1.6094 imaging_chest_ct(right_upper_lobe_mass)
   src: Quekel LG et al…`). The orchestrator must feed the judge the
   actual audit trail, or the experiment measures the framework with its
   defining feature amputated.
2. **Real calibration issue — the engine over-collapses to ~100%.** A
   ~100% / ~99.9% posterior on a case that took weeks and a biopsy to
   resolve is overconfident. LR-aggregation producing extreme posteriors
   is a genuine finding: the framework should carry residual uncertainty
   here (an `uncertain`/kickback signal), not collapse to certainty. Ties
   directly to the "uncertainty at the core" design goal.
3. **Both arms elevated the myeloid red herring** (plain Claude as its
   lead; the framework at 99.4% on `hematologic_malignancy_myeloid`),
   confirming the leukemoid reaction is a shared trap — a rulebook that
   scoped the myeloid signal as paraneoplastic-secondary would help.

Net: an honest, narrow framework win on diagnostic direction, plus two
concrete fixes for the orchestrator and the engine calibration.
