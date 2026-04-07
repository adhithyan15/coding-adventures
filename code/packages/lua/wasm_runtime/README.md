# wasm-runtime (Lua)

WebAssembly 1.0 runtime orchestrator for the coding-adventures educational stack.

## What it does

`wasm_runtime` ties together the lower-level WASM packages into a single
convenient API:

```
parse → validate → instantiate → call
```

It also provides `WasiStub`, a host environment that implements the
`wasi_snapshot_preview1` API so that WASM programs compiled with WASI support
can be executed without an operating system.

## Where it fits in the stack

```
wasm_runtime          ← this package (orchestration + WASI host)
  wasm_execution      ← bytecode interpreter + linear memory
    virtual_machine   ← generic stack VM
  wasm_validator      ← structural validation
  wasm_module_parser  ← binary → structured module
    wasm_leb128       ← LEB128 decode
    wasm_types        ← type constants
    wasm_opcodes      ← opcode constants
```

## Usage

### Simple computation (no WASI)

```lua
local wasm_runtime = require("coding_adventures.wasm_runtime")

local runtime = wasm_runtime.WasmRuntime.new()
local results = runtime:load_and_run(wasm_bytes, "square", {5})
-- results == {25}
```

### Hello World with WASI (fd_write + proc_exit)

```lua
local output = {}
local wasi = wasm_runtime.WasiStub.new({
    stdout = function(text) output[#output + 1] = text end,
})
local runtime = wasm_runtime.WasmRuntime.new(wasi)
runtime:load_and_run(hello_world_wasm, "_start", {})
print(table.concat(output))  -- "Hello, World!\n"
```

### Programs with args, env, clock, and random (Tier 3 WASI)

```lua
local wasi = wasm_runtime.WasiStub.new({
    args   = {"myprogram", "--verbose"},
    env    = {"HOME=/home/user", "PATH=/usr/bin"},
    stdout = function(text) io.write(text) end,
    stderr = function(text) io.stderr:write(text) end,
    -- Inject a custom clock for deterministic testing:
    clock  = my_fake_clock,
    -- Inject a custom PRNG for deterministic testing:
    random = my_fake_random,
})
```

## WASI functions implemented

### Tier 1 (I/O)
| Function   | Description                            |
|------------|----------------------------------------|
| `fd_write` | Write to stdout (fd=1) or stderr (fd=2)|
| `proc_exit`| Terminate with an exit code            |

### Tier 3 (args, environ, clock, random)
| Function              | Description                                      |
|-----------------------|--------------------------------------------------|
| `args_sizes_get`      | Query argc and total argv buffer size            |
| `args_get`            | Fill argv pointer array and string buffer        |
| `environ_sizes_get`   | Query env var count and total buffer size        |
| `environ_get`         | Fill environ pointer array and string buffer     |
| `clock_time_get`      | Read wall or monotonic time (i64 nanoseconds)    |
| `clock_res_get`       | Query clock resolution (i64 nanoseconds)         |
| `random_get`          | Fill memory region with pseudo-random bytes      |
| `sched_yield`         | Cooperative yield (no-op; returns ESUCCESS)      |

Any other imported WASI function returns `ENOSYS` (errno 52) instead of
crashing on a missing import.

## WasiClock and WasiRandom interfaces

These are injected as Lua table objects (duck-typed), enabling you to swap them:

```lua
-- WasiClock interface
-- clock:realtime_ns()     → integer (ns since Unix epoch)
-- clock:monotonic_ns()    → integer (ns, monotonic)
-- clock:resolution_ns(id) → integer (ns)

-- WasiRandom interface
-- random:fill_bytes(n)    → table of n integers in [0, 255]
```

The defaults (`SystemClock` and `SystemRandom`) use Lua's `os.time()`,
`os.clock()`, and `math.random`. For testing, inject deterministic fakes:

```lua
local FakeClock = {}
FakeClock.__index = FakeClock
function FakeClock.new() return setmetatable({}, FakeClock) end
function FakeClock:realtime_ns() return 1700000000000000001 end
function FakeClock:monotonic_ns() return 42000000000 end
function FakeClock:resolution_ns(_id) return 1000000 end
```

## Dependencies

- `coding-adventures-wasm-execution`
- `coding-adventures-wasm-validator`
- `coding-adventures-wasm-module-parser`
- `coding-adventures-wasm-leb128`
- `coding-adventures-wasm-types`
- `coding-adventures-wasm-opcodes`
- `coding-adventures-virtual-machine`

## Development

```bash
# Run all tests (installs deps via luarocks, runs busted)
export PATH="$PATH:/opt/homebrew/bin"
bash BUILD
```

Requires Lua 5.3+ (for 64-bit integer support) and LuaRocks with busted.
