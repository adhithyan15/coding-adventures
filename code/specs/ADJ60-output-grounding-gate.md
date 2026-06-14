# ADJ60 — The Output-Grounding Gate (bidirectional byte provenance)

> **Status (2026-06-04):** Built and run end-to-end. The byte-provenance invariant
> is now enforced in **both** directions — input coverage (nothing dropped) AND
> output grounding (nothing invented) — and verified on a fresh case. The experiment
> also surfaced the next cut (ADJ61): the gate must distinguish *evidence claims*
> from the *conclusion*. Implementation:
> [`code/specs/data/adj57/pipeline/`](data/adj57/pipeline/). Answers the question
> ADJ59 §6 posed. Builds on [ADJ58](ADJ58-universal-stage-contract.md).

## 1. The missing half of the invariant

ADJ57/58 enforce **input coverage**: every input byte is used or
discarded-with-reason (*nothing dropped*). They do not enforce the dual —
**output grounding**: every output claim must trace back to input bytes
(*nothing invented*). That gap is how the ADJ59 geology over-specification slipped
through: the answer asserted "tremolitized"/"diopside" with no supporting bytes
(no FTIR/Raman in the scenario), smuggled from training.

The output-grounding gate ([`output_gate.py`](data/adj57/pipeline/output_gate.py)):
a claim is **grounded** iff at least one of its citations is a *verbatim* span of the
allowed input (the case text + the used-fact terms). A claim with no retrievable
citation is **ungrounded** — it came from outside the input — and is **rejected and
kicked back** (the ADJ06 self-correction loop) to re-derive: ground it or drop it.
Together with input coverage, the output becomes a pure **function of the input
bytes**, traceable both ways.

## 2. The run — bidirectional provenance verified

A fresh case (neurobrucellosis, PMC2769393), decomposed and answered under both
gates ([`grounded.workflow.js`](data/adj57/pipeline/grounded.workflow.js) +
[`run_grounded.py`](data/adj57/pipeline/run_grounded.py)):

```
INPUT grounding   100% COVERAGE: 25 facts + 1 reasoned discard = all 1812 bytes (clean)
OUTPUT grounding  20/20 claims grounded — every claim cites a verbatim input span
=> BIDIRECTIONAL PROVENANCE COMPLETE
```

Every claim maps to its source span, e.g. *"The patient traveled through East
Africa"* ← `"had gone on a business trip to Africa traveling…"`.

## 3. The finding — the gate conflates *evidence* with *conclusion*

The held-aside answer is **neurobrucellosis** — but `brucella` appears nowhere in
the case bytes (the serology was held back). So the strict gate **forbade naming
it**, and the model generalized to a fully byte-grounded but *under-committed*
answer: *"an insect-transmitted infection causing meningoencephalitic involvement…
the case text does not name a specific organism."* It even drifted toward
"vector-borne," a **red herring** (brucellosis is dairy/animal-acquired; the bite
was just the inoculation site the bytes happened to mention).

The principle needs one more cut, which the experiment isolates by contrast:

| claim | type | byte-grounded? | correct call |
|---|---|---|---|
| geology "it is **tremolitized**" | **evidence** (a property of the specimen) | no — needs FTIR not in input | **reject** ✓ |
| "this is **neurobrucellosis**" | **conclusion** (inference naming the pattern) | the *name* is not a byte, but every *supporting fact* is | **allow** (as a flagged inference) |

"Tremolitized" is a *false evidence claim* — it asserts the input shows something it
doesn't. "Neurobrucellosis" is a *diagnostic inference* — it names what the
byte-grounded evidence indicates, using domain knowledge. The current gate treats
both as ungrounded (no verbatim byte → reject), so it correctly kills **invention**
but also gags the legitimate **conclusion**.

## 4. Next — ADJ61: split evidence from conclusion

> **Delivered (2026-06-04):** [ADJ61](ADJ61-justification-gate.md) replaces this
> substring gate with a **justification** gate (combine bytes → justified fact, claims
> typed evidence vs conclusion). Re-running this exact case, the framework now *names*
> neurobrucellosis as a hedged, byte-grounded inference. The plan below is what it built.


The output-grounding gate should classify each output element:
- **Evidence claim** (a statement about the input) → must trace to input bytes
  (strict; rejects "tremolitized").
- **Conclusion / hypothesis** (an inference from the evidence) → allowed to be a
  *named* answer, **provided** (a) it rests only on byte-grounded evidence and (b) it
  is flagged AS an inference, not asserted as a byte-fact.

Then the framework can answer *"neurobrucellosis — inferred from [these byte-grounded
findings]"* without smuggling a single false evidence claim — completing the
invariant without gagging the conclusion. Both halves of byte provenance are now in
place; ADJ61 is the cut that lets the framework actually *answer* under them.
