# Changelog

All notable changes to this package will be documented in this file.

## [0.2.0] - 2026-04-06

### Added

- **WASI Tier 3**: 8 new WASI functions implemented in `WasiStub`:
  - `args_sizes_get`: Reports argc and total buffer size for command-line args.
  - `args_get`: Writes null-terminated argv strings and pointer array into memory.
  - `environ_sizes_get`: Reports count and buffer size for environment variables.
  - `environ_get`: Writes "KEY=VALUE\0" strings and pointer array into memory.
  - `clock_res_get`: Reports clock resolution as u64 nanoseconds (via WasiClock).
  - `clock_time_get`: Returns wall clock or monotonic time as u64 nanoseconds.
  - `random_get`: Fills a buffer with cryptographically random bytes (via WasiRandom).
  - `sched_yield`: No-op in single-threaded WASM (returns ESUCCESS immediately).

- **`WasiClock` interface**: Injectable clock abstraction with `realtimeNs()`,
  `monotonicNs()`, and `resolutionNs(id)`. Enables deterministic clock tests via
  `FakeClock` injection.

- **`WasiRandom` interface**: Injectable random source with `fillBytes(buf)`.
  Enables deterministic random tests via `FakeRandom` injection.

- **`SystemClock` class**: Default WasiClock implementation. Uses
  `process.hrtime.bigint()` in Node.js and `performance.now()` in browsers.
  Reports 1ms resolution for all clock IDs (browser security floor).

- **`SystemRandom` class**: Default WasiRandom implementation. Uses
  `crypto.randomFillSync()` in Node.js and `globalThis.crypto.getRandomValues()`
  in browsers. Both are cryptographically secure (CSPRNG).

- **`WasiConfig` interface**: Expanded configuration type for `WasiStub`. Replaces
  the anonymous `{ stdout?, stderr? }` options type with a named, documented
  interface. Adds `args`, `env`, `clock`, and `random` fields. Fully backwards
  compatible — all fields are optional.

- **`tests/wasi_tier3.test.ts`**: 22 new tests covering all 8 new functions.
  Uses `FakeClock` and `FakeRandom` for deterministic assertions plus real
  `SystemClock` and `SystemRandom` smoke tests.

### Changed

- `WasiStub` constructor now accepts `WasiConfig` (superset of old options).
  Old code using `{ stdout?, stderr? }` is fully backwards compatible.

- `resolveFunction` now dispatches 8 new function names to real implementations
  instead of the ENOSYS stub.

### Exports Added

- `WasiClock`, `WasiRandom`, `WasiConfig` (type exports)
- `SystemClock`, `SystemRandom` (class exports)

## [0.1.0] - 2026-04-05

### Added

- WasmRuntime: complete pipeline from .wasm bytes to execution results.
  - load(): parse .wasm binary via WasmModuleParser.
  - validate(): semantic validation via wasm-validator.
  - instantiate(): allocate memory/tables/globals, resolve imports,
    apply data/element segments, call start function.
  - call(): call exported function by name with number[] args → number[] results.
  - loadAndRun(): convenience one-shot method.
- WasmInstance: holds all runtime state (memory, tables, globals, exports).
- WasiStub: minimal WASI host implementation.
  - fd_write: captures stdout/stderr output via callbacks.
  - proc_exit: throws ProcExitError with exit code.
  - All other WASI functions return ENOSYS (52) — clearly documented as stub.
- End-to-end test: hand-assembled square(n)=n*n module passes all cases
  including i32 overflow wrapping (square(2147483647)=1).
