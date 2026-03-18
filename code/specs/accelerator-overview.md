# Accelerator Computing Stack — Architecture Overview

## Motivation

The CPU computing stack (Layers 1-10) handles general-purpose computing:
sequential instruction execution, branching, function calls. But modern
workloads — ML training, graphics rendering, physics simulation — need
massive parallelism. That's what accelerators provide.

This spec defines a parallel stack that mirrors the CPU stack but for
accelerator hardware: GPUs, TPUs, and NPUs.

## The two stacks side by side

```
CPU Stack                              Accelerator Stack
═════════                              ═════════════════
Layer 1:  Source code                  Layer A1: Source code (same!)
Layer 2:  Lexer                        Layer A2: Lexer (same!)
Layer 3:  Parser                       Layer A3: Parser (same!)

Layer 4a: Bytecode compiler            Layer A4: Kernel compiler
Layer 5:  Virtual machine              Layer A5: Runtime / driver

              ← FORK →

Layer 6:  Assembler                    Layer A6: PTX/SPIR-V assembler
Layer 7:  ISA (RISC-V, ARM)            Layer A7: GPU ISA (SASS, GCN)

Layer 8:  CPU (fetch-decode-execute)   Layer A8: GPU (SIMT execution)

Layer 9:  ALU (integer arithmetic)     Layer A9: FPU + Tensor Core
Layer 10: Logic gates                  Layer A10: Logic gates (same!)
```

Key insight: the top (source code, lexer, parser) and bottom (logic gates)
are shared. The middle diverges — CPUs optimize for latency (fast single
thread), accelerators optimize for throughput (many slow threads).

## Layer breakdown

### Layer A10: Logic Gates (shared with CPU)
Already implemented. AND, OR, XOR, NOT, NAND.

### Layer A9: Floating-Point Unit (FPU)
**Package: `fp-arithmetic`**

Unlike the CPU's ALU (integer-focused), the GPU's core compute unit is
a floating-point unit. IEEE 754 arithmetic in multiple precisions:

- FP32 (32-bit float) — standard GPU precision
- FP16 (16-bit half) — ML inference, 2x throughput
- BF16 (bfloat16) — ML training, same range as FP32
- FP8 (8-bit) — emerging for inference (future)

Key operations: add, multiply, FMA (fused multiply-add).
FMA is the fundamental GPU operation — one multiply-add per clock per core.

### Layer A8: GPU Core Simulator
**Package: `gpu-core`**

A single GPU core (CUDA core / shader processor / execution unit).
Like a CPU core but simpler — no branch prediction, no out-of-order
execution, just: load → compute → store.

Supports: FP add, FP multiply, FMA, load, store, simple branching.
Execution model: in-order, single-issue.

### Layer A7: SIMT Engine
**Package: `simt-engine`**

SIMT = Single Instruction, Multiple Threads.

A "warp" (NVIDIA) or "wavefront" (AMD) of 32 threads that execute
the same instruction simultaneously but on different data.

```
Instruction: FMA R1, R2, R3    (one instruction)

Thread 0:  R1[0]  = R2[0]  * R3[0]  + R1[0]
Thread 1:  R1[1]  = R2[1]  * R3[1]  + R1[1]
Thread 2:  R1[2]  = R2[2]  * R3[2]  + R1[2]
...
Thread 31: R1[31] = R2[31] * R3[31] + R1[31]
```

All 32 happen in parallel. This is where GPU parallelism comes from.

Handles: warp divergence (when threads take different branches),
predicated execution, barrier synchronization.

### Layer A6: Streaming Multiprocessor (SM)
**Package: `sm-simulator`**

An SM contains multiple warp schedulers, each managing multiple warps.
A typical SM has 4 warp schedulers and can run 64 warps (2048 threads).

Handles: warp scheduling, shared memory, register file, L1 cache.

### Layer A5: GPU Simulator
**Package: `gpu-simulator`**

The full GPU: multiple SMs, global memory, memory controllers,
L2 cache, PCIe interface to CPU.

Handles: kernel launch, thread block → SM mapping, memory hierarchy.

### Layer A4: CUDA/OpenCL Simulator
**Package: `cuda-simulator`**

The programming model layer. Simulates the CUDA runtime:
- Kernel launch: `kernel<<<blocks, threads>>>(args)`
- Thread indexing: `threadIdx`, `blockIdx`, `blockDim`
- Memory management: `cudaMalloc`, `cudaMemcpy`
- Synchronization: `__syncthreads()`

Also: an OpenCL variant for vendor-neutral GPU programming.

### Layer A3-A1: Compiler → Source (shared/extended)
The existing lexer/parser/compiler extended to emit GPU kernel code
instead of (or in addition to) CPU bytecode.

## Tensor Core simulator (future)
**Package: `tensor-core`**

Specialized hardware for matrix multiply-accumulate:
D = A × B + C where A, B are 4×4 matrices.

One instruction does 128 FMA operations (4×4×4 + 4×4 adds).
This is NVIDIA's key advantage for ML training.

## TPU simulator (future)
**Package: `tpu-simulator`**

Google's Tensor Processing Unit. Systolic array architecture:
data flows through a grid of processing elements, each doing
one multiply-add and passing results to the next.

## NPU simulator (future)
**Package: `npu-simulator`**

Neural Processing Unit (Apple Neural Engine, Qualcomm Hexagon).
Optimized for specific neural network operations (convolution,
matrix multiply) rather than general-purpose GPU computing.

## Implementation order

Phase 1 (this PR):
1. fp-arithmetic — IEEE 754 FP32/FP16/BF16

Phase 2:
2. gpu-core — single core simulator
3. simt-engine — warp-level parallelism

Phase 3:
4. sm-simulator — streaming multiprocessor
5. tensor-core — matrix multiply hardware

Phase 4:
6. gpu-simulator — full GPU with memory hierarchy
7. cuda-simulator — CUDA runtime

Phase 5 (future):
8. tpu-simulator — systolic array
9. npu-simulator — neural engine

Each phase builds on the previous. Each package is independently
testable and demonstrates one layer of the GPU architecture.
