# wasm_module_parser

Parse a raw `.wasm` binary into a structured `WasmModule`. Takes bytes,
produces structured data. No execution.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures)
ground-up computing stack.

## What It Does

This gem reads the binary WebAssembly (`.wasm`) format and decodes every
section into typed, inspectable data structures. It covers all 12 section types
in the WASM 1.0 specification:

| Section ID | Name     | What it contains                                |
|------------|----------|-------------------------------------------------|
| 0          | Custom   | Named byte blobs (debug info, metadata)         |
| 1          | Type     | Function signatures (params + results)          |
| 2          | Import   | Host-provided functions/tables/memories/globals |
| 3          | Function | Type-section indices for local functions        |
| 4          | Table    | Indirect-call tables (funcref arrays)           |
| 5          | Memory   | Linear memory declarations                      |
| 6          | Global   | Module-level global variables                   |
| 7          | Export   | Host-visible definitions                        |
| 8          | Start    | Entry-point function index                      |
| 9          | Element  | Table initializer segments                      |
| 10         | Code     | Function bodies with locals and bytecode        |
| 11         | Data     | Memory initializer segments                     |

## Installation

Add to your Gemfile:

```ruby
gem "coding_adventures_wasm_module_parser"
```

## Usage

```ruby
require "coding_adventures_wasm_module_parser"

parser = CodingAdventures::WasmModuleParser::Parser.new

begin
  # Accepts binary String or Array of Integer bytes
  bytes = File.binread("example.wasm")
  mod   = parser.parse(bytes)

  # Inspect types
  mod.types.each do |t|
    puts "FuncType: (#{t.params.inspect}) → (#{t.results.inspect})"
  end

  # Inspect exports
  mod.exports.each do |e|
    puts "Export: #{e.name} (kind=#{e.kind}, index=#{e.index})"
  end

rescue CodingAdventures::WasmModuleParser::WasmParseError => e
  puts "Parse failed at byte offset 0x#{e.offset.to_s(16)}: #{e.message}"
end
```

## API

### `Parser`

```ruby
parser = CodingAdventures::WasmModuleParser::Parser.new
mod    = parser.parse(data)  # data: String (binary) or Array<Integer>
                              # raises WasmParseError on malformed input
```

The parser is stateless between calls. Reuse the same instance for multiple files.

### `WasmParseError`

```ruby
class WasmParseError < StandardError
  attr_reader :offset  # byte offset where the problem was detected
end
```

### `WasmModule` (from `coding_adventures_wasm_types`)

```ruby
class WasmModule
  attr_accessor :types      # Array[FuncType]
  attr_accessor :imports    # Array[Import]
  attr_accessor :functions  # Array[Integer]  (type indices)
  attr_accessor :tables     # Array[TableType]
  attr_accessor :memories   # Array[MemoryType]
  attr_accessor :globals    # Array[Global]
  attr_accessor :exports    # Array[Export]
  attr_accessor :start      # Integer | nil
  attr_accessor :elements   # Array[Element]
  attr_accessor :code       # Array[FunctionBody]
  attr_accessor :data       # Array[DataSegment]
  attr_accessor :customs    # Array[CustomSection]
end
```

## WASM Binary Format Overview

```
┌─────────────────────────────────────────────────────────────┐
│  Magic:   0x00 0x61 0x73 0x6D  ("\0asm")  — bytes 0–3       │
│  Version: 0x01 0x00 0x00 0x00  (1)        — bytes 4–7       │
├─────────────────────────────────────────────────────────────┤
│  Section: id:u8 + size:u32leb + payload:bytes               │
│  Section: id:u8 + size:u32leb + payload:bytes               │
│  ...                                                        │
└─────────────────────────────────────────────────────────────┘
```

All integers in the payload are ULEB128-encoded (variable-length).

## Dependencies

- [`coding_adventures_wasm_leb128`](../wasm_leb128) — ULEB128/SLEB128 decode/encode
- [`coding_adventures_wasm_types`](../wasm_types) — `WasmModule` and all type definitions
- [`coding_adventures_wasm_opcodes`](../wasm_opcodes) — opcode table

## Development

```bash
bundle install
bundle exec rake test
```

## Layer in the Stack

```
wasm_module_parser  ← you are here (parses binary → WasmModule)
wasm_types          ← WasmModule, FuncType, Import, Export, ... (data model)
wasm_leb128         ← LEB128 variable-length integer encoding
wasm_opcodes        ← opcode table for all 183 WASM 1.0 instructions
```
