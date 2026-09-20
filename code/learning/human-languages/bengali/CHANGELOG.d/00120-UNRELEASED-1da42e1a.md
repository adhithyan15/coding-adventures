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


