# ADJ68 — Open-Book Defensibility Audit (verifiability, not recall)

> **Status (2026-06-04):** Built and run. Corrects the methodological error of ADJ67: the
> framework is **open-book, always** — it spiders and grounds the knowledge a question
> needs (or retrieves it from the CAS), and it targets **auditable, defensible work, not
> recall.** So this run is scored on a different axis: an adversarial auditor tries to
> *fault* each answer's chain, scoring **verifiability**, not whether the answer is right.
> Result: the open-book spider grounded the very fact a closed-book model could only assert
> from memory, and a correct-but-uncited recall answer is shown to be **indefensible**.
> Implementation: [`pipeline/audit.workflow.js`](data/adj57/pipeline/audit.workflow.js) +
> [`pipeline/run_audit.py`](data/adj57/pipeline/run_audit.py).

## 1. Why this experiment exists

ADJ67 tested the framework **closed-book** and scored **final-answer correctness** — a
recall test with the framework's defining capability (retrieval) switched off. That is the
one axis the framework is built to lose: the world's knowledge already lives in a frontier
model's weights, so a bare model out-recalls a reasoning scaffold every time. The framework
must never even *derive* a known, citable fact — it must **fetch and cite it**. Its product
is not "got the answer"; it is **a chain an expert can audit and not fault.**

So ADJ68 runs **open-book** and scores **defensibility**, on the same chemistry question
(the Nicolaou endiandric-acid cascade) that ADJ67 round 2 ran closed-book.

## 2. The pipeline

[`audit.workflow.js`](data/adj57/pipeline/audit.workflow.js):
1. **Ground (spider, open-book):** an agent fetches the facts the question needs, each in a
   **verbatim source passage** (CAS-style) — the cascade *order*, the Woodward–Hoffmann
   rule, the Diels–Alder = [4+2] definition.
2. **Chain vs bare:** the framework builds the answer as a **defensible chain** (every node
   cites a grounded fact, a cited rule, arithmetic, or a prior node — no unsupported leaps);
   in parallel a **bare closed-book recall** agent answers from memory.
3. **Audit:** an **adversarial auditor** scrutinises every claim in each answer, marking it
   *verifiable* (cited source / stated rule / arithmetic) or *unsupported*, and returns a
   verdict — **PASS only if a reader can verify every link.** Correctness is explicitly not
   rewarded.

## 3. The run

```
GROUNDED (spidered open-book — the CAS):
  [G1] cascade ORDER: 8π closes first, then 6π   <- PMC2766600 (verbatim: "conrotatory 8π …
       electrocyclization … disrotatory 6π … electrocyclization … intramolecular Diels–Alder")
  [G2] Woodward–Hoffmann thermal rule (4n→con, 4n+2→dis)  <- Wikipedia (verbatim)
  [G3] Diels–Alder = [4+2]                                 <- Wikipedia/Cycloaddition

DEFENSIBILITY AUDIT (both answers were CORRECT):
  Arm A  bare recall:     3/9 claims verifiable (33%)  — FAIL, indefensible (zero citations)
  Arm B  grounded chain:  6/7 claims verifiable (86%)  — FAIL, one citation mismatch
```

Three things this settles:

1. **Open-book, the spider grounded the *order* — the exact fact ADJ67-closed-book got
   tangled deriving.** The PMC source states it verbatim ("conrotatory 8π … then disrotatory
   6π"). This answers the open question from ADJ67: **byte provenance *can* catch the
   established order — it was never a grounding failure, only an artifact of running
   closed-book.** The framework should never have tried to derive it.

2. **A correct answer can be indefensible.** Arm A reached the right answer but the auditor
   faulted **6 of 9 claims** — the synthesis identity, every step's structure, and the
   Woodward–Hoffmann rule itself were asserted from memory with **zero citations**. Correct,
   and *un-auditable.* This is precisely the gap a correctness-only metric (like ADJ67's) is
   blind to.

3. **The audit has teeth — it faulted the framework too.** The grounded chain was *not*
   rubber-stamped: the auditor caught **node 6**, where the cited Cycloaddition page confirms
   "Diels–Alder = [4+2]" but does **not** state the appended "diene = 4 atoms / dienophile =
   2 atoms" sub-claim — a **citation that doesn't fully support its claim.** One unsupported
   link ⇒ FAIL under the strict rule.

## 4. The honest read

- **Both got a binary FAIL** (PASS requires *every* link to verify), but the defensibility
  *scores* are night and day: **33% vs 86% verifiable.** The metric discriminates exactly as
  intended — and shows a correct bare answer as the indefensible one.
- **The framework's remaining gap is citation *precision*** — the cited source must support
  the *full* claim, not just its headline (node 6 over-reached its Wikipedia citation). That
  is the **three-axis weight/claim verifier** flagged earlier: a citation must back the
  claim's sign, magnitude, *and* its specific content. ADJ68 makes the need concrete.
- This is the **right axis.** The framework's value is not out-recalling a model; it is
  producing work where an expert can follow every link to a source or a rule. Plain recall —
  even when correct — cannot offer that, and the audit makes the difference measurable.

## 5. Next

- **Citation-precision verifier:** for each chain node, check the cited passage actually
  establishes the *whole* claim (would have turned Arm B's 6/7 into 7/7). The three-axis
  verifier, applied to chain nodes.
- **The screening harness, re-scoped to defensibility:** run many questions open-book,
  scoring the **defensibility fraction** (verifiable links / total) for bare-recall vs the
  grounded chain — the real number, on the axis that matters, with an error bar.
