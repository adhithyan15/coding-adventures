---
category: CI & GitHub Actions
---

# Sharding that assigns packages and then adds their prerequisites rebuilds the shared core on every shard

Main's full build is split into prerequisite-closed shards: each shard builds
whatever its packages depend on, so no runner needs another's artifacts. The
v1 sharder dealt packages across shards by a heuristic cost and only then
added prerequisites. Nothing pulled a package towards the shard that already
held its dependencies, so every shard rebuilt most of the shared core. On the
2026-10-18 plan, 15 shards of ~355 assigned packages expanded to 872-1,131
each: 14,878 package builds for a 5,327-package repo (2.8x). One 21-minute
crate, `rust/smart-home-integration-catalog`, was built on twelve shards per
run. Duplicated work cost both time (timeouts) and disk (more test binaries
per runner).

The number that exposes it is free: `sum(len(shard.package_names))` against
`len(packages)` in the emitted plan. Nobody looked, because each shard's own
count looked plausible.

The fix (sharding v2, `-shard-weights`) assigns whole closures, heaviest
first, to the shard that lacks the least of them while staying within a fair
share, balanced on measured seconds from the Build Report. Same plan: 5,908
builds, every shard 69 serial minutes.

What to do differently:
- For any partitioning that duplicates shared inputs, measure the duplication
  factor before tuning the balance; balancing a 2.8x-inflated workload just
  spreads the waste evenly.
- "Least resulting load" alone does not keep clusters together: while shards
  are empty, an empty shard always looks better than the one holding a
  dependent's expensive prerequisite. Cap by a fair share and prefer least
  marginal work under the cap.
- Prove a scheduling change on the real plan artifact (CI uploads
  `build-plan.json`), not only on a toy fixture.
