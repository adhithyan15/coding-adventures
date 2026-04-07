# wasm-runtime

WebAssembly 1.0 runtime: parse, validate, instantiate, and execute WASM modules.

The top-level package in the TypeScript WASM stack. Composes the parser, validator,
and execution engine into a single API, plus a WASI Tier 3 host implementation.

## Dependencies

- wasm-leb128
- wasm-types
- wasm-opcodes
- wasm-module-parser
- wasm-validator
- wasm-execution
- virtual-machine

## Quick Start

### Pure Computation (no WASI needed)

```typescript
import { WasmRuntime } from "@coding-adventures/wasm-runtime";

const runtime = new WasmRuntime();
const result = runtime.loadAndRun(wasmBytes, "square", [5]);
console.log(result); // [25]
```

### Hello World (with WASI I/O)

```typescript
import { WasmRuntime, WasiStub } from "@coding-adventures/wasm-runtime";

const output: string[] = [];
const wasi = new WasiStub({
  stdout: (text) => output.push(text),
  stderr: (text) => process.stderr.write(text),
});
const runtime = new WasmRuntime(wasi);
runtime.loadAndRun(wasmBytes);
console.log(output.join(""));
```

## WasiConfig

`WasiStub` accepts a `WasiConfig` object (all fields optional):

```typescript
interface WasiConfig {
  /** Command-line arguments. Defaults to []. */
  args?: string[];

  /** Environment variables. Defaults to {}. */
  env?: Record<string, string>;

  /** Called when the WASM program writes to stdout (fd 1). */
  stdout?: (text: string) => void;

  /** Called when the WASM program writes to stderr (fd 2). */
  stderr?: (text: string) => void;

  /** Clock source for WASI time syscalls. Defaults to SystemClock. */
  clock?: WasiClock;

  /** Random byte source for WASI random_get. Defaults to SystemRandom. */
  random?: WasiRandom;
}
```

### Example: Full Config

```typescript
const wasi = new WasiStub({
  args: ["myapp", "--verbose", "input.txt"],
  env: { HOME: "/home/user", PATH: "/usr/bin" },
  stdout: (text) => process.stdout.write(text),
  stderr: (text) => process.stderr.write(text),
});
```

## WASI Tier 3 Functions

`WasiStub` implements these WASI syscalls:

| Function            | Description                                           |
|---------------------|-------------------------------------------------------|
| `fd_write`          | Write to stdout/stderr (captured via callbacks)       |
| `proc_exit`         | Terminate with exit code (throws `ProcExitError`)     |
| `args_sizes_get`    | Report argc and total argv buffer size                |
| `args_get`          | Fill argv pointer array and null-terminated strings   |
| `environ_sizes_get` | Report env count and total environ buffer size        |
| `environ_get`       | Fill environ pointer array and "KEY=VALUE\0" strings  |
| `clock_res_get`     | Report clock resolution in nanoseconds                |
| `clock_time_get`    | Return wall clock or monotonic time in nanoseconds    |
| `random_get`        | Fill a buffer with cryptographically random bytes     |
| `sched_yield`       | No-op (single-threaded WASM has nothing to yield to)  |

All other WASI functions return `ENOSYS` (52).

## Injecting Custom Clock and Random

Clock and random are injected via interfaces so they can be swapped for testing
or custom implementations:

```typescript
import { WasiClock, WasiRandom, WasiStub } from "@coding-adventures/wasm-runtime";

// Deterministic clock for testing
class FakeClock implements WasiClock {
  realtimeNs()  { return 1_700_000_000_000_000_000n; }
  monotonicNs() { return 0n; }
  resolutionNs(_id: number) { return 1_000_000n; }
}

// Deterministic random for testing
class ZeroRandom implements WasiRandom {
  fillBytes(buf: Uint8Array) { buf.fill(0); }
}

const wasi = new WasiStub({
  clock: new FakeClock(),
  random: new ZeroRandom(),
});
```

## Development

```bash
# Run tests
bash BUILD
```

## WASI Stack Position

```
wasm-runtime  ← you are here
    ├── wasm-module-parser
    ├── wasm-validator
    └── wasm-execution
            └── virtual-machine
```
