# Changelog

## [Unreleased]

### Changed

- **`test_oct_out_syscall_raises_at_runtime` → `test_rejects_oct_out_syscall_at_compile_time`**:
  updated to reflect the new compile-time SYSCALL validation added to
  `ir-to-cil-bytecode`.  Oct's `out(17, val)` → SYSCALL 57 is now rejected by
  `lower_ir_to_cil_bytecode()` with `CILBackendError` (not `CLRVMError` at
  runtime).  The test previously called `write_cli_assembly` and
  `run_clr_entry_point`; it now only calls `lower_ir_to_cil_bytecode()` and
  verifies the early `CILBackendError`.
- Updated module docstring in `tests/test_oct_8bit_e2e.py` to describe the
  compile-time validation behaviour (replacing the stale "no compile-time
  validator" paragraph).

### Added (in a previous unreleased update)

- **Oct 8-bit arithmetic e2e tests** (`tests/test_oct_8bit_e2e.py`):
  6 end-to-end tests confirming the CLR backend correctly compiles and
  executes 8-bit integer arithmetic IR — the same IR that the Oct compiler
  generates.  Tests use the full IR → CIL bytecode → CLI PE assembly → CLR
  VM pipeline.  Key findings:
  - Pure 8-bit arithmetic compiles correctly through the full CLR pipeline.
  - The CLR host's SYSCALL 1 uses the GE-225 character encoding (e.g., byte
    value 7 → digit character '7'), which is expected behaviour inherited from
    the Dartmouth BASIC / GE-225 era design.
  - Oct's I/O intrinsics (SYSCALL 40+PORT / 20+PORT) pass through the CLR
    compiler (no compile-time validator) but raise ``CLRVMError`` at runtime.
    This is safe (no silent misbehavior) but errors are caught later than in
    the WASM and JVM backends.  The ``test_oct_out_syscall_raises_at_runtime``
    test documents this difference explicitly.

## 0.1.0

- Add end-to-end Brainfuck to CLR compiler facade.
- Add simulator execution support for Brainfuck output and input syscalls.
