## Chapter capability ledger for Chapters 3–27 (2026-08-06)

- Added [`chapters.json`](./chapters.json), the track's HL05 chapter capability
  ledger: one `canDo` promise and one validated payoff for each of Chapters
  3–27. Titles and labels are copied from `core/book-generation.json` so the
  two agree until HL-C04 inverts that dependency; `spineNodes` are derived from
  `curriculum.json`'s path segments; every `payoff.assesses` atom is taken from
  the payoff lesson's own `practises.knowledge`, never invented.
- **Chapters 1–2 are deliberately absent.** Their terminal practice lessons
  (`AR-C01-practice`, `AR-C02-practice`) are still schema v1 and declare no
  `practises.knowledge`, so no payoff can name an atom without fabricating one.
  Those two chapters also have no `book-generation.json` target to copy a title
  from. The gap is recorded in the file's own `note` and stays visible to the
  HL05 gap report rather than being filled with a placeholder.
- Only Chapters 3 and 4 end in a `practice` lesson; every later chapter's payoff
  is its last lesson by `sequence`, which is what the corpus actually offers.
  All sixteen inline `writing` steps fall inside Chapters 1–4, so no chapter's
  payoff is a script-formation exercise — Chapter 3's payoff does include
  writing **بخير** by hand, and its summary says so.

