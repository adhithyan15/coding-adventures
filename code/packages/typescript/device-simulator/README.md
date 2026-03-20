# @coding-adventures/device-simulator

Complete accelerator device simulators -- Layer 6 of the accelerator computing stack.

## What is this?

This package simulates **complete accelerator devices**, assembling multiple compute units (Layer 7) with global memory, L2 cache, and work distribution into full devices that can launch and execute kernels.

```
Layer 9:  gpu-core (one core, one instruction at a time)
    |
Layer 8:  parallel-execution-engine (warps, wavefronts, systolic arrays)
    |
Layer 7:  compute-unit (SM, CU, MXU, XeCore, ANECore)
    |
Layer 6:  device-simulator (THIS PACKAGE)
    |
    +-- NvidiaGPU       -- many SMs + HBM + L2 + GigaThread
    +-- AmdGPU          -- CUs in Shader Engines + Infinity Cache
    +-- GoogleTPU       -- Scalar/Vector/MXU pipeline + HBM
    +-- IntelGPU        -- Xe-Cores in Xe-Slices + L2
    +-- AppleANE        -- NE cores + SRAM + DMA + unified memory
```

## Five Device Architectures

| Device | Compute | Memory | Work Distribution |
|--------|---------|--------|-------------------|
| NVIDIA GPU | SMs with warp schedulers | HBM + L2 | GigaThread Engine (round-robin) |
| AMD GPU | CUs in Shader Engines | GDDR6 + Infinity Cache + L2 | Command Processor |
| Google TPU | Scalar/Vector/MXU pipeline | HBM | Sequencer (tile pipeline) |
| Intel GPU | Xe-Cores in Xe-Slices | GDDR6 + L2 | Command Streamer |
| Apple ANE | NE Cores with MAC arrays | Unified Memory (zero-copy!) | Schedule Replayer |

## Quick Start

```typescript
import { NvidiaGPU, makeKernelDescriptor } from "@coding-adventures/device-simulator";
import { limm, halt } from "@coding-adventures/gpu-core";

// Create a small GPU for testing
const gpu = new NvidiaGPU({ numSMs: 4 });

// Allocate and copy data
const addr = gpu.malloc(1024);
gpu.memcpyHostToDevice(addr, new Uint8Array(1024));

// Launch a kernel
gpu.launchKernel(makeKernelDescriptor({
  name: "saxpy",
  program: [limm(0, 2.0), halt()],
  gridDim: [4, 1, 1],
  blockDim: [32, 1, 1],
}));

// Run to completion
const traces = gpu.run(1000);
console.log(`Completed in ${traces.length} cycles`);
```

## Dependencies

- `@coding-adventures/compute-unit` -- Layer 7 compute units (SM, CU, MXU, etc.)
- `@coding-adventures/cache` -- L2 and Infinity Cache simulation
- `@coding-adventures/gpu-core` -- ISA and instruction types
- `@coding-adventures/fp-arithmetic` -- Floating-point format support
- `@coding-adventures/clock` -- Clock signal generation

## Testing

```bash
npm test                    # Run tests
npm run test:coverage       # Run with coverage
```
