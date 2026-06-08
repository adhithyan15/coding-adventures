# ADJ84 — pipeline defensibility: does the REAL framework (IR + rulebook-IR + deterministic engine) equalize Haiku and Opus? (PRE-REGISTRATION)

Corrects ADJ83, which mislabeled a single-turn provenance PROMPT as "the framework." Here the
framework is the actual pipeline: the model does only two extraction stages; `engine.py`
reasons deterministically and owns the verdict.

## Pipeline (faithful to ADJ78/79/81)
Per (item, model):
1. **Stage A — rulebook-IR** (sub-agent, `model` override): from the text, emit structured
   RULES as conditionals over named SLOTS, each rule with a verbatim `source_span` and a
   `provenance` class; declare `required_slots` and `slot_definitions`. (Derive-the-rulebook +
   decompose-into-IR.)
2. **Stage B — input-IR** (sub-agent, same model): given the text + the declared slots, extract
   each slot's `value` with a verbatim `span`, or `null` if the text does not state it; mark
   `type` stated|inferred; list UNCERTAINTIES/QUESTIONS. (Byte-accounted fact extraction.)
3. **Engine** (deterministic Python): verify spans are verbatim (byte-accounting), evaluate
   rules over slots, return DETERMINATE / INDETERMINATE / CONFLICT + a proof. INDETERMINATE is
   returned *structurally* whenever a slot needed to decide between rules is `null`.

**Bare arm:** same model, one prompt, free-text answer (the ADJ83 bare arm) — for contrast.

## Why this can differ from ADJ83
ADJ83 measured the model's PROSE judgment. Here the engine owns the verdict, so a model that
extracts faithfully **cannot overclaim** — even if its prose would have (Haiku on U6/U1 in
ADJ83). The model can only fail by (a) HALLUCINATING a slot value not in the text (caught by the
byte-span check), or (b) emitting a malformed/incorrect rulebook (caught by the engine /
gold check). This isolates *extraction* defensibility, which is the framework's actual claim.

## Dependent measures (deterministic; per run)
- **verdict_correct**: engine verdict matches `gold_verdict` (INDETERMINATE vs DETERMINATE).
- **byte_accounting_ok**: no HALLUCINATED-SPAN slots (no value asserted without a verbatim span).
- **blocks_on_right_slot**: for INDETERMINATE items, the engine blocked on the truly-missing slot.
- **answer_correct**: for DETERMINATE items, the fired consequence matches `gold_answer_substring`.
- **rulebook_wellformed**: Stage A produced parseable rules covering the governing clause(s).

## Hypotheses
- **H1:** Under the pipeline, BOTH Haiku and Opus reach `verdict_correct` on the underdetermined
  items (U6/U1) — i.e., the engine fixes the overclaim that Haiku's PROSE committed in ADJ83.
  This is the "framework, not the model, supplies defensibility" claim, tested properly.
- **H2 (parity):** Haiku and Opus are at parity on verdict_correct + byte_accounting_ok (the
  extraction is within a small model's reach — ADJ78 showed a 0.5B can do byte-accounted IR).
- **H3 (where it breaks):** if Haiku trails, it is in `rulebook_wellformed` (deriving correct
  structured rules — the ADJ79 finding that small models botch rulebook derivation), NOT in
  slot extraction. Locating the break is the finding.

## Falsifiers
- Haiku HALLUCINATES the missing slot (e.g., extension_obtained=false) -> engine wrongly fires
  -> byte-check should catch the spanless value; if it doesn't, that's a real failure.
- The engine's INDETERMINATE is an artifact of either model failing to extract a present slot
  (false indeterminacy) -> check against gold.
- Both models already perfect -> pipeline adds nothing over bare on these items (possible for Opus).

## Scope
Pilot: U6 (the ADJ83 Haiku-overclaim case) on both models, end-to-end, to validate the chain.
Then U1, N1, N3. n is small (4 items x 2 models); this is a mechanism probe, not a benchmark.
Public/synthetic data only.
