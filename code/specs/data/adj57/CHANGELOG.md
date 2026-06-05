# Changelog — adj57 byte-provenance pipeline

## [0.12.0] — 2026-06-04

### Added

- **ADJ69 — defensible output on a contested question (worked example).** Runs the ADJ68
  open-book defensibility pipeline on a user-chosen question whose honest answer is *not a
  single name*: *"who was Jason's maternal great-grandfather?"* — two independent ambiguities
  (definitional: father of the maternal grandfather vs grandmother; source: Apollonius'
  Alcimede vs Apollodorus' Polymede).
  - **`pipeline/jason.workflow.js`** — spider+ground each genealogical link in a verbatim
    source, flag contested links, surface both ambiguities; vs a bare closed-book recall
    answer; adversarial auditor scores defensibility (every link sourced AND ambiguities
    surfaced — not single-name correctness).
  - **Result** (`pipeline/jason-results.json`): **bare recall hallucinated** — led with
    "Cretheus" (Jason's *paternal* grandfather: wrong side AND wrong generation), zero
    citations → **NOT_DEFENSIBLE** (6/6 links unsupported). **The grounded trace** produced a
    cited map — Deion (Apollonius), Hermes (Apollodorus), Minyas (grandmother line) — 6/6
    links cited, contested mother split per source, both ambiguities surfaced →
    **DEFENSIBLE**, refusing to collapse.
  - **Both pillars on one case:** accounting (bare committed commission "Cretheus" + omission
    of the grandmother branch; grounded did neither) and correctability (every branch a
    citable editable node — incl. a real findable imprecision, "his sister Alcimede", for a
    human to correct). **Honest limit:** document-granularity citations (citation-precision
    verifier is the next build). Spec: [ADJ69](../../ADJ69-contested-question-worked-example.md).

## [0.11.0] — 2026-06-04

### Added

- **ADJ68 — open-book defensibility audit (verifiability, not recall).** Corrects ADJ67's
  methodological error: the framework is **open-book always** and targets **auditable,
  defensible work, not recall** — so this scores an adversarial *fault-finding* audit, not
  final-answer correctness, on the same Nicolaou endiandric-acid chemistry question.
  - **`pipeline/audit.workflow.js`** — (1) spider+ground the needed facts open-book, each in
    a verbatim source passage (CAS-style); (2) build the answer as a defensible CHAIN (every
    node cites a grounded fact / cited rule / arithmetic / prior node) vs a bare closed-book
    recall answer; (3) an adversarial auditor scores **verifiability** (PASS = a reader can
    verify every link). **`pipeline/run_audit.py`** reports it + a deterministic
    defensibility fraction.
  - **Run:** the spider grounded the **step ORDER** (8π closes first, then 6π) verbatim from
    PMC2766600 — the exact fact ADJ67-closed-book got tangled *deriving*. This settles the
    open question: **byte provenance CAN catch the established order; the ADJ67 tangle was an
    artifact of running closed-book, not a grounding failure.**
  - **Defensibility:** both answers were correct, but bare recall scored **3/9 verifiable
    (33%) — indefensible** (synthesis identity, step structures, and the W-H rule itself
    asserted from memory, zero citations), while the grounded chain scored **6/7 (86%)**. The
    audit **has teeth** — it faulted the framework too (node 6: the cited page confirms
    Diels–Alder=[4+2] but not the appended diene=4/dienophile=2 atom assignment — a
    citation-precision miss). Both binary-FAIL (every link must verify), but the scores are
    night and day.
  - **Lesson:** a correct answer can be indefensible; defensibility is the product, and it is
    measurable. Next: a **citation-precision verifier** (the cited passage must establish the
    *whole* claim — would make 6/7 → 7/7) and the screening harness re-scoped to the
    defensibility fraction. Spec: [ADJ68](../../ADJ68-defensibility-audit.md).

## [0.10.0] — 2026-06-04

### Added

- **ADJ67 — does the grounding discipline help? (blind HLE head-to-head + weak-model
  ladder).** Two experiments, reported honestly including where the framework did NOT help.
  - **Blind HLE head-to-head** (`hle-headtohead.json`): plain Claude vs a grounding-discipline
    agent, both blind + closed-book, on real Humanity's Last Exam questions with
    independently-verified answers held aside.
    - *Round 1 (Palmyrene, RIB 1065):* **framework win** — coverage forced engagement with
      the `ḤRY` token; plain Claude pattern-matched `BT…BR` to "daughter of… son of…" and
      committed the wrong literal (dropping freedwoman-of-Barates). Both knew the inscription —
      a reasoning-faithfulness/commitment win, not knowledge.
    - *Round 2 (Nicolaou endiandric-acid cascade):* **tie; plain cleaner** — both reached
      `[8π]-con, [6π]-dis, [4+2]`, but the framework's derive-from-rules discipline tangled on
      the step *order* (which W-H does not determine), where plain holistic recall was clean.
  - **Weak local model on the HLE chemistry item** (`pipeline/ollama_hle_chem.py`): fed
    atomic + grounded, a ~Gemma-class local model reaches `[8π]-con, [6π]-dis, [4+2]` — the
    answer Opus-framework got tangled on. The framework supplies the π-counts/order/rule
    (stand-in for spider/CAS grounding); the weak model does only rule-application. The
    structural fact Opus slipped on (ordering) is exactly what the grounded decomposition
    hands over.
  - **Atomic-feed ladder** (`pipeline/ollama_atomic.py`, `pipeline/atomic_haiku.workflow.js`)
    on a marine-forensics trap case with blind controls: atomic decomposition defeats the
    holistic exposure-prior trap even at 0.5B; grounded per-atom criteria fix a mid-size
    model's wrong domain knowledge (Gemma net→propeller); a **capability floor** exists (0.5B
    degenerate), and a **discrimination gate** detects ~zero-variance verdicts and refuses to
    launder them into a confident answer (analog of ADJ65's "margin rests on assumed").
  - **Honest conclusions:** n=2 (anecdote, likely-contaminated); the framework is NOT
    uniformly better (1 win, 1 tie); it helps the pattern-match-and-drop class and can
    over-formalize recall-dependent steps; a real number needs the screening harness. Spec:
    [ADJ67](../../ADJ67-grounding-discipline-headtohead.md).

## [0.9.0] — 2026-06-04

### Added

- **ADJ66 — the spider: grounding the rulebook in source bytes.** ADJ65 flagged that the
  decision rested on `assumed` weights; the spider grounds them. The principle: *nothing
  may be asserted that is not grounded to bytes — input OR the rulebook we derive for it.*
  A weight (a likelihood ratio) is a rulebook claim, so the spider
  (`pipeline/spider.workflow.js`) runs **live web search + fetch** per discriminating fact,
  copies a **verbatim passage** from an authoritative source (MSD/NCBI/WHO/PMC) for each
  fact→hypothesis link, derives the weight from it, and records URL + quote (a weight with
  no source is set to 0 and flagged, never invented). **74 web fetches** produced a fully
  cited weight matrix — the rulebook. `pipeline/run_spider.py` re-runs the decision on
  grounded weights, before/after.
  - **Run (neurobrucellosis):** grounding did **not** flip the answer — and that is the
    finding. BEFORE: East African trypanosomiasis 99.7% on assumed weights. AFTER: still
    trypanosomiasis 99.2%, but **8/12 facts grounded and `margin rests on assumed = False`**
    — the chancre-at-bite-site (+11 dB, MSD/NCBI) and East-Africa-restricted *T. b.
    rhodesiense* (+12 dB) genuinely support HAT *from the case bytes*. The spider satisfied
    the principle (rulebook byte-cited), **refused to launder** the answer into "correct,"
    and isolated the residual to the right place: the datum that overturns it (**Brucella
    serology**) is a missing **input** byte — an ADJ64 named hole — not a rulebook gap.
    Same lesson as the axle case (faithfulness ≠ completeness), now on both grounding axes.
  - **Honest limits:** passage→decibans is still the model's mapping (next: a verifier that
    the passage supports the magnitude); 6/12 facts grounded (load-bearing first); source
    authority not yet graded / not recursed to primary studies. Spec:
    [ADJ66](../../ADJ66-spider-rulebook-grounding.md).

## [0.8.0] — 2026-06-04

### Added

- **ADJ65 — uncertainty as a first-class primitive (weight of evidence + sensitivity).**
  Makes the hypothesis competition first-class and answers *"if we make some probability
  shift, how would the decision shift?"*
  - **`pipeline/sensitivity.py`** — Good's weight of evidence (the math behind MYCIN
    certainty factors): each fact contributes **decibans** (10·log10 LR) toward each
    hypothesis; the **decision is argmax of the summed log-odds** (deterministic — softmax
    is a display *view* only, no temperature knob). Reports the **margin** (robustness),
    load-bearing evidence, one-out flips, per-weight tipping points, and — the honest part
    — whether the margin **rests on `assumed` (ungrounded) weights**. 10 unit tests
    (`pipeline/test_sensitivity.py`).
  - **`pipeline/sensitivity.workflow.js`** — the model proposes hypotheses + a weight
    matrix, each weight tagged grounded (cites a real LR) or assumed.
    **`pipeline/run_sensitivity.py`** runs the engine + reports.
  - **Run (neurobrucellosis):** the engine picked **East African trypanosomiasis at 99.7%
    (+26 dB margin)** — **wrong** (truth neurobrucellosis, 4th). But it flagged that **every
    load-bearing weight is `assumed`**; the only grounded weights contribute ~0 dB. The
    99.7% is an artifact of four made-up numbers. ADJ65 doesn't make the model right — it
    makes its **confidence auditable**, converting overconfidence into a prioritized
    fetch-list. Spec: [ADJ65](../../ADJ65-uncertainty-primitive.md).

## [0.7.0] — 2026-06-04

### Added

- **ADJ63 — bidirectional justification end-to-end** (`pipeline/bidirectional.workflow.js`,
  `pipeline/run_bidirectional.py`). The ADJ61 (output) + ADJ62 (input) gates wired into one
  pipeline and run on a fresh, non-medical case the agent found itself — railway-axle
  metallurgical failure analysis (MDPI PMC12387781). All four provenance corners held:
  coverage 100% (17 facts / 1975 bytes), input extraction 32/32 (24 extracted + 8 inferred),
  output grounding 13/13 (7 evidence + 6 conclusion), 0 rejected. **Finding —
  faithfulness ≠ completeness:** the byte-faithful answer ("manufacturing/surface cause")
  diverged from the held-aside truth ("operating stress exceeded the fatigue limit")
  because the decisive datum (the stress-vs-fatigue-limit comparison) was never in the
  bytes. The framework did not fabricate it — it gave the answer the bytes support and the
  gap stayed visible. Spec: [ADJ63](../../ADJ63-bidirectional-end-to-end.md).
- **ADJ64 — the underdetermination gate** (`pipeline/underdetermination.py`,
  `pipeline/underdetermination.workflow.js`, `pipeline/run_underdetermination.py`). The dual
  of invention: stops a conclusion from singling out one cause when the datum that would
  distinguish it from the rivals is **absent**. For each rival fitting the same bytes, the
  model gives the discriminating observation + whether it is present (verbatim citation) or
  absent; a rival is *resolved* iff present-and-cited, else *open* (its observation is a
  **named provenance hole** — a query for the spider/CAS). Conclusion is *underdetermined*
  iff any rival is open. 5 unit tests (`pipeline/test_underdetermination.py`). **Run** on
  the ADJ63 axle conclusion: 7 rivals, 5 open → **UNDERDETERMINED**; the gate named five
  missing measurements, including the **operating-stress-vs-fatigue-limit comparison — the
  ground-truth root cause** — and replaced the single-cause answer with a disjunction that
  keeps every grounded finding. **Honest limit:** the "resolved" verdict is an LLM judgment
  (a citation being present ≠ it discriminating); read conservatively the 2 "resolved"
  rivals are also open, making the safe verdict only stronger. Spec:
  [ADJ64](../../ADJ64-underdetermination-gate.md).

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
