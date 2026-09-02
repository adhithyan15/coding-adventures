## HL-C312 — a hand-written chapter's grammar owns no atom, so the exam inventory cannot see it

French chapter 1 is now generated. **Nine hand-written French chapters remain,
twenty across the corpus.** Retiring it moved French A1 exam coverage
**27/74 → 30/74**, and none of the three points that moved needed a single new
word written. They were all taught on the book's first pages, and had been
since the chapter was written.

### The finding

`measureExamCoverage` resolves a probe against **introduced knowledge atoms**.
A hand-written chapter has no atoms — its teaching lives in `grammarlens`,
`cousinweb` and `sounds` environments in the `.tex`, which own nothing. So a
point the corpus teaches perfectly well reports as **uncovered**, and the
inventory's uncovered list — the authoring brief for the vocabulary work that is
most of the road to C2 — names work that does not need doing.

The three that moved say it plainly:

- **A1-LEX-01, greetings and farewells.** The farewells had atoms, because
  chapter 4 was generated. The greetings did not, because chapter 1 was not. So
  half the point existed and the whole point read as absent — in a track whose
  first chapter is titled *Greetings*.
- **A1-D-01, the definite article `le / la / les`.**
- **A1-A-01, adjective agreement in gender and number.**

The last two are the two rules the greetings run on: *bon* versus *bonne* **is**
agreement, and *le* / *la* is where the gender it agrees with becomes visible.

### What to do with it

**Do not read an exam inventory's uncovered list as a content gap while the
track still owns hand-written chapters.** Some fraction of it is representation
debt, not teaching debt, and the fraction is exactly the hand-written range. The
cheap check before scheduling authoring: for each uncovered point, ask whether a
hand-written chapter already teaches it. If it does, generating that chapter
closes the point for free — and until it is generated, no amount of new
vocabulary will.

The corollary bounds the optimism: this is **not** a reason to expect every
retirement to close points. Chapter 1's three closed because its content was
grammar the inventory enumerates. A vocabulary chapter's retirement adds atoms
that no A1 point names.

### A second, smaller one: generated books flatten ordered lists

`1. … 2. … 3.` in a lesson body renders as **one run-on paragraph** in the
generated `.tex` — the renderer emits `itemize` for `-` bullets and nothing at
all for `1.`. Chapter 1's delayed-copy runway printed *"1. cover salut 2. wait
five seconds 3. write the word once from memory 4. uncover…"* as a single line
where the hand-written chapter had a real `enumerate`.

**169 lessons across the corpus author ordered lists**, and every generated
chapter that contains one is already shipping this — Arabic chapters 1 to 4
among them. The narrow fix used here was to rewrite the four steps as bullets.
The real fix is an `enumerate` branch in `src/book.ts`, which would re-hash a
large share of the generated corpus and so wants a commit of its own.
