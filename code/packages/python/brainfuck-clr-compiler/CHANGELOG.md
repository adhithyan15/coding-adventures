# Changelog

## [Unreleased]

### Added

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
