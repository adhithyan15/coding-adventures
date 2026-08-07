# Changelog

All notable changes to `@coding-adventures/human-language-data` are documented here.

## [Unreleased]

### Added — every generated chapter opens by saying what the reader will be able to do (HL-C49)

- **288 of 407 chapters opened on a bare title** — `\chapter{}`, `\label{}`, straight
  into the first lesson section. Nothing told the reader why they were there. All
  **302 generated chapters** now carry a short opening, and **all 302 had the data
  already**: every one has a `canDo` in its HL05 capability ledger.
- **Derived, never authored.** `book.ts` composes the opening from `canDo` and
  `payoff.summary`. 302 hand-written intros would be 302 places to drift from the
  lessons they describe, and the generated file says at the top that editing it is
  pointless. `canDo` is quoted verbatim, so the book and the ledger cannot disagree
  about the same sentence.
- **It must stand alone in English**, per HL09 §8 — the book is a standalone artifact
  and English is its only requirement. Naming a *source* language is not a violation
  and is the point of the book ("negro inherited from Latin", "trace *hermano* through
  *germānus*"); naming another **track of this course** is, because it dangles for a
  reader holding one PDF. One real violation was found and fixed at source: Telugu
  ch11's payoff said "the borrowed blue every language in this course now shares".
- The blurb that used to sit here explained how the chapter was *produced*. Removing
  it was right; leaving nothing was not.

### Fixed — the reconstruction asterisk was being deleted, turning reconstructions into attested forms

- `renderInlineMarkdown` reads a bare `*` as an italic opener, so `PIE *ne` printed as
  `PIE ` with the rest of the sentence italicised. In five chapters across German,
  Hindi and Telugu that **silently converted a reconstructed form into an attested
  one** — a false etymological claim, in the part of the book that exists for
  etymology. Lesson authors already wrote `\*`; the ledger authors did not, and
  nothing warned them. Escaped at source, with a test.

### Fixed — four books explained their own build system to the reader

- Four `payoff.summary` fields ended with a note addressed to the gap report:
  *"Chapter 17 has no terminal practice lesson, so the payoff is the last lesson by
  sequence (4 of 12 atoms, below the 0.5 floor)."* Printing that under the chapter
  title broke the very rule that got the old blurb removed. Moved to a
  non-printed `payoff.note`; a test rejects it returning.

### Known follow-up

- `canonicalChapterHash` covers lessons only, not the capability. CI still catches a
  stale chapter — `book-cli --check` compares full text, and the workflow's path
  filter includes `chapters.json` — but `core/generated-book-hashes.json` is
  byte-identical after a capability edit, so `language-ladder`'s `bookHashStatus`
  reports a genuinely stale `.tex` as synced. Folding the capability into the hash is
  the fix, and it regenerates every chapter, so it ships separately.

### Added — a track must EARN a level, not touch one (HL09 §3.1)

- Add `src/level-gate.ts`. The gap report now publishes two numbers per track where
  it published one:

      levels: 650 pre-A1, 297 A1, 186 A2; 148 unmapped (88% placed)
      levels ATTAINED (HL09 §3.1): none; 22 tracks touch a level they have not attained

- **This is the gate that would have caught "Spanish reaches A2".** Nothing lied:
  `TrackLevelCoverage.reach` is documented as *the highest level this track has any
  lesson at*, and that was true. The mistake was letting a number that means
  **touches** be read as **means**. One lesson pointing at one A2 node moves `reach`;
  it is nowhere near enough to sit the exam.
- `touches` keeps the old meaning. `attained` is the highest level where all four
  §3.1 criteria hold at that level **and every level below**: every spine node
  realized, cumulative vocabulary met, no lesson over the atom budget, every atom
  revisited twice. **Zero of 22 tracks have attained even pre-A1.**
- Spanish is *in progress at pre-A1*: **44 distinct headwords at or below pre-A1
  against a 300 target** (shortfall 256), plus 92 atoms revisited fewer than twice.
- **Every criterion is scoped "at or below the level", and getting that wrong was the
  first version of this module committing the exact error it exists to catch.** The
  initial implementation measured whole-track vocabulary (Spanish 138) against a
  per-level cumulative target, and applied the atom-budget and reinforcement criteria
  track-wide — so Hindi's single over-budget lesson, which sits *above* pre-A1, blocked
  pre-A1, making criterion 3 unfalsifiable at the bottom of the ladder. Security review
  caught it; the honest pre-A1 vocabulary is **44**, not 138.
- Criterion 4 counts atoms revisited **fewer than twice**, per §3.1 — not "never
  revisited". The looser reading hid 51 of Spanish's 141 failures.
- Vocabulary counts only `CONTENT_TYPES` lessons. Counting every lesson type credited
  drill titles and grammar labels as vocabulary — `(practice)`, `qu-`, `fact or wish?` —
  25 of Spanish's 138.
- A level with **no authored spine nodes fails** criterion 1 rather than passing it
  vacuously. `spine.json` has zero B1-C2 nodes, and "no node is unrealized" is not
  "every node is realized" — the same touches-vs-means error, one level up.
- **Failures name the criterion and the shortfall**, not a bare `false` — a boolean
  would move the argument rather than settle it. `vocabulary: teaches 138 distinct
  headwords against 300 for pre-A1, shortfall 162`.
- The gate stops at the **first** failing level, because the criteria are cumulative:
  a level above a failing one is unreachable by definition.
- Vocabulary targets live in `LEVEL_VOCABULARY` and are **editorial** per §10 —
  conventional working figures for CEFR receptive vocabulary, not a claim about any
  awarding body's syllabus. They are named so a failure can cite what it was measured
  against.
- Absent, not empty, when the caller supplies no policy: *not measured* and *attained
  nothing* are opposite facts, and a test pins that distinction.

### Fixed — the CLI had never once printed the level figures

- `report-cli` never passed `curricula` or `spine` to `buildCurriculumGapReport`, so
  the `levels` section has been silently absent from every CLI run since HL-C10
  shipped it. Both `levels` and the new gate now render. The section existed, was
  tested, and was invisible to anyone reading the report.

### Changed — 17 R1 reinforcement windows closed in Spanish chapters 3-6 (HL09 step 3)

- Records practice the lessons **already do**. 17 atoms across 11 lessons gain an entry
  in `practises.knowledge` **and** in the `assesses=[...]` directive of the specific
  body block that exercises them.

      corpus R1 misses      766 -> 749   (exactly the 17 wired)
      corpus never revisited 767 -> 755   (12; five already had a distant revisit)
      Spanish never revisited 102 -> 90 of 199

  Those are the figures on the corpus of the day. A verb tranche landed on main in
  parallel, so the committed pins read 1599 atoms / 745 never revisited / R1 778;
  what this change is accountable for is the 17 windows and the 12-atom move.

- **Only 17 of the 58 R1 misses in these chapters could be wired.** The other **41 are
  genuine absence** — no lesson in the window touches the atom at all, so there is no
  practice to record. That is what HL09 §7.2 predicted, and it is the honest result:
  a `practises` entry the prose does not back is worse than an open window.
- **A frontmatter-only edit was tried first and rejected by the schema.** Adding an
  atom to `practises.knowledge` without declaring it in a body block fails validation
  with `schema-v2-block-assessment-missing`. That rule is the schema enforcing HL09
  §7.2's honesty principle directly: **you cannot claim practice without pointing at
  where it happens.** The rejected attempt was reverted, not worked around.
- Placement is evidenced, not guessed. Each atom went to the block containing the
  drill or recall that exercises it — mostly `## Guided Practice` and
  `## The word, taken apart`. A "what you've learned" bullet was **rejected as
  practice** five times during the audit; a recall *task* ("order the three *hasta*
  goodbyes by time") was accepted, because it cannot be done without the words.
- **R2 is unchanged at 1107, and that is correct** — closing a near window does not
  close a far one. R2, R3 and R4 need dedicated `review` lessons, per §7.2.
- **Open question for the project owner: 15 of 18 `ES-ETYMON-*` atoms could not be
  wired.** This is systematic rather than eighteen oversights — an etymon is cited when
  introduced and never re-cited; only `hasta` comes back. Either etymon atoms should be
  exempt from the retrieval schedule, or lessons should re-cite earlier etymons the way
  they re-use vocabulary. Not decided here.

### Changed — Spanish's `sequence` numbers are renumbered on a clean 10-spaced run

- Every one of Spanish's 148 sequenced lessons is renumbered to **10, 20, 30 … 1680**,
  in the same reading order it already had. **Relative order is unchanged**, and so
  is every measurement derived from it: forward prerequisites 5, forward reviews 6,
  forward references 99, atoms 199/102. Byte-identical answers, different integers.
- **Why it needed doing.** HL09 step 2 had to fit chapters 7–18 into the 129 integers
  between 510 (chapter 6's end) and 640 (chapter 19's start), because chapters 19–33
  were already sequenced at 640–845. That forced a spacing of **2**. Gap census before:
  51 gaps of 2, 33 of 5, and a scattering of 3s and 4s. After: **147 gaps, every one of
  them 10.**
- **Chapter 7 now has room.** Its six lessons are still unsequenced pending the owner's
  ruling on their order, but the renumbering reserves **210 numbers — 21 slots — between
  chapter 6 and chapter 8** for six lessons plus the splits they will need. Previously
  the gap was 10, which would have forced a second renumber the moment chapter 7 landed.
- Safe by construction, and verified rather than assumed: the security review of #10047
  confirmed **nothing consumes a sequence's absolute value** — every comparison in
  `ramp.ts`, `book.ts`, `modality.ts`, `hash.ts` is relative, and the only absolute
  predicate is `curriculum.ts`'s `Number.isInteger(sequence) && sequence > 0`. The values
  are persisted verbatim into three generated artifacts, so this is a regeneration event,
  and the byte-exact `--check` CLIs fail loudly rather than silently on a stale one.
- Diff shape confirms the claim: lesson files changed **only** in their `sequence:` line,
  and the 21 regenerated book chapters changed **only** in their `canonical-source-hash`
  comment. No rendered content moved.

### Changed — Spanish has a declared reading order (HL09 step 2)

- 50 Spanish lessons across chapters 8–18 gain a `sequence:`, recovered from
  evidence rather than invented: the `Next: …` sentence ending each lesson's
  Wrap-up Recall, corroborated by `prerequisites:` and `reviews_of`.
- **26 of Spanish's 31 "forward prerequisites" were never real.** With no declared
  order the walk fell back to sorting alphabetically inside a chapter, which put
  `beber` before `comer` and then reported `beber` as depending on a later lesson.
  Declaring the true order removed them:

      Spanish              before   after
      no sequence              56       6
      forward prerequisites    31       5
      forward references      143      99

  Corpus-wide: 565 → **515** unsequenced, 271 → **245** forward prerequisites,
  331 → **300** forward reviews. Spanish's atom figures are unchanged by the
  ordering itself, as they must be — ordering moved no content; the corpus totals
  moved only because a verb tranche landed on main in parallel.
- **Chapter 7's six lessons are deliberately left unsequenced.** `curriculum.json`
  says comer → beber → qué → vivir → dónde; the prose `Next:` chain **and**
  `ES-C07-beber`'s own `reviews_of` say comer → vivir → beber → qué → dónde. Under
  the ledger's order, `beber` reviews a lesson that has not happened. Guessing
  would bake a false ramp into every later measurement, so they wait for a ruling.
  A test pins exactly which six remain, so this cannot be forgotten.
- Chapter 18 is the weakest recovery: none of its ten lessons carries a `Next:`
  line, so its order rests solely on `prerequisites`/`reviews_of`. Those happen to
  form one clean chain, but with no prose corroboration.
- **Known remainder: the numbering is cramped.** Chapters 19–33 were already
  sequenced at 640–845, so chapters 7–18 had to fit between 510 and 640 — 129
  integers for 56 lessons. Spacing is therefore **2**, not the intended 10, leaving
  almost no insertion room in a track meant to grow from 146 lessons to thousands.
  Renumbering the whole track by 10s is mechanical and should follow.

### Added — does the course have a memory of itself? (HL09 step 1)

- Add `src/continuity.ts`: `measureContinuity` measures the three things a
  per-lesson budget cannot see, published in the gap report's new `continuity`
  section. The ramp budgets measure how big each *step* is; this measures whether
  the steps hold together.

      order: 565 lessons with no declared sequence across 19 tracks;
             271 prerequisites and 331 reviews pointing forward
      reinforcement: 746 of 1469 atoms never revisited (51%);
             missed windows R1 745, R2 1068, R3 649, R4 132
      forward references: 509 uses of material a later lesson teaches

- **You cannot review a lesson that has not happened yet**, and **331** do. A
  forward `reviews_of` cannot close a reinforcement window — it names lessons, not
  atoms — but it is still an authored claim about order, and a claim pointing
  forward is wrong on its own terms. `ES-C07-beber` reviews `ES-C07-vivir`, which
  `curriculum.json` places *after* it.
- **Order comes first because everything else depends on it.** 565 lessons carry
  no `sequence`, so their reading order exists only inside hand-typed LaTeX —
  Spanish 56 of 146, French **64 of 73**. A ramp whose order is unknown cannot be
  verified at all, so every other number here is provisional until this is zero.
- **51% of taught atoms are never practised again.** HL00 specified the schedule
  (N+1, N+3, N+7, N+15), defined a `review` lesson type to carry it, and named
  `session-map.md` as the artifact that verifies it. The corpus has **zero**
  `review` lessons and a session map covering 3 chapters of 33. The schedule was
  specified and never built.
- The measurement reads `practises.knowledge`, **never `reviews_of`** — which 144
  of Spanish's 146 lessons set, and which cannot close a window because it names
  *lesson ids* while atoms live in another namespace. Measuring that field would
  report a corpus that reinforces beautifully and teaches nothing twice.
- Windows are judged **only where the track is long enough to contain them**. A
  25-lesson track missing R4 has not failed; it has not got there yet.
- **Forward references are proved, not guessed.** A word is reported only when a
  *later lesson's own headword* teaches it, so the finding carries its own
  evidence and cannot false-positive on ordinary English prose. It reproduces
  what a human reviewer found by reading: `ES-C07-beber` rewards the learner with
  *"Como pan y bebo agua"* while **`pan` and `agua` are chapter 26**, and
  `ES-C08-practice` drills `diecinueve` in a chapter that taught 1–10.
- Three false-positive classes were found by censusing the output rather than
  guessing, and each is excluded on principle: **single-character headwords** from
  `writing` lessons (a Cyrillic `е` or a Devanagari mātrā matched in every lesson
  of its script — five scripts' worth), **pattern notation** like `e→ie`, and
  English collisions like `once` (18 hits) — only lessons whose type is `word` or
  `phrase` create a matcher at all.
- Honest limit, stated because it changes how the number reads: a word the course
  **never** teaches anywhere is invisible here. Chapter 7's `¿Algo más?` and the
  untaught `un`/`una` do not appear, because nothing in the data says they are
  target language. 509 is a floor.
- Report-only, per the HL05 precedent: the debt predates the measurement.
- `readingOrder`, `frontmatterList` and `introducedAtoms` are now exported from
  `ramp.ts` and shared. Two independent orderings that drifted apart would make
  the two reports disagree about which lesson comes first, silently.

### Added — the ramp now includes the script (HL-C18C)

- Add `measureScriptRamp` to `src/ramp.ts`, and two budgets to `core/chapter-policy.json`:
  `maxNewGlyphsPerLesson` (**3**) and `maxNewScriptSystemsPerLesson` (**1**).
- **The atom budget was measuring one of two burdens.** `maxNewAtomsPerLesson` counts
  units of *meaning*. `HI-W01-shirorekha-na-ma` declares **one** atom and puts **twelve**
  new Devanagari glyphs on the page, and passed cleanly for a whole release. It is not an
  outlier: **61 lessons** exceed three new glyphs and **38 of them declare zero atoms**, so
  they read as maximally gentle while teaching up to a dozen new shapes. Decoding is a
  separate skill on a separate curve, and nothing was watching it.
- **3 is the corpus's own p90**, the same rule that justified `maxNewAtomsPerLesson` — not
  the observed max of 12, because a budget placed at the worst case is not a budget. The
  median non-Latin lesson introduces **zero** new glyphs, so this flags genuine spikes
  rather than taxing ordinary lessons.
- **Target script and the cousin layer are counted separately, and only the first is
  charged.** A Kannada Chapter 1 lesson showing the same word in Devanagari, Tamil, Telugu
  and Malayalam looks like a **34-glyph cliff** when the two are conflated; its actual
  Kannada load is **7**. Sister-script material is context for a reader who already knows a
  relative, and English is the only requirement for each book — so it is reported (119
  lessons, up to 26 foreign glyphs in one) and never penalised. What that footprint
  justifies is keeping the layer visually skippable.
- Counting rules, each load-bearing: charged **once**, in reading order, so revision is
  free; **Latin excluded**, or romanization would swamp the signal; **combining marks
  included**, because an abugida is mostly marks; **script digits included**, because ०१२
  is not readable to someone born to ASCII; and **`Script_Extensions`, not `Script`**,
  because ー is formally `Common` and the narrow property undercounts コーヒー by the
  mark that makes it a long vowel.
- `maxNewScriptSystemsPerLesson: 1` states the rule that you cannot introduce more than
  one script at a time. It flags **5** lessons, all Japanese Chapter 1, which opens kanji
  beside hiragana in its first lesson and adds katakana in its fifth.
- Report-only, per the HL05 precedent: the debt predates the measurement.

### Fixed — `measureRamp` was called by nothing but its own test

- The gap report now carries a `ramp` section, so `maxNewAtomsPerLesson` and
  `maxNewAtomsPerChapter` are finally read by something a human sees. They had been
  declared in `core/chapter-policy.json` since HL08 and enforced by nobody — policy in the
  sense that a sign is policy. The first published figures: **40** lessons over the atom
  budget, **25** chapters, with **572 lessons (47%) unmeasurable** because schema-v1
  declares no atoms.

### Fixed — three tracks silently resolved to the Latin script

- `LANGUAGE_SCRIPT` had no entry for **Gujarati** — which was the worked example in its
  own doc comment — so all 39 Gujarati lessons resolved to `latin`. Glyph-coverage
  validation looked Gujarati headwords up in the *Latin* inventory, and `romanization`
  fell back to the Gujarati headword itself, so the narration export published
  `"romanization": "આભાર"` — **Gujarati script in the field a speech engine reads as
  Latin.** Regenerating `lesson-modality.json` and the seven Gujarati narration chapters
  is the whole blast radius; no lesson content changed.
- **Chinese** and **Japanese** were missing from the same map and were saved only by
  shipping a `track.json` the loader prefers. A fallback that is wrong for some tracks
  fails only in the paths that skip the loader — which is exactly where a unit test lives.
  Completing the map removes the trap.

### Added — every lesson declares the level it builds toward (HL-C10)

- Add `src/levels.ts`: `CEFR_LEVELS` (`pre-A1` … `C2`), `deriveLessonLevel`,
  `summarizeLevels`, and `lessonsUpToLevel` — the filter a "gentle ramp to A1" edition
  applies. Published through the gap report's new `levels` section, and
  `core/exam-levels.json` records how the language-specific exams line up.
- **Derived, never authored.** A lesson sits in a realization-path segment, the segment
  names a spine node, the node declares a CEFR stage. HL08 refused to write `modality:`
  into 1,134 frontmatter files because that is 1,134 places for a computed fact to go
  stale; a level is the same kind of fact. Deriving it also means a track cannot claim A1
  by editing frontmatter — it has to actually realize the A1 spine nodes.
- **The measured answer to "how far is each track from Advanced":**

      pre-A1 657 | A1 307 | A2 0 | B1 0 | B2 0 | C1 0 | C2 0
      964 of 1,134 lessons placed (85%); 170 unmapped, all schema-v1

  **No track has reached A2**, and five (`chinese`, `japanese`, `persian`, `russian`,
  `urdu`) have not reached A1. A ramp-to-A1 edition would today contain **964 lessons** —
  as a filter over the one corpus, not a second corpus.
- Unmapped lessons report `null` and are **excluded** from a ramp edition rather than
  included by default. A wrong level is worse than a missing one: it would put material a
  reader is not ready for inside a book that promises a gentle ramp, so the honest failure
  is a shorter book.
- `core/spine.json` `stages` extends to `B2`, `C1`, `C2` so later tranches can declare
  their own stage. The project owner's direction is that the content reaches the most
  advanced level, gently, with page count explicitly not a cost.
- `core/exam-levels.json` maps CEFR onto the exams a learner would actually sit, and
  **every one of the 22 tracks is mapped — no gaps.** An unmapped track silently drops out
  of every level report, and a learner asking "what is A1 in Tamil?" deserves an answer.
- **What is recorded instead of a gap is the KIND of answer.** `basis: published` means the
  awarding body states the alignment (DELE, DELF/DALF, Goethe, CILS, CAPLE, TORFL, HSK);
  `research` means a widely-cited third-party correspondence (JLPT, Arabic ILR/ACTFL);
  `editorial` means this project's judgement — a working default to be corrected, never a
  claim about what a certificate is worth. A test enforces that every registered track has
  a mapping and a valid basis, so registering a track now requires answering the question.
- Judgement calls worth knowing: **Hindi** is anchored to the Dakshina Bharat Hindi Prachar
  Sabha ladder (Prathmic → Praveen), which is real and widely sat but built to spread Hindi
  within India rather than against CEFR descriptors. **Tamil** is mapped straight to CEFR
  because its diglossia makes any mapping unclean — this curriculum teaches the spoken
  register first, so A1 means the CEFR descriptor, not a claim about a Tamil examination.
  **Latin** takes CEFR too, with the honest note that CEFR is communicative and Latin is
  read; a reading-only ladder would fit it better. A second test requires a caveat on any
  mapping that names a specific foreign ladder without the awarding body's backing — it
  caught a bare Persian/AMFA correspondence during this change.



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
