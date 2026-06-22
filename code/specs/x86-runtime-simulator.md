# x86 / x86-64 Runtime Simulator (`x86-simulator`)

## Why this exists

The LANG-FULL matrix verifies every code-gen backend by **running** its output:
WASM on the in-repo `wasm-runtime`, CLR on `ilasm`+`dotnet`, JVM on real `java`,
LLVM via `clang`. The one backend that is **never executed locally on this
machine** is `x86_64-backend`: the matrix's `NativeAot` cell uses `run_native`,
which compiles **for the host architecture**. On an Apple Silicon (aarch64) host
that means the aarch64 backend runs and the x86_64 backend does **not** — its
codegen is verified locally only by byte-comparison unit tests and actually
*executed* only on the Linux CI runner.

This crate closes that gap. `x86-simulator` is a Rust **runtime simulator** that
decodes and executes x86-64 machine code, and — crucially — can load and run the
`x86_64-backend`'s emitted function blobs (with host-import shims and a managed
heap/stack), returning the program's exit code. That makes the x86_64 column of
`lang_matrix.rs` runnable **locally on aarch64**, so x86_64 codegen (E3 SSE
floats, E5 arrays, and everything future) can be *run*, not just byte-compared,
without waiting for an Intel machine.

It is the Rust runtime sibling of the existing `riscv-simulator` crate (same
`new`/`load`/`run`/`step` shape), and it consumes the x86-64 **ISA semantics**
already specified in detail by [`07w-x86-64-simulator.md`](07w-x86-64-simulator.md)
(register file, RFLAGS, REX/ModRM/SIB encoding, per-instruction behaviour, flag
update rules, condition codes). This spec does **not** restate those tables; it
references 07w as the semantic authority and specifies the **Rust crate, the
instruction subset to prioritise, and the lang-aot integration harness** — the
parts 07w does not cover.

## Scope

- **x86-64 (long mode)** — the primary target, because that is what
  `x86_64-backend` emits. Implemented first.
- **x86 (32-bit, i386 protected mode)** — in scope (per the project directive),
  as a later phase. Our backend emits 64-bit only, so 32-bit is educational
  coverage that reuses the same decoder with operand-size/address-size handling;
  it is not on the critical path to running our codegen and lands after the
  x86-64 path is verified.
- **Out of scope**: real mode, segmentation beyond a flat model, privileged/ring
  transitions, paging, interrupts, the full ISA. We implement the subset the
  in-repo `x86_64-encoder` emits, plus enough to run the runtime helpers, and
  grow it as backends emit more. Unknown opcodes are a clean `DecodeError`
  (fail-closed), never undefined behaviour.

## Instruction subset (priority 1 — what `x86_64-encoder` emits)

Derived from `x86_64-encoder/src/lib.rs`. These must decode + execute first,
because running them *is* the goal:

- **Moves / addressing**: `mov r64, imm32/imm64`; `mov r64, [base+disp]`;
  `mov [base+disp], r64`; `movzx r64, byte [base]`; `mov byte [base], r8`;
  `lea r64, [rip+disp]` (RIP-relative, resolved against relocations).
- **Integer ALU**: `add r64,r64`; `add r64,imm32`; `imul r64,r64`; `shl r64,imm8`;
  `cmp r64,r64`; `cmp r64,imm32`; `test r64,r64`; `sub`/`and`/`or`/`xor` as the
  backend grows.
- **Control flow**: `jmp rel32`; `jcc rel32` (all 16 conditions via RFLAGS — the
  bounds-check `jb`, the comparison branches, etc.); `call rel32` (incl. external
  PLT-relocated calls); `ret`; `push`/`pop r64`.
- **Trap**: `ud2` (0F 0B) → a clean `Trap::IllegalInstruction` (SIGILL analogue;
  this is how E5's out-of-bounds array access aborts).
- **SSE2 scalar double** (priority 2 — E3 ALGOL reals): `movsd`, `addsd`, `subsd`,
  `mulsd`, `divsd`, `ucomisd`, `cvtsi2sd`/`cvttsd2si`, and the XMM register file.

## Crate design

`code/packages/rust/x86-simulator/` (Rust; BUILD + README + CHANGELOG per repo
standards), modelled on `riscv-simulator`'s module split:

```
src/
├── lib.rs        # public API + re-exports
├── state.rs      # CpuState: 16 GPRs, rip, rflags (CF/ZF/SF/OF/PF/AF), XMM0..15
├── flags.rs      # add_with_flags / sub_with_flags / condition_holds (per 07w)
├── decode.rs     # REX/ModRM/SIB/disp/imm decoder → a typed Instr enum
├── execute.rs    # step(): execute one decoded Instr against CpuState + Memory
├── memory.rs     # flat little-endian address space; stack region; bump heap
└── harness.rs    # MachineCodeHarness — load backend blobs, run, return exit code
```

### CPU + memory model

- `CpuState`: `gpr: [u64; 16]`, `rip: u64`, `rflags` (the 6 arithmetic flags
  this backend needs), `xmm: [u128; 16]` (used by the SSE2 phase).
- `Memory`: one flat `Vec<u8>`, little-endian load/store with width (1/2/4/8) and
  an explicit bounds check → `Trap::MemoryFault` on out-of-range (the simulator
  is itself a sandbox; a buggy emitted program faults cleanly, never touches host
  memory). Layout: a **code region** (the loaded function bytes), a **stack**
  (rsp initialised near the top, grows down), and a **bump heap** that backs
  `__twig_alloc_bytes`.

### `step` and `run`

- `step(&mut self) -> Result<StepOutcome, Trap>` — decode at `rip`, execute,
  advance `rip` (or branch), return `Continue` / `Halt(exit_code)` / a `Trap`.
  Mirrors `riscv-simulator`'s `step`/`run` shape for family consistency. (SIM00
  is a Python protocol; this Rust crate follows the *shape*, not the literal
  Python `Protocol`, exactly as `riscv-simulator` does.)

### The integration harness (the load-bearing new piece)

`MachineCodeHarness::run(functions, entry, host) -> i32` — the bridge that makes
the x86_64 backend runnable locally:

1. **Load**: take the `x86_64-backend` output — each function's machine-code
   `Vec<u8>` + the per-function symbol→offset map + the external relocations
   (`ExternalReloc`, e.g. `PltRel32` calls). Concatenate the function bytes into
   the code region and record each symbol's absolute address.
2. **Relocate**: patch each relocation site. Internal `call`/`jmp` targets
   resolve to the loaded function addresses; **external** symbols
   (`__twig_alloc_bytes`, `putchar`, `getchar`, `__print_i64`) resolve to
   sentinel addresses that the executor recognises.
3. **Host imports** (the analogue of `wasm-runtime`'s host): when `call` targets
   an external sentinel, dispatch to a Rust closure instead of executing bytes —
   `__twig_alloc_bytes(n)` bumps the heap and returns a pointer (matching the
   System V ABI: arg in `rdi`, result in `rax`); `putchar`/`getchar`/`print_i64`
   capture I/O. This is exactly how the WASM/CLR/JVM runtimes shim their host
   functions.
4. **Run**: initialise the stack, set `rip` to the entry symbol, step until
   `ret` returns to the initial frame (or a `Trap`). Return `rax & 0xFF` as the
   process exit code — the same convention `run_native`/`run_wasm` use.

This harness is what `lang_matrix.rs` calls to run the x86_64 column locally.

## Verification

- **Unit tests** per module (decode round-trips against `x86_64-encoder` output;
  execute tests per instruction group; flag-update tests against 07w's rules).
- **Differential against the encoder**: assemble a sequence with `x86_64-encoder`,
  run it on this simulator, assert the architectural result — the encoder and
  simulator cross-check each other.
- **End-to-end**: run an `x86_64-backend`-compiled function (e.g. the matrix's
  Twig `42`, then an arithmetic program) through the harness and assert the exit
  code, locally on aarch64.
- **Retro-verify E5 / E3**: once the harness runs, execute the E5 native-array
  `Prog` and the E3 SSE-float `Prog`s' **x86_64** output locally — the x86_64
  codegen that today only runs in CI.

## PR sequence

0. **S0 — this spec** (specs-first), committed for sign-off. ✅ done (#6406).
1. **S1 — integer core + harness**: `CpuState`/`Memory`/`flags`/`decode`/`execute`
   for the priority-1 integer subset + the `MachineCodeHarness`. Proof: run a
   simple `x86_64-backend`-compiled function locally → correct exit code.
   ✅ done (#6412).
2. **S2 — SSE2 scalar doubles**: XMM file + `movsd`/`addsd`/…/`ucomisd`/`cvt*`.
   Proof: run an E3 ALGOL `real` program's x86_64 output locally. ✅ done (#6416).
3. **S3 — wire into the matrix**: ✅ done. `tests/lang_matrix_x86.rs` replicates
   `twig-aot`'s native per-function pipeline (`compile_source_to_iir` →
   `infer_types` → `aot_specialise` → `compile_function_with_relocs`) and runs
   the emitted x86_64 machine code on the simulator — so the matrix's x86_64
   column executes **locally on aarch64**. Seven cells (Twig arithmetic, an ALGOL
   procedure call exercising internal `call` relocations, E3 `real` SSE2 floats,
   and an E5 native bounded array) retro-verify E5 native arrays and E3 floats on
   the x86_64 backend. (CI still runs real x86_64 as the independent check.)
   *Implementation note:* rather than adding a `run_x86_sim` arm inside
   `lang_matrix.rs` (which would couple the matrix harness to a dev-dep on this
   crate), the run path lives in this crate's own integration test, keeping the
   "run our x86_64 codegen" capability self-contained where the simulator is.
4. **S4 — group-3 opcodes + broader matrix coverage**: ✅ done. Running 8 more
   matrix programs through the simulator (`tests/lang_matrix_x86.rs`, now 15
   cells: Twig `define`, Nib u8-wrap/`~`, ALGOL switch + `for`-loop array, Oct
   `out`/`~`, BASIC `PRINT`/`FOR`) surfaced a missing opcode: the backend's
   `not`/`neg`/`div`/`idiv` lower to **group-3 `0xF7`** (+ `cqo` `0x99`), which
   S1–S3 never decoded. Added them (dividing the 128-bit `rdx:rax` pair) plus a
   `#DE` `Trap::DivideError` for divide-by-zero / quotient-overflow. This is the
   pattern the simulator is *for*: broadening coverage flushes out real codegen
   gaps the byte tests didn't. (x86-simulator 0.4.0.)
5. **S5 — byte-tape store + Brainfuck**: ✅ done. Running a Brainfuck program
   (`++++++++[>++++++++<-]>+.` ⇒ `A`) surfaced the **8-bit store `mov r/m8, r8`
   (`0x88`)** the byte-tape `store_byte` emits (S1–S4 only had the `movzx` load
   side), plus the `__twig_putchar`/`__twig_getchar` host-shim aliases. Added
   both; Brainfuck now runs on the x86_64 column locally. (Stdin-driven cat
   `,[.,]` still pends an input buffer in the harness.) (x86-simulator 0.5.0.)
6. **S6 — 32-bit x86 (i386)**: operand/address-size handling + 32-bit decode, as
   educational coverage (not on the run-our-codegen critical path — no matrix
   program emits 32-bit x86, so it's unit-tested against hand-assembled bytes).

## Open questions (for §0 sign-off)

- **Naming**: crate `x86-simulator` (covers both widths) vs `x86_64-simulator`?
  This spec assumes `x86-simulator` since 32-bit is in scope.
- **Relationship to the Python 07w**: 07w stays the canonical *behavioral ISA*
  spec; this Rust crate is the *runtime* simulator for backend codegen. Should
  the two share golden test vectors (assemble once, check both)?
- **Heap model for `__twig_alloc_bytes`**: a simple monotonic bump allocator is
  enough for the matrix programs (no free); confirm that is acceptable for v1.
