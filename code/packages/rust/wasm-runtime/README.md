# wasm-runtime

Complete WebAssembly 1.0 runtime — parse, validate, instantiate, execute.

This crate composes the lower-level WASM packages into a single, user-facing API. It sits at the top of the WASM stack:

```text
.wasm bytes  -->  Parse  -->  Validate  -->  Instantiate  -->  Execute
    |               |            |               |               |
&[u8]         WasmModule  ValidatedModule  WasmInstance    WasmValue[]
    |               |            |               |               |
(input)      (wasm-module-   (wasm-         (this crate)  (wasm-execution)
              parser)         validator)
```

## Dependencies

- `wasm-leb128` — LEB128 decoder used by the parser
- `wasm-types` — shared type definitions (WasmModule, FuncType, etc.)
- `wasm-opcodes` — opcode constants
- `wasm-module-parser` — binary `.wasm` parser
- `wasm-validator` — validates a parsed module before execution
- `wasm-execution` — interpreter and host function interface
- `virtual-machine` — generic stack VM used by the execution engine

## Quick Start

```rust
use wasm_runtime::WasmRuntime;

let runtime = WasmRuntime::new();
let result = runtime.load_and_run(&wasm_bytes, "square", &[5]).unwrap();
assert_eq!(result, vec![25]);
```

## WASI Support

### Tier 1: `WasiStub`

A minimal stub that handles `proc_exit` and returns ENOSYS for everything else.

```rust
use wasm_runtime::{WasmRuntime, WasiStub};

let wasi = WasiStub::new(|text| print!("{}", text));
let runtime = WasmRuntime::with_host(Box::new(wasi));
```

### Tier 3: `WasiEnv`

A full WASI preview1 host implementation with 8 functions:

| Function           | Description                                       |
|--------------------|---------------------------------------------------|
| `args_sizes_get`   | Return argc and total argv buffer size            |
| `args_get`         | Write argv pointers and null-terminated strings   |
| `environ_sizes_get`| Return envc and total environ buffer size         |
| `environ_get`      | Write environ pointers and null-terminated strings|
| `clock_res_get`    | Return clock resolution in nanoseconds            |
| `clock_time_get`   | Return current clock time in nanoseconds          |
| `random_get`       | Fill WASM memory with random bytes                |
| `sched_yield`      | Yield (no-op in single-threaded host)             |

```rust
use wasm_runtime::{WasmRuntime, WasiConfig, WasiEnv};

let cfg = WasiConfig {
    args: vec!["myapp".into(), "--verbose".into()],
    env:  vec!["HOME=/home/user".into()],
    ..Default::default()
};
let wasi = WasiEnv::new(cfg);

// After instantiation, attach the instance's memory to the WASI env.
// WasiEnv needs memory access to write args/clock/random data.
let runtime = WasmRuntime::with_host(Box::new(wasi));
```

### Injecting a deterministic clock and RNG for tests

```rust
use wasm_runtime::{WasiClock, WasiRandom, WasiConfig, WasiEnv};

struct FakeClock;
impl WasiClock for FakeClock {
    fn realtime_ns(&self)  -> i64 { 1_700_000_000_000_000_000 }
    fn monotonic_ns(&self) -> i64 { 1_000_000_000 }
    fn resolution_ns(&self, _: i32) -> i64 { 1_000_000 }
}

struct ZeroRandom;
impl WasiRandom for ZeroRandom {
    fn fill_bytes(&self, buf: &mut [u8]) { buf.fill(0); }
}

let cfg = WasiConfig {
    clock:  Box::new(FakeClock),
    random: Box::new(ZeroRandom),
    ..Default::default()
};
```

## Clock IDs (WASI preview1)

| ID | Meaning                        |
|----|--------------------------------|
|  0 | CLOCK_REALTIME — wall clock    |
|  1 | CLOCK_MONOTONIC — never rewinds|
|  2 | PROCESS_CPUTIME (→ realtime)   |
|  3 | THREAD_CPUTIME (→ realtime)    |

## Security Notes

- `SystemRandom` (the default) is **NOT cryptographically secure**. It uses
  `DefaultHasher` mixed with `SystemTime`. For security-sensitive WASM
  programs, inject a `getrandom`- or `ring`-backed `WasiRandom`.
- All memory writes go through `LinearMemory::write_bytes` and `store_i32`,
  which enforce WASM bounds checks. No out-of-bounds writes are possible.

## Development

```bash
# Run tests
cd code/packages/rust
mise exec -- cargo test -p wasm-runtime

# Build all WASM packages
mise exec -- cargo build -p wasm-runtime -p wasm-execution -p wasm-validator -p wasm-module-parser -p wasm-types -p wasm-leb128
```
