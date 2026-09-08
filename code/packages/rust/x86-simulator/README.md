# x86-64 Functional and Runtime Simulator (Rust)

This crate now contains two deliberately separate public lanes:

- `functional::X86FunctionalSimulator` is the complete Spec 07w architectural
  machine: exact owned 64 KiB wrapping memory, 16 GPRs, RIP, the five specified
  RFLAGS bits, HLT state, installed-program metadata, immutable snapshots,
  checked direct access, atomic reset/load/restore/step/run operations, and
  complete before/after traces.
- `Simulator` and `harness::MachineCodeHarness` preserve the backend-runtime
  lane that **decodes and executes the 64-bit x86 machine code the in-repo
  `x86_64-backend` emits**, including SSE and host-call shims.

On an Apple Silicon (aarch64) machine
the LANG-FULL matrix's `NativeAot` cell only ever runs the *aarch64* backend; the
x86_64 backend is verified locally by byte tests and actually executed only on an
x86 CI runner. This crate closes that gap: it runs x86_64 codegen **locally**.

It is the runtime sibling of `riscv-simulator` (same `new`/`load`/`run`/`step`
shape) and uses the ISA semantics in
[`07w-x86-64-simulator.md`](../../../specs/07w-x86-64-simulator.md).

## What it runs

The architectural lane covers the complete integer surface specified by 07w:

- **Moves / addressing**: `mov` reg↔reg, reg↔`[base+disp]`, `mov reg,imm32`,
  `movzx reg,byte[mem]`, `mov byte[mem],reg8` (`0x88` — the byte-tape store),
  `lea reg,[mem]` (incl. RIP-relative); REX/ModRM/SIB.
- **Integer ALU**: `add` / `sub` / `cmp` / `and` / `or` / `xor` / `test` (reg and
  imm forms), `shl` / `shr` / `sar`, `imul` — all with full CF/ZF/SF/OF/PF flags.
- **Group-3 + division**: `not` / `neg` (`0xF7 /2`,`/3`), `div` / `idiv`
  (`0xF7 /6`,`/7`, dividing the 128-bit `rdx:rax` pair), and `cqo` (`rax`→`rdx:rax`
  sign-extend). Divide-by-zero / quotient-overflow raise a `#DE` **trap**.
- **Control flow**: `jmp`, `jcc` (all 16 conditions), `call` / `ret`, `push` /
  `pop`, `ret imm16`, LOOP/LOOPE/LOOPNE/JRCXZ, `cmovcc`, `setcc`, and `ud2`.
- **Transfer / bit / strings**: MOVSX/MOVZX/MOVSXD, XCHG, BSF/BSR/BT/BSWAP,
  PUSH immediates, MUL/IMUL/INC/DEC/ADC/SBB, rotates and CL-counted shifts, and
  REP STOSD/STOSQ.
- **SSE2 scalar double** (ALGOL `real` / E3): `movsd` (load/store/reg), `addsd` /
  `subsd` / `mulsd` / `divsd`, `ucomisd`, and `movabs r64, imm64` — enough to run
  the backend's `f64` arithmetic + comparison output.

Anything outside Spec 07w is a typed, transition-atomic error in the functional
lane. The legacy backend lane keeps its checked-memory sandbox and SSE surface.

## How to use it

For the architectural machine:

```rust
use x86_simulator::functional::X86FunctionalSimulator;

let mut cpu = X86FunctionalSimulator::new();
let result = cpu.run_checked(&[0x48, 0xc7, 0xc0, 42, 0, 0, 0, 0xf4], 8)?;
assert_eq!(result.final_state.gpr[0], 42);
# Ok::<(), x86_simulator::functional::X86Error>(())
```

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
internal `call` relocations, resolves the **`_twig_globals` data symbol** (a
zeroed 512-slot region between the code and heap — the `PcRel32` `lea` to it is
patched the same way the real linker would), routes external calls
(`__twig_alloc_bytes` via a bump heap, `putchar` / `print_i64` via captured I/O,
and `__twig_gc_write_barrier` as a no-op — the bump heap never collects, so there
is no remembered set to notify) to host shims, sets up a stack with a return
sentinel, and runs from the entry —
returning `rax & 0xFF` as the exit code (the same convention as `run_native` /
`run_wasm`). With `_twig_globals` support, programs that use **module globals**
(LANG-FULL E6 captured scalars, O3 Oct `static`, AL6 ALGOL `own`) run locally.

## Running the LANG-FULL matrix's x86_64 column locally

`tests/lang_matrix_x86.rs` drives the **real** language frontends through the
**real** AOT pipeline (`compile_source_to_iir` → `infer_types` → `aot_specialise`
→ `x86_64-backend`) and *runs the emitted x86_64 machine code* on this simulator.
On an Apple-Silicon host the matrix's `NativeAot` cell only ever builds+runs the
*aarch64* backend; this test exercises the **x86_64** column end-to-end —
**locally on aarch64**, retro-verifying columns the matrix could previously
execute only on x86 CI. The cells span Twig (const/arithmetic/`define`),
Nib (u8 wrap, `~` complement, unsigned division), ALGOL (procedure call,
switch/computed-goto, signed `div`, E3 `real` SSE2 floats, E5 arrays straight-
line and in a `for` loop, **E6 module globals**), Oct (`out`, `~`), Dartmouth BASIC (`PRINT`,
`FOR`/`NEXT` — stdout-captured via the host shims), and Brainfuck (`.` over a
byte tape, plus `,`-driven stdin: increment, echo, and cat — fed via
`MachineCodeHarness::stdin`). Each new language exposed a missing opcode — the
Nib/Oct `~` cells surfaced group-3 `0xF7`, and the Brainfuck cell surfaced the
`0x88` byte store —
which this crate now decodes.

## Safety

The simulator is a sandbox: every memory access is bounds-checked and every
unknown/illegal instruction or unresolved symbol is a `Trap`. A guest program
can only ever fault — it cannot escape or touch host memory.

## Layout

```
src/
├── functional.rs # exact Spec 07w state + checked atomic lifecycle
├── state.rs    # CpuState: 16 GPRs, rip, RFLAGS subset, XMM file
├── flags.rs    # add/sub_with_flags, condition_holds (07w rules)
├── memory.rs   # flat little-endian address space + bump heap
├── decode.rs   # REX/ModRM/SIB decoder → typed Instr
├── execute.rs  # per-instruction execution
├── harness.rs  # MachineCodeHarness — load + run backend output
└── lib.rs      # Simulator (step/run) + host imports
```
