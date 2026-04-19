# dartmouth-basic-ge225-compiler

**Dartmouth BASIC → GE-225 compiled pipeline**

This package ties together every stage of the Dartmouth BASIC ahead-of-time
compiler into a single `run_basic()` call.  It faithfully recreates the
experience of running BASIC programs on the 1964 Dartmouth time-sharing system:
programs are compiled to GE-225 20-bit machine words and executed on a
behavioural simulator of the same hardware.

```
BASIC source → lexer/parser → IR compiler → GE-225 backend → simulator
```

## Installation

```bash
pip install coding-adventures-dartmouth-basic-ge225-compiler
```

## Quick start

```python
from dartmouth_basic_ge225_compiler import run_basic

# Sum of 1 to 100 (Gauss)
result = run_basic("""
10 LET S = 0
20 FOR I = 1 TO 100
30 LET S = S + I
40 NEXT I
50 PRINT S
60 END
""")
print(result.output)          # "5050\n"
print(result.var_values["S"]) # 5050
print(result.steps)           # number of GE-225 instructions executed
```

## Supported BASIC subset (V1)

| Statement | Notes |
|-----------|-------|
| `LET v = expr` | Arithmetic: `+`, `-`, `*`, `/`; parentheses; 20-bit integers |
| `PRINT expr, …` | String literals and numeric expressions; trailing newline |
| `FOR v = a TO b [STEP s]` | Integer step; forward and backward |
| `NEXT v` | Closes the nearest enclosing `FOR` loop |
| `IF expr rel expr THEN lineno` | Relations: `<`, `>`, `=`, `<>`, `<=`, `>=` |
| `GOTO lineno` | Unconditional branch |
| `REM …` | Comment — ignored |
| `STOP` / `END` | Halt program |

### Not supported in V1

- `GOSUB` / `RETURN` (no call stack)
- `DIM`, arrays, string variables
- `INPUT`
- `^` (exponentiation)

## API reference

### `run_basic(source, *, memory_words=4096, max_steps=100_000) → RunResult`

Compile and run a Dartmouth BASIC program on the GE-225 simulator.

**Args**

- `source` — BASIC source text; lines must start with a line number.
- `memory_words` — total GE-225 memory in 20-bit words (default 4096).
- `max_steps` — safety limit on simulated instructions; raises `BasicError`
  if exceeded.

**Returns** a `RunResult`.

**Raises** `BasicError` for parse, compile, codegen, or runtime errors.

### `RunResult`

| Field | Type | Description |
|-------|------|-------------|
| `output` | `str` | Typewriter output from `PRINT` statements (`\n`-terminated lines) |
| `var_values` | `dict[str, int]` | Final values of all BASIC variables A–Z |
| `steps` | `int` | Number of GE-225 instructions executed |
| `halt_address` | `int` | Word address of the halt stub |

### `BasicError`

Raised when the program cannot be compiled or executed.  The original
exception is always attached as `__cause__`.

## Pipeline internals

The four packages that make up the pipeline are independent; each can be used
on its own:

| Stage | Package | Entry point |
|-------|---------|-------------|
| Parse | `coding-adventures-dartmouth-basic-parser` | `parse_dartmouth_basic(source)` |
| IR compile | `coding-adventures-dartmouth-basic-ir-compiler` | `compile_basic(ast)` |
| GE-225 backend | `coding-adventures-ir-to-ge225-compiler` | `compile_to_ge225(ir_program)` |
| Simulate | `coding-adventures-ge225-simulator` | `GE225Simulator` |

### Memory layout

```
addr 0           : TON  (enable typewriter — emitted by the backend)
addr 1 …         : compiled IR code
addr code_end    : BRU code_end  (halt self-loop)
addr data_base … : spill slots (one per virtual register / BASIC variable)
addr …           : constants table
```

### Numeric printing

PRINT of a numeric expression is compiled entirely to IR — no Python-level
integer-to-string conversion at runtime.  The pipeline uses repeated
divide-and-modulo by powers of 10 (100 000 → 10 000 → … → 1) to extract each
digit, then exploits the fact that the GE-225 typewriter codes for digits 0–9
equal the digit values themselves, so the raw digit can be loaded into the
SYSCALL argument register and dispatched directly.

## Examples

### Fibonacci sequence

```python
result = run_basic("""
10 LET A = 0
20 LET B = 1
30 FOR I = 1 TO 10
40 LET C = A + B
50 LET A = B
60 LET B = C
70 NEXT I
80 PRINT B
90 END
""")
print(result.output)  # "89\n"
```

### Countdown with GOTO

```python
result = run_basic("""
10 LET N = 5
20 PRINT N
30 LET N = N - 1
40 IF N > 0 THEN 20
50 END
""")
print(result.output)  # "5\n4\n3\n2\n1\n"
```

### Mixed PRINT

```python
result = run_basic("""
10 LET X = 6
20 LET Y = 7
30 PRINT "PRODUCT IS ", X * Y
40 END
""")
print(result.output)  # "PRODUCT IS 42\n"
```

## Historical context

In 1964, John Kemeny and Thomas Kurtz designed BASIC at Dartmouth College to run
on the GE-225 time-sharing system — one of the first interactive computing
environments available to students.  A student typed a program on a Teletype
terminal, pressed RETURN, and within seconds saw the output printed on the same
terminal.

This package recreates that experience end-to-end: BASIC source → GE-225
machine code → behavioural simulation → typewriter output.

## License

MIT
