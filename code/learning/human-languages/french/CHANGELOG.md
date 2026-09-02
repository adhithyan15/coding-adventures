# Changelog

## Chapter 16 leaves the hand-written set, and where a colour goes in the sentence

Chapter 16 (Colours) was hand-written LaTeX over **two** schema-v1 lessons that
held **four colours** between them — `noir, blanc` in one and `rouge, bleu` in
the other.

### The grammar the chapter taught in passing

The `.tex`'s closing `grammarlens` said: *"put them to work on a noun you already
have: le vin blanc, le vin rouge."* That sentence is **adjective position**, and
French puts the colour **after** the noun where English puts it before. It is one
of the first real word-order differences a learner meets, it was taught as an
aside, and **no lesson owned it**.

It now has a lesson, and the lesson uses something the reader already says.
Stating "French adjectives follow the noun" would be contradicted immediately by
**bonjour** — *bon* + *jour*, adjective in front. So the rule is given in the
form that survives contact with the corpus: **short, common adjectives lead;
everything else, colours included, follows.**

Eight lessons replace two, eleven atoms against a ceiling of twelve:

| lesson | the one new thing |
|---|---|
| `FR-C13-noir` | *noir*, and *denigrate* as "to blacken a name" |
| `FR-C13-blanc` | *blanc*, a **Frankish** word inside a Romance language |
| `FR-C13-aube` | where *albus* went when it lost — the dawn |
| `FR-C13-rouge` | *rouge*, and that English *red* is a **cousin**, not a lookalike |
| `FR-C13-bleu` | *bleu*, the Germanic word that went abroad and came home |
| `FR-C13-vin-rouge` | the colour goes **after** the noun |
| `FR-C13-tricolore` | *bleu, blanc, rouge* — and the flag is two-thirds Germanic |
| `FR-C13-practice` | the chapter payoff |

### The finding one lesson per word made sayable

Half of French's four basic colours are **not Latin**. *blanc* and *bleu* are
Frankish, and that direction — a Germanic word displacing a Latin one **inside**
a Romance language — is the opposite of the usual traffic.

Given its own lesson, that stops being trivia and becomes a claim with a
consequence: the French **tricolour is two-thirds Germanic**, and the chapter
says so. That is the chapter's one culture claim, and it lived in a `culture`
block owned by nobody.

*albus* got a lesson too, because a displaced word is worth watching. It did not
die when *blanc* took its job; it went into the **specific** and stayed there —
*l'aube* the dawn, *l'aubépine* the hawthorn, English *album* and *albino*.

## Chapter 1 is generated, and three exam points it always taught are finally visible

Chapter 1 — *Greetings*, the first thing a reader of this book ever meets — was
hand-written LaTeX over eleven schema-v1 lessons and four schema-v2 writing
lessons. It is now **fifteen** schema-v2 lessons and generated from them.

### Four ways of sizing it, and what each one said

| instrument | answer |
| --- | --- |
| `handwritten_parity.py` | **0** — the chapter does not even appear in the report |
| `grep -l '^chapter: 1$' lessons/*.md` | **15** lessons (11 content, 4 writing) |
| the `.tex` | **15** sections |
| taught-word census | **0** French words owned by no lesson |

The census's residue — 98 distinct emphasised tokens in the `.tex`, of which 52
match no French headword — is entirely Latin etymons (*diurnum*, *noctem*,
*ille / illa / illos*), Spanish comparanda (*hola*, *bueno / buena*, *tarde*),
pronunciation respellings (*zhoor*, *swahr*, *nwee*), morphological fragments
(*-ct-*, *-it-*, *nn*) and inflected forms of words the chapter already owns
(*le jour*, *les soirs*, *bonnes*). Not one unowned French headword.

For once every instrument agreed, and the agreement was real: chapter 1's
hand-written `.tex` mirrors its lessons section for section, including the
Latin/Spanish/French/English cognate table under *nuit* and the three-line
agreement display under *bonne nuit*. **Nothing had to be written.** The whole
job was typing what was already there, which is why this is the only French
retirement so far with a net lesson change of zero.

That is the exception rather than the rule, and it is worth naming why: chapter 1
is the one chapter whose lessons were authored *before* the LaTeX rather than
extracted from it.

### It lands at exactly twelve atoms, and did not split

`maxNewAtomsPerChapter` is 12 and `atomChapterSpikes` for French is 0, so the
question was real. Ten words and phrases (*salut*, *bien*, *bon*, *jour*,
*bonjour*, *soir*, *bonsoir*, *nuit*, *bonne nuit*), the two grammar rules the
greetings run on, and the writing runway's `FR-ORTHO-SALUT-01` come to **twelve
exactly**. Chapters 3, 6 and 7 sit at the same ceiling.

The etymology stays prose. *salus*, *bene*, *bonus*, *ille*, *diurnum*, *sērum*
and *noctem* are all still taken apart on the page, and none of them is a typed
atom — the same call chapter 7 made, where only three of eight day-names earned
an `FR-ETYMON-*`. Typing all seven here would have been sixteen atoms and a
split, and a split of chapter 1 renumbers every chapter in the book.

### Two grammar rules that were being taught and could not be measured

- **`FR-GRAM-ADJ-AGREEMENT-04`** — *bon*, *bonne*, *bons*, *bonnes*, introduced
  in the *bon* lesson and then spent across three: *bonjour* and *bonsoir* take
  the masculine, *bonne nuit* the feminine. The rule arrives with the words that
  need it rather than as a table.
- **`FR-GRAM-LE-LA-GENDER-05`** — *le* / *la* / *les* as one atom, because they
  are one system: the article is where a French noun's gender is visible, and
  *les* is the single form both genders share.

### It closes three exam points, and none of them needed new content

French A1 coverage **27/74 → 30/74**: A1-LEX-01 (greetings and farewells),
A1-D-01 (the definite article) and A1-A-01 (adjective agreement). All three were
on page one from the beginning; none could be probed, because a hand-written
chapter's `grammarlens` owns no atom. A1-LEX-01 had the farewells already —
chapter 4 is generated — so half the point existed while the whole point read as
absent. Recorded in `BACKLOG.d` as HL-C312, because it applies to every
hand-written chapter that remains.

### The path was out of order, and a cross-reference rotted

Two repairs the gates found:

- `FR-PATH-005` (*bonne nuit*) sat **before** `FR-PATH-006` (*soir*, *bonsoir*)
  in the curriculum path, while its lesson sequence is later. Harmless while the
  lessons were schema-v1 and unenforceable; a hard prerequisite-order failure the
  moment they were typed. The two segments are swapped, which is also the order
  the chapter is read in.
- `ch02-introductions.tex` pointed at `\S\ref{lesson:le-la}`, a label the
  hand-written chapter 1 defined and the generated one does not. Rewritten to
  name the thing — "the *le* / *la* lesson" — which is the rule
  `chapter-references.test.ts` already holds French to for chapter numbers.

### One defect only the page showed

The delayed-copy writing runway authored its four steps as a markdown ordered
list. The generator emits `itemize` for `-` bullets and **nothing** for `1.`, so
the four steps printed as one run-on line where the hand-written chapter had a
real `enumerate`. Rewritten as bullets. 169 lessons across the corpus have the
same shape and every generated chapter among them is already shipping it; the
general fix is a `src/book.ts` change and wants its own commit.

### Measured, against the merged tree

- Hand-written French chapters: **10 -> 9**; corpus **21 -> 20**.
- `handwritten_parity.py french`: **45** blocks at risk, unchanged — chapter 1
  contributed none of them.
- Schema-v2 French lessons: **108 -> 119**.
- Atoms taught: **190 -> 201**.
- Atom-measurement-blind lessons: **39 -> 28**.
- Chapters over the 12-atom budget: **0 -> 0**.
- Culture claims: **17 -> 20** — *salut* is strictly informal, *bonjour* is
  near-obligatory on entering anywhere, *bonne nuit* is for bed and not for the
  evening. All three were prose the `.tex` boxed as `culture` and no lesson owned.
- Forward references: **48 -> 48**.
- Reinforcement window misses: **416 -> 443**. Eleven new atoms exist to be
  missed; every one of them is revisited at least once, so `atomsNeverRevisited`
  holds at **37**.
- Cross-chapter prose references: **0 -> 0**.
- Completion-plan uncovered points: **851 -> 848** when this branch was cut,
  re-measured by running the CLI rather than by subtracting three. HL-C310
  landed on `main` in the meantime and removed that absolute pin from
  `plan-cli.test.ts` altogether, for exactly the reason this entry keeps
  re-measuring rather than composing: the figure moved nine times in one day and
  two branches that both lower it merge quietly because they agree. The test now
  asserts the invariant it actually owns — that duplicating an inventory changes
  neither figure — and the corpus total after this merge is **785**.
- The book compiles under XeLaTeX with `missing_character = 0` and every warning
  class at its baseline, and the rendered pages were read.
## Chapter 14 becomes two chapters, split at the seam it was already about

Chapter 14 (Numbers Eleven to Twenty) was hand-written LaTeX over **two**
schema-v1 lessons that owned **ten numbers** between them, plus the rules for how
those numbers are built.

Ten numbers plus their formation cannot fit `maxNewAtomsPerChapter` at one atom
per word. This is the second French chapter split, and the second one where the
budget was not the interesting part.

### The split point is the chapter's own subject

The chapter exists to show that **at seventeen, French changes its mind**. Up to
*seize* the numbers are inherited from Latin already fused; from *dix-sept* they
are built in the open out of *dix* and a digit. That is the seam, and it is where
the chapter divides:

| chapter | what it holds | atoms |
|---|---|---|
| **14 — The Welded Teens** | *onze* to *seize*, the `-ze` that is *decem*, digit-first order | 8 |
| **15 — Where French Changes Its Mind** | *dix-sept* to *vingt*, the seam, *vīgintī* | 6 |

Sixteen lessons replace two, and every French chapter after the split renumbered
by **+1** (old 15–35 → 16–36). Lesson ids do not move: all sixteen are
`FR-C12-*`, including the seven that land in chapter 15.

### What one lesson per number made room for

The old lesson listed six words in a table and moved on. Given a lesson each,
several facts fit that had nowhere to live before:

- ***quinze* needs English to be legible.** *cinq* and *quinze* do not look
  related, because *cinq* travelled further from *quīnque* than *quinze* did.
  **quintet** stands between them, and the lesson says so.
- **The seam is two changes, not one.** Everyone notices *fused → built*. Almost
  nobody notices that the halves **swap order**: *seize* is six-ten, *dix-sept*
  is ten-seven. The seam now has its own lesson because it holds two facts.
- **Latin counted 18 and 19 downward** — *duodēvīgintī*, "two from twenty" — and
  every daughter language abandoned it. The plain *ten-eight* is a survivor, not
  a simplification.
- ***neuf* is two words** — nine from *novem*, new from *novus* — worn into the
  same four letters.
- ***vingt* is a beginning.** It explains *quatre-vingts* before the reader ever
  meets it, which is a better place to learn it than at eighty.

### The renumber's stale shard, and how it was found

A renumber leaves **stale book-hash shards**, and `check:books` says only
*"missing, stale, or malformed"* without naming one. `check:shards` named it —
`book-generation targets identity set differs: missing [french/0019]` — and a
by-hand diff of the hash-shard set against the ledger target set confirmed
exactly one: `generated-book-hashes/french.d/0019.json` still described
`ch19-head-and-hand.tex`, which is now chapter 20. The generator writes shards
but does not prune them, so a chapter number vacated by a renumber keeps its old
hash file until someone deletes it.

### Two false sight-cues, found by reading the rendered page

Chapter 14 first printed *"Hands-free start: **first 1** of 9 lessons"* — meaning
the second lesson broke the run. The modality classifier had marked `douze`
`sight`, and its reason was one phrase: *"this one lets you **see the** join."*
The same thing had happened in the seam lesson, on *"**Look at** which half comes
first."* Both were metaphors about hearing, both fired a literal page-pointing
cue, and both lessons are now genuinely hands-free — chapters 14 and 15 print
*"all 9 lessons"* and *"all 7 lessons."*

Nothing but the rendered page shows this. Every gate was green with a
voice-first chapter that told a listener to look at something.

## Chapter 13 leaves the hand-written set, and the word its own promise required

Chapter 13 (Bread, Water, Wine) was hand-written LaTeX over **two** schema-v1
lessons holding **three headwords** — `le pain` in one and `l'eau, le vin` in
the other.

### The chapter promised something nothing taught

Its own `canDo` said the reader could *"name **and request** bread, water, and
wine."* Requesting them needs the **partitive** — *du pain*, *de l'eau* — and no
lesson owned it. It appeared exactly twice: once as a bare Guided Practice line,
`[YOU SAY: "un pain, du pain"]`, with no explanation anywhere in the lesson, and
once inside a `culture` block as a closing sentence.

This is the shape the block-gap measure cannot see, and the reason it scored
this chapter at 4 while the authoring was five lessons: a word taught in a
sentence inside a surviving block costs **zero blocks and a whole lesson**.

Five schema-v2 lessons replace two:

| lesson | the one new thing | atoms |
|---|---|---|
| `FR-C11-pain` | *le pain*, and *com + pānis* = **companion** | 2 |
| `FR-C11-eau` | *l'eau*, and the erosion *aqua → eau* | 2 |
| `FR-C11-vin` | *le vin*, and why *vīnum* held its shape | 2 |
| `FR-C11-du` | **de + article** — asking for *some* | 1 |
| `FR-C11-practice` | the chapter payoff | 0 |

Seven atoms, under `maxNewAtomsPerChapter`'s 12, so this chapter does not split.

### The pairing the split made room for

*aqua* and *vīnum* sat in one lesson as two etymologies. Given a lesson each,
they become a **contrast**: one word dissolved to a single vowel, the other is
still legible in *wine*, *vine*, *vinegar* and *vintage* — and the difference is
not linguistic. Everyday things wear down; **traded** things stay legible,
because they keep having to be read in three countries. That is a rule the
reader can use on words this book has not taught yet, and it only exists once
the two words stop sharing a paragraph.

### Corrected on the way through

`FR-C11-eau-vin` closed with *"Next chapter: ordering and asking for these at a
table."* Chapter 14 is **Numbers Eleven to Twenty**. The pointer had been true
of some earlier draft of the book and was never revisited; the new lessons point
at what actually follows them.

## Chapter 12 leaves the hand-written set, and the asterisk the book was eating

Chapter 12 (Family) was hand-written LaTeX over **two** schema-v1 lessons that
held **four headwords between them** — `le père, la mère` in one and
`le frère, la sœur` in the other. No word in the chapter owned a lesson, so no
word could be practised, reviewed or scheduled on its own.

Seven schema-v2 lessons replace them, one new thing each:

| lesson | the one new thing | atoms |
|---|---|---|
| `FR-C10-pere` | *le père*, and *pater* inside *paternal* | 2 |
| `FR-C10-mere` | *la mère*, and *māter* inside **matter** | 2 |
| `FR-C10-grimm` | **Grimm's law** — *p → f*, *t → th* | 1 |
| `FR-C10-frere` | *le frère*, and *friar* as a worn-down *frère* | 2 |
| `FR-C10-soeur` | *la sœur*, and why it broke the rhyme | 2 |
| `FR-C10-oe` | **œ**, one letter and not two | 1 |
| `FR-C10-practice` | the chapter payoff | 0 |

Ten atoms, under `maxNewAtomsPerChapter`'s 12, so this one does **not** split.

### The two things the `.tex` held that no lesson owned

**Grimm's law** lived in a `grammarlens` block. It is the single fact that turns
*père/father*, *mère/mother* and *frāter/brother* from three separate
memorisations into one, and it now has a lesson rather than a box.

**œ** is a *letter*, and it was a parenthesis inside a `sounds` block. It earns a
lesson on the corpus's own rule that a root must be cashed in at least three
times: **œ** returns in `l'œuf` and `l'œil` later in this book, and in *cœur*
everywhere outside it. The lesson says plainly which of those the book teaches
and which it does not, rather than promising four and delivering two.

### Sized four ways, because the gap said 2

`handwritten_parity.py` scored the chapter at **2** blocks. Counting the French
words the `.tex` teaches against the lessons that own them gives **4 headwords
against 2 lessons**; the honest denominator, `grep -l '^chapter: 12$'`, gives
**2**; and reading the tables finds three of them, all vocabulary rather than
paradigm — which is exactly why this chapter did not need splitting and
chapter 9 did.

### A defect only the printed page could show

The Grimm's law table lists reconstructed forms, and every reconstructed form is
written with a leading asterisk. The second row printed as *méh₂tēr* — **without
its asterisk** — because `\\` followed by `*` is LaTeX's *starred line break*,
so the row separator above swallowed it. The first row was fine; it follows
`\midrule`.

Nothing could catch this but looking. The `.tex` on disk contained the asterisk,
so `check:books`, the hash shards and every text assertion were green while the
printed page dropped the one mark that says a form is reconstructed. `book.ts`
now brace-protects a leading `*` in a table cell, and `book.test.ts` pins it
with a **two-row** fixture, because a one-row table cannot reproduce it.

Two further page-only defects, same method: a `[WRITE: …]` cue written without
the `YOU ` prefix printed its brackets verbatim (389 lessons write
`[YOU WRITE:`; this was the only one that did not), and two Guided Practice
sections ended their prose with a colon, so the renderer's own *"Say these
aloud:"* landed underneath as a stutter.

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