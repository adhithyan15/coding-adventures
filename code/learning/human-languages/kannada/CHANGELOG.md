# Changelog

## Unreleased — a four-minute first greeting (#12242)

- Rewrite `KA-C01-namaskara` from 333 computed seconds to 240 effective
  seconds while retaining the greeting, left-to-right decode, respectful
  gesture, and `namas` + `kāra` etymology payoff.
- Remove the duplicated script explanation and six-language comparison from the
  learner's first five minutes. Add one finger trace and one guided copy so the
  shorter lesson still begins writing gently.
- Reduce the corpus duration backlog from two lessons to one. Kannada is now at
  zero; the independently logged Telugu lesson remains visible.

## Unreleased — Chapters 60-66: Thirty-five more words, round three

Two rounds took Kannada from 47 pre-A1 headwords to 117, and that was still last
of the seven tracks being carried to the 300-word floor together — spanish 118,
malayalam 119, tamil 119, hindi 120, sanskrit 139, telugu 151. These seven
chapters answer with thirty-five more on the same terms: **one new word per
lesson**, and unlimited reuse of everything already taught.

  60 Animals of the House       ಹಸು ಮೇಕೆ ಕೋಳಿ ಕಾಗೆ ಹಕ್ಕಿ
  61 In the Kitchen             ಮೊಸರು ತುಪ್ಪ ಎಣ್ಣೆ ಬೆಲ್ಲ ಸಕ್ಕರೆ
  62 Things People Make         ನೂಲು ಸೂಜಿ ಹಗ್ಗ ಪೊರಕೆ ಕೊಡೆ
  63 Hunger, Sleep and Health   ಹಸಿವು ನಿದ್ದೆ ಜ್ವರ ನೋವು ಆರೋಗ್ಯ
  64 Joining Words              ಆಮೇಲೆ ಕೂಡ ಇನ್ನೂ ಬೇಗ ಹಾಗಾದರೆ
  65 More Good Manners          ವಿನಂತಿ ಅನುಮತಿ ಸಂಕೋಚ ನಂಬಿಕೆ ಸೌಜನ್ಯ
  66 The Field                  ಹೊಲ ಹುಲ್ಲು ಭತ್ತ ಬೆಳೆ ಸುಗ್ಗಿ

Attainment 117 → 152 of 300, a rise of exactly thirty-five. All seven pre-A1
spine nodes are used a third time, one per chapter, in the same order round two
used them, and the chain runs unbroken from ಬೆಂಕಿ at the end of the weather run
to ಸುಗ್ಗಿ at the end of this one — each chapter's second lesson still practising
the previous chapter's last word.

The ramp got gentler again. R1 falls 0.2664 → 0.2642 with its numerator held at
exactly 1140: not one of the thirty-five new atoms misses its first
reinforcement window, so the whole improvement is a larger denominator.

### The collisions, and which check caught each

Every candidate was checked in four classes: in a lesson body, inside an
existing headword, inside a romanization, and — the one no string search
reaches — inside the GLOSS of a morpheme or a stock phrase.

**Inside an existing headword.** ದಾರ, the obvious word for thread, sits whole
inside ದಾರಿ from the river-and-road chapter. ಕಣ, a threshing floor, sits whole
inside ಕಣ್ಣು from the face chapter, which is why chapter 66 ends on ಸುಗ್ಗಿ
instead. ಸಹ for "also" sits inside ಸಹಾಯ; ಮತ್ತೆ is the first word of the headword
ಮತ್ತೆ ಸಿಗೋಣ; ಎಷ್ಟು is inside the headword ನಿನ್ನ ವಯಸ್ಸು ಎಷ್ಟು?.

**Inside a romanization.** ಕನ್ನಡಿ, a mirror, spells *kannaḍa* inside itself, and
that string is on more pages of this book than any other.

**Inside a gloss — the largest class again, six of them, and none reachable by
grep.** This is HL-C207 holding on a fourth track.

  ಅಕ್ಷರ     "a written character"  all 24 writing lessons say "the single character X"
  ಸ್ವಾಗತ     "welcome"              ಪರವಾಗಿಲ್ಲ has been glossed "you're welcome" since the how-are-you chapter
  ಮಾತ್ರ      "only"                 the replies chapter glosses the emphatic -ē as "only that one"
  ಬೆಳಕು      "light"                the morning lesson glosses beḷaku, "light", outright
  ಬಲ        "strength"             the age lesson glosses vayas as "age, youth, vigor, strength"
  ನಿಧಾನ     "slowly"               every guided-practice block says "it once more, slowly"

ಅಕ್ಷರ is the same shape as sanskrit's *shiilam* and telugu's *aksharam*: the
collision is with a phrase twenty-four files independently use, not with any one
lesson. ಸ್ವಾಗತ is the sharpest of the six, because "welcome" and "you're welcome"
are two different acts wearing one English word, and the reader has had the
second since chapter 3.

ಎಣ್ಣೆ was examined under the same rule and KEPT. The friend-lesson glosses the
Sanskrit *snēha* as "oil, unctuousness, smoothness", but that gloss teaches a
Sanskrit etymon, not a Kannada headword; ಎಣ್ಣೆ is Kannada's own and had never been
on the page. The lesson makes the pair explicit rather than pretending it is not
there. ಪ್ರೀತಿ, "affection", is the word that gloss really does block, and it is not
in this tranche.

Three near-misses were kept and turned into teaching instead of discarded, each
because the two words are genuinely distinct in Kannada letters and only look
close in a romanization: *hakki* against *akki*, ಕೊಡೆ against ಕೊಡು, and ಕೂಡ
against ಕೂದಲು. ಹುಲ್ಲು against ಹಲ್ಲು is the closest pair in the whole book — one
vowel sign — and chapter 66 says so out loud rather than hoping.

### Script, and what the taught-glyph filter cost

Every headword is spelled entirely from glyphs the track's own writing lessons
teach, so the tranche adds no script-closure violation and nothing is laundered
through an exposure-exempt headword: Kannada's exempted-glyph count is unmoved at
162 and its violation count at 37. The same filter shaped the prose — ಉಪ್ಪು is
cited in romanization in the jaggery lesson, because ಉ is a letter no writing
lesson has taught. Forward references hold at 511, rule statements at thirty, and
Kannada's cross-chapter prose references at their pinned 80.

### What the chapters actually do

Chapter 61 is a chain rather than a list: ಹಸು gives ಹಾಲು, ಹಾಲು gives ಮೊಸರು and
ತುಪ್ಪ, and ಎಣ್ಣೆ is the one that comes from a ಬೀಜ instead — so the chapter teaches
where five kitchen words come from, not just what they are.

Chapter 63 teaches one sentence shape five times. ನನಗೆ ಹಸಿವು has exactly the
build of ನನಗೆ ಕನ್ನಡ ಗೊತ್ತು from the knowing lesson, and ನೋವು then attaches to
every body part the book has taught, which is the best trade in the tranche: one
new word, a sentence for each part of the body.

Chapter 66 completes a set three rounds in the making — ಭತ್ತ in the field, ಅಕ್ಕಿ
in the sack, ಅನ್ನ on the plate, one plant under three names that Kannada keeps
strictly apart.

The last lesson deliberately makes no claim about the book ending (HL-C201): it
says these five words end where a farming year does, which stays true however
many chapters come after it.

## Unreleased — Chapters 53-59: Thirty-five more words, round two

The first tranche took Kannada from 47 pre-A1 headwords to 82. That left it last
of the seven tracks that are being carried to the 300-word floor together. These
seven chapters answer with thirty-five more, on the same terms as before: **one
new word per lesson**, and unlimited reuse of everything already taught.

  53 The Sky                    ಆಕಾಶ ಬಿಸಿಲು ಚಂದ್ರ ಚುಕ್ಕಿ ಮೋಡ
  54 The Tree                   ಮರ ಕೊಂಬೆ ಎಲೆ ಬೇರು ಬೀಜ
  55 River and Road             ನದಿ ಕೆರೆ ಬೆಟ್ಟ ಹಳ್ಳಿ ದಾರಿ
  56 In the House               ಚಾಪೆ ಬುಟ್ಟಿ ಚಾಕು ಪಾತ್ರೆ ಪೆಟ್ಟಿಗೆ
  57 More of the Body           ಕುತ್ತಿಗೆ ಬೆನ್ನು ತುಟಿ ಭುಜ ಮೂಳೆ
  58 More Short Replies         ಸ್ವಲ್ಪ ಹೆಚ್ಚು ಕಡಿಮೆ ಬೇಡ ಅಷ್ಟೇ
  59 Weather and Ground         ಗಾಳಿ ಮಂಜು ಮರಳು ಕೆಸರು ಬೆಂಕಿ

Attainment 82 → 117 of 300, a rise of exactly thirty-five. All seven pre-A1
spine nodes are used a second time, one per chapter, and the chain runs unbroken
from ಹೂಮಾಲೆ at the end of the welcome chapter to ಬೆಂಕಿ at the end of this run —
each chapter's second lesson still practising the previous chapter's last word.

The ramp got gentler again rather than steeper. R1 falls 0.2869 → 0.2843 with
its numerator held at exactly 1106: not one of the thirty-five new atoms misses
its first reinforcement window, so the whole improvement is a larger denominator.

Every candidate was checked against the whole track before it was written, in
both directions and inside existing headwords. That check is what kept ಸೂರ್ಯ out
of the sky chapter: the ordinary word for the sun is already on the page, whole,
inside ಸೂರ್ಯೋದಯ in the good-morning lesson, so teaching it here would have made
that earlier page point forwards. ಬಿಸಿಲು took the slot instead, which is the more
useful word anyway — a Kannada sentence about the afternoon is far likelier to be
about the sun's heat than about the sun. ಮಳೆ went the same way, already present
inside ಮಳೆಗಾಲ. The corpus figure for forward references holds at its 500 ceiling
and rule statements hold at thirty.

Every headword is spelled entirely from glyphs the track's own writing lessons
teach, so the tranche adds no script-closure violation and nothing is laundered
through an exposure-exempt headword: Kannada's exempted-glyph count is unmoved at
162. The same filter shaped the prose — ಊಟ appears in romanization rather than in
script, because ಊ is a letter no writing lesson has taught yet.

Two sound laws now carry the run rather than sitting in one remark each. ಬೇರು,
ಬೆಟ್ಟ and ಬೆನ್ನು put Kannada's *b* against the family's *v* in three separate
chapters, and ಹಳ್ಳಿ shows the *p* that became *h* written on road signs the
length of the state, where the unchanged *-palli* survives across the border.

Chapter 59 closes on a deliberate refusal: ಮರಳು has ಮರ standing at the front of
it and is not related to it, and the lesson says so, because knowing when an
etymology is a coincidence is part of what this book is teaching.

Fixed, in `roadmap.md`: the Tamil word cited beside ಬರೆ carried a KANNADA LETTER
RA (U+0CB0) in place of the Tamil one, so a Tamil word was rendering with one
letter drawn from the Kannada font. Both fonts load and nothing was missing, so
the build had always exited 0 with zero warnings while printing the wrong glyph.
Found by decomposing every word in the tree into Unicode letter names from the
FILE BYTES and flagging any word naming two scripts (backlog HL-C202).

## Unreleased — Chapters 46-52: Thirty-five everyday words, one per lesson

Kannada was the furthest behind of every track on the pre-A1 vocabulary floor:
47 headwords against the 300 that floor asks for. These seven chapters answer it
with thirty-five, and with nothing else — **one new word per lesson**, and reuse
of everything already taught, unlimited and on purpose.

  46 Things You Ask For             ಹಣ್ಣು ಬಟ್ಟೆ ದೀಪ ಉಪ್ಪು ಪುಸ್ತಕ
  47 The Leg and the Tooth          ಕಾಲು ಹಲ್ಲು ಕೂದಲು ಬೆರಳು ಹೊಟ್ಟೆ
  48 Naming Who Someone Is          ಶಿಕ್ಷಕ ವಿದ್ಯಾರ್ಥಿ ವೈದ್ಯ ರೈತ ಅತಿಥಿ
  49 Answering With More Than Yes   ನಿಜ ಸಾಕು ಖಂಡಿತ ಬಹುಶಃ ಹಾಗೆ
  50 Taking Your Leave              ಈಗ ನಾಡಿದ್ದು ಪ್ರಯಾಣ ಹೊರಡು ಬೀಳ್ಕೊಡುಗೆ
  51 Courtesy Words                 ಕೃತಜ್ಞತೆ ಉಪಕಾರ ಗೌರವ ಆಶೀರ್ವಾದ ವಂದನೆ
  52 Welcoming a Guest              ಬಾಗಿಲು ಕುರ್ಚಿ ರಂಗೋಲಿ ಹೂವು ಹೂಮಾಲೆ

Each chapter sits on one of the seven pre-A1 spine nodes, five lessons sharing
it, and each lesson chains to the one before — so a word introduced by a
chapter's payoff lesson is still being practised two lessons into the next
chapter. That is why the ramp got *gentler* rather than steeper: the R1
reinforcement ratio falls 0.3064 to 0.3034 even though the corpus grew. Not one
of the thirty-five new atoms misses its R1 window.

Every headword was checked against the whole track before it was written, so
none of the thirty-five re-teaches anything and none of them adds a forward
reference — the corpus figure holds at its 500 ceiling, and so does the
thirty-count on rule statements.

The sound law that separates Kannada from its sisters is the spine of the whole
run rather than a remark in one lesson: ಹಣ್ಣು, ಹಲ್ಲು, ಹೊಟ್ಟೆ, ಹೊರಡು and ಹೂವು all
carry an *h* where the family has a *p*, and ಉಪ್ಪು and ಪುಸ್ತಕ say why two
particular *p* sounds survived — one because it is not at the front of its word,
the other because it walked in from Sanskrit after the law had finished.

Chapters 51 and 52 each close on a word that takes apart in front of the reader:
ಆಶೀರ್ವಾದ carries the same *-vāda* as ಧನ್ಯವಾದ from the first page, and ಹೂಮಾಲೆ is a
Dravidian syllable welded to a Sanskrit one.

Chapters 43, 44 and 45 also move from a bare `unicodeScript` to the track's
`kannada-comparisons` script set, matching chapters 6-42 (backlog HL-C200). Their
rendered output is unchanged, because they happen to cite no cousin script today;
what changes is that a comparison table added to them later can no longer drop
its glyphs silently into the Latin font at exit 0.

## Unreleased — Chapter 41: Pointing, and Asking

Six words and the pattern behind them: ಇದು ಅದು ಇಲ್ಲಿ ಅಲ್ಲಿ ಯಾರು ಎಲ್ಲಿ

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

8 recognition segments, one character each, in chapters 6-13: ನ ◌ು ◌್ ◌ಿ ತ ದ ರ ◌ಾ

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

## Unreleased — 38 words a reader can now say

Added `romanization` to 38 lessons that had none, so their headwords become
HL11 *exposure* — something the reader is shown and can use — rather than script
they are stuck on. Each is recovered from the pronunciation the lesson already
gives in its own prose, then checked against the headword's script so a wrong
grab cannot pass. Nothing is transliterated: a mechanical romanization of this
script disagrees with its own authors often enough to teach mispronunciations.

## Chapters 33–34 — The eight verbs, and Kannada as the third Dravidian voice (2026-08-07)

- Added eight lessons under the canonical `VERB-*` tags, in two chapters of
  four. **Chapter 33, "Four Verbs of Mind and Page"**: `KA-C33-yocisu`
  (ಯೋಚಿಸು, `VERB-THINK`), `KA-C33-artha-maadiko` (ಅರ್ಥಮಾಡಿಕೊ,
  `VERB-UNDERSTAND`), `KA-C33-oodu` (ಓದು, `VERB-READ`), `KA-C33-bare` (ಬರೆ,
  `VERB-WRITE`). **Chapter 34, "Four Verbs Between People"**: `KA-C34-tegeduko`
  (ತೆಗೆದುಕೊ, `VERB-TAKE`), `KA-C34-keelu` (ಕೇಳು, `VERB-ASK`),
  `KA-C34-sahaya-maadu` (ಸಹಾಯ ಮಾಡು, `VERB-HELP`), `KA-C34-ishta` (ನನಗೆ ಕನ್ನಡ
  ಇಷ್ಟ, `VERB-LIKE-LOVE`). Sequences 670–740, schema v2, one prerequisite chain
  running out of `KA-C32-gottu`.
- **Two chapters, not one.** Twenty new atoms, ten per chapter, against
  `maxNewAtomsPerChapter: 12` — a single eight-lesson chapter would have
  doubled the budget. Each chapter carries its own `canDo` and its own payoff
  closing over its own four lessons.
- **Coverage.** Kannada goes from **6 to 14** of the shared spine's core forty
  verbs. Each of these eight concepts was already taught by fifteen other
  tracks; Kannada makes each a sixteen-way join, and the third Dravidian voice
  in it after Tamil and Telugu.
- **The Dravidian comparison, now that three sisters are in the corpus.**
  ಬರೆ *bare* is Proto-Dravidian *\*warV-* "to scratch, to draw lines" (DEDR
  5263), the same word as Tamil வரை *varai* and Telugu (వ)రాయు *(v)rāyu*, all
  still meaning **draw** as readily as **write** — and Tamil's *v* against
  Kannada's *b* is the second of the two front-of-the-word laws, set beside
  *p* → *h* (*pattu*/*hattu*, *pasir*/*hasiru*) in one three-column table.
  ಕೇಳು *kēḷu* means **both** "ask" and "hear," because the root is reconstructed
  with both senses (DEDR 2017) and Tamil, Malayalam and Tulu carry the pair;
  Telugu is the sister that split them into *vinu* and *aḍugu*. The lesson
  names the branch split — Kannada, Tamil and Malayalam on one branch, Telugu on
  the next — and then **limits** the claim honestly, because the root reaches
  Telugu's own branch-mates Gondi and Kui: Telugu dropped a word rather than
  never having had one. ತೆಗೆದುಕೊ *tegeduko* cuts the other way, and is allowed
  to: its *tege* files with Telugu *tīyu* (DEDR 3407) and its *koḷḷu* with Tamil
  *koḷ* and Telugu *konu* (DEDR 2151), so Kannada matches **Telugu** on both
  roots and Tamil on only one — descent does not decide every word.
- **The borrowing thread, per word rather than per topic.** ಯೋಚಿಸು is the noun
  ಯೋಚನೆ (Sanskrit योजना *yojanā*, from युज् *yuj* "to yoke" — the root of *yoga*
  and English *yoke*) plus **‑ಇಸು**, the verb-making suffix Chapter 9 already
  used on *kṣamā*; beside it the inherited ನೆನೆ *nene* (DEDR 3683 *\*nen-ay* "to
  think") survives with the job of remembering, which is the very root Tamil
  promoted to its everyday நினை *ninai*. ಓದು runs the other way: it is the
  inherited word (DEDR 1052 *\*ōtu* "to recite, read") left in its ordinary job,
  where Tamil's cognate ஓது narrowed to chanting and everyday reading passed to
  படி *paṭi*. ಸಹಾಯ is Sanskrit सहाय, most probably *saha* "with" + *aya*
  "going" — "one who goes with" — welded to the native ಮಾಡು, a Sanskrit-plus-
  Dravidian hybrid of the same kind Chapter 20's Persian-plus-Sanskrit ಹವಾಮಾನ
  already was.
- **ಇಷ್ಟ, and the claim held down to what is true.** The word is Sanskrit इष्ट,
  past participle of इष् *iṣ*, from the Indo-European root behind English *ask*
  — so the chapter carries two unrelated asking-words. But the **frame** is not
  borrowed: inherited ಬೇಕು *bēku* takes the same dative subject and answers
  Tamil வேண்டும் *vēṇṭum* through the same *v* → *b* law. The cross-language note
  deliberately carries **no census**: a count of how many tracks build liking on
  an experiencer is stale the moment the next tranche lands. The lesson names
  Spanish *me gusta* and Italian *mi piace* beside Tamil *piḍikkum* and says the
  true, permanent thing — Romance and Dravidian borrowed nothing from each other
  here, and arrived at one shape independently.
- **Reinforcement at two cadences, and the orphan count halved twice over.**
  Every lesson practises atoms from the immediately preceding one to three
  lessons, across the chapter seam (`KA-C34-tegeduko` practises Chapter 33's
  *bare*, *ōdu* and *arthamāḍiko*). Each payoff reaches back several chapters:
  `KA-C33-bare` rescues `KA-LEX-C32-BAA-01`, `KA-LEX-C32-TINNU-01`,
  `KA-GRAMMAR-C32-NOODU-02`, `KA-LEX-C07-NUMBERS-6-10-01`,
  `KA-ETYMON-C07-NUMBERS-6-10-02`, `KA-ETYMON-C22-HASIRU-HALADI-01` and
  `KA-LEX-C20-HAVAMANA-02`; `KA-C34-ishta` rescues all three Chapter 6
  dative-subject atoms plus both `KA-C32-gottu` atoms. Measured on the corpus:
  Kannada's never-revisited atoms fall from **20 of 79 to 9 of 99**.
- **Drivability held.** All eight derive `voice`; Chapters 33 and 34 are fully
  drivable, keeping the whole 32–34 verb arc car-safe. No script blocks — the
  canonical `## The letters in this word` heading classifies as a `script`
  block and is **not** detachable, so a lesson carrying one derives `sight`
  (this is why Tamil's and Telugu's own C33–C34 are undrivable). The letters
  each word needs are taught in a `Sounds you'll need` block instead, exactly as
  Chapter 32 does. Every table is three columns at most; the whole-word sight-cue
  scan is clean.
- Wiring: `curriculum.json` gains `KA-PATH-026` on `SPINE-SAY-WHAT-I-DO` with
  the two required extensions `KA-EXT-026-MIND-VERBS` and
  `KA-EXT-027-SOCIAL-VERBS`, and that node's `omits` ledger drops the eight
  concepts now realised (36 → 28). `chapters.json` gains Chapters 33 and 34,
  each payoff assessing **10 of its chapter's 10** introduced atoms (1.00,
  against the 0.5 floor). `core/book-generation.json` gains both targets; the
  generated `ch33-mind-verbs.tex` and `ch34-social-verbs.tex` are `\input` from
  `book.tex`.
- Verified: `tests/integration.test.ts` and `tests/cli.test.ts` 19/19 green;
  `check:modality`, `check:books` and `check:narration` clean; all eight lessons
  under the duration budget (computed 290–299s against the 300s threshold), and
  the track has zero duration violations. The book compiles under XeLaTeX at 120
  pages with **zero** "Missing character" reports; build artefacts removed. A
  throwaway glyph probe first confirmed that Latin Modern Roman itself lacks
  ṁ, ḻ, ṉ, ṟ, ḱ, ʰ, ʼ and the ring-below — the Kannada preamble's
  `
ewunicodechar` fallbacks are what make the first four safe in this book.

## Chapter 32 — The Core Verbs (2026-08-06)

- Added six lessons under the **canonical** `VERB-*` concept tags, the track's
  first: `KA-C32-iru` (ಇರು, `VERB-BE`), `KA-C32-hoogu` (ಹೋಗು, `VERB-GO`),
  `KA-C32-baa` (ಬಾ, `VERB-COME`), `KA-C32-tinnu` (ತಿನ್ನು, `VERB-EAT`),
  `KA-C32-noodu` (ನೋಡು, `VERB-SEE`) and `KA-C32-gottu` (ಗೊತ್ತು, `VERB-KNOW`).
  Sequences 610–660, schema v2, in a single prerequisite chain.
- **Why they were needed.** The track already taught four verbs — *mātanāḍu*,
  *iru*, *hōgu*, *kelasa māḍu* — but every one of them under a Kannada-only tag
  (`KA-VERB-IRU`, `KA-VERB-HOOGU`, …). A namespaced tag joins nothing across
  languages, so on the cross-language measurement Kannada covered **zero** of
  the canonical forty core verbs. It now covers six.
- **One idea per lesson, each on the word that needs it.** The three slots,
  stem + tense + person, so *iruttēne* is literally be + present + I and the
  last bead already means "I" (*iru*). The **p → h law** — Kannada alone among
  the four literary Dravidian languages softened old word-initial \*p- to h-,
  so Tamil *pōgu / pattu / pāl / peyar* answer Kannada *hōgu / hattu / hālu /
  hesaru* while Telugu and Malayalam keep the p, with Old Kannada's surviving ಪ
  spellings as the evidence (*hōgu*). The form you call a verb by against the
  form the beads attach to, *bā* but *baru-*, which finally opens Chapter 4's
  ಹೋಗಿ ಬರುತ್ತೇನೆ — plus Kannada's second initial-consonant habit, \*v- → b-
  (*bā*). The tense bead, and the fact that everyday Kannada keeps **no**
  separate future, leaving *-utt-* to cover both and a word like *nāḷe* to
  settle it (*tinnu*). The person bead, gendered only in the third person, and
  the genuine four-way split in the everyday see-word — *nōḍu*, *pār*, *cūḍu*,
  *kāṇuka* (*nōḍu*). And the closing symmetry: ಗೊತ್ತು is not a verb at all, so
  it has no person slot and the knower must ride outside it in the dative
  (*gottu*).
- **Dravidian discipline held.** No Indo-European cognates invented for native
  Kannada words; every cousin cited is a Dravidian sister with its form
  supplied (Tamil *iru / pōgu / vā / tiṉ / pār / teḷi / aṟi*, Telugu *uṇḍu /
  pōvu / vaccu / tinu / cūḍu / teliyu / telusu*, Malayalam *irikkuka / pōkuka /
  varuka / tinnuka / kāṇuka / aṟiyuka*), and unsettled reconstructions are
  flagged rather than asserted — *nōḍu*'s kinship with Tamil *nōkku* is given
  with the hedge it deserves.
- **Drivability held deliberately.** All six derive `voice`. No script blocks,
  no sight cues, and the two tables are three columns wide at most. The letters
  each word needs are taught inline in its "Sounds you'll need" block, never as
  a gated reading course.
- Wiring: `curriculum.json` gains `KA-PATH-025` on `SPINE-SAY-WHAT-I-DO` — the
  track's first content on that node — with the six lessons attached as the
  required `KA-EXT-025-CORE-VERBS` extension, and that node's `omits` ledger
  drops the six concepts now realised. `chapters.json` gains a Chapter 32 entry
  whose payoff, `KA-C32-gottu`, assesses 7 of the chapter's 12 atoms (0.58,
  above the 0.5 floor). `core/book-generation.json` gains the Chapter 32
  target; the generated `ch32-core-verbs.tex` is `\input` from `book.tex`.
- Verified: integration suite 16/16 green, `check:modality` / `check:books` /
  `check:narration` all clean, and every lesson under the duration budget
  (computed 271–291s against the 300s threshold). The book compiles under
  XeLaTeX with zero "Missing character" reports; build artefacts removed.

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
  chapter's **last lesson by `sequence`**, which for these single- and
  double-lesson chapters is also the lesson that recombines the chapter's
  material in its Guided Practice block.
- **Chapters 1–5 are deliberately absent.** Their lessons are still schema v1,
  carry no `practises.knowledge`, and have no `core/book-generation.json`
  target to derive a title from. A payoff written for them would be invented,
  not derived; the gap is left visible as honest debt rather than stubbed.
- Thinnest payoffs, for the representativeness gate that lands next: Chapter 20
  covers 2 of the 4 atoms its two lessons introduce (exactly the 0.5 threshold,
  because the weather lesson closes a chapter that also teaches 11–20), and
  Chapter 6 covers 4 of 6.

## Warning-free complete book (2026-08-03)

- Added explicit static-font faces for every comparison script and readable
  Unicode bookmark fallbacks, eliminating all font-shape and Hyperref warnings.
- Made the five handwritten recap labels unique, shortened only the titles that
  exceeded header or bookmark widths, and added natural page bottoms.
- Adjusted a small set of canonical multilingual examples at durable line-break
  boundaries; regenerated chapters and source hashes remain shared with
  Language Ladder rather than diverging into book-only prose.
- The forced 96-page XeLaTeX build now reports zero missing glyphs, overfull or
  underfull boxes, duplicate destinations, Hyperref warnings, LaTeX warnings,
  and font warnings. All pages were inspected again with no clipping,
  collision, accidental blank page, or leaked schema metadata.

## Canonical Chapters 6–31 in the book (2026-08-03)

- Migrated all thirty Kannada lessons after Chapter 5 to the strict schema-v2
  curriculum contract: canonical spine nodes, unique prerequisite-safe
  sequence, explicit sub-five-minute budgets, typed block boundaries, and
  closed knowledge introductions and assessments.
- Generated twenty-six LaTeX chapters from those canonical lessons instead of
  copying app content into a separate book source. The committed source-hash
  manifest is independently checked against Language Ladder for Chapters 6–31.
- Added a reusable Kannada comparison-font set for Kannada, Tamil, Telugu,
  Malayalam, Devanagari, and Arabic-script examples. The 96-page PDF has zero
  missing glyphs and preserves the full 33-entry top-level chapter outline.
- Rendered and inspected all 96 pages, including the dense case, number,
  calendar, PIE-etymology, and register sections. No clipping, collision,
  accidental blank page, or leaked schema metadata was found.
- The expanded artifact's cleanup baseline is nine overfull boxes, three
  underfull horizontal boxes, seven underfull vertical boxes, four duplicate
  practice labels, 106 Hyperref warnings, and nine font warnings. `HL-B25`
  tracks that publication-hygiene pass separately.
- The single all-books publication gate still compiles all twenty downloadable
  volumes successfully.

## Sub-five-minute lesson remediation (2026-08-02)

- All thirty-seven Kannada duration violations are resolved. Thirty-six lessons
  already computed below five minutes and now declare an honest four-minute
  budget without changing their teaching content.
- The genuinely long Chapter 6 lesson becomes two prerequisite-ordered steps:
  learn **-ಗೆ/-ಿಗೆ/-ಕ್ಕೆ**, **ನಾನು → ನನಗೆ**, and the family *k → g* history;
  then contrast transparent Dravidian suffix stacking with fused Latin endings
  before applying the dative in **ನನಗೆ ಕನ್ನಡ ಗೊತ್ತು**.
- The rewritten suffix lesson computes to 205 seconds and the new stacking
  lesson to 196. The support lesson brings the Kannada track to 60 lessons with
  zero unknown prerequisite ids.
- A forced book build succeeds at 29 pages with no missing glyphs. Canonical
  lessons continue through Chapter 31 while the book stops at Chapter 5
  (`HL-B24`); existing layout, bookmark, duplicate-label, and font warnings are
  tracked in `HL-B25`; roadmap and session-map drift is tracked in `HL-M03`.

## Chapter 6 — Case endings, and the sentence with no subject

- **Chapter 6 authored** (`KA-C06-dative-ge`, `-dative-stacking`,
  `-dative-subject`): the track's first
  **case ending** — reviewing Ch.2/3/5 via `reviews_of`.
- **-ಗೆ** (`KA-C06-dative-ge`): the dative "to/for," taught as the doorway to
  **agglutination**. Kannada **adds** a suffix carrying **one** meaning with the
  **seam visible** (*hesar* + *ige*), where a Latin ending like *-īs* **fuses**
  case+number+declension inseparably. The distinctive Kannada content is a
  **sound-history** point: its **g** is the shared Dravidian dative ***k* softened
  between vowels** — a voicing Kannada carried **into the dative** where its
  sisters kept the hard consonant — while the
  *-kke* form (*kelasakke*) preserves the original hard doubled consonant. Includes
  *nānu* → **ನನಗೆ** *nanage*.
- **Stacking** (`KA-C06-dative-stacking`): the visible Kannada noun-plus-case
  seam and one-suffix/one-job principle now get their own micro-lesson before
  the contrast with Latin *-īs*, which fuses case, number, and declension.
- **ನನಗೆ ಕನ್ನಡ ಗೊತ್ತು** (`KA-C06-dative-subject`): "I know Kannada" — literally
  "**to-me Kannada known**," with **no nominative subject** (contrast Ch.5's *nānu
  kannaḍa mātanāḍuttēne*), and with the further observation that ***gottu* isn't an
  action verb at all** — so nothing in the sentence is *doing* anything. Explains
  the **dative-subject** rule with English's "**methinks**" as the bridge.
- **The Dravidian family thread**, new in this chapter: *-ge / -ukku / -ku / -ikku*
  are visibly the **same suffix** across the four sisters, all building "I know X"
  the same subjectless way.
- Taxonomy: namespaced `KA-CASE-DATIVE`, `KA-DATIVE-SUBJECT`.

## Chapters 3–5 — How-are-you, Farewells, First Verbs

- Three new chapters carry Kannada to Chapter 5, matching the leading tracks' arc.
  One word per lesson, atom-first, Kannada script inline; every root traced
  (`lessons/KA-C0{3,4,5}-*`, `book/chapters/ch0{3,4,5}-*.tex`). Concept tags reuse
  the universal `HL01` taxonomy; verbs namespaced (`KA-VERB-*`). The
  Sanskrit-borrowing-yet-Dravidian-grammar thread runs throughout.
- **Ch. 3 — How Are You**: *hēge* (how; the native *ē-/yā-* questions) → *nīvu
  hēgiddīrā?* (the verb *iru* "to be," the same word Tamil uses) → *nānu* (I ←
  Proto-Dravidian, unrelated to *me*) → *cennāgi* (well ← *cennu* "beautiful";
  *nānu cennāgiddēne* "I'm well") → *paravāgilla* ("no harm" = you're welcome, on
  the Dravidian *illa* shared with Tamil/Malayalam — where Telugu uses *lēdu*) →
  practice.
- **Ch. 4 — Farewells**: *hōgu*/*bā* → *hōgi baruttēne* ("I'll go and come back,"
  tabled across the family) → *nāḷe sigōṇa* (see you tomorrow; *nāḷ* "day" shared
  with Tamil, the "let's ___" *-ōṇa*) → *matte sigōṇa* (we'll meet again; native
  *sigu*, where Tamil borrowed Sanskrit *sandi*) → practice.
- **Ch. 5 — First Verbs**: *mātanāḍu* (← *mātu* "word") → *nānu kannaḍa
  mātanāḍuttēne* (I speak Kannada; no 1st-person gender) → *iru* (to be/stay/live;
  the postposition *-alli*) → *kelasa māḍu* (to work; noun + *māḍu*, the twin of
  Hindi's *karnā*) → practice. Book compiles clean with XeLaTeX (0 missing chars,
  0 undefined refs).

## Chapter 2 — Introducing Yourself

- New chapter around the introduction dialogue (*nanna hesaru … / nimma hesaru
  ēnu?*), atom-first, Kannada inline (`lessons/KA-C02-*`,
  `book/chapters/ch02-introductions.tex`). Every atom traced:
  - **ಹೆಸರು** hesaru ("name") ← Proto-Dravidian *\*pesar*, via Kannada's **p→h**
    shift — cousin of Tamil *peyar*, **not** the Indo-European *name/nām*.
  - **ನನ್ನ** nanna ("my") ← *nānu* ("I").
  - **ನನ್ನ ಹೆಸರು …** — **"my name is…"**; the **zero copula** (no "is").
  - **ನೀನು / ನೀವು** nīnu/nīvu — "you," familiar/respectful; respect by plural.
  - **ಏನು** ēnu ("what") ← Dravidian question-stem *\*yā-/\*e-*.
  - **ನಿಮ್ಮ ಹೆಸರು ಏನು?** — **"what's your name?"**
  - **ಸಂತೋಷ** santōṣa — "pleased to meet you," a **Sanskrit** loan (vs. Tamil's
    native *magiḻcci*) — the Kannada-borrows-Sanskrit thread continued.
  - **practice** — the whole dialogue.
- Book compiles clean with XeLaTeX.

## Chapter 1 — Greetings (Kannada script taught inline)

- New Kannada track on the HL00 framework — the second of the four Dravidian
  tracks, after the Tamil anchor. One word per lesson, slug ids, atom-first,
  derivations shown, LaTeX book. Uses the **vendored** Noto Sans Kannada font
  (loaded by relative `Path=` so local and CI builds match; script shaped via
  `Script=Kannada`, no polyglossia language module needed).
- **No reading course.** Per `HL00`'s inline-letters rule, Kannada is taught
  *inside* each word lesson: a *"The letters in this word"* section introduces
  exactly the letters that word needs, so reading and meaning arrive together.
- Chapter 1 (`lessons/KA-C01-*`), greetings + conversational glue:
  - **ನಮಸ್ಕಾರ** namaskāra ("hello," **Sanskrit** *namas* + *kāra*) — teaches
    the inherent *a*, vowel signs, and the ಸ್ಕ *ottakṣara* (stacked conjunct).
  - **ಧನ್ಯವಾದ** dhanyavāda ("thanks," **Sanskrit** *dhanya* + *vāda*) — the
    aspirated ಧ and the ನ್ಯ conjunct.
  - **ಹೌದು** haudu ("yes," native) — the au/u vowel signs; verb-echo "yes."
  - **ಇಲ್ಲ** illa ("no / isn't," native, root *il*) — cognate with Tamil
    *illai*; negation by a negative existential verb.
  - **ಸರಿ** sari ("okay," native) — the *same word* as Tamil *sari*, one word
    in two scripts.
  - **practice** — recap + the *hōgi baruttēne* / *hōgi banni* farewell (the
    same "go and come back" logic as Tamil's *pōy varugiṟēṉ*).
- The recurring thread: **Kannada borrowed Sanskrit for greetings/politeness
  (namaskāra, dhanyavāda) but kept native Dravidian for the everyday grammar
  (haudu, illa, sari)** — each lesson carries an "Across the family" cognate box
  (English / Sanskrit / Hindi / Tamil / Telugu / Malayalam), every form
  supplied so nothing is assumed. Book compiles clean with XeLaTeX.
