# Changelog — adj57 byte-provenance pipeline

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
