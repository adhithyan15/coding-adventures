# cobol-iir-compiler

Lower a parsed **COBOL-60** program to the shared **IIR** (`interpreter_ir::IIRModule`)
so COBOL runs on every execution backend the LANG VM AOT chain targets —
Native-AOT, LLVM, WASM, JVM, CLR, and the generic register VM / JIT.

This is the COBOL sibling of [`flow-matic-iir-compiler`](../flow-matic-iir-compiler),
and step 4 of [PL09](../../specs/PL09-codegen.md). The tree-walking interpreter
[`cobol-runtime`](../cobol-runtime) is the **semantic oracle**: the compiled
program's output must be byte-identical to what the interpreter `DISPLAY`s.

## Where it sits in the stack

```
COBOL-60 source (carded, 80-column)
   │  cobol-lexer + cobol-parser   (cobol.grammar → CST)
   ▼
GrammarASTNode  (rule_name == "program")
   │  cobol-iir-compiler::compile_source     ← this crate
   ▼
interpreter_ir::IIRModule   (one `main`, returns i64 exit code)
   │  lang-aot  (Language::Cobol60)
   ▼
NativeAOT · LLVM · WASM · JVM · CLR · VM · JIT
```

## This slice (v0.1 — the `DISPLAY` / `MOVE` / `STOP RUN` core)

COBOL's WORKING-STORAGE is a **PICTURE-typed** data model. The first rung lowers
the three verbs that need no arithmetic, as **pure string I/O**:

| COBOL | IIR |
| --- | --- |
| elementary item `01 N PIC 9(5)` | one `str` register holding its stored PICTURE image |
| `VALUE <lit>` | the register's initial `str_const` (literal formatted into the picture at compile time) |
| `MOVE <lit> TO item` | re-`str_const` the receiver, the literal formatted into *its* picture |
| `DISPLAY op…` | each operand's image `print_str`'d in turn, then `putchar('\n')` |
| `STOP RUN` | `ret 0` |

### Why it is exact

A numeric item does not display as a plain integer: `PIC 9(5)` holding 42 shows
`00042`, and `PIC 9(2)V9` holding 123.456 shows `234` (truncated, implied point).
Because this rung has **no arithmetic**, every value a program stores is known at
compile time (a `VALUE` clause or a `MOVE` of a *literal*). So the compiler calls
the very same picture/value functions the oracle uses — `cobol-runtime`'s
`move_into_numeric` / `move_into_char`, re-exported for exactly this reuse — at
compile time, and emits the resulting digit string as a `str` constant. The
DISPLAYed bytes are byte-identical to the interpreter's by construction.

A numeric *literal*, by contrast, displays as its **source text** (`DISPLAY 42`
prints `42`, not `00042`) — COBOL only reshapes a value when it lands in a field.
The compiler honours that distinction.

### Deliberately a later rung

Each of these is a clean `CompileError::Unsupported` (never wrong output), landing
on its own PR: item-to-item `MOVE` (runtime picture reshaping), arithmetic
(`ADD`/`SUBTRACT`/`MULTIPLY`/`DIVIDE`/`COMPUTE`), `IF`, `PERFORM`, `GO TO`, group
items, and signed numerics (`PIC S9…`, trailing-overpunch display).

## Usage

```rust
use cobol_iir_compiler::compile_source;

let src = "\
000000 IDENTIFICATION DIVISION.
000000 PROGRAM-ID. HELLO.
000000 PROCEDURE DIVISION.
000000 MAIN.
000000     DISPLAY \"HELLO, WORLD\".
000000     STOP RUN.";
let module = compile_source(src, "hello").unwrap();
assert!(module.validate().is_empty());
// → run `module` on any backend; it prints "HELLO, WORLD\n".
```

## Testing

* `cargo test -p cobol-iir-compiler` — unit tests (compile shape + honest-failure
  errors), `tests/backend_compat.rs` (every AOT backend validator accepts the IIR),
  and `tests/jit_e2e.rs` (compiled-and-run output is byte-identical to the
  `cobol-runtime` oracle).
* `lang-aot/tests/lang_matrix.rs` carries the COBOL rows proven across the backend
  columns.
