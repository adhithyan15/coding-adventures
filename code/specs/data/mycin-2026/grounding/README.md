# Grounding harness — the cold-path on-ramp to the CAS

**Nothing in MYCIN-2026 is human-authored.** Every fact enters the content-addressed
store (CAS) only through the *cold path*: spidered from primary sources,
byte-provenanced (a verbatim quote + URL), adversarially re-read, gated, and only then
committed. Humans maintain the system **only by correcting wrong CAS entries** — never
by authoring. This directory is the reusable machinery for that.

## Pieces

| File | Role |
|---|---|
| `harness.py` | **G0 — the generic core.** The shared write-gate verdict (`gate`), the decomposed-**source object** model (`SourceObject`/`SourceClaim`, content-addressed by sha256), CAS read/write under `cas/sources/`, deterministic **citation verification** (`verify_citation`), and the system-wide **provenance ledger** (`build_ledger`). Domain-agnostic — every artifact's per-domain gate sits on top of this. |
| `test_harness.py` | Pure guard for the above: gate verdicts, source content-addressing (tamper-evident hash), and that a citation verifies **iff** its quote is genuinely in the decomposed source. |
| `ground_sources.py` | The recursive source driver. `--list` emits the worklist of sources the grounded facts cite; default mode commits the decomposed sources to the CAS and **verifies every citation against its source**. |
| `workflows/ground-organism-id.workflow.js` | The G1 spider: grounds the organism-identification priors + morphology from primary sources (ground → independent re-extract → adversarial verdict). |
| `workflows/decompose-source.workflow.js` | The recursive source spider: fetches each cited source and decomposes it into byte-provenanced claims (+ child-citation frontier). |
| `organism-id-grounding.json` | G1 spider output: 13 records (grounded / direction_only / refuted) feeding the organism-id write gate. |
| `source-objects.json` | decompose-source spider output: the decomposed cited sources (committed to `cas/sources/`). |

## No blind trust in citations (the recursion)

A grounded fact says *"S. pneumoniae ≈ 51% of episodes"* and cites a quote from a
source. But a quote can be cherry-picked or misread — the G1 run itself caught a
"community" prior whose quote was actually about *nosocomial* infection. So the cited
**source itself** is fetched, decomposed into byte-provenanced claims, and committed to
the CAS as a *source object*. The fact's citation is then **verified** against that
decomposed source: is the cited quote genuinely present? A citation whose quote isn't
in the decomposed source is flagged `UNVERIFIED` and surfaced for fix-up. Sources a
source itself cites become the recursion frontier (`cites`), so provenance is a Merkle
graph: `fact → source object → cited source object`.

`verify_citation` is the deterministic floor ("the quote is really in the source"); a
deeper "the quote *entails* the implication" check is an adversarial agent pass layered
on top, but a quote not even present fails here first. It is robust to the Markdown
emphasis and HTML entities a web fetch injects around the same bytes (`_S. pneumoniae_`,
`P&lt;0.001`). A citation may also be a **composite** stitched with an ellipsis
(`"A … B"`); each fragment is checked independently, and the result reports both
`verified` (every fragment present) and `core_verified` (at least the first,
load-bearing span present). An over-stuffed citation — a real proportion plus a bundled
context span the source decomposition didn't capture — is thus flagged `◑ core ✓
(over-reach)` for fix-up without falsely discrediting the fact's actual evidence.

It is robust to web-fetch byte-class artifacts that wrap the same prose: Markdown
emphasis (`_x_`), HTML entities (`&lt;`), and en/em-dash vs `--` in ranges (`9–23` ≡
`9--23`). What it deliberately does NOT paper over is a genuinely different *sentence*:
if a fact cites one verbatim span and the independent decomposition captured a
different (even semantically-equivalent) one, the deterministic floor reports it
unverified — that's an entailment-level match for the adversarial pass, not a verbatim
one.

**Status after G2 (organism-id priors + morphology + 14 host-factor LRs all grounded):
authored-debt 0.** Across the 27 organism-id citations: **17 fully verified, 2
core-verified (composite over-reach), 8 unverified** against the 25 decomposed sources
in `cas/sources/`. Of the **18 grounded (ACCEPT)** facts, 17 verbatim/core-verify; the
sole exception, `host_neonate_gnb`, is content-corroborated by its *same* source (Merck
independently lists *E. coli* among predominant neonatal pathogens) but cited a
different sentence than the decomposition captured — flagged for verbatim
re-decomposition. The 8 unverified are otherwise all `FLAG` facts
(direction_only/refuted) — exactly the citations the rulebook never trusts as
authoritative.

**G3 added a second ledger artifact — meningitis dosing** (`treatment/antibiotics/`).
Dose grounding is the hardest frontier: the canonical IDSA Tunkel Table 6 is an
unreadable image, so the adversarial verifier refused doses that were pediatric-only,
general-indication, or non-verbatim. Honest result: **2 grounded, 3 direction_only,
3 refuted (authored-debt)** across 8 drugs — the gate attaches that status per drug
(`dose_grounding` in the formulary manifest) and the dose-window numeric model stays a
structural feasibility band. `ground_sources.py` now emits one artifact per domain
(organism-id + dosing), each with its own grounded/flagged/authored-debt counts.

## Run it

```bash
# 1. emit the worklist of sources the grounded facts cite
python3 ground_sources.py --list            # -> source-list.json

# 2. decompose those sources (spider; resume on transient rate limits)
#    Workflow tool: workflows/decompose-source.workflow.js -> source-objects.json

# 3. commit source objects to the CAS + verify every citation + rebuild the ledger
python3 ground_sources.py                    # -> cas/sources/*.json, PROVENANCE-LEDGER.md

# guard
python3 test_harness.py
```

The provenance ledger (`../PROVENANCE-LEDGER.md`) tracks every fact as **grounded |
inferred-flagged | authored-debt** plus its source-verification status, so authoring
debt is visible and drives toward zero as the spider expands specialties and diseases.
