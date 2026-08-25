### Added - the completion plan is computed, not typed (HL15, HL-C208)

- Add `completion-plan.ts`: the work queue is now a pure function of the measured deficit, built from the level gate, the script-closure report and the external exam inventories rather than from a hand-ordered list in `BACKLOG.md`.
- Add `plan-cli.ts` and an `npm run plan` target, kept separate from `report` so picking up the next item does not mean reading 100 lines of diagnostics.
- Add `listExamInventories`, which reads each inventory's own declared `language`/`level` rather than parsing the filename — the file for Spanish is `exam-inventory-es-a1.json`, so a filename-keyed queue would have reported Spanish's A1 target as missing and queued it to be written twice.
- Order by three mechanical keys: level rank (the floor is universal), family priority, then furthest-behind-first — and ROTATE across tracks so every language moves once before any moves twice.
- Extend the definition of done with two criteria the four-criterion gate did not carry: external exam-point coverage, and script closure.
- Measured today: 22 tracks, 89 enumerable items, ~10,172 projected to C2; 459 glyphs shown but never taught corpus-wide; 1 of 132 exam inventories written.


