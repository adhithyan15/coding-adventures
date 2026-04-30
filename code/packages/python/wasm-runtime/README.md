# wasm-runtime

A complete WebAssembly 1.0 runtime written in pure Python.  It parses,
validates, instantiates, and executes `.wasm` binaries, and provides a
WASI host implementation so real WASM programs can interact with the outside world.

## How it fits in the stack

```
wasm-leb128          ← binary encoding/decoding primitives
wasm-types           ← type definitions (ValueType, FuncType, …)
wasm-opcodes         ← opcode constants
wasm-module-parser   ← parse binary WASM into structured sections
wasm-validator       ← type-check the parsed module
wasm-execution       ← execute instructions, manage linear memory/stack
wasm-runtime         ← top-level orchestration + WASI host (YOU ARE HERE)
```

## Dependencies

- `wasm-leb128`
- `wasm-types`
- `wasm-opcodes`
- `wasm-module-parser`
- `virtual-machine`
- `wasm-validator`
- `wasm-execution`

## Usage

### Running a WASM function

```python
from wasm_runtime import WasmRuntime

wasm_bytes = open("my_module.wasm", "rb").read()
runtime = WasmRuntime()
result = runtime.load_and_run(wasm_bytes, "square", [5])
print(result)  # [25]
```

### WASI Tier 1 (stdin/stdout/stderr capture)

```python
from wasm_runtime import WasiHost, WasmRuntime

output = []
host = WasiHost(stdout=output.append)
runtime = WasmRuntime(host=host)
runtime.load_and_run(wasm_bytes, "_start", [])
print("".join(output))
```

### WASI Tier 3 (args, environ, clock, random)

Use `WasiConfig` to supply all host configuration in one place:

```python
from wasm_runtime import WasiHost
from wasm_runtime.wasi_host import WasiConfig, SystemClock, SystemRandom

config = WasiConfig(
    args=["myapp", "--flag"],
    env={"HOME": "/home/user", "PATH": "/usr/bin"},
    stdin=lambda n: b"input"[:n],
    stdout=print,
    stderr=print,
    clock=SystemClock(),    # default: real OS clock
    random=SystemRandom(),  # default: OS CSPRNG via secrets module
)
host = WasiHost(config)
```

#### Injecting a fake clock for testing

```python
from wasm_runtime.wasi_host import WasiClock, WasiConfig, WasiHost

class FakeClock(WasiClock):
    def realtime_ns(self): return 1_700_000_000_000_000_000
    def monotonic_ns(self): return 0
    def resolution_ns(self, clock_id): return 1_000_000

host = WasiHost(WasiConfig(clock=FakeClock()))
```

### Compiler math imports

`WasiHost` also resolves the generic compiler pipeline's `compiler_math`
imports for standard unary f64 math: `f64_sin`, `f64_cos`, `f64_atan`,
`f64_ln`, and `f64_exp`. These imports are intentionally separate from WASI
so generated modules declare the non-core WASM math surface they need.

## WASI functions implemented

| Function            | Tier | Description                                      |
|---------------------|------|--------------------------------------------------|
| `fd_write`          | 1    | Write iovec buffers to stdout or stderr          |
| `fd_read`           | 1    | Read iovec buffers from stdin                    |
| `proc_exit`         | 1    | Terminate the WASM program (raises ProcExitError)|
| `args_sizes_get`    | 3    | Report argument count and buffer size            |
| `args_get`          | 3    | Copy args into WASM linear memory                |
| `environ_sizes_get` | 3    | Report env-var count and buffer size             |
| `environ_get`       | 3    | Copy env vars into WASM linear memory            |
| `clock_res_get`     | 3    | Report clock resolution in nanoseconds           |
| `clock_time_get`    | 3    | Read realtime or monotonic clock                 |
| `random_get`        | 3    | Fill memory buffer with CSPRNG bytes             |
| `sched_yield`       | 3    | Cooperative scheduler yield (no-op)              |

All other WASI imports resolve to a stub that returns `ENOSYS` (52) so
modules can link without errors even if they import functions not yet
implemented.

## Development

```bash
# Run tests
bash BUILD

# Or directly:
uv run pytest tests/ -v
```

Test coverage is ~93% and must remain above 80%.
