# swift/wasm-simulator

A pure Swift implementation of the minimal WebAssembly stack-machine simulator
specified in [`07c-wasm-simulator.md`](../../../specs/07c-wasm-simulator.md).
It models the six-instruction educational subset shared by the other language
ports:

| Opcode | Instruction | Effect |
|---:|---|---|
| `0x0B` | `end` | Halt execution |
| `0x20` | `local.get` | Push a local onto the operand stack |
| `0x21` | `local.set` | Pop a value into a local |
| `0x41` | `i32.const` | Push a signed 32-bit constant |
| `0x6A` | `i32.add` | Add the top two values with i32 wrapping |
| `0x6B` | `i32.sub` | Subtract the top two values with i32 wrapping |

Every step records the program counter, decoded instruction, stack before and
after execution, local snapshot, description, and halted state.

## Usage

```swift
import WasmSimulator

let program = assembleWasm([
    encodeI32Const(1),
    encodeI32Const(2),
    encodeI32Add(),
    encodeLocalSet(0),
    encodeEnd(),
])

let simulator = WasmSimulator(localCount: 4)
let traces = try simulator.run(program)

assert(traces.count == 5)
assert(simulator.locals[0] == 3)
```

## Development

```sh
swift test
```
