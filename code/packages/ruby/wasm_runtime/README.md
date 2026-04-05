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
