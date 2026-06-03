# ADJ51 — byte-recursive provenance experiments

Two end-to-end runs of the framework on real published clinical cases,
demonstrating the byte-accounting contracts at the ingestion and
rulebook layers, with sub-agent-driven generic pipelines.

Spec: [`code/specs/ADJ51-byte-recursive-provenance.md`](../../ADJ51-byte-recursive-provenance.md).

## Layout

```
adj51/
├── Cargo.toml              — runner crate manifest (standalone)
├── README.md               — this file
├── src/
│   └── main.rs             — domain-agnostic runner (cargo run --bin adj51)
├── experiment/             — PMC12750962 (cardiology stress test)
│   ├── 00-ground-truth.txt
│   ├── 01-case-prose.txt
│   ├── 02-ingestion.json   — ingester output (byte-accounted)
│   ├── 03-derived-rulebook.adj          — deriver output (LR magnitudes)
│   ├── 03a-derived-rulebook-as-logits.adj — deriver's original signed-logit form (pre-conversion)
│   ├── 04-vignette.adj     — generated observe + query block
│   ├── 05-run-output.txt   — captured engine output
│   ├── validate_ingestion.py  — structural validator for ingestion JSON
│   ├── build_vignette.py      — JSON → adj-lang vignette generator
│   └── convert_logits_to_lr.py — converts signed logits to multiplicative LRs
└── experiment2/            — PMC12914605 (lymphoma masquerading as infection)
    ├── 00-ground-truth.txt
    ├── 01-case-prose.txt   — sanitised prose (no final-diagnosis or expert-recommendation leaks)
    ├── 02-ingestion.json
    ├── 03-derived-rulebook.adj           — derived under the rulebook byte-contract
    ├── 03-derived-rulebook.adj.pairs.json — rationale/clause pairs extracted by validator
    ├── 04-vignette.adj
    ├── 05-run-output.txt
    └── validate_rulebook.py  — structural validator for derived rulebook
```

## Reproducing

### Engine run (fast — < 1 second)

```bash
cd code/specs/data/adj51
cargo run --bin adj51                       # defaults to experiment/
ADJ51_DIR=experiment2 cargo run --bin adj51 # experiment 2
```

The runner reads `<dir>/03-derived-rulebook.adj` + `<dir>/04-vignette.adj`,
compiles via adj-lang, runs each query through `logic-engine`'s LR
aggregator, and prints per-query posteriors with fired clauses and
coverage reports.

### Re-running the sub-agents (slow — ~10 minutes per case)

The subagent prompts that produced each `02-ingestion.json` and
`03-derived-rulebook.adj` are recorded in
[`code/specs/ADJ51-byte-recursive-provenance.md`](../../ADJ51-byte-recursive-provenance.md).
A future milestone industrialises this as a single command per case.

### Validating an ingestion JSON

```bash
python3 experiment/validate_ingestion.py \
  experiment/01-case-prose.txt \
  experiment/02-ingestion.json
```

Confirms 100% byte coverage with no overlaps and no gaps; every
extracted disposition references a real observation; every
observation's source span is contained in some extracted disposition.

### Validating a derived rulebook

```bash
python3 experiment2/validate_rulebook.py \
  experiment2/03-derived-rulebook.adj
```

Confirms every clause is immediately preceded by a rationale block.
Emits a sidecar `<rulebook>.pairs.json` for downstream semantic
verification.

## Results at a glance

### Experiment 1 — PMC12750962 (cardiology)

| Query | Posterior | Decision | Ground truth |
|---|---:|---|---|
| `diagnosis(acute_coronary_syndrome)` | 3.8% | DISCHARGE | ❌ |
| `diagnosis(stable_angina)` | ~100% | ABOVE | ✓ |
| `disposition(admit_for_further_cardiac_workup)` | ~100% | **ADMIT** | ✓ |

Q1 was wrong because the troponin rule fired uniformly across the
ACS umbrella when it should have been scoped to MI sub-types (UA is
troponin-negative). This is the failure mode that drove the
rulebook byte-accounting contract added in experiment 2.

### Experiment 2 — PMC12914605 (lymphoma masquerading as infection)

| Query | Posterior | Decision | Ground truth |
|---|---:|---|---|
| `diagnosis(underlying_pulmonary_pathology)` | 82.4% | ABOVE | ✓ |
| **`diagnosis(pulmonary_malignancy)`** | **100.0%** | **ABOVE** | ✓ **true diagnosis** |
| `diagnosis(pulmonary_actinomycosis)` | 10.5% | BELOW | ✓ (red herring rejected) |
| **`next_diagnostic_step(biopsy_or_advanced_workup)`** | **99.9%** | **ABOVE** | ✓ **correct disposition** |
| `diagnosis(pulmonary_tuberculosis)` | 35.0% | (marginal) | ~ shared imaging pattern |
| `diagnosis(hematologic_malignancy_myeloid)` | 99.4% | ABOVE | ~ paraneoplastic vs. primary needs immunophenotype (excluded from sanitised prose) |

The framework got the load-bearing decisions right — malignancy
high, actinomyces red herring rejected, biopsy required — while
honestly tagging two language limitations the deriver could not
encode (self-attributed weight loss; Actinomyces as
colonizer-vs-pathogen).

## Notes

- The runner is a standalone Cargo crate (not a workspace member);
  it has its own `Cargo.lock`. `target/` is gitignored. Build with
  `cargo build --bin adj51` from this directory.
- The rulebook in `experiment/` was emitted by the deriver as signed
  logits per the prompt's magnitude calibration table; adj-lang's
  `contributes` surface syntax expects multiplicative LRs. The
  conversion is mechanical: `LR = exp(signed_logit)`, applied by
  `convert_logits_to_lr.py`. Experiment 2's deriver prompt was
  corrected to use LRs directly.
- The cases were sourced via WebFetch from PMC; prose for experiment
  2 was sanitised to remove any byte that named the final diagnosis
  or named expert recommendations, forcing the framework to derive
  the answer from raw clinical observations alone.
