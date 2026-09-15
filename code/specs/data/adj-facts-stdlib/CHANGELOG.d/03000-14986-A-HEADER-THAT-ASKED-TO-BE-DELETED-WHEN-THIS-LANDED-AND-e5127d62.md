- **#14986: a header that asked to be deleted when this landed, and the note that was too kind to itself.**
  `earth-science/speleothem-alt-name.adj` held all four justifying sentences already — the COLUMN one
  as the envelope `source`, the other three as table-level `cites`. So every row carried all four, and
  the seven rows that are not about columns were warranted, **in the field that carries the tier**, by
  a sentence about a stalagmite meeting a stalactite.

  ### The file had filed its own bug, and understated it

  Its header carried a long note ending: *"Row-level provenance is filed as issue #13893; when it
  lands, this header should be revisited and the note deleted rather than left to rot."* It landed.
  This is that deletion.

  But the note called the problem **harmless in JSON** — *"nothing cited is false, and the grounding
  sentence is always among them, but the reader has to match sentence to row themselves"* — and that
  was too kind to it. The column sentence was the `source` for all eight rows, so a `corallite` answer
  was not merely *accompanied* by an unrelated sentence: it was **warranted** by one. The note framed a
  wrong warrant as a presentation inconvenience. (#13898, the `--explain` renderer, stays open and is
  now irrelevant to this file: there are no corroborations left to drop.)

  The note's other claim survives and is kept: four single-purpose tables would fragment one lookup
  into four and make the backward query — the useful one — unaskable in a single goal.

  ### Pins

  **14 of 14 mutants killed, one control.** All eight rows broken one at a time — including **each of
  the five that share the frozen-waterfall sentence**, since one broken copy behind four intact ones
  is the failure `skeleton-bones` (#15171) shipped and `speleothem-substrate` (#15175) designed out.
  Plus warranting the coralloid row with the column sentence, restoring the old table-level `cites`,
  rebinding an alt name, dropping a row's source, repointing the locator, and fabricating the
  envelope — a kill, because the envelope's wording is pinned against the shipped file (#15176).

  Queries bind the ALT NAME, which is unique per row; binding the speleothem would return five rows
  for `frozen_waterfall` and let a needle be satisfied by a sibling's intact copy.

  Measured: all four row spans and the framing sentence occur **exactly once** on the NPS page;
  near-miss controls score zero. Suite run with `RUSTFLAGS="-Dwarnings"`, the way CI compiles it.

  The README row for this table stated the same limitation and is updated rather than left behind.

  ### A pre-existing assertion that proves less than its message claims

  Review measured it: `a_frozen_waterfall_answers_to_five_other_names` does a whole-stdout `contains`
  over a **five-answer** query, and it did **not** fire when one copy of the shared sentence was
  broken, or silently edited ("most common" → "least common"). Only the new per-row test killed those.
  The old assertion is left in place — it still checks the five bindings, which is what its name says —
  but it is not what pins the spans, and the new helper's doc comment records why.

  ### And the note's deletion left its own banner behind

  Review caught it: the removed limitation note's headline, `*** A LIMITATION OF THIS TABLE IN
  PARTICULAR, STATED UP FRONT. ***`, survived directly above the paragraph announcing that the
  limitation is gone. The one line of the note not rewritten was the line that announced it — "left to
  rot" in the exact file whose header asked not to be.

  Four other comment defects came out of the same review, and one is worth naming because it is this
  cascade's own discipline turned inward: the envelope gloss said the framing sentence shows "that
  this domain HAS many names", while the sentence counts *"38 different **types** … subtypes and
  varieties"* — taxonomy, not nomenclature. **A gloss that overstates its own citation**, in the file
  that exists to stop exactly that.

