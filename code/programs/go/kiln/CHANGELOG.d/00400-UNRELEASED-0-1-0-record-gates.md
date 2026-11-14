## [0.1.0] - UNRELEASED

### Added (stage 4 PR 2b-2c-3, KILN04 decision P7)

- Gates. `legacy.Traced.Gate(rel)` and `Traced.Probes()` compute a package's gate
  with `gateFacts`/`gateFrom`, the same functions `legacy-plan`'s `record()` now
  uses, so the plan and the recorder cannot drift (priority: never, other hard
  reasons, probes; the smallest string wins within a priority). `Probes()` returns
  an error too, so a failed preparation is never sealed as an empty list.
- `legacy.ProbeOptions.Timeout` (per probe; the recorder passes 60 s).
- Toolchain probes in a sandbox. `internal/record/probe_toolchain.go`: each probe
  runs as plain `kiln sandbox-exec` (no strace) on
  `<dir>/probe/toolchains-sandbox.json`, whose checkout is the empty
  `<dir>/probe/root`, with no state roots and no masks. `prepare` always passes
  this runner: `Toolchains.Probe` with no runner would exec on the host. A probe
  that mismatches, times out or is refused by the sandbox (exit 125) is a gate;
  any other runner error fails `prepare`.
- Action files are `kiln-record-action/3`: a `gate` field (printable, at most 200
  bytes), decoded in canonical form. `collect` copies it into the record (the
  merge turns it into `precondition:<gate>`).
- `pre/probes.json` `toolchains` is a strict, sorted list of
  `{tool, status, detail}` instead of a reserved empty list.
- `Options.ProbeRun` (a fake runner for unit tests).
- Linux end-to-end test `TestSandboxRecordGates`, registered in
  `ci/assert_sandbox_tests.py`; the shared policy fixtures now carry the boundary
  file that `Gate` reads.

### Changed

- Spec: P7, P4 layout, P14, action /3, collect's gate text, the list of programs
  `prepare` starts, section 13 item 5. The first CI run settled two open points:
  a survivor's exit line precedes the init child's own exit line (F2), and the
  kernel's locked-child-mount rule makes a shadowing host mount fail every step.
