## Unreleased — `-plan-closed-language L`: opt-in prerequisite-closed single-language plan

- New flag `-plan-closed-language L`, valid only with `-emit-plan` (refused otherwise, and refused
  for `all`). The emitted plan holds only the packages of language L whose every transitive
  prerequisite is also of language L, on all three runner platforms. A package with a foreign
  prerequisite is left out, not pulled in with it. Edges, affected lists, per-platform state, shards
  and weighted shard costs cover the selected set only; CI gate verdicts still judge the whole
  change. See `plan_language.go`.
- Unchanged: `-language` with `-emit-plan` (it still emits every package, so a TypeScript plan keeps
  its Rust providers; `game_input_frame_plan_test.go` is byte-identical to before), the default and
  `-language all` plans (byte-identical on the real repo), the plan JSON schema (no new fields),
  `-emit-kiln-graph`, normal builds and `-plan-file` replay.
- A graph `Successors` error during selection now fails the plan instead of being skipped.
- Prints `Closed-language plan L: N packages selected, M excluded for a non-L prerequisite`.
- Tests in `language_plan_test.go`: closure, transitive exclusion, determinism, default and `all`
  unchanged, `-language go` not narrowing, weights, platform union, unknown node fail-closed,
  injected successor error, flag validation.
