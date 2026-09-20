## Unreleased — counting one word at a time, and a script ladder that arrives before the words it reads

Chapter 7 taught all ten numbers in **two** lessons, each opening on a
five-row, four-column reveal table. It is now **ten** lessons — one number
word each, one new knowledge atom each — and the ten Kannada digits ೦–೯ ride
behind them as ten one-glyph script lessons spread across chapters 9–18.

Separately, the script ladder was resequenced. It used to start at chapter 6,
which meant the first thirty-three lessons printed Kannada that no lesson had
introduced. Every letter lesson now sits at the last position that still
precedes its first consumer **and** still follows the word it cites.

Measured (`measureScriptClosure`, `measureContinuity`, `measureRamp`):

- closure violations **30 → 10**
- glyphs shown but never taught **10 → 0**
- headwords with no romanization **14 → 0**
- distinct headwords **204 → 212**
- glyph spikes **6 → 4**; chapter atom spikes **0 → 0**; forward references **13 → 13**

### Chapter 7 — one headword per lesson

`KA-C07-numbers-1-5` and `KA-C07-numbers-6-10` are replaced by
`KA-C07-ondu`, `-eradu`, `-muuru`, `-naalku`, `-aidu`, `-aaru`, `-eelu`,
`-entu`, `-ombattu`, `-hattu`. Each teaches one word and one atom, and each
carries exactly one of the chapter's ideas rather than bundling five:

  ondu     Telugu breaks rank (okaṭi)        aaru     Tamil's rare ṟ flattens
  eradu    Tamil ṇṭ softens to ḍ             eelu     Tamil ḻ hardens to ḷ
  muuru    the long ū the family kept        entu     where the family parts
  naalku   the virama holds ಲ್ಕ together      ombattu  the anusvara's third face
  aidu     the softening, a second time      hattu    the p → h shift

`KA-LEX-C07-NUMBERS-6-10-01`, `KA-ETYMON-C07-NUMBERS-6-10-02` and
`KA-ETYMON-C07-NUMBERS-6-10-03` are kept and re-homed on the lessons that
still teach them, so chapters 33 and 34 need no edit to their `requires`.

The chapter's two wide tables are gone with the lessons that held them, which
is why the corpus-wide narration refusal ceiling drops 60 → 58.

### The digits, spread rather than blocked

Ten digits in one chapter would be an alphabet block, and chapter 7 has an
atom budget of twelve. So `KA-S140`–`KA-S149` teach ೧ ೨ ೩ ೪ ೫ ೬ ೭ ೮ ೯ ೦ one
per chapter across chapters 9–18, each riding a number word the reader has
already been saying.

The closure module credits a script lesson with **every** glyph in its body, so
each digit lesson was measured for what it actually credits. Seven credit only
their own digit. Three credit one letter as well — `KA-S140` (೧) is the first
teaching lesson to print ಒ, `KA-S142` (೩) the first to print ೂ, `KA-S144` (೫)
the first to print ಐ — because those three vowels appear in no word before
chapter 7 and the letter ladder has no lesson for them. That is real debt: the
digit lessons are carrying three letters they do not teach. It is recorded in
the backlog rather than hidden by removing the words, which would have pushed
`neverTaughtGlyphs` back up to 3.

### The ladder, resequenced

The two letter chains kept their relative order except where a letter's word
did not yet exist. `KA-S110-letter-ba` moved to chapter 4, because ಬ appears
in no word before ಹೋಗಿ ಬರುತ್ತೇನೆ; `KA-S114-vowel-sign-o` moved to chapter 6 for
the same reason with ಗೊತ್ತು. `KA-S111-vowel-sign-e` becomes the chain head.
Ten letter lessons now land in chapters 1–5, whose book chapters are
handwritten, so each is embedded there with a `canonical-insertion` marker and
declared in `core/book-generation.d/handwritten.d/kannada-000N.json`.

Example lists were trimmed wherever a letter lesson said "you already say
these" about a word the reader had not met — that claim was false at the new
positions, and fixing it is what keeps forward references at their 13 baseline
rather than 42.

### What did NOT move, and why

**Pre-A1 vocabulary is unchanged at 152/300.** Chapter 7 is staged `A1` in
`KA-EXT-011-LANGUAGE-SPECIFIC`, matching Tamil, Telugu, Malayalam and Hindi,
whose counting nodes are all `A1` too. Ten new headwords therefore land at A1,
not pre-A1. Restaging the chapter would have moved the number by relabelling
rather than by teaching, and would have broken parity with four sibling
tracks, so it was not done. See the backlog entry.

**Chapter payoff representativeness got worse**, from 3 payoff surprises to 9:
chapters 1–5 are schema-v1 with empty `assesses`, and they now contain script
lessons whose atoms no payoff claims. That is recorded debt, not a regression
in the teaching.

### The ten violations that remain

`KA-C01-illa`, `KA-C01-practice`, `KA-C02-practice`, `KA-C02-santosha`,
`KA-C03-cennaagi`, `KA-C04-hoogu`, `KA-C04-naale-sigona`, `KA-C05-iru`,
`KA-C05-kelasa-maadu`, `KA-C09-kshamisi`. Their blocking glyphs, each traced
to the lesson that first teaches it:

- **ಅ** (`KA-C02-practice`, `KA-C05-iru`, `KA-C05-kelasa-maadu`) and **ಭ**
  (`KA-C02-santosha`, `KA-C09-kshamisi`) exist only inside the 38-glyph grid
  lessons `KA-S124-letter-pa` (ch. 15) and `KA-S122-letter-ca` (ch. 13).
  Moving a 38-glyph grid to chapter 2 would be an alphabet block, so the real
  fix is to split the grids into single-letter segments.
- **ಓ** (`KA-C04-naale-sigona`) waits for `KA-S126-letter-u` at chapter 16.
- **ಬ** (`KA-C04-hoogu`) cannot be fixed by placement at all: ಬ's first word,
  ಬಾ, is inside the very lesson that needs it, and no earlier word contains
  the letter.
- the rest — ದ, ೆ, ಿ, ೇ, ಂ, ಷ, ಣ, ತ, ೋ, ಟ, ೂ — are within-chapter ordering,
  and every one of them is already at the earliest position that still follows
  the word its own lesson cites. Moving them further up would trade a closure
  violation for a forward reference one-for-one.

