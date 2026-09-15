### Language-neutral CI gate selection

- Added a closed process-free build-tool fixture domain for deterministic CI
  gate verdicts, including exact package and path matching, portable globstar
  behavior, null-versus-empty snapshots, fail-open machinery sentinels, and
  stable Actions output names.
- Made the production Go evaluator consume all seven shared cases while
  keeping Git discovery, registry I/O, graph construction, workflow output,
  and scheduling outside the fixture authority boundary.

