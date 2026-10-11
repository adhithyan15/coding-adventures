## Fixed — drivable lessons stop asking a driver to gesture

A spoken cue is read to a driver as an ordinary turn, and so is bare prose.
Drivable lessons in this track still asked for a hand or a gesture inside one:
pointing near and far (issue #12070, ninth pass). Each ask is now said for the
ear and voice where that keeps the learning goal, or moved into a cue the
narration defers (`[YOU POINT: …]`, `[YOU READ: …]`: "once you have stopped
driving — …"). The new gesture check in human-language-data demands zero such
spoken cues in drivable lessons. Every edited lesson stays `drivable: true`
(only its `core/lesson-modality` source hash changes).

- **Count:** 2 prose instructions in 2 drivable lessons.
- MR-C128-cauk: "Point near and far: **हा** …" → "Name near and far: …".
- MR-C37-kon: "Point it at somebody you have already named" (the question,
  figuratively) → "Aim it at …", so no listener takes it as a request to
  point.
- Left alone: MR-R41-in-the-house's "could only be answered by pointing" (a
  description).
