## [0.1.0] - UNRELEASED

### Added

- S1 (KILN04 section 4.7): an `ro_roots` entry may lie strictly beneath `home`
  (the tool roots `~/.cargo/bin`, `~/.rustup`, `~/.dotnet`, `~/.ghcup` of
  `host-roots.json`). `Plan` binds it after `home`'s tmpfs, at the same path. A
  direct child of `home` is itself the mount point; a deeper entry goes into a
  tmpfs **anchor** (the first directory below `home`) that is sealed read-only
  once filled (`KindSealDir`), with the mount points of any state root beneath
  the anchor created first. Reason: `rename` of a directory that merely contains
  a mount succeeds (only a mount point gives `EBUSY`), so without the anchor a
  step could replace `~/.cargo` and build its own `~/.cargo/bin`. `Validate`
  still refuses an entry that is, or contains, `home`, and any overlap with the
  checkout, `tmpdir` or a state root; it adds: the anchor holds neither the
  checkout nor `tmpdir`, and entries beneath `home` do not nest. `sandbox.HomeAnchor`.
- S2: `kiln sandbox-exec --stdout-fd N --stderr-fd N`. The named descriptors of
  the root (3 or more; open, writable, **pipes only**) become the step's 1 and 2
  through the one spawning `clone`, so the init needs no `dup2`; every other
  descriptor from 3 up is still close-on-exec. `sandbox.CheckOutputFD` decides
  the type from `st_mode` and `F_GETFL`, on every OS.
- S3: `sandbox.StraceArgs(traceFile)`, `sandbox.TraceSet` and
  `sandbox.TraceNames` are the one definition of the recorder's strace argv. The
  `-e trace=` set now names all 31 refused calls, including the ten the stage 3b
  review added to the filter (`io_uring_enter`, `io_uring_register`,
  `process_vm_writev`, `userfaultfd`, `kexec_load`, `kexec_file_load`,
  `perf_event_open`, `add_key`, `request_key`, `keyctl`), which it left out.
  `trace.SyscallClass` and `trace.RefusedNames` expose the parser's table.

### Changed

- `TestSandboxRealStrace` and `TestSandboxStraceTraceSetNamesAreValid` use the
  exported argv; the latter also runs the whole argv.
- `ParseExecArgs` returns `StdoutFD` and `StderrFD` (`NoFD` when absent).
- KILN04 sections 4.3 and 4.7 updated.

### Tests

- Pure: `TestValidateRoRootsBeneathHome`, `TestHomeAnchor`,
  `TestPlanRoRootsBeneathHome`, `TestPlanAnchorsAreSortedAndSeparate`,
  `TestParseExecArgsOutputFDs`, `TestCheckOutputFD`,
  `TestTraceSetAgreesWithTheParser` (every traced name is known to the parser;
  the parser's refused class, the filter's list plus `clone3`, and the traced
  names agree exactly), `TestStraceArgs*`. `internal/sandbox` coverage is 97.2%
  on a non-Linux host.
- Linux, required in the `kiln-sandbox` job and named in
  `ci/assert_sandbox_tests.py`: `TestSandboxROBeneathHome`,
  `TestSandboxOutputDescriptors`; plus `TestInspectOutputFD` in
  `internal/sandbox`.
