# ADJ77 — framework-level scaffold that makes small local models reason

Motivation: only small local models deploy in airgapped / compliance environments, so the
framework must make *them* capable. ADJ76 diagnosed the monolithic-contract failure as
multi-objective instruction overload (not truncation). This builds and tests a
framework-level fix.

## Two scaffold designs

**v1 — structured atomic (FAILED, instructive).** Framework owns segmentation + resolution;
per-clause the model answers a rigid schema `APPLIES: yes/no | VALUE: <v>`. On 0.5b this
collapsed: the model emitted bare "NO" (wrong judgment *and* wrong format), so no clause
was ever counted and the resolver defaulted to "none" for every item. The apparent 0.58
was a scoring artifact ("none" coincidentally matches the zero-concept items).
**Lesson: a rigid output schema is itself multi-objective overload for a tiny model** — v1
repeated the monolithic mistake at the clause level. (Probe: asked an *open* question the
same 0.5b reasoned correctly; asked the *tagged* question it said "NO".)

**v2 — NATURAL atomic (WORKS).** Two natural calls, no schema:
  1. **FOCUS** — "describe only the specific category/characteristics of the subject"
     (directs attention to the subject's distinguishing attributes).
  2. **ANSWER** — re-ask with the focus prepended + "an exception overrides the general
     rule; use the rule that specifically applies." Framework parses only FINAL ANSWER,
     leniently.

## v2 results — monotonic across the ladder, never hurts

| model | bare PS | v2 PS | numeric-only (artifact-proof) |
|---|---:|---:|---|
| qwen2.5:0.5b | 0.25 | **0.50** | 2/5 → 4/5 |
| qwen2.5:1.5b | 0.50 | **0.67** | 4/5 → 5/5 |
| qwen2.5:3b | 0.83 | **1.00** | 5/5 → 5/5 |

v2 improves accuracy at **every** scale and never hurts — unlike free-form staging
(ADJ74), which helped 0.5b but *hurt* 1.5b (drift over 5 turns). The numeric-only subset
(where the "none" artifact cannot accidentally score) confirms the lift is real:
0.5b 2→4, 1.5b 4→5, 3b 5→5.

## The validated design principle

For small local models, the framework-level scaffold must:
1. **Decompose into NATURAL single-focus steps** — one plain question at a time. Do NOT
   impose a rigid output schema; the schema is itself the overload (v1).
2. **Keep steps few and targeted** — 2 natural calls (focus → answer) beat 5 free-form
   turns (which drift). The framework owns the control flow and parses leniently.
3. **Carry the rule-precedence hint** — "an exception overrides the general rule" — so the
   model applies the specific rule rather than the salient general one.

This is the actionable answer to "why can't small models reason, and how do we fix it at
the framework level": they fail the monolithic contract via instruction overload (ADJ76);
they are rescued by a small number of natural, single-focus steps with framework-owned
control flow (ADJ77). It directly serves the airgapped/compliance deployment regime.

## Honest limitations
- n=12, one item family (rule-with-override), one model family (Qwen). 0.5b is still only
  0.50 — the scaffold lifts but does not saturate the smallest model.
- The win is on a controlled synthetic task; validation on more families and the real-case
  regime (Palmyrene-style) is next.
- v2's FOCUS step occasionally surfaces the answer directly; that is acceptable (the
  framework using a focusing step to make the answer tractable is the intended mechanism),
  but the per-step provenance/justification discipline still needs to be layered back on
  for the full contract — here we isolated the reasoning lift.
