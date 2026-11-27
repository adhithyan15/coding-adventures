# RISC-V Simulator (Go)

**Layer 4b of the computing stack** — implements the full RISC-V RV32I base integer instruction set with M-mode privileged extensions for OS support.

## Overview

RISC-V (pronounced "risk-five") is an open-source instruction set architecture designed at UC Berkeley. It is built on the philosophy of a Reduced Instruction Set Computer — favoring a small number of cleanly encoded, simple instructions over complex CISC-style operations.

This simulator interfaces with the `cpu-simulator` generic fetch-decode-execute pipeline.

When used through the configurable Go `core`, the RV32I adapter opts into typed
`rs1`/`rs2` forwarding metadata. The core applies these values to decoded
operands before arithmetic, branch comparison, or store data selection. Existing
sequential `RiscVSimulator` behavior remains the independent architectural
reference. The default core integration covers classic five-stage forwarding
and load-use stalls; precise word-memory behavior is a separate explicit mode
described below.

### Opt-in precise word-memory mode

`NewRiscVCoreWithPreciseMemory(config, memorySize)` enables a separate bounded
mode for aligned, in-range `LW` and `SW` over flat RAM. Stores commit when the
instruction reaches WB; a younger same-address load can read the pending older
store. An invalid or unaligned access becomes a terminal `PreciseFaultOutcome`
after older instructions drain. The selected fault token is not counted as a
successful retirement. Effective addresses wrap modulo 2^32 before alignment
and bounds checks.

This mode is opt-in and requires hazard detection and the classic
IF/ID/EX/MEM/WB stage order. It does not implement privileged traps, MMIO,
byte/halfword operations, fetch faults, or general rollback. Existing constructors retain their existing
MEM-time store behavior. `Core.PreciseStoreReceipts()` returns copies of
successful WB store commits; `Core.PreciseFaultOutcome()` returns the terminal
fault identity when one was selected.

## Supported Instructions

### RV32I Base Integer (37 instructions)

| Category | Instructions |
|----------|-------------|
| **Arithmetic** | add, sub, addi, slt, sltu, slti, sltiu |
| **Logical** | and, or, xor, andi, ori, xori |
| **Shifts** | sll, srl, sra, slli, srli, srai |
| **Loads** | lb, lh, lw, lbu, lhu |
| **Stores** | sb, sh, sw |
| **Branches** | beq, bne, blt, bge, bltu, bgeu |
| **Jumps** | jal, jalr |
| **Upper Immediate** | lui, auipc |
| **System** | ecall |

### M-mode Privileged Extensions

| Category | Instructions |
|----------|-------------|
| **CSR Access** | csrrw, csrrs, csrrc |
| **Trap Return** | mret |

**CSR Registers:** mstatus (0x300), mtvec (0x305), mscratch (0x340), mepc (0x341), mcause (0x342)

### Trap Handling

When `mtvec` is configured with a trap handler address:
- `ecall` saves PC to `mepc`, sets `mcause` to 11 (M-mode environment call), disables interrupts, and jumps to `mtvec`
- `mret` restores PC from `mepc` and re-enables interrupts
- When `mtvec` is 0 (no handler), `ecall` halts the CPU (simple program behavior)
- When `mtvec` is 0 and a `HostIO` object is attached, simple host syscalls
  are handled directly: `a7=1` writes the low byte of `a0`, `a7=2` reads one
  byte into `a0`, and `a7=10` exits.

## Architecture

```
simulator.go  — top-level simulator struct and factory
opcodes.go    — opcode and funct3/funct7 constants
decode.go     — instruction decoder (binary → structured fields)
execute.go    — instruction executor (structured fields → state changes)
csr.go        — Control and Status Register file for M-mode
encoding.go   — helpers to construct machine code for testing
```

### Register x0

RISC-V forces Register 0 (`x0`) to always be strictly `0`. Writes are silently ignored. This enables pseudoinstructions like:
- `addi x1, x0, 5` — load immediate 5 into x1
- `addi x1, x2, 0` — move x2 into x1 (mv pseudoinstruction)

## Usage

```go
import riscvsimulator "github.com/adhithyan15/coding-adventures/code/packages/go/riscv-simulator"

sim := riscvsimulator.NewRiscVSimulator(65536)

// Compute Fibonacci(10) = 55
program := riscvsimulator.Assemble([]uint32{
    riscvsimulator.EncodeAddi(1, 0, 0),     // x1 = 0 (fib[0])
    riscvsimulator.EncodeAddi(2, 0, 1),     // x2 = 1 (fib[1])
    riscvsimulator.EncodeAddi(4, 0, 2),     // counter = 2
    riscvsimulator.EncodeAddi(5, 0, 11),    // limit = 11
    riscvsimulator.EncodeAdd(3, 1, 2),      // x3 = x1 + x2
    riscvsimulator.EncodeAddi(1, 2, 0),     // x1 = x2
    riscvsimulator.EncodeAddi(2, 3, 0),     // x2 = x3
    riscvsimulator.EncodeAddi(4, 4, 1),     // counter++
    riscvsimulator.EncodeBne(4, 5, -16),    // loop if counter != limit
    riscvsimulator.EncodeEcall(),           // halt
})

traces := sim.Run(program)
// sim.CPU.Registers.Read(2) == 55
```

### Host Syscall Example

```go
host := riscvsimulator.NewHostIO([]byte("A"))
sim := riscvsimulator.NewRiscVSimulatorWithHost(65536, host)

program := riscvsimulator.Assemble([]uint32{
    riscvsimulator.EncodeAddi(17, 0, riscvsimulator.SyscallReadByte),
    riscvsimulator.EncodeEcall(), // a0 = 'A'
    riscvsimulator.EncodeAddi(17, 0, riscvsimulator.SyscallWriteByte),
    riscvsimulator.EncodeEcall(), // host.Output = "A"
    riscvsimulator.EncodeAddi(10, 0, 0),
    riscvsimulator.EncodeAddi(17, 0, riscvsimulator.SyscallExit),
    riscvsimulator.EncodeEcall(),
})

sim.Run(program)
// host.OutputString() == "A"
```

### Trap Handler Example

```go
sim := riscvsimulator.NewRiscVSimulator(65536)

// Set up trap handler, trigger ecall, handler returns
program := riscvsimulator.Assemble([]uint32{
    riscvsimulator.EncodeAddi(1, 0, 0x100),
    riscvsimulator.EncodeCsrrw(0, riscvsimulator.CSRMtvec, 1),  // mtvec = 0x100
    riscvsimulator.EncodeEcall(),                                 // trap!
    // ... execution resumes here after mret
})
```

## Testing

```bash
go test -v ./...       # run all tests
go test -cover ./...   # check coverage (97%)
```
