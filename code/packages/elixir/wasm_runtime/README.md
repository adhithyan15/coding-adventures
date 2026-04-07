# wasm-runtime (Elixir)

WebAssembly 1.0 execution runtime for the `coding-adventures` monorepo.

This package is the final layer of the WASM execution pipeline:

    wasm_leb128        — LEB128 variable-length integer encoding
    wasm_types         — shared type definitions (WasmModule, FuncType, etc.)
    wasm_opcodes       — opcode constants
    wasm_module_parser — binary WASM parser
    wasm_validator     — validation (type-checking, import resolution)
    wasm_execution     — linear memory, value types, instruction dispatch
    wasm_runtime       ← YOU ARE HERE: instantiation + execution + WASI

## What it does

- **Engine** (`engine.ex`) — instantiates a `ValidatedModule` into an
  execution context (linear memory, globals, function bodies), then runs
  the GenericVM's step loop handling nested calls.

- **Instance** (`instance.ex`) — wraps Engine with a simple
  `from_validated/2` + `call/3` API.

- **Runtime** (`runtime.ex`) — one-shot `instantiate_bytes/1` + `call/3`
  convenience API that runs the full parse → validate → instantiate pipeline.

- **WasiStub** (`wasi_stub.ex`) — WASI host functions for
  `wasi_snapshot_preview1`. Implements two tiers:

  - **Tier 1**: proc_exit, fd_write, fd_read, fd_close, fd_seek (stubs)
  - **Tier 3**: args_sizes_get, args_get, environ_sizes_get, environ_get,
    clock_res_get, clock_time_get, random_get, sched_yield (full implementations)

## WASI Tier 3 design

Clock and random are injected as Elixir behaviours so they can be swapped
for deterministic fakes in tests:

```elixir
defmodule FakeClock do
  @behaviour CodingAdventures.WasmRuntime.WasiClock
  def realtime_ns(), do: 1_700_000_000_000_000_001
  def monotonic_ns(), do: 42_000_000_000
  def resolution_ns(_id), do: 1_000_000
end

defmodule FakeRandom do
  @behaviour CodingAdventures.WasmRuntime.WasiRandom
  def fill_bytes(n), do: :binary.list_to_bin(List.duplicate(0xAB, n))
end

config = %CodingAdventures.WasmRuntime.WasiConfig{
  args: ["myapp", "--flag"],
  env: %{"HOME" => "/home/user"},
  clock: FakeClock,
  random: FakeRandom
}

host_fns = CodingAdventures.WasmRuntime.WasiStub.host_functions(config)
```

Memory-writing WASI functions are called via `WasiStub.call_with_memory/4`:

```elixir
memory = CodingAdventures.WasmExecution.LinearMemory.new(1)
wasm_args = [Values.i32(0), Values.i32(4)]  # argc_ptr=0, buf_size_ptr=4

{results, updated_memory} =
  CodingAdventures.WasmRuntime.WasiStub.call_with_memory(
    host_fns, "args_sizes_get", wasm_args, memory
  )

# results => [%{type: 0x7F, value: 0}]  (errno 0 = success)
# LinearMemory.load_i32(updated_memory, 0) => 2  (argc)
# LinearMemory.load_i32(updated_memory, 4) => 12 (buf_size for ["myapp","hello"])
```

## Quick start

```elixir
# Run a WASM module
{:ok, instance} = CodingAdventures.WasmRuntime.Runtime.instantiate_bytes(wasm_bytes)
results = CodingAdventures.WasmRuntime.Runtime.call(instance, "square", [Values.i32(5)])
# => [%{type: 0x7F, value: 25}]

# With WASI config
host_fns = CodingAdventures.WasmRuntime.WasiStub.host_functions(%WasiConfig{
  args: ["myapp"],
  env: %{"HOME" => "/tmp"}
})
{:ok, instance} = Instance.from_validated(validated, host_fns)
```

## Dependencies

- wasm-leb128
- wasm-types
- wasm-opcodes
- wasm-module-parser
- virtual-machine
- wasm-validator
- wasm-execution

## Development

```bash
# Run tests
cd code/packages/elixir/wasm_runtime
mix deps.get && mix test
```
