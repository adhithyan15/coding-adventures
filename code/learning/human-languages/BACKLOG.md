# Human Languages Backlog

This is the ordered delivery backlog for the shared-spine curriculum, books,
and Language Ladder. Reprioritize it after every merged work item. Add newly
discovered work here before starting it so the repository, rather than an agent
session, remains the source of truth.

Last prioritized: 2026-08-03. Current baseline after schema-v2 block-boundary
validation: 20 registered tracks, 1,065 Markdown lessons, 20 downloadable LaTeX
books, and zero duration violations. HL-V01 keeps the remaining migration debt
reproducible in both JSON and human-readable reports; the first 51 Spanish
lessons now prove one typed, knowledge-closed source across Language Ladder and
six generated book chapters, and the completed HL-D01 tranches prove duration
remediation without discarding deep content.

## Priority rules

1. Close a learner-visible broken promise before adding breadth.
2. Prefer work that makes later corpus growth measurable or generated.
3. Finish a small vertical slice before starting the same migration everywhere.
4. Keep the application, book, and canonical lesson content aligned.

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
| HL-V02 | Complete (#9653) | Validate learner-facing target-language prompts against block-level knowledge declarations and prerequisite closure. | Schema-v2 production and recall blocks cannot ask for an undeclared form or a form absent from the lesson's transitive knowledge frontier. |
| HL-V03 | Queued | Compile individual prompt, answer, accepted-variant, feedback, and response-time contracts from typed activity blocks. | Every compiled activity names a non-empty subset of its block's assessed atoms and resolves all answer variants without scraping prose. |
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
| HL-B14 | Complete (#9728) | Publish Italian Chapters 2–17 from their canonical lessons rather than hand-copying sixteen book chapters. | Forty-nine schema-v2 lessons now generate sixteen chapters whose source hashes are independently verified against the Language Ladder corpus. |
| HL-B15 | Complete in this PR | Remove Italian's LaTeX layout and Unicode bookmark warnings. | The forced 104-page build now has zero missing glyphs, overfull or underfull boxes, duplicate destinations, Hyperref warnings, or LaTeX warnings. |
| HL-B16 | Next | Publish Portuguese Chapters 2–17 from their canonical lessons rather than hand-copying sixteen book chapters. | Portuguese has canonical app content through Chapter 17, but its downloadable PDF contains only Chapter 1; schema-v2 migration plus generation should close that drift safely. |
| HL-B17 | Queued | Remove Portuguese's LaTeX layout warnings. | A forced build succeeds with no missing glyphs, overfull boxes, duplicate labels, or Hyperref warnings, but reports three underfull boxes; the clean-build signal is zero. |
| HL-B18 | Queued | Publish French Chapters 17–23 from their canonical lessons rather than hand-copying seven book chapters. | The French PDF reaches Chapter 16 while canonical app content continues through Chapter 23; schema-v2 migration plus generation should close that drift safely. |
| HL-B19 | Queued | Remove French's LaTeX layout and Unicode bookmark warnings. | A forced build succeeds with no missing glyphs or duplicate labels but reports sixteen overfull boxes, nine underfull boxes, and six Hyperref warnings; the clean-build signal is zero of each. |
| HL-B20 | Queued | Publish German Chapters 17–23 from their canonical lessons rather than hand-copying seven book chapters. | The German PDF reaches Chapter 16 while canonical app content continues through Chapter 23; schema-v2 migration plus generation should close that drift safely. |
| HL-B21 | Queued | Remove German's LaTeX layout and Unicode bookmark warnings. | A forced build succeeds with no missing glyphs or duplicate labels but reports seventeen overfull boxes, eleven underfull boxes, and three Hyperref warnings; the clean-build signal is zero of each. |
| HL-B22 | Queued | Publish Telugu Chapters 6–31 from their canonical lessons rather than hand-copying twenty-six book chapters. | The Telugu PDF reaches Chapter 5 while canonical app content continues through Chapter 31; schema-v2 migration plus generation should close that drift safely. |
| HL-B23 | Queued | Remove Telugu's LaTeX layout, duplicate-label, bookmark, and font warnings. | A forced build succeeds with no missing glyphs but reports four overfull boxes, three underfull boxes, four duplicate practice labels, 27 Hyperref warnings, and a font-shape substitution warning; the clean-build signal is zero of each. |
| HL-B24 | Queued | Publish Kannada Chapters 6–31 from their canonical lessons rather than hand-copying twenty-six book chapters. | The Kannada PDF reaches Chapter 5 while canonical app content continues through Chapter 31; schema-v2 migration plus generation should close that drift safely. |
| HL-B25 | Queued | Remove Kannada's LaTeX layout, duplicate-label, bookmark, and font warnings. | A forced build succeeds with no missing glyphs but reports four overfull boxes, five underfull boxes, four duplicate practice labels, 30 Hyperref warnings, and undefined bold/italic Kannada font shapes; the clean-build signal is zero of each. |
| HL-B26 | Queued | Publish Malayalam Chapters 6–31 from their canonical lessons rather than hand-copying twenty-six book chapters. | The Malayalam PDF reaches Chapter 5 while canonical app content continues through Chapter 31; schema-v2 migration plus generation should close that drift safely. |
| HL-B27 | Queued | Remove Malayalam's LaTeX layout, duplicate-label, bookmark, and font warnings. | A forced build succeeds with no missing glyphs but reports seven overfull boxes, eight underfull boxes, four duplicate practice labels, 28 Hyperref warnings, and undefined bold/italic Malayalam font shapes; the clean-build signal is zero of each. |
| HL-B28 | Queued | Publish Arabic Chapters 3–27 and its writing companions from canonical lessons rather than hand-copying another twenty-five book chapters. | The Arabic PDF stops after Chapter 2 while canonical app content continues through Chapter 27 and sixteen dependency-ordered writing lessons; schema-v2 migration plus generation should close that drift safely. |
| HL-B29 | Queued | Remove Arabic's LaTeX layout, duplicate-label, bookmark, and font warnings. | A forced build succeeds with no missing glyphs but reports one overfull box, four underfull boxes, one duplicate practice label, 14 Hyperref warnings, and undefined bold/italic Arabic font shapes; the clean-build signal is zero of each. |
| HL-B30 | Queued | Publish Hindi Chapters 6–33 and its writing companions from canonical lessons rather than hand-copying another twenty-eight book chapters. | The Hindi PDF stops after Chapter 5 while canonical app content continues through Chapter 33 and eleven dependency-ordered writing lessons; schema-v2 migration plus generation should close that drift safely. |
| HL-B31 | Queued | Remove Hindi's LaTeX layout, duplicate-label, bookmark, and font warnings. | A forced build succeeds with no missing glyphs but reports two overfull boxes, five underfull boxes, three duplicate practice labels, 29 Hyperref warnings, undefined bold/italic Devanagari font shapes, and a visibly colliding final-page running header; the clean-build signal is zero of each. |
| HL-B32 | Queued | Publish Tamil Chapters 6–31 and its writing companions from canonical lessons rather than hand-copying another twenty-six book chapters. | The Tamil PDF stops after Chapter 5 while canonical app content continues through Chapter 31 and eight dependency-ordered writing lessons; schema-v2 migration plus generation should close that drift safely. |
| HL-B33 | Queued | Remove Tamil's LaTeX layout, duplicate-label, bookmark, and font warnings. | A forced build succeeds with no missing glyphs but reports six overfull boxes, six underfull boxes, four duplicate practice labels, 27 Hyperref warnings, and undefined bold/italic Tamil font shapes; the clean-build signal is zero of each. |
| HL-B34 | Queued | Publish Latin Chapters 2–36 from canonical lessons rather than hand-copying another thirty-five book chapters. | The Latin PDF contains only Chapter 1 while canonical app content continues through Chapter 36; schema-v2 migration plus generation should close that drift safely. |
| HL-B35 | Queued | Remove Latin's remaining LaTeX layout and font warnings. | A forced build succeeds with no missing glyphs, overfull boxes, duplicate labels, or Hyperref warnings, but reports one underfull box and a small-caps font-shape substitution; the clean-build signal is zero of each. |
| HL-B36 | Queued | Publish Spanish Chapters 19–33 from canonical lessons rather than hand-copying another fifteen book chapters. | The Spanish PDF stops after Chapter 18 while canonical app content continues through Chapter 33; schema-v2 migration plus generation should raise book coverage from 55% to 100%. |
| HL-B37 | Queued | Remove Spanish's remaining legacy LaTeX layout, bookmark, and font warnings. | After Chapters 4–6 generation, a forced build has no missing glyphs or duplicate labels but reports 50 overfull boxes, 14 underfull boxes, 14 Hyperref warnings, and one undefined small-caps shape; the clean-build signal is zero of each. |
| HL-M01 | Queued | Add per-track spine realization maps and language-specific extension nodes. | Enables safe cross-language scheduling beyond the current concept join. |
| HL-M02 | Queued | Extend Telugu's roadmap and authoritative session map through canonical Chapter 31. | The roadmap narrative stops at Chapter 6 and the session map at Chapter 5 even though prerequisite-ordered lessons continue through Chapter 31; every canonical lesson, including the new register support step, needs a scheduled place. |
| HL-M03 | Queued | Extend Kannada's roadmap and authoritative session map through canonical Chapter 31. | The roadmap narrative stops at Chapter 6 and the session map at Chapter 5 even though prerequisite-ordered lessons continue through Chapter 31; every canonical lesson, including the new stacking support step, needs a scheduled place. |
| HL-M04 | Queued | Extend Malayalam's roadmap and authoritative session map through canonical Chapter 31. | The roadmap narrative stops at Chapter 6 and the session map at Chapter 5 even though prerequisite-ordered lessons continue through Chapter 31; every canonical lesson, including the four new support steps, needs a scheduled place. |
| HL-M05 | Queued | Reconcile Arabic's roadmap and authoritative session map with canonical Chapters 1–27 and the sixteen-step writing sequence. | The roadmap details only Chapters 1–4 and still calls Chapter 5+ planned; the session map stops at Chapter 2 even though prerequisite-ordered canonical lessons continue through Chapter 27. |
| HL-M06 | Queued | Reconcile Hindi's roadmap and authoritative session map with canonical Chapters 1–33 and the eleven-step writing sequence. | The roadmap details only Chapters 1–6 and still calls Chapter 6 planned; the session map stops at Chapter 5 even though prerequisite-ordered canonical lessons continue through Chapter 33. |
| HL-M07 | Queued | Reconcile Tamil's roadmap and authoritative session map with canonical Chapters 1–31 and the eight-step writing sequence. | The roadmap details only Chapters 1–6 and still calls Chapter 7+ planned; the session map stops at Chapter 5 even though prerequisite-ordered canonical lessons continue through Chapter 31. |
| HL-M08 | Queued | Reconcile Latin's roadmap and authoritative session map with canonical Chapters 1–36. | Both files stop at Chapter 1 and describe Chapter 2+ as planned even though prerequisite-ordered canonical lessons continue through Chapter 36. |
| HL-M09 | Queued | Reconcile Spanish's roadmap and authoritative session map with canonical Chapters 1–33 and the new support steps. | The roadmap stops at Chapter 18 and calls Chapter 19 next, while the session map stops at Chapter 3; both lag the prerequisite-ordered canonical curriculum. |
| HL-T01 | Queued | Complete session maps and pronunciation references for Persian and Urdu. | The starter-book work supplies both roadmaps and changelogs; these remaining pieces complete the standard track shape. |
| HL-U01 | Queued | Vendor and verify an appropriately licensed static Nastaliq font for normal Urdu presentation. | Naskh remains an explicit accessibility fallback, not the intended printed style. |

## P2 — corpus growth

- Extend Persian and Urdu through the first three shared-spine clusters.
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

## Completed foundations

- HL04 defines the 45-concept shared spine and migration contract.
- The 20-track registry, Persian/Urdu pilots, full Markdown bodies, registry-driven
  language selection, RTL app rendering, and fail-closed prerequisites are merged.
- One CI job now installs TeX once, compiles every book, uploads one publication
  bundle, and publishes the catalog after changes reach `main`.
