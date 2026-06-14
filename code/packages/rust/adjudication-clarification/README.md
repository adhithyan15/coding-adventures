# adjudication-clarification (Rust)

ADJ06 — clarification dialogue. When a checker pass surfaces a
violation, re-prompt the LLM with the structured diagnostic and try
again.

## Why this matters

The adjudication framework's design principle is that **structured
checkers + a re-prompt loop make small local models do extraordinary
work.** ADJ06 is the re-prompt loop.

A 7B-class local model often gets the IR slightly wrong on the first
try — a 1-byte coverage gap, a wrong-typed term, a missing query
node. Without ADJ06 we'd be stuck with a Blocked verdict. ADJ06 turns
that situation around: the deterministic checker tells the model
exactly what went wrong, the model tries again, the checker re-runs,
and the model gets it right on the second or third try.

The model didn't get smarter; the system gave it feedback.

## What v0.1.0 ships

- `retry_decompose_on_coverage_failure(req, gateway, max_attempts, now)` —
  the headline entry point. Takes the original `DecomposeTextRequest`,
  the violation description from ADJ02, and the previous IR JSON.
  Re-prompts up to `max_attempts` times.
- `CoverageClarificationRequest` / `CoverageClarificationOutcome` —
  the typed in/out shapes.
- `ClarificationError::Exhausted` carries the full dialogue trail so
  callers can escalate (Rung 2 / Rung 3) with context.
- `CLARIFICATION_PROMPT_VERSION = "clarification-v1"` so the audit
  trail records which prompt version produced each turn.
- Every retry is recorded as an `adjudication_audit_trail::DialogueTurn`
  with `rung = Rung1ReprompT`, the violation text in `question_text`,
  the model's response in `response.text`, and
  `(prompt_version, prompt_hash)` for replay.

## What v0.1.0 deliberately does NOT do

- **Re-validate the corrected IR.** That's the pipeline's job. We
  hand the new IR back; the caller re-runs ADJ02 on it.
- **Other violation kinds.** ADJ03 / ADJ04 / ADJ05 have their own
  correction shapes and land in follow-ups. v0.1 focuses on coverage
  because that's the most common small-model failure mode.
- **Rung 2 (different model) / Rung 3 (human).** v0.1 stays at
  Rung 1 (same model).

## How the correction prompt works

```text
Your previous IR was REJECTED by the ADJ02 coverage checker.

Violation:
  RootsDoNotTileDocument { missing_ranges: [(2, 3)] }

The coverage rule is non-negotiable: every byte of SOURCE must be
covered by exactly one non-Query node's source_spans. Whitespace and
punctuation count. If a byte is intentionally outside the domain,
assign it to a `Discarded` node with a `discard_reason` like
`Pleasantry` or `DocumentMetadata`.

Your previous output was:
{
  "document_id": "doc1",
  "nodes": [ ... your IR ... ]
}

Produce a CORRECTED IR with the same `document_id`, fixing the
coverage gap. Same flat-array shape, same field names, same rules as
before.
```

The model sees the exact violation, its own prior output, and a
specific corrective instruction. This is much more useful than just
re-asking the original question — the model has the structural
diagnostic it needs to fix the specific mistake.
