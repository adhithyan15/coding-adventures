# ADJ73 — mechanistic omission ablation: pre-registration (before running)

## Question
Is **justified-discards** (not mere coverage) the active ingredient by which the
byte-accounting contract attacks **omission**-class hallucination?

## Design
- **Item type:** rule-with-buried-override. A passage states a salient general rule,
  then a specific override/exception; the case falls under the override. The salient
  general rule pulls attention toward a wrong answer (the "skim trap").
- **Strata:**
  - **PS (present-but-skimmed):** the answer-determining override clause IS in the
    passage. Correct = override answer; skim-failure = general-rule answer.
  - **AB (absent):** same passage structure, but the question asks about a category the
    passage does not cover. Correct behavior = abstain ("not specified").
- **Conditions (prompt only; same model):**
  - **BARE:** answer the question.
  - **COVERAGE:** list every clause, mark [USE]/[DISCARD] — discards need NO reason.
  - **JUSTIFIED:** list every clause, mark [USE]/[DISCARD] — every [DISCARD] requires a
    specific reason why that clause does not apply.
- **Models (open-weights via Ollama):** qwen2.5:1.5b, qwen2.5:3b, gemma4:latest,
  llama3.1:8b. Temperature 0.
- **Metric:** accuracy per (model × stratum × condition); for PS also the skim-trap rate;
  for AB the abstain vs fabricate rate.

## Pre-registered predictions
1. **PS accuracy: JUSTIFIED > COVERAGE ≈ BARE.** The mechanism is that to discard the
   override clause you must justify it, which forces engagement with the override and
   surfaces that it applies. Mere coverage lets the model tag the override [DISCARD]
   without reasoning — i.e., skim it the same as bare.
2. **AB:** JUSTIFIED should NOT inflate fabrication; ideally it raises abstention (forcing
   a reason for using/discarding makes "the passage doesn't cover this" explicit). The
   framework should not create knowledge — AB accuracy (abstention) should be ≥ bare.
3. **Scale:** the COVERAGE≈BARE vs JUSTIFIED gap should be largest where the model is
   capable enough to follow the justification instruction but still prone to skimming
   (mid-ladder). Below a floor, instruction-following collapses.

## Falsifiers
- If COVERAGE ≈ JUSTIFIED on PS, the active ingredient is mere *coverage*, not
  justification — the hypothesis is wrong.
- If JUSTIFIED lifts PS but also inflates AB fabrication, the contract trades omission for
  commission — a bad trade to report honestly.
