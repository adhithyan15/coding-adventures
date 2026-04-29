# Build Plan Sharding

## Goal

The build system must scale from thousands of packages to tens of thousands of
packages without requiring one CI runner to hold every build artifact at once.
A single monorepo build plan is still the source of truth, but that plan can be
split into multiple independently executable shards.

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

Shards may duplicate prerequisite packages. That is intentional for the first
version: correctness and runner isolation are more important than perfect work
deduplication. A later artifact-sharing layer can remove duplicate work.

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

The detect job emits `build_shards`, a compact JSON array for a GitHub Actions
matrix. Build jobs combine the OS axis with the shard axis:

```yaml
strategy:
  matrix:
    os: ["ubuntu-latest", "macos-latest"]
    shard: ${{ fromJSON(needs.detect.outputs.build_shards) }}
```

Each matrix job downloads the same `build-plan.json` and passes
`-shard-index ${{ matrix.shard.shard_index }}`.

## Scaling Notes

- Start with 4-5 shards.
- Keep `fail-fast: false` so independent shards reveal multiple failures.
- Preserve a scheduled/manual full build path, but run it through shards.
- Keep per-runner cleanup as defense in depth; sharding is the real capacity
  fix.
- Future versions can use historical durations and artifact exchange to reduce
  duplicate prerequisite work.
