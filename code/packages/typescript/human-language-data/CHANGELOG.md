# Changelog

All notable changes to `@coding-adventures/human-language-data` are documented here.

## [Unreleased]

### Fixed — "detachable" and "is a writing segment" are two different things

- `DETACHABLE_BLOCK_TYPES` gains `script`, so a hands-free renderer may set aside the
  inline-letters section. HL00 makes it optional scaffolding by design — "a reader who
  already knows the script skims that section" — and nothing later in the lesson depends
  on having read it.
- **This required separating two ideas the model had merged.** `writingSegments` was
  computed as `blocks.filter((block) => block.detachable)` — named for writing, filtered on
  detachability. That was harmless only while `writing` was the sole detachable type. The
  moment a second type joined, every inline-letters section counted as a writing segment,
  which set `hasWritingBlock` and dragged the lesson to `pen`: **`pen` 53 → 309, and 276
  reported "writing segments" that teach no writing at all.** Detachability is about what a
  renderer may skip; pen-ness is about what the learner's hand must do.
- `writingSegments` now filters on `block.type === "writing"`, and a new
  `detachableSegments` carries what a hands-free view sets aside — a superset.
- **Result: the book stays honest and the driver gets more.** Whole-lesson modality is
  unchanged (`voice` 726, `sight` 355, `pen` 53) because the printed book really does show
  glyphs; the core — what the driving edition reads — is **972 lessons, 86%**, above even
  the 84% that stood before the inline-letters section was classified honestly.
- `drivablePercent` is derived from `coreVoice` and now legitimately differs from
  `voice / totalLessons`. The invariant test was updated to assert the correct relationship
  rather than the coincidence that held while core and whole were always equal, and gained
  two more: the whole-lesson partition still closes, and `coreVoice >= voice` always
  (detaching can only help).
- A chapter whose only obstacle was a script section is no longer blocked; the gap
  report's blocked-chapter fixture was moved to a four-column paradigm, which the
  lineariser genuinely refuses, so the test still proves a real blocker gets named.
- **Next slice:** the manifest still publishes the conservative whole-lesson figure (64%)
  while the gap report publishes the core (86%). `coreModality` is the additive key HL-C44
  reserved for exactly this; emitting it and flipping `features.blockModality` closes the
  gap.

### Changed — the inline-letters section is a `script` block, honestly

- `## The letters in this word` — HL00's inline-letters section, used by **240 lessons
  across 12 tracks** — parsed as `unknown`, which schema v2 rejects. That single gap
  blocked the v2 migration for every Indic track at once. It now parses as `script`,
  which is what it has always been: the place a word lesson teaches the glyphs that word
  needs.
- **This costs 20 points of drivable share (84% → 64%) and that is the point.** A glyph
  shape cannot be read aloud, so the previous number advertised a driving edition that
  would have narrated "ब plus the o-mātrā" at somebody on a motorway. Corpus moves
  `voice` 957 → 726, `sight` 124 → 355, `pen` unchanged at 53, unstartable chapters
  44 → 92.
- **The loss is recoverable and the route is known.** HL-C41 gave `writing` blocks a
  `coreModality` so a hands-free view can set them aside, and the inline-letters section
  is detachable in exactly that sense — HL00 calls it optional scaffolding a fluent reader
  skims. Adding `script` to `DETACHABLE_BLOCK_TYPES` was tried and reverted here: the
  model currently conflates "detachable" with "is a writing segment", so script blocks
  began claiming a lesson needs a **pen** to read letters (`pen` 53 → 309) and reported
  276 writing segments that are nothing of the kind. Separating those two ideas returns
  the core share to ~86% with the honest label intact, and is the natural next slice.

### Added — HL-C10: the shared spine reaches above A1

- Add an **A2 tranche** of five spine nodes — `SPINE-SAY-WHAT-I-DO`,
  `SPINE-NEGATE-AND-ASK`, `SPINE-SAY-WHAT-I-WANT`, `SPINE-TALK-ABOUT-PAST`,
  `SPINE-TALK-ABOUT-FUTURE` — and the seven canonical concepts they own
  (`VERB-INFINITIVE`, `VERB-PRESENT-HABITUAL`, `VERB-NEGATE`, `QUESTION-POLAR`,
  `VERB-WANT`, `VERB-PAST`, `VERB-FUTURE`).
- **This unblocks the entire Easy-to-Advanced grammar arc, and nothing else could.**
  Schema v2 requires a canonical `spine_node`. Every one of the previous eleven nodes was
  an A1 social function — greeting, taking leave, counting to five — with nothing covering
  verbs or tense, so a lesson teaching a present tense had no node it could legally
  declare. The arc was unauthorable in v2 for all 22 tracks. It was found the hard way,
  by trying to migrate a Hindi verb lesson and discovering its chapter belongs to no node.
- All 22 realization ledgers declare where they stand on each new node. An unrealized node
  is recorded as `segments: []` **with `omits` naming every concept it is not yet
  delivering** — the validator requires this, and rightly: "we have not built this yet" is
  a recorded position, so the debt stays countable instead of being an absent key nobody
  can see. Today that is all 22 tracks on all five nodes; those numbers are the burn-down.
- The taxonomy grows 46 → 53 concepts. Each concept is owned by exactly one node, which
  the validator enforces, so a later tranche cannot quietly re-file a concept it wants.

### Added — HL-C03: the nine HL05 chapter gates, as measurement rather than judgement

- Add `src/chapters.ts` with all nine HL05 gates — `chapter-missing-capability`,
  `chapter-unknown-payoff-lesson`, `chapter-payoff-not-closed`,
  `chapter-payoff-not-representative`, `chapter-duplicate`, `chapter-title-drift`,
  `pattern-slot-not-closed`, `pattern-missing-production`, `pattern-multiple-atoms` —
  and publish them through the gap report's new `chapters` section.
- **Report-only, and that is the design, not caution.** 98 of the corpus's 377 book
  chapters carry no capability entry. Wiring these into `validateCurriculum()` as errors
  would have converted a measurement of pre-existing debt into 98 build failures on a
  corpus nobody had regressed. Per-track rollups carry a `clean` flag so a track flips to
  hard errors once its own debt is zero — the HL-V01 precedent, and the same reasoning
  that ships the LaTeX warning baselines unseeded.
- **The first published snapshot: 377 book chapters, 279 declared, 98 without a
  capability, 24 payoffs below the 0.5 representativeness floor, and zero unclosed
  payoffs, zero unknown payoff lessons, zero title drift, zero duplicates.** Three tracks
  — `chinese`, `japanese`, `latin` — are already clean and could flip to errors today.
- **`payoffsNotClosed` read 279 — every authored chapter — on the first run, and that was
  this module, not the corpus.** Introduced atoms live in a FLAT dotted frontmatter key
  (`introduces.knowledge`) plus block-level `hl-knowledge` directives; reading a nested
  `introduces: { knowledge }` object returns `undefined` for every lesson in the corpus,
  which silently empties the "taught so far" set instead of failing. The fix reads the
  union of both sources. A gate reporting total corpus failure is usually reporting on
  itself, and the pinned snapshot exists so that stays visible.
- The three `pattern` rules find nothing, because HL-C05 has not added the `pattern`
  lesson type yet. They are wired now so the first authored pattern is checked the moment
  it exists rather than being remembered later.
- Summary gains `chaptersWithoutCapability`, `chapterPayoffsNotRepresentative` and
  `chapterGateCleanTracks`, each `null` rather than `0` when a caller passes no ledgers —
  "not measured" and "measured, none found" are different facts.

### Changed — HL-C38: the generated books read as books, not as exports

- **`src/book.ts` gains one documented "book voice" section.** Lessons are
  authored as audio scripts (HL00) so a track can be recorded; the book view was
  printing those stage directions. It no longer does. The transformation is
  book-view only — `block.markdown` still holds every cue, and a future narration
  exporter must read it directly rather than reusing `bookVoice`.
  - `[PAUSE Ns]` is deleted. A reader paces themselves.
  - `[REPEAT xN]` becomes prose: *Twice through:* …
  - `[YOU <VERB>: …]` becomes a printed practice prompt. A run of bullets sharing
    one verb gets a single lead-in (*Say these aloud:*); a mixed or lone cue gets
    a per-bullet italic label (*Say it:*, *Write it:*, *Trace it:*). Twenty-eight
    cue verbs are mapped in `CUE_VOICES`, with a sentence-case fallback so an
    unmapped verb still prints as English. Writing and tracing prompts are real
    printed exercises and are never suppressed.
- **Printed block headings.** The internal block-type names are replaced from one
  table, `BOOK_BLOCK_TITLES`: `Guided Practice` → **Your turn**, `Wrap-up recall`
  → **Before you move on**, `You'll want to know first` → **What to know first**.
  The warm-up loses its printed label entirely and stands as the section's
  indented lead-in — several lessons share a chapter, and a bold `Warm-up.` five
  times on one spread reads like a worksheet. Headings the author extended with a
  descriptive tail are left untouched.
- **The chapter blurb is gone.** Every chapter opened with "This chapter is
  generated from the canonical micro-lessons used by Language Ladder. Each
  section stays independently resumable…". Books do not describe their build
  system.
- **Links: the book is a standalone artefact.** `absoluteBookLink` replaces
  `resolveMarkdownLink`. Absolute HTTP(S) citations (UT Austin, MSU, Wiktionary)
  stay live `\href`s; repository-relative destinations print their label with no
  link, because a reader holding the PDF cannot follow them. `sourceBaseUrl` is
  still required and validated in `book-generation.json` — it is that config's
  statement of where the curriculum lives — but it no longer reaches the
  renderer, so `BookGenerationTarget.sourceBaseUrl` and `MarkdownRenderContext`
  are removed.
- `bookVoice` and `bookBlockTitle` are exported for testing and reuse.
- Regenerated all 270 chapters. Source hashes are unchanged: no lesson file was
  edited, and `core/generated-book-hashes.json` is byte-identical.

### Added — HL08 narration export: the drivable course, out loud (HL-C16)

- Add `src/speech.ts`: the shared judgement of **what can be said aloud**. Markdown
  inline → words a voice can pronounce (emphasis, code fences, link destinations and
  the linguist's reconstruction asterisk removed; `→` `←` `·` given spoken readings),
  and Markdown tables → spoken utterances or a *reasoned refusal*. Both `modality.ts`
  and `narration.ts` import it, so "this lesson is drivable" and "the export can
  actually narrate this lesson" are the same question asked once.
- Add `src/narration.ts`: the pure narration builder. From the canonical lesson AST it
  produces typed segments — `speech`, `pause`, `repeat`, `prompt`, `table`,
  `table-skipped`, `activity` — plus the continuous plain-text script rendered from
  them. This is the **audio-script output HL04's one-source pipeline diagram has named
  since it was written and which nothing had ever built**.
- Add `src/narration-cli.ts`: `--write` / `--check`, modelled joint for joint on
  `book-cli.ts`. Writes `<language>/narration/chNN.txt` and `.json` for all 375
  chapters plus a hash manifest at `core/generated-narration-hashes.json`. `--check`
  compares byte for byte and exits 1, so a lesson edited without re-running the
  exporter fails the build instead of leaving a voice assistant confidently teaching a
  lesson that no longer exists.
- **`[PAUSE Ns]`, `[REPEAT xN]` and `[YOU …: …]` are preserved as structured
  directives, not flattened into prose.** Cue parsing is a depth-tracking bracket scan,
  because the corpus nests brackets inside cues for real
  (`[YOU SAY: the pattern — "[nā] [pēru]"]`), and a Markdown link that is not a cue is
  handed back intact rather than mistaken for one.
- **A `[YOU SAY: …]` cue is never treated as an answer key.** Cues become `prompt`
  segments with `scored: false`; only `hl-activity` contracts, compiled through
  `compileLessonActivities`, become `activity` segments carrying `acceptedResponses`.
  This is `activity.ts`'s own rule — runtime consumers use only the typed AST and never
  recover prompts or answers from learner-facing Markdown — and the narration export
  would have been the easiest place in the package to break it.
- **Tables are linearised, never dropped.** A two-column word→gloss table becomes
  *"नमस्ते means hello"*; a three-column table becomes labelled facts. A column with no
  heading is spoken as a bare value rather than refused, because `| Read | | Meaning |`
  — script, romanization, gloss — is the corpus's commonest practice-table shape and
  the blank heading is one a sighted reader does not have either. A run of pipe rows
  with no delimiter row is read as an unlabelled sequence for the same reason.
- **A table that cannot be linearised is spoken, not skipped**: the learner hears its
  size, its column headings, and why it needs eyes, and the lesson is marked `sight` so
  they are told before they start. `sight` and `pen` lessons still export in full,
  opening with a notice naming what they will need and which sections to leave until
  they have stopped.
- Target-script text carries its `romanization` alongside — *"خداحافظ (khodâ hâfez)"* —
  drawn from the **whole chapter's** headwords, so a lesson can pair a word a
  neighbouring lesson introduced. Pairing is whole-word only: the Arabic track teaches
  ا (*alif*) as its own lesson, and a plain substring replace turned سلام into
  `سلا (alif)م`, splicing the pronunciation guide into the middle of the word.
- Report `narration-block-unrenderable` when a lesson carries a table the export cannot
  speak yet claims `voice`, and `narration-activity-invalid` when an authored contract
  will not compile. Both are collected, never thrown — one bad directive must not
  silence a lesson.

### Changed — `maxLinearisableTableColumns` moves from 0 to 3

- The knob shipped at **0** in the modality slice on purpose: no lineariser existed,
  and claiming a table was speakable would have claimed a capability nothing
  implemented. The lineariser now exists, so the default is its measured value, **3**,
  and it is authored in `core/chapter-policy.json` (validated on load: an integer from
  0 through 16) rather than living only as a constant.
- Three, and not four, because that is where a table stops being a list of labelled
  facts a listener can hold — *"Language: Telugu. Hello: namaskāram. Source:
  Sanskrit."* — and starts being a grid whose meaning lives in the comparison *across*
  rows. The corpus's own four-column tables prove the point: `| | numeral | word | said |`
  has an unlabelled first column that means something only because of where it sits on
  the page. Measured over the 340 table-bearing lesson files: 99 are 2 columns wide,
  173 are 3, 60 are 4, and 8 are 5 or more.
- At width 3 the lineariser reads **371 of the corpus's 442 tables (84%)**, covering
  272 of the 340 table-bearing files. The corpus moves from **694 drivable lessons
  (63%) to 925 (84%)**. Of the 120 that still need eyes, 65 carry a wide table, 61
  point at the page in prose, 7 have a `script` block, and **52 need eyes for a wide
  table and nothing else** — HL08's table-remediation burn-down list, now measured.
- `modality.ts`'s `wide-table` rule no longer means "wider than N". It means *"the
  narration lineariser refuses it"*, which is strictly larger: a three-column table
  inside the limit is still unspeakable when its rows are ragged. Asking the exporter's
  own judgement is the only way `voice` can be a promise the export is able to keep.
- `report-cli.ts` reads the same policy width, so the published drivable percentages
  and the committed narration export can never be computed at different settings.
- `tableRowColumns` now delegates its cell splitting to `speech.ts`, so the count a
  lesson is judged on is produced by the same scan the narration is built from.

### Added — HL-C41 block-level modality: one lesson, two answers

- Add the `writing` lesson-body block type (`## Writing: …`), for a section that
  teaches the **hand** to form a letter — as against `script`, which teaches the
  **eye** to recognise one. It is the first and so far only **detachable** block type:
  nothing later in a lesson depends on it, so a renderer that cannot use a hand may
  set it aside and still deliver a coherent lesson.
- Derive modality at two scales. `LessonModality.modality` is unchanged and still
  describes the whole lesson — what the **book** signs. New `coreModality` describes
  the lesson minus its detachable blocks — what a hands-free view can deliver. New
  `coreDerived`, `coreReasons`, `blocks` (per-block `BlockModality`) and
  `writingSegments` expose the derivation. New `deriveBlockModality`,
  `lessonCoreText`, `isDetachableBlock`, `DETACHABLE_BLOCK_TYPES`,
  `strongerModality`, `weakerModality`.
- **This is why it exists, and it is not what an earlier framing assumed.** The
  project owner's ruling is that the book is a standalone artifact and keeps all
  writing content in full; a dictation-friendly edition is a *separate output view*
  over the same canonical source, exactly as the narration export is. `coreModality`
  is the metadata that view reads. It is a strict improvement for that view: today a
  lesson with any pen content is lost to a commuter wholesale, whereas block marking
  lets them take the voice core and defer only the segment.
- Sight cues and tables are now attributed to the block they occur in, so a cue inside
  a writing segment does not follow it out into the core, while a cue in ordinary prose
  still does.
- An authored `modality:` override **caps** the core, giving the invariant a hands-free
  view relies on: `coreModality` is never stronger than `modality`.
- `drivablePrefix` and `drivablePercent` now count the core; `coreVoice` and
  `lessonsWithWritingSegments` are published beside the unchanged `voice`/`sight`/`pen`
  counts so the book's numbers and the hands-free numbers reconcile in the gap report.
- New report-only finding `modality-writing-segment-not-separable`: a lesson that is
  not `type: writing` may carry one writing segment; several means it should be split
  or declared a writing lesson. `type: writing` lessons are exempt.
- **Measured no-op.** No track has authored an interspersed writing segment yet, so
  every lesson's core equals its full modality and no published number moves — the
  regenerated `core/lesson-modality.json` is byte-identical in its summary (1,133
  lessons, 725 `voice`, 64% drivable). Pinned as `coreVoice === voice` alongside
  `lessonsWithWritingSegments === 0`, so the first interspersed lesson has to break the
  equality deliberately. Deliberately *not* pinned as an absolute literal here: the
  corpus totals live in one place, `modality-manifest.test.ts`, against the generated
  manifest.
- `features.blockModality` stays **false**: this change derives block modality but the
  manifest does not yet emit block rows, and the flag exists precisely so a consumer can
  tell those two states apart.
- Amends [`HL08`](../../../specs/HL08-modality-gentle-ramp-and-the-drivable-course.md),
  which had assumed one modality per lesson.

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
- `maxLinearisableTableColumns` defaulted to **0** in this slice: until HL08's
  narration exporter could linearise a two-column table into speech, no table was
  speakable, and claiming otherwise would let a learner silently miss content they
  were never told they had missed. (Superseded above: HL-C16 built the lineariser and
  the default is now 3.)
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
