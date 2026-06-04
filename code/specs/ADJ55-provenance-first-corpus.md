# ADJ55 — Provenance-First Corpus Construction (MYCIN-2026)

> **Status (2026-06-04):** Proven end-to-end on pulmonary embolism. The grounded
> PE corpus (12/12 links traced to primary data) lives at
> [`code/specs/data/adj52/corpus/pulmonary_embolism/`](data/adj52/corpus/pulmonary_embolism/);
> the construction + evaluation tooling at
> [`code/specs/data/adj52/provenance/`](data/adj52/provenance/). Builds on
> [ADJ54](ADJ54-calibration-regression-harness.md) (which showed the framework's
> losses are calibration, not correctness) and [ADJ51](ADJ51-byte-recursive-provenance.md)
> (byte-recursive provenance).

## 1. The goal — a modern MYCIN

MYCIN (1970s) was an expert system: a hand-curated rulebook + a deterministic
inference engine. Its fatal limit was knowledge acquisition — experts hand-encoding
rules. ADJ55 reconstructs MYCIN on a 2020s footing by splitting the system into three
parts with a hard boundary between the fallible and the auditable:

1. **LLM as a natural-language frontend.** The model reads free text (a case, a paper)
   and maps it onto a controlled vocabulary. It ingests and routes; it does **not**
   invent the reasoning weights.
2. **A framework that enforces byte provenance.** Every quantity in the knowledge base
   — a prior, a likelihood ratio — must trace, through a recursive citation crawl, to a
   literal span of primary data (a published sensitivity/specificity, odds ratio, or
   prevalence). A number that cannot be grounded does not enter as a number; it is an
   explicit data-gap.
3. **A deterministic, executable program.** Given the grounded corpus + a case's
   observations, the verdict is a pure function — a sequential Bayesian update that
   produces the same posterior every run, every multiplier annotated by its source. No
   model in the loop at decision time.

The boundary is the point: the LLM (creative, fallible) is confined to the frontend;
the arithmetic (reproducible, auditable) is the program; and the bridge between them —
the corpus — is held to the byte-provenance invariant.

## 2. The invariant — byte provenance is the antidote to hallucination

> Every magnitude must point at a datum. If it cannot, it is not a number.

This is the ADJ02 "every byte represented" rule pushed down one level: not just every
*source byte* must be represented in the IR, but every *number in the knowledge base*
must point at a primary-data byte. The motivation is empirical. In ADJ54's case-5
(urology), the deciding clause — a likelihood ratio that reinterpreted the
cancer-defining findings as "reactive" and argued *against* the true diagnosis — was a
**fabricated magnitude with an authoritative-looking citation**. The whole posterior
(0.9924 for the wrong answer) was manufactured from invented numbers. A confident,
cited, quantified trail for a *wrong* answer is worse than no trail: it is the apparatus
of defensibility deployed in service of an error, and it is exactly what talks an expert
out of a correct instinct.

Byte provenance removes the failure mode at the root: the deriver cannot write a number
it cannot ground.

## 3. The construction — a forward spider, like a crawler

The corpus is **not** built by inventing a rulebook and auditing it afterward. It is
built *forwards*, the way a search engine grows its index: start from a finding, crawl
to the primary study, byte-anchor the sensitivity/specificity/OR, compute the LR from
that data, and admit the node only if it terminates in real numbers. A link that comes
back `direction_only` (source supports the direction but states no usable quantity) or
`fabricated` (no support, or a non-existent citation) is recorded as a data-gap, not a
number.

- **Forward grounding spider:** [`provenance/pe/ground.workflow.js`](data/adj52/provenance/pe/ground.workflow.js)
  — one agent per `finding → diagnosis` link, recursing through citations to primary
  data, computing `LR+ = sens/(1-spec)` etc., byte-anchoring each hop.
- **Assembler:** [`provenance/pe/build_corpus.py`](data/adj52/provenance/pe/build_corpus.py)
  → the canonical `corpus/<domain>/corpus.json`.
- **Deterministic evaluator:** [`provenance/pe/eval_case.py`](data/adj52/provenance/pe/eval_case.py)
  — a sequential Bayesian update; `grounded` mode trusts only `grounded` LRs and abstains
  on data-gaps (no invented push).

## 4. The proof — pulmonary embolism, end to end

Phase 1 (corpus, **case-blind**): 12 PE discriminators grounded → **12/12 traced to
primary data** (Christopher study for prevalence; Blood Advances 2020 / Cochrane for
D-dimer; PIOPED II for CTPA; Wells-item meta-analyses). Contrast: the same spider run on
case-5 urology grounded **0/19** — byte-provenance *discriminates* a genuinely-evidenced
domain from one where confident numbers would be hallucinated.

Phase 2–4 (a real case, blind): PMC11999957 — a 55-year-old with chest pain, ECG
ST-elevation mimicking ACS, **Wells score 0** (low pretest), elevated D-dimer. Ground
truth: **PE was present** (the pretest rule said "unlikely," the clot was real).

| arm | pretest P(PE) | after CTPA | outcome |
|---|---|---|---|
| **Grounded corpus** (every LR → a study) | **0.28** ("can't exclude, image") | **0.89** confirmed | **correct** |
| **Ungrounded deriver** (invented LRs, self-disclosed) | **0.01** ("PE excluded") | — | **missed a real PE** |

The case turns on one grounded number: D-dimer's true `LR+ = 1.64` (a *weak* positive,
because it is a rule-out test). That weak-positive, on the grounded 0.192 base rate,
lands at 28% — above any exclusion threshold, mandating the CTPA that found the clot. The
ungrounded deriver gave Wells-0 an invented strong rule-out and hallucinated a negative
CTPA, compounding three made-up numbers into a confident, fatal exclusion. **Byte
provenance is the variable that flips one to the other.**

## 5. What this is and isn't (honest scope)

- The grounded vs ungrounded contrast isolates *grounding the numbers* — but the arms did
  not get perfectly matched inputs (the ungrounded arm also misread the prose). Both are
  failure modes the provenance-first pipeline structurally prevents.
- **Plain frontier Claude was tested (the third arm) — and it was wrong too.** Given the
  same case with no framework, plain Claude put PE at **3–5%** and was "comfortable not
  doing CTPA" on a patient who had a PE — the same Wells-0/D-dimer-distractor trap the
  ungrounded deriver fell into. Only the grounded corpus (0.28 → image → 0.89) caught it.
  So on this case the framework's edge was **correctness, not merely auditability**: the
  grounded base rate (0.192, vs Claude's ~3–5% gestalt) and mechanical LR application kept
  the real PE on the table where unconstrained reasoning let a better-fitting narrative
  exclude it. Full three-arm writeup + verbatim plain-Claude run:
  [`provenance/pe/arms/`](data/adj52/provenance/pe/arms/three-arm-comparison.md).
  **Caveat: n = 1** — an existence proof, not a rate; the rule-out and high-Wells cases are
  still owed (see §6).
- The corpus grounds *present*-finding LRs; LR-for-absence is not yet grounded (it is why
  the corpus correctly does not over-weight Wells-0, but a fuller corpus would ground both).

## 6. Next

- **Extend the three-arm comparison to n > 1** — a true rule-out (low Wells + *negative*
  D-dimer) and a high-Wells confirm — to learn whether PMC11999957 flattered the framework
  or the pattern holds. (The first three-arm run is done; see §5.)
- LR-for-absence grounding.
- A second grounded domain, to show the construction generalizes.
- Wire the deterministic evaluator into the adj52 engine path so a grounded `corpus.json`
  can be executed as an adj-lang program (closing the loop to part 3 of the MYCIN-2026
  architecture).
