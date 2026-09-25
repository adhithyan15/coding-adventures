### Changed — the curriculum digest is pinned per track, one file each (HL-C442)

`tests/curriculum-membership-shards.test.ts` pinned one SHA-256 and one lesson
count over the whole live curriculum graph, so every content PR on every track
edited the same two lines and any two of them in flight conflicted.

The test now pins each track's own graph in
`tests/curriculum-digests/<track>.json` (`digest`, `lessons`), and asserts the
set of pin files equals the set of tracks. A PR that adds Telugu lessons edits
`telugu.json` only, and that file's diff is the attribution. Rewrite the pins
after a deliberate change with
`UPDATE_CURRICULUM_DIGESTS=1 npx vitest run tests/curriculum-membership-shards.test.ts`.
Verified to fail, naming the track, when a pin is off by one lesson.
