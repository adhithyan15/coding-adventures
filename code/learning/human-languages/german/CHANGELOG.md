# Changelog

## German chapter 16 is generated at exactly the ceiling, and the .tex had named a word it never taught

`ch16-family.tex` is now generated as **chapter 16, *The Pair That Proves
Grimm's Law***. German's hand-written chapters: **4 -> 3**. **No renumber** —
one chapter in, one chapter out, and the thirteen lessons fit the thirteen free
sequence slots between chapter 15 and chapter 17.

### Sizing it four ways

| instrument | answer |
|---|---|
| `handwritten_parity.py` | gap of **7** (sounds ×2, grammarlens ×1, culture ×2, etymology ×1, morphologybox ×1) |
| `grep -l '^chapter: 16$'` | **2 lessons** |
| the `.tex` | **2 sections**, two three-column tables, 2 cousinweb, 2 culture, 1 grammarlens, 1 etymology, 1 morphologybox |
| taught-form census | *Vater*, *Mutter*, *Bruder*, *Schwester*, *Geschwister* — and ***Eltern***, which the `.tex` names a section after and never teaches |

**12 atoms against a ceiling of 12, so one chapter and no split.** The rule is
the ceiling, not the median.

It fits at twelve for two separate reasons, and both are worth stating because
neither is luck.

**Three things it argues are already owned.** `GE-GRAMMAR-SOUND-LAW-01` from
chapter 6 is Grimm's law itself; `GE-SOUND-V-AS-F-01` from chapter 7 is the *V*
of *Vater*, given on *vier*; `GE-GRAMMAR-NATIVE-VS-LOAN-01` from chapter 9 is
"the months were bought and the family was not." All three are **spent**. Without
them the chapter is fifteen and splits.

**The culture claim rides as a culture claim.** `GE-CULTURE-FAMILIE-GERMANISCH-01`
is declared only in `introduces_culture_claims`, exactly as
`GE-CULTURE-DREI-SCHICHTEN-01` is in chapter 12, so it does not consume a
thirteenth atom slot.

### *die Eltern* — a word the chapter was named after and never taught

The `.tex` labels its first section `lesson:eltern` and then teaches only *Vater*
and *Mutter*. German's own README already advertised the chapter as "Eltern,
Geschwister". So the word was claimed in two places, used as a section name, and
taught nowhere — the fourth chapter in a row to turn up one of these.

It is taught now, with what is underneath it: *Eltern* is a **comparative frozen
into a noun**, "the older ones," and English made the same word into *elders*
and narrowed it. Its sibling collective *Geschwister* is built the other way, by
gathering with **Ge-** — so the chapter closes on two plural-only nouns made from
German's own parts by two different machines.

### One law, four verdicts

The `.tex` shows Grimm's law on *Vater*/*pater* and *Mutter*/*mater* and then
lists *Bruder* and *Schwester* as further cousins. The lessons make the four a
single argument, because the interesting one is the third:

| pair | what moved |
|---|---|
| *Vater* / *pater* | **p → f** |
| *Mutter* / *mater* | **t → th**, then German's own later shift again |
| *Bruder* / *frater* | the swap runs the **other way** |
| *Schwester* / *soror* | **nothing** — the law does not touch an *s* |

*Bruder* looks like a contradiction of *Vater* and is not: the two began from
different ancient sounds, a plain *p* and a breathy *b*, and each moved once in
its own direction. That is what makes it a law you can run forwards on a word
you have never seen, rather than a preference for *f*.

### One finding no gate had shown me before

`GE-C10-schwester` closed with "Next: the third language in the room," meaning
Latin. `standalone-book.test.ts` refuses **`the <ordinal> language`** anywhere in
a shipped book, because it is an ordinal over a set the reader is not holding and
cannot check. The guard is deliberately blunt and the phrase was rewritten rather
than the guard weakened.

### Counters, re-measured against the merged tree

| counter | before | after |
|---|---|---|
| German hand-written chapters | 4 | **3** |
| `handwritten_parity.py` german | 29 | **22** blocks at risk |
| German lessons (schema-v2) | 249 | **262** |
| atoms taught | 421 | **433** |
| atom-measurement-blind lessons | 8 | **6** |
| chapters over the 12-atom budget | 5 | **5** |
| culture claims | 25 | **26** |
| atoms never revisited | 81 | **81** |
| forward references | 31 | **29** |

Forward references fall by two because the schema-v1 `GE-C10-eltern` named
*Bruder* and *Schwester* before either was taught. Both are taught in this
chapter now.

### Bundle bands

Chapter 16's lessons keep `GE-C10-` ids, which land in ladder band C9. The gate
is unchanged at **591 batches over 591 bands, zero splits, 191 kB largest** —
the width-3 narrowing in the previous PR had the room, as measured.

## German chapter 13 becomes three chapters, and chapter 9 turns out to have already paid for one of them

`ch13-months-seasons.tex` is now generated, as **chapter 13, *The Roman
Calendar***, **chapter 14, *When the Calendar Stopped Naming***, and
**chapter 15, *The Seasons Go Native***. German's hand-written chapters:
**5 -> 4**, and German is still the only track that has any. Old chapters 14-40
renumber to 16-42.

### Sizing it four ways

| instrument | answer |
|---|---|
| `handwritten_parity.py` | gap of **5** (culture x2, etymology x2, morphologybox x1) |
| `grep -l '^chapter: 13$'` | **2 lessons** |
| the `.tex` | **2 sections**, a **twelve-row** table and a four-row one |
| taught-form census | 22 German forms, **20 of them headwords nowhere** |

**24 atoms against a ceiling of 12**, so three chapters at **8, 6 and 10**, cut
at the seams the `.tex` argues for itself.

### The chapter that was already paid for

Chapter 9's `GE-C06-monate-latein` already teaches `GE-GRAMMAR-NATIVE-VS-LOAN-01`,
already makes the Roman-year claim, and already prints the
*septem/octo/novem/decem* against *sieben/acht/neun/zehn* table. The `.tex`'s
whole "September–Dezember = 7–10" etymology box is therefore **argued upstream**.
Chapter 14 **spends** it rather than restating it, which is why the six numbered
and imperial months cost **six atoms and not eight** — and why splitting at the
named/numbered seam would have produced a four-atom chapter with no argument
left in it. The cut runs Jan–Jun / Jul–Dez instead, and chapter 14's thesis is
the calendar running out of ideas: two men who could order a month renamed did
so, one after the other, and then it is only arithmetic.

To make that spendable at all, `GE-CULTURE-ROMAN-YEAR-BEGAN-IN-MARCH-01` is now
declared in `introduces.knowledge` as well as `introduces_culture_claims`. It was
a culture claim only, and a culture claim cannot be `require`d. The corpus
already had the dual-declaration pattern — `GE-C17-kopf-haupt` declares
`GE-CULTURE-HEAD-CONTAINERS-05` both ways so `GE-C17-hand` can spend it. Chapter
9 goes 6 -> 7 atoms.

### `GE-LEX-JAHR-01` moves back eight chapters, deliberately

Chapter 15 glosses *Jahreszeit* as **year-time**, and glossing a compound with a
word the reader has never been taught is the defect this migration keeps finding.
So *das Jahr* is taught here, where the compound needs it, and old chapter 23 —
now 25 — **spends** it and introduces ***die Jahre*** instead, which is the form
the age sentence actually wanted and which its own gloss already said so. Its
warm-up used to read "a noun you have already met inside a longer word"; that
sentence was true only while *Jahreszeit* came first, and it is gone.

The plural is deliberately **not** previewed in chapter 15. An earlier draft said
"the plural is *die Jahre*" there, which made a forward reference to chapter 25's
new headword — caught by measurement, not by reading.

### Two forward references I wrote and then removed

Drafting produced exactly the defect this programme exists to remove:
`GE-C09-maerz` explained the Roman year by naming *September*, and `GE-C09-juni`
noted that *Juni* and *Juli* sound alike. Both words are taught in the *next*
chapter. Both sentences now make their point without the word, and the corpus
forward-reference count is unchanged at **31** rather than **33**.

### Counters, re-measured against the merged tree

| counter | before | after |
|---|---|---|
| German hand-written chapters | 5 | **4** |
| `handwritten_parity.py` german | 34 | **29** blocks at risk |
| German lessons (schema-v2) | 222 | **249** |
| atoms taught | 396 | **421** |
| atom-measurement-blind lessons | 10 | **8** |
| chapters over the 12-atom budget | 5 | **5** |
| culture claims | 25 | **25** |
| atoms never revisited | 81 | **81** |
| forward references | 31 | **31** |
| corpus info-dump findings | 118 | **118** |

The three new chapters land at **8, 6 and 10** atoms against a German band whose
median is 9 and whose floor is 3, so none of them is an outlier in either
direction.

## German chapter 12 is generated, and the chapter's own thesis is why it fits

`ch12-time.tex` is now generated. German's hand-written chapters: **6 -> 5**, and
German is still the only track that has any. **No renumber** — the chapter stays
at 12 and keeps its filename, because it did not split.

### Sizing it four ways

| instrument | answer |
|---|---|
| `handwritten_parity.py` | gap of **1** (`morphologybox` x1) |
| `grep -l '^chapter: 12$'` | **2 lessons** |
| the `.tex` | **2 sections**, two **three-column** tables |
| taught-form census | *Uhr*, *Stunde*, *Mitte*, *Mittag*, *Mitternacht* + the *es ist* frame |

**11 atoms against a ceiling of 12, so one chapter.** This is the first chapter
in the German queue that does not split, and the reason it fits is worth stating,
because it is not luck. The chapter's thesis — *a language borrows what arrives
and builds what was always there* — is `GE-GRAMMAR-NATIVE-VS-LOAN-01` for the
**third** time. It is therefore **required** from chapter 9 rather than
reintroduced, which is one atom the chapter does not have to spend. Had the
thesis been new here, the chapter would have been at 12 and one word away from a
split.

### The seam the `.tex` argues, made into two halves that answer each other

The hand-written chapter says the thing twice and never joins it up: *Uhr* is
Latin, *Mittag* and *Mitternacht* are native. The lessons make that one argument
with two sides.

* ***Uhr*** came from Latin *hōra* with the Roman and monastic clocks — the same
  word as French *heure*, Italian *ora* and English **hour**. German already had
  ***Stunde*** for an hour's span, so the loan took the job the native word could
  not do (reading a machine) and *Stunde* kept the one it could.
* ***Mittag*** and ***Mitternacht*** are *Mitte* with *Tag* and *Nacht* —
  compounds a reader can assemble before being told. French reached the identical
  thought from Latin *medius diēs* as *midi*. **Same idea, built twice, out of
  whatever each language owned.**

### Two payoffs spent rather than re-argued

| atom | from | spent on |
|---|---|---|
| `GE-GRAMMAR-NATIVE-VS-LOAN-01` | ch9 | *Uhr* against *Stunde*, and *Mittag* against *midi* |
| `GE-ETYMON-ACHT-EIGHT-02` | the numbers | *Nacht*/**night**, the *-cht*/*-ght* pair |

The second one is measurable: `GE-ETYMON-ACHT-EIGHT-02` was an atom **introduced
and never revisited** until this chapter. `Mitternacht` is the first lesson to
spend it, and the corpus counter moves by exactly that one.

### One table reshaped so the narrator can speak it

The practice lesson's recap first read `| Es ist ein Uhr. | It is one o'clock. |`
down four rows. German's person labels include ***es***, so four rows starting
with *Es* is a **partial paradigm grid** to `info-dump.ts` — a finding on a
chapter that teaches no paradigm at all. Putting the English first
(`| one o'clock | Es ist ein Uhr. |`) says the same thing and is not a paradigm.
The corpus info-dump count is unchanged at 118 rather than 119.

### Counters, re-measured against the merged tree

| counter | before | after |
|---|---|---|
| German hand-written chapters | 6 | **5** |
| `handwritten_parity.py` german | 35 | **34** blocks at risk |
| German lessons (schema-v2) | 212 | **222** |
| atoms taught | 385 | **396** |
| atom-measurement-blind lessons | 12 | **10** |
| chapters over the 12-atom budget | 5 | **5** |
| culture claims | 24 | **25** |
| atoms never revisited | 82 | **81** |
| forward references | 33 | **31** |
| corpus narration refusals | 45 | **45** |
| corpus info-dump findings | 118 | **118** |

Forward references fall by two because the v1 `GE-C08-uhr` named *Mittag* and
*Mitternacht* before either was taught. Both are now taught in this chapter, four
and six lessons after *Uhr*, so the preview is gone rather than excused.

### Modality

All ten lessons are **voice**, and the chapter keeps a **hands-free start** for
all ten. No table in the chapter exceeds three columns.

## German chapter 10 becomes two chapters, and the weekday pattern gets its exceptions

`ch10-days.tex` is now generated, as **chapter 10, *The Gods of the Week***, and
**chapter 11, *The German Weekend***. German's hand-written chapters: **7 -> 6**,
and German is the only track that has any. Old chapters 11-39 renumber to 12-40.

### Sizing it four ways

| instrument | answer |
|---|---|
| `handwritten_parity.py` | gap of **4** |
| `grep -l '^chapter: 10$'` | **2 lessons** |
| the `.tex` | **2 sections**, two **four-column** `tabularx` tables |
| taught-form census | 7 day names + *Mond*, *Donner*, *Sonne*, *Sabbat*, *Sonnabend* |

**18 atoms against a ceiling of 12**, so two chapters at the `.tex`'s own
weekday/weekend seam: **11 and 7**. A three-way split (5/6/7) was considered and
rejected — 11 fits, and splitting *below* the ceiling for its own sake is not the
rule. The rule is the ceiling, not the median.

### The chapter's own thesis is a rule with two exceptions

The `.tex` closes by saying it: German names its days for **gods**, like English,
not for planets — **with two edits**. So the pattern gets a lesson
(`GE-C07-tag-muster`), five days apply it, and the two that break it get a lesson
each:

* ***Mittwoch*** has no god and no *Tag*. The medieval Church declined to name a
  day for Wodan, and English kept him where German did not. That is a decision
  you can **date and attribute**, sitting inside an everyday word — most
  etymology in this book is drift with nobody deciding anything.
* ***Samstag*** is not Saturn's day but the **Sabbath**, Hebrew *shabbāt* by way
  of Greek *sábbaton*, which makes German *Samstag* and Spanish *sábado* the same
  word while English alone still points at a Roman planet.

### Five untaught words the chapter was leaning on

*Mond*, *Donner*, *Sonne*, *Sabbat* and *Sonnabend* were headwords nowhere.
*Mond* and *Donner* are now taught **before** the days built on them, so *Montag*
and *Donnerstag* are assembled rather than memorised.

### Six payoffs spent rather than re-argued

| atom | owned by | spent on |
|---|---|---|
| `GE-LEX-TAG-02` | ch1 | the *-tag* in every day name |
| `GE-SOUND-NACHT-CH-01` | ch1 | *Mittwoch*'s raspy *ch* |
| `GE-LEX-ABEND-02` | ch1 | makes *Sonnabend* transparent |
| `GE-SOUND-EI-AS-EYE-01` | ch6 | *Freitag*'s *ei* |
| `GE-GRAMMAR-SOUND-LAW-01` | ch6 | *Donner* from *thunder*, *th* -> *d* |
| `GE-GRAMMAR-NATIVE-VS-LOAN-01` | ch9 | *Samstag*, for the second time |

Three of those six did not exist before chapter 6 was migrated. Working bottom-up
is what makes them available rather than accidental.

### Counters, re-measured against the merged tree

| Measure | Before | After |
|---|---|---|
| German hand-written chapters | 7 | **6** |
| corpus hand-written chapters | 7 | **6** |
| `handwritten_parity.py` german | 39 | **35** blocks at risk |
| German lessons (schema-v2) | 197 | **212** |
| atoms taught | 367 | **385** |
| atom-measurement-blind lessons | 14 | **12** |
| chapters over the 12-atom budget | 5 | **5** |
| culture claims | 19 | **24** |
| atoms never revisited | 83 | **82** |
| forward references | 33 | **33** |
| corpus narration refusals | 47 | **45** |
| corpus paradigm tables | 95 | **91** |
| corpus full paradigm grids | 21 | **20** |
| corpus lessons with info-dump findings | 121 | **118** |
| book pages | 367 | **383** |

Every corpus-wide info-dump number **fell**: the two four-column day tables are
gone, and the two recaps that replace them are two-column German/English lists
the narrator can speak. All fifteen lessons are `voice`, and both chapters open
hands-free.

### Five defects caught in the draft, before CI

Four banned words (*not just the slot*, *not just to which day*, *you have just
seen*, *the noun you just met*) and one sight cue (*But look at the pair*). The
banned words mattered more than usual: `banned-words.test.ts` had been ratcheted
to 1076 hours earlier, and shipping these would have pushed it straight back over
a ceiling that had only just come down.

### Verification

125/125 test files, 1801 tests; all eleven `check:*` gates; language-ladder 39/39
files, 442 tests; the German book compiles under XeLaTeX with zero errors, zero
overfull or underfull boxes and zero missing characters (383 pages), and both new
chapters were read on the page.

## German chapter 6 leaves the hand-written set, as four chapters

Chapter 6 — the numbers one to ten — is now generated from its lessons, as
**chapters 6, 7, 8 and 9**. German's hand-written chapters: **8 -> 7**. Old
chapters 7-36 renumber to 10-39.

### Two lessons were holding ten numbers

| Method | Said |
|---|---|
| `handwritten_parity.py german` | a gap of **1 block** (5 in the `.tex`, 6 in the lessons) |
| `grep -l '^chapter: 6$' lessons/*.md` | **2 lessons**, and the `.tex` renders **2 sections** |
| the German the `.tex` teaches vs. the lessons that own it | **ten numbers owned by two lessons, five each**, plus *ein*/*eine*, the sound law, the *-cht-*/*-ght-* correspondence and the Latin-months observation owned by nobody — **fourteen items, two owners** |
| **reading the tables** | two **four-column** tables, five rows each, against a `maxLinearisableTableColumns` of **3** |

The first two methods were the misleading ones again, and for a new reason:
this chapter's `.tex` is honest about its own size. Five blocks, two sections,
two lessons — everything agrees, and everything is wrong, because *one lesson
teaching five numbers* is five items in one sitting no matter how tidy the
prose around it looks. The rule is one new item per lesson, and length is never
a cost.

**2 lessons became 16**: one per number, the sound law that makes the
differences predictable rather than random, the article *ein*/*eine*, a run for
each five, the seam between homegrown numbers and imported month names, and a
closing exchange. All sixteen are `voice` and drivable.

### The four-column tables are gone, and the narrator notices

Both of the old tables were `German | English twin | shared ancestor | Latin
cousin` — four columns, which `chapter-policy.json` calls unspeakable and the
narration lineariser refuses outright. `narration.test.ts` counts corpus-wide
refusals and it moves **51 -> 49**, the same shape and the same cause as the
French chapter 6 entry sitting beside it in that pin.

What replaced them is not a narrower version of the same grid. Each number now
carries its own English twin in its own lesson, and the two recap tables are
three-column runs (*German | Said | English twin*) that a narrator can read
aloud.

### The reconstructions went; the sound law stayed

The `shared ancestor` column held ten Proto-Indo-European reconstructions.
Every one is gone, and none of the teaching went with them. A beginner counting
to ten cannot use a reconstruction; what they can use is the *rule* the
reconstructions were evidence for, which now has a lesson of its own — German
and English are siblings, Latin is a cousin, and a sound law is a change that
caught every word at once, which is why *drei*/*three*/*tres* differ
predictably.

That lesson also connects to something the reader already owns: *gut* from
*good* and *machen* from *make* are the **later**, German-only shift, and
*zehn* is the one number where both shifts are visible in sequence — an old
**d** to **t**, which is where English stopped, and then **t** to **ts**, which
is where German went on.

### Two examples were swapped for words the reader has

*ein Kaffee* and *eine Katze* illustrated the article with two nouns the book
does not teach until chapters 28 and 22. They are now *ein Tag* and *eine
Nacht*, from *guten Tag* and *gute Nacht*, which the reader has owned since the
greetings. Same point, no forward reference, and the *der/die* split it depends
on is already theirs.

### What moved

- `handwritten.d/german-0006.json` is now a `targets.d/` entry.
- `handwritten_parity.py german`: **40 -> 39** blocks at risk.
- **No path shard was renumbered.** All sixteen lessons sit on the existing
  `GE-PATH-015`, so `curriculum.d/path/` is untouched apart from that node's
  own lesson list.
- One sequence moved outside the chapter: `GE-C07-wochentage-1` from 225 to
  227, because sixteen lessons need 211-226 and chapter 7 was sitting at 225.
  Its lesson id did not move.
- Both old lesson ids were **kept**, as the two recap lessons.
  `GE-C06-zahlen-1-5` and `GE-C06-zahlen-6-10` are referenced as prerequisites
  or reviews by six lessons in chapters 7, 8, 9, 12 and 31; retiring the ids
  would have broken every one of them, and a run of five numbers is a real
  thing to practise rather than a placeholder.

### Counters, re-measured against the merged tree rather than composed

| Measure | Before | After |
|---|---|---|
| German hand-written chapters | 8 | **7** |
| atoms taught | 335 | **367** |
| lessons with measured budgets | 181 | **197** |
| atom-measurement-blind lessons | 16 | **14** |
| chapters over the 12-atom budget | 6 | **5** |
| culture claims | 18 | **19** |
| forward references | 36 | **33** |
| atoms never revisited | 78 | **83** |
| reinforcement window misses | 687 | **800** |
| `handwritten_parity.py` german | 40 | **39** blocks at risk |
| corpus narration refusals | 50 | **48** |
| book pages | 346 | **367** |

RE-MEASURED ON RETRIEVAL. This chapter was authored before German chapters 14,
15, 16, 17 and 19 landed, so every "before" above was recomputed against the
merged tree rather than carried over from the original commit. `paradigmTables`
(95), `lessonsWithFindings` (121), `ruleStatements` (30) and `fullParadigmGrids`
(21) are all unchanged.

### Thirty-two atoms was a cram, and the band that justified it was the wrong band

This chapter first shipped as ONE chapter of **32 atoms**, argued from the five
chapters beside it: German's opening runs 31, 23, 36, 30, 30. That argument was
measured against a **sub-population**, not the track.

German's actual band, across all 29 atom-bearing chapters at the time:

| | German | French |
|---|---|---|
| median | **9** | 9 |
| mean | 12.7 | 8.5 |
| max | 36 | 12 |
| over the ceiling | 6 of 29 | 0 of 37 |

German's median is the same as French's. The six over-budget chapters are the
opening band, and **the opening band is debt, not a design** — 23 of 29 chapters
already sat at or under the ceiling. Against that, 32 is nearly three times
`maxNewAtomsPerChapter`, and the rule is not ambiguous: if a chapter cannot fit
its material, split it; never cram, never raise a ceiling.

So it is four chapters, cut at the `.tex`'s own 1–5 / 6–10 seam and then again
inside each half, because halves of 18 and 14 were both still over:

| chapter | atoms | lessons |
|---|---|---|
| 6 One, Two, Three | **9** | 4 |
| 7 Four, Five, and the Article Hiding in *Eins* | **9** | 4 |
| 8 Six, Seven, Eight | **8** | 3 |
| 9 Nine, Ten, and the Months That Are Numbers | **6** | 5 |

`atomChapterSpikes` moves **6 → 5** rather than 5 → 6: chapter 6 leaves the
over-budget list instead of joining it, and the only German chapters still above
the ceiling are the five nobody has split yet.

German stays pinned at **zero** cross-chapter prose references. Three drafts
said "since chapter one"; all three now name the thing instead — "since your
first *danke*", "the two nouns *guten Tag* and *gute Nacht* are built from".

### Deliberately not carried over

The ten PIE reconstructions, and Latin *unus* and *duo*, which added a fourth
column without adding a claim the reader can use. Everything else the
hand-written chapter taught is present: all ten numbers with their English
twins, both sound-law showpieces, the *-cht-*/*-ght-* fossil, the article, and
the Roman calendar's off-by-two months. The German month spellings themselves
stay in chapter 9, where they are taught, and the lesson says so rather than
previewing them.


## German chapter 14 leaves the hand-written set, and its age half moves to where the copula is

`ch14-haben-alter.tex` is now generated — as **chapter 14, *To Have***, plus a
new **chapter 19, *Being Your Years***, carrying the half of it that could not
stay. German's hand-written chapters: **9 -> 8**. Old chapters 19-35 renumber to
20-36.

### Sizing it four ways

| instrument | answer |
|---|---|
| `handwritten_parity.py` | gap of **6** |
| `grep -l '^chapter: 14$' lessons/*.md` | **2 lessons** |
| the `.tex` | **2 sections** |
| taught-form census | *haben* x6 forms, *habēre*, *alt*, *Jahr/Jahre*, *einen* |

Whole, the chapter is **~17 atoms against a ceiling of 12**. Split at the seam
the ramp already dictated, it is **8 and 6** — and chapter 14 then needs no
split of its own.

### The age sentence moved rather than carrying a documented forward reference

*Ich bin zwanzig Jahre alt* needs *ich bin*; *Wie alt bist du?* needs *du bist*.
Both are chapter 18. Teaching the sentence four chapters before the copula is
the ramp violation this programme exists to remove, and accepting a documented
forward reference would have bought nothing, so the material moved instead.

It could not join chapter 18 — that chapter introduces 9 atoms and the age block
is 6, which is 15 against a ceiling of 12 — so it is a chapter of its own,
placed **immediately after** it. That interrupts the *sein* arc on purpose: a
reader who has just met six forms of a verb should use them for something real
before meeting its past. Parking it at the end of the book would have renumbered
nothing and divorced the sentence from both verbs it argues about; that was the
cheap option and it is not the one taken.

### Four regular cells share a lesson, and the two irregular ones do not

Chapter 5 is now schema-v2, so the weak ending machine is an **atom this chapter
requires** rather than a pattern it assumes. On *hab-* that machine is right four
times out of six. Those four share one lesson which makes the reader *build*
them — ending first, then the form, then the same machine run on *wohn-* to show
it is not about this verb. *Du hast* and *er hat* get a lesson each, because
nothing generates them.

This is a different call from chapter 15's *Perfekt*, and deliberately so: there
no stated rule existed and the fourth cell would have been a guess, which is what
`maxNewGrammarCellsPerLesson` forbids. Here the learner applies a rule they own.

### The running example nobody had taught

*Ich habe einen Bruder* is the `.tex`'s own first example, and ***einen* is a
headword nowhere in German**. It is taught here as one atom — the form for a
masculine thing you have — flagged as the first sighting of a system that gets
its own chapter later, not as a case lesson.

Teaching it surfaced a **real defect in chapter 1**: `GE-C01-guten-tag` cited
*ich wünsche einen guten Tag* to explain the frozen *-en*, which was an 89-lesson
preview that no gate could see, because the gate cannot flag a reference to a
word nothing teaches. `continuity.test.ts` pins German chapter 1 at **zero**
previews, and this change made the existing one visible. Chapter 1 now makes the
same point in English and keeps every teaching claim.

### Counters, re-measured against the merged tree

| Measure | Before | After |
|---|---|---|
| German hand-written chapters | 9 | **8** |
| corpus hand-written chapters | 18 | **17** |
| `handwritten_parity.py` german | 46 | **40** blocks at risk |
| German lessons (schema-v2) | 166 | **181** |
| atoms taught | 321 | **335** |
| atom-measurement-blind lessons | 18 | **16** |
| chapters over the 12-atom budget | 5 | **5** |
| culture claims | 16 | **18** |
| atoms never revisited | 81 | **78** |
| forward references | 36 | **35** |
| cross-chapter prose references | 0 | **0** |
| book pages | 327 | **346** |

`paradigmTables` (95), `lessonsWithFindings` (121), `ruleStatements` (30) and
`fullParadigmGrids` (**21**) are all unchanged: `GE-C14-haben`'s grid finding
left and `GE-C14-practice`'s recap replaced it, and the recap uses the
`singular | plural` shape so it stays under `FULL_GRID_ROWS` — German does not
get a full paradigm back and chapter 5's inverted `GE-C05-wohnen` fixture stays
valid.

Forward references moved **both ways** and netted -1: two retired (*ich bin* and
*sein*, now behind the age chapter rather than ahead of it), two newly exposed by
teaching words nothing had taught (*Jahr* in `GE-C03-gehen`, *einen* in
`GE-C01-guten-tag`), and one of those two then fixed in chapter 1. The *Jahr*
one was kept and paid off: `GE-C03-gehen` names *Jahr* as an example of the
silent lengthening *h*, and `GE-C14-jahr` now cashes that in instead of leaving
it a promise.

### Verification

124/124 test files, 1746 tests; all eleven `check:*` gates; language-ladder
39/39 files, 442 tests; the German book compiles under XeLaTeX with zero errors,
zero overfull or underfull boxes and zero missing characters (346 pages), and
chapters 14 and 19 were read on the page. All 29 teaching claims in
`ch14-haben-alter.tex` were checked across into the new lessons.

## German chapter 15 becomes three chapters, and leaves the hand-written set

`ch15-perfekt.tex` — the top of the German hand-written range — is now
generated, as **chapters 15, 16 and 17**. German's hand-written chapters:
**10 -> 9**. Old chapters 16-33 renumbered to 18-35.

### Sizing it four ways, and none of them was the size

- `handwritten_parity.py` scored it at a gap of **5 blocks**.
- `grep -l '^chapter: 15$' lessons/*.md` said **3 lessons**, where the `.tex`
  rendered **2 sections** — the third, `GE-C15-praeteritum-map`, was on no
  curriculum path at all and reached the book only through the chapter ledger.
- Counting the German forms it teaches gave eleven: *gesagt*, *gemacht*,
  *gelernt*, *gewohnt*, *gehabt*, *gestern*, *sagte*, *machte*, *hatte*,
  *konnte*, and the two tense names.

The real cost is the **person slots of a compound tense**. The `.tex` printed
the *Perfekt* with three rows because three rows are enough for a reader to
infer the fourth, and that inference is exactly what
`maxNewGrammarCellsPerLesson: 1` forbids. Four cells, four lessons — the fourth
because the chapter's own closing line, *Er machte das … Er hat das gemacht*,
uses a person the table never showed.

Add the participle recipe and its four verbs, the clause-final bracket, the
*-te* and its four verbs, register, the survivors and the areal map, and the
material is **twenty-four atoms** against a `maxNewAtomsPerChapter` of twelve.
Length is never a cost here, so it became **three chapters of 7, 8 and 9 atoms**
rather than one chapter at twice the ceiling. Three lessons became
**twenty-seven**.

### The running example was never taught

Every participle in the chapter is built on *sagen*, and *sagen* was not a
headword anywhere in the track. Hand-written prose can use a word it never
introduced; a generated chapter cannot. Chapter 15 now opens with *sagen*
(`GE-C15-sagen`) — one lesson that no count could see, because it is not new
material but material the chapter assumed.

### Every paradigm-shaped table is a recap

The infinitive-to-participle table now sits in `GE-C15-partizip-practice`, after
six lessons that built it one verb at a time. The four *Perfekt* slots sit in
`GE-C15-perfekt-practice`, after four lessons that met them one at a time. The
*Präteritum*-against-*Perfekt* comparison sits in
`GE-C15-praeteritum-practice`, and its first column is a meaning rather than a
person, so it is a comparison and not a paradigm at all. Corpus `paradigmTables`
is unchanged at 95 and `lessonsWithFindings` unchanged at 121: one finding left
`GE-C15-perfekt` and one arrived in `GE-C15-perfekt-practice`. `fullParadigmGrids`
is 21, which is where German chapter 5 left it; nothing here moved it.

### Lesson ids did not move

All twenty-seven lessons keep the `GE-C15-*` prefix and live across chapters 15,
16 and 17 — the Spanish convention, where `ES-C03-*` lessons sit in chapters 4,
5 and 6.

### Counters, re-measured against the tree rather than derived

| Measure | Before | After |
|---|---|---|
| German hand-written chapters | 10 | **9** |
| corpus hand-written chapters | 20 | **19** |
| `handwritten_parity.py` german | 51 | **46** blocks at risk |
| German lessons (schema-v2) | 139 | **166** |
| atoms taught | 297 | **321** |
| atom-measurement-blind lessons | 21 | **18** |
| chapters over the 12-atom budget | 5 | **5** |
| culture claims | 14 | **16** |
| atoms never revisited | 81 | **81** |
| forward references | 36 | **36** |
| cross-chapter prose references | 0 | **0** |
| book pages | not re-measured on `main` | **327** |

Measured against the merged tree, not composed: German chapter 5 landed on
`main` while this branch was open, so every "before" above is `main` after that
commit rather than the number this branch started from.

Forward references held at 36 because both candidates were phrased around rather
than cut: the closing "Next:" line names the job instead of *sein*, and the
survivor list says "the past of the verb for 'to be'" instead of *war*, which
chapter 19 teaches inside the comma-separated headword `bin, ist, war`.

### One defect the gates caught

`GE-C15-du-hast-gesagt` wrote "the part of this tense that **never has** to be
relearned" — a sentence about the learner, not about German. `info-dump.ts`
matched it as a rule statement and pushed the pinned ceiling from 30 to 31. The
sentence was rewritten; the pin was not moved.

### Verification

124/124 test files; all eleven `check:*` gates; language-ladder 39/39 files,
442 tests; the German book compiles under XeLaTeX with zero errors, zero
overfull or underfull boxes and zero missing characters (327 pages), and
chapters 15, 16 and 17 were read on the page. All 52 teaching claims in
`ch15-perfekt.tex` were checked across into the new lessons.

## German chapter 5 leaves the hand-written set

Chapter 5 — the first verbs — is now generated from its lessons. German's
hand-written chapters: **11 -> 10**.

### The hole was a paradigm, not a word list

Three of the four sizing methods understated this chapter, and the fourth is the
one that found the work:

- `handwritten_parity.py german` scored chapter 5 at a gap of **4 blocks**
  (12 in the `.tex`, 9 in the lessons).
- `grep -l '^chapter: 5$' lessons/*.md` gave **5 lessons**, and the `.tex`
  rendered **5 sections** — no writing lessons hidden off the page this time,
  unlike chapter 4.
- Counting the German the `.tex` teaches against the lessons that own it found
  **eight items owned by nobody**: *er*, *sie* (she), *wir*, *ihr*, *in*, *wo*,
  *was* and *Deutsch*. Three of them — *wo*, *was* and *in* — were used in the
  chapter's own closing dialogue and taught nowhere at all.
- Reading the chapter's **tables** is what sized it. Three of its four tables
  are six-row person paradigms, and `maxNewGrammarCellsPerLesson` is **1**. A
  row-per-person table is a paradigm, and no grep reports that.

`GE-C05-wohnen` alone introduced the verb, a six-row present-tense grid, and
five pronouns nobody had taught — including the *-st* that *du heißt* had been
hiding behind an eszett. Against `maxNewAtomsPerLesson: 3` and
`maxNewGrammarCellsPerLesson: 1` that is one lesson doing about eight lessons'
work.

Chapter 5 is now **15 lessons**: the verb, the stem rule, one lesson per new
pronoun with the ending it takes, the audibility contrast, the three question
and place words, the three verbs, the first self-assembled sentence, and the
overt-subject rule. All fifteen are `voice` and drivable, so the chapter keeps
its hands-free start.

### Why it is one chapter and not two

Chapter 5 introduces **30 atoms**, over `maxNewAtomsPerChapter`. It was not
split, and that is a deliberate call rather than an oversight: German chapters
1-4 are already generated and merged at **31, 23, 36 and 30** atoms, because
this track's opening word lessons each carry a sound atom and an etymon atom
alongside the word. Chapter 5 at 30 sits exactly with its four neighbours, so a
split here would buy consistency with the ceiling at the cost of consistency
with the four chapters either side of it — and would renumber every later German
chapter while chapters 10-15 are being retired in parallel. `atomChapterSpikes`
moves 4 -> 5 and records the debt where the whole opening range already sits.
Chapter 16 was split, and was split because a `sein` paradigm genuinely does not
fit; this is the other case.

### What moved

- `handwritten.d/german-0005.json` is now a `targets.d/` entry.
- `handwritten_parity.py german`: **55 -> 51** blocks at risk.
- `SPINE-SAY-WHAT-I-DO` needed a **segment split**: *machen* carries the
  canonical concept `VERB-DO-MAKE` and therefore already sat on
  `SPINE-NAME-EVERYDAY-ACTIONS` in its own path node, while six chapter-5
  lessons come after it. The path is now `GE-PATH-014` (through *wo*) ->
  `GE-PATH-A1-MACHEN` -> `GE-PATH-014-B` (from *was*), with the spine's segment
  ledger updated to match. Inserting that path shard renumbered the 31 path
  files after it, which `check:shards` verifies.
- Lesson ids did not move: all fifteen keep the `GE-C05-*` prefix. Sequences
  shifted inside the 188-215 gap left by chapters 4 and 6.

### Counters, re-measured against the merged tree rather than composed

| Measure | Before | After |
|---|---|---|
| German hand-written chapters | 11 | **10** |
| atoms taught | 267 | 297 |
| lessons with measured budgets | 124 | 139 |
| atom-measurement-blind lessons | 26 | 21 |
| chapters over the 12-atom budget | 4 | 5 |
| forward references | 40 | **36** |
| atoms never revisited | 73 | 81 |
| reinforcement window misses | 563 | 645 |
| full paradigm grids (corpus) | 22 | **21** |
| `handwritten_parity.py german` | 55 | 51 |

Forward references fell by four: teaching *er*, *wir*, *ihr*, *wo*, *was* and
*in* retired previews earlier chapters had been making. The direction was read
off the tree, not assumed — this number has moved both ways across the four
German chapters retired so far.

German stays pinned at **zero** cross-chapter prose references.

### The full-grid count went down, and that is the gate working

`info-dump.test.ts` pinned `fullParadigmGrids` at 22 and named
**`GE-C05-wohnen`** as one of two canonical fixtures — a lesson presenting a
complete conjugation in a single table. Splitting that grid across the lessons
that teach it removed the last German example: no German lesson now presents a
full paradigm in one table, and the corpus count is 21.

That six-row table was the exact shape the info-dump gate exists to flag, so the
count falling is the gate working rather than the detector breaking. The named
fixture that protects the detector is `FR-C05-parler` alone now, and the German
assertion was **inverted to `toBe(false)`** with the reason written beside it
rather than deleted — so if anything ever reintroduces a German full grid, the
test still fails loudly. French chapter 5 has the same grid, so whoever retires
it faces this decision too.

### Deliberately not carried over

*Ich mache einen Kuchen* would have introduced **two** untaught words (*einen*,
*Kuchen*) to illustrate a verb that already has an example. The *lernen*
etymology's detour through Gothic *lais* and a shoemaker's *last* is weight
without payoff next to *learn*/*lore*, which carries the same point. Also
dropped: *won't* as a cousin of *wohnen*, archaic *won*/*wone*, French
*thiois*, and the reconstructed `*þeudō` behind *diutisc*. Everything else the
hand-written chapter taught survives — 50 of the 56 claims read across, and the
six above are the six.

## German chapter 16 becomes three chapters, and leaves the hand-written set

`ch16-sein.tex` — the top of the German hand-written range — is now generated,
as **chapters 16, 17 and 18**. German's hand-written chapters: **12 -> 11**.
Old chapters 17-31 renumbered to 19-33.

### Sizing it four ways, and only the fourth was right

- `handwritten_parity.py` scored it at a gap of **8 blocks**.
- `grep -l '^chapter: 16$' lessons/*.md` said **3 lessons**, and the `.tex`
  rendered **2 sections**. No hidden writing lessons this time.
- Counting the German words it teaches gave six new ones: *sein*, *müde*,
  *kommen*, *fahren*, *werden*, *bleiben*.

None of those is the real size, because chapter 16's cost is **grammar, not
vocabulary**. It carries two paradigms — the present of *sein* and its past —
and a cell is one filled slot, not a table. Six present cells plus four past
cells plus six lexical, sound and rule atoms is **twenty-four atoms**, against a
`maxNewAtomsPerChapter` of twelve.

Length is never a cost here, so it became **three chapters of 9, 7 and 8 atoms**
rather than one chapter at twice the ceiling. Three lessons became **twenty-six**.

### The paradigm grid appears only as a recap

HL10's rule is that no paradigm table may be printed until every cell in it has
been taught individually, at which point the table is a recap rather than an
introduction. So `sein` is met one form per lesson — *bin*, *bist*, *ist*,
*sind*, *seid*, *sind* — and the grid the hand-written chapter opened with now
sits in `GE-C16-praesens-practice`, at the end, where it is the ninth thing the
reader sees rather than the first.

The past does the same across chapter 17, and by chapter 18 the
*ich bin gegangen* table is a recap twice over: every cell of *sein*'s present is
owned, and the participle does not inflect.

### Lesson ids did not move

All twenty-six lessons keep the `GE-C16-*` prefix and live across chapters 16,
17 and 18 — the Spanish convention, where `ES-C03-*` lessons sit in chapters 4,
5 and 6. An id names the chapter a lesson was written for; chapter numbers move
and ids do not.

### German joins the tracks whose chapters move

Fifteen chapters renumbered at once, and **36 of German's 65 cross-chapter prose
references pointed into that range** — every one of them would have rotted on
this single commit. `chapter-references.test.ts` says what to do: name the thing,
never the number. All **65 -> 0**, and German is now pinned at zero beside
Spanish and French. The pass used "the food lesson", "the doing verbs", "the
*Hand* table", "your first verbs", "the eszett rule", and for the purely
decorative ones the thing itself: "**hören** showed", "**Hund** showed".

### Counters, re-measured against the tree rather than derived

| Measure | Before | After |
|---|---|---|
| German hand-written chapters | 12 | **11** |
| `handwritten_parity.py german` | 63 | **55** blocks at risk |
| lessons (schema-v2) | 98 | 124 |
| atoms taught | 243 | 267 |
| atom-measurement-blind lessons | 29 | 26 |
| chapters over the 12-atom budget | 4 | 4 |
| culture claims | 12 | 14 |
| atoms never revisited | 75 | 73 |
| forward references | 39 | **40** |
| cross-chapter prose references | 65 | **0** |

Forward references went **up** by one, and the direction is the point. Teaching
*ich bin* made `GE-C14-alter`'s existing *ich bin zwanzig Jahre alt* visible as a
preview for the first time — the gate cannot see a reference to a word no lesson
teaches. Two rose that way, one fell (`nein` was cut from a dialogue that did not
need it), and one is deliberate: *die eingeschlafene Katze* is the hand-written
chapter's own example of a participle taking an adjective ending, and it is kept
with a note rather than swapped for a duller noun.

### Two defects the gates caught

1. **A capital eszett's cousin.** `GE-C16-muede` wrote the stress as *MÜ-duh*.
   Capital **Ü** is in neither `core/main-font-charset.json` nor `book.ts`'s
   escape map — free while the chapter was hand-written, a hard `glyph-coverage`
   failure once generated. Lowercase *mü-duh* with the stress stated in words.
2. **Two dialogues read as paradigms.** `info-dump.ts` calls a table with three
   or more person-labelled first cells a paradigm table, and a German exchange
   whose lines open *Ich…*, *Du…*, *Wir…* looks exactly like one. Rewritten so
   the speakers open with something other than a pronoun; corpus `paradigmTables`
   back to 95 and `lessonsWithFindings` to 121, both unchanged from before.

Also fixed on the way past: `GE-C16-ich-war` said "look at the shape of it",
which is a `sight-cue`, and that one phrase cost chapter 17 its hands-free start.
It says "hear" now, and all three chapters open drivable.

### Verification

124/124 test files, 1739 tests; every `check:*` gate; language-ladder 39/39
files, 442 tests; the German book compiles under XeLaTeX with zero errors and
zero overfull or underfull boxes (279 pages), and chapters 16, 17 and 18 were
read on the page. All 70 teaching claims in `ch16-sein.tex` were checked across
into the new lessons.

## German chapter 4 leaves the hand-written set

Chapter 4 — the farewells chapter — is now generated from its lessons. German's
hand-written chapters: **13 -> 12**.

### Sizing by the .tex found most of the hole, but not all of it

Counting what `ch04-farewells.tex` actually teaches against the lessons that own
it predicted the work well: *auf Wiedersehen* is welded from **auf**, **wieder**
and **sehen**, and none of the three had a lesson; *bis*, *bald* and *spät* were
all taught inside phrase lessons rather than owned by one. Seven new lessons.

What the `.tex` could **not** show is that chapter 4 also owns three **writing**
lessons — `GE-W01-eszett`, `GE-W02-umlauts`, `GE-W03-capitalization` — which are
staged separately and appear nowhere in that file. They surfaced only when
`book.ts` refused to generate:

```
Error: GE-W01-eszett: generated books require schema version 2
```

So the sizing rule needs one more clause: **count the lessons the chapter owns,
not only the ones its `.tex` renders.** `grep -l '^chapter: N$' lessons/*.md` is
the honest denominator. Chapter 4 went from 9 lessons to 16.

### What moved

- `handwritten.d/german-0004.json` is now a `targets.d/` entry.
- Sixteen lessons: seven new (*sehen*, *wieder*, *auf*, *bis*, *bald*, *spät*,
  *morgen* as "tomorrow"), six migrated, and the three writing lessons migrated
  from schema v1.
- `handwritten_parity.py german`: **69 -> 63** blocks at risk.
- Chapter 4's lessons now sit in `GE-PATH-013` with three extension nodes, one
  of them a writing kit.

### Counters, re-measured rather than derived

| Measure | Before | After |
|---|---|---|
| atoms taught | 213 | 243 |
| lessons with measured budgets | 82 | 98 |
| atom-measurement-blind lessons | 38 | 29 |
| chapters over the 12-atom budget | 3 | 4 |
| culture claims | 8 | 12 |
| senses | 4 | 5 |
| forward references | 42 | **39** |

Forward references went *down*: teaching *sehen*, *wieder* and *auf* removed
three previews that earlier chapters had been making.

### Four defects the gates caught, and one they did not

1. **A payoff that quietly fell below the floor.** Claiming 14 atoms looked
   generous against the thirteen farewell lessons — but the chapter introduces
   **30** atoms once the writing lessons are counted, so 14/30 = 0.47 sat under
   the 0.5 representativeness floor and `payoffSurprises` went 1 -> 2. The fix
   was to claim the writing kit's three transferable rules as well, and to widen
   the chapter's `canDo` to admit that chapter 4 teaches writing too. Now 17/30.
2. **A writing stage four steps too early.** `GE-W03` was authored as
   `controlled-composition`; German has only `guided-copy` evidence at this
   point, so the cumulative HL19 ledger rejected it. Its task is a copy with the
   model in view — `guided-copy` was the honest label all along.
3. **A 327-second lesson.** `GE-W03` exceeded the computed five-minute ceiling.
   Prose came out; the declared number was not touched.
4. **Two glyphs that had never had to render.** The v1 eszett lesson used the
   long s and the capital eszett. Neither is in `main-font-charset.json` nor in
   `book.ts`'s escape map — harmless while the chapter was hand-written LaTeX,
   fatal the moment it is generated. Both are now described in words. Adding
   `\newunicodechar` mappings to all 24 book preambles would be the alternative,
   and is not worth it for two decorative glyphs.

### One JSON-writing trap worth recording

`json.dumps` defaults to `ensure_ascii=True`, which wrote `—` into
`chapters.d/0004.json`. The canonical re-shard writes literal UTF-8, so
`chapters-shards.test.ts` failed on bytes that *looked* identical in every
diff view. Always pass `ensure_ascii=False` when writing these files by hand.


## German chapter 3 leaves the hand-written set

Chapter 3 — the *Wie geht es dir?* chapter — was hand-written LaTeX. It is now
generated from its lessons, and generating it turned out to require real
authoring rather than a format flip.

### The chapter had a hole, and the flip exposed it

Eight lessons carried a chapter whose LaTeX taught roughly twenty things. The
gap was not padding: *es*, *mir*, *dir*, *Ihnen*, *sehr*, *nicht*, *und*, *so
lala*, *schön* and *vielen Dank* were all taught in the book and owned by no
lesson. `GE-C03-gehen` alone introduced *gehen*, *es*, *mir* and the dative in
one sitting — four new things in a lesson budgeted for three.

So the chapter was authored out to **18 lessons**, one new word each:

- **Ten new lessons** — `schoen`, `vielen-dank`, `es`, `mir`, `dir`, `ihnen`,
  `sehr`, `nicht`, `so-lala`, `und`.
- **Eight migrated** from schema v1 to v2, with the material that now has its
  own lesson removed from them. `GE-C03-gehen` keeps only *gehen*; the "dressing
  it up" list left `danke` for the three lessons that own its parts.

Every lesson is within `maxNewAtomsPerLesson: 3`, and the chapter introduces
**36 atoms**.

### What moved

- `core/book-generation.d/handwritten.d/german-0003.json` is now a `targets.d/`
  entry. German's hand-written chapters: **14 -> 13**; corpus-wide **38 -> 37**.
- `handwritten_parity.py german`: **77 -> 69** blocks at risk. Chapter 3's own
  gap went 8 -> 0 by being generated; before the flip it had already fallen 8 ->
  3 as the prose was carried into lessons. The residue was two `morphologybox`
  environments and one `grammarlens`, none of which the generator can emit —
  their content was re-homed into `cousinweb` tables (the *ich/mich/mir* and
  *doer/to-whom* grids) and a `grammarlens` on returning the question.
- The payoff is atom-scored instead of authored boilerplate: 23 of the 36 atoms
  — the lexical spine, the dative, and the two pragmatic rules. Per-word sound
  and etymology atoms are deliberately not claimed.
- Chapter 3's lessons are now in the curriculum path (`GE-PATH-010`,
  `GE-PATH-012`) and three new extension nodes carry the language-specific and
  consolidation lessons.

### Counters, re-measured rather than derived

All of these were regenerated against the merged tree, not adjusted by
arithmetic:

| Measure | Before | After |
|---|---|---|
| atoms taught | 177 | 213 |
| lessons with measured budgets | 64 | 82 |
| atom-measurement-blind lessons | 46 | 38 |
| chapters over the 12-atom budget | 2 | 3 |
| culture claims | 6 | 8 |

Chapter 3 joining the over-budget list is a **measurement, not a regression** —
the same debt chapters 1 and 2 surfaced. The honest fix stays a chapter split,
which renumbers every later German chapter, and is still deliberately left open
as HL-C242.

### Two defects caught by reading the rendered page

A block count cannot see either of these, and both were found by compiling the
book and looking at it.

1. **The dialogue ran on.** Both practice lessons wrote their four-line exchange
   as consecutive `>` lines. Markdown joins those into one paragraph, so
   *Hallo! Wie geht's? — Gut, danke, und dir? — Es geht.* printed as a single
   run-on line instead of an exchange. Chapter 2 already had the answer — a
   two-column German/English table, one line per row — and both chapter 3
   dialogues now use it.
2. **A wrong sound tag.** `GE-C03-gehen` declared `h-pronounced` while its own
   prose said the *h* "merely lengthens the vowel." The registry has the right
   tag, `h-silent-lengthening`, and the lesson now uses it and teaches the rule
   as transferable — *sehr*, *Ihnen* and *Jahr* all hide the same silent *h*.

### One deliberate wart

Chapter 3 introduces *gehen* first, but `GE-C27-gehen` already owned
`GE-LEX-GEHEN-02` and three chapter-27 lessons require it. Rather than
re-point a generated chapter's atoms, chapter 3 mints `GE-LEX-GEHEN-01` for the
wellbeing verb and chapter 27 keeps `GE-LEX-GEHEN-02` for the motion verb. Two
atoms for one headword is not ideal; folding them belongs with the chapter-27
work, not here.


## German chapters 1 and 2 leave the hand-written set

The book's two opening chapters were hand-written LaTeX. Nothing built them from
the lessons, so every lesson-level gate — the five-minute ceiling, the atom
budgets, the ramp report — was true of the corpus and irrelevant to the pages a
reader actually opened. Both are now generated.

### What moved

- `core/book-generation.d/handwritten.d/german-0001.json` and `german-0002.json`
  are now `targets.d/` entries. German's hand-written chapters: **16 -> 14**.
  `handwritten_parity.py german`: **78 -> 77** blocks at risk.
- Nineteen legacy lessons migrated to schema v2. Generated books refuse anything
  else (`book.ts`: "generated books require schema version 2"), which is the real
  reason chapter 1 could not be flipped even though its measured prose gap was
  already zero. German's atom-measurement-blind lessons: **65 -> 46**.
- Both chapter payoffs are now atom-scored instead of authored boilerplate, and
  the chapter openings carry the hand-written introductions rather than the
  generator's "Complete GE-C0N-practice" stub. Corpus payoffs below the
  representativeness floor: **86 -> 85**.

### Prose carried rather than dropped

A block-count parity check cannot see a sentence lost inside a block that
survived, so each chapter was diffed word-for-word against its hand-written
original. Eight pieces of chapter 1 were living only in the LaTeX and have been
re-homed in the owning lesson:

- the plurals *die Tage*, *die Morgen*, *die Abende*, *die Nächte*, and the rule
  that every German plural takes *die* whatever the gender — now the new lesson
  **GE-C01-die-plural**, which also teaches the umlaut that *Nächte* writes down;
- *Tag* rhyming with English *tock* rather than *tag*, and the final *-g*
  hardening to *k*;
- the *Nacht* **ch** as the same throat-scrape as Spanish *j* and Arabic *kh*;
- the full *ich wünsche Ihnen einen guten Tag* behind the frozen *Guten Tag*;
- first-syllable stress on *Morgen* and *Abend*, the *-en* sinking to *-un*, the
  crisp final *t* of *gut*, the vowel-like murmur of *der*, and French *le/la*
  beside Spanish *el/la*.

Chapter 2's one measured gap was its second `cousinweb`: the etymology of *mich*,
which the hand-written chapter taught inside the *freut mich* section under a
heading the renderer has no type for. It is now **GE-C02-mich**, a lesson of its
own placed *before* *freut mich*, so the phrase no longer uses a word the book
has not taught.

### Splits, and the rule behind them

Three lessons were created, all by splitting rather than by inventing content:

| new lesson | lifted from | why |
|---|---|---|
| `GE-C01-die-plural` | the four day-part grammar lenses | a transferable rule, and four plurals with nowhere else to live |
| `GE-C02-heissen-endungen` | `GE-C02-heissen` | *-e / -t / -en* is "the same machine on every German verb", not a fact about *heißen* |
| `GE-C02-mich` | `GE-C02-freut-mich` | a second headword inside a one-headword lesson |

The rule applied throughout: one lexical, one sound and one etymology atom per
word lesson; a grammar lens that explains *this word* assesses the lexical atom,
while a grammar lens that teaches a transferable rule gets its own atom and, if
that makes four, its own lesson.

`GE-C02-du-sie` also moved from sequence 80 to 68, ahead of *heißen*, so the
conjugation lesson can name *du* and *Sie* without borrowing them from later in
the chapter.

### One defect the parity gate could not see

Reading the generated chapter against its hand-written original — the step a
block count cannot do for you — caught a rendering fault. The High German
Consonant Shift table in `GE-C01-gut` and the day-roots list in `GE-C01-tag`
were written *inside* a markdown blockquote, and the book renderer does not nest
block structure inside a quote. Both collapsed into one unreadable run-on line
with stray `>` markers left in the LaTeX:

```
| English | German | |---|---| | good | gut | | day | Tag | ...
```

Un-nesting them, prose untouched, gives the generator what it can render: a real
`tabularx` for the four shift pairs and an `itemize` for the two day-roots. No
other German lesson nests a list or table in a blockquote, and no other
generated German chapter carries the artifact.

### What this reveals, and does not fix

Making these chapters measurable makes them measured. Chapter 1 introduces 31
atoms across 14 lessons and chapter 2 introduces 23 across 10, against a
twelve-atom chapter budget, so German's `atom-step` queue moves 8 -> 10 and the
corpus line from 27 chapters above 12 to 29. That debt was always there; schema
v1 simply hid it from the gate. Paying it means splitting the chapters
themselves, which renumbers every later German chapter and belongs in its own
change — recorded in `BACKLOG.d` as HL-C242.

### Verification

`node dist/cli.js validate` clean; the full vitest suite green apart from the
pre-existing `figure*.test.ts` (missing paint-vm dependency) and
`script-closure.test.ts` (477 vs a >500 pin, on non-Latin tracks German is not
part of). Every `check:*` gate exits 0. The German book compiles under XeLaTeX
to 211 pages with zero overfull, underfull or missing-character warnings,
holding its all-zero entry in `core/latex-warning-baseline.json`.

## Unreleased — contract the German pre-A1-to-C2 exam ladder

- Added a project-defined pre-A1 bridge followed by the adult Goethe-Zertifikat
  ladder: A1 Start Deutsch 1, A2, B1, B2, C1, and C2 GDS.
- Recorded the official whole-exam A1/A2 pass arithmetic and independently
  passed B1-C2 modules without misattributing the project's stronger per-skill
  two-mock readiness rule to Goethe.
- Required the complete cumulative writing ramp, current-form task inventories,
  two original timed mocks per rung, official-grid-aligned scoring, calibration,
  and book-only human validation before any exam-readiness claim.
- Kept every missing artifact explicit. The contract and existing A1 inventory
  name destinations; they do not prove that the current book reaches them.

## Writing now starts with the first word (#12282)

- Migrate the opening **Hallo** lesson to measurable schema-v2 knowledge.
- Add one model-visible trace, followed by a two-minute guided copy: no recall,
  no free composition, and no new word beyond the greeting already taught.
- Keep the pen block detachable so the voice-first core remains available.

## German chapters 1-16 regain their reading order (#12248)

- Add one global, spaced sequence to all 66 legacy lessons, recovered from the
  hand-authored book sections and closed against every prerequisite and review.
- Remove 66 missing-sequence findings plus 30 forward prerequisites and 36
  forward reviews that alphabetical filename fallback had fabricated. German's
  order-integrity backlog moves from 132 defects to zero.
- Keep genuine content-placement debt separate: nine apparent forward-language
  uses disappear with the real order, while 55 still require teaching or
  reseating work. No learner content is silently declared taught.

## Pre-A1 vocabulary tranche — fourteen everyday nouns, four chapters (2026-08-07)

The level gate (`src/level-gate.ts`) reports every track blocked on
**vocabulary**: 300 distinct headwords at or below pre-A1. This tranche
authors fourteen concrete nouns across four new chapters, the third such
tranche after Hindi, Arabic and Tamil, and confirms the same mechanism:
`vocabularyOf()` counts distinct `headword:` strings, so fourteen one-headword
lessons move German's pre-A1 vocabulary by exactly fourteen — **31 → 45**
distinct headwords at or below pre-A1 (against the 300 target, shortfall
269 → 255), and 77 → 91 distinct headwords track-wide. No bulk credit; measured,
not assumed.

| Lesson | Concept | Word |
|---|---|---|
| `GE-C28-kaffee` | `GE-FOOD-COFFEE` | der Kaffee |
| `GE-C28-tee` | `GE-FOOD-TEA` | der Tee |
| `GE-C28-milch` | `GE-FOOD-MILK` | die Milch |
| `GE-C29-freund` | `GE-PEOPLE-FRIEND` | der Freund |
| `GE-C29-freundin` | `GE-PEOPLE-FRIEND-FEMININE` | die Freundin |
| `GE-C29-familie` | `GE-FAMILY-WHOLE` | die Familie |
| `GE-C30-auge` | `GE-BODY-EYE` | das Auge |
| `GE-C30-ohr` | `GE-BODY-EAR` | das Ohr |
| `GE-C30-mund` | `GE-BODY-MOUTH` | der Mund |
| `GE-C30-nase` | `GE-BODY-NOSE` | die Nase |
| `GE-C31-arm` | `GE-BODY-ARM` | der Arm |
| `GE-C31-finger` | `GE-BODY-FINGER` | der Finger |
| `GE-C31-fuss` | `GE-BODY-FOOT` | der Fuß |
| `GE-C31-herz` | `GE-BODY-HEART` | das Herz |

**Atom-first, three genders taught with every noun.** Each lesson introduces
2–3 knowledge atoms (`GE-LEX-*`, usually `GE-SOUND-*` or `GE-GRAMMAR-*`, and
`GE-ETYMON-*`), at or under `maxNewAtomsPerLesson: 3`. Chapter 28 introduces 9
atoms, Chapter 29 introduces 9, Chapters 30 and 31 introduce 12 each — all at
or under `maxNewAtomsPerChapter: 12`. Every noun's article is taught with the
noun (der/die/das), and German's capitalize-every-noun rule and three-gender
system — already established in Chapter 11 — are reinforced rather than
re-explained.

**Chapter 28 — Coffee, Tea, and Milk** (`SPINE-POLITE-REQUEST-REPAIR`):
extends Chapter 19's `Wasser, bitte` pattern to two more drinks, then closes
on the native word. **Kaffee** is a loanword three hops deep — Arabic *qahwa*
→ Ottoman Turkish *kahve* → Italian *caffè* → German *Kaffee*. **Tee** is a
different loan by a different route — Hokkien Chinese *tê*, carried by Dutch
sea traders — and the lesson names the well-known *tea*/*chai* isogloss split
by name (Hindi, Russian and Turkish took the overland Chinese syllable
instead) without turning it into an uncited language count. **Milch** closes
the trio as the one native Germanic word, from PIE *\*h₂melg-*, "to milk" —
deliberately mirroring Chapter 11's Wasser (native) beside Wein (loan) shape,
now run a third time with two loans instead of one. The payoff lesson also
rescues Chapter 27's two never-revisited orphan atoms, `GE-LEX-SCHLIESSEN-10`
and `GE-ETYMON-SCHLIESSEN-11`.

**Chapter 29 — Friend and Family** (`SPINE-EXCHANGE-NAMES`): **Freund** and
English *friend* are the same inherited word, a frozen Proto-Germanic present
participle of "to love" (PIE *\*preyH-*) — the same root inside Chapter 2's
*freut mich*. **Freundin** teaches German's native feminine suffix *-in* as a
general, reusable rule (Lehrer/Lehrerin, Student/Studentin) and names its one
surviving English fossil, **vixen**. **Familie** is the chapter's one loan —
Latin *familia*, "household," related to *famulus*, "servant" — and closes by
naming that it is the group Chapter 10's Eltern and Geschwister already
belong to.

**Chapter 30 — Eyes, Ears, Mouth, Nose** (`SPINE-CHECK-WELLBEING`): extends
Chapter 17's *Kopf*/*Hand* body-part material with four more parts of the
face. **Auge**/*eye* and **Ohr**/*ear* both trace to confirmed PIE roots;
**Mund**/*mouth* is inherited but its root beyond Proto-Germanic is not agreed
upon, and the lesson says so rather than inventing an ancestor; **Nase**/nose
is cousin to Latin *nasus* (English *nasal*) by shared descent, not
borrowing — the same *rot*/*rouge* shape Chapter 13 already taught. The
payoff also rescues Chapter 26's disputed "sharp-eared" link between *hören*
and *Ohr*, never revisited since it was flagged.

**Chapter 31 — Arm, Finger, Foot, Heart** (`SPINE-CHECK-WELLBEING`): Chapter
17's *Hand* lesson printed a five-word comparison — Hand, Arm, Finger, Fuß,
Herz — and taught only the first. This chapter teaches the other four.
**Arm** sits entirely outside Grimm's law's reach (its consonants were never
in the law's path), which is *why* it looks nearly identical to English *arm*
where *Vater*/*father* do not. **Finger** is identical to its English cousin,
with a proposed but explicitly unproven link to *fünf* ("five"). **Fuß** is a
second *p → f* Grimm's-law case beside *Vater*/*father*, and its **ß**
follows Chapter 13's own long-vowel rule. **Herz** closes the chapter — and
the whole five-word list — with a third instance of Grimm's law's *k → h*
swap, alongside *hören*/*akoúein* (Chapter 26) and *Hund*/*canis* (Chapter
22).

**Reach-back at two cadences (HL09 §7).** Every lesson names atoms from the
one to three lessons immediately before it. Each chapter's payoff also
reaches back several chapters: Chapter 28 to Chapter 19 and Chapter 27;
Chapter 29 to Chapter 2 and Chapter 10; Chapter 30 to Chapters 17 and 26;
Chapter 31 to Chapters 10, 13, 17, 22 and 26. Chapter 31's payoff closes over
all twelve of its own chapter's atoms (1.00 representativeness).

**No forward references.** Where a new word needed an example sentence, every
lesson uses the case-safe `Das ist der/die/das ___.` construction (predicate
nominative, no accusative article) rather than risk an untaught case form on
the mostly-masculine new nouns — Chapter 27's own note that "this track has
not taught cases" still holds. Drink requests reuse Chapter 19's `___, bitte.`
pattern rather than reach for the untaught verb `trinken`.

**Font check.** One lesson draft used Cyrillic (`Tee`'s *чай*) and one used
the unmapped PIE palatovelar diacritic `ǵ` (`Milch`'s root); both were caught
by a forced XeLaTeX compile ("Missing character: there is no ч in font
Latin Modern Roman…") and fixed before commit — Cyrillic dropped in favor of
the transliteration already given in prose, `ǵ` flattened to `g`. One
lesson (`GE-C31-herz`) originally tripped the sight-cue scanner on the literal
phrase "the table"; reworded to "list" throughout, restoring the chapter to
fully `voice`/drivable.

**Verification.** A forced XeLaTeX build of the 166-page book has zero
missing characters, zero overfull/underfull boxes, and zero duplicate labels
from this tranche (the corpus's one pre-existing underfull box, in Chapter
17, predates it). All four new chapters generate as `voice`, drivable
end-to-end. `npx vitest run tests/integration.test.ts tests/cli.test.ts`
passes (19/19); `check:modality`, `check:books` and `check:narration` all
pass with no diff beyond the new chapters. The six corpus-wide pinned-number
tests (`chapters`, `continuity`, `levels`, `modality-manifest`, `narration`,
`ramp`) shift with any authored content and are left failing, per standing
instruction — their numbers are reported here, not re-pinned.

**Wiring**: `GE-PATH-031`–`GE-PATH-034` are four new path segments (one on
`SPINE-POLITE-REQUEST-REPAIR`, one on `SPINE-EXCHANGE-NAMES`, two on
`SPINE-CHECK-WELLBEING`), each with a matching `GE-EXT-0{31..34}-LANGUAGE-SPECIFIC`
extension — both steps are required, since `lessonSpineNodes` only walks
`curriculum.path[].lessons`.

## Eight more core verbs, in two more chapters (2026-08-07)

Chapters 26 and 27 realize the eight core-verb concepts that **no track in the
corpus had realized anywhere**. German goes **14/40 → 22/40** on the taxonomy's
core verbs, and the count of core verbs unrealized in every track drops from
**15 to 7**.

| Lesson | Concept | Word |
|---|---|---|
| `GE-C26-sitzen` | `VERB-SIT` | sitzen |
| `GE-C26-stehen` | `VERB-STAND` | stehen |
| `GE-C26-schlafen` | `VERB-SLEEP` | schlafen |
| `GE-C26-hoeren` | `VERB-HEAR` | hören |
| `GE-C27-gehen` | `VERB-WALK` | gehen |
| `GE-C27-laufen` | `VERB-RUN` | laufen, rennen |
| `GE-C27-oeffnen` | `VERB-OPEN` | öffnen, aufmachen |
| `GE-C27-schliessen` | `VERB-CLOSE` | schließen, zumachen |

**Two chapters, not one**, on the Chapter 24/25 precedent: ten new atoms each,
against `maxNewAtomsPerChapter: 12`, each with its own capability and its own
payoff closing over all ten of its own atoms.

**This is the set where the blood relationship is the lesson.** Every one of
these eight is a cousin rather than a loan, and each shows a *different* and
*teachable* correspondence, so the chapter can say **why** the words look alike:

- *sitzen* **is** *sit* — Proto-Germanic \**sitjaną*, Old High German *sizzen*.
  The second (High German) shift's **t**-branch, which the track had never
  named: Germanic *t* → German *z*/*ss*. It retro-explains *Wasser* (ch. 11)
  and *zehn* (ch. 6), taught long before there was a name for what had happened
  to them. Chapter 25 had already given the **p**-branch on *helfen*.
- *stehen* / *stand* — Germanic ran two stems, \**stāną* and \**standaną*.
  English generalized the one with the nasal, German the one without; German's
  **past** (*ich stand*, *gestanden*) hands the *n* back. PIE \**steh₂-* is the
  root chapter 24 already named inside *verstehen*, and Latin *stō*/*stāre*
  descends from it independently — as Latin *sedeō* does from *sitzen*'s
  \**sed-*. Two roots, two families, no borrowing in either direction.
- *schlafen* / *sleep* — two German changes stacked: the second shift's
  **p** → **f**, plus *s* → *sch* before *l*, *m*, *n*, *w* (*schwimmen*,
  *Schnee*, *Schmied*), which German also did before *p* and *t* and never
  spelt — which is why *stehen* is said *SHTAY-en*. And the honest twist: the
  inherited Indo-European verb for sleeping was \**swep-* (Latin *somnus*,
  Greek *hýpnos*, Old Norse *sofa*). German and English replaced it
  **together**, on \**sleb-* "be slack", before they were two languages.
- *hören* / *hear* — the one change they **share**: Gothic *hausjan* keeps the
  *s* that both Old English *hīeran* and Old High German *hōren* had already
  turned to *r*. The initial *h* is Grimm's law on an old *k*, still audible in
  Greek *akoúein* → English **acoustic**, and the same swap that gave *Hund*
  against Latin *canis*. The "sharp-eared" analysis of the root is reported as
  widely cited and unsettled, not as fact.
- *laufen* / *leap* — a fourth **p**/**f** pair, after *helfen*, *offen* and
  *schlafen*; *lope*, *elope* and *interloper* are the English scraps. Outside
  Germanic there are no secure cousins, and the lesson says so.
- *offen* / *open* — the pair chapter 25 *listed* when it named the second
  shift, now taught. Neither word is a root: both are built on **up**, which is
  why *offen* and *auf* are relatives, and why *aufmachen* is the same recipe
  spoken out loud.
- *schließen* has **no English cousin at all**. Germanic \**sleutaną* survives
  in German and Dutch and left nothing in English, which closes things with
  Latin's *close* and with *shut* — native, but really *shoot*, a bolt shot
  across a door. That is the same story as chapter 25's *nehmen*, and the two
  are named together.
- *gehen* / *go* is the one verb that **cannot** be followed past Germanic; the
  root is disputed and no agreed cousin list exists. Both languages had to
  borrow a past: English took *went* from *wend*, German took *ging* and
  *gegangen* from \**ganganą* — which English still owns in *gangway*.

**The walk/run boundary, taught honestly.** English cuts between *walk* and
*run*; German does not cut in the same place. *gehen* is unhurried, *rennen* is
flat out, and *laufen* lies across the line, so **Wir laufen** is "we're
walking" or "we're running" and only the situation decides. That is true across
most of Germany and **not** in Austria, where *laufen* means *run* — recorded
in the lesson as a regional split, per HL09 §8.1, rather than dropped.

**Separable verbs, introduced with nothing new.** The track had never taught
them. HL09 §5.2 allows a new structural move only on vocabulary the reader
already holds, and both halves were held: *auf* from chapter 4's *auf
Wiedersehen*, *machen* from chapter 5. So `GE-C27-oeffnen` introduces the split
— **Ich mache die Hand auf** — and `GE-C27-schliessen` immediately re-uses it
for *zumachen*. The object is *die Hand* throughout, because feminine and
neuter accusative articles are identical to the nominative and this track has
not taught cases.

**Reach-back at two cadences (HL09 §7).** Every one of the eight names atoms
from the one to three lessons immediately before it, across the chapter seam,
and both payoffs reach several chapters back. Six atoms that no lesson had ever
revisited are revisited here — `GE-SOUND-HAND-03`, `GE-ETYMON-HUND-03`,
`GE-LEX-REGNET-05`, `GE-LEX-MOEGEN-LIEBEN-09`, `GE-ETYMON-MOEGEN-LIEBEN-10`,
`GE-GRAMMAR-GERN-11`. The track's never-revisited share falls from **31 of 61
atoms (51%) to 27 of 81 (33%)**. The two atoms of the final lesson are orphans
because nothing follows them yet.

All eight lessons are `voice` — drivable, no tables, no sight cues. Book: 140
pages, zero missing characters, zero overfull boxes under XeLaTeX.

## Eight core verbs, in two chapters (2026-08-07)

Chapters 24 and 25 add the eight verbs Spanish, Latin and Portuguese landed
last, so each of them turns a three-way cross-language join into a four-way one.
German goes **6/40 → 14/40** on the taxonomy's core verbs.

| Lesson | Concept | Word |
|---|---|---|
| `GE-C24-denken` | `VERB-THINK` | denken |
| `GE-C24-verstehen` | `VERB-UNDERSTAND` | verstehen |
| `GE-C24-lesen` | `VERB-READ` | lesen |
| `GE-C24-schreiben` | `VERB-WRITE` | schreiben |
| `GE-C25-nehmen` | `VERB-TAKE` | nehmen |
| `GE-C25-fragen` | `VERB-ASK` | fragen |
| `GE-C25-helfen` | `VERB-HELP` | helfen |
| `GE-C25-moegen-lieben` | `VERB-LIKE-LOVE` | mögen, lieben |

**Two chapters, not one.** Eight one-verb lessons introduce twenty atoms
against `maxNewAtomsPerChapter: 12`. Splitting is the resolution, not raising
the budget: chapter 24 introduces 10, chapter 25 introduces 10, and each has
its own capability and its own payoff. Page count is never a cost.

**What only this track can do.** These are English's *blood* relatives, not
loans, so the cousin webs are the point:

- *denken* **is** *think* — Proto-Germanic \**þankijaną*, with Grimm's law
  turning PIE \**t* into the *th* English kept and German softened to *d*. It
  is also the verb chapter 3 promised inside **danke**, finally given its own
  forms.
- *verstehen* is *ver-* + *stehen*, "to stand around" — and English built
  *understand* out of the same inherited verb and its own prefix, separately.
  The "under = among" account is given as **standard but not certain**.
- *lesen* first meant "to gather" (*die Weinlese*) — and the resemblance to
  Latin *legere*, which walked the same road from "gather" to "read", is
  named as **probably not** a shared word: the sounds do not correspond, and
  the standard account ties *lesen* to a separate root. English *read* is a
  third story entirely, Old English *rædan*, "to advise" — German *raten*.
- *schreiben* is the one **borrowing**: Latin *scrībere*, taken in early
  enough to pass through German sound changes (*sc-* → *sch-*) and join the
  native strong verbs. English refused the loan and kept *write*, also "to
  scratch". *Manuskript* then closes chapter 17's circle — the Latin hand-word
  German never inherited, bolted to the Latin writing-verb it did.
- *nehmen* is the verb **English threw away**; *numb* ("taken" by cold) and
  *nimble* are its fossils, and Greek's form of the root gives *nomad*,
  *economy* and *nemesis*.
- *fragen* is PIE \**preḱ-*; English lost the native cousin and gets the root
  back only through Latin *precārī* — *pray*, *precarious*. Latin *rogāre* is
  flagged as **not** related.
- *helfen* **is** *help*, split by the **second** (High German) shift, not by
  Grimm's law — *Schiff*/ship, *offen*/open, *scharf*/sharp — and English's own
  *holp*/*holpen* show *help* was once strong too. Outside Germanic the verb
  has **no secure cousins**, and the lesson says so rather than inventing one.
- *mögen* is *may*, *lieben* is *love*, and **gern** is *yearn*.

**Two grammar payoffs German alone can give this set.** Strong-verb vowel
change is introduced on *lesen* (`GE-GRAMMAR-STRONG-VOWEL-09`) and then
re-practised on *nehmen* (*du nimmst*, with the silent *h* dropping and the
*m* doubling) and *helfen* (*du hilfst*) — with *schreiben* and *fragen* as the
counter-examples that keep the rule exact. And *mögen* / *lieben* / **gern**
is German's three ways of liking, where *ich lese gern* ("I read gladly") has
no English shape at all.

**False friends flagged, not skipped**: *also* means "therefore", never
"also"; *bekommen* means "to receive", never "become".

**Reinforcement at two cadences (HL09 §7).** Every lesson names atoms from the
one to three lessons immediately before it — across the chapter seam — because
a chapter-end payoff cannot close the R1 window. On top of that, each payoff
reaches back much further: chapter 24's to `GE-LEX-HAND-02`,
`GE-ETYMON-HAND-MANUS-05` and `GE-SOUND-GRIMMS-LAW-04` (chapter 17), and
chapter 25's to all four chapter-24 verbs plus `GE-LEX-HUND-02`,
`GE-LEX-KATZE-04` and `GE-LEX-WETTER-02`. The reach-backs are real practice —
*Ich denke, es ist kalt*, *Die Hand schreibt*, *Ich mag die Katze*, *Der Hund
mag Wasser* — not name-checks.

**No forward references.** Nothing is used before it is taught, and no lesson
teases the next one. Where a construction the course has not reached would be
needed — the accusative article after *mögen*, the dative object of *helfen* —
the lesson says so and stays inside what the reader can already produce.

**Wiring**: `GE-PATH-027` and `GE-PATH-028` are two new `SPINE-SAY-WHAT-I-DO`
segments, and the eight concepts leave that node's `omits` ledger (36 → 28).
All eight lessons derive as `voice`, so both chapters are drivable end to end;
effective durations are 282–298 s against the 300 s ceiling.

## German joins the cross-language core verbs (2026-08-07)

- Retagged **six** verb lessons from language-local ids to the canonical
  concepts owned by `SPINE-SAY-WHAT-I-DO`, so German's verbs finally join the
  cross-language corpus instead of being seven private ids no other track can
  see: `GE-C16-sein` → `VERB-BE`, `GE-C14-haben` → `VERB-HAVE`,
  `GE-C03-gehen` → `VERB-GO`, `GE-C05-machen` → `VERB-DO-MAKE`,
  `GE-C05-lernen` → `VERB-LEARN`, `GE-C05-wohnen` → `VERB-LIVE`
  (*wohnen* is taught as "to live / to dwell", which is exactly `VERB-LIVE`).
  German's core-verb coverage goes **0/40 → 6/40**, and `VERB-DO-MAKE` and
  `VERB-LEARN` leave the corpus-wide `universallyMissing` list (29 → 27) —
  German is the first track anywhere to realize either.
- `GE-C02-heissen` keeps its namespaced `DE-VERB-HEISSEN`. No core concept
  means "to be called": *heißen* is not a translation of anything on the
  shared list, and forcing it onto one would be a false join.
- **Rewired `curriculum.json` so the realization path matches the retag.** A
  canonical concept obliges its lesson to sit in the segment of the node that
  owns it, so the four verb tranches moved into their own
  `SPINE-SAY-WHAT-I-DO` segments — `GE-PATH-011` (*gehen*), `GE-PATH-014`
  (*wohnen*, *machen*, *lernen*), `GE-PATH-018` (*haben*), `GE-PATH-021`
  (*sein*) — and left `SPINE-CHECK-WELLBEING` and `SPINE-TIME-OF-DAY`, where
  they had been sitting as language-specific extension material.
- **Teaching order is untouched.** No chapter was reordered and no lesson
  renumbered: each moved lesson holds the exact position it already had, and
  the segments were split around it rather than resequenced. *gehen* still
  lands immediately before *wie geht es*, which needs it.
- Three orphan lessons entered the path because the retag required it.
  `GE-C05-lernen` and `GE-C16-sein` realize canonical concepts and so cannot
  be absent from it; *sein* in turn declares `GE-C15-praeteritum` as a
  prerequisite (its *war/waren* forms are Präteritum), which pulled
  `GE-C15-perfekt` and `GE-C15-praeteritum` in behind it. Those two are a
  German-local past, not `VERB-PAST`, so they are recorded as a
  `SPINE-TALK-ABOUT-PAST` segment (`GE-PATH-020`) whose omission ledger still
  names `VERB-PAST` as undelivered. The node is now realized without the
  debt being quietly written off.
- Path segments were renumbered `GE-PATH-001..026` and extensions renamed to
  match their host segment, keeping the ids monotonic in path order as every
  other track has them. `GE-EXT-011-LANGUAGE-SPECIFIC` was deleted outright:
  it existed only to classify *gehen* as local support, and *gehen* is now
  shared content.
- Derived levels move accordingly: German reaches **A2** for the first time,
  corpus `A2` 91 → 99, `A1` 307 → 304, `pre-A1` 657 → 656, unmapped 170 → 166.

## Chapter capability ledger — Chapters 17–23 (2026-08-06)

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
- **Chapter 17 fails the 0.5 representativeness floor at 4/12 = 0.33.** It runs
  three word lessons deep — *Kopf*, *Kopf/Haupt*, *Hand* — with no terminal
  consolidation lesson, so the payoff can only be the last lesson by
  `sequence` and reaches just its own third of the chapter. The shortfall is
  recorded in the ledger rather than hidden; the fix is a real
  Kopf/Haupt/Hand practice lesson, not a longer `assesses` list.
- Chapter 18 also lacks a terminal practice lesson but still reaches 5/8 = 0.63
  because *nein* reassesses *ja*. Chapters 19–23 are single-lesson chapters and
  assess everything they introduce (1.00).
- Titles and labels are copied verbatim from `core/book-generation.json`, so the
  `chapter-title-drift` gate holds through the HL-C04 inversion.

## Warning-free 104-page book (2026-08-03)

- Made intentionally short micro-lesson pages explicit with `\raggedbottom`,
  removing eleven underfull vertical boxes without padding learner content.
- Added concise running titles and a prose-only Chapter 12 bookmark, made the
  Chapter 10 practice path breakable, and reflowed three dense explanations.
- Replaced rigid legacy comparison tables with bounded paragraph columns while
  preserving every vocabulary, grammar, register, and etymology comparison.
- Shortened only the visible `Entschuldigung` heading and reflowed the canonical
  `Kopf` recall; regenerated hashes keep the book and Language Ladder on the
  same source while the full explanations remain intact.
- A forced XeLaTeX build produces 104 pages with zero missing glyphs, overfull
  or underfull boxes, duplicate destinations, Hyperref warnings, or LaTeX
  warnings. All 104 rendered pages were inspected, and the outline retains the
  Preface, pronunciation reference, and all twenty-three chapters.

## Canonical Chapters 17–23 (2026-08-03)

- Migrated the ten lessons in Chapters 17–23 to schema version 2 with typed
  blocks, explicit shared-spine concepts, prerequisite-closed knowledge atoms,
  and honest sub-five-minute duration contracts.
- Repaired the missing shared-spine step between yes/no and sorry: a new
  164-second `bitte` lesson assembles only previously learned words into
  **Wasser, bitte**, while `Entschuldigung` moves from Chapter 19 to Chapter 20.
- Generated seven LaTeX chapters from those canonical lessons and added
  independent Language Ladder source-hash and lesson-count assertions, so the
  app and downloadable book now consume one source of truth through Chapter 23.
- Expanded the book from 84 to 104 pages. A forced XeLaTeX build has no missing
  glyphs, duplicate destinations, LaTeX warnings, or leaked generator metadata;
  all 104 rendered pages and the complete outline were inspected.
- Recorded eighteen overfull boxes, one underfull horizontal box, eleven
  underfull vertical boxes, and three Hyperref warnings for the focused HL-B21
  cleanup tranche.

## Sub-five-minute lesson remediation (2026-08-02)

- All twenty-seven German duration violations are resolved. Twenty-two lessons
  already computed below five minutes and now declare an honest four-minute
  budget without changing their teaching content.
- Five lessons that genuinely exceeded the limit become prerequisite-ordered
  micro-sequences: informal wellbeing → formal *Ihnen* register → separate
  casual/formal practice; Präteritum forms → its north/south areal map; the
  *sein*-perfect auxiliary family → French/German agreement; *Kopf* as cup →
  inherited *Haupt* and the Grimm's-law/container comparison.
- The five new support lessons bring the German track to 86 lessons. Every new
  or rewritten step computes between 147 and 244 seconds, with zero unknown
  prerequisite ids.
- A forced build still succeeds at 84 pages with no missing glyphs or duplicate
  labels. Its existing seventeen overfull boxes, eleven underfull boxes, and
  three Hyperref warnings are recorded separately in `HL-B21`; publishing the
  canonical Chapters 17–23 is recorded in `HL-B20`.

## The book catches up -- Chapters 3-16 typeset

The lessons had run ahead of the published artifact: 61 authored lessons through
Chapter 16, but the LaTeX book still stopped at Chapter 2 ("Introducing
Yourself"). Because the CI book build only compiles what is wired into
`book.tex`, the missing chapters were invisible to CI and the gap drifted
silently. This closes it -- **fourteen new book chapters**, written from the
existing `GE-C03`-`GE-C16` lessons and wired into `book.tex`:

- **Ch3** How Are You (danke, bitte, gehen, wie geht es, es geht)
- **Ch4** Farewells (auf Wiedersehen, tschuess, bis bald, bis morgen)
- **Ch5** The First Verbs (wohnen, machen, lernen, ich lerne Deutsch)
- **Ch6** Numbers One to Ten * **Ch7** The Days of the Week (and Mittwoch)
- **Ch8** Telling the Time * **Ch9** Months and Seasons (Herbst/harvest)
- **Ch10** Family * **Ch11** Bread, Water, Wine
- **Ch12** Numbers Eleven to Twenty (elf/zwoelf, the "-lif = left over" story)
- **Ch13** Colours * **Ch14** To Have, and How Old You Are (the habere false
  cognate)
- **Ch15** The Two Past Tenses (Perfekt, Praeteritum)
- **Ch16** To Be, and the Past That Takes It (sein -- three ancient verbs in one
  paradigm -- and the Perfekt built on it)

Each chapter follows the established book conventions: one `\section` per lesson
with a slug `\label`, the `cousinweb` / `culture` / `grammarlens` / `sounds` /
`etymology` / `morphologybox` boxes, `booktabs` conjugation tables, and every
atom traced to its root -- the German/English cognate webs are the spine.
Content is faithful to the lessons -- no new etymologies introduced.
Practice-section labels are chapter-qualified (`lesson:chN-practice`).

The book grows to **84 pages**; compiles clean with XeLaTeX (0 errors, 0 missing
characters, 0 undefined references, 0 duplicate labels) and was rasterized and
visually QA'd -- the umlauts, the eszett, `fui` with macron, and the PIE
superscripts all render correctly.

## Chapter 17 — The body: a cup for a head, and a hand with no Latin cousin

- **Chapter 17 authored** (`GE-C17-kopf`, `-kopf-haupt`, `-hand`) — the **body**, the theme the
  parallel-track roadmaps name next.
- **der Kopf** (`GE-C17-kopf`): *Kopf* did not originally mean "head." It meant a
  **cup or bowl** — the same word as English **cup**, both early borrowings of
  Late Latin ***cuppa*** — and it displaced the inherited **das Haupt**, the
  Grimm's-law cognate of Latin *caput* and English *head*. The clean
  demonstration there is **k→h** (*caput* / *Haupt* / *head*); the later
  consonants involve a second shift, so the lesson takes k→h and leaves the rest.
  *Haupt* survives in compounds: *Hauptstadt*, *Hauptbahnhof*, *Hauptsache*.
  - **The chapter's best fact is a coincidence.** French replaced "head" with a
    **pot** (*testa* → *tête*) and German with a **cup** (*cuppa* → *Kopf*), with
    nobody coordinating — and **both** kept the old word for chiefs and capitals.
    It is the **metaphor** that was invented twice, not the vocabulary: both
    vessel-words trace back to Latin. Heads look like bowls in any language.
  - *(Corrected here: #8746 fixed this formula in the lesson, roadmap and
    taxonomy but missed the CHANGELOG, which kept a wrong `p→f/d` and called
    \*kuppaz native Germanic. A claim lives in four places.)*
  - Includes the **-pf** note: one sound, *p* released into *f*, with no English
    equivalent.
- **die Hand** (`GE-C17-hand`): the easy word, kept deliberately for what it
  teaches about **absence of connection**. Germanic \**handuz*, inherited
  straight into English (*Hand, Arm, Finger, Fuß, Herz*), with the **final-devoicing**
  note — *Hand* ends in a *t* sound, and the *d* returns in *die Hände*.
  - **Every Romance track in this course builds "hand" on *manus*** (*main*,
    *mano*, *mão*), and \**handuz* **is not related to it**. The lesson says this
    outright, because a curriculum that keeps finding connections can start to
    imply everything connects. It doesn't — and this is where the two families
    diverged early and completely.
  - *Manus* did reach German, but only as **borrowed** learned vocabulary
    (*Manuskript*, *Maniküre*, *manuell*), sitting beside the native word without
    displacing it.

## Chapter 16 — *sein*: three ancient verbs wearing one infinitive

- **Chapter 16 authored** (`GE-C16-sein`, `-perfekt-sein`,
  `-perfekt-sein-agreement`). Ch. 15 taught only
  the *haben* half of the Perfekt because *sein* had never been taught. Fixed.
- **sein** (`GE-C16-sein`): the present, plus *war/waren*, and then the reason
  they look unrelated — they **are** unrelated. *sein* is assembled from **three
  Proto-Indo-European roots**:
  - *ist, sind, seid, sein* ← \**h₁es-* (Latin *est, sunt*; French *est, sont*)
  - *bin, bist* ← \**bʰuH-* "grow, become" (English **be**; Latin *fuī*)
  - *war, waren* ← \**wes-* "dwell, remain" (English **was, were**)
  - The cross-track payoff: \**bʰuH-* is the root of **Spanish *fui***, taught in
    ES-C14 as a *pretérito fuerte*. The **same** root surfaced in German's
    **present** and Spanish's **past**.
- The lesson also states the general law rather than leaving it as trivia: **the
  most-used words are the most irregular**, because regularity spreads by
  **analogy** and you never have to guess at "to be". Rare words get regularised;
  common ones are protected fossils. This is the answer to "why is *sein* like
  this" in every language at once.
- **Perfekt with sein** (`GE-C16-perfekt-sein`): the **motion / change-of-state**
  split, on *gehen* (Ch. 3), *kommen*, *fahren*, *werden*, plus the set that
  breaks the pattern and must simply be learned — ***sein*** and ***bleiben***,
  which are the *opposite* of change, along with *gelingen*, *geschehen* /
  *passieren* and *begegnen*.
  - **The contrast that matters: no agreement.** French makes the participle
    agree with the subject (*elle est allé**e***); **in the perfect** German
    makes it agree with **nothing** (*sie ist gegangen*, for every person and
    gender). Scoped deliberately to the perfect, because German participles **do**
    still inflect attributively (*der angekommen**e** Zug*, *ein geschrieben**er**
    Brief* — chosen over *der gegangene Weg*, which only licenses attributively
    via the marked transitive *einen Weg gehen* and reads stiff); Old High German
    inflected them in the perfect too, and German lost that.
  - **Corrected direction of influence.** German did **not** inherit this from
    Latin. The *haben*- and *sein*-perfects are native Germanic developments that
    grew up **alongside** the Romance ones through centuries of contact — the
    same areal spread this repo already credits (in `FR-PAST-SIMPLE-LITERARY`)
    for the simple past retreating in French, German and Italian together. Stated
    as *split parallel, agreement not shared*.

## Chapter 15 — The Perfekt, and the tense it pushed aside

- **Chapter 15 authored** (`GE-C15-perfekt`, `-praeteritum`,
  `-praeteritum-map`): the everyday past,
  built on Ch.14's *haben* — reviewing Ch.5/14 via `reviews_of`.
- **Perfekt** (`GE-C15-perfekt`): *haben* + past participle (*ich habe gesagt*),
  with two things German does that English can't. First, the weak participle is
  **wrapped** — a **ge-…-t circumfix**, not a suffix. Second, it goes to the **end
  of the clause** (*Ich habe gestern Deutsch **gelernt*** — "I have yesterday German
  learned"), which is simply ungrammatical in English. Plus the semantic note that
  it means the **plain past** ("I said"), not only "I have said." Etymology: **ge-**
  ← Germanic *\*ga-* "together, completely," a **perfective** marker — exactly what
  a past participle is for. English had it as *y-* and dropped it, leaving two
  fossils: **enough** (Old English *genōg*) and archaic *yclept*. So English once
  wrapped its participles the same way; German never stopped.
- **Präteritum** (`GE-C15-praeteritum`): *ich sagte* — the simple past, same
  meaning as the *Perfekt* but a different register. Its **-te** is the Germanic
  **dental preterite**, the identical machinery behind English **-ed** (*walked*)
  and Dutch *-te* — a **Germanic invention** with no Latin equivalent, since Romance
  builds its past from inherited perfect endings instead (*parla*, *habló*).
  Register and geography: nearly gone from speech in the south, better preserved in
  the north, standard in **narrative writing**, with *war*, *hatte* and the modals
  resisting everywhere. Closes on the three-language table — **German, French and
  Italian** each let a "have" compound displace their simple past — an AREAL change spread by contact, not three separate inventions.
- Taxonomy: namespaced `GE-PAST-COMPOUND`, `GE-PAST-SIMPLE-WRITTEN`.

## Chapter 14 — haben, and being your years

- **Chapter 14 authored** (`GE-C14-haben`, `-alter`): the workhorse verb plus the
  one everyday place German won't use it, reviewing Ch.5/9/10/12/13 via
  `reviews_of`.
- **haben** (`GE-C14-haben`): *habe/hast/hat/haben/habt/haben*, where *du hast*
  and *er hat* **drop the b** — precisely as English *have* → *ha**s*** (and
  archaic *hast*), one shortcut the two languages inherited together. The
  showpiece is a **false cognate**: *haben* ← Germanic *\*habjaną* ← PIE *\*kap-*
  "to **seize**," whose Latin child is ***capere*** (→ *capture, captive, capable,
  accept*) — while Latin ***habēre*** (which gave French *avoir* and Italian
  *avere*) descends from *\*gʰabʰ-*, whose English descendant is **give**. The two
  words that look most alike and mean the same thing come from **opposite**
  ancestries; German *haben* is kin to *capture*, Latin *habēre* to *give*.
- **ich bin zwanzig Jahre alt** (`GE-C14-alter`): the one everyday slot where
  German **refuses** *haben* — age takes **sein**, producing word-for-word the
  English sentence, and shortening the same way (*ich bin zwanzig*). *Jahr* ←
  *\*jēra* = **year**; *alt* ← *\*aldaz* = **old**, with the Latin cousin *alere*
  "to nourish, grow" behind English *adult*. Closes on the five-language table:
  **all four Romance sisters *have* their years; German sides with English and
  *is* its years** — and does so even though it borrowed its month names from
  Latin (Ch.9).
- Sets up the *Perfekt*, which is built on *haben*.
- Taxonomy: namespaced `GE-VERB-HAVE`, `GE-AGE`.

## Chapter 13 — Colours

- **Chapter 13 authored** (`GE-C13-schwarz-weiss`, `-rot-blau`): German as the
  **lender** rather than the borrower, reviewing Ch.11/12 via `reviews_of`.
- **schwarz & weiß** (`GE-C13-schwarz-weiss`): both **native Germanic**, no Latin
  anywhere. *Schwarz* ← *swartaz*, whose English cousin survives as **swarthy**, and
  which is kin to Latin *sordēs* ("dirt") → *sordid* — black and grubby from one
  idea. *Weiß* ← *hwītaz* = **exactly** English *white*; includes the **ß** rule
  (sharp *s* after a long vowel; Swiss spelling *weiss*). The showpiece: German's own
  **blank** ("shiny, polished, bare") is the very word Romance **borrowed** for
  **white** — *blanc/bianco/branco* — while German kept the original meaning. This
  reverses the direction seen in Ch.11 (*Wein* ← *vīnum*, *Fenster* ← *fenestra*).
- **rot & blau** (`GE-C13-rot-blau`): *rot* ← *raudaz* ← PIE ***h₁rewdʰ-***, so *rot*
  and French *rouge* are related **by descent, not borrowing** — they split millennia
  before either language existed. *Blau* ← *blēwaz* is the **second** German colour
  word Romance took (*bleu*, *blu*), and English took **blue from French** rather
  than from its own Germanic stock. Closes with a four-row table of which words
  Romance borrowed and which it already had a cousin for.
- Taxonomy: namespaced `GE-COLOUR-BLACK-WHITE`, `GE-COLOUR-RED-BLUE`.

## Chapter 12 — Numbers 11–20

- **Chapter 12 authored** (`GE-C12-elf-zwoelf`, `-zahlen-13-20`): the teens,
  atom-first, reviewing Ch.6/Ch.11 via `reviews_of`.
- **elf / zwölf** — the showpiece: ← *ainlif / twalif*, where **-lif** means "**to
  leave, remain**," so they literally say "**one left over**" and "**two left
  over**" — left over from your **ten fingers**. English *eleven/twelve* are not
  merely similar but **the same inherited words**, which is why both languages share
  the oddity. Extends the Germanic-twin thread from *Vater/father*, *Wasser/water*.
- **dreizehn–zwanzig** — then the pattern turns perfectly regular: **digit + zehn**,
  no exceptions, exactly mirroring English *-teen* (which **is** *ten*: *thir-teen* =
  "three-ten"). *Sechzehn/siebzehn* clip a sound just as English clipped
  *three→thir-*, *five→fif-*; *zwanzig* ← *twaintig* "two tens" (= English *-ty*).
- **The contrast made explicit**: the Romance sisters all **break** their teens
  pattern partway (PT at 16, FR/IT at 17); **German never breaks** — two leftovers,
  then one clean rule to twenty, with English marching alongside the whole way.
- Taxonomy: namespaced `GE-NUM-11-12`, `GE-NUM-13-20`.

## Chapter 11 — Food (bread, water, wine)

- **Chapter 11 authored** (`GE-C11-brot`, `-wasser-wein`): the everyday table
  trio, atom-first, reviewing Ch.10/Ch.1 via `reviews_of`.
- **Brot** ("bread") — **inherited Germanic**, the direct twin of English *bread*
  (NOT the Latin *pānis* the Romance sisters use); introduces the **neuter das**
  (completing *der/die/das*) and the rule that German **capitalizes all nouns**.
- **Wasser / Wein** — the native-vs-borrowed pair: **Wasser** ("water," *w*=*v*) is
  a native Germanic twin of *water*, but **Wein** ("wine," *ei*="eye") is an
  **ancient Latin loan** ← *vīnum*, taken with the grapevine Rome carried north —
  which is exactly why *Wein*, English *wine*, and *vīnum* all match (one loan, not
  three cousins).
- Taxonomy: namespaced `GE-FOOD-BREAD`, `GE-FOOD-DRINKS`.

## Chapter 10 — Family

- **Chapter 10 authored** (`GE-C10-eltern`, `-geschwister`): the immediate family,
  atom-first, reviewing Ch.9/Ch.1 via `reviews_of` — and the **mirror image of the
  months chapter**.
- **der Vater / die Mutter** — taught as **inherited Germanic** words (NOT Latin
  loans like the months), the Grimm's-law twins of English *father / mother*: the
  *V* of *Vater* is pronounced *f*, and German/English agree (*f-*, *m-*) precisely
  because both are Germanic, while French/Latin sit across Grimm's line. The
  standout thread: **family is native where the calendar was borrowed**.
- **der Bruder / die Schwester** — Germanic twins of *brother / sister*; plus
  **die Geschwister** ("siblings"), built with the **collective ge-** prefix that
  English lacks.
- Taxonomy: namespaced `GE-FAMILY-PARENTS`, `GE-FAMILY-SIBLINGS`.

## Chapter 9 — Months & seasons

- **Chapter 9 authored** (`GE-C09-monate`, `-jahreszeiten`): the calendar year,
  atom-first, reviewing Ch.6–8 via `reviews_of`.
- **The native-vs-Latin split deepens** (numbers native, weekday-gods Germanic,
  clock *Uhr* Latin — now): the **months are Latin loans** (Januar ← Janus, *März*
  ← Mars = *Dienstag*'s Tiw, September–Dezember = Latin 7–10), reaching for Rome
  just as *Uhr* did — while German's own numbers stay *sieben, acht, neun, zehn*.
- **The seasons swing back to native Germanic**: *Frühling* ← *früh* "early" (the
  early-season); *Sommer/Winter* = the plain twins of English *summer/winter*; and
  the surprise, **Herbst = English harvest** — the same Germanic reaping-word, which
  English narrowed to the *act* while taking Latin *autumn* for the season.
- Taxonomy: namespaced `GE-MONTHS`, `GE-SEASONS`.

## Chapter 8 — Time & the clock

- **Chapter 8 authored** (`GE-C08-uhr`, `-mittag-mitternacht`): telling the time,
  atom-first, reviewing Ch.6–7 via `reviews_of`.
- **Uhr** — the **standout Latin loanword**: German's numbers are native (*eins,
  zwei*) and its weekdays are Germanic gods (*Donnerstag*), but its *clock*-word
  came from Latin **hōra** (the same *hōra* behind French *heure*, Italian *ora*,
  English *hour*). Three layers of the day, three origins — native numbers,
  Germanic day-gods, Latin clock. (Native *Stunde* = an hour's span; *Uhr* =
  o'clock, no plural: *es ist zwei Uhr*.)
- **Mittag / Mitternacht** — noon/midnight swing back to **native** compounds:
  *Mitte* ("middle") + *Tag* ("day") / *Nacht* ("night," the *Nacht/night* twin
  from the numbers). Same meaning as French *midi/minuit* (*medius diēs*), but
  built from German's own words rather than borrowed.
- Taxonomy: namespaced `GE-TIME-HOUR`, `GE-TIME-NOON-MIDNIGHT`.

## Chapter 7 — Days of the week

- **Chapter 7 authored** (`GE-C07-wochentage-1`, `-wochentage-2`): the seven days,
  atom-first, reviewing Ch.6 via `reviews_of`.
- **wochentage-1** (Montag–Freitag): like the numbers, German goes **Germanic** —
  its weekdays are the **twins of the English days**, named for Germanic gods, not
  Latin planets. *Donnerstag* (*Donner* "thunder" = Donar/**Thor**) = *Thursday*,
  standing in for the Roman Jupiter; *Freitag* (Frigg) = *Friday*. The odd one,
  **Mittwoch** "mid-week," is a religious edit — the Church replaced "Woden's day"
  (which English kept as *Wednesday*), mirroring how Portuguese numbered its days.
- **wochentage-2** (Samstag, Sonntag): the surprise that **Samstag is the Sabbath,
  not Saturn** — *Sabbat* ← Greek *sábbaton* ← Hebrew *shabbāt*, reaching German
  through the early Church, so *Samstag* and Spanish *sábado* share a root while
  English alone keeps *Saturday* = Saturn; *Sonntag* = "Sun's day" = *Sunday* (a day
  the Church left un-renamed, unlike Romance *domingo/dimanche*).
- Taxonomy: namespaced `GE-DAYS-WEEKDAYS`, `GE-DAYS-WEEKEND`.

## Chapter 6 — Numbers 1–10

- **Chapter 6 authored** (`GE-C06-zahlen-1-5`, `-zahlen-6-10`): counting to ten,
  atom-first, each ~5 min, reviewing Ch.5 via `reviews_of`.
- **The distinctive German story**: unlike the Romance tracks, German numbers are
  **not Latin loans** — they're German's own Germanic words and the **near-twins of
  English one…ten**, with Latin as a *cousin* one sound-shift away. **Grimm's Law**
  is the through-line: old *p* → Germanic *f* (*\*pénkʷe* → *fünf/five*, while Latin
  kept *quīnque*); old *d* → *t* → German *z* (*decem → ten → zehn*); and the
  *acht/eight* ~ *Nacht/night* *-cht-/-ght-* correspondence.
- **The month names *are* Latin loans** (*September, Oktober…*), so the 7–10
  calendar trick still shows even though *sieben/acht/neun/zehn* look nothing like
  them — numbers homegrown, month labels imported.
- Taxonomy: namespaced `GE-NUM-1-5`, `GE-NUM-6-10`.

## Chapter 5 — The first verbs (sentences start to move)

- **Chapter 5 authored** (`GE-C05-wohnen`, `-machen`, `-lernen`,
  `-ich-lerne-deutsch`, `-practice`): German's first **grammar-engine** chapter,
  parallel to French Ch.5 / Spanish Ch.6. Uses **regular (weak) verbs only** —
  *sprechen* is irregular and deferred.
- **The regular weak present tense** — drop *-en*, add *-e/-st/-t/-en/-t/-en*.
  Taught on **wohnen** and cemented on **machen** and **lernen**. Unlike French,
  **German endings are audible** (*wohne/wohnst/wohnt* differ).
- **The pronoun rule completed across three languages**: Spanish **drops** *yo*
  (the ending says who); French **keeps** *je* (endings silent); German **keeps**
  *ich* — for yet another reason: its grammar needs an **overt subject**
  (structure, not sound).
- **Etymology, English-cousins-you-own**: *wohnen* ← *wonēn* (→ *wont*
  "accustomed"); *machen* ← *makōn* (= English **make**; the High German *k*→*ch*
  shift); *lernen* ← *liznōjan* (= **learn**; kin of *lore*); *Deutsch* ←
  *diutisc* "of the people" (→ English **Dutch**, **Teutonic**). First
  self-assembled sentence: **Ich lerne Deutsch**.
- Taxonomy: namespaced `GE-VERB-WOHNEN/MACHEN/LERNEN`, `GE-WORD-DEUTSCH`
  documented.

## Writing nuances — the eszett, the umlauts, capital nouns

- **First German `writing`-type lessons** (`GE-W01-eszett`, `GE-W02-umlauts`,
  `GE-W03-capitalization`): orthography taught etymology-first, once enough
  special-character words have accumulated (*heißen*, *weiß*, *Straße*).
- **ß (eszett)**: a long-*s* + *s/z* ligature (hence "es-zett"), always a sharp
  *s*; the rule **ß after long vowels, ss after short** (*Straße* vs *Fluss*) —
  which doubles as a vowel-length cue; no word-initial/lowercase-only quirks
  (ALL-CAPS → SS; Switzerland drops it).
- **Umlauts ä/ö/ü**: the two dots as a **shrunken migrated *e*** (ASCII fallback
  *ae/oe/ue*: *Müller = Mueller*); "um-laut" = around-sound (vowel fronting); and
  the grammar it marks — plural/comparative/diminutive fronting (*Mann→Männer*,
  *groß→größer*, *Hund→Hündchen*). Contrasted with the French tréma.
- **Großschreibung**: German capitalizes **every noun**, mid-sentence and all —
  a part-of-speech signal that disambiguates (*essen* "to eat" vs *das Essen*
  "the food"), a living fossil of older European printing (English dropped it
  ~1700s).
- Uses the `writing` lesson type (no `concept_tag`) — no taxonomy change.

## Chapter 4 — Farewells (completes the ES/FR/DE farewell trilogy)

- **Chapter 4 authored** (`GE-C04-auf-wiedersehen`, `-tschuss`, `-bis-bald`,
  `-bis-morgen`, `-practice`): closing a conversation, atom-first, reviewing
  Chapter 3. Reuses the shared `FAREWELL` / `FAREWELL-SOON` / `FAREWELL-TOMORROW`
  concepts and adds `FAREWELL-CASUAL`.
- **auf Wiedersehen** = "on the seeing-again" (*sehen* = English *see*) — the
  exact twin of French *au revoir*, both against Spanish *adiós* "to God".
- **tschüss**, the best etymology in the chapter: *tschüss* ← Low German
  *atschüs* ← Walloon *adjûs* ← French *adieu* — so the breeziest German bye is
  secretly **"to God"**, a far-travelled cousin of *adiós* and *adieu*.
- **The "bis …" family** mirrors Spanish *hasta* / French *à*: *bis bald* (soon —
  *bald* ← Old High German "bold/quick", = English *bold*), *bis später* (later),
  *bis morgen* (tomorrow — *Morgen* = English *morning/morrow*, the same
  morning→tomorrow move as *mañana* / *demain*).
- Taxonomy: `FAREWELL-CASUAL` added (canonical, `core:false`).

## Chapter 3 — "Wie geht's?" (completes the how-are-you trilogy)

- **Chapter 3 authored** (`GE-C03-danke`, `-bitte`, `-gehen`, `-wie-geht-es`,
  `-wie-geht-register`, `-es-geht`, `-practice`, `-formal-practice`): the
  "how are you?" exchange, atom-first, reviewing
  Chapter 2. Third of a deliberate cross-language trilogy in this PR (Spanish
  Ch.4 / French Ch.3 / German Ch.3), all sharing the canonical concepts
  `STATE-HOW-ARE-YOU`, `COURTESY-YOUREWELCOME`, `WORD-SOSO`.
- **The etymologies English speakers already own**:
  - *danke* ← *denken* "to think" — and English *thank* IS *think* (both from
    Old English *þancian*/*þencan*), set against *merci* (reward) and *gracias*
    (grace).
  - *bitte* ← *bitten* "to ask/pray" — cognate of English *bid* and *bead* (a
    bead was a prayer); the one word doing please / you're-welcome / here-you-go
    / pardon.
  - *gehen* IS English *go* (straight Germanic cognate); *es geht mir gut* = "it
    goes well *to me*" — gently introduces the **dative** (*mir/dir/Ihnen*).
  - *es geht* ("it goes," nothing added) as the understated shrug for "so-so."
- **The trilogy's payoff**, stated in-lesson: German and French say wellbeing as
  motion ("how does it **go**?"), Spanish as posture ("how are you
  **standing**?" — *estar*).
- Taxonomy: namespaced `DE-VERB-GEHEN` documented.

## Chapter 2 — Introducing Yourself

- New chapter built around the introduction dialogue (*Ich heiße Susanne. / Wie
  heißen Sie? / Ich heiße David. / Freut mich.*), atom-first, one word per
  lesson (`lessons/GE-C02-*`, `book/chapters/ch02-introductions.tex`):
  - **ich** ("I" ← *\*ik* / PIE *\*eǵ*; cousin of Latin *ego*, English *I*).
  - **heißen** ("to be called" ← *\*haitaną*; English archaic *hight*, *behest*)
    — German names with a plain verb, no reflexive "myself."
  - **ich heiße…** — **"my name is…"** ("I am called"), with literal *mein Name
    ist* (*Name* ← *\*namô*, English *name* / Latin *nōmen*) as the alternative.
  - **du / Sie** (familiar / formal "you") — *Sie* is the capitalized 3rd-person
    plural "they" used as polite "you"; the third route to politeness beside
    Spanish *usted* and French *vous*.
  - **wie** ("how" ← *\*hwī* / PIE *\*kʷo-*; English *how/what/who*).
  - **wie heißen Sie?** — **"what's your name?"** ("how are you called?");
    verb-second word order; informal *wie heißt du?*.
  - **freut mich** ("pleased to meet you" = "it gladdens me"; ← *froh*, "glad").
    Its object pronoun **mich** ("me") is traced too — ← *\*mek* / PIE *\*me-*,
    cousin of English *me/my/mine* and French *me* (every atom rooted, not
    glossed).
  - **practice** — the whole dialogue.
- Book compiles clean with XeLaTeX.

## Beginner-audience + parity pass

Brought the German book fully to the Hindi/Spanish standard. Two things:

**Stop assuming prior Spanish/French (HL00 Audience rule).** The books are for a
true beginner whose only shared language is English; German leaned on the other
tracks as knowledge already owned.
- Preface: dropped "exactly as the Spanish book used the *-ct-→-ch-* rules" and
  "Because the reader also knows Spanish (and is meeting French)"; states the
  true-beginner framing and that every Spanish/French form is supplied in full.
- `ch01-greetings.tex`: "German's version of the Spanish *-ct-→-ch-* rule" →
  self-contained sound-law framing; "the same job *bueno/buena* and *bon/bonne*
  did" → "the same job Romance adjectives do."
- Practice lessons `GE-C01-gut` ("the rules you met in Spanish") and
  `GE-C01-der-die-das` ("You've met gender in Spanish and French") de-assumed.

**Filled the parity gaps the audit flagged.**
- Added per-word **`sounds` boxes** (the book previously gave pronunciation only
  inline): *hallo*, *gut*, *der/die/das*, *Tag*, *Morgen*, *Abend*, *Nacht* ---
  including German final-devoicing (*Tag* → *tahk*, *Abend* → *AH-bent*) and the
  *ach*-laut in *Nacht*.
- Added noun **plurals**: *die Tage*, *die Morgen*, *die Abende*, *die Nächte*.
- Book still compiles clean with XeLaTeX (14 pages).

## Chapter 1 — Greetings (track bootstrapped)

- New German track on the HL00 framework: one word per lesson, slug ids,
  gender-before-nouns, atom-first, derivations shown, LaTeX book (CI
  auto-discovers `german/book/`).
- Chapter 1 (`lessons/GE-C01-*`), atom-first, with German's Germanic-roots
  flavor:
  - **hallo** (a *real* cousin of English "hello," unlike Spanish *hola*)
  - **gut** ("good" *and* "well" ← Germanic *\*gōdaz* = English *good*;
    introduces the **High German Consonant Shift** d→t as a recurring decoder)
  - **der / die / das** ("the"; **three** genders — German kept the neuter;
    ← Germanic *\*sa/\*sō/\*þat*, cousins of English *the/that*)
  - **Tag** (← *\*dagaz* = English *day*; ≠ Latin *dies* behind *día/jour*)
  - **Guten Tag** (assembled; the *-en* accusative ending)
  - **Morgen** (← *\*murganaz* = *morning/tomorrow*) · **Guten Morgen**
  - **Abend** (← *\*ābanþs* = English *eve*; contrast with Romance "late"
    words) · **Guten Abend**
  - **Nacht** (← PIE *\*nókʷts* — the four-way *Nacht/night/noche/nuit*
    reunion; feminine) · **Gute Nacht** (feminine agreement, *-e* not *-en*)
  - **practice**
- Grounds each word against English (direct Germanic cousin), with Spanish and
  French alongside for contrast. Book compiles clean with XeLaTeX (13 pages).
