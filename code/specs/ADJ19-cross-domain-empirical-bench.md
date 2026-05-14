# ADJ19 — Cross-Domain Empirical Bench: methodology + prerequisites

## Overview

ADJ18 broadens the TSA bench to 8 declarations × 5 models × 3 Arm
A modes. ADJ19 takes the **same shape across two other domains**
(clinical triage and contract review) to test whether the
rulebook-injection flip pattern from ADJ15/17 is **TSA-specific or
a generalizable property of small-model + rulebook adjudication**.

The framework's headline claim — "shrink the model down, push
intelligence into the framework" — is only as strong as the
breadth of domains it works in. If the rulebook-injection flip
only works for TSA-style declarations (short, item-list, single
decision), that constrains the framework's deployment story. If it
generalizes to clinical case summaries and contract clauses, the
story is much stronger.

## Status: planning spec

ADJ19 is a **methodology + prerequisites** document. The full
cross-domain bench cannot run today because the clinical-demo and
contract-demo crates haven't reached parity with tsa-demo's v0.12
API. Specifically:

- Clinical demo (v0.1) — no rulebook injection, no priming mode.
- Contract demo (v0.2) — no rulebook injection, no priming mode.

ADJ19 documents the bench methodology AND lists the implementation
prerequisites so a later PR series can land the plumbing and then
run the bench.

## What we're measuring (mirrors ADJ18)

Per cell: verdict, truncation flag, latency, token usage, raw
output. Same parser as ADJ18's harness.

What changes per domain is **the declaration set** and **the
expected-verdict rationale** — both authored by hand from
authoritative reference material (TSA's published rules; UpToDate
or similar for clinical; AmJur or specific state statutes for
contract).

## Domain 1: clinical triage

The clinical-demo's existing canonical text:

> "Patient: shortness of breath, mild fever, no known drug allergy."

The Arm B IR encodes three facts including a **denied-polarity**
node ("no known drug allergy") — clinical is the domain where
denied polarity carries the most weight (negative findings change
treatment decisions). The bench panel below leans into that.

### Proposed declaration set (8 cases)

| ID | Declaration | Expected | Rationale |
|---|---|---|---|
| `mi-rule-out` | `"55yo male, chest pain radiating to left arm, sweating. PMH: hypertension."` | URGENT-EVAL | Classic ACS / MI presentation; rule out MI per AHA. |
| `viral-uri` | `"22yo female, sore throat, low fever 38.2°C, dry cough. No rash. Vitals normal."` | OUTPATIENT | Viral URI; supportive care per IDSA. |
| `meningitis` | `"19yo male, stiff neck, photophobia, fever 39.5°C, headache."` | URGENT-EVAL | Bacterial meningitis suspected; LP indicated per IDSA. |
| `stable-asthma` | `"30yo female, mild wheeze, peak flow 85% personal best, no accessory muscle use."` | OUTPATIENT | Mild asthma exacerbation; outpatient bronchodilator per GINA. |
| `severe-asthma` | `"30yo female, severe wheeze, peak flow 40% personal best, tripod position, RR 32."` | URGENT-EVAL | Severe asthma exacerbation; ED evaluation per GINA. |
| `dehydration-mild` | `"28yo female, 1 day diarrhea, alert, normal vitals, no orthostasis."` | OUTPATIENT | Mild gastroenteritis; oral rehydration per WHO. |
| `dehydration-severe` | `"3yo male, 3 days vomiting, lethargic, sunken eyes, no tears, capillary refill 4s."` | URGENT-EVAL | Severe pediatric dehydration; IV fluids per WHO. |
| `allergy-denied` | `"45yo female, contrast CT ordered, denies known iodine allergy."` | PROCEED-WITH-MONITORING | No known allergy ≠ no allergy; supports proceeding per ACR with standard monitoring. |

Expected verdicts use a 3-value space: `URGENT-EVAL` /
`OUTPATIENT` / `PROCEED-WITH-MONITORING`. The clinical-demo's
v0.1 verdict shape (free-form text) will need a structured
verdict set — see prerequisites below.

The `allergy-denied` case is the **denied-polarity stress test**:
the IR encodes "no known iodine allergy" as `Denied(iodine_allergy)`,
which propagates differently than a positive finding. The
expected verdict tests whether the LLM reasons correctly about
the absence of contraindication.

### Reference rulebook

For mode 2 / mode 3 (rulebook-injected), the system prompt
includes a fixture clinical rulebook with ~10 numbered rules:

```
1. Chest pain radiating to arm + sweating + cardiac history → urgent
   evaluation per AHA 2019 ACS guideline.
2. Stiff neck + photophobia + fever → urgent evaluation; LP
   indicated per IDSA bacterial meningitis criteria.
3. Severe asthma indicators (peak flow <50%, accessory muscle use,
   tripod) → urgent evaluation per GINA 2024.
4. Mild URI without red-flag features → outpatient supportive care
   per IDSA pharyngitis guideline.
... (etc.)
```

Same format as `fixture_tsa_rulebook()` — numbered rules,
authoritative-source citations.

## Domain 2: contract clause review

The contract-demo's existing canonical text:

> *"Vendor shall deliver goods within 30 days. Force majeure events
> (acts of God, war) extend the deadline by 14 days."*

This is the **Rule + Exception** shape — the second clause is a
typed `Exception` modifying the first, encoded via the IR's
`Modality::Conditional`. The bench panel leans into this.

### Proposed declaration set (8 cases)

| ID | Declaration | Expected | Rationale |
|---|---|---|---|
| `force-majeure-cycle` | `"Vendor failed to deliver within 30 days due to a hurricane (force majeure event)."` | NOT-IN-BREACH | Force majeure clause extends deadline 14 days. |
| `plain-breach` | `"Vendor failed to deliver within 30 days; no extenuating circumstances declared."` | IN-BREACH | No exception applies. |
| `ordinary-delay` | `"Vendor failed to deliver within 30 days, citing supplier delays."` | IN-BREACH | Supplier delays are not enumerated force-majeure events. |
| `war-event` | `"Vendor failed to deliver within 30 days due to civil war disrupting logistics."` | NOT-IN-BREACH | War is enumerated as a force-majeure event. |
| `delivery-on-time` | `"Vendor delivered goods on day 28 of 30-day window."` | NOT-IN-BREACH | Within contractual deadline; no breach. |
| `late-within-exception` | `"Hurricane on day 25; vendor delivered on day 42 (12 days late)."` | NOT-IN-BREACH | 30 + 14 = 44 days; 42 < 44 so within extended deadline. |
| `late-beyond-exception` | `"Hurricane on day 25; vendor delivered on day 48 (18 days late)."` | IN-BREACH | 48 > 44 so beyond even the extended deadline. |
| `non-enumerated-act-of-god` | `"Earthquake disrupted supply chain. Vendor delivered on day 50."` | DISPUTED | "Earthquake" not explicitly enumerated; depends on jurisdiction. |

Expected verdicts: `IN-BREACH` / `NOT-IN-BREACH` / `DISPUTED`. The
`DISPUTED` case is interesting because it exercises ADJ16 step 3's
`DisputedAnswer` machinery: with two rulebooks (one strict, one
lenient on "act of God"), the engine arm should surface the
dispute. The LLM arms should either pick a verdict (one or the
other) or hedge.

### Reference rulebook

```
1. Vendor must deliver within the contractually-specified window;
   failure constitutes breach.
2. Force majeure events (acts of God, war, natural disasters)
   extend the deadline by 14 days per the contract's force-majeure
   clause.
3. Supplier delays, market conditions, and pricing disputes are
   NOT force-majeure events absent specific contract language.
4. Hurricanes, floods, earthquakes are explicit acts of God in
   most U.S. jurisdictions per Restatement (Second) of Contracts
   §261.
... (etc.)
```

## Prerequisites for running ADJ19

Before the bench can run, the following changes need to land:

### Prerequisite 1: Bring clinical-demo to v0.12 parity

- Add `rulebook_text: Option<String>` to clinical-demo's
  `DemoConfig`.
- Add `max_answer_tokens` and `arm_a_mode: ArmAMode`
  (mirror tsa-demo's v0.12 fields).
- Implement two-turn priming dispatch.
- Add `fixture_clinical_rulebook()` returning the ~10-rule string.
- Plumb `ADJ_DEMO_RULEBOOK_MODE`, `ADJ_DEMO_MAX_ANSWER_TOKENS`,
  `ADJ_DEMO_ARM_A_MODE` env vars through `config_from_env`.

### Prerequisite 2: Bring contract-demo to v0.12 parity

Same changes as Prereq 1, with `fixture_contract_rulebook()`.

### Prerequisite 3: Generalize the bench harness

`scripts/adj18_bench.py` is currently TSA-specific (hardcoded
declarations). Generalise to:

- Domain-keyed declaration sets loaded from a config file.
- Per-domain binary paths (different demo crates produce
  different binaries).
- Same parser and JSON output shape.

Or alternatively keep three separate harness scripts
(`adj18_bench.py` for TSA, `adj19_clinical_bench.py`, etc.) for
clarity. Decision deferred to the implementation PR.

### Prerequisite 4: Verdict-set parity across domains

Different domains have different verdict spaces:

- TSA: `COMPLIANT` / `NON-COMPLIANT` (2-value)
- Clinical: `URGENT-EVAL` / `OUTPATIENT` / `PROCEED-WITH-MONITORING` (3-value)
- Contract: `IN-BREACH` / `NOT-IN-BREACH` / `DISPUTED` (3-value)

The verdict-first system prompt and the verdict regex in the
harness both need to accept the per-domain set. This is a small
plumbing change but affects every demo's prompt builder.

## Hypotheses (mirror ADJ18's, scoped per-domain)

- **H5**: Rulebook injection improves verdict accuracy in clinical
  triage. If the model has the urgent-evaluation criteria in its
  system prompt, it should flip dangerous-presentation cases from
  the no-rulebook baseline (which might say "see your PCP") to
  URGENT-EVAL.
- **H6**: Rulebook injection improves verdict accuracy in contract
  review. If the model has the force-majeure clause in its system
  prompt, it should flip the `force-majeure-cycle` and `war-event`
  cases from a default "vendor is in breach" to NOT-IN-BREACH.
- **H7**: The `allergy-denied` clinical case stress-tests
  denied-polarity reasoning. The model must reason that "no known
  drug allergy" supports proceeding (a negative finding that
  *removes* a contraindication, not a positive finding).
- **H8**: The `DISPUTED` contract case exercises rulebook conflict.
  In the engine arm (Arm C, when it's available), this should
  surface as `disputed_answers.len() >= 1` per ADJ16 step 3. In
  the LLM arms, the model either hedges or picks one verdict; the
  bench captures which.
- **H9**: The flip rate scaling pattern from ADJ17 (small models
  benefit more from rulebook injection than larger models)
  generalizes across domains. If true at 3B for TSA, it should
  hold at 3B for clinical and contract too.

## What this gives us once it runs

A 3-domain × 5-model × 3-mode × 8-declaration matrix = 360 cells.
Roughly 8-12 hours of bench wallclock on commodity hardware
(longer if Ollama swaps models between calls).

The data file format mirrors ADJ18's: one record per cell with
verdict, truncation, latency, raw output. Analysis tooling can
reuse ADJ18's parser.

The output is the framework's **most important empirical
artifact to date**: cross-domain evidence that the
rulebook-injection pattern generalizes. If it holds, that's the
deployment story for the framework — write a rulebook once
(human-reviewed), ship to small-model deployments, get auditable
verdicts. If it doesn't hold uniformly, the bench tells us
*which* domains break and *how*, which informs where the
framework needs work.

## Sequencing

1. **ADJ19 spec lands** (this PR).
2. **Prereq PRs land** (clinical-demo v0.3, contract-demo v0.3 —
   each adds rulebook injection + priming + new env vars).
3. **Harness generalization** (one PR).
4. **Bench data PR** (run the bench, commit the JSON, append the
   empirical results section to this spec).

Total: roughly 4-5 PRs of plumbing before the bench runs. Each
plumbing PR is small (mirrors changes already proven in
tsa-demo v0.12); no architectural risk.

## See also

- [ADJ18](ADJ18-broadened-tsa-empirical-bench.md) — methodology
  precursor; this spec mirrors its structure across two more
  domains.
- [ADJ15](ADJ15-recursive-rulebook-empirical-results.md),
  [ADJ17](ADJ17-adversarial-rulebook-empirical-results.md) —
  the n=1 results ADJ18 and ADJ19 are designed to generalize.
- [ADJ16](ADJ16-engine-programmatic-adjudication.md) — Arm C
  (engine arm) is currently TSA-only via the
  `tsa_rulebook_strict_ir()` / `tsa_rulebook_lenient_ir()`
  fixtures. Cross-domain Arm C requires per-domain fixture
  rulebook IRs and per-domain source IR shapes; the prereq PRs
  above set up the LLM arms first, and Arm C cross-domain follows
  once the prerequisite plumbing has settled.
