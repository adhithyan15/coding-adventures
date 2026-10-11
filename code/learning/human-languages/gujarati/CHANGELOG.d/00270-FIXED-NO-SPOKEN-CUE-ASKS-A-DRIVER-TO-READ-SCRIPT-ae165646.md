## Fixed — no spoken cue asks a driver to read script

`[YOU SAY: …]` is a spoken cue action, so the narration reads a SAY prompt to
a driver as an ordinary turn. Two drivable chapter-39 prompts held a reading
step after the spoken one — "say: છ on its own, then read છે", "say: chha,
sāt — then read સાત sign by sign" — asking someone at the wheel to read
printed script (issue #12070). The reading check in human-language-data now
reads every spoken cue, not only recalls, and each prompt is split in the
authored order: the spoken part stays in `[YOU SAY: …]`, the reading becomes
`[YOU READ: …]`, which the narration defers ("once you have stopped driving —
read: સાત sign by sign"). Both lessons stay `drivable: true` (only their
`core/lesson-modality` source hash changes).

- **Count:** 2 cues in 2 drivable lessons.
- GU-C39-chha: `[YOU SAY: **છ** on its own, then read **છે** — same shape,
  different job]` → `[YOU SAY: **છ** on its own]`, `[YOU READ: **છે** —
  same shape, different job]`.
- GU-C39-saat: `[YOU SAY: *chha, sāt* — then read **સાત** sign by sign]` →
  `[YOU SAY: *chha, sāt*]`, `[YOU READ: **સાત** sign by sign]`.
- Judgement call: the note "— same shape, different job" travels with the
  reading, since it describes **છે** against the **છ** just said and only
  makes sense once **છે** is in front of the learner. Saying the bold letter
  **છ** stays a SAY: the narration speaks the letter, so the ear can do it.
- Regenerated: book chapter 43 and its hash, the narration and narration
  hash for the same chapter, and the 2 `core/lesson-modality` owners (source
  hash only).
