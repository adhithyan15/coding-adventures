# ADJ100 protocol

## Clarified success metric

The primary goal is **defensible and auditable work**:

- reduce wrong accepted answers;
- expose unsupported or convention-dependent answers;
- keep mechanical accuracy from collapsing;
- make accepted answers inspectable through input bytes, source bytes, and executed programs.

Raw accuracy is still reported, but it is secondary. A framework answer can be mechanically correct and
still fail strict acceptance if the parent cannot verify the bytes or rule assumptions that support it.

## Workflow

1. Use the frozen ADJ99 HLE item set from `code/specs/data/adj99-hle100-run/items_100.json`.
2. Hide gold answers from blind and framework agents.
3. Run a blind baseline agent on raw question text only.
4. Run framework proposal agents on raw question text only. Do not provide domain labels.
5. Require framework proposal agents to:
   - decompose input into typed IR spans;
   - mark discarded bytes with reasons;
   - provide URLs and exact quote strings for external facts;
   - write programs for arithmetic, logic, enumeration, simulation, or descriptor checks.
6. Parent verifier owns acceptance:
   - store input bytes in CAS;
   - fetch source bytes into CAS;
   - verify proposed quotes against fetched bytes or recorded extracted text;
   - execute programs and store source/output bytes;
   - reject answers with missing source bytes, unresolved conventions, or unsupported assumptions.
7. Score against gold only after the answer and strict-acceptance decision are fixed.

## Labels

- `blind_correct`: blind answer matches gold under the manual/equivalence scorer.
- `framework_mechanical_correct`: framework candidate matches gold, regardless of strict acceptance.
- `strict_accept`: parent accepts the framework answer as defensible.
- `strict_correct`: strict-accepted answer also matches gold.
- `wrong_accepted`: strict-accepted answer does not match gold. This is the most important failure
  metric for the clarified goal.

## Known limitations

- This is a 20-item pilot, not a statistically stable benchmark.
- Source fetching was not fully hermetic. In run 2, a broad parent web search surfaced a Hugging Face
  HLE-style mirror for one item; that source was not used as accepted evidence, and the item was
  downgraded from strict acceptance when source bytes could not be fetched elsewhere.
- PDF and rendered-document provenance still needs a stronger adapter that maps extracted text spans
  back to stable source bytes.
- Some benchmark gold answers appear to rely on hidden conventions, platform assumptions, or prompt/gold
  conflicts. The strict framework should reject these until the convention is byte-grounded.
