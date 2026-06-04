# ADJ64 — The Underdetermination Gate (over-attribution under missing evidence)

> **Status (2026-06-04):** Built and run. The dual of invention. ADJ60/61 stop a claim
> from citing bytes that are not there; ADJ64 stops a *conclusion* from singling out one
> cause when the datum that would distinguish it from the rivals **is not in the input**.
> Run on the [ADJ63](ADJ63-bidirectional-end-to-end.md) axle conclusion, it flags it
> **underdetermined** and names five missing measurements — including the exact one
> (operating stress vs. fatigue limit) that is the held-aside ground-truth root cause.
> Implementation:
> [`underdetermination.py`](data/adj57/pipeline/underdetermination.py) +
> [`underdetermination.workflow.js`](data/adj57/pipeline/underdetermination.workflow.js).

## 1. Invention is not the only way to be wrong

"If the data is not present, we cannot reason over it." True — and the byte-anchor
enforces it: you cannot derive a fact the bytes do not entail. But ADJ63 showed the model
has a *second* way to go wrong that the byte-anchor does not catch. When the observation
that would decide between rival explanations is **absent**, the model does not fabricate
it (the gate holds) — it lets the conclusion **drift to the loudest present lever** and
names one cause anyway. The axle answer named "machining/surface" as the root cause; the
truth was "operating stress exceeded the fatigue limit." Both fit the *same* bytes; the
discriminating measurement was simply never decomposed. Every claim was grounded; the
*selection among causes* was not.

So a conclusion can be **underdetermined**: more than one hypothesis fits the present
bytes, and the datum that separates them is missing. The honest move is not to guess — it
is to keep the conclusion as a **disjunction** over the live rivals and emit the missing
observation as a **named provenance hole**: a query the spider/CAS can fetch. *A single
step cannot reason over absent data; naming the hole is how the loop turns absence into
the next thing to retrieve.*

## 2. The gate

[`underdetermination.py`](data/adj57/pipeline/underdetermination.py). For each rival
hypothesis that fits the same bytes, the model supplies the single **discriminating
observation** that would settle leading-vs-rival, and whether it is PRESENT (with a
verbatim citation) or ABSENT. The deterministic rule, mirroring every other gate —
*"present" is only ever as good as a real byte*:

- a rival is **resolved** iff its discriminating observation is marked present **and** the
  citation is verbatim in the input (you cannot *claim* the data is there without a byte);
- otherwise the rival is **open** — its discriminating observation is a **hole** (absent,
  or a fabricated "present").

The conclusion is **determined** iff no rival is open. If any rival is open it is
**underdetermined**: soften to the disjunction over the open rivals, and report the holes
as required-but-missing data. The model proposes the rivals; this module adjudicates. 5
unit tests ([`test_underdetermination.py`](data/adj57/pipeline/test_underdetermination.py)),
incl. *claimed-present-but-fabricated-citation is open* and *absent-datum becomes a hole*.

## 3. The run — the gate catches the axle over-attribution

[`underdetermination.workflow.js`](data/adj57/pipeline/underdetermination.workflow.js) +
[`run_underdetermination.py`](data/adj57/pipeline/run_underdetermination.py), on the ADJ63
leading conclusion:

```
7 rivals — 2 resolved by present bytes, 5 OPEN — conclusion UNDERDETERMINED
```

The five **named provenance holes** (queries to fetch before a cause can be singled out):

1. the fillet radius / stress-concentration factor Kt vs. the design allowable;
2. **a comparison of operating bending stress at the fillet to the EA1N fatigue limit;**
3. fretting-damage evidence at the wheel-seat contact vs. a clean free-surface origin;
4. the required surface residual-stress state (was compressive specified and absent?);
5. the drawing surface-finish tolerance vs. the measured 342 µm step / 0.95 mm marks.

Hole **#2 is the ground-truth root cause** — the exact measurement whose absence let
ADJ63 over-attribute to "machining." The gate replaced the single-cause answer with a
disjunction that keeps every grounded finding (bending fatigue at the fillet; surface
concentrators present; material clean) and refuses to choose among the five mechanisms
until the missing data are retrieved.

## 4. Honest limitations

- **The "resolved" verdict is an LLM judgment.** The deterministic gate verifies the
  citation is verbatim, but cannot verify the cited datum actually *discriminates*. Two
  rivals here were marked resolved on citations that merely show a feature is *present*
  (e.g. "corrosion pits exist") without truly settling which feature nucleated the crack.
  Read conservatively, those two are *also* open — which would only make the conclusion
  **more** underdetermined, never less. So the gate erred toward calling rivals resolved,
  yet still reached the safe verdict (UNDERDETERMINED). Hardening — a multi-verifier vote
  on "does this cited byte actually discriminate?" — is the same fix flagged for ADJ61/62.
- **Rival completeness is the model's.** The gate adjudicates the rivals it is given; it
  cannot prove the rival set is exhaustive. A missing *rival* (not just a missing datum)
  is a deeper hole — a candidate for the completeness-critic pattern.

## 5. Where this leaves the framework

The framework now distinguishes three epistemic states, all byte-disciplined: a claim is
**grounded** (cites real bytes), a conclusion is **determined** (the present bytes
discriminate it from its rivals), or it is **underdetermined** (and the missing
discriminators are named as queries). It will no longer convert a missing measurement into
a confident single cause — it converts it into a question. That is the antidote ADJ63
asked for: not reasoning over absent data, but making the absence the next thing to fetch.
