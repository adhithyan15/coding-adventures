## Chapter capability ledger — HL05 (2026-08-06)

- Added [`chapters.json`](./chapters.json), the track's HL05 chapter capability
  ledger. Twenty-six chapters — 6 through 31 — each declare a first-person
  `canDo`, the shared spine nodes they realise, and a `payoff` naming the lesson
  that proves the claim.
- Every `payoff.assesses` list is copied verbatim from its payoff lesson's own
  `practises.knowledge`, so the ledger cannot claim an atom the lesson does not
  actually practise.
- No chapter from 6 on has a terminal `practice`/`practice-mix` lesson — the
  Chapter 5 recap was the last one authored. Each payoff is therefore the
  chapter's **last lesson by `sequence`**. For Chapter 31 that is the register
  lesson `TE-C31-subha-madhyahnam-register`, which is the right payoff anyway:
  the chapter's promise is knowing *when* the greeting fits, not just saying it.
- **Chapters 1–5 are deliberately absent.** Their lessons are still schema v1,
  carry no `practises.knowledge`, and have no `core/book-generation.json`
  target to derive a title from. A payoff written for them would be invented,
  not derived; the gap is left visible as honest debt rather than stubbed.
- Thinnest payoff, for the representativeness gate that lands next: Chapter 20
  covers 2 of the 4 atoms its two lessons introduce — exactly the 0.5 threshold,
  because the weather lesson closes a chapter that also teaches 11–20.

