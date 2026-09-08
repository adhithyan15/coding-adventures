# Layer 07a2 — RISC-V RV32I Gate-Level Simulator

## Scope

This specification is the gate-level partner to Spec 07a and its completed
Rust `riscv-simulator::Rv32ISimulator` oracle. It implements the same RV32I base
integer surface plus the repository's five M-mode CSR, ECALL trap, and MRET
teaching extension. Legacy RV32M and host-service compatibility APIs are not
part of this gate cell.

The model operates at instruction boundaries. It does not reproduce analog
timing, a physical pipeline, caches, privilege levels beyond the documented
teaching extension, an MMU, package pins, or transistor placement.

## Persistent topology

The machine contains exactly **525,505 D flip-flops**:

| Bank | Width/count | DFFs |
|---|---:|---:|
| Little-endian memory | 65,536 x 8 | 524,288 |
| X0-X31 GPRs | 32 x 32 | 1,024 |
| Program counter | 32 | 32 |
| `mstatus`, `mtvec`, `mscratch`, `mepc`, `mcause` | 5 x 32 | 160 |
| Halt | 1 | 1 |
| **Total** | | **525,505** |

X0 occupies physical state but reads zero, discards writes, and must be zero in
restored snapshots. Installed program origin and length are checked lifecycle
metadata rather than architectural hardware state. Reset, restore, program
installation, direct writes, instruction transitions, CSR updates, and halt
transitions clock repository DFF primitives.

## Combinational execution

The crate owns an independent strict decoder for LUI, AUIPC, JAL, JALR, all six
branches, byte/halfword/word loads and stores, immediate and register ALU
families, shifts, ECALL, MRET, and CSRRW/CSRRS/CSRRC over the five supported
CSRs. Reserved funct fields, unsupported CSRs, and non-RV32I encodings fail
closed.

Repository Boolean gates implement opcode and funct predicates, muxes, bitwise
ALU operations, equality, sign/zero extension, and CSR bit updates. Repository
ripple-carry networks implement PC sequencing, target and effective addresses,
ADD/SUB and comparison. Fixed gate barrel networks implement logical and
arithmetic shifts. Host integers may assemble traces and index checked memory;
they do not replace architectural ALU results.

## Lifecycle and faults

The public `Rv32IState`, `Rv32IError`, `Rv32IStepTrace`, and
`Rv32IExecutionResult` contract is shared with `riscv-simulator`.
`load_checked`, `load_at_checked`, `restore`, register access, and byte access
are validated and typed. A faulting step preserves every entry-state bit. A
bounded run either returns complete traces and final state or rolls the entire
machine back on any fault or step-limit exhaustion.

Fetch is restricted to the installed aligned range. Halfword and word data
accesses require natural alignment, and all accesses are bounded by the exact
64 KiB memory. Unsupported encodings and CSRs report typed faults. ECALL with a
zero trap vector clocks halt; otherwise it records the documented trap state,
and MRET restores the documented machine-interrupt-enable bit and target PC.

## Conformance

Completion requires exact topology and lifecycle suites, strict malformed
decode and fault tests, and the reproducible 256-vector Python corpus covering
every RV32I/CSR/MRET family across four seeded complete machine states. Each
gate transition must equal the functional Rust trace and complete state, and
its projected state hash must equal the Python oracle. Strict Rustfmt, Clippy
with warnings denied, rustdoc with warnings denied, dependent consumers, and at
least 80% package line coverage must pass.

The completed package passes six topology/datapath unit tests, six lifecycle
and fault suites, and all 256 Python full-state transitions in complete
trace/state lockstep with the functional oracle. Strict formatting, Clippy,
and rustdoc pass. Package line coverage is 97.96% (769/785).
