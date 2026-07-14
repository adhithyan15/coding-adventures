# Changelog

## Chapter 1 — Greetings (Tamil script taught inline)

- New Tamil track on the HL00 framework — the **anchor** of the four Dravidian
  tracks. One word per lesson, slug ids, atom-first, derivations shown, LaTeX
  book. Uses the **vendored** Noto Sans Tamil font (loaded by relative `Path=`
  so local and CI builds match).
- **No reading course.** Per `HL00`'s inline-letters rule, Tamil is taught
  *inside* each word lesson: a *"The letters in this word"* section introduces
  exactly the letters that word needs, so reading and meaning arrive together.
  A Tamil script reference page is included in the book as a lookup, explicitly
  not a gated pre-course.
- Chapter 1 (`lessons/TA-C01-*`), greetings + conversational glue:
  - **வணக்கம்** vaṇakkam ("hello," from the native verb *vaṇaṅku*, "to bow") —
    teaches the inherent *a*, the puḷḷi (vowel-killing dot), the retroflex ண.
  - **நன்றி** naṉṟi ("thanks," literally "goodness," from *nal*) — introduces
    Tamil's three-way dental/alveolar/retroflex *n* (and the parallel *l*, *r*
    sets).
  - **ஆம்** ām ("yes") — independent word-initial vowels; verb-echo "yes."
  - **இல்லை** illai ("no / there isn't") — negation carried by a negative verb
    of existence, a deeply Dravidian habit.
  - **சரி** sari ("okay") — one Tamil letter standing for several stop-sounds
    (voicing read from position).
  - **practice** — recap + the *pōy varugiṟēṉ* / *pōy vā* farewell ("go and
    come back," never a bare "I'm leaving").
- The recurring thread: **Tamil's native word-stock vs. its sisters' Sanskrit
  borrowing** — each lesson carries an "Across the family" cognate box
  (English / Sanskrit / Hindi / Kannada / Telugu / Malayalam), every form
  supplied so nothing is assumed. Grounds against English + the Dravidian
  family + Sanskrit. Book compiles clean with XeLaTeX.
