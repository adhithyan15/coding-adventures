# ADJ81 — end-to-end: a 0.5B that only extracts facts drives a compiled rulebook to correct answers

Tests the deployment bet: a small local model doing ONLY fact extraction can drive a
pre-compiled rulebook ("library") to the correct answer, with reasoning done
deterministically on CPU — the airgapped/compliance regime where no frontier model and no
network are available.

## Pipeline
1. **Library = compiled rulebook** (`leave_library.py`): general rule 20 days; override
   part-time hired after 2020 → 12. Declares a SCHEMA of the facts it needs. (Represents
   the OFFLINE capable-model derivation + compilation; cf. ADJ71 CAS program-cache on a
   real legal source.)
2. **Runtime 0.5B = fact extraction only**: one natural question per SCHEMA slot.
3. **Framework emits the program** deterministically from the extracted facts (imports the
   library, calls it). Execute on CPU.

## Result (qwen2.5:0.5b)

| arm | correct |
|---|---|
| **A — 0.5B extracts → framework emits program → library runs** | **5/5** (incl. both override cases and the override-doesn't-apply case) |
| B — 0.5B writes the whole program freehand | **0/5** (SyntaxErrors, garbage) |

The 0.5B never reasons; the compiled library does, deterministically. It **cannot** write
the program itself — so the **framework** must emit it. Division confirmed: **model
extracts, framework emits, library reasons.**

## A real extraction-design finding (caught mid-experiment)
Categorical slot extraction is prompt-sensitive on a 0.5B:
- "Answer one word: full or part" → **"full"** for everything (forced either/or biases to
  the default) — this faked a 3/5 (only the genuinely-full-time cases scored).
- **"Copy the exact words describing employment status"** → "…part-time employee…" —
  correct AND byte-anchorable.
**Rule:** extract categorical slots by COPY-THE-PHRASE (the 0.5B's strength + free
provenance), never forced either/or. Numeric slots ("what year?") already work.

## Why this matters
This is the capstone of the small-model deployment thread:
- ADJ78: a 0.5B builds a byte-accounted compact IR (facts).
- ADJ79: a 0.5B should NOT derive rulebooks (knowledge-bound) → derive offline on the
  capable model, compile to a library.
- **ADJ81: a 0.5B doing only extraction + a framework-emitted program + the compiled
  library = correct answers, including overrides.**
The reasoning is in the deterministic, auditable, CPU-bound library; the tiny local model
only extracts facts (with provenance). That is the airgapped/HIPAA-deployable shape, end to
end.

## Limitations
- 5 cases, one domain; the library is hand-built here (representing the offline
  capable-model derivation, which ADJ71 proved separately on a real legal source). A fully
  autonomous chain is ADJ79-on-capable → compile (ADJ71) → this runtime.
- The 0.5B's extraction reliability depends on copy-phrase prompts; needs validation across
  more schemas/domains and noisier inputs.
