# ADJ96 — auditability: can a reasonable human catch exactly where Haiku went wrong?

The question that matters most for the thesis: the framework doesn't make Haiku *correct* — does it
make Haiku's errors *locatable*? Test: give a **blind auditor** (Opus as a domain-competent reviewer,
**no answer key**, told to audit the given reasoning rather than re-derive) the framework-Haiku audit
trail (sourced CAS facts + a cited reasoning chain) vs the plain-Haiku output, and see whether it can
pinpoint the exact step where Haiku went wrong. An **oracle** (Opus + the gold answer) establishes the
true error locus for scoring. **Same auditor on both arms**, so the delta is the *trail's*
auditability, not the auditor. 6 reasoning-bound items (errors live in the chain). Blind LLM auditor =
proxy for a reasonable human reviewer.

## Result
**Framework trail: 5/5 errors pinpointed (hit). Plain output: 2 hit, 1 partial, 2 miss.**

| item | Haiku's wrong answer | true error (oracle) | framework trail | plain output |
|---|---|---|---|---|
| divisors | 720 | falsely assumed `f` multiplicative (step 1) | **hit** (reproduced counterexample `f(21)=2≠f(3)f(7)`) | **miss** — *validated the wrong assumption* |
| river | 9v₀L/8v | rising velocity branch used past the center | **hit** | partial (right area, mischaracterized) |
| LSM-tree | 484 B | buffer-entry-count `4096/121` (step 2) | **hit** | hit |
| LoRaWAN | SF9/**10**dBm | mid-range power, not the minimum (step 7) | **hit** | **miss** — chased a different issue |
| Al(OH)₃ | 1.18×10⁻⁷ | dropped K_f / `[OH⁻]=3s` (steps 3,8) | **hit** | hit |
| integral | 5487 | *none* — gold `5482` is itself wrong | n/a | n/a |

## The two decisive cases
- **divisors:** the framework chain stated its load-bearing premise explicitly ("`f` is multiplicative
  → the divisor-sum factors"), so the blind auditor checked it, found it false, and **reproduced the
  exact counterexample**. The plain prose buried the same premise — and the auditor was **fooled into
  affirming it as valid** (`found_flaw=false`). The trail converted a hidden fatal assumption into a
  checkable line.
- **LoRaWAN:** the chain's step 7 visibly chose "mid-range 10 dBm … for headroom," contradicting its
  own stated "minimize energy" goal — caught instantly. In the plain answer that choice was invisible,
  so the auditor pursued an unrelated (wrong) issue and missed the real error.

## A bonus: the trail catches errors in the answer key
On the **integral**, the framework trail (and the oracle) show Haiku's `5487` is **correct** and the
**HLE gold `5482` is wrong**. Auditability cuts both ways — it localizes the model's errors *and*
surfaces errors in the reference.

## The thesis, sharpened
**The framework doesn't make Haiku correct — it makes Haiku's errors locatable.** Every Haiku failure
mode catalogued across ADJ88→95 (dropped facts, false premises, execution slips, unjustified choices)
becomes a **specific, checkable line** a reasonable reviewer can find **without the answer key** —
5/5 vs 2/5 for plain. This is "auditable and correctable, not correct": the human as auditor, not
author, demonstrated.

## Honest caveats
- **N=6, single oracle/auditor/scorer (all Opus).** Directional. The blind auditor is an LLM proxy
  for a reasonable human reviewer (domain-competent, blind to the gold); a human study would be the
  rigorous version.
- The auditor was explicitly told to audit the given reasoning, not re-derive — but a strong auditor
  can still partly re-derive; the **framework-vs-plain delta with the same auditor** is the controlled
  signal, and it is large (5/5 vs 2/5).
- One item (integral) had a wrong gold, so only 5 of 6 had a real error to localize.

## Companion (Haiku failure-mode catalogue, ADJ88→95)
Dropped input (K_f) · mention-not-use laundering · execution-floor blowups · localized coefficient
slips · over-abstention · weak-spider retrieval gaps · context contamination · wrong domain knowledge ·
shared hard-problem floor · inheriting the CAS-builder's mistakes. Almost all are *localized* failures
— which is precisely why the audit trail makes them catchable.
