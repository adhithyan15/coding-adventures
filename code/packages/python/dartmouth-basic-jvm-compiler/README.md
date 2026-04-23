# dartmouth-basic-jvm-compiler

Compiles Dartmouth BASIC programs to JVM bytecode and runs them through the
JVM simulator in a single `run_basic()` call.

## What it does

This package chains four independent packages into a single end-to-end pipeline:

```
BASIC source
    │
    ▼  dartmouth_basic_parser
   AST
    │
    ▼  dartmouth_basic_ir_compiler (char_encoding="ascii")
  IrProgram
    │
    ▼  ir_to_jvm_class_file (syscall_arg_reg=0)
  .class bytes
    │
    ▼  jvm_runtime.JVMRuntime
  stdout string
```

### Why four stages instead of five?

The WASM backend requires a separate encoder stage (`wasm_module_encoder`) to
package the WASM instructions into a binary module.  The JVM backend skips
this step: `ir_to_jvm_class_file` emits a complete `.class` file in one pass,
which the JVM simulator loads directly.

### Why `syscall_arg_reg=0`?

The `ir_to_jvm_class_file` lowerer was originally written for Brainfuck, which
places the SYSCALL print argument in register 4 (its only data cell).  The
Dartmouth BASIC IR compiler follows a different convention: it stores the
current PRINT value in register 0.  Passing `syscall_arg_reg=0` tells the JVM
lowerer which static field to read when emitting `System.out.write(byte)` calls.

### Why `char_encoding="ascii"`?

The original GE-225 computer used a proprietary 6-bit character code.  The JVM
backend writes raw bytes via `System.out.write(byte)`, which expects standard
ASCII.  The `char_encoding="ascii"` flag makes the IR compiler emit `ord()`
values for string literals and offset numeric digits by 48 (`ord('0')`).

## Installation

```bash
pip install coding-adventures-dartmouth-basic-jvm-compiler
```

## Usage

```python
from dartmouth_basic_jvm_compiler import run_basic, BasicError, RunResult

# Simple arithmetic
result = run_basic("10 LET A = 6 * 7\n20 PRINT A\n30 END\n")
print(result.output)  # "42\n"

# FOR/NEXT loop — Gauss sum
result = run_basic("""
10 LET S = 0
20 FOR I = 1 TO 100
30 LET S = S + I
40 NEXT I
50 PRINT S
60 END
""")
print(result.output)  # "5050\n"

# Error handling
try:
    run_basic("10 GOSUB 100\n20 END\n100 RETURN\n")
except BasicError as e:
    print(e)  # GOSUB is not supported in V1 of the compiled pipeline
```

## API

### `run_basic(source, *, max_steps=100_000) -> RunResult`

Compiles and runs a Dartmouth BASIC program.  `max_steps` is accepted for
interface parity with the GE-225 runner but is not enforced (the JVM simulator
has no step counter).

Raises `BasicError` if the program cannot be parsed, uses an unsupported
feature (GOSUB, DIM, INPUT, arrays, `^` operator), cannot be lowered to JVM
bytecode, or the JVM simulator raises an unexpected error.

### `RunResult`

| Field          | Type         | Value for JVM backend                      |
|----------------|--------------|--------------------------------------------|
| `output`       | `str`        | Program's standard output                  |
| `var_values`   | `dict`       | Always `{}` (JVM fields not exposed)       |
| `steps`        | `int`        | Always `0` (no instruction counter)        |
| `halt_address` | `int`        | Always `0` (halt is a method return)       |

### `BasicError`

Raised when the pipeline fails at any stage.  The original exception is
available as `__cause__`.

## Supported BASIC subset (V1)

| Statement          | Supported |
|--------------------|-----------|
| `LET var = expr`   | ✓         |
| `PRINT expr, ...`  | ✓         |
| `FOR`/`NEXT`/`STEP`| ✓         |
| `IF expr THEN line`| ✓ (all six relational operators) |
| `GOTO line`        | ✓         |
| `END` / `STOP`     | ✓         |
| `REM`              | ✓         |
| `GOSUB`/`RETURN`   | ✗ (raises BasicError) |
| `DIM` / arrays     | ✗ (raises BasicError) |
| `INPUT`            | ✗ (raises BasicError) |
| `^` (exponent)     | ✗ (raises BasicError) |

## Where this fits in the stack

```
code/packages/python/
├── dartmouth-basic-parser          ← stage 1: lex & parse
├── dartmouth-basic-ir-compiler     ← stage 2: AST → IR
├── ir-to-jvm-class-file            ← stage 3: IR → .class bytes
├── jvm-runtime                     ← stage 4: run .class in simulator
└── dartmouth-basic-jvm-compiler    ← this package: wires all four stages
```

The companion package `dartmouth-basic-wasm-compiler` does the same thing for
the WebAssembly backend; `dartmouth-basic-ge225-compiler` targets the original
GE-225 hardware simulator.
