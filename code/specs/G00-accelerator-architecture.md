# G00 — Accelerator Computing Stack Architecture

## Overview

This document describes the accelerator computing stack — a parallel track to
the existing CPU stack. It covers three architectures (GPU, TPU, NPU) that all
share the same foundation (logic gates + floating-point arithmetic) but diverge
in how they organize computation above that.

The goal: trace the journey of training or running inference on a neural network
from `result = model(input)` in Python all the way down to NAND gates — across
all three accelerator types.

## Layer numbering (top-down)

```
Layer 1:  Python ML Code           result = model(input)  /  loss.backward()
Layer 2:  Framework Simulator      PyTorch-like autograd + tensor ops
Layer 3:  Kernel Compiler          Python → PTX (GPU) / HLO (TPU) / ANE IL (NPU)
Layer 4:  Runtime Simulator        cudaRuntime / XLA Runtime / CoreML Runtime
Layer 5:  ISA Simulator            PTX (GPU) / HLO (TPU) / ANE Instructions (NPU)
Layer 6:  Device Simulator         Full GPU / TPU / NPU device
Layer 7:  Compute Unit Simulator   SM (GPU) / MXU (TPU) / Neural Engine Core (NPU)
Layer 8:  Execution Unit           Warp-SIMT (GPU) / Systolic Array (TPU) / MAC Array (NPU)
Layer 9:  Core Simulator           CUDA Core (GPU) / PE (TPU) / MAC Unit (NPU)
Layer 10: FP Arithmetic            IEEE 754 FP32/FP16/BF16, FMA — SHARED with CPU stack
Layer 11: Logic Gates              AND, OR, XOR, NAND — SHARED with CPU stack
```

## Three architectures

### GPU (NVIDIA CUDA) — throughput via massive parallelism

GPUs achieve performance through thousands of simple cores executing the same
instruction across many threads simultaneously (SIMT — Single Instruction,
Multiple Threads). Each core is simple (one FP32 ALU), but there are thousands
of them. A modern GPU like the H100 has 18,432 CUDA cores.

**Key unit:** Warp (32 threads executing in lockstep)
**Optimized for:** General parallel workloads, ML training, inference
**Programming model:** CUDA (explicit thread management)

### TPU (Google) — throughput via systolic arrays

TPUs use a fundamentally different approach: instead of many independent cores,
they use a systolic array — a grid of processing elements where data flows
through like a wave. Each PE performs one multiply-accumulate per cycle, and
the data moves to the next PE automatically. This is incredibly efficient for
matrix multiplication (the core operation of neural networks).

**Key unit:** Systolic Array (NxN grid of processing elements, data flows through)
**Optimized for:** Large matrix operations, ML training at scale
**Programming model:** XLA (compiler-managed, no explicit threads)

### NPU (Apple/Qualcomm) — efficiency via specialized MAC arrays

Neural Processing Units are designed for on-device inference with extreme power
efficiency. They have dedicated multiply-accumulate (MAC) arrays optimized for
the specific tensor operations used in neural networks. Unlike GPUs (general
purpose) and TPUs (training-focused), NPUs are inference-focused and trade
flexibility for efficiency.

**Key unit:** MAC Array (multiply-accumulate pipelines)
**Optimized for:** On-device inference, power efficiency
**Programming model:** CoreML / ONNX (model-level, no explicit programming)

## How they differ at each layer

| Layer | GPU | TPU | NPU |
|-------|-----|-----|-----|
| Core (9) | CUDA core: 1 FP ALU | Processing Element: 1 MAC | MAC unit |
| Execution (8) | Warp: 32 threads SIMT | Systolic array: data flows | MAC array: parallel MACs |
| Compute unit (7) | SM: scheduler + shared mem | MXU: systolic + accumulators | NE core: MACs + activations |
| Device (6) | Many SMs + VRAM | MXU + vector + scalar + HBM | Multiple cores + SRAM + DMA |
| ISA (5) | PTX: thread-centric | HLO: operation-centric | Tile-centric instructions |
| Runtime (4) | CUDA: explicit memory | XLA: compiler-managed | CoreML: model-level |
| Compiler (3) | Source → PTX | Graph → HLO | Graph → tiled ops |
| Framework (2) | PyTorch (eager, autograd) | JAX (functional, JIT) | CoreML (inference) |

## Key insight: same matmul, three paths

All three compute C = A @ B, but the hardware journey is completely different:

**GPU:** Each of 32 threads in a warp computes one output element. All threads
execute the same multiply-accumulate instruction but on different data. Shared
memory enables cooperative tiling to reduce global memory traffic.

**TPU:** Data A flows through the rows of the systolic array while data B is
pre-loaded into the columns. Each PE multiplies its inputs and passes the
partial sum to the next PE. After N cycles, the complete dot products emerge.

**NPU:** Input tiles are loaded from memory into on-chip SRAM. The MAC array
processes the tile (all multiply-accumulates in parallel). Results are stored
back. The compiler determines the optimal tiling strategy at compile time.

## Shared foundation

`fp-arithmetic` (Layer 10) is shared by the CPU stack AND all three accelerator
stacks. It implements IEEE 754 floating-point operations from logic gates:

```
logic-gates (existing, Layer 11)
    │
    ├──→ arithmetic (existing, CPU integer path)
    │       └──→ cpu-simulator → ISA simulators → ...
    │
    └──→ fp-arithmetic (NEW, shared Layer 10)
            ├──→ cuda-core-simulator (GPU path)
            ├──→ tpu-pe-simulator (TPU path)
            └──→ npu-mac-simulator (NPU path)
```

## Spec numbering

- `G00` — This architecture overview
- `G01` through `G10` — GPU track specs
- `T01` through `T10` — TPU track specs
- `N01` through `N10` — NPU track specs
- `FP01` — Floating-point arithmetic (shared)
