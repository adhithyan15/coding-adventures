# dartmouth-basic-ir-compiler

Lowers a Dartmouth BASIC AST into target-independent IR, ready for any backend
(GE-225, WASM, JVM, CIL).

## What it does

```
BASIC source
  → coding_adventures_dartmouth_basic_parser::parse_dartmouth_basic()
                              — lex + parse BASIC tokens
  → dartmouth_basic_ir_compiler::compile_basic_source()
                              — emit target-independent IrProgram
  → IrProgram                 — structured IR output
```

## Where it fits

`dartmouth-basic-ir-compiler` is the BASIC-specific frontend of the compilation
pipeline. It produces the same `IrProgram` type that all language frontends emit,
so every backend (GE-225, WASM, JVM, CIL) can consume it without modification.

| Package                        | Role                                 |
|-------------------------------|--------------------------------------|
| `dartmouth-basic-lexer`        | Tokenises BASIC source text          |
| `dartmouth-basic-parser`       | Parses tokens into a grammar AST     |
| **`dartmouth-basic-ir-compiler`** | **Lowers AST → IrProgram**        |
| `ir-to-ge225-compiler`         | IrProgram → GE-225 binary            |
| `ir-to-wasm-compiler`          | IrProgram → WASM binary              |
| `ir-to-jvm-class-file`         | IrProgram → JVM class file           |
| `ir-to-cil-bytecode`           | IrProgram → CLR CIL bytecode         |

## Quick start

```rust
use dartmouth_basic_ir_compiler::compile_basic_source;

let result = compile_basic_source("10 LET A = 5\n20 END\n").unwrap();
println!("Instructions: {}", result.program.instructions.len());
println!("Variable A is in register v{}", result.var_regs["A"]);  // → v1
```

## API

### Free functions

```rust
// Compile with default options (GE-225 encoding, 32-bit integers)
compile_basic_source(source: &str) -> Result<CompileResult, CompileError>

// Compile with explicit options
compile_basic_with_options(
    source: &str,
    char_encoding: CharEncoding,
    int_bits: u32,
) -> Result<CompileResult, CompileError>
```

### `CompileResult` fields

| Field      | Type                     | Description                              |
|------------|--------------------------|------------------------------------------|
| `program`  | `IrProgram`              | The compiled IR program                  |
| `var_regs` | `HashMap<String, usize>` | Maps BASIC variable names to register indices |

### `CharEncoding`

| Variant | Description                                      |
|---------|--------------------------------------------------|
| `Ge225` | GE-225 6-bit typewriter codes (default)          |
| `Ascii` | Standard ASCII byte values (for WASM/WASI)       |

### `CompileError`

| Field     | Type     | Description                    |
|-----------|----------|-------------------------------|
| `message` | `String` | Human-readable error description |

## Virtual register layout

```
v0        — syscall argument (char code for SYSCALL 1)
v1–v26    — BASIC scalar variables A–Z  (v1=A, v2=B, …, v26=Z)
v27–v286  — BASIC two-character variables A0–Z9
v287+     — expression temporaries (fresh per intermediate value)
```

## V1 supported statements

| Statement      | Example                    |
|----------------|----------------------------|
| `REM`          | `10 REM HELLO WORLD`       |
| `LET`          | `20 LET A = B + 3 * C`     |
| `PRINT`        | `30 PRINT "HELLO"`         |
| `GOTO`         | `40 GOTO 100`              |
| `IF … THEN`    | `50 IF A <= B THEN 200`    |
| `FOR … TO`     | `60 FOR I = 1 TO 10`       |
| `FOR … STEP`   | `70 FOR I = 0 TO 20 STEP 2`|
| `NEXT`         | `80 NEXT I`                |
| `END`          | `90 END`                   |
| `STOP`         | `90 STOP`                  |

### Arithmetic operators

`+`, `-`, `*`, `/`, unary `-`

All six relational operators: `<`, `>`, `=`, `<>`, `<=`, `>=`

### Not supported in V1

`GOSUB`, `RETURN`, `DIM`, `INPUT`, `DATA`, `READ`, `RESTORE`, `DEF FN`,
array element access `A(I)`, exponentiation `^`.

## `int_bits` parameter

Controls the number of digit positions unrolled for `PRINT` of numeric
expressions.  Must match the target backend's word width:

| `int_bits` | Max value   | Digit positions |
|------------|-------------|-----------------|
| 32 (default) | 2,147,483,647 | 10 |
| 20 (GE-225)  | 524,287       |  6 |

Passing `int_bits=32` for a GE-225 backend would cause the constant
1,000,000,000 to overflow a 20-bit register and produce garbled output.

## Tests

43 tests (36 unit + 7 doc-tests).  Run with:

```bash
cargo test -p dartmouth-basic-ir-compiler
```

## Dependencies

| Crate                                        | Role                                |
|----------------------------------------------|-------------------------------------|
| `coding-adventures-dartmouth-basic-parser`   | Dartmouth BASIC lexer + parser      |
| `compiler-ir`                                | `IrProgram`, `IrOp`, `IrOperand`    |
| `parser`                                     | `GrammarASTNode`, `ASTNodeOrToken`  |
| `lexer`                                      | `Token`, `effective_type_name()`    |
