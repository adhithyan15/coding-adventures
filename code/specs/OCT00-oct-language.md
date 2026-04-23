# OCT00 — Oct Language Specification

## Overview

Oct is a small, statically-typed, 8-bit systems programming language designed to
compile to the Intel 8008 microprocessor (1972).  Like its sibling Nib (which targets
the 4-bit Intel 4004), Oct exposes the native word size and distinctive features of
its target processor directly in the language:

- **8-bit words** — `u8` is the native integer type, matching the 8008's 8-bit ALU
- **Port I/O** — `in(port)` and `out(port, val)` are first-class expressions,
  reflecting the 8008's separate I/O port space (8 input ports, 24 output ports)
- **Carry arithmetic** — `adc()` and `sbb()` expose the 8008's carry/borrow flag for
  multi-byte arithmetic (the hardware mechanism behind 16-bit and 32-bit math on 8-bit
  machines)
- **Rotations** — `rlc()`, `rrc()`, `ral()`, `rar()` expose the four accumulator
  rotation instructions that the 8008 provides for efficient bit manipulation
- **Register discipline** — only 4 general-purpose registers (B, C, D, E) are
  available for local variables, teaching students why function decomposition matters
  on resource-constrained hardware

Oct programs compile through the standard repository IR pipeline:

```
Oct source (.oct)
    ↓  [oct-lexer]        tokenise
    ↓  [oct-parser]       parse to AST
    ↓  [oct-type-checker] type check and annotate
    ↓  [oct-ir-compiler]  lower to compiler_ir IrProgram
    ↓  [intel-8008-ir-validator]   pre-flight IR check
    ↓  [ir-to-intel-8008-compiler] code generate to 8008 assembly
    ↓  [intel-8008-assembler]      two-pass assemble to binary
    ↓  [intel-8008-packager]       produce Intel HEX
    ↓  [intel8008-simulator] / [intel8008-gatelevel]
```

---

## Design Principles

**Principle 1 — Faithfulness to hardware.**  Every language feature maps directly to
one or a small sequence of 8008 instructions.  There are no hidden costs.  A student
reading the generated assembly should be able to trace every Oct construct back to the
instructions they have studied.

**Principle 2 — No silently-ignored constraints.**  If a program exceeds hardware
limits (too many locals, call nesting too deep, port number out of range), the compiler
rejects it with a precise, actionable error message rather than silently truncating or
wrapping.

**Principle 3 — Subset of 8080.**  Every valid Oct program that runs on the 8008
simulator also runs on the Intel 8080 (which is a strict superset of the 8008 ISA).
This means Oct is a foundation for the planned Oct v2 targeting the 8080, which will
add `u16` types backed by 16-bit register-pair arithmetic.

---

## Lexical Structure

### Character Set

Oct source files are ASCII.  UTF-8 is accepted but only ASCII characters may appear
outside string literals and comments (Oct has no string type in v1).

### Comments

Line comments begin with `//` and extend to the end of the line.

```oct
// This is a comment.
let x: u8 = 5;  // so is this
```

Block comments `/* ... */` are not supported in v1.

### Keywords

```
fn  let  static  if  else  while  loop  break  return  true  false
in  out  adc  sbb  rlc  rrc  ral  rar  carry  parity
```

### Identifiers

Identifiers match `[A-Za-z_][A-Za-z0-9_]*` and must not be a keyword.

### Integer Literals

| Form | Example | Range |
|------|---------|-------|
| Decimal | `42`, `255` | 0–255 |
| Hex | `0xFF`, `0x3A` | 0x00–0xFF |
| Binary | `0b10110011` | 0b00000000–0b11111111 |

All integer literals represent `u8` values.  A literal out of the 0–255 range is a
compile-time error.

### Boolean Literals

`true` (encoded as 1) and `false` (encoded as 0).

---

## Type System

Oct has two value types and no pointer types in v1.

### `u8` — Unsigned 8-bit Integer

The primary type.  Values 0–255.  All arithmetic wraps modulo 256 unless the program
explicitly checks for overflow using `carry()`.

```oct
let x: u8 = 200;
let y: u8 = x + 100;  // wraps: y == 44, carry() == true
```

### `bool` — Boolean

Values `true` (1) and `false` (0).  Stored as `u8` but type-checked as distinct.
A `bool` may be used wherever `u8` is expected (implicit coercion), but a `u8`
may not be used where `bool` is expected without an explicit comparison.

```oct
let flag: bool = carry();
if flag { out(1, 0xFF); }
```

### Type Annotations

All variable declarations and function parameters require explicit type annotations.
There is no type inference.

---

## Expressions

### Arithmetic

| Operator | Meaning | IR opcode | 8008 instruction |
|----------|---------|-----------|-----------------|
| `a + b` | add, wrap modulo 256 | `ADD` | `ADD r` |
| `a - b` | subtract, wrap modulo 256 | `SUB` | `SUB r` |

There is no multiplication or division operator.  The Intel 8008 has no hardware
multiply or divide instruction.  Attempting to use `*` or `/` is a compile-time error
with the diagnostic:

```
error: operator '*' is not supported on the Intel 8008
hint: implement multiply as repeated addition or use a shift-and-add loop
```

### Bitwise

| Operator | Meaning | IR opcode | 8008 instruction |
|----------|---------|-----------|-----------------|
| `a & b` | bitwise AND | `AND` | `ANA r` |
| `a \| b` | bitwise OR | `OR` | `ORA r` |
| `a ^ b` | bitwise XOR | `XOR` | `XRA r` |
| `~a` | bitwise NOT | `NOT` | `XRI 0xFF` |

Note: `OR`, `XOR`, and `NOT` require new opcodes in the `compiler_ir` package
(values to be assigned during Phase 1 implementation).  See §Implementation Roadmap.

### Comparisons

| Operator | Meaning | IR opcodes used |
|----------|---------|-----------------|
| `a == b` | equal | `CMP_EQ` |
| `a != b` | not equal | `CMP_NE` |
| `a < b` | less than (unsigned) | `CMP_LT` |
| `a > b` | greater than (unsigned) | `CMP_GT` |
| `a <= b` | less-or-equal (unsigned) | `CMP_GT` + NOT |
| `a >= b` | greater-or-equal (unsigned) | `CMP_LT` + NOT |

All comparisons treat operands as **unsigned** bytes.  The result type is `bool`.

For signed comparisons (checking the S flag), use the `carry()` and `parity()`
intrinsics after a subtraction.  Signed arithmetic is not first-class in Oct v1.

### Logical

| Operator | Meaning |
|----------|---------|
| `a && b` | logical AND (short-circuit) |
| `a \|\| b` | logical OR (short-circuit) |
| `!a` | logical NOT |

These operate on `bool` operands and return `bool`.

### Carry-Aware Arithmetic (Intrinsics)

These intrinsics expose the 8008's carry and borrow flags for multi-byte arithmetic.
They are the building blocks for 16-bit or 32-bit operations on 8-bit hardware.

```
adc(a: u8, b: u8) -> u8
```
Add `a + b + carry_flag`.  Useful as the *high-byte* addition after a low-byte add:

```oct
// 16-bit add: (hi_a:lo_a) + (hi_b:lo_b) → (hi_r:lo_r)
fn add16_lo(lo_a: u8, lo_b: u8) -> u8 {
    return lo_a + lo_b;          // sets carry flag on overflow
}
fn add16_hi(hi_a: u8, hi_b: u8) -> u8 {
    return adc(hi_a, hi_b);      // adds carry from the low-byte addition
}
```

```
sbb(a: u8, b: u8) -> u8
```
Subtract `a - b - carry_flag` (subtract with borrow).  Mirrors `adc` for subtraction.

```
carry() -> bool
```
Read the current carry/borrow flag.  Valid immediately after `+`, `-`, `adc()`,
`sbb()`, or a rotation.  The carry flag is clobbered by AND, OR, and XOR operations.

```oct
let sum: u8 = 200 + 100;  // wraps to 44
if carry() {
    out(0, 0x01);           // signal overflow on port 0
}
```

### Rotation Intrinsics

The Intel 8008 provides four single-instruction byte rotations.  These operate on the
accumulator and are among the most efficient bit-manipulation primitives available on
the chip.

```
rlc(a: u8) -> u8     // Rotate Left Circular
```
Shift `a` left by 1.  The bit that falls off the top (bit 7) wraps to the bottom
(bit 0) and is also stored in the carry flag.

```
     CY ← bit 7
     A  ← (A << 1) | bit7
```

```
rrc(a: u8) -> u8     // Rotate Right Circular
```
Shift `a` right by 1.  Bit 0 wraps to bit 7 and is stored in carry.

```
ral(a: u8) -> u8     // Rotate Left through cArry (9-bit)
```
Rotate `a` and the carry flag together as a 9-bit value shifted left.
The old bit 7 goes into carry; the old carry enters bit 0.

```
     new_CY ← bit 7
     A      ← (A << 1) | old_CY
```

This is the hardware instruction for efficiently multiplying by 2 (or extracting
the high bit into the carry flag for a conditional check).

```
rar(a: u8) -> u8     // Rotate Right through cArry (9-bit)
```
Mirrors `ral` for right rotation.

```
     new_CY ← bit 0
     A      ← (old_CY << 7) | (A >> 1)
```

**Example — count set bits (population count):**

```oct
fn popcount(x: u8) -> u8 {
    let count: u8 = 0;
    let n: u8 = 8;          // loop 8 times, once per bit
    while n != 0 {
        x = rlc(x);         // shift bit 7 into carry
        if carry() {
            count = count + 1;
        }
        n = n - 1;
    }
    return count;
}
```

### `parity(a: u8) -> bool`

Returns `true` if `a` has **even parity** (an even number of 1-bits), `false` if odd.
This reads the P flag that the 8008 hardware computes in parallel with every ALU result.

```oct
let b: u8 = in(0);
if parity(b) {
    out(1, 0x01);   // even parity: output 1
}
```

### Port I/O

```
in(PORT) -> u8
```

Read one byte from input port `PORT` (compile-time constant, 0–7).  The 8008 has 8
input ports wired directly to external hardware (keyboards, ADCs, sensors, etc.).

```
out(PORT, val: u8)
```

Write `val` to output port `PORT` (compile-time constant, 0–23).  The 8008 has 24
output ports wired to displays, DACs, LEDs, serial transmitters, etc.

Port numbers must be **compile-time constants**.  The 8008 encodes the port number
directly in the instruction opcode — there is no "variable port" instruction.

```oct
// Read from sensor port 0, write result to display port 8
let reading: u8 = in(0);
out(8, reading);
```

---

## Statements

### Variable Declaration

```oct
let name: type = expr;
```

Declares a **local variable** bound to a physical register for the lifetime of the
enclosing function.  Local variables are stored in registers B, C, D, or E — not in
memory.  At most **4 local variables** (including function parameters) may be live at
any point within a single function, reflecting the 8008's register file.

```oct
fn clamp(val: u8, limit: u8) -> u8 {  // 2 params (uses B, C)
    let exceeded: bool = val > limit;   // local (uses D)
    if exceeded { return limit; }
    return val;
}
```

If more than 4 locals are needed, the compiler emits:
```
error: register exhaustion — function 'foo' requires 5 locals but the Intel 8008
       provides only 4 general-purpose registers (B, C, D, E)
hint: split this function or promote some values to static variables
```

### Static Variable Declaration

```oct
static NAME: u8 = VALUE;
```

Declares a **global variable** stored in memory (the 8008's 16 KiB address space).
Static variables are read and written via the H:L register pair.  They are accessible
from any function in the program.  Initial value must be a literal (0–255).

```oct
static counter: u8 = 0;

fn tick() -> u8 {
    let c: u8 = counter;
    counter = c + 1;
    return c;
}
```

### Assignment

```oct
name = expr;
```

Assigns to a local variable or static variable.  Types must match.

### Conditional

```oct
if expr {
    statements
}

if expr {
    statements
} else {
    statements
}
```

The condition must be of type `bool`.

### While Loop

```oct
while expr {
    statements
}
```

Repeats as long as `expr` is `true`.  The condition is checked at the **top** of each
iteration (unlike some hardware loops that check at the bottom).

### Infinite Loop

```oct
loop {
    statements
}
```

An unbounded loop.  Useful for event-driven programs (polling I/O ports forever).
Use `break` to exit.

### Break

```oct
break;
```

Exits the innermost `while` or `loop`.

### Return

```oct
return expr;   // return a value (for functions declared with -> type)
return;        // return void
```

A `return` at the top level of `main` causes the processor to execute `HLT`.

---

## Functions

### Declaration

```oct
fn name(param1: type1, param2: type2) -> return_type {
    statements
}
```

Parameters occupy physical registers:

| Parameter index | Physical register |
|----------------|-------------------|
| 1st | B |
| 2nd | C |
| 3rd | D |
| 4th | E |

A function may take at most 4 `u8` parameters.  Each parameter counts against the
4-local limit.

Return values are passed in the accumulator (A register).

### Main Function

Every Oct program must contain exactly one function named `main`:

```oct
fn main() { ... }
```

`main` takes no arguments and has no return value.  The compiler emits a `CAL main`
at address 0, followed by `HLT`, as the program entry point.

### Call Depth

The Intel 8008 has an 8-level push-down stack where one level is always consumed by
the current PC.  This means **at most 7 levels of function call nesting** are
available.  An 8th nested call silently overwrites the oldest return address on the
real hardware.

The compiler tracks call depth statically (via call graph analysis) and rejects
programs that exceed 7 levels:

```
error: call depth from 'main' reaches 8 via main → a → b → c → d → e → f → g → h
       the Intel 8008 stack has only 7 usable levels
```

---

## Complete Program Examples

### Example 1 — Echo input to output

Read bytes continuously from input port 0 and echo them to output port 8.

```oct
fn main() {
    loop {
        let b: u8 = in(0);
        out(8, b);
    }
}
```

### Example 2 — Count to 255

Count from 0 to 255 and output each value to port 1.

```oct
fn main() {
    let n: u8 = 0;
    while n != 255 {
        out(1, n);
        n = n + 1;
    }
    out(1, 255);
}
```

### Example 3 — XOR checksum

Read 8 bytes from port 0, compute their XOR checksum, send the result to port 1.

```oct
fn main() {
    let checksum: u8 = 0;
    let i: u8 = 0;
    while i != 8 {
        let b: u8 = in(0);
        checksum = checksum ^ b;
        i = i + 1;
    }
    out(1, checksum);
}
```

### Example 4 — 16-bit counter using carry

Increment a 16-bit counter stored in two static bytes, output the high byte when it
overflows the low byte.

```oct
static lo: u8 = 0;
static hi: u8 = 0;

fn tick() {
    let l: u8 = lo;
    l = l + 1;
    lo = l;
    if carry() {
        let h: u8 = hi;
        h = h + 1;
        hi = h;
        out(1, h);   // output high byte on overflow
    }
}

fn main() {
    loop {
        tick();
    }
}
```

### Example 5 — Bit reversal using rotations

Reverse the 8 bits of a byte using RAL (rotate left through carry).

```oct
fn reverse_bits(x: u8) -> u8 {
    let result: u8 = 0;
    let i: u8 = 0;
    while i != 8 {
        x = ral(x);           // shift x bit 7 into carry
        result = rar(result); // shift carry into result bit 7
        i = i + 1;
    }
    return result;
}

fn main() {
    let b: u8 = in(0);
    out(1, reverse_bits(b));
}
```

---

## IR Mapping Summary

The Oct IR compiler lowers Oct AST to `compiler_ir.IrProgram`.  The table below
shows which IR opcodes each Oct construct generates.

| Oct construct | IR opcodes generated |
|--------------|---------------------|
| `let x: u8 = v` (literal) | `LOAD_IMM` |
| `let x: u8 = y` (copy) | `ADD_IMM 0` (copy via zero-add) |
| `static x: u8` read | `LOAD_BYTE` |
| `static x = val` write | `STORE_BYTE` |
| `a + b` | `ADD` |
| `a - b` | `SUB` |
| `a + c` (literal c) | `ADD_IMM` |
| `a & b` | `AND` |
| `a \| b` | `OR` *(new opcode)* |
| `a ^ b` | `XOR` *(new opcode)* |
| `~a` | `NOT` *(new opcode)* |
| `a == b` | `CMP_EQ` |
| `a != b` | `CMP_NE` |
| `a < b` | `CMP_LT` |
| `a > b` | `CMP_GT` |
| `if cond` | `BRANCH_Z` or `BRANCH_NZ` |
| `while cond` | `BRANCH_Z` at loop end, `JUMP` at top |
| `fn call` | `CALL` |
| `return v` | `RET` |
| `adc(a, b)` | `SYSCALL 3` *(add with carry)* |
| `sbb(a, b)` | `SYSCALL 4` *(subtract with borrow)* |
| `rlc(a)` | `SYSCALL 11` *(rotate left circular)* |
| `rrc(a)` | `SYSCALL 12` *(rotate right circular)* |
| `ral(a)` | `SYSCALL 13` *(rotate left through carry)* |
| `rar(a)` | `SYSCALL 14` *(rotate right through carry)* |
| `carry()` | `SYSCALL 15` *(read carry flag)* |
| `parity(a)` | `SYSCALL 16` *(read parity flag)* |
| `in(PORT)` | `SYSCALL 20 + PORT` *(read port, PORT ∈ 0–7)* |
| `out(PORT, val)` | `SYSCALL 40 + PORT` *(write port, PORT ∈ 0–23)* |
| halt / end of main | `HALT` |

Notes:
- `OR`, `XOR`, and `NOT` require new `IrOp` values to be added to `compiler-ir`
  before the Oct IR compiler can be implemented.  This is Phase 1 of the roadmap.
- SYSCALL numbers 3–4, 11–16, 20–27, 40–63 are reserved for 8008-specific operations.
  The 8008 IR validator enforces this whitelist.

---

## Constraints and Limits

| Resource | Limit | Reason |
|----------|-------|--------|
| Local variables per function | 4 | Only B, C, D, E are available |
| Function parameters | 4 | Same register file |
| Call nesting depth | 7 | 8008 has 8-level push-down stack; one level is the PC |
| Program size | 16 KB | 8008 has 14-bit address space |
| Input port range | 0–7 | 8008 supports 8 input ports |
| Output port range | 0–23 | 8008 supports 24 output ports |
| Static variable size | 1 byte | Only `u8` is supported in v1 |
| Integer range | 0–255 | 8-bit unsigned |

---

## Implementation Roadmap

The Oct compiler is built in 9 phases, each producing a standalone package:

| Phase | Package | Depends on |
|-------|---------|-----------|
| 0 | Add `OR`, `OR_IMM`, `XOR`, `XOR_IMM`, `NOT` to `compiler-ir` | `compiler-ir` |
| 1 | `intel-8008-ir-validator` | `compiler-ir`, `intel8008-simulator` |
| 2 | `intel-8008-assembler` | *(standalone)* |
| 3 | `intel-8008-packager` | `intel-8008-assembler` |
| 4 | `ir-to-intel-8008-compiler` | `compiler-ir`, `intel-8008-assembler` |
| 5 | `oct-lexer` | `lexer` |
| 6 | `oct-parser` | `oct-lexer`, `parser` |
| 7 | `oct-type-checker` | `oct-parser`, `type-checker-protocol` |
| 8 | `oct-ir-compiler` | `oct-type-checker`, `compiler-ir` |
| 9 | `oct-compiler` | all of the above |

The backend pieces (Phases 1–4) can be built and tested independently of the Oct
language (Phases 5–9) using handwritten IR test programs.

---

## Relationship to Intel 8080

Every Oct program runs on the Intel 8080 unchanged.  The 8080 executes 8008 programs
without modification (its instruction set is a strict superset of the 8008's, with
the same binary encoding for all 8008 instructions).

Oct v2 (a future extension) will add:
- `u16` type backed by 16-bit register pairs (BC, DE, HL)
- `DAD` instruction for 16-bit addition
- Stack-based local storage via `PUSH`/`POP` (the 8080 has a real memory stack,
  unlike the 8008's internal push-down register stack)

---

## Version History

| Version | Date | Description |
|---------|------|-------------|
| 0.1.0 | 2026-04-20 | Initial language specification |
