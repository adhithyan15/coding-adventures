## Fixed — drivable recalls stop asking a driver to read script

`[YOU RECALL: …]` is a spoken cue action, so the narration reads a recall
to a driver as an ordinary turn ("your turn — recall: …") with no deferral.
The spaced script-recognition recalls in this track's drivable lessons held a
reading step inside that spoken cue (`[YOU RECALL: say *X*, then read
**Y**]`), so a driver heard "recall: say X, then read Y" and was asked to read
printed script at speed (issue #12070). Each such cue is now split: the spoken
steps stay in a `[YOU RECALL: …]` cue and each reading step becomes a
`[YOU READ: …]` cue, one bullet each, in the order the author gave. READ is a
manual cue action, so the narration says "once you have stopped driving —
read: Y", and the book prints "*Recall:* say *X*" and "*Read it:* **Y**". The
new reading check in human-language-data (`tests/drivable-writing-cues.test.ts`)
now fails on any drivable recall that asks for printed script to be read
("read **…**", "read the sign **…**", "read a printed …"). Every edited lesson
stays `drivable: true` (only its `core/lesson-modality` source hash changes).

- **Count:** 187 cues in 173 drivable lessons.
- `say *X*, then read **Y**` (86) and `read **Y**, then say *X*` (61) → a
  recall and a READ, in the authored order; `read **Y**` alone (4) →
  `[YOU READ: **Y**]`.
- Six spaced `[YOU RECALL: read **Y** — **R2**]`-style recalls (TA-C82-second,
  -third; TA-C83-date, -ninth, -seventh) → `[YOU READ: **Y** — **R2**]`, keeping the spacing tag on the cue
  that now does the recalling.
- Three-step recalls keep their order, and consecutive spoken steps stay in
  one recall: "say, read, say" (12) → recall, READ, recall; "read, say, say"
  (7) → READ, then one recall with both says; the conversational recalls of
  chapters 75-81 likewise ("answer *sariyā?*, then read **சரியா**, then ask
  *varalāmā?*" → recall, READ, recall; "read **மற்றது**, then ask *vilai
  evvaḷavu?*, then say both forms of *but*" → READ, then one recall).
  TA-C81-which-tamil's cue was wrapped across two source lines; both new cues
  sit on one line each.
- Judgement call, left for a separate decision: twelve drivable
  `[YOU RETURN TO: read **…**, say … — three distances back — then …]`
  reviews (TA-C74-also through TA-C81-which-tamil) also ask for a reading
  inside a spoken verb. RETURN TO binds three steps to one spacing note and a
  closing task, so it does not split as mechanically as a recall; the new
  check reads RECALL only.
- Regenerated: 43 book chapters and their hashes, the narration and narration
  hashes for the same chapters, and the 173 `core/lesson-modality` owners
  (source hash only).
