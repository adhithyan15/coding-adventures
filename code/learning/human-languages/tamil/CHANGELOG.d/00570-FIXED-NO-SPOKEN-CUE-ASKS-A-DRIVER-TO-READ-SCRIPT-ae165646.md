## Fixed — no spoken cue asks a driver to read script

The previous pass split the reading steps out of this track's drivable
recalls and left twelve `[YOU RETURN TO: …]` reviews for a decision of their
own. RETURN TO is a spoken cue action too, so the narration read, for
example, "return to: say mūḍu, say āṉāl and read எப்போது — three distances
back — then set two of them against each other" to a driver (issue #12070).
The reading check in human-language-data now reads every spoken cue, and
each review is split in the authored order: spoken items stay in
`[YOU RETURN TO: …]`, the reading becomes `[YOU READ: …]` (which the
narration defers: "once you have stopped driving — read: எப்போது"). Every
edited lesson stays `drivable: true` (only its `core/lesson-modality` source
hash changes).

- **Count:** 12 cues in 12 drivable lessons, TA-C74-also through
  TA-C81-which-tamil.
- Reading first (TA-C74-also, TA-C75-permission, TA-C76-choice,
  TA-C77-opinion, TA-C78-therefore) → `[YOU READ: **X**]`, then the review
  with its two spoken items, distance note and closing task unchanged.
- Reading in the middle (TA-C79-then, TA-C80-one-and-other, TA-C80-other,
  TA-C81-which-tamil) → `[YOU RETURN TO: say A]`, `[YOU READ: **X**]`,
  `[YOU RETURN TO: say B — three distances back — then …]`.
- Reading last (TA-C75-polar, TA-C78-why, TA-C80-price) →
  `[YOU RETURN TO: say A and say B — three distances back]`,
  `[YOU READ: **X**]`, `[YOU RETURN TO: <closing task>]`.
- Judgement calls: the distance note "— three distances back —" stays on a
  RETURN TO cue, the cue doing the recalling, right after the spoken item it
  followed; it never moves onto the READ cue, because it spans all three
  items rather than tagging the reading. When the reading was the last item,
  the closing task ("then set two of them against each other …") comes after
  the READ cue so the order is kept, as its own RETURN TO cue with the
  leading "then" dropped. Two spoken items left side by side are joined
  "say A and say B", the shape the reading-first reviews already had. The
  closing task still says "two of them" / "one of them", counting the item
  that was read.
- Left alone: TA-W31-read-aanaal and TA-W34-read-eppothu carry the same
  review shape but are not drivable (their narration already opens with the
  hands-and-eyes notice).
- Regenerated: book chapters 74-81 and their hashes, the narration and
  narration hashes for the same chapters, and the 12 `core/lesson-modality`
  owners (source hash only).
