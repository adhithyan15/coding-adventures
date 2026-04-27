# NIB00 — Nib Language Specification

## Overview

Nib is a safe, statically-typed toy programming language designed to compile down to
Intel 4004 machine code. The name is a pun on "nibble" — the 4-bit quantity that is
the native data size of the 4004. You write Nib source code; the compiler produces a
binary image that can be burned to 4004 ROM.

### Why Nib Exists

The Intel 4004 (released November 1971) was the world's first commercial
microprocessor. Federico Faggin, Ted Hoff, and Stanley Mazor designed it at Intel for
the Busicom 141-PF desktop calculator. It was a 4-bit CPU with:

- 2,300 transistors
- 740 kHz clock speed
- 4-bit data registers
- 12-bit program counter (addresses 4 KB of ROM)
- A **3-level hardware call stack** (no software stack in RAM)
- 640 **nibbles** (320 bytes) of RAM — with only 160 bytes addressable as general RAM
- 46 instructions

Writing software for the 4004 in raw assembly is error-prone:

- The programmer must manually track which registers are live
- Overflow behavior is implicit and easy to miss
- The 3-level stack constraint is not enforced by the assembler
- BCD arithmetic requires manual DAA (decimal adjust) instructions

Nib solves this by providing a higher-level language that:

1. **Expresses overflow semantics explicitly** — `+%` wraps, `+?` saturates
2. **Enforces hardware constraints statically** — call depth, RAM usage, no recursion
3. **Gives the compiler enough information** to generate correct 4004 code
4. **Reads like modern code** while producing code that would run on 1971 silicon

### Relationship to the Coding-Adventures Stack

Nib sits on top of the existing Intel 4004 simulator layer:

```
Logic Gates → Arithmetic → 4004 Gate-Level → 4004 Behavioral Simulator
                                                          ↑
Nib Source → [Lexer] → [Parser] → [Semantic Checker] → [4004 Code Generator] → ROM Binary
```

The Nib compiler does NOT go through the generic VM layer. It targets 4004 assembly
directly, then assembles to Intel HEX format for burning to ROM.

---

## Intel 4004 Hardware Constraints

Understanding these constraints is essential to understanding why Nib looks the way
it does.

### Register File

The 4004 has **16 registers**, each holding 4 bits (one nibble). They are numbered R0
through R15. Registers are also organized as 8 **register pairs** for 8-bit operations:

| Pair | High Register | Low Register |
|------|--------------|-------------|
| P0   | R0           | R1          |
| P1   | R2           | R3          |
| P2   | R4           | R5          |
| P3   | R6           | R7          |
| P4   | R8           | R9          |
| P5   | R10          | R11         |
| P6   | R12          | R13         |
| P7   | R14          | R15         |

A pair holds an 8-bit value: `pair_value = (R_high << 4) | R_low`.

### The Accumulator

Most arithmetic goes through a special register called the **accumulator** (A). It is
also 4 bits wide. The 4004 is an **accumulator architecture**: you load a value into
A, perform an operation, and the result is stored back in A.

This is very different from modern CPUs (x86, ARM, RISC-V) where any register can be
the destination of any instruction.

### Call Stack

The 4004 has a **hardware call stack** with exactly **3 levels**. When you call a
subroutine (JMS instruction), the 12-bit return address is pushed onto this stack.
When the subroutine returns (BBL instruction), the address is popped. There is no
software stack, no stack pointer, and no way to save more than 3 return addresses.

Consequence: **maximum call depth is 3**. If you have function A calling function B
calling function C calling function D, function D has no way to return — it would
need a 4th stack level that does not exist.

Nib enforces: the static call graph must have depth ≤ 2 (so with the main function
occupying level 1, the chain A→B→C is the deepest allowed).

### RAM

The 4004's RAM is organized in a hierarchy of banks → registers → characters:

```
4 banks × 4 registers/bank × 16 main characters/register = 256 nibbles (128 bytes)
4 banks × 4 registers/bank × 4 status characters/register = 64 nibbles (32 bytes)
Total: 320 nibbles = 160 bytes
```

But only the main characters are general-purpose storage. Status characters are
reserved for I/O. So the usable general RAM is:

```
256 nibbles = 128 bytes of main RAM
```

The 4004 documentation often cites "640 nibbles" but most of that is status RAM
and output port latches. Nib conservatively limits static RAM to 160 bytes (the
total of main + status, as a safe upper bound the compiler can check statically).

### ROM

4 KB of ROM (4096 × 8-bit bytes). This holds the program code. Nib programs compile
to 4004 assembly and then to binary ROM images. The compiler must ensure the compiled
program fits within 4096 bytes.

### No Multiply, No Divide

The 4004 instruction set has **no multiplication or division instructions**. It has:
- ADD (add to accumulator with carry)
- SUB (subtract from accumulator with borrow)
- Logical operations (AND, OR, XOR, complement)
- Rotate (RAL/RAR — rotate accumulator left/right through carry)
- Increment/decrement for registers and pairs

Multiplication requires a software loop; division requires long division in software.
Both are expensive in ROM and RAM. Nib v1 omits these operations entirely.

---

## Type System

Nib has exactly four types. This minimal type system reflects the 4004's native data
sizes.

### `u4` — Unsigned 4-bit Integer

- Storage: 1 nibble (one 4004 register, R0–R15)
- Range: 0 to 15 (inclusive)
- Operations: `+`, `-`, `+%`, `+?`, `&`, `|`, `^`, `~`, comparisons

The 4004's natural word size. Most BCD digits, nibble masks, and small counters fit
in u4. Overflow wraps unless you use `+%` (explicit wrap) or `+?` (saturate).

### `u8` — Unsigned 8-bit Integer

- Storage: 1 register pair (two nibbles in adjacent registers, e.g. R0:R1 = P0)
- Range: 0 to 255 (inclusive)
- Operations: same as u4, but multi-step on the 4004

An 8-bit quantity. Useful for byte addresses, loop trip counts up to 255, and
accumulating two BCD digits. The 4004 handles 8-bit values by operating on each
nibble separately, then combining them.

### `bcd` — Binary-Coded Decimal

- Storage: 1 nibble (same as u4)
- Range: 0 to 9 (values 10–15 are invalid — compiler error if proven out-of-range)
- Operations: `+` (with automatic DAA), comparisons

BCD is the 4004's raison d'être. The original Busicom calculator stored each decimal
digit in one BCD nibble. The 4004's `DAA` instruction adjusts the result of an ADD
to keep the value in the 0–9 range.

In Nib, `bcd +` automatically emits an ADD followed by DAA. You do not need to write
DAA yourself. Out-of-range BCD literals (e.g. `bcd = 10`) are rejected at compile
time.

### `bool` — Boolean

- Storage: 1 nibble (stored as 0 for false, 1 for true)
- Range: `true` (1) or `false` (0)
- Operations: `!`, `&&`, `||`, `==`, `!=`

The 4004 has no dedicated boolean type. Booleans are stored as nibbles with the
convention that 0 = false and non-zero = true. The compiler generates appropriate
test-and-branch sequences for logical operators.

---

## Operators

### Arithmetic Operators

| Operator | Name              | Types        | Description                                      |
|----------|-------------------|--------------|--------------------------------------------------|
| `+`      | Add               | u4, u8, bcd  | Add. Compiler warns if overflow is provable.     |
| `-`      | Subtract          | u4, u8       | Subtract. Borrows propagate for multi-nibble.    |
| `+%`     | Wrapping Add      | u4, u8       | Add, then mask to type width. Never overflows.   |
| `+?`     | Saturating Add    | u4, u8, bcd  | Add, then clamp to type maximum. Never wraps.    |

Note: `*` and `/` are reserved tokens but **not available in v1**. See "What's NOT in v1".

### Bitwise Operators

| Operator | Name              | Types        | Description                                      |
|----------|-------------------|--------------|--------------------------------------------------|
| `&`      | Bitwise AND       | u4, u8       | ANL instruction on the 4004.                     |
| `\|`     | Bitwise OR        | u4, u8       | ORL instruction on the 4004.                     |
| `^`      | Bitwise XOR       | u4, u8       | XRL instruction on the 4004.                     |
| `~`      | Bitwise NOT       | u4, u8       | CMA (complement accumulator) on the 4004.        |

### Comparison Operators

| Operator | Name              | Result | Description                                      |
|----------|-------------------|--------|--------------------------------------------------|
| `==`     | Equal             | bool   | Implemented via SUB + zero check.               |
| `!=`     | Not Equal         | bool   | Implemented via SUB + non-zero check.           |
| `<`      | Less Than         | bool   | Implemented via SUB + carry check.              |
| `>`      | Greater Than      | bool   | Implemented via SUB with operands swapped.       |
| `<=`     | Less or Equal     | bool   | Combination of == and <.                        |
| `>=`     | Greater or Equal  | bool   | Combination of == and >.                        |

### Logical Operators

| Operator | Name              | Result | Description                                      |
|----------|-------------------|--------|--------------------------------------------------|
| `&&`     | Logical AND       | bool   | Short-circuit: right not evaluated if left false.|
| `\|\|`   | Logical OR        | bool   | Short-circuit: right not evaluated if left true. |
| `!`      | Logical NOT       | bool   | Negates a boolean value.                         |

### Operator Precedence Table

Listed from lowest precedence (evaluated last) to highest (evaluated first):

| Level | Operators           | Associativity |
|-------|---------------------|---------------|
| 1     | `\|\|`              | Left          |
| 2     | `&&`                | Left          |
| 3     | `==`, `!=`          | Left          |
| 4     | `<`, `>`, `<=`, `>=`| Left          |
| 5     | `+`, `-`, `+%`, `+?`| Left          |
| 6     | `&`, `\|`, `^`      | Left          |
| 7     | `!`, `~` (unary)    | Right (prefix)|
| 8     | Primary             | —             |

---

## Grammar Reference

The complete grammar in EBNF notation (matches `code/grammars/nib.grammar`):

```
program       = { top_decl } ;

top_decl      = const_decl | static_decl | fn_decl ;

const_decl    = "const" NAME ":" type "=" expr ";" ;
static_decl   = "static" NAME ":" type "=" expr ";" ;

fn_decl       = "fn" NAME "(" [ param_list ] ")" [ "->" type ] block ;
param_list    = param { "," param } ;
param         = NAME ":" type ;

block         = "{" { stmt } "}" ;

stmt          = let_stmt
              | assign_stmt
              | return_stmt
              | for_stmt
              | if_stmt
              | expr_stmt
              ;

let_stmt      = "let" NAME ":" type "=" expr ";" ;
assign_stmt   = NAME "=" expr ";" ;
return_stmt   = "return" expr ";" ;
for_stmt      = "for" NAME ":" type "in" expr ".." expr block ;
if_stmt       = "if" expr block [ "else" block ] ;
expr_stmt     = expr ";" ;

type          = "u4" | "u8" | "bcd" | "bool" ;

expr          = or_expr ;
or_expr       = and_expr { "||" and_expr } ;
and_expr      = eq_expr { "&&" eq_expr } ;
eq_expr       = cmp_expr { ( "==" | "!=" ) cmp_expr } ;
cmp_expr      = add_expr { ( "<" | ">" | "<=" | ">=" ) add_expr } ;
add_expr      = bitwise_expr { ( "+" | "-" | "+%" | "+?" ) bitwise_expr } ;
bitwise_expr  = unary_expr { ( "&" | "|" | "^" ) unary_expr } ;
unary_expr    = ( "!" | "~" ) unary_expr | primary ;
primary       = INT_LIT
              | HEX_LIT
              | "true"
              | "false"
              | call_expr
              | NAME
              | "(" expr ")"
              ;
call_expr     = NAME "(" [ arg_list ] ")" ;
arg_list      = expr { "," expr } ;
```

---

## Safety Invariants

The Nib compiler statically enforces the following invariants. A program that violates
any of these is rejected at compile time with a descriptive error message.

### 1. Call Depth ≤ 2

The Intel 4004 has a 3-level hardware call stack. Level 1 is always occupied by the
return address of the currently-executing function. This leaves 2 levels for nested
calls.

The compiler builds a **static call graph** — a directed graph where an edge from
function A to function B means "A calls B". It then computes the longest path from
any entry point (`main`). If this path has more than 2 edges, compilation fails.

Example (rejected):
```nib
fn main() { step1(); }
fn step1() { step2(); }
fn step2() { step3(); }   // depth = 3 — ERROR
fn step3() { }
```

### 2. Static RAM ≤ 160 Bytes

The total size of all `static` variable declarations must not exceed 160 bytes (320
nibbles). The compiler sums the sizes of all static variables:

- `u4` = 1 nibble
- `u8` = 2 nibbles
- `bcd` = 1 nibble
- `bool` = 1 nibble

If the total exceeds 320 nibbles, compilation fails.

### 3. No Recursion

A function may not call itself, directly or indirectly. The compiler checks the static
call graph for cycles. Any cycle (even indirect: A→B→A) is a compile error.

Recursion would require a software stack (to save local state across recursive calls),
which requires RAM that the 4004 does not have in useful quantity. And even if it did,
the 3-level hardware stack would overflow after 3 recursive calls.

### 4. No Heap

Nib has no dynamic memory allocation — no `malloc`, no `new`, no garbage collector.
All memory is either:
- **Stack-allocated**: `let` variables in function bodies (allocated in registers)
- **Static**: `static` variables in ROM-mapped data (allocated at link time)
- **Literals**: constant values embedded in ROM

This matches the 4004's memory model exactly.

### 5. Loop Bounds Must Be Const or Literal

For loops must have bounds that are known at compile time:

```nib
// Allowed — literal bounds:
for i: u4 in 0..8 { ... }

// Allowed — const bounds:
const COUNT: u4 = 8;
for i: u4 in 0..COUNT { ... }

// Rejected — variable bounds:
let n: u4 = get_input();
for i: u4 in 0..n { ... }  // ERROR: n is not const
```

This restriction exists because the 4004's DJNZ (decrement and jump) loop pattern
requires the trip count to be loaded into a register at loop entry. If the bound is
a variable, computing the trip count requires subtraction and potentially overflow
checks, which eat precious instructions and registers.

---

## Example Programs

### Example 1: BCD Digit Addition

```nib
// Add two BCD digits, producing a sum and a carry.
// Returns the sum digit in the low nibble, carry in the high nibble.
fn bcd_add(a: bcd, b: bcd) -> u8 {
    let sum: u4 = a + b;
    if sum >= 10 {
        // Carry out: sum wraps around 10 (like decimal addition)
        let adjusted: bcd = sum - 10;
        return adjusted | 0x10;  // high nibble = 1 (carry), low nibble = adjusted digit
    } else {
        return sum;  // no carry, high nibble = 0
    }
}
```

### Example 2: Nibble Extraction

```nib
// Extract the high nibble of an 8-bit value.
// Example: high_nibble(0xAB) = 0xA
fn high_nibble(x: u8) -> u4 {
    // Shift right by 4 positions using bitwise tricks.
    // Nib v1 has no shift operator, so we rotate via carry.
    // (In practice the compiler emits RAR x4 + mask sequence.)
    let hi: u4 = (x & 0xF0) >> 4;  // Note: >> reserved for v2; shown for clarity
    return hi;
}

// Extract the low nibble of an 8-bit value.
// Example: low_nibble(0xAB) = 0xB
fn low_nibble(x: u8) -> u4 {
    return x & 0x0F;
}
```

### Example 3: Counting Loop

```nib
// Sum the BCD digits 0..9 (should equal 45).
static total: u8 = 0;

fn sum_digits() {
    for i: bcd in 0..9 {
        total = total + i;
    }
}

fn main() {
    sum_digits();
}
```

### Example 4: Wrapping Counter

```nib
// A 4-bit wrapping counter that rolls over from 15 to 0.
// Models a 4004 counter register that cycles endlessly.
static counter: u4 = 0;

fn tick() {
    counter = counter +% 1;  // +% means: wrap at 16 (0..15, then 0 again)
}

fn main() {
    for cycle: u8 in 0..255 {
        tick();
    }
}
```

### Example 5: Saturating Accumulator

```nib
// Accumulate values but never exceed 9 (max BCD digit).
// Models a BCD accumulator with saturation rather than overflow.
static accumulator: u4 = 0;

fn add_clamped(delta: u4) {
    accumulator = accumulator +? delta;  // +? means: clamp at 15 (u4 max)
}
```

---

## What Is NOT in Nib v1

These features are intentionally absent in v1. They may appear in future versions or
companion libraries.

| Feature          | Reason for exclusion                                         |
|------------------|--------------------------------------------------------------|
| Multiplication   | 4004 has no MUL instruction; software mul is expensive       |
| Division         | 4004 has no DIV instruction; software div is expensive       |
| While loops      | For loops with const bounds are sufficient for v1            |
| Pointers/refs    | 4004's addressing model doesn't map well to pointers          |
| Arrays           | Would require dynamic addressing; reserved for v2            |
| Strings          | 4004 has no string support whatsoever                        |
| Floating-point   | 4004 has no FPU; software float is impractical               |
| Signed integers  | 4004 has no signed arithmetic instructions                   |
| Shift operators  | Available via DAA/rotate workaround; explicit in v2          |
| Struct/record    | No compound types; 4 primitive types are sufficient          |
| Imports/modules  | Single-file programs only in v1                              |
| Generics         | Not needed given the minimal type system                     |
| Error handling   | No exceptions; error codes via return values                 |

---

## Implementation Status

| Phase | Description                               | PR   | Status     |
|-------|-------------------------------------------|------|------------|
| 1     | Grammar files + spec documents            | PR 1 | This PR    |
| 2     | Lexer (Python)                            | PR 2 | Planned    |
| 3     | Parser (Python)                           | PR 3 | Planned    |
| 4     | Semantic checker + type checker (Python)  | PR 4 | Planned    |
| 5     | 4004 code generator (Python)              | PR 5 | Planned    |
| 6     | Assembler + Intel HEX packager (Python)   | PR 6 | Planned    |
| 7     | End-to-end tests + example programs       | PR 7 | Planned    |
| 8     | Multi-language ports (Go, Rust, TypeScript)| PR 8| Planned    |

---

## Version History

| Version | Date       | Description                  |
|---------|------------|------------------------------|
| 0.1.0   | 2026-04-12 | Initial specification draft  |
