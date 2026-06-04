# ADJ62 — Input Justification (extract/infer → which bytes → why)

> **Status (2026-06-04):** Built and run. ADJ61 put the justification gate on the
> *output*. ADJ62 puts the **same gate on the input**: after decomposing, the
> framework asks the agent *"what facts did you extract or infer, which bytes do they
> come from, and why do those bytes prove it?"* Coverage proved nothing was *dropped*;
> this proves nothing was *mis-extracted*. Run on the neurobrucellosis bytes, it forced
> the decomposer to separate 41 **extracted** facts from 8 **inferred** ones — catching
> interpretations it would otherwise have smuggled in as fact. Implementation:
> [`code/specs/data/adj57/pipeline/`](data/adj57/pipeline/). Refines
> [ADJ61](ADJ61-justification-gate.md) by making its gate stage-symmetric.

## 1. The gap coverage left open

ADJ57/58 enforce **coverage** on the input: every byte is used or
discarded-with-reason — *nothing dropped*. But coverage says only that the decomposer
*touched* every byte; it never checks that the facts it claims to have pulled out are
**faithful** to those bytes. A decomposer can:

- attach a fact to a span that does not state it (mis-grounding), or
- silently **infer** a fact (a reading, a label, a clinical synthesis) and present it
  as something the text **extracted** — the input-side form of the exact invention
  ADJ61 catches on the output.

Coverage is blind to both. The fix is the user's: turn the question around on the
decomposer — *"what did you take from these bytes, and why do the bytes prove it?"* —
and run the ADJ61 gate on the answer.

## 2. The same gate, made stage-symmetric

[`justify_gate.py`](data/adj57/pipeline/justify_gate.py) now serves both ends. A fact is
**grounded** iff:

1. **byte-anchor (deterministic):** every cited span is verbatim in the input.
2. **justification (adversarial verifier):** the cited bytes, combined, justify the
   fact at its stated **kind** —
   - **extracted** (strict) — the bytes *state* it. If the fact adds any reading the
     bytes do not contain, it is **not** extracted → rejected (re-file as inferred).
   - **inferred** (hedged) — the bytes *warrant* it as a defensible interpretation,
     flagged as an inference, not smuggled in as a byte-fact.

This is the input mirror of evidence/conclusion: `extracted ≙ evidence`,
`inferred ≙ conclusion`. One gate, both directions
([`justify_input.workflow.js`](data/adj57/pipeline/justify_input.workflow.js) +
[`run_justify_input.py`](data/adj57/pipeline/run_justify_input.py); 15 gate tests incl.
the four input kinds).

## 3. The run — the gate does real epistemic work

Decompose → "account for what you took" → adversarial verify, on the neurobrucellosis
bytes:

```
COVERAGE    100%: 27 fact-segments + 3 discards = all 1812 bytes (nothing dropped)
EXTRACTION  49/49 facts grounded — 41 extracted + 8 inferred — 0 rejected, clean
=> INPUT PROVENANCE COMPLETE
```

What makes this more than coverage is the **8 facts the gate forced into the *inferred*
column** — each one a reading the text does not literally state:

| inferred fact | why it is NOT extracted (the bytes never say it) |
|---|---|
| **the patient is male** | the case never says "male" — only the pronoun *"He"*. Sex is *inferred*. |
| Uganda/Tanzania/Kenya are **East African** | the text says only *"Africa"*; the region is geographic knowledge. |
| antibiotics gave a **partial response** | a reading of *"minor relief"*. |
| **hepatosplenomegaly** | a composite of separately stated hepatomegaly + splenomegaly. |
| **albuminocytologic dissociation** | a clinical label for *acellular + raised protein*. |
| the hand tremors are the **known essential tremor** | a correlation of two separately stated facts. |
| **CNS/meningeal involvement** | a synthesis of meningeal enhancement + pontine lesions + raised CSF protein. |
| a returning-traveler **syndrome** | a clinical summary across four findings. |

Conversely, the verifier kept *"the patient had tachycardia"* as **extracted** — because
the word *"tachycardia"* appears verbatim — rather than the model's prior instinct to
call it an interpretation of "pulse 116". The gate separated **what the text says** from
**what the reader brings to it**, byte by byte. *"The patient is male"* is the sharpest
case: a fact almost everyone would treat as given, which the bytes only ever *imply*.

## 4. Honest limitations (unchanged from ADJ61)

- **Layer 2 is an LLM verdict** — only as strict as the verifier. Here it graded every
  fact justified on the first pass; the kind-discipline held up under spot-reading, but a
  **multi-verifier vote** is still the hardening that makes layer 2 bite as reliably as
  layer 1.
- **The reject path was not exercised live** (clean first pass — covered only by unit
  tests). A decomposer that deliberately mislabels an inference as extracted would prove
  the kickback bites in practice.

## 5. Where this leaves the framework

Both *ends* of the pipeline now carry both *halves* of provenance:

| | nothing dropped | nothing invented / mis-extracted |
|---|---|---|
| **input** | coverage (ADJ57/58) | **extraction justification (ADJ62)** |
| **output** | — | grounding + justification (ADJ60/61) |

Every byte is accounted for, and every fact — pulled in *or* pushed out — is anchored to
the bytes that justify it, with extracted facts separated from honest inferences. The
next hardening is the shared one: a multi-verifier vote on layer 2, and a live kickback
demonstration at either end.
