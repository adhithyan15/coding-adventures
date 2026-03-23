# wasm_module_parser

Parse raw `.wasm` binary bytes into a structured `WasmModule`. No execution — pure decoding.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) monorepo,
a ground-up implementation of the computing stack from transistors to operating systems.

## What it does

A WebAssembly (WASM) binary file is a compact, structured sequence of bytes. This package
decodes those bytes — header, sections, and payloads — into the typed `WasmModule` struct
from `wasm_types`, ready for validation, interpretation, or further analysis.

This Elixir implementation uses idiomatic binary pattern matching (`<<>>`) throughout.
There are no mutable data structures; each decoder function receives and returns binaries
in pure functional style.

## Where it fits in the stack

```
wasm_leb128      ←── variable-length integer decoding
wasm_types       ←── WasmModule struct and all sub-types
wasm_opcodes     ←── opcode constants (used transitively)
wasm_module_parser  ←── THIS MODULE: binary → WasmModule
wasm_simulator   ←── execution (depends on this module)
```

## Usage

```elixir
alias CodingAdventures.WasmModuleParser

# Read a .wasm file and parse it
bytes = File.read!("module.wasm")

case WasmModuleParser.parse(bytes) do
  {:ok, module} ->
    IO.puts("types:     #{length(module.types)}")
    IO.puts("imports:   #{length(module.imports)}")
    IO.puts("exports:   #{length(module.exports)}")
    IO.puts("functions: #{length(module.functions)}")
    IO.puts("code:      #{length(module.code)}")

  {:error, message} ->
    IO.puts("parse error: #{message}")
end
```

## WASM Binary Format (overview)

```
┌──────────────────────────────────────────────────────────────────┐
│  Magic: <<0x00, 0x61, 0x73, 0x6D>>  ("\\0asm")                   │
│  Version: <<0x01, 0x00, 0x00, 0x00>>                             │
├──────┬───────────────────────────────────────────────────────────┤
│ §  0 │ Custom  — tool metadata (debug names, source maps, DWARF) │
│ §  1 │ Type    — function signature pool                         │
│ §  2 │ Import  — things needed from the host                     │
│ §  3 │ Function— type indices for local functions                │
│ §  4 │ Table   — function reference tables                       │
│ §  5 │ Memory  — linear memory declarations                      │
│ §  6 │ Global  — module-level globals with init expressions      │
│ §  7 │ Export  — names exposed to the host                       │
│ §  8 │ Start   — optional auto-called function index             │
│ §  9 │ Element — table initialisation data                       │
│ § 10 │ Code    — function bodies (locals + bytecode)             │
│ § 11 │ Data    — memory initialisation data                      │
└──────┴───────────────────────────────────────────────────────────┘
```

## API

```
parse(data :: binary()) :: {:ok, WasmModule.t()} | {:error, String.t()}
```

## Dependencies

- `coding_adventures_wasm_leb128` — LEB128 unsigned integer decoding
- `coding_adventures_wasm_types` — `WasmModule` and all sub-types
- `coding_adventures_wasm_opcodes` — opcode constants (transitive)

## Development

```bash
# Install deps
mix deps.get

# Run tests with coverage
mix test --cover

# Run just tests
mix test
```
