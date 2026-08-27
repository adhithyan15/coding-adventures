## 0.9.0 — the session orchestrator: sweep + grounded connections (HL03 phase 3)

- `src/session.ts` — `buildSession(concept, lessons, activeCount)` takes the
  teaching sweep (phase 2) and annotates each stop with the **connections back
  to earlier languages in the sweep**: where two languages' lessons for the
  concept share an etymological root, the link is surfaced. This is the payoff
  of interleaving — meeting "thank you" in Telugu right after Kannada and Hindi,
  and being shown all three carry the Sanskrit root *dhanya*.
- **The grounding rule, enforced in code**: a connection exists *iff* the two
  stops' lessons literally share a root string (from `lesson.roots`). Nothing is
  inferred or invented; the reported `sharedRoots` is the exact set intersection,
  sorted. Connections always point backward in chain order.
- Verified against the real curriculum: Kannada and Telugu "thank you" link back
  to Hindi via `dhanya`. Controls bite — asserting a connection without a shared
  root fails the grounding test; over-reporting (union instead of intersection)
  fails the "never from thin air" control; a concept sharing no roots surfaces
  no link. Pure, deterministic, no UI.

