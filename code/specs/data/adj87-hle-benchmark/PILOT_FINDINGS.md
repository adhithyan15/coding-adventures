# ADJ87 HLE pilot — findings (2 real cais/hle items, 6 arms)

`{Haiku, Opus} × {plain, framework-closed-book, framework+spider+CAS}`, blind defensibility
adjudicator + accuracy vs gold. Items: Israel "good faith" statute → *Sale Law*; BAR flying
T1 stun unit → *Shuriken*. The canonical run is **de-contaminated** (see §4); the first run was
invalidated by a context bug, kept as `hle_pilot_results_v1_contaminated.json`.

## Scorecard (de-contaminated run — defensibility 0–1 / accuracy)
| arm | good-faith | BAR |
|---|---|---|
| **fwSpider:opus** | **0.88 / correct** | **0.82 / correct** |
| fwClosed:opus | 0.80 / correct | 0.92 / incorrect |
| fwClosed:haiku | 0.68 / partial | 0.66 / incorrect |
| fwSpider:haiku | 0.50 / partial | 0.70 / incorrect |
| plain:opus | **0.22 / correct** | **0.12 / incorrect** |
| plain:haiku | 0.08 / incorrect | 0.60 / incorrect |

## 1. Thesis (a): framework-Haiku reaches — and exceeds — plain-Opus DEFENSIBILITY
| item | plain-Opus | fw-Haiku (closed / spider) |
|---|---|---|
| good-faith | 0.22 | **0.68 / 0.50** |
| BAR | 0.12 | **0.66 / 0.70** |
On both items, **framework-assisted Haiku is MORE defensible than bare Opus.** The framework
lifts the weak model's auditability above the bare frontier model — because it forces
traceability/flagging, while bare-Opus asserts or fabricates with none.

## 2. Thesis (b): Opus+framework+spider beats plain-Opus
`fwSpider:opus` is correct on both items (the ONLY arm correct on the hard BAR question) and
highly defensible (0.88, 0.82). It found the official BAR wiki and quoted it verbatim
("Cortex's Shuriken EMP drone … can stun enemy units"). **plain-Opus** got good-faith right but
indefensibly (0.22, bare assertion) and got BAR **wrong** — thrashing between four fabricated
unit names ("Blitz"/"Vanguard"/"Lancet"/…), the least-defensible arm (0.12). So
Opus+framework+spider dominates plain-Opus on the conjunction (accuracy + defensibility).

## 3. The honest boundary + the recurring lesson
- **Defensibility ≠ accuracy.** Framework-Haiku is *more defensible* than plain-Opus but *less
  accurate* (Haiku partial/incorrect; Opus correct). The framework makes the weak model's work
  auditable and honest — it **flags or refuses rather than fabricates**. ("Defensible &
  auditable, not always correct.")
- **Bare frontier model is the least defensible thing in the matrix** (plain-Opus 0.22 & 0.12) —
  it knows things but cites nothing and hallucinates confidently.
- **Closed-book framework is high-variance.** This run `fwClosed:opus` was MOST defensible on
  BAR (0.92) by honestly hedging "specific unit name uncertain" — but INCORRECT (never named
  Shuriken). In the v1 run it *fabricated* "Pixie" and was least defensible. So the closed-book
  arm's defensibility depends entirely on whether the model honestly flags recall vs overclaims
  it — only the spider's verified citations make grounding reliable.
- **Grounding discipline caught a bad source:** `fwSpider:haiku` noticed one of its facts came
  from Zero-K (a different RTS) and refused rather than use it — a defensible refusal.

## 4. The v1 bug we fixed (why a re-run was needed)
v1: all three Haiku arms read the HLE question as a query about the **local codebase**
("cannot be answered from the coding-adventures repository"), grepping the repo and citing
`file://…/README.md`. The `general-purpose` workflow agents inherit this repo's CLAUDE.md + cwd.
Fix: a context-neutralizer prepended to every prompt ("general-knowledge question, NOT about any
repo, don't touch local files"), spider restricted to web-only, decomposition capped at 4 facts.
Result: Haiku answers the actual question, and the run dropped from ~19 min to **3.3 min**.

## Next
- The 6-arm pilot now gives a valid Haiku-vs-Opus read; both theses hold on N=2.
- To scale: more items (now that the HF token + parquet loader work), keep the spider bounded,
  and weight knowledge-recall-hard items (where the spider earns its keep).
