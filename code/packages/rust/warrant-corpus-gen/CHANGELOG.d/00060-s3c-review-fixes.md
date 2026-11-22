## 0.3.0 - S3c review fixes

### Fixed

- `fsio`: the temporary file is unlinked and created with `create_new` (O_EXCL), so a planted hard
  link no longer clobbers its victim and a FIFO at the temp name no longer hangs the write.
- `--check` and the CLI read only regular files, through a handle, at most one byte past the
  expected length (4 MiB for an input): a FIFO or an oversize file is a finding or an error, not a
  hang or an allocation (`fsio::read_regular`).
- `--check` reports a symbolic link at `chunks/`; write re-checks it after `create_dir_all`.
- File names found on disk are escaped in reports (no terminal escapes or workflow commands).
- Chunk file names may be 133 bytes (the spec maximum); Windows reserved device names are
  `W0701` at generation.
- The CLI says when `--bindings` was not given.
- `required_capabilities.json` targets are now the honest "caller-named" input files and output
  directory, with the guards named; module and capability text no longer claim more than the
  code does.
- BUILD's negative gate requires the located drift finding (not just any failure) and the
  absence of `test-hooks` from the normal dependency tree; BUILD_windows explains why the shell
  gate is a cargo test there.
- Spec: 5.6 (pre rewrites outside `admit`; claims named by suite id), 8.3, 16.6 (OPEN items
  classified, `ir.fixed` form, hardening).

### Added

- Tests for hard links, FIFOs, oversize files, a symlinked `chunks/` and `--out`, hostile names,
  the 133-byte boundary, reserved names and `tfv` refusals.
- A `debug_assert` and comment on the uniform-rewrite assumption of the stand-in.
