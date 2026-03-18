# 00 — Architecture Overview

## The Computing Stack

This project builds every layer of computation from high-level language down to
logic gates. The architecture has a **fork** after parsing — the same source code
can take two different paths to execution:

```
Layer 1:  Source Code              "x = 1 + 2"
              │
Layer 2:  Lexer                    NAME('x') EQUALS NUMBER(1) PLUS NUMBER(2)
              │
Layer 3:  Parser                   AST (Abstract Syntax Tree)
              │
              ╔═══════════════════════╦════════════════════════════╗
              ║  Path A: Interpreted  ║  Path B: Compiled          ║
              ╠═══════════════════════╬════════════════════════════╣
              ║                       ║                            ║
Layer 4a: Bytecode Compiler         Layer 4b: Machine Code Compiler
              ║  AST → stack bytecode ║  AST → assembly language   ║
              ║                       ║                            ║
Layer 5:  Virtual Machine           Layer 6:  Assembler            ║
              ║  stack-based eval     ║  assembly → binary         ║
              ║  loop (like JVM/CLR)  ║                            ║
              ║         │             ║                            ║
              ║    [JIT: Layer 5b]    Layer 7:  ISA Simulator      ║
              ║    compiles hot       ║  RISC-V / ARM / WASM /    ║
              ║    bytecode to        ║  Intel 4004                ║
              ║    native code        ║  decode + execute          ║
              ║         │             ║         │                  ║
              ╚═════════╩═════════════╩═════════╩══════════════════╝
                                       │
Layer 8:  CPU                     fetch-decode-execute cycle
              │                   registers, program counter, memory
              │
Layer 9:  ALU / Arithmetic        half adder, full adder, ripple-carry
              │
Layer 10: Logic Gates             AND, OR, XOR, NOT, NAND, NOR
```

## Two paths from the same source

### Path A: Interpreted (Python, Ruby, Java, C#)

The **bytecode compiler** translates the AST into a sequence of simple stack
instructions. These are NOT real machine instructions — they are invented
instructions for our virtual CPU (the VM).

The **virtual machine** executes these bytecode instructions one at a time,
using a stack to hold intermediate values. This is exactly how Python (CPython),
Ruby (YARV), Java (JVM), and C# (CLR) work.

```
"x = 1 + 2" → AST → Bytecode:     VM executes:
                     LOAD_CONST 1   push 1          stack: [1]
                     LOAD_CONST 2   push 2          stack: [1, 2]
                     ADD             pop 2, add      stack: [3]
                     STORE_NAME x   pop, store      x = 3
```

The **JIT compiler** (future) watches the VM execute. When it spots hot code
(a loop running thousands of times), it compiles that bytecode directly to
native machine instructions so subsequent runs skip the VM entirely.

### Path B: Compiled (C, Rust, Go)

The **machine code compiler** translates the AST directly into assembly language
for a specific processor (RISC-V, ARM, x86).

The **assembler** converts that human-readable assembly into binary machine code.

The **ISA simulator** decodes and executes those binary instructions using
simulated registers, memory, and an ALU.

```
"x = 1 + 2" → AST → RISC-V assembly:     Simulator executes:
                     addi x1, x0, 1        R1 = 0 + 1 = 1
                     addi x2, x0, 2        R2 = 0 + 2 = 2
                     add  x3, x1, x2       R3 = 1 + 2 = 3
```

### What both paths share

Below Layer 7, everything converges. Whether instructions come from the VM
(via JIT) or from the assembler, they execute on the same simulated hardware:

- **CPU** (Layer 8): The fetch-decode-execute cycle, registers, program counter
- **ALU** (Layer 9): Arithmetic circuits built from adders
- **Logic Gates** (Layer 10): The physical foundation — AND, OR, XOR, NOT

## Package map

| Layer | Package | Status |
|-------|---------|--------|
| 1 | (source code — not a package) | — |
| 2 | `lexer` | Implementing |
| 3 | `parser` (module: `lang_parser`) | Implementing |
| 4a | `bytecode-compiler` | Implementing |
| 4b | `machine-code-compiler` | Shell (future) |
| 5 | `virtual-machine` | Implementing |
| 5b | `jit-compiler` | Shell (future) |
| 6 | `assembler` | Shell |
| 7 | `arm-simulator` | Implemented |
| 7 | `riscv-simulator` | Implemented |
| 7 | `wasm-simulator` | Implemented |
| 7 | `intel4004-simulator` | Implemented |
| 8 | `cpu-simulator` | Implemented |
| 9 | `arithmetic` | Implemented |
| 10 | `logic-gates` | Implemented |

## Cross-cutting packages

| Package | Purpose |
|---------|---------|
| `pipeline` | Orchestrator — chains all layers together |
| `html-renderer` | Generates visual HTML reports showing every stage |

## ISA comparison

Our four ISA simulators represent three different CPU architectures:

| ISA | Architecture | Year | Width | Operands |
|-----|-------------|------|-------|----------|
| RISC-V (RV32I) | Register machine | 2010 | 32-bit | From named registers |
| ARM (ARMv7) | Register machine | 1985 | 32-bit | From named registers |
| WASM | Stack machine | 2017 | 32-bit | From operand stack |
| Intel 4004 | Accumulator | 1971 | 4-bit | Accumulator + register |

They all use the same ALU underneath — the difference is how operands get to
the ALU and where results are stored.
