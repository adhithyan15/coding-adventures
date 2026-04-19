# IR03 — Dartmouth BASIC → JVM Pipeline

## 1. Overview

This spec covers the three changes needed to run Dartmouth BASIC programs
on the JVM backend:

1. **Add `MUL` and `DIV` to `ir-to-jvm-class-file`** — the BASIC IR
   compiler emits both opcodes for arithmetic expressions; the JVM lowerer
   currently rejects them.

2. **Make the SYSCALL argument register configurable in `ir-to-jvm-class-file`** —
   the lowerer hard-codes register 4 as the I/O argument register (Brainfuck
   convention); BASIC IR uses register 0.

3. **Create `dartmouth-basic-jvm-compiler`** — a new integration package
   that chains the existing BASIC parser, IR compiler, JVM lowerer, and JVM
   simulator into a single `run_basic(source)` call, mirroring the
   `dartmouth-basic-wasm-compiler` API.

No changes to the BASIC parser, IR compiler, or JVM simulator are required.
The JVM lowerer already handles arbitrary control flow (`JUMP`, `BRANCH_Z`,
`BRANCH_NZ`) natively via JVM `goto` / `ifeq` / `ifne` bytecodes, so the
unstructured labels emitted by `GOTO` and `IF … THEN` work without a dispatch
loop.

---

## 2. Gap Analysis

### 2.1 What Dartmouth BASIC IR emits

The BASIC IR compiler (`dartmouth_basic_ir_compiler`) produces programs that
use these IR opcodes:

| IR Opcode | Source construct | JVM lowerer status |
|---|---|---|
| `LABEL` | Line numbers, FOR/IF labels | ✅ supported |
| `LOAD_IMM` | Literals, constants | ✅ supported |
| `LOAD_ADDR` | Data segment references | ✅ supported |
| `LOAD_BYTE` | PRINT char from memory | ✅ supported |
| `STORE_BYTE` | Memory writes | ✅ supported |
| `LOAD_WORD` | Numeric variable reads | ✅ supported |
| `STORE_WORD` | Numeric variable writes | ✅ supported |
| `ADD` | `LET A = B + C` | ✅ supported |
| `ADD_IMM` | `ADD_IMM v0, v2, 48` (digit offset) | ✅ supported |
| `SUB` | `LET A = B - C` | ✅ supported |
| `MUL` | `LET A = B * C` | ❌ **missing** |
| `DIV` | `LET A = B / C` | ❌ **missing** |
| `AND_IMM` | Internal masking | ✅ supported |
| `CMP_EQ` / `CMP_NE` / `CMP_LT` / `CMP_GT` | `IF A = B THEN` | ✅ supported |
| `JUMP` | `GOTO N` | ✅ supported |
| `BRANCH_Z` / `BRANCH_NZ` | `IF … THEN`, FOR condition | ✅ supported |
| `HALT` | `END`, `STOP` | ✅ supported |
| `SYSCALL 1` | `PRINT` (write byte) | ⚠️ wrong register (reads 4, needs 0) |

### 2.2 SYSCALL argument register mismatch

The JVM lowerer's `__ca_syscall` helper reads the byte to print from **local
variable slot 4** (register 4 — the Brainfuck convention):

```python
# ir-to-jvm-class-file/backend.py, _build_syscall_method
self._emit_reg_get(builder, 4)   # ← hardcoded; wrong for BASIC
```

The BASIC IR compiler assigns the print argument to **register 0**
(`_REG_SYSCALL_ARG = 0`). Running BASIC through the current JVM lowerer would
print null bytes instead of the intended characters.

The fix mirrors what was done for `ir-to-wasm-compiler` in v0.3.0: add a
`syscall_arg_reg: int` parameter (default `4` for backward compatibility with
Brainfuck) and thread it to the syscall helper.

### 2.3 MUL / DIV absence

`IrOp.MUL` and `IrOp.DIV` exist in `compiler_ir` but the JVM lowerer raises
`JvmBackendError("Unsupported IR opcode in prototype backend: MUL")` when it
encounters them. The JVM simulator already implements `IMUL` and `IDIV`
correctly. Only the lowerer needs updating.

---

## 3. Changes to `ir-to-jvm-class-file`

### 3.1 Add `IrOp.MUL` → `IMUL`

In the `_emit_instruction` dispatch (currently a `match` / `if-elif` chain in
`backend.py`), add a branch for `IrOp.MUL`:

```
case IrOp.MUL:
    # dst = lhs * rhs
    dst  = expect_register(operands[0])
    lhs  = expect_register(operands[1])
    rhs  = expect_register(operands[2])
    emit_reg_get(builder, lhs)
    emit_reg_get(builder, rhs)
    emit_opcode(builder, IMUL)           # JVM: pops two i32, pushes product
    emit_reg_set(builder, dst)
```

Semantics: signed 32-bit multiplication with silent two's-complement overflow,
matching Java's `int *` semantics and the BASIC spec (no overflow detection
required in V1).

### 3.2 Add `IrOp.DIV` → `IDIV`

```
case IrOp.DIV:
    # dst = lhs / rhs  (signed integer division, truncates toward zero)
    dst  = expect_register(operands[0])
    lhs  = expect_register(operands[1])
    rhs  = expect_register(operands[2])
    emit_reg_get(builder, lhs)
    emit_reg_get(builder, rhs)
    emit_opcode(builder, IDIV)           # JVM: pops two i32, pushes quotient
    emit_reg_set(builder, dst)
```

Semantics: Java `int /` — truncates toward zero, raises `ArithmeticException`
on divide-by-zero. In V1 Dartmouth BASIC we do not add a guard; division by
zero is a fatal program error (the JVM exception will propagate as a runtime
error in the simulator).

### 3.3 Add `syscall_arg_reg` parameter

**Public API change** — add `syscall_arg_reg: int = 4` keyword parameter to
`IrToJvmCompiler.compile()` (or the equivalent public entry point):

```python
def compile(
    self,
    program: IrProgram,
    function_signatures: list[FunctionSignature] | None = None,
    *,
    syscall_arg_reg: int = 4,          # ← new; default preserves Brainfuck
) -> bytes:                             # returns raw .class bytes
```

Thread `syscall_arg_reg` to the `_build_syscall_method` helper so the read
(`SYSCALL 1`) and write (`SYSCALL 2`) paths use `syscall_arg_reg` instead of
the literal `4`:

```python
# SYSCALL 1 (write)
self._emit_reg_get(builder, syscall_arg_reg)   # was: hardcoded 4
masked = byte & 0xFF
System.out.write(masked)

# SYSCALL 2 (read)
byte = System.in.read()
self._emit_reg_set(builder, syscall_arg_reg, byte)   # was: hardcoded 4
```

The default value `4` preserves backward compatibility with
`brainfuck-jvm-compiler` and `nib-jvm-compiler` which do not pass the
parameter.

**Version bump**: `ir-to-jvm-class-file` v0.X.0 → v0.(X+1).0.

---

## 4. New Package: `dartmouth-basic-jvm-compiler`

### 4.1 Location

```
code/packages/python/dartmouth-basic-jvm-compiler/
├── BUILD
├── CHANGELOG.md
├── README.md
├── pyproject.toml
└── src/
    └── dartmouth_basic_jvm_compiler/
        ├── __init__.py        # re-exports run_basic, RunResult, BasicError
        └── runner.py          # pipeline implementation
```

### 4.2 Pipeline

Five stages, exactly mirroring `dartmouth-basic-wasm-compiler`:

```
source (str)
  │
  ▼ Stage 1: parse
dartmouth_basic_parser.parse_dartmouth_basic(source)
  │ → BasicAST
  ▼ Stage 2: IR compile  (char_encoding="ascii")
dartmouth_basic_ir_compiler.compile_basic(ast, char_encoding="ascii")
  │ → IrResult (IrProgram)
  ▼ Stage 3: JVM lower
ir_to_jvm_class_file.IrToJvmCompiler().compile(
    ir_result.program,
    function_signatures=[FunctionSignature(label="_start", param_count=0)],
    syscall_arg_reg=0,                     # BASIC uses register 0
)
  │ → bytes (.class file)
  ▼ Stage 4: run
jvm_runtime.JvmRuntime().load_and_run(class_bytes, stdout=output_chunks.append)
  │ → stdout captured in output_chunks
  ▼ Stage 5: return
RunResult(output="".join(output_chunks))
```

**`char_encoding="ascii"`**: the BASIC IR compiler emits PRINT as raw ASCII
byte values (not GE-225 6-bit codes) when this flag is set, because the JVM
`System.out.write()` path expects standard bytes. Numeric digits are offset by
48 (`ord('0')`) before `SYSCALL 1`.

### 4.3 Public API

```python
@dataclass
class RunResult:
    output: str          # stdout produced by PRINT statements
    var_values: dict[str, int] = field(default_factory=dict)   # always {}
    steps: int = 0       # always 0 (JVM runtime has no step counter)
    halt_address: int = 0                                       # always 0


class BasicError(Exception):
    """Wraps any pipeline failure (parse, IR compile, JVM lower, runtime)."""


def run_basic(
    source: str,
    *,
    max_steps: int = 100_000,   # accepted for API parity; not enforced
) -> RunResult:
    ...
```

`run_basic()` raises `BasicError` on any pipeline failure. The original
exception is attached as `__cause__`.

### 4.4 Character encoding note

Same caveat as the WASM backend. The GE-225 typewriter used a proprietary
6-bit encoding. `char_encoding="ascii"` makes the IR compiler emit ASCII byte
values so JVM `System.out.write(byte)` produces correct output.

### 4.5 BUILD file

```python
python_package(
    name = "dartmouth-basic-jvm-compiler",
    deps = [
        "dartmouth-basic-parser",
        "dartmouth-basic-ir-compiler",
        "ir-to-jvm-class-file",
        "compiler-ir",
        "jvm-runtime",
    ],
)
```

Dependencies must be listed in leaf-to-root order.

---

## 5. Test Plan

### 5.1 File

`code/packages/python/dartmouth-basic-jvm-compiler/tests/test_dartmouth_basic_jvm_compiler.py`

### 5.2 Coverage target

≥ 95% line coverage. The test structure mirrors `test_dartmouth_basic_wasm_compiler.py`.

### 5.3 Test classes

| Class | What it covers |
|---|---|
| `TestLet` | Arithmetic: `+`, `-`, `*`, `/`; variable assignment and read-back |
| `TestPrintString` | String literals, spaces, newlines, REM ignored |
| `TestPrintNumeric` | Single-digit, multi-digit, negative numbers, zero |
| `TestForNext` | Ascending/descending step, nested loops, Gauss sum |
| `TestIfThen` | All six relational operators (`=`, `<>`, `<`, `>`, `<=`, `>=`) |
| `TestGoto` | Forward GOTO, backward GOTO (loop), STOP |
| `TestClassicPrograms` | Fibonacci, factorial, Collatz, countdown, multiplication table |
| `TestErrors` | Parse error, IR compile error, JVM lowering error, runtime error |
| `TestRunResult` | `var_values == {}`, `steps == 0`, `halt_address == 0` |

### 5.4 Error path tests

The four error stages require mocking to exercise without valid inputs:

```python
# Stage 1: parse error
with patch("dartmouth_basic_parser.parse_dartmouth_basic",
           side_effect=ValueError("bad")):
    with pytest.raises(BasicError):
        run_basic("garbage")

# Stage 2: IR compile error
with patch("dartmouth_basic_ir_compiler.compile_basic",
           side_effect=RuntimeError("ir fail")):
    with pytest.raises(BasicError):
        run_basic("10 END\n")

# Stage 3: JVM lowering error
with patch("ir_to_jvm_class_file.IrToJvmCompiler.compile",
           side_effect=RuntimeError("jvm fail")):
    with pytest.raises(BasicError):
        run_basic("10 END\n")

# Stage 4: runtime error
with patch("jvm_runtime.JvmRuntime.load_and_run",
           side_effect=RuntimeError("crash")):
    with pytest.raises(BasicError, match="runtime error"):
        run_basic("10 END\n")
```

---

## 6. Implementation Order

1. **`ir-to-jvm-class-file`** — add `MUL`, `DIV`, `syscall_arg_reg`; bump
   version; update CHANGELOG.
2. **`dartmouth-basic-jvm-compiler`** — create package; implement runner;
   write tests; write README and CHANGELOG.
3. **Commit and PR** — both packages go in the same PR; CI must pass for all
   affected packages (`ir-to-jvm-class-file`, `brainfuck-jvm-compiler`,
   `nib-jvm-compiler`, `dartmouth-basic-jvm-compiler`).

The Brainfuck and Nib JVM compilers are unaffected by the `syscall_arg_reg`
default of `4` — they do not pass the parameter, so they continue using the
existing behaviour.

---

## 7. Out of Scope (V1)

- **`IrOp.MOD`** (modulo) — not emitted by the BASIC IR compiler in V1.
- **Floating-point** — BASIC V1 uses integer arithmetic only.
- **GOSUB / RETURN** — not implemented in BASIC IR compiler V1.
- **Arrays / DIM** — not implemented in BASIC IR compiler V1.
- **INPUT** — `SYSCALL 2` (read) is not used by the BASIC IR compiler V1.
- **JVM native-image / GraalVM output** — the runner targets the JVM simulator
  only; native `.exe` generation is future work.
- **OR, XOR, SHL, SHR** — not needed by BASIC V1.
