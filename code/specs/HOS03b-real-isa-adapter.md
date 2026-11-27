# HOS03b — Existing ISA simulator adapter probe

## Scope

This probe connects the existing TypeScript RV32I simulator package to the
HOS03a bus and board without changing the RISC-V or generic CPU packages. It
uses the repository's `RiscVSimulator`, encoder helpers, register file, and
pipeline traces. The guest is authentic RV32I instruction encoding; the two
boards and console remain synthetic. This demonstrates an adapter seam, not
full RV32 platform, privilege, or device compatibility.

## Integration seam and deliberate limit

The repository CPU exposes its `Memory` as a replaceable object, and the
RISC-V high-level simulator exposes both that CPU and one-instruction `step()`.
The adapter subclasses the repository `Memory` and routes byte/word accesses
through the HOS03a bus. It loads the program into board RAM during composition,
then sets the public CPU program counter to the mapped start address. It does
not use `RiscVSimulator.run(program)`, whose loader assumes address zero and
would collide with the board's read-only configuration ROM.

The current CPU fetches a 32-bit instruction through the same `readWord`
method that the executor uses for a guest `lw`. The adapter identifies
instruction fetch when the word address equals the current program counter;
other word reads are data accesses. Thus the seam is unambiguous for this
probe's instruction subset and guest, which does not issue `lw` from its own
current PC. A general adapter needs an explicit fetch callback or access-kind
parameter in the CPU memory interface.

The guest uses byte loads and byte stores. The bus can report an unmapped or
denied access, but a word store in the repository RISC-V executor is currently
implemented as multiple byte writes. A word store crossing a mapping boundary
could apply earlier bytes before a later byte faults; this probe does not use
such an operation or claim transaction-wide store atomicity. Fixing that
requires a bus transaction seam and is outside this adapter.

## Guest and observable behavior

The portable fixture stores the 32-bit instruction words for a small RV32I
program. It loads the console pointer from the board's little-endian
configuration bytes using `lbu`, combines the high byte with `slli` and `add`,
then uses `addi` and `sb` to write `OK\n`. `ecall` halts under the existing
simulator's default zero trap vector. The program and instruction simulator
are identical on both boards; only the board's console address and RAM end
differ.

The adapter runs through a fixed instruction budget and records the original
RISC-V pipeline traces plus the data-only console effects emitted by the
board's bus-backed memory. Fixture checks compare instruction words,
mnemonics, register results, final PC/halt state, output bytes, and mapped
write addresses. The package graph installs transitive TypeScript dependencies
leaf-to-root: transistors, logic gates, arithmetic, generic CPU, RISC-V, then
the HOS architecture package. These local dependencies do not change the
repository-wide CPU or VOS implementation.

## Fixture build inputs

`code/specs/fixtures/os-machine-v1/riscv.json` is the portable RV32I program
and expected output. `code/specs/fixtures/os-machine-v1/verify_riscv.test.ts`
is the external build-time verifier; it reads that JSON and runs the adapter
through the public `RiscVSimulator` API. Both files are exact repository-input
paths for the existing `typescript/os-architecture-simulator` selector and
fallback hash row, alongside the four HOS03a1 profile and synthetic-board
inputs. The fixture reader remains outside the package, whose capabilities
manifest stays empty.
