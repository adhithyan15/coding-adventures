# gpu_core (Lua)

Generic, pluggable accelerator processing element — the building block for GPU, TPU, and NPU simulations.

## Overview

`gpu_core` implements a single compute unit at layer 9 of the accelerator stack. It sits above floating-point arithmetic and serves as the foundation that all vendor-specific GPU implementations build on.

Think of it as an abstract "compute cell" that can be configured to behave like:

- An NVIDIA CUDA Core (SIMT, warps of 32)
- An AMD Stream Processor (wavefronts of 32/64)
- An Intel Arc Vector Engine
- An ARM Mali Execution Engine

## Usage

```lua
local gpu_core = require("coding_adventures.gpu_core")

local isa  = gpu_core.GenericISA.new()
local core = gpu_core.GPUCore.new(isa)

-- SAXPY: y = 2*x + y = 2*3 + 1 = 7
core:load_program({
  gpu_core.limm(0, 2.0),     -- R0 = a = 2.0
  gpu_core.limm(1, 3.0),     -- R1 = x = 3.0
  gpu_core.limm(2, 1.0),     -- R2 = y = 1.0
  gpu_core.ffma(3, 0, 1, 2), -- R3 = a * x + y = 7.0
  gpu_core.halt(),
})

local traces = core:run()
print(core.registers:read(3))  -- 7.0
```

## Architecture

- **FPRegisterFile** — 32 floating-point registers, all initialized to 0.0
- **LocalMemory** — 4096-slot scratchpad memory
- **GenericISA** — teaching instruction set: FADD, FSUB, FMUL, FFMA, FNEG, FABS, LOAD, STORE, MOV, LIMM, BEQ, BLT, BNE, JMP, NOP, HALT
- **GPUCore** — fetch-execute loop with cycle-accurate tracing

## Dependencies

None — this package is self-contained.
