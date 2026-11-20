---
category: Testing & coverage
---

# A fresh Warrant generation check does not validate a nearest-even rounding model

The first full Q16.16 model in `code/specs/fixed-point.warrant` passed
`gen_evidence --check`, because that command checks generated-file freshness and
binding consistency; it does not evaluate the model's arithmetic. The model's
`NearestEven` branch selected the truncation quotient when the remainder was
greater than half and always adjusted on an exact tie, ignoring quotient parity.
It therefore got `3/4` wrong (0 instead of 1), `1/2` wrong (1 instead of 0),
and `5/2` wrong (3 instead of 2).

Fix: compare twice the absolute remainder with the positive denominator. Below
half keeps the truncation quotient; above half moves one integer in the
rational's sign; at an exact tie keep an even quotient and otherwise move to
the adjacent even integer. Add native evaluator checks for above-half and both
even/odd ties before launching wide edge campaigns. Keep the arbitrary-precision
oracle as a separate derivation, and compare actual Warrant results on named
vectors and generated inputs; generation freshness alone is not a model oracle.
