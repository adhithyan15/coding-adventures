# wasm-opcodes (Go)

Complete WASM 1.0 opcode lookup table — 172 instructions with name, byte value,
category, immediates, and stack effect metadata.

## What it does

This package provides a single source of truth for every WebAssembly 1.0
instruction. You can look up an instruction by its byte value or by its
mnemonic name and get back an `OpcodeInfo` struct with:

- **Name** — the text-format mnemonic (e.g., `"i32.add"`)
- **Opcode** — the byte value in the binary encoding (e.g., `0x6A`)
- **Category** — the instruction group (`"numeric_i32"`, `"control"`, etc.)
- **Immediates** — slice of immediate operand type names encoded after the byte
- **StackPop** — how many values this instruction consumes from the stack
- **StackPush** — how many values it produces onto the stack

## Where it fits in the stack

```
wasm-types      — type system (ValueType, FuncType, etc.)
wasm-leb128     — variable-length integer encoding
wasm-opcodes    ← you are here (instruction metadata table)
wasm-parser     — binary decoder that uses all three layers above
```

## Usage

```go
import wasmopcodes "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-opcodes"

// Look up by byte value
info, ok := wasmopcodes.GetOpcode(0x6A)
if ok {
    fmt.Println(info.Name)      // "i32.add"
    fmt.Println(info.Category)  // "numeric_i32"
    fmt.Println(info.StackPop)  // 2
    fmt.Println(info.StackPush) // 1
}

// Look up by mnemonic
info, ok = wasmopcodes.GetOpcodeByName("memory.grow")
if ok {
    fmt.Printf("0x%02X\n", info.Opcode)    // 0x40
    fmt.Println(info.Immediates)            // [memidx]
}

// Unknown byte/name returns ok=false
_, ok = wasmopcodes.GetOpcode(0xFF)         // ok == false
_, ok = wasmopcodes.GetOpcodeByName("nope") // ok == false

// Direct map access (zero value + ok=false for unknown entries)
info, ok = wasmopcodes.Opcodes[0x6A]
info, ok = wasmopcodes.OpcodesByName["i32.add"]
```

## Opcode categories

| Category | Description | Example |
|----------|-------------|---------|
| `control` | Structured control flow | `block`, `br`, `call` |
| `parametric` | Type-agnostic stack ops | `drop`, `select` |
| `variable` | Local/global access | `local.get`, `global.set` |
| `memory` | Loads, stores, size/grow | `i32.load`, `memory.grow` |
| `numeric_i32` | 32-bit integer arithmetic | `i32.add`, `i32.lt_s` |
| `numeric_i64` | 64-bit integer arithmetic | `i64.mul`, `i64.eqz` |
| `numeric_f32` | 32-bit float arithmetic | `f32.sqrt`, `f32.copysign` |
| `numeric_f64` | 64-bit float arithmetic | `f64.div`, `f64.nearest` |
| `conversion` | Type conversions | `i32.wrap_i64`, `f64.promote_f32` |

## Stack effects explained

WASM is a *stack machine*. Instructions communicate via an implicit operand
stack rather than named registers. `StackPop` and `StackPush` tell you how many
values each instruction takes from and gives back to that stack.

For example, `i32.add` pops two i32 values and pushes their sum:

```
Before: [ a (i32) | b (i32) ]   ← b is on top
After:  [ a+b (i32) ]
```

## Development

```bash
cd code/packages/go/wasm-opcodes
go test ./... -v -cover
go vet ./...
```

## Dependencies

- Go 1.26+
- `wasm-types` (for context in the broader parsing stack)
