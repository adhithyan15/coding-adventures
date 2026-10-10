### Fixed — a lesson announced as drivable no longer asks a driver to read, write, point or gesture

- **The defect (security review, driver safety).** A lesson whose core is
  `voice` is announced by the narration as "you can do this one in the car".
  Only its detachable sections (`## Writing — …`, `## The letters in this
  word`, `## Script — …`) were set aside behind the stop guard; every other
  section was read out plainly. 166 steps in the cores of 132 such lessons, in
  fifteen tracks, asked the driver to use their eyes or hands — wrap-up recalls
  such as "[PAUSE 3s] Read **नमस्ते**.", recalls such as
  `[YOU RECALL: say *āmi*, then read **কেমন**]`, "Uncover **આજ** once", "Write
  **か** once from memory", "Point to **ش · ک · ر**", "Say **namaskāra** with a
  small bow". By imperative: 98 reading steps (32 inside spoken recalls), 30
  writing, 28 cover or uncover, 4 pointing, 6 gestures.
- **Why it slipped through.** The detectors for exactly these steps already
  existed, as a corpus test (`tests/drivable-writing-cues.test.ts`) that
  demanded zero — but only over `drivable` lessons, those that are `voice`
  through and through. A lesson with a detachable section is never `drivable`,
  so its core was never read. `modality.ts`'s own rules look only for what
  cannot be *spoken* (a script block, a pointing cue such as "look at", an
  unspeakable table); a reading step is perfectly speakable, so the
  `no-visual-dependency` core stood.
- **The classifier is now honest.** The detectors moved from
  `tests/drivable-writing-imperatives.ts` to `src/drivable-instructions.ts`,
  and `modality.ts` runs them (`eyesOrHandsSteps`, via `eyesOrHandsStepsIn`) on
  the preamble and every section a renderer keeps. A step there makes the block
  and the lesson `sight`, at both scales, with the new reason code
  `eyes-or-hands-step` (also accepted by `modality-shards.ts`); the steps are
  quoted on `BlockModality.eyesOrHandsSteps` and
  `LessonModality.coreEyesOrHandsSteps`. Headings are not read (a heading names
  a section; "Practice — hear, say, read, and write water" lists skills), and
  detachable sections are not read (their type already needs the eye or the
  hand, so the reason would only be noise there). A deferred cue —
  `[YOU READ: …]`, `[YOU WRITE: …]`, `[YOU FEEL: …]`, any verb in
  `MANUAL_CUE_ACTIONS` — is not a step: the narration already says "once you
  have stopped driving" before it. It is `sight`, not `pen`, even for a writing
  step: `pen` signs a lesson that teaches the hand.
- **Narration.** The notice of a lesson with the reason adds "your eyes or your
  hands for the steps that ask you to read, point, write or gesture", and names
  every section carrying a step in `waitUntilStopped`, so the stop guard is
  spoken when that section begins.
- **Detector widenings, each a closed vocabulary with the precision measured
  on the corpus.** A prose reading step now also reaches the page through a
  page noun after a determiner ("Read these shuffled numerals", "the visible
  row once"; not "line", which a dialogue has too), "your" plus the learner's
  own writing ("Read your copy"), "what you wrote", a source ("from the page",
  "from your own handwriting"), or a pronoun or count plus the manner of a
  reading pass ("Read it, and say …", "each one and say …", "it one small
  piece at a time", "both, then answer"). Excluded: "read X as Y" without
  script, a bare "once" ("taught once and read once" describes). A new prose
  check (`proseAsksToPointOrGesture`) catches "point to / point at" where a
  step starts (not before a wh-word: "point at what it means") and a hand or a
  bow as the manner of a spoken clause ("Say … with a small bow"; not "*Vaṇakkam*
  is said with pressed palms", not "Imagine …"); over the corpus it fired on
  162 spans, every one in a lesson that needs eyes in full. Two gaps in the
  existing prose checks were found by the new non-vacuity test and closed: a
  list item ("- Read **নাম**.") was not a step start, and a reading step after
  a quoted sentence ("Say “I do not understand.” Read **…**") was hidden
  because blanking the quotation also removed its full stop.
- **Cost.** The step walk is screened by `mayAskForEyesOrHands`, a word-start
  vocabulary that is a superset of every check's anchor (held by a corpus
  test), and its answer is cached per block object (re-walked if the block's
  text changes). Measured locally over the 29,566-lesson corpus, a full
  derivation takes about 3.2 s the first time (about 1.6 s of it the step
  walk) and 1.2 s once the walk is cached.
- **Refactor.** The cue vocabulary and splitter (`MANUAL_CUE_ACTIONS`,
  `SPOKEN_CUE_ACTIONS`, `isManualCueAction`, `parseNarrationCue`,
  `splitNarrationCues`, `PROMPT_RESPONSE_SECONDS` and the cue types) moved from
  `narration.ts` to `narration-cues.ts`, so `modality.ts` can read a lesson as
  the narrator does without importing it (`narration.ts` imports
  `modality.ts`). `narration.ts` re-exports every name.
- **Corpus.** 78 lessons in twelve tracks were rewritten so the step is
  deferred in a cue (or, for two bows, a pointing gesture and one gloss the
  gesture check misread, said for the ear) and stay drivable; 54 lessons whose
  point *is* the eye or the hand (Gujarati word decoding and reading-score
  reviews, script-recall reviews, numeral and writing practice) are now `sight`
  at the core. No lesson's full modality moved, and no `drivable: true` lesson
  changed. See each track's changelog.
- **Tests.** `tests/drivable-instructions.test.ts`: the new detector objects
  (positive and negative), the pointing and gesture rule, every check family
  reaching the aggregator and every deferred cue staying out, the vocabulary
  screen as a superset, linearity on ~50,000-character inputs, the classifier
  and the narration on synthetic lessons, and the corpus gate — zero
  eyes-or-hands steps in any kept section of a lesson the committed manifest
  marks `coreDrivable`, cross-checked (on a fixed quarter of those lessons) by
  every separate check with no screen, plus a synthetic non-vacuity
  demonstration: every core-drivable lesson's deferred `[YOU READ: **…**]`,
  turned back into "Read **…**.", must take that lesson out of the car.
  `tests/drivable-writing-cues.test.ts`: its reading-cue anti-vacuity floor
  (more than 40 non-drivable lessons) was debt, not a property of the
  detector — this change's recall splits took it from 66 to 35 — so it became a
  synthetic demonstration that grows as debt is paid: every drivable lesson's
  deferred READ cue, spoken again as `[YOU RECALL: read **…**]`, must be
  caught. `tests/modality.test.ts` lists `eyes-or-hands-step` among the known
  causes of a sight lesson.
- **Not fixed here (follow-up).** 1,498 scored `hl-activity` prompts in the
  kept sections of core-drivable lessons (mostly Japanese and Marwadi) are
  typed-answer contracts that open "Type …" or "Write …"; the narration frames
  each as "question — say your answer", but the prompt text itself still says
  "type" or "write". Activity prompts live in the typed AST, not in prose, so
  they are outside this rule; whether the narration should reword them or the
  rule should read them is a separate decision.
- **Regenerated:** `core/lesson-modality` owners (reasons and source hashes),
  the narration (`.json`, `.txt`) and narration hashes, and the book chapters
  and book hashes of the edited lessons' chapters.
