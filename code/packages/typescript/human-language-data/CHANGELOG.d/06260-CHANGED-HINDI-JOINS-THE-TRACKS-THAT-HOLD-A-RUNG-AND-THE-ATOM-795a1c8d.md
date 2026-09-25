### Changed — Hindi joins the tracks that hold a rung, and the atom-budget scoping test uses it as proof

`tests/level-gate.test.ts`: `tracksWithAnyLevel` 2 -> 3 and `hindi: "pre-A1"`
joins the pinned map. "scopes the atom budget to the level" asserted that Hindi
was still working pre-A1 with no atom-budget blocker; Hindi now ATTAINS pre-A1
with its one over-budget A1 lesson still in the track, and that lesson shows up
as a blocker only at A1, which is the claim the test is named for, shown end
to end. `curriculum-digests/hindi.json` moves for chapters 106-127.
