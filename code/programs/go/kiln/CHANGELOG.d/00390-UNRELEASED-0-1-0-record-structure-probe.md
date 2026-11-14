## [0.1.0] - UNRELEASED

### Added (stage 4 PR 2b-2c-2, KILN04 decision P14)

- `kiln record prepare` runs the in-sandbox structure probe: `pipes`, `walk`,
  `shadow` (only when a host mount point lies inside a read-only root, the
  checkout or a state root) and `signals`, in the real sandbox under the real
  strace, after `CheckHost`. Every run is checked strictly (no write event, exact
  argv, sealed digests, a known init exit line); any failure fails `prepare` with
  nothing sealed. Probe traces go to `<dir>/probe/`, the result to the sealed
  `pre/probes.json` (`kiln-record-probes/1`), which `VerifyPre` now requires and
  checks against the sandbox configuration.
- `record.ShadowedMounts`, `ProbeSpecs`, `CheckProbeRun`, `CheckProbeReport`,
  `EncodeProbes`/`DecodeProbes`, `RunStructureProbes` and the hook
  `Options.Structure` (unit tests never start a sandbox).
- Linux end-to-end test `TestSandboxRecordProbe` (success; a FIFO in a state
  root; a strace that hides signals), registered in `ci/assert_sandbox_tests.py`.
- `ci/assert_sandbox_tests.py` prints the `F2:` and `PROBE:` lines of PASSING
  tests (at most 20, 300 characters each), so facts a test logs reach the CI log.

### Changed

- `runStrace` takes the working directory and an optional deadline;
  `parseHelperTrace` is factored out of `parseStep` and shared by `step` and the
  probes; `stepProcessEnv` builds strace's environment for both.
- Spec: P4 layout, P8, section 13 "Before recording", the section 14 shadow
  bullet, the list of programs `prepare` starts, and new decision P14.
