# Changelog

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

## Chapter 8 is generated, and the clock is finished rather than promised

Chapter 8 was hand-written LaTeX built from two schema-v1 lessons that owned
**three words between them** — *heure*, *midi*, *minuit* — for a chapter whose
`.tex` taught considerably more than three things.

It is now **nine** schema-v2 lessons.

### What the block gap could not see

`handwritten_parity.py` scored chapter 8 at a gap of **one block**. Counting the
French the `.tex` actually teaches against the lessons that own it tells a
different story. Owned by **no lesson at all**:

- **`il est … heures`** — the frame the whole chapter runs on, taught in a
  `grammarlens` and owned nowhere.
- **`une heure` against `deux heures`** — the gender and number agreement, taught
  in a table's `note` column.
- **`et quart`, `et demie`, `moins le quart`** — named in one sentence of that
  same lens and explicitly deferred: *"add … later."*

None of those cost a prose block. Every one of them costs a lesson. A chapter
called *Telling the Time* that stops at whole hours has not taught the reader to
tell the time, so the deferred three are now taught here rather than promised.

### One new word per lesson

`heure`, `il est … heures`, `une heure`, `midi`, `minuit`, `et quart`,
`et demie`, `moins le quart`, then the chapter practice. The etymology is
carried whole and several strands now close on each other:

- **`heure` and `hour` are one word** — Latin *hōra* from Greek *hōrā*, "a
  season, the right time." Both languages keep a silent *h* neither ever said,
  and that silent *h* is precisely **why** *deux heures* liaises to *deu-z-eur* —
  which the reader met as a rule two chapters earlier.
- **`midi` = `mi-` + `-di`** — the *diēs* of *lundi*, come round to the front of
  a different word. **`minuit` = `mi-` + `nuit`**, the *nuit* of Chapter 1.
  One idea used twice.
- **agreement you cannot hear** — the plural *-s* of *heures* and the feminine
  *-e* of *demie* are both silent, which is why the number in front is
  load-bearing rather than decorative.
- **`moins le quart` subtracts from the hour that is coming**, and *quart* is
  *quatre* doing a different job.

### Measured

- Hand-written French chapters: **11 -> 10**.
- Chapter atoms: **11**, inside `maxNewAtomsPerChapter`; `atomChapterSpikes` 0.
- Forward references: **50 -> 48**. The chapter contributes none, and teaching
  *midi*/*minuit* separately retired two that existed before.
- Measurable French lessons: **99 -> 108**.
- Culture claims: **16 -> 17** (English *noon* is *nōna hōra*, "the ninth hour,"
  which drifted from mid-afternoon to midday).
- The book compiles under XeLaTeX with `missing_character = 0`, and the rendered
  pages were read.

### It closes an exam point, because it finishes the job

**A1-LEX-07 (telling the time)** moves from unmapped to covered. It could not
have been claimed before at any wording of the probe: the hand-written chapter
stopped at whole hours. The probe lists **all seven** atoms rather than a
sample, because a candidate asked for half past does not get partial credit for
o'clock. French A1 exam coverage: **26/74 -> 27/74**.

One gate finding worth recording. `info-dump` flagged the practice tables as
**verb paradigms**, because every row began *Il est …* and its French person
list contains *il*. The tables are clock times, not a paradigm — but the fix
improves them anyway: the repeated *Il est* is now a one-line instruction above
the table (*"put each of these into the frame"*), and the rows carry only what
differs.

## French joins Spanish at zero cross-chapter number references, before the split

Three of the remaining hand-written French chapters cannot be authored inside
`maxNewAtomsPerChapter` at one atom per new word: chapter 9 carries **twelve
months and four seasons**, chapter 12 carries **ten numbers**, and chapter 16
carries a **six-person paradigm plus the verb list that selects it**. Length is
never a cost here — `chapter-policy.json` says so in its own note — so those
become **more chapters** rather than denser ones, and every French chapter after
a split point renumbers.

`chapter-references.test.ts` says what to do about that, and says it plainly:

> Spanish is held at ZERO because Spanish is the track that actually renumbers …
> **When a track starts splitting chapters, clear it first and move it to zero.**

French was at **32**. It is now at **0**, cleared *before* the first split rather
than after the first rot.

### The fix is never a fresher number

A sentence like *"you learned this in Chapter 14"* is correct when written and
wrong three renumbers later, and **nothing fails** — the reader simply follows a
pointer into the wrong chapter. So every one of the 32 was rewritten to **name
the thing**:

| was | is |
|---|---|
| "from Chapter 1: **bien**" | "from your first greetings: **bien**" |
| "the *tu/vous* choice you learned in Chapter 2" | "the *tu/vous* choice you learned when you gave your name" |
| "in Chapter 5 you learned **habiter**" | "among your first verbs you learned **habiter**" |
| "Chapter 28 handed you *le lait* and *le sucre*" | "the café chapter handed you *le lait* and *le sucre*" |
| "the nasal vowel from Chapter 11's *pain*" | "the nasal vowel of *pain*, the bread" |
| "(Chapter 17), was Roman soldiers' slang for a pot" | "was Roman soldiers' slang for a pot" |

Where the number was pure decoration it is simply gone; where it was doing work,
the work is now done by a description that cannot go stale.

French is additionally pinned with its own `toBe(0)` rather than left to the
shared ceiling. A ceiling of zero and an assertion of zero are the same number
today and different promises: the assertion says the track is **cleared**, not
merely not-growing.

### Reading the page caught what the gate could not

The gate counts `Chapter \d+` and cannot read. Two rewrites were wrong and only
the compiled book showed it:

- `FR-C31-ventre` glossed **oui** as *"(the plain no)"* — the replacement for a
  bare "(Chapter 18)" pointer. *oui* is **yes**. The parenthetical carried no
  information the sentence lacked and is deleted.
- `FR-C14-avoir` read *"the aqua → eau you met with water"*, which is circular:
  *eau* **is** water. Now "the *aqua* → **eau** behind the word for water."

### Measured

- French cross-chapter prose references: **32 -> 0**.
- 26 lessons edited across chapters 2, 3, 4, 10, 12, 14, 15, 16, 17, 27, 28, 29,
  30 and 31. No lesson ids, atoms, chapter numbers or ledger entries move: this
  changes prose only, and is deliberately separable from the split it unblocks.
- The book compiles under XeLaTeX with `missing_character = 0`, and the rewritten
  pages were read.
## Chapter 7 is generated, and the week arrives one day at a time

Chapter 7 was hand-written LaTeX built from two schema-v1 lessons: one taught
**five** day-names at once behind a four-column reveal table, the other taught
the remaining two. Neither declared any atoms, so the chapter could not be
generated and no lesson-level gate could see it.

It is now **nine** schema-v2 lessons.

### The measurement that sized this, and why the parity gap did not

`handwritten_parity.py` scored chapter 7 at a gap of **one block**. That is a
true count of LaTeX environments and a bad estimate of the work, because it
counts *environments* and the debt is in *words*: a word taught in a paragraph
inside a surviving `cousinweb` costs **zero blocks and a whole lesson**.

So the chapter was sized a second way -- **count the French words the `.tex`
teaches, against the lessons that own them.** Chapter 7 teaches eight: the seven
days, and **la lune**, which the `.tex` glossed inside a parenthesis in a table
cell and which no lesson owned at all. One block of gap; eight words with two
lessons between them.

`la lune` now has its own lesson, and it goes **first** -- so *lundi* arrives as
a word the reader can take apart rather than a shape to memorise.

### One new word per lesson

`la lune`, then `lundi`, `mardi`, `mercredi`, `jeudi`, `vendredi`, `samedi`,
`dimanche`, then the chapter practice. Every block of the hand-written prose is
carried and several are promoted from an aside to the point of a lesson:

- **the `-di` rule** -- every weekday ends in Latin *diēs*, "day" -- is now a
  grammar lens in `FR-C07-lundi` rather than a sentence before a table. Half of
  five words comes free, and the reader is told so before meeting them.
- **`interpretatio germanica`** -- the Germanic peoples swapped their own gods
  into the Roman week role for role, which is why *mardi* and *Tuesday* are the
  same day sharing no sound -- is a typed culture claim owned by `FR-C07-mardi`,
  and the three days after it apply the key instead of restating it.
- **the weekend rewrite** -- *samedi* takes the **Sabbath** where English keeps
  Saturn, *dimanche* the **Lord's day** where English keeps the Sun -- is two
  lessons and two claims rather than one dense `cousinweb`.
- **the `di-` that moved to the front** of *dimanche* now gets a grammar lens
  that says the pattern flipped rather than broke.

The chapter's closing French/English recap survives as a **two-column** table in
the practice lesson, which the narrator can speak; the two four-column tables it
replaces were refused as unspeakable.

### Measured

- Hand-written French chapters: **12 -> 11**.
- Chapter atoms: **12**, exactly at `maxNewAtomsPerChapter`; French's
  `atomChapterSpikes` stays at **0**.
- Forward references: **50 -> 50**. The chapter contributes none.
- Narration refusals: **54 -> 52**.
- Measurable French lessons: **90 -> 99**.
- Culture claims: **14 -> 16**.
- The book compiles under XeLaTeX with `missing_character = 0`, and the rendered
  pages were read: the day recap sets as a real two-column table, one day a row.

## Chapter 6 is generated, and its ten numbers now arrive one at a time

Chapter 6 was hand-written LaTeX built from two schema-v1 lessons. Each of those
lessons taught **five numbers at once**, behind a four-column reveal table, and
declared no atoms at all -- so `renderBookChapter` refused to generate from
them, and every lesson-level gate reported on content no reader saw.

It is now twelve schema-v2 lessons, and this is authoring rather than a format
flip.

### One new word per lesson

`un`, `deux`, `trois`, `quatre`, `cinq`, `six`, `sept`, `huit`, `neuf`, `dix` --
one number per lesson, each with its Latin parent, its English cousins and its
own pronunciation note. Then two lessons that spend what the ten built:

- **`FR-C06-mois-romains`** carries across the hand-written grammar lens that
  explained why *septembre* to *décembre* say seven to ten and sit at nine to
  twelve. It introduces no new French word; its job is to make four facts the
  number lessons planted click together, so it practises those four atoms and
  records the Roman-calendar fact as a culture claim.
- **`FR-C06-practice`** is the chapter payoff, and it is now atom-scored against
  every atom the chapter introduces rather than carrying the legacy schema-v1
  note that said the payoff was authored rather than measured.

### Nothing was dropped, and there is more than there was

The hand-written chapter held **6 prose blocks** (2 `sounds`, 2 `cousinweb`,
2 `grammarlens`). The generated chapter holds **26** (10 `sounds`,
11 `cousinweb`, 5 `grammarlens`). Every block of the original writing is
carried: the nasal *un*, the rounded *eu* and silent *x*, the *trois*/*tres*
pair worn two ways from one Roman *trēs*, `cinque` on a die, the *six*/*dix*
on-and-off ending, the *octō* → *oit* → *huit* road and the *oct-* that survived
in *octobre*, the *dime* from *disme* ← *decima*, and the months two places out
of place. French's hand-written chapter count falls **13 -> 12**; the parity
number stays at 47 because Chapter 6's gap was already zero -- which is exactly
why it was the safe one to take first, and why the twelve that remain are not.

### Liaison is now a rule with a name, which closes an exam point

The hand-written chapter mentioned the *six ans* → *see-z-ans* and *neuf ans* →
*neu-v-ans* changes in passing, inside two `sounds` blocks. `FR-C06-six` now
teaches liaison as a named rule with its own atom -- final consonant sounded
when counting, silent before a consonant, back as a *z* before a vowel -- and
`FR-C06-neuf` extends it to the *f* → *v* softening rather than minting a second
atom for the same phenomenon.

That is what a probe can resolve against, so **A1-PRON-03 (obligatory liaison)**
moves from unmapped to covered. French A1 exam coverage: **25/74 -> 26/74**.

### Measured

- Chapter atoms: 12, exactly at `maxNewAtomsPerChapter`. French's
  `atomChapterSpikes` stays at 0.
- Forward references: **51 -> 50**. The chapter contributes none. The "Next:"
  pointer at the end of each lesson names the next number in English rather than
  in French, and the gender example uses *un jour* / *une nuit* from Chapter 1
  instead of the *café* Chapter 28 owns.
- Narration refusals: **56 -> 54**. The two wide reveal tables the narrator had
  to refuse are gone.
- The book compiles under XeLaTeX with `missing_character = 0`, and the rendered
  pages were read.

## Chapters 3, 4 and 5 are generated from their lessons, not hand-written

The standing directive is that no hand-written book chapter may remain: a
hand-written `.tex` is not built from lessons, so every lesson-level gate --
the five-minute ceiling, the one-headword ramp, the atom budget -- reports on
content the reader never sees and says nothing about the chapter that is
actually printed. French held sixteen such chapters, the joint-largest holding
in the corpus. Three of them are now retired.

### Carried the prose across first, so the flip deletes nothing

`handwritten_parity.py` scored French at **60 prose blocks** that a naive flip
would have silently dropped. Almost none of that writing was actually missing:
it was **mis-homed**. The renderer builds its boxes from four headings, and the
owner's prose was sitting under headings it does not map -- `## Across the
family`, a `###` sub-heading nested inside an etymology, a cross-language
comparison welded onto the tail of a grammar lens. So the work was re-homing,
not rewriting, and the prose survives verbatim.

- **Chapter 3** (gap 5 -> 0): `merci`'s three-metaphors-for-gratitude table
  becomes a `culture` block; `de rien`'s French/Spanish *rien*/*nada* parallel
  splits out of the etymology; the shrug comparison splits off the answer-set
  lens; the register ladder and the `Comment allez-vous` liaison get their own
  grammar and sounds blocks.
- **Chapter 4** (gap 3 -> 0): the *Auf Wiedersehen* pairing, the Latin *tarde*
  that became French "late" and Spanish "afternoon", and the *demain*/*mañana*
  twin each become `culture` blocks.
- **Chapter 5** (gap 5 -> 0): `travailler`'s Spanish *trabajar* twin becomes a
  `culture` block, "why French keeps its pronouns" becomes its own lens, and
  `français` gains the sounds and etymology blocks its prose already contained.

French's parity number falls **60 -> 47**. Chapters 1 and 2 were already at
zero and are prose-ready; the thirteen chapters from 7 up still hold 47 blocks.

### Migrated twenty lessons to schema v2, which is what the flip really costs

A generated chapter requires every one of its lessons to be schema v2 --
`renderBookChapter` throws otherwise. This, not the prose, is the real blocker
behind the retirement, and it is why these chapters had never moved.

- Twenty lessons gain typed knowledge atoms, per-block `hl-knowledge`
  boundaries, declared durations, spine nodes, and skills/modes/strands.
- Thirty-two new atoms, held inside the gentle-ramp policy rather than merely
  under the hard gates: `maxNewAtomsPerLesson` is 3 and `maxNewAtomsPerChapter`
  is 12, and the chapters land on 12, 10 and 10 with no lesson above 3.
- Seventeen headings that no block type recognised were re-homed onto headings
  the renderer emits. Nothing was dropped: `## Review first (20 seconds)` was
  merged into the warm-up it duplicated, and the accent lessons' "How to write
  it" sections became real `Writing:` blocks.
- Every lesson stays under the **computed** five-minute ceiling. The binding
  number is `max(declared, computed)`; the tightest are FR-C03-practice at 293s
  and FR-C03-comment-registers at 286s.

### What the reader gains

- Chapter 4 now prints the three accent lessons -- `é è ê`, `ç`, `ï ë ü`. The
  hand-written chapter never printed them at all; they existed only as lessons.
- Every lesson now carries its own "Your turn" and "Before you move on" boxes.
  The hand-written chapters had no per-lesson practice or recall in the book.
- Chapter openings carry the hand-written opening prose rather than the
  generator's boilerplate, and the three payoffs are now atom-scored instead of
  carrying a "legacy schema-v1, not atom-scored" note.

### Fixed a font gap the hand-written chapters had been hiding

Latin Modern Roman ships no precomposed `À`, `Ü`, `Ê`, `Ï`, `Ë` or `Ô`. The
hand-written `.tex` wrote them as LaTeX escapes, so the glyph-coverage gate
never saw them; a generated chapter emits the character itself. Four gaps were
fixed at source, in the track's own ASCII respelling system rather than by
widening the charset: the IPA `ɛ` in three pronunciation notes, the capital `À`
in the chapter-4 farewell table and dialogue, and `Ü` in the cedilla lesson.
`FR-C01-bien` was fixed too, so the same gap cannot block chapter 1.

## French chapters 1-16 regain their reading order (#12250)

- Add one global, spaced sequence to all 64 legacy lessons, recovered from the
  hand-authored book sections and closed against every prerequisite and review.
- Remove 64 missing-sequence findings plus 21 forward prerequisites and 28
  forward reviews that alphabetical filename fallback had fabricated. French's
  order-integrity backlog moves from 113 defects to zero.
- Keep genuine content-placement debt separate: eleven apparent
  forward-language uses disappear with the real order, while 52 still require
  teaching or reseating work. No learner content is silently declared taught.

## [Unreleased]

### Added — complete pre-A1 writing-stage runway

- Put four tiny writing lessons immediately after the already-taught *salut*:
  observe and trace, copy with the model visible, hide and copy after a short
  delay, then write from a heard cue. Every lesson is capped at 90 or 120
  seconds and introduces no new French vocabulary.
- Give French valid cumulative evidence for all four pre-A1 writing stages
  without conflating that foundation with DILF readiness. Contact details,
  numbers, forms, practical messages, timed mocks, and human scoring remain
  explicit later work.

### Added — sourced DILF A1.1 four-skill task shapes (HL18)

- Inventory the official 25-minute listening, 25-minute reading, 10-minute
  individual speaking, and 15-minute writing sections, including all seventeen
  activity families published by France Éducation international.
- Preserve the real 35/15/35/15 weighting and the grouped pass rule: 50/100
  overall plus 35/70 across the two oral tests. DILF publishes no independent
  four-skill floors, so the inventory records them as unknown rather than
  manufacturing four tidy percentages.
- Record item counts, text and response lengths, audio speed and length, replay
  counts, aids, and part-level score allocations as explicitly unpublished.
  The separate assessment contract keeps the book's stricter 60%-per-skill
  two-mock readiness margin.

### Added — DILF/DELF/DALF assessment contract through C2 (HL16)

- Name the official external target at every rung: DILF A1.1 as the closest
  pre-A1 runway, DELF *tout public* at A1–B2, and DALF at C1–C2.
- Preserve the awarding body's real aggregate and eliminatory score rules while
  requiring a safer local 60% in reading, listening, writing, and speaking on
  both timed mocks. A strong receptive score may not hide weak writing.
- Require the complete gentle writing sequence from tracing through timed
  independent production, without lengthening any lesson beyond five minutes.
- Keep readiness honest: only French pre-A1 and A1 have task-shape inventories
  today; later inventories, mocks, rubrics, answer keys, calibration, and
  book-only human validation remain named dependencies.

### Added — Chapter 32, asking a question (HL-C229)

Nine lessons. **L'interrogation 0/5 → 5/5**; French A1 exam coverage **27% → 34%**.

Before this chapter a reader could say a great deal in French and **could not ask
anything**. The chapter teaches all three ways and, more importantly, when each is
used:

- **raising your voice** — costs no new words at all, and is by far the commonest
  in speech;
- **est-ce que** — neutral, and the default in writing;
- **inversion** — formal, and the one a beginner is usually taught *first* and
  then sounds like a textbook forever.

The order is deliberate: inversion is the one to **recognise** immediately and
**produce** last.

Five question words follow — *où, quand, comment, combien, pourquoi* — and the
chapter closes on **quel**, the only question word that agrees with its noun, whose
four forms all sound identical so the agreement is purely a spelling problem.

**It also realizes `SPINE-ASK-LOCATION`**, an A1 spine node French had explicitly
declared *omitted*. That declaration is no longer true and the ledger says so.


## Sixteen pre-A1 nouns, and what the level gate did — 2026-08-08

- Authored **Chapters 28–31**: sixteen everyday nouns, one per lesson, all
  filed under pre-A1 spine nodes through new `FR-PATH-025..028` segments and
  `FR-EXT-025..028-LANGUAGE-SPECIFIC` extensions:

  | Chapter | Spine node | Words |
  |---|---|---|
  | 28 Coffee, Tea, Milk, Sugar | `SPINE-POLITE-REQUEST-REPAIR` | café, thé, lait, sucre |
  | 29 The People You Introduce | `SPINE-EXCHANGE-NAMES` | ami/amie, famille, enfant, personne |
  | 30 Cheese, Butter, Salt, Egg | `SPINE-POLITE-REQUEST-REPAIR` | fromage, beurre, sel, œuf |
  | 31 Eyes, Nose, Mouth, Stomach | `SPINE-CHECK-WELLBEING` | œil/yeux, nez, bouche, ventre |

  This continues the pre-A1 vocabulary probe HL-C53 ran on Hindi, Arabic and
  Tamil: `vocabularyOf()` counts distinct `headword:` strings, one per
  lesson, so sixteen lessons move `levelGate.tracks[french].vocabulary` by
  exactly sixteen — 26 → 42 (of the 300 target), shortfall 274 → 258.
- **Checked against the lessons directory before writing, not assumed.**
  *l'eau*, *le vin*, *le pain*, *le père/la mère*, *le frère/la sœur*,
  *la tête* and *la main* are already taught; this tranche adds only the
  words genuinely missing.
- **Gender and elision made explicit, never an afterthought.** Every noun
  carries its article at the point of teaching. `FR-GRAMMAR-ELISION-ARTICLE-02`
  formalises, for the first time as its own atom, the *le/la → l'* rule
  already used quietly at *l'eau* (Chapter 11) — introduced at *l'ami*, reused
  at *l'enfant*, *l'œuf* and *l'œil*. Chapter 29 lines up four different
  relationships between grammatical gender and the person named: *l'ami* /
  *l'amie* changes with its referent, *la famille* is fixed regardless,
  *l'enfant* is one spelling under two articles, *la personne* is fixed
  feminine even naming a man.
- **Cognates verified, not assumed.** *sel*/*salt*, *œuf*/*egg* and
  *nez*/*nose* are genuine common-descent cousins from a shared PIE root;
  *lait*/*milk* share no root at all; *café*/*coffee* and *beurre*/*butter*
  are borrowings, not inheritances — and *café* and *thé* are a matched pair
  with opposite stories, one word taking a single road through Ottoman
  Turkish, the other forking in two depending which Chinese port a trader
  dealt with.
- **Etymology corrected against sources during authoring:** the "Roman
  soldiers were paid in salt" story behind *salary* appears in no ancient
  source and is recorded as a later legend, not fact; *persona* = "sound
  through" is flagged as doubted on phonetic grounds, with Etruscan *phersu*
  given as the likelier root; *boútūron* "cow-cheese" (behind *beurre*/
  *butter*) is treated as a probable folk-reshaping of a foreign word, since
  butter was never native to Greece or Rome.
- **Household objects considered and dropped**, matching the finding all
  three prior tranches reported independently: the seven pre-A1 spine nodes
  are all social speech acts and hold no concept for a concrete object.
- **Reinforcement, closed to zero.** Every lesson practises the one to three
  lessons before it; each chapter's payoff reaches back further —
  `FR-C28-sucre` recovers *si* and the *langue d'oïl* / *langue d'oc* split
  from Chapter 18; `FR-C29-personne` closes a four-way gender synthesis and
  reassesses Chapter 18's *non*; `FR-C30-oeuf` reaches to Chapter 29's
  elision rule; `FR-C31-ventre`, the tranche's grand payoff, names one word
  from each new chapter and re-closes three Chapter 17 *tête* atoms nothing
  had revisited since it was written. All nine pre-A1 atoms the continuity
  ledger reported as revisited fewer than twice are now revisited at least
  twice: `levelGate.tracks[french]` reinforcement blocker clears from 9 to 0.
- All sixteen lessons derive `coreModality: voice`; the book compiles under
  XeLaTeX at 156 pages with zero `Missing character` warnings. Two raw glyphs
  caught during authoring and fixed before commit: *ȳ* (in *būtȳrum*,
  *tȳrós*, *cȳse*) has no Latin Modern Roman glyph, simplified to *y*; a
  literal IPA *ɔ* and bracketed IPA notation were removed in favour of plain
  respelling, matching house style.
- The atom-budget blocker (3 lessons: `FR-C17-main`, `FR-C17-tete`,
  `FR-C18-oui`) is pre-existing debt from before this branch and is
  untouched.

## Eight verbs no track had ever taught — 2026-08-07

- Authored **Chapters 26 and 27**: eight canonical verb concepts that **no
  track in the corpus realised** before this tranche. One verb per lesson,
  gone deep, `concept_tag` verbatim:

  | Lesson | Concept | Verb |
  |---|---|---|
  | `FR-C26-entendre` | `VERB-HEAR` | entendre |
  | `FR-C26-dormir` | `VERB-SLEEP` | dormir |
  | `FR-C26-marcher` | `VERB-WALK` | marcher |
  | `FR-C26-courir` | `VERB-RUN` | courir |
  | `FR-C27-sasseoir` | `VERB-SIT` | s'asseoir |
  | `FR-C27-se-lever` | `VERB-STAND` | se lever / debout |
  | `FR-C27-ouvrir` | `VERB-OPEN` | ouvrir |
  | `FR-C27-fermer` | `VERB-CLOSE` | fermer |

  French goes from **14 of the 40** core verbs to **22 of 40** (35% → 55%), the
  deepest verb coverage in the corpus. The corpus-wide `universallyMissing`
  list — concepts nobody teaches — falls from 15 to 7.
- **Two chapters, not one.** Eight one-verb lessons introduce twenty-two atoms
  against a per-chapter budget of twelve. Split, Chapter 26 introduces **11**
  and Chapter 27 introduces **11**, so the ramp report finds **zero** lesson or
  chapter violations for either and the corpus totals hold at 40 and 25.
- ***entendre*** gets the tranche's signature block, and the framing is
  corrected against the record. It is Latin *intendere*, "**to stretch
  toward**" — English's **intend** is the plainest survival — but the "hear"
  sense is **not** a late arrival: it is attested from the oldest French texts
  (c. 1050), alongside attending, understanding and intending. What changed
  later is that the others fell away as *ouïr* (← *audīre*) wore out, leaving
  them in *entendu*, *s'entendre* "to get along" and *l'entente*. English kept
  the *audīre* family instead. Three mind-verbs, three body pictures:
  *comprendre* grasps, *penser* weighs, *entendre* stretches.
- **Reflexive body positions, honestly taught.** *S'asseoir* ships with **two**
  accepted paradigms; the lesson teaches the *-ie-* series whole and marks the
  *-oi-* singulars as ordinary speech whose plurals (*assoyons*, *assoyez*) are
  much rarer in France, rather than presenting the two as symmetric. *Je suis
  assis* is separated from *je m'assieds* as state against movement. And French
  has **no single verb** for "to stand": the movement is *se lever*, the state
  is *être debout* — *de* + *bout*, "on end."
- **The conjugation trap is named.** *Ouvrir* ends in *-ir* and takes *-er*
  endings, the same set as *marcher*; *fermer* owns them outright, so the two
  opposites drill as one pair.
- **Every uncertain link is marked uncertain.** *Marcher*'s origin is left
  **unsettled** between Frankish **markōn* and Latin *marcus* "a hammer";
  English *march* is secure (a borrowing *from* French) while English *mark* is
  a relative **only if** the Frankish account holds, and *la marche*
  "borderland" is flagged as a separate arrival from the Germanic noun. The
  **dormouse** is named as **folk etymology** — the story needs an Anglo-Norman
  word nobody has found. Latin *carrus* (→ *car*, *cargo*) is a **Gaulish
  loan** sharing *currere*'s root, not a form built from it. English *butt* is
  a **loan back out of French**, not a Germanic cousin of *bout*; the cousin by
  descent is **beat**. *Ouvrir* is *aperīre* **reshaped** on its antonym,
  "probably" — not a blend of two unrelated verbs. And *farm* ← *firmāre* is
  given as the mainstream account with Old English *feorm* named as
  interference.
- **Reinforcement at two cadences (HL09 §7).** Every lesson practises atoms
  from the **one to three lessons immediately before it**, across the chapter
  seam — the only cadence that can close the R1 window, since a chapter-end
  payoff is out of range for the chapter's opening material. On top of that,
  each payoff reaches several chapters back: `FR-C26-courir` uses *courir*'s
  hard *c* to re-earn why *canis* became *chien* (Ch.22), and runs *il fait
  chaud* / *il pleut* (Ch.21) and *ne … pas* (Ch.18) against the new verbs;
  `FR-C27-fermer` opens and closes *la main* (Ch.17), closes because *il pleut*
  (Ch.21), and asks in both registers before apologising (Ch.19, Ch.20).
  Measured: **eleven** French atoms that nothing had ever revisited now are —
  `FR-ETYMON-CHIEN-03`, `FR-ETYMON-CHAT-05`, `FR-ETYMON-MAINTENIR-05`,
  `FR-SOUND-MAIN-03`, `FR-GRAMMAR-IL-FAIT-03`, `FR-LEX-IL-PLEUT-04`,
  `FR-GRAMMAR-NEGATION-04`, `FR-LEX-DESOLE-02`, `FR-PRAGMATICS-SORRY-04`,
  `FR-ETYMON-COMPRENDRE-02`, `FR-ETYMON-PENSER-05`. French's
  `atomsNeverRevisited` falls **33 → 25** while its atoms rise **54 → 76**.
  Two of the tranche's own atoms remain unrevisited — `FR-ETYMON-COURIR-11`
  and `FR-ETYMON-LEVER-05` — and that is recorded, not padded away.
- **Payoffs clear the floor on their own merits**: Chapter 26 assesses **9 of
  11** of its atoms (0.82), Chapter 27 **8 of 11** (0.73), both well above the
  0.5 representativeness floor, with no `assesses` list padded.
- **One pre-existing defect became visible and is not hidden.**
  `FR-C16-passe-compose-etre` drills the learner on *Marcher, courir, nager* and
  *danser* as movement verbs that take *avoir* — none of which the track taught.
  Two of those four are now taught, ten chapters later, so the continuity report
  reports two new forward references (corpus 510 → 512). The earlier lesson was
  left alone: deleting the words would move the number without fixing the
  defect, which is that a Chapter 16 grammar lesson has no taught movement verb
  to point at.
- All eight lessons are schema v2 and modality `voice`: both chapters are
  4-of-4 drivable, there is no table anywhere in them, and the six present-tense
  forms are always a spoken bullet list. Effective durations **295–299 s**
  against the 300 s ceiling.
- Wiring: `curriculum.json` gains `FR-PATH-024`, a third `SPINE-SAY-WHAT-I-DO`
  segment carrying all eight in reading order, and the eight concepts drop out
  of that node's `omits` (28 → 20). `chapters.json`, `core/book-generation.json`,
  `book.tex`, the generated ch26/ch27 TeX and the ch26/ch27 narration follow.
  The French book compiles with XeLaTeX at **134 pages** (was 114) with zero
  `Missing character` warnings and zero overfull or underfull boxes.

## Eight more core verbs, split across two chapters — 2026-08-07

- Authored **Chapters 24 and 25**: the eight canonical verb concepts Spanish,
  Latin and Portuguese landed in their own second tranche, so every lesson here
  turns a three-way cross-language join into a four-way one. One verb per
  lesson, gone deep, `concept_tag` verbatim:

  | Lesson | Concept | Verb |
  |---|---|---|
  | `FR-C24-prendre` | `VERB-TAKE` | prendre |
  | `FR-C24-demander` | `VERB-ASK` | demander |
  | `FR-C24-aider` | `VERB-HELP` | aider |
  | `FR-C24-aimer` | `VERB-LIKE-LOVE` | aimer |
  | `FR-C25-comprendre` | `VERB-UNDERSTAND` | comprendre |
  | `FR-C25-penser` | `VERB-THINK` | penser |
  | `FR-C25-lire` | `VERB-READ` | lire |
  | `FR-C25-ecrire` | `VERB-WRITE` | écrire |

  French goes from **6 of the 40** core verbs to **14 of 40** (15% → 35%), the
  second-deepest verb coverage in the corpus after Latin.
- **Two chapters, not one, and the split is the point.** Eight one-verb lessons
  introduce twenty atoms against a per-chapter budget of twelve. Every track in
  the previous wave broke that ceiling; splitting is the resolution rather than
  raising the budget. Chapter 24 introduces **11** atoms and Chapter 25 **9**,
  so the ramp report finds **zero** lesson or chapter violations for either —
  the corpus totals hold at 40 and 25 exactly.
- **Ordering follows the etymology, not the alphabet.** *Comprendre* is *com-*
  + *prendre*, so *prendre* is taught first and Chapter 25 opens by spending
  Chapter 24 rather than by teaching a paradigm twice. The learner meets the
  irregular stem once (*je prends / nous prenons / ils prennent*) and gets the
  second verb almost free — which is stated in the lesson as the reason the
  drill was worth doing.
- **Two things French does that English does not, each given its own block.**
  *Aimer* covers both "like" and "love": the **thing named** chooses, and adding
  *bien* makes the sentence **weaker**, not stronger — *je t'aime bien* is the
  polite refusal, and that is the single most reliable way for an English
  speaker to say the wrong thing here. *Demander* is flagged as a **false
  friend**: it is the ordinary, polite word for asking and carries none of
  English *demand*'s force.
- **Every resemblance that is not real is named rather than skipped.** English
  *help* is Germanic and unrelated to *aider*; English *think* is Germanic and
  unrelated to *penser*; *juvenile* is from *iuvenis*, not *iuvāre*;
  *impregnable* has nothing to do with pregnancy; *le livre* is *liber* and not
  in *legere*'s family; Greek *legein* is *legere*'s **cousin**, not its parent.
  And where a trail genuinely ends, the lesson says so: *amāre* is given **no**
  reconstructed ancestor, because it has none that is agreed.
- **Reinforcement reaches back past the chapter (HL09 §7).** The corpus was
  measured at 50% of taught atoms never revisited, median zero. Both payoffs
  therefore re-practise earlier material in the guided practice, not as a
  name-check: `FR-C24-aimer` says *j'aime le chien* / *j'aime le chat*
  (Chapter 22), *j'aime le vert* (Chapter 23) and repeats the *s'il vous plaît*
  request with its register pair (Chapter 19); `FR-C25-ecrire` says *j'écris à
  la main* and pulls **manuscript** apart into *manus* + *scrībere*, which is
  Chapter 17's hand doing real work rather than being mentioned. Chapter 24's
  *demander* also re-earns *manus*, since *mandāre* contains it.
  Measured effect: **eight French atoms that had never been revisited by
  anything now are** — `FR-LEX-MAIN-02`, `FR-ETYMON-MAIN-04`, `FR-LEX-CHIEN-02`,
  `FR-LEX-CHAT-04`, `FR-LEX-PLEASE-02`, `FR-GRAMMAR-PLEASE-REGISTER-04`,
  `FR-LEX-NON-02`, `FR-SOUND-NON-03`. Corpus `atomsTaught` moves 1519 → 1539
  while `atomsNeverRevisited` **holds at 767**, so the tranche pays for its own
  new atoms and eight older ones besides.
- **No forward references.** Every French word used in either chapter is one
  the track has already taught (*le pain*, *l'heure*, *la main*, *le chien*,
  *le chat*, *le vert*, *oui*, *non*, *s'il vous plaît* / *s'il te plaît*, *le
  français*), and no lesson teases a word a later one will introduce. The
  continuity report finds **zero** forward references in Chapters 24 and 25;
  the corpus total holds at 517.
- **Wiring.** [`curriculum.json`](./curriculum.json) gains `FR-PATH-023`, a
  second `SPINE-SAY-WHAT-I-DO` segment carrying all eight in reading order, and
  the eight concepts drop out of that node's `omits` (36 → 28). No extension was
  needed: every lesson realises a canonical concept, and the prerequisite
  closure reaches back only to lessons already earlier on the local path
  (`FR-C17-main`, `FR-C18-oui`/`FR-C18-non`, `FR-C19-sil-vous-plait`,
  `FR-C22-chien-chat`, `FR-C23-vert-jaune`), so nothing had to be dragged under
  a node it does not realise. [`chapters.json`](./chapters.json),
  `core/book-generation.json`, `book/book.tex`, the generated ch24/ch25 TeX and
  the ch24/ch25 narration all follow.
- All eight lessons are schema v2 and modality `voice`: both chapters are 4-of-4
  drivable, no table anywhere exceeds three columns, and the six present-tense
  forms are always a spoken bullet list rather than a paradigm grid. Computed
  durations run **247–285 s** against the 300 s ceiling.
- The book compiles under XeLaTeX at **114 pages** (was 98) with **zero**
  `Missing character` warnings and **zero** overfull or underfull boxes.

## French joins the cross-language verb corpus — 2026-08-07

- Retagged the track's six shared verbs from language-local `FR-VERB-*` ids to
  the canonical concepts every other track already realises: *être* →
  `VERB-BE`, *avoir* → `VERB-HAVE`, *aller* → `VERB-GO`, *parler* →
  `VERB-SPEAK`, *habiter* → `VERB-LIVE`, *travailler* → `VERB-WORK`. French
  contributed **zero** core verbs to the cross-language join before this and
  now contributes **six**; it is the only track in the corpus that realises
  `VERB-WORK`, which stops being universally missing.
- Left `(s')appeler` on `FR-VERB-APPELER`. No core concept covers the
  reflexive naming verb, and inventing one to make the number look better
  would be a false realisation.
- Rewired [`curriculum.json`](./curriculum.json) so the realisation path tells
  the truth about those six lessons. A canonical `VERB-*` concept is owned by
  `SPINE-SAY-WHAT-I-DO`, so each retagged lesson has to sit in a segment for
  **that** node — the node French previously realised with nothing at all
  (`segments: []`, all forty-two verb concepts in `omits`). Four new
  `SPINE-SAY-WHAT-I-DO` segments now carry them, each spliced in at exactly
  the point the lesson already occupied so no chapter moves and no lesson is
  renumbered:
  - `FR-PATH-010` — *aller*, lifted out of the head of the old Chapter 3
    wellbeing segment and placed immediately before it, where it already sat.
  - `FR-PATH-014` — *parler*, *habiter*, *travailler*, lifted off the front of
    the long Chapter 5–13 support run.
  - `FR-PATH-016` — *avoir*, with `FR-C14-age` following it as local support,
    because *j'ai X ans* is built out of *avoir* and has to stay behind it.
  - `FR-PATH-017` — *être*, preceded by the two Chapter 15 past-tense lessons
    it declares as prerequisites.
- Put **four lessons on the path that had never been on it**: *travailler* and
  *être* (both now shared realisations, so the path must carry them) and, as
  the prerequisite closure *être* pulls in, `FR-C15-passe-compose` and
  `FR-C15-passe-simple`. The two past-tense lessons are French-local support,
  so they are attached as an extension rather than pretending to realise a
  shared concept.
- Split the old Chapter 5–13 segment where *avoir* had to be extracted, and
  split its language-specific extension to match, since an extension's lessons
  must all live inside the single segment it attaches to. Segment and
  extension ids are renumbered to stay ascending in path order, which is the
  convention every other track follows; `FR-EXT-010` is gone because *aller*
  was its only lesson and *aller* is no longer local support.
- Dropped the six concepts from `SPINE-SAY-WHAT-I-DO`'s `omits` and refreshed
  every node's `segments` ledger against the authored path.
- Consequence worth naming: `SPINE-SAY-WHAT-I-DO` is an A2 node, so the nine
  lessons now sitting under it are derived as A2 rather than A1. French's
  reach moves from A1 to A2 — the same reach the other fifteen verb-realising
  tracks already report — and five lessons leave the ramp-to-A1 edition.

## Chapter capability ledger — Chapters 17–23 — 2026-08-06

- Added [`chapters.json`](./chapters.json), the track's HL05 capability ledger:
  a first-person `canDo`, the shared-spine nodes realised, and a validated
  payoff for each of the **seven** chapters that carry schema-v2 lessons.
- Deliberately authored **only Chapters 17–23**. Chapters 1–16 are still schema
  v1 and declare no `practises.knowledge`, so every payoff written for them
  would have to assess atoms that do not exist. They are omitted rather than
  stubbed: an absent entry is honest debt the gap report can count, a
  placeholder is destroyed signal.
- Every `payoff.assesses` list is a strict subset of the payoff lesson's own
  `practises.knowledge`; no atom is invented, and none is padded to clear a
  threshold.
- Chapters 19–23 are single-lesson chapters, so their payoff assesses
  everything the chapter introduces (representativeness 1.00). Chapters 17 and
  18 have **no terminal consolidation lesson**, so their payoff is the last
  lesson by `sequence`: Chapter 18 still reaches 4/7 = 0.57 because *non*
  reassesses *oui*, and Chapter 17 sits exactly on the 0.50 floor at 4/8. Both
  facts are recorded in the ledger's per-chapter `summary`.
- Titles and labels are copied verbatim from `core/book-generation.json`, so the
  `chapter-title-drift` gate holds through the HL-C04 inversion.

## Warning-free 98-page book — 2026-08-03

- Added natural page bottoms for intentionally short micro-lessons and concise
  optional running titles for six long legacy section headings.
- Made two Chapter 12 bookmarks prose-only, allowed three internal source paths
  to break naturally, and gave five dense comparison tables flexible final
  columns without removing any vocabulary, grammar, or etymology.
- Reflowed one pronominal-verb explanation into clearer sentence boundaries
  while preserving its agreement rule and examples.
- A forced XeLaTeX build now reports zero missing glyphs, overfull or underfull
  boxes, duplicate destinations, Hyperref warnings, or LaTeX warnings across
  all 98 pages.

## Canonical Chapters 17–23 — 2026-08-03

- Migrated the nine lessons in Chapters 17–23 to schema version 2 with typed
  blocks, explicit shared-spine concepts, prerequisite-closed knowledge atoms,
  and honest sub-five-minute duration contracts.
- Generated seven LaTeX chapters from those canonical lessons and added
  independent Language Ladder source-hash and lesson-count assertions, so the
  app and downloadable book now consume one source of truth through Chapter 23.
- Expanded the book from 79 to 98 pages. A forced XeLaTeX build has no missing
  glyphs, duplicate destinations, LaTeX warnings, or leaked generator metadata;
  all 98 rendered pages and the 25-entry outline were inspected.
- Recorded the unchanged baseline of sixteen overfull boxes, nine underfull
  boxes, and six Hyperref warnings for the focused HL-B19 cleanup tranche.

## Sub-five-minute remediation — 2026-08-02

- Corrected twenty-two declared five- or six-minute estimates whose lesson
  bodies already compute below 300 seconds.
- Replaced three computed violations with three prerequisite-ordered support
  lessons for explicit register/liaison, *être* suppletion, and pronominal past
  agreement.
- Preserved the vocabulary, grammar, etymology, exceptions, and cross-language
  depth. The shared report now measures zero French duration violations.
- `FR-C03-practice` at 293 computed seconds and `FR-C15-passe-simple` at 291 are
  the tightest remaining French lessons and should be watched during copy edits.

## The book catches up -- Chapters 3-16 typeset

The lessons had run ahead of the published artifact: 61 authored lessons through
Chapter 16, but the LaTeX book still stopped at Chapter 2 ("Introducing
Yourself"). Because the CI book build only compiles what is wired into
`book.tex`, the missing chapters were invisible to CI and the gap drifted
silently. This closes it -- **fourteen new book chapters**, written from the
existing `FR-C03`-`FR-C16` lessons and wired into `book.tex`:

- **Ch3** How Are You (merci, de rien, aller, comment ca va, comme ci comme ca)
- **Ch4** Farewells (au revoir, a plus tard, a bientot, a demain)
- **Ch5** The First Verbs (parler, habiter, travailler, je parle francais)
- **Ch6** Numbers One to Ten * **Ch7** The Days of the Week (the planet-gods)
- **Ch8** Telling the Time * **Ch9** Months and Seasons
- **Ch10** Family (parents, freres/soeurs -- with the Grimm's-law table)
- **Ch11** Bread, Water, Wine * **Ch12** Numbers Eleven to Twenty
- **Ch13** Colours * **Ch14** To Have, and How Old You Are (avoir, age)
- **Ch15** The Compound Past (passe compose, passe simple)
- **Ch16** To Be, and the Past That Takes It (etre, and the verbs that take it)

Each chapter follows the established book conventions: one `\section` per lesson
with a slug `\label`, the `cousinweb` / `culture` / `grammarlens` / `sounds`
boxes (the only four this book's preamble defines), `booktabs` tables, and every
atom traced to its root. Content is faithful to the lessons -- no new etymologies
introduced. Practice-section labels are chapter-qualified (`lesson:chN-practice`)
so they stay unique.

The book grows to **79 pages**; compiles clean with XeLaTeX (0 errors, 0 missing
characters, 0 undefined references, 0 duplicate labels) and was rasterized and
visually QA'd -- the PIE forms (*ph2ter, *bhreh2ter), the `oe` ligature in
*soeur*, and the nested Grimm's-law table all render correctly.

Also fixed: `FR-C07-jours-1.md` called Tiw "the Norse war-god". Tiw is the Old
English form (the Norse cognate is Tyr), so it now reads "the Germanic war-god"
-- matching how the German track's parallel lesson already phrased it.

## Chapter 17 — The body: a head that was a pot, and a hand inside English

- **Chapter 17 authored** (`FR-C17-tete`, `-main`) — the **body**, which is the
  theme all four parallel-track roadmaps name next after family and food.
- **la tête** (`FR-C17-tete`): the headline is that **French threw away the Latin
  word for "head."** *Tête* is not from *caput* but from ***testa***, an
  **earthenware pot** — Roman soldiers' slang for the skull, the way English says
  *noggin*. The joke replaced the real word, so *j'ai mal à la tête* is
  historically "**my pot hurts**."
  - *Caput* is shown surviving where it did: French **chef** ("chief" = head) and
    **chapitre**, and abroad in *cabeza*, *capo*, *captain*, *decapitate*.
  - The **circumflex is a receipt** — *testa* → Old French *teste* → *tête* — and
    the payoff is that English, which borrowed *before* the *s* fell, still has
    **test**, originally the shallow pot an alchemist assayed metals in.
- **la main** (`FR-C17-main`): flagged as **feminine despite a consonant
  ending**, because French gender is not predictable from the ending and this
  course always supplies the article. From ***manus***, presented as the most
  productive hand in English — *manual*, *manuscript*, *maintain*, *manage*,
  *manoeuvre* — with two things done deliberately:
  - **manufacture** ("made by hand") is called out as now naming precisely the
    thing that isn't.
  - **maintenir** = Latin *manū tenēre*, "hold in the hand" — so *maintenance* is
    literally keeping something in hand. (A draft cited "Ch. 14's *tenir*"; the
    French track has **no *tenir* lesson** — Ch. 14 is *avoir* and *âge*. Caught
    by grepping. *Ter* ← *tenēre* is the **Portuguese** Ch. 14.)

## Chapter 16 — *être*, and the half of the past Chapter 15 couldn't reach

- **Chapter 16 authored** (`FR-C16-etre`, `-etre-roots`,
  `-passe-compose-etre`, `-pronominal-past`). Chapter 15
  could only teach the *avoir* half of the compound past, because ***être* was
  taught in no lesson of any track**. This chapter supplies it and closes the
  other half.
- **être / its roots** (`FR-C16-etre`, `-etre-roots`): the six present forms, presented honestly as
  unpatternable, and then explained. *être* is **suppletive across three stems**:
  - *es-* — *suis/es/est/sommes/êtes/sont* **and the infinitive** *être*
    (← \**essere*), all from Latin ***esse***
  - *fu-* — the passé simple *je fus, il fut* (← the old perfect ***fuī***, PIE
    \**bʰuH-*), which ties straight back to Ch. 15's tense
  - ***ét-*** — **every form beginning *ét-***, from ***stāre***, "to stand":
    the participle *été*, the present participle *étant* (← *stantem*), and the
    whole imperfect *étais/était* (← *stābam*). Stated as a **limb**, not a stray
    form, because that is what it is — and it makes the lesson's own thesis
    stronger, not weaker.
  - The payoff is comparative: *stāre* is exactly the verb **Spanish** kept as
    ***estar*** (**ES-C04**, contrasted with *ser* in ES-C09). Spanish keeps the
    two **apart** and makes you choose; French kept *esse* and **swallowed a
    large piece** of the other. Noted that *stāre* also left French words outside
    *être* (*rester* ← *re-stāre*, *coûter* ← *constāre*) — what it didn't do is
    survive as a **separate** "to be" the way *estar* did. Anchored to English
    *go/went* so "suppletion" names something the learner already does.
- **passé composé with être / pronominal agreement**
  (`FR-C16-passe-compose-etre`, `-pronominal-past`): verbs of **motion
  and change of state** take *être* — taught as a **shape** (going, coming, being
  born, dying) rather than a list to memorise — built on *aller* from Ch. 3,
  **plus all pronominal verbs**. Then the visible part: the participle **agrees
  with the subject** (*elle est allé**e***, *elles sont allé**es***).
  - Two warnings included so the rule doesn't mislead: plain motion verbs
    (*marcher, courir, nager, danser, voyager*) take *avoir*, and
    *monter/descendre/sortir/rentrer/passer* **switch to *avoir* when
    transitive** (*j'ai monté les valises*).
  - **A dedicated "pronominal exception" section**, because the two additions
    above would otherwise contradict each other: pronominal verbs take *être* but
    their agreement follows the ***avoir*** rule — a **preceding direct object**,
    which is usually the reflexive pronoun and sometimes isn't. *Elle s'est
    lavé**e*** (reflexive = direct object) vs *elle s'est lav**é** les mains*
    (object follows) vs *elles se sont parl**é*** (reflexive is indirect). This
    lands as a *third* sighting of Ch. 15's rule rather than a new one — the
    auxiliary looks like *être*, the agreement behaves like *avoir*. And the one
    group that escapes even that: **essentially pronominal** verbs (*se
    souvenir*, *s'enfuir*, *s'évanouir*), which have no non-reflexive form for
    the pronoun to be an object *of*, so they agree with the subject after all
    (*elles se sont souvenu**es***).
  - The chapter's real argument is that this is **not a second rule**. Ch. 15
    established that *j'ai parlé* was once "I have [a thing] spoken", with the
    participle an **adjective** agreeing with the object — which is why it still
    agrees with a *preceding* object. *Elle est allée* was likewise once "**she
    is** gone", *gone* describing **her**. One idea underneath both: **the
    participle was an adjective and agrees with whatever it described.** The two
    auxiliaries differ only in what that was.
- Prerequisites and `reviews_of` verified against existing ids (Ch. 3 *aller*,
  Ch. 14 *avoir*, Ch. 15 both lessons).

## Chapter 15 — The compound past, and the tense it drove out

- **Chapter 15 authored** (`FR-C15-passe-compose`, `-passe-simple`): the everyday
  past, built on Ch.14's *avoir* — reviewing Ch.5/14 via `reviews_of`.
- **passé composé** (`FR-C15-passe-compose`): *avoir* + past participle (*-er*→*-é*
  ← *-ātum*, *-ir*→*-i* ← *-ītum*, *-re*→*-u* ← *-ūtum*), noting that *parler*,
  *parlé* and *parlez* are **homophones** — three spellings, one sound. The
  etymology carries the lesson: *j'ai parlé* was once literally "**I have [a thing]
  spoken**," from Latin *habeō litterās scriptās* ("I have letters written") — a
  **possessive** in which the participle was an **adjective** agreeing with the
  object. Over centuries "I possess a written thing" slid into "I wrote," so a
  possessive construction **hardened into a tense** — and the fossil is still
  working: when the object comes first, the participle **still agrees** (*les
  lettres que j'ai écrit**es***), a two-thousand-year-old adjective ending doing its
  old job.
- **passé simple** (`FR-C15-passe-simple`): *il parla* ← Vulgar Latin **\*parabolāvit**, the
  direct inheritance — framed as recognise-don't-produce, since it fills the past
  tense of essentially all French literature and appears in no conversation. Its
  value here is comparative: it is the **same tense** as Spanish *habló*, Portuguese
  *falou* and Italian *parlò*, and the chapter closes on the cross-language
  observation that **French, German and Italian all** built a "have"
  compound and let it push the inherited simple past out of speech, while
  **Spanish and Portuguese, at the western edge, kept theirs**.
- Taxonomy: namespaced `FR-PAST-COMPOUND`, `FR-PAST-SIMPLE-LITERARY`.

## Chapter 14 — avoir, and having your years

- **Chapter 14 authored** (`FR-C14-avoir`, `-age`): the verb the rest of the
  course is built on, reviewing Ch.5/9/12/13 via `reviews_of`.
- **avoir** (`FR-C14-avoir`): *j'ai/tu as/il a/nous avons/vous avez/ils ont* —
  with the observation that the three singular forms are **homophones** (*ai · as
  · a*), so only the pronoun tells you who. Etymology: ← *habēre*, and the payoff
  is that Chapter 5 already taught this root — **habiter** ← *habitāre* is
  *habēre*'s frequentative, "to keep having a place," so *avoir* and *habiter* are
  the same word twice. English took the family whole: *habit* (what you have
  regularly), *inhabit*, *exhibit* ("hold out"), *prohibit* ("hold back"). Plus how
  far it wore down — Latin *habeō* → **j'ai**, a single vowel sound, French's usual
  erosion (cf. *aqua* → *eau*).
- **j'ai vingt ans** (`FR-C14-age`): age takes **avoir**, never *être*, and *ans*
  is **obligatory** where English drops "years old." *An* ← *annus* →
  *annual/anniversary/annals*. Includes **liaison** — the silent *t* of *vingt*
  wakes up before a vowel: *vin-t-an*. Closes on the five-language table:
  **French, Spanish, Italian and Portuguese all *have* their years; German and
  English *are* theirs** — age as possession vs age as identity.
- Sets up the compound past: *avoir* is the auxiliary the *passé composé* needs.
- Taxonomy: namespaced `FR-VERB-HAVE`, `FR-AGE`.

## Chapter 13 — Colours

- **Chapter 13 authored** (`FR-C13-noir-blanc`, `-rouge-bleu`): the **borrowing**
  chapter, reviewing Ch.11/12 via `reviews_of`.
- **noir & blanc** (`FR-C13-noir-blanc`): *noir* ← Latin *niger* (→ *denigrate*) is
  the expected inheritance — but **blanc is not from Latin *albus*** at all. It is
  **Frankish *blank*** ("shining, gleaming"), a **Germanic** word borrowed **into**
  French: the reverse of the usual Latin→Germanic flow, and it displaced *albus*
  entirely, probably by being the more **vivid** option. *Albus* didn't die, it just
  stopped being a colour: **aube** ("dawn"), *aubépine* ("white thorn"), *album*,
  *albinos*.
- **rouge & bleu** (`FR-C13-rouge-bleu`): *rouge* ← *rubeus* ← PIE ***h₁rewdʰ-***,
  making *rouge*, English *red/rust/ruby* and German *rot* **cousins by descent**,
  not borrowings — one of the oldest reconstructible colour words. *Bleu* is a
  **second** Germanic loan (*blāo*), and English then borrowed **back** from French,
  so *blue* is a Germanic word that came home in disguise; *azur* (← Arabic
  *lāzaward*) noted alongside. Payoff: of **bleu-blanc-rouge**, **two of three are
  loanwords**.
- Taxonomy: namespaced `FR-COLOUR-BLACK-WHITE`, `FR-COLOUR-RED-BLUE`.

## Chapter 12 — Numbers 11–20

- **Chapter 12 authored** (`FR-C12-nombres-11-16`, `-17-20`): the teens, atom-first,
  reviewing Ch.6/Ch.11 via `reviews_of`.
- **onze–seize** — the six numbers French inherited **already fused** from Latin
  (*ūndecim, duodecim … sēdecim*). The shared **-ze** is *decem* ("ten") worn thin —
  the **same ten** the learner already knows in **dix** and **décembre** (Ch.9).
  Each word's front is its Chapter 6 digit (*deux→dou-*, *six→sei-*).
- **dix-sept–vingt** — the **seam**: at 17 French **abandons** the fusion and goes
  transparent, *dix-sept* = plainly "ten-seven" — and the **order flips** with it
  (digit-first *seize* → ten-first *dix-sept*). Notes that Latin itself wobbled here
  (*duodēvīgintī*, "two-from-twenty"), a subtraction all the sisters dropped.
  *vingt* ← *vīgintī* → English **vigesimal**, and the seed of *quatre-vingts*.
- Taxonomy: namespaced `FR-NUM-11-16`, `FR-NUM-17-20`.

## Chapter 11 — Food (bread, water, wine)

- **Chapter 11 authored** (`FR-C11-pain`, `-eau-vin`): the everyday table trio,
  atom-first, reviewing Ch.10/Ch.1 via `reviews_of`.
- **pain** ("bread") ← *pānis* — with the payoff that a **companion** is literally
  "one you **share bread** with" (*com-* + *pānis*); also *company*, *pantry*.
- **eau / vin** — **eau** ("water") is French's **most eroded** loan from Latin:
  *aqua → eau*, worn down to a bare vowel "oh" (three silent letters for one
  sound), while English kept the loud original in *aquatic/aquarium*. **vin** ←
  *vīnum* held its shape → *wine/vine/vinegar/vintage*.
- Taxonomy: namespaced `FR-FOOD-BREAD`, `FR-FOOD-DRINKS`.

## Chapter 10 — Family

- **Chapter 10 authored** (`FR-C10-parents`, `-freres-soeurs`): the immediate
  family, atom-first, reviewing Ch.9/Ch.1 via `reviews_of`.
- **père / mère** ← *pater / māter* ← PIE *\*ph₂tḗr / \*méh₂tēr*. Taught as the
  **same inherited words** as English *father / mother*, split only by **Grimm's
  law** (*p → f*, *t → th*) — French kept Latin's *p*, English shifted it. Root
  payoff: paternal/patron, maternal/matron.
- **frère / sœur** ← *frāter / soror* → fraternal/friar, sorority; the **œ**
  ligature is introduced as the fused vowel spelling worn-down *soror*.
- Taxonomy: namespaced `FR-FAMILY-PARENTS`, `FR-FAMILY-SIBLINGS`.

## Chapter 9 — Months & seasons

- **Chapter 9 authored** (`FR-C09-mois`, `-saisons`): the calendar year, atom-first,
  reviewing Ch.6–8 via `reviews_of`.
- **The months** are a parade of Roman gods and emperors: *janvier* ← Janus (the
  two-faced god of beginnings), *février* ← the *Februa* purification, *mai* ← Maia,
  *juin* ← Juno, with **two big payoffs** — *mars* is the **same Mars** behind
  *mardi* (Tuesday), and *septembre–décembre* still mean the Latin **7–10** learned
  in the numbers chapter (the Roman year began in March; *juillet/août* ← Julius/
  Augustus were inserted and shifted the count).
- **The seasons**: *printemps* = *prime* + *temps*, "the **first time / prime
  season**"; *été* ← *aestas* "heat"; *automne* ← *autumnus*; *hiver* ← *hibernum*
  "wintry" (cousin of English **hibernate**). Plus the *au printemps* / *en été…*
  preposition split.
- Taxonomy: namespaced `FR-MONTHS`, `FR-SEASONS`.

## Chapter 8 — Time & the clock

- **Chapter 8 authored** (`FR-C08-heure`, `-midi-minuit`): telling the time,
  atom-first, reviewing Ch.6–7 via `reviews_of`.
- **heure** ← Latin *hōra* ← Greek *hṓrā* ("a time of day," the *Horae* being the
  season-goddesses) → English *hour*, the same word spelt apart (both keep the
  silent Latin *h-*). Telling time: *il est une heure / deux heures* ("it is two
  hours"), with the liaison *deu-z-eur*.
- **midi / minuit** ← *medius diēs* "mid-day" / *media nox* "mid-night" — the two
  unnumbered hours, each *mi-* ("middle," ← *medius*) + *-di* (*diēs*, the day of
  *lundi*) / *-nuit* (*noctem*, cousin of English *night*). Aside: English *noon*
  is Latin *nōna hōra* "ninth hour," drifted from mid-afternoon to midday.
- Taxonomy: namespaced `FR-TIME-HOUR`, `FR-TIME-NOON-MIDNIGHT`.

## Chapter 7 — Days of the week

- **Chapter 7 authored** (`FR-C07-jours-1`, `-jours-2`): the seven days, atom-first,
  reviewing Ch.6 via `reviews_of`, with the **planet-god week** as the through-line.
- **jours-1** (lundi–vendredi): every weekday is *[planet-god]* + **-di** (← *diēs*
  "day") — *lundi* = *lūnae diēs* "Moon's day," etc. The centrepiece is the
  **Roman-planet ↔ Germanic-god bridge**: *mardi* and English *Tuesday* are the same
  day (the war-god's), named *Mars* in Latin but *Tiw* in Germanic; *jeudi*
  (Jupiter) = *Thursday* (Thor) — *interpretatio germanica*.
- **jours-2** (samedi, dimanche): where **religion overwrote astronomy** — *samedi*
  ← *Sabbatum* (the Hebrew Sabbath, so English *Saturday*/Saturn and French *samedi*
  are the same day, two names); *dimanche* ← *diēs Dominica* "the **Lord's** day"
  (*Dominus* → dominion/dame), the *di-* fossil moved to the front.
- Taxonomy: namespaced `FR-DAYS-WEEKDAYS`, `FR-DAYS-WEEKEND`.

## Chapter 6 — Numbers 1–10

- **Chapter 6 authored** (`FR-C06-nombres-1-5`, `-nombres-6-10`): counting to ten,
  atom-first, each ~4–5 min, reviewing Ch.5 via `reviews_of`; every number carries
  its Latin source, **Spanish twin** (French grounds in its Romance sibling), and
  English cousins.
- **1–5** (*un/deux/trois/quatre/cinq* ← *ūnus/duo/trēs/quattuor/quīnque*): *un/une*
  doubles as "a/an" (like Spanish *un/una*); *cinq* → English *cinque* (the 5 on
  dice).
- **6–10** (*six/sept/huit/neuf/dix*): the dramatic erosion of *octō → oit → huit*
  (the 8 survives only in the month *octobre*); *dix* → English **dime** (via Old
  French *disme* ← *decima*); and the **septembre–décembre = Latin 7–10** calendar
  trick (the Roman year began in March; *juillet/août* pushed the counting months
  down two).
- Taxonomy: namespaced `FR-NUM-1-5`, `FR-NUM-6-10`.

## Chapter 5 — The first verbs (sentences start to move)

- **Chapter 5 authored** (`FR-C05-parler`, `-habiter`, `-travailler`,
  `-je-parle-francais`, `-practice`): French's first **grammar-engine** chapter,
  mirroring the Spanish Ch.6 verbs chapter. The learner stops reciting phrases and
  starts **building sentences from a pattern**.
- **The regular -er present tense** — the biggest French verb family: drop *-er*,
  add *-e/-es/-e/-ons/-ez/-ent*. Taught on **parler** and cemented on **habiter**
  and **travailler**.
- **The silent-ending insight + the pro-drop contrast**: *-e/-es/-ent* are all
  **silent**, so *je parle / tu parles / ils parlent* sound identical (*parl*) —
  which is exactly **why French keeps its subject pronouns** where Spanish/Italian
  drop them (the ear can't hear the person, so the pronoun must). Stated as the
  single biggest structural difference from the Iberian cousins.
- **Etymology**: *parler* ← *parabolāre* "tell parables" (→ parable/parole/
  palaver/parley); *habiter* ← *habitāre* "keep having a place" (→ habitat/
  inhabit); *travailler* ← *tripalium* "torture" (→ **travail/travel**; twin of
  Spanish *trabajar*); *français* ← *Francia* (the Franks, whose name meant
  "free" → English *frank*). First self-assembled sentence: **Je parle français**.
- Taxonomy: namespaced `FR-VERB-PARLER/HABITER/TRAVAILLER`, `FR-WORD-FRANCAIS`
  documented.

## Writing nuances — the accents, the cédille, the tréma

- **First French `writing`-type lessons** (`FR-W01-accents`, `FR-W02-cedille`,
  `FR-W03-trema`): orthography taught etymology-first, the same way as the
  Spanish writing lessons, once enough accented words have accumulated.
- **The three accents on *e*** (`é è ê`): *é* aigu = "ay", *è* grave = open "eh"
  (and the grave that only separates look-alikes, *a/à*, *ou/où*), and the star —
  the **circonflexe ê as a tombstone for a lost *s***, with the English cousin
  usually keeping it (*forêt*→forest, *hôpital*→hospital, *île*→isle,
  *bête*→beast, *être*→*stāre*). The single most useful French reading trick.
- **The cédille ç**: keeps *c* soft (*s*) before *a/o/u* (*français*, *garçon*),
  and the hook's origin as a shrunken subscript *z* (Spanish *zedilla*, "little z").
- **The tréma ï/ë**: "pronounce these vowels **separately**" (*naïve*, *Noël*,
  *maïs* vs *mais*) — explicitly contrasted with the German umlaut (which *changes*
  a vowel rather than *splitting* two).
- Uses the `writing` lesson type (no `concept_tag`) — no taxonomy change.

## Chapter 4 — Farewells (parallel of Spanish Ch. 5)

- **Chapter 4 authored** (`FR-C04-au-revoir`, `-a-plus-tard`, `-a-bientot`,
  `-a-demain`, `-practice`): closing a conversation, atom-first, reviewing
  Chapter 3. Reuses the canonical `FAREWELL` + `FAREWELL-LATER/TOMORROW/SOON`
  concepts introduced with Spanish Ch. 5, mapping each French goodbye to its
  Spanish twin.
- **The "see you again" metaphor**: *au revoir* = "until the re-seeing" (*voir* ←
  *vidēre* → vision/video/revise) — explicitly paired with German *auf
  Wiedersehen* ("on the seeing-again"), against Spanish *adiós* ("to God").
- **Cross-language root callbacks**: *à plus tard* — *tard* ← Latin *tarde*, the
  same word as Spanish *tarde*; *à demain* — *demain* ← *dē māne* "from the
  morning", sharing *māne* with Spanish *mañana* (and English *matinée*).
- **A writing-nuance aside**: the circumflex on *bientôt* (← *tost*) as the ghost
  of a dropped *s* (*hôtel* ← *hostel*), tying back to the accent-mark thread.
- All soft goodbyes are **à** + a time, mirroring Spanish's **hasta**.

## Chapter 3 — "Comment ça va ?" (the parallel of Spanish Ch. 4)

- **Chapter 3 authored** (`FR-C03-merci`, `-de-rien`, `-aller`,
  `-comment-ca-va`, `-comment-registers`, `-comme-ci-comme-ca`, `-practice`):
  the "how are you?"
  exchange, atom-first, reviewing Chapter 2 throughout. Built deliberately as the
  cross-language mirror of the Spanish Chapter 4 shipped in the same PR — same
  canonical concepts (`STATE-HOW-ARE-YOU`, `COURTESY-YOUREWELCOME`, `WORD-SOSO`),
  so the interleaving method has real parallel material.
- **Etymology contrasts made explicit** (the point of the curriculum):
  - *merci* ← *mercēs* "reward / wages" (→ mercy/merchant/commerce) — set against
    Spanish *gracias* ← *grātia* "grace" and Portuguese *obrigado* ← "obliged".
  - *de rien* ← *rem* "a thing" → "nothing" — the exact twin of Spanish *de nada*
    ← *nāta* "a born thing" (a callback the Spanish lesson already forward-references).
  - *aller* "to go" as the state-verb ("how does it *go*?") — contrasted with
    Spanish *estar* "to stand"; its suppletive paradigm traced to *ambulāre*
    (amble/ambulance), *vādere* (invade/evade), *īre* (exit/transit).
  - *comme ci, comme ça* — *comme* shares *quōmodo* with *comment*; the shrug set
    against Spanish *más o menos* and Italian *così così*.
- Taxonomy: namespaced `FR-VERB-ALLER` documented in the examples list.

## Chapter 2 — Introducing Yourself

- New chapter built around the introduction dialogue (*Je m'appelle Susanne. /
  Comment vous appelez-vous? / Je m'appelle David. / Enchanté.*), atom-first,
  one word per lesson (`lessons/FR-C02-*`, `book/chapters/ch02-introductions.tex`):
  - **je** ("I" ← *ego*; English *ego*)
  - **me** ("myself" ← Latin *mē*; English *me*, *my*, *mine*) — its own lesson,
    with the reflexive set *me / te / se* traced. (Every atom of *je m'appelle*
    is taught and rooted, not just glossed.)
  - **(s')appeler** ("to call [oneself]" ← *appellāre*; *appeal*, *appellation*)
    — introduces **reflexive verbs**.
  - **je m'appelle…** — assembled: **"my name is…"** ("I call myself"), with the
    literal *mon nom est* (← *nōmen*, English *noun*) as the stiffer alternative.
  - **tu / vous** (familiar / formal "you" ← *tū / vōs*) — politeness by using
    the plural on one person; contrasted with Spanish *usted*.
  - **comment** ("how" ← *quo modo*; same source as Spanish *cómo*).
  - **comment vous appelez-vous?** — **"what's your name?"** by inversion; the
    informal *comment tu t'appelles?*.
  - **enchanté(e)** ("pleased to meet you" ← *in-cantāre*; *enchant*,
    *incantation*, *chant*) — gender agreement with the speaker.
  - **practice** — the whole dialogue.
- Also fixed two leftover beginner-audience slips the earlier pass missed
  (`roadmap.md` "the learner's in-progress language"; `session-map.md` "the
  Spanish twin"). Book compiles clean with XeLaTeX.

## Beginner-audience pass — Spanish no longer assumed as prior knowledge

Corrected a systemic violation of HL00's Audience rule: the book and practice
lessons addressed a reader who was "also learning Spanish" and leaned on
Spanish as knowledge already owned. The books are for a true beginner whose
only shared language is English; Spanish comparisons are enrichment the text
must supply in full, not a baseline it may assume.

- Preface rewritten: drops "Because the reader is also learning Spanish…" and
  "exactly as in the Spanish book"; states the true-beginner framing and that
  every Spanish comparison is supplied by the text (a reader who knows Spanish
  "simply nods along").
- Chapter 1 (`book/chapters/ch01-greetings.tex`) and the matching practice
  lessons: recast every "Spanish twin," "the *bueno/buena* machine from
  Spanish," "One mercy over Spanish," and "you know this from Spanish" into
  self-contained "Spanish, another daughter of Latin, does X" enrichment.
  Section title "*bien* — and a Spanish twin" → "*bien* — 'well'."
- Filled the two missing noun plurals the standard wants: *les soirs*,
  *les nuits* (a new Grammar Lens on *soir*, extended on *nuit*).
- Book still compiles clean with XeLaTeX (13 pages).

## Chapter 1 — Greetings (track bootstrapped)

- New French track, built on the same HL00 framework as Spanish: one word per
  lesson, slug ids, gender-before-nouns, atom-first assembly, derivations
  shown (not just roots named), LaTeX book.
- Chapter 1 (`lessons/FR-C01-*`), atom-first:
  - **salut** (informal hi ← Latin *salus* "health") · **bien** ("well" ←
    *bene*; the Spanish twin) · **bon / bonne** ("good" ← *bonus*; agreement)
  - **le / la / les** ("the"; grammatical gender ← Latin *ille/illa/illos*,
    same as Spanish *el/la*, also the source of *il/elle*)
  - **jour** ("day" ← *diurnum* ← *dies*; the detour that gives English
    *journal*/*journey* and explains why French *jour* ≠ Spanish *día*)
  - **bonjour** (assembled; *singular*, contrasted with plural *buenos días*)
  - **soir** ("evening" ← *sērus* "late"; parallels Spanish *tarde* ←
    *tardus*) · **bonsoir**
  - **nuit** ("night" ← *noctem*; the *-ct-→-ch-* (Spanish) vs *-ct-→-it-*
    (French) sound-change table) · **bonne nuit** (feminine agreement)
  - **practice**
- Grounds each word against English **and Spanish** (the learner's in-progress
  language), foregrounding the Romance twins' differences.
- Book compiles clean with XeLaTeX (13 pages); the CI workflow auto-discovers
  `french/book/` and builds it as a PDF artifact.
