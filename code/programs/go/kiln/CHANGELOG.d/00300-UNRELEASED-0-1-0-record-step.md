## [0.1.0] - UNRELEASED

### Added

- `kiln record step --dir <abs> -- sh -c <line>` (KILN04 stage 4 PR 2b-2a), the
  per-step supervisor the build tool runs as its `-exec-wrapper`. It re-verifies
  `pre/` against `SEALED` and the binding (P11), checks the pinned Kiln and strace
  hashes, requires the inherited environment to equal `build_tool_env` plus the three
  `KILN_*` variables (as multisets of `NAME=value`, refusing a repeated name),
  validates `KILN_ACTION`, `KILN_STEP` and `KILN_STEP_COUNT` against the plan and the
  step order, requires argv to be exactly `sh -c <line>` and the working directory to
  be `<root>/<action>`, and refuses to start under 4 GiB of free disk. It then runs
  the pinned strace (never found on `PATH`) around `kiln sandbox-exec` with the step's
  descriptors 1 and 2 as fresh pipes (`--stdout-fd 3 --stderr-fd 4`) relayed to its own
  output, strace's own streams captured apart, `Pdeathsig = SIGKILL` from a locked OS
  thread, a polled size cap (1 GiB) and disk floor that kill strace and mark the step
  truncated, and exits with the traced root's status. Status 125 is recorded and never
  retried.
- The per-step sidecar `traces/<sha256(action)>/<step>.json` (`kiln-record-step/1`,
  written exact and `O_EXCL`): strace's exit and signal, whether its streams were
  empty, the root's status, truncation and its reason, the trace's size and SHA-256,
  times, `line_in_plan`, `step_netns` (true only when strace was started with the
  sandbox command), and `clean`, the supervisor's half of the G7 completeness rule.
  `record.StepSidecar` has a strict encoder and decoder for `collect`.
- `record.VerifyPre(dir)`: the re-verification of `pre/`, reusable by `collect`.
- `TestSandboxRecordStep` in the required `kiln-sandbox` job: `prepare` on a fixture
  repository, then `step` three times with the real strace and sandbox; the trace
  parses in helper mode, the sidecar is clean, descriptors 1 and 2 are pipes, the
  output is relayed, and the step cannot see the record directory, the sealed binding
  or its own trace. It is named in `ci/assert_sandbox_tests.py`.

### Specification

- KILN04 4.8: decision P12 (the step supervisor), every 2b-2 item re-tagged 2b-2a or
  2b-2b, P1's command surface (`step` implemented, `collect` still a stub). 4.3
  amended: the trace is on disk with a polled cap and a free-space floor, not under
  `RLIMIT_FSIZE` on a tmpfs (G8), and tracer completeness is the G7 rule (status equal
  to the root's, nothing on strace's own stdout or stderr). Divergence from the design
  plan: traces live in `traces/<sha256(action)>/`, never under the action's name (P4).

### Tests

- A fake strace behind `tools.json` covers: every environment mismatch, duplicate and
  missing variable; a bad action, step, count and order; a step run twice (and its
  marker); an argv that is not `sh -c`; `O_EXCL` on the trace; the size cap and the
  disk floor (at the start and while running); status mirroring; exit 125 not
  retried; a non-empty strace stdout or stderr, a signalled strace, a survivor holding
  the pipes and a broken relay target; the sidecar's content; `step_netns` false when
  strace could not be started; and tampering with `pre/` (a modified, extra, missing or
  symlinked file, a bad or resealed `SEALED`, a binding that names other files).
