# ADJ57 — Byte-Provenance Pipeline (every byte accounted for, at every layer)

> **Status (2026-06-04):** Thin vertical slice proven end-to-end on a fresh case
> (pheochromocytoma, PMC11521393). The four layers — a content-addressed source
> store, a case→IR byte-coverage checker, fact-driven derivation, and a
> recursive-to-root grounding spider — run as one pipeline with the invariant
> enforced at each. Implementation: [`code/specs/data/adj57/`](data/adj57/).
> Converges [ADJ02](ADJ02-coverage-checker.md) (coverage),
> [ADJ55](ADJ55-provenance-first-corpus.md) (source grounding), and
> [ADJ51](ADJ51-byte-recursive-provenance.md) (indexed-source corpus).

## 1. The invariant (stated once, enforced everywhere)

> At **every layer**, every byte of input is either **represented in the reasoning**
> (with a retrievable span back to its source) or **explicitly discarded with a
> reason**. Nothing is silently dropped, and no number exists without bytes you can
> pull up.

This is ADJ02's total-coverage rule, generalized past the case-text layer to the
*source* layer — and it is what catches the failure modes the corpus work surfaced
(ADJ56): a fabricated magnitude has no bytes to point at; an out-of-population LR
discards its conditions-of-validity bytes without a reason.

## 2. The four layers

### L0 — the CAS (content-addressed source store) — [`pipeline/cas.py`](data/adj57/pipeline/cas.py)
Every source the spider reads is interned by `sha256(content)`. An entry holds the
raw content, the **byte spans cited from it** (each with what it was used for), and
its **onward citations** (the edges the spider follows toward a root source).
`cite()` *rejects any quote not literally present* — a citation must point at real
bytes, not a paraphrase. Content-addressing gives free deduplication.

**The keystone:** decomposition cost is paid **once per source, ever**. The first
case that needs PIOPED II interns it with byte-provenanced spans; every future case,
any domain, that cites it reuses the span for free. This is ADJ51's "indexed-source
corpus," made real, and the answer to "1000 cases overnight isn't reachable until
the corpus exists."

### L1 — case → IR with byte coverage — [`pipeline/coverage.py`](data/adj57/pipeline/coverage.py)
The case text is decomposed into an **ordered partition** of segments, each a typed
`fact` (its span = its own text, trivially retrievable) or a reasoned `discard`. The
checker verifies the segments **concatenate back to the exact input, byte for byte**
— total coverage by construction, no gaps, no overlaps, and (per the
no-byte-arithmetic rule) the model emits literal text in order while the framework
derives offsets. A `fact` must carry a typed term; a `discard` must carry a reason —
or the byte is neither represented nor reasoned-about.

### L2 — IR facts → rulebook links
The extracted facts determine which `finding → diagnosis` links the rulebook needs.
Derivation is a function of the facts, not free invention.

### L3 — rulebook → grounded magnitudes (recursive to a root source)
For each decisive link the spider fetches a source, byte-provenances the supporting
span, and — if that source *borrows* the number — follows the citation onward,
**recursively until a root source** states the primary measured number. Every source
fetched is interned into the CAS. A link that reaches root primary data is
`grounded`; one that finds only a direction is `direction_only`; one with no support
is `fabricated`.

## 3. The slice (proof) — PMC11521393, pheochromocytoma

**L1 — 100% byte coverage, with the enforce→correct loop demonstrated.** The
ingester's first partition collapsed a `\n\n` paragraph break to a space; the
checker **caught it at byte 1296**, localized it to segment 42; it was corrected and
re-verified clean. Final: **22 typed facts (57.5%) + 24 reasoned discards (42.5%) =
100% of 1499 bytes**, every fact typed, every discard reasoned.

**L3 + L0 — grounding traced to root, byte-anchored in the CAS:**

```
blood_pressure(160/100) → pheochromocytoma   LR = 0.762   [grounded, root reached]
  CAS 3590a06e…  "Does this patient have Pheochromocytoma? a systematic review" (PMC4815191)
  cited bytes [165,297):  "Hypertension had a positive LR of 0.762 (0.562–1.033) … based on 5 studies"
  verify: content[165:297] == quote  →  True
night_sweats(present)  → pheochromocytoma   LR = 2.184   [grounded, 2-hop chain to root]
weight_loss(5kg…)      → pheochromocytoma   LR = 0       [direction_only — no root data, so no number]
```

The thesis in one line: a likelihood ratio that **points at the exact bytes of a
primary source**, stored in a reusable CAS, retrievable forever — and where the data
doesn't reach root, the framework returns *no number* rather than an invented one.
(The grounded `LR = 0.762` for sustained hypertension is *weakly against* pheo — the
data, not a guess; an inventing deriver would have made it large.)

## 4. How this closes the open loops

- **ADJ56 population extrapolation:** "based on 5 studies of [population]" is *bytes
  in the CAS source*. The rulebook must carry them and the evaluator must enforce
  them — an out-of-population application discards a condition-of-validity byte
  *without a reason*, which the invariant forbids. The guard is now a natural L3+L2
  addition, not a bolt-on (next).
- **ADJ55 fabrication:** structurally impossible at L3 — a number with no root source
  is `direction_only`/`fabricated`, never grounded.

## 5. Next

- **Population guard:** record each grounded LR's source population (bytes already in
  the CAS) and have the evaluator flag/withhold out-of-population application (the
  ADJ56 §3.1 infant-strep loop).
- **Scale via CAS reuse:** re-run the corpus domains (PE/strep/meningitis) through
  this pipeline so their sources populate the CAS; measure the reuse rate as cases
  accrete.
- **Deterministic auto-repair** for whitespace-only coverage divergences (the common
  ingester slip), reserving the LLM re-prompt for substantive ones.
- Promote the IR partition + coverage into the broader adjudication-ir/coverage
  crates so the case layer shares one implementation with the engine path.
