### Added - source-backed four-skill task shapes (#12212)

- Add HL18 and a strict `<track>/task-shapes/<level>.json` parser. Reading,
  listening, writing and speaking parts now name their prompt, response,
  interaction, timing, replay, scoring, aid and source shapes; unpublished
  speed or length data stays explicitly unknown instead of becoming an estimate.
- Add the first complete proof inventory: the eleven task parts of the official
  Goethe-Zertifikat A1: Start Deutsch 1, including its 65-minute written exam,
  15-minute group speaking exam, approximately 30-word writing task, and real
  aggregate 60/100 pass rule.
- Preserve the distinction between Goethe's aggregate award rule and this
  project's stricter independent-skill evidence. Goethe does not publish four
  independent pass thresholds, so the inventory records four nulls and directs
  the later assessment contract to add internal thresholds explicitly.
- Add a finite, round-robin `task-shape/<language>/<level>` backlog. One valid
  inventory removes one item; absent or invalid data can never read as clean.

