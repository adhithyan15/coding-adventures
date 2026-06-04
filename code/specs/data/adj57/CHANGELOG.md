# Changelog — adj57 byte-provenance pipeline

## [0.6.0] — 2026-06-04

### Added

- **ADJ62 — input justification (extract/infer → which bytes → why).** Applies the
  ADJ61 justification gate to the **input** side. Coverage (ADJ57/58) proved nothing was
  *dropped*; this proves nothing was *mis-extracted*: after decomposing, the framework
  asks the agent *"what did you extract or infer, from which bytes, and why do those
  bytes prove it?"* and runs the two-layer gate on the answer.
  - **`pipeline/justify_gate.py`** generalized to be **stage-symmetric** — kinds
    `extracted` (strict, ≙ evidence) / `inferred` (hedged, ≙ conclusion) alongside the
    output kinds; `by_kind`/`n_strict`/`n_inference` counts; reads either `claim` (output)
    or `fact` (input) as the assertion text. Output-stage driver back-compat preserved.
  - **`pipeline/justify_input.workflow.js`** — decompose (coverage) → "account for what
    you took" → adversarial extraction verifier → two-layer gate + kickback.
  - **`pipeline/run_justify_input.py`** — reports BOTH input gates (coverage +
    extraction justification) and the extracted/inferred split.
  - 5 new gate tests (15 total) covering the input kinds.
  - **Run (neurobrucellosis bytes):** coverage 100% (27 fact-segments + 3 discards =
    1812 bytes); extraction 49/49 grounded — **41 extracted + 8 inferred**, 0 rejected.
    The gate forced 8 readings into the *inferred* column that coverage would have let
    pass as fact — incl. **"the patient is male"** (the case says only *"He"*, never
    "male"), "East African" countries (text says only "Africa"), "hepatosplenomegaly"
    (a composite), "albuminocytologic dissociation" (a label) — while correctly keeping
    "tachycardia" as *extracted* (the word appears verbatim). Separates what the text
    *says* from what the reader *infers*, byte by byte. Spec:
    [ADJ62](../../ADJ62-input-justification.md).
  - **Honest limitations (unchanged):** layer 2 is an LLM verdict (multi-verifier vote
    still pending); the live reject/kickback path was not exercised (clean first pass).

## [0.5.0] — 2026-06-04

### Added

- **ADJ61 — the justification gate (combine bytes → justified fact).** Replaces
  ADJ60's *substring* output gate, which was both too tight (an honest fact built from
  several bytes has no single verbatim span; the conclusion name is never a byte) and
  too loose (it checked a citation *exists*, never that it *supports* the claim).
  - **`pipeline/justify_gate.py`** — two layers: (1) **byte-anchor** (deterministic) —
    *every* cited span must be verbatim (no fabricated citations; strictly stronger than
    ADJ60); (2) **justification** (an adversarial verifier verdict) — the cited bytes,
    *combined*, must justify the claim. Claims are typed **evidence** (statement about
    the input — strict) vs **conclusion** (inference from the evidence — allowed as a
    hedged hypothesis). 10 unit tests (`pipeline/test_justify_gate.py`).
  - **`pipeline/justified.workflow.js`** — derive typed claims → adversarial
    justification verifier → two-layer gate with kickback loop.
  - **`pipeline/run_justified.py`** — reports the gate, the evidence/conclusion split,
    and each claim's combined cited bytes.
  - **Run (same neurobrucellosis bytes as ADJ60):** 20/20 grounded (16 evidence + 4
    conclusion), 0 rejected, clean first pass. The framework now **names the diagnosis**
    — *"most likely … disseminated brucellosis (neurobrucellosis)"* — as a hedged
    inference grounded by **combining seven bytes**, with rickettsial/atypical-mycobacterial
    held as alternatives, **without inventing a single evidence byte**. ADJ60 refused to
    name it and drifted to a "vector-borne" red herring.
  - **Honest limitations:** layer 2 is an LLM verdict (only as strict as the verifier —
    it flagged-but-passed one mild evidence overstatement); the live reject/kickback path
    was not exercised (clean first pass — covered by unit tests only). Spec:
    [ADJ61](../../ADJ61-justification-gate.md).

## [0.4.0] — 2026-06-04

### Added

- **ADJ60 — the output-grounding gate (bidirectional byte provenance).** The dual of
  input coverage: every output claim must trace back to input bytes (*nothing
  invented*), completing the invariant ADJ57/58 only half-enforced.
  - **`pipeline/output_gate.py`** — a claim is grounded iff a citation is a *verbatim*
    span of the allowed input (case text + used-fact terms); ungrounded claims (no
    retrievable citation) are rejected for kick-back. 6 unit tests
    (`pipeline/test_output_gate.py`).
  - **`pipeline/grounded.workflow.js`** — ingest with byte coverage → derive answer as
    grounded claims → inline output-grounding gate with a **kickback loop** (re-derive
    any ungrounded claim until every claim cites verbatim input bytes).
  - **`pipeline/run_grounded.py`** — verifies BOTH gates and prints the bidirectional
    trail + each claim mapped to its input span.
  - **Run (neurobrucellosis, PMC2769393):** INPUT 100% covered (25 facts + 1 discard =
    1812 bytes); OUTPUT 20/20 claims grounded → bidirectional provenance complete.
  - **Finding:** the strict gate made the framework *refuse to name neurobrucellosis*
    (the answer name isn't a byte; the serology was held aside) — revealing the gate
    conflates **evidence claims** (must be byte-grounded — rejects geology's
    "tremolitized") with the **conclusion** (an inference from grounded evidence —
    should be allowed as a flagged hypothesis). Next: ADJ61 splits the two. Spec:
    [ADJ60](../../ADJ60-output-grounding-gate.md).

## [0.3.0] — 2026-06-04

### Added

- **ADJ59 — cross-domain validation + the qualitative verdict.** Ran the framework
  head-to-head vs plain Claude with a blind judge across SIX non-medical domains
  (engineering, astronomy, cybersecurity, geology, paleontology, linguistics).
  - **Qualitative verdict** (`pipeline/run.py`): when no quantified posterior is
    groundable (no published likelihood ratios — the norm outside medicine), commit
    to the derive-stage leading answer with its byte-provenanced evidential basis
    instead of abstaining. This flipped the framework from **0–3 to 4 wins / 1 tie /
    1 loss** (correct in all 6 domains; plain Claude 3 correct / 2 partial / 1 wrong).
  - **Head-to-head harness:** `crossdomain.workflow.js` + `crossdomain2.workflow.js`
    (3 domains each, framework pipeline + plain-Claude arm), `make_judge.py` (builds
    blinded A/B contexts + framework reports via run.py), `judge*.workflow.js` (blind
    expert judge). Results in `pipeline/crossdomain-judge-results.json`.
  - **Methodology bug caught + fixed:** `run.py` printed the held-aside ground truth,
    which `make_judge.py` leaked into the framework's report; the blind judge flagged
    it; stripped and re-judged clean.
  - **CAS accumulation:** 8 (pheo+KFD) → 17 → 23 sources across runs, deduplicated.
  - Two open weaknesses (spec §4/§6): (1) over-specification beyond byte evidence —
    the missing **output-grounding gate** (the dual invariant: nothing dropped AND
    nothing invented); (2) answer-first report format. Spec:
    [ADJ59](../../ADJ59-cross-domain-validation.md).

## [0.2.0] — 2026-06-04

### Added

- **ADJ58 — the universal stage contract.** Byte provenance enforced at EVERY
  pipeline arrow, not just case→IR.
  - **`pipeline/stage.py`** — a generic `Stage` gate + composed `Trail`. Two input
    shapes, one contract: TEXT inputs partition byte-for-byte; ELEMENT inputs
    partition the id set. `clean` = covered + every used cites a `produced` + every
    discard a `reason`. `Trail.ok()` = unbroken byte-trail end to end. 9 unit tests
    (`pipeline/test_stage.py`).
  - **`pipeline/run.py`** — drives a full-run result through EVERY stage's gate
    (decompose / derive / ground:\* / aggregate), composing the trail, interning
    sources into the CAS, computing the verdict. A stage that fails to cover its
    input shows up as a HOLE — the framework stops claiming auditability it lacks.
  - **`derive` retrofit** — the workflow now emits `fact_dispositions`: every fact
    is USED (with a role) or DISCARDED (with a reason). Closes the silent-drop hole
    (comorbidities like glaucoma must be discarded-with-reason, not ignored).
  - Spec: [ADJ58](../../ADJ58-universal-stage-contract.md).

## [0.1.0] — 2026-06-04

### Added

- **ADJ57 — the four-layer byte-provenance pipeline**, proven end-to-end on a fresh
  case (pheochromocytoma, PMC11521393).
  - **L0 — CAS** (`pipeline/cas.py`): content-addressed source store. Interns each
    source by `sha256(content)` (free dedup), records byte-anchored citation spans,
    and rejects any quote not literally present. The reusable indexed-source corpus.
  - **L1 — coverage** (`pipeline/coverage.py`): the case→IR partition must
    reconstruct the input byte-for-byte. Demonstrated the enforce→correct loop —
    caught a collapsed `\n\n` at byte 1296, localized to segment 42, corrected,
    re-verified clean (22 typed facts + 24 reasoned discards = 100% of 1499 bytes).
  - **L1–L3 workflow** (`pipeline/slice.workflow.js`): lossless typed partition →
    fact-driven link derivation → recursive-to-root grounding spider.
  - **driver** (`pipeline/assemble.py`): ties the layers together and writes the
    byte-addressable `rulebook.json`.
  - **Proof:** `blood_pressure(160/100) → pheochromocytoma  LR = 0.762 [grounded,
    root]` traced to a JAMA Rational Clinical Examination span byte-anchored in the
    CAS (`content[165:297] == quote`). `weight_loss` returned `direction_only` (no
    root data → no number).
  - Spec: [ADJ57](../../ADJ57-byte-provenance-pipeline.md).
