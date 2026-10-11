### Fixed — no spoken cue asks a driver to read script

- Issue #12070, eighth pass. The previous pass split reading steps out of
  drivable `[YOU RECALL: …]` cues but read RECALL only. The narration reads
  every cue `isManualCueAction` does not defer as an ordinary turn, so a
  reading step inside any other spoken verb still reached a driver: twelve
  Tamil `[YOU RETURN TO: read **இன்று**, say … — three distances back — then
  …]` reviews, two Gujarati `[YOU SAY: … then read **સાત** …]` prompts, and
  one Urdu `[YOU RUN: …, then read the closing line off the script alone]`.
- Detector (`tests/drivable-writing-imperatives.ts`):
  `recallCueAsksToReadScript` / `readingRecallCues` become
  `spokenCueAsksToReadScript` / `readingSpokenCues`, and the cue filter is
  now "not `isManualCueAction`" instead of "RECALL" — every head verb in
  `SPOKEN_CUE_ACTIONS` (the classification test guarantees no head verb sits
  outside both sets). SAY and ANSWER material stays quiet for three reasons,
  each held by a control: "read" must open a step (not "I read", "to read"),
  its object must be on the page (bold script, "printed", "script"; not an
  italic meaning or "read it — AH-weh"), and quotations are blanked first.
- Phrasings a security review found missing: em-dash and colon step links
  ("say a — read **x**", "say a: read **x**"), "now" after a link
  (", now read **x**"), a colon after the verb ("then read: **x**"), "it" as
  a particle with an optional colon ("then read it: **x**", "read it aloud:
  **x**"), a noun phrase of up to four plain words before the script ("read
  the very long shop sign **x**", bounded so the object still sits inside the
  80-character window), a colon closing that phrase ("read the sign: **x**"),
  and "script" as a page word beside "printed" ("off the script"). The step
  regex is still one alternation of fixed literals plus an optional fixed
  literal, ending in a one-character lookahead; the object test is plain
  code over a fixed number of words in at most 80 characters, so the scan
  stays linear.
- Measured over all 248 spoken corpus cues that contain "read" (123 RECALL,
  108 SAY, 14 RETURN TO, one each of RUN, LIST, CONTRAST): before the fix the
  widened check fired on exactly 15 drivable cues — the ones above — and, in
  non-drivable lessons, only on real reading steps (reading recalls, five
  Bengali digit drills, a Hindi stem, two Tamil reviews). No SAY or ANSWER
  material fired.
- Tests (`tests/drivable-writing-cues.test.ts`): 33 positives (the 15
  original recall shapes, the three Tamil review shapes, both Gujarati SAY
  prompts, the Urdu RUN, two non-drivable SAY drills, an ANSWER, and the nine
  new phrasings), 31 controls (the fixed forms of every new split, SAY/ANSWER
  material such as `"legō" — I read`, `**khândan** — to read`, `read it —
  AH-weh`, a LOOK cue, and a five-word phrase past the bound), 21
  ~50,000-character adversarial inputs asserting answers only, the corpus
  check (zero in drivable lessons) and the anti-vacuity floor (> 40; 67
  non-drivable lessons measured).
- Corpus: each cue split in the authored order, spoken parts in their own
  verb, each reading step a `[YOU READ: …]` cue; judgement calls recorded in
  the Tamil, Gujarati and Urdu changelogs. Modality: no lesson changes
  drivability; `core/lesson-modality` changes only the source hash of the 15
  edited lessons.
- Regenerated: Tamil chapters 74-81, Gujarati chapter 43 and Urdu chapter 5
  (book, narration and their hashes), and the 15 lesson-modality owners.
