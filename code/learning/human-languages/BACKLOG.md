# Human Languages Backlog

This is the ordered delivery backlog for the shared-spine curriculum, books,
and Language Ladder. Reprioritize it after every merged work item. Add newly
discovered work here before starting it so the repository, rather than an agent
session, remains the source of truth.

Last prioritized: 2026-08-06, when the Step-by-Step capability program (HL05–HL07)
was specified and placed ahead of the remaining roadmap and migration work. Current
baseline after the Japanese Chapter 1 tranche (HL-C40), which followed the Mandarin
Chinese scale test (HL-C39), the Latin chapter payoffs (HL-C24) and the Spanish
gentle-ramp split (HL-C18A): **22** registered tracks, **1,133** Markdown lessons,
**22** downloadable LaTeX books, zero duration violations, and 22 validated
realization maps containing **354** ordered path segments, **298** typed extension
nodes, and **964** prerequisite-closed mapped lessons. The preceding Persian and Urdu
Chapter 5 baseline measured 20 tracks and 1,096 lessons; the four tranches since add
37 lessons between them (+4 Latin payoffs, +18 Spanish micro-lessons, +7 Mandarin,
+8 Japanese). Twenty-five mapped non-lexical lessons across 18 tracks now carry
compiled objective activities; 94 mapped non-lexical lessons remain explicit
activity-coverage debt, including 16 legacy lessons that first need schema-v2
body contracts. HL-V01 keeps the remaining migration debt reproducible in both
JSON and human-readable reports; the canonical schema-v2 tranches prove one
typed source across Language Ladder and generated book chapters without
discarding deep content.

## Priority rules

1. Close a learner-visible broken promise before adding breadth.
2. Prefer work that makes later corpus growth measurable or generated.
3. Finish a small vertical slice before starting the same migration everywhere.
4. Keep the application, book, and canonical lesson content aligned.

## P0 — Step-by-Step capability program (HL05–HL08)

Specified in [HL05](../../specs/HL05-chapter-capability-and-step-by-step-shape.md),
[HL06](../../specs/HL06-visual-system.md),
[HL07](../../specs/HL07-spine-expansion-to-b1.md), and
[HL08](../../specs/HL08-modality-gentle-ramp-and-the-drivable-course.md). This program
adds a chapter-level capability layer above the existing lessons, gives the books a
visual system including inline script-writing instruction, grows the spine far enough
to carry a complete book, and makes the corpus teachable aloud by a voice assistant
while the learner drives. It rewrites no authored lesson content.

The measured starting point: 379 chapters, **zero** of which declare a goal or a
payoff; 11 spine nodes with **zero** at A2 or B1; **zero** images in any of the 20
books; and a complete, font-validated stroke-path model in `strokes.ts` that holds one
letter and is rendered nowhere.

On modality and ramp, measured across all 1,096 lessons: 51 need a pen and 7 carry a
script block, but of the remaining 1,038, some 322 contain a Markdown table and 56 a
sight cue — so **695 lessons, about 63% of the corpus, are drivable exactly as
authored**, and the table, not the script, is the main obstacle to the rest. The ramp
is already gentle in aggregate (mean 2.31 new atoms per lesson, median 2, p90 3) but
undefended: 52 lessons exceed a budget of 3 and the steepest teaches ten numbers at
once. Length is explicitly not a cost — splitting for gentleness is the intended
direction, and no gate may penalise page, lesson, or chapter count.

| ID | Status | Work item | Completion signal |
|---|---|---|---|
| HL-C01 | Complete in this PR | Specify the chapter capability layer, the visual system, and spine expansion to B1. | HL05, HL06 and HL07 are committed before any implementation, per repo policy. |
| HL-C02 | Queued | Add the `chapters.json` schema, loader, and `core/chapter-policy.json`. | Types, loader beside `loadLanguageCurricula`, and the representativeness threshold load and round-trip; no gates yet. |
| HL-C03 | Queued | Land the nine HL05 gates as report-only output and publish the first chapter snapshot. | The gap report measures all 379 chapters for missing capability, payoff closure, and representativeness without failing CI on recorded debt. |
| HL-C04 | Queued | Derive book chapter titles and labels from `chapters.json`. | `core/book-generation.json` stops owning `title`/`label`; `chapter-title-drift` proves the two agree through the transition. |
| HL-C05 | Queued | Add the `pattern` lesson type with slot-closure and production gates. | A `pattern` lesson introduces one `*-PATTERN-*` atom, declares only in-closure slot fillers, and instantiates at least three in a `guided-production` block. |
| HL-C06 | Queued | Add the figure pipeline: SVG generation, `graphicx`, SVG→PDF in CI, and a `--check` hash gate. | A generated figure round-trips from canonical data into a compiled PDF and fails CI on drift, reusing `paint-vm-svg`'s `renderToSvgString`. |
| HL-C07 | Complete in this PR | Add the log-scanning warning gate with recorded per-track baselines. | Overfull/underfull boxes, missing glyphs, hyperref warnings, duplicate destinations, and font substitutions are machine-checked by `scan_latex_log_warnings.py` after the `latexmk` loop, against `core/latex-warning-baseline.json`. Baselines ship unseeded — `null` means unmeasured, never zero — so the gate reports today and fails the moment a seeded track regresses. The first CI run on main emits the real counts into the job summary for a human to paste back. |
| HL-C08 | Queued | Render the ductus in Language Ladder. | `penPathD`/`penTip` drive an SVG stroke build-up in the app; book and app teach handwriting from one source. |
| HL-C09 | Queued | Expand `DUCTUS` to cover the nine scripts with cited prose stroke order. | ~190 letters authored, each passing the on-ink, join-tolerance, and coverage invariants with citation and URL. |
| HL-C10 | Queued | Complete A1 and add the A2 and B1 spine tranches with all 20 realization ledgers. | Every declared stage carries nodes; every track has a non-drifting ledger entry for every node. |
| HL-C11 | Queued | Author chapter capabilities and payoffs across all 20 tracks. | All 379 chapters declare a `canDo` and a closed, representative payoff; gates flip to errors per track as debt clears. |
| HL-C12 | Queued — licensing decided, pipeline outstanding | Add the Class C illustration pipeline with provenance sidecars and a size budget. Licensing is settled and recorded in [`_assets/LICENSE.md`](./_assets/LICENSE.md); the remaining work is the pipeline itself. | Every asset carries `license`, `rightsAsserted`, `generator`, `model`, `prompt`, `date`, and `sha256`; CI fails any asset without a provenance sidecar or a recorded licence, and enforces the per-track size budget. |
| HL-C13 | Queued | Deploy Language Ladder to GitHub Pages. | The app is reachable at `/coding-adventures/language-ladder/`; it is currently referenced by no workflow and has never been published. |
| HL-C14 | Complete | Derive modality (`voice`/`sight`/`pen`) for every lesson and each chapter's drivable prefix. | The gap report publishes per-track modality counts and the corpus-wide drivable percentage; overrides without a recorded reason are reported. |
| HL-C15 | Queued | Print modality signs and the drivable prefix at every chapter opening. | The book shows 🚗/👁/✍ beside the HL05 capability, so a reader knows before starting whether a chapter needs eyes or a pen. |
| HL-C16 | Complete in this PR | Build the narration export (`narration-cli`) with `--write`/`--check`. | Done. `src/speech.ts` + `src/narration.ts` + `src/narration-cli.ts` emit `<track>/narration/chNN.txt` and `.json` for all 375 chapters, hash-gated by `core/generated-narration-hashes.json` and checked byte-for-byte in CI. `[PAUSE Ns]`, `[REPEAT xN]` and `[YOU …: …]` survive as typed directives; a spoken answer is scored only against a compiled `hl-activity` contract, never against a cue. Tables linearise up to **3 columns** (371 of 442 tables, 272 of 340 table-bearing files); a refused table is *spoken* — size, headings, and reason — and marks its lesson `sight`. `maxLinearisableTableColumns` moved 0 → 3 in `core/chapter-policy.json`, taking the corpus from **63% to 84% drivable** (694 → 925 lessons). This is the audio-script output HL04 named and nothing had ever built. |
| HL-C17 | Queued — 80% done by HL-C16, 68 files remain | Linearise or reclassify the 322 table-bearing lessons. | HL-C16 discharged the invariant: every table now either reads aloud or is spoken as a refusal that marks its lesson `sight`, so the export already never silently drops content. What remains is *remediation*, not correctness — **71 tables across 68 lesson files are four columns or wider**, and 52 lessons need eyes for a wide table and nothing else. Reshaping those 52 tables into two- or three-column form would move 52 more lessons into the car (84% → 89%). Widening `maxLinearisableTableColumns` to 4 would reach the same number without earning it, and is the wrong fix: at four columns the meaning lives in the comparison across rows. |
| HL-C18 | Queued | Burn down the 52 lessons that exceed the gentle-ramp budget. | No lesson introduces more than `maxNewAtomsPerLesson`; over-budget lessons are split into prerequisite-ordered micro-lessons, longest first, starting with `ES-C31-numeros-11-20` at seven. |

| HL-C16 | Queued | Build the narration export (`narration-cli`) with `--write`/`--check`. | Plain-text and structured-JSON scripts emit from the canonical AST with `[PAUSE]`/`[REPEAT]`/`[YOU SAY]` preserved as directives and hash-gated against the lesson AST. This implements the audio-script output HL04 named and nothing ever built. |
| HL-C17 | Queued — 308 remaining after HL-C32 | Linearise or reclassify the 308 table-bearing lessons. | Every table either reads correctly aloud or its lesson is honestly marked `sight`; the export never silently drops content. |
| HL-C18 | Queued — Spanish slice complete | Burn down the 52 lessons that exceed the gentle-ramp budget. | No lesson introduces more than `maxNewAtomsPerLesson`; over-budget lessons are split into prerequisite-ordered micro-lessons, longest first, starting with `ES-C31-numeros-11-20` at seven. |
| HL-C18A | Complete | Split the fifteen over-budget Spanish lessons, including the corpus-worst `ES-C31-numeros-11-20` at seven. | Spanish measures zero over-budget lessons; the fifteen become thirty-three prerequisite-ordered micro-lessons and the corpus figure drops from 52 to 37. |
| HL-C18B | Queued | Split the remaining 37 over-budget lessons across the other sixteen tracks. | Every track measures zero lessons above `maxNewAtomsPerLesson`; the corpus maximum drops from 6 to 3. |
| HL-C19 | Queued | Verify every prose `strokeOrder` against an authored ductus, so no letter's step list implies a pen lift nothing has checked. | All 190 prose stroke orders across the nine scripts (`arabic` 21, `chinese` 24, `cyrillic` 33, `devanagari` 28, `gujarati` 29, `hebrew` 22, `perso-arabic` 9, `tamil` 10, `urdu-nastaliq` 13) either carry a font-checked pen path with `penLifts` + `strokeOrderSource`, or are worded so they claim part order only. Today exactly one letter — Tamil ம — is verified; the audit that found it is written up in [`data/scripts/README.md`](data/scripts/README.md). Follows HL-C09, which authors the paths this check consumes. |
| HL-C30 | Closed — no move is both legal and useful | Recover Arabic's drivable prefix by moving the writing lessons that open Chapters 3 and 4 later in their chapters. | Measured and answered: zero. Both chapters are prefix-0 under **every** legal ordering because neither has a `voice` lesson without an in-chapter prerequisite, and all 18 of Arabic's `sight` lessons are tables, not script. Corpus-wide only 2 chapters (`portuguese ch2`, `italian ch2`, +4 lessons) can be improved by reordering at all; 116 of the 123 zero-prefix chapters are table-blocked at the root and belong to HL-C17. See *Findings from HL-C30*. |
| HL-C24 | Complete in this PR | Pilot real chapter payoff lessons on the weakest Latin chapters. | Latin chapters 19, 21, 33, and 36 each own a dedicated terminal consolidation lesson built only from already-taught material, and `chapters.json` points their `payoff.lesson` at it. |
| HL-C25 | Queued | Scale the HL-C24 payoff pattern across the remaining 32 Latin chapters and the other 19 tracks. | Every chapter's payoff is a lesson written to be a payoff, not the chapter's last teaching lesson pressed into service. |
| HL-C26 | Complete in this PR | Give the hand-written early chapters a checkable title and label without making them generated. | The 105 chapters with a committed `.tex` but no `targets[]` entry are recorded in a new `handwritten[]` list in `core/book-generation.json`, transcribed from what each `\chapter{}`/`\label{}` actually declares. `generatedBookOutputs` never walks that list, so `check:books` still passes byte-for-byte and no authored chapter can be overwritten; `chapter-title-drift` no longer skips them. |
| HL-C44 | Complete in this PR | Emit the derived modality as a generated, drift-gated manifest so different outputs can be filtered from one source. HL-C14 derived `voice`/`sight`/`pen` per lesson and a drivable prefix per chapter, but only at runtime and only into the human-readable gap report — no book builder, app, or driving-edition renderer had a file to filter on. | `core/lesson-modality.json` carries per-lesson `id`/`language`/`chapter`/`sequence`/`modality`/`derived`/`drivable`/`reasons`/`sourceHash`, per-chapter drivable prefix and ordered `drivableLessonIds`, per-track rollups, and a corpus summary (1,096 lessons; 708 `voice`, 337 `sight`, 51 `pen`; 65% drivable; 375 chapters; 551 lessons reachable in prefix order; 199 fully drivable chapters; 121 unstartable by ear). `modality-cli --write`/`--check` mirrors the `book-cli` contract, `check:modality` runs in CI beside `check:books`, and the schema reserves room for HL-C41's `coreModality` as a purely additive key. |
| HL-C32 | Complete in this PR | Diagnose and repair the Russian track, worst in the corpus on two independent measurements: 9% drivable with **zero** lessons reachable by ear in either chapter, and payoff representativeness of 0.20. | Russian measures 73% drivable (16 `voice`, 1 `sight`, 5 `pen`) with 15 lessons reachable in chapter-prefix order, and Chapter 2's payoff representativeness is 0.67 against the 0.5 floor. Zero new validation errors, zero duration violations. |
| HL-C39 | Complete in this PR | Add Mandarin Chinese as the 21st track — a **scale test** for whether the curriculum model generalises beyond Indo-European and Dravidian. Chapter 1 only, authored deep rather than swept wide. | A complete, CI-green track: 7 schema-v2 Chapter 1 lessons, an 11-node `curriculum.json` ledger, an HL05 capability with a payoff assessing 10 of 11 chapter atoms, and a generated book chapter. Three findings reported rather than hidden: **(1)** the cousin web does **not** transfer — Chinese shares no ancestor with English, so character composition replaces etymology and is honestly a weaker hook, since it anchors to knowledge being acquired in the same breath rather than knowledge the reader already had; **(2)** HL00's word→letter script rule becomes word→character→component, three levels not two, and for Chinese the "letters in this word" and "the word, taken apart" sections collapse into one analysis; **(3)** tone needed a data-layer extension (`ScriptData.tones` and `ScriptData.toneSandhi`) because `Letter.tone` can label a glyph but cannot express an inventory or a rule that changes pitch across a *sequence*. Also added the `pronunciation` lesson type, because no earlier track ever needed a lesson about sound. |
| HL-C40 | Complete in this PR | Add **Japanese** as a track, Chapter 1 only, as the corpus's hardest scale test: three writing systems at once, kanji with multiple readings, grammatical politeness, and no shared ancestry with English. | 21st registered track; 8 schema-v2 lessons; a ledger entry for all 11 spine nodes; `data/scripts/japanese.json` covering hiragana, katakana, and kanji in one inventory; `_fonts/NotoSansJP-Subset.ttf` with `subset-jp.sh`; generated Chapter 1 compiling under XeLaTeX with zero overfull boxes and zero missing glyphs. Findings recorded below. |
| HL-C41 | Complete in this PR (modality half); Telugu half blocked | Derive modality per BLOCK as well as per lesson, so a voice lesson can carry one detachable `writing` segment — the interspersed-writing pattern. Teach Telugu handwriting. | `deriveBlockModality` and `coreModality` ship with the `writing` block type; the drivable prefix counts the core; HL08 records the amendment and the project owner's ruling that the book keeps all writing content. The Telugu ductus is **not** shipped: no citable stroke-order source for a single letter could be reached, so zero letters were authored rather than any uncited ones. See Findings from HL-C41. |
| HL-C42 | Queued | Let a track declare more than one script. HL01 gives a track exactly one `script` id and validates headword glyphs against exactly one inventory, so Japanese's hiragana, katakana, and kanji share one file with a per-sign `role` doing the separating. | A track declares `scripts: [...]`, `uncoveredGlyphs` resolves a character against any declared inventory, and a lesson can say which system a word is written in without a naming convention inside `role`. |
| HL-C45 | Queued | Structure the `register` field, which Japanese has outgrown. Today it is an open string, adequate for a *tú*/*usted* word choice and inadequate for a system where politeness is verb morphology on every predicate and keigo swaps the verb outright (言う → おっしゃる / 申す). | `register` becomes a small record — speech level, addressee honorification, referent honorification — that the 20 existing tracks map onto losslessly and that Japanese Chapter 3's honorific prefix **お-** can express without a free-text convention. |
| HL-C46 | Queued | Author the first interspersed `writing` segment, in a track whose ductus is already sourced. | One ordinary Tamil lesson carries a `## Writing: ம` segment citing the UT Austin primer; the book prints it, `coreModality` stays `voice`, and the corpus regression pin moves deliberately. |
| HL-C47 | Blocked on provenance | Author Telugu / Kannada / Malayalam base-consonant ductus. | One openable published primer with numbered stroke arrows per script; then ~36 base consonants and the vowel signs per script pass the three `strokes.ts` font invariants with citation and URL. Blocked, not queued: no source was reachable in HL-C41. |
| HL-C27 | Complete in this PR | Run the book catalog builder's tests in CI. `test_build_human_language_book_catalog.py` existed but was executed by no workflow, so the script that writes the published `index.html` and `catalog.json` shipped with its tests never running. | `human-languages-books.yml` runs the suite in its own named step before the expensive XeLaTeX build, and both the workflow's `paths:` trigger and the `detect` job's `git diff` list include the test file so a change to it re-runs the job. |

The project owner decided on 2026-08-06 that the curriculum ships **two editions from
one canonical source**: the complete book, which keeps everything including the writing
instruction, and a later dictation-friendly **driving edition**, which omits what a
driver cannot do. HL-C44 is the machinery that makes the filter possible and nothing
more — `core/lesson-modality.json` is the file both editions read. HL-C43 builds the
driving edition itself. The manifest's `modality` field is the conservative
whole-lesson answer, so the edition filter is correct today; HL-C41 adds block-level
modality as a purely additive `coreModality` key, at which point the driving edition
can skip a lesson's short optional writing segment instead of dropping the whole
lesson. Today, any pen content costs a commuter the entire lesson — 121 of the 375
chapters cannot be started by ear at all.

The illustration licensing question HL06 raised is **settled**. The project owner
decided on 2026-08-06 that the books stay CC BY-SA 4.0 and that generated
illustrations are marked `CC0-1.0` with `rightsAsserted: false`, each carrying a
provenance sidecar — because a CC licence grants copyright permissions that purely
AI-generated output likely cannot support, and CC0 is safe whichever way the law
settles. The decision, its reasoning, the required sidecar fields, and the two
operational constraints on prompting and generator terms are recorded in
[`_assets/LICENSE.md`](./_assets/LICENSE.md), following the `_fonts/OFL.txt`
precedent. HL-C12 is therefore unblocked: CI still gates every Class C asset on having
a sidecar and a recorded licence, but the licence to record is now known.

HL05 also reserves, and deliberately does not implement, a `presents.knowledge` tier
that would let a payoff use a glossed-but-never-assessed word. Strict closure is kept,
so early chapters ramp slightly more slowly than a trade step-by-step grammar does.
Reserving the key now makes enabling it later a flag flip rather than a corpus
migration.

## P0 — current publication and validation gaps

| ID | Status | Work item | Completion signal |
|---|---|---|---|
| HL-B01 | Complete (#9472) | Publish the five authored Persian lessons as a two-chapter LaTeX starter book. | XeLaTeX builds the book; CI discovers an 18th PDF; chapters map to lessons 1 and 2. |
| HL-B02 | Complete (#9474) | Publish the five authored Urdu lessons as a two-chapter starter book. | XeLaTeX builds with correct RTL shaping; Urdu appears in the public catalog. |
| HL-B03 | Complete (#9478) | Publish Russian's two authored chapters as a starter book. | The existing Cyrillic lessons and roadmap produce a downloadable PDF. |
| HL-V01 | Complete (#9483) | Add a machine-readable curriculum gap report and computed duration budget. | CI reports lessons at or above 300 seconds, missing prerequisites, book coverage, and track-schema status. |

The Russian publication audit found eight of its thirteen Chapter 1--2
curriculum lessons currently declare five minutes or more, including one at six
minutes. This is concrete input to HL-V01 and HL-D01, not silently treated as
fixed by the book: the starter edition presents shorter dependency-ordered
micro-sections while the canonical duration split remains measurable debt.

The first deterministic HL-V01 snapshot measures 485 lessons at or above 300
effective seconds, zero unknown prerequisite ids, 42 later-chapter lessons with
no declared prerequisite, 257 lesson chapters without a matching book chapter,
and all 20 tracks still on the legacy lesson schema. The report is evidence for
the next migrations; it deliberately does not fail CI on already-recorded debt.

## P1 — one-source migration

| ID | Status | Work item | Why it follows P0 |
|---|---|---|---|
| HL-S01 | Complete (#9497) | Migrate Spanish Chapters 1–3 to schema version 2 with typed body blocks and knowledge closure. | The 24-lesson slice has unique order, transitive knowledge closure, typed blocks, and no effective-duration violation. |
| HL-G01 | Complete (#9505) | Generate a Spanish LaTeX chapter from the canonical lesson AST and compare source hashes with the app. | Removes the first handwritten book copy now that the AST contract is executable. |
| HL-G02 | Complete (#9509) | Generate Spanish Chapters 2–3 from their canonical schema-v2 lesson AST. | Extends the proven one-source path across the rest of the migrated pilot before broad corpus work. |
| HL-D01A | Complete in the Russian duration PR | Remove all nine sub-five-minute violations from the complete Russian starter track. | The report measures zero Russian violations; every changed or added lesson is below 300 effective seconds. |
| HL-D01B | Complete in the Marathi duration PR | Remove all eight sub-five-minute violations from the Marathi track. | The report measures zero Marathi violations; the one genuinely long lesson is now two prerequisite-ordered micro-lessons. |
| HL-D01C | Complete in the Gujarati duration PR | Remove all nine sub-five-minute violations from the Gujarati track. | The report measures zero Gujarati violations; the one genuinely long lesson is now two prerequisite-ordered micro-lessons. |
| HL-D01D | Complete in the Punjabi duration PR | Remove all ten sub-five-minute violations from the Punjabi track. | The report measures zero Punjabi violations; the one genuinely long lesson is now two prerequisite-ordered micro-lessons. |
| HL-D01E | Complete in the Sanskrit duration PR | Remove all ten sub-five-minute violations from the Sanskrit track. | The report measures zero Sanskrit violations; the 513-second anchor lesson is now three prerequisite-ordered micro-lessons. |
| HL-D01F | Complete in the Bengali duration PR | Remove all eleven sub-five-minute violations from the Bengali track. | The report measures zero Bengali violations; all eleven lesson bodies remain unchanged because their computed durations were already below 300 seconds. |
| HL-D01G | Complete in the Italian duration PR | Remove all twenty sub-five-minute violations from the Italian track. | The report measures zero Italian violations; four new prerequisite-ordered micro-lessons preserve the register, metaphor, suppletion, and agreement content that did not fit safely in the original lessons. |
| HL-D01H | Complete in the Portuguese duration PR | Remove all twenty-three sub-five-minute violations from the Portuguese track. | The report measures zero Portuguese violations; five new prerequisite-ordered micro-lessons preserve the register, suppletion, grammar-choice, and etymology content from the five computed violations. |
| HL-D01I | Complete in the French duration PR | Remove all twenty-five sub-five-minute violations from the French track. | The report measures zero French violations; three new prerequisite-ordered micro-lessons preserve register, suppletion, and pronominal-agreement depth. |
| HL-D01J | Complete in the German duration PR | Remove all twenty-seven sub-five-minute violations from the German track. | The report measures zero German violations; five new prerequisite-ordered micro-lessons preserve register, practice, areal-history, agreement, and etymology depth. |
| HL-D01K | Complete in the Telugu duration PR | Remove all thirty-six sub-five-minute violations from the Telugu track. | The report measures zero Telugu violations; one new prerequisite-ordered micro-lesson separates phrase formation from register and source-evidence judgment. |
| HL-D01L | Complete in the Kannada duration PR | Remove all thirty-seven sub-five-minute violations from the Kannada track. | The report measures zero Kannada violations; one new prerequisite-ordered micro-lesson separates suffix forms and sound history from the agglutinative-versus-fusional comparison. |
| HL-D01M | Complete in the Malayalam duration PR | Remove all thirty-seven sub-five-minute violations from the Malayalam track. | The report measures zero Malayalam violations; four new prerequisite-ordered micro-lessons separate vocabulary from etymology, register, and cross-language comparison. |
| HL-D01N | Complete in the Arabic duration PR | Remove all thirty-nine sub-five-minute violations from the Arabic track. | The report measures zero Arabic violations; four new prerequisite-ordered writing steps preserve the abjad, joining, whole-word assembly, and hamza content. |
| HL-D01O | Complete in the Hindi duration PR | Remove all forty sub-five-minute violations from the Hindi track. | The report measures zero Hindi violations; thirteen new prerequisite-ordered lessons preserve its script, etymology, grammar, and register depth. |
| HL-D01P | Complete (#9604) | Remove all forty-two sub-five-minute violations from the Tamil track. | The report measures zero Tamil violations; twenty new prerequisite-ordered lessons preserve its script, etymology, grammar, register, and source-evidence depth. |
| HL-D01Q | Complete (#9610) | Remove all forty-three sub-five-minute violations from the Latin track. | The report measures zero Latin violations; six new prerequisite-ordered lessons preserve its grammar, etymology, usage, and attestation depth. |
| HL-D01R | Complete (#9624) | Remove all fifty-five remaining sub-five-minute violations from the Spanish track. | The report measures zero Spanish violations; twelve new prerequisite-ordered lessons preserve the grammar, etymology, usage, writing, and practice depth from the genuinely long lessons. |
| HL-D01 | Complete (#9624) | Split or rewrite every lesson whose computed duration is at least 300 seconds. | The deterministic report now reaches zero effective-duration violations across all twenty tracks. |
| HL-S02 | Complete (#9634) | Migrate Spanish Chapters 4–6 to schema v2 before generating their book chapters. | All 27 lessons have typed blocks, unique sequence, transitive knowledge closure, and sub-five-minute duration guarantees. |
| HL-G03 | Complete (#9646) | Generate Spanish Chapters 4–6 from their canonical schema-v2 lesson ASTs after HL-S02. | All six generated chapters now share lesson hashes with Language Ladder; Markdown tables retain their structure in print. |
| HL-G04 | Complete (#9915) | Normalize paired straight quotation marks when canonical prose is rendered into LaTeX. | Generated prose uses true opening and closing marks under every book's language rules without changing the canonical app text, code spans, escaped literals, or link destinations. |
| HL-G05 | Complete in this PR | Preserve canonical Markdown hyperlinks in generated LaTeX chapters. | Source notes and learner-facing links render as live `\href` targets in PDFs instead of retaining only their labels. |
| HL-G06 | Complete (#9915) | Preserve indented continuation lines inside generated Markdown blockquotes. | Multiline learner examples remain inside one LaTeX quote/callout, so typography and layout do not split halfway through a canonical example. |
| HL-V02 | Complete (#9653) | Validate learner-facing target-language prompts against block-level knowledge declarations and prerequisite closure. | Schema-v2 production and recall blocks cannot ask for an undeclared form or a form absent from the lesson's transitive knowledge frontier. |
| HL-V03 | Complete (#9900) | Compile individual prompt, answer, accepted-variant, feedback, and response-time contracts from typed activity blocks. | Compact JSON directives compile into validated runtime answer sets; each activity names a non-empty assessed-atom subset, carries feedback/time, and never scrapes prose. |
| HL-A01 | In progress (#9901 + Russian and Persian/Urdu slices) | Author objective activity coverage for every mapped non-lexical frontier. | The first tranche covers every ready schema-v2 track; later slices cover Russian's naming chain and Persian/Urdu Chapters 3–5 practice. Coverage is 25 of 119 across 18 tracks; 94 lessons remain, including 16 that first need schema-v2 migration. |
| HL-Q01 | Queued | Restore a clean standalone TypeScript typecheck for Language Ladder. | `npx tsc --noEmit` should pass after fixing the pre-existing DOM element type, review-log cast, missing Node test types, and unused/implicit test symbols; HL-V03 introduces no additional type errors. |
| HL-B04 | Complete (#9661) | Publish Marathi Chapter 6 from its two canonical lessons rather than hand-copying another book chapter. | Both schema-v2 lessons now generate the PDF chapter from the same source hashes independently verified by Language Ladder. |
| HL-B05 | Complete (#9663) | Remove Marathi's duplicate practice labels and Unicode bookmark warnings. | Stable recap labels, bookmark-safe Devanagari, natural page bottoms, and explicit static-font shapes make the forced six-chapter build warning-free. |
| HL-B06 | Complete (#9669) | Publish Gujarati Chapter 6 from its two canonical lessons rather than hand-copying another book chapter. | Both schema-v2 lessons now generate the PDF chapter from the same source hashes independently verified by Language Ladder. |
| HL-B07 | Complete (#9675) | Remove Gujarati's missing punctuation glyphs and LaTeX layout/bookmark warnings. | Canonical recap labels, main-font punctuation, bookmark-safe Gujarati, natural page bottoms, and explicit static-font shapes make the forced six-chapter build warning-free. |
| HL-B08 | Complete (#9680) | Publish Punjabi Chapter 6 from its two canonical lessons rather than hand-copying another book chapter. | Both schema-v2 lessons now generate the PDF chapter from the same source hashes independently verified by Language Ladder. |
| HL-B09 | Complete (#9683) | Remove Punjabi's LaTeX layout, duplicate-label, font-shape, and Unicode bookmark warnings. | Stable recap labels, bookmark-safe Gurmukhi, natural page bottoms, explicit static-font shapes, and a shorter running title make the forced six-chapter build warning-free. |
| HL-B10 | Complete (#9690) | Publish Sanskrit Chapter 6 from its three canonical lessons rather than hand-copying another book chapter. | All three schema-v2 lessons now generate the PDF chapter from the same source hashes independently verified by Language Ladder. |
| HL-B11 | Complete (#9698) | Remove Sanskrit's LaTeX layout, duplicate-label, font-shape, and Unicode bookmark warnings. | Stable recap labels, bookmark-safe Devanagari, natural page bottoms, explicit static-font shapes, and shorter running titles make the forced six-chapter build warning-free. |
| HL-B12 | Complete (#9705) | Publish Bengali Chapter 6 from its canonical lesson rather than hand-copying another book chapter. | The schema-v2 lesson now generates the PDF chapter from the same source hash independently verified by Language Ladder. |
| HL-B13 | Complete (#9711) | Remove Bengali's missing glyphs and LaTeX layout/bookmark warnings. | Main-font punctuation, stable recap labels, bookmark-safe Bengali, natural page bottoms, explicit static-font shapes, and a breakable long title make the forced six-chapter build warning-free. |
| HL-I01 | Complete (#9715) | Reduce unified all-books workflow setup time without splitting the single publication bundle. | A focused, preflighted XeLaTeX dependency closure replaces `texlive-full`; the unchanged job still builds all 20 books, verifies one bundle, and publishes that bundle from `main`. |
| HL-I02 | Queued | Update the human-language data package beyond the vulnerable PostCSS 8.5.19 transitive development dependency. | A clean install reports GHSA-fxqj-rqcc-2cmp through Vitest/Vite; PostCSS 8.5.25 is available, but this development-only moderate finding remains behind reader-facing book and app gaps. |
| HL-I03 | Queued | Derive the top-level track progress table from canonical curriculum and book-generation data. | The hand-maintained table can lag shipped chapters after a generated-book migration; a checked or generated summary should keep every prior language visible without repeating stale progress claims. |
| HL-I04 | Complete (#9908) | Restore exact-main full CI after `perl/wasm-module-encoder` exposed undeclared local test dependencies. | Its `cpanfile` and `Makefile.PL` declare every local package injected by `BUILD`; clean full-build metadata validation and focused Perl tests pass. |
| HL-I05 | Complete (#9910) | Make the repository's Lua bootstrap resilient to a temporary lua.org connection outage. | All CI platforms install pinned Lua 5.4.7 through an OS-specific cache or checksum-verified byte-identical Debian/Ubuntu source fallback without weakening the Windows MSVC ordering or silently skipping Lua tests. |
| HL-B14 | Complete (#9728) | Publish Italian Chapters 2–17 from their canonical lessons rather than hand-copying sixteen book chapters. | Forty-nine schema-v2 lessons now generate sixteen chapters whose source hashes are independently verified against the Language Ladder corpus. |
| HL-B15 | Complete (#9735) | Remove Italian's LaTeX layout and Unicode bookmark warnings. | The forced 104-page build now has zero missing glyphs, overfull or underfull boxes, duplicate destinations, Hyperref warnings, or LaTeX warnings. |
| HL-B16 | Complete (#9744) | Publish Portuguese Chapters 2–17 from their canonical lessons rather than hand-copying sixteen book chapters. | Fifty schema-v2 lessons now generate sixteen chapters whose source hashes are independently verified against the Language Ladder corpus. |
| HL-B17 | Complete (#9748) | Remove Portuguese's LaTeX layout warnings. | The forced 105-page build now has zero missing glyphs, overfull or underfull boxes, duplicate destinations, Hyperref warnings, or LaTeX warnings. |
| HL-B18 | Complete (#9752) | Publish French Chapters 17–23 from their canonical lessons rather than hand-copying seven book chapters. | Nine schema-v2 lessons now generate seven chapters whose source hashes are independently verified against the Language Ladder corpus. |
| HL-B19 | Complete (#9761) | Remove French's LaTeX layout and Unicode bookmark warnings. | The forced 98-page build now has zero missing glyphs, overfull or underfull boxes, duplicate destinations, Hyperref warnings, or LaTeX warnings. |
| HL-B20 | Complete (#9765) | Publish German Chapters 17–23 from their canonical lessons rather than hand-copying seven book chapters. | Ten schema-v2 lessons now generate seven chapters whose source hashes are independently verified against the Language Ladder corpus. |
| HL-B21 | Complete (#9779) | Remove German's LaTeX layout and Unicode bookmark warnings. | The forced 104-page build now has zero missing glyphs, overfull or underfull boxes, duplicate destinations, Hyperref warnings, or LaTeX warnings. |
| HL-B22 | Complete (#9803) | Publish Telugu Chapters 6–31 from their canonical lessons rather than hand-copying twenty-six book chapters. | Thirty schema-v2 lessons now generate twenty-six chapters whose source hashes are independently verified against the Language Ladder corpus. |
| HL-B23 | Complete (#9815) | Remove Telugu's LaTeX layout, duplicate-label, bookmark, and font warnings. | The forced 95-page build now has zero missing glyphs, overfull or underfull boxes, duplicate destinations, Hyperref warnings, LaTeX warnings, or font warnings. |
| HL-B24 | Complete (#9823) | Publish Kannada Chapters 6–31 from their canonical lessons rather than hand-copying twenty-six book chapters. | Thirty schema-v2 lessons now generate twenty-six chapters whose source hashes are independently verified against the Language Ladder corpus. |
| HL-B25 | Complete (#9828) | Remove Kannada's LaTeX layout, duplicate-label, bookmark, and font warnings. | The forced 96-page build now has zero missing glyphs, overfull or underfull boxes, duplicate destinations, Hyperref warnings, LaTeX warnings, or font warnings. |
| HL-B26 | Complete (#9838) | Publish Malayalam Chapters 6–31 from their canonical lessons rather than hand-copying twenty-six book chapters. | Thirty-three schema-v2 lessons now generate twenty-six chapters whose source hashes are independently verified against the Language Ladder corpus. |
| HL-B27 | Complete (#9844) | Remove Malayalam's LaTeX layout, duplicate-label, bookmark, font, and header-only-verso warnings. | The forced 107-page build now has zero missing glyphs, overfull or underfull boxes, duplicate destinations, Hyperref warnings, LaTeX warnings, or font warnings; intentionally empty versos are truly empty. |
| HL-B28 | Complete (#9854) | Publish Arabic Chapters 3–27 and their writing companions from canonical lessons rather than hand-copying another twenty-five book chapters. | Forty-five schema-v2 lessons now generate twenty-five chapters whose source hashes are independently verified against the Language Ladder corpus. |
| HL-B29 | Complete (#9861) | Remove Arabic's LaTeX layout, duplicate-label, bookmark, font, and header-only-verso warnings. | The forced 104-page build now has zero missing glyphs, overfull or underfull boxes, duplicate destinations, Hyperref warnings, LaTeX warnings, or font warnings; intentionally empty versos are truly empty. |
| HL-B30 | Complete (#9868) | Publish Hindi Chapters 6–33 and its writing companions from canonical lessons rather than hand-copying another twenty-eight book chapters. | Fifty-one lessons now use schema v2; forty later lessons generate twenty-eight chapters whose source hashes are independently verified against Language Ladder, while eleven prerequisite-ordered writing companions remain inside the gentle hand-authored opening chapters. |
| HL-B31 | Complete (#9871) | Remove Hindi's LaTeX layout, duplicate-label, bookmark, font, and header-only-verso warnings. | The forced 114-page build now has zero missing glyphs, overfull or underfull boxes, duplicate destinations, Hyperref warnings, LaTeX warnings, or font warnings; intentionally blank chapter versos are truly empty. |
| HL-B32 | Complete (#9875) | Publish Tamil Chapters 6–31 and its writing companions from canonical lessons rather than hand-copying another twenty-six book chapters. | Fifty-one lessons now use schema v2; forty-three later lessons generate twenty-six chapters whose source hashes are independently verified against Language Ladder, while eight prerequisite-ordered writing companions remain inside the gentle hand-authored opening chapter. |
| HL-B33 | Complete (#9880) | Remove Tamil's LaTeX layout, duplicate-label, bookmark, font, and header-only-verso warnings. | The forced 117-page build now has zero missing glyphs, overfull or underfull boxes, duplicate destinations, Hyperref warnings, LaTeX warnings, or font warnings; intentionally blank chapter versos are truly empty. |
| HL-B34 | Complete (#9883) | Publish Latin Chapters 2–36 from canonical lessons rather than hand-copying another thirty-five book chapters. | All 53 Latin lessons now use schema v2; Chapters 2–36 are generated with source hashes independently verified against Language Ladder. |
| HL-B35 | Complete (#9885, repaired by #9887) | Remove Latin's remaining LaTeX layout, font, and header-only-verso warnings. | The forced 115-page build now has zero missing glyphs, overfull or underfull boxes, duplicate destinations, Hyperref warnings, LaTeX warnings, or font warnings; intentionally blank chapter versos are truly empty. |
| HL-B36 | Complete (#9890) | Publish Spanish Chapters 19–33 from canonical lessons rather than hand-copying another fifteen book chapters. | The Spanish PDF now includes all 33 canonical chapters; all 21 later lessons use schema v2 and generate fifteen chapters from the same content consumed by Language Ladder. |
| HL-B37 | Complete (#9891) | Remove Spanish's remaining legacy LaTeX layout, bookmark, font, and header-only-verso warnings. | The forced 214-page build now has zero missing glyphs, overfull or underfull boxes, duplicate destinations, Hyperref warnings, LaTeX warnings, or font warnings; all 19 intentionally blank physical pages are truly empty. |
| HL-P01 | Complete (#9893) | Make the unified Human Languages Books result a protected merge gate, or add an equivalent gate that auto-merge must await. | Every pull request now receives a stable books gate; relevant changes run the one all-books build, unrelated changes receive a checked fast-path, and pull-request and push contexts remain distinct. |
| HL-M01 | Complete (#9894) | Add per-track spine realization maps and language-specific extension nodes. | All 20 tracks have validated repeated-segment local paths, explicit omissions/relocations, typed extensions, and a pure prerequisite-safe frontier planner. |
| HL-M10 | Complete (#9896) | Replace Learn's global concept cursor with per-language frontier progression and focused-before-mixed eligibility. | Stable per-language completed prefixes drive each next lesson; wrong focused answers cannot advance; only independently passed, visually distinguishable lessons enter mixed review. |
| HL-M02 | Queued | Extend Telugu's roadmap and authoritative session map through canonical Chapter 31. | The roadmap narrative stops at Chapter 6 and the session map at Chapter 5 even though prerequisite-ordered lessons continue through Chapter 31; every canonical lesson needs a scheduled place, and the map must explicitly split or justify Chapter 20's numbers-and-weather topic collision. |
| HL-M03 | Queued | Extend Kannada's roadmap and authoritative session map through canonical Chapter 31. | The roadmap narrative stops at Chapter 6 and the session map at Chapter 5; every canonical lesson needs a scheduled place, and Chapter 20's unrelated numbers/weather pairing must be split or explicitly justified. |
| HL-M04 | Queued | Extend Malayalam's roadmap and authoritative session map through canonical Chapter 31. | The roadmap narrative stops at Chapter 6 and the session map at Chapter 5 even though prerequisite-ordered lessons continue through Chapter 31; every canonical lesson, including the four new support steps, needs a scheduled place. |
| HL-M05 | Queued | Reconcile Arabic's roadmap and authoritative session map with canonical Chapters 1–27 and the sixteen-step writing sequence. | The roadmap details only Chapters 1–4 and still calls Chapter 5+ planned; the session map stops at Chapter 2 even though prerequisite-ordered canonical lessons continue through Chapter 27. |
| HL-M06 | Queued | Reconcile Hindi's roadmap and authoritative session map with canonical Chapters 1–33 and the eleven-step writing sequence. | The roadmap details only Chapters 1–6 and still calls Chapter 6 planned; the session map stops at Chapter 5 even though prerequisite-ordered canonical lessons continue through Chapter 33. |
| HL-M07 | Queued | Reconcile Tamil's roadmap and authoritative session map with canonical Chapters 1–31 and the eight-step writing sequence. | The roadmap details only Chapters 1–6 and still calls Chapter 7+ planned; the session map stops at Chapter 5 even though prerequisite-ordered canonical lessons continue through Chapter 31. |
| HL-M08 | Queued | Reconcile Latin's roadmap and authoritative session map with canonical Chapters 1–36. | Both files stop at Chapter 1 and describe Chapter 2+ as planned even though prerequisite-ordered canonical lessons continue through Chapter 36. |
| HL-M09 | Queued | Reconcile Spanish's roadmap and authoritative session map with canonical Chapters 1–33 and the new support steps. | The roadmap stops at Chapter 18 and calls Chapter 19 next, while the session map stops at Chapter 3; both lag the prerequisite-ordered canonical curriculum. |
| HL-T01 | Complete (#9904) | Complete session maps and pronunciation references for Persian and Urdu. | Both five-lesson prefixes now have authoritative N+1/N+3/N+7/N+15 ledgers and sound-id-keyed references; the Urdu guide explicitly preserves the Naskh fallback debt. |
| HL-U01 | Queued | Vendor and verify an appropriately licensed static Nastaliq font for normal Urdu presentation. | Naskh remains an explicit accessibility fallback, not the intended printed style. |

## P2 — corpus growth

| ID | Status | Work item | Completion signal |
|---|---|---|---|
| HL-E01 | Complete (#9906) | Author Persian and Urdu Chapter 3 through the rest of `SPINE-EXCHANGE-NAMES`. | Each track gains prerequisite-safe, schema-v2 micro-lessons for the name question, its formality distinction, and a meeting response; realization maps, objective activities, generated book chapters, and Language Ladder consume the same AST. |
| HL-E02 | Complete (#9913) | Author Persian and Urdu Chapter 4 through `SPINE-CHECK-WELLBEING` and reconcile the older identity-first roadmap. | Both tracks gain a gentle wellbeing exchange before identity grammar, with language-specific register/script extensions, exact review ledgers, objective practice, and generated book chapters from the canonical AST. |
| HL-E03 | Complete (#9914) | Author Persian and Urdu Chapter 5 through `SPINE-TAKE-LEAVE`. | Both tracks gain prerequisite-safe micro-lessons for ending a short respectful interaction, local script/register/grammar/etymology extensions, objective practice, exact review ledgers, and generated book chapters from the canonical AST. |

- Expand every track toward B1 using the gap report to choose the next missing
  can-do, skill, mode, register, or realization.
- Add controlled dialogues and micro-stories whose tokens are validated against
  prior knowledge.
- Add provenance-labelled listening and dictation activities from the same
  canonical lesson blocks.

## Findings from HL-S01

- Spanish Chapters 1–3 contain 24 schema-v2 lessons after three overlong
  explanations were split into prerequisite-ordered support lessons for noun
  gender, the Latin *qu-* question family, and the origin of *usted*.
- The resulting snapshot has 976 lessons, 481 duration violations, and 40
  later-chapter prerequisite roots: four and two fewer respectively than the
  HL-V01 baseline, with the remaining debt still explicit.
- Every migrated lesson computes below 300 seconds; the tightest current budget
  is *buenos días* at 296 seconds, which should be watched during copy edits.
- Schema v2 now validates canonical spine mapping, unique local sequence,
  typed body blocks, explicit coverage metadata, same-language prerequisites,
  and transitive knowledge closure. Block-boundary prompt/answer knowledge
  declarations remain a later refinement; this slice does not claim them.

## Findings from HL-S02

- Spanish Chapters 4–6 now contain 27 schema-v2 lessons: the 25 existing
  vocabulary, grammar, practice, and writing lessons plus two short repair
  lessons that teach **y** and **café** before later dialogue asks learners to
  use them. Spanish now has 51 schema-v2 lessons and 77 legacy lessons.
- Every migrated lesson has a unique sequence from 250 through 510, begins with
  a typed warm-up, ends with typed recall, closes all declared knowledge over
  its transitive prerequisites, and remains below 300 effective seconds.
- Forward references to later material such as *sí*, *un poco*, *ojalá*, and
  untaught future farewells were removed from production prompts. The surviving
  dialogues and script exercises require only what the learner already knows.
- The editorial audit also caught undeclared prompt tokens that lesson-level
  atom closure cannot see. HL-V02 records block-level prompt closure as the
  next validation enhancement before schema migration expands beyond this
  carefully audited slice.
- The typed-block parser now recognizes `Script` sections explicitly, so the
  accent, eñe, and inverted-question lessons remain first-class canonical app
  content rather than falling into an unknown presentation bucket.
- This tranche deliberately does not replace the handwritten LaTeX chapters.
  HL-G03 is next and will generate Chapters 4–6 only after this canonical
  content contract has merged.

## Findings from HL-G01

- Spanish Chapter 1 is generated deterministically from seven canonical
  schema-v2 lessons in authored `sequence` order; the 18-book source is now 122
  rendered pages with no generated-chapter overfull boxes.
- The generated chapter and Language Ladder independently combine the same
  per-lesson FNV-1a fingerprints. The app exposes `book synced` only when its
  loaded Chapter 1 lesson AST matches the committed manifest.
- The unified book job now fails when generated TeX or the hash manifest is
  missing or stale. The fingerprint is a deterministic drift signal, not a
  cryptographic integrity claim.
- At the end of HL-G01, Chapter 1 was the first one-source slice and Chapters
  2–18 remained handwritten. That finding deliberately scoped HL-G02 to the
  already-schema-v2 Chapters 2–3 rather than skipping validation to generate
  later chapters.

## Findings from HL-G02

- All 24 schema-v2 Spanish lessons in Chapters 1–3 now generate their three
  LaTeX chapters and independently match Language Ladder's loaded AST. Chapter
  2 combines five lesson hashes; Chapter 3 combines twelve.
- The expanded canonical content produces a 138-page book. Rendered checks of
  both chapter openers, grammar and etymology boxes, nested emphasis, practice
  lists, and wrap-up recall found no generated-chapter overfull box or Hyperref
  warning.
- The renderer now handles nested bold-within-italic Markdown, wraps practice
  lists ragged-right, and keeps math arrows out of bookmark/running-header
  strings. Those fixes apply to every later generated chapter.
- The next learner-visible promise is the sub-five-minute cap. Russian is the
  smallest complete existing track with measurable debt: nine violations, of
  which five are computed at 312–405 seconds and four only need honest declared
  budgets below the cap. HL-D01A is therefore the next bounded tranche.

## Findings from HL-G03

- All 51 schema-v2 Spanish lessons in Chapters 1–6 now generate the same six
  LaTeX chapters whose source hashes Language Ladder recomputes from its loaded
  AST. The per-chapter lesson counts are 7, 5, 12, 13, 7, and 7.
- The shared renderer now preserves valid Markdown tables as width-aware LaTeX
  tables and maps the approximation sign safely. This keeps register contrasts,
  question families, farewell choices, and verb forms structured in both app
  and book instead of flattening them into pipe-delimited prose.
- The forced XeLaTeX build produces 158 pages. Every rendered page in the
  Chapter 4–6 span was checked for clipping, collisions, broken diacritics,
  malformed tables, and accidental blank pages; those generated chapters have
  no missing glyph, overfull/underfull box, or Hyperref warning.
- Remaining Spanish PDF warnings come from legacy Chapters 7–18 and stay
  explicit in HL-B37. HL-V02 is next because the HL-S02 editorial audit showed
  that lesson-level atom closure alone cannot detect undeclared target-language
  tokens inside learner production and recall prompts.

## Findings from HL-V02

- All 51 schema-v2 Spanish lessons now declare introductions and assessments at
  every one of their typed body boundaries. Production and recall blocks require
  non-empty assessment declarations, and all other blocks retain explicit empty
  lists when they change no knowledge state.
- Validation follows rendered order: assessed atoms must already belong to the
  lesson's transitive prerequisite frontier or an earlier block, block
  introductions must account exactly for `introduces.knowledge`, and assessed
  atoms must be declared in `practises.knowledge`.
- The editorial migration removed premature *muy bien*, *¿y usted?*, *el gusto
  es mío*, *ojalá*, and next-chapter question-form production. It promoted *te
  llamas* and *gusto* to explicit atoms and completed grammar, script,
  etymology, and phrase practice declarations exposed by the boundary audit.
- Block metadata changes the shared canonical hashes but not learner copy. The
  six generated Spanish chapters omit the directives, and Language Ladder now
  explicitly filters them from its lightweight Markdown view.
- Individual prompt/answer/variant/feedback records remain prose rather than a
  compiled activity schema. HL-V03 records that next validation layer; HL-B04
  is the next bounded learner-visible publication gap.

## Findings from HL-D01A

- Russian now has zero duration violations. The repository snapshot contains
  980 lessons and 472 violations overall, down from 481 before this tranche;
  unknown prerequisites remain at zero.
- Four lessons already computed below five minutes and only needed their
  declared estimates corrected. The five genuinely long lessons were shortened
  through de-duplication or split into four prerequisite-ordered support and
  practice lessons.
- The cross-language formality comparison, naming-as-action comparison, person
  shapes, and precise zero-copula explanation remain in the canonical corpus.
  The tightest changed lesson is `RU-C01-privet` at 293 computed seconds; every
  other changed or new lesson has a larger buffer.
- Marathi's eight violations are the smallest remaining track-sized set, ahead
  of Gujarati's nine and Punjabi's and Sanskrit's ten each. HL-D01B is therefore
  the next bounded duration tranche after this PR merges.

## Findings from HL-D01B

- Marathi now has zero duration violations. The repository snapshot contains
  981 lessons and 464 violations overall, down from 472 before this tranche;
  unknown prerequisites remain at zero.
- Seven lessons already computed between 126 and 171 seconds and only needed
  honest four-minute declared budgets. The one genuinely long lesson computed
  at 321 seconds.
- That counting lesson is now a 163-second core followed by a 240-second
  etymology lesson. The analogy and retention explanations remain complete and
  prerequisite-ordered in the canonical corpus consumed by Language Ladder.
- The audit also made a publication boundary explicit: Marathi Chapter 6 has
  canonical lessons but is not in the current five-chapter PDF. HL-B04 records
  the one-source migration and generation work instead of adding another manual
  copy.
- A forced build of the unchanged five-chapter book still succeeds with zero
  overfull boxes, but exposes four duplicate practice labels, 32 Unicode
  bookmark warnings, and two underfull boxes. HL-B05 records that pre-existing
  publication hygiene debt separately from the lesson remediation.
- Gujarati's nine violations are now the smallest remaining track-sized set,
  ahead of Punjabi's and Sanskrit's ten each. HL-D01C is therefore next after
  this PR merges.

## Findings from HL-B04

- Marathi Chapter 6 now comes from its two canonical schema-v2 lessons. The
  generator manifest and Language Ladder independently combine the same ordered
  lesson hashes, so an app/book edit can no longer drift silently.
- The strict migration adds the first shared `SPINE-COUNT-ONE-TO-FIVE` can-do
  node. Marathi keeps its local Devanagari, pronunciation, and historical
  extensions while later number chapters can reuse the communicative spine.
- Generated non-Latin chapters exposed a reusable pipeline need: each target may
  name a Unicode Script property and its book's existing LaTeX font command.
  Devanagari runs are wrapped automatically; Latin prose and bookmark-safe
  romanization remain in the main font.
- The deterministic report still measures 1,065 lessons, 20 books, zero duration
  violations, and zero unknown prerequisites. Publishing this chapter reduces
  the lesson-to-book chapter gap from 257 to 256 and moves Marathi from legacy
  to mixed schema status.
- The forced 31-page XeLaTeX build has zero missing glyphs and zero overfull
  boxes. The generated pages preserve Devanagari shaping, width-aware tables,
  box titles, and recall prompts without clipping; one new underfull page joins
  the older warning debt recorded in HL-B05.
- HL-B05 records the bounded cleanup for the older handwritten chapters so its
  presentation warnings are not confused with generated-chapter drift.

## Findings from HL-B05

- Each handwritten recap now uses its canonical lesson id (`MR-C01-practice`
  through `MR-C05-practice`) instead of five copies of `lesson:practice`.
- Hyperref's PDF-string fallback strips only the `\mr` / `\marathifont`
  presentation wrapper. The inspected outline keeps readable Devanagari and
  romanization in all handwritten section bookmarks, while the generated
  chapter keeps its intentionally Latin short titles.
- `\raggedbottom` lets pages around unbreakable lesson callouts end naturally
  instead of stretching vertical glue. Visual inspection of the three formerly
  underfull pages plus the final recall page found no clipping, collision, or
  awkward box spacing.
- The vendored static Devanagari file is now explicitly selected for regular,
  bold, italic, and bold-italic requests. This matches the old fallback's glyph
  appearance while avoiding misleading unavailable-shape warnings.
- The forced 31-page XeLaTeX build now reports zero package or LaTeX warnings,
  missing glyphs, overfull boxes, underfull boxes, and duplicate destinations.
  HL-B06 is next: publish Gujarati Chapter 6 through the same canonical pipeline.

## Findings from HL-B06

- Gujarati Chapter 6 now comes from its two canonical schema-v2 lessons. The
  generator manifest and Language Ladder independently combine the same ordered
  lesson hashes, so the app and downloadable book cannot drift silently.
- Both lessons realize `SPINE-COUNT-ONE-TO-FIVE` while retaining Gujarati's
  local headless-script clue, the *dvé → be* assimilation path, and the learned
  restoration of *r* in *traṇ*. Their effective 174- and 253-second boundaries
  remain below the strict five-minute ceiling.
- The shared Unicode generator wraps Gujarati runs with the book's existing
  `\gu` command, while authored romanization supplies readable section
  bookmarks. The final outline contains `ek be traṇ chār pā̃ch` and `be · traṇ`.
- The deterministic report now measures 1,065 lessons, 20 books, zero duration
  violations, zero unknown prerequisites, and 255 lesson chapters without book
  chapters. Gujarati joins Spanish and Marathi as the third mixed-schema track.
- The forced letter-size XeLaTeX build is 27 pages. Visual inspection of every
  generated page found shaped Gujarati, width-aware tables, intact callouts, and
  no clipping; tightening the final four-question recall kept it on one page.
- The generated chapter adds no new missing-glyph, overfull, underfull,
  duplicate-label, bookmark, or font warnings. HL-B07 remains the bounded cleanup
  for warnings already present in the five handwritten chapters.

## Findings from HL-B07

- Each handwritten recap now uses its canonical lesson id (`GU-C01-practice`
  through `GU-C05-practice`) instead of five copies of `lesson:practice`.
- Latin commas and the ellipsis now sit outside `\gu`, so the Gujarati-only
  static font is asked to shape only Gujarati characters while the visible
  punctuation remains unchanged.
- Hyperref's PDF-string fallback strips only the `\gu` / `\gujaratifont`
  presentation wrapper. The inspected outline keeps readable Gujarati and
  romanization across all handwritten sections, including `મારું નામ…છે`, while
  generated Chapter 6 keeps its intentionally Latin short titles.
- `\raggedbottom` lets pages around unbreakable lesson callouts end naturally.
  Visual inspection of all four formerly underfull pages plus the repaired
  Chapter 5 recap found no clipping, collision, or awkward box spacing.
- The vendored static Gujarati file is now explicitly selected for regular,
  bold, italic, and bold-italic requests. This preserves the prior glyph
  appearance without reporting unavailable font shapes.
- Rephrasing the three copula forms gives TeX natural breakpoints without
  changing the recap's meaning. The forced 27-page XeLaTeX build now reports
  zero package or LaTeX warnings, missing glyphs, overfull boxes, underfull
  boxes, and duplicate destinations. HL-B08 is next: publish Punjabi Chapter 6
  through the same canonical pipeline.

## Findings from HL-B08

- Punjabi Chapter 6 now comes from its two canonical schema-v2 lessons. The
  generator manifest and Language Ladder independently combine the same ordered
  lesson hashes, so the app and downloadable book cannot drift silently.
- Both lessons realize `SPINE-COUNT-ONE-TO-FIVE` while retaining Punjabi's
  Gurmukhi top-line clue, addak/tippi distinction, Chapter 5 five-rivers
  callback, and the independent Punjabi/Persian paths to *panj*.
- The strict knowledge gate confirms every prompt against its block and
  prerequisite frontier. The corpus report now has 1,065 lessons, 20 books,
  zero duration violations, zero unknown prerequisites, and 254 lesson chapters
  without book chapters. Punjabi joins Spanish, Marathi, and Gujarati as the
  fourth mixed-schema track.
- The forced letter-size XeLaTeX build is 30 pages. Visual inspection of all
  four generated pages found shaped Gurmukhi, width-aware comparison tables,
  intact callouts, a complete spaced-recall close, and no clipping. The PDF
  outline retains the romanized `ikk do tinn chār panj` and `panj · panj`
  section bookmarks.
- The generated chapter adds no new missing-glyph, overfull, underfull,
  duplicate-label, bookmark, or font warning. The audit did expose three
  pre-existing font-shape warnings omitted from the earlier inventory; HL-B09
  now includes those alongside the handwritten chapters' other warning debt.

## Findings from HL-B09

- Each handwritten recap now uses its canonical lesson id (`PA-C01-practice`
  through `PA-C05-practice`) instead of five copies of `lesson:practice`.
- Hyperref's PDF-string fallback strips only the `\pa` / `\punjabifont`
  presentation wrapper. The inspected outline retains readable Gurmukhi and
  romanization across every handwritten section, while generated Chapter 6
  keeps its intentional romanized short titles.
- `\raggedbottom` lets pages around unbreakable lesson callouts end naturally.
  Visual inspection of all four formerly underfull pages found shaped text,
  intact boxes, and no clipping, collision, or awkward vertical stretching.
- The vendored static Gurmukhi file is now explicitly selected for regular,
  bold, italic, and bold-italic requests. This preserves the existing glyph
  appearance without reporting unavailable font shapes.
- The `ਤੂੰ` / `ਤੁਸੀਂ` section now has a natural-language separator and a shorter
  running title, removing the lone overfull header without changing the lesson.
  The forced 30-page XeLaTeX build now reports zero package or LaTeX warnings,
  missing glyphs, overfull boxes, underfull boxes, and duplicate destinations.
  HL-B10 is next: publish Sanskrit Chapter 6 through the canonical pipeline.

## Findings from HL-B10

- Sanskrit Chapter 6 now comes from its three canonical schema-v2 lessons. The
  generator manifest and Language Ladder independently combine the same ordered
  lesson hashes, so the app and downloadable book cannot drift silently.
- All three lessons realize `SPINE-COUNT-ONE-TO-FIVE`, then extend it with
  Sanskrit's dual and gendered numeral forms, the daughter languages' neuter
  inheritance, PIE sound-law outcomes, and qualified lexical histories.
- The canonical prose now says Sanskrit preserves the Old Indo-Aryan forms
  behind the daughter languages and labels *four*'s `f-` as the usual analogy
  explanation rather than presenting either relationship too absolutely.
- The strict knowledge gate confirms every prompt against its block and
  prerequisite frontier. The corpus report remains at 1,065 lessons and 20
  books, with zero duration violations, zero unknown prerequisites, and 253
  lesson chapters without book chapters. Sanskrit becomes the fifth
  mixed-schema track.
- The forced letter-size XeLaTeX build is 35 pages. Visual inspection of all
  five generated pages found shaped Devanagari, width-aware grammar and
  sound-law tables, intact callouts and recall prompts, and no clipping. The PDF
  outline retains all three romanized section bookmarks.
- A text mapping renders the PIE superscript `ʷ` without a missing glyph. The
  generated chapter adds one visually benign underfull-page warning but no new
  overfull, duplicate-label, bookmark, or font warning. The full build's three
  font-shape warnings and seven underfull warnings are now recorded accurately
  in HL-B11 alongside the older handwritten warning debt.

## Findings from HL-B11

- Sanskrit's five authored recap anchors now use stable chapter-qualified ids
  (`SA-C01-practice` through `SA-C05-practice`) instead of five copies of
  `lesson:practice`.
- Devanagari is retained in the PDF outline while the presentation-only font
  switch is suppressed there. The vendored static font is selected explicitly
  for regular, bold, italic, and bold-italic requests, preserving the existing
  glyph appearance without unavailable-shape substitutions.
- Short pages now end naturally around kept-together lesson callouts. Concise
  running titles for the long “you,” “what,” and *karomi* sections remove the
  three overfull headings without removing lesson content. The forced 35-page
  XeLaTeX build reports zero package or LaTeX warnings, missing glyphs,
  overfull boxes, underfull boxes, and duplicate destinations. HL-B12 is next:
  publish Bengali Chapter 6 through the canonical pipeline.

## Findings from HL-B12

- Bengali Chapter 6 now comes from its canonical schema-v2 lesson. The
  generator manifest and Language Ladder independently combine the same source
  hash, so the app and downloadable book cannot drift silently.
- The lesson realizes `SPINE-COUNT-ONE-TO-FIVE` in a 290-second boundary and
  extends it with Bengali script, chandrabindu nasalization, the conservative
  vowel in *dui*, the everyday numeral's simplified *dv-* cluster, and the
  qualified Assamese/Odia/Nepali comparison.
- HL-I01 records publication latency observed after HL-B11: content drift and
  Bengali's warning debt remain learner-visible priorities ahead of optimizing
  a successful but slow single-job TeX setup.
- The strict data suite passes all 80 tests; Language Ladder passes all 287
  tests and its production build. The report remains at 1,065 lessons and 20
  books with zero duration violations and zero unknown prerequisites, while
  book drift falls to 252 lesson chapters and Bengali becomes the sixth
  mixed-schema track.
- The forced letter-size XeLaTeX build is 29 pages. Visual inspection of all
  three generated pages found shaped Bengali, width-aware numeral and etymology
  tables, an intact chandrabindu and recall box, and no clipping. The outline
  retains the authored `ek dui tin chār pā̃ch` bookmark.
- The generated chapter adds one visually benign underfull-page warning but no
  new missing glyph, overfull, duplicate-label, bookmark, or font warning. The
  full build's six missing glyphs, one overfull box, five underfull boxes, four
  duplicate recap labels, 27 Hyperref warnings, and three font-shape warnings
  are recorded accurately in HL-B13.

## Findings from HL-B13

- Bengali's five authored recap anchors now use stable chapter-qualified ids
  (`BN-C01-practice` through `BN-C05-practice`) instead of five copies of
  `lesson:practice`.
- Ellipsis, comma, and morpheme-boundary hyphen punctuation now stays in the
  Latin main font instead of being asked of the Bengali-only static font.
  Bengali remains visible in the PDF outline while the presentation-only font
  command is suppressed there, and all requested font shapes resolve to the
  vendored static file.
- Short pages end naturally around kept-together callouts, and the long
  “we'll meet again” title has a safe line-break point. The forced 29-page
  XeLaTeX build reports zero package or LaTeX warnings, missing glyphs,
  overfull boxes, underfull boxes, and duplicate destinations. HL-I01 is next:
  reduce successful unified-publication setup latency without splitting the
  single all-books job.
- Visual regression inspection covered nine formerly affected pages: Bengali
  shaping and main-font punctuation remain intact, the bilingual farewell title
  breaks cleanly, split callouts are unclipped, and short pages have deliberate
  natural bottoms. All 39 PDF outline entries remain readable, ending with the
  generated romanized Chapter 6 title.

## Findings from HL-I01

- Three successful unified-publication baselines spent 7:00, 8:09, and 8:02
  installing `texlive-full`; the most recent then built all 20 books in 1:41.
  Setup, not the single-job book loop, was the dominant and variable cost.
- The complete TeX source inventory uses the standard `book` class and eleven
  packages. Ubuntu's `texlive-xetex` dependency closure provides the LaTeX base,
  recommended, and extra collections containing those packages; the focused
  install adds `texlive-lang-arabic` for `bidi.sty`, `lmodern` for the named
  Latin Modern faces, `texlive-fonts-recommended` for Hyperref's `pzdr.tfm`,
  and `latexmk` as the build driver.
- All non-Latin faces are repository-vendored static fonts, so the system-wide
  Noto font collections are unrelated to the current builds. A fail-closed
  preflight now resolves the engine, driver, all eleven packages, the class,
  RTL support, and Latin Modern before compiling any book.
- The workflow remains one job with one setup, one dynamically discovered
  20-book loop, one verified artifact, and one `main`-only Pages publication.
  The exact merged-main run installed the focused toolchain in 87 seconds,
  built all books in 93 seconds, verified the bundle, and published Pages.
  HL-B14 and HL-B15 have since closed the Italian app/book drift and its measured
  presentation debt. HL-B16 is next: apply the same canonical generation path
  to Portuguese Chapters 2–17.

## Findings from HL-B14

- Italian Chapters 2–17 now comprise 49 strict schema-v2 micro-lessons with
  explicit shared-spine anchors, prerequisite-closed knowledge atoms, typed
  teaching blocks, and authored skill, mode, strand, register, variety, and
  duration contracts. Chapter 1 remains readable legacy content, so the track
  is intentionally mixed while one-source migration proceeds incrementally.
- All 49 migrated lessons remain below five minutes. `IT-C17-mano` is the
  tightest at 298 computed seconds; curriculum validation reports zero duration
  violations and zero unknown or misordered prerequisites.
- Sixteen deterministic generation targets now publish Chapters 2–17 from the
  same lesson AST loaded by Language Ladder. Their manifest covers all 49
  lessons, and app tests independently reproduce every chapter hash and lesson
  count. Repository-wide missing book chapters fall from 252 to 236.
- The generic renderer recognizes scoped “taken apart” headings and emits
  portable TeX for the scholarly symbols `↔`, `ṓ`, `₁`, and `ʰ`, preserving the
  app's precise Unicode while avoiding font-dependent gaps in generated PDFs.
- A forced XeLaTeX build produces a 104-page book with zero missing glyphs,
  duplicate destinations, or leaked generator metadata. All 104 rendered pages
  were inspected; the cover, three-page contents, chapter openings, callouts,
  dense tables, and final recall are unclipped, and the PDF outline contains
  Preface, pronunciation, and all seventeen chapter destinations.
- Four overfull boxes, ten underfull boxes, and the three pre-existing Chapter
  1 Hyperref warnings remain. HL-B15 is next and now records the expanded,
  measured clean-build debt rather than the old 13-page baseline.

## Findings from HL-B15

- The inline renderer now honors backslash escapes for Markdown punctuation, so
  an etymological reconstruction such as `**\*parabolāvit**` becomes one bold
  literal form rather than malformed nested emphasis. A focused regression test
  keeps the canonical app text and generated TeX aligned.
- Generated tables now begin without paragraph indentation. This removes the
  otherwise invisible 17-point width excess while preserving full-width,
  ragged-right columns for every generated language chapter.
- Italian's legacy Chapter 1 heading now supplies a bookmark-safe short title,
  and `\raggedbottom` makes deliberately short lesson pages explicit. Targeted
  canonical copy and table-cell edits remove the remaining horizontal layout
  warnings without dropping any vocabulary, grammar, or etymology.
- A forced XeLaTeX build produces 104 pages with zero missing glyphs, overfull
  or underfull boxes, duplicate destinations, Hyperref warnings, or LaTeX
  warnings. All pages were rendered and inspected with no clipping or
  collisions; the outline retains Preface, pronunciation, and Chapters 1–17,
  and no schema or source-hash metadata leaks into extracted text.
- HL-B16 is next: migrate Portuguese Chapters 2–17 to the same strict schema-v2
  lesson AST and publish them through the shared book/app generation path.

## Findings from HL-B16

- Portuguese Chapters 2–17 now comprise 50 strict schema-v2 micro-lessons with
  explicit shared-spine anchors, prerequisite-closed knowledge atoms, typed
  teaching blocks, and authored skill, mode, strand, register, variety, and
  duration contracts. Chapter 1 remains readable legacy content while the
  one-source migration proceeds incrementally.
- All 50 migrated lessons remain below five minutes: computed durations span
  141–298 seconds, with `PT-C17-mao` the tightest. Curriculum validation reports
  zero duration violations and zero unknown or misordered prerequisites.
- Sixteen deterministic generation targets now publish Chapters 2–17 from the
  same lesson AST loaded by Language Ladder. Their manifest covers all 50
  lessons, and app tests independently reproduce every chapter hash and lesson
  count. Repository-wide missing book chapters fall from 236 to 220.
- Chapter 4 preserves Arabic `حتى` beside transliterated *ḥattā*. Its generated
  run now uses the repository-vendored Noto Naskh Arabic font, preventing the
  Latin-only PDF path from silently dropping source-script evidence.
- A forced XeLaTeX build produces a 105-page book with zero missing glyphs,
  duplicate destinations, Hyperref warnings, LaTeX warnings, or leaked
  generator metadata. All pages were rendered and inspected; the outline
  retains Preface, pronunciation, and Chapters 1–17.
- Six overfull boxes and thirteen underfull boxes remain in the expanded book.
  HL-B17 is next and records this measured 105-page presentation debt rather
  than the old 13-page baseline.

## Findings from HL-B17

- Portuguese now uses `\raggedbottom`, making intentionally short micro-lesson
  pages explicit and removing eleven underfull vertical boxes without padding
  or stretching learner content.
- Six canonical lessons received small copy-flow edits: shorter resumable
  headings, clearer sentence boundaries, and two deliberate warm-up paragraph
  breaks. The same meaning, vocabulary, grammar, and etymology remain in both
  Language Ladder and the generated book.
- Regeneration updates the six affected chapter fingerprints, which Language
  Ladder independently reproduces from the canonical AST. All fifty migrated
  lessons remain below five minutes and prerequisite-closed.
- A forced XeLaTeX build produces 105 pages with zero missing glyphs, overfull
  or underfull boxes, duplicate destinations, Hyperref warnings, or LaTeX
  warnings. Every page was rendered and inspected; the outline retains Preface,
  pronunciation, and Chapters 1–17, and no generator metadata leaks into text.
- HL-B18 is next: close the smaller seven-chapter French app/book gap before its
  measured presentation-cleanup follow-up.

## Findings from HL-B18

- French Chapters 17–23 now comprise nine strict schema-v2 micro-lessons with
  explicit shared-spine anchors, prerequisite-closed knowledge atoms, typed
  teaching blocks, and authored skill, mode, strand, register, variety, and
  duration contracts. Chapters 1–16 remain readable legacy content while the
  one-source migration proceeds incrementally.
- All nine migrated lessons remain below five minutes: computed durations span
  194–287 seconds. Curriculum validation reports zero duration violations and
  zero unknown or misordered prerequisites.
- Seven deterministic generation targets now publish Chapters 17–23 from the
  same lesson AST loaded by Language Ladder. Their manifest covers all nine
  lessons, and app tests independently reproduce every chapter hash and lesson
  count. Repository-wide missing book chapters fall from 220 to 213.
- A forced XeLaTeX build produces a 98-page book with zero missing glyphs,
  duplicate destinations, LaTeX warnings, or leaked generator metadata. All
  pages were rendered and inspected; the outline retains Preface,
  pronunciation, and Chapters 1–23.
- The expanded book retains the exact pre-existing warning baseline: sixteen
  overfull boxes, nine underfull boxes, and six Hyperref warnings. HL-B19 is
  next and records cleanup against the full 98-page artifact.

## Findings from HL-B19

- French now uses `\raggedbottom`, making intentionally short micro-lesson
  pages explicit and removing nine underfull vertical boxes without padding or
  stretching learner content.
- Six concise optional section titles keep legacy running headers inside the
  text block, while two prose-only Chapter 12 titles provide clean PDF
  bookmarks without changing the visible mathematical arrows.
- Three internal source paths can now break naturally. Five dense legacy tables
  use a flexible final column, preserving every comparison while removing
  horizontal overflow, and one pronominal-verb explanation has clearer sentence
  boundaries for the same grammatical rule.
- A forced XeLaTeX build produces 98 pages with zero missing glyphs, overfull or
  underfull boxes, duplicate destinations, Hyperref warnings, or LaTeX warnings.
- HL-B20 is next: close the seven-chapter German app/book gap before its measured
  presentation-cleanup follow-up.

## Findings from HL-B20

- German Chapters 17–23 now comprise ten strict schema-v2 micro-lessons with
  explicit shared-spine anchors, prerequisite-closed knowledge atoms, typed
  teaching blocks, and authored skill, mode, strand, register, variety, and
  duration contracts. Their computed durations range from 164 to 262 seconds.
- The audit found that `Entschuldigung` occupied Chapter 19 while no Chapter 20
  lesson existed. A new Chapter 19 lesson now reviews `bitte` as “please” in
  **Wasser, bitte** using only Chapters 3 and 11; the unchanged apology content
  follows as prerequisite-dependent Chapter 20.
- Seven generated chapters carry deterministic hashes and lesson ids that
  Language Ladder independently reproduces from the canonical AST. The corpus
  grows to 1,066 lessons, repository-wide missing book chapters fall from 213
  to 207, and unknown prerequisites and duration violations remain at zero.
- A forced XeLaTeX build produces 104 pages with zero missing glyphs, duplicate
  destinations, LaTeX warnings, or leaked generator metadata. All pages were
  rendered and inspected; the outline retains Preface, pronunciation, and
  Chapters 1–23.
- The expanded warning baseline is eighteen overfull boxes, one underfull
  horizontal box, eleven underfull vertical boxes, and three Hyperref warnings.
  HL-B21 is next and records cleanup against the full 104-page artifact.

## Findings from HL-B21

- German now uses `\raggedbottom`, making intentionally short micro-lesson
  pages explicit and removing eleven underfull vertical boxes without adding
  filler or stretching learner content.
- Concise running titles, one prose-only bookmark, a breakable practice path,
  and three reflowed passages remove header, path, and paragraph overflow while
  preserving the same grammar and etymology explanations.
- Ten dense legacy tables now use responsive or explicitly bounded paragraph
  columns. Every register, vocabulary, conjugation, weekday, and word-origin
  comparison remains present and readable inside the text block.
- The two canonical copy edits keep the generated German chapters and Language
  Ladder hashes aligned: the `Kopf` recall reflows cleanly, while the shorter
  visible `Entschuldigung` heading leaves its complete “un-guilting” etymology
  in the lesson body.
- Full-page inspection found that straight ASCII quotes elsewhere in generated
  prose can become right-only quotation marks under German language rules.
  HL-G04 records a cross-book generator fix; it follows the larger missing-book
  gaps rather than expanding this focused layout tranche.
- A forced XeLaTeX build produces 104 pages with zero missing glyphs, overfull
  or underfull boxes, duplicate destinations, Hyperref warnings, or LaTeX
  warnings. All 104 rendered pages were inspected; the outline retains the
  Preface, pronunciation reference, and Chapters 1–23. HL-B22 is next: publish
  Telugu Chapters 6–31 from canonical lessons.

## Findings from HL-B22

- All thirty canonical Telugu lessons after Chapter 5 now use schema v2 with
  explicit spine nodes, prerequisite-safe sequences, honest sub-five-minute
  duration budgets, typed knowledge boundaries, skills, modes, strands,
  register, and variety metadata. The first thirty lessons remain schema v1,
  so the track is intentionally mixed while migration proceeds incrementally.
- Twenty-six generated chapters carry deterministic hashes and lesson ids that
  Language Ladder independently reproduces from the canonical AST. Telugu book
  coverage is now 100%, and the app and downloadable book share the same source
  through Chapter 31.
- The shared generator now supports named multi-script font sets. Telugu's
  comparison passages can render Telugu, Tamil, Kannada, Malayalam, Devanagari,
  and Arabic-script examples without hand-authored LaTeX or missing glyphs.
- Chapter 20 currently combines the numbers 11–20 with an unrelated weather
  lesson. HL-M02 records the need for the authoritative roadmap and session map
  to split that progression or explain the grouping explicitly.
- A forced XeLaTeX build produces 95 pages with zero missing glyphs or leaked
  generator metadata. All pages were rendered and inspected; the outline keeps
  Preface, the script reference, and Chapters 1–31 in order.
- The expanded warning baseline is eleven overfull boxes, nine underfull
  vertical boxes, four duplicate practice labels, 104 Hyperref warnings, and
  nine font warnings. No visual clipping was found; HL-B23 is next and records
  cleanup against the complete 95-page artifact.
- The repository corpus contains 1,066 lessons with zero unknown prerequisites
  and zero duration violations.
- A clean data-package install surfaced the moderate, development-only
  GHSA-fxqj-rqcc-2cmp advisory through Vitest, Vite, and PostCSS 8.5.19. A
  non-breaking 8.5.25 resolution is available; HL-I02 records the lockfile
  maintenance behind the remaining reader-facing book and app gaps.

## Findings from HL-B23

- Explicit regular, bold, italic, and bold-italic faces for every vendored
  comparison font remove nine substitution warnings while keeping Telugu,
  Tamil, Kannada, Malayalam, Devanagari, and Arabic-script examples available.
- Bookmark-safe definitions preserve the visible script while removing font
  presentation commands from PDF strings. All 104 Hyperref warnings disappear,
  and the outline retains Preface, the script reference, and Chapters 1–31.
- Five legacy practice sections now have chapter-specific labels. `\raggedbottom`
  makes natural micro-lesson page endings explicit, removing four duplicate
  destinations and nine underfull vertical boxes without adding filler.
- Concise visible headings, one responsive table, a three-part month list, and
  a shorter Chapter 20 title remove eleven overfull lines while preserving every
  vocabulary item, grammar explanation, comparison, and etymology in the body.
- Full-page review caught a long Section 4.4 running header touching its page
  number even after the build log was clean. A prose-only running title fixes
  that collision while retaining the complete Telugu heading in the lesson.
- A forced XeLaTeX build produces 95 pages with zero missing glyphs, overfull or
  underfull boxes, duplicate destinations, Hyperref warnings, LaTeX warnings,
  or font warnings. All pages were rendered and inspected; metadata, 33
  top-level bookmarks, 93 total outline entries, and generator-leak checks pass.
- HL-B24 is next: publish Kannada Chapters 6–31 from canonical lessons.

## Findings from HL-B24

- All thirty canonical Kannada lessons after Chapter 5 now use schema v2 with
  explicit spine nodes, prerequisite-safe sequences, honest sub-five-minute
  duration budgets, typed knowledge boundaries, skills, modes, strands,
  register, and variety metadata. The first thirty lessons remain schema v1,
  so the track is intentionally mixed while migration proceeds incrementally.
- Twenty-six generated chapters carry deterministic hashes and lesson ids that
  Language Ladder independently reproduces from the canonical AST. Kannada
  book coverage is now 100%, and the app and downloadable book share the same
  source through Chapter 31.
- A reusable Kannada comparison-font set renders Kannada, Tamil, Telugu,
  Malayalam, Devanagari, and Arabic-script examples without hand-authored
  LaTeX. The expanded book has zero missing glyphs, including PIE subscript and
  accented transliteration characters used by the etymology lessons.
- Chapter 20 currently combines numbers 11–20 with an unrelated weather lesson.
  HL-M03 records the need for the authoritative roadmap and session map to split
  that progression or explain the grouping explicitly.
- A forced XeLaTeX build produces 96 pages with 33 top-level and 93 total
  outline entries, correct title and author metadata, and no leaked generator
  directives. Every rendered page was inspected; no clipping, collision, or
  accidental blank page was found.
- The expanded warning baseline is nine overfull boxes, three underfull
  horizontal boxes, seven underfull vertical boxes, four duplicate practice
  labels, 106 Hyperref warnings, and nine font warnings. HL-B25 is next and
  records cleanup against the complete 96-page artifact.
- The unified publication gate builds all twenty books successfully, while the
  data package passes 84 tests and Language Ladder passes 385 tests plus its
  production build.

## Findings from HL-B25

- Explicit regular, bold, italic, and bold-italic faces cover every script used
  by Kannada comparisons without changing the vendored glyph source. Bookmark
  fallbacks retain readable Unicode while omitting presentation-only font
  commands.
- The five handwritten recap labels are unique, and shorter visible or running
  titles preserve transliteration and etymology in the lesson body without
  overflowing page headers or PDF bookmarks.
- Narrow canonical copy edits keep the complete teaching content while giving
  long multilingual lines natural breakpoints. The generated chapter hashes
  continue to be reproduced independently by the data package and Language
  Ladder.
- Natural page bottoms and the final line-break fixes make the forced 96-page
  build completely clean: zero missing glyphs, overfull or underfull boxes,
  duplicate destinations, Hyperref warnings, LaTeX warnings, and font warnings.
- All 96 rendered pages were inspected again after cleanup. The 33 top-level
  chapter bookmarks, 93 total outline entries, metadata, and generator-leak
  checks remain intact, with no clipping, collision, or accidental blank page.
- HL-B26 is next: publish Malayalam Chapters 6–31 from canonical lessons before
  addressing that expanded book's bounded warning cleanup in HL-B27.

## Findings from HL-B26

- All thirty-three canonical Malayalam lessons after Chapter 5 now use schema
  v2 with explicit spine nodes, prerequisite-safe sequences, honest
  sub-five-minute duration budgets, typed knowledge boundaries, skills, modes,
  strands, register, and variety metadata. The first thirty-one lessons remain
  schema v1, so the track is intentionally mixed while migration proceeds
  incrementally.
- Twenty-six generated chapters carry deterministic hashes and lesson ids that
  Language Ladder independently reproduces from the canonical AST. Malayalam
  book coverage is now 100%, and the app and downloadable book share the same
  source through Chapter 31.
- A reusable Malayalam comparison-font set renders Malayalam, Tamil, Telugu,
  Kannada, Devanagari, and Arabic-script examples without hand-authored LaTeX.
  Source-normalized chillus and IAST plus an explicit labialization fallback
  leave the expanded book with zero missing glyphs.
- A forced XeLaTeX build produces 107 pages with 33 top-level and 97 total
  outline entries, correct title and author metadata, and no leaked schema or
  generator directives. All 107 rendered pages were inspected; no teaching
  content is clipped, colliding, or accidentally omitted.
- The expanded warning baseline is 17 overfull boxes, four underfull horizontal
  boxes, ten underfull vertical boxes, four duplicate practice labels, 108
  Hyperref warnings, and seven font warnings. Several expected open-right verso
  pages still carry running headers. HL-B27 records both cleanup targets against
  the complete artifact.
- The corpus report remains at zero duration violations and zero unknown
  prerequisites across 1,066 lessons. It now reports 129 lesson chapters without
  book chapters, 26 fewer than before this migration.
- The unified publication gate builds and catalogs all twenty books in one job
  (270.4 seconds locally), while the data package passes 84 tests and Language
  Ladder passes 411 tests plus its production build.
- HL-B27 follows by making the complete Malayalam artifact warning-free before
  Arabic's larger canonical-book migration in HL-B28.

## Findings from HL-B27

- Explicit static bold and italic faces now cover Malayalam and all five
  comparison scripts, while bookmark-safe Unicode commands preserve readable
  outlines without asking Hyperref to interpret font switches.
- The five handwritten recap labels are unique. Concise running titles and
  narrow copy-flow edits in Chapters 1–3, 12, 20, and 22 remove every remaining
  horizontal overflow without dropping or weakening teaching content.
- Intentionally short micro-lessons use natural page bottoms, and open-right
  chapter breaks now insert genuinely empty versos with no running header or
  page number.
- A forced XeLaTeX build produces 107 pages with zero missing glyphs, overfull
  or underfull boxes, duplicate destinations, Hyperref warnings, LaTeX
  warnings, or font warnings. All 107 rendered pages were inspected again.
- The correct title and author metadata, 33 top-level and 97 total outline
  entries, generated source hashes, and zero schema or generator leaks remain
  intact.
- HL-B28 is next: publish Arabic Chapters 3–27 and the dependency-ordered
  writing companions from the canonical app corpus.

## Findings from HL-B28

- All forty-five canonical Arabic lessons in Chapters 3–27, including six
  dependency-ordered writing companions, now use schema v2 with explicit spine
  nodes, prerequisite-safe sequences, honest sub-five-minute duration budgets,
  typed knowledge boundaries, skills, modes, strands, register, and variety
  metadata. Chapters 1–2 remain intentionally hand-authored so their existing
  inline script introduction stays intact while migration proceeds
  incrementally.
- Twenty-five generated chapters carry deterministic hashes and lesson ids that
  Language Ladder independently reproduces from the canonical AST. Arabic book
  coverage is now 100%, and the app and downloadable book share one source
  through Chapter 27.
- Reusable Arabic and Hebrew script mappings render the Semitic comparisons
  without hand-authored LaTeX. The vendored static fonts leave the expanded
  artifact with zero missing glyphs.
- A forced XeLaTeX build produces 104 pages with 29 top-level and 90 total
  outline entries, correct title and author metadata, and no leaked schema or
  generator directives. All 104 rendered pages were inspected; no teaching
  content is clipped, colliding, or accidentally omitted.
- The expanded warning baseline is five overfull boxes, ten underfull vertical
  boxes, one duplicate practice label, 77 Hyperref warnings, two LaTeX warnings,
  and six font warnings. Several expected open-right verso pages still carry
  running headers. HL-B29 records both cleanup targets against the complete
  artifact.
- The corpus report remains at zero duration violations and zero unknown
  prerequisites across 1,066 lessons. It now reports 104 lesson chapters without
  book chapters, 25 fewer than before this migration.
- HL-B29 follows by making the complete Arabic artifact warning-free before
  Hindi's larger canonical-book migration in HL-B30.

## Findings from HL-B29

- Explicit static bold and italic faces now cover Arabic and Hebrew, while
  bookmark-safe Unicode commands preserve readable outlines without asking
  Hyperref to interpret font switches.
- The two handwritten recap labels are unique. A small emergency line-break
  reserve removes all five horizontal overflows without dropping or weakening
  teaching content.
- Intentionally short micro-lessons use natural page bottoms, and open-right
  chapter breaks now insert genuinely empty versos with no running header or
  page number.
- A forced XeLaTeX build produces 104 pages with zero missing glyphs, overfull
  or underfull boxes, duplicate destinations, Hyperref warnings, LaTeX
  warnings, or font warnings. All 104 rendered pages were inspected again.
- The correct title and author metadata, 29 top-level and 90 total outline
  entries, generated source hashes, and zero schema or generator leaks remain
  intact.
- HL-B30 is next: publish Hindi Chapters 6–33 and the dependency-ordered
  writing companions from the canonical app corpus.

## Findings from HL-B30

- Fifty-one Hindi lessons now use schema v2 with explicit shared-spine nodes,
  unique topological sequence numbers, honest sub-five-minute budgets, typed
  teaching blocks, and prerequisite knowledge boundaries. The set comprises
  forty lessons across Chapters 6–33 plus eleven dependency-ordered writing
  companions already placed inside Chapters 1–2.
- Twenty-eight generated chapters now carry canonical source hashes into the
  book. Language Ladder independently rebuilds and verifies every Chapter
  6–33 hash, so the browser and downloadable book consume the same lesson AST
  instead of parallel copies.
- The existing hand-authored opening remains intact: its writing companions
  gently introduce the headline, inherent vowel, mātrās, preposed short *i*,
  spineless letters, virama, conjuncts, and whole-word assembly
  exactly where the learner first needs them.
- Reusable Devanagari, Arabic, and Cyrillic font mappings preserve Hindi's
  Sanskrit, Perso-Arabic, and cross-language etymology comparisons. The shared
  renderer now emits stable LaTeX for stacked accents, PIE subscripts and
  superscripts, and comparison symbols; the forced PDF build has zero missing
  glyphs.
- The corpus remains at 1,066 lessons with zero duration violations and zero
  unknown prerequisites. Missing lesson chapters in books fall from 104 to 76.
- The expanded PDF builds successfully at 114 pages. Its remaining measured
  warning baseline is nine overfull boxes, one underfull line, five underfull
  pages, three duplicate practice labels, 108 Hyperref warnings, and seven
  font-shape warnings. Physical PDF pages 20, 40, 48, 52, 60, 74, 78, 82, 86,
  90, 94, and 112 are open-right versos containing only running headers and
  page numbers; pages 2 and 4 are the same front-matter pattern. HL-B31 is next
  and owns that cleanup plus the complete rendered-page audit.

## Findings from HL-B31

- Explicit static bold and italic faces now cover Devanagari, Arabic, and
  Cyrillic without changing glyph sources between local and CI builds.
- Bookmark-safe script commands keep readable Hindi, Arabic, and Russian text
  in the outline while preventing Hyperref from interpreting presentation-only
  font switches.
- Four hand-authored practice labels are unique. Natural page bottoms and a
  small emergency line-break reserve remove layout warnings without deleting
  teaching content; the one long Chapter 5 running title has a concise short
  form.
- The twelve open-right chapter versos and two front-matter versos remain in
  the print-friendly layout but are now genuinely empty: no running header and
  no page number.
- A forced XeLaTeX build produces 114 pages with zero missing glyphs, overfull
  or underfull boxes, duplicate destinations, Hyperref warnings, LaTeX
  warnings, or font warnings. All 114 rendered pages were inspected again.
- Correct title/author metadata, 35 top-level and 107 total outline entries,
  generated source hashes, and zero schema or generator leaks remain intact.
- TeX Live places two additional chapter transitions on blank even pages that
  MiKTeX does not need. The production artifact therefore has sixteen blank
  pages versus fourteen locally; both layouts remain 114 pages and preserve
  all content, and the two platform-specific versos were visually confirmed
  empty.
- HL-B32 is next: publish Tamil Chapters 6–31 and its dependency-ordered
  writing companions from the canonical app corpus.

## Findings from HL-B32

- Fifty-one Tamil lessons now use the strict schema-v2 contract: eight inline
  writing steps followed by forty-three content micro-lessons through Chapter
  31. Sequences 100–600 are unique and prerequisite-safe, and every typed body
  block declares the knowledge it introduces or assesses.
- The complete Tamil slice remains below five minutes. Effective durations
  range up to 299 seconds; the retroflex writing step and dative-subject lesson
  remain the intentionally watched boundary cases rather than losing their
  script, grammar, or etymology depth.
- Twenty-six generated chapters replace twenty-six potential hand-maintained
  copies. The committed source manifest lets Language Ladder independently
  reproduce every lesson id and canonical hash used by the PDF; repository-wide
  lesson chapters without a book chapter fall from 76 to 50.
- A forced XeLaTeX build expands the book from 29 to 117 pages with zero
  missing glyphs, correct title/author metadata, 33 top-level and 106 total
  outline entries, and zero schema or generator leaks.
- The expanded book exposes 30 overfull lines, five underfull lines, eleven
  underfull pages, four duplicate practice labels, 146 Hyperref warnings, and
  19 font warnings. HL-B33 is next and owns the full rendered-page and blank-
  verso cleanup instead of mixing publication hygiene into canonical content.
- The roadmap and authoritative session map still stop before the complete
  Chapter 31 corpus. HL-M07 continues to own that progression-metadata work.

## Findings from HL-B33

- Explicit static-font shape mappings remove all nineteen Tamil and comparison-
  script substitutions without introducing a system-font dependency.
- PDF-safe definitions preserve Tamil text in the outline while eliminating
  all 146 Hyperref warnings. Five unique practice labels remove the four
  duplicate destinations.
- Flexible page bottoms and a true-empty `\cleardoublepage` keep the open-right
  print layout while removing all eleven underfull-page warnings and every
  header-only verso.
- Shorter canonical headings, a two-column weekday comparison, and a scannable
  recall checklist remove the remaining line warnings in both book and app
  content. Regeneration keeps their lesson ids and source hashes independently
  verifiable instead of creating book-only copies.
- The forced XeLaTeX build remains 117 pages with correct title/author metadata,
  106 outline entries, zero schema leaks, and zero missing glyphs, overfull or
  underfull boxes, duplicate labels, Hyperref warnings, LaTeX warnings, or font
  warnings. All rendered pages were inspected again.
- HL-B34 is next: publish Latin Chapters 2–36 from the canonical app corpus.

## Findings from HL-B34

- All 53 Latin lessons now use schema v2. Sequences are unique and
  prerequisite-safe; every lesson has a shared-spine placement, explicit
  knowledge boundaries, stable typed blocks, and an effective duration below
  five minutes. Latin becomes the first all-v2 track in the twenty-language
  corpus.
- Thirty-five generation targets turn canonical Chapters 2–36 into book-ready
  LaTeX while retaining each lesson id and a deterministic source hash. The
  generated-chapter check independently guards the content Language Ladder and
  the book share instead of maintaining a second copy.
- The curriculum report remains at 1,066 lessons, 20 tracks, 20 books, zero
  duration violations, and zero unknown prerequisites. Publishing these
  chapters reduces the lesson-to-book chapter gap from 50 to 15.
- A forced XeLaTeX build expands the Latin volume from 12 to 113 pages with
  correct title/author metadata, 38 top-level and 91 total outline entries,
  zero missing glyphs, zero duplicate destinations, zero Hyperref warnings,
  and no schema metadata leaks.
- The expanded book exposes six overfull lines, six underfull lines, eight
  underfull header-only versos, and one unavailable small-caps shape (reported
  in two font warning blocks). HL-B35 owns that focused layout, font, and verso
  cleanup.
- The roadmap and authoritative session map still stop at Chapter 1. HL-M08
  continues to own the progression-metadata reconciliation through Chapter 36.
- HL-B35 is next: remove Latin's remaining layout and font warnings.

## Findings from HL-B35

- Selecting Latin Modern Caps explicitly supplies the book's small-caps shape
  without adding a system-font or vendored-font dependency.
- Natural page bottoms, a two-em emergency stretch, and compact numeric section
  marks remove every overfull and underfull line while keeping generated prose
  readable.
- A true-empty `\cleardoublepage` preserves open-right chapter starts while
  leaving intentionally blank versos free of running heads and page numbers.
- Three dense canonical recall paragraphs now use scannable bullet lists. Their
  generated Chapters 16, 17, and 20 retain independently checked lesson ids and
  source hashes, so the readability improvement reaches both app and book from
  the same source.
- The forced 115-page XeLaTeX build has correct title and author metadata, 38
  top-level and 91 total outline entries, zero schema leaks, and zero missing
  glyphs, overfull or underfull boxes, duplicate destinations, Hyperref
  warnings, LaTeX warnings, or font warnings. Every rendered page was visually
  inspected for clipping, collisions, broken tables, and malformed callouts.
- The original cleanup in #9885 exposed a cross-platform font-name mismatch in
  the unified books job after auto-merge had already completed. #9887 replaced
  the MiKTeX-specific font filename with the portable family name and verified
  the exact `main` artifact in production. HL-P01 records the missing protected
  books gate so a publication failure cannot lose that race again.
- HL-B36 is next: publish Spanish Chapters 19–33 from the canonical app corpus.

## Findings from HL-B36

- All 21 Spanish lessons in Chapters 19–33 now use schema v2, with unique
  sequences, shared-spine placement, explicit prerequisite and knowledge
  boundaries, stable typed blocks, and effective durations below five minutes.
  The *mano* and *agua / vino* lessons now explicitly require the grammatical-
  gender concept they already assumed.
- Fifteen generation targets turn the canonical later lessons into book-ready
  LaTeX while retaining each lesson id and deterministic source hash. The
  generated-chapter check independently guards the same AST loaded by Language
  Ladder instead of maintaining a second content copy.
- The curriculum report remains at 1,066 lessons, 20 tracks, 20 books, zero
  duration violations, and zero unknown prerequisites. Publishing Chapters
  19–33 reduces the lesson-to-book chapter gap from 15 to zero.
- Spanish book generation now supports an inline Arabic script command backed
  by the repository's static Naskh font. Chapter 22 preserves **لازورد** with
  correct right-to-left shaping and no missing glyphs on a clean machine.
- A forced XeLaTeX build expands the Spanish volume to 210 pages with correct
  title and author metadata, 35 top-level and 155 total outline entries, zero
  schema leaks, zero missing glyphs, and zero duplicate destinations. Every
  rendered page was inspected for clipping, collisions, broken tables, and
  malformed callouts.
- The expanded warning baseline is 51 overfull hboxes, 3 underfull hboxes, 19
  underfull vboxes, 14 Hyperref warnings, 2 font-warning matches, and zero
  generic LaTeX warnings. These are concentrated in the legacy layout and are
  owned by HL-B37 rather than hidden by this publication tranche.
- HL-B37 is next: remove Spanish's remaining legacy print warnings and
  header-only chapter versos.

## Findings from HL-B37

- Portable Latin Modern small caps remove the final font fallback on both
  MiKTeX and TeX Live. Natural page bottoms, a two-em emergency stretch,
  compact numeric running heads, and a true-empty `\cleardoublepage` remove
  legacy line, page, and header-only-verso warnings without hiding them.
- Fixed-width legacy grammar tables now use width-aware, ragged-right columns.
  Dense conjugation, tense, mood, and etymology comparisons wrap within the
  printed measure instead of protruding into the margin.
- Plain-text bookmark alternatives preserve the visible vowel-change notation
  while removing all fourteen math-token warnings from the PDF outline.
- The dense Chapter 21 weekday recall is now a scannable canonical bullet list.
  Its generated chapter retains the independently checked lesson id and source
  hash, so the readability improvement reaches both Language Ladder and the
  book from the same source.
- The forced 214-page XeLaTeX build has correct title and author metadata, 35
  top-level and 155 total outline entries, zero schema leaks, and zero missing
  glyphs, overfull or underfull boxes, duplicate destinations, Hyperref
  warnings, LaTeX warnings, or font warnings. All 214 rendered pages were
  inspected for clipping, collisions, broken tables, malformed callouts, and
  Arabic shaping; all 19 intentionally blank physical pages are truly empty.
- That audit led to HL-P01, which made the unified books result a protected
  merge gate that auto-merge cannot outrun.

## Findings from HL-P01

- A path-filtered workflow cannot be required globally: pull requests outside
  those paths would never receive its status context. The workflow now starts
  on every pull request, performs a low-cost path decision, and runs the
  existing single all-books build only when book inputs changed.
- `Human Languages Books gate` is stable and always present on pull requests.
  It fails when detection fails, when a relevant all-books build does not pass,
  or when the build/result combination is inconsistent. A legitimately skipped
  irrelevant build is the only fast-path success.
- Main pushes and manual runs publish `Human Languages Books push gate`, so a
  push result cannot satisfy the protected pull-request context while its book
  build is still in progress.
- Pull request #9893 and the exact merged `main` revision both passed the full
  20-book job. Repository protection now requires both `CI gate` and the
  pull-request-only `Human Languages Books gate`; the exact-main bundle is live
  in the public catalog.
- HL-M01 follows by making the shared/local curriculum relationship executable.

## Findings from HL-M01

- All 20 registered tracks now have an explicit `curriculum.json`: 346 ordered
  path segments map 896 lessons and attach 247 required, supporting, reference,
  or not-applicable extension nodes. All 11 current spine nodes are present in
  every map, including planned/empty ledgers.
- A shared node may recur in a track's path. Contracting each node to one
  contiguous occurrence creates false cycles because real curricula revisit
  greetings, time, definiteness, and other abilities after intervening grammar
  or script work.
- Validation proves canonical and schema-v2 lesson coverage, recursive
  prerequisite closure and topological order, exact extension attachment, and
  explicit omissions. Persian and Urdu each place a required script-entry
  extension inline with the first greeting lesson.
- Spanish, Kannada, Latin, Malayalam, Tamil, and Telugu intentionally teach
  `GREETING-GOODNIGHT` under `SPINE-TIME-OF-DAY` even though the canonical
  concept belongs to `SPINE-TAKE-LEAVE`; those six relocations are now data,
  not exceptions hidden in consumer code.
- The pure planner returns one safe local frontier per selected language and
  groups only frontiers currently ready at the same shared node. HL-M10 follows
  by moving the visible Learn flow and review eligibility onto those frontiers.

## Findings from HL-M10

- Learn no longer has one global concept index or a jump control that can expose
  a late local realization. Every selected language contributes exactly its
  first incomplete mapped lesson, so Persian can advance while Urdu remains at
  its own greeting frontier.
- Progress persists by stable lesson id per language. On every load, saved data
  is reduced to the longest valid local prefix; unknown ids, gaps, and a newly
  inserted prerequisite cannot grant progress past the first missing lesson.
- Lexical lessons require an English-meaning retrieval with the lesson and all
  other language cards hidden. Wrong answers reveal feedback but do not advance.
  Script, grammar, and other support lessons use their authored final recall as
  a self-check until HL-V03 compiles objective typed activity contracts.
- Mixed review contains only independently focused-successful shared lessons.
  It waits for two visually distinct answers, so Persian and Urdu's identical
  `سلام` cannot produce a fake one-option quiz; once another form is unlocked,
  the existing adaptive SRS and confusion log operate on the safe grid.
- Rendered Persian/Urdu QA proved wrong-answer blocking, independent advancement,
  persistence across reload, explicit RTL/script treatment, and delayed mixed
  eligibility with zero browser errors. The app build passes 30 test files and
  478 tests after this tranche.
- HL-V03 is next because objective prompt/answer contracts are the remaining
  prerequisite for replacing non-lexical self-confirmation without scraping
  lesson prose or inventing accepted answers.

## Findings from the Russian HL-A01 slice

- Russian was the only track with non-lexical debt that the first 15-track
  tranche could not cover. Its two frontiers depended on legacy pronoun and
  naming lessons, so attaching activity comments directly would have left their
  knowledge prerequisites unowned.
- The minimal honest migration is six lessons in one closed chain: *я* →
  *ты/вы* → polite *вы* → *меня зовут* → *как вас зовут* → the
  cross-language *how/what* comparison. Stable sequence values, typed block boundaries,
  explicit skill/mode/strand metadata, and transitive knowledge atoms preserve
  the existing prerequisite order and learner prose.
- Objective final-recall contracts now ask for the safest adult form (*вы*) and
  the comparison language that asks *what* (English). Both add eight seconds;
  all six lessons remain below five minutes.
- Measured activity coverage rises from 17 to 19 of 113 mapped non-lexical
  lessons across 16 tracks. The remaining debt falls from 96 to 94, and legacy
  non-lexical debt falls from 18 to 16.

## Findings from the first HL-A01 tranche

- Fifteen schema-v2 tracks with outstanding non-lexical debt now contribute one
  objective final-recall activity apiece. The slice spans Arabic, German,
  Gujarati, Hindi, Italian, Kannada, Latin, Malayalam, Marathi, Portuguese,
  Punjabi, Sanskrit, Spanish, Tamil, and Telugu instead of deepening only the
  original Spanish pilot.
- Every activity asks one exact question already answered by its lesson, assesses
  a containing-block knowledge atom, provides explicit accepted variants and
  feedback, and adds only eight seconds of learner response time. Italian's
  297-second Chapter 2 practice frontier was deliberately left untouched in
  favor of its 237-second Chapter 3 practice lesson, preserving the strict
  sub-five-minute gate.
- Bengali, French, Persian, and Urdu currently have no mapped non-lexical
  self-check debt. Russian's two remaining candidates are both legacy lessons,
  so their activity coverage stays coupled to an honest schema-v2 migration.
  The measured backlog falls from 111 to 96 lessons; all 18 legacy candidates
  remain explicit.
- A clean package install still reports the already-recorded HL-I02 advisory:
  Vitest 4.1.10 reaches PostCSS 8.5.19 through Vite 8.1.5, and the dry-run fix
  resolves it by moving the transitive package to 8.5.25.

## Findings from HL-V03

- A typed block can now carry one or more compact JSON `hl-activity` directives
  immediately after `hl-knowledge`. The canonical AST retains their stable id,
  text-response kind, assessed atoms, prompt, answer, variants, feedback, and
  response budget while book and app learner copy omits the metadata.
- Validation rejects malformed or misplaced JSON, non-lesson-prefixed or
  duplicate ids, empty/out-of-block assessment sets, ambiguous normalized
  variants, missing feedback, and response budgets outside 1–299 seconds.
  Runtime compilation therefore resolves every accepted response once without
  recovering answers from Markdown prose.
- Duration model v2 adds each activity's authored response budget. The first
  grammar and script pilots remain at 180 and 240 effective seconds, the full
  curriculum keeps zero errors and zero duration violations, and regenerated
  Spanish Chapters 1 and 4 retain identical learner prose with refreshed hashes.
- Language Ladder prefers a final-recall activity when present, hides the
  answer-bearing lesson summary during retrieval, shows authored corrective or
  success feedback, and advances only after a correct response plus explicit
  continue. Existing lexical meaning checks remain available.
- The measured follow-up is HL-A01: 113 mapped non-lexical lessons exist, these
  two pilots leave 111 without objective activities, and 18 of those still need
  schema-v2 body contracts before an activity can be attached honestly.

## Findings from HL-D01C

- Gujarati now has zero duration violations. The repository snapshot contains
  982 lessons and 455 violations overall, down from 464 before this tranche;
  unknown prerequisites remain at zero.
- Eight lessons already computed between 110 and 184 seconds and only needed
  honest four-minute declared budgets. The one genuinely long lesson computed
  at 370 seconds.
- That counting lesson is now a 174-second core followed by a 253-second
  etymology lesson. The *dvé → be* inheritance, cross-Indic comparison, and
  restored *r* in *traṇ* remain complete and prerequisite-ordered in the
  canonical corpus consumed by Language Ladder.
- Gujarati Chapter 6 has canonical lessons but is not in the current
  five-chapter PDF. HL-B06 records its one-source migration and generation work
  instead of adding another manual copy.
- A forced build of the unchanged five-chapter book succeeds, but exposes four
  missing punctuation glyphs, one overfull box, four underfull boxes, four
  duplicate practice labels, and 28 Unicode bookmark warnings. HL-B07 records
  that pre-existing publication hygiene debt separately.
- Punjabi and Sanskrit tie for the smallest remaining set at ten violations,
  each with nine declaration-only lessons and one genuine split. Punjabi's long
  lesson computes at 405 seconds versus Sanskrit's 513, so HL-D01D takes Punjabi
  first as the safer bounded tranche.

## Findings from HL-D01D

- Punjabi now has zero duration violations. The repository snapshot contains
  983 lessons and 445 violations overall, down from 455 before this tranche;
  unknown prerequisites remain at zero.
- Nine lessons already computed between 106 and 172 seconds and only needed
  honest four-minute declared budgets. The one genuinely long lesson computed
  at 405 seconds.
- That lesson is now a 229-second counting-and-script core followed by a
  241-second etymology lesson. The Gurmukhi mark distinction, Chapter 5 callback,
  same-source *panjāh/pacās* evidence, and convergence explanation remain
  complete and prerequisite-ordered in the Language Ladder corpus.
- Punjabi Chapter 6 has canonical lessons but is not in the current
  five-chapter PDF. HL-B08 records its one-source migration and generation work
  rather than adding another manual copy.
- A forced build of the unchanged five-chapter book succeeds with no missing
  glyphs, but exposes one overfull box, four underfull boxes, four duplicate
  practice labels, and 28 Unicode bookmark warnings. HL-B09 records that
  pre-existing publication hygiene debt separately.
- Sanskrit's ten violations are now the smallest remaining track-sized set.
  Nine are declaration-only; its 513-second numbers lesson will require a more
  careful split than the three preceding tranches, so HL-D01E is next.

## Findings from HL-D01E

- Sanskrit now has zero duration violations. The repository snapshot contains
  985 lessons and 435 violations overall, down from 445 before this tranche;
  unknown prerequisites remain at zero.
- Nine lessons already computed between 107 and 186 seconds and only needed
  honest four-minute declared budgets. The anchor numbers lesson computed at
  513 seconds and required two new support lessons rather than one.
- Chapter 6 is now a 232-second forms/grammar core, a 240-second east-west
  cognate and sound-law lesson, and a 180-second *pañca* travel lesson. The dual,
  gendered daughter forms, PIE outcomes, Grimm's law, analogy, and qualified
  lexical histories remain complete and prerequisite-ordered in Language Ladder.
- Sanskrit Chapter 6 has canonical lessons but is not in the current
  five-chapter PDF. HL-B10 records its one-source migration and generation work
  rather than adding another manual copy.
- A forced build of the unchanged five-chapter book succeeds with no missing
  glyphs, but exposes three overfull boxes, six underfull boxes, four duplicate
  practice labels, and 28 Unicode bookmark warnings. HL-B11 records that
  pre-existing publication hygiene debt separately.
- Bengali's eleven violations are now the smallest remaining track-sized set.
  All eleven already compute below 300 seconds (maximum 290), so HL-D01F is a
  bounded honest-budget correction with no content split required.

## Findings from HL-D01F

- Bengali now has zero duration violations. The repository snapshot remains at
  985 lessons and drops to 424 violations overall, down from 435 before this
  tranche; unknown prerequisites remain at zero.
- All eleven lessons already computed between 121 and 290 seconds, so only their
  declared estimates changed. No canonical lesson body, prerequisite, book
  source, or app behavior needed rewriting.
- `BN-C06-numbers-1-5` is the tightest corrected lesson at 290 seconds and
  should be watched during later copy edits.
- Bengali Chapter 6 has canonical app-ready content but is not in the current
  five-chapter PDF. HL-B12 records its one-source migration and generation work.
- A forced build of the unchanged book succeeds, but exposes six missing glyphs,
  one overfull box, four underfull boxes, four duplicate practice labels, and 27
  Unicode bookmark warnings. HL-B13 records that pre-existing hygiene debt.
- Italian's twenty violations are now the smallest remaining track-sized set.
  Seventeen are declaration-only and three are genuinely computed, with a
  404-second maximum, so HL-D01G is next.

## Findings from HL-D01G

- Italian now has zero duration violations. The repository grows from 985 to
  989 lessons and drops from 424 to 404 violations overall; unknown
  prerequisites remain at zero.
- Seventeen lessons needed only honest declared-budget corrections. The three
  computed violations were replaced by prerequisite-ordered steps: informal
  `Come stai?` → formal `Come sta?` → register-neutral `Come va?`; `essere`
  forms → the borrowed `stato` story → `andare` → participle agreement.
- The first attempted combined `Come sta? / Come va?` extension still measured
  325 seconds. Splitting register from metaphor produced two independent
  micro-lessons without deleting the cross-language or etymological depth.
- `IT-C02-practice` at 297 computed seconds and `IT-C17-mano` at 296 are the
  tightest remaining Italian lessons and should be watched during copy edits.
- The Italian PDF builds successfully at 13 pages but contains only Chapter 1,
  while canonical app lessons run through Chapter 17. HL-B14 records the
  schema-v2 migration and generated publication work for Chapters 2–17.
- That forced build reports no missing glyphs, overfull boxes, or duplicate
  labels, but does expose one underfull box and three Unicode bookmark warnings.
  HL-B15 records the pre-existing clean-build debt.
- Portuguese's twenty-three violations are now the smallest remaining set.
  Eighteen are declaration-only and five genuinely compute above the limit,
  with a 565-second maximum, so HL-D01H is next.

## Findings from HL-D01H

- Portuguese now has zero duration violations. The corpus grows from 989 to 994
  lessons and drops from 404 to 381 violations overall; unknown prerequisites
  remain at zero.
- Eighteen lessons needed only honest declared-budget corrections. Five new
  prerequisite-ordered lessons preserve all of the longer content: verb-free
  `Tudo bem?` → `Como vai? / Como está?` → casual practice → formal practice;
  `ser` forms → its two-verb, three-stem history → the core `ser/estar` choice →
  adjective meaning shifts; `cabeça` pronunciation → the `caput` doublet map.
- The new and rewritten lessons compute between 143 and 236 seconds.
  `PT-C17-mao` is the tightest remaining Portuguese lesson at 293 seconds and
  should be watched during later copy edits.
- The Portuguese PDF builds successfully at 13 pages but contains only Chapter
  1 while canonical lessons run through Chapter 17. HL-B16 records the
  schema-v2 migration and generated publication work for Chapters 2–17.
- The build has no missing glyphs, overfull boxes, duplicate labels, or Hyperref
  warnings, but reports three underfull boxes. HL-B17 records that pre-existing
  clean-build debt.
- French's twenty-five violations are now the smallest remaining set.
  Twenty-two are declaration-only and three genuinely compute above the limit,
  with a 489-second maximum, so HL-D01I is next.

## Findings from HL-D01I

- French now has zero duration violations. The corpus grows from 994 to 997
  lessons and drops from 381 to 356 violations overall; unknown prerequisites
  remain at zero.
- Twenty-two lessons needed only honest declared-budget corrections. Three new
  prerequisite-ordered lessons preserve the longer content: neutral `Ça va?` →
  explicit `tu/vous` register and liaison; `être` forms → its `es-/fu-/ét-`
  roots; motion/change agreement → pronominal direct-object agreement.
- The new and rewritten lessons compute between 147 and 244 seconds.
  `FR-C03-practice` at 293 seconds and `FR-C15-passe-simple` at 291 are the
  tightest remaining French lessons and should be watched during copy edits.
- The French PDF builds successfully at 79 pages through Chapter 16 while
  canonical lessons continue through Chapter 23. HL-B18 records the schema-v2
  migration and generated publication work for Chapters 17–23.
- The build has no missing glyphs or duplicate labels, but reports sixteen
  overfull boxes, nine underfull boxes, and six Hyperref warnings. HL-B19 records
  that pre-existing clean-build debt.
- German's twenty-seven violations are now the smallest remaining set.
  Twenty-two are declaration-only and five genuinely compute above the limit,
  with a 360-second maximum, so HL-D01J is next.

## Findings from HL-D01J

- German now has zero duration violations. The corpus grows from 997 to 1,002
  lessons and drops from 356 to 329 violations overall; unknown prerequisites
  remain at zero.
- Twenty-two lessons needed only honest declared-budget corrections. Five new
  prerequisite-ordered lessons preserve the longer content: informal wellbeing
  language → formal *Ihnen* register → casual practice → formal practice;
  Präteritum forms → north/south areal history; *sein*-perfect auxiliaries → the
  French/German agreement contrast; *Kopf* as cup → inherited *Haupt* and the
  Grimm's-law/container comparison.
- The new and rewritten lessons compute between 147 and 244 seconds.
  `GE-C16-sein` at 287 seconds and `GE-W03-capitalization` at 285 are the
  tightest remaining German lessons and should be watched during copy edits.
- The German PDF builds successfully at 84 pages through Chapter 16 while
  canonical lessons continue through Chapter 23. HL-B20 records the schema-v2
  migration and generated publication work for Chapters 17–23.
- The build has no missing glyphs or duplicate labels, but reports seventeen
  overfull boxes, eleven underfull boxes, and three Hyperref warnings. HL-B21
  records that pre-existing clean-build debt.
- Telugu's thirty-six violations are now the smallest remaining set.
  Thirty-five are declaration-only and one genuinely computes above the limit,
  with a 360-second maximum, so HL-D01K is next.

## Findings from HL-D01K

- Telugu now has zero duration violations. The corpus grows from 1,002 to 1,003
  lessons and drops from 329 to 293 violations overall; unknown prerequisites
  remain at zero.
- Thirty-five lessons needed only honest declared-budget corrections. The one
  genuinely long lesson is now a prerequisite-ordered pair: build
  **శుభ మధ్యాహ్నం** from the widened “noon” word → distinguish the two-source
  formal-register claim from the one-source lower-frequency claim.
- The two Chapter 31 steps compute to 152 and 193 seconds.
  `TE-C06-dative-subject` at 285 seconds and `TE-C29-subhodayam` at 279 are the
  tightest remaining Telugu lessons and should be watched during copy edits.
- The Telugu PDF builds successfully at 29 pages through Chapter 5 while
  canonical lessons continue through Chapter 31. HL-B22 records the schema-v2
  migration and generated publication work for Chapters 6–31.
- The build has no missing glyphs, but reports four overfull boxes, three
  underfull boxes, four duplicate practice labels, 27 Hyperref warnings, and a
  font-shape substitution warning. HL-B23 records that pre-existing clean-build
  debt.
- The roadmap narrative stops at Chapter 6 and the authoritative session map at
  Chapter 5. HL-M02 records the progression-metadata work through Chapter 31.
- Kannada's thirty-seven violations are now the smallest remaining set.
  Thirty-six are declaration-only and one genuinely computes above the limit,
  with a 360-second maximum, so HL-D01L is next.

## Findings from HL-D01L

- Kannada now has zero duration violations. The corpus grows from 1,003 to
  1,004 lessons and drops from 293 to 256 violations overall; unknown
  prerequisites remain at zero.
- Thirty-six lessons needed only honest declared-budget corrections. The one
  genuinely long lesson is now a prerequisite-ordered sequence: **-ಗೆ/-ಿಗೆ/-ಕ್ಕೆ**
  forms and *k → g* family history → visible Dravidian stacking versus fused
  Latin endings → the existing dative-subject application.
- The rewritten suffix lesson computes to 205 seconds, the new stacking lesson
  to 196, and the following dative-subject application to 281.
  `KA-C01-namaskara` at 295 seconds and `KA-C22-hasiru-haladi` at 294 are the
  tightest remaining Kannada lessons and should be watched during copy edits.
- The Kannada PDF builds successfully at 29 pages through Chapter 5 while
  canonical lessons continue through Chapter 31. HL-B24 records the schema-v2
  migration and generated publication work for Chapters 6–31.
- The build has no missing glyphs, but reports four overfull boxes, five
  underfull boxes, four duplicate practice labels, 30 Hyperref warnings, and
  undefined bold/italic Kannada font shapes. HL-B25 records that pre-existing
  clean-build debt.
- The roadmap narrative stops at Chapter 6 and the authoritative session map at
  Chapter 5. HL-M03 records the progression-metadata work through Chapter 31.
- Malayalam's thirty-seven violations are now the smallest remaining set.
  Thirty-three are declaration-only and four genuinely compute above the limit,
  with a 360-second maximum, so HL-D01M is next.

## Findings from HL-D01M

- Malayalam now has zero duration violations. The corpus grows from 1,004 to
  1,008 lessons and drops from 256 to 219 violations overall; unknown
  prerequisites remain at zero.
- Thirty-three lessons needed only honest declared-budget corrections. Four
  genuinely long lessons become prerequisite-ordered pairs: **ഉച്ച** “peak” noon
  → **പാതിരാ** half-night; Sanskrit *divasam/dinam* → surviving native **നാൾ**;
  Sanskrit **രാത്രി** and its PIE history → native *iravŭ/iruḷ* register split;
  formal **ശുഭ മധ്യാഹ്നം** → the Malayalam/Kannada/Telugu convergence map.
- The eight new or rewritten steps compute between 141 and 235 seconds.
  `ML-C26-raavile` at 299 seconds and `ML-C06-dative-subject` at 294 are the
  tightest remaining Malayalam lessons and should be watched during copy edits.
- The Malayalam PDF builds successfully at 31 pages through Chapter 5 while
  canonical lessons continue through Chapter 31. HL-B26 records the schema-v2
  migration and generated publication work for Chapters 6–31.
- The build has no missing glyphs, but reports seven overfull boxes, eight
  underfull boxes, four duplicate practice labels, 28 Hyperref warnings, and
  undefined bold/italic Malayalam font shapes. HL-B27 records that pre-existing
  clean-build debt.
- The roadmap narrative stops at Chapter 6 and the authoritative session map at
  Chapter 5. HL-M04 records the progression-metadata work through Chapter 31.
- Arabic's thirty-nine violations are now the smallest remaining set.
  Thirty-five are declaration-only and four genuinely compute above the limit,
  with a 360-second maximum, so HL-D01N is next.

## Findings from HL-D01N

- Arabic now has zero duration violations. The corpus grows from 1,008 to 1,012
  lessons and drops from 219 to 180 violations overall; unknown prerequisites
  remain at zero.
- Thirty-five lessons already computed below five minutes and needed only
  honest four-minute declared budgets. Four longer writing lessons are now
  prerequisite-ordered pairs: direction and *alif* → hidden short vowels;
  positional shapes → **سل/لا** joining; the dot family → writing **سلام**; and
  short-vowel marks → hamza.
- The eight new or rewritten writing steps compute between 135 and 279 seconds.
  `AR-C16-al-saa` at 299 seconds and `AR-C14-ashhur` at 298 are the tightest
  remaining Arabic lessons and should be watched during later copy edits.
- The Arabic PDF builds successfully at 18 pages with no missing glyphs, but it
  contains only Chapters 1–2 while canonical lessons continue through Chapter
  27 alongside sixteen writing steps. HL-B28 records the schema-v2 migration
  and generated publication work for the missing content.
- The build reports one overfull box, four underfull boxes, one duplicate
  practice label, 14 Hyperref warnings, and undefined bold/italic Arabic font
  shapes. HL-B29 records that pre-existing clean-build debt.
- The roadmap details only Chapters 1–4 and still labels Chapter 5+ as planned;
  the authoritative session map stops at Chapter 2. HL-M05 records the
  progression-metadata reconciliation through Chapter 27 and the expanded
  writing sequence.
- Hindi's forty violations are now the smallest remaining set. Twenty-nine are
  declaration-only and eleven genuinely compute above the limit, with a
  501-second maximum, so HL-D01O is next.

## Findings from HL-D01O

- Hindi now has zero duration violations. The corpus grows from 1,012 to 1,025
  lessons and drops from 180 to 140 violations overall; unknown prerequisites
  remain at zero.
- Twenty-nine lessons already computed below five minutes and needed only
  honest four-minute declared budgets. Eleven genuinely long lessons become
  thirteen new prerequisite-ordered steps: six script companions, two history
  supports for one-to-five, and focused lessons for age grammar, later-number
  sound changes, cat history, yellow-word evidence, and evening register.
- The 24 new or rewritten steps compute between 114 and 293 seconds.
  `HI-W02-abugida-ka-ta` at 293 seconds and `HI-W04-ra-sa-mera-naam` at 278 are
  the tightest remaining Hindi lessons and should be watched during copy edits.
- The Hindi PDF builds successfully at 29 pages with no missing glyphs, but it
  contains only Chapters 1–5 while canonical lessons continue through Chapter
  33 alongside eleven writing steps. HL-B30 records the schema-v2 migration and
  generated publication work for the missing content.
- The build reports two overfull boxes, five underfull boxes, three duplicate
  practice labels, 29 Hyperref warnings, and undefined bold/italic Devanagari
  font shapes. Visual inspection also finds the final running header colliding
  with the page number. HL-B31 records that pre-existing clean-build debt.
- The roadmap describes only Chapters 1–6 and still labels Chapter 6 as planned;
  the authoritative session map stops at Chapter 5. HL-M06 records the
  progression-metadata reconciliation through Chapter 33 and the expanded
  writing sequence.
- Tamil's forty-two violations are now the smallest remaining set. Twenty-two
  are declaration-only and twenty genuinely compute above the limit, with a
  441-second maximum, so HL-D01P is next.

## Findings from HL-D01P

- Tamil now has zero duration violations. The corpus grows from 1,025 to 1,045
  lessons and drops from 140 to 98 violations overall; unknown prerequisites
  remain at zero.
- Twenty-two lessons already computed between 107 and 285 seconds and needed
  only honest four-minute declared budgets. Twenty genuinely long lessons are
  now prerequisite-ordered pairs, adding focused script, etymology, grammar,
  register, family-comparison, and source-evidence steps without discarding the
  original depth.
- The forty rewritten or new split steps compute between 127 and 296 seconds.
  `TA-W02-ma-retroflex-na` at 296 seconds and `TA-C06-dative-subject` at 294 are
  the tightest remaining Tamil lessons and should be watched during copy edits.
- The Tamil PDF builds successfully at 29 pages with no missing glyphs, but it
  contains only Chapters 1–5 while canonical lessons continue through Chapter
  31 alongside eight writing steps. HL-B32 records the schema-v2 migration and
  generated publication work for the missing content.
- The build reports six overfull boxes, six underfull boxes, four duplicate
  practice labels, 27 Hyperref warnings, and undefined bold/italic Tamil font
  shapes. Visual inspection of the cover, middle, and final pages finds no
  additional clipping or collision. HL-B33 records that pre-existing
  clean-build debt.
- The roadmap details only Chapters 1–6 and still labels Chapter 7+ as planned;
  the authoritative session map stops at Chapter 5. HL-M07 records the
  progression-metadata reconciliation through Chapter 31 and the expanded
  writing sequence.
- Latin's forty-three violations are now the smallest remaining set.
  Thirty-seven are declaration-only and six genuinely compute above the limit,
  with a 370-second maximum, so HL-D01Q is next.

## Findings from HL-D01Q

- Latin now has zero duration violations. The corpus grows from 1,045 to 1,051
  lessons and drops from 98 to 55 violations overall; unknown prerequisites
  remain at zero.
- Thirty-seven lessons already computed between 143 and 297 seconds and needed
  only honest four-minute declared budgets. Six genuinely long lessons become
  prerequisite-ordered pairs: weather-word history → impersonal weather verbs;
  wellbeing questions → the `valeō/valē` family; dative possession → authorial
  name-case variation; the Plautine meeting phrase → its usage limits;
  `vesper`/west → Greek and Romance afterlives; and the missing afternoon
  formula → time-independent `salvē`.
- The twelve rewritten or new split steps compute between 153 and 295 seconds.
  `LA-C19-quid-agis` at 295 seconds and `LA-C17-canis-feles-cattus` at 297 are
  the tightest remaining Latin lessons and should be watched during copy edits.
- The Latin PDF builds successfully at 12 pages with no missing glyphs,
  overfull boxes, duplicate labels, or Hyperref warnings, but it contains only
  Chapter 1 while canonical lessons continue through Chapter 36. HL-B34 records
  the schema-v2 migration and generated publication work for the missing
  content.
- The build reports one underfull box and a small-caps font-shape substitution.
  Visual inspection of the cover, reference page, and final page finds no
  clipping or collision. HL-B35 records that pre-existing clean-build debt.
- The roadmap and authoritative session map stop at Chapter 1 and still call
  Chapter 2+ planned. HL-M08 records the progression-metadata reconciliation
  through Chapter 36.
- Spanish's fifty-five violations are the only duration debt left in the
  corpus. Forty-one are declaration-only and fourteen genuinely compute above
  the limit, led by a 731-second subjunctive lesson; HL-D01R is the final
  duration tranche.

## Findings from HL-D01R

- Spanish now has zero duration violations. The corpus grows from 1,051 to
  1,063 lessons and drops from 55 to zero violations overall; unknown
  prerequisites remain at zero. The integration suite now enforces zero as an
  invariant instead of asserting that migration debt must exist.
- Forty-one declaration-only lessons already computed below the limit and keep
  their bodies unchanged with honest four-minute budgets. The fourteen
  genuinely long lessons become prerequisite-ordered micro-steps or lose only
  duplicated recap prose.
- Twelve new support lessons separate regular subjunctive formation, inherited
  stems, outliers, the mood's name, two-subject clause traps, Arabic
  *ojalá*, form/subject/mood practice, formal/informal register, Arabic
  *hasta* limits, future conjecture, diacritic accents, and punctuation span.
  The unchanged long-form book narrative still preserves the combined depth.
- All 26 rewritten, added, or borderline-trimmed lessons inspected directly
  compute between 122 and 299 seconds. `ES-C17-practice` at 299 seconds and
  four lessons at 294–295 seconds should be watched during future copy edits.
- The 138-page Spanish PDF builds with no missing glyphs or duplicate labels,
  and visual inspection of its cover, middle, and final pages finds no clipping
  or collision. It stops at Chapter 18 while canonical lessons continue through
  Chapter 33; HL-B36 records the fifteen missing generated chapters.
- Chapters 4–18 remain handwritten rather than source-hash-checked from the
  canonical lesson AST. HL-S02 and HL-G03 form the next migration/generation
  slice for Chapters 4–6 before that approach expands further.
- The build reports 52 overfull boxes, 19 underfull boxes, 14 Hyperref warnings,
  and two font warnings. HL-B37 records that pre-existing clean-build debt.
- The roadmap stops at Chapter 18 and still calls Chapter 19 next, while the
  authoritative session map stops at Chapter 3. HL-M09 records reconciliation
  through canonical Chapter 33 and the new micro-lesson chains.
- With duration debt closed, HL-S02 is next: migrate Spanish Chapters 4–6 to the
  strict one-source schema, then generate the same content for the book and app.

## Findings from HL-T01

- Persian and Urdu each had a valid five-lesson dependency chain and a roadmap,
  but neither had the standard session map or on-demand pronunciation reference.
- Both new maps preserve the exact authored prefix and place every N+1, N+3,
  N+7, and N+15 retrieval through session 20 without inventing future lessons.
- Both references are keyed to the sound ids already declared in lesson
  frontmatter, teach script inside known words, and keep transliteration as
  temporary scaffolding rather than a reading prerequisite.
- The Urdu reference distinguishes Nastaliq as the intended presentation from
  the current vendored Noto Naskh Arabic fallback. HL-U01 remains open; this
  documentation does not silently claim that type-style work is complete.
- The next smallest corpus-growth slice is now explicit as HL-E01: complete the
  shared name exchange in both tracks from one canonical schema-v2 source before
  advancing either language to the wellbeing cluster.

## Findings from HL-E01

- Persian and Urdu each add five prerequisite-safe Chapter 3 micro-lessons:
  address/register, the question word, the complete name question, a meeting
  response, and cumulative objective practice. Each track now has ten mapped
  lessons across three published chapters.
- The two earlier name-statement lessons now use schema v2, so every Chapter 3
  prompt closes over explicitly owned knowledge. All twelve touched lessons
  remain below five minutes, with effective budgets from 210 through 240 seconds.
- Both realization maps add register, script, grammar, culture, and consolidation
  extensions without changing the shared path-segment count. Their session maps
  schedule the new lessons and every N+1, N+3, N+7, and N+15 retrieval through
  session 25 while allowing Chapter 4 to begin at session 11.
- Objective activity coverage rises from 19 of 113 to 21 of 115 mapped
  non-lexical lessons across 18 tracks. The remaining count stays at 94 because
  the two newly mapped practice lessons arrive with activities; 16 legacy
  candidates still require schema-v2 migration first.
- Generated Persian and Urdu Chapter 3 files carry the same canonical lesson
  hashes consumed by Language Ladder. The audit also found that Markdown link
  labels survive generation while their URLs do not; HL-G05 records that
  traceability gap instead of widening this curriculum tranche.
- The shared spine next calls for `SPINE-CHECK-WELLBEING`, while both older
  roadmaps currently plan identity grammar for Chapter 4. HL-E02 therefore
  reconciles that order explicitly rather than silently letting the two tracks
  drift from the shared spine.

## Findings from HL-E02

- Persian and Urdu each add six prerequisite-safe Chapter 4 micro-lessons and
  now reach the same shared wellbeing can-do through sixteen canonical lessons.
  The spine stays shared while the local teaching order differs where grammar
  requires it.
- Persian reuses ezafe in **hâl-e shomâ**, teaches one reliable careful question,
  and introduces only attached first-person **-am** in **khubam**. Colloquial
  contraction is visible for recognition but remains outside assessed production.
- Urdu gives **kaise/kaisī** agreement its own step before honorific
  **āp ... haiṅ**, then separates **maiṅ ... hūṅ** from **ṭhīk**. The Hindi
  bridge appears only after the Urdu form is independently readable and
  retrievable.
- Every new lesson carries an objective activity, a declared sub-five-minute
  budget, and closed knowledge prerequisites. Exact review ledgers extend from
  S25 through S31 at N+1, N+3, N+7, and N+15.
- The exact-main verification after HL-I04 found a repeated external Lua setup
  outage: every matrix shard received `ECONNREFUSED` from the Lua download host
  on both the initial attempt and the failed-job rerun. HL-I05/#9910 added a
  pinned cache and checksum-verified source fallback; the consolidated 20-book
  workflow itself was green at the same revision.

## Findings from HL-E03

- Persian and Urdu each add four prerequisite-safe Chapter 5 micro-lessons:
  **khodâ/khudā**, **hâfez/hāfiz**, the complete farewell, and cumulative
  start-versus-end practice. Both tracks now have twenty mapped lessons across
  five generated-book chapters.
- The shared phrase keeps different local writing contracts: Persian normally
  joins **خداحافظ**, while Urdu keeps **خدا حافظ** spaced. Language Ladder may
  compare them only after each local form has passed its own objective check.
- The etymology ramp is deliberately layered: the Persian history of
  **khodâ/khudā** comes first, the Arabic **ḥ-f-ẓ** guard-and-preserve root comes
  second, and the protective formula is assembled only after both words are
  independently readable.
- Every new lesson carries one compiled activity and remains below five minutes.
  Objective non-lexical coverage rises from 23 of 117 to 25 of 119 while the
  explicit 94-item debt remains unchanged.
- Exact review ledgers preserve all older due items and add N+1, N+3, N+7, and
  N+15 retrieval through S35. Casual, later, soon, tomorrow, and good-night
  forms remain explicit omissions until their own prerequisites are taught.
- The corpus report reaches 1,096 lessons, 20 books, zero duration violations,
  zero unknown prerequisites, and zero lesson-to-book chapter gaps. Both new
  Chapter 5 files and hashes are generated from the same AST loaded by the app.

## Findings from HL-G04

- All 270 generated chapter targets now render paired authored ASCII double
  quotes with explicit opening and closing LaTeX text commands. A corpus audit
  finds 5,631 balanced pairs, zero imbalanced generated files, and zero raw
  ASCII double quotes left in generated chapter prose.
- The pairing pass understands emphasis boundaries and nested quotations while
  deliberately preserving code spans, escaped literal quotes, link
  destinations, existing curly quotes, and genuinely unmatched marks. The
  canonical Markdown consumed by Language Ladder remains unchanged.
- The audit exposed indented continuation lines escaping Markdown blockquotes.
  HL-G06 records and completes the supporting fix: continued learner examples
  now remain in one generated quote/callout and one typography pass.
- A single local pass rebuilt all 20 books with zero LaTeX, package, box,
  missing-glyph, or font-warning matches. Visual checks cover Spanish emphasis,
  nested Arabic/RTL glosses, and a continued Telugu example without clipping.
- HL-G05 remains the next queued one-source book gap: generated link labels are
  readable, but their canonical destinations are not yet live PDF links.

## Findings from HL-G05

- The generator now preserves all 55 canonical links in the nine configured
  chapters that contain them: Spanish Chapters 1–3 and Persian/Urdu Chapters
  3–5. Absolute research citations stay on their authored HTTP(S) targets.
- Relative prerequisite and pronunciation-reference links resolve against the
  lesson's stable GitHub source URL. This keeps links useful after a book is
  downloaded, instead of emitting paths relative to an arbitrary PDF folder.
- Link labels still pass through the same emphasis, script-font, and quotation
  renderer as surrounding prose, while destinations use their own LaTeX-safe
  escaping and remain outside typography transformations.
- Generation fails closed when a relative link has no canonical source base or
  when a destination uses a non-HTTP(S) protocol. All lesson filenames match
  their canonical ids, so no additional source-path metadata is needed.
- The audit found 117 authored links across the wider lesson corpus. The 62 not
  yet represented in generated targets remain canonical app content and will
  become live automatically when those chapters migrate to book generation.

## Findings from HL-C18A

- Spanish had **fifteen** over-budget lessons, not the two the HL-C18 row named.
  All fifteen are now split, into **thirty-three** prerequisite-ordered
  micro-lessons; the corpus grows 1,096 → 1,114 and the over-budget count falls
  52 → 37, with the maximum dropping from 7 to 6.
- Splitting was the fix in every case. No lesson was waived, and no atom list
  was trimmed while the body kept teaching the material — each atom the original
  introduced is still introduced exactly once, by whichever half now owns it.
- Every boundary landed on a seam the language already had, not on an atom
  count. The clearest is `ES-C31-numeros-11-20`: Spanish 11–15 are **fused**
  Latin compounds (*ūndecim* → *once*, with only a worn *-ce* left of the
  "ten"), while 16–19 are **transparent** *dieci-* + digit. That is the
  difference between vocabulary you remember and grammar you generate, so the
  split falls after *quince* — and Latin's own subtractive *duodēvīgintī* /
  *ūndēvīgintī*, which Spanish refused to inherit, earns a lesson of its own.
- Five paired lessons were renamed to single-word ids (`ES-C22-rojo`,
  `ES-C26-agua`, `ES-C31-once-quince`, `ES-C32-gato`, `ES-C33-verde`) so the
  filename does not promise content the lesson no longer holds.
- Seven chapter payoffs moved to the new terminal lesson (Chapters 20, 22, 23,
  30, 31, 32, 33). `assesses` stays a subset of the payoff lesson's own
  `practises.knowledge` in every case.
- No lesson genuinely resisted splitting. The nearest thing to a hard case was
  `ES-C23-hermano-hermana` at four, where three atoms form one etymological
  story (*germen* → *germānus* → *hermano*) and the fourth is a sound-history
  correction about the silent *h*; that becomes a 3 + 1 split, and a one-atom
  lesson is well inside the corpus norm (median 2).
- The eighteen new lessons compute between 157 and 275 effective seconds, so the
  five-minute rule holds with room. Two of them are `writing` lessons, which
  moves the pinned modality counts: `pen` 51 → 53, `sight` 351 → 360, `voice`
  694 → 701, drivable share unchanged at 63%.
- HL-C18B is the remainder: 37 lessons across sixteen tracks, led by German (8),
  French (6), Sanskrit (3), Urdu (3) and Italian (3). Bengali, Punjabi and
  Sanskrit each hold a six-atom lesson, all three of them number lessons with
  the same shape as the Spanish one just split.

## Findings from HL-C24

- Four Latin chapters now end on a lesson written to be a payoff:
  `LA-C19-practice`, `LA-C21-practice`, `LA-C33-practice`, and
  `LA-C36-practice`. Latin had exactly one `practice` lesson across 36
  chapters before this tranche; it now has five, and the corpus reaches 1,100
  lessons with zero duration violations and zero prerequisite errors.
- **The representativeness gate cannot see this gap.** All 36 Latin chapters
  already measured 100% before the change, and all 36 still measure 100% after
  it, because a chapter's last teaching lesson cumulatively practises every atom
  the chapter introduced. Representativeness answers "does the payoff touch the
  chapter's material" — it cannot answer "is the payoff something the reader can
  *do*." HL-C03's gate set needs a distinct signal for that: the honest one
  available today is whether the chapter's terminal lesson is of a consolidation
  type (`practice`, `practice-mix`, `pattern`) at all. On that measure Latin was
  1 of 36 and is now 5 of 36.
- Three of the four payoffs are genuine `dialogue`s built only from taught
  words. Chapter 33 is deliberately **not**: it teaches *vesper* and its
  afterlives, with no greeting or exchange anywhere in it, so its payoff is a
  `task` — sort any European evening word into the *vesper* family or the
  *sērus* family, then produce *vespere*. Forcing a conversation there would have
  misrepresented what a taproot track is for.
- The constraint that actually bites is **strict knowledge closure combined with
  a single-word-per-lesson corpus**. A payoff may only recombine what the
  transitive prerequisite chain introduced, and Latin's chain is a thin line
  (each lesson names one or two prerequisites), so useful material taught in a
  *sibling* branch is invisible unless the payoff names it as an extra
  prerequisite. Chapter 19 could reach *grātiās tibi agō* only because
  `LA-C19-quid-agis` happens to depend on `LA-C01-ita-non`; chapter 36 had to
  name `LA-C34-bonum-vesperum` and `LA-C19-practice` explicitly to see the
  *bonus*-phrase and wellbeing atoms at all. Any track-wide scale-up should
  expect to author prerequisite edges, not just lessons.
- Chapters whose material is purely etymological or purely metalinguistic resist
  a usable payoff on principle, not on effort. Latin chapter 33 is the clean
  example; chapters 2 (numbers), 5 (weekday names), and 11 (months) are the same
  shape. `task` and `production` payoffs are the right answer there, and the
  ledger should say so rather than labelling them `dialogue`.

## Findings from HL-C26

- The gap is larger than the ledger work suggested: **105** chapters have a committed
  `book/chapters/ch*.tex` but no `targets[]` entry, across 19 tracks. They are not a
  scattering of stragglers — they are a contiguous hand-written *prefix* of nearly every
  book, ending where generation was switched on. French and German chapters 1–16, Spanish
  7–18, and all of Russian were missing from the informal list this work started from.
- **A `targets[]` entry is not a description; it is an instruction to generate.**
  `generatedBookOutputs` renders every target and `runBookGeneration --write` writes the
  result over the file at `output`. Minting targets for these chapters — the obvious
  reading of the task — would have destroyed them. Confirmed empirically: adding a target
  for `latin` ch1 made `check:books` report the committed file stale immediately, and the
  output it wanted to write was a different 235-line document (banner, regenerated prose,
  `\label{lesson:LA-C01-salve}` in place of `\label{lesson:salve}`) replacing 168 lines of
  authored text.
- The fix is therefore a separate `handwritten[]` list rather than a `generated: false`
  flag on `targets[]`. The two fail in opposite directions: a flag leaves authored prose
  one forgotten `if` away from being overwritten, whereas a second array cannot be
  rendered at all, because `generatedBookOutputs` only ever walks `config.targets`. The
  worst a mistake in `handwritten[]` can do is leave a chapter unchecked — today's
  behaviour — instead of destroying it.
- Every generated chapter opens with `% GENERATED FILE.` and no hand-authored chapter
  does (270/270 and 0/105). That makes the banner a list-independent check on the
  generator's claim, and it catches the one mistake the lists cannot see themselves: a
  chapter *promoted* out of `handwritten[]` into `targets[]`, which leaves the
  hand-written list and so escapes every check keyed on membership.
- **Chapter labels follow three incompatible conventions, and they are left alone.** Most
  hand-written chapters use a bare slug (`ch:greetings`), generated chapters use an
  ISO-code prefix (`ch:fa-`, `ch:la-`, `ch:it-`, `ch:ar-`), and hand-written Persian,
  Urdu, and Russian chapters use a language-*name* prefix (`ch:persian-name`,
  `ch:urdu-name`, `ch:russian-greetings`). So Persian ch2 is `ch:persian-name` while its
  generated ch3 sibling is `ch:fa-ask-and-answer-names`, in one book. Renormalising would
  break every existing `\hyperref`, so `handwritten[]` records what each `.tex` declares.
  No label collides with another inside the same track today. Worth a deliberate decision
  before HL-C04 makes `chapters.json` canonical.
- The bare-slug convention means `ch:greetings` is reused across 16 tracks. That is safe
  only because each track compiles its own PDF; any future combined volume would collide.

## Findings from HL-C30

HL-C30 asked whether Arabic's low drivable share (52%, 31 lessons reachable in
chapter-prefix order) could be recovered cheaply by moving the `AR-W*` writing
lessons that open Chapters 3 and 4 later in their own chapters. **It cannot.**
The measured answer is that no legal reordering changes any Arabic number, and
the premise that "most of the lessons after those openings are `voice`" does not
survive contact with the corpus. No lesson was moved. The findings below are the
whole deliverable.

- **Arabic Chapters 3 and 4 are provably immovable, and the writing lessons are
  not the reason.** A chapter's drivable prefix can only begin with a lesson that
  has no in-chapter prerequisite. Chapter 3 has exactly two such roots —
  `AR-W07-hook-family-ha-kha` (`pen`) and `AR-C03-kayfa` (`sight`, two-column
  table) — so **every** legal ordering starts with a non-`voice` lesson and the
  prefix is 0 no matter what moves. Chapter 4 has a single root,
  `AR-W10-ayn`, because `AR-C04-maa-with` declares it as a prerequisite: مع
  cannot be read without ʿayn. Deleting all six writing lessons outright would
  still leave both chapters at prefix 0.
- **The blocker is the table, not the script.** Of Chapter 3's six non-writing
  lessons only `AR-C03-bi-khayr` is `voice`, and it sits behind
  kayfa → hal → kayfa-ḥāluka by prerequisite. All five of Chapter 4's
  non-writing lessons are `sight`. Every one of Arabic's 18 `sight` lessons is
  `sight` because of a Markdown table — 18 of 18. Arabic's drivable share is an
  HL-C17 problem end to end.
- **Pedagogy would have blocked the move independently.** `AR-C03-kayfa`'s
  "letters in this word" section states that ك has already been written "in the
  writing set", i.e. it assumes `AR-W08-kaf-and-ra`, which requires `AR-W07`;
  `AR-W09-khayr-bikhayr` assembles خير so the learner can hand-write the
  *bi-khayr* reply the chapter ends on. The inline-letters rule in HL00 puts
  those lessons exactly where they are.
- **Arabic's other five zero-prefix chapters have nothing to reorder.**
  Chapters 12, 14, 19 and 20 hold one table-bearing lesson each; Chapter 8's
  second lesson declares its table-bearing first lesson as a prerequisite.
- **Corpus-wide, reordering is nearly worthless.** 123 chapters have a drivable
  prefix of 0. Only **7** contain a `voice` lesson with no in-chapter
  prerequisite, so only 7 are candidates at all; the other **116 are blocked at
  the root by a table** and belong to HL-C17. Arabic contributes none of the 7.
- **Only two of those 7 are genuine**, and both are one-lesson lifts of a
  table-bearing opener that nothing else in the chapter depends on:
  `portuguese ch2` (0 → 3, move `PT-C02-de-nada` after `PT-C02-tudo-bem`) and
  `italian ch2` (0 → 1, move `IT-C02-prego` later). That is the entire
  reordering burn-down for the corpus: **+4 lessons.**
- **The other five are a measurement artifact, and so is much of the report.**
  `orderChapterLessons` sorts by `sequence` with null last, tie-broken by id.
  In mixed-schema tracks the `W*` writing lessons are schema v2 and carry a
  sequence while the word lessons are still legacy and carry none, so the pen
  block sorts to the front of a chapter it does not actually open. **85
  chapters — 25 of them among the 123 zero-prefix chapters — are ordered by the
  report in a way that contradicts their own in-chapter prerequisite graph.**
  `hindi ch1` is reported as opening with `HI-W01-shirorekha-na-ma`, which
  *declares `HI-C01-namaste` as a prerequisite*; `tamil ch1` reports
  `TA-W01-curves-va-ka`, which declares `TA-C01-vanakkam-family-register`. Those
  chapters do not open with a writing lesson at all, and their prefixes are
  measured against an order no author wrote.
- **The list of chapters reported as opening with a `writing` or script-block
  lesson**, i.e. the burn-down order HL-C30 asked for: `arabic ch3`, `arabic ch4`
  (both real and both immovable, above); `hindi ch1`, `hindi ch2`, `tamil ch1`
  (all three artifacts of the ordering bug — nothing to move); `telugu ch7`,
  `telugu ch8` (real, but the openers are table- and script-block-bearing word
  lessons with no `voice` lesson anywhere in the chapter).
- **Arabic Chapters 1 and 2 are undercounted by the same artifact.** All 26 of
  their lessons are legacy and unsequenced, so they sort alphabetically:
  Chapter 1 reports a prefix of 4 where the authored `curriculum.json` path
  gives 7, and Chapter 2 reports 6 where the path gives 7. Recovering those +4
  lessons means giving legacy lessons a `sequence`, which **0 of the corpus's
  565 legacy lessons currently have** and which `validateCurriculum` does not
  check for uniqueness outside schema v2 — a schema migration with a silent
  collision hazard, not a reorder. It is deliberately left undone here. Arabic's
  sparse numbering has room reserved for it: Chapters 1–2 hold exactly 26
  lessons and slots 10–260 are free below Chapter 3's first sequence of 270.
- **Recommended follow-up owners.** The ordering artifact belongs beside HL-C14
  (either the comparator falls back to a prerequisite-respecting order, or the
  legacy tracks get sequences); the 116 table-blocked chapters belong to
  HL-C17. Reordering itself is closed at +4 lessons corpus-wide.

## Findings from HL-C32

The Russian repair is worth reading as a diagnosis, because the diagnosis
generalises and the fix does not.

- **Russian's 9% was one rule firing fifteen times out of fifteen.** Every
  `sight` lesson in the track tripped `wide-table`. Not one carried a `script`
  block. Twelve tripped nothing else. It was never a script-heavy track; it was a
  table-heavy one.
- **Two of the three sight cues that did match were false positives.** The cue
  list is literal substring matching, so `"the course's first look at case"`
  matched `look at`, and a sentence describing a comparison as `"the most extreme
  change in the table"` matched `the table` only because the table existed. Only
  `RU-C02-practice-cases` — *"cover the right column"* — points at anything real.
- **The tables were carrying prose.** Almost every one was a cross-language
  word→gloss list: `| Language | "yes" | built from |`. `RU-C01-privet` and
  `RU-C01-zdravstvuyte` carry exactly that section as sentences, and they were
  the track's only two `voice` lessons. The same content, set two ways, produced
  two different modalities — which is the whole finding.
- **One table was genuinely visual and stayed.** `RU-C02-practice-cases` is a
  cover-the-column retrieval drill; the table *is* the exercise. It remains
  `sight`, and Chapter 2's drivable prefix correctly stops there at 8 of 10.
- **The pattern is corpus-wide, and Russian was only its most extreme case.**
  Of 337 `sight` lessons remaining, 271 trip `wide-table` and nothing else.
  Grouped by track, `onlyTable` counts run: spanish 57, german 33, portuguese 32,
  french 29, italian 29, arabic 14, tamil 14, and so on. Those five European
  tracks sit at 43–47% drivable for the same reason Russian sat at 9%. HL-C17 is
  therefore not a Russian problem that leaked; it is the corpus's single largest
  modality lever, and this pass is a worked example of how to pull it —
  distinguishing a two- or three-column word→gloss list, which linearises, from a
  real multi-column paradigm, which does not.
- **Representativeness was a migration symptom, not an authoring failure.**
  Chapter 2's payoff pointed at a cross-language etymology lesson because that was
  the last schema-v2 lesson by sequence; the chapter's actual consolidation
  lesson was schema v1 and declared no atoms. Migrating that one lesson took
  representativeness from 0.20 to 0.67 without inventing content. The same
  substitution is visible in the remaining sub-floor chapters (arabic ch3/ch4 at
  0.11/0.13, spanish ch3 at 0.25, hindi ch2 at 0.17), and the same one-lesson fix
  should work wherever the chapter's consolidation lesson is the schema-v1 one.
- **Two artefacts remain and only a full migration closes them.** Fifteen Russian
  lessons are still schema v1. Because `sequence` is a schema-v2 field, Chapter 1
  is still ordered alphabetically rather than pedagogically for modality
  purposes, and `RU-C02-practice` now sorts ahead of its own schema-v1
  prerequisite. Neither affects validation or the drivable prefix, and neither is
  worth a cosmetic patch.

## Findings from HL-C38

- The book read like an export because it printed the lesson files' **audio
  scaffolding**: 1,438 `[PAUSE Ns]`, 1,411 `[YOU SAY: …]`, 30 `[REPEAT x2]`, and
  the internal block-type names `Warm-up` / `Guided Practice` / `Wrap-up recall`
  as printed headings, across all 270 generated chapters. None of that is a
  lesson-authoring bug: HL00 is right that lessons are audio scripts. The bug was
  that the **book view** rendered the stage directions.
- Fixed entirely in `src/book.ts`, in one documented "book voice" section. No
  lesson Markdown, `chapters.json`, or hash-manifest entry was touched, and the
  270 chapters regenerate to identical source hashes.
- `[PAUSE Ns]` is deleted (a reader sets their own pace). `[REPEAT xN]` becomes
  *Twice through:*. `[YOU <VERB>: …]` becomes a printed prompt — a single lead-in
  above a uniform run of bullets (*Say these aloud:*), or a per-bullet italic
  label (*Say it:*, *Write it:*) where a list mixes cue kinds. Twenty-eight cue
  verbs are mapped in one table; writing and tracing prompts render as real
  printed exercises and are never suppressed.
- Printed headings now read `Your turn`, `Before you move on`, `What to know
  first`; the warm-up loses its label entirely and stands as the section's
  indented lead-in. `You'll want to know — <descriptive tail>` headings are left
  alone: they are authorial prose, and rewriting them mechanically read worse.
- The chapter blurb ("generated from the canonical micro-lessons used by Language
  Ladder") is gone. A book does not describe its own build system.
- **The book is now a standalone artefact.** The pronunciation/script section
  moved from directly after `\mainmatter` to `\backmatter` in all 17 books that
  front-loaded it — HL00 forbids a front-loaded sounds chapter, so the book had
  been contradicting its own framework. Nothing was deleted; it is reference now,
  not a gate.
- `sourceBaseUrl` no longer feeds the book view, reversing that half of HL-G05.
  A reader holding the PDF cannot follow a link into a Git repository. Relative
  destinations (`./ES-C01-bien.md`, `../pronunciation-reference.md`) now print
  their label unlinked; absolute scholarly citations (UT Austin, MSU, Wiktionary)
  stay live `\href`s. The config field stays, validated, for other consumers.
- All 20 prefaces rewritten, each keeping its own track's material: a welcome, a
  "How to use this book" section, and the removal of the sentence that
  rationalised the front-loaded pronunciation section. The Latin and Sanskrit
  prefaces now say plainly that they are not learned for conversation; Tamil
  addresses the heritage reader HL00 describes. The title-page pointer at
  `code/learning/human-languages/<track>/lessons/` is gone from all 20.
- The 20 track READMEs keep their engineering detail below a `## For
  contributors` line; above it the spec-ID citations, `schema-v2`, source-hash,
  and Language Ladder references are gone.
- **"payoff" was checked case by case and kept.** All 10 uses in book prose are
  ordinary English ("here is the payoff", "two payoffs land at once"), not the
  HL05 field name. Only the README uses referring to `chapters.json` were moved
  below the contributor line.

## Findings from HL-C41

HL-C41 set out to teach Telugu handwriting and to design the interspersed-writing
pattern the project owner asked for. **One half landed and one half is blocked**, and
the blocked half is worth recording precisely, because the block is not a scheduling
problem.

- **Telugu handwriting is blocked on provenance, not on effort.** `strokes.ts` admits
  a letter only with a `citation` and a `url` for its stroke ORDER — the shape is
  checked against the font, the order cannot be, so it must trace to a real source.
  No such source could be reached for a single Telugu letter. The owner's pointer
  (`youtube.com/watch?v=57LhnFmilLs`) returns HTTP 403 and was treated as unverified.
  The candidates a search surfaced — Vemuri's *The Shapes of Telugu* (UC Davis), the
  Peace Corps *Conversational Telugu*, Wikisource's 1857 Brown grammar, Omniglot,
  `teluguaksharalu.com` — were all unreachable from the working session, and none
  could be opened to confirm what it says about any individual letter. A GitHub-wide
  search for an Indic stroke-order dataset returns nothing.
  **Zero letters authored, ~36 base consonants skipped.** Fewer letters honestly beats
  more letters invented, and the same conclusion holds for Kannada and Malayalam.
  What is needed is one openable primer with numbered stroke arrows; with it, the
  base-consonant inventory is a day's work, because the font-validation half of the
  pipeline already exists and `_fonts/NotoSansTelugu-Static.ttf` is vendored.
- **The one substantive claim that *is* attested is a warning, not a shortcut.** The
  premise behind the request — that Telugu is written largely without lifting the pen
  — is a simplification. The recurring published statement about Telugu stroke
  direction is that *the order of the strokes is not uniform across the letters; for
  some it is clockwise and for others counter-clockwise.* Telugu's roundness makes
  many letters **loop-continuous**, which is a real and teachable property, but it is
  not the same claim as "one stroke, no lifts", and the `talakattu` tick that crowns
  most consonants is widely described as a separate mark. So `penLifts` for Telugu is
  exactly the field that must stay ABSENT — meaning NOT VERIFIED — until a path is
  authored and checked.
- **The parts-vs-strokes rule is now written down** in
  [`data/scripts/README.md`](./data/scripts/README.md) and in the syllabary
  generator's own header, where the next author will meet it: only base consonants
  and vowel signs are ever authored, a syllable's figure is assembled from its parts,
  `penLifts` absent means NOT VERIFIED, and it must never be inferred from
  `strokeOrder.length`. Authoring 455 Telugu syllables was never the work; authoring
  ~36 shapes is.
- **Block-level modality landed, with its purpose corrected mid-flight.** The first
  framing — protect the drivable percentage from interspersed writing — was rejected
  by the project owner: *"the book is a standalone artifact… include the writing
  lessons in the books."* The amendment is therefore metadata for a future
  dictation-friendly edition, not a lever on the book. It is a strict improvement for
  that edition: today a lesson with any pen content is lost to a commuter wholesale;
  with block marking they get the voice core and defer only the segment.
- **The amendment is a measured no-op today, on purpose.** No track has authored an
  interspersed `writing` segment yet, so every lesson's core equals its full modality
  and the corpus figure is unmoved at **708 drivable, 65%** — pinned as a regression
  test alongside `lessonsWithWritingSegments === 0`, so the first interspersed lesson
  must move the number deliberately rather than by accident.
- **No demonstration lesson shipped, and that is the finding.** The interspersed
  pattern is implemented and unit-tested against synthetic lessons, but no Telugu
  lesson demonstrates it, because a writing segment for Telugu would have to assert a
  stroke order this repository cannot cite. HL-C42 carries that forward: the first
  interspersed lesson should land in a track whose ductus is already sourced —
  Tamil's ம traces to the UT Austin primer — rather than waiting on Telugu.

## Completed foundations

- HL04 defines the 45-concept shared spine and migration contract.
- The 20-track registry, Persian/Urdu pilots, full Markdown bodies, registry-driven
  language selection, RTL app rendering, and fail-closed prerequisites are merged.
- One CI job now installs TeX once, compiles every book, uploads one publication
  bundle, and publishes the catalog after changes reach `main`.
