# dartmouth-basic-ir-compiler

Dartmouth BASIC 1964 frontend: lowers a parsed BASIC AST to target-independent IR.

## Overview

This package is the compiler frontend in the Dartmouth BASIC compiled pipeline:

```
BASIC source
    ↓  dartmouth-basic-lexer     (tokenize)
    ↓  dartmouth-basic-parser    (parse to AST)
    ↓  dartmouth-basic-ir-compiler   (this package — lower to IR)
    ↓  ir-to-ge225-compiler         (emit GE-225 machine words)
    ↓  ge225-simulator               (execute)
```

It accepts an `ASTNode` from `dartmouth_basic_parser` and emits a
`compiler_ir.IrProgram` that any backend (GE-225, WASM, JVM) can compile to
native code.

## Usage

```python
from dartmouth_basic_parser import parse_dartmouth_basic
from dartmouth_basic_ir_compiler import compile_basic

source = "10 FOR I = 1 TO 5\n20 PRINT \"HELLO\"\n30 NEXT I\n40 END\n"
ast = parse_dartmouth_basic(source)
result = compile_basic(ast)

# result.program: IrProgram ready for any backend
# result.var_regs["I"]: virtual register index for BASIC variable I
```

## V1 Supported Statements

| Statement | Notes |
|-----------|-------|
| `REM` | No-op |
| `LET var = expr` | Scalar variables A–Z only; all arithmetic operators |
| `PRINT "string"` | String literals only; auto-uppercased |
| `GOTO lineno` | Unconditional jump |
| `IF expr relop expr THEN lineno` | All six relational operators |
| `FOR var = expr TO expr [STEP expr]` | Positive step; pre-test loop |
| `NEXT var` | Closes innermost FOR |
| `END` / `STOP` | Halt |

## Historical Note

Dartmouth BASIC was designed in 1964 by John Kemeny and Thomas Kurtz to run on
the GE-225 mainframe at Dartmouth College. It was the world's first language
designed for time-sharing: students typed programs on Teletype terminals and
received results in seconds. This package is a faithful compiled implementation
of that original language, targeting the same GE-225 hardware architecture.
