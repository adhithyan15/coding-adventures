# ADJ97 — does Haiku self-auditing its own trail raise accuracy? No (auditability ≠ self-repair)

Natural payoff of ADJ96 (which showed the framework trail makes Haiku's errors *locatable* by an
external Opus auditor, 5/5). Question: can Haiku find and fix its *own* errors by walking its own
trail before finalizing? Per item (6 reasoning-bound items, known errors), 3 arms sharing one
Haiku-built CAS: **base** (no self-audit), **self-audit** (Haiku reviews+revises its own chain,
iterating until it stops finding errors), **opus-audit** (Opus reviews+revises the *same* Haiku trail
= ceiling). Deterministic grading; gold never shown to any solver/auditor.

## Result — self-audit did NOT raise accuracy
| item | gold | base | self-audit (Haiku) | opus-audit (ceiling) |
|---|---|---|---|---|
| divisors | 432 | 720 ✗ | 720 ✗ — **0 iters, found nothing** | 640 ✗ (changed) |
| river | 3v₀/16v_r·L | 9v₀L/8v ✗ | 9v₀L/8v ✗ — **0 iters, found nothing** | 9v₀L/8v ✗ (0 iters) |
| Al(OH)₃ | 1.776e-3 | 1.2e-7 ✗ | 1.2e-7 ✗ — **0 iters, found nothing** | 1.2e-7 ✗ (*derived 1.8e-3, rejected it*) |
| LSM-tree | 321 | 968 ✗ | ">320 / 321 valid" ~partial (3 iters) | **321 ✓** (fixed, 1 iter) |
| LoRaWAN | SF9/6dBm | SF9/8dBm (partial) | **SF10/11/14dBm ✗ — regressed** (3 iters) | SF8 ✗ (regressed, 3 iters) |
| integral | (5482 — wrong gold) | 5487 ✓ | 5487 — correctly left alone | 5487 — left alone |

**Self-audit net: 0→0 correct, plus one regression (LoRaWAN). Opus ceiling: 0→1 (LSM), with two
regressions/churns (LoRaWAN, Al(OH)₃).**

## Three findings
1. **Haiku is largely blind to its own errors.** On 3 of 5 wrong items (divisors, river, Al(OH)₃) the
   self-audit ran **0 iterations — reviewed its own chain and found nothing wrong.** It cannot see its
   own dropped K_f, false-multiplicativity premise, or broken velocity profile.
2. **When it engages, it over-corrects.** LoRaWAN: deep self-scrutiny (3 iters) **regressed** a correct
   SF9 into a wrong SF10/11. LSM: found the right structure (320 < E ≤ 968) but muddled the answer to
   ">320" instead of committing to 321.
3. **It does not falsely correct a *right* answer** (integral — left alone). So it is neutral-to-
   negative, not pure damage.

## The pivotal contrast — self-audit ≠ external audit
ADJ96's auditor caught **5/5**; ADJ97 self-audit catches ≈0 and regresses one. The only difference is
**stance**: ADJ96 = a *dedicated skeptical review of someone else's work*; ADJ97 = *"double-check your
own work."* That framing is dramatically weaker — and weaker *even for Opus*: the ceiling fixed only
**1/5** (vs 5/5 in ADJ96), regressed LoRaWAN, and on Al(OH)₃ **Opus derived the correct 1.8×10⁻³ and
then rationalized back to the wrong 1.2×10⁻⁷.** A model auditing its own reasoning *defends* it; a
model auditing a foreign trail *attacks* it. (Consistent with the whole byte-provenance line: the
ADVERSARIAL, EXTERNAL skeptic is what works — ADJ91 grounded adversarial read, ADJ96 external auditor.)

## Architectural lesson
The audit trail's value is enabling an **independent adversarial auditor** (a separate agent or a
human) to catch errors — **not self-repair**. Do NOT ask the cheap model to double-check itself (it
rationalizes and over-corrects). Route the auditable trail to a separate skeptic. The 5/5 lives in
external audit; self-review gets ≈0. For a correction loop, the auditor must be a *different* role/
instance in adversarial framing — ideally one that does not "own" the reasoning it is checking.

## Honest caveats
- N=6, single run/arm. Directional. Deterministic grading; the integral's gold (5482) is itself wrong
  (5487 is correct) so only 5/6 had a real error to fix.
- The self-audit prompt ("double-check your own work, be honest") is one framing; a more adversarial
  self-prompt might do better, but the ADJ96-vs-ADJ97 gap (external vs self) is large and the
  Opus-ceiling underperformance in the self-revision framing corroborates that stance, not capability,
  is the lever.

## Next
- Test a **separate-instance adversarial auditor** in a correction loop (fresh agent, skeptic framing,
  does not own the chain) — the ADJ96 stance applied to *fix*, not just *localize* — and measure
  accuracy lift without the self-rationalization penalty.
