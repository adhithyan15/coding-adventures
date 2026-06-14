# MYCIN-2026 — Roadmap to full 1976 coverage, the missing language features, and the local pivot

This is the plan-of-record for finishing MYCIN-2026: (1) cover **everything the
1976/80s MYCIN did**, on our byte-provenance + constraint substrate; (2) add the
**language features** the engine still needs for all of it to be *native and
audited* (not Python wrapping the CLI); then (3) **pivot to the fully-local small
model** so a doctor can go from messy spoken input → diagnosis (with an audit
trail) → uncertainties that could shift the result → a treatment/dosing plan
constrained by the input — with **0 answer-time cloud calls** and patient data
that never leaves the machine.

Hard product invariant (restated): **this never replaces a physician.** It is
decision *support* — it works the combinatorial/numeric details instantly and
hands over the entire flow as an inspectable, overridable audit trail. It must
DEFER/abstain visibly rather than fabricate. The physician makes the call.

## Where we are (merged + this PR)

- **M0–M8 merged** (adj-lang 0.11.0, adj-lang-cli 0.5.0): the language, the
  dictionary/rulebook/import grammar, the warm decompose→diagnose pipeline (1
  model call → 0 answer-time calls), CAS as importable libraries + adversarial
  write gate, VOI "what to check next", IIS consistency, the five proofs.
- **This PR (the treatment vertical):** therapy as a *solved constraint problem*
  — a grounded, adversarially-gated, content-addressed **formulary** of drug
  *facts* (not hard-coded regimens), from which `derive_regimen.py` DERIVES a
  minimum-cost **set-cover** regimen, customizes it per patient (exclusions,
  combinations), and solves a **dose window** that goes **UNSAT + IIS** when no
  safe-and-effective dose exists. The spider grounded the formulary and the gate
  *corrected three hand-authored errors*. `consult.py` composes the whole flow.

## What MYCIN (1976) actually did, and our coverage

| MYCIN capability | Our mechanism | Status |
|---|---|---|
| ~450 production rules, backward chaining | grounded adj-lang rulebooks in the CAS | ✅ (meningitis) |
| Certainty factors (−1..+1) for uncertainty | probabilistic LR aggregation (a *superset* of CF) | ✅ |
| Identify the **significant organism(s)** from clinical + lab findings | differential over an organism-identity rulebook | ⬜ **A1/A2** |
| Significant-infection / site determination | site/source rulebook | ⬜ A2 |
| Interactive consultation (asks the physician) | VOI "what to check next" + an interactive ask loop | ◑ (VOI ✅; ask loop ⬜ B3) |
| Explanation: WHY / HOW | proof DAG + rendered audit trail | ✅ |
| Therapy: cover all organisms with the **fewest drugs**, by sensitivity & preference, dosed by weight/renal | minimum-cost **set-cover** + dose-window solve | ✅ (this PR) |
| In-vitro **sensitivity** data overriding empiric choice | `observe sensitive/resistant` facts gating coverage | ⬜ B2 |
| Domain = **bacteremia** (blood) + meningitis | meningitis done; bacteremia/sepsis next | ◑ (A2) |

We already *exceed* 1976 on three axes: calibrated probabilities (not CFs),
byte-cited grounding with an adversarial write gate, and a dosing model that can
prove "no safe dose exists" (IIS) — a call humans miss under load.

## Work items (ordered; one PR each; stacked, merge bottom-up)

### Phase A — finish MYCIN's diagnostic scope (identify the organism, then treat it)
- **A1 — Organism-identification rulebook (meningitis).** A grounded rulebook
  mapping findings (Gram-stain morphology, age band, immune status, exposures,
  CSF pattern) → a posterior over *specific organisms* (S. pneumoniae, N.
  meningitidis, Listeria, H. influenzae, gram-negative rods, MRSA). This is the
  step MYCIN did before therapy. Its output set of probable organisms feeds the
  set-cover we already built → identify → cover → dose, end to end.
- **A2 — Bacteremia / sepsis domain.** MYCIN's *primary* domain. A second site:
  source/portal-of-entry rulebook + organism priors by source (urinary, line,
  abdominal, skin) + formulary entries for gram-negative & staph coverage.
  Proves the substrate generalizes across infection sites with no new engine.

### Phase B — the missing language features (make it native + audited)
- **B1 — native `select` / minimum-cost set-cover in adj-lang.** Today the
  set-cover lives in Python (`derive_regimen.py`) wrapping the CLI; the regimen
  has no engine proof DAG. Surface a `select … covering … minimizing …`
  construct (grammar + AST + lower + a logic-engine set-cover/ILP tactic) so the
  regimen is derived *inside* the engine and carries the same byte-cited proof
  DAG the diagnosis already has. **The headline language feature.**
- **B2 — native combination & sensitivity predicates.** First-class
  `combination_covers(...)` and `sensitive/resistant(...)` so combination rules
  and in-vitro data are audited engine facts, not Python-side special cases.
- **B3 — interactive `ask` loop (optional).** Drive VOI as MYCIN's interactive
  consultation: surface the highest-value unobserved finding, accept the answer,
  re-derive. Pure orchestration over existing VOI; no new reasoning.

### Phase C — the fully-local small-model pivot (privacy / HIPAA by architecture)
- **C1 — land the local-model artifacts.** Track `bench/` (model-floor ladder +
  findings) and `train/` (framework-authored data gen + LoRA eval) as a clean
  PR (the specialist decomposer: Gemma-3-1B/4B, base 0/4 → specialist 4/4).
- **C2 — wire the specialist into the warm path.** Make the trained local model
  the decomposer (messy prose → typed IR), so the *entire* warm path is local:
  1 small on-device call + CPU engine. Fall back to cloud only if unavailable.
- **C3 — the ER spine.** `mlx-whisper` voice → transcript → decompose → triage
  acuity → immediate actions. The "someone walks into the ER" demo: spoken input
  to a triage decision + first-actions list + audit trail, fully on-device.

### Phase D — chart interop (the "open format like EPIC" question)
- **D1 — FHIR ingestion.** The open standard the question points at is **HL7
  FHIR** (the API EPIC/Cerner expose). Parse a FHIR `Bundle`
  (Patient/Condition/Observation/AllergyIntolerance/MedicationStatement) → our
  chart shape → decompose. Lets the pipeline run off a real chart export, not
  just free text. Future, after C.

## Operating rules for the overnight loop
- One PR per item; **stacked** on the previous (I cannot self-merge — user signs
  off). PRs are labeled "stacked on #N, merge after." Merge **bottom-up**.
- Each item: spec → tests → impl → CHANGELOG/README → `/security-review` →
  push → babysit CI/conflicts. adj-lang/engine changes: `cargo build
  --workspace`; never `cargo fmt` generated grammar files; never `git add -A`.
- Honesty markers stay: grounded vs inferred vs authored-illustrative; abstain
  rather than fabricate; every premise one-edit-overridable in the CAS.
