## Chapter 9 becomes three chapters, because sixteen words do not fit in one

Chapter 9 was hand-written LaTeX built from two schema-v1 lessons that owned
**two headwords between them** — `les mois` and `les saisons` — for **twelve
months and four seasons**.

That is not a chapter that needed converting. It is a chapter that had never
been written.

### Why this one splits instead of getting denser

Sixteen words plus the *au*/*en* rule cannot fit `maxNewAtomsPerChapter` at one
atom per new word. Two ways out: cram, or split.

`chapter-policy.json` settles it in its own note — *"Length is never a cost: no
threshold here may penalise page, lesson, or chapter count"* — and the owner's
standing rule says the same: page and lesson counts are never a constraint, and
a reader wants a **finishable unit** to come back to. So chapter 9 became three:

| chapter | what it holds | atoms |
|---|---|---|
| **9 — The Months: Gods and Beginnings** | *janvier* to *juin*, then practice | 7 |
| **10 — The Months: Caesars and Counting** | *juillet* to *décembre*, then practice | 7 |
| **11 — The Four Seasons** | the four, the *au*/*en* rule, then practice | 7 |

Twenty lessons replace two, and every French chapter after the split renumbered
by **+2** (old 10–33 → 12–35).

### What the renumber actually touches, and what it deliberately does not

Chapter numbers live in five places that must move together, and missing one is
**silent**: `book.tex`'s `\input` list is derived from the ledger, so a chapter
left behind vanishes from the book while its `.tex` stays committed and
hash-checked. It moves: lesson `chapter:` and `sequence:`, the `chapters.d`
shard (filename *and* field), the book-generation ledger entry (filename,
`chapter`, `output`), the `.tex` filename, and `\hlchaptermodality` inside any
hand-written `.tex`.

**Lesson ids do not move.** `FR-C17-tete` keeps its 17 in chapter 19. That is the
corpus's own convention — Spanish proves it, with `ES-C03-*` lessons spread
across chapters 4, 5 and 6 — and renaming ids would break every
`prerequisites`/`reviews_of` edge to buy cosmetic agreement.

For the same reason the twenty new lessons are all **`FR-C09-`**, including the
ones that land in chapters 10 and 11: the id names the chapter a lesson was
*written for*, and all of this was written for chapter 9. It also avoids a
genuine collision — `FR-C10-parents` already exists and is now chapter 12.

The **sequence** shift is not cosmetic either. Sequences must increase along the
reading order, and twenty lessons do not fit in the thirteen integers between
chapter 8's last and old chapter 10's first. Everything from the split point up
moved by a constant.

### One new word per lesson

Every month gets its own lesson and its own figure: **Janus** in the doorway of
the year, the **Februa** that cleaned the city before it turned, **Mars** who
already owns *mardi*, **Maia** of growth, **Juno** whose husband owns *jeudi*.
Then the two men — *juillet* and *août* — and the four that are still counting.

Three things the split made room for that the single lesson could not hold:

- **`avril` is a guess, and the lesson says so.** *Aperīre*, "to open," is the
  best answer and an uncertain one. A book with a confident story for every word
  is inventing some of them.
- **`octobre` rescues a number.** *Octō* is unrecognisable inside **huit** and
  perfectly legible inside **octobre** — so when a word wears down, look for a
  longer relative that did not.
- **`au printemps` stops being an exception.** Three seasons take *en* and spring
  takes *au* because *printemps* is the only one starting with a **consonant** —
  the same vowel-or-consonant question that decided *le* against *l'*.

### It closes an exam point that was deliberately held back

**A1-LEX-06 (days, months, seasons)** moves from unmapped to covered. It was left
unwired through two earlier tranches on purpose: the days were taught, but two
headwords for sixteen words meant any probe naming a month would have been a
claim the corpus could not support. French A1 coverage: **27/74 → 28/74**.

### Measured

- Hand-written French chapters: **10 → 9**; corpus-wide **33 → 32**.
- French chapters: **33 → 35**.
- Parity blocks at risk: **45 → 43**.
- Measurable French lessons: **108 → 128**.
- Culture claims: **17 → 19**.
- Forward references: **48 → 53**, and this rise is the point rather than a
  regression. The gate cannot see a forward reference to a word **no lesson
  teaches** — so teaching the months made chapter 6's previews of them visible
  for the first time. Five of the nine that appeared were decorative and are
  gone (chapter 6 now names *Julius Caesar* and *Augustus* rather than *juillet*
  and *août*, and says "the four months at the end of the year"). The three that
  remain are load-bearing: *sept* → **septembre**, *huit* → **octobre**, *neuf* →
  **novembre** are the whole didactic move of that chapter, and you cannot make
  the point without naming the month.
- Chapter atoms: **7, 7 and 7** — all three chapters well inside the budget the
  single chapter could not have met.
- `language-ladder`: 39 files, 442 tests, all pass. It reads the handwritten set
  from the ledger rather than hard-coding it, so the renumber needed no edit
  there.
- The book compiles under XeLaTeX with `missing_character = 0`, and the rendered
  pages of all three new chapters were read.