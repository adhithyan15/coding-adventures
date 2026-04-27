# RV01 RISC-V Compiler Roadmap

## Current RISC-V path

The Go toolchain now has the core pieces for a direct RISC-V path:

```text
source language
  -> language frontend
  -> compiler-ir
  -> ir-to-riscv-compiler
  -> riscv-assembler
  -> riscv-simulator
```

`ir-to-riscv-compiler` emits both human-readable RV32I assembly and machine-code
bytes. `riscv-assembler` assembles the text form into the byte image loaded by
`riscv-simulator`.

## Language readiness

| Language | Current status | To run directly on RISC-V |
|----------|----------------|---------------------------|
| Nib | Go parser, type checker, IR compiler, and `nib-riscv-compiler` package exist. Simple `main` programs, nested calls, argument calls, and caller locals that survive calls lower to RISC-V assembly, assemble to bytes, and execute on `riscv-simulator`. | Extend static variable reads if needed; add broader ABI tests for multi-argument returns and values that outgrow the starter register map. |
| Brainfuck | Go parser, `brainfuck-ir-compiler`, and `brainfuck-riscv-compiler` package exist. Programs lower to RISC-V assembly, assemble to bytes, and execute on `riscv-simulator` with host byte I/O syscalls for `.` and `,`. | Add broader corpus tests, debug-mode bounds-check E2E tests, and richer host I/O controls if interactive execution is needed. |
| Dartmouth BASIC | Go lexer/parser exist. Python has `dartmouth-basic-ir-compiler`, GE-225, WASM, and JVM-oriented pipelines. | Port or bridge the Python IR compiler to Go `compiler-ir`; add `MUL`/`DIV` to Go `compiler-ir` and the RISC-V backend or lower them to helper loops; implement PRINT syscalls/trap host output in the RISC-V VM. |
| Oct | Python has lexer/parser/type-checker/IR compiler for the Intel 8008 pipeline. | Port or bridge Oct typed AST/IR into Go; add Go `compiler-ir` opcodes used by Oct but not yet present here (`OR`, `XOR`, `NOT`, and any target-specific syscall shapes); add RISC-V syscall host behavior for `in`, `out`, carry/parity/rotate intrinsics; add a real register allocator for values that survive across calls. |
| Starlark | Go compiler targets the Starlark VM bytecode, not `compiler-ir`. | Add a Starlark-to-`compiler-ir` lowerer or a direct RISC-V backend for its bytecode. |
| Mosaic | Go analyzer produces Mosaic-specific IR for UI/component emitters. | Add a Mosaic-to-`compiler-ir` lowering only if there is a meaningful native runtime target; otherwise it is not a natural RISC-V candidate. |
| Parser-only languages | Many languages currently have lexers/parsers only. | Add semantic analysis plus a `compiler-ir` lowering before the common RISC-V backend can be reused. |

## Backend gaps

- Calling convention: current IR-to-RISC-V emits a hidden call stack, saves
  `ra` for labels targeted by `CALL`, and preserves mapped caller virtual
  registers around calls while treating `v0` and `v1` as volatile starter-ABI
  registers. It still uses a starter fixed register mapping, so programs that
  outgrow that map need spill-slot allocation.
- Stack and frames: `ra` save/restore exists. Spilled virtual registers, local
  storage beyond the fixed physical-register map, and deeper runtime ABI support
  still need real frame layout work.
- Syscalls: `riscv-simulator` now has an opt-in host syscall layer for simple
  byte I/O and exit when no trap handler is installed. Richer language runtimes
  still need a standard ABI for strings, numbers, files, and interactive input.
- IR parity: Python-side languages already use extended IR ideas such as
  multiplication, division, OR, XOR, and NOT. The Go `compiler-ir` and RISC-V
  backend need those opcodes or canonical lowering passes.
- Runtime ABI: each frontend needs a documented result/argument convention. Nib
  currently returns through virtual `v1`, which maps to RISC-V `x6`.

## Suggested sequence

1. Add broader Nib and Brainfuck E2E corpus tests.
2. Add register allocation or spill slots for virtual registers beyond the
   starter physical-register map.
3. Port/bridge Dartmouth BASIC IR to Go and add `MUL`/`DIV`.
4. Port/bridge Oct IR to Go and add the remaining bitwise/syscall ABI support.
5. Standardize a richer RISC-V language runtime ABI for text and numeric I/O.
