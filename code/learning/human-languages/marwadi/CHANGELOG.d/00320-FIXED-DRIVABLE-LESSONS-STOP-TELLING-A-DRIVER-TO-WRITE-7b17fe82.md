## Fixed — drivable lessons stop telling a driver to write

The modality manifest marks 220 Marwadi lessons `drivable: true`, but each
still asked for writing in bare prose: 377 flagged passages such as "With the
page covered, write **राम** and **सा** separately.", "Look, cover, wait five
seconds, and write **पति**.", "Say and write market, then recall …" and the
"4. **Write:** hear the question, wait ten seconds, and write it" line of the
four-skill checkpoints. Narration reads bare prose unhedged, so the audio
edition told a driver to write (issue #12070). Each writing task is now a
`[YOU WRITE: …]` cue, the form the Chinese track and the twelve smaller
tracks already use: the narration defers it ("[once you have stopped
driving — write: …]") and the book prints it as "*Write it:* …". The cue does
not create a writing block, so every lesson stays drivable.

- **Lessons:** every drivable lesson in chapters 1-39 and their review
  lessons that the detector flagged — the warm-ups of the -hear-, word and
  practice lessons of chapters 1-26, the look-cover-write Guided Practice of
  every word lesson, the four-skill checkpoints (MW-C01 to C07-practice,
  MW-C08-family-four/-seven, MW-C09-family-twelve, the travel, weather,
  shopping, transport, food and counter payoffs), the counting and ticket
  lessons of chapters 32-39, and the MW-R08 to MW-R39 review and script-close
  lessons.
- A spoken half stays prose ("Say mother. [YOU WRITE: the word for
  father]"). A sentence that followed the old instruction and asks for
  something else ("Then recall the seven-word family map.", "Say which
  belongs to formal thanks …", "Point to yourself on *mhāro*.") is now its own
  paragraph, so every cue ends its paragraph and the book never runs prose on
  after "*Write it:* …" without a stop.
- Advice about the written word goes inside the cue after a dash: the check
  after a look-cover-write ("— check that **ब** did not turn into **प**"), the
  uncover-compare-repair step, and "read each pair aloud" or "mark which **दो**
  counts" where the learner needs the page they just wrote.
- Where the warm-up named a meaning rather than a word ("write market",
  "write *take it*"), the cue says "the word for market" / "the words for …",
  so the book does not read as "Write it: market" and invite the English
  word. Named Devanagari targets stay as they were.
- "Copy once. Cover it, wait five seconds, and write it." (MW-C09-bachcha,
  MW-C09-patni) becomes "[YOU WRITE: one copy of **बच्चा**; then cover it,
  wait five seconds, and write it again — …]", so the cue still says the
  learner is copying a model before recalling it.
- "Look, cover, wait five seconds, and write X" becomes "Look, cover, and wait
  five seconds. [YOU WRITE: X — …]", the shape the Chinese fix used for
  "Cover it and wait five seconds."; where the model is covered the cue says
  "the word" rather than repeating it.
- The four-skill checkpoints labelled their writing line `**Write:**` or
  `**Write.**` (14 lines). That label was itself the bare imperative, so the
  item is now just the cue, which the book prints as "*Write it:* …" and so
  keeps the Listen / Speak / Read / Write order. A hearing step that preceded
  the writing ("hear the question, wait ten seconds") moves inside the cue
  ("the question with no model, ten seconds after hearing it"), because a
  driver who heard it now cannot write it later. The `**Writing.**` labels of
  chapters 32-35 are nouns and stay, with the cue after them.
- Each numbered step stays one list item. Where a step heard something and
  then wrote it ("3. Hear *sāt*, *chha*, *bīs* and write the FIGURE each
  time."), the hearing stays prose and the cue closes the item.
- Writing tasks the detector does not match are cues too: eleven warm-ups
  asked the learner to "form" signs (MW-C11-barsaat, -hawa; MW-C12-garmi,
  -mausam, -thandi; MW-C13-bhaav, -dukan, -hear-dukan, -hear-vastu, -vastu;
  MW-C14-samaan); MW-R36-letters-close ("Five signs, alone, from spoken
  cues …") and MW-R36-script-close ("Everything this slice added, from
  dictation …") set a dictation with no verb at all, so a listener heard the
  task with no hint that it needed a pen. MW-C01-practice's wrap-up said "mark
  that sign"; it now says "note which one".
- Four of these lessons (MW-C39-refusal-four, MW-R37-count-fifteen,
  MW-R38-count-twenty, MW-R39-script-close) opened with spaced-recall cues
  such as `[YOU RECALL: write **पंदरा** — **R1**]`. The detector ignores
  everything inside a cue, but RECALL is a spoken action, so the narration
  said "your turn — recall: write पंदरा" with no deferral. Those eight cues
  are now `[YOU WRITE: **पंदरा** from memory — **R1**]`, keeping their
  spacing tag. Eleven other drivable Marwadi lessons in chapters 37-39 (not
  in the debt ledger) carry the same RECALL-write shape and are left for a
  follow-up, together with the same shape in other tracks.
- MW-R25-bring-two now asks "Say which of the five the tea order uses." as a
  spoken question after the cue rather than inside it, since it can be
  answered without the page. MW-R25-script-close moves "mark which two carry
  the breath" into the writing cue and keeps "Say all five." as prose.
- No flagged sentence was a non-writing use, and no lesson was left in debt,
  so `tests/drivable-writing-debt/marwadi.json` in human-language-data is
  deleted. The 96 non-drivable Marwadi lessons that still say "write" in
  prose (the MW-W writing lessons, script lessons such as MW-C32-ek, and four
  review lessons) are untouched: they are legitimate pen work, and their
  narration already opens with the hands-and-eyes notice.
- Regenerated: the 39 affected book chapters, narration (`.json` and `.txt`),
  their generated book and narration hashes, and each lesson's
  `core/lesson-modality` owner (source hash only; all still `drivable: true`).
