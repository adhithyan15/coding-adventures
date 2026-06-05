# ADJ67 — Does the Grounding Discipline Help? (blind HLE head-to-head + weak-model ladder)

> **Status (2026-06-04):** Two experiments, reported honestly — including where the
> framework *didn't* help. (1) Blind, closed-book, externally-verified head-to-head of
> plain Claude vs a grounding-discipline agent on real **Humanity's Last Exam** questions.
> (2) A weak-local-model **atomic-feed ladder** (Haiku → Gemma → 0.5B) showing what lifts a
> small model and where the floor is. Result: the framework is **not uniformly better** —
> it wins on one failure class, ties (loses on cleanliness) on another, and the atomic+
> grounded harness can reduce a hard HLE step to rule-applications a 9.6 GB local model
> performs correctly. Artifacts:
> [`pipeline/ollama_atomic.py`](data/adj57/pipeline/ollama_atomic.py),
> [`pipeline/ollama_hle_chem.py`](data/adj57/pipeline/ollama_hle_chem.py),
> [`pipeline/atomic_haiku.workflow.js`](data/adj57/pipeline/atomic_haiku.workflow.js),
> [`hle-headtohead.json`](data/adj57/hle-headtohead.json).

## 1. The blind HLE head-to-head

Each question was sourced from a public source with an **independently-verified** answer
(not the gated benchmark key) held aside. Two **blind, closed-book** Claude subagents per
question — **Arm A** plain one-shot; **Arm B** the grounding discipline (cover every
token/condition, ground each step, flag what it cannot ground, calibrated answer). This
isolates the *reasoning/grounding discipline itself* — no retrieval advantage.

### Round 1 — Classics (Palmyrene): **framework win**

Translate `RGYNᵓ BT ḤRY BR ᶜTᵓ ḤBL` (the Regina tombstone, RIB 1065). Truth: *"Regina, the
freedwoman of Barates, alas."* Grading rule rejects any answer that drops the
freedwoman-of-Barates relationship.

| | committed answer | score |
|---|---|---|
| **Arm A** (plain) | *"Regina, daughter of Ḥari, son of ʿAta. Alas!"* (mentioned the correct reading but elevated the literal as "strict") | **wrong** — drops freedwoman-of-Barates |
| **Arm B** (framework) | *"Regina, the freedwoman of Barates. Alas!"* — flagged `ḤRY` as the ungroundable token, ~60% | **correct** |

Both *knew* the inscription. Arm A's reasoning pattern-matched `BT … BR …` to the familiar
"daughter of… son of…" filiation formula and **committed the wrong literal**. Arm B's
**coverage rule** forced it to stop on `ḤRY` (the Ḥ-R-R "free/freed" root) and surface the
freedwoman reading — and to name the exact uncertainty. A **reasoning-faithfulness /
commitment** win, not a knowledge win.

### Round 2 — Chemistry (pericyclic cascade): **tie; plain was cleaner**

Nicolaou endiandric-acid cascade. Truth: `[8π]-con, [6π]-dis, [4+2]`.

| | committed answer | score |
|---|---|---|
| **Arm A** (plain) | `[8π]-con, [6π]-dis, [4+2]` | **correct** — clean, coherent, correct order |
| **Arm B** (framework) | `[8π]-con, [6π]-dis, [4+2]` | **correct** — but tangled on the step *order* mid-derivation, then reconciled; honestly flagged that order isn't W-H-derivable |

Here the framework's **derive-from-rules** discipline *over-formalized*: the Woodward–Hoffmann
rule gives con/dis from an electron count, but it does **not** give the cascade *order* (which
step is 8π vs 6π) — that needs holistic structural recall, which plain Claude just did. Arm B
got the right final answer but messier, while correctly localizing the genuinely
non-rule-derivable part.

**Honest tally across the two: 1 win, 1 tie (plain cleaner). The framework helps when the
failure is "pattern-match and drop a detail"; it can hurt when the answer needs holistic
recall the rules underdetermine.**

## 2. The weak-model arm — and a real HLE answer from a local model

Fed the **same chemistry question atomically + grounded** to a local ~Gemma-class model
([`ollama_hle_chem.py`](data/adj57/pipeline/ollama_hle_chem.py)): the framework's
decomposition supplied, per step, the π-electron count + the W-H rule (a stand-in for
spider/CAS grounding from the Nicolaou literature); Gemma did **only** the local
rule-application. Result: **`[8π]-con, [6π]-dis, [4+2]` — correct** — the answer
frontier-Opus-framework got *tangled* on.

> The step *order* — the structural fact Opus slipped on — is exactly what the grounded
> decomposition hands over. A weak model doing only rule-application **cannot** make that
> error. **Honest boundary:** the intelligence is in the framework's grounded decomposition,
> not the weak model; a fully autonomous result needs the spider to ground the counts/order
> from sources (documented, but not run for this item).

## 3. The atomic-feed ladder (and the floor)

On a marine-forensics trap case (gray-seal wounds; truth = propeller; trap = shark exposure
prior), with **blind controls**:

| model | blind (no framework) | atomic from memory | atomic + grounded |
|---|---|---|---|
| Haiku | propeller ✓ | propeller ✓ | — |
| Gemma | "gear or vessel" (not trapped, vague) | net ✗ (shark suppressed) | **propeller ✓** |
| Qwen 0.5B | **shark ✗ (trapped)** | degenerate | degenerate — **below floor** |

Findings:
- **Atomic decomposition (structural)** defeats the holistic exposure-prior trap even at
  0.5B — "sharks feed here" becomes one vote of seven.
- **Grounded per-atom criteria (knowledge)** fix a mid-size model's wrong domain knowledge
  (Gemma net→propeller). *Both* ingredients are needed; neither alone suffices for Gemma.
- There is a **capability floor**: a 0.5B model emits non-discriminating verdicts even with
  criteria, and the harness would otherwise **launder them into a confident wrong answer**.
  The **discrimination gate** ([`ollama_atomic.py`](data/adj57/pipeline/ollama_atomic.py))
  detects ~zero-variance verdicts and refuses — the analog of ADJ65's "margin rests on
  assumed weights," one level down.
- The aggregate is **robust to a single noisy atom** (errors stay independent and small),
  unlike one holistic leap.

## 4. Honest conclusions

- **n = 2** HLE questions — anecdote, not a measurement; both are public, likely-contaminated
  samples.
- The framework is **not uniformly better**. Its win is in the reasoning-faithfulness /
  commitment / calibration lane (and, open-book, the retrieval lane via the spider — not
  tested here). It does **not** create knowledge a closed-book model lacks, and it can
  over-formalize.
- The atomic+grounded harness genuinely lets a weak local model compose a correct hard-HLE
  answer — *because the framework supplies the grounded structure*, which is the thesis,
  stated honestly.
- A real number requires the **screening harness**: many HLE items, scoring committed-answer
  correctness **and** calibration, plain vs framework, ideally less-contaminated items. That
  is the next build.
