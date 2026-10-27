### Added — a v0.4.0 release upgrade fixture

`fixtures/release-upgrade-v0.4.0.json` is what a v0.4.0 user's state file holds
after creating one deck and one card. It was captured by running the v0.4.0
adapter itself in a worktree at `engram-v0.4.0`, not written by hand, and it is
the first fixture in the shared `mosaic/release-upgrade-fixture` format
(`code/specs/mosaic-release-state-gate-v1.md`).

It accepts a live file at version 1 (a host that has not re-persisted yet) or
version 3 (what the current adapter writes), and says where the deck and card
live in each: `/decks/0/...` in v1, `/session/state/decks/0/...` in v3.

`engram-mosaic-app` gains `the_v0_4_0_release_fixture_survives_the_upgrade`,
which restores the fixture as a v1 snapshot, checks the deck and card, and
checks every pointer expectation against both the v1 state and the v3 bytes a
fresh `snapshot()` writes. `release-engram.yml` now also runs
`test_mosaic_release_state_gate.py`, which verifies the fixture end to end
through the shared gate. The release lane does not yet seed it into the native
bundles; that is the next step.
