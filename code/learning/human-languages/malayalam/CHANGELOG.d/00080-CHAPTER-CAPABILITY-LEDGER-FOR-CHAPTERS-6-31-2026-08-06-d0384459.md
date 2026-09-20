## Chapter capability ledger for Chapters 6–31 (2026-08-06)

- Added [`chapters.json`](./chapters.json), the track's HL05 chapter capability
  ledger: one `canDo` promise and one validated payoff for each of Chapters
  6–31. Titles and labels are copied from `core/book-generation.json` so the two
  agree until HL-C04 inverts that dependency; `spineNodes` are derived from
  `curriculum.json`'s path segments; every `payoff.assesses` atom is taken from
  the payoff lesson's own `practises.knowledge`, never invented.
- Derived from the lessons and `curriculum.json` rather than from
  [`roadmap.md`](./roadmap.md) or [`session-map.md`](./session-map.md), which
  still lag the canonical Chapters 6–31 (known debt, HL-M04).
- **Chapters 1–5 are deliberately absent.** Their terminal practice lessons
  (`ML-C01-practice` … `ML-C05-practice`) are still schema v1 and declare no
  `practises.knowledge`, so no payoff can name an atom without fabricating one.
  Those five chapters also have no `book-generation.json` target to copy a title
  from. The gap is recorded in the file's own `note` and stays visible to the
  HL05 gap report rather than being filled with a placeholder.
- No chapter from 6 on ends in a `practice` lesson, so every payoff is that
  chapter's last lesson by `sequence`. Where that terminal lesson is an
  `etymology` lesson (Chapters 23, 24, 31) the payoff is typed `task` and its
  summary describes the sorting the reader actually does.

