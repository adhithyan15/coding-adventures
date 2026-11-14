## [0.1.0] - UNRELEASED

### Fixed

- M1: a symlink `ro_roots` entry beneath `home` is refused, by `CheckHost`
  (`checkRoRootBeneathHome`) and again by `applyRORoot`. A recreated symlink is
  an ordinary inode of `home`'s writable tmpfs, not a mount point, so a step could
  `rm` it and build a directory of its own at the same path. Entries beneath
  `home` must be real directories or files. KILN04 section 4.7 no longer says "or
  a recreated symlink" for them.
- M2: the `/tmp` exemption is gone. A root beneath `home` is refused whenever
  `home` lies strictly inside `/tmp`, `/var/tmp`, `tmpdir`, the checkout or a
  state root, because the step could rename an ancestor of `home` (`mv /tmp/k
  /tmp/.gone`) and rebuild the root itself. Home's ancestors must sit on the
  sealed root tmpfs. `home=/var` with a root in `/var/tmp` is refused (L1/L2).
  The S1 integration test builds its home under the runner's real home directory
  (`~/.kiln-s1-*`) and asserts that moving `home`'s parent fails.
- L4: `CheckOutputFD` requires `O_WRONLY`; a read-write descriptor (a FIFO opened
  `O_RDWR`) is refused.
- L6: `--stdout-fd` and `--stderr-fd` must be given together or not at all.

### Added

- L7: the seccomp filter answers `ENOSYS` for every x86-64 system call number
  above `MaxKnownSyscall` (461, `lsm_list_modules`, the highest of the pinned
  strace 6.8's table). Calls newer than the pin (`getxattrat`, `listxattrat`,
  `setxattrat`, `removexattrat`, `open_tree_attr`, `file_getattr`,
  `file_setattr`) could be neither traced nor refused. Raise the threshold when
  the pinned strace is bumped (KILN04 section 14).
- L3: `trace.HandledNames` and `TestTraceSetCoversEveryHandledCall`: the trace
  set and the parser's table are checked in both directions; deleting `getdents64`
  from the explicit names fails a test.
- L5 and the PR 2b list (KILN04 sections 4.7 and 4.8): exit 125 is never
  retried unsandboxed; the recorder must declare `~/.cargo/registry`,
  `~/.cargo/git` and the rest as state roots under a sealed anchor, and must not
  anchor something broad like `~/.local`.

### Tests

- Pure: `TestValidateHomeAncestorsMustBeSealed`,
  `TestCheckHostRoRootBeneathHomeRefusesSymlink`,
  `TestFilterNewerSyscallsGetENOSYS`, `TestTraceSetCoversEveryHandledCall`,
  the L4 and L6 cases of `TestCheckOutputFD` and `TestParseExecArgsOutputFDs`.
- Linux, required: `TestSandboxROBeneathHomeRefusals` (new, named in
  `ci/assert_sandbox_tests.py`); `TestSandboxROBeneathHome` also asserts `rm -rf`
  of a direct-child root and `mv` of `home` and its parent fail;
  `TestSandboxOutputDescriptors` covers the read-write FIFO and the single-flag
  refusals.
