# Changelog

All notable changes to this package will be documented in this file.

## [0.2.0] - 2026-04-06

### Added

- **WASI Tier 3** — 8 new WASI `snapshot_preview1` functions:
  - `args_sizes_get` — report argc and total argv buffer size
  - `args_get` — fill argv pointer array + null-terminated string data into linear memory
  - `environ_sizes_get` — report env-var count and total buffer size for "KEY=VALUE\0" strings
  - `environ_get` — fill environ pointer array + "KEY=VALUE\0" string data into memory
  - `clock_res_get` — write clock resolution (i64 ns) to memory
  - `clock_time_get` — write current time (i64 ns) to memory; supports clock ids 0–3
  - `random_get` — fill a memory region with cryptographically random bytes
  - `sched_yield` — no-op yield that returns ESUCCESS
- **`SystemClock`** class — injectable clock backed by `Process.clock_gettime`
- **`SystemRandom`** class — injectable CSPRNG backed by `SecureRandom`
- **Constructor extended** with `args:`, `env:`, `clock:`, `random:` keyword args;
  old `stdout_callback:` / `stderr_callback:` keywords still accepted (backward-compat)
- **`test_wasi_tier3.rb`** — 26 new unit tests covering all 8 Tier 3 functions with
  `FakeClock` / `FakeRandom` fakes for deterministic assertions; also covers backward-
  compat constructor keywords and the end-to-end square-WASM smoke test

### Changed

- `wasi_stub.rb` — constructor signature expanded; `resolve_function` now dispatches
  all 10 implemented WASI names instead of just `fd_write` / `proc_exit`
- Existing stub test updated to call `path_open` (truly unimplemented) instead of
  `args_sizes_get` which is now a real Tier 3 implementation

## [0.1.0] - 2026-04-05

### Added

- Complete WasmRuntime composing parser, validator, and execution engine
- WasmInstance struct representing a live WASM module instance
- Runtime#load: parse .wasm binary bytes
- Runtime#validate: structural validation
- Runtime#instantiate: resolve imports, allocate memory/tables/globals, apply data/element segments
- Runtime#call: invoke exported functions by name with automatic type conversion
- Runtime#load_and_run: convenience all-in-one method
- WasiStub: minimal WASI implementation (fd_write, proc_exit)
- ProcExitError for clean WASM program termination
- End-to-end tests: square(5)=25, square(0)=0, square(-3)=9
