# wasm-runtime

Complete WebAssembly 1.0 runtime for Swift.

## Overview

The runtime is the user-facing entry point that composes all lower-level
WASM packages into a single API. It handles the full pipeline:

```
.wasm bytes  ->  Parse  ->  Validate  ->  Instantiate  ->  Execute
```

## Usage

```swift
import WasmRuntime

// Simple: compute square(5) from a .wasm binary
let runtime = WasmRuntime()
let result = try runtime.loadAndRun(squareWasm, entry: "square", args: [5])
// result == [25]

// With WASI for programs that do I/O:
let wasi = WasiStub()
let runtime = WasmRuntime(host: wasi)
try runtime.loadAndRun(helloWorldWasm)
print(wasi.stdoutOutput)
```

## WASI support

The `WasiStub` class implements the `wasi_snapshot_preview1` ABI.

### Tier 1 (baseline I/O)

| Function   | Description                            |
|-----------|----------------------------------------|
| `fd_write`  | Write iovec buffers to stdout/stderr  |
| `proc_exit` | Set exit code and return              |

### Tier 3 (args, environ, clock, random)

| Function              | Description                                      |
|----------------------|--------------------------------------------------|
| `args_sizes_get`      | Query argc and argv buffer size                 |
| `args_get`            | Copy argv strings + pointer table into memory   |
| `environ_sizes_get`   | Query envc and environ buffer size              |
| `environ_get`         | Copy environ strings + pointer table into memory|
| `clock_res_get`       | Query clock resolution (nanoseconds)            |
| `clock_time_get`      | Read realtime or monotonic clock                |
| `random_get`          | Fill a buffer with random bytes                 |
| `sched_yield`         | Yield (no-op on cooperative runtimes)           |

### Injecting custom clock and random

Clock and randomness are protocol-based, making tests fully deterministic:

```swift
import WasmRuntime

// Fake implementations for deterministic tests
struct FakeClock: WasiClock {
    func realtimeNs()  -> Int64 { 1_700_000_000_000_000_001 }
    func monotonicNs() -> Int64 { 42_000_000_000 }
    func resolutionNs(clockId: Int32) -> Int64 { 1_000_000 }
}

struct FakeRandom: WasiRandom {
    func fillBytes(count: Int) -> [UInt8] { Array(repeating: 0xAB, count: count) }
}

let config = WasiConfig(
    args: ["myapp", "--flag"],
    env: ["HOME": "/home/user"],
    clock: FakeClock(),
    random: FakeRandom()
)
let wasi = WasiStub(config: config)
let runtime = WasmRuntime(host: wasi)
try runtime.loadAndRun(myWasm)
```

## Components

- **WasmRuntime**: Main entry point with load/validate/instantiate/call methods
- **WasmInstance**: Live module instance with allocated memory, tables, globals
- **WasiStub**: WASI Tier 1+3 implementation with injectable clock and random
- **WasiConfig**: Configuration struct for args, env, clock, random, stdout/stderr
- **WasiClock**: Protocol for clock injection (SystemClock / FakeClock)
- **WasiRandom**: Protocol for random injection (SystemRandom / FakeRandom)

## Dependencies

- wasm-leb128
- wasm-types
- wasm-opcodes
- wasm-module-parser
- wasm-validator
- wasm-execution
- virtual-machine

## Development

```bash
# Run tests
bash BUILD
```
