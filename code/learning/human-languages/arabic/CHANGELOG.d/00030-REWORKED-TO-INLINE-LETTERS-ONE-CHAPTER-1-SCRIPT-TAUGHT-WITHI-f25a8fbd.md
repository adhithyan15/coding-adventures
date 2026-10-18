## Reworked to inline-letters: one Chapter 1, script taught within the words

Replaced the standalone reading-course structure (Ch1 reading course + Ch2
greetings) with the inline-letters model the rest of the curriculum now uses —
per `HL00`'s updated rule and direct user feedback ("introduce the letters as
you introduce words that use them… I do not want people to sit through a reading
course before they start").

- **Merged into a single `Chapter 1 — Greetings`** (`lessons/AR-C01-*`,
  `book/chapters/ch01-greetings.tex`): salām → marḥaban → al- → as-salāmu
  ʿalaykum → ṣabāḥ al-khayr → masāʾ al-khayr → shukran → practice. Each word
  lesson now carries a *"The letters in this word"* section teaching only the
  new letters that word needs (RTL, connecting letters, dots-on-a-skeleton, the
  emphatic ṣ, ʿayn, hamza), so reading + meaning + root arrive together. The
  root engine (k-t-b) and the three script facts moved into the chapter intro.
- **Removed** the old `book/chapters/ch01-reading.tex` + `ch02-greetings.tex`
  and the `AR-C01-read-*` / `AR-C02-*` lesson files (their content folded into
  the new inline lessons).
- **Beginner-audience fixes** (HL00 Audience rule): the preface and appendix no
  longer say "you already read Arabic (rusty)" or assume the reader knows
  Spanish; the Spanish-loanword thread is now self-contained enrichment.
- Retitled the book's green callout box from "Sounds & script you'll need" to
  "The letters in this word." Removed HL00's note that Arabic was still to be
  reworked (now done). Book compiles clean with XeLaTeX.

