---
category: CI & GitHub Actions
---

# A whole-invocation -jobs 1 for one language's shared state serializes every package on the runner

OCaml BUILDs share one opam switch, and the executor had no lock for it, so
the OCaml lane told ci.yml to pass `-jobs 1` to the build tool whenever an
OCaml switch was active. On a pull request that only touched OCaml this cost
little. On main's sharded full build it decided the run: sharding scattered
the six OCaml packages over five of the fifteen shards, and each of those
shards then built its ~1,000 packages on ONE worker. Shards 11, 12 and 13 hit
the 150-minute ceiling at ~95% done every run (#17631) and looked like ordinary
"too much work" timeouts.

How it was found: the per-package Build Report of a slow shard summed to
147.4 minutes against a 147.4-minute wall clock. Replaying the progress lines
(`N/M  Building: a, b, c`) as a timeline gave a time-weighted concurrency of
exactly 1.00. Every shard with that ratio was an OCaml shard.

The fix scoped the serialization to the resource: a `global:opam-switch` key
in the executor's resource locks, held by every OCaml package and every
BUILD that runs `opam`. OCaml packages still never overlap; nothing else waits.
The workflow contract now pins the lock, and forbids a worker override, in
place of pinning `-jobs 1`.

What to do differently:
- Serialize on the shared resource (a lock key), never on the worker count.
  A worker count is global; the hazard almost never is.
- When a shard is slow, divide its summed package durations by its wall clock
  before rebalancing anything. A ratio near 1 means it ran serially, and no
  amount of rebalancing fixes that.
- A safety rule written for a narrow PR build must be re-checked when the
  same step starts running a forced build of the whole repository.
