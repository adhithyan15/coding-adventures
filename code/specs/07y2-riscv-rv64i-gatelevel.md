# Layer 07y2 — RISC-V RV64I + M Gate-Level Simulator

## Scope

This specification is the gate-level partner to Spec 07y and the completed
Rust `riscv-rv64i-simulator::Rv64ISimulator` oracle. It implements the same
complete RV64I base integer and M multiply/divide surface, including the
repository's distinct all-zero halt sentinel.

The model operates at instruction boundaries. It does not reproduce analog
timing, a physical pipeline, caches, compressed or atomic instructions,
floating point, privileged state, an MMU, interrupts, package pins, or
transistor placement.

## Persistent topology

The machine contains exactly **526,401 D flip-flops**:

| Bank | Width/count | DFFs |
|---|---:|---:|
| Little-endian memory | 65,536 x 8 | 524,288 |
| X0-X31 GPRs | 32 x 64 | 2,048 |
| Program counter | 64 | 64 |
| Halt | 1 | 1 |
| **Total** | | **526,401** |

X0 occupies physical state but reads zero, discards writes, and must be zero in
restored snapshots. Installed program origin and length are checked lifecycle
metadata rather than architectural hardware state. Reset, restore, program
installation, direct writes, instruction transitions, memory stores, and halt
transitions clock repository DFF primitives.

## Combinational execution

The crate owns an independent strict decoder for LUI, AUIPC, JAL, JALR, all six
branches, all documented byte/halfword/word/doubleword loads and stores,
immediate and register ALU families, RV64 word-result families, the full M and
M-word families, FENCE, FENCE.I, ECALL, EBREAK, and the zero sentinel. Reserved
opcode, funct, shift, fence, and SYSTEM combinations fail closed.

Repository Boolean gates implement opcode and funct predicates, muxes, bitwise
ALU operations, equality, sign/zero extension, and selection. Ripple-carry
networks implement PC sequencing, targets, effective addresses, ADD/SUB, and
comparison. Fixed gate barrel networks implement 64-bit and word logical and
arithmetic shifts. Fixed-width shift-add multiplication and restoring division
networks implement all signed, unsigned, high-half, division-by-zero, and
signed-overflow semantics. Host integers may assemble traces and index checked
memory; they do not replace architectural datapath results.

## Lifecycle and faults

The public `Rv64IState`, `Rv64IError`, `Rv64IStepTrace`, and
`Rv64IExecutionResult` contract is shared with `riscv-rv64i-simulator`.
`load_checked`, `load_at_checked`, `restore`, register access, and byte access
are validated and typed. A faulting step preserves every entry-state bit. A
bounded run either returns complete traces and final state or rolls the entire
machine back on any fault or step-limit exhaustion.

Fetch is restricted to the installed aligned range. Multi-byte data accesses
require natural alignment, and all accesses are bounded by the exact 64 KiB
memory. Unsupported encodings report typed faults. ECALL, EBREAK, and the zero
sentinel halt with distinct trace mnemonics; stepping an already halted machine
is a typed error and a bounded run on it is a successful zero-step result.

## Conformance

Completion requires exact topology and lifecycle suites, strict malformed
decode and fault tests, and the reproducible 364-vector Python corpus spanning
every RV64I+M family and arithmetic edge seeds. Each gate transition must equal
the functional Rust trace and complete state, and its projected state hash must
equal the Python oracle. Strict Rustfmt, Clippy with warnings denied, rustdoc
with warnings denied, the functional consumer, the declared BUILD target, and
at least 80% package line coverage must pass.

The completed package passes three topology/datapath unit tests, six lifecycle
and fault suites, and all 364 Python full-state transitions in complete
trace/state lockstep with the functional oracle. Strict formatting, Clippy,
and rustdoc pass. Package line coverage is 98.76% (717/726).
