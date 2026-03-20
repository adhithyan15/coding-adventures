# G01 — Accelerator Processing Element (Generic Core)

## Overview

This package implements a **generic, pluggable processing element** — the smallest
compute unit in any accelerator. It sits one layer above floating-point arithmetic
and serves as the foundation that all vendor-specific implementations build on.

Think of it as an abstract "compute cell" that can be configured to behave like:
- An NVIDIA CUDA Core (SIMT, warps of 32)
- An AMD Stream Processor (SIMD, wavefronts of 32/64)
- An Intel Arc Vector Engine (SIMD8 × 4)
- An ARM Mali Execution Engine (SIMT, warps of 16)
- A Google TPU Processing Element (systolic dataflow)
- An NPU MAC Unit (scheduled multiply-accumulate)

## Layer position

```
Layer 11: Logic Gates (AND, OR, XOR, NAND)
    │
Layer 10: FP Arithmetic (IEEE 754 add/mul/fma)
    │
Layer 9:  Accelerator Core ← YOU ARE HERE
    │
    ├──→ GPU: Warp/SIMT Engine (32 cores in lockstep)
    ├──→ TPU: Systolic Array (NxN grid of PEs)
    └──→ NPU: MAC Array (parallel MACs)
```

## Why a generic core?

Every accelerator architecture has a "processing element" at its heart — a tiny
compute unit that does one floating-point operation per clock cycle. But the
details differ wildly across vendors:

```
NVIDIA:  CUDA Core    = 1 FP32 ALU + register file + scheduler port
AMD:     Stream Proc  = 1 FP32 ALU + VGPR file + wavefront slot
Intel:   Vector Engine = SIMD8 ALU + GRF + thread arbiter
ARM:     Exec Engine   = 1 FP32 ALU + register bank + warp slot
TPU:     PE           = 1 MAC unit + weight register + accumulator
NPU:     MAC Unit     = 1 MAC + activation function + buffer
```

Despite these differences, they all share a common pattern:

1. They have **local state** (registers, accumulators, buffers)
2. They **compute** (FP add, multiply, fused multiply-add)
3. They receive **instructions** or **data** from above
4. They can access some form of **local memory**

By defining a common protocol, we can build one simulation framework and plug in
vendor-specific behavior without rewriting the core infrastructure each time.

## Architecture: Protocol-Based Design

### The ProcessingElement Protocol

The most generic abstraction — any compute unit in any accelerator:

```python
class ProcessingElement(Protocol):
    """Any compute unit: GPU core, TPU PE, NPU MAC."""

    def step(self) -> ExecutionTrace:
        """Execute one cycle. Returns a trace of what happened."""
        ...

    @property
    def halted(self) -> bool:
        """True if this PE has finished execution."""
        ...

    def reset(self) -> None:
        """Reset to initial state (keep program if any)."""
        ...
```

### The InstructionSet Protocol

A pluggable instruction decoder/executor — swap this to change the ISA:

```python
class InstructionSet(Protocol):
    """Vendor-specific instruction decoder and executor."""

    @property
    def name(self) -> str:
        """ISA name: 'Generic', 'PTX', 'GCN', 'Xe', 'Mali'."""
        ...

    def execute(
        self,
        instruction: Instruction,
        registers: FPRegisterFile,
        memory: LocalMemory,
    ) -> ExecuteResult:
        """Decode and execute one instruction."""
        ...
```

### The GPUCore

A concrete ProcessingElement that uses the register-file + instruction-stream
execution model (suitable for all GPU vendors):

```python
class GPUCore:
    """A single GPU processing element with pluggable ISA.

    This is the simulation of one CUDA core, one AMD stream processor,
    one Intel vector engine, or one ARM Mali execution engine — depending
    on which InstructionSet you plug in.
    """

    def __init__(
        self,
        isa: InstructionSet,
        fmt: FloatFormat = FP32,
        num_registers: int = 32,
        memory_size: int = 4096,
    ) -> None: ...

    def load_program(self, program: list[Instruction]) -> None: ...
    def step(self) -> GPUCoreTrace: ...
    def run(self, max_steps: int = 10000) -> list[GPUCoreTrace]: ...
    def reset(self) -> None: ...
```

## Key components

### FPRegisterFile

A configurable floating-point register file. Each register holds a `FloatBits`
value (from the fp-arithmetic package).

```
┌───────────────────────────────────┐
│         FP Register File          │
├───────────────────────────────────┤
│  R0:  [sign][exponent][mantissa]  │  ← FloatBits (FP32/FP16/BF16)
│  R1:  [sign][exponent][mantissa]  │
│  R2:  [sign][exponent][mantissa]  │
│  ...                              │
│  R31: [sign][exponent][mantissa]  │
└───────────────────────────────────┘
```

Configuration:
- **Register count**: 32 (default), up to 255 for NVIDIA, 256 for AMD VGPRs
- **Float format**: FP32 (default), FP16, BF16
- All registers initialize to +0.0
- Provides both `FloatBits` and `float` read/write interfaces

### LocalMemory

A small, byte-addressable scratchpad memory with floating-point aware load/store.
This represents the per-thread local memory in a GPU, or the accumulator buffer
in a TPU/NPU.

```
┌─────────────────────────────────┐
│        Local Memory (4 KB)      │
├─────────────────────────────────┤
│  Address 0x000: [byte][byte]... │
│  Address 0x004: [byte][byte]... │
│  ...                            │
│  Address 0xFFC: [byte][byte]... │
└─────────────────────────────────┘
```

Key operations:
- `load_float(address, fmt)` → reads bytes, returns FloatBits
- `store_float(address, value)` → converts FloatBits to bytes, writes them
- Byte-addressable with bounds checking
- Little-endian byte order (matches x86/ARM convention)

### ExecutionTrace

Every instruction execution produces a trace record for educational visibility:

```python
@dataclass
class GPUCoreTrace:
    cycle: int              # Which clock cycle
    pc: int                 # Program counter before execution
    instruction: Instruction
    description: str        # "R3 = R1 * R2 + R3 = 1.0 * 2.0 + 3.0 = 5.0"
    registers_changed: dict[str, float]
    memory_changed: dict[int, float]
    next_pc: int
    halted: bool
```

## Generic Instruction Set

The GenericISA is a simplified educational instruction set that proves the
pluggable design works. It's not tied to any vendor — it's the "teaching ISA."

### Arithmetic instructions

| Opcode | Operands | Semantics | FP operation used |
|--------|----------|-----------|-------------------|
| FADD | Rd, Rs1, Rs2 | Rd = Rs1 + Rs2 | fp_add |
| FSUB | Rd, Rs1, Rs2 | Rd = Rs1 - Rs2 | fp_sub |
| FMUL | Rd, Rs1, Rs2 | Rd = Rs1 × Rs2 | fp_mul |
| FFMA | Rd, Rs1, Rs2, Rs3 | Rd = Rs1 × Rs2 + Rs3 | fp_fma |
| FNEG | Rd, Rs1 | Rd = -Rs1 | fp_neg |
| FABS | Rd, Rs1 | Rd = \|Rs1\| | fp_abs |

### Memory instructions

| Opcode | Operands | Semantics |
|--------|----------|-----------|
| LOAD | Rd, Rs1, imm | Rd = Mem[Rs1 + imm] (load float from memory) |
| STORE | Rs1, Rs2, imm | Mem[Rs1 + imm] = Rs2 (store float to memory) |

### Data movement

| Opcode | Operands | Semantics |
|--------|----------|-----------|
| MOV | Rd, Rs1 | Rd = Rs1 (register to register) |
| LIMM | Rd, imm | Rd = immediate float value |

### Control flow

| Opcode | Operands | Semantics |
|--------|----------|-----------|
| BEQ | Rs1, Rs2, offset | if Rs1 == Rs2: PC += offset |
| BLT | Rs1, Rs2, offset | if Rs1 < Rs2: PC += offset |
| BNE | Rs1, Rs2, offset | if Rs1 ≠ Rs2: PC += offset |
| JMP | target | PC = target (absolute) |
| NOP | | PC += 1 (do nothing) |
| HALT | | Stop execution |

Branch offsets are in **instruction units** (not bytes), relative to the
instruction *after* the branch. `BEQ R0, R1, +2` means "skip 2 instructions
forward if R0 == R1."

### Instruction encoding

Instructions use a dataclass representation, not binary encoding:

```python
@dataclass(frozen=True)
class Instruction:
    opcode: Opcode
    rd: int = 0           # destination register
    rs1: int = 0          # source register 1
    rs2: int = 0          # source register 2
    rs3: int = 0          # source register 3 (FMA only)
    immediate: float = 0.0  # literal value or branch offset
```

Binary encoding belongs at the assembler/ISA layer above (e.g., PTX assembler).
At this layer, we work with structured data — which is how real GPU hardware
receives instructions from the instruction cache after decode.

### Helper constructors

Readable program construction:

```python
fadd(rd, rs1, rs2)        → Instruction(FADD, rd, rs1, rs2)
fmul(rd, rs1, rs2)        → Instruction(FMUL, rd, rs1, rs2)
ffma(rd, rs1, rs2, rs3)   → Instruction(FFMA, rd, rs1, rs2, rs3)
load(rd, rs1, offset)     → Instruction(LOAD, rd, rs1, immediate=offset)
store(rs1, rs2, offset)   → Instruction(STORE, rs1=rs1, rs2=rs2, immediate=offset)
limm(rd, value)           → Instruction(LIMM, rd, immediate=value)
halt()                    → Instruction(HALT)
```

## Example programs

### SAXPY: y = a·x + y

The "hello world" of GPU programming. Scalar `a` times vector element `x` plus
vector element `y`, computed with a single FMA instruction:

```python
program = [
    limm(0, 2.0),        # R0 = a = 2.0
    limm(1, 3.0),        # R1 = x = 3.0
    limm(2, 1.0),        # R2 = y = 1.0
    ffma(3, 0, 1, 2),    # R3 = a * x + y = 2.0 * 3.0 + 1.0 = 7.0
    halt(),
]
```

### Dot product: sum of element-wise products

```python
program = [
    # Load vector A elements
    limm(0, 1.0),        # R0 = A[0]
    limm(1, 2.0),        # R1 = A[1]
    limm(2, 3.0),        # R2 = A[2]
    # Load vector B elements
    limm(3, 4.0),        # R3 = B[0]
    limm(4, 5.0),        # R4 = B[1]
    limm(5, 6.0),        # R5 = B[2]
    # Accumulate: R6 = A[0]*B[0] + A[1]*B[1] + A[2]*B[2]
    limm(6, 0.0),        # R6 = accumulator = 0.0
    ffma(6, 0, 3, 6),    # R6 = 1.0 * 4.0 + 0.0 = 4.0
    ffma(6, 1, 4, 6),    # R6 = 2.0 * 5.0 + 4.0 = 14.0
    ffma(6, 2, 5, 6),    # R6 = 3.0 * 6.0 + 14.0 = 32.0
    halt(),
]
```

### Loop: sum first N numbers

```python
program = [
    limm(0, 0.0),        # R0 = sum = 0.0
    limm(1, 1.0),        # R1 = i = 1.0
    limm(2, 1.0),        # R2 = increment = 1.0
    limm(3, 5.0),        # R3 = limit = 5.0
    # Loop body (PC=4):
    fadd(0, 0, 1),       # sum += i
    fadd(1, 1, 2),       # i += 1
    blt(1, 3, -2),       # if i < limit: jump back 2 instructions
    halt(),              # sum = 1+2+3+4 = 10.0
]
```

## How vendor ISAs plug in (future)

The InstructionSet protocol makes it trivial to add new ISAs. For example,
to add NVIDIA PTX support:

```python
class PTXISA:
    """NVIDIA Parallel Thread Execution instruction set."""

    @property
    def name(self) -> str:
        return "PTX"

    def execute(self, instruction, registers, memory) -> ExecuteResult:
        match instruction.opcode:
            case PTXOp.ADD_F32:
                result = fp_add(registers.read(instruction.rs1),
                               registers.read(instruction.rs2))
                registers.write(instruction.rd, result)
                return ExecuteResult(...)
            case PTXOp.FMA_RN_F32:
                # ... and so on
```

Then to use it:

```python
core = GPUCore(isa=PTXISA(), num_registers=255)  # NVIDIA uses up to 255 regs
core.load_program(ptx_program)
traces = core.run()
```

Similarly for AMD GCN, Intel Xe, ARM Mali — each is just a new class
implementing the InstructionSet protocol. The GPUCore infrastructure
(registers, memory, fetch loop, tracing) is reused unchanged.

## Differences from the CPU simulator

| Aspect | CPU Simulator | GPU Core |
|--------|--------------|----------|
| Register type | Integer (bit lists) | Floating-point (FloatBits) |
| ALU | Integer arithmetic | FP arithmetic (fp_add, fp_mul, fp_fma) |
| Pipeline | Fetch-decode-execute | Fetch-execute (no separate decode stage) |
| Branch prediction | Complex (in deep-cpu) | None (handled by warp scheduler above) |
| Out-of-order | Possible | Never (in-order, single-issue) |
| ISA | Fixed (RISC-V/ARM) | Pluggable (any vendor) |
| Memory | Unified address space | Small local scratchpad |

The GPU core is intentionally simpler — GPUs achieve performance through
massive parallelism (thousands of simple cores) rather than per-core complexity.

## Dependencies

- **fp-arithmetic**: FloatBits, FloatFormat, FP32/FP16/BF16, fp_add/sub/mul/fma/neg/abs/compare, float_to_bits/bits_to_float
- **clock**: Clock (for future pipelined mode)

## Package name

`gpu-core` across all languages (Ruby: `gpu_core` per naming convention).

## Implementation languages

Python, Ruby, Go, TypeScript, Rust — all five languages in the repo.
