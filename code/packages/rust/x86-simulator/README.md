# x86 / x86-64 Runtime Simulator (Rust)

A runtime simulator that **decodes and executes the 64-bit x86 machine code the
in-repo `x86_64-backend` emits** — so that backend's output can be *run* on any
host architecture, not just byte-compared. On an Apple Silicon (aarch64) machine
the LANG-FULL matrix's `NativeAot` cell only ever runs the *aarch64* backend; the
x86_64 backend is verified locally by byte tests and actually executed only on an
x86 CI runner. This crate closes that gap: it runs x86_64 codegen **locally**.

It is the runtime sibling of `riscv-simulator` (same `new`/`load`/`run`/`step`
shape) and uses the ISA semantics in
[`07w-x86-64-simulator.md`](../../../specs/07w-x86-64-simulator.md).

## What it runs

The subset the `x86_64-encoder` emits (and growing):

- **Moves / addressing**: `mov` reg↔reg, reg↔`[base+disp]`, `mov reg,imm32`,
  `movzx reg,byte[mem]`, `lea reg,[mem]` (incl. RIP-relative); REX/ModRM/SIB.
- **Integer ALU**: `add` / `sub` / `cmp` / `and` / `or` / `xor` / `test` (reg and
  imm forms), `shl` / `shr` / `sar`, `imul` — all with full CF/ZF/SF/OF/PF flags.
- **Group-3 + division**: `not` / `neg` (`0xF7 /2`,`/3`), `div` / `idiv`
  (`0xF7 /6`,`/7`, dividing the 128-bit `rdx:rax` pair), and `cqo` (`rax`→`rdx:rax`
  sign-extend). Divide-by-zero / quotient-overflow raise a `#DE` **trap**.
- **Control flow**: `jmp`, `jcc` (all 16 conditions), `call` / `ret`, `push` /
  `pop`, `setcc`, and `ud2` → an illegal-instruction **trap** (how an E5
  out-of-bounds array access aborts).
- **SSE2 scalar double** (ALGOL `real` / E3): `movsd` (load/store/reg), `addsd` /
  `subsd` / `mulsd` / `divsd`, `ucomisd`, and `movabs r64, imm64` — enough to run
  the backend's `f64` arithmetic + comparison output.

Pending phases: `cvtsi2sd` / `cvttsd2si` (int↔float), and 32-bit x86. Anything
unimplemented is a clean `DecodeError`.

## How to use it

The high-level entry is the **`MachineCodeHarness`** — the bridge that runs the
backend's output, mirroring `wasm-runtime`'s host-import model:

```rust
use x86_simulator::harness::{MachineCodeHarness, Reloc};

// `bytes` + `relocs` come from x86_64_backend::compile_function_with_relocs(...)
let mut sim = MachineCodeHarness::new()
    .function("main", &bytes, &relocs)
    .build("main")?;
let exit_code = sim.run()?;   // executes the real x86_64 machine code
```

The harness lays the function bytes into a flat sandboxed address space, patches
internal `call` relocations, routes external calls (`__twig_alloc_bytes` via a
bump heap, `putchar` / `print_i64` via captured I/O) to host shims, sets up a
stack with a return sentinel, and runs from the entry — returning `rax & 0xFF`
as the exit code (the same convention as `run_native` / `run_wasm`).

## Running the LANG-FULL matrix's x86_64 column locally

`tests/lang_matrix_x86.rs` drives the **real** language frontends through the
**real** AOT pipeline (`compile_source_to_iir` → `infer_types` → `aot_specialise`
→ `x86_64-backend`) and *runs the emitted x86_64 machine code* on this simulator.
On an Apple-Silicon host the matrix's `NativeAot` cell only ever builds+runs the
*aarch64* backend; this test exercises the **x86_64** column end-to-end —
**locally on aarch64**, retro-verifying columns the matrix could previously
execute only on x86 CI. The 17 cells span Twig (const/arithmetic/`define`),
Nib (u8 wrap, `~` complement, unsigned division), ALGOL (procedure call,
switch/computed-goto, signed `div`, E3 `real` SSE2 floats, E5 arrays straight-
line and in a `for` loop), Oct (`out`, `~`), and Dartmouth BASIC (`PRINT`,
`FOR`/`NEXT` — stdout-captured via the host shims). Adding the Nib/Oct `~` cells
is what surfaced the missing group-3 `0xF7` opcode that this crate now decodes.

## Safety

The simulator is a sandbox: every memory access is bounds-checked and every
unknown/illegal instruction or unresolved symbol is a `Trap`. A guest program
can only ever fault — it cannot escape or touch host memory.

## Layout

```
src/
├── state.rs    # CpuState: 16 GPRs, rip, RFLAGS subset, XMM file
├── flags.rs    # add/sub_with_flags, condition_holds (07w rules)
├── memory.rs   # flat little-endian address space + bump heap
├── decode.rs   # REX/ModRM/SIB decoder → typed Instr
├── execute.rs  # per-instruction execution
├── harness.rs  # MachineCodeHarness — load + run backend output
└── lib.rs      # Simulator (step/run) + host imports
```
