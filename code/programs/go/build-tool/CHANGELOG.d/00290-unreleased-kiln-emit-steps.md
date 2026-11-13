## Unreleased — Kiln expected-steps export (KILN04 stage 4a, G6)

- **`-emit-kiln-steps <path>`** writes `kiln-legacy-steps/1`: per package
  `rel_path` (sorted), the lines `executor.EffectiveStepLines` returns for this
  OS (`os` is `runtime.GOOS`), so the Swift and Windows rewrites and Starlark
  evaluation are already applied. Kiln's recorder uses them as the expected
  recipe, because Kiln cannot derive them from BUILD bytes.
- Only valid together with `-emit-kiln-graph` (otherwise exit 1 before any
  work): both files are written by one invocation, from one discovery pass,
  before any BUILD runs. The graph is written first; a stale steps file is
  removed at startup so a missing file means the run did not finish. A symlink
  or non-regular path is refused; output is atomic and byte-stable.
- Tests: equality with `EffectiveStepLines` for a plain BUILD, a Swift line
  (rewritten on Linux) and a Starlark package; refusal without the graph flag;
  empty path; byte stability; stale-file removal; symlink refusal; failure
  paths of the emitter.
- Spec: KILN04 sections 4.6 and 4.8 (G6).
