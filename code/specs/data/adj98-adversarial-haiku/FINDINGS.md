# ADJ98 — a fresh, context-free, adversarial Haiku catches errors self-audit missed (stance is the lever)

ADJ97 showed Haiku self-auditing its own trail ("double-check your work") fails (~0 detection) and
argued the cause is STANCE (defend your own work) not capability. This isolates it: on ONE shared base
trail per item, three auditors localize the flaw — same fresh-agent, no-shared-context setup; the only
knobs are **model × framing**:
- **adv-Haiku** — fresh Haiku told it reviews a SEPARATE, SUSPECT system (attack stance) — THE TEST
- **adv-Opus** — fresh Opus, same attack stance (ADJ96 replication)
- **self-Haiku** — Haiku told it is its OWN work (defend stance, ADJ97 replication)

An Opus oracle (+gold) sets the true error locus; a scorer grades each hit/partial/miss. A fresh Haiku
then CORRECTS using the adv-Haiku flag. 6 reasoning-bound items (5 with a real error; 1 — the integral —
is actually correct with a wrong gold). No hints; gold never shown to solver/auditors/corrector.

## Detection result (5 items with a real error)
| auditor | stance | hit | partial | miss |
|---|---|---|---|---|
| self-Haiku | defend ("your work") | 1 | 0 | 4 |
| **adv-Haiku** | **attack ("foreign suspect")** | **2** | **1** | 2 |
| adv-Opus | attack | 4 | 1 | 0 |

**Same cheap model, only the framing changed: detection tripled (1/5 → 3/5).** Stance is a real, cheap
lever — the decisive confirmation of ADJ97: self-audit fails because of the *defend* stance, not (only)
capability. A separate adversarial auditor is a genuinely better reviewer than self-review.

## Per-item (advH detection → correction)
| item | true error | adv-Haiku detect | catch→fix (base → corrected) |
|---|---|---|---|
| divisors | coupled-parity f(d) formula | **miss** (subtle number theory) | 720 → 720 ✗ |
| river | velocity profile past center | **miss** (physics) | wrong → wrong ✗ |
| LSM | min/max constraint inverted | partial | 968 → muddled ✗ (flagged, fix failed) |
| LoRaWAN | SF7 infeasible / power | **hit** | SF7 → SF7 ✗ (flagged, **fix re-justified wrong answer**) |
| Al(OH)₃ | dropped K_f / complex | **hit** | 1.18e-7 → **1.78e-3 ✓** (full loop success) |
| integral | (none — gold wrong) | n/a, **correctly passed** | 5487 (correct) |

## Three findings
1. **Stance is the lever (cheap):** adversarial framing tripled Haiku's detection (1/5 → 3/5) at fixed
   model. Confirms ADJ97 — the failure was the defend stance, not capability alone.
2. **Capability still matters:** adv-Haiku (3/5) < adv-Opus (5/5). Haiku-as-skeptic catches the
   obvious-once-attacked errors (dropped K_f, SF7-infeasibility) but misses the subtle ones (coupled-
   parity number theory, velocity profile) that Opus catches. It is *stance × capability*.
3. **No false positive + partial fix:** adv-Haiku correctly PASSED the correct integral (self-Haiku
   false-flagged it). The full cheap loop (catch→fix) lifted accuracy on Al(OH)₃ (1.18e-7 → 1.78e-3),
   but on LoRaWAN/LSM Haiku flagged the error and still couldn't execute the fix — catching ≠ fixing
   (the ADJ97 execution floor persists).

## Architecture implication
A **cost-tiered adversarial audit**: cheap solver → cheap, fresh, adversarial auditor (catches ~60% of
errors, no false positives, fixes what it can execute) → escalate the residual/hardest to a stronger
auditor (Opus catches the rest). Role separation + adversarial stance buys most of the auditing value
cheaply; only the subtle remainder needs a bigger model. (Self-audit is the wrong loop; a separate
adversarial instance is the right one.)

## Honest caveats
- N=6 (5 real errors), single run/arm; Haiku is high-variance (base answers differ run-to-run, e.g.
  river 9v₀L/8v in ADJ96/97 vs 15v₀L/8v here). Directional.
- The adversarial prompt says "assume at least one error"; it did not cause a false positive on the one
  correct control item here, but at larger N false-positive rate should be measured.
- Oracle/scorer are Opus; correction graded deterministically.

## Next
- Measure the **false-positive rate** of the adversarial auditor on a set of *correct* answers (the
  "assume there's an error" prior could over-flag at scale).
- Implement the **tiered loop** (cheap adversarial auditor → escalate unresolved to Opus) and measure
  net accuracy + cost vs all-Opus.
