# ADJ61 — The Justification Gate (combine bytes → justified fact)

> **Status (2026-06-04):** Built and run. ADJ60 closed the byte-provenance invariant
> but its output gate was a *substring* test — at once too tight (an honest fact
> synthesized from several bytes has no single verbatim span) and too loose (it
> never checked the citation *supports* the claim). ADJ61 replaces it with a
> **justification** gate: combine multiple input bytes into one fact, and the
> *combination* must justify it. Re-running the ADJ60 neurobrucellosis case, the
> framework now **names the diagnosis** — as a hedged, byte-grounded inference with
> a real differential — instead of refusing. Implementation:
> [`code/specs/data/adj57/pipeline/`](data/adj57/pipeline/). Refines
> [ADJ60](ADJ60-output-grounding-gate.md).

## 1. What ADJ60 got wrong

ADJ60 asked one question of every output claim: *is a citation a verbatim substring
of the input?* That syntactic test fails in both directions:

- **too tight** — a true fact synthesized from *several* bytes ("disseminated
  granulomatous infection", combining hepatosplenomegaly + bone granuloma + fever +
  CNS lesions) has no single verbatim span; and it **gagged the conclusion**
  ("neurobrucellosis"), because an answer *name* is never a byte. ADJ60 §3 logged
  exactly this: the framework refused the right answer and drifted to a "vector-borne"
  red herring.
- **too loose** — it checks a citation *exists*, never that it *supports* the claim.
  A claim could cite any present-but-irrelevant span and pass.

The fix is to stop matching strings and start checking **justification**: *you may
combine multiple input bytes into one fact, and the combined bytes must justify it.*

## 2. The gate — two layers, claims typed

[`justify_gate.py`](data/adj57/pipeline/justify_gate.py). A claim is **grounded** iff
both layers pass:

1. **Byte-anchor (deterministic).** *Every* cited span must be verbatim in the input —
   not just one (ADJ60's rule). You cannot pad a claim with a fabricated citation. This
   is strictly stronger than ADJ60 and stays a hard, Python-checkable gate.
2. **Justification (semantic).** An adversarial verifier decides whether the cited
   bytes, *taken together*, justify the claim at its stated strength. Combining bytes
   is the point. The bar depends on the claim's **kind**:
   - **evidence** — a statement *about* the input. The cited bytes must state or
     directly imply it. (Geology's "it is tremolitized" / a "Brucella serology
     positive" that the case never reported → **reject**. This is invention.)
   - **conclusion** — an *inference from* the evidence. The cited bytes must
     collectively make it the warranted reading, and it must be **hedged as an
     inference** (a leading hypothesis), not asserted as a byte-fact. ("neurobrucellosis
     is the most likely diagnosis given <these byte-grounded findings>" → **allow**.)

A claim that fails either layer is kicked back (the ADJ06 loop): cite real bytes that
justify it, soften an over-asserted conclusion to a hedged inference, or drop it. The
deterministic layer + aggregation is unit-tested
([`test_justify_gate.py`](data/adj57/pipeline/test_justify_gate.py), 10 cases — incl.
*one fabricated citation fails the whole claim*, *a true verdict cannot rescue a
fabricated citation*, *a justified conclusion is grounded*, *an unjustified conclusion
is rejected*).

## 3. The run — the same case, flipped

[`justified.workflow.js`](data/adj57/pipeline/justified.workflow.js) +
[`run_justified.py`](data/adj57/pipeline/run_justified.py), re-run on the **identical**
neurobrucellosis bytes from ADJ60:

```
20/20 claims grounded (16 evidence + 4 conclusion); 0 rejected; 1 attempt (clean)
```

| | ADJ60 (substring gate) | ADJ61 (justification gate) |
|---|---|---|
| names the answer? | **no** — "no specific organism can be asserted" | **yes** — *"most likely … disseminated brucellosis (neurobrucellosis)"* |
| drift | toward "vector-borne" (a red herring) | insect bite kept as *portal*; rickettsial/atypical-mycobacterial held as **alternatives** |
| conclusion claims | impossible (no verbatim byte) | 4, each combining 3–7 cited spans, each verifier-justified |
| invented evidence | — | **none** — every one of the 16 evidence claims traces to verbatim bytes |

The decisive claim — *"Brucellosis with neurobrucellosis is the leading unifying
hypothesis"* — is grounded by **combining seven bytes** (`travel_to_east_africa`,
fever, `hepatosplenomegaly`, the bone granuloma, the acellular/normal-glucose CSF, the
raised protein, the meningeal enhancement). No single byte says "brucella"; the
*combination* justifies it. That is precisely the refinement: a fact built from many
bytes, grounded because the bytes justify it. The held-aside ground truth is
neurobrucellosis — reached without inventing a single evidence byte.

## 4. Honest limitations

- **Layer 2 is an LLM verdict.** The byte-anchor (layer 1) is deterministic and hard;
  the justification check is only as strict as the verifier. In this run the verifier
  *noticed* a mild overstatement — the evidence claim "malaria smear negative,
  *excluding* malaria" (a single smear is not definitive) — yet graded it justified
  because the cited byte is real and the "exclusion" is the case's own framing. That is
  the softness to harden next: a **multi-verifier vote** / a stricter refuter, so layer
  2 bites as reliably as layer 1.
- **The reject path wasn't exercised live.** The model's first pass was clean (0
  kickbacks), so we did not *watch* the gate kick back an over-assertion end-to-end
  here — that path is covered only by the unit tests. A sharp next experiment: feed a
  case (or a deliberately over-asserting deriver) that forces a live kickback, to prove
  the loop bites in practice, not just in tests.

## 5. Where this leaves the framework

> **Extended (2026-06-04):** [ADJ62](ADJ62-input-justification.md) applies this exact
> gate to the **input** side (`extracted` ≙ evidence, `inferred` ≙ conclusion), so every
> fact — pulled in *or* pushed out — is anchored to the bytes that justify it.

Both directions of byte provenance now hold *and* the framework can answer under them:
every input byte accounted for (ADJ57/58), every evidence claim traced to input bytes,
and the conclusion stated as a justified, hedged inference over **combined** bytes —
with the differential the evidence leaves open. The next hardening is layer 2's
strictness (multi-verifier) and a live kickback demonstration.
