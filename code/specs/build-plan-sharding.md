# Build Plan Sharding

## Goal

The build system must scale from thousands of packages to tens of thousands of
packages without requiring one CI runner to hold every build artifact at once.
A single monorepo build plan is still the source of truth, but that plan can be
split into multiple independently executable shards.

This document defines target behavior. Sharding is currently implemented most
completely by the Go build tool; final cross-language requirements and
deterministic fixtures are defined by
[`build-tool-conformance.md`](build-tool-conformance.md).

## Model

The detect job computes the normal build plan:

1. discover packages
2. evaluate BUILD/Starlark metadata
3. resolve the dependency graph
4. determine the affected package set, or all packages in force mode
5. compute toolchain requirements

It then partitions the scheduled packages into shard roots. For every shard,
the build tool expands those roots with their transitive prerequisites. This
means each CI runner receives a prerequisite-closed DAG slice and does not need
artifacts from another runner.

Shards may duplicate prerequisite packages. Correctness and runner isolation
come first, so duplication is always permitted. How much of it there is depends
on the assignment strategy below, and in practice it decides whether a shard
fits its time and disk budget.

## Assignment strategies

### v1: packages by heuristic cost

The original strategy, pinned by the cross-port conformance fixtures
(`build-tool-conformance.md`, "Sharding"): scheduled packages are sorted by a
heuristic cost and each goes to the shard with the lowest direct-root cost.
Prerequisites are added afterwards. Nothing steers a package towards the shard
that already holds its prerequisites, so every shard ends up rebuilding a large
share of the shared core. On main's 2026-10-18 plan (5,327 packages, 15
shards) every shard was assigned ~355 packages and expanded to 872-1,131,
for 14,878 package builds: 2.8x the repository.

### v2: closures by measured duration (`-shard-weights`)

Selected by passing a `ci-package-durations-v1` file
(`code/specs/data/ci-package-durations.json`, produced by
`code/scripts/ci_package_durations.py` from the Build Report each shard prints):

```json
{
  "schema": "ci-package-durations-v1",
  "unit": "seconds",
  "source": { "runs": ["39139594621"], "statistic": "median of BUILT rows" },
  "durations": { "rust/regex-engine": 1150.0, "python/zstd": 289.3 }
}
```

1. Each package weighs its measured seconds. An unmeasured package weighs its
   toolchain's median; a toolchain with no measurement falls back to the v1
   cost scaled by the median seconds-per-cost-unit. Every weight is at least
   0.1s.
2. The *fair share* is the summed weight of every package some root needs,
   divided by the shard count, plus 5%.
3. Scheduled packages are visited heaviest prerequisite closure first (ties by
   name). A package already present in a shard is assigned directly to the
   lightest such shard. Otherwise, among shards that stay within the fair share
   after adding the closure members they lack, the one lacking the least weight
   wins (then the lighter shard, then the lower index). If none stays within
   it, the shard with the lowest resulting load wins.
4. `package_names` is the union of the closures placed on the shard;
   `estimated_cost` is its load in whole seconds.

The result keeps every v1 guarantee: deterministic, exactly one direct
assignment per scheduled package, prerequisite-closed, toolchains computed from
the closed shard, stable indexes and names. On the same 2026-10-18 plan it
makes 5,908 package builds, with every shard at 69 serial minutes (v1:
134-233). An unreadable or malformed weights file is an error
(`SHARD_WEIGHTS_UNREADABLE`, `SHARD_WEIGHTS_INVALID`), never a silent return to
v1.

v2 is implemented by the Go build tool only. Other ports conform to v1;
porting v2 is not a parity requirement until its fixtures exist.

## Plan Fields

`build-plan-v1` remains the base schema. Sharding is additive through the
optional `shards` field:

```json
{
  "shards": [
    {
      "index": 0,
      "name": "shard-1-of-5",
      "assigned_packages": ["ruby/twig"],
      "package_names": ["ruby/interpreter-ir", "ruby/vm-core", "ruby/twig"],
      "languages_needed": { "ruby": true },
      "estimated_cost": 17
    }
  ]
}
```

`assigned_packages` are the packages directly assigned to the shard.
`package_names` are the assigned packages plus their prerequisites. The build
job executes `package_names`.

## CLI

Emit a sharded plan and GitHub Actions matrix data:

```bash
./build-tool \
  -root . \
  -diff-base "$BASE" \
  -emit-plan build-plan.json \
  -shard-count 5 \
  -emit-shard-matrix
```

Run one shard:

```bash
./build-tool \
  -root . \
  -plan-file build-plan.json \
  -shard-index 2 \
  -validate-build-files \
  -language all
```

Force mode is represented in the plan when the detect job runs with `-force`.
Consumers may also pass `-force` with `-plan-file`; that overrides the plan's
affected package list and rebuilds the selected shard.

## CI Shape

Sharding is reserved for forced full builds: every `main` push, and a pull
request carrying the `ci-full-build` label (so a change to the main build itself
can be proven before it merges; label, then push). Other pull requests and
branch pushes continue to run the normal affected-package build plan without a
shard index, because those builds are already narrow and should not wait on
multiple macOS/Windows shard jobs.

For `main` pushes, the detect job emits `build_shards`, a compact JSON array
for a GitHub Actions matrix. The CI workflow turns those shard entries into
Ubuntu runner jobs:

```yaml
strategy:
  matrix:
    include: ${{ fromJSON(needs.detect.outputs.build_matrix) }}
```

Each sharded main job downloads the same `build-plan.json` and passes
`-shard-index ${{ matrix.shard_index }}`.

## Scaling Notes

- Start with 4-5 shards.
- Keep `fail-fast: false` so independent shards reveal multiple failures.
- Preserve a scheduled/manual full build path, but run it through shards.
- Keep per-runner cleanup as defense in depth; sharding is the real capacity
  fix.
- Historical durations now drive assignment (v2). Artifact exchange between
  shards could remove the remaining duplicate prerequisite work.

## Conformance

The shared fixtures require deterministic assignment, exactly one direct
assignment per scheduled package, prerequisite closure, shard-local toolchain
calculation, stable indexes, and explicit invalid-count/index errors. A port
that can read a plan but ignores its `shards` field is not conforming.
