# coding-adventures-jvm-simulator

A Lua simulator for the Java Virtual Machine (JVM) bytecode instruction set.

## What it does

Executes JVM bytecode — the format used by Java, Kotlin, Scala, Clojure, and Groovy programs. Each executed instruction produces a trace record with full before/after state, making it ideal for learning and debugging.

## How it fits

```
Java/Kotlin source → javac → JVM Bytecode (.class) → JVM Simulator (this package)
```

## Key design: typed opcodes

Unlike the CLR (`add` for all types), JVM has separate opcodes per type:

- `iadd` — integer (32-bit) add
- `ladd` — long (64-bit) add
- `fadd` — float add
- `dadd` — double add

This simulator implements the `i` (integer) variants. Real JVM programs use these for all 32-bit integer math.

## Usage

```lua
local jvm = require("coding_adventures.jvm_simulator")

-- Assemble: x = 1 + 2
local code = jvm.assemble({
    jvm.encode_iconst(1),   -- iconst_1
    jvm.encode_iconst(2),   -- iconst_2
    { jvm.IADD },           -- iadd
    jvm.encode_istore(0),   -- istore_0
    { jvm.RETURN },         -- return
})

local sim = jvm.new()
sim = jvm.load(sim, code)
local final, traces = jvm.run(sim)
print(final.locals[1])  -- 3
```

## Supported instructions

| Opcode      | Hex  | Description                         |
|-------------|------|-------------------------------------|
| `iconst_0-5`| 0x03-08| Push int 0-5                      |
| `bipush`    | 0x10 | Push signed byte as int             |
| `sipush`    | 0x11 | Push signed short as int            |
| `ldc`       | 0x12 | Push from constant pool             |
| `iload_0-3` | 0x1A-1D| Load int from local 0-3           |
| `iload`     | 0x15 | Load int from local N               |
| `istore_0-3`| 0x3B-3E| Store int to local 0-3            |
| `istore`    | 0x36 | Store int to local N                |
| `iadd`      | 0x60 | Add two ints                        |
| `isub`      | 0x64 | Subtract                            |
| `imul`      | 0x68 | Multiply                            |
| `idiv`      | 0x6C | Integer divide (truncate toward 0)  |
| `if_icmpeq` | 0x9F | Branch if ints equal                |
| `if_icmpgt` | 0xA3 | Branch if int > int                 |
| `goto`      | 0xA7 | Unconditional branch                |
| `ireturn`   | 0xAC | Return int from method              |
| `return`    | 0xB1 | Return void                         |

## Installation

```sh
luarocks make --local coding-adventures-jvm-simulator-0.1.0-1.rockspec
```

## Testing

```sh
cd tests && busted . --verbose --pattern=test_
```
