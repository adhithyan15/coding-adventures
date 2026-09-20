## Unreleased — Chapters 67-73: 35 more words on pre-A1 nodes, round four

Telugu's pre-A1 vocabulary criterion moves 151/300 → 186/300. The shortfall falls
149 → 114, a drop of exactly 35 — one per lesson, which is the proof that every
lesson landed on a pre-A1 spine node rather than merely on a good word.

Seven chapters appended after chapter 66, chained from `TE-C66-harvest`, one pre-A1
node per chapter and all seven nodes reused a FOURTH time. There are only seven
pre-A1 nodes in the whole spine and the vocabulary criterion, not node coverage,
is what binds, so a fourth pass over the same seven is the shape the gate asks for.

  67 The House Itself       గోడ కప్పు గడప మెట్టు కిటికీ
  68 Sound and Voice        శబ్దం గొంతు పాట అరుపు కబురు
  69 Tools                  గొడ్డలి రంపం పార సుత్తి కర్ర
  70 Small Creatures        ఎలుక కప్ప దోమ ఉడుత సాలీడు
  71 The Wider Family       మామ అత్త బావ మరదలు అల్లుడు
  72 The Market             అంగడి డబ్బు ధర తూకం బేరం
  73 Tastes                 తీపి పులుపు చేదు కారం వగరు

One new headword per lesson (HL14); reuse of what is already taught is unlimited
and is what makes the ramp gentler. R1's numerator held at 1188 while the
denominator grew 4636 → 4671, so the tight ratio improved 0.2563 → 0.2543 — every
one of the 35 new atoms is re-practised inside the 1-3 lesson window, because each
chapter's second lesson practises the previous chapter's final atom and each payoff
practises all five of its own. The corpus-wide never-revisited share falls 14% to
13% on the same arithmetic: numerator held, denominator grown.

Two chapters carry an argument rather than a list. Chapter 71 is the whole Dravidian
cross-cousin reckoning in five words — మామ and అత్త each cover an uncle and an
in-law because for a long time they were the same person, and బావ and మరదలు are
the pair that reckoning produces. Chapter 73 is the Telugu taste list, and its
payoff వగరు names something English has no everyday word for at all.

### Collision checking: the mechanized class-(d) report ran FIRST

Per HL-C214, the hardest collisions are with a GLOSS rather than a token, and no
substring search reaches them. Both passes were run over the pre-tranche tree
BEFORE any word was chosen, so the report shaped the list instead of invalidating
it. Neither pass is a gate; each is a queue of things to READ.

**Pass one, script tokens.** Every Telugu-script token anywhere in the 454-file
telugu tree: 614 distinct, of which 338 are never any lesson's headword. Those 338
are words the reader has been handed a meaning for without being taught.

**Pass two, romanized forms.** The known limit of pass one is that it sees SCRIPT
only — a word glossed purely in transliteration is invisible to it, which is how
sanskrit's कथा survived its own check. The same three steps were run again over
the romanized surface, restricted to tokens the corpus marks as target-language
(inside `*emphasis*`, or carrying a transliteration diacritic) because everything
else is English and buries the signal: 1,574 marked tokens, 1,354 never any
lesson's declared romanization. The etymology notes were then read by hand as well.

Between them the two passes caught two of the candidates actually under
consideration, and pass two caught one that pass one could not see:

- **కోడలు "a daughter-in-law"** — caught by BOTH passes. `TE-C36-koduku` gives it
  in full, in script and in romanization, inside an etymology note about కొడుకు.
  It was first kept and turned INTO teaching, and the corpus rejected that: teaching
  it as a headword creates a forward reference from `TE-C36-koduku`, 164 lessons
  early, and `forwardReferences` is at its ceiling with zero headroom. Reverted and
  replaced with మరదలు, which is the better word for the chapter anyway — it is
  బావ's exact counterpart and completes the cross-cousin pair.
- **డబ్బు "money"** — invisible to pass one and caught by pass two alone. Its script
  form appears nowhere before this tranche, but `TE-C03-paravaaledu` prints
  *ḍabbu lēdu*, "there's no money", as an example of how లేదు denies existence.
  KEPT and turned into teaching: the lesson opens by saying the reader has already
  used the word once without being told what it was. That is honest and it is the
  precise case pass two exists for.

**Cleared after reading, which is what makes this a report rather than a filter.**

- **ధర "a price"** sits inside అర్ధరాత్రి, but across a morpheme boundary:
  `TE-C17` glosses అర్ధ as "half" and రాత్రి as "night" separately, so no meaning
  was ever handed over for the string in between. The token ధర appears nowhere.
- **కారం "pungency"** sits inside నమస్కారం, and `TE-C01` does gloss the piece —
  but it glosses the SANSKRIT ETYMON *kāra*, "a making". A gloss of an etymon of a
  different word is not a gloss of this one, and the lesson says so out loud.
- **అత్త "an aunt"** romanizes to *atta*, whose only hits in the whole tree are
  inside the English word "attaches". A gloss of an English word is not a collision.
- **సంత "a weekly market"** was DISCARDED rather than cleared: *santa* is a literal
  prefix of *santōṣaṁ* and sits inside వసంత, which is the shape earlier rounds cut
  ఎరువు and కోత for. అంగడి took the slot and is the better village word.
- **మాట "a word"** was DISCARDED. `TE-C05-maatlaadu` glosses it twice, in script and
  in romanization, as "word, speech" inside మాట్లాడు, and `TE-C44-speak` repeats
  it. That is a complete handover in both surfaces, the strongest hit available.
  కబురు took the slot and closes chapter 68 better.
- **నేల, తాడు, పాము and చేప** were all in the first draft and all withdrawn on
  sight of this track's own round-two and round-three changelogs, which record each
  of them as already cut — తాడు twice. Honouring the earlier rulings rather than
  re-litigating them, chapter 67 uses గడప, chapter 69 రంపం, and chapter 70
  ఉడుత and సాలీడు.

Three near-misses were read and KEPT on purpose, because separating them is the
lesson. పాట "a song" differs from the already-taught పాత "old" by one retroflex,
and `TE-C68-song` opens by drawing that line and closes by saying పాత పాట. కప్ప
"a frog" differs from కప్పు "a roof", taught three chapters earlier, by the final
vowel, and `TE-C70-frog` names the pair in its second sentence. గొంతు "a voice"
sits beside the already-taught మెడ "a neck" and says so.

### Script

Every headword is spelled entirely from the 40 Telugu glyphs the track's own
writing lessons teach, and so is every Telugu citation in every body — verified by
recomputing the taught set from the writing lessons with the same rule
`script-closure.ts` uses (24 script lessons, 40 glyphs, matching the report) and
checking all 81 strings the tranche prints against it. The tranche therefore adds
**zero** script-closure violations (telugu holds at 46), **zero** exposure-exempted
glyphs (holds at 158) and **zero** new shown glyphs (holds at 64). `shownGlyphs`
UNCHANGED was the target and it was met.

`forwardReferences` holds at its 501 ceiling, `ruleStatements` at 30, and telugu's
cross-chapter prose references at 46 — no chapter number is named anywhere in the
new prose.

`TE-C66-harvest`, the previous last lesson, was read in full before anything was
appended (HL-C201) and needed NO rewording. Its finality claims are all local: "it
is what this whole chapter has been walking towards" is chapter-scoped and stays
true, and "a good place to take your leave" is about the day and the season, not
about the volume. Nothing in it says the book ends.

### HL-C202 / C203 / C208: whole-tree script purity

A per-WORD sweep over all 510 files finds **zero** mixed-script words, zero Latin
runs carrying an Indic combining mark, and zero unassigned codepoints inside a
word. Classification is by BLOCK RANGE rather than `unicodedata.name` (which raises
on unassigned codepoints) and rather than `\w`/`isalnum` (which exclude combining
marks); Latin Extended Additional is included; U+FEFF is excluded from the Arabic
presentation-forms range so it cannot fake an Arabic+Latin word; Cn is treated as
word-internal; and ASCII apostrophe and hyphen are word boundaries.

The detector was self-tested before its clean result was believed: five synthetic
DIRTY fixtures built from `chr(<codepoint>)` in a script file (never pasted as
literals, per the sanskrit tranche that nearly planted a defect in its own
changelog), seven CLEAN negative controls including a pure-Telugu word carrying a
ZWNJ and a Latin word carrying U+0101, and — the strongest control — a REDISCOVERY
run against `git show` copies of the five defects this changelog already records
under HL-C202, in `TE-C37-noru.md`, `chapters.json` and both `narration/ch37`
files. All five were rediscovered. The BOM control initially FAILED and found a
real hole in the classifier, which is why U+FEFF is now excluded by name.

One finding is REPORTED AND NOT CHANGED, unchanged from the previous round:
`TE-C05-undu` carries U+200C ZWNJ between హైదరాబాద్ and లో, where it correctly
BLOCKS a conjunct. That is deliberate orthography, it predates this tranche, and
it propagates to `narration/ch05.*` and `book/chapters/ch05-first-verbs.tex`. No
typos or other defects were found anywhere in the tree.

### Verification

`npx vitest run` exits 0 at **45 test files / 805 tests passing**, against a
clean-tree baseline of **45 files / 805 tests** measured on this worktree before
any change — same file count, same test count, same exit code.

Three pins moved, each by hand, each an integer, each targeting its own assertion:

- `chapter-modality-book.test.ts` chapter ledger 996 → 1003 (+7, the seven new
  chapters). The `toHaveLength(22)` beside it is the TRACK count and was left alone.
- `continuity.test.ts` `neverRevisitedPercent` 14 → 13 and `missedByWindow.R2`
  3642 → 3677. The R2 move is +35 by construction; the percent move is an
  IMPROVEMENT and a rounding boundary, with the numerator held at 629 while the
  denominator grew 4636 → 4671.
- `modality-manifest.test.ts` corpus summary: `totalLessons` 3357 → 3392,
  `voice` and `drivableLessons` 2429 → 2464, `drivablePrefixTotal` 2181 → 2216,
  `chapterCount` 996 → 1003, `fullyDrivableChapters` 662 → 669, `drivablePercent`
  72 → 73. All 35 new lessons are ear-only and fully drivable, so `sight` and `pen`
  do not move at all and every new chapter contributes its whole 5-lesson prefix.
  The synthetic fixtures earlier in that file were left alone.

`check:books`, `check:narration`, `check:modality` and `check:progress` all exit 0.
All twenty-two books build clean from
`data/scripts/build_all_books.sh` — every track exit 0 with missing, overfull and
underfull all ZERO, 113s wall clock at 8-way parallelism. Telugu is 430 pages.

