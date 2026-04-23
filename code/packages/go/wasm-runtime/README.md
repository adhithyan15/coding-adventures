# wasm-runtime

A WebAssembly 1.0 runtime for the coding-adventures monorepo.

Orchestrates the full pipeline: parse → validate → instantiate → execute.
Provides a WASI host implementation (`WasiHost`, with `WasiStub` kept as a
backward-compatible alias) so WASM modules can call
standard system functions via injectable clock and random interfaces.

## Stack position

```
wasm-module-parser  wasm-validator  wasm-execution
        └──────────────┴─────────────────┘
                        │
                  wasm-runtime   ← this package
```

## WASI support (Tier 1 + Tier 3)

| Function            | Status     | Notes                                     |
|---------------------|------------|-------------------------------------------|
| `fd_write`          | Tier 1     | stdout/stderr via callback                |
| `proc_exit`         | Tier 1     | panics with `*ProcExitError`              |
| `args_sizes_get`    | Tier 3     | injectable `Args []string`                |
| `args_get`          | Tier 3     | null-terminated strings in memory         |
| `environ_sizes_get` | Tier 3     | injectable `Env []string`                 |
| `environ_get`       | Tier 3     | null-terminated KEY=VALUE strings         |
| `clock_res_get`     | Tier 3     | injectable `WasiClock` interface          |
| `clock_time_get`    | Tier 3     | realtime (IDs 0,2,3) and monotonic (ID 1) |
| `random_get`        | Tier 3     | injectable `WasiRandom` interface         |
| `sched_yield`       | Tier 3     | no-op (WASM is single-threaded)           |
| everything else     | ENOSYS stub| returns errno 52                          |

## Usage

### Basic (production)

```go
import wasmruntime "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-runtime"

rt := wasmruntime.New(nil)
instance, err := rt.Instantiate(module)
results, err := rt.Call(instance, "main", nil)
```

### With WASI

```go
wasi := wasmruntime.NewWasiHostFromConfig(wasmruntime.WasiConfig{
    Args:           []string{"myapp", "--verbose"},
    Env:            []string{"HOME=/home/user", "PATH=/usr/bin"},
    StdoutCallback: func(s string) { fmt.Print(s) },
    StderrCallback: func(s string) { fmt.Fprint(os.Stderr, s) },
    // Clock and Random default to SystemClock{} and SystemRandom{}
})

rt := wasmruntime.New(wasi)
instance, err := rt.Instantiate(module)
wasi.SetMemory(instance.Memory)
```

### Injecting fakes for tests

```go
type FakeClock struct{}
func (FakeClock) RealtimeNs() int64         { return 1_700_000_000_000_000_001 }
func (FakeClock) MonotonicNs() int64        { return 42_000_000_000 }
func (FakeClock) ResolutionNs(int32) int64  { return 1_000_000 }

wasi := wasmruntime.NewWasiHostFromConfig(wasmruntime.WasiConfig{
    Clock:  FakeClock{},
    Random: MyFakeRandom{},
})
```

## Dependencies

- `wasm-leb128` — LEB128 integer encoding
- `wasm-types` — shared type definitions
- `wasm-opcodes` — opcode constants
- `wasm-module-parser` — binary WASM decoder
- `wasm-execution` — interpreter and linear memory
- `wasm-validator` — module validation

## Development

```bash
# Run tests
go test ./... -v -cover
```
