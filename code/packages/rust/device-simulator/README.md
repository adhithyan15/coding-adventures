# device-simulator

**Layer 6 of the accelerator computing stack** -- complete device simulators that combine multiple compute units, global memory, caches, and work distributors into full accelerator models.

## What is a Device Simulator?

A device simulator models a **complete accelerator** -- not just one compute unit, but the entire chip with all its compute units, global memory, caches, and the work distributor that ties them together.

```text
Layer 7 (Compute Unit):    One SM / CU / MXU -- a single factory floor
Layer 6 (Device):          The whole factory -- all floors + warehouse +
                           shipping dock + floor manager's office
```

## Five Device Types

| Device     | Distributor        | Memory    | Execution Model       |
|------------|--------------------|-----------|-----------------------|
| NvidiaGPU  | GigaThread Engine  | HBM3      | SIMT thread blocks    |
| AmdGPU     | Command Processor  | GDDR6     | SIMD wavefronts       |
| GoogleTPU  | TPU Sequencer      | HBM2e     | Systolic pipeline     |
| IntelGPU   | Command Streamer   | GDDR6     | SIMD + threads        |
| AppleANE   | Schedule Replayer  | Unified   | Compiler-driven       |

## Usage

```rust
use device_simulator::nvidia_gpu::NvidiaGPU;
use device_simulator::protocols::{AcceleratorDevice, KernelDescriptor};
use gpu_core::opcodes::{limm, halt};

// Create a small NVIDIA GPU for testing
let mut gpu = NvidiaGPU::new(None, 4);

// Allocate and copy data
let addr = gpu.malloc(1024);
gpu.memcpy_host_to_device(addr, &[0u8; 1024]);

// Launch a kernel
let mut kernel = KernelDescriptor::default();
kernel.name = "saxpy".to_string();
kernel.program = Some(vec![limm(0, 2.0), halt()]);
kernel.grid_dim = (4, 1, 1);
kernel.block_dim = (32, 1, 1);
gpu.launch_kernel(kernel);

// Run to completion
let traces = gpu.run(2000);
assert!(gpu.idle());
```

## Architecture

The device layer adds four concepts on top of the compute unit layer:

1. **Global Memory (VRAM)** -- sparse HashMap-based VRAM/HBM model with coalescing, partitioning, and host transfer simulation.

2. **Work Distributor** -- three strategies: GPU block distributor (round-robin, fill-first, least-loaded), TPU sequencer (scalar/MXU/vector pipeline), ANE schedule replayer.

3. **L2 Cache** -- shared cache between compute units and global memory.

4. **Host Interface** -- PCIe/NVLink for discrete GPUs, zero-copy unified memory for Apple ANE.

## Dependencies

- `compute-unit` -- Layer 7 compute unit implementations
- `gpu-core` -- Layer 9 instruction set and core types
