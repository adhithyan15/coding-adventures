<!-- learning-concepts: wasm-execution, wasm-leb128, wasm-module-parser, wasm-opcodes, wasm-runtime, wasm-types, wasm-validator, wasm-module-encoder, brainfuck-wasm-compiler, nib-wasm-compiler, ir-to-wasm-compiler -->
# WebAssembly From Bytes To Execution

WebAssembly is often introduced as "assembly for the web." That description
misses its most useful idea: WebAssembly is a compact, typed, portable machine
format with a deliberately small contract between producers and runtimes.

A compiler does not need to know which CPU will eventually run the program. A
runtime does not need to understand the source language. They meet at a
validated module.

```text
source language
    |
    v
Wasm module bytes
    |
    +--> decode --> validate --> instantiate --> execute
```

Each arrow has a different responsibility. Keeping those responsibilities
separate makes malformed input easier to reject and execution easier to test.

## The Binary Container

A binary module begins with a magic number and version, followed by sections.
Sections describe types, imports, functions, tables, memories, globals, exports,
code, and data. Most sections contain a vector:

```text
count, item 0, item 1, ... item n
```

Integers use LEB128, a variable-length encoding. Seven bits of each byte carry
data; the high bit says whether another byte follows. For unsigned 624485:

```text
data groups:  0x65  0x08  0x26
wire bytes:   0xE5  0x88  0x26
               ^     ^
               more  more
```

Variable-length integers save space for common small values, but they create a
security boundary. A decoder must cap the number of bytes, detect overflow, and
reject an input that never terminates. "Keep reading while the high bit is set"
is not sufficient for hostile bytes.

The module parser turns bytes into structure. It should answer questions such
as "what functions are declared?" and "what instructions are in this body?"
It should not execute those instructions or assume they are type-correct.

## Types And Opcodes

WebAssembly 1.0 has four basic numeric value types:

| Type | Meaning |
| --- | --- |
| `i32` | 32-bit bit pattern used by signed and unsigned integer operations |
| `i64` | 64-bit bit pattern used by signed and unsigned integer operations |
| `f32` | IEEE 754 binary32 floating point |
| `f64` | IEEE 754 binary64 floating point |

Signedness belongs to an operation, not to an integer value. The same `i32`
can flow into `i32.lt_s` or `i32.lt_u` and be interpreted differently.

An opcode identifies an instruction. Some instructions carry immediate data:
a local index, branch depth, memory alignment, or constant. An opcode table is
therefore shared knowledge between decoders, validators, encoders, disassemblers,
and execution engines.

## Validation Is Abstract Execution

Parsing proves that bytes have a legal shape. Validation proves that the module
could execute without violating WebAssembly's static rules.

The validator tracks an abstract operand stack. For:

```wat
i32.const 6
i32.const 7
i32.mul
```

the stack evolves as:

```text
[] -> [i32] -> [i32, i32] -> [i32]
```

`i32.mul` requires two `i32` operands and produces one. If the second constant
were `f64`, the bytes could still parse, but validation would fail.

Validation also checks index bounds, branch result types, mutability, function
signatures, memory and table limits, and the special stack-polymorphic rules for
unreachable code. The key invariant is:

> A validated instruction sequence has one consistent type effect at every
> reachable control-flow edge.

This is why a runtime should validate before instantiation instead of scattering
type checks through every hot execution path.

## Instantiation Connects A Module To A World

A module is still a template after validation. Instantiation:

1. resolves imported functions, memories, tables, and globals;
2. allocates module-defined state;
3. evaluates constant initializers;
4. applies element and data segments;
5. exposes the requested exports;
6. optionally invokes a start function.

Imports are capability boundaries. A pure module cannot print, open a file, or
read a clock unless the host gives it a function that does so. WASI is one
standardized host interface, but a small educational runtime can define a much
narrower one.

## Execution Uses Two Stacks

The operand stack stores values. The call stack stores function activations:
locals, return position, and control state.

```text
call add
  call stack:    [caller, add]
  operand stack: [..., 6, 7]

return 13
  call stack:    [caller]
  operand stack: [..., 13]
```

Structured control instructions such as `block`, `loop`, `if`, and `br` avoid
arbitrary jumps. A branch targets a nesting depth, so the runtime can restore
the operand stack to the target block's expected shape.

Linear memory is a growable byte array divided into 64 KiB pages. Loads and
stores calculate an effective address and must trap when the requested range is
out of bounds. Tables hold references used by indirect calls, whose signatures
must still match at runtime.

## Producers And Consumers

A module encoder performs the parser's job in reverse: it writes canonical
section structure, lengths, opcodes, and immediates. Source compilers such as
the Brainfuck and Nib Wasm compilers are producers. They choose a memory layout
and lower source operations into typed Wasm instructions.

The clean composition is:

```text
frontend -> typed IR -> Wasm encoder -> module bytes
                                      |
                                      v
parser -> validator -> runtime -> observable result
```

Round-trip and differential tests connect both sides. Encode a module, parse it
again, validate it, run it here, and compare its result with a mature external
runtime. Each comparison catches a different class of mistake.

## Failure Checklist

When implementing or reviewing a Wasm layer, ask:

- Can a length or LEB128 value overflow?
- Are section and index bounds checked before allocation or lookup?
- Does validation model unreachable control flow correctly?
- Are memory address additions checked for overflow?
- Are instruction and call budgets available for untrusted modules?
- Are host functions explicit capabilities?
- Do encoder and parser agree on canonical forms?

The repository's detailed contract is
[`W01-wasm-runtime.md`](../../specs/W01-wasm-runtime.md). The package split
mirrors this lesson: bytes, types, opcodes, parsing, validation, execution, and
runtime orchestration remain independently testable.
