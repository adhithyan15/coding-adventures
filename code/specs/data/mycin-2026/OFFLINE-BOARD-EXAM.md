# OFFLINE-BOARD-EXAM — pass the boards with zero online model calls

## The claim

MYCIN-2026 answers board-exam questions **without ever calling an online model.** The
only model anywhere on the path is a **local, in-memory** one, and it does exactly one
thing: turn a prose question into a typed ADJ query. It never answers the medical
question. The **native adj-lang engine** answers — deterministically, on the CPU, over
the grounded knowledge graph, with a citation for every answer.

This is the operational form of the north star ([[project_board_exam_goal]]) under the
constraint the user set: *"pass all the board exams without having to make a live
[online] model call; it can use a local in-memory model for typed-IR and ADJ program
generation."* The reasoning lives in the framework (the grounded edges + the engine),
not in the weights ([[project_dumber_models_constrained_envs]],
[[project_total_coverage_forces_reasoning]]). The model is a translator on a short
leash; the engine is the reasoner.

## The pipeline

```
prose board stem
   │   LOCAL model (decompose_query.decompose)            ← the ONLY model call, on-device
   ▼
{relation, subject, $Var}                                 ← a typed ADJ recall query
   │   = an ADJ program:  import "<domain>-edges.adj" … ? relation(subject, $Var)
   │   native adj-lang-cli  (board_eval.resolve_recall)   ← the CPU reasoner answers
   ▼
binding + citing edge (source + locator + trust)   OR   honest abstention
   │   scored: correct / abstained / wrong  (wrong = the only real failure)
   ▼
offline-scorecard.json     —  online_calls: 0  (ENFORCED by a network-egress guard)
```

Everything from the query down is the same one engine that runs the differential and
the constraint solver. There is **no second resolver** — the Python `recall.py`
`RelationStore` prototype is deprecated and off the answer path (this PR's first half).

## What makes it safe with a weak local model (and the honest limit)

The model's output is **constrained**: the relation must be one of the 11 legal recall
relations, and the subject a canonical entity the grounded graph knows. That buys ONE
guarantee — **no fabrication**: every answer the engine returns cites a real grounded
edge, and an *off-vocabulary* subject finds no edge and **abstains**.

It does **not** make decomposition errors free. If the model mis-maps the prose to a
*different but valid* entity — "Von Gierke disease" → subject `fabry` — the engine
faithfully answers the wrong question and returns a confident **wrong** answer. The
live 4B demo did exactly this on ~5/27 stems before the gate (grounding kills
*fabrication*, not *misdirection*).

The fix is a **two-sided decomposition-faithfulness gate** — byte-provenance applied to
the query, the same principle the whole framework rests on:

- **subject gate** (`decompose_query.attested_in_stem`): the chosen entity must be
  named by the stem. Rejects "Von Gierke disease" → subject `fabry`.
- **relation gate** (`decompose_query.relation_attested_in_stem`): the stem's
  interrogative must ASK for what the relation answers (cues derived from the relation
  name + its conventional variable — `has_mcv`+`Class` → `{mcv, class}` — no new
  hand-authored knowledge, whole-word matched). Rejects a stem asking for the classic
  *finding* decomposed as `has_mcv(...)` — right subject, wrong question.

Either gate failing rejects the query, so a mis-decomposition (wrong entity OR wrong
question-type) becomes an **abstention** instead of a wrong answer. With both gates, a
weak model's errors show up as *more abstention*, not wrong answers — on the recorded
live runs **both Gemma-3-4B and Qwen-0.5B reach 0 wrong / 100% defensibility** (see
board/OFFLINE-DEMO.md). That is exactly what lets a small on-device model drive the
board safely: the floor is honest "UNKNOWN", not hallucination. The reasoning is in the
framework; the model is a translator on a short, *checked* leash.

## Proving "no online call" instead of promising it

`offline_guard.no_network()` monkeypatches the socket connect surface to raise
`OnlineCallError` on any non-loopback outbound connection, while permitting loopback
and `AF_UNIX` (a local model server or the adj-lang-cli subprocess is *offline*). The
board answer path runs inside it, so a single dialed-out byte crashes the run. It is a
tripwire for the common case (urllib / requests / http.client / MLX HTTP clients all
sit on the socket layer), not a kernel sandbox — an `os.system("curl …")` would slip
past — but for a pure-Python + local-subprocess + in-process-MLX path it is exactly the
right granularity, and it is generic enough to drop around any "no online call" claim.

## Two run modes

| mode | decompose source | determinism | use |
|---|---|---|---|
| **cached** (default) | each item's gold query in `free_text_board.json` | deterministic, no model, no net | the committed `offline-scorecard.json`; CI; proves engine + plumbing + guard |
| **`--model PATH`** | a real local MLX model decodes the prose | model-dependent | the live proof a small on-device model drives the board; scores decompose accuracy too |

A `--model` run additionally reports **decompose accuracy** — how often the local model
produced the gold query — separately from **end-to-end defensibility**. The two are
decoupled on purpose: the engine's correctness does not depend on the model being
right, only on it being *constrained*.

## Files

| file | role |
|---|---|
| `board/offline_guard.py` | `no_network()` egress tripwire + `proves_offline` decorator (generic, reusable) |
| `board/decompose_query.py` | prose → `{relation, subject, $Var}` via an injected `gen()`; legal-relation + canonical-subject vocabulary; lazy `local_gen()` MLX loader |
| `board/free_text_board.json` | 27 prose board stems across 5 domains (25 covered + 2 uncovered→abstain) with gold answer + gold query |
| `board/board_offline.py` | decompose → resolve via the native engine inside `no_network()` → score; emits `offline-scorecard.json`; gate = no wrong AND no online call |
| `board/run_offline_demo.py` | live demo driver: runs a real local model, writes `offline-demo-transcript-<tag>.json` |
| `board/test_board_offline.py` | pins the guard, the decomposer, and end-to-end scoring |

## Results (verified locally; see OFFLINE-DEMO.md for the live-model run)

Cached mode, native engine, network blocked:

```
correct 25 · abstained 2 · wrong 0  (of 27)
defensibility 100%  ·  grounded-coverage 100%  ·  ONLINE MODEL CALLS: 0
```

## Non-goals / honest limits

- The guard is a tripwire, not a sandbox (see above).
- The free-text bank names the entity in the stem (a recall/translation task); full
  vignette→differential→recall (inferring the disease from findings before the recall
  hop) is the reverse-direction case the same engine already supports via the
  differential tactic — wiring prose vignettes through it is the next slice.
- These tests are verified locally; `code/specs/data/` is not yet wired into CI (no
  BUILD file there). Wiring the board package into CI — with a dependency on
  adj-lang-cli so the engine is built first — is a clean follow-up.
