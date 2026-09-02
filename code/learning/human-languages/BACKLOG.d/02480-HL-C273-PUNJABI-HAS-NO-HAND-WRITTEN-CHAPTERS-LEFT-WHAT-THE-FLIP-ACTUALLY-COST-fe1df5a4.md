## HL-C273 — Punjabi has no hand-written chapters left; what the flip actually cost

Punjabi's last two hand-written chapters — Chapter 4 (farewells) and Chapter 5
(first verbs) — are now GENERATED from their lessons. The track has nothing
hand-written left, and `handwritten_parity.py --check punjabi` now answers
"already retired, nothing handwritten remains" rather than "nothing would be
lost".

**The flip was not the work. The schema-v2 migration was.** `book.ts` refuses to
generate a chapter from a schema-v1 lesson, so the ten legacy lessons behind
those two chapters had to declare what they teach before the chapter could be
built at all. That is the whole reason a hand-written chapter is dangerous: it
is not merely unmeasured, it is *unmeasurable*, and the lesson-level gates read
"0 lessons over 300 effective seconds" over a chapter they cannot see. Punjabi
went from a MIXED-schema track to a version-2 track in this one change, and its
atom-measurement-blind lessons fell from 18 to 8.

**Declaring atoms makes reinforcement debt appear, and the number goes UP.**
Twenty-five knowledge atoms that the corpus had been teaching in prose and
counting nowhere are now typed. Every R1-R4 window they fail to close became
visible at once: Punjabi's missed windows moved R1 45→48, R2 103→113, R3
158→175, R4 72→92. Nothing got worse for the reader; the instrument simply
reaches further than it did. **Expect this on every hand-written retirement,
and do not read the rise as a regression** — the serviced-debt assertions that
name specific atoms all still hold exactly.

**Two Chapter 5 lessons were packing several headwords each, and the migration
is what exposed it.** `PA-C05-main-punjabi-bolda-han` taught the word *panjābī*,
its Persian etymology, the gendered present habitual AND the whole sentence;
`PA-C05-kamm-karna` taught the noun *kamm*, the verb *karnā*, the √kṛ root and
the noun-plus-*karnā* pattern. Split into four lessons, every Chapter 4 and 5
lesson now introduces at most three atoms and exactly one new headword.

**Reading order changed, deliberately, and for the better.** The hand-written
.tex opened both chapters with their script section. The generated chapter puts
it where the lesson sequence puts it — near the end — which is what the script
lesson's own prose has always assumed: it says the glyphs live "inside a word
you already say" and asks the reader to "look back at the headword at the top of
any earlier lesson in this chapter". Placed first, that text pointed at nothing.
Gloss-first survives the move: the reader meets *phir* and *bolṇā* by ear, and
the pieces arrive later in the same chapter.

### What is still owed

- **Chapter 5 carries 15 new atoms against a chapter budget of 12.** That is one
  new atom-chapter spike (Punjabi 11 → 12), and it is the shape Chapter 3 (17)
  and Chapter 6 (13) already have. Clearing it means splitting the chapter, which
  renumbers Chapters 6-36 and is a separate tranche.
- **The 25 new Chapter 4 and 5 atoms have no later lesson putting them back in
  front of the reader**, which is where the R3/R4 rise above comes from. The next
  reinforcement tranche should service them.
- **`PA-C05-bolna` states that every Punjabi infinitive ends in -ṇā but does not
  own an atom for it**, because `PA-C07-hona` introduces `PA-GRAMMAR-NA-INFINITIVE`
  two chapters later. Chapter 5 teaches it first and should own it; moving the
  atom back is a small, separate change to a generated Chapter 7.
