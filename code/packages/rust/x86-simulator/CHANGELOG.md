# Changelog — x86-simulator

## 0.1.0 — 2026-06-21 — integer core + MachineCodeHarness (x86-sim PR-S1)

The first runnable slice: a Rust runtime simulator that decodes and executes the
64-bit x86 integer subset the `x86_64-backend` emits, and a harness that runs the
backend's compiled output locally — closing the gap where the x86_64 backend was
verified locally only by byte tests and *executed* only on an x86 CI runner.

### Added
- **`state`** — `CpuState`: 16 GPRs (hardware numbering), `rip`, the RFLAGS
  subset (CF/ZF/SF/OF/PF/AF) emitted code uses, and the XMM file (for the coming
  SSE2 phase).
- **`flags`** — `add_with_flags`/`sub_with_flags`/`logic_flags` and the 16
  condition codes via `condition_holds`, per `07w-x86-64-simulator.md`.
- **`memory`** — a flat, little-endian, **bounds-checked** address space (a
  sandbox) with a monotonic bump heap backing `__twig_alloc_bytes`.
- **`decode`** — a REX/ModRM/SIB decoder for the `x86_64-encoder` subset:
  `push`/`pop`, `mov` (reg/mem/imm), `movzx`, `lea`, `add`/`sub`/`cmp`/`and`/
  `or`/`xor`/`test` (reg + imm), `shl`/`shr`/`sar`, `imul`, `jmp`/`jcc`/`call`/
  `ret`, `ud2`. Unknown opcodes → a clean `DecodeError` (fail-closed).
- **`execute`** — per-instruction execution with full flag computation and a
  `Flow` the step loop acts on.
- **`Simulator`** (`lib`) — `step`/`run`; `ret` to a stack sentinel halts and
  yields the exit code (`rax & 0xFF`); host-import shims for `__twig_alloc_bytes`
  / `putchar` / `getchar` / `print_i64` (System V ABI), like `wasm-runtime`.
- **`harness::MachineCodeHarness`** — load the backend's per-function byte blobs
  + relocations, patch internal `call`s, route external calls to host shims, set
  up the stack, and produce a ready-to-run `Simulator`.

### Verified
- Decodes the exact prologue/`const`/`ret` bytes the `x86_64-backend` emits.
- **End-to-end**: compiles `const 42; ret` and `40 + 2` with the **real**
  `x86_64-backend` and runs the machine code → exit **42**, locally (on an
  aarch64 host). 15 tests (flags, decode, memory, execute, integration).

### Next
S2 — SSE2 scalar doubles (run E3 ALGOL `real` x86_64 output); S3 — wire a local
`run_x86_sim` matrix path + retro-verify E5 native arrays / E3 floats; S4 —
32-bit x86.
