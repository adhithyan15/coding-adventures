# MYCIN-2026 — grounded, content-addressed reasoning libraries

This directory is the MYCIN-2026 rebuild on the byte-provenance + constraint
substrate. The thesis (see [SPEC.md](SPEC.md)): a rulebook is **derived once**
(cold path, with the model + an adversarial gate), grounded in primary sources
byte-by-byte, written in `adj-lang` as a **library**, and then reused on many
cases at **zero answer-time model calls** (warm path, the model only decomposes
prose into typed findings).

## The CAS is a registry of importable libraries

The key architectural move (M4–M5): **every entry in the content-addressed store
(CAS) is a runnable `adj-lang` library** — not a passive JSON blob — and
libraries compose by `import` (the M3 mechanism). The dependency graph:

```
        meningitis.adj            ← composed rulebook (the differential)
        /            \
bacterial-arm.adj   viral-arm.adj  ← grounded evidence arms (one rulebook each)
        \            /
       meningitis-vocab.adj        ← the controlled vocabulary (one dictionary)
                                      imported once, deduped
```

A case file then `import`s the composed rulebook and adds only `observe` lines:

```adj
import "meningitis.adj"
observe csf_gram_stain(positive)
observe csf_glucose(low)
% the rulebook's ? queries make it a differential; the case adds no diagnosis.
```

Because each library is content-addressed by the `sha256` of its source, a case
that cites a rulebook cites an **immutable** object; editing the rulebook
produces a new hash, and every citing case re-derives against the new object at
0 model calls (the golden-rulebook + cost-to-correct proofs, M8).

### `lib/` (authored source) → `cas/` (content-addressed)

- **`lib/`** — the human-authored libraries (this is where a clinician edits).
  - `meningitis-vocab.adj` — the dictionary (closed vocabulary; M1 `dictionary`/`define`).
  - `bacterial-arm.adj` — bacterial evidence (`rulebook` + `use`; grounded LRs).
  - `viral-arm.adj` — viral/aseptic evidence.
  - `meningitis.adj` — composes the arms + declares the `?` differential.
- **`grounding/`** — the spider's output: for every clause, the primary-source
  URL, a **verbatim byte-quote**, the numbers, the computed LR, and an
  independent re-extraction check (`grounding-results.json`).
- **`cas/`** — the content-addressed store, built by `cas_build.py`:
  - `cas/objects/<hash>.adj` — each library, with its `import`s rewritten to
    **dependency hashes** (Merkle-style: editing the vocab changes its hash,
    which changes every arm, which changes the composed rulebook).
  - `cas/objects/<hash>.json` — the manifest: kind, dependency hashes, and the
    write-gate verdict per clause (accepted-at-trust-tier vs flagged→`inferred`).
  - `cas/registry.json` — the index: `name → hash`, the root object, the graph.
- **`cases/`** — (M6) clinical vignettes + their decomposed `.adj`.
- **`proofs/`** — (M8) the five end-to-end proofs.

## The spider (cold-path grounding)

`grounding/grounding-results.json` is produced by a live-web spider: one agent
per clause searches for the primary source, reads it, copies a **verbatim**
span that states the sensitivity/specificity/ratio, computes the LR
(`LR+ = sens / (1 - spec)`), and a second, independent agent re-extracts the
same passage to check the quote survives re-reading and that the numbers
actually *entail* the claimed magnitude (not just the direction). A clause is
`grounded` only if both the extraction and the independent re-extraction agree
and the magnitude is entailed; otherwise it is `direction_only` /
`magnitude_leap` and the M5 gate flags it (kept runnable at trust `inferred`,
never silently dropped).

> **Honest limit on "byte"-stability.** With `WebFetch` (which returns
> model-summarized markdown, not raw bytes) the strongest available check is
> *independent re-extraction agreement*, not a literal byte-diff against fetched
> HTML. The SPEC's "byte-stability" is realized here as two-reader re-extraction
> stability; a true byte-diff would require a raw-fetch tool and is noted as a
> follow-up.

## Deliberate naïvety (for the cost-to-correct proof)

`bacterial-arm.adj` encodes the four CSF-chemistry findings (neutrophilic
pleocytosis, low glucose, high protein, high lactate) as **independent**
`contributes`. They are not independent — they are joint effects of one
inflammatory process — so multiplying their LRs over-saturates the posterior
(4 correlated CSF findings alone → P(bacterial) ≈ 0.9995 *pre-culture*, which is
indefensible). This over-count is left in **on purpose**: M8's cost-to-correct
proof localizes it from the proof DAG / IIS and fixes it with a single
explaining-away `interacts` clause, then shows the fix propagates to every
citing case at 0 model calls.

## Building & checking the CAS

```sh
python3 cas_build.py          # lib/ + grounding/ → cas/objects/<hash>.{adj,json} + registry.json
python3 cas_build.py --check  # CI: assert the committed CAS matches a fresh build
python3 test_cas.py           # determinism + a case imports objects/<root>.adj and decides
```

The **write gate** in `cas_build.py` decides, per clause, ACCEPT (keep the
declared trust tier) vs FLAG (downgrade to `inferred`, but **never delete** — a
flagged clause stays runnable, so dropping a prior can't break the rulebook). A
clause is accepted iff the spider's quote survived independent re-extraction
**and** the rulebook's LR matches the magnitude the source's numbers entail
(`computed_lr`) — i.e. the rulebook was calibrated to the evidence. Of 14
clauses, 12 accept; 2 flag: `csf_culture` (271 is definitional, not
study-anchored) and `csf_neutrophilic_pleocytosis` (15 is a conservative value;
the source supports a higher LR at extreme thresholds). The gate also consumes an
optional independent N-reader refute vote (`gate/votes.json`) when present.

## Roadmap (M4 → M8)

- **M4** ✓ — grounded vocab + arm libraries; the spider grounds every LR.
- **M5** ✓ — the CAS: content-address each library (Merkle hashes) + manifest;
  the write gate accepts-at-trust-tier vs flags-and-downgrades per clause.
- **M6** ✓ — the warm path: decompose prose → typed findings → diagnosis at **0
  answer-time model calls** (see below).
- **M7** ✓ — value-of-information ("order-next") + rulebook self-consistency:
  - `warm/voi.py <case>` ranks the **unobserved** findings by how much observing
    each would move the differential (pure CPU, re-decides). On the knife's-edge
    pre-culture case it flags that `csf_lactate(normal)` / `enteroviral_pcr(positive)`
    would *flip* the diagnosis — i.e. the highest-value tests to order next, each
    citing the rulebook clause that would fire.
  - `consistency/*.adj` encode a rulebook invariant (the two priors must partition
    to 1.0) as `constrain`/`check`. The real priors → SAT; a mis-authored prior
    → UNSAT with an **IIS `core`** naming the exact conflicting clauses —
    machine-checked "these rules contradict", the basis of error localization.
- **M8** ✓ — the five proofs + [FINDINGS](proofs/FINDINGS.md):

```sh
python3 proofs/test_proofs.py          # all five proofs + VOI/IIS (CI entry point)
python3 proofs/cost_to_correct.py      # headline: 1 clause edit, 0.9995→0.7752, propagates
python3 proofs/golden_and_cpu.py       # derive-once/0-calls + CPU-bound (~26 ms/case)
python3 proofs/audit_trail.py <case>   # verdict → clause → verbatim byte-quote + URL
```

  1. **golden-rulebook** — cases decided twice, identical, 4/4 correct, 0 calls.
  2. **cost-to-correct** — the naïve correlated-CSF over-count (0.9995) localizes
     from the proof DAG; **one** explaining-away `interacts` clause calibrates it
     to 0.7752 and propagates to every case at 0 model calls. (Honest sub-finding:
     calibration drops bacterial below the aseptic base rate — pre-culture you
     treat on cost, not probability.)
  3. **auditable** — every contribution cites its source + byte-quote.
  4. **error-localizable** — a wrong verdict is one clause (proof DAG + IIS).
  5. **CPU-bound** — ~26 ms/case; the reasoning is the engine, not a model.

## Warm path — `warm/` (decompose once, decide on the CPU)

```sh
python3 warm/dict_export.py    # lib/meningitis-vocab.adj → warm/dictionary.json (one source of truth)
python3 warm/decompose.py      # the ONE model touchpoint: Ollama llama3.1:8b, prose → ir/<id>.json
python3 warm/run_warm.py       # DETERMINISTIC: ir_to_adj → decide (imports the CAS rulebook); scores
python3 warm/test_warm.py      # CI: the committed IRs re-decide at 0 answer-time model calls
```

The model runs **once per case** (`decompose.py`), constrained to the dictionary,
and writes *only* typed findings (`ir/<id>.json`) — it never names a diagnosis.
Everything after is CPU: `ir_to_adj.py` turns the IR into `observe` lines
(dropping denied findings, adversarial-LEAP inferences, and — the closed-vocab
gate — any hallucinated functor/value, all recorded), and `decide.py` links the
case to the content-addressed rulebook by `import "objects/<root>.adj"` and runs
`adj-lang-cli` for the differential + proof DAG.

Result on the 4 vignettes: **4/4 correct, answer-time model calls = 0**. The
bacterial case dropped 2 small-model hallucinations at the vocabulary gate —
they never reached the engine. Re-running `run_warm.py` reproduces every
diagnosis with no model in the loop (the golden-rulebook property, proved in M8).
- **M6** — warm pipeline: decompose prose → typed findings (dictionary-constrained,
  decompose-only) → `import`-linked case → differential at 0 answer-time calls.
- **M7** — rulebook self-consistency (`check`/IIS) + value-of-information
  (`uncertain {…}` "order-next").
- **M8** — the five proofs + FINDINGS.
