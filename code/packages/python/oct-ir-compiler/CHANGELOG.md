# Changelog

All notable changes to `coding-adventures-oct-ir-compiler` will be documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [Unreleased]

### Added

- **`OctCompileConfig`** — frozen dataclass with two optional fields:
  - `write_byte_syscall: int | None` — SYSCALL number for `out()` when
    targeting a cross-platform backend.  `None` (default) keeps the Intel 8008
    port-based encoding (`SYSCALL 40+PORT`).
  - `read_byte_syscall: int | None` — SYSCALL number for `in()` when targeting
    a cross-platform backend.  `None` (default) keeps `SYSCALL 20+PORT`.

- **Pre-defined I/O configs** (all exported from `oct_ir_compiler`):
  - `INTEL_8008_IO` — default; `None` values for both fields → port encoding.
  - `WASM_IO` — `write=1` (WASI `fd_write`) / `read=2` (WASI `fd_read`).
  - `JVM_IO` — `write=1` (`System.out.write`) / `read=4` (`System.in.read`).
  - `CLR_IO` — `write=1` (`Console.Write`) / `read=2` (`Console.Read`).

### Changed

- **`compile_oct(typed_ast, config=INTEL_8008_IO)`** — updated signature adds
  an optional `config: OctCompileConfig` argument.  Old callers that pass only
  `typed_ast` continue to work unchanged (default = `INTEL_8008_IO`, same
  behaviour as before).
- When `config.write_byte_syscall` is not `None`, `out(PORT, val)` emits
  `SYSCALL [write_byte_syscall, v2]` (two-operand form with explicit arg
  register; PORT is ignored — cross-platform backends have a single output
  channel).
- When `config.read_byte_syscall` is not `None`, `in(PORT)` emits
  `SYSCALL [read_byte_syscall, v1]` (scratch register as explicit arg reg;
  PORT is ignored).
- 22 new tests in `TestOctCompileConfig` (added to `test_oct_ir_compiler.py`):
  - Default 8008 port encoding verified for `in()` and `out()`.
  - One-operand SYSCALL form (no arg-register operand) for 8008 target.
  - WASM_IO / JVM_IO / CLR_IO SYSCALL number and arg-register assertions.
  - PORT-independence of cross-platform targets (`out(3, x)` still → SYSCALL 1
    under `WASM_IO`, not SYSCALL 43).
  - Custom config with arbitrary SYSCALL numbers.
  - Predefined config field values checked directly.

### Motivation

Oct's `in()` and `out()` intrinsics use Intel 8008 hardware port numbers
(`SYSCALL 20+PORT` / `SYSCALL 40+PORT`).  Those numbers are meaningless on
WASM, JVM, and CLR — each expects completely different SYSCALL ABIs.
`OctCompileConfig` lets the Oct IR compiler emit the correct SYSCALL numbers
for any target, eliminating the ABI mismatch at compile time rather than at
the backend's pre-flight validator or (worse) at runtime.

## [0.1.0] — 2026-04-20

### Added

- **`compile_oct(typed_ast: ASTNode) -> OctCompileResult`** — module-level entry
  point.  Accepts a type-annotated Oct AST (produced by `oct-type-checker`) and
  returns an `OctCompileResult` wrapping the compiled `IrProgram`.

- **`OctCompileResult`** — dataclass with a single field:
  - `program: IrProgram` — the compiled IR ready for the backend validator.

- **Three-phase compilation**:
  - **Phase 1 (static data)**: collects all `static` declarations into
    `IrDataDecl` entries (1 byte each, `u8`-typed).
  - **Phase 2 (entry point)**: emits `LABEL _start`, `LOAD_IMM v0, 0`
    (constant-zero register), `CALL _fn_main`, `HALT`.
  - **Phase 3 (function bodies)**: lowers each `fn` declaration into IR
    instructions preceded by `LABEL _fn_NAME` and followed by `RET`.

- **Virtual register layout**:
  - `v0` = constant zero (preloaded at `_start`)
  - `v1` = scratch / expression temporary / return value
  - `v2+` = named locals and parameters (allocated in declaration order)

- **Calling convention** — arguments in `v2, v3, v4, v5`; return value in `v1`.

- **Full statement coverage**:
  - `let` declarations → `LOAD_IMM` + `ADD_IMM` copy into dedicated register
  - `assign_stmt` — local: `ADD_IMM` copy; static: `LOAD_ADDR` + `STORE_BYTE`
  - `return` → move to `v1` if needed, then `RET`
  - `if`/`else` → `BRANCH_Z`, `JUMP`, unique `if_N_else` / `if_N_end` labels
  - `while` → top-checking loop: `BRANCH_Z` exit + `JUMP` back
  - `loop` → unconditional `JUMP` back + `loop_N_end` label for `break`
  - `break` → `JUMP loop_N_end` (targets innermost enclosing loop via stack)
  - `expr_stmt` → compiled for side effects (used for bare `out(...)` calls)

- **Full expression coverage**:
  - Integer/hex/binary literals → `LOAD_IMM`
  - Boolean literals `true`/`false` → `LOAD_IMM 1` / `LOAD_IMM 0`
  - Local variable reads → return the variable's dedicated register (no emit)
  - Static variable reads → `LOAD_ADDR` + `LOAD_BYTE`
  - Arithmetic: `+` → `ADD`, `-` → `SUB`
  - Bitwise: `&` → `AND`, `|` → `OR`, `^` → `XOR`, `~` → `NOT`
  - Comparison: `==` → `CMP_EQ`, `!=` → `CMP_NE`, `<` → `CMP_LT`, `>` → `CMP_GT`
  - LE/GE via operand swap: `<=` → `CMP_GT(b,a)`, `>=` → `CMP_LT(b,a)`
  - Logical NOT: `!a` → `CMP_EQ(a, v0)`
  - Logical AND: `&&` → `AND` (bool ×bool)
  - Logical OR: `||` → `ADD` + `CMP_NE(result, v0)`
  - User function calls → argument staging + `CALL _fn_NAME` + restore
  - Hardware intrinsic calls → SYSCALL (see below)

- **All 10 hardware intrinsics mapped to SYSCALL**:
  - `in(PORT)` → `SYSCALL 20+PORT`
  - `out(PORT, val)` → stage `val` in `v2`, then `SYSCALL 40+PORT`
  - `adc(a, b)` → stage `a`→`v2`, `b`→`v3`, `SYSCALL 3`
  - `sbb(a, b)` → same pattern, `SYSCALL 4`
  - `rlc(a)` → stage `a`→`v2`, `SYSCALL 11`
  - `rrc(a)` → `SYSCALL 12`
  - `ral(a)` → `SYSCALL 13`
  - `rar(a)` → `SYSCALL 14`
  - `carry()` → `SYSCALL 15` (no args)
  - `parity(a)` → stage `a`→`v2`, `SYSCALL 16`

- **Live-register save/restore around function calls**: before staging arguments
  in `v2+`, live caller locals are copied to fresh temporaries above the current
  register high-water mark.  After `CALL`, they are restored.  This preserves
  caller state when a function with live locals calls another function.

- **`IrDataDecl` for statics**: size always 1 (Oct statics are single bytes),
  initial value extracted from the literal initialiser.

- **Comprehensive test suite** (`tests/test_oct_ir_compiler.py`):
  - 20 test classes, ~70 individual test cases
  - Full pipeline tests (parse → type-check → compile)
  - Opcode-level assertions for every language construct
  - All five OCT00 spec example programs validated
  - Label naming, register allocation, and SYSCALL number checks

### Design Notes

- **OR / XOR / NOT opcodes**: these were added to `compiler-ir` as Phase 0 of
  the Oct implementation.  The Nib compiler emulated XOR via SUB (since Nib
  targets the Intel 4004 which has no XOR); the 8008 has native XOR, XRA, and
  CMA, so the new `IrOp.OR`, `IrOp.XOR`, `IrOp.NOT` opcodes map 1-to-1.

- **PORT baking**: `in(PORT)` and `out(PORT, val)` encode the port number in
  the SYSCALL instruction number (`20+PORT` and `40+PORT`).  This matches
  the Intel 8008's `INP` / `OUT` instructions, which encode the port as a
  3-bit field inside the opcode byte.  No runtime dispatch is needed.

- **Compiler is a pure AST walker**: it holds no mutable shared state between
  functions other than the `_statics` set and the ID/label counters.  Each
  function compiles independently with a fresh register map.

- **`loop_end_stack`**: implemented as a Python list used as a stack.  The
  top element is always the end-label of the innermost enclosing loop/while.
  `break` pops nothing — it just peeks (the `_compile_loop_stmt` /
  `_compile_while_stmt` methods push and pop).
