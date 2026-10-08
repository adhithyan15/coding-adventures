### Fixed — drivable lessons stop asking a driver to gesture

- Issue #12070, ninth pass. The narration reads every cue
  `isManualCueAction` does not defer as an ordinary turn, and the previous
  checks read spoken cues for writing, pointing at a sign (RECALL only) and
  reading script — not for the body. 135 spoken cues in drivable lessons asked
  a driver for a gesture: 66 demonstrative drills in six Indic tracks
  (`[YOU SAY: "இங்கே" three times, pointing at something different each
  time]`), 24 Japanese mora drills (`[YOU SAY: *denwa*, clapping three
  beats]`), four nested `[YOU HEAR: *añcŭ*; YOU SHOW: 5]` finger counts and
  two "while raising one more finger" counts in the Tamil and Malayalam number
  lessons, touching a body part as it is named, raising each hand for right
  and left, "offered with both hands", a hand-wobble, a small bow, and review
  labels that name a gesture ("pointing at them — *ei bhāirā*").
- Detector (`tests/drivable-writing-imperatives.ts`): new
  `spokenCueAsksForGesture` / `gestureSpokenCues`, over every cue the
  narration does not defer. Three shapes, each a closed vocabulary: a gesture
  verb where a step starts (the reading check's links plus " while ",
  " as you ", " by "; "point"/"pointing" but not "point out" or "points",
  "clap", "tap" only with "twice"/"once"/"out" or a count of beats, groups,
  syllables or morae, "touch" with a pronoun or indefinite object, "raise"
  with a finger or hand, "hold up", "show" with fingers, "gesture", "wave
  goodbye", "nod", "shake your head"); a hand or bow as the manner of saying
  ("with both hands", "with a small bow", "palms together", "hand in front of
  your mouth"), anywhere in the cue; and a nested `YOU <VERB>:` whose verb
  `isManualCueAction` calls manual. Quotations are blanked first. Linear with
  no nested quantifier: fixed-literal alternations with optional fixed words
  or single characters, one pass each, and the nested-cue scan a fixed
  literal plus one run of capitals.
- `MANUAL_CUE_ACTIONS` gains SHOW and CLAP (no cue heads with either; SHOW is
  the verb the nested finger counts used, and the nested-cue scan reads it
  through `isManualCueAction`). `cue-action-classification.test.ts` checks
  that `YOU SHOW: 5` is deferred.
- Measured over every spoken cue in the corpus before the fix, the check
  fired on exactly the 135 drivable cues that were rewritten and, in
  non-drivable lessons, on 29 cues that are real gesture work there (Kannada
  digit lessons pointing at a printed figure, aspiration drills with a hand in
  front of the mouth, Bengali and Urdu demonstrative reviews, two Japanese
  writing recalls that clap). No gloss ("the Tamil for to touch"), vocabulary
  list, tongue tap ("one soft tap on the *r*"), speech act ("wave away an
  apology") or voice wobble fired.
- Tests (`tests/drivable-writing-cues.test.ts`): 27 positives (corpus cues as
  authored before the fix, the non-drivable shapes, a nested WRITE and a cue
  wrapped across two lines), 31 controls (the rewrites, deferred POINT and
  READ cues, glosses, vocabulary, material, the tongue tap, a speech act, a
  single consonant, a wobble in the voice, nested spoken cues), 15
  ~50,000-character adversarial inputs asserting answers only, the corpus
  check (zero in drivable lessons) and an anti-vacuity floor on non-drivable
  lessons (> 20; 29 measured).
- Corpus: 136 spoken cues (the 135 above plus SA-C30-anjali's "name what your
  hands are doing", which presupposes the gesture without naming a movement
  and was found by reading) and 130 prose instructions in 212 drivable lessons
  across 18 tracks. Each is rewritten for the ear and voice where that keeps
  the learning goal — counting beats aloud, picturing what a demonstrative
  lands on, saying the number or the meaning heard, "say *right* or *left*
  after each", "listen for the breath" — and pointing at printed script, which
  the ear cannot do, moves into 15 `[YOU POINT: …]` cues and two
  `[YOU READ: …]` cues the narration defers. Prose is not gated: "pointing"
  is too common in explanations of demonstratives for a clean check, so the
  prose was found by inventory and fixed by hand. Judgement calls are recorded
  in each track's changelog.
- Modality: no lesson changes drivability; `core/lesson-modality` changes
  only the source hash of the 212 edited lessons. Regenerated the books,
  narration and their hashes for the touched chapters.
