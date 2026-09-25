## HL-C442-2380bc8c — The curriculum membership digest pin serializes every content PR

**Status: OPEN.** Found while adding Telugu chapters 84-87.

`tests/curriculum-membership-shards.test.ts` pins ONE corpus-wide SHA-256 and
ONE lesson count. Every PR that adds a lesson to any track must edit the same
two lines, so any two content PRs in flight conflict with each other, whatever
tracks they touch. That is the shared-hot-file problem the HL21-HL40 sharding
removed everywhere else, still present in the test that guards the sharding.

Proposed fix: pin a digest and count per track (one small file or map entry per
track), so concurrent PRs on different tracks never touch the same line, and a
PR's pin change names exactly the track it changed.
