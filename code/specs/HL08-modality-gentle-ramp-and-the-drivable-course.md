# HL08 — Modality, the Gentle Ramp, and the Drivable Course

## Status and purpose

This spec makes two implicit properties of the curriculum explicit and checkable:

1. **Which channel a lesson needs** — can it be learned by ear alone, does it need
   eyes, does it need a pen? — marked with signs in the book and exported so a
   voice assistant can teach the hands-free parts aloud.
2. **How steep the ramp is** — how much new material a single lesson may introduce,
   enforced as a budget rather than left to authorial judgement.

It extends [HL00](./HL00-human-language-curriculum-framework.md), whose lessons are
already written in audio-script style with `[PAUSE Ns]`, `[REPEAT x2]` and
`[YOU SAY: …]` cues, and implements the **audio-script output** that
[HL04](./HL04-shared-spine-and-content-pipeline.md)'s one-source pipeline diagram
names but nothing has ever built. Chapter-level signs attach to the capability object
defined in [HL05](./HL05-chapter-capability-and-step-by-step-shape.md).

The design requirements are:

1. A learner in a car can be taught, aloud, everything that does not require eyes.
2. A lesson that needs eyes or a pen says so, visibly, before the learner starts it.
3. The ramp is gentle by measurement, not by assertion.
4. Length is not a cost. A longer book made of smaller steps is the preferred outcome.

## The gap this closes

### Modality is real but undeclared

`skills: [listening, speaking, reading, writing]` already exists on every schema-v2
lesson — but it records what a lesson **develops**, not what it **requires**. 501 of
the 531 schema-v2 lessons declare `[listening, speaking, reading]`, and a reader can
learn *hola* perfectly well by ear despite the `reading` entry. Modality therefore
cannot be derived from `skills`, and treating the two as the same thing would
mislabel almost the entire corpus.

Deriving instead from lesson type and block structure, measured over all 1,096 lessons:

| Requirement | Lessons |
|---|---|
| `type: writing` — needs a pen | 51 |
| Carries a `script` block — needs eyes | 7 |
| Neither | 1,038 |

That 1,038 is not the drivable count, because a lesson can be sight-dependent without
a script block:

| Among those 1,038 | Lessons |
|---|---|
| Contain a Markdown table | 322 |
| Contain a sight cue (*"see the"*, *"look at"*, *"the chart"*, *"column"*) | 56 |
| **Genuinely free of both** | **695** |

So roughly **63% of the corpus is drivable exactly as authored**, and the single
largest obstacle to the rest is the table — not the script. That is a tractable
problem: a two-column word→gloss table reads aloud fine, while a five-column paradigm
does not.

### The ramp is already gentle, and undefended

Knowledge atoms introduced per schema-v2 lesson:

| Statistic | Value |
|---|---|
| Mean | 2.31 |
| Median | 2 |
| p90 | 3 |
| Max | **7** (`ES-C31-numeros-11-20`, since split by HL-C18) |
| Lessons introducing more than 3 | 52 |
| Lessons introducing more than 5 | 5 |

The curriculum is already gentle in aggregate. What it lacks is a floor: nothing stops
the next lesson from introducing nine atoms, and the original worst case taught ten
numbers at once — precisely the "drilling ten greetings" antipattern HL00 was written
to reject.

**Burn-down (HL-C18, Spanish slice).** The fifteen over-budget Spanish lessons became
thirty-three prerequisite-ordered micro-lessons, dropping the corpus figure from 52 to
37 and the corpus maximum from 7 to 6. `ES-C31-numeros-11-20` is now the four-lesson
chain `ES-C31-once-quince` → `ES-C31-dieciseis-diecinueve` → `ES-C31-teens-latinos` →
`ES-C31-veinte`. The remaining 37 belong to sixteen other tracks and are unchanged.

## Modality

Three channels, each naming what the learner must have available:

| Value | Meaning | Sign |
|---|---|---|
| `voice` | Learnable by ear alone. A voice assistant can teach it while the learner drives. | 🚗 |
| `sight` | Needs eyes — letter shapes, figures, or a table that cannot be read aloud. | 👁 |
| `pen` | Needs a hand — handwriting formation and practice. | ✍ |

Modality is **monotonic**: `pen` implies `sight`. A chapter's modality is the union of
its lessons'.

> **Amended (HL-C41).** This section originally gave each lesson exactly one modality.
> That is still what the book prints, but it is no longer the whole model: modality is
> now derived at two scales, the lesson and the block. See
> [Block-level modality](#block-level-modality-hl-c41) below.

### Derived by default, overridable with a reason

Hand-annotating 1,096 lessons invites drift, so modality is computed:

1. `type: writing` → `pen`;
2. otherwise a `script` block, a sight cue, or a table wider than the configured
   linearisable width → `sight`;
3. otherwise → `voice`.

An author may override with an explicit `modality:` in frontmatter, but an override
that contradicts the derivation requires a `modality_reason:`. The validator reports
unexplained overrides. This keeps the common case free and the exceptional case honest.

### The drivable prefix

A chapter reports how many of its lessons, **in authored order**, are `voice` before
the first one that is not. This is the number that matters to a commuting learner:
*"you can do the first six of this chapter's nine lessons in the car."*

The book prints it at the chapter opening beside the capability from HL05. It is
derived, never authored.

Ordering note (from the implementation): only schema-v2 lessons carry an explicit
`sequence`. Legacy lessons without one sort last, by id, rather than being given an
invented position — the same comparator the book generator already uses, so the
prefix counts lessons in the order the reader actually meets them.

### As implemented — migration step 1

`code/packages/typescript/human-language-data/src/modality.ts` implements the
derivation, the monotone closure, and the drivable prefix; the gap report publishes
them. Two things to record where the next reader will find them:

- **The linearisable width starts at 0, not 2.** The narration exporter that turns a
  two-column table into speech is migration step 3 below. Until it exists, calling a
  table speakable would claim a capability nothing implements, and the failure mode
  is the worst available — a learner told a lesson is drivable who then silently
  misses what the table carried. Every table means eyes today; the width moves to 2
  when the lineariser lands.
- **The measured drivable count is 694, not 695.** Every structural count in the
  tables above reproduces exactly (51 `pen`, 7 `script` blocks, 1,038 remaining, 322
  with a table). The gap is entirely in the sight-cue list, whose exact contents this
  spec never recorded: the implemented list matches 61 lessons where the original
  measurement found 56. The detector was not tuned to close the difference, because a
  cue list fitted to a target number measures the target, not the corpus.

### The modality manifest — migration step 1b (HL-C44)

Step 1 above computed modality and printed it into a human-readable report. That is
enough for a person and useless to a program, and the project then committed to
shipping **two editions from one canonical source**: the complete book, keeping
everything including the writing instruction, and a dictation-friendly **driving
edition** omitting what a driver cannot do. A prose paragraph is not something either
builder can filter on, so the derivation is now also emitted as data.

`code/learning/human-languages/core/lesson-modality/*.json` is generated by
`modality-cli --write` and gated by `modality-cli --check`, which runs in CI beside
`check:books`. It is a derived artifact in exactly the sense
`core/generated-book-hashes.json` is: nobody edits it, and it cannot drift, because a
stale manifest fails the build. The drift is not cosmetic — a lesson that quietly
gained a paradigm table would still read `drivable: true`, and the driving edition
would hand a chart to somebody at 70mph.

Each language owns one shard. Consumers reassemble the same corpus view at read time,
including its summary and source hash, while parallel curriculum PRs update disjoint
files instead of serializing on one generated artifact.

It carries per lesson `id`, `language`, `chapter`, `sequence`, `modality`, `derived`,
`drivable`, `reasons`, and the lesson AST's `sourceHash`; per chapter the drivable
prefix, the first blocker, and the prefix's lesson ids in order; per track the rollup;
and a corpus summary. This does **not** reverse the decision above: modality is still
derived, not authored into 1,096 frontmatter files.

**Reserved for block-level modality — since claimed by HL-C41, specified below.** At the
time this manifest landed, modality was a whole-lesson property, and that was the model's
real limit: a lesson taught entirely aloud that ends with a short "now trace the letter"
segment is stamped `pen`, and the driving edition drops all of it. The manifest is shaped
so per-block modality lands additively — every lesson
row is an object, `modality` permanently means the strongest channel the lesson needs
anywhere (so a consumer that never learns the new key stays correct, merely
pessimistic), `coreModality` arrives beside it as an optional key read as
`entry.coreModality ?? entry.modality`, and `features.blockModality` in the header
announces whether a build carries block data. The shape of the companion block records
is deliberately left unspecified here rather than guessed: an absent key is additive, a
wrong one is a breaking change.

## Block-level modality (HL-C41)

### What the amendment is for

The project owner asked for handwriting to be **interspersed**: a letter introduced
earlier comes back for two minutes inside an ordinary five-minute lesson, building
the hand up gradually, instead of being batched into separate writing lessons the way
Hindi (11), Arabic (16) and Tamil (8) currently do.

Under one-modality-per-lesson, those two minutes cost the whole lesson. Any `pen`
content makes the lesson `pen`, so a listener loses all five minutes of it — including
the four that were perfectly listenable. That is not a true statement about the
lesson; it is a limitation of the model.

### What it is *not* for

An earlier framing of this work treated the amendment as a way to protect the drivable
percentage — to keep books from being dragged back to `pen`. **That framing is
rejected, and this section records the rejection so it is not re-derived later.**

The project owner's ruling:

> *"Remember that the book is a standalone artifact. We can publish a driving only book
> later which doesn't teach writing. But for now, include the writing lessons in the
> books. Then we can always add a dictation friendly book that can be used while
> driving."*

So:

1. **The book keeps all writing content, in full.** No writing segment is omitted,
   deferred, shortened, or moved in the book view. A reader with the PDF gets the
   complete course, including how to form the letters. Nothing in this spec may
   thin the book to serve driving.
2. **A dictation-friendly edition is a separate output view** over the same canonical
   source — the same relationship the narration export already has to the book. Block
   modality is the metadata *that* view reads.
3. **Drivability is a measure of that view's reach, not of book quality.** If honest
   handwriting instruction lands in a track and the track's `pen` count rises, that is
   the correct outcome and no gate here may push back on it.

Read positively, block-level marking is a strict improvement for the driving edition
too. Today a lesson with any pen content is lost to a commuter wholesale; with block
marking they get the voice core and defer only the segment.

### The model

Two derivations over the same rules, differing only in what they read:

| Scale | Reads | Answers |
|---|---|---|
| **full** | the whole lesson, every block | what the **book** signs at the chapter opening |
| **core** | the lesson minus its **detachable** blocks | what a hands-free view can deliver |

A block type is **detachable** when nothing later in the lesson depends on it, so a
renderer that cannot use the channel it needs may skip that block whole and still
deliver a coherent lesson. Exactly one block type is detachable today:

| Block type | Heading | Modality | Detachable |
|---|---|---|---|
| `script` | `## Script — …` | `sight` — teaches the **eye** to recognise a letter | no |
| `writing` | `## Writing: …` | `pen` — teaches the **hand** to form one | **yes** |

The set is deliberately tiny. Every addition to it is a promise about content that only
an author can make honestly, and "detachable" is a claim about *renderers*, never about
readers: the book prints all of them.

Rules, extending the three above rather than replacing them:

1. `type: writing` → `pen`, for **both** scales. The whole lesson is the writing; there
   is nothing separable to set aside.
2. A `writing` **block** → the lesson's full modality is `pen`; the core is derived from
   the remaining blocks. This is the interspersed case.
3. Sight cues and tables are attributed to the block they occur in. A cue inside a
   writing segment does not follow it out into the core; a cue in ordinary prose does.
4. An authored `modality:` override speaks for the lesson as a whole and therefore
   **caps** the core. The invariant that falls out — *the core is never stronger than
   the full modality* — is what lets a hands-free view trust `coreModality` alone.

### Separability, enforced

An interspersed lesson carries **one** writing segment. A lesson that sprouts several
has stopped being an ordinary lesson with an aside and should either be split or
declared `type: writing` outright. Reported as `modality-writing-segment-not-separable`,
report-only per the HL-V01 precedent. A `type: writing` lesson is exempt — many writing
blocks are exactly what it is for.

### What moves, and what does not

- **The drivable prefix counts the core.** A lesson that is voice apart from a two-minute
  writing aside no longer ends a commuter's run through a chapter.
- **`drivablePercent` is computed from `coreVoice`**, not `voice`. The `voice`/`sight`/`pen`
  counts still describe the book, and both numerators are published side by side so the
  difference is visible rather than silent.
- **Nothing else.** No track has yet authored an interspersed writing segment, so every
  lesson's core equals its full modality and the published corpus figure is unmoved at
  **708 drivable, 65%**. The implementation pins that as a regression test, along with
  `lessonsWithWritingSegments === 0`, so the first interspersed lesson has to move the
  number deliberately.

## The narration export

A fourth output view beside the book, the app, and the exercise bank — the one HL04's
pipeline diagram already promises.

For every lesson and chapter, `narration-cli` emits from the canonical AST:

- **Plain text** — a continuous script an AI voice assistant or TTS engine can read.
- **Structured JSON** — blocks, cues, and prompts with their types preserved, so a
  voice agent can pause where the lesson says pause, wait for a spoken answer where
  the lesson says `[YOU SAY: …]`, and score it against the compiled activity contract
  from HL-V03 rather than against prose.

Rules:

- The existing `[PAUSE Ns]`, `[REPEAT x2]` and `[YOU SAY: …]` cues are preserved as
  structured directives, not flattened into prose.
- **Tables are linearised into speech**, not dropped. A two-column table becomes a
  sequence of *"X means Y"* utterances. A table that cannot be linearised within the
  configured width marks its lesson `sight` — the export never silently omits content
  the learner would then not know they had missed.
- `sight` and `pen` lessons still export, prefixed with a spoken notice naming what
  the learner will need and what they can safely skip until they stop driving.
- Target-language text carries its `romanization` alongside, so a voice engine reading
  a Latin-script transcription is never guessing at the script.
- The export is hash-gated against the lesson AST exactly like generated `.tex`, so
  narration cannot drift from the book.

The export is a **script for a voice agent, not a recording**. Producing audio,
selecting voices, and provenance-labelling recordings remain out of scope, as in HL04.

### What shipped, and where it diverged (HL-C16)

Migration step 3 is complete. `speech.ts`, `narration.ts` and `narration-cli.ts` write
`<track>/narration/chNN.txt` and `.json` for all 375 chapters, hash-gated by
`core/generated-narration-hashes.json` and checked byte-for-byte in CI. Three points
where the implementation went further than, or differently from, the text above:

1. **The width is 3, not 2.** This spec's worked example is a two-column table, but
   the corpus's own distribution argues for three: of 340 table-bearing lesson files,
   99 are 2 columns wide and 173 are 3. A three-column row reads aloud as labelled
   facts — *"Language: Telugu. Hello: namaskāram. Source: Sanskrit."* — which a
   listener holds without effort. Four is where the meaning moves into the comparison
   *across* rows, and the corpus's four-column tables prove it: `| | numeral | word |
   said |` has an unlabelled first column that means something only because of where
   it sits on the page. At 3 the lineariser reads 371 of 442 tables, and the corpus
   goes from **63% drivable to 84%** (694 → 925 of 1,096 lessons).

2. **Modality asks the lineariser, rather than counting columns.** This spec says "a
   table wider than the configured linearisable width → `sight`". The implementation
   asks the exporter itself whether it can speak the table, which is strictly larger:
   a three-column table inside the width is still unspeakable when its rows are ragged
   or its heading row has nothing under it. Counting columns would have let `voice`
   mean something the export could not deliver.

3. **`narration-block-unrenderable` is a narration finding, not yet a
   `validateCurriculum()` gate.** It is emitted per lesson and collected into the
   export manifest, alongside a new `narration-activity-invalid` for a contract that
   will not compile. Folding both into the validator's finding list is the remaining
   piece of the gates table below.

**Remaining table debt** (HL08 migration step 4, tracked as HL-C17): 71 tables across
68 lesson files are four columns or wider. 52 lessons need eyes for a wide table and
nothing else, so reshaping just those tables moves 52 more lessons into the car.
Raising the width to 4 would reach the same number without earning it, and is
explicitly not the fix.

## The gentle-ramp budget

A new configured limit in `core/chapter-policy.json`:

- `maxNewAtomsPerLesson` — default **3**, at the corpus's existing p90.
- `maxNewAtomsPerChapter` — a chapter-scoped ceiling, so gentleness cannot be gamed by
  splitting one steep lesson into two steep ones.

Exceeding the budget is a violation to be **split, not waived**. This is the same move
the HL-D01 series already made for the five-minute rule: the fix for a lesson that
introduces seven atoms is more prerequisite-ordered lessons, each teaching less.

52 lessons currently exceed a budget of 3, and 5 exceed 5. That is the burn-down list,
recorded as debt rather than treated as a new regression.

### Length is explicitly not a cost

Splitting for gentleness makes every book longer, and that is the intended direction.
A book of thousands of pages made of two-minute steps is a better outcome than a
compact book that loses the reader in chapter three. No gate in this project may
penalise page count, lesson count, or chapter count.

## Amendment — the script ramp (HL-C18C)

*Added 2026-08-07, on the project owner's direction that "the ramp should also include
the script; sometimes you can't introduce more than one script at a time."*

### Why the atom budget was not enough

`maxNewAtomsPerLesson` counts **units of meaning**. It cannot see the other burden a
lesson imposes, and the two come apart badly:

> `HI-W01-shirorekha-na-ma` declares **one** atom and puts **twelve** new Devanagari
> glyphs on the page. It passes the gentle-ramp budget comfortably.

It is not an outlier. **61 lessons** exceed three new glyphs; **38 of them declare zero
atoms**, so they read as maximally gentle while teaching up to a dozen new shapes.
Before a learner can *mean* anything by नमस्ते they must decode it, and decoding is a
separate skill on a separate curve. A curriculum that ramps meaning gently and script
steeply is not gentle.

### The two new budgets

- `maxNewGlyphsPerLesson` — default **3**, at the corpus's own p90 for target-script
  glyphs, the same rule that justified `maxNewAtomsPerLesson`. The median non-Latin
  lesson introduces **zero** new glyphs, so this flags 61 genuine spikes rather than
  taxing ordinary lessons. It is deliberately **not** set at the observed max of 12: a
  budget placed at the worst case is not a budget.
- `maxNewScriptSystemsPerLesson` — **1**, the owner's rule stated directly. Today it
  flags five lessons, all Japanese Chapter 1, which opens kanji beside hiragana in its
  very first lesson and adds katakana in its fifth.

### Target script versus the cousin layer

The measurement counts a lesson's **target-script** glyphs — the ones the learner is
signing up to read. Glyphs from *other* scripts are counted, reported, and **never
charged to the budget**.

That distinction is what makes the number honest. A Kannada Chapter 1 lesson that shows
the same word in Devanagari, Tamil, Telugu and Malayalam looks like a **34-glyph cliff**
when the two are conflated. Its actual Kannada load is **7**. The sister-script material
is *context* — it says "your word for thanks is Hindi's word too" to a reader who
happens to know Hindi — and per the owner's rule that **English is the only requirement
for each book**, it must be skippable by everyone else.

So the cousin layer is not penalised. What its measured footprint justifies — 119
lessons, up to 26 foreign glyphs in one — is keeping that layer **visually separable**,
so a reader who knows no sister language can pass it by without passing by the lesson.

### Counting rules, and why each one is deliberate

- **In reading order, charged once.** A glyph taught in Chapter 1 is free in Chapter 30.
  This is a ramp measurement, not a density measurement; charging revision would invert
  the incentive the whole spec exists to create.
- **Latin is not counted.** Romanization (`namaskāram`) rides alongside every non-Latin
  headword. Counting `ā` as a glyph to learn would swamp the signal with the very thing
  that exists to make the script approachable.
- **Combining marks are counted.** A Devanagari mātrā is a shape the reader must decode;
  dropping marks would undercount every abugida in the corpus.
- **Script digits are counted.** ०१२ and ۱۲۳ are `\p{N}`, and also glyphs nobody born to
  ASCII can already read. A numbers lesson genuinely does teach script.
- **`Script_Extensions`, not `Script`.** Japanese's prolonged-sound mark ー is formally
  `Script=Common` because hiragana and katakana share it. The narrow property undercounts
  コーヒー by the mark that makes it a long vowel.
- **Latin-script tracks carry no decoding burden** and are measured but never flagged.

### Report-only, per the HL05 precedent

The debt predates the measurement, so it is published and burnable rather than turned
into a build failure on a corpus nobody regressed. Gates flip per track as debt clears.

### A prerequisite this surfaced

The ramp could not see Gujarati at all: `LANGUAGE_SCRIPT` never got a `gujarati` entry —
Gujarati was the *worked example in its own doc comment* — so all 39 lessons resolved to
`latin`. Glyph-coverage validation looked Gujarati headwords up in the Latin inventory,
and `romanization` fell back to the Gujarati headword itself, handing the narration
export **Gujarati script in the field a speech engine reads as Latin**. Chinese and
Japanese were missing from the same map and were saved only by shipping a `track.json`.
A track whose script is unknown reads as having no script to learn, so completing that
map is part of this amendment, not a separate concern.

## Validation gates

Added to `validateCurriculum()`, multi-pass, collecting every violation before
reporting.

| Code | Rule |
|---|---|
| `modality-unexplained-override` | an authored `modality` contradicting the derivation has no `modality_reason` |
| `modality-unknown-value` | `modality` is not one of `voice`/`sight`/`pen` |
| `modality-writing-segment-not-separable` | a lesson that is not `type: writing` carries more than one `writing` block |
| `narration-block-unrenderable` | a block cannot be linearised into speech and the lesson is not marked `sight` |
| `ramp-budget-exceeded-lesson` | a lesson introduces more than `maxNewAtomsPerLesson` |
| `ramp-budget-exceeded-chapter` | a chapter introduces more than `maxNewAtomsPerChapter` |
| `narration-drift` | the committed narration export does not match the lesson AST |

All land **report-first**, per the HL-V01 precedent, and flip to errors per track as
each track's debt clears.

The gap report gains a modality section: per track, the count of `voice`/`sight`/`pen`
lessons, each chapter's drivable prefix, and the corpus-wide drivable percentage — so
"how much of this can I do in the car?" is a measured number, published every build.

## Migration order

1. **Derivation and report** — modality computed for all 1,096 lessons; drivable
   prefixes and ramp debt published. No gates.
2. **Signs in the book** — chapter openings print modality and drivable prefix beside
   the HL05 capability; lessons carry an inline marker.
3. **Narration export** — `narration-cli` with `--write`/`--check`, table
   linearisation, and the hash gate.
4. **Table remediation** — the 322 table-bearing lessons reviewed; each table either
   linearised or its lesson honestly marked `sight`.
5. **Ramp burn-down** — the 52 over-budget lessons split into prerequisite-ordered
   micro-lessons, longest first.
6. **Interspersed writing** (HL-C41) — block modality derived and published; tracks
   author `writing` segments inside ordinary lessons as their scripts' stroke data
   becomes citable. The book prints them; the future dictation edition skips them.

## Acceptance criteria

Complete when every lesson has a derived or explained modality; every chapter reports
a drivable prefix; the book prints modality signs at every chapter opening; the
narration export round-trips every lesson and is hash-gated against the AST; every
table is either linearised for speech or its lesson is marked `sight`; no lesson
exceeds the ramp budget; and the published gap report states what percentage of each
track can be learned entirely by ear.
