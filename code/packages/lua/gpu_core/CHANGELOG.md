# Changelog — gpu_core (Lua)

## 0.1.0 — 2026-03-31

Initial release.

- `FPRegisterFile` — configurable floating-point register file (default 32 registers)
- `LocalMemory` — scratchpad memory with bounds-checked load/store
- `GenericISA` — teaching instruction set with arithmetic, memory, and branch instructions
- `GPUCore` — fetch-execute loop with cycle-accurate tracing via `step()` / `run()`
- Instruction constructor helpers: `fadd`, `fsub`, `fmul`, `ffma`, `fneg`, `fabs`, `load`, `store`, `mov`, `limm`, `beq`, `blt`, `bne`, `jmp`, `nop`, `halt`
- Example programs: `SAXPY_PROGRAM`, `DOT_PRODUCT_PROGRAM`
