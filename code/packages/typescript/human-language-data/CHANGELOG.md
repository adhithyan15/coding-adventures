# Changelog

All notable changes to `@coding-adventures/human-language-data` are documented here.

## [Unreleased]

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
- `modality.test.ts`: total lessons 1,096 → 1,103; `voice` 694 → 699; `sight`
  351 → 353; non-writing lessons carrying a `script` block 7 → 9; the remainder
  1,038 → 1,043. The `pen` count (51), the table-bearing count (322) and the
  corpus-wide drivable share (63%) are unchanged, because no Chinese lesson needs a
  pen and none carries a table.

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
