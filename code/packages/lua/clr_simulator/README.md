# coding-adventures-clr-simulator

A Lua simulator for the .NET Common Language Runtime (CLR) Intermediate Language (IL).

## What it does

Executes CLR IL bytecode — the same bytecode format used by C#, F#, and VB.NET programs on the .NET runtime. The simulator is educational: every instruction produces a detailed trace record showing the operand stack before/after, local variables, and a plain-English description.

## How it fits in the stack

```
C# / F# source → CLR Compiler → CLR Bytecode → CLR Simulator (this package)
```

## Key differences from JVM

| Feature     | JVM                | CLR                    |
|-------------|--------------------|-----------------------|
| Typed ops   | `iadd`, `ladd`, `fadd` | `add` (type inferred) |
| Generics    | Type erasure       | Reified at runtime    |
| Value types | Objects only       | Structs on stack      |
| Opcodes     | 1-byte all         | 2-byte prefix (0xFE)  |

## Usage

```lua
local clr = require("coding_adventures.clr_simulator")

-- Assemble: x = 1 + 2
local code = clr.assemble({
    { clr.LDC_I4_1 },  -- push 1
    { clr.LDC_I4_2 },  -- push 2
    { clr.ADD },       -- pop two, push sum
    { clr.STLOC_0 },   -- store in local[0]
    { clr.RET },       -- return
})

local sim = clr.new()
sim = clr.load(sim, code)
local final, traces = clr.run(sim)

print(final.locals[1])  -- 3
for _, t in ipairs(traces) do
    print(string.format("PC=%d  %-12s  %s", t.pc, t.opcode, t.description))
end
```

## Supported instructions

| Opcode         | Hex    | Description                              |
|----------------|--------|------------------------------------------|
| `nop`          | 0x00   | No operation                             |
| `ldnull`       | 0x01   | Push null                                |
| `ldc.i4.0-8`   | 0x16-1E| Push int32 constant 0-8                  |
| `ldc.i4.s`     | 0x1F   | Push signed byte as int32                |
| `ldc.i4`       | 0x20   | Push 32-bit integer (4-byte operand)     |
| `ldloc.0-3`    | 0x06-09| Load local variable 0-3                  |
| `stloc.0-3`    | 0x0A-0D| Store to local variable 0-3              |
| `ldloc.s`      | 0x11   | Load local variable N                    |
| `stloc.s`      | 0x13   | Store to local variable N                |
| `ret`          | 0x2A   | Return                                   |
| `br.s`         | 0x2B   | Unconditional short branch               |
| `brfalse.s`    | 0x2C   | Branch if 0/null                         |
| `brtrue.s`     | 0x2D   | Branch if non-zero/non-null              |
| `add`          | 0x58   | Add (type inferred)                      |
| `sub`          | 0x59   | Subtract                                 |
| `mul`          | 0x5A   | Multiply                                 |
| `div`          | 0x5B   | Integer divide (truncate toward zero)    |
| `ceq`          | 0xFE01 | Compare equal                            |
| `cgt`          | 0xFE02 | Compare greater-than                     |
| `clt`          | 0xFE04 | Compare less-than                        |

## Installation

```sh
luarocks make --local coding-adventures-clr-simulator-0.1.0-1.rockspec
```

## Testing

```sh
cd tests && busted . --verbose --pattern=test_
```
