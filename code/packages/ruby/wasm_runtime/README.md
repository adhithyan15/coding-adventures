# wasm-runtime (Ruby)

Complete WebAssembly 1.0 runtime for Ruby. Composes the parser, validator, and execution engine into a single user-facing API.

## Pipeline

```
.wasm bytes  ->  Parse  ->  Validate  ->  Instantiate  ->  Execute
    |              |           |             |              |
  String      WasmModule  ValidatedModule  WasmInstance  WasmValue[]
```

## Usage

```ruby
require "coding_adventures_wasm_runtime"

# Simple: compute square(5) from a .wasm binary
runtime = CodingAdventures::WasmRuntime::Runtime.new
result = runtime.load_and_run(wasm_bytes, "square", [5])
# result = [25]

# Step by step
wasm_module = runtime.load(wasm_bytes)
runtime.validate(wasm_module)
instance = runtime.instantiate(wasm_module)
result = runtime.call(instance, "square", [5])
```

## WASI Support

`WasiStub` implements the `wasi_snapshot_preview1` ABI in tiers:

| Tier | Functions |
|------|-----------|
| 1 | `fd_write`, `proc_exit` |
| 3 | `args_sizes_get`, `args_get`, `environ_sizes_get`, `environ_get`, `clock_res_get`, `clock_time_get`, `random_get`, `sched_yield` |

Clock and random behaviour are injectable for testing:

```ruby
class FakeClock
  def realtime_ns  = 0
  def monotonic_ns = 0
  def resolution_ns(_id) = 1_000_000
end

class FakeRandom
  def fill_bytes(n) = Array.new(n, 0)
end

wasi = CodingAdventures::WasmRuntime::WasiStub.new(
  args:   ["myapp", "--flag"],
  env:    {"HOME" => "/home/user"},
  stdout: ->(text) { print text },
  clock:  FakeClock.new,
  random: FakeRandom.new
)
```

Any unimplemented WASI function returns ENOSYS (52).

## Dependencies

- wasm-leb128
- wasm-types
- wasm-opcodes
- wasm-module-parser
- virtual-machine
- wasm-execution
- wasm-validator

## Development

```bash
bundle install
bundle exec rake test
```
