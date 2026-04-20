# TET00 — Tetrad Language Specification

## Overview

Tetrad is a small, statically-scoped, interpreted programming language whose bytecode
can be executed by a register-based virtual machine small enough to run on an Intel 4004
microprocessor. The name is a pun on "tetrad" — the precise technical term for a 4-bit
group (a nibble), the native data size of the 4004.

Tetrad is the **interpreted counterpart** to Nib (spec NIB00), which compiled directly
to 4004 machine code. The key difference in approach:

```
Nib:    source → compiler → 4004 machine code → 4004 runs the program
Tetrad: source → compiler → bytecode          → 4004 runs the interpreter
                                                  interpreter executes bytecode
```

The indirection of an interpreter buys significant expressive power:

| Capability | Nib (compiled) | Tetrad (interpreted) |
|---|---|---|
| Variables with dynamic names | Not possible | Resolved by interpreter |
| While loops with variable bounds | Not possible | Interpreter handles jump |
| Software call stack > 3 deep | Not possible | Interpreter manages RAM stack |
| Arithmetic (mul, div) | Not supported v1 | Interpreter runs software loop |
| Extensible without recompiling | No | Add opcodes to interpreter |

The 4004 pays a ~30× speed cost — running ~20,000–37,000 interpreted instructions per
second versus ~740,000 native instructions per second — but gets a language that feels
like modern procedural code.

---

## Design Goals

1. **Interpretable on Intel 4004.** The VM dispatch loop must fit within 4 KB of ROM
   alongside the bytecode program. VM state must fit within 128 bytes of usable RAM.

2. **Accumulator-centric register VM.** Following V8 Ignition: one implicit accumulator
   register, 8 explicit named registers R0–R7. Most instructions read from / write to
   the accumulator; the register operand is always named explicitly.

3. **Feedback vectors from day one.** Every binary operation and every call site carries
   a feedback slot index. The VM records observed types and call shapes into these slots.
   This is the substrate that makes adding a JIT compiler straightforward.

4. **Broadens to Lisp.** The bytecode instruction set and VM metrics API are designed so
   that a Lisp front-end can compile to the same bytecode. Where Tetrad's types are
   statically monomorphic (all values are u8), a Lisp front-end will generate polymorphic
   feedback — and the JIT can specialize on it.

5. **Literate, learnable source.** Tetrad programs should be readable by someone who
   knows any procedural language. Syntax is deliberately minimal.

---

## Intel 4004 Interpreter Model

The physical hardware model this spec targets:

```
4004 ROM (4 KB)
  ┌──────────────────────────────────────┐
  │  Tetrad Interpreter (4004 assembly)  │  ~2 KB
  │  ──────────────────────────────────  │
  │  Tetrad Program (bytecode)           │  ~2 KB
  └──────────────────────────────────────┘

4004 RAM (128 bytes usable general RAM)
  ┌──────────────────────────────────────┐
  │  Accumulator          1 byte         │
  │  Registers R0–R7      8 bytes        │
  │  Program counter      2 bytes        │
  │  Frame pointer        1 byte         │
  │  Call stack           32 bytes       │  4 frames × 8 bytes/frame
  │  Variable pool        84 bytes       │  up to 84 named u8 variables
  └──────────────────────────────────────┘
```

### Call Stack Frame Layout (8 bytes each, 4 frames max)

```
Byte 0–1  Return instruction pointer (12-bit, zero-padded to 16 bits)
Byte 2    Saved frame pointer (variable pool base index)
Byte 3–3  Register save: R0 (so callee can clobber R0 for its own use)
Byte 4–7  Padding / future use (keeps frame power-of-2 aligned)
```

With 4 call frames (interpreter occupies 1 hardware stack level), the maximum
user-visible call depth is 4. This is enforced by the VM at runtime.

### Why RAM Fits

```
Accumulator:   1 byte
Registers:     8 bytes (R0–R7, each 1 byte = 8-bit value)
PC:            2 bytes
Frame pointer: 1 byte
Call stack:    32 bytes (4 frames × 8 bytes)
──────────────────────
Fixed overhead: 44 bytes

Available for variables: 128 − 44 = 84 bytes → 84 distinct u8 variables
```

84 variables is enough for embedded control programs. A Busicom-style calculator needs
fewer than 20.

---

## Type System

Tetrad v1 has a single type: **u8** — an unsigned 8-bit integer (0–255).

This is the only type that makes practical sense given the 4004's architecture:
- Stored in one 4004 register pair (two 4-bit nibbles = one 8-bit byte)
- Wraps on overflow (modular arithmetic)
- No sign bit, no fractions, no pointers

Arithmetic wraps at 256. There are no overflow errors — this matches the 4004's hardware
behavior for 8-bit register pairs.

### Why Only u8?

The feedback vector machinery (spec TET03) is present in v1 even though all values are
u8 and feedback is always monomorphic. This is intentional: when a Lisp front-end later
compiles to the same bytecode, it will introduce dynamically-typed values (integers,
pairs, symbols, closures). The JIT will read those feedback slots and specialize. By
wiring the slots in from the start, the interpreter does not need to change when the
Lisp compiler arrives.

---

## Operators

### Arithmetic

| Operator | Description | Wraps on overflow |
|---|---|---|
| `+` | Addition | Yes |
| `-` | Subtraction | Yes (wraps below 0) |
| `*` | Multiplication | Yes |
| `/` | Integer division | Halts if divisor is 0 |
| `%` | Remainder | Halts if divisor is 0 |

### Bitwise

| Operator | Description |
|---|---|
| `&` | Bitwise AND |
| `\|` | Bitwise OR |
| `^` | Bitwise XOR |
| `~` | Bitwise NOT (unary) |
| `<<` | Left shift |
| `>>` | Right shift (logical, zero fill) |

### Comparison (result is 1 if true, 0 if false)

| Operator | Description |
|---|---|
| `==` | Equal |
| `!=` | Not equal |
| `<` | Less than |
| `<=` | Less than or equal |
| `>` | Greater than |
| `>=` | Greater than or equal |

### Logical

| Operator | Description |
|---|---|
| `&&` | Logical AND (short-circuit) |
| `\|\|` | Logical OR (short-circuit) |
| `!` | Logical NOT (unary) |

Logical operators treat 0 as false, any non-zero value as true. Result is 1 or 0.

### Operator Precedence (lowest to highest)

| Level | Operators | Associativity |
|---|---|---|
| 1 | `\|\|` | Left |
| 2 | `&&` | Left |
| 3 | `==`, `!=` | Left |
| 4 | `<`, `>`, `<=`, `>=` | Left |
| 5 | `\|` | Left |
| 6 | `^` | Left |
| 7 | `&` | Left |
| 8 | `<<`, `>>` | Left |
| 9 | `+`, `-` | Left |
| 10 | `*`, `/`, `%` | Left |
| 11 | `!`, `~`, unary `-` | Right (prefix) |
| 12 | primary | — |

---

## Grammar Reference

Full grammar in EBNF notation:

```ebnf
program       = { top_decl } EOF ;

top_decl      = fn_decl | global_decl ;

global_decl   = "let" NAME "=" expr ";" ;

fn_decl       = "fn" NAME "(" [ param_list ] ")" block ;
param_list    = NAME { "," NAME } ;

block         = "{" { stmt } "}" ;

stmt          = let_stmt
              | assign_stmt
              | if_stmt
              | while_stmt
              | return_stmt
              | expr_stmt
              ;

let_stmt      = "let" NAME "=" expr ";" ;
assign_stmt   = NAME "=" expr ";" ;
if_stmt       = "if" expr block [ "else" block ] ;
while_stmt    = "while" expr block ;
return_stmt   = "return" [ expr ] ";" ;
expr_stmt     = expr ";" ;

expr          = or_expr ;

or_expr       = and_expr { "||" and_expr } ;
and_expr      = bitor_expr { "&&" bitor_expr } ;
bitor_expr    = bitxor_expr { "|" bitxor_expr } ;
bitxor_expr   = bitand_expr { "^" bitand_expr } ;
bitand_expr   = eq_expr { "&" eq_expr } ;
eq_expr       = cmp_expr { ( "==" | "!=" ) cmp_expr } ;
cmp_expr      = shift_expr { ( "<" | ">" | "<=" | ">=" ) shift_expr } ;
shift_expr    = add_expr { ( "<<" | ">>" ) add_expr } ;
add_expr      = mul_expr { ( "+" | "-" ) mul_expr } ;
mul_expr      = unary_expr { ( "*" | "/" | "%" ) unary_expr } ;
unary_expr    = ( "!" | "~" | "-" ) unary_expr
              | primary
              ;
primary       = INT_LIT
              | HEX_LIT
              | NAME
              | call_expr
              | "in" "(" ")"
              | "(" expr ")"
              ;
call_expr     = NAME "(" [ arg_list ] ")" ;
arg_list      = expr { "," expr } ;
```

### Notes on the Grammar

- There is no `for` statement in v1. Use `while` with a manual counter.
- `in()` is a keyword-expression that reads one byte from the I/O port.
- `out(expr)` is a statement-level call to the I/O output port.
- All variables are `u8`. There are no type annotations.
- `let` in a block creates a local variable scoped to that block.
- `let` at the top level creates a global variable.

---

## Reserved Words

```
fn  let  if  else  while  return  in  out
```

---

## Examples

### Example 1: Counting Down

```tetrad
fn count_down(n) {
    while n > 0 {
        out(n);
        n = n - 1;
    }
}

fn main() {
    count_down(10);
}
```

### Example 2: Bitwise Nibble Extraction

```tetrad
fn high_nibble(x) {
    return (x >> 4) & 15;
}

fn low_nibble(x) {
    return x & 15;
}

fn main() {
    let val = 171;   // 0xAB
    out(high_nibble(val));   // outputs 10 (0xA)
    out(low_nibble(val));    // outputs 11 (0xB)
}
```

### Example 3: BCD Digit Sum (replicating Busicom logic)

```tetrad
fn bcd_add(a, b) {
    let sum = a + b;
    if sum >= 10 {
        sum = sum - 10;
        out(1);      // carry signal
    } else {
        out(0);      // no carry
    }
    return sum;
}

fn main() {
    let result = bcd_add(7, 5);   // 7 + 5 = 12 → result=2, carry=1
    out(result);
}
```

### Example 4: Software Multiply (no hardware MUL on 4004)

```tetrad
fn multiply(a, b) {
    let result = 0;
    while b > 0 {
        result = result + a;
        b = b - 1;
    }
    return result;
}

fn main() {
    out(multiply(6, 7));   // outputs 42
}
```

### Example 5: Reading from I/O

```tetrad
fn echo_loop() {
    let running = 1;
    while running {
        let byte = in();
        if byte == 0 {
            running = 0;
        } else {
            out(byte);
        }
    }
}

fn main() {
    echo_loop();
}
```

---

## Relationship to Nib

Tetrad is a sibling to Nib, not a replacement. They coexist in the stack:

| Aspect | Nib | Tetrad |
|---|---|---|
| Execution model | Compiled → 4004 machine code | Interpreted via bytecode VM |
| Speed | Fast (~740 kHz native) | Slow (~20–37 kHz interpreted) |
| Language expressiveness | Limited by 4004 ISA | Limited only by VM RAM budget |
| Call depth | 2 (hardware 3-level stack) | 4 (software stack in RAM) |
| While with variable bounds | Not supported | Supported |
| Multiplication, division | Not supported v1 | Supported (software loop) |
| Type system | u4, u8, bcd, bool | u8 only |
| Feedback for JIT | Not applicable | Built-in feedback vector |
| Future Lisp support | Not planned | Yes — bytecode broadens to Lisp |

Use Nib when you need raw speed and know the loop bounds at compile time.
Use Tetrad when you need a more expressive language and can afford the interpreter overhead.

---

## Relationship to OCT

OCT (spec OCT00) targets the Intel 8008. Tetrad targets the 4004. The 4004 is
significantly more constrained than the 8008:

| | Intel 4004 | Intel 8008 |
|---|---|---|
| Year | 1971 | 1972 |
| Data width | 4-bit | 8-bit |
| Registers | 16 × 4-bit | 7 × 8-bit |
| Usable RAM | 128 bytes | 16 KB |
| ROM | 4 KB | 16 KB |
| Hardware stack | 3 levels | 8 levels |

OCT programs would not fit on a 4004 without significant redesign. Tetrad is
intentionally designed around the more severe 4004 constraints.

---

## What Is NOT in Tetrad v1

| Feature | Reason for exclusion |
|---|---|
| Strings | No RAM budget; no string instructions on 4004 |
| Floating-point | No FPU; software float not practical in 128 bytes |
| Arrays | Dynamic indexing requires addressing not available in 128-byte VM state |
| Closures | Capture would exceed per-frame RAM budget |
| Recursion | Allowed but limited to 4 deep by software stack; no TCO in v1 |
| Modules / imports | Single-file programs only in v1 |
| Multiple types | v1 is u8-only; Lisp front-end will introduce dynamic types |
| Structs / records | No compound types in v1 |

---

## Implementation Pipeline

The full Tetrad pipeline across five packages:

```
Tetrad Source Text
    │
    ▼
tetrad-lexer      TET01   Produces token stream
    │
    ▼
tetrad-parser     TET02   Produces AST (Pratt parser)
    │
    ▼
tetrad-compiler   TET03   Produces bytecode (CodeObject with feedback slots)
    │
    ▼
tetrad-vm         TET04   Executes bytecode, populates feedback vectors, exposes metrics API
    │
    ▼
tetrad-jit        TET05   Reads feedback vectors, emits x86-64 native code for hot functions
```

Each stage is a standalone Python package with its own `pyproject.toml`, tests,
README, and CHANGELOG. The implementation language is Python 3.12 throughout.

---

## Implementation Status

| Phase | Package | Spec | Status |
|---|---|---|---|
| 1 | Language spec | TET00 | This document |
| 2 | Lexer | tetrad-lexer | TET01 — Planned |
| 3 | Parser | tetrad-parser | TET02 — Planned |
| 4 | Bytecode compiler | tetrad-compiler | TET03 — Planned |
| 5 | Register VM | tetrad-vm | TET04 — Planned |
| 6 | JIT compiler | tetrad-jit | TET05 — Planned |

---

## Version History

| Version | Date | Description |
|---|---|---|
| 0.1.0 | 2026-04-20 | Initial specification draft |
