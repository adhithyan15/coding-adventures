# Changelog

## Unreleased — source-verified independent short e

- Add independent short **ఎ** from the packaged *Write Telugu Alphabets* 2.6
  tracing guide: movements 1–2 close the compact lower loop, then movement 3
  restarts after one lift and climbs the broad outer arch.
- Fit both pen-down runs to the vendored Noto Sans Telugu outline and replace
  `TE-S124`'s provisional copy-only disclaimer with the sourced order.

## Unreleased — first source-verified handwriting shape

- Add independent short **అ** as the first Telugu inventory row with a cited
  stroke order: four numbered movements grouped into two pen-down runs and one
  lift.
- Fit both runs to the vendored Noto Sans Telugu outline and expose the same
  four-frame filmstrip in Language Ladder.
- Keep every other recognition-only row explicitly unverified; this is the
  start of a handwriting tranche, not a claim that the track is a writing
  course yet.

## Unreleased — a four-minute first greeting (#12245)

- Rewrite `TE-C01-namaskaram` from 314 computed seconds to a four-minute
  effective lesson while retaining the greeting, left-to-right decode,
  respectful gesture, final anusvāra, and `namas` + `kāra` etymology payoff.
- Remove the duplicated script explanation and six-language comparison from the
  learner's first five minutes. Add one finger trace and one guided copy so the
  shorter lesson still begins writing gently.
- Reduce the corpus duration backlog from two lessons to one on this branch.
  Telugu is now at zero; the independently logged Kannada lesson remains
  visible until #12244 lands.

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

## Unreleased — Chapters 60-66: 35 more words on pre-A1 nodes, round three

Telugu's pre-A1 vocabulary criterion moves 116/300 → 151/300. The shortfall falls
184 → 149, a drop of exactly 35 — one per lesson, which is the proof that every
lesson landed on a pre-A1 spine node rather than merely on a good word.

Seven chapters appended after chapter 59, chained from `TE-C59-fire`, one pre-A1
node per chapter and all seven nodes reused a THIRD time. There are only seven
pre-A1 nodes in the whole spine and the vocabulary criterion, not node coverage,
is what binds, so a third pass over the same seven is the shape the gate asks for.

  60 Animals                    గేదె మేక కోడి చీమ పక్షి
  61 In the Kitchen             పెరుగు నెయ్యి నూనె చక్కెర మిరప
  62 Things People Make         దారం సూది చీపురు గొడుగు అద్దం
  63 Hunger, Sleep and Health   ఆకలి నిద్ర దగ్గు రోగం ఆరోగ్యం
  64 Words That Join an Answer  కూడా మాత్రమే అయితే తరువాత ఇంకా
  65 More Good Manners          స్వాగతం మర్యాద మనవి అనుమతి నమ్మకం
  66 The Field                  పొలం వరి గడ్డి కల్లం పంట

One new headword per lesson (HL14); reuse of what is already taught is unlimited
and is what makes the ramp gentler. R1 improves 0.2699 → 0.2677 with its numerator
held at 1130 while the denominator grows 4186 → 4221 — every one of the 35 new
atoms is re-practised inside the 1-3 lesson window, because each chapter's second
lesson practises the previous chapter's final atom and each payoff practises all
five of its own.

Two chapters carry an argument rather than a list. Chapter 63 teaches రోగం and
then ఆరోగ్యం, which is the same word with the Sanskrit "not" on the front, so the
second headword costs one word and explains itself. Chapter 61 runs పాలు → పెరుగు →
నెయ్యి, three things out of one already-taught word.

### Collision checking, in four passes

The track carries 206 existing lessons and two rounds of everyday vocabulary, so
every candidate was checked four ways. About twenty-five were cut.

**(a) Plain prose.** కుండ "a clay pot" is glossed inside `TE-C56-vessel`; నొప్పి "pain"
is already said aloud in `TE-C57-bone`'s practice as *nāku noppi*; చాలా "very" is
glossed in `TE-C49-enough`; ఎప్పుడు "when" is spelled out in `TE-C50-now`; సాయం is
named in `TE-C34-sahayam-ceyu` as the spoken short form of సహాయం.

**(b) Inside a headword.** కూర "a cooked vegetable" sits whole inside `TE-C43-sit`'s
కూర్చో; కుండ sits whole inside `TE-C49-certainly`'s తప్పకుండా as well; మళ్ళీ "again"
is the first half of `TE-C04-malli-kaluddaam`; సాయం sits inside సాయంత్రం.

**(c) Inside a romanization.** ఆవు "a cow" is *āvu* inside *avunu*, and `TE-C03`
discusses *-avu* as the familiar "you" ending; చేప "a fish" is one vowel from
`TE-C56-mat`'s *cāpa*; పాము "a snake" is one letter from *pālu*; ఏనుగు "an
elephant" is *ēnugu*, which sits inside *tenugu*, the proposed origin of the name
Telugu printed in `TE-C05`; మందు "medicine" is one letter from *mañcu*, taught in
the chapter immediately before this tranche; నేల "the ground" is one vowel-length
from *nela*, the month word; ఎద్దు "an ox" is one letter from *vaddu*; మడి from
*nadi*; చలి from *cālu*; కోత sits inside *kotta*; గట్టు collides with *guṭṭa*;
ఎరువు sits inside *ceruvu*; నాగలి "a plough" carries *gāli* inside it.

తాడు "a rope" was WRITTEN and then withdrawn. It passed a script scan cleanly, but
this track's own round-two changelog records తాడు as already dropped for sitting
inside మాట్లాడు, and `TE-C05` does print the conjugated *māṭlāḍatāḍu*. Honouring
the earlier ruling rather than re-litigating it, chapter 62 opens on దారం instead
and గొడుగు takes the free slot.

**(d) Inside a gloss of a morpheme, or a stock phrase.** No string search reaches
these; they came out of READING the track's etymology notes and its writing
lessons. SIX were caught here, more than any other class:

- అక్షరం "a written character" — all twenty-four writing lessons gloss their own
  headword as "the single character X", so the gloss was already spoken for. The
  same case as Sanskrit's शीलम् one tranche ago.
- ఆప్యాయత "affection" — `TE-C35-snehitudu` glosses Sanskrit स्नेह as "affection,
  love".
- బలం "strength" — `TE-C19-vayasu` glosses Sanskrit वयस् as "age, vigor, strength".
  రోగం took the slot, which turned out better: it sets up ఆరోగ్యం.
- మొదట "first, at the start" — `TE-C10-vaaram` glosses ఆది as "the FIRST day",
  from Sanskrit *ādi* "beginning". అయితే took the slot.
- బావి "a well" — `TE-C03-baagaa` glosses బాగా as "well, nicely, very". A headword
  whose English gloss is "a well" walks straight into it. కల్లం, the threshing
  floor, took the slot and suits the chapter better.
- దీవెన "a blessing" and మన్నించు "to pardon" — already the glosses of ఆశీర్వాదం
  and క్షమించండి. పళ్ళెం "a flat plate" is glossed in passing in `TE-C56-vessel`.
  మెల్లగా "slowly" was left alone for a related reason: every guided-practice
  block in the track already says "it once more, slowly".

Three near-misses were read and KEPT, with reasons. మేక against *meḍa* and పంట
against *gaṇṭa* are one-consonant pairs, and this corpus already carries గుట్ట
beside బుట్ట in ADJACENT chapters. వరి's only romanization hits are the English
word "variety" in frontmatter. స్వాగతం does share the English word "welcome" with
పరవాలేదు's gloss, and మర్యాద sits beside గౌరవం — both were kept because separating
them is the lesson, and each opens by drawing the line explicitly.

### Script

Every headword is spelled entirely from the 40 Telugu glyphs the track's own
writing lessons teach, and so is every Telugu citation in every body. Four words
were romanized instead because they need untaught letters: *bhōjanaṁ*, *atithi*,
*hṛdayaṁ* and *gauravaṁ*. The tranche therefore adds **zero** script-closure
violations (telugu holds at 46) and **zero** exposure-exempted glyphs (holds at
158). `forwardReferences` holds at its 511 ceiling, `ruleStatements` at 30, and
telugu's cross-chapter prose references at 46 — no chapter number is named
anywhere in the new prose.

`TE-C59-fire`, the previous last lesson, was read before anything was appended
(HL-C201) and needed no rewording. It closes on "Weather enough to take your
leave on" and makes no claim about the book ending; its "the last thing done
before sleep" is about a household, not about the volume.

### HL-C202: the last three mixed-script defects on this track

Five hits, three distinct sources, all of the LATIN-BASE sub-class — a
romanization wearing an Indic combining mark, which renders as plausible wrong
text and builds at exit 0. Only the codepoint changed in each:

- `lessons/TE-C37-noru.md`, `etymology_hook`: `v` + U+0BBE TAMIL VOWEL SIGN AA +
  `y` → `v` + U+0101 LATIN SMALL LETTER A WITH MACRON + `y`.
- `lessons/TE-C37-noru.md`, body: the same U+0BBE, in the romanization printed
  beside Tamil வாய், → U+0101, giving *vāyi*.
- `chapters.json`, chapter 37's payoff summary: `v` + U+0C3E TELUGU VOWEL SIGN AA
  + `y` → U+0101.

The intended form was verified against the surrounding prose before anything was
touched: the same paragraph already writes *vāy* twice and *bāyi* once with
U+0101, and the guided practice reads "vāy … bāyi … nōru". `narration/ch37.json`
and `narration/ch37.txt` are generated from the lesson and cleared on
regeneration, which the sweep confirms.

A whole-tree per-word script-purity sweep over all 453 files now finds zero
mixed-script words and zero Latin runs carrying an Indic combining mark. The
detector was self-tested against synthetic dirty fixtures built from
`chr(<codepoint>)` in a script file, against four clean negative controls, AND
required to REDISCOVER all five documented defects in `git show HEAD:` copies of
the three sources plus the two narration files before its clean result was
believed. It caught TWO bugs in itself along the way, both worth writing down.
First it reported the LATIN-BASE cases under the generic mixed-script label,
which is why the sub-class is now checked first. Then, drafting this entry,
the tooling wrote U+0BE3 — an UNASSIGNED codepoint in the Tamil block — into a
Latin run, and the sweep called the file clean: `unicodedata.name` raises on an
unassigned codepoint, so the classifier returned "not script", and its category
is Cn, so the word-cutter split the word around it and both halves came out pure
Latin. Both halves now fall back to the block RANGE. A hole a bad write can
aim at is not a hole worth leaving.

One finding is REPORTED AND NOT CHANGED: `TE-C05-undu` contains U+200C ZWNJ between
హైదరాబాద్ and లో, where it correctly stops ద్ and ల forming a conjunct. That is
deliberate Indic orthography rather than a defect, it predates this tranche, and
removing it would change how the word renders. It propagates to
`narration/ch05.*` and `book/chapters/ch05-first-verbs.tex`.

### Verification

`npx vitest run` exits 0 at 804 passing tests, against a clean-tree baseline of
804 passing measured on this worktree before any change. Four corpus pins moved,
each by hand and each targeting its own assertion: the chapter ledger 934 → 941,
`missedByWindow.R2` 3254 → 3289, and the modality manifest's `totalLessons`
3031 → 3066, `voice` 2161 → 2196, `drivableLessons` 2161 → 2196,
`drivablePercent` 71 → 72, `chapterCount` 934 → 941, `drivablePrefixTotal`
1932 → 1967 and `fullyDrivableChapters` 612 → 619. All seven new chapters are
fully drivable. The synthetic fixtures in `modality-manifest.test.ts` were left
alone.

`check:books`, `check:narration`, `check:modality` and `check:progress` all exit
0. The book builds clean with XeLaTeX from a `latexmk -C` start: exit 0, 381
pages, 0 missing characters, 0 overfull, 0 underfull.

## Unreleased — Chapters 53-59: Thirty-five more everyday words, round two

The first tranche took Telugu from 46 pre-A1 headwords to 81. Six other tracks
then did the same thing, and Telugu was last again — 81 against a floor of 300.
These seven chapters answer with thirty-five more, on the same terms: **one new
word per lesson**, and unlimited reuse of everything already taught.

  53 The Sky                 ఆకాశం సూర్యుడు చంద్రుడు చుక్క మబ్బు
  54 The Tree                చెట్టు కొమ్మ వేరు విత్తనం మొక్క
  55 River and Road          నది చెరువు గుట్ట గ్రామం బాట
  56 In the House            చాప బుట్ట కత్తి గిన్నె పెట్టె
  57 More of the Body        మెడ వీపు పెదవి గోరు ఎముక
  58 More Short Replies      కొంచెం ఎక్కువ తక్కువ వద్దు అంతే
  59 Weather and Ground      గాలి మంచు ఇసుక బురద నిప్పు

The seven pre-A1 spine nodes are each used a second time, one per chapter, five
lessons sharing it. Chapter 53 chains from పూలదండ, the garland that closed the
last tranche, and every lesson after it chains to the one before, so an atom
introduced by a chapter's payoff is still being practised two lessons into the
next chapter. The ramp gets gentler again: the R1 reinforcement ratio falls
0.2895 to 0.2869 with the numerator held at 1106 while the corpus grew.

Every headword was checked against all 171 existing lessons before it was
written — in both scripts, in romanization, and inside other words — so none of
the thirty-five re-teaches anything and none adds a forward reference. Around
twenty candidates were cut. కొండ was the sharpest: it is invisible on its own
but sits whole inside పదకొండు, "eleven", so teaching it here would have made the
numbers chapter point forward. తాడు went the same way inside మాట్లాడు, and ఆకు
inside నాకు.

The second filter was the script. Telugu's own writing lessons teach forty
letters and signs, and every one of these thirty-five headwords is spelled
entirely from those forty — so the tranche adds zero script-closure violations
and zero exposure-exempted glyphs. Nothing is laundered through a headword. The
same rule was applied to the prose, which is why a few cousin words appear in
romanization rather than in their own script: *ūru* beside గ్రామం, *bījaṁ*
beside విత్తనం.

Three pairs run through the chapters and are the point of them. పొద్దు is the
sun that rises and సూర్యుడు is the sun that is prayed to; నిప్పు is the coal in
the hearth and అగ్ని is the fire in the ritual; చుక్క is what a child points at
and నక్షత్రం is what an astrologer reads. Each time, the inherited word does the
work and the borrowed one does the ceremony.

Also in this change: the three telugu book targets for chapters 43, 44 and 45
move from a bare `unicodeScript` to `telugu-comparisons`, which closes HL-C200.
The rendered .tex for all three is byte-identical afterwards and their book
hashes do not move.

## Unreleased — Chapters 46-52: Thirty-five everyday words, one per lesson

Telugu was the furthest behind of every track on the pre-A1 vocabulary floor:
46 headwords against the 300 that floor asks for. These seven chapters answer it
with thirty-five, and with nothing else — **one new word per lesson**, and reuse
of everything already taught, unlimited and on purpose.

  46 Things You Ask For             పండు బట్ట దీపం ఉప్పు పుస్తకం
  47 The Leg and the Tooth          కాలు పన్ను జుట్టు వేలు కడుపు
  48 Naming Who Someone Is          గురువు విద్యార్థి వైద్యుడు రైతు అతిథి
  49 Answering With More Than Yes   నిజం చాలు తప్పకుండా ఏమో అలాగే
  50 Taking Your Leave              ఇప్పుడు ఎల్లుండి ప్రయాణం బయలుదేరు వెళ్ళొస్తాను
  51 Courtesy Words                 కృతజ్ఞత మేలు గౌరవం ఆశీర్వాదం దండం
  52 Welcoming a Guest              తలుపు కుర్చీ ముగ్గు పువ్వు పూలదండ

Each chapter sits on one of the seven pre-A1 spine nodes, five lessons sharing
it, and each lesson chains to the one before — so a word introduced by a
chapter's payoff lesson is still being practised two lessons into the next
chapter. That is why the ramp got *gentler* rather than steeper: the R1
reinforcement ratio falls 0.3094 to 0.3064 even though the corpus grew.

Every headword was checked against the whole track before it was written, so
none of the thirty-five re-teaches anything and none of them adds a forward
reference. Two candidates were cut for exactly that reason — కాదు and దయ are
already used in chapters 1 and 8, and teaching them here would have made those
earlier pages point forward.

Chapters 51 and 52 share a word on purpose: the దండ inside దండం, the salutation,
is the దండ inside పూలదండ, the garland. Both are a straight line.

## Unreleased — Chapter 41: Pointing, and Asking

Six words and the pattern behind them: ఇది అది ఇక్కడ అక్కడ ఎవరు ఎక్కడ

Until now the reader could NAME things and not point at them. With these they
can: *this one*, *that one*, *here*, *there*, and the two questions that matter
first — *who?* and *where?* Everything already in the book becomes a sentence
they can use.

The seventh lesson is the reason these six are one chapter. They are not six
words, they are **i- / a- / e-** — three beginnings on the same ending, and changing
the front walks the meaning from near to far to a question. A reader who sees
that once does not have to be taught the third member of the next family they
meet; they will work it out.

The whole chapter is **voice**: nothing in it needs eyes, so it is learnable end
to end at the wheel.

## Unreleased — the first 8 characters this book actually teaches

8 recognition segments, one character each, in chapters 6-13: త ◌ు క ◌ం ◌్ ర ◌ా ◌ి

Until now this track taught **no letters at all**. Every word was printed in its
own script and the reader had no way in — HL12's measurement put the track at A2
by aspiration and pre-A1 by attainment, with the script strand simply missing.

Each segment names one character, says what it carries, and shows it inside four
words the reader **already says** — so nothing new has to be learned in order to
do the recognising. That is HL12 §2.1's rule made concrete: a lesson may sit at
the frontier of decoding or of meaning, never both, because a reader who fails
one that is new in both cannot tell which one they failed.

They teach recognition and not writing, and that is a sourcing fact rather than a
pedagogical preference. This script has **no cited stroke order** in the corpus —
its own script file says *"Recognition only"* — and HL11 §5 forbids a pen path
without one, because a learner cannot tell an invented stroke order from an
attested one and will drill it for years. So the reader is asked to trace the
printed shape, which needs no source, and the book says plainly that where to
start the character and which way to travel are not written down yet.

Each segment sits **last** in its chapter, after every word in that chapter that
contains its character — so it consolidates rather than pre-teaches, and it costs
the driving edition nothing: `drivablePrefixTotal` is unchanged corpus-wide.

## Unreleased — 36 words a reader can now say

Added `romanization` to 36 lessons that had none, so their headwords become
HL11 *exposure* — something the reader is shown and can use — rather than script
they are stuck on. Each is recovered from the pronunciation the lesson already
gives in its own prose, then checked against the headword's script so a wrong
grab cannot pass. Nothing is transliterated: a mechanical romanization of this
script disagrees with its own authors often enough to teach mispronunciations.

## Chapters 35–40: pre-A1 vocabulary depth — family, face, heart, tea and a meal (2026-08-08)

- **Telugu's pre-A1 vocabulary count was 33 distinct headwords against a 300-word
  target, not because the track lacked content but because its earlier family,
  body, and food chapters taught several words per lesson under one shared
  `headword:` field.** `vocabularyOf()` counts distinct headword strings 1:1 with
  lessons, so six family words in `TE-C12-kutumbam` counted as **one**. This
  tranche adds thirteen new one-headword-per-lesson lessons; pre-A1 vocabulary
  moves from **33 to 46**, and the track's total (any level) from **66 to 79**.
- **Telugu's seven pre-A1 spine nodes were already fully realized before this
  tranche** — `SPINE-MEET-GREET`, `SPINE-COURTESY-THANK`, `SPINE-RESPOND-BASIC`,
  `SPINE-EXCHANGE-NAMES`, `SPINE-CHECK-WELLBEING`, `SPINE-POLITE-REQUEST-REPAIR`,
  and `SPINE-TAKE-LEAVE` all have segments in `curriculum.json`, and the level
  gate reports no `spine-nodes` blocker for Telugu at pre-A1. There was no
  spine-node gap to close; this tranche is pure vocabulary depth.
- Adds **Chapter 35 — Family and Friends** (`TE-PATH-028`, sequences 750–760):
  `TE-C35-kutumbam` (కుటుంబం, the Sanskrit *tatsama* collective noun none of
  Chapter 12's six specific-person words supplied) and `TE-C35-snehitudu`
  (స్నేహితుడు, the everyday Sanskrit-derived word for "friend," set beside the
  native DEDR-attested నేస్తం, *nēstaṁ*).
- Adds **Chapter 36 — Son and Daughter, Two Different Roots** (`TE-PATH-029`,
  sequences 770–780): `TE-C36-koduku` (కొడుకు, "son," Proto-Dravidian *\*kōẓ-*,
  cognate not with "daughter" but with కోడలు, "daughter-in-law") and
  `TE-C36-kuuturu` (కూతురు, "daughter," a wholly separate Proto-Dravidian root
  *\*kūnttu*, cognate with Kannada ಕೂಸು *kūsu*, "infant, maiden"). Telugu does
  **not** build son/daughter on one shared root the way Kannada's
  *magu/maga/magalu* does — the honest, Telugu-specific finding, corrected from
  this brief's working assumption that the Dravidian family tranches would share
  that structure.
- Adds **Chapter 37 — Four Words on the Face** (`TE-PATH-030`, sequences
  790–820): `TE-C37-kannu` (కన్ను, "eye" — closing a loop Chapter 32's *cūḍu*
  lesson left open, having named the word without ever teaching it),
  `TE-C37-cevi` (చెవి, "ear," a near-exact match with Tamil செவி), `TE-C37-mukku`
  (ముక్కు, "nose," a regular Proto-Dravidian reflex flagged irregular in the
  comparative record, likely contaminated by the separate root *\*mok-*, "face"),
  and `TE-C37-noru` (నోరు, "mouth" — the one face word with **no** confirmed
  source for its own root; unlike Tamil, Malayalam, and Kannada, which all share
  one *vāy*/*bāyi* root, నోరు shows no visible trace of it, and nothing is
  invented to fill the gap).
- Adds **Chapter 38 — The Heart, Two Ways** (`TE-PATH-031`, sequences 830–840):
  `TE-C38-gunde` (గుండె, the everyday native word, Proto-Dravidian *\*kuṇṭV*,
  cognate with Kannada's own native ಗುಂಡಿಗೆ *guṇḍige*) and `TE-C38-hrudayam`
  (హృదయం, the literary Sanskrit loan, a genuine Proto-Indo-European cousin of
  English *heart*, Latin *cor*/*cordis*, and Greek *kardía*, all from PIE
  *\*ḱērd-*).
- Adds **Chapter 39 — Tea and Milk, Two Roads** (`TE-PATH-032`, sequences
  850–860): `TE-C39-tii` (టీ, "tea" — a direct English loan whose own root
  travelled by **sea**, Hokkien *tê* through Malay *teh*, the opposite road from
  the **overland** *chai* — Mandarin *chá* through Persian *chāy* — that Hindi,
  Kannada, and Marathi carry) and `TE-C39-paalu` (పాలు, "milk," Proto-Dravidian
  *\*pāl* unchanged, matching Tamil *pāl* and Malayalam *pāl* exactly; here
  Kannada ಹಾಲು *hālu* is the odd one out, via its own regular *p*-to-*h* law,
  not Telugu).
- Adds **Chapter 40 — A Meal** (`TE-PATH-033`, sequence 870): `TE-C40-bhojanam`
  (భోజనం, "a meal" — Sanskrit *bhoja/bhuj*, "to enjoy, to eat," a word Chapter
  32's *tinu* lesson already named as native తిండి's polite counterpart without
  ever teaching it; this lesson closes that second loop the way Chapter 37's
  కన్ను closed the first).
- **Reinforcement discipline, both directions.** Every new lesson's
  `practises.knowledge` reaches back into the preceding one to three lessons
  (closing R1/R2 windows), and each chapter's last lesson closes its own
  chapter's atoms for the HL05 payoff. Seven atoms that were revisited fewer
  than twice — `TE-LEX-C08-DAYACHESI-01`, `TE-PRAGMATICS-C08-DAYACHESI-03`,
  `TE-SCRIPT-C08-DAYACHESI-04` (rescued in `TE-C39-tii`, the natural "please"
  callback for a drink request), `TE-LEX-C12-KUTUMBAM-01` (rescued in
  `TE-C35-kutumbam`), `TE-LEX-C13-SHARIRA-BHAGALU-01` (rescued in
  `TE-C37-kannu`), `TE-GRAMMAR-C19-VAYASU-02` (rescued in `TE-C36-koduku`), and
  `TE-PRAGMATICS-C09-KSHAMINCHANDI-03` (rescued in `TE-C40-bhojanam`, apologising
  for a late meal) — are now revisited at least twice, distributed thematically
  across the tranche rather than dumped in a single capstone lesson. A second
  pass caught six of this tranche's **own** new atoms sitting at only one
  revisit each (`TE-LEX-C35-KUTUMBAM-01`, `TE-LEX-C35-SNEHITUDU-01`,
  `TE-LEX-C36-KODUKU-01`, `TE-LEX-C36-KUUTURU-01`, `TE-LEX-C37-MUKKU-01`,
  `TE-LEX-C37-NORU-01`); each now gets a second reach-back one lesson further
  down the chain. Telugu's pre-A1 reinforcement blocker is now **fully
  closed** — zero atoms at or below pre-A1 are revisited fewer than twice.
- **Font/etymology corrections made against this brief's assumptions.** Tamil's
  word for "buttocks/pit," குண்டி (*kuṇṭi*), was dropped from the heart
  etymology note rather than cited as a semantic cognate of గుండె, because its
  modern sense could not be verified as the Wiktionary-cited Proto-Dravidian
  cognate's original meaning — the lesson cites only the safely attested Telugu/
  Kannada pair instead. Chapter 38's vocalic-*r* note distinguishes the
  *independent* letter ఋ (Chapter 14's ఋతువు) from the *dependent* vowel sign
  ృ (హృదయం), rather than calling them "the same mark." No English
  Proto-Indo-European cousin is claimed for భోజనం (Sanskrit *bhuj*) — that
  etymology is left at its Sanskrit root, the same discipline the corpus already
  applies to అర్థం.
- Wiring: six new `curriculum.json` path segments (`TE-PATH-028`–`TE-PATH-033`)
  on `SPINE-EXCHANGE-NAMES` (×2), `SPINE-CHECK-WELLBEING` (×2), and
  `SPINE-POLITE-REQUEST-REPAIR` (×2), each with a matching required
  `TE-EXT-029`–`TE-EXT-034` extension; the three spine nodes' `segments` ledgers
  updated to match (their `omits` ledgers are unchanged, since every new
  `concept_tag` is Telugu-namespaced and matches no canonical HL01 concept — the
  same convention Chapters 12/13/15/19 already use). Six new `chapters.json`
  entries, each with a `canDo`, a `payoff`, and an `assesses` list copied
  verbatim from its payoff lesson's own `practises.knowledge`. Six new
  `core/book-generation.json` targets and six new `book.tex` inputs.
- **Chapter 8's pre-existing atom-budget violation (`TE-C08-dayachesi`, 4 atoms
  against a 3 budget) is untouched** — it predates this tranche and sits outside
  its scope; fixing it was explicitly out of scope for this pass.
- **Verified.** `npm run build` clean. `check:modality`, `check:books`, and
  `check:narration` all clean after regeneration. `npx vitest run
  tests/integration.test.ts tests/cli.test.ts` — 19/19 green. The HL05 chapter
  gates report zero findings for chapters 35–40. The book compiles under
  XeLaTeX to 150 pages with **zero** `Missing character` lines and no undefined
  references; a stray romanization typo (Telugu glyphs bleeding into an
  italicized romanization for స్నేహితురాలు) was caught by that compile and
  fixed. Build artifacts removed before commit.
- **Corpus-snapshot pins in `modality-manifest`, `levels`, `chapters`,
  `continuity`, `narration`, and `ramp` tests are DELIBERATELY left failing.**
  This is wave 5 of the pre-A1 vocabulary program; Telugu was authored on its
  own branch in parallel with the other wave-5 tracks, and only the merged
  numbers are the real ones — re-pinning here alone would repeat the mistake
  the verbs test's own comment already records.

## Chapters 33–34: the eight verbs eleven other tracks teach (2026-08-07)

- **Eleven tracks taught VERB-THINK, VERB-UNDERSTAND, VERB-READ, VERB-WRITE,
  VERB-TAKE, VERB-ASK, VERB-HELP and VERB-LIKE-LOVE. Telugu taught none of
  them**, sitting at 6 of the canonical 40 after Chapter 32. Each of these eight
  lessons widens an eleven-way cross-language join to twelve, and Telugu is the
  second Dravidian contributor after Tamil.
- Adds **Chapter 33 — Four Verbs of the Mind** (sequences 670–700, schema v2):
  `TE-C33-anuko` (అనుకో, `VERB-THINK`), `TE-C33-artham-cesuko` (అర్థం చేసుకో,
  `VERB-UNDERSTAND`), `TE-C33-caduvu` (చదువు, `VERB-READ`), `TE-C33-raayu`
  (రాయు, `VERB-WRITE`). Ten new atoms.
- Adds **Chapter 34 — Four Verbs Between People** (sequences 710–740, schema
  v2): `TE-C34-tiisuko` (తీసుకో, `VERB-TAKE`), `TE-C34-adugu` (అడుగు,
  `VERB-ASK`), `TE-C34-sahayam-ceyu` (సహాయం చేయు, `VERB-HELP`),
  `TE-C34-ishtam` (నాకు తెలుగు ఇష్టం, `VERB-LIKE-LOVE`). Ten new atoms. Telugu
  now covers **14 of the core 40**.
- **The reflexive -కొను is the tranche's spine, and it makes three verbs
  transparent.** *anukō* is **అను** ("to say", Proto-Dravidian *\*aHn-*, kept by
  Tamil *eṉ*, Kannada *annu*, Malayalam *ennuka*) plus the ending that hands an
  action back to its doer — so Telugu's native word for thinking is literally
  *saying it to yourself*. *arthaṁ cēsukō* is the Sanskrit noun on the native
  *cēyu* wearing the same ending: *make the meaning your own*. *tīsukō* is
  *tīyu* ("pull out") plus that ending, and its source verb **కొను** is the
  family's, with Tamil **கொள்** and Malayalam **കൊള്ളുക** using it as an ending
  in the same way (Wiktionary; Dravidian reflexive/self-benefactive *koḷ*).
- **Why Telugu keeps going its own way, finally named.** Chapters 1, 7 and 32
  recorded the symptom — *lēdu* against the sisters' *il-*, *uṇḍu* against their
  *iru*, *enimidi* and *tommidi* which will not derive from *eṭṭu*/*oṉpatu*.
  `TE-C34-adugu` names the cause: **Telugu is South-Central Dravidian**, with
  Gondi, Konda, Kui, Kuvi, Pengo and Manda, where Tamil, Kannada and Malayalam
  are South Dravidian. The hook is DEDR 81, whose firmest match for *aḍugu* is
  not in any sister but in **Parji** *aḍ-*.
- **Two Indo-European cousin-webs, both attested, both in English.** *sahāya* =
  **सह** ("with", PIE *\*sm̥dʰé* — the "one, together" root behind *same* and
  *similar*) + **आय** ("a going", the go-root Latin carries as *īre*, English
  *exit* and *transit*). And **ఇష్ట** is the past participle of **इष्**, from
  PIE *\*h₂eys-*, whose plain English descendant is the verb **ask** — so the
  chapter carries two *different* asking-roots out of two Sanskrit words,
  *prach* behind *pray* and *iṣ* behind *ask*, and Telugu's own *aḍugu* is
  cousin to neither.
- **నాకు తెలుగు ఇష్టం is a sentence with no verb in it**, and the fourth
  dative-subject shape the track has built (after *nāku telugu vaccu*, *nāku
  telusu* and *nāku arthamaindi*). Beside it the **native** నచ్చు (cognate Tamil
  *naccu*, Kannada *naccu*) and ఇష్టపడు, which is *iṣṭaṁ* plus **పడు** — the
  falling of Chapter 20's *varṣaṁ paḍutōndi*. Across the corpus this is the
  **sixth** language to build liking backwards, after Spanish *gustar*, Italian
  *piacere*, Hindi *pasand*, Bengali *bhālo lāgā* and Tamil *piḍikkum* — three
  unrelated families, one design.
- **Reach-back at two cadences, and the numbers moved.** Every lesson practises
  atoms from the immediately preceding one to three lessons, across the 33/34
  chapter seam; each payoff reaches several chapters back. Thirteen atoms that
  had never been revisited at any distance are now practised — the three
  dative-subject atoms of Chapter 6, both Chapter 20 weather atoms, the
  Chapter 7 numbers 6–10 lexicon and its "eight and nine strike out alone"
  etymon, the source-discipline pragmatics atom of Chapter 31, and five of
  Chapter 32's (*telusu* ×2, *cūḍu*'s -అండి, *rā*'s two-stem rule, *tinu*'s
  native-verb/Sanskrit-noun split). Telugu's never-revisited share falls from
  **19 of 78 (24%) to 9 of 98 (9%)**; six of the remaining nine could not be
  reached honestly and are left standing rather than claimed.
- **What is deliberately not claimed.** No source consulted states that everyday
  **సాయం** is a worn-down **సహాయం**; both are attested side by side, and
  `TE-C34-sahayam-ceyu` says so and stops, in the same discipline Chapter 31
  applied to a greeting. Sanskrit **अर्थ** carries only an Avestan cognate on
  the reconstruction available, so no English cousin-web was built on it — the
  lesson uses instead the documented fact that Telugu kept *both* of *artha*'s
  senses, "meaning" and "wealth".
- Wiring: `TE-PATH-027` on `SPINE-SAY-WHAT-I-DO` with extensions
  `TE-EXT-027-MIND-VERBS` and `TE-EXT-028-SOCIAL-VERBS`; the eight concepts
  dropped from that node's `omits`; HL05 ledger entries for chapters 33 and 34
  (both payoffs cover **10/10** of their chapter's atoms, well over the 0.5
  representativeness floor); `core/book-generation.json` targets; generated book
  chapters and `book.tex` inputs; regenerated narration and modality.
- Both chapters stay **drivable**: all eight lessons carry a voice core, with the
  canonical `## The letters in this word` section as the detachable script block.
  Every lesson computes under 300 effective seconds and every one stays inside
  `maxNewAtomsPerLesson: 3`; each chapter introduces 10 against the 12 budget.
  Book compiles under XeLaTeX to 120 pages with **zero** `Missing character`
  lines.

## Chapter 32: the core verbs, under canonical tags (2026-08-06)

- **Telugu taught 60 lessons across 31 chapters and four verbs — *māṭlāḍu*,
  *cēyu*, *uṇḍu*, *veḷḷu*/*vaccu* — every one of them under a Telugu-only tag
  (`TE-VERB-MATLADU`, `TE-VERB-CEYU`, `TE-VERB-UNDU`, `TE-VERB-VELLU`).** A
  namespaced tag joins nothing across languages, so on the cross-language
  measurement the track covered **zero** of the canonical forty core verbs.
- Adds **Chapter 32 — The Core Verbs**: six lessons, one verb each, in a single
  prerequisite chain — `TE-C32-undu` (ఉండు, `VERB-BE`), `TE-C32-vellu` (వెళ్ళు,
  `VERB-GO`), `TE-C32-raa` (రా, `VERB-COME`), `TE-C32-tinu` (తిను, `VERB-EAT`),
  `TE-C32-cuudu` (చూడు, `VERB-SEE`), `TE-C32-telusu` (తెలుసు, `VERB-KNOW`).
  Sequences 610–660, all schema v2. Telugu now covers **6 of the core 40**.
- **One idea per lesson, and each is a place Telugu differs from its sisters
  rather than a place it agrees.** The three slots on Telugu's own be-verb, plus
  the fact that *uṇḍu* **cannot be negated** — Telugu switches to the separate
  verb *lē-*, the root already met as Chapter 1's *lēdu*, and now conjugable
  (*lēnu*, *lēḍu*, *lēdu*), where Tamil, Kannada and Malayalam all negate on the
  shared *il-* (undu). One tense-piece covering both "I go" and "I will go",
  because Telugu merged habit and future where Tamil keeps them apart — and
  "right now" rebuilt out of *unnā-*, so *veḷtunnānu* literally contains the
  previous lesson's be-verb (vellu). The command-shape against the suffixing
  stem, *rā* but *vaccu-*, which is what Chapter 4's *veḷḷi vastānu* has been
  carrying all along (raa). The inherited eat-root Telugu **kept** where Tamil
  demoted its *tiṉ* and assembled *sāppiḍu* — with the honest register split
  beside it: the verb stayed Dravidian while the polite mealtime nouns
  (*bhōjanaṁ*, *āhāraṁ*) are Sanskrit *tatsama* loans (tinu). Four sisters, four
  unrelated see-verbs but one shared eye (*kannu* · *kaṇ* · *kaṇṇu* · *kaṇṇ*),
  and the respectful **-అండి** of *kūrcōṇḍi*/*kṣamin̄caṇḍi* generalised into a
  slot every stem can fill — *cūḍaṇḍi*, *raṇḍi*, *tinaṇḍi*, *veḷḷaṇḍi* (cuudu).
  And the closing asymmetry: *telusu* has **no person-ending at all**, so the
  knower rides in the dative, which finally lets Chapter 6's *nāku telugu vaccu*
  be separated from *nāku telusu* — *vaccu* marks a **skill**, *telusu* a
  **fact** (telusu).
- **Dravidian discipline held.** No Indo-European cognate was invented for any
  Telugu word; every cousin cited is a Dravidian sister with its form supplied
  (Tamil *iru*/*pō*/*vā*/*tiṉ*/*pār*/*teri*, Kannada *iru*/*hōgu*/*bā*/*tinnu*/
  *nōḍu*/*gottu*, Malayalam *irikkuka*/*pōkuka*/*varuka*/*tinnuka*/*kāṇuka*/
  *aṟiyām*), Sanskrit words are marked as loans, and unsettled roots are flagged:
  *veḷḷu*'s own deeper history is explicitly left open rather than guessed at.
- **Drivability held.** All six derive `voice`. No script blocks, no sight cues,
  and the three tables are three wide. The letters each word needs are taught
  inline in a **"Sounds you'll need"** block — the schema-v2 spelling of the
  track's *"The letters in this word"* section, which has no v2 block type; the
  track's other v2 spelling, *"Script you'll notice"*, would have derived `sight`
  and cost the chapter its drivability. Every letter used had already appeared in
  an earlier chapter, so nothing new was gated behind reading.
- **Wiring.** `curriculum.json` gains `TE-PATH-026` on `SPINE-SAY-WHAT-I-DO` —
  the track's first content above A1 — with the six lessons attached as the
  required `TE-EXT-026-CORE-VERBS` extension, and that node's `omits` ledger
  drops the six concepts now realised (`VERB-INFINITIVE` and
  `VERB-PRESENT-HABITUAL` stay omitted, because they are). `chapters.json` gains
  a Chapter 32 entry whose payoff, `TE-C32-telusu`, assesses 7 of the chapter's
  12 atoms (0.58, above the 0.5 floor) and fires no chapter-gate finding.
  `core/book-generation.json` gains the Chapter 32 target; the generated
  `ch32-core-verbs.tex` is `\input` from `book.tex`.
- **Verified.** Integration suite 16/16 green; `check:modality`, `check:books`
  and `check:narration` all clean; every lesson under the duration budget
  (computed 279–295s against the 300s threshold). The book compiles under
  XeLaTeX with zero `Missing character` reports; build artefacts removed.
- **Corpus pins in `modality-manifest`, `levels`, `verbs`, `chapters` and
  `narration` tests are DELIBERATELY left failing.** Telugu was authored in
  parallel with other tracks and only the merged numbers are real; re-pinning
  here alone would repeat the mistake the verbs test's own comment records.

## Handwriting: the gap named, not filled — HL-C41 (2026-08-06)

- **No Telugu handwriting was authored, and that is the deliberate outcome.** The
  track still teaches zero letter formation: `data/scripts/telugu.json` has 0 of 455
  entries with a `strokeOrder`, and there are no `type: writing` lessons. Tamil has
  11/11 and Devanagari 28/28, so this remains a real gap in three of twenty tracks
  (Kannada 0/455 and Malayalam 0/468 are identical).
- **The blocker is provenance.** `strokes.ts` admits a letter only with a citation
  and a URL for its stroke ORDER — the pen path's shape is checked against the
  vendored font, but no font records the order, so it must trace to a published
  source. Not one such source could be opened for a single Telugu letter. Zero
  letters were authored rather than any uncited ones. The full search record is in
  [`BACKLOG.md`](../BACKLOG.md), *Findings from HL-C41*.
- **"Telugu is written without lifting the hand" is a simplification.** Telugu's
  roundness does make many letters loop-continuous, which is genuinely teachable, but
  the published statement about Telugu stroke direction is that the order is *not*
  uniform — clockwise for some letters, counter-clockwise for others — and the
  `talakattu` tick crowning most consonants is described as its own mark. So
  `penLifts` stays **absent** for every Telugu entry, which means NOT VERIFIED.
- **`telugu.json` now states the rule it is governed by.** Its `notes` record that
  only base consonants and vowel signs are ever authored (a syllable's figure is
  assembled from its parts), that `penLifts` absent means NOT VERIFIED and never
  none, and that it must never be inferred from `strokeOrder.length`. The rule is
  expanded in [`data/scripts/README.md`](../data/scripts/README.md). Authoring 455
  syllables was never the work; authoring ~36 base shapes is.
- No lesson content changed. Chapter counts, book output, and the track's 78%
  drivable figure are untouched.

## Chapter capability ledger — HL05 (2026-08-06)

- Added [`chapters.json`](./chapters.json), the track's HL05 chapter capability
  ledger. Twenty-six chapters — 6 through 31 — each declare a first-person
  `canDo`, the shared spine nodes they realise, and a `payoff` naming the lesson
  that proves the claim.
- Every `payoff.assesses` list is copied verbatim from its payoff lesson's own
  `practises.knowledge`, so the ledger cannot claim an atom the lesson does not
  actually practise.
- No chapter from 6 on has a terminal `practice`/`practice-mix` lesson — the
  Chapter 5 recap was the last one authored. Each payoff is therefore the
  chapter's **last lesson by `sequence`**. For Chapter 31 that is the register
  lesson `TE-C31-subha-madhyahnam-register`, which is the right payoff anyway:
  the chapter's promise is knowing *when* the greeting fits, not just saying it.
- **Chapters 1–5 are deliberately absent.** Their lessons are still schema v1,
  carry no `practises.knowledge`, and have no `core/book-generation.json`
  target to derive a title from. A payoff written for them would be invented,
  not derived; the gap is left visible as honest debt rather than stubbed.
- Thinnest payoff, for the representativeness gate that lands next: Chapter 20
  covers 2 of the 4 atoms its two lessons introduce — exactly the 0.5 threshold,
  because the weather lesson closes a chapter that also teaches 11–20.

## Warning-free 95-page book (2026-08-03)

- Explicit static font faces and bookmark-safe script commands remove all font
  substitution and Hyperref warnings across Telugu and its five comparison
  scripts without dropping any inline examples.
- Chapter-specific legacy practice labels and natural `\raggedbottom` page
  endings remove duplicate destinations and underfull-page warnings.
- Concise headings, a responsive family table, a reflowed traditional-month
  list, and a shorter Chapter 20 title remove every remaining overfull line.
  The full vocabulary, grammar, comparison, and etymology content remains in the
  lesson bodies shared with Language Ladder.
- The forced 95-page XeLaTeX build now reports zero missing glyphs, box warnings,
  duplicate destinations, Hyperref warnings, LaTeX warnings, or font warnings.
  All pages and the complete 93-entry outline were inspected; a visual-only
  running-header collision found during that review is fixed as well.

## Canonical Chapters 6–31 publication (2026-08-03)

- Thirty later-track lessons now use schema v2 with explicit shared-spine
  placement, prerequisite-safe sequences, typed knowledge boundaries, honest
  sub-five-minute budgets, skills, modes, strands, register, and variety.
- Twenty-six generated chapters extend the downloadable book through Chapter
  31. Their source hashes and lesson ids are independently reproduced by
  Language Ladder, keeping book and app content synchronized.
- A reusable multi-script generator set selects the right vendored font for
  Telugu, Tamil, Kannada, Malayalam, Devanagari, and Arabic-script comparisons.
  The 95-page forced XeLaTeX build has zero missing glyphs; every page and the
  complete outline were inspected.
- The expanded book's remaining layout, duplicate-label, bookmark, and font
  warnings are recorded in `HL-B23`; Telugu roadmap/session-map reconciliation,
  including Chapter 20's numbers-and-weather grouping, remains in `HL-M02`.

## Sub-five-minute lesson remediation (2026-08-02)

- All thirty-six Telugu duration violations are resolved. Thirty-five lessons
  already computed below five minutes and now declare an honest four-minute
  budget without changing their teaching content.
- The genuinely long Chapter 31 lesson becomes two prerequisite-ordered steps:
  build **శుభ మధ్యాహ్నం** from the widened “noon” word shared with Kannada,
  then distinguish the two-source formal-register claim from the one-source
  lower-frequency claim. They compute to 152 and 193 seconds.
- The new support lesson brings the Telugu track to 60 lessons with zero unknown
  prerequisite ids.
- A forced book build succeeds at 29 pages with no missing glyphs. Canonical
  lessons continue through Chapter 31 while the book stops at Chapter 5
  (`HL-B22`); existing layout, bookmark, duplicate-label, and font warnings are
  tracked in `HL-B23`; roadmap and session-map drift is tracked in `HL-M02`.

## Chapter 6 — Case endings, and the sentence with no subject

- **Chapter 6 authored** (`TE-C06-dative-ku`, `-dative-subject`): the track's first
  **case ending** — reviewing Ch.2/3/4/5 via `reviews_of`.
- **-కు/-కి** (`TE-C06-dative-ku`): the dative "to/for," taught as the doorway to
  **agglutination**. Telugu **adds** a suffix carrying **one** meaning, keeping its
  shape with the **seam visible** (*pēru* + *ku*), where a Latin ending like *-īs*
  **fuses** case+number+declension into one indivisible lump; a four-row table sets
  the systems side by side. Notes that *-ku* and *-ki* are **one suffix adjusting
  to the preceding vowel**, and includes the pronoun shift *nēnu* → **నాకు** *nāku*.
- **నాకు తెలుగు వచ్చు** (`TE-C06-dative-subject`): "I know Telugu" — literally
  "**to-me Telugu COMES**." Two payoffs at once: there is **no nominative subject**
  (contrast Ch.5's *nēnu telugu māṭlāḍatānu*), and the verb is **వచ్చు**, the very
  "to come" taught in Ch.4 — a language you know is a thing that *comes to you*.
  Explains the **dative-subject** rule (knowing, liking, wanting *happen to* you)
  with English's "**methinks**" as the bridge.
- **The Dravidian family thread**, new in this chapter: *-ku / -ukku / -ge / -ikku*
  are visibly the **same suffix** across the four sisters, all of which build "I
  know X" the same subjectless way.
- Taxonomy: namespaced `TE-CASE-DATIVE`, `TE-DATIVE-SUBJECT`.

## Chapters 3–5 — How-are-you, Farewells, First Verbs

- Three new chapters carry Telugu to Chapter 5, matching the leading tracks' arc.
  One word per lesson, atom-first, Telugu script inline; every root traced
  (`lessons/TE-C0{3,4,5}-*`, `book/chapters/ch0{3,4,5}-*.tex`). Concept tags reuse
  the universal `HL01` taxonomy; verbs namespaced (`TE-VERB-*`). Telugu's
  heavy-Sanskrit-borrowing-yet-Dravidian-grammar character runs throughout.
- **Ch. 3 — How Are You**: *elā* (how; the native *e-* questions) → *mīru elā
  unnāru?* (the verb *uṇḍu* "to be") → *nēnu* (I ← Proto-Dravidian, unrelated to
  *me*) → *bāgā* (well; *nēnu bāgunnānu* "I'm well") → *paravālēdu* ("no harm" =
  you're welcome, built on Telugu's own *lēdu* — where Tamil/Kannada/Malayalam
  use *illa*) → practice.
- **Ch. 4 — Farewells**: *veḷḷu*/*vaccu* → *veḷḷi vastānu* ("I'll go and come
  back," tabled across the Dravidian family) → *rēpu kaluddām* (see you tomorrow;
  the "let's ___" *-ddām*) → *maḷḷī kaluddām* (we'll meet again; native *kalu*,
  where Tamil borrowed Sanskrit *sandi*) → practice.
- **Ch. 5 — First Verbs**: *māṭlāḍu* (← *māṭa* "word"; stem + tense + person) →
  *nēnu telugu māṭlāḍatānu* (I speak Telugu — "the Italian of the East"; no
  1st-person gender) → *uṇḍu* (to be/stay/live; the postposition *-lō*) → *pani
  cēyu* (to work; noun + *cēyu*, the twin of Hindi's *karnā*) → practice. Book
  compiles clean with XeLaTeX (0 missing chars, 0 undefined refs).

## Chapter 2 — Introducing Yourself

- New chapter around the introduction dialogue (*nā pēru … / mī pēru ēmiṭi?*),
  atom-first, Telugu inline (`lessons/TE-C02-*`,
  `book/chapters/ch02-introductions.tex`). Every atom traced:
  - **పేరు** pēru ("name") ← Proto-Dravidian *\*pēr* — twin of Tamil *peyar*,
    **not** the Indo-European *name/nām* (even Sanskrit-heavy Telugu kept the
    native word).
  - **నా** nā ("my") ← *nēnu* ("I").
  - **నా పేరు …** — **"my name is…"**; the **zero copula** (no "is").
  - **నువ్వు / మీరు** nuvvu/mīru — "you," familiar/respectful; respect by plural.
  - **ఏమిటి** ēmiṭi ("what") ← Dravidian question-stem *\*yā-/\*e-*.
  - **మీ పేరు ఏమిటి?** — **"what's your name?"**
  - **సంతోషం** santōṣam — "pleased to meet you," a **Sanskrit** loan (as in
    Kannada; vs. Tamil's native *magiḻcci*).
  - **practice** — the whole dialogue.
- Example names are invented (Mira / Arun), not reused from any source text.
  Book compiles clean with XeLaTeX.

## Chapter 1 — Greetings (Telugu script taught inline)

- New Telugu track on the HL00 framework — the third of the four Dravidian
  tracks. One word per lesson, slug ids, atom-first, derivations shown, LaTeX
  book. Uses the **vendored** Noto Sans Telugu font (relative `Path=`, shaped
  via `Script=Telugu`, no polyglossia language module needed).
- **No reading course.** Per `HL00`'s inline-letters rule, Telugu is taught
  *inside* each word lesson.
- Chapter 1 (`lessons/TE-C01-*`), greetings + conversational glue:
  - **నమస్కారం** namaskāram ("hello," **Sanskrit** namas + kāra) — inherent
    *a*, the talakaṭṭu, vowel signs, the స్క below-stacking conjunct, and the
    anusvāra ం.
  - **ధన్యవాదములు** dhanyavādamulu ("thanks," **Sanskrit** stem + Telugu plural
    *-mulu*) — the aspirated ధ, న్య conjunct, and a first look at Dravidian
    agglutination.
  - **అవును** avunu ("yes," native) — yes/no as statements of being.
  - **లేదు** lēdu ("no / there isn't," native) — Telugu's *different* root
    (*lē-* / *kā-*), where its sisters use *il-*; the existence-vs-identity
    split (*lēdu* / *kādu*).
  - **సరే** sarē ("okay," native) — the family word *sari* in Telugu dress.
  - **practice** — recap + the *veḷḷi vastānu* / *veḷḷi raṇḍi* farewell (same
    "go and come back" logic as Tamil and Kannada).
- The recurring thread: **Sanskrit for greetings/politeness, native Dravidian
  for the everyday grammar** — plus Telugu's own twist, its divergent "no."
  Each lesson carries an "Across the family" cognate box (English / Sanskrit /
  Hindi / Tamil / Kannada / Malayalam), every form supplied so nothing is
  assumed. Book compiles clean with XeLaTeX.
