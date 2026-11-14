## [0.1.0] - UNRELEASED

### Fixed

- H1: the recorder's `PATH` and `git`. `kiln record prepare` takes `--git` (default
  `/usr/bin/git`), resolves it once (canonical, regular file, outside the checkout,
  `tmpdir`, state roots and record directory), hashes it into the binding
  (`git_sha256`, with `strace_sha256`) and runs every Git call through the new
  `source.NewGitAt`, never through `PATH`. `SafePath` refuses a `PATH` that is
  empty or has an empty, relative, or checkout/`tmpdir`/record-directory entry
  (spelled out or through a symlink); the build tool is started with that
  validated `PATH` only. `pre/tools.json` records the absolute `git`, `strace` and
  `PATH` for `step` and `collect`. `record.VerifyTreeWith`, `InventoryWith` and
  `ScanTreeWith` take the pinned runner.
- M1: a state root may not overlap, in either direction, the record directory,
  `RUNNER_TEMP`, the Kiln, build-tool, `strace` or `git` binary, the directories of
  `GITHUB_ENV`, `GITHUB_PATH`, `GITHUB_OUTPUT`, `GITHUB_STEP_SUMMARY` or
  `GITHUB_EVENT_PATH`, or any directory of the recorder's `PATH`.
- M2: the host scan skips a directory it cannot list only when it also lacks search
  permission on it; a `--x` directory fails the scan.
- P3 decided: `sandbox.CheckHost` refuses a configuration that overlaps
  `repo_root`, `tmpdir`, a state root or an `ro_roots` entry (either direction),
  and no longer refuses one merely beneath `home`; `--dir $RUNNER_TEMP/kiln-record`
  works. The "config in home" test is replaced by an accept test and refusals for
  a configuration inside an ro root beneath home and over a state root.
- `ScanTree` refuses a socket or FIFO in the checkout.
- `XDG_CONFIG_HOME` must be absolute.
- The host scan treats a different `st_dev` as a mount boundary only if
  `/proc/self/mountinfo` lists the directory (otherwise it scans through), and stops
  after `MaxReported` findings.
- An existing `--dir` and `tmpdir` must be owned by the user and not group/world
  writable, and are forced to `0700`.
- `SEALED` is written only after `pre/` is checked to hold nothing but what was
  written (no stray file, no symlink); if the read-only pass fails, `SEALED` is
  removed.
- The graph, steps and plan exports are read once and the digest is taken over the
  bytes that are parsed.
- `execTool` and `straceVersion` set `cmd.WaitDelay` (10 s).
- `strace` goes through `ResolveBinary` and is hashed into the binding.

### Changed

- `recfmt.Binding` gains `strace_sha256` and `git_sha256`, required.

### Specification

- KILN04 section 4.7 (configuration-path rule), 4.8 (P3 decided, P4, P5, P6, P8,
  P9, new P11, the 2b-2 items "`step`/`collect` use only the recorded git path"
  and "re-verify `pre/` against `SEALED`", the empty-shard decision), 7.1 (B3: the
  recorded binary digests are audit-only) and 14 (digest backlog, host-mount
  shadowing).
