### Fixed — drivable recall cues stop asking a driver to read script

- Issue #12070, seventh pass. RECALL is a spoken cue action, so the narration
  reads `[YOU RECALL: …]` to a driver as an ordinary turn. 761 recall cues in
  736 drivable lessons held a reading step — `[YOU RECALL: say *dīyā*, then
  read **कान**]` was narrated "your turn — recall: say dīyā, then read कान" —
  and reading printed script is the eyes' job. The previous pass left them
  out of the pointing check as a decision of their own; this is it.
- Corpus: each cue is split into bullets in the authored order. Spoken steps
  stay in `[YOU RECALL: …]` (consecutive ones stay in one cue, joined by
  ", then"); each reading step becomes `[YOU READ: …]`, the form existing
  corpus READ cues use (`[YOU READ: **X**]`, `[YOU READ: **X**, then say what
  it means]`). READ is in `MANUAL_CUE_ACTIONS`, so the narration now says
  "[once you have stopped driving — read: कान]", and the book prints
  "*Recall:* say *dīyā*" and "*Read it:* **कान**". Per track: Tamil 187 cues
  (173 lessons), Sanskrit 155 (155), Telugu 136 (136), Hindi 128 (118),
  Kannada 83 (82), Malayalam 66 (66), Urdu 3 (3), Japanese, Marathi and
  Marwadi 1 each.
- Judgement calls, each recorded in its track's changelog: a spoken step that
  is about the word just read ("and say what it means", "and say its sound",
  "then say it without looking", "and say what the swap did") travels with
  the reading inside the READ cue, since it cannot be done without it; ", and
  say" there becomes ", then say". A spacing tag (**R1**–**R4** and its
  distance note) stays on the cue that does the recalling; every tagged cue
  was a reading only (13 cues), so the tag moved onto its READ cue, as the
  writing recalls' tags moved onto WRITE. Recalls whose "read" object is an
  italic English meaning (the 11 Japanese chapter 142 sign recalls, "read
  *reception* on a sign") or a gloss ("the Japanese for to read") are left
  as they are: nothing is on the page.
- Ratchet: `tests/drivable-writing-imperatives.ts` gains
  `recallCueAsksToReadScript` and `readingRecallCues`. A recall fires when
  "read" sits where a step starts (the start of the content, or after ", ",
  ", and ", "; ", " and ", " then ", optionally followed by "then ") and its
  object is on the page: bold script straight after the verb (optionally
  after "aloud"/"out"), an article and at most two plain words reaching bold
  script ("the sign **ೇ**", "the form label **आवडती कृती**"), or "printed"
  ("a printed ticket"). The step regex is one alternation of fixed literals
  ending in a lookahead, with no nested quantifier; the object test is plain
  code over at most 80 characters after each match, so the scan is linear.
  `tests/drivable-writing-cues.test.ts` adds 15 corpus positives, 15
  controls (glosses, italic meanings, "read out the number you heard", a
  phrase that stops at a comma, "thread", a quotation, the READ cue itself,
  a SAY cue), 11 ~50,000-character adversarial inputs asserting answers
  only, a corpus check demanding zero such cues in drivable lessons, and an
  anti-vacuity floor (more than 40; 66 measured) on non-drivable lessons,
  whose reading recalls are legitimate. Run on the corpus before the split,
  the check flagged exactly the 761 cues the split touched and none of the
  16 drivable "read" recalls it left.
- Not in scope, named in the detector's comment: twelve drivable Tamil
  `[YOU RETURN TO: read **…**, say … — three distances back — then …]`
  reviews and two Gujarati `[YOU SAY: … then read **…**]` prompts also put a
  reading inside a spoken verb; the check reads RECALL only.
- Modality: no lesson changes drivability. A READ cue is not a structural
  sight signal (no script block, no table, no "look at"/"see the" phrase), so
  `core/lesson-modality` changes only the source hash of the 736 edited
  lessons.
- Regenerated: 227 book chapters and their book hashes, the narration
  (`.json` and `.txt`) and narration hashes for the same chapters, and the
  736 `core/lesson-modality` owners.
