# Multi-architecture backend plan

**Date:** 2026-06-01.  Addendum to
`MULTILANG-BACKEND-PLAN.md` covering native architectures beyond
x86_64 / aarch64 (the two already in the AOT chain).

## 1. Simulators already in the workspace

We can validate machine-code emission for many architectures without
real hardware by running it on the in-tree simulators.

| Simulator crate          | What it executes                                 | Status |
|--------------------------|--------------------------------------------------|--------|
| `arm-simulator`          | ARMv7 subset ("the architecture in your phone")  | Available |
| `arm1-simulator`         | ARM1 (ARMv1) behavioral ISA                      | Available |
| `riscv-simulator`        | RV32I + M-mode privileged extensions             | Available |
| `intel8008-simulator`    | First 8-bit x86 ancestor — Oct's native target!  | Available |
| `intel4004-simulator`    | World's first commercial microprocessor          | Available |
| `ge225-simulator`        | The original 1964 Dartmouth BASIC machine        | Available |
| `cpu-simulator`          | Generic 8-bit teaching ISA                       | Available |
| `arm1-gatelevel`         | Gate-level ARM1                                  | Available |
| `intel4004-gatelevel`    | Gate-level 4004                                  | Available |
| `clr-simulator`          | CIL bytecode (already used by iir-to-cil)        | Hooked up |
| `jvm-simulator`          | JVM bytecode (already used by iir-to-jvm)        | Hooked up |
| `wasm-simulator`         | wasm bytecode (already used by iir-to-wasm)      | Hooked up |
| `wasm-simulator`         | wasm bytecode (used downstream by Brainfuck)     | Hooked up |

We **already have** assemblers for two of these:
- `intel-4004-assembler`
- `intel-8008-assembler`

…but no `iir-to-*` chain for any of the bare-metal CPUs.

## 2. Backend plan, prioritised

The architecture targets to wire up, in order of strategic value:

### A1: RISC-V (RV32I)

**Why first.** RISC-V is the most strategically valuable: open ISA,
modern, widely deployed, RV32I is a small enough instruction set
that lowering from IIR is tractable, and the in-tree
`riscv-simulator` gives us cheap end-to-end execution.

**Mapping sketch** (full table in the implementation PR):

| IIR opcode     | RV32I instruction(s)                     |
|----------------|------------------------------------------|
| `const i32`    | `lui rd, hi20; addi rd, rd, lo12`        |
| `add`          | `add rd, rs1, rs2`                       |
| `sub`          | `sub rd, rs1, rs2`                       |
| `mul`          | not in I — needs M extension (defer)     |
| `cmp_lt`       | `slt rd, rs1, rs2`                       |
| `cmp_eq`       | `xor t0, rs1, rs2; sltiu rd, t0, 1`      |
| `jmp label`    | `jal x0, label`                          |
| `jmp_if_false` | `beq rs, x0, label`                      |
| `ret`          | `addi a0, rs, 0; jalr x0, x1, 0`         |
| `call f, ..`   | `jal x1, f`                              |
| `load_mem`     | `lw rd, off(rs)`                         |
| `store_mem`    | `sw rs, off(rd)`                         |

**Tests.** End-to-end: BASIC `10 LET A = 42 / 20 END` → IIR →
`iir-to-riscv` → RV32I machine code → load into `RiscVSimulator`
→ run → assert register `a0 == 42` at halt.  This is the
substitute for "compile to a `.elf` and run on real hardware".

**Crate to create:** `iir-to-riscv` (mirrors `iir-to-wasm`'s
shape: `validate.rs`, `lower.rs`, `tests/`).

### A2: Intel 8008 (Oct's native target)

**Why.** Oct was designed as a typed 8-bit systems language
*specifically for the 8008*.  Closing the loop — Oct source →
IIR → 8008 machine code → run on `intel8008-simulator` — is a
satisfying arc and validates Oct's value proposition against its
original target.

**Mapping.** The 8008 has a 14-bit address bus, 7 registers, no
multiply.  Limit Oct programs to: no recursion (one return-
address register), no multiply (use shift+add), pointers limited
to 14 bits.  The `intel-8008-assembler` already does the byte
encoding; we just need IIR → assembler-AST.

**Crate to create:** `iir-to-intel8008` consumes the existing
`intel-8008-assembler`.

### A3: ARMv7 (phone-class ARM)

**Why.** Complements aarch64.  Phones from ~2010-2017 run
ARMv7.  A24+ may make this obsolete but it's still a useful
training target.

**Crate to create:** `iir-to-armv7` (sibling of
`aarch64-backend`).  Or refactor `aarch64-backend` to be
configurable across ARM modes.

### A4: Intel 4004 (the original)

**Why.** Historical/educational.  Tiny ISA, 4-bit data, single
accumulator.  The 4004 is essentially a one-shot demo target —
arithmetic + jumps only.  Brainfuck would actually fit perfectly.

**Crate to create:** `iir-to-intel4004` consuming
`intel-4004-assembler`.  Likely only Brainfuck and trivial Oct
programs are practical; everything else hits ROM-size / address-
space limits.

### A5: GE-225 (the machine BASIC was born on)

**Why.** Dartmouth BASIC ran on a GE-225 mainframe in 1964.
Compiling BASIC → IIR → GE-225 machine code and running it on
`ge225-simulator` is a perfect nostalgia / authenticity arc.

**Crate to create:** `iir-to-ge225` consuming whatever assembler
the GE-225 simulator wants (TBD — audit the simulator's input
format first).

## 3. Order of execution

Items are sized as roughly-1-PR steps so the babysitter loop can
chew through them.

The architecture backends go AFTER the existing wasm/jvm/clr/llvm
gaps (G1-G5, LLVM01-04) and BEFORE the AOT-DBG track — the DWARF
plumbing is much easier to validate once we have several native
targets that wouldn't otherwise have host debuggers.

Architecture-specific items (numbered to extend the master plan):

14. **A1**: `iir-to-riscv` crate skeleton + const_i64 + ret + smoke
    test running on `riscv-simulator`.
15. **A1+**: arithmetic + cmp + control flow on RISC-V.
16. **A1++**: cross-fn calls + locals + print (via ecall) on RISC-V.
17. **A1+++**: lang-aot `--target=riscv32` wiring + e2e BASIC/Nib/Oct.
18. **A2**: `iir-to-intel8008` crate + smoke (Oct's `fn main()
    { let x: u8 = 42; }` runs on 8008 simulator).
19. **A2+**: full Oct V1 subset on 8008 (drop intrinsics that
    aren't on the chip, validate the rest).
20. **A3**: `iir-to-armv7` crate.
21. **A4**: `iir-to-intel4004` crate (Brainfuck + trivial Oct).
22. **A5**: `iir-to-ge225` crate (BASIC).

Each PR ships per the repo standards: spec → tests → impl →
CHANGELOG → README → security-review → push → babysitter cron.

After (22), every language we support has at least one "real" CPU
backend in addition to its WASM/JVM/CLR/LLVM bytecode targets,
validated by in-tree simulator execution.

## 4. What this gives us

For each language × architecture pair, we'll have:
- A validator (does this backend accept this IR shape?)
- An encoder (produce real machine code)
- A simulator runner (execute and check observable behaviour)

The matrix gets big.  We document only what's wired and what's
deferred (matching the BEAM posture from the original plan).

## 5. Out of scope (for this addendum)

- **Real-hardware deployment.**  Producing `.elf` for RISC-V
  Linux, ARM Android, etc.  The simulator validates the codegen;
  shipping to real CPUs needs OS-specific linkers and signing
  flows that are orthogonal.
- **GPU backends.**  We have `gpu-core` but no IIR-to-GPU.  Not
  in scope here.
- **FPGA targets** (Verilog/VHDL emission from IIR).  Not in
  scope.
- **JIT for these architectures** (e.g. running RISC-V codegen
  in-process via a sandboxed RWX page).  Defer — the AOT story
  is enough for the matrix-coverage claim.
