### Fixed — hands-on cues (copy, circle, …) wait until the driver has stopped

- Issue #12070, sixth pass. The narration defers a cue whose verb is in
  `MANUAL_CUE_ACTIONS` ("[once you have stopped driving — write: …]") and
  reads every other cue as an ordinary turn ("[your turn — …]" plus an
  eight-second pause). The set held six verbs (WRITE, TRACE, POINT, GESTURE,
  LABEL, FEEL) on the theory that any other manual verb would be caught by a
  writing block first. It was not: `[YOU COPY: **राम-राम सा** once …]`
  (MW-C01-raam-raam-saa), `[YOU COPY: **ا** once beside the visible model …]`
  (FA-C01-practice), `[YOU COPY: **வ** once …]` (TA-C01-practice) and
  `[YOU CIRCLE: the first base letter **ఝ**]` (TE-R152-jhari-recall) were all
  read to a driver, and so were 40 drivable `[YOU LOOK: at आओगे and put your
  finger on …]` cues and 134 drivable `[YOU READ: **でぐち** …]` cues.
- An inventory of every cue in the corpus (about 112,600, 89 distinct
  actions) classified each head verb. Added to `MANUAL_CUE_ACTIONS`, each with
  its reason in the table above the set: COPY, CIRCLE, COVER (always "cover,
  then write"), TAP (beats tapped with a finger, beside GESTURE), TEST (its one
  use is "hand at the mouth", FEEL by another name), LOOK, READ (READ ALOUD
  included: reading printed script is the eyes' job), FIND, CHECK (its one use
  checks a handwritten shape) and STACK ("point below 五"). DRAW, UNDERLINE,
  MARK and TICK are listed with no corpus use yet, so the first cue to use one
  is deferred rather than read out.
- New `SPOKEN_CUE_ACTIONS` lists the 63 head verbs a driver can do by voice and
  ear (SAY, RECALL, HEAR, NOTICE, CONTRAST, COUNT, …). The narration does not
  read it; it exists for the guard below. New `isManualCueAction(action)`
  treats a compound action as manual when ANY of its words is manual, so
  `COVER AND WRITE` and a future `SAY AND WRITE` are deferred; the narration
  used to look only at the first word. All three are exported.
- Guard: `tests/cue-action-classification.test.ts` walks every cue in every
  lesson (preamble, block titles and bodies) with the renderers' own
  `closingBracket` and `parseDeliveryCue`, and fails on a head verb in
  neither set, naming its use count, its drivable count and an example. It
  also checks the sets are disjoint, that the narration's `spoken` flag obeys
  both, that compound actions are read word by word, and that the four cues
  above are now deferred.
- `tests/drivable-writing-imperatives.ts` gains `recallCueAsksToPoint` and
  `pointingRecallCues`: a `[YOU RECALL: …]` cue that asks the learner to
  "point to" or "point at" the page, at the start of the cue or straight after
  a step link (", ", ", and ", "; ", " and ", " then "). Exactly "point", so
  "points at something near", "its old pointing stem" and the gloss "to point
  out" stay quiet; quotations are blanked first. One alternation of fixed
  literals, linear; seven ~50,000-character adversarial inputs assert answers
  only. `tests/drivable-writing-cues.test.ts` demands zero such cues in
  drivable lessons. "read" is deliberately not included: about 770 drivable
  recall cues say "then read **आँख**", which is a driving-edition decision of
  its own.
- Corpus: the 25 drivable recalls that asked a driver to point (24 Japanese,
  1 Hindi) are rewritten for the ear, and JA-C09-sumimasen's
  "[YOU HEAR: *sumimasen* → point to **repair** …]" says "choose" like its
  siblings. Details are in the Japanese and Hindi changelogs.
- Regenerated: narration for 177 chapters across 18 tracks (1,275 cues now
  deferred, 183 of them in drivable lessons), the generated narration hashes,
  the eight Japanese and one Hindi book chapters whose lessons changed and
  their book hashes, and the 26 edited lessons' `core/lesson-modality` owners
  (source hash only; every lesson is still `drivable: true`). The book prints
  the new manual verbs exactly as before ("*Copy:* …", "*Circle:* …"):
  deferral is a narration concern.
