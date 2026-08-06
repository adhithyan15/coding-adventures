# Changelog

All notable changes to `@coding-adventures/human-language-data` are documented here.

## [Unreleased]

### Changed — corpus pins moved by the Japanese track (HL-C40)

No source change: the Japanese track is content, and the package loaded it without
a code edit because `japanese/track.json` declares the script (the built-in
`LANGUAGE_SCRIPT` map was deliberately left alone, proving that path works). The
pinned corpus measurements moved, and each pin now records why:

- `registeredTracks`, `authoredBooks`, `schemas.tracks`, `books.tracks`: 21 → **22**,
  Japanese following Mandarin Chinese (HL-C39) as the 22nd track.
- `modality-manifest.test.ts`: `totalLessons` 1125 → **1133**, `voice` 724 → **725**,
  `sight` 348 → **355**, `chapterCount` 376 → **377**, `unstartableChapters`
  121 → **122**; `pen` stays 53 and the drivable share stays **64%**.
- `drivablePrefixTotal` does **not** move (558). Japanese ch1 opens on one of its
  seven `script` lessons, so the chapter's drivable prefix is zero — which is also
  why `unstartableChapters` gains one.
- The compiled-activity id list gains the eight `JA-C01-*` activities.

Seven of the eight Japanese lessons carry a `script` block and therefore derive as
`sight`. That is the honest classification — a kana or kanji shape cannot be read
aloud — and it was chosen over routing the same content through `input` blocks,
which would have held the drivable percentage flat by mislabelling it.

Added one integration test, `keeps the Japanese Chapter 1 mixed-script chain closed
and under five minutes`, which asserts the property rather than only the counts:
the same chapter carries a hiragana, a katakana, and a kanji headword; every lesson
is schema-v2 with exactly one compiled activity; nothing exceeds the duration
budget; and the plain and polite thanks keep distinct `register` values.

### Added — tone in the script data model, and a `pronunciation` lesson type (HL-C39)

Driven entirely by the Mandarin Chinese track, which was added as a scale test for
whether the curriculum model generalises outside Indo-European and Dravidian.

- `ScriptData` gains `tones?: Tone[]` and `toneSandhi?: ToneSandhiRule[]`.
  `Letter.tone` already existed and labels the tone a *character* carries, which is
  enough to tag a glyph and nothing more. It cannot say what tone 3 *is* (contour
  214, low and creaky), and it cannot express **sandhi** — a rule that changes a
  syllable's pitch because of the syllable *after* it while the characters and the
  printed pinyin stay identical. Every previously modelled script encodes
  pronunciation segmentally, and a segment always attaches to a glyph; tone is
  suprasegmental, so the existing shape did not stretch. `data/scripts/chinese.json`
  populates both fields.
- `EXEMPT_TYPES` gains `pronunciation`. No earlier track ever needed a lesson
  *about* sound, because segmental facts belong to letters and therefore live inside
  the word lesson that first uses that letter (HL00, "Pronunciation & Script:
  Inline, Never a Gate"). Folding Mandarin's tone system into its first character
  lesson pushed that lesson to 352 effective seconds, past the five-minute contract,
  and HL08's rule is to split rather than waive. `grammar` would have misfiled a
  sound rule as morphology; an unrecognised type would have produced a permanent
  validator warning. Like `grammar` and `etymology`, `pronunciation` is exempt from
  the cross-language concept join because its progression lives in knowledge atoms.

### Changed — corpus pins moved by the new track, never weakened

Adding a 21st track necessarily moves whole-corpus measurements. Every pin below
was updated with a comment naming this change as the cause; none was relaxed.

- `integration.test.ts`: registered tracks, authored books, schema tracks and book
  coverage 20 → 21; compiled activity ids 51 → 57. Duration violations and unknown
  prerequisites remain **0**.
- `cli.test.ts`: reported `registeredTracks` 20 → 21.
- `modality-manifest.test.ts`: total lessons 1,118 → 1,125; `voice` 719 → 724;
  `sight` 346 → 348; `trackCount` 20 → 21; `chapterCount` 375 → 376;
  `drivablePrefixTotal` 557 → 558. The `pen` count (53) and the corpus-wide drivable
  share (64%) are unchanged, because no Chinese lesson needs a pen and none carries a
  table. The two `sight` lessons are `ZH-C01-ni` and `ZH-C01-hao`, which each teach a
  character's components in a `script` block.
- **No `modality.test.ts` edit, and no Language Ladder test edit.** Both used to hold
  hard-coded track and corpus counts and were rewritten upstream to derive them —
  `modality.test.ts` now asserts size-independent invariants, and the Language Ladder
  suites read `LANGUAGE_ORDER.length` / `LANGUAGE_CHAIN.length` instead of the literal
  20. Registering a track no longer requires touching any of them, which is why this
  entry is shorter than the same entry would have been a week ago.

### Fixed — HL-C26: hand-written chapters are described, not generated

- Add a `handwritten[]` list to `core/book-generation.json` recording the **105**
  chapters that have a committed `book/chapters/ch*.tex` but no `targets[]`
  entry, with `title` and `label` transcribed from what each `\chapter{}` and
  `\label{}` actually declares. These are the hand-authored prefixes of nearly
  every book, written before the generator existed and mostly still schema-v1.
- The obvious fix — giving them `targets[]` entries — would have **destroyed
  them**. A target is not a description but an instruction: `generatedBookOutputs`
  renders every target and `--write` writes the result over the file at `output`.
  A separate array is used instead of a `generated: false` flag precisely because
  the two fail in opposite directions; `generatedBookOutputs` only ever walks
  `config.targets`, so nothing in `handwritten[]` can be rendered by a missed
  branch. The worst a mistake there can do is leave a chapter unchecked.
- Add `handwrittenBookChapters()`, which reads the list without rendering
  anything. `check:books` output is unchanged, byte for byte.
- `chapter-title-drift` previously **skipped** any chapter with no target, which
  left those titles verified by nothing. It now checks them against
  `handwritten[]`, and a new test fails if any ledger chapter is covered by
  neither list — so the assertion cannot decay back into a silent `continue`.
- Add tests that re-read every hand-written `.tex` to prove its recorded title and
  label were transcribed rather than invented, that the two lists never claim the
  same chapter, that no hand-written path appears in `generatedBookOutputs()`, and
  that every committed chapter file is accounted for by one list or the other.
- Add a check that every generation target's committed file opens with
  `% GENERATED FILE.` (true of 270/270 generated and 0/105 hand-written chapters).
  This is the only guard that catches a chapter *promoted* into `targets[]`, which
  by leaving `handwritten[]` escapes every membership-based check.
- Labels are recorded as declared, not normalised. Three conventions coexist — a
  bare `ch:greetings` slug, an ISO-code `ch:fa-`/`ch:la-` prefix, and a
  language-name `ch:persian-`/`ch:urdu-`/`ch:russian-` prefix — so Persian ch2 is
  `ch:persian-name` beside a generated `ch:fa-ask-and-answer-names`. Rewriting a
  `\label` breaks existing `\hyperref` cross-references, so the inconsistency is
  recorded in the backlog for a deliberate decision rather than silently fixed.

### Added — stroke-order provenance on `Letter`

- Add `StrokeOrderSource` and two optional `Letter` fields, `penLifts` and
  `strokeOrderSource`. A `strokeOrder` list names a letter's **parts** in writing
  order; it has never counted **pen-down runs**, but a numbered list of three
  reads to a learner as three strokes and two lifts. Tamil ம is the counter-
  example that forced the distinction: its prose listed three parts while the
  authored, font-checked pen path in Language Ladder's `strokes.ts` shows one
  unbroken stroke with zero lifts. `penLifts` records that number only where a
  verified path supports it — absent means *not verified*, never *none* — and
  `strokeOrderSource` carries the citation, URL, and the honest `variation` note
  for scripts (every Indic script, Arabic, Hebrew) that have no national
  standard. Both are optional, so every existing script file still validates.
- Document the parts-vs-strokes rule on `strokeOrder` itself, where the next
  author writing one will actually read it.

### Added — HL-C44 the modality manifest, so two editions build from one source

- Add `src/modality-manifest.ts` and `src/modality-cli.ts`, emitting
  `code/learning/human-languages/core/lesson-modality.json`. HL-C14 already derived
  `voice`/`sight`/`pen` per lesson and a drivable prefix per chapter, but only at
  runtime and only into the human-readable gap report — a paragraph of English is not
  something a book builder can filter on. This slice makes the derivation *data*, so
  the complete book, the app, and the forthcoming dictation-friendly driving edition
  (HL-C43) each filter the same canonical corpus rather than maintaining three copies.
- **Per lesson:** `id`, `language`, `chapter`, `sequence`, `modality`, `derived`,
  `drivable`, `reasons`, and the lesson AST's `sourceHash`. The three override fields
  (`authored`, `authoredReason`, `overridden`) are emitted only on the lessons that
  have them, rather than a thousand copies of the empty string. The monotone closure
  (`pen` implies `sight`) is deliberately *not* emitted: it is a three-entry lookup
  table, and restating it beside every pen lesson would add sixty kilobytes of
  duplicating `requiredChannels()`.
- **Per chapter:** the drivable prefix, `firstNonVoiceLesson`, the modality union,
  whether the whole chapter is drivable, and `drivableLessonIds` — the prefix spelled
  out in order, so a driving-edition renderer never has to re-implement "authored
  order" and quietly disagree with the generator about it.
- **Per corpus:** a `summary` pinned by tests — 1,096 lessons, 708 `voice`, 337
  `sight`, 51 `pen`, 65% drivable, 20 tracks, 375 chapters, 551 lessons reachable in
  the car once prerequisite order is respected, 199 fully drivable chapters, 121 that
  cannot be started by ear at all, zero overrides, zero chapterless lessons.
- **Designed for HL-C41's block-level modality to land additively.** Every lesson row
  is a JSON object, not a positional tuple. `modality` keeps its meaning permanently —
  the strongest channel the lesson needs *anywhere* — so a consumer that never learns
  about block modality keeps producing a correct, merely pessimistic driving edition,
  which is the safe direction to be wrong in. `coreModality` arrives as a new optional
  key beside it (`entry.coreModality ?? entry.modality` is correct before and after),
  and the header's `features.blockModality` flag says at a glance whether a build
  carries block data. The shape of the companion block records is deliberately not
  guessed here: an absent key is additive, a wrong key is a breaking change.
- **Nothing is authored.** The manifest is derived, exactly like
  `core/generated-book-hashes.json`. HL08 refused to add `modality:` to 1,096
  frontmatter files precisely because that is 1,096 places for a computed fact to go
  stale, and this artifact does not reintroduce the problem.
- Add `npm run generate:modality` / `npm run check:modality`, mirroring the
  `generate:books` / `check:books` contract: `generatedModalityOutputs()` returns a
  path → content map so `--write` and `--check` consume identical bytes, `--check`
  compares byte for byte and exits 1 on any drift, and the corpus is fingerprinted with
  `fnv1a64` from `hash.ts`.
- Wire `npm run check:modality` into `human-languages-books.yml` beside
  `check:books`. A stale manifest is not cosmetic: a lesson that gained a paradigm
  table would still read `drivable: true`, and the driving edition would tell somebody
  at 70mph to look at a chart. The `books-gate` job's name expression and pass/fail
  contract are untouched.
- Add `loadModalityManifest()` and `modalityManifestById()` to `loader.ts`, exported
  from `index.ts` with the manifest types. The index returns a `Map`, never a plain
  object: the keys come out of parsed JSON, and `index[lesson.id] = lesson` with an id
  of `__proto__` writes the prototype instead of a property.
- Ordering is total and null-last (track, chapter, `sequence`, id), so the file is
  byte-stable regardless of directory-walk order — otherwise `--check` would fail on a
  colleague's machine for no reason. The corpus fingerprint sorts by id rather than
  reusing `combineLessonHashes`, whose `sequence`-first ordering degenerates on the
  many lessons that carry no sequence (`Number(undefined)` is `NaN`, and every
  comparison against `NaN` is false).
- `safeOutput()` fails closed on path escape, checking containment *after* `resolve`
  rather than scanning the input string for `..`, and requires a `.json` extension so a
  mistake cannot land on an authored `.tex` chapter or `.md` lesson.
- 33 new tests: manifest round-trip, order-independent bytes, drift detection
  (including a byte-level reformat), the missing-manifest case, the full path-escape
  matrix, the `__proto__` index case, the additive-`coreModality` read, and the corpus
  summary pinned field by field. `modality-manifest.ts` reaches 100% statement
  coverage. No existing assertion was weakened.

### Added — HL08 modality and the drivable prefix (report only, no gates)

- Add `src/modality.ts`: a pure module deriving each lesson's required channel
  (`voice` / `sight` / `pen`) and each chapter's **drivable prefix** — how many of
  its lessons, in authored `sequence` order, are learnable by ear before the first
  that is not. Implements the first migration step of
  [`HL08`](../../../specs/HL08-modality-gentle-ramp-and-the-drivable-course.md).
- **Modality is derived from lesson type and block structure, never from `skills:`.**
  `skills` records what a lesson *develops*, not what it *requires*: 501 of the 531
  schema-v2 lessons declare `[listening, speaking, reading]`, yet *hola* is
  perfectly learnable by ear. Deriving from `skills` would have stamped roughly 95%
  of the corpus "needs eyes" and made the drivable course an empty promise. The
  rules are: `type: writing` → `pen`; otherwise a `script` block, a sight cue, or a
  table wider than the configured linearisable width → `sight`; otherwise `voice`.
- Modality is monotonic — `pen` implies `sight` — exposed as `requiredChannels()`
  and `unionModalities()`, and a chapter's modality is the union of its lessons'.
- `maxLinearisableTableColumns` defaults to **0**: until HL08's narration exporter
  can linearise a two-column table into speech, no table is speakable, and claiming
  otherwise would let a learner silently miss content they were never told they had
  missed. The value is an option, so it moves to 2 the day the lineariser lands.
- Support an authored `modality:` frontmatter override. An override that
  *contradicts* the derivation requires a `modality_reason:`; unexplained overrides
  (`modality-unexplained-override`) and unrecognised values (`modality-unknown-value`,
  which falls back to the derivation) are collected across the whole corpus and
  reported once. Nothing throws, and nothing gates — the HL-V01 precedent.
- Add a modality section to `buildCurriculumGapReport()` and its text renderer:
  per-track `voice`/`sight`/`pen` counts, each chapter's drivable prefix, the
  chapters that cannot be started by ear at all, and the corpus-wide drivable
  percentage. New summary fields: `drivableLessons`, `drivablePercent`,
  `chaptersWithoutDrivablePrefix`, `unexplainedModalityOverrides`.
- Measured over all 1,096 lessons: **51 `pen`**, **7** carrying a `script` block,
  and among the remaining 1,038, **322 carry a Markdown table** — the single largest
  obstacle to a hands-free course, and far more tractable than the script.
  **694 lessons (63%) are drivable exactly as authored.** Track extremes: Bengali
  and Persian at 90%, Russian at 9%.
- `tests/modality.test.ts` covers every derivation branch, monotonicity, the
  override-plus-reason rule, drivable-prefix computation (including a chapter whose
  prefix is 0), and pins the corpus-wide drivable count as a regression. The pin
  exists because a parser change that renamed a block's `markdown` field would make
  every lesson scan clean and silently report a 100%-drivable curriculum.
- Divergence from HL08's recorded baseline, stated rather than tuned away: the spec
  reports 56 cue-bearing lessons and 695 drivable. The published `SIGHT_CUES` list
  matches 61 lessons and lands on 694. Every structural count reproduces exactly
  (51 / 7 / 1,038 / 322), so the gap is entirely in the cue list, whose exact
  contents the spec never recorded. The detector was left alone.

### Added — HL05 chapter capability layer (data only, no gates)

- Add `ChapterCapability`, `ChapterPayoff`, `TrackChapters`, and `ChapterPolicy`
  types for the chapter capability ledger specified in
  [`HL05`](../../../specs/HL05-chapter-capability-and-step-by-step-shape.md).
  A chapter was previously nothing but an integer stamped on each lesson, so
  nothing in the data model knew what a chapter was for and nothing could check
  that finishing one left the reader able to do anything.
- Add `loadTrackChapters()` and `loadChapterPolicy()` beside the existing
  `loadLanguageCurricula()`. Tracks without a `chapters.json` are **skipped, not
  defaulted** — an absent ledger means "not yet authored", which the gap report
  must be able to tell apart from "authored and empty". Inventing a placeholder
  would erase exactly the debt the report exists to measure.
- Add `core/chapter-policy.json` carrying the HL05 payoff-representativeness
  threshold and the HL08 gentle-ramp budgets, with the corpus measurements the
  values were drawn from recorded alongside them. Thresholds sit at the existing
  distribution: 3 new atoms per lesson (the current p90, flagging 52 lessons) and
  12 per chapter (just above the chapter p90 of 10, flagging 17).
- Add `spanish/chapters.json` covering Chapters 1–3 as the authored proof of
  shape. Chapters 4 onward are deliberately absent rather than stubbed.
- This slice ships **no validation gates**. Those are the next work item, and
  they land report-only over all 379 chapters before any track fails on them.

### Fixed — live generated curriculum links

- Preserve canonical Markdown links as live LaTeX `\href` targets instead of
  dropping every destination during book generation.
- Resolve relative lesson and pronunciation-reference links against stable
  GitHub source URLs while preserving absolute source citations and rich link
  labels from the same canonical blocks consumed by Language Ladder.
- Reject missing relative-link bases and non-HTTP(S) destinations, escape URL
  metacharacters for LaTeX, and regenerate the nine affected chapters with 55
  working links.

### Fixed — generated quotation typography

- Render paired straight double quotes in canonical lesson prose with explicit
  LaTeX opening and closing quote commands across every generated chapter.
- Preserve code spans, escaped literals, link destinations, existing curly
  quotes, and unmatched marks while handling emphasis and nested quotations.
- Keep indented Markdown blockquote continuations inside the same generated
  quote/callout so multiline learner examples are not split during rendering.
- Regenerate all 270 configured chapter targets without changing the canonical
  Markdown consumed by Language Ladder.

### Added — Persian and Urdu take-leave frontiers

- Extend both RTL tracks through `SPINE-TAKE-LEAVE` with four schema-v2
  Chapter 5 micro-lessons apiece: the two historical word layers, the complete
  local-script farewell, and cumulative start-versus-end practice.
- Compile one objective contract for every new lesson, raising mapped
  non-lexical coverage from the Chapter 4 baseline to 25 of 119 lessons while
  leaving the 94-item debt unchanged.
- Generate both Chapter 5 LaTeX files from the same prerequisite-closed lesson
  AST consumed by Language Ladder, preserving Persian joined **خداحافظ** and
  Urdu spaced **خدا حافظ**.

### Added — Persian and Urdu shared name exchange

- Extend both RTL tracks through `SPINE-EXCHANGE-NAMES` with five schema-v2
  Chapter 3 micro-lessons apiece: address/register, question word, complete
  name question, meeting response, and cumulative practice.
- Compile one objective practice contract per track, raising coverage to 21 of
  115 mapped non-lexical lessons across 18 tracks while leaving the 94-item debt
  unchanged.
- Generate both Chapter 3 LaTeX files from the same prerequisite-closed lesson
  AST consumed by Language Ladder and verify their combined source hashes.

### Added — Russian activity prerequisite closure

- Migrate the six-lesson Russian pronoun and naming chain to schema v2 so its
  two mapped non-lexical frontiers have transitive, block-bound knowledge rather
  than activities attached to unowned legacy prerequisites.
- Compile objective checks for polite *вы* and the cross-language *how/what*
  naming contrast, raising coverage to 19 of 113 mapped non-lexical lessons
  across 16 tracks and leaving 94 explicit gaps, 16 of them legacy.

### Added — cross-language objective activity coverage

- Add one prerequisite-closed final-recall contract to a ready non-lexical
  lesson in each of 15 tracks with schema-v2 coverage debt: Arabic, German,
  Gujarati, Hindi, Italian, Kannada, Latin, Malayalam, Marathi, Portuguese,
  Punjabi, Sanskrit, Spanish, Tamil, and Telugu.
- Keep every new response budget at eight seconds and select a safe Italian
  Chapter 3 frontier rather than pushing its 297-second Chapter 2 practice lesson
  past the strict five-minute ceiling.
- Raise measured objective coverage from 2 to 17 of 113 mapped non-lexical
  lessons while leaving the 18 legacy migration prerequisites explicit.

### Added — compiled activity contracts

- Parse compact JSON `hl-activity` directives beside typed block knowledge and
  keep prompts, canonical answers, accepted variants, corrective feedback, and
  response budgets in the canonical AST while omitting metadata from learner copy.
- Compile normalized answer sets once for browser consumers and validate stable
  activity ids, non-empty assessed-atom subsets, block-bound assessment closure,
  unique variants, complete feedback, and 1–299 second response budgets.
- Count authored activity response time in duration model v2 and add objective
  grammar and script pilots to two Spanish lessons without changing book prose.

### Added — per-track shared-spine realization maps

- Load and validate one ordered `curriculum.json` for every registered track,
  with repeatable spine segments, explicit omission/relocation ledgers, and
  typed language-specific extensions placed before, inline, or after a segment.
- Require canonical and schema-v2 lesson coverage, prerequisite-closed local
  order, and exact support-lesson extension classification across all 20 maps.
- Add pure local-path and independent mixed-frontier queries so downstream apps
  can schedule the next safe lesson without borrowing another language's
  progress.

### Added — non-Latin canonical book chapters

- Let a generated-book target declare a Unicode Script property and its existing
  LaTeX font command, wrapping only target-script runs while keeping surrounding
  prose in the book's main font.
- Use authored romanization for non-Latin section bookmarks and fail closed when
  only half of the script-rendering configuration is present.
- Generate Marathi Chapter 6 from its two strict canonical lessons and expose the
  same ordered source hash to Language Ladder.
- Generate Gujarati Chapter 6 from its two strict canonical lessons, preserving
  Gujarati-script runs and bookmark-safe romanization from the shared AST.
- Generate Punjabi Chapter 6 from its two strict canonical lessons, preserving
  Gurmukhi runs and bookmark-safe romanization from the shared AST.
- Generate Sanskrit Chapter 6 from its three strict canonical lessons,
  preserving Devanagari forms, comparison tables, and romanized bookmarks from
  the shared AST.
- Generate Bengali Chapter 6 from its strict canonical lesson, preserving the
  Bengali numeral forms, *dui* history, and bookmark-safe romanization from the
  shared AST.

### Added — block-boundary knowledge closure

- Parse canonical `hl-knowledge` directives beside every schema-v2 body block
  while excluding the metadata from learner-facing Markdown.
- Validate introductions and assessments in rendered order, reject undeclared or
  unavailable prompt knowledge, and require production and recall blocks to name
  what they assess.
- Migrate all 51 Spanish Chapters 1–6 lessons to the fail-closed block contract
  and refresh their shared app/book source hashes.

### Added — canonical LaTeX chapter generation

- Added deterministic lesson-AST fingerprints and a pure Markdown-block to
  LaTeX renderer, now covering all 24 Spanish Chapter 1–3 schema-v2 lessons.
- Preserved nested inline emphasis, wrapped long practice lists, and emitted
  text-safe short titles for running headers and PDF bookmarks.
- Added write/check CLI modes, a committed chapter-hash manifest, path-safety
  validation, and a unified-book CI drift gate.
- Exposed each parsed lesson's source hash so book and app consumers can verify
  that they loaded the same canonical content.

### Added — schema-v2 lesson AST and strict curriculum contract

- Parse one-level nested lesson frontmatter and losslessly expose level-two
  Markdown sections as stable typed body blocks.
- Enforce schema-v2 spine mapping, local sequence, strict computed duration,
  block shape, coverage metadata, same-language prerequisites, stable knowledge
  atoms, unique introductions, and transitive knowledge closure.
- Prove the contract on all 24 Spanish Chapter 1–3 lessons while preserving
  schema-v1 compatibility for the rest of the corpus.

### Added — curriculum migration gap report

- Added deterministic JSON and text reports for effective lesson duration,
  unknown and omitted prerequisites, book-chapter coverage, and per-track schema
  migration status.
- Added a CLI format switch so CI can publish both report forms with the unified
  human-language book artifact without turning existing migration debt into a
  false regression gate.

## [0.3.0] - 2026-07-18

### Added — `writing` lesson type (orthography / writing nuances)
- **New exempt lesson type `writing`** for lessons that teach a *writing-system*
  nuance — an accent mark, a diacritic, an inverted punctuation mark — rather
  than a vocabulary word. Its `headword` is the mark itself and it carries **no
  `concept_tag`** (a mark does not join across languages), so it is exempt from
  the cross-language concept join, exactly like `practice`/`review`.
- Validator now accepts `writing` without flagging `unknown-type` or requiring a
  concept; added a test covering it. Supports the curriculum's "teach the
  accent marks and other writing nuances" goal (HL00) and gives HL02's
  hand-writing practice a lesson type to draw from.

## [0.2.0] - 2026-07-18

### Changed — general script model (teach any writing system)
- **`Script` is now an open string**, not a closed union — a new script needs no
  type edit.
- **Generalized the script-data schema** to cover all three families with one
  shape: `alphabet`, `abugida`, `abjad`. `ScriptData` gains `name`, `direction`
  (ltr/rtl), `system`, and `complete`; `Glyph`→`Letter` (with `role`, optional
  contextual `forms` for cursive/abjad scripts, `inherentVowel` for abugidas);
  `VowelSign`→`Mark` (vowel signs *or* harakat/niqqud). (Breaking, but nothing
  consumed the old shape yet.)
- **Tracks may self-declare their script** via `<track>/track.json`
  (`{ "script": "hebrew" }`); `parseLesson` takes an optional resolved script and
  the loader passes it in. Adding a new-script language needs no shared-map edit.
- **Coverage hardens with `complete`**: unknown headword characters are warnings
  while a script file has `"complete": false`, and become errors once it's `true`.

### Added
- `data/scripts/devanagari.json` (abugida) and `data/scripts/arabic.json`
  (abjad, rtl, contextual forms) — the two reference inventories proving the
  general schema across LTR-abugida and RTL-abjad.
- `data/scripts/README.md` — the "add any script" checklist (Gujarati, Bengali,
  Hebrew, …): author `<script>.json`, vendor the font, point a track at it.
- `trackScript` loader export; tests for open script ids, contextual-form
  coverage, and complete→error escalation.

## [0.1.0] - 2026-07-17

### Added
- Initial release — the HL01 data layer over the Human Languages curriculum.
- **Types** (`types.ts`): `Concept`, `Realization`, `Dataset`, `Taxonomy`,
  `ScriptData`/`Glyph`/`VowelSign`, `Issue`.
- **Frontmatter reader** (`frontmatter.ts`): a tiny zero-dependency parser for the
  `key: value` / `[list]` frontmatter shape the lesson schema uses (BOM- and
  CRLF-tolerant, quote-stripping, comment-skipping).
- **Parser** (`parse.ts`): `parseLesson` derives a `Realization` from lesson
  frontmatter (romanization defaults to headword for Latin scripts; gender sniffed
  from the gloss when unfielded); `buildDataset` joins content lessons through the
  taxonomy into concepts + per-language indexes.
- **Validator** (`validate.ts`): the round-trip consistency gate — resolves every
  concept tag, forbids duplicate realizations per language, checks required fields
  and field shapes, script-glyph coverage, and core-concept coverage. Errors fail
  CI; warnings/info are tolerated.
- **Queries** (`queries.ts`): `allConcepts`, `conceptsByLanguage`,
  `languagesForConcept`, `coverageByLanguage`.
- **Loader + CLI** (`loader.ts`, `cli.ts`): the filesystem boundary — reads the
  curriculum and runs `validate`. Declared `fs:read`/`fs:list` capabilities.
- Tests for the pure core (frontmatter, parse, validate, queries) plus an
  integration test that validates the **real** curriculum in CI and asserts the
  cross-language joins (e.g. `GREETING-HELLO` across all 16 tracks).

### Notes
- `data/scripts/*.json` character-breakdown data is authored incrementally in
  follow-up work; the package degrades gracefully when it is absent.
