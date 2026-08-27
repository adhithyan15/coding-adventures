## 0.9.5 — the Learn session, teaching view (HL03 phase 6, slice 6b-1)

- **New "Learn" mode, now the default** — the curriculum walked the way the book
  does: one concept at a time, forward along the language chain. It renders the
  engine's *teaching pass* (`planSession(...).teaching`) as a numbered sweep of
  cards, one per active language that teaches the concept, in chain order.
- Each card shows the word in its own script, its gloss/romanization, its
  etymology hook, and — the point of the whole app — the **connections back** to
  earlier languages that share a root (e.g. Telugu *ధన్యవాదములు* → Hindi and
  Kannada via `dhanya, vada`). Connections are grounded and backward-only; the
  first stop, where the concept enters, wears an "introduced here" badge.
- Prev / Next walk the **concept spine** (`sweepableConcepts`) — the concepts in
  book order (earliest chapter first). Consolidation lessons (`practice`,
  `practice-mix`, `review` — placeholder headwords, no roots, `reviews_of`
  links) are filtered OUT of the spine: that kind of revisiting is what the
  review quiz is for, so the learner walks real words, not "(practice)". 205 →
  186 concepts.
- DOM-only shell; all sequencing stays in the tested engine. Verified in a real
  browser: every script renders (no tofu), the ten-language THANKS sweep shows
  its `dhanya`/`nal` threads. The review quiz (`pickNext`/`applyAnswer`) is the
  next slice (6b-2).

