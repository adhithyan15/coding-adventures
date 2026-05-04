# dartmouth-basic-ge225-compiler

The full **Dartmouth BASIC → GE-225** compiled pipeline in a single Rust crate.

In 1964, undergraduate students at Dartmouth College typed BASIC programs on
Teletype terminals.  The GE-225 time-sharing system compiled them on the fly,
ran the result on the simulator, and printed the output back on the same
terminal — often in under a second.  This crate recreates that experience.

## Pipeline

```
BASIC source text
    │
    ▼  [dartmouth-basic-ir-compiler]  (int_bits = 20)
  IrProgram
    │
    ▼  [ir-to-ge225-compiler]
  CompileResult (packed 20-bit binary image)
    │
    ▼  [coding-adventures-ge225-simulator]
  RunResult { output, var_values, steps, halt_address }
```

`int_bits = 20` is critical: the GE-225 stores signed integers in 20-bit
two's-complement words (−524 288 to 524 287).  The IR compiler uses this
setting to compute the power-of-ten constants for `PRINT`-number output —
passing the default 32-bit setting would emit `LOAD_IMM 1_000_000_000`,
silently wrapping in a 20-bit word and garbling the output.

## Quick start

```rust
use dartmouth_basic_ge225_compiler::run_basic;

let result = run_basic("10 LET A = 6 * 7\n20 END\n").unwrap();
assert_eq!(result.var_values["A"], 42);
assert_eq!(result.output, "");

let result = run_basic(
    "10 FOR I = 1 TO 5\n20 PRINT I\n30 NEXT I\n40 END\n"
).unwrap();
assert_eq!(result.output, "1\n2\n3\n4\n5\n");
```

## API

### `run_basic(source: &str) -> Result<RunResult, BasicError>`

Compile and run a BASIC program with the default options:
- 4 096-word GE-225 memory (the full historical machine)
- 100 000 instruction safety limit

### `run_basic_with_options(source, memory_words, max_steps) -> Result<RunResult, BasicError>`

Same as `run_basic` but with explicit control over memory size and the
maximum number of simulated instructions before the safety limit fires.

### `RunResult`

| Field | Type | Description |
|-------|------|-------------|
| `output` | `String` | Typewriter output from `PRINT` statements. GE-225 carriage-return codes (0o37) are converted to Unix `\n`. |
| `var_values` | `HashMap<String, i32>` | Final values of BASIC scalar variables A–Z, sign-extended from 20-bit two's complement to `i32`. |
| `steps` | `usize` | Number of GE-225 instructions executed. |
| `halt_address` | `usize` | Word address of the halt self-loop stub (`BRU halt_address`). |

### `BasicError`

A string-backed error returned when any pipeline stage fails:
- Parse error (malformed BASIC)
- IR compile error (unsupported feature in V1 — GOSUB, DIM, arrays, `^`)
- GE-225 codegen error
- Runtime error (divide by zero, max-steps exceeded)

## Supported BASIC subset (V1)

| Statement | Notes |
|-----------|-------|
| `LET var = expr` | Arithmetic: `+`, `-`, `*`, `/`, unary `-`, parentheses |
| `PRINT [expr | "string"] [, ...]` | Numbers with leading-zero suppression; strings |
| `FOR var = start TO limit [STEP n]` | Pre-test loop |
| `NEXT var` | Closes the nearest open FOR |
| `GOTO lineno` | Unconditional jump |
| `IF expr relop expr THEN lineno` | `<`, `>`, `=`, `<>`, `<=`, `>=` |
| `END` | Halts the program |

Not supported: GOSUB/RETURN, DIM, INPUT, arrays, string variables, `^` (power).

## Memory layout

```
word 0           : TON  (enable typewriter — emitted by the backend)
word 1 …         : compiled IR code
word code_end    : BRU code_end  (halt self-loop)
word data_base … : spill slots (one per virtual register)
word …           : constants table (unique LOAD_IMM values)
```

## GE-225 branch semantics note

The GE-225 uses **inhibit** semantics for its conditional skip instructions:
the named condition *prevents* the skip, not causes it.  For example:

- `BZE` is **inhibited by zero** — the BRU following it executes when A==0.
- `BNZ` is **inhibited by non-zero** — the BRU executes when A≠0.
- `BMI` is **inhibited by minus** — the BRU executes when A<0.
- `BOD` is **inhibited by odd** — the BRU executes when A is odd.

This is the opposite of the names' intuitive meaning and is a common source
of confusion when reading original GE-225 programs.

## Where this fits in the stack

```
dartmouth-basic-lexer
    │
dartmouth-basic-parser
    │
dartmouth-basic-ir-compiler   ──── compiler-ir (IrProgram / IrOp)
    │                                     │
    │                          ir-to-ge225-compiler
    │                                     │
    └──────────────────── dartmouth-basic-ge225-compiler (this crate)
                                          │
                          coding-adventures-ge225-simulator
```
