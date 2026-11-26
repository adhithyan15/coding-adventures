## Unreleased — `-language L -emit-plan` emits a prerequisite-closed single-language plan

- Before: `-emit-plan` ignored `-language` when choosing packages, so `-language go -shard-count 4`
  sharded all 5893 packages of all 23 languages; only the toolchain flags were language-aware.
- Now: with `-language L` (L other than `all`) the emitted plan holds only the packages of language
  L whose every transitive prerequisite is also of language L, on all three runner platforms. A
  package with a foreign prerequisite is left out; its prerequisite is not pulled in. Edges,
  affected lists, per-platform state, shards and weighted shard costs cover the selected set only.
  CI gate verdicts still judge the whole change. See `plan_language.go`.
- Unchanged: `-language all` (verified byte-identical on the real repo), the plan JSON schema (no
  new fields), `-emit-kiln-graph` (still forces `all`), and normal and `-plan-file` builds, which
  keep silently dropping foreign prerequisites.
- Prints `Language plan L: N packages selected, M excluded for a non-L prerequisite`.
- Tests in `language_plan_test.go`: closure, transitive exclusion, determinism, `all` unchanged,
  weights, platform union, unknown-node fail-closed.
