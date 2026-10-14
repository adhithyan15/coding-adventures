---
category: CI & GitHub Actions
---

# A sharded check must resolve names against the whole repo, and check only its shard

`build-tool -validate-build-files` ran once per CI shard, against the
shard's package list. It used that same list to resolve names: the Perl
`cpanfile` names and the paths a BUILD file points at. A package's test-only
dependencies (`on 'test'`) are not graph edges, so they were not in the
shard. The packages those dependencies bring in were in the shard for other
reasons, and so looked like undeclared references. Shards 14 and 15 of
every main run failed with `perl/brainfuck-wasm-compiler … undeclared local
package refs: perl/wasm-execution` from July to October, and no PR ever
saw it, because PRs do not shard (X-7).

Fix: `ValidateBuildFilesAgainst(packages, known, graph)`. It checks the
shard's packages and resolves against every package on the platform.

Do differently: a check that runs on a subset still needs the whole
universe for lookups. Reproduce a main-only failure locally by emitting
the same plan (`-force -shard-count 15 -emit-plan`) and running each
`-shard-index` with `-dry-run`. That takes seconds, and it is how this one
was confirmed.
