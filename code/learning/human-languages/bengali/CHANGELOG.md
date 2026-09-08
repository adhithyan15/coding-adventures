# Changelog

## Chapter 39 — the ten digits, and the two shapes a reader who already reads digits will get wrong

`BN-A1-Q-03` read "NOT ONE DIGIT IS TAUGHT." It is now covered, and so is
`BN-A1-LIP-11`, which had been filed in the script column for the same piece of
work and said in its own note that closing it in one place would close it in
both. Coverage 139/244 → **141/244**. Twelve lessons in one chapter, eleven
atoms, ten new glyphs.

**SIXTY SCRIPT LESSONS HAD PUT NOTHING BUT LETTERS ON THE PAGE.** That is the
finding underneath the point. The Bengali strand is the largest in the corpus
after Tamil's, and every one of its lessons taught a letter or a sign — so a
price, a date, a clock face and a page number were all unreadable, and chapter
24's `BN-W02-tin-read` had taught the reader to read **তিন**, the *word*, while
**৩** remained a shape they had never seen. The chapter's payoff is that
distinction.

**THE ORDER IS BY WHAT THE READER CAN BRING TO EACH SHAPE, AND THE FOUR BANDS
WERE READ OFF THE RENDERED GLYPHS RATHER THAN REMEMBERED.** Every claim below was
checked by rasterising the digits in the book's own vendored font at 80pt and
looking at them:

- **০ and ২ agree.** ০ is a plain ring, exactly like **0**, and means zero. ২ is
  close enough to **2** to be kept after one telling, and means two. Wikipedia
  calls the Bengali numerals *a variety of the Hindu–Arabic numeral system* —
  one system, not two — which is why the agreement is there at all.
- **৩ collides with a letter rather than with a digit.** Set **৩** beside **ও**,
  which this book taught long ago: same rounded body, and **ও** carries a stroke
  down its right-hand side that **৩** does not. One stroke is the whole
  difference between a number and a letter here, and it is the only such
  collision in the set.
- **১, ৫ and ৬ carry nothing** and are simply learned. The book says so rather
  than inventing a mnemonic.
- **৪ and ৭ LIE, and they lie in the same way.** **৪** is drawn as an **8** and
  means **four**. **৭** is drawn as a **9** and means **seven**. In both cases
  the Bengali digit that really means eight or nine — **৮**, an upright with a
  single bowl, and **৯**, a body with its tail sweeping *up* — refused the
  familiar shape, leaving it empty for a smaller number to move into. So each
  trap is taught **immediately after** the digit whose value its shape suggests:
  ৮ then ৪, ৯ then ৭. Two of ten, and the rule lesson says which eight may be
  trusted so that a reader slows down on two shapes rather than distrusting the
  page.

**NO PEN PATH IS CLAIMED FOR ANY OF THE TEN, AND THAT WAS SEARCHED FOR RATHER
THAN ASSUMED.** HL-C212 established that Commons holds stroke-order animations
for every Bengali independent vowel and for no consonant at all. The same search
run for the digits finds nothing: the Commons numeral categories hold
photographs, the letter-writing app the Gujarati track cites covers Bengali
*letters* only, and Wiktionary gives ৪ as "Bengali Digit Four, U+09EA" with no
writing guidance. So every lesson teaches the **shape** against something the
reader already holds and cites the Unicode Bengali chart — exactly what
`BN-W05-tha` and `BN-W06-nya` do, and for the same reason.

**THE RULE ATOM SHIPS WITH BOTH ITS EDGES.**
`BN-SCRIPT-DIGIT-FALSE-FRIENDS-01` names where the reader's existing reading
helps (০, ২), where it is simply absent (১ ৩ ৫ ৬ ৮ ৯), and where it actively
misleads (৪, ৭) — and the last band is closed at exactly two members.

**REINFORCEMENT, DECOMPOSED ATOM BY ATOM RATHER THAN BY TOTALS:**

    reinforcementWindowMisses        242 -> 242
    reinforcementMissesByWindow-R1    38 ->  38
    reinforcementMissesByWindow-R2    49 ->  49
    reinforcementMissesByWindow-R3    90 ->  90
    reinforcementMissesByWindow-R4    65 ->  65
    atomsTaught                      206 -> 217
    atomsNeverRevisited                3 ->   3
    scriptClosureViolations           21 ->  21
    neverTaughtGlyphs                  8 ->   8
    taughtGlyphs                      37 ->  47

Every window is flat. The tranche's own eleven atoms create **zero** debt: each
digit lesson recalls the previous digit by name (R1), and the rule lesson and the
payoff between them retrieve all ten (R2). Twelve more lessons make **25**
(atom, window) slots newly judgeable on OLDER atoms — five R2, ten R3, ten R4 —
debt the length **exposes** rather than creates, and **all twenty-five are
answered**. The ten R4 slots fall on a clean diagonal: lesson *n* services the
atom introduced at position *n*−80, so the four clothing words of chapter 15 land
on the four hardest digit lessons, where a price is exactly what a reader would
be looking at. Zero regressions; nothing closed outright, which is honest — this
chapter had no pre-existing miss inside its reach.

**THE SCRIPT COST WAS MEASURED PER LESSON, AND ONE LEAK WAS CAUGHT THAT WAY.**
Because a script lesson teaches every target-script glyph in its body, a lesson
that quotes an untaught letter silently marks it taught. A per-lesson walk over
the chapter — taught set derived from the corpus at point of use, not
transcribed — caught three: **১** appearing in the *zero* lesson before its own
lesson, and **শ** entering through শাড়ি and চশমা in two warm-up recalls. All
three were removed; the two clothing recalls now cue their words in romanization,
which retrieves the same lexical atoms without teaching a letter by accident.
Every lesson now teaches exactly its own digit and nothing else, and
`scriptClosureViolations` and `neverTaughtGlyphs` are both unchanged.

**WHAT IS NOT CLAIMED, and the pages say it rather than leaving it to be
inferred.** This chapter teaches no *word*. The reader can now READ every number
and SAY only the first five, because the count in this book still stops at পাঁচ.
The sixth lesson states that boundary in the middle of the chapter, the payoff
restates it, and the test asserts that no lexical atom exists for শূন্য, ছয়,
সাত, আট or নয়. Two of those words additionally need য়, a letter the track does
not teach — which is a second reason the spoken half is a separate piece of work
and is filed as `BN-A1-Q-01`.

## Chapter 38 — the ordinals are a second layer of the language, and chapter twelve had already said so

`BN-A1-Q-04` read "Untaught. No ordinal exists in the corpus, so nothing can be
put in a sequence — not a floor, not a date, not a turn." It is now covered:
seven lessons, nine atoms, and one new letter.

**THE NOTE SAID THE MACHINERY WAS UNAVAILABLE. THE CORPUS SAID OTHERWISE.**
Bengali's ordinals are Sanskrit *tatsama* — borrowed back whole — so none of the
"see the number inside it" reading that Gujarati, Marathi and Kannada used is
open here. But **BN-C06-numbers-1-5 had already taught the reason**: it says in
so many words that the old *dv-* is gone from the everyday numeral and is
"alive in words borrowed straight back out of Sanskrit", and it gives **দ্বার**
*dvār* as its example. **দ্বিতীয়** is another of those words. The chapter
therefore opens on **SECOND**, and the thing already present in another guise is
not a word but a **relationship** — a prediction the reader was handed
twenty-six chapters early, coming true.

**THE ORDER, AND WHY IT IS NOT NUMERICAL.**

- **দ্বিতীয়** first: the one ordinal whose relation to its cardinal the reader
  has already been taught. Its lesson also has to say that **the cluster is on
  the page and not in the mouth** — Wiktionary gives the pronunciation as
  *ditiyô*, and a book that romanized it *dvitīyô* would be lying to the reader
  about a word it is teaching them to say.
- **তৃতীয়** second: *dvitīya* : *tṛtīya*, one Sanskrit ending, and now the
  **-তীয়** shape has two members. The atom is introduced HERE and not on
  দ্বিতীয়, because one word is not a shape.
- **চতুর্থ** third: a second ending, **-র্থ**, with one member. Named, and
  explicitly NOT called a pattern — the point it makes is that the borrowed
  layer arrived with more than one shape in it.
- **প্রথম** fourth: no number in it at all. And the finding is that **Bengali
  did not do that** — Sanskrit's *one* was *eka* and its *first* was *prathama*,
  and the two never shared a sound. The gap is older than the language being
  learned. Kannada's rule, reached by a different road: coming fourth, after a
  shape has held twice and a second ending has appeared once, the word can be
  read for what it is instead of taken as a Bengali oddity.
- **ঞ**, then **পঞ্চম** last, because the fifth is the payoff.

**THE PAYOFF IS THAT THE TWO LAYERS ARE VISIBLY ONE WORD.** Samsad's
*Bengali-English Dictionary* keeps **পঞ্চ** *ponchô* as a Bengali headword
glossed **five**, with **পঞ্চম** under it as *fifth*. Set that beside **পাঁচ**
and the two differ by exactly one thing: where the borrowed word has the letter
**ঞ**, the everyday word has the **ঁ** that chapter 12 introduced as "the
chandrabindu, nasalising the vowel". Wiktionary's entry for **পাঁচ** writes its
Sanskrit source as **পঞ্চ** and walks the road — *pañca*, *paṃca*, a Middle
Bengali form already spelled with the mark, **পাঁচ**. So the chapter's thesis is
checkable rather than asserted, and the evidence is four letters wide.

**THE ONE LETTER, TAUGHT THE WAY HL-C212 REQUIRES.** পঞ্চম needs **ঞ**, which
was shown in the corpus and taught nowhere. Commons holds a stroke-order
animation for every Bengali independent vowel and for **no consonant at all**,
so — following BN-W05-tha and BN-W05-tta exactly — the lesson teaches place of
articulation and the row **ঞ** completes, cites the Unicode Bengali chart, and
**claims no pen path**, because none can be sourced. `neverTaughtGlyphs` falls
**9 → 8**.

**NO AGREEMENT, AND THE CHAPTER SAYS SO.** A Bengali ordinal stands in front of
its noun and never changes, exactly like **লাল**. That is worth one sentence and
no more, and it is the opposite of what the Gujarati and Marathi ordinal
chapters had to teach.

**REINFORCEMENT, DECOMPOSED ATOM BY ATOM RATHER THAN BY TOTALS:**

    reinforcementWindowMisses        249 -> 242
    reinforcementMissesByWindow-R1    38 ->  38
    reinforcementMissesByWindow-R2    49 ->  49
    reinforcementMissesByWindow-R3    90 ->  90
    reinforcementMissesByWindow-R4    72 ->  65
    atomsTaught                      197 -> 206
    atomsNeverRevisited                3 ->   3
    neverTaughtGlyphs                  9 ->   8

The tranche's own nine atoms create **zero** debt. Seven more lessons make **17**
(atom, window) slots newly judgeable on OLDER atoms — three R2, six R3, eight R4
— debt the length EXPOSES rather than creates; each was assigned by window
arithmetic to the exact new lesson inside its window and **all seventeen are
answered**. Seven pre-existing R4 misses close outright, and they are exactly the
material this chapter had a reason to reach for: the five numbers, the two
chapter-12 histories under দুই, the chandrabindu, and the three letters of the
চ row that **ঞ** completes. Zero regressions on previously judged slots.

**THE RECALL LINES ARE IN THE PROSE, NOT ONLY IN THE FRONTMATTER.** Every
warm-up names its recalls by word, and the declared `assesses` list is exactly
those; the blocks were audited one at a time against the paragraphs beside them,
and four lists that named an atom the prose did not engage were corrected rather
than left to make the metric look tidy.

**THE SCRIPT COST WAS MEASURED, NOT ASSUMED.** The taught set was DERIVED from
the corpus at point of use — every Bengali codepoint in the headword of every
`BN-W*` lesson, 36 of them before this chapter and 37 after — and every Bengali
character in the tranche is in it. `scriptClosureViolations` holds at 21, all of
it pre-existing. Every Latin diacritic used was checked against
`core/main-font-charset.json`.

**NOT CLAIMED.** The ordinals past fifth (Bengali keeps borrowing them, but each
one is a separate word and none of them is derivable from a cardinal the reader
has). The Bengali digits, still zero of ten. And `BN-A1-Q-04` closed **without
any of the cardinals moving**, which is worth recording: the ordinal column here
never depended on the count going past five, because none of these words is
built on a count.

**VERIFIED.** `npm run build`, then all ten `check:*` gates from `dist`; the
whole package suite; language-ladder via `bash BUILD`; and the Bengali book
compiled with XeLaTeX through the materialized entrypoint, with the new pages
read as rendered images rather than through `pdftotext`.

## Unreleased — the letter that was in the way, and the grammar behind it

Thirty-five lessons in seven chapters (31-37), twenty-eight of them items, close
**seventeen A1 exam points**. Three columns close outright — definiteness 1/3 to
**3/3**, deixis 0/3 to **3/3**, possession 2/3 to **3/3** — and pronouns reach
7/8.

The ratio comes from a **letter**. ট was shown in the corpus and taught nowhere,
and six points sat behind it: a learner could not say *the* shirt, *a* shirt,
*another* shirt, *this* shirt or *what is this?*, because none of those can be
written in Bengali without it. Teaching one consonant unblocked a whole grammar.

### Every number re-measured against the merged tree, not derived from a delta

    bengali A1 exam coverage                 121/244 (50%) -> 138/244 (57%)
    bengali lessons                                   158 -> 193
    bengali atoms taught                              169 -> 197
    bengali atoms never revisited                       4 ->   3
    bengali forward references                          4 ->   8   (see below)
    bengali forward prerequisites / reviews           0/0 -> 0/0   (held)
    bengali script closure violations                  21 ->  21   (held)
    bengali never-taught glyphs                        11 ->   9   (ট and থ)
    bengali headwords without romanization              0 ->   0   (held)
    bengali reinforcement window misses               208 -> 249
      R1 (1-3)                                          38 ->  38
      R2 (5-15)                                         47 ->  49
      R3 (20-60)                                        85 ->  90
      R4 (80-250)                                       38 ->  72
    lessons at or over the computed 300s ceiling         0 ->   0
    four-column tables the narrator must refuse         44 ->  44   (held)
    corpus rule statements                              31 ->  31   (held)

### Seventeen points from twenty-eight items, and the ratio is one letter

ট is taught first, as the fourth corner of a square whose other three the book
already owned: ত dental voiceless, দ dental voiced, ড retroflex voiced. The
classifier টা follows it, and then একটা, আরেকটা, এটা and the stacking rule all
follow the classifier. Six points, one consonant.

The animacy split does the same work a second time: one question about a noun —
is it alive? — chooses the plural ending in chapter 33 and the object ending in
chapter 36, so N-06, N-05 and KAR-01 are three points on one rule.

### What is taught, one item per lesson

    ch31  ট, টা, একটা, আরেকটা          the letter, the ending Bengali has
                                       instead of an article, and what it
                                       unlocks
    ch32  এই, ওই, এই …টা, এটা          two distances, both ends of the phrase
                                       at once, and *what is this?*
    ch33  জীব/জিনিস, রা, গুলো, লাল জামা  one question, two plurals, and where an
                                       adjective stands
    ch34  সে, তিনি, ও, তার             the third person, graded for respect and
                                       sorted by distance
    ch35  আমরা, তোমরা, তারা, আমাদের     six persons on one ending, and the rule
                                       that the back slot holds ONE suffix
    ch36  কে, আমাকে, তোমাকে, কে?        the object case, and chapter nine's
                                       dative frame finally filled
    ch37  থ, এখানে, ওখানে, কোথায়        the ninth cell of a three-by-three grid

### Two spine omission ledgers go empty

BN-C20-tta-classifier realizes GRAMMAR-THE, so SPINE-DEFINITE-REFERENCE omits
nothing; BN-C26-kothay realizes QUESTION-WHERE, so SPINE-ASK-LOCATION omits
nothing. Both are declared through `conceptAliases` in the track's
`curriculum.d/_meta.json`, because the classifier is not an article and the
lesson keeps its own name for it.

### The script debt did not move, and two glyphs were bought

Every candidate was tested against the union of taught glyphs before it was
written, and the worked examples were tested too — প্রাণী was the first draft's
word for "living thing" and it hides a ণ the track has never taught, so the
lesson says জীব instead.

ট and থ are **consonants**, and HL-C212 established that Wikimedia Commons holds
a stroke-order animation for every Bengali independent vowel and for **no
consonant at all**. So neither lesson claims a pen path. Both are written the
way this track's other twenty-six consonant lessons are written — place of
articulation, the square the letter completes, and a Unicode Bengali chart
citation. A guessed ductus is not an option, and none is guessed here.

Still blocked on a letter, and named in the inventory beside the point:

    কারণ   "because"        needs ণ  -> ADV-08 stays null
    অনেক   "much, many"     needs অ  -> Q-05 stays null
    মানুষ  "person"         needs ষ  -> not needed by any open point

### The reinforcement number, decomposed

    created by this tranche      0   no atom introduced by any of the 35 new
                                     lessons misses a window it was long enough
                                     to have
    exposed by its added length 41   all on PRE-EXISTING atoms, and 34 of the 41
                                     are R4 alone: thirty-five more lessons made
                                     R4 judgeable for older atoms that had never
                                     been 80 lessons from the end of the track
    atoms never revisited        4 -> 3

Every chapter opener retrieves the two preceding items by name and carries a
bulleted recall line for an item two chapters back, which is the R2 reach; every
chapter review carries a distant band that reaches four and five chapters back,
which is the R3 reach.

### Forward references: one false positive fixed, four made attributable

    4 -> 9 -> 8

BN-C05-kaj-kora takes করা apart as ক + রা and emphasised the second syllable,
which the detector read as the plural suffix this tranche now teaches. The
Bengali is still on the page; the emphasis is not. That is the same false
positive and the same remedy as the বা case in chapters 27-30.

The remaining four are **not** false positives and are **not** new debt. They are
places where the corpus already put a word in front of the reader and taught it
nowhere at all; teaching it — late — is what made the earlier use measurable:

    BN-C07-jaowa        তিনি      101 lessons early, taught at ch34
    BN-C09-sahajjo-kora আমাকে     100 lessons early, taught at ch36
    BN-C13-sbagotom     আমাদের     68 lessons early, taught at ch35
    BN-C15-jama         লাল জামা   44 lessons early, taught at ch33

An untaught preview counts as nothing; a taught one counts as a forward
reference. The number rising is the debt becoming attributable, not appearing.

### Left uncovered, with the reason written into the inventory

    BN-A1-JOIN-07   যে … সে — the correlative itself, now that the pronoun exists
    BN-A1-JOIN-10   the purpose suffix; the verbal noun is already taught
    BN-A1-ADV-08    কেন and কারণ as items; কারণ cannot be printed at all (ণ)
    BN-A1-QUE-05    কখন is glossed inside যখন's table and in ch36's, not taught
    BN-A1-PRON-08   the reflexive নিজে
    BN-A1-Q-01/03   the numerals stop at five, and no Bengali digit is taught

## Unreleased — the track can say no, join two things, and ask for a repeat

Nineteen lessons in four chapters (27-30), fifteen of them items, close
**seventeen A1 exam points**. The negation column goes from 1/5 to **5/5**, the
first column in this track to close completely, and the joining column from 1/11
to 8/11.

The functional hole was named in one line by the inventory: "আমি বুঝি is taught,
in chapter 14; না is taught, in chapter 1; the two are never put together
anywhere in 139 lessons." Nothing in the book could deny a sentence, and nothing
could join two nouns, so a learner who owned four foods, four garments and five
colours could not say *tea and milk* and could not say *I do not know*.

### Every number re-measured against the merged tree, not derived from a delta

    bengali A1 exam coverage                 104/244 (43%) -> 121/244 (50%)
    bengali lessons                                   139 -> 158
    bengali atoms taught                              154 -> 169
    bengali atoms never revisited                       5 ->   4
    bengali forward references                          4 ->   4   (see below)
    bengali forward prerequisites / reviews           0/0 -> 0/0   (held)
    bengali script closure violations                  21 ->  21   (held)
    bengali never-taught glyphs                        11 ->  11   (held)
    bengali headwords without romanization              0 ->   0   (held)
    bengali reinforcement window misses                191 -> 208
      R1 (1-3)                                          38 ->  38
      R2 (5-15)                                         51 ->  47
      R3 (20-60)                                        88 ->  85
      R4 (80-250)                                       14 ->  38
    lessons at or over the computed 300s ceiling         0 ->   0

### Seventeen points from fifteen items, and the ratio is one word

না was taught in chapter one as the answer "no" and never used again. Three of
the seventeen points ride on it: it negates a verb (NEG-02, SEN-02), it is the
second half of কেননা, "because" (JOIN-08), and doubled and moved to the front it
is *neither … nor* (NEG-05). Ranking clusters by points-per-item before designing
anything is what put negation before vocabulary here.

### What is taught, one item per lesson

    ch27  না, নয়, নেই, the rising question   three negators for three kinds of
                                              sentence, and every frame in the
                                              book becomes a question
    ch28  আর, -ও, বা, না … না                 joining, marking, choosing, and
                                              refusing both
    ch29  কিন্তু, একমত, কেননা                 objecting, agreeing, explaining
    ch30  যে, যখন, আবার বলবেন, ধীরে           reporting a thought, placing it in
                                              time, and rescuing a conversation

Two of the fifteen introduce **no new vocabulary at all**. আবার বলবেন is আবার
from chapter seven, বলা from chapter eleven and the polite -বেন from chapter
twenty-two: the repair sentence was missing because nobody had assembled it, not
because it was hard.

### Two spine omission ledgers go empty

BN-C16-na-verb realizes VERB-NEGATE and BN-C16-polar realizes QUESTION-POLAR, so
SPINE-NEGATE-AND-ASK omits nothing; BN-C18-kenona realizes CONNECTIVE-BECAUSE, so
SPINE-SAY-WHY omits nothing.

### The script debt did not move, and that was designed rather than lucky

Every candidate word was tested against the union of taught glyphs before it was
written. আর, বা, কিন্তু, কেননা, যে, যখন, একমত and ধীরে need no sign the track has
not taught. Three words that would have covered the same points do:

- **কারণ** ("because") needs ণ, so **কেননা** carries JOIN-08 instead and কারণ is
  named in romanization. BN-A1-ADV-08 stays null for that letter alone.
- **এবং** ("and") needs ং, so আর carries JOIN-01.
- **বোঝা** ("to understand") needs ঝ, so *āmi bujhi nā* — the sentence the
  inventory named as the one a beginner needs most — is printed in romanization
  and the Bengali sentences are আমি জানি না and আমি ভাত খাই না.

Each of those names its own fix, and the fix is a script lesson rather than a
vocabulary lesson. Violations held at 21 and never-taught glyphs at 11.

### The forward references this tranche created, and the fix

Teaching বা as a headword retroactively made four earlier lessons forward
references: BN-C01-dhonnobad, BN-C05-ami-bangla-boli, BN-C08-bhaba and
BN-C11-poribar each spell a syllable **বা** in bold inside ধন্যবাদ, বাংলা, ভাবা
and পরিবার. forwardReferences went 4 -> 8 for that reason alone and is back at 4:
the Bengali is still on the page, the emphasis is not, and the detector reads
only emphasised runs.

### The reinforcement number, decomposed

    created by this tranche      0   (zero new atoms miss any window)
    exposed by its added length 24   (R4 14 -> 38, all pre-existing atoms)
    pre-existing misses cleared  7   (R2 -4, R3 -3)

Nineteen more lessons made R4 judgeable for 24 older atoms that had never been
80 lessons from the end of the track before. Every chapter opener retrieves the
two preceding items by name and carries a bulleted recall line for an item two
chapters back, which is the R2 reach — and no new atom misses R1, R2, R3 or R4.

### Left uncovered, with the reason written into the inventory

    BN-A1-JOIN-05    একটা … আরেকটা needs the classifier AND ট
    BN-A1-JOIN-07    যে … সে needs the third person of BN-A1-PRON-03
    BN-A1-JOIN-10    the purpose suffix; the verbal noun is already taught
    BN-A1-ADV-08     কেন and কারণ as items; কারণ cannot be printed at all
    BN-A1-QUE-05     কখন is glossed inside যখন's table, not taught as an item

### Verified

`npx vitest run` (133 files, 1910 passed), every `check:*` gate, and the Bengali
book compiled with XeLaTeX: exit 0, zero missing characters, four new chapters
read on the page as images rather than through `pdftotext`, which mangles
Bengali clusters.
## The pronunciation reference stops being hand-written LaTeX

`bengali/book/chapters/appendix-pronunciation.tex` was hand-authored and printed as a
`\chapter*`. It is now rendered from `bengali/pronunciation-reference.md`.

**Restored from the LaTeX before the flip:** the macron in *bād* — the Markdown
had written Sanskrit *vāda* → *bad*, losing the long vowel the example is about —
and the claim that Bengali *carries an immense literary tradition*, which the
Markdown had compressed into a bare mention of Tagore.

**Corrected:** the conjunct example read `স্ + ক → স্ক, চ + ছ → চ্ছ`. The second
half lost its hasanta, so the rule the sentence states was contradicted by its
own example. It now reads `চ্ + ছ → চ্ছ`, and the hasanta ্ is shown in the
sentence that names it.

The endonym stays as the Markdown's *Bāṅlā* rather than the LaTeX's *Bānglā*:
বাংলা is written with an anusvara, and *Bāṅlā* is what transliterates it.

## Unreleased — the retrieval is seated per lesson, and Bengali enters R2

Eighth track through the HL-C313 fix, using the per-lesson seating rule
(HL-C318), and the one where the ceiling and the achieved number disagree most —
for a reason worth stating rather than hiding.

**Seat capacity said 72 of 85 were seatable. Only 34 closed.** The gap is not
budget and not the window. It is that the rule sources retrieval only from WORD
lessons, because a word lesson has a headword and a romanization that make
`say *X*` / `read **X**` a real task. Bengali's R2 debt does not live in word
lessons: of the 51 that remain, **32 were introduced by a writing lesson**, 8 by
a word lesson, 6 by a phrase lesson and 5 by a practice lesson.

Bengali also has a third kind of blocker the other tracks do not: 13 of its 85
misses have **no word lesson anywhere inside their 5-15 window**, because its
word lessons are sparse rather than full. No seating rule reaches those; only a
lesson that does not exist yet would.

### Every number re-measured against the merged tree, not derived

    bengali R2 misses (5-15)                           85 ->  51   (-34)
    bengali R1 misses (1-3)                            38 ->  38   (held)
    bengali R3 misses (20-60)                          88 ->  88   (held)
    bengali R4 misses (80-250)                         14 ->  14   (held)
    bengali reinforcement window misses               225 -> 191
    bengali atoms taught                              154 -> 154   (held)
    bengali atoms never revisited                       5 ->   5   (held)
    bengali lessons                                   139 -> 139   (held)
    forward prerequisites                               0 ->   0   (held)
    forward references                                  4 ->   4   (held)
    script closure violations                         271 -> 271   (held)
    corpus R2 misses                                 4212 -> 3725
    lessons at or over the 300s ceiling                 0 ->   0
    computed seconds, median                          172 -> 172   (held)

25 lessons gained a line; the book carries 25 recall lines, 17 with a read.

Falsified before shipping: reverting `BN-C02-naam` and re-measuring put R2 back
to 54 — the three atoms carried by the three lessons it retrieves.

## [Unreleased]

### Added — eight shapes, a body-parts reading chapter, and a retrieval chapter (HL-C201)

The previous tranche measured its own ceiling honestly: with every script lesson
hypothetically pre-taught, closure still reported 36, so **reordering was worth
at most five more and the rest was inventory**. This tranche buys the inventory.

**Twenty-four lessons, four chapters, no vocabulary invented.** Every one of the
eight new shapes is anchored on a word the learner has already said aloud, taught
one per lesson, and placed before the lessons that show it wherever gloss-first
allows:

| chapter | shape | anchor, first said in |
|---|---|---|
| 4 | **য** | হ্যাঁ, the greetings chapter |
| 4 | **ঁ** | হ্যাঁ, the greetings chapter |
| 8 | **গ** | লাগলো, inside *ālāp kore bhālo lāglo* |
| 9 | **ছ** | আছি, the responding chapter |
| 10 | **ও** | হওয়া, glossed in the farewells chapter |
| 10 | **়** | য় inside হওয়া |
| 17 | **ড** | পড়া, the mind-and-page chapter |
| 21 | **ৃ** | হৃদয়, the body chapter |

`neverTaughtGlyphs` **22 → 11**; `taughtGlyphs` 26 → 34; `scriptLessons` 45 → 60.

Chapter 8 split into two so the grid could take গ and ছ without breaching the
twelve-atom chapter ceiling: chapter 8 now closes the throat row (ক/খ/গ) and
chapter 9 closes the whole twelve-cell grid. Two chapters are new outright —
**chapter 10**, which buys ও and the dot below and reads হওয়া whole, and
**chapter 21**, which reads চোখ, নাক and মুখ for free before spending ৃ on হৃদয়.
মুখ moved from the colours-reading chapter to the body-reading chapter, where the
word it names was taught.

### Changed — Sanskrit citation forms are given in IAST, not in Bengali script

An **editorial** decision, and a pedagogical one rather than an orthographic one.
Bengali script genuinely *is* used for Sanskrit in Bengal; `√দৃশ্` is not wrong
the way a Devanagari citation would be wrong in a book that loads no Devanagari
font. But a citation form set in Bengali script puts letters in front of a reader
who has not been taught them, and `measureScriptClosure` counts that, correctly.
The IAST was already sitting in italics beside every one of these forms, so
dropping the Bengali-script twin costs the reader nothing.

Fifty-four forms across thirty-two lessons: twenty-one Sanskrit roots (*√kṛ*,
*√dṛś*, *√bhū*, *√jñā*, …), the Sanskrit and Prakrit word-forms cited as
ancestors (*asmi*, *bhadra*, *bhāvayati*, *dugdha*, *bhrātṛ*, *cakṣus*, *cakhu*,
*coukh*, *karpaṭa*, *śāṭī*), the Persian *chashm*, and the prefixes *saha-* and
*su-*. A Bengali **word** stays in Bengali script even where its Sanskrit
ancestor is spelled identically — হৃদয়, মুখ, দয়া, নীল, ভগিনী, ক্ষমা, শ্বেত — because
the rule is about which language a form is being *cited as*, not how it looks.

### Fixed — script closure 41 → 21, and the two levers measured apart

The tranche ran both levers and measured each on its own, because the corpus has
five other tracks with the same citation pattern and they deserve a number rather
than a repeat of the experiment:

- the orthography change **alone** clears **6** of the 41 lessons;
- the eight new letters **alone** clear **9**;
- **5** more clear only when both are done — the lesson's untaught set had one
  glyph from each lever.

That is **41 → 21**, and corpus-wide 518 → 498. `headwordsWithoutRomanization`
stays 0. Bengali's cross-chapter prose references stay at their ceiling of 47
after all 139 lessons were remapped for the new chapter numbering, and forward
references stay at 4.

Six of the twenty-one that remain want **ঞ** or **ষ**, both of which appear in
ordinary Bengali words rather than citations (জিজ্ঞাসা, ওষুধ, ক্ষমা) and so are the
next inventory to buy.

### Added — chapter 26, nine lessons that teach nothing

A `SCRIPT-RECOG` atom used to be taught, called back once, and abandoned. The
three reading chapters were the first answer; this is the second, and it copies
the shape Gujarati proved.

Nine lessons, **zero new atoms**, each returning material **98–104 lesson
positions** back — the R4 window, reached deliberately rather than by accident.
The hand work sits in a detachable `Writing — from sound` block, so the whole
chapter has a **voice core** and can be driven by ear; that is the shape Urdu and
Sanskrit lost sixteen and sixty-one lessons of hands-free reach by not using.

- retrieval misses at **R4: 26 → 14**
- atoms never revisited: **9 → 5**
- chapter-prefix reach (hands-free): **70 → 79 lessons**, and chapter 26 is
  startable by ear
- lessons rescued by a detachable writing segment: 55 → 64

R1 34 → 38, R2 70 → 85 and R3 80 → 88 all grew, and that is reported rather than
hidden: fifteen new atoms open fifteen new sets of windows, and a return inside
R2 (5–15 lessons) or R3 (20–60) cannot be manufactured from the end of the book.
Two honest R2 returns were added where the prose could carry them — আমি-read and
কেমন-read now read হ্যাঁ back, seven and fourteen lessons after it was taught.

### Changed — chapter numbering, 22 chapters to 26

Chapters 1–8 keep their numbers. Old 9–18 shift by one (the split of chapter 8
and the new chapter 10), old 19–22 by two (the new chapter 21). All 139 lessons,
the chapter ledger, the book targets, the generated LaTeX, narration, modality
and the gentle-ramp snapshots were regenerated rather than hand-edited. Chapter
17 lost its "No New Pieces" title, which stopped being true when it took ড.

### Changed — the script strand is now interleaved, not appended (HL-C194)

**No lesson was added, removed, or renamed.** All 115 lessons are the same 115
lessons; what changed is where they sit in reading order, and the book grew from
19 chapters to 22 without gaining a page of new material.

The forty-five script lessons used to sit in one block, chapters 16–19, *after*
all fifteen content chapters. `measureScriptClosure` walks lessons in reading
order, so a glyph taught in chapter 17 was untaught for every chapter 1–15 lesson
that showed it, and the violation count sat at **65** no matter how many letters
the strand taught. The previous tranche measured that directly: adding
thirty-five script lessons moved `neverTaughtGlyphs` 39 → 22 and moved closure
violations 65 → 65.

The strand is now **seven chapters**, each landing after the speech chapter whose
words it hands to the eye:

| chapter | pieces added | words read back |
|---|---|---|
| 2 | ন া আ হ ম স ক ্ র | নমস্কার |
| 4 | ি ই ত ু | নাম, তুমি |
| 6 | ল ে এ ো | আমি, কেমন |
| 8 | ব ভ খ প দ ধ চ জ | আবার, ভালো |
| 15 | — | চা, জল, দুধ |
| 17 | — | পরিবার, ভাই |
| 21 | ী | মুখ, লাল, কালো, সাদা, নীল, তিন, এক |

**Gloss-first survived the move, and constrained it.** A reading lesson may only
read a word the learner has already met romanized, so the placement of every
`*-read` lesson is pinned by its content chapter rather than chosen. That is why
চা, জল and দুধ cannot move above chapter 15 and why the colours cannot move above
chapter 21 — and why the resequence had to reorder the ladder *within* the strand
rather than simply lift the block forward. Eight letter lessons were re-pointed
along the chain (ত, ু, ব, প, দ, ধ, চ, জ), and two independent vowels — **ই** and
**এ** — were moved up beside the signs they twin (**ি** in chapter 4, **ে** in
chapter 6), which is the pattern **আ**/**া** already set in chapter 2.

**Closure: 65 → 41.** The resequence alone takes it to 53. The remaining twelve
come from the second half of the same rule: twenty-five chapter 1–9 headwords
carried their romanization only in prose, never in the `romanization` field, so
`measureScriptClosure` read them as load-bearing decodes rather than as the
gloss-first exposure they have always been. Every one of them now declares it.
`headwordsWithoutRomanization` **25 → 0**.

The forty-one that remain all show one of the twenty-two shapes this track still
never teaches. No resequencing can reach them; `BACKLOG.d` records what would.

**Script atoms now get spaced returns.** Under the old shape a `SCRIPT-RECOG`
atom appeared in two lessons — taught, called back once, abandoned. The three
reading chapters at 15, 17 and 21 exist to return to letters bought in chapter 8
and earlier, several chapters after the hand learned them; `BN-W04-ja` closes
chapter 8 by writing all eight of its consonants back from their places of
articulation, and `BN-W04-ek-read` closes the book's script strand by counting
twenty-six pieces and nineteen words.

### Fixed — three false forward-review claims

- `BN-C02-alaap` now records the earlier `BN-C02-amar-naam` lesson that its
  warm-up and knowledge directives actually rehearse.
- `BN-C02-ki` now records its real name-word review instead of pointing ahead
  to the not-yet-taught pronoun lesson.
- `BN-C05-kaj-kora` no longer claims to review the following `thaka` lesson;
  its exercises revisit `BN-C05-bola` exactly as the knowledge ledger says.

The authored reading order does not change. Bengali's order-integrity debt
falls from three false forward reviews to zero, so the five-minute ramp now
describes what the learner actually encounters.

### Added — Chapters 17–19, seventeen more pieces and eighteen readable words (HL-C194)

Thirty-five lessons in three chapters. **Seventeen teach one piece each; eighteen
introduce no new shape at all** and hand back a word the reader has been saying
since chapters 1–15.

`scriptLessons` 10 → 45, `taughtGlyphs` 9 → 26, `neverTaughtGlyphs` **39 → 22**.
Bengali was the worst never-taught glyph count in the corpus and is no longer in
the bottom four.

**Gloss-first, then glyph-by-glyph, interleaved.** No chapter here is a block of
alphabet. A piece arrives, the next lesson spends it on a word already in the
mouth, and the piece after that waits until something has come between — so
**ি** is taught, spent on আমি, and only then does **ল** arrive. Every one of the
eighteen words was romanized and spoken in an earlier chapter before it was ever
shown as a shape to decode.

**Nothing untaught is ever printed.** A script lesson is credited by
`measureScriptClosure` with teaching *every* target-script glyph in its body, so
these chapters were written against a cumulative allow-list and checked
mechanically: no lesson shows a Bengali glyph that an earlier lesson has not
taught. That guard caught a first draft of `BN-W02-lal-read` quoting **কাল দেখা
হবে** — four untaught shapes — which is now romanized instead.

**Signs before letters, and why.** Three of chapter 17's five additions are vowel
signs, because a sign multiplies where a consonant adds: every consonant already
held can take every sign. Chapter 18 then does the plain-and-breathy square
(**ব**/**ভ**, **ক**/**খ**, **ত**/**দ**/**ধ**), which is the one distinction an
English-speaking reader has no habit for at all. Chapter 19 closes the machinery
by giving three vowels both of their bodies — **আ**/**া**, **ই**/**ি**,
**এ**/**ে** — so the choice between a sign and a full letter becomes predictable
rather than three separate facts.

**The closure violations did not move, and that is placement, not pedagogy.**
`scriptClosureViolations` stays at 65 because the measurement walks lessons in
reading order and these chapters sit after all fifteen content chapters. Replaying
the measurement over hypothetical orderings put numbers on the alternatives:
relocating chapter 16 alone buys three violations, while interleaving all
twenty-six pieces across chapters 1–6 buys seventeen. That restructure is filed as
HL-C194 in `BACKLOG.d/`, together with the two obstacles it has to plan around —
the `language-ladder` test that pins Bengali chapter 6, and the payoff
representativeness floor.

**What this costs, stated rather than buried.** Thirty-five writing lessons
cannot be done hands-free, so Bengali's drivable share falls from 88% to 61% —
the price every script-teaching track pays, and Bengali now sits between Tamil
and Chinese rather than above them. The chapter-prefix reachable count is
unchanged at 70: no existing chapter lost its ear-drivable opening. Distinct
pre-A1 headwords are unchanged at 46, because these lessons re-read words the
track already teaches rather than adding vocabulary — `vocabularyOf()` absorbs
the duplicate headwords, which is the intended behaviour here and not a
shortfall being hidden.

Reinforcement did not regress. The three chapter payoffs would each have been
retrieved fewer than twice, so `BN-W02-kemon-read` now reads the greeting back
alongside its own six words, `BN-W03-kalo-read` re-reads কেমন, and
`BN-W04-cha` declares the breathy dental its warm-up was already asking for.
Under-revisited pre-A1 atoms stay at 13, and `BN-SCRIPT-NOMOSHKAR-READ-01` —
which had **no** retrieval anywhere before this tranche — now has two.

The forced nineteen-chapter build is warning-free: **194 pages, zero
`Missing character`, zero over/underfull boxes, zero LaTeX warnings.** All
seventeen new shapes render from the vendored Noto Sans Bengali font with no
preamble change.

### Added — Chapter 16, the first nine pieces of the script (HL-C222)

Ten lessons. **Nine teach one piece each; one introduces nothing** and assembles
the greeting from pieces the reader can already write.

`scriptLessons` 0 → 10, `taughtGlyphs` 0 → 9, `neverTaughtGlyphs` **48 → 39**.

**The inherent vowel is not *a*.** It is **ɔ**, the vowel of English *awe*. A bare
Bengali consonant says *kɔ* where a bare Devanagari one says *ka*, and that single
default is most of why Bengali does not sound like Hindi read aloud. It is taught
on the very first shape, because every letter after it inherits the difference.

**One letter is written *s* and said *sh*.** স descends from the Sanskrit *s* and
every transliteration writes it that way; Bengali normally pronounces it *sh*. The
spelling records the ancestry, the sound records what Bengali did afterwards, and
both are true — the same way English keeps *knight* and *through*.

The four abugida ideas are Marathi's, in Marathi's order, because the greeting
carries a conjunct and the virama lesson therefore has somewhere to land. Bengali
calls that mark the **hasanta**, and its conjuncts fuse more thoroughly than
Devanagari's — the principle is unchanged, the shapes take a moment longer to take
apart.


## Chapters 13–15 — the second pre-A1 noun tranche — 2026-08-12

- Authored **thirteen** schema-v2 lessons in **three** chapters, continuing
  the cross-track pre-A1 vocabulary program's second round and confirming
  the same measured mechanism on Bengali a second time:
  `vocabularyOf()` counts distinct `headword:` strings 1:1 with lessons, so
  thirteen new word lessons move Bengali's pre-A1 vocabulary by exactly
  thirteen. Measured before/after with the level gate: headwords at or
  below pre-A1 **33 → 46** (shortfall of 300, **267 → 254**); track-wide
  vocabulary (any level) **52 → 65**; `attained`/`inProgressAt` unchanged at
  `null`/`pre-A1` — vocabulary alone still needs roughly 254 more lessons of
  this shape.
- **Confirmed, not found: no pre-A1 spine-node gap.** All seven pre-A1
  spine nodes already had at least one segment before this tranche
  (`SPINE-POLITE-REQUEST-REPAIR` was closed by the prior tranche); the
  level-gate report's only blocker at the start of this tranche was
  `vocabulary`, and `spine-nodes`, `atom-budget` and `reinforcement` were
  all clean at pre-A1. This tranche closed two *universal concepts* instead
  — `COURTESY-PLEASE` and `COURTESY-SORRY` on `SPINE-POLITE-REQUEST-REPAIR`,
  plus `GREETING-WELCOME` on `SPINE-MEET-GREET` — which is a different,
  finer-grained thing than a spine-node gap: the node already had a
  segment (via Chapter 10's polite-offer workaround), but two of its three
  named concepts, and one of `SPINE-MEET-GREET`'s four, had never been
  taught by any actual word. `curriculum.json`'s per-node `omits` ledgers
  are updated to match exactly what `validateCurriculum` computes from
  `concept_tag` matches, not hand-edited to a guess.
- **Chapter 13 — Please, Sorry, and Welcome**
  (`SPINE-POLITE-REQUEST-REPAIR`, `SPINE-MEET-GREET`), 4 lessons, 6 new
  atoms: **দয়া করে** *doya kore* (Sanskrit দয়া *dayā* "compassion," most
  Indo-Europeanists' PIE *\*deh₂-* "to divide" — the same proposed root
  behind Greek *dêmos* and English **democracy**/**epidemic** — plus করা's
  conjunctive-participle shape **করে**, the noun-plus-করা pattern's third
  demonstration) → **দুঃখিত** *dukkhito* (Sanskrit দুঃখিত *duḥkhita*; the
  secure half, দুঃ/*dus-*, is PIE *\*dus-*, the same prefix inside Greek
  **dys-**; the traditional "bad axle-hole" story for the other half, খ, is
  reported as contested rather than settled, per Mayrhofer) → **মাফ করবেন**
  *maf korben* (the identical Arabic-via-Persian loan Hindi's own maaf-based
  "sorry" phrase already uses, this time softened by করা's **future** tense
  rather than a present-habitual command — a third grammatical shape for
  করা in one chapter) → **স্বাগতম** *shbagotom* (the payoff — Sanskrit
  *svāgatam*, *su-* "good" [PIE *\*h₁su-*, cousin of Greek *eu-*] fused by
  ordinary sandhi onto *āgata* "arrived," which opens with the very আ-
  "hither" prefix আসা's own lesson already named, riding √gam this time,
  PIE *\*gʷem-*, as secure a root as this book has shown: English **come**,
  and via Latin *venīre*, **advent** and **convene**).
- **Chapter 14 — Five Colors** (`SPINE-CHECK-WELLBEING`), 5 lessons, 5 new
  atoms: **লাল** *lāl* (a Persian loan, *la'l*, which named a ruby or spinel
  before it named the color — the identical loan Hindi already teaches) →
  **নীল** *nīl* (Sanskrit *nīla*, "dark blue"/indigo, the same root as
  Hindi's *nīlā*, except Bengali marks no gender on it; English "indigo" is
  a separate word, Greek *indikón* "the Indian thing," linked by trade and
  not by descent) → **কালো** *kālo* (built on Sanskrit *kāla*, "time,"
  tied to Kālī and Yama, the same word and the same still-debated
  one-root-or-two question as Hindi's *kālā*) → **সাদা** *shādā* (a tadbhava
  of Sanskrit *śveta*, with a tatsama twin, শ্বেত, alive in compounds — the
  one color in this set where Bengali and Hindi genuinely diverge: Hindi
  replaced its native word with Persian *safed*, Bengali kept its own) →
  **সবুজ** *shôbuj* (the payoff — a second Persian loan, *sabz*, the same
  root behind Hindi and Urdu's *sabzī*, "vegetable"; the chapter's five
  colors split two Persian loans, two Sanskrit words, and one tadbhava, a
  more even mix of inherited and borrowed than food or family showed).
- **Chapter 15 — Cloth, Shirt, Sari, and Glasses** (`SPINE-CHECK-WELLBEING`),
  4 lessons, 4 new atoms: **কাপড়** *kāpoṛ* (Sanskrit *karpaṭa*, "rag" — but
  even Sanskrit's own dictionaries call *karpaṭa* a **deśī** word, homegrown
  rather than inherited from any reconstructed root, and the identical word
  gives Hindi *kapṛā*, Marathi *kapaḍā*, Gujarati *kapaḍũ* and Punjabi
  *kapṛā*, a genuinely pan-Indo-Aryan family) → **জামা** *jāmā* (a Persian
  loan, *jāma*, "robe" — identical to Hindi's *jāmā*, a third Persian loan
  in this small set beside লাল and সবুজ) → **শাড়ি** *shāṛi* (Sanskrit
  *śāṭī*, "a strip of cloth," worn down through Middle Indic — and, unlike
  every other English cousin this track has traced through millennia of
  sound change, this one reached English directly, in the modern era, as
  the ordinary loanword **"sari"**; also introduces **শ**, Bengali's second
  *s*-family letter, distinct in spelling from স though merged with it in
  speech) → **চশমা** *chôshmā* (the payoff — built on Persian *chashm*,
  "eye," the exact word চোখ's own lesson already named as its only cousin
  outside the family, turned into "eye-thing": spectacles).
- **A correction made against this tranche's own brief, not just this
  track's history.** The brief proposed teaching "please"/"sorry" as pure
  vocabulary padding. Checking the spine ledger first showed both concepts
  were genuine, still-open gaps (`COURTESY-PLEASE` and `COURTESY-SORRY` on
  `SPINE-POLITE-REQUEST-REPAIR`'s own `omits` list) — closing them was not
  optional filler but real, previously-unrealized track debt, and is
  reported as such rather than as ordinary vocabulary depth.
- **Etymological corrections made against a first draft, before commit**:
  a first draft of মাফ করবেন rendered the Arabic root *'afw* in a
  synthesized Bengali-script spelling that no dictionary uses; corrected to
  a plain romanization, matching চা's and চোখ's own established convention
  of never inventing non-Latin, non-Bengali script in lesson prose. A first
  draft of লাল also quoted the Persian source word in Perso-Arabic script,
  repeating — inside this very tranche — the exact mistake Chapters 10–12's
  own changelog names as a 91-error incident; caught and converted to a
  romanization before the font check, not after.
- **Reinforcement, closed at pre-A1 even after the track grew past it.**
  Adding thirteen lessons after Chapter 12 extended several older R2/R3
  reinforcement windows from "not yet reachable" (the track was too short
  to judge them) into "reachable and missed," newly exposing seven pre-A1
  atoms the level gate had never previously flagged: হৃদয় and নাক from
  Chapter 12, and five of this tranche's own atoms. All seven are closed
  with a second revisit apiece, threaded into natural recall lines (দয়া
  and হৃদয় share a warmth; সাদা কাপড়, "white cloth," gives সাদা a second
  home) rather than mechanically repeated. The level gate's `reinforcement`
  criterion, which briefly regressed to `7 atom(s) at or below pre-A1
  revisited fewer than twice` during authoring, reports **zero** for
  pre-A1 in the final state — matching the state before this tranche
  began. Five A2-level atoms from Chapter 9 and one A1-level atom from
  Chapter 6 remain thin under the same newly-reachable-window effect; they
  do not block pre-A1 and are left visible as debt for a tranche scoped to
  that level.
- **Wired via both required steps**: `BN-PATH-018`–`BN-PATH-021` path
  segments (`BN-PATH-018` and the pre-existing `BN-PATH-014` both now
  realize `SPINE-POLITE-REQUEST-REPAIR`; `BN-PATH-019` adds a second
  segment to `SPINE-MEET-GREET`; `BN-PATH-020`/`BN-PATH-021` add two more
  to `SPINE-CHECK-WELLBEING`) plus matching `BN-EXT-018`–`BN-EXT-021`
  extensions, `chapters.json`, `core/book-generation.json`, `book/book.tex`,
  and the generated narration. Verified after every edit that all lessons
  remain on a path.
- **Verification**: the forced two-pass XeLaTeX build of the 119-page book
  has zero `Missing character`, zero over/underfull boxes, and zero
  undefined references after the second pass. `npx vitest run
  tests/integration.test.ts tests/cli.test.ts tests/chapter-references.test.ts
  tests/track-progress.test.ts` passes; `check:modality`, `check:books`,
  `check:narration`, `check:figures` and `check:progress` all pass. All
  thirteen new lessons compute well under the 300-second ceiling (declared
  250–295 s, computed 172–291 s). Every table stays at 2–3 columns; no
  lesson trips the sight-cue scanner or the info-dump rule-statement gate.
  The corpus-wide pinned-number tests (chapters, continuity, levels,
  modality-manifest, narration, info-dump, metalanguage, root-ledger,
  chapter-modality-book) shift with any authored content and are left
  failing per standing instruction, for the orchestrator to re-measure once
  after all four wave-6 branches merge.

## Chapters 10–12 — the pre-A1 noun tranche — 2026-08-08

- Authored **twelve** schema-v2 lessons in **three** chapters of four,
  continuing the cross-track pre-A1 vocabulary program (Hindi, Arabic, Tamil,
  German, French, Portuguese, Italian) and confirming the same measured
  mechanism on Bengali: `vocabularyOf()` counts distinct `headword:` strings
  1:1 with lessons, so twelve new word lessons move Bengali's pre-A1
  vocabulary by exactly twelve. Measured before/after with the level gate:
  headwords at or below pre-A1 **21 → 33** (shortfall of 300, **279 → 267**);
  track-wide vocabulary (any level) **40 → 52**; `attained`/`inProgressAt`
  unchanged at `null`/`pre-A1` — vocabulary alone needs roughly 267 more
  lessons of this shape, and nothing about the mechanism makes that cheaper.
- **A second gain, not just vocabulary**: `SPINE-POLITE-REQUEST-REPAIR` was
  the one pre-A1 spine node Bengali had never realized (`"segments": []`),
  and was blocking pre-A1 on its own, independent of the vocabulary
  shortfall. Chapter 10 realizes it — not with a dedicated "please" word,
  which the track does not have, but by reusing Chapter 7's respectful
  imperative of খাওয়া (আপনি খান) as a polite-offer pattern: **চা খান / জল
  খান / দুধ খান / ভাত খান**. The `spine-nodes` blocker is gone from the
  level-gate report; only `vocabulary` remains.
- **Chapter 10 — Tea, Water, Milk, and Rice** (`SPINE-POLITE-REQUEST-REPAIR`),
  4 lessons, 6 new atoms: **চা** *chā* (a Chinese loan via Persian, the
  overland route, unlike the Hokkien-via-Dutch sea route behind English
  *tea*) → **জল** *jôl* (prised out of Chapter 7's `জল খাওয়া`; its own root
  is genuinely disputed, while the Bangladesh-register **পানি** traces
  cleanly to Chapter 7's √pā, "to drink") → **দুধ** *dudh* (√duh, PIE
  *dʰewgʰ-*; the one secure English cousin is **doughty**, not the
  look-alike **dough**, which is a different PIE root entirely — a
  correction against the false lead an earlier track in this program had
  to make explicitly) → **ভাত** *bhāt* (√bhaj "to divide, to share," PIE
  *bʰeh₂g-* — the payoff, and the root Chapter 11 picks back up).
- **Chapter 11 — Friend, Family, Brother, and Sister**
  (`SPINE-EXCHANGE-NAMES`), 4 lessons, 4 new atoms: **বন্ধু** *bôndhu*
  (Sanskrit bandhu kept whole, a **tatsama**; √bandh, PIE *bʰendʰ-* —
  English **bind**/**bond**/**band**, an unusually undisguised cousin) →
  **পরিবার** *pôribār* (a second tatsama, *pari-* + √vṛ, "what surrounds
  you"; *pari-* is the same prefix English itself borrowed as *peri-*, but
  √vṛ's secure cousins are Latin and Lithuanian, not English) → **ভাই**
  *bhāi* (a **tadbhava** this time — worn down through Prakrit rather than
  kept whole — and simply *is* English "brother," PIE *bʰréh₂tēr*, not a
  cousin standing in for it) → **বোন** *bon* (the payoff: not built on PIE
  *swésōr* at all, but on Sanskrit *bhaginī* ← *bhaga*, "a share" — the same
  √bhaj that named Chapter 10's ভাত two lessons earlier. Bengali's "sister"
  and "rice" are cousins; its "sister" and English's are not). Also states
  plainly, and demonstrates on বন্ধু, that **Bengali marks no grammatical
  gender on any of these words** — Chapter 7 already established this for
  verbs; this chapter is where a reader feels it on nouns. A light,
  non-systematic touch on address-term honorificity: দাদা/দিদি as respectful
  terms for non-relatives, named in the ভাই/বোন lessons and not built into a
  system.
- **Chapter 12 — Eye, Mouth, Nose, and Heart** (`SPINE-CHECK-WELLBEING`),
  4 lessons, 4 new atoms: **চোখ** *chokh* (an **ardhatatsama** — half-worn
  through Old/Middle Bengali *cakhu*/*coukh* — √cakṣ, PIE *kʷeḱ-*, "to see";
  no secure English cousin, only Persian *čašm*) → **মুখ** *mukh* (a full
  tatsama with a tadbhava twin, **মু**, alive only inside compounds; its own
  deepest root is a genuine, unresolved Dravidian-vs-Indo-European dispute
  among Sanskritists, reported rather than picked) → **নাক** *nāk* (a
  tadbhava, PIE *néh₂s-*, as secure as etymology gets — the direct ancestor
  of English **nose** and Latin *nāsus*) → **হৃদয়** *hridoy* (the payoff: a
  tatsama with its own tadbhava twin, **হিয়া**, alive in poetry rather than
  speech — the reverse of মুখ's pair — and root হৃদ্, PIE *ḱérd-*, the
  widest confirmed cousin family in the track: English **heart**, Greek
  *kardía*, Latin *cor*).
- A finding worth naming precisely rather than repeating the generic one:
  this program has independently confirmed, in earlier tracks, that the
  seven pre-A1 spine nodes have no concept for a concrete object and that
  household nouns (table, window, key) get shortlisted and dropped. That
  finding does not apply cleanly here in the direction the brief assumed —
  Bengali's own words split roughly evenly between **tatsama** (borrowed
  whole from Sanskrit: বন্ধু, পরিবার, মুখ, হৃদয়) and **tadbhava** (worn down
  by inherited sound change: জল, দুধ, ভাত, ভাই, বোন, নাক), with চোখ as an
  **ardhatatsama** astride both and চা a loan from neither. "Mostly
  tadbhava" undersells how much of this vocabulary Bengali kept unassimilated
  from Sanskrit rather than inheriting.
- **Reinforcement, closed at both cadences.** Each lesson's
  `practises.knowledge` names atoms from the 1–3 lessons immediately before
  it; each chapter's payoff reaches back further. All twelve new atoms are
  revisited at least twice within their window — the level gate's
  `reinforcement` criterion, which briefly flagged three thin atoms
  (`BN-SOUND-C10-DUDH-02` at zero revisits, `BN-LEX-C10-DUDH-01` and
  `BN-LEX-C11-BHAI-01` at one) during authoring, reports **zero** for
  pre-A1 in the final state. The three chapter payoffs also rescue every
  atom the level gate reported as under-reinforced anywhere in the earlier
  corpus: Chapter 6's five numbers atoms (via বোন's sibling count, "āmār ek
  bhāi. āmār dui bon."), three thin Chapter 7 grammar atoms and Chapter 9's
  doubled-letter sound rule (via ভাত's closing recall), and Chapter 8's
  flapped-ড় and causative-আনো atoms (via হৃদয়'s closing recall) — matching
  the discipline the Chapters 8–9 tranche set.
- **Caught and corrected before commit, the FONT CHECK's actual finding**: a
  first pass quoted Sanskrit roots in **Devanagari** script (बन्धु, दुग्ध,
  हृदय, and others), plus Persian, Gurmukhi and Han characters in etymology
  asides — none of which the book's fonts cover, and none of which this
  track's own Chapters 7–9 ever do; their established convention is to
  render every Sanskrit citation in **Bengali script** instead (see
  Chapter 7's জ্ঞা for √jñā). A forced two-pass XeLaTeX compile first
  surfaced 91 `Missing character` errors from this; converting every
  citation to Bengali script and dropping the non-Latin asides (Chinese 茶,
  Persian چا/چشم, Punjabi ਚਾਹ — kept only as romanizations, matching how
  German's Kaffee lesson handles its own loanword route) brought the count
  to **zero**. A stray unmapped romanization character, `ẏ`, introduced for
  য়, was also replaced with the track's existing convention of plain `y`
  (as in *sahāya*, *kolkātāy*). One further overfull `\hbox` in the
  generated পরিবার section heading was cleared by shortening its gloss.
- **Verification**: the forced two-pass XeLaTeX build of the 76-page book
  has zero `Missing character`, zero over/underfull boxes, and zero
  duplicate labels. `npx vitest run tests/integration.test.ts
  tests/cli.test.ts` passes (19/19); `check:modality`, `check:books` and
  `check:narration` all pass. All twelve new lessons compute well under the
  300-second ceiling (declared 255–290 s), and the shared duration report
  measures zero Bengali duration violations. Every table stays at 2 columns;
  no lesson trips the sight-cue scanner. The corpus-wide pinned-number tests
  (chapters, continuity, levels, modality-manifest, narration, ramp) shift
  with any authored content and are left failing per standing instruction.
- Wired via both required steps: `BN-PATH-014`–`BN-PATH-016` path segments
  (attaching to `SPINE-POLITE-REQUEST-REPAIR`, `SPINE-EXCHANGE-NAMES` and
  `SPINE-CHECK-WELLBEING` respectively) plus matching
  `BN-EXT-014`–`BN-EXT-016-LANGUAGE-SPECIFIC` extensions, `chapters.json`,
  `core/book-generation.json`, `book/book.tex`, and the generated narration.
  Verified after every edit that all lessons remain on a path, given this
  track's prior history of an orphaned `curriculum.json` segment.

## Chapters 8 and 9 — the eight-verb tranche — 2026-08-07

- Authored **eight** schema-v2 lessons in **two** chapters of four, realizing the
  canonical `SPINE-SAY-WHAT-I-DO` concepts `VERB-THINK`, `VERB-UNDERSTAND`,
  `VERB-READ`, `VERB-WRITE`, `VERB-TAKE`, `VERB-ASK`, `VERB-HELP` and
  `VERB-LIKE-LOVE`. Each of the eight was taught by exactly **three** tracks
  before this (Spanish, Latin, Portuguese) and is now taught by **four**. Bengali
  goes from **6 of 40** core verbs to **14 of 40**.
- **Chapter 8 — The Mind and the Page**, 4 lessons, **8** new atoms:
  - **ভাবা** *bhābā* — Sanskrit *bhāvayati* is the **causative** of √bhū, which
    is Chapter 7's হওয়া. Thinking is making something be. The gear is still
    live in Bengali as **-আনো**: দেখা → দেখানো "show," খাওয়া → খাওয়ানো "feed."
  - **বোঝা** *bojhā* — √budh "to wake," the root that titled the **Buddha**;
    PIE *\*bʰewdʰ-* → English **bid**, **forbid**. Vowel harmony returns on a new
    vowel, and this time the **spelling moves**: বুঝি against বোঝে. Three
    knowings now stand where English has one — জানা, চেনা, বোঝা.
  - **পড়া** *pôṛā* — √paṭh "to recite aloud." A single intervocalic retroflex
    softens into **ড়**, which is why the letter exists at all; the same
    softening dragged √pat "to fall" onto the identical spelling, and it is the
    **falling** twin that owns **feather**, **petition** and **pterodactyl**.
  - **লেখা** *lekhā* — √likh "to scratch," beside Latin *scrībere* and Germanic
    *wrītan*, both also "scratch": three unrelated roots, one idea, named as
    **convergence and not kinship**. And every **-া** form is simultaneously a
    **noun**, which is what Chapter 4's *dækhā hôbe* had been doing all along.
- **Chapter 9 — Taking, Asking, Helping, Liking**, 4 lessons, **9** new atoms:
  - **নেওয়া** *neowā* — √nī "to lead." Its working life is as the verb that
    **closes a compound**: লিখে নেওয়া "write it down," নিয়ে আসা "bring"
    (Bengali has no separate word for it), নিয়ে যাওয়া "take away."
  - **জিজ্ঞাসা করা** *jijñāsā kôrā* — জিজ্ঞাসা is the Sanskrit **desiderative**
    of √jñā, so asking is literally *wanting to know*: Chapter 7's জানা with an
    appetite, on the same PIE *\*ǵneh₃-* that gives English **know**. And **noun
    + করা** is not a compound but *the* way Bengali makes verbs — the door this
    lesson opens is wider than the word.
  - **সাহায্য করা** *sāhājjo kôrā* — *sahāya*, "a companion," is **সহ-**
    "together" + **√i** "to go"; both cousins are secure (PIE *\*sem-* → **same**,
    Greek *homo-*; PIE *\*h₁ey-* → Latin *īre* → **exit**, **transit**). The
    doubled **য্য** finally demonstrates the word-final **inherent o** that
    Chapter 6 had to admit its five numerals could not show.
  - **ভালো লাগা** *bhālo lāgā* — √lag "to attach." *Āmār bhālo lāge* is "good
    sticks **to me**": the liker is not the subject, the same inversion Spanish
    makes with *gustar*. It wears the identical clothes as আমার … আছে, "I have."
    Set against **ভালোবাসা**, where you *are* the subject — the chapter's payoff
    is the contrast.
- **Honest dead ends, again named rather than papered over**: √nī left no living
  English descendant; √paṭh has no secure Indo-European pedigree past Sanskrit;
  and ভালোবাসা's *bāsā* half has a **disputed** origin, so the commonest proposal
  (√vas "to dwell," which would make English **was** its cousin) is reported as a
  proposal and left open.
- **Reinforcement at two cadences**, which is the point of splitting this into
  two chapters rather than one. Every lesson's `practises.knowledge` names atoms
  from the one to three lessons immediately before it, across the chapter seam;
  the two payoffs reach several chapters back. Measured result: Bengali's
  never-revisited atom count falls from **12 of 18** to **4 of 35** — and all
  twelve of the previously orphaned atoms (Chapter 6's six and Chapter 7's six)
  are now genuinely practised, not merely listed. The four that remain are the
  three introduced by the track's final lesson, which nothing can follow, and
  the doubled **য্য** of সাহায্য, which is recorded rather than claimed.
- Windows closed: **R1** for both দেখা atoms and both জানা atoms; **R2** for all
  six Chapter-6 atoms and all six Chapter-7 pairs. R1 for Chapters 6 and 7 was
  already out of reach — those windows close at reading positions 31–37, which
  are lessons this tranche does not edit — and that residue is left visible.
- Wired into `curriculum.json` (`BN-PATH-012`/`BN-PATH-013`,
  `BN-EXT-012-MIND-AND-PAGE`/`BN-EXT-013-CONJUNCT-VERBS`, and the eight concepts
  struck from the `SPINE-SAY-WHAT-I-DO` omission ledger), `chapters.json`,
  `core/book-generation.json`, `book/book.tex`, and the generated narration.
  All 45 Bengali lessons are on a path; none is orphaned.
- All eight use the canonical **`## The letters in this word`** heading, which
  types as a `script` block. That labels them `sight` and **detachable**, so
  every one has a `voice` core: the track's drivability rises to **98%**, with
  30 lessons rescued for the hands-free view. Every table is 2 or 3 columns; no
  lesson contains a sight cue. Computed durations **256–298 s**, all inside the
  300 s ceiling.
- The forced nine-chapter XeLaTeX build is **warning-free**: 54 pages, zero
  `Missing character`, zero over/underfull boxes. The new conjuncts — **ড়**,
  **জ্ঞ**, **য্য**, **দ্বার** — all render from the vendored Noto Sans Bengali
  with no preamble change.

## Chapter 7 — The Core Verbs — 2026-08-06

- Authored six schema-v2 lessons realizing the canonical `SPINE-SAY-WHAT-I-DO`
  concepts `VERB-BE`, `VERB-GO`, `VERB-COME`, `VERB-EAT`, `VERB-SEE` and
  `VERB-KNOW`. Before this the track realized **no** canonical verb concept: its
  only four verbs (*bôlā*, *thākā*, *kôrā*, *dækhā hôbe*) were all namespaced
  `BN-VERB-*` and none of them was on the shared spine.
- One idea per lesson, each one a thing Bengali does that its neighbours do not:
  - **হওয়া** *hôwā* — Bengali has **two** be-verbs, and আছ- is unfinished: it
    has a present and a past and nothing else, so the future falls to *hôbe* or
    to Chapter 5's থাকা. Root: Sanskrit √bhū, PIE *\*bʰuH-* → English **be**,
    **been**, **future**, **physics**.
  - **যাওয়া** *jāwā* — the honorific level lives in the **verb ending**:
    *jāsh* / *jāo* / *jān* for তুই / তুমি / আপনি, and *se jāy* against *tini
    jān* in the third person. Drop the pronoun and the register still stands.
  - **আসা** *āsā* — **no grammatical gender, anywhere**, set against Hindi
    *ātā/ātī*, Marathi *yeto/yete* and Gujarati's *āvyo/āvī* past. Not a
    beginner's simplification the grammar takes back later.
  - **খাওয়া** *khāwā* — Bengali **eats its drinks**: *jôl khāwā*, *chā khāwā*,
    where Hindi keeps *pīnā*. The formal পান করা carries √pā → English
    **potion**, **potable**.
  - **দেখা** *dækhā* — **vowel harmony**: *dekhi* closes where *dækhe* and
    *dækho* stay open, and the spelling দে never moves. Root: Sanskrit √dṛś, PIE
    *\*derḱ-* → Greek *drákōn* → English **dragon**.
  - **জানা** *jānā* — জানা for facts against চেনা for people, the *savoir* /
    *connaître* line English lost. Root: √jñā, PIE *\*ǵneh₃-* → **know**,
    **notice**, **diagnosis**.
- Flagged two dead ends honestly rather than inventing cousins: যাওয়া's PIE
  *\*yeh₂-* has no living English descendant, and খাওয়া's *khād-* has no secure
  Indo-European pedigree outside Indo-Aryan.
- All six derive as **`voice`** — the chapter's `drivablePrefix` is 6, every
  table is two columns, and no lesson leans on a sight cue. Computed durations
  257–281 s, all inside the 300 s ceiling.
- Wired the chapter into `curriculum.json` (`BN-PATH-011`,
  `BN-EXT-011-CORE-VERBS`, and six concepts struck from the
  `SPINE-SAY-WHAT-I-DO` omission ledger), `chapters.json` (payoff
  `BN-C07-jana`, 8/12 introduced atoms = 0.67, above the 0.5 floor),
  `core/book-generation.json`, and `book/book.tex`.
- Gave the book preamble an optional `grammarlens` title — the generator passes
  each lesson's own "Grammar Lens: …" heading through, which the old
  no-argument box could not accept — plus composed glyphs for the PIE palatals
  `ǵ` and `ḱ`. The seven-chapter build is still warning-free, with no missing
  characters and no over/underfull boxes.

## Chapter capability ledger — 2026-08-06

- Added `chapters.json`, the HL05 chapter capability ledger, covering Chapter 6:
  the reader can count *ek, dui, tin, chār, pā̃ch* in Bengali script and say what
  **দুই** kept that Hindi *do* and Marathi *don* flattened away.
- Made `BN-C06-numbers-1-5` the chapter payoff — the chapter's only lesson, and
  its only schema-v2 one. It is typed `production`: the payoff is counting the
  five aloud, then placing *dui* in its family.
- Recorded `SPINE-COUNT-ONE-TO-FIVE` as the chapter's spine node, matching
  `BN-PATH-010` in `curriculum.json`.
- Omitted Chapters 1–5 rather than stubbing them: all 30 of their lessons are
  schema v1 and declare no `practises.knowledge`, so no payoff there could name
  atoms a lesson actually exercises. Their absence is the debt the HL05 gap
  report exists to measure.
- Measured payoff representativeness for Chapter 6 at 6/6 introduced atoms
  (1.00), comfortably above the 0.5 policy floor.

## Book warning cleanup — 2026-08-03

- Kept punctuation outside the Bengali-only font and replaced five duplicate
  recap anchors with stable chapter-qualified labels.
- Preserved Bengali in PDF bookmarks while suppressing the font-only command
  there, and mapped the vendored static font to every requested shape.
- Let short lesson pages end naturally and made the long farewell title
  breakable so the forced six-chapter build has no layout, bookmark, label,
  font, punctuation-glyph, or package warnings.

## Canonical Chapter 6 publication — 2026-08-03

- Migrated the numbers lesson to schema v2 with the shared
  `SPINE-COUNT-ONE-TO-FIVE` can-do node, a 290-second ceiling, and block-level
  knowledge closure.
- Generated the downloadable Chapter 6 from the same lesson AST and source hash
  that Language Ladder loads instead of maintaining a second content copy.
- Preserved Bengali numeral forms, the chandrabindu note, the qualified history
  of *dui*, and bookmark-safe romanization in the generated chapter; the book
  preamble now supplies the shared width-aware table renderer it uses.

## Sub-five-minute remediation — 2026-08-02

- Corrected eleven declared five-minute estimates whose computed durations were
  already between 121 and 290 seconds.
- Preserved every lesson body unchanged; no split or content reduction was
  necessary. The shared report now measures zero Bengali duration violations.
- The 290-second numbers lesson is the tightest Bengali budget and should be
  watched during later copy edits.

## Chapter 6 — Numbers 1–5, and the conservative "two"

- **Chapter 6 authored** (`BN-C06-numbers-1-5`): *ek, dui, tin, chār, pā̃ch*
  (using *ek*, not the *êk* of a first draft, which would have introduced a
  diacritic this track never defines — its established mark is **ô**).
- **দুই *dui* is the lesson.** Against Hindi *do* and Marathi *don*, Bengali
  **keeps a trace of the vowel that followed the old cluster**, which is why it
  has two syllables where its neighbours have one.
- **Two absolutes scoped back**, both of which were false as first written:
  - "No modern Indo-Aryan language kept the *dv-* cluster" is true of the
    **everyday numeral** only — *dv-* is alive in words re-borrowed straight from
    Sanskrit, like Hindi *dvār* "door."
  - "The vowel survives **only** here" ignores **Assamese, Odia and Nepali**,
    which all have *dui*. Bengali is unusual only among the four languages this
    chapter compares. (Maithili was in a first draft of that list and removed —
    it has *dū*, not *dui*.)
- **A claim removed rather than repaired.** A first draft said the numbers
  demonstrate Chapter 1's o-leaning inherent vowel. They don't — **এক** opens
  with the independent vowel এ, and none of the five contains a bare
  inherent-vowel syllable. The observation is still mentioned (it's true, and
  `BN-C01` does teach it), but now explicitly as something *not* visible in this
  data, so the learner doesn't go looking for it here.
- The **ঁ** on *pā̃ch* is named as the same **chandrabindu** the Devanagari
  tracks use.

## Chapters 2–5 — Introductions, How-are-you, Farewells, First Verbs

- Four new chapters carry Bengali from Chapter 1 to Chapter 5, matching the
  leading tracks' arc. One word per lesson, atom-first, Bengali script inline;
  every root traced (`lessons/BN-C0{2,3,4,5}-*`, `book/chapters/ch0{2,3,4,5}-*.tex`).
  Concept tags reuse the universal `HL01` taxonomy; verbs namespaced (`BN-VERB-*`).
  Two Bengali distinctives run throughout: the **zero copula** (no "is" in the
  present) and **no grammatical gender at all**.
- **Ch. 2 — Introducing Yourself**: *nām* (← *nāman* → *name*) → *āmār* (no
  gender, unlike *merā/merī*) → *āmār nām …* (the zero copula) → *tumi/āpni* (+
  *tui*: Bengali's three-way "you") → *ki* → *tomār nām ki?* → *ālāp kore bhālo
  lāglo* (*ālāp* ← Sanskrit, a rāga's opening) → practice.
- **Ch. 3 — How Are You**: *kemon* → *tumi kemon āchho?* (the verb *āchhā* — the
  copula returns for state) → *āmi* (← *asmi* → English **am**) → *bhālo* (←
  *bhadra*) → *kono bæpār nā* ("no matter" = you're welcome; *nā* ← PIE *ne) →
  practice.
- **Ch. 4 — Farewells**: *ābār* → *dækhā hôbe* (the impersonal "a seeing will
  happen") → *ābār dækhā hôbe* (the fuller form of Ch.1's *āshi*) → *kāl dækhā
  hôbe* (*kāl* = both tomorrow and yesterday ← *kāla*) → practice.
- **Ch. 5 — First Verbs**: *bôlā* → *āmi bānglā bôli* (*bôngo* → the Ganges
  delta) → *thākā* (to live ← *sthā* → English *stand/stay/state*) → *kāj kôrā*
  (to work; ← √kṛ, the root of *nômoshkar*) → practice. **The verb changes for
  person but never for gender.** Book compiles clean with XeLaTeX (0 missing
  chars, 0 undefined refs).

## Chapter 1 — Greetings (Bengali script taught inline)

- New Bengali track on the HL00 framework — Indo-Aryan, written in the Bengali
  script (vendored Noto Sans Bengali font). One word per lesson, slug ids,
  atom-first, derivations shown, LaTeX book. No reading course: the script is
  taught *inside* each word lesson.
- Chapter 1 (`lessons/BN-C01-*`):
  - **নমস্কার** nômoshkar ("hello/goodbye") — the *same* word as Sanskrit
    *namaskāra*, used to introduce Bengali's fingerprint shifts (*a→ô*, *s→sh*)
    plus the inherent-ô vowel and the স্ক conjunct.
  - **ধন্যবাদ** dhônyobad ("thank you") — Sanskrit *dhanya*+*vāda*; shows *a→ô*
    again and *v→b* (Bengali has no "v").
  - **হ্যাঁ / না** hyã / nā ("yes / no") — the *chandrabindu* nasal; *nā* on PIE
    *ne (English *no/not/none*).
  - **আচ্ছা** āchchhā ("okay / I see") — the standalone vowel-letter আ and the
    চ্ছ conjunct; the conversational workhorse.
  - **আসি** āshi ("I'll be going") — literally "I come," the "promise of return"
    goodbye shared with Tamil and Marathi; Bengali marks no gender on the verb.
  - **practice**.
- The recurring thread: Bengali's one sound-fingerprint (inherent **ô**, *s→sh*,
  *v→b*) that disguises familiar Sanskrit words, taught so the learner can
  un-shift any word back. Script facts documented in the appendix. Book compiles
  clean with XeLaTeX.
