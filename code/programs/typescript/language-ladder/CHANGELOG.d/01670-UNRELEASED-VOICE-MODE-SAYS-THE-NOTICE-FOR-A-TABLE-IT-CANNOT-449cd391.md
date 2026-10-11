## Unreleased — voice mode says the notice for a table it cannot read

- `buildVoiceScript` skipped `table-skipped` narration segments in silence:
  they fell through to the `default` branch kept for unknown kinds. That is the
  segment the generator emits in place of a table too wide or irregular to say
  as sentences, and it carries the sentence a voice should read instead
  ("There is a table here I cannot read to you — 5 columns and 3 rows, and …
  Come back and look at it when you have stopped."). The narration's
  plain-text script prints that line, so a driver listening to the `.txt`
  heard that something had been left out and a driver in voice mode did not.
- It is now a `speak` step, trimmed, in place, with no answer gap (there is
  nothing to answer), and an empty notice is dropped like empty speech. It is
  not added to the end-of-lesson queue: the notice itself already says to come
  back to the table, exactly as the plain-text script leaves it.
- Tests: a fixture block with the notice between two speech segments, an empty
  notice, and French chapter one's real narration, where every
  `table-skipped` notice must be spoken.
