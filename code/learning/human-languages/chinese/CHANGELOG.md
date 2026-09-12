# Changelog — Mandarin Chinese track

## Chapter 29 — reading: characters, words, and a first passage

Mandarin's reading rung, and the one that makes the reading measure mean
something: this chapter lands with the fix that counts a spaceless script by
character rather than by whitespace, so the 34 beside it is characters, which is
the unit Chinese's own task shape asks for.

Three lessons, ramping 6 -> 17 -> 34 characters, with no new character in any of
them.

- **Characters**: 人 书 看 有 和 十 — each one a whole word. The lesson names what
  that changes: in an alphabet a single letter is almost never a word, so there
  is no sounding out here. A character either lands whole or it does not, and
  the only thing that moves it is having met it.
- **Words**: 中国人, 我的书, 看书的人, 一个, 十八岁, 喜欢 — six words that are NOT
  six characters. Mandarin writes no spaces, so a reader has to decide where a
  word ends with nothing on the page to help. 看书的人 is four characters and one
  idea; 喜欢 is two characters that mean nothing apart.
- **Passage**: ten lines that turn at line six, from about the writer to
  addressed to you. 你呢 is two characters doing the work of a whole English
  sentence, and it only means that because of the five lines before it -- the
  first time this book asks a line to depend on what came earlier. 我十八岁 has
  no verb at all, and a reader who waits for one waits forever.

pre-A1 reading moves 0/1 -> 1/1, the track's only declared reading part.

Two conventions the gates enforced rather than me remembering. The passage
originally ended its lines with 。 and ？; the Chinese books' font covers
neither, and the track's own examples (我喜欢看书, 你喜欢什么) carry no terminal
punctuation at all. And the drafting check needed to become substring-aware:
十八 never appears as a standalone token because Mandarin writes no boundaries,
only inside 我十八岁, which is exactly the property the lesson is about.


## The particles, the joining zero, and five points that cost no character

Thirty-five lessons in seven chapters (22-28), chosen by **points-per-character**
against the uncovered list in `core/exam-inventory-chinese-a1.json` rather than by
topic. Chinese was measured at 38% (73/191), among the lowest in the corpus. This
closes **twenty-five points for TEN CHARACTERS** -- 98/191, 51%.

**A character is the expensive unit in this track**, and the inventory had already
priced it: a writing lesson, a reading lesson, a source-verified stroke record in
`data/scripts/chinese.json`, and a regeneration of the vendored subset font. So the
tranche was chosen the way that price implies -- by what each character buys.

### FIVE POINTS COST NO CHARACTER AT ALL

Chapter 22 adds **not one character** and closes five points.

  * **`ZH-A1-V-02`** -- a Mandarin verb has one form, for every person, every
    number and every time. The old note called this the cheapest sentence in the
    whole Mandarin queue and it was not written. It is now.
  * **`ZH-A1-NP-03`** and **`ZH-A1-VP-03`** -- nothing agrees, in either
    direction: not the verb with its subject, not what follows 是 with how many
    there are.
  * **`ZH-A1-ART-01`** -- 书 is *a* book, *the* book, and books. All four Spanish
    article points collapse onto that one fact, and no lesson had said it.
  * **`ZH-A1-ADJ-02`** -- 中国人. 中国 was taught in the country chapter and 人 was
    the first character this book ever wrote, and nothing had put them together.

Three of those five are things the reader must **stop** doing, which is why no
lesson had ever said them: an absence leaves no word to teach. `ZH-A1-S-04`, the
topic-comment order, is a sixth free point and rides in the 的 chapter because
that is where the first slot becomes worth filling.

### THE TEN CHARACTERS, AND WHAT EACH BOUGHT

| character | strokes | points |
|---|---|---|
| 的 | 8 | POSS-01, PRON-05 |
| 有 | 6 | V-04, NG1-01 |
| 个 | 3 | LIANG-01, NG2-01 |
| 了 | 2 | V-06, PART-03, ASP-01 |
| 呢 | 8 | PART-02, F2-01 |
| 对 | 5 | NG6-07, F6-02 |
| 岁 | 6 | NG5-04, F1-03 |
| 和 | 8 | J-01 |
| 喜 + 欢 | 12 + 6 | PRON-03, F3-01, NG6-04 |

**了 is two strokes** -- fewer than any character in the book but 一 and 二 -- and
before it every sentence in this track happened in an eternal present. **个 is
three strokes and two of them are 人.**

### THE PARTICLE GAP AND THE JOINING ZERO

The file's headline finding was that Mandarin carries almost all of its grammar in
a handful of toneless particles and **not one was taught**. That had already moved
once (吗, in the asking chapter). It is now **4 of 7**: 的, 了 and 呢 arrive here,
and the two that carry the most grammar between them cost eight strokes and two.
`ZH-A1-PART-04` stays open because a class point is not answered by four of seven,
and its note now names 吧, 过 and 着 as what is left.

**The joining column comes off zero** -- flat in seven tracks running. `ZH-A1-J-08`
had already done the work of saying why and which word was cheapest, and this
tranche spent exactly that budget: 和 is eight strokes, three of which are 口, and
it is a phono-semantic compound like 请 whose sounding half **is** *hé*. The lesson's
own content is the restriction: 和 joins two **nouns** and never two clauses.

### ONE NOTE HAD DECAYED, AND IT WAS THE NUMERALS

`ZH-A1-NG5-04` read "blocked on the numerals". The numeral tranche of chapters
20-21 had already lifted that block: only 岁 was missing. Six strokes closed age
AND `ZH-A1-F1-03` -- name, nationality, age -- because 名字 was already taught and
nationality had just been closed for no character at all. Four other uncovered
notes carrying the same stale blocker (`NUM-05`, `NG6-02`, `NG2-03`, `NG4-02`) are
narrowed rather than left alone, and `NG4-02` now says the calendar is the largest
single return left in the file.

### THE FONT WAS REGENERATED FROM ITS OWN SCRIPT

Ten characters were added to `data/scripts/chinese.json` with **source-verified PRC
stroke order**, each citing the same pinned Hanzi Writer Data commit the existing
fifty use, and `_fonts/subset-cjk.sh` was re-run. The upstream SHA was verified
against the pin in the script before anything was trusted, and the script was first
run unchanged to confirm it reproduces the committed subset **byte for byte**. A
character in a lesson and absent from that inventory is tofu that every other gate
passes.

**And the track's own punctuation habit was followed rather than broken.** The
inventory reports `Biaodian` at 0 of 7 -- not one mark taught in the corpus -- and
the subset font carries no CJK punctuation. So these lessons write no 。, no ，and
no ？either, and where Chinese would put a comma at a topic seam the page prints a
dash and says why.

### REINFORCEMENT, DECOMPOSED

Measured against the merged tree, not asserted:

```
reinforcementWindowMisses           238 -> 192
reinforcementMissesByWindow-R1       21 ->  21
reinforcementMissesByWindow-R2       57 ->  57
reinforcementMissesByWindow-R3       98 ->  87
reinforcementMissesByWindow-R4       62 ->  27
atomsNeverRevisited                  11 ->   9
atomsTaught                         166 -> 212
```

Adding thirty-five lessons makes **47** pre-existing atoms window-judged for the
first time (R2 6, R3 23, R4 18) -- debt the added LENGTH exposes rather than debt
the tranche creates. **All 47 are paid**, the tranche's own 46 atoms create **zero**
new debt in any window, and pre-existing debt falls by 46 on top of that (R3 11,
R4 35).

Every lesson opens with six recalls: R1 at distance 1, R2 at distance 5, two at R3
and two at R4. The second slot in each wide window is what pays the pre-existing
debt; one slot per lesson would only have covered the newly exposed. R1 and R2 do
not move and cannot -- their misses sit at positions the new lessons cannot reach
back into within a fifteen-lesson window.

### THE INVENTORY IS UPDATED IN THE SAME COMMIT

Landing 46 atoms and leaving the probes at `null` would have kept `98/191` reading
as `73/191` with a completely green suite. Both halves of the gate were falsified:
a fabricated atom id fails "probes only atoms that EXIST" AND the count, and nulling
a probe fails the count.

**The pinyin decision at `ZH-A1-PY-06` is untouched.** This file treats pinyin as a
pronunciation claim and not a script claim, a test forbids a pinyin point being
probed with a `ZH-SCRIPT` atom, and no point here re-litigates it.

### WHAT IT DELIBERATELY DOES NOT CLOSE

`ZH-A1-DEM-01` -- 这 and 那 -- is still the most expensive single gap in the file
and is the obvious next tranche after the calendar. 们 is one character from
`PRON-02` and half of `N-05`, whose first half this tranche closed. Food and drink
is one Spanish point and not one Mandarin word. Punctuation is seven points and not
one mark. And translation stays 0 of 2: no lesson in the track declares `mediation`.

## The other half of the count, and ninety-nine numbers from ten words (HL-C350)

This track taught **一 二 三 四 五** in chapter 3 — each with its own character
and its own writing lesson — and then stopped. For **seventeen chapters** it
could name a family, a school and a greeting and could not give an age, a price,
a floor or a date. Chapters 20–21 add 18 lessons and close `ZH-A1-NUM-01`,
`ZH-A1-NUM-02` and `ZH-A1-AB-02`.

- **六 七 八 九 十**, one character and one word each, finishing the run. **Five
  characters and twelve strokes between them** — four, two, two, two, two. The
  three largest single digits are the three quickest characters in the book to
  write, which is the opposite of the intuition a learner brings.
- **十 was already on the page.** It is the lower half of **早**, and the morning
  lesson said in as many words that on its own it is the character for ten and
  then went on to something else. `ZH-W20-shi` is that sentence cashed, thirteen
  chapters late.
- **八 against 人.** Eight is the *piě* and *nà* of *rén* with the join at the
  top removed. The gap is the entire difference between a person and the number
  eight.
- **Ten words reach ninety-nine.** **十一** is ten-one, eleven; **二十** is
  two-ten, twenty; the same two words in the other order, and the order is the
  whole grammar. Chapter 21 teaches the rule and has the reader **build** the
  rest rather than recall them.
- **九十九 is the third-tone sandhi drill the track had no material for.** Two
  third tones around a rise, and the first pair of them the course could build.
- **一百 is where 一 moves.** Counted alone it is level; in **一百** it falls, the
  same shifting behaviour the book named for **不** in its first chapter.
- **5 and 五 are both *wǔ*, and a price tag prints the first.** `ZH-C21-digits`
  teaches which register each belongs to — digits for prices, dates, phone
  numbers and addresses; characters for formal and legal text, cheque amounts
  and running prose — and that both are read aloud as the same words. It costs
  no new character.
- **The script inventory and the vendored font grew with the content.** 六 七 八
  九 十 百 join `data/scripts/chinese.json` with stroke orders taken from the
  same pinned Hanzi Writer Data snapshot the existing entries cite, and
  `NotoSansSC-Subset.ttf` was regenerated by its own script from that file. The
  upstream font's SHA-256 matched the pin exactly, so the subset is reproducible
  rather than hand-edited.
- **Exam coverage 70/191 (37%) → 73/191 (38%)**. Both halves of that measurement
  were falsified before shipping: a fabricated atom id and a removed probe each
  break the suite.
- **Zero reinforcement debt created**, and the R4 window count FELL, 64 → 62.
  Script-closure violations (4), never-taught glyphs (0), forward references
  (0), atom and chapter spikes (0), payoff surprises (0) and duration violations
  (0) are all unchanged.

All notable changes to this track. The format follows the other human-language
tracks: one entry per authored tranche, describing what was added and why.

## [Unreleased]

### Added — 吗, and the chapter that made the book able to ask (chapter 19)

Sixteen lessons, one new character, and the two ways Mandarin asks a yes-or-no
question. Chinese: 18 chapters and 175 lessons -> 19 and 191.

**What the A1 inventory measured, re-verified before anything was designed.**
Not one particle was taught — de, le, ma, ne, ba, guo and zhe each returned zero
occurrences across all 175 lesson files, in characters and in tone-marked
pinyin. The track could greet, name, count to five, say what it saw and say what
it read, and could not ask a yes-or-no question about any of it. The inventory
called 吗 "the single cheapest sentence-level gain available to this track", and
it was right.

**One character, and it does more than its own weight.** 吗 goes at the END of a
finished statement and moves nothing else, so every statement the track already
teaches is now also a question. English cannot do that — it moves words about
and borrows a helper verb — and saying so is the point of the grammar lesson.
The second pattern cost no character at all: 好不好 is 好, the 不 the negation
lesson taught, and 好 again.

**WHY THIS TRANCHE IS SIXTEEN LESSONS AND NOT THIRTY-FIVE.** In this track a
character is not a word — it is a five-lesson cycle (hear, observe-trace,
guided copy, delayed copy, and the reading that meets the shape), plus a
source-verified stroke record in `data/scripts/chinese.json`, plus a ductus path
checked against the vendored font outline, plus a regeneration of the subset
font itself. All four were paid here: the stroke order is derived from the
pinned Hanzi Writer Data snapshot 68d10a4, the ductus was fitted to the real
Noto Sans SC 吗 outline, and `_fonts/NotoSansSC-Subset.ttf` was rebuilt with
`subset-cjk.sh`. That cost is now written into the joining column's own note so
a future tranche budgets for it rather than rediscovering it.

**Coverage 66/191 to 70/191 (35% to 37%).**

| Column | Before | After |
|---|---|---|
| Zhuci — the particles | 0/4 | 1/4 |
| Danju — the simple sentence | 2/4 | 3/4 |
| Gei he qiu xinxi — giving and asking for information | 2/5 | 3/5 |
| Biaoda yijian — opinions, attitudes and knowledge | 3/9 | 4/9 |
| Shejiao — social contact | 4/7 | 4/7 |
| Hanzi — the characters | 5/10 | 5/10 |

**Reinforcement fell while the track grew.** Window misses 234 -> 227: the
tranche's own ten atoms miss nothing the corpus is long enough to measure, seven
pre-existing misses are closed, and every window the added length exposed was
closed with it. Atoms never revisited 11 -> 11. Script-closure violations 4 -> 4
and never-taught glyphs 0 -> 0.

**Left uncovered, with the reason in the inventory file.** The whole joining
column (J-01 to J-08) — every one of he, gen, huozhe, haishi, keshi, danshi,
yinwei, suoyi and suiran needs at least one untaught character, and putting one
on the page without its writing cycle is the debt this corpus exists to avoid.
he is the cheapest of the nine and is the natural next tranche. PART-04, the
particles as a class, is not answered by one member of the class.

### Added — four school compounds with four-stage retrieval (#12789)

- Added 17 super-gentle lessons for **中学**, **中学生**, **同学**, and
  **上学**, each capped below five minutes and introduced sound-before-shape.
- Added source-pinned stroke order and real-font ductus for only the two new
  characters, **中** and **同**, reusing already secure **上**, **学**, and
  **生** instead of front-loading another script cluster.
- Added four spaced mixed-retrieval passes and a separately scored four-skill
  payoff, including no-model writing and contrast with the already learned
  primary-school and university ladders.

### Added — first official HSK 3.0 vocabulary clusters (#12654, #12729)

- Added three super-gentle chapters that secure 15 official Level 1 family,
  school, and primary-school words through 40 lessons of five minutes or less.
- Staged every new character as sound before shape, guided writing before
  no-model recall, and familiar-character recombination before longer words.
- Added four-skill retrieval and payoff checks that keep listening, speaking,
  reading, and writing independently passable instead of letting pinyin or a
  stronger skill hide a weaker one.
- Added the authoritative chapter and curriculum shards, generated book and
  narration artifacts, progress snapshots, and the CJK subset glyphs needed to
  render the new stroke-component explanations without substitutions.

### Changed — complete the pre-A1 writing-stage runway (#12449)

- Made the existing first-character practice an explicit model-visible guided
  copy rather than leaving the learner to infer how much support was allowed.
- Turned the already component-safe **你** assembly into a ten-second delayed
  copy with half-by-half repair.
- Added sound-only dictation of **好** only after **女** and **子** are secure,
  without adding a new character or extending any lesson past five minutes.

### Added — project-defined pre-A1 four-skill task shapes (#12369)

- Turned the Chinese assessment contract's pre-A1 target into bounded reading,
  listening, writing, and speaking tasks without inventing an official HSK
  level below HSK 3.0 Level 1.
- Made productive handwriting independently scorable through delayed character
  recall, dictation/transcription, and a tiny cue-only response. Tracing and
  visible copying remain teaching steps and cannot satisfy the pass.
- Kept every response to one doorway-sized turn and every skill independently
  passable, so future five-minute lessons can approach the mock one atom at a
  time rather than jumping straight to an exam-length performance.

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
first. See HL-C210 in the [`BACKLOG.d/`](../BACKLOG.d/) history for the second.

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
