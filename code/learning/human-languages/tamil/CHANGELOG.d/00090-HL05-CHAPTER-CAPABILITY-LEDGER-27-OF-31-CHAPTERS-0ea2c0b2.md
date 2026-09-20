## HL05 chapter capability ledger — 27 of 31 chapters

- Added [`chapters.json`](./chapters.json), the track's authored chapter
  capability ledger: a first-person `canDo`, the shared spine nodes the chapter
  realises, and a payoff (lesson, kind, one-line summary, and the knowledge
  atoms it exercises) for every chapter that can honestly carry one.
- `title` and `label` are reproduced exactly from `core/book-generation.json`
  for Chapters 6–31, so the HL-C04 title inversion cannot silently rename a
  printed chapter. Chapter 1 has no target there and takes its printed name
  from `book/chapters/ch01-greetings.tex`, the file it already prints from.
- `canDo` is written for this track's actual reader — someone fluent and
  literate in Tamil who never studied the grammar formally. The claims are
  about grammatical and etymological *control* (choosing the register a moment
  calls for, keeping இரவு and இருள் apart, reporting a hedged etymology with
  its hedge intact), not about decoding the script.
- **Chapters 2, 3, 4 and 5 are deliberately absent.** Every lesson in them is
  still schema v1 with no `practises.knowledge`, so no payoff could name a real
  atom without inventing one. An absent entry is honest, measurable debt; a
  stubbed one would destroy the signal the HL05 gap report exists to produce.
- No Tamil chapter has a terminal `practice` or `practice-mix` lesson under
  schema v2, so every payoff is the chapter's last lesson by `sequence`. Eight
  of those chapters end on a non-conversational lesson, and their `payoff.kind`
  says so: Chapter 1 ends on an inline `writing` lesson and is recorded as a
  `task` (write நன்றி from its three pieces), as are the four chapters whose
  closing work is weighing etymological evidence rather than speaking.
- Four payoffs assess less than half the atoms their chapter introduces and
  will fail HL-C03's representativeness gate at the configured 0.5 threshold:
  Chapter 1 (5/17), Chapter 7 (3/8), Chapter 6 (2/5), and Chapter 20 (2/5).
  Each is a chapter whose final lesson is a narrow etymology or family
  comparison rather than a consolidation; the fix is a real terminal practice
  lesson, not a wider `assesses` list.

