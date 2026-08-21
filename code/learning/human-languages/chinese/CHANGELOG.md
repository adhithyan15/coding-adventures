# Changelog — Mandarin Chinese track

All notable changes to this track. The format follows the other human-language
tracks: one entry per authored tranche, describing what was added and why.

## [Unreleased]

### Added — transitional HSK 3.0 four-skill assessment contract (#12367)

- Replaced the stale pre-2021 six-band mapping with a dated GF0025-2021 /
  HSK 3.0 target that distinguishes the 2026 Levels 1–6 trials from the
  operational combined Levels 7–9 exam.
- Labelled every CEFR correspondence project-defined rather than presenting it
  as an awarding-body claim, and added companion skill papers wherever a live
  external form does not independently score the curriculum's four skills.
- Contracted the full pre-A1-to-C2 gentle writing ladder, timed mocks,
  calibration, and book-only human validation without claiming the current
  chapters meet the target.

### Changed — writing now starts with the first lesson

The opening tone lesson now asks for one model-visible pass over the four pinyin
tone contours. It does not ask the learner to recall a shape, and the first
Chinese character remains the following two-stroke lesson.

### Added — Chapter 2, the components, one glyph per lesson (HL-C209)

Seven writing lessons, each teaching **exactly one** glyph, in the order that
lets the reader build the greeting rather than memorise it:

    人  ->  亻  ->  尔  ->  你          (person, squashed person, sound half, assembly)
    女  ->  子  ->  好                  (woman, child, assembly)

Two of the seven are **assembly** lessons — they introduce no new stroke at all.
They take two pieces the reader wrote a moment earlier and show what happens when
the pieces share a square. This is the join HL14 §1.1 names as missing: the moment
a reader stops recognising a character and starts *decoding* one.

**What moved.** `scriptLessons` 0 -> 7, `taughtGlyphs` 0 -> 7,
`neverTaughtGlyphs` **7 -> 0**. Corpus-wide, `tracksTeachingNothing` falls 8 -> 7:
Chinese was one of eight tracks that showed a reader a writing system and never
taught a letter of it.

**What did NOT move, stated plainly.** The four closure *violations* stand at 4.
Every glyph is now taught, but the four lessons that use them sit in Chapter 1,
ahead of the chapter that teaches them, so in reading order the debt is unchanged.
Teaching the glyphs and teaching them *first* are two different fixes; this is the
first. See HL-C210 in `BACKLOG.md` for the second.

**Why a new chapter rather than more of Chapter 1.** Folding seven atoms into
Chapter 1 took it to 18 against a per-chapter budget of 12 — a measured ramp
regression. HL-C167's rule is to change the content, not re-seat the number, and
the content change that fits is a second chapter. Chapter 1 keeps its spoken
greeting, drivable end to end; Chapter 2 is the pen.

Three things the authoring got wrong first, all caught by gates rather than by
reading:

- The stroke-order **citation carried its Chinese title**, pulling eleven untaught
  glyphs into a lesson whose whole job is to teach one. HL-C191 already says
  cross-script citations must be romanised; this is why.
- "if you are **used to** an alphabet" tripped the info-dump rule-statement
  detector. A false positive — and rephrasing is still the right move, because
  arguing with a gate in prose is how prose gets worse.
- The chapter title carried the target script, which drifts against the generated
  `\zh{...}` wrapper.


### Added — the track, and Chapter 1 (HL-C39)

Mandarin joins as the twenty-first registered track and the first outside the
Indo-European and Dravidian families. Registered in `core/languages.json` as
`chinese` — family Sinitic, script `chinese`, status `active`.

**Chapter 1 — "Nǐ Hǎo: Hello, Character by Character"**, seven schema-v2 lessons,
all under 300 effective seconds, at most three new knowledge atoms each, eleven in
the chapter against a budget of twelve:

1. `ZH-C01-tones` — **mā má mǎ mà**. One syllable, four pitches, four unrelated
   words. Establishes that pitch is part of a Mandarin word.
2. `ZH-C01-ni` — **你** *nǐ*, "you". The character as a block of named components
   (亻 + 尔), and the word itself.
3. `ZH-C01-hao` — **好** *hǎo*, "good". Introduces 女 beside 子 and, with it, the
   component gloss as this track's replacement for the cousin web — flagged as a
   traditional gloss rather than settled palaeography.
4. `ZH-C01-nihao` — **你好**, the greeting. Realises `GREETING-HELLO`. Its Grammar
   Lens establishes that a word may be more than one character.
5. `ZH-C01-tone-sandhi` — third-tone sandhi. Written *nǐ hǎo*, spoken *ní hǎo*.
6. `ZH-C01-hao-fond` — **好** *hào*, "to be fond of". One character, two words,
   told apart by tone alone.
7. `ZH-C01-practice` — the two-line meeting, and the chapter's HL05 payoff, which
   assesses ten of the chapter's eleven atoms.

The tone facts were originally folded into the 你 lesson. That lesson measured 352
effective seconds — over the five-minute contract — because a first Chinese word
lesson has to carry a sound system, a character, its components, and a word, where
a first Spanish lesson carries a word and no new letters at all. HL08 says the fix
for an over-budget lesson is to split it, not to waive it, so the tone material
became `ZH-C01-tones` and both halves now sit comfortably inside the budget.

Also added: `curriculum.json` with a ledger entry for all eleven spine nodes and
five typed local extensions; `chapters.json` with a first-person `canDo` and a
payoff assessing ten of the chapter's eleven atoms; `roadmap.md`,
`session-map.md`, `pronunciation-reference.md`, `README.md`; and the XeLaTeX book,
whose Chapter 1 is generated from the canonical lessons via
`core/book-generation.json`.

### Changed — the data model grew one field group, in the data layer

`ScriptData` gained `tones?: Tone[]` and `toneSandhi?: ToneSandhiRule[]`, and
`data/scripts/chinese.json` now populates both.

This is the only schema change the track needed, and it was needed because of a
real gap rather than convenience. `Letter.tone` already existed and records which
tone a *character* carries. It cannot express two other things:

- **the inventory** — what tone 3 *is* (contour 214, low and creaky) is a property
  of the sound system, not of any glyph that happens to carry it;
- **sandhi** — a rule that changes a syllable's pitch because of the syllable
  *after* it, while the characters and the printed pinyin stay identical. That is
  a fact about a sequence. There is no `Letter` for it to hang on.

Every previously modelled script encodes pronunciation *segmentally*, and a
segment is always attached to a glyph. Tone is suprasegmental, which is why the
existing shape did not stretch.

The **lesson frontmatter** schema needed no change: `sounds: [tone-3,
tone-sandhi-third]` points into `pronunciation-reference.md` exactly as
`sounds: [rtl, long-a]` does for Persian. Worth recording as a negative result —
the lesson-level part of HL00's "sounds you'll need" design did generalise.

### Changed — one new lesson type, `pronunciation`

`EXEMPT_TYPES` in `human-language-data/src/constants.ts` gained `pronunciation`,
used by `ZH-C01-tones` and `ZH-C01-tone-sandhi`.

Every earlier track's pronunciation facts are segmental and therefore attach to a
letter, so they live inside the word lesson that first uses that letter and never
need a lesson of their own. A tone attaches to a syllable, and sandhi attaches to
a *sequence* of syllables. Neither fits inside a word lesson without pushing it
over the duration budget, and neither is grammar. Filing them as `type: grammar`
would have misdescribed a sound rule as morphology; leaving them as an unknown
type would have produced a permanent validator warning. Adding the type is the
smaller and more honest change. Like `grammar` and `etymology`, it is exempt from
the cross-language concept join, because its progression lives in knowledge atoms.

### Not changed, and why

- **No English cousin web.** Chinese shares no ancestor with English, so no
  lesson claims one. HL00's ban on invented connections is followed literally;
  the substitute is character composition, and `README.md` states plainly that it
  is a weaker hook than the device it replaces.
- **`bridges: []`** in the registry. The honest bridges (Japanese, Korean,
  Vietnamese) are not tracks here, and listing English for the sake of a few
  loanwords would claim a bridge no lesson can walk across.
- **No new characters, and no font binary touched.** Chapter 1 was authored
  inside the existing 24-entry `chinese.json` inventory and the font subset built
  from it, so `_fonts/NotoSansSC-Subset.ttf` is unchanged. Two consequences worth
  recording, because they constrain future chapters:
  - the subset contains **only** the 50 CJK codepoints named in `chinese.json` —
    not even Chinese punctuation, so the fullwidth `！` was removed from the two
    lesson dialogues rather than shipped as a dropped glyph;
  - adding any character means editing `chinese.json` and re-running
    `_fonts/subset-cjk.sh`, which downloads the ~17 MB upstream Noto Sans SC.
    `roadmap.md` orders the planned chapters around that constraint.

### Known and reported, not hidden

- **Drivable prefix 1 of 7.** `ZH-C01-tones` is pure sound and is derived `voice`,
  but `ZH-C01-ni` and `ZH-C01-hao` teach character shapes and are correctly
  derived `sight`, so a commuter can start this chapter but not get past its
  second lesson. In an alphabetic track a word can be handed over by ear and
  spelled later; in a logographic one the character is the word's written
  identity, and there is nothing smaller to hand over first. `session-map.md`
  records the derivation per lesson.
- **Corpus pins moved, not weakened.** `modality.test.ts` and
  `integration.test.ts` pin whole-corpus measurements. Adding a track necessarily
  moves them: 1,096 → 1,103 lessons, 694 → 699 `voice`, 351 → 353 `sight`, 7 → 9
  script-block lessons, 20 → 21 tracks and books, 51 → 57 compiled activities. The
  `pen` count, the table-bearing count and the 63% drivable share are unchanged.
  Every moved pin carries a comment naming this change as the cause.
