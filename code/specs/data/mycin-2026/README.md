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
- **M6** — warm pipeline: decompose prose → typed findings (dictionary-constrained,
  decompose-only) → `import`-linked case → differential at 0 answer-time calls.
- **M7** — rulebook self-consistency (`check`/IIS) + value-of-information
  (`uncertain {…}` "order-next").
- **M8** — the five proofs + FINDINGS.
