# MLE-PASS — Design Spec: Pass a Medical Licensing Exam on the ADJ/MYCIN-2026 Engine

**Status:** Design / specs-first. No source modified.
**Author:** evaluation-systems architecture pass, 2026-06-26.
**North star:** A USMLE-style single-best-answer vignette is *decomposed* (structure
only) by a small LOCAL model into an ADJ program importing the grounded MYCIN-2026
libraries plus one typed `ask`. The native adj-lang engine does **all** reasoning +
math on CPU, maps its output to ONE option letter (A–E), and emits a single
machine-checkable proof object. The LLM never computes an answer, a number, a
posterior, or a verdict.

All claims are cited `file:line` against the repo at
`/Users/adhithya/Downloads/coding-adventures`. **This spec composes with — does not
duplicate — `scratchpad/ADJ-REASON-MATH-DESIGN.md` (henceforth ARM).** ARM defines
the engine/language evolution (the deduction↔evidence bridge, the unified
`ReasoningTrace`, the CAS rewrite trail, the `ask` surface, the independent
re-checker). MLE-PASS defines the *exam harness, the question protocol, the
option-mapping layer, the scoring, the two-factor failure diagnostic, and the
gap-driven campaign* that sits **on top of** ARM and consumes it.

---

## 0. Executive correction (read first)

The harness is **further along than the framing implies.** Three load-bearing facts,
verified by reading source:

1. **A mature board-eval harness already exists and runs end-to-end today.**
   `code/specs/data/mycin-2026/board/board_eval.py` scores a 155-item bank into the
   THREE-outcome model `correct / abstained / wrong` (`board_eval.py:6`–`17`), gates
   on never-fabricate (`board_eval.py:377`–`378`: `wrong > 0 → exit 1`), and already
   reports the abstention-aware metrics this spec calls for —
   `defensibility = (correct+abstained)/total` (`board_eval.py:267`),
   `accuracy_on_attempted = correct/attempted` (`board_eval.py:269`), and
   `grounded_coverage` (`board_eval.py:272`). **Current committed scorecard:** 155
   items, **148 correct / 7 abstained / 0 wrong**, defensibility 1.0,
   accuracy-on-attempted 1.0, grounded-coverage 0.9928
   (`board/board-scorecard.json` summary).

2. **The decompose pipeline (prose → ADJ) exists, is offline-guarded, and is
   measured.** `board/board_offline.py` runs `prose → LOCAL model → typed ADJ query →
   native CLI → answer`, all inside a network-egress tripwire that makes "zero online
   calls" a crash-to-violate property (`board_offline.py:7`–`23`, `:67`–`79`,
   `:116`). The decomposer `board/decompose_query.py` is model-injectable
   (`decompose(stem, gen, vocab)`, `:46`–`49`), constrains the model to a **closed
   33-relation vocabulary** (`REL_VAR`, `:70`–`104`), and applies a **two-sided
   byte-provenance faithfulness gate** — a SUBJECT gate (entity must be named in the
   stem) and a RELATION gate (the interrogative must ask what the relation answers)
   (`decompose_query.py:31`–`44`). Measured: a 4B local model decomposes ~74% of
   stems correctly, and **every mis-decomposition degrades to an honest abstention,
   never a wrong answer** (`:42`–`44`). Offline scorecard: 27 items, 25/2/0, 0 online
   calls (`board/offline-scorecard.json`).

3. **BUT: today's harness answers a STRUCTURED question bank, not raw USMLE MCQs.**
   The bank (`board/items.json`, 155 items) is **145 recall + 2 differential + 8
   management** (verified by counting). Recall items are already typed
   `{relation, subject, var, gold}` (`items.json` schema) — the prose→query
   decomposition is the only NL step, and even that is pre-cached as gold queries in
   CI mode. **There is NO five-option MCQ layer, NO option-letter mapping, and NO
   distractor handling.** The "exam" today is "answer the question," not "pick the
   single best of five." That gap — turning a defensibility scoreboard into a
   *licensing-exam pass/fail* — is the core of MLE-PASS.

**The honest one-liner:** the engine + decomposer + offline guard + scoring +
three-outcome discipline all exist and work. What MLE-PASS adds is (a) a real
five-option MCQ format with distractors and an option-mapping layer, (b) a
contamination-controlled question source, (c) the two-factor failure diagnostic that
turns a score into a worklist, and (d) the multi-hop diagnostic path — which is
**blocked on ARM PR-1 (the deduction→evidence bridge)** for any vignette where the
answer-bearing premise must be *derived* rather than observed.

---

## Phase 1 — Inventory (cited)

### 1.1 The board-eval harness (REL-5 / board bank)

- **Location:** `code/specs/data/mycin-2026/board/board_eval.py`; bank
  `board/items.json`; committed scorecard `board/board-scorecard.json`; tests
  `board/test_board_eval.py`.
- **Question format accepted (today):** three tactics, three shapes:
  - `recall` — `{id, domain, tactic, relation, subject, var, gold}`
    (`items.json`, e.g. `tay_sachs_enzyme`: `deficient_in(tay_sachs,$Enzyme) →
    hexosaminidase_a`). Rendered to ONE ADJ program: import all edge files, then
    `? relation(subject, $Var)` per item (`board_eval.py:85`–`92`).
  - `differential` — `{program: cases/X.adj, gold}`; runs the CLI on the case file,
    reads the `decision` dict (`board_eval.py:149`–`183`).
  - `management` — `{chart: [[kind,value,span]...], gold: [drugs]|"INFEASIBLE"}`;
    compiles chart→constraint program, solves to min-cost regimen or proves
    INFEASIBLE (`board_eval.py:194`–`225`).
- **Scoring:** three outcomes (`board_eval.py:6`–`17`). Abstention is first-class
  (`abstained ≠ wrong`). Metrics: `defensibility`, `accuracy_on_attempted`,
  `grounded_coverage`, `grounded_correct` (`board_eval.py:245`–`274`).
  **Abstention metric: YES, present.**
- **End-to-end today: YES.** Requires the Rust CLI built at
  `code/packages/rust/target/debug/adj-lang-cli` (`board_eval.py:78`); if absent,
  every engine-backed item **abstains honestly** (no Python fallback,
  `board_eval.py:104`, `:156`, `:374`).
- **Gate:** a single `wrong` is a non-zero exit (`board_eval.py:377`). This is the
  never-fabricate hard gate.

### 1.2 The decompose pipeline (prose → typed ADJ)

- **Files:** `board/board_offline.py` (orchestrator), `board/decompose_query.py`
  (the decomposer), `board/offline_guard.py` (network tripwire),
  `board/free_text_board.json` (27 prose stems + gold query + gold answer),
  `board/run_offline_demo.py`, transcripts `offline-demo-transcript-*.json`.
- **Model:** any LOCAL `gen(prompt) -> str` (`decompose_query.py:46`); demo uses MLX
  models — recorded for **Gemma-3-4B** and **Qwen2.5-0.5B**
  (`offline-demo-transcript-gemma3-4b.json`,
  `offline-demo-transcript-qwen2.5-0.5b.json`).
- **Output:** a typed recall query `{relation, subject, var}` — i.e. an ADJ program
  `? relation(subject, $Var)` (`decompose_query.py:17`–`20`). **Structure only, no
  answer.**
- **Faithfulness gate:** two-sided byte-provenance — SUBJECT attested in stem +
  RELATION attested in stem (`decompose_query.py:31`–`44`). A mis-map → abstention,
  never a wrong answer. This is the existing realization of ARM §F's "decomposition
  contract"; MLE-PASS extends it to the MCQ setting (§2.4).
- **Wired to a bank: YES** — `free_text_board.json` (27 items), measured at
  decompose-accuracy 1.0 in cached mode (`offline-scorecard.json`). **But these are
  single-answer recall stems, NOT five-option MCQs.**

### 1.3 The knowledge libraries (recall edges + differential rulebooks)

- **Recall edge libraries:** `code/specs/data/mycin-2026/recall/*-edges.adj` —
  **124 `.adj` files** total in `recall/` (verified `ls | wc -l`); ~17 are the
  loaded `*-edges.adj` knowledge files (`board_eval.py:67`–`71`,
  `decompose_query.py:60`–`64`), the rest are paired `*-recall.query.adj` query
  programs. Each edge is a typed, byte-provenanced `relate rel(subj, obj)` with
  `source`/`locator`/`trust` (`recall/iem-edges.adj:30`–`45`). An ACCEPTed edge
  carries a grounded byte-quote + `trust authoritative` + URL; ungrounded edges keep
  `trust consensus` and a `% [FLAG]` so authored-debt is visible
  (`recall/iem-edges.adj:11`–`16`). **GENERATED from spider output — never
  hand-edited** (`iem-edges.adj:11`). This is the "studied-student notes": built
  from primary sources, never from the exam (the contamination guarantee, §2.2).
- **Differential rulebooks:** `code/specs/data/mycin-2026/lib/` — `meningitis.adj`,
  `meningitis-vocab.adj`, `bacterial-arm.adj`, `viral-arm.adj`. Plus per-case
  programs under `board/cases/` (`meningitis_bacterial.adj`,
  `meningitis_equivocal.adj`). A differential case is `prior … / contributes … /
  observe … / ? hyp` (`board/cases/meningitis_bacterial.adj:1`–`11`). **Coverage is
  thin** — exactly ONE differential domain (meningitis) is wired today (2 items).
- **Treatment/constraint libs:** `treatment/antibiotics/` (chart→COP engine,
  `board_eval.py:191`–`204`), `treatment/constraints/`.

### 1.4 The adj-lang-cli JSON output shape (how an ANSWER + proof are read)

The CLI prints ONE JSON object (`adj-lang-cli/src/main.rs:514`–`521`). Order of
computation: constraint **solve/check/optimize FIRST** (`main.rs:412`–`422`), each
outcome → a **status atom** injected as an observed `Fact` (`main.rs:424`–`427`),
THEN `decide(&lowered)` runs the differential (`main.rs:429`). Keys:

| Key | Shape | Used for |
|---|---|---|
| `queries` | `[echo strings]` | every declared query (ground + binding) |
| `ranked` | `[{hypothesis, posterior, posterior_logit, normalized_share, proof}]` | **most-likely-diagnosis / mechanism** answer + proof DAG (`main.rs:436`–`445`) |
| `decision` | `{type: determinate{leader,…} \| kickback{leader,runner_up,…} \| empty}` | the committed answer or abstention (`main.rs:447`–`472`) |
| `recall` | `[{query, abstained, answers:[{bindings, citations:[{trust,…}]}]}]` | **recall / fact-binding** answer + citing edge (`main.rs:478`–`486`) |
| `governing` | `[…]` | precedence-resolved view (ADJ73) |
| `solve` / `check` / `optimize` | solver outcomes (regimen/feasible/optimal + certificate) | **calculation / next-best-step** verdict (`main.rs:498`–`512`) |

**Answer read-out today:** recall → `recall[].answers[0].bindings[var]` +
`citations[0].trust` (`board_eval.py:139`–`145`); differential → `decision.leader`
when `type==determinate` (`board_eval.py:177`–`181`); management →
`result.regimen | "INFEASIBLE"` (`board_eval.py:219`–`225`). **MLE-PASS adds one
layer above this: map the engine read-out to ONE of the five option atoms (§2.3).**

### 1.5 The deduction↔evidence gap (the multi-hop blocker)

Confirmed by reading source: `logic-engine/src/lib.rs:933`–`955`,
`observed_evidence`. The gate filters
`f.probability == Probability::Certain && f.term == evidence_term` (`lib.rs:944`–
`947`) and returns `None` if no **directly asserted** Certain fact matches. The
doc-comment is explicit: *"only `Certain` Facts gate contributions. Probabilistic
Facts and Rule-derived evidence are deliberately not yet routed here … deferred to
v0.2"* (`lib.rs:937`–`942`). **Consequence:** an atom that is only *derivable* via
`rule { … }` cannot trigger a `contributes … from <atom> to <verdict>`. So a vignette
where "neutrophilic + low-glucose CSF ⇒ csf_suggestive ⇒ (weighs toward) bacterial
meningitis" cannot be expressed as ONE query until **ARM PR-1 (deduction→evidence
bridge)** lands. **This is the dependency for the multi-hop majority of USMLE.** MLE-
PASS treats ARM PR-1 as the gating unlock and structures the campaign around it (§3).

**Integration with ARM:** MLE-PASS does not re-spec the engine. It consumes ARM's
deliverables: PR-1 (bridge) → multi-hop diagnosis; PR-5 (`ask` surface) → the typed
answer-shape contract; PR-3/6/7 (exact compute, CAS wiring, evaluate-at) →
calculation items; PR-9 (`adj-verify` re-checker) → the machine-checkable audit
trail. Where ARM has not yet shipped a capability, MLE-PASS items that need it are
*classified* into bucket (c) "engine/language gap" (§2.5) rather than scored as
wrong — turning ARM into an exam-gap-driven worklist.

---

## Phase 2 — The MLE-PASS harness design

### 2.1 Question source & contamination

Three concrete options, each with legitimacy / contamination risk / required check:

| Option | What it is | Legitimacy | Contamination risk | Required check |
|---|---|---|---|---|
| **A. NBME "Free 120"** (official free practice items for USMLE Step 1/2) | Real, exam-authored single-best-answer items released publicly by NBME | High — the canonical "is it USMLE?" yardstick | **HIGH for trained LLMs** (public, in pretraining), but **LOW for our engine** because the *engine never sees the stem* and the *libraries are built from primary sources, not these items* | A *library-provenance audit*: assert no `*-edges.adj` `source`/`locator` byte-quote or URL derives from an NBME item (see §2.2 guardrail). Plus a manual spot-audit that no gold answer was copied into a library. |
| **B. A licensed/official practice bank the user supplies** (e.g. UWorld/NBME self-assessments under license) | High; closest to the real exam difficulty | Same as A; plus licensing constraints on committing items to the repo | Same guardrail as A + **do not commit item text** — store hashes/IDs only; keep items out of the CAS and out of any library-grounding run. |
| **C. Held-out SYNTHESIZED set from primary sources NOT used to build libraries** | Items we author from a *disjoint* set of primary sources (e.g. write items from First Aid pages / journal articles that were explicitly excluded from the spider's grounding corpus) | Medium — defensible *only if* the synthesis is blind to the library corpus and the difficulty is calibrated against A | **LOWEST** — we control provenance on both sides | A *source-disjointness proof*: the set of source URLs/DOIs used to write the items ∩ the set of `locator` URLs in the grounded libraries = ∅ (machine-checkable over the `*-edges.adj` `locator` fields). |

**Recommended default: C for the campaign loop + A as the periodic external
yardstick.** Rationale: C gives a *defensible number we fully control for
contamination* and can grow on demand to fill exam-gap buckets; A is run
periodically (un-tuned) as the "is this really USMLE-hard?" external check. The number
we *publish* is the C number with the source-disjointness proof attached; the A number
is reported as a calibration sanity check, not the headline, precisely because the
engine's open-book legitimacy (§2.2) is cleanest when we can prove the libraries and
the items came from disjoint primary sources.

**Protocol that makes the number defensible (all options):**
1. **Freeze libraries before items, by commit hash.** Record the
   `git rev-parse HEAD` of the libraries at the moment the item set is frozen; the
   item set must be authored/added *after* that hash and the libraries must not change
   until after scoring. (Mirrors the existing "generated, never hand-edited" edge
   discipline, `iem-edges.adj:11`.)
2. **Source-disjointness check** (option C) / **library-provenance audit** (A/B): a
   small checker walks every `*-edges.adj` `locator`/`source` and asserts no overlap
   with the item-source set. Ship it as `board/contamination_check.py`, gating CI.
3. **Blind decompose:** the decomposer sees only the stem + the five options + the
   closed vocab — never the gold answer (already true of `decompose_query.py`; extend
   to MCQ in §2.4).
4. **No item enters the CAS or any grounding run, ever** (option B hard requirement;
   good hygiene for all).

### 2.2 Open-book legitimacy (why this is a fair "studied-student" analog)

A human test-taker is *open-book on their own studied knowledge*: they walk in having
read primary sources (First Aid, Robbins, journal articles) and reason over that
internal library to pick the best option. They are **closed-book on the exam items
themselves** — they have not seen these specific vignettes. MLE-PASS reproduces
*exactly* this: the engine reasons over its **own grounded libraries built from
primary sources** (`recall/*-edges.adj`, byte-provenanced to NCBI/journal URLs,
`iem-edges.adj:30`–`45`) and is **closed-book on the items** (decompose sees the stem,
never the key; libraries frozen by hash *before* items exist, §2.1).

This is consistent with the standing principle that the framework is **open-book by
design and is scored on defensibility, not closed-book recall**
(`feedback_framework_openbook_reasoning_not_recall`). Passing a board on *grounded,
cited, re-checkable* reasoning over a studied library is precisely the legitimate
claim — and it is *stronger* than a raw LLM's, because every answer carries a proof.

**The guardrail that proves libraries weren't built from the test items:** the
source-disjointness / library-provenance check (§2.1 step 2), made a **blocking CI
gate**. Because every edge already carries a `locator` URL and a `source` byte-quote
(`iem-edges.adj:33`–`45`), disjointness is *machine-checkable*, not a promise. If any
library edge's provenance overlaps an item source, the run fails. This is the
exam-analog of "the student didn't get a copy of the answer key while studying."

### 2.3 Vignette → answer flow (the exact stages)

```
                                   five options A–E (atoms)
                                            │
  USMLE vignette (prose stem + 5 options)   │
        │                                    │
        ▼  STAGE 1: DECOMPOSE (local model, structure-only)
   ADJ program:  import "<libs>.adj" …       │   model emits ONLY:
                 <reading clauses>           │   relate/rule/observe/contributes
                 <one typed `ask`>           │   + symbol/constrain/math
        │                                    │   + ONE `ask` naming the answer shape
        │                                    │   NEVER a number / posterior / verdict / root
        ▼  STAGE 2: ENGINE (native adj-lang-cli, CPU)
   JSON: {ranked, decision, recall, solve/check/optimize}  + proof object
        │
        ▼  STAGE 3: OPTION-MAP  (new, the MLE-PASS layer)
   engine answer atom  ──map──▶  the option letter whose atom == answer
        │  (no match → ABSTAIN; ambiguous tie → ABSTAIN)
        ▼  STAGE 4: RECORD
   {letter, outcome, proof_object, failure_bucket?}
```

**STAGE 1 (decompose, structure-only).** Reuses `decompose_query.py`'s
model-injection + two-sided faithfulness gate (`decompose_query.py:31`–`44`),
extended to: (a) emit the five option atoms as the candidate hypothesis set, and (b)
choose the `ask` *shape* (not the answer) from the stem's interrogative. The model is
forbidden from emitting any computed value — enforced by ARM §F's "no-result-literals"
gate (every literal in a `math`/`constrain`/`ask` RHS must trace to a cited source
span).

**STAGE 2 (engine).** The CLI runs unchanged; constraint-first then differential
(`main.rs:412`–`429`).

**STAGE 3 (option-map) — the new MLE-PASS component.** The five MCQ options are
declared as **atoms** in the ADJ program (e.g. `hexosaminidase_a`,
`bacterial_meningitis`, `quantity(29.4, m_s)`). The engine produces an *answer atom*
per shape (table below). The option-mapper compares the engine's answer atom to each
option's atom and returns the matching **letter**; **no exact match ⇒ ABSTAIN**
(defensible), **a tie / two options match ⇒ ABSTAIN** (the engine refused to
discriminate). This is the single new piece of logic and it is deliberately *dumb* —
the discrimination is the engine's job, the mapper only reads a verdict off the JSON.

**STAGE 4 (record).** Emit `{item_id, letter, gold_letter, outcome, proof}` and, for
non-correct items, the failure bucket (§2.5).

#### Answer-shape → `ask` form → engine read-out → option atom

| USMLE answer shape | `ask` form (ARM §E) | Engine output key | Option atom is… | Map rule |
|---|---|---|---|---|
| **Most-likely diagnosis** | `ask most_likely among {optA..optE}` | `decision` (determinate→leader) / `ranked[0]` | the disease atom | letter whose atom == `decision.leader`; `kickback`/`empty` → ABSTAIN (`board_eval.py:177`–`183`) |
| **Next best step / management** | `ask optimal (minimize cost) …` *or* `ask prove next_step(X)` | `solve`/`optimize` (regimen) or `decision` | the intervention atom | regimen/leader atom == option atom; `INFEASIBLE` → ABSTAIN-or-"none-of-the-above" if present (`board_eval.py:207`–`225`) |
| **Mechanism / "most associated"** | `ask most_likely among {optA..optE}` (mechanism atoms as hypotheses) **or** `? relation(subject,$Var)` | `decision`/`ranked` or `recall` | the mechanism / associated-entity atom | leader atom or `recall.answers[0].bindings[var]` == option atom |
| **Pure recall ("which enzyme…")** | `? relation(subject, $Var)` | `recall` | the fact atom | `recall.answers[0].bindings[var]` == option atom; empty → ABSTAIN (`board_eval.py:136`–`145`) |
| **Calculation (dose / acid-base / stats)** | `ask compute <expr>` (ARM PR-3/6/7) | `ranked`/a computed value + `DerivationNode` | the numeric option atom (e.g. `quantity(0.9)`) | computed value == option value within exact/dimensional equality; **needs ARM exact-compute (PR-3) / evaluate-at (PR-7)** |

**Note on "none of the above" / "two correct":** if the engine commits to an atom not
among the five options, the mapper returns ABSTAIN unless an explicit "none of the
above" option is present (then it maps to that). This keeps the never-fabricate gate
intact end-to-end.

### 2.4 The MCQ extension to the decomposition contract

Today's decomposer targets a *single* typed query and gates SUBJECT + RELATION
against the stem (`decompose_query.py:31`–`44`). MLE-PASS extends it minimally:
- The **five options become the closed hypothesis set** for `most_likely`/`prove`
  shapes — the model may not invent a sixth.
- The **option atoms must each map to a library entity** (or be a literal value for
  calculation items); an option the libraries don't know simply can't win — it
  abstains rather than misleads (the grounding guarantee, `decompose_query.py:26`–
  `31`).
- The faithfulness gate now has a **third leg, the no-result-literals check** (ARM
  §F): reject any decomposition whose `ask`/`math`/`constrain` RHS contains a literal
  absent from the cited source span — i.e. the model tried to pre-compute the answer.

### 2.5 The two-factor diagnostic (the critical part)

For **every** wrong-or-abstained item, classify the failure into exactly one bucket.
This is what turns a score into a worklist and **separates small-model
decompose-fidelity from engine-correctness** — the central measurement.

| Bucket | Definition | Detected by | Fix lands as |
|---|---|---|---|
| **(a) Missing library/edge** | The decomposition was correct and the engine ran, but no grounded edge/rule/contribution supports the answer (engine abstained or led to the wrong atom *because the knowledge isn't there*) | Re-run the decomposition's `ask` against a *manually correct* ADJ program; if the manual program also abstains/misleads, the gap is knowledge | Ground the missing edge(s) via the spider→provenance→adversarial-gate pipeline (`iem-edges.adj:11`); add to the relevant `*-edges.adj` |
| **(b) Decomposition error** | The model picked the wrong frame (wrong relation, wrong subject, wrong `ask` shape, wrong option set) | The faithfulness gate rejected it (→ abstain) OR the produced `ask` differs from a hand-authored gold `ask` for that item | Improve the decomposer prompt / vocab / few-shot; measure as `decompose_accuracy` (`board_offline.py:118`) — **does NOT count against engine-correctness** |
| **(c) Engine/language gap** | The correct reasoning is *not expressible/runnable* on today's engine (e.g. needs the deduction→evidence bridge, exact arithmetic, the CAS wiring, or a missing `ask` shape) | A hand-authored "ideal" ADJ program for the item cannot be written, or runs but produces no usable verdict, on the current engine | An **ARM PR** (PR-1 bridge / PR-3 exact / PR-6 CAS / PR-5 ask-surface). This bucket *is* the exam-driven prioritization of ARM. |
| **(d) Genuinely-hard / novel framing** | Even a perfect decomposition + complete library + full engine could not defensibly commit (ambiguous vignette, judgment call, multi-correct) | Human review after (a)/(b)/(c) are excluded | Accept as a defensible abstention; do not chase |

**The key separation:** decompose-fidelity is measured as the fraction of items whose
*produced `ask`* matches a hand-authored gold `ask` (bucket-b rate), independent of
whether the engine then got it right. Engine-correctness is measured on the subset
where the decomposition was correct (buckets a/c/d). This lets us say, e.g., "the 0.5B
model decomposes 70% correctly; on correctly-decomposed items the engine is 95%
correct; the remaining 5% are 3 missing edges + 2 needing ARM PR-1" — a worklist, not
a mystery.

### 2.6 Scoring & pass threshold

Report, per run and per organ-system:
- **raw_accuracy** = correct / total (the conventional exam number).
- **defensibility** = (correct + abstained) / total (`board_eval.py:267`) — never
  fabricated.
- **accuracy_on_attempted** = correct / (correct + wrong) (`board_eval.py:269`) — of
  what it committed to, how often right.
- **coverage** = attempted / total — how often it was willing to commit.
- **defensibility axis** = fraction of correct answers backed by a re-checkable proof
  object (recall: a citing edge with trust tier; differential: an LR proof DAG;
  calculation: a `ReasoningTrace` re-verified by ARM `adj-verify`). Today recall
  proofs are cited edges (`board_eval.py:139`–`145`); ARM PR-9 makes *every* shape
  re-checkable.

**Pass threshold.** USMLE historical pass ≈ 60% raw. **MLE-PASS target:**
- **Primary headline: pass the exam on a no-abstention basis** — i.e. force a commit
  on every item (map ABSTAIN to the engine's top-`ranked` atom even below the margin)
  and clear **≥ 60% raw_accuracy**. This is the apples-to-apples "did it pass" number.
- **Secondary (the real claim): defensibility ≥ 0.95 with accuracy_on_attempted ≥
  0.90** — i.e. it passes *and* almost never fabricates, abstaining honestly on the
  rest. A licensing-exam pass that is also defensible is the differentiated result; a
  raw-only pass that fabricates is not the goal.

### 2.7 The audit trail (machine-checkable)

Each scored item records a **proof object**, not just a letter. Today: recall cites an
edge + trust tier (`board_eval.py:141`–`145`); differential carries the LR proof DAG
in `ranked[].proof` (`main.rs:438`). Under ARM, the proof object becomes the unified
`ReasoningTrace` (ARM §E) re-verified step-by-step by `adj-verify` (ARM PR-9): a
failing re-check **localizes the error to one clause + citation**. MLE-PASS requires
that for any *correct* item to count toward the defensibility axis, its proof object
must pass `adj-verify`. This is the licensing-exam analog of "show your work, and the
work checks out."

---

## Phase 3 — Staged campaign (specs-first, measurable milestones)

**Design rule:** the FIRST milestone is a real baseline *number* on a small held-out
set, even if low, *before* any gap-closing. Then loop: baseline → classify failures
→ (ground missing libs | fix decomposer | land an ARM PR) → re-measure.

### M0 — Baseline number on 20–40 items (no gap-closing) — **DO THIS FIRST**

- **Build:** (1) author a **30-item held-out MCQ set** (option C, §2.1) spanning
  ~10 pure-recall + ~15 multi-hop-diagnosis + ~5 calculation, each as
  `{stem, options:[5 atoms], gold_letter, item_sources:[urls]}` in a new
  `board/mle_items.json`. (2) Write `board/contamination_check.py` (source-
  disjointness, §2.1) and run it green. (3) Write the **option-map layer** (STAGE 3,
  §2.3) as `board/mle_eval.py`, reusing `board_eval.resolve_recall` / `run_differential`
  and `decompose_query.decompose`. (4) Run end-to-end with the cached gold `ask`
  (decomposer in cached mode, like `board_offline.py:62`) to isolate the engine.
- **Output:** the first MLE-PASS scorecard `board/mle-scorecard.json` with raw /
  defensibility / accuracy_on_attempted / coverage, **plus the per-item failure
  bucket (a/b/c/d)**.
- **Honest expectation (see §3 honesty):** pure-recall items should largely pass
  *today* (the recall path is mature, board-scorecard 139/145 correct); multi-hop
  items will mostly land in **bucket (c) blocked on ARM PR-1**; calculation items in
  **bucket (c) blocked on ARM PR-3/6/7**. The baseline number will likely be
  *recall-dominated* and that is the point — it tells us exactly how much of "passing"
  is already in hand vs. gated on ARM.

### M1 — Close decomposition (bucket b) on the existing recall path

- Run M0 with a **live local model** (Gemma-3-4B / Qwen2.5-0.5B,
  `decompose_query.local_gen`) instead of cached gold `ask`. Measure
  `decompose_accuracy` separately from engine-correctness (§2.5). Improve prompt /
  few-shot / vocab until bucket-b rate is small. This is pure decomposer work, no
  engine change — fully parallel to ARM.

### M2 — **ARM PR-1 (deduction→evidence bridge): the multi-hop unlock**

- **This is the single highest-leverage step for the exam.** Until it lands, any
  vignette whose answer-bearing premise must be *derived* (e.g. "neutrophilic +
  low-glucose CSF ⇒ csf_suggestive ⇒ bacterial") cannot be one query
  (`lib.rs:937`–`955`). After it lands, the multi-hop majority of USMLE becomes
  expressible. Re-run M0/M1; watch bucket-(c)-PR1 items convert to correct (or to
  bucket-a "missing edge," which is then a grounding task).
- MLE-PASS does not implement PR-1 — it **consumes** it (ARM Phase 3, PR-1) and uses
  the exam to *prioritize* it.

### M3 — Gap-driven library grounding (bucket a), exam-shaped not breadth-first

- The recall loop today is breadth-first (add a domain → add edges). MLE-PASS
  **inverts it**: every bucket-(a) failure is a *named missing edge* the exam
  demanded. Ground exactly those edges via the spider pipeline
  (`iem-edges.adj:11`), re-measure. The exam becomes the grounding backlog
  generator — knowledge is grounded *because an item needed it*, with the item as the
  acceptance test.

### M4 — Calculation items (bucket c → ARM PR-3/6/7)

- Acid-base, renal, dosing, biostatistics items need exact arithmetic + dimensional
  checking (ARM PR-3) and, for symbolic ones, CAS wiring + evaluate-at (ARM PR-6/7).
  Map each calculation item to the ARM PR it needs; land, re-measure.

### M5 — Defensibility hardening (ARM PR-9 `adj-verify`)

- Require every *correct* item's proof object to pass `adj-verify` (§2.7). Report the
  defensibility axis. This converts "got it right" into "got it right *and the proof
  re-checks*," which is the licensing-exam-grade claim.

### Milestone loop summary

```
M0 baseline number  ──▶  classify (a/b/c/d)
                          │
        ┌─────────────────┼──────────────────────┐
   bucket a            bucket b               bucket c
 (ground edge)     (fix decomposer)        (land ARM PR)
   = M3               = M1                = M2(PR1)/M4(PR3/6/7)
        └─────────────────┴──────────────────────┘
                          ▼
                    re-measure ──▶ (repeat)
                          ▼
              M5: every correct item's proof re-checks (adj-verify)
```

### The single highest-leverage first step

**M0 — produce a real baseline number on a 30-item held-out MCQ set with the
contamination check green and per-item failure-bucket classification.** It is small,
needs only the option-map layer (no engine change), and its *failure buckets directly
prioritize everything after it* — it tells us, with numbers, exactly how much of
"passing the boards" is already in hand (recall) vs. gated on ARM PR-1 (multi-hop) vs.
gated on ARM PR-3/6/7 (calculation). Building the score before building the fixes is
the whole discipline.

---

## Three representative worked items (decompose → ADJ → answer → audit)

### (1) Pure recall — passes TODAY

**Stem:** "A 6-month-old of Ashkenazi descent has developmental regression, an
exaggerated startle, and a cherry-red macula. Tay-Sachs is diagnosed. Which enzyme is
deficient?" Options: A hexosaminidase_a · B glucocerebrosidase · C
sphingomyelinase · D alpha_galactosidase_a · E galactocerebrosidase.

**Decompose (model, structure-only):**
```
import "iem-edges.adj"
? deficient_in(tay_sachs, $Enzyme)        % SUBJECT+RELATION gated to the stem bytes
```
**Engine:** `recall: [{query:"deficient_in(tay_sachs, Enzyme)", abstained:false,
answers:[{bindings:{Enzyme:"hexosaminidase_a"}, citations:[{trust:"authoritative",
locator:"…NBK1218/"}]}]}]` (`main.rs:478`–`486`, edge `iem-edges.adj:33`–`37`).
**Option-map:** `hexosaminidase_a` == option A → **A**.
**Audit:** the citing edge's grounded byte-quote + URL + `trust authoritative`
(`iem-edges.adj:34`–`36`). **Status: works on the current binary** (board-scorecard
has this exact item correct).

### (2) Multi-hop diagnosis — **needs ARM PR-1**

**Stem:** "An 8-month-old with fever, neck stiffness; CSF shows 4,000
neutrophils/µL, glucose 20 mg/dL, protein 250 mg/dL. Most likely diagnosis?"
Options: A bacterial_meningitis · B viral_meningitis · C fungal_meningitis · D
subarachnoid_hemorrhage · E febrile_seizure.

**Decompose (model, structure-only):**
```
import "meningitis.adj"
rule { head: csf_suggestive when: csf_neutrophilic, csf_low_glucose }   source "stem"
observe csf_neutrophilic        % from "4,000 neutrophils/µL"
observe csf_low_glucose         % from "glucose 20 mg/dL"
ask most_likely among { bacterial_meningitis, viral_meningitis, fungal_meningitis,
                        subarachnoid_hemorrhage, febrile_seizure }
```
**Engine (after ARM PR-1):** PR-1 lets the *derived* `csf_suggestive` (proved from
the two observations) gate `contributes 12 from csf_suggestive to bacterial_meningitis`
in `lib/bacterial-arm.adj`; the differential commits `decision.determinate{leader:
bacterial_meningitis}` (`main.rs:447`–`457`). **Today** (`lib.rs:937`–`955`) the
derived atom can't gate the contribution → either the program must `observe
csf_suggestive` directly (which the no-result-literals gate should discourage) or the
item lands in **bucket (c)-PR1**.
**Option-map:** `bacterial_meningitis` == option A → **A**.
**Audit:** `ReasoningTrace` interleaving `FromRule(csf_suggestive)` →
`FromContribution(LR 12, evidence_proof=…)` → posterior, each cited (ARM §E);
`adj-verify` re-runs each step. **Status: BLOCKED on ARM PR-1** — this is the
multi-hop majority's gating dependency.

### (3) Calculation — **needs ARM PR-3 (exact compute / dimensional)**

**Stem:** "A patient has measured serum osmolality 320, Na 140, glucose 360 (mg/dL),
BUN 28 (mg/dL). What is the osmolar gap?" (calculated osm = 2·Na + glucose/18 +
BUN/2.8). Options: A 0 · B 5 · C 10 · D 15 · E 20.

**Decompose (model, structure-only — emits the FORMULA, never the number):**
```
let calc_osm = 2*Na + Glucose/18 + Bun/2.8     source "osmolality formula" locator "stem"
observe Na(140) ; observe Glucose(360) ; observe Bun(28)
let gap = Measured_osm - calc_osm
observe Measured_osm(320)
ask compute gap
```
**Engine (ARM PR-3 exact + dimensional):** `compute.rs` evaluates the
`DerivationNode` tree exactly: calc_osm = 280 + 20 + 10 = 310; gap = 320 − 310 = **10**;
every leaf traces to an observed fact (`compute.rs:105`–`144`), dimensions checked via
`dimension.rs::combine`.
**Option-map:** `10` == option C → **C**.
**Audit:** the `FromCompute` `DerivationNode` re-evaluated by `adj-verify`; the
no-result-literals gate confirms the model emitted only the formula + the stem's
numbers, never `10`. **Status: BLOCKED on ARM PR-3** (today `let` is f64, no exact /
dimensional guarantee; `compute.rs` exists so the lift is small).

---

## Brutally honest scope summary

- **What exists and works end-to-end TODAY:** the board-eval harness with the
  three-outcome model + never-fabricate gate (`board_eval.py`), the offline
  prose→model→ADJ→engine pipeline with a network-egress tripwire and a measured
  ~74%-accurate 4B/0.5B local decomposer (`board_offline.py`, `decompose_query.py`),
  124 recall `.adj` files (~17 loaded edge libraries) with byte-provenanced grounded
  edges, the native CLI emitting a full JSON answer+proof object
  (`main.rs:514`–`521`), and abstention-aware metrics (`board_eval.py:245`–`274`).
  Committed numbers: 148/7/0 on 155 structured items.
- **Could the system plausibly PASS today on pure-recall items? YES.** The recall
  path is mature (139/145 correct, grounded-coverage 0.99). A recall-heavy slice
  would clear 60%.
- **Could it pass the MULTI-HOP majority of a real USMLE today? NO.** The
  deduction→evidence gap (`lib.rs:937`–`955`) means a derived premise can't weigh on a
  differential in one query — and most USMLE vignettes require exactly that derivation
  chain. **ARM PR-1 is the unlock**; calculation items additionally need ARM
  PR-3/6/7.
- **What MLE-PASS must BUILD (not present today):** (1) a real five-option MCQ format
  + the option-map layer (STAGE 3, §2.3) — small, no engine change; (2) a
  contamination-controlled held-out item set + the source-disjointness check (§2.1–
  2.2) — small; (3) the two-factor failure diagnostic (§2.5) — the measurement that
  makes the campaign a worklist; (4) consumption of ARM PR-1/3/6/7/9 for multi-hop,
  calculation, and re-checkable proofs.
- **The defensible claim MLE-PASS targets** is not "a model scored X%." It is "an
  engine reasoning over its own grounded, source-disjoint libraries passed a licensing
  exam, committed only when defensible (defensibility ≥ 0.95), and every correct
  answer carries a proof that an independent checker re-verifies." That is a stronger
  and more honest claim than a raw score — and the infrastructure to make it is mostly
  in place; what's missing is the MCQ layer and ARM PR-1.
