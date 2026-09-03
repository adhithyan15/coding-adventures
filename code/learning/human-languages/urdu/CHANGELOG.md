# Changelog

## Unreleased — the retrieval is seated per lesson, and Urdu enters R2

Fifth track through the HL-C313 fix, and the first one **measured before it was
scheduled** rather than after. That measurement changed the fix.

### What the measurement said, before anything was written

Urdu is not the vocabulary-tranche shape the first four tracks were. Its
chapters carry one to five word lessons, each lesson introduces two or three
atoms, and its distance histogram has **no cliff at 5** — 111 practice events
already land inside the R2 window, against 202 inside R1. Urdu's 108 R2 misses
are individual atoms that happen to get no hit, not a structural wall.

Two things follow, and both were counted rather than assumed.

**Seat capacity is the binding constraint, not the window.** 28 of Urdu's 56
word lessons have 25 seconds of headroom under the 300-second ceiling, so at
three recalled items each there are 84 seats for 108 atoms. Counting per atom:
70 have a seat, 36 have seats that are all taken, and 2 have no word lesson in
their window at all. **65% is the ceiling here**, and no placement rule beats it.

**The chapter-boundary rule is the wrong granularity.** Both rules were applied
to a clean tree and measured, rather than one measured and the other guessed:

    chapter-granular (chapter N retrieves chapter N-2)   108 -> 65
    per-lesson seating (nearest free seat at ~10)        108 -> 48

Pairing chapter to chapter spends one of a very scarce set of seats on a whole
chapter and starves the next. Seating each source lesson individually fits the
same budget three times better, and on a uniform tranche it makes the same
assignment the chapter rule does — it is a generalisation, not a replacement
(HL-C318).

### The change

23 word lessons in chapters 3-18 gained one `[YOU RECALL: ...]` task naming a
word introduced five to fifteen lessons earlier, preferring ten. The line is not
shortened to fit a budget: a seat is a word lesson that can afford it, and a
source with no affordable seat is reported rather than forced.

Only **3 of the 24 recall lines carry a read**. A word is offered to read only
where every one of its glyphs has already been taught, and Urdu's Nastaliq
ladder arrives late in the book, so most of these retrievals are spoken on both
sides. That is the honest answer rather than a defect: asking a reader to decode
untaught Nastaliq is exactly what `script-closure.ts` counts.

### Every number re-measured against the merged tree, not derived

    urdu R2 misses (5-15, "first real retrieval")   108 ->  48   (-60)
    urdu R1 misses (1-3)                             38 ->  38   (held)
    urdu R3 misses (20-60)                           75 ->  75   (held)
    urdu R4 misses (80-250)                          22 ->  22   (held)
    urdu reinforcement window misses                243 -> 183
    urdu atoms taught                               186 -> 186   (held)
    urdu atoms never revisited                        8 ->   4   (improved)
    urdu lessons                                     89 ->  89   (held)
    forward prerequisites                             0 ->   0   (held)
    forward references                                2 ->   2   (held)
    script closure violations                       271 -> 271   (held)
    corpus R2 misses                               4550 -> 4490
    lessons at or over the 300s ceiling               0 ->   0
    computed seconds, median of ch1-18              219 -> 220

60 of the 70 seatable atoms closed. The remaining ten were lost to greedy
first-come seating rather than to any hard limit, and a better assignment would
recover some of them — nobody should read 48 as the floor.

The derivation was falsified before shipping: reverting the single lesson
`UR-C04-thik` and re-measuring put R2 back to 52, the four atoms carried by the
two lessons it retrieves.

Of the 48 that remain, 17 sit in chapters 1-5 — phrase, writing and practice-mix
lessons rather than word lessons — and 31 in chapters 6-17, where the seats are
full. They close when those lessons are split; HL-C317 has the measurement, and
Urdu is the worst-affected track in the corpus by that count.


## Unreleased — chapter 1 is generated; the track owns no hand-written chapter (HL-C287)

`book/chapters/ch01-greetings-and-responses.tex` was the last hand-written
chapter Urdu held, and the only one left in the whole book. It is now generated
from its six lessons, so `handwritten_parity.py --check urdu` answers "already
retired, nothing handwritten remains" and exits 0. Urdu now owns no
hand-written chapter at all; measured against the merged tree, four tracks
still do — french, german, kannada and marathi.

- **The migration was the work.** `UR-C01-salam` was still schema v1, which
  declares no knowledge atoms, and `book.ts` refuses to generate a chapter from
  a v1 lesson — so the flip could not even be attempted first. The lesson now
  carries v2 frontmatter, `spine_node: SPINE-MEET-GREET` (the path segment it
  was already in), an `hl-knowledge` directive on every block, and one atom,
  `UR-LEX-SALAM`. **One** new atom, not three: the chapter policy allows three,
  and the etymology block introducing the headword is the shape
  `UR-C01-shukriya` already uses.
- **Four headings were re-pointed by PREFIXING, never by rewriting.**
  `Notice the direction` -> `Script — notice the direction`;
  `Meaning and history` -> `The word, taken apart` (the corpus's own name for
  an etymology block); `Use it` -> `Guided Practice — use it`, printed as
  "Your turn — use it"; `Quick recall` -> `Wrap-up Recall`. A per-character
  non-ASCII census across every edited lesson shows the only additions are em
  dashes and one `ū`, and **zero** Urdu or IAST characters lost.
- **The reported parity gap of 23 was 22 parts instrument.** `checkpoint`x6,
  `rootweb`x4, `scriptstep`x6 and `usage`x7 are environments the generator
  cannot emit, and `unportable_blocks` charges for them with no markdown side
  to subtract — but their prose was already in the lessons, under headings that
  classify to `script`, `input`, `guided-production` and `recall`. Sized instead
  by counting what the chapter TEACHES: 17 distinct Urdu tokens in the `.tex`,
  **0** owned by no lesson, 6 sections against 6 lessons. Recorded as HL-C287.
- **Five things the hand-written chapter taught and the lessons did not say are
  now authored into the lessons**, found by a content-word census of the two
  `.tex` files rather than by the block gap: the name **nūn ghunna** for `ں`;
  that **shukriyā** is an ordinary everyday modern expression and not a
  literary relic; that the safest reply to **salām** is the same word back; the
  instruction to cover the romanization before saying **salām**; and the
  contrast that made **nahī̃**'s etymology a point — unlike everything before it
  in the chapter, it is inherited Indo-Aryan rather than Persian-Arabic.
- **One paragraph is genuinely replaced, not carried.** The hand-written
  chapter opened on a note about its own construction ("builds one miniature
  exchange in dependency order... every step stays under five minutes"). The
  generator opens every chapter on `chapteropening` instead, so chapter 1 now
  matches the other seventeen. Its two substantive claims survive in the
  lessons that make them — `UR-W01-shukriya-first-three` still says tracing is
  enough, and `UR-C01-shukriya` still says copying with a model is.
- **The Hindi cousin is no longer printed in Devanagari.** `UR-C01-nahin`
  showed **नहीं**; the Urdu book's font covers no Devanagari, so generating the
  chapter turned four codepoints into `glyph-coverage` failures. The
  hand-written chapter had given the cousin in romanization only and said why —
  "without making Hindi script a prerequisite" — so the lesson had drifted and
  the hand-written chapter was right. This was invisible while the chapter was
  hand-written, because nothing rendered the lesson.
- **`UR-LEX-SALAM` is reinforced, not merely introduced.** `UR-C01-shukriya`
  already printed *salām* in both its practice and its recall; it now declares
  the atom, so the reuse is visible to the reinforcement windows.

Counters, each re-measured against the tree rather than derived:

- Urdu's lesson-content budget moves **88 -> 89** — `UR-C01-salam` becoming
  measurable, not a new lesson. Idioms, senses and culture claims are unchanged
  at 2 / 4 / 4.
- `atomsTaught` **185 -> 186**; `atomMeasurementBlindLessons` **1 -> 0**, which
  clears the track's `measurement-blind` finding outright.
- `firstWritingPracticeAt` **1 -> 0** clears the `writing-ramp` finding. This is
  the migration making existing content visible, not new content: lesson one
  always showed the four letters of **سلام**, but the heading did not classify
  as a `script` block, so `isWritingPractice` could not see it.
- `reinforcementWindowMisses` **240 -> 243** — a legitimate RISE. A newly
  declared atom is a newly measured one, and `UR-LEX-SALAM` is revisited once,
  so its R2/R4 windows are now counted as missed. The debt existed before; the
  gate could not see it while the lesson declared nothing.
- The `grouped-shards` handwritten ratchet is untouched, per its own in-file
  directive. `handwritten.d` holds **34** entries with this change in — counted
  from the directory on the merged tree, not derived. It was 38 when this
  branch was cut; Arabic ch2, German ch3 and a French chapter landed on main
  in between, so subtracting one from a remembered number would have been
  wrong by three.

Verified: human-language-data 124 test files / 1730 passing; all eleven
`check:*` gates; language-ladder 39 files / 442 passing; the whole Urdu book
compiles under XeLaTeX with zero overfull boxes, and the six changed pages were
read as rendered PDF to confirm the Nastaliq shapes, joins and the nasal
`hā̃` diacritic survive typesetting. (`check:compile` needs bash 4's `mapfile`
and macOS ships 3.2, so this went through `book-cli.js
--materialize-compile-inputs` and latexmk directly.)

## Unreleased — spell the review words out of letters the reader has (HL-C242)

HL-C241 measured that Urdu's remaining closure debt could not be bought with
more letters or a better order: simulating the *entire* remaining alphabet at
the policy's own pace still left 40 violations, because eight glyphs are first
demanded at content lesson 27. The lever it named was prose, and this is that
pass.

- **Closure violations 41 → 4** (corpus-wide 518 → 481). Every lesson from
  chapter 7 to chapter 15 was printing review words in Nastaliq spelled with
  letters no lesson had taught yet. Those words are now printed the way the
  reader can actually use them — romanized — and the Nastaliq that remains in
  a lesson body is either its own headword or spelled entirely from the
  fifteen letters the ladder has handed over by that point.
- **The four survivors are deliberate and are not typos.** `UR-C08-puchhna`,
  `UR-C13-lal`, `UR-C13-kala` and `UR-C15-hawa` cite Persian and Arabic etymons
  in the source language's own orthography — *pursān-i ḥāl*, *lāl*, *mīrā*,
  *hawāʾ* — carrying ه, ء and the vowel marks ◌َ ◌ِ. HL-C241 classified these
  as a gap in the cousin-layer exemption rather than a spelling mistake:
  Arabic and Urdu are one script system as far as `SCRIPT_SYSTEMS` is
  concerned, so the "shows another script, never charged" rule never fires.
  Repairing them would damage the etymology layer, so they stay, and the
  remaining count of 4 is the honest floor of this pass rather than its
  failure. The fix belongs in the measurement.
- **Nothing was laundered into headwords.** `exposureExemptedGlyphs` is
  unchanged at 142 across the whole pass — the number that would have moved if
  script had been pushed into the exempt headword slot to make the count fall.
  `headwordsWithoutRomanization` stays at 0. `exposureOnly` rose 18 → 49,
  which is the honest half: those are lessons now clean whose only untaught
  glyphs sit in a headword the reader was given a romanization for.
- **Letters the reader cannot yet see are now named rather than shown.** The
  aspirate strand that runs through chapters 7–12 — the two-eyed **ھ** taking
  a base letter and turning it into an aspirate — survives intact, because
  **ھ** rides the headword of every lesson that teaches it. What changed is
  its partners: *jīm*, *ṭe*, *ṛe* and *che* are named in words where their
  shapes used to be printed untaught. Naming a letter the reader has been
  taught to say is more honest than showing a shape nobody taught them, and it
  is what the audio carried in either case.
- **Fixed two genuine forward references** found by the pass rather than
  introduced by it. `UR-C06-bolna` told the reader "you have already read
  **پ**" a full chapter before `UR-W07-pe` hands it over; it now points
  forward to the letter instead of backward to a lesson that has not happened.
  `UR-C05-practice` printed a four-line exchange of which the reader could
  decode one line, and now runs the exchange from its romanization and says
  which three pieces of it are inside the ladder today.
- **Etymon citations that were not orthography were transliterated.** Persian
  *guftagū* and *nastaʿlīq*, Arabic *fikr*, *qalam* and *kitāb*, and *namāz*
  were set in Nastaliq in bodies where the reader could not decode them; they
  are now printed the way they are said. The genuine source-language citations
  (the four above) were left alone.
- **One shared number adjusted, and none of its structure touched.**
  `tests/script-closure.test.ts` used to assert corpus violations `> 500` — a
  FLOOR on debt, which fails when the work succeeds. Marathi's runway (PR
  #13976) hit it at 498 and rewrote it properly before this branch landed: the
  floor became a live comparison against `measureScriptRamp`'s own violation
  count, so the module's claim is asserted as a relation rather than a
  magnitude, plus a ceiling on the absolute debt. That restructuring is theirs,
  not this tranche's. All this change does is move their ceiling 498 → 380 to
  the value measured after Urdu's 41 → 4 landed on top of their 44 → 0, with a
  one-line note saying which track moved it. Their comparison is untouched, and
  the relation still holds with room to spare (380 against a pace budget of 37).
  Persian's expectations, and every other track's, are untouched.

## Unreleased — move the Nastaliq ladder to the front of the book (HL-C241)

- Redistributed all fourteen letter lessons out of chapters 16–18 and into
  chapters 1–8, two content lessons apart, so the ladder now runs from sequence
  35 to 295 instead of 730 to 1000. `UR-W16-*`, `UR-W17-*` and `UR-W18-*` were
  renumbered to `UR-W01-*` … `UR-W08-*` to match the chapters they now live in.
- **Closure violations fell 46 → 41**, and the number the exposure rule was
  quietly carrying fell with them: glyphs excused by the headword-romanization
  exemption dropped 263 → 142 inside Urdu. Headwords without romanization
  stayed at 0; glyphs shown but never taught stayed at 22.
- Retired chapter 16's outright breach of `minLessonsBetweenScriptSegments: 2`
  — seven letter lessons in seven consecutive positions. No two script lessons
  in the track are now closer than two content lessons apart.
- Reordered the ladder so **sīn** comes first, before alif, and taught it as
  **ش** *shīn* with its three dots taken off. The reader's first letter drawn
  from scratch is one they have already been looking at since *shukriyā*, and
  it is what makes `UR-C02-mera-naam` decodable in chapter 2 rather than 16.
- Rewrote every letter lesson that had been anchored to chapter 17 and 18
  vocabulary so it stands on words already taught where it now sits: gol he
  demonstrates its three faces on **ہم**, **کہانی** and **کمرہ** and points at
  the **ہ** that has closed *shukriyā* since chapter 1; nūn ghunna keeps
  **میں**, **ہیں** and **نہیں**; baṛī ye trades **کالے** for **میرے**; alif
  madda shows its two spellings of long *ā* inside **آسان**; pe uses **آپ**,
  **پیر** and **اپنا**; vāʾo uses **وکیل**, **ہوں** and **کون**. Every example
  word in every letter lesson is now spelled entirely from letters already
  taught.
- Stopped two lessons showing the reader script they could not yet decode:
  `UR-C06-hona` now names *hūṅ*'s dotless nūn instead of printing **ہوں** a
  lesson before the letter arrives, and `UR-C06-ana` names *āp* instead of
  printing **آپ** two chapters before pe. Both now say when the missing shape
  is coming, which is a better lesson than the silent one it replaces.
- Gave the moved letters real spaced returns rather than a single appearance:
  mīm reviews lām, baṛī ye's warm-up rewrites nūn ghunna, gol he is assessed
  again inside nun ghunna's and vāʾo's word blocks, and chapters 16–18 keep
  their reading payoffs, which now cash in letters learned eleven chapters
  earlier. Corpus-wide missed R3 reinforcement windows fell 4342 → 4324.
- Rebuilt the curriculum path to match: the ladder is interleaved into path
  segments 003–008-B with nine new script extension nodes, and the three old
  script extension nodes keep only their reading payoffs. Chapters 16 and 18
  were retitled, and chapters 1–8 now state the letters they teach.
- Recorded in `BACKLOG.d` that redistribution has a floor of 41 and why:
  simulating the whole remaining alphabet taught at policy pace still leaves 40,
  because eight untaught glyphs are first demanded inside chapter 7. The entry
  also names eight glyphs whose pedagogy already exists in content lessons that
  earn no closure credit, the measured letter order whose payoff cliff is at
  **ظ**, four "untaught" glyphs that are really Arabic and Persian etymon
  citations, and the three metrics this move cost.

## Unreleased — interleave the Nastaliq ladder with the words it spells (HL-C240)

- Added chapters 17 and 18: twenty lessons that alternate **one** new letter
  with glossed vocabulary and review, so no word is ever decoded before the
  glyph that spells it has been taught, and no glyph is taught before a word
  has already put it in the reader's ear.
- Taught six letters — **ہ** *gol he*, **ے** *baṛī ye*, **ں** *nūn ghunna*,
  **آ** *alif madda*, **پ** *pe*, **و** *vāʾo* — each in the positional forms
  the chapter's own words need, rather than as an isolated shape the next
  lesson silently joins. Gol he is taught in all four faces because its middle
  form is the one that reads as a different letter.
- Raised the letters Urdu teaches from 9 to 15 of the 37 it shows, so glyphs
  shown but never taught fell from 28 to 22.
- Added six pre-A1 headwords — **یہ** *yih*, **کام** *kām*, **کہاں** *kahāṅ*,
  **ماں** *māṅ*, **آم** *ām*, **آسمان** *āsmān*, **پرانا** *purānā*, **وہ**
  *voh* — taking the track from 43 to 51 of the 300-headword pre-A1 target.
  Each is chosen so a letter arriving two or three lessons later completes it.
- Completed the pre-A1 cumulative writing-stage ladder. `guided-copy`,
  `delayed-copy` and `dictation-transcription` now have valid evidence beside
  the `observe-trace` that chapter 1 already carried, closing a level-gate
  blocker that had stood since the ladder was contracted.
- Made two payoffs land on the page rather than in the ear: **میرا نام ... ہے**
  becomes readable at chapter 17's second letter, and the whole opening
  exchange — *salām*, **آپ کا نام کیا ہے؟**, **میرا نام ... ہے۔**,
  **آپ کیسے ہیں؟**, **میں ... ہوں۔** — is readable and writable from dictation
  by the end of chapter 18.
- Ran the etymology layer as an argument rather than decoration: **کام** and
  English *karma* are one Sanskrit word by two roads; **کہاں** shares its
  opening consonant with the whole English *wh-* family through PIE *\*kʷis*;
  **آسمان** descends from a root meaning *stone* and is a real cousin of
  English *hammer*; and **آم** is deliberately set against English *mango*,
  which is Dravidian and unrelated, so the reader learns that a resemblance
  settles nothing either way.
- Recorded in `BACKLOG.d` that closure violations stayed at 46 and why: the
  whole script ladder sits after every vocabulary chapter, so letters taught at
  sequence 840 cannot retire glyphs a reader met at sequence 20. The remaining
  debt is bought by moving letters earlier, not by teaching more of them.

## Unreleased — project-defined pre-A1 four-skill task shapes

- Made the Urdu pre-A1 bridge executable as independent reading, listening,
  writing, and speaking papers with exact timing, items, replay, aids, scoring,
  and a 60/100 floor on every skill.
- Kept writing productive and script-aware: scored work covers delayed recall,
  dictation, and bounded independent Urdu-script responses with right-to-left
  order, joins, non-joins, dots, and word-boundary control. Roman Urdu and
  Devanagari cannot substitute for an Urdu-script response.
- Kept the construct typographically fair: ordinary legible handwriting is
  enough, Nastaliq calligraphic imitation is not scored, approved Naskh remains
  an accessibility presentation, and unwritten short vowels are not errors.
- This project rung is not an external qualification. Mocks, rubrics, keys,
  calibration, curriculum coverage, and book-only validation remain required.

## 2026-08-21 — Contract the seven-rung Urdu assessment ladder (#12444)

- Defined pre-A1 through C2 as project-owned four-skill assessments with
  independent pass floors, cumulative writing stages, and two timed mocks per
  rung.
- Kept CEFR as the descriptive framework rather than inventing an external
  Urdu certificate or implying Council of Europe endorsement.
- Published Urdu-specific Nastaliq, Naskh fallback, ordinary-handwriting,
  register, variety, and transliteration boundaries for fair scoring.
- Fixed executable task envelopes and standard-setting requirements so a level
  label cannot stand in for task inventories, rubrics, mocks, or human evidence.

## Gentle `shukriyā` writing split (#12261)

- Add a 3.5-minute recognition-and-tracing step for only **ش**, **ک**, and
  **ر**, then stop before joining or recall from memory.
- Rewrite the following four-minute `shukriyā` lesson to add only **ی** and
  final **ہ**, then guide one model-visible copy of the complete word.
- Keep the opening dependency order intact while reducing its first-seen Urdu
  script load from five shapes in one lesson to three and then two.
- Record the first micro-lesson as valid `observe-trace` evidence in the shared
  cumulative writing-stage ladder.
- Bind Chapter 1's terminal checkpoint to the six typed atoms it actually
  retrieves, so the gentler split does not create new chapter-payoff debt.

## Opening chapter reading order (#12260)

- Add sequence 10 through 40 to the four legacy opening lessons in the book's
  explicit greet, thank, respectful yes, no dependency order.
- Remove Urdu's missing-sequence and false forward-prerequisite debt without
  changing the short opening lessons.

## 0.11.0 — 2026-08-12

Thirteen everyday-noun lessons across three new chapters (13–15), the
track's second pre-A1 vocabulary tranche (wave 6), continuing the
cross-track program: `vocabularyOf()` moves Urdu's pre-A1 vocabulary from
30 to 43 distinct headwords (43 → 56 overall). All seven pre-A1 spine
nodes were already realized after the first tranche; this wave adds depth,
not new spine coverage.

- **Ch. 13 — Four Colors** (`SPINE-POLITE-REQUEST-REPAIR`): lāl, safed,
  kālā, nīlā. Sorts the set into two adjectives that never change ending
  (lāl, safed) and two that agree with the noun's gender the way *merā*
  does (kālā/kālī, nīlā/nīlī) — the chapter's first new grammar atom,
  `UR-GRAMMAR-ADJECTIVE-AGREEMENT`. safed reuses the دل/hṛdaya two-roads
  shape, this time with English **white** itself as the third cousin
  (PIE \*ḱweyt-). kālā is inherited from Sanskrit *kāla* but that root is
  usually traced to Proto-Dravidian \*kār- rather than to
  Proto-Indo-European — the first inherited word in this book whose own
  source sits outside the Indo-European family. nīlā's own root is a dead
  end backward, but its cousin *nīlī* travelled forward through Persian,
  Arabic *an-nīl*, and Portuguese *anil* into English **aniline**.
- **Ch. 14 — Four Things You Wear** (`SPINE-POLITE-REQUEST-REPAIR`):
  qamīz, jūtā, ṭopī, koṭ. Names all three loan directions in one small
  set: Persian and Arabic into Urdu (qamīz, probably the same Late Latin
  *camisia* that gives English **chemise**, by two completely different
  roads); Urdu out into English (ṭopī, borrowed directly as **topee**,
  1825–35); and English straight into Urdu (koṭ, a homograph of the older,
  unrelated *koṭ* "fort"). jūtā traces to Sanskrit *yukta* "joined, yoked"
  from PIE \*yewg-, a cousin of English **yoke** and of Sanskrit's own
  word **yoga**. Two new letters: **ق** *qāf* and **ض** *zād* (qamīz), and
  **ت** *te*, contrasted by dot-count with already-known retroflex **ٹ**
  (jūtā).
- **Ch. 15 — Rain, Sun, Wind, Heat, Cold** (`SPINE-CHECK-WELLBEING`):
  bārish, dhūp, hawā, garmī, sardī. Sorts all five by how far their root
  reaches: two Persian dead ends (bārish, sardī), one inherited dead end
  (dhūp, the same honest wall as کان and روٹی), one loan of a loan (hawā,
  Persian's own borrowing from Arabic *hawāʾ* — a shape this book had not
  shown before), and garmī, whose root reaches Greek **thermós**
  (PIE \*gʷʰer-) — with English **warm** flagged honestly as only a
  disputed, not settled, cousin. One new letter, **گ** *gāf* (garmī, one
  extra stroke on already-known **ک**). sardī closes the tranche and
  reaches back several chapters to rescue orphaned atoms.
- Corrected two assumptions before writing: لال ("red") is a Persian loan
  with no further documented root, not a Sanskrit *lākṣā* ("lac dye")
  derivation as an initial search suggested; and English **warm**'s link
  to گرمی's PIE \*gʷʰer- root is only a proposed "distant doublet," not a
  settled cousin the way Greek *thermós* is — both lessons state the
  weaker or absent connection rather than the stronger claim that first
  seemed plausible.
- Reinforcement: every lesson's `practises.knowledge` reaches back to the
  preceding one to three lessons, and the tranche is threaded with
  rescue passes for atoms the corpus had never adequately revisited —
  `UR-CROSSLINGUAL-KHUDA-HAFIZ-SPELLING`, `UR-SCRIPT-MERA-NAAM-HAI`,
  `UR-CROSSLINGUAL-THIK`, `UR-DIALOGUE-TAKE-LEAVE`,
  `UR-GRAMMAR-KHUDA-HAFIZ-ELLIPSIS`, and sixteen more spanning chapters
  2–12. sardī, the tranche's final payoff, reaches back to دل's PIE
  \*ḱērd- argument and to `UR-REGISTER-TWO-ROADS-ONE-ROOT`. Pre-A1
  reinforcement debt (revisited fewer than twice) fell from 24 atoms to
  12; the remainder is this tranche's own tail lessons and a few
  pre-existing atoms whose thinness only became measurable once the
  longer track made the R3 window apply — debt left visible for the next
  wave, not papered over.
- Declared `chapters.json` chapters 13–15, each with a `canDo` and a
  payoff whose `assesses` list is the payoff lesson's own
  `practises.knowledge` verbatim. Wired all three into
  `core/book-generation.json` (chapter/output/script fields only,
  deriving title and label from `chapters.json`) and `book/book.tex`.
- No lesson exceeds three new atoms; no chapter exceeds twelve. The three
  pre-existing atom-budget violations (chapters 3–5) and the pre-existing
  chapter-4/3/6 budget overruns are untouched — outside this tranche's
  scope.
- Book compiles clean with XeLaTeX: 135 pages, zero errors, zero `Missing
  character` warnings. Two Devanagari citations and one bare IPA glottal
  stop (ʔ) were caught by the compile and replaced with plain
  transliteration, matching the track's transliteration-only convention
  for Sanskrit forms.

## 0.10.1 — 2026-08-08

- Replaced the explicitly temporary Naskh presentation with static Noto
  Nastaliq Urdu Regular and Bold faces in both the downloadable book and
  Language Ladder.
- Kept Naskh as a browser fallback rather than presenting its letter style as
  the Urdu course's normal typography.
- Pinned the official Noto distribution revision and file hashes in the shared
  font inventory, and verified Urdu coverage, OpenType shaping tables, the
  XeLaTeX book, and the production application bundle.

## 0.10.0 — 2026-08-08

Thirteen everyday-noun lessons across four new chapters (9–12), continuing the
cross-track pre-A1 vocabulary program and confirming the same measured
mechanism a further time: `vocabularyOf()` counts distinct `headword:` strings
1:1 with lessons, so thirteen new word lessons move Urdu's pre-A1 vocabulary
by exactly thirteen (17 → 30 distinct headwords at or below pre-A1). All seven
pre-A1 spine nodes are now realized.

- **Ch. 9 — People You'd Introduce** (`SPINE-EXCHANGE-NAMES`): bhāī, bahan,
  dost, khāndān. Chooses *merā*/*merī* correctly for each noun's grammatical
  gender, and sorts the four into inherited *bhāī*/*bahan* against Persian
  *dost*/*khāndān* — the same native-versus-borrowed split Chapter 8's verbs
  drew.
- **Ch. 10 — Four Words on the Face** (`SPINE-CHECK-WELLBEING`): kān, nāk,
  āṅkh, muṅh. Notices *kān* and *nāk* share the same three letters reversed;
  reads the plain nūn that nasalizes *āṅkh* and *muṅh* against the dedicated
  ں of *maiṅ*/*hūṅ*; counts three settled Indo-European cousins (*nāk*/nose,
  *āṅkh*/eye) against *kān*'s honest dead end.
- **Ch. 11 — The Heart**: dil. Traces *dil* through Middle Persian and
  Proto-Iranian back to the same PIE \*ḱērd- that gives Sanskrit *hṛdaya*,
  English *heart*, Latin *cor* and Greek *kardía* — *dil* and *hṛdaya* split
  at the Indo-Iranian fork, the same root by two roads, not opposite
  lineages, echoing *pūchhnā* and *porsīdan*.
- **Ch. 12 — Water, Tea, Milk, Bread** (`SPINE-POLITE-REQUEST-REPAIR`,
  previously unrealized — this is the resolution): pānī, chāy, dūdh, roṭī.
  Reuses Chapter 8's dative-experiencer frame on a plain noun (*mujhe roṭī
  pasand hai*) instead of an infinitive; sorts *pānī*/*dūdh*/*roṭī* as
  inherited against *chāy*'s Persian loan, and corrects a tempting false
  cognate between *dūdh* and English *dough*.
- Book compiles clean with XeLaTeX; all four new chapters wired into
  `book-generation.json`.

## 0.9.0 — 2026-08-07

- Added eight schema-v2 lessons in **two** chapters of four, never one of
  eight, so neither chapter exceeds the 12-atom `maxNewAtomsPerChapter` budget
  (both land at 11). Chapter 7, *Four Verbs of the Mind*: **سوچنا** *sochnā*
  (`VERB-THINK`), **سمجھنا** *samajhnā* (`VERB-UNDERSTAND`), **پڑھنا**
  *paṛhnā* (`VERB-READ`), **لکھنا** *likhnā* (`VERB-WRITE`). Chapter 8, *Four
  Verbs Between People*: **لینا** *lenā* (`VERB-TAKE`), **پوچھنا** *pūchhnā*
  (`VERB-ASK`), **مدد کرنا** *madad karnā* (`VERB-HELP`), **پسند** *pasand*
  (`VERB-LIKE-LOVE`). Eleven other tracks already taught these eight; Urdu is
  the twelfth, and `SPINE-SAY-WHAT-I-DO` now omits eight fewer concepts.
- Gave Urdu the argument only Urdu can make. **پوچھنا** *pūchhnā* is Sanskrit
  *pṛcchati* and Persian **پرسیدن** *porsīdan* is the **same** Indo-European
  root — one arriving by inheritance through India, the other borrowed through
  Iran — so Urdu's inherited verb and its loaned **پرسش** *pursish* are cousins
  that met again inside one language. **مدد** *madad* puts a second Arabic
  triliteral root, **م-د-د** "to stretch out, extend", beside Chapter 5's
  **ح-ف-ظ**, and the conjunct verb (noun + *karnā*) is named as the device that
  let Urdu absorb centuries of Persian and Arabic without ever conjugating a
  foreign word. *sochnā*'s inherited **سوچ** *soch* sits beside Arabic **فکر**
  *fikr*, both meaning a thought and, just as ordinarily, a worry.
- Taught **مجھے پڑھنا پسند ہے** — "to me, reading is pleasing" — as a
  dative-experiencer sentence with the liked thing as grammatical subject. The
  liked slot takes any **-nā** infinitive, because an Urdu infinitive doubles as
  a noun, so the payoff closes over the chapter's own verbs by substitution.
  Named as the sixth language in the course whose ordinary word for liking
  refuses to make you the subject, beside Spanish *gustar*, Italian *piacere*,
  Tamil *piḍikkum* and Bengali *bhālo lāge*.
- Kept the etymology honest, per lesson: *sochnā* ← Prakrit *soccadi* ←
  Sanskrit *śocati* "burns, grieves", with *śuc-* flagged as having **no secure
  English cousin**; *samajhnā* ← *sam-* + *budh-* "to wake", the root of
  **Buddha** and, through PIE \**bʰewdʰ-*, of English **bode** and **forbid**;
  *paṛhnā* ← Prakrit *paḍhaï* ← *paṭh-* "to recite aloud", flagged as having
  **no secure Indo-European ancestry**, with the original sense shown alive in
  **نماز پڑھنا**; *likhnā* ← *likhati* "scratches", beside three unrelated
  roots that made the same picture; *lenā* ← *labh-*, levelled by Prakrit
  against *de-* so *lenā* and *denā* rhyme; and *pasand*'s deeper root reported
  as **disputed** rather than asserted.
- Three new letters across eight lessons, one per lesson and none in the last
  four: **چ** (the **ج** body with three dots instead of one), **ھ**
  *do-chashmī he* (no sound of its own, aspirates its right-hand neighbour),
  and **ڑ** (**ر** wearing the same retroflex mark that rides on **ٹ**).
  **لینا** gives **ی** a third value, long *e*. Added `che-vs-jim-dots`,
  `do-chashmi-he`, `retroflex-flap-rre` and `majhul-e` to
  `pronunciation-reference.md`, with the shape-family notes to match.
- Reached back at two cadences. Every lesson names the preceding one to three
  lessons' atoms in `practises.knowledge`, across the chapter seam, so R1 and
  R2 close at zero new lessons; and both payoffs reach several chapters back.
  Rescued sixteen atoms that had never been revisited at any distance —
  `UR-SCRIPT-THIK`, `UR-SCRIPT-BE-LETTER`, `UR-SCRIPT-KYA`, `UR-SCRIPT-JANA`,
  `UR-SCRIPT-KAISE-KAISI`, `UR-SCRIPT-MAIN-HUN`, `UR-SCRIPT-JANNA-DOUBLE-NUN`,
  `UR-ETYMON-KYA-QUESTION-FAMILY`, `UR-ETYMON-KHUSH-PERSIAN`,
  `UR-ETYMON-KHUDA-PERSIAN`, `UR-ETYMON-HAFIZ-ARABIC`, `UR-ETYMON-ANA-TOWARD-GO`,
  `UR-ETYMON-JANNA-KNOW`, `UR-CHUNK-AAP-SE-MIL-KAR`, `UR-REGISTER-INDO-ARYAN-CORE`,
  and `UR-LEX-JANNA`. Track atoms never revisited fell from **24 of 59 (41%)**
  to **12 of 81 (15%)**.
- Declared `chapters.json` chapters 7 and 8. Chapter 7's payoff `UR-C07-likhna`
  assesses 11/11 of the chapter's atoms; chapter 8's payoff `UR-C08-pasand`
  assesses 7/11 = 0.64. Both are above the 0.5 representativeness floor and
  both are closed — no new `chapter-payoff-not-closed` or
  `chapter-payoff-not-representative` findings. Chapters 4 and 5 remain below
  the floor and chapter 1 remains without a capability; that debt is untouched.
- Every lesson routes its letter notes through the canonical
  `## The letters in this word` heading rather than hiding them in prose. That
  block is detachable, so all eight lessons keep a `voice` **core** modality and
  the HL-C41 report puts Urdu at **100% drivable** (nine lessons rescued by a
  detachable segment). The older HL08 manifest still derives `drivable` from the
  conservative whole-lesson modality, where the same honest labelling moves Urdu
  from 92% to 73%; that gap closes when `blockModality` flips true in
  `core/lesson-modality.json`.
- Generated `book/chapters/ch07-mind-verbs.tex` and `ch08-social-verbs.tex` and
  compiled the eight-chapter book with XeLaTeX: **53 pages, zero errors, zero
  `Missing character`, zero overfull or underfull boxes**. Urdu's
  `core/latex-warning-baseline.json` entry is still `null` (never seeded); the
  measured counts are zero across every tracked class. The PIE citation for
  *pūchhnā* uses \**prek-* rather than \**pr̥sḱéti* because U+0325 and U+1E31
  are both absent from Latin Modern Roman.
- No lesson exceeds three new atoms or three new glyphs; all eight sit under
  300 effective seconds. No table appears in any of the eight lessons.

## 0.8.0 — 2026-08-06

- Added five schema-v2 Chapter 6 micro-lessons, the track's first verbs:
  **ہونا** *honā* (`VERB-BE`), **جانا** *jānā* (`VERB-GO`), **آنا** *ānā*
  (`VERB-COME`), **بولنا** *bolnā* (`VERB-SPEAK`), and **جاننا** *jānnā*
  (`VERB-KNOW`). Urdu had taught zero verbs before this; `SPINE-SAY-WHAT-I-DO`
  is now realized rather than wholly omitted, and the track reaches A2.
- Taught the **-نا** *-nā* infinitive ending as a tool rather than a label:
  strip it and the stem falls out, so each new verb costs less than the last.
- Introduced the two simultaneous agreements the Urdu present makes — the
  participle for gender and number, the copula for person — on *jānā*, where a
  real stem exists to hang them on, and reused the frame unchanged on *bolnā*
  and *jānnā* to show the machine is already built.
- Kept the etymology honest per track: *honā* ← Sanskrit *bhavati* ← PIE
  \**bʰuH-* (English **be**, **build**, **future**, **physics**); *jānnā* ←
  *jñā-* ← \**ǵneh₃-* (English **know**, **notice**, **diagnosis**); *ānā* is
  the *ā-* "toward" preverb welded to the same *yā-* root that hardened into
  *jānā*'s **j-**. *bolnā* is flagged as a genuine dead end — the trail stops
  at Prakrit *bollaï* and the Sanskrit *brūte* link is proposed, not settled.
  *jānā* is flagged as having no English cousin at all rather than being given
  a decorative one.
- Placed the Persian and Arabic literary register beside the Indo-Aryan core on
  *bolnā* (homely *bolnā* against Persian-derived *guftagū*), which is also
  where *nastaʿlīq* is named as part of that same Persian inheritance.
- Cross-language comparison stays self-contained: Hindi is described as the
  other standard form of the same spoken language, never assumed as knowledge
  the reader already has.
- One new letter across the whole chapter: **ب**, taught against already-read
  **پ** as a dot-count contrast. **جاننا**'s doubled **ن** is taught as the
  only thing separating "know" from "go". Added `be-vs-pe-dots` and
  `geminate-nun` to `pronunciation-reference.md`.
- All five lessons derive `voice` modality, so Chapter 6 is fully drivable and
  the track's drivable share rises from 90% to 92%.
- Declared `chapters.json` chapter 6 with `UR-C06-janna` as payoff; measured
  representativeness 9/14 = 0.643, above the 0.5 policy threshold. Generated
  `book/chapters/ch06-core-verbs.tex` and compiled the six-chapter book with
  XeLaTeX: zero `Missing character` warnings, zero errors. Sanskrit forms are
  cited in transliteration only, because this book vendors no Devanagari face.

## 0.7.0 — 2026-08-06

- Added `chapters.json`, the HL05 chapter capability ledger, for Chapters 2–5:
  each declares a first-person `canDo`, the spine nodes it realises, and the
  payoff lesson that proves the claim.
- Every `payoff.assesses` list is the payoff lesson's own
  `practises.knowledge` set verbatim — nothing is claimed that the lesson does
  not already practise, and nothing is padded to clear a threshold.
- Chapter 1 is deliberately omitted. Its four lessons are still schema v1 and
  declare no knowledge atoms, so any payoff written for it would be invented
  rather than derived. The gap is left visible as debt.
- Measured payoff representativeness (assessed ÷ chapter-introduced atoms)
  against the 0.5 policy threshold: ch2 3/3 = 1.00, ch3 8/14 = 0.571,
  ch4 6/16 = 0.375, ch5 4/12 = 0.333. Chapters 4 and 5 sit below threshold
  because their word lessons introduce script, cross-lingual, and etymon atoms
  that the consolidating dialogue does not re-exercise; that is a content gap
  for a later revision, not something to paper over here.
- Chapter capability text describes what the shipped book actually renders. The
  Naskh fallback recorded in HL-U01 remains unfixed, so no chapter claims to
  teach Nastaliq letterforms.

## 0.6.0 — 2026-08-04

- Added four schema-v2 Chapter 5 micro-lessons for **خدا**, **حافظ**, spaced
  **خدا حافظ**, and a start-versus-end interaction.
- Secured the Urdu form before using its Persian and Arabic history as a bridge;
  mixed comparison preserves Urdu spacing and Persian joining.
- Extended the exact N+1/N+3/N+7/N+15 ledger through S35, with objective
  activities and a generated five-chapter book.

## 0.5.0 — 2026-08-04

- Added six schema-v2 Chapter 4 micro-lessons for *kaise/kaisī*, respectful
  **āp ... haiṅ**, the first-person **maiṅ ... hūṅ** frame, *ṭhīk*, the polite
  reply, and cumulative practice.
- Kept addressee agreement separate from honorific register, introduced the
  retroflex-aspiration sequence only inside **ٹھیک**, and made the Hindi bridge
  follow independent Urdu-script retrieval.
- Extended the sound-id reference and exact N+1/N+3/N+7/N+15 ledger through
  S31, with objective activities and a generated four-chapter book.

## 0.4.0 — 2026-08-03

- Added five schema-v2 Chapter 3 micro-lessons for **āp/tum/tū**, *kyā*, the
  full name question, the meeting response, and cumulative practice.
- Added objective activity contracts and prerequisite-closed knowledge atoms for
  the migrated Chapter 2 name frame and every new lesson.
- Generated Chapter 3 for the downloadable book from the same canonical lesson
  AST used by Language Ladder and extended the review ledger through S25.

## 0.3.0 — 2026-08-03

- Added the authoritative five-lesson session map with exact N+1, N+3, N+7,
  and N+15 review placements.
- Added an on-demand Urdu pronunciation and script reference that labels the
  current Naskh presentation fallback without weakening the Nastaliq goal.

## 0.2.0 — 2026-08-02

- Added the first downloadable LaTeX edition.
- Published Chapter 1 (greetings and responses) and Chapter 2 (giving your name)
  from the five dependency-ordered starter lessons.
- Added a B1-oriented track roadmap with Urdu-specific extension points.

## 0.1.0 — 2026-08-02

- Added the Urdu shared-spine pilot with five under-five-minute lessons.
