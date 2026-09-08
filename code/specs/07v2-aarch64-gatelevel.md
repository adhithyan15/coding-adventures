# Layer 07v2 — AArch64 Gate-Level Simulator

## Scope

This specification is the gate-level partner to Spec 07v and the completed
Rust `aarch64-simulator::AArch64Simulator` oracle. It implements the same
complete documented integer surface and the repository's zero-word halt
sentinel at instruction boundaries.

The model does not reproduce analog timing, a physical pipeline, caches,
floating point or SIMD, privileged state, an MMU, interrupts, package pins, or
transistor placement.

## Persistent topology

The machine contains exactly **526,469 D flip-flops**:

| Bank | Width/count | DFFs |
|---|---:|---:|
| Big-endian memory | 65,536 x 8 | 524,288 |
| X0-X31 GPR slots | 32 x 64 | 2,048 |
| Stack pointer | 64 | 64 |
| Program counter | 64 | 64 |
| NZCV | 4 | 4 |
| Halt | 1 | 1 |
| **Total** | | **526,469** |

The XZR slot occupies physical state but reads zero, discards writes, and must
be zero in restored snapshots. Installed program origin and length are checked
lifecycle metadata rather than architectural hardware state. Reset, restore,
program installation, direct writes, instruction transitions, memory stores,
flag updates, and halt transitions clock repository DFF primitives.

## Combinational execution

The crate owns an independent strict decoder for the complete Spec 07v integer
surface: immediate and register branches, conditional/compare/test branches,
add/subtract, logical and move-wide operations, unsigned-offset loads and
stores with sign-extension forms, divide and variable shifts, bit reverse and
byte reverse, leading-zero count, multiply-add/subtract and high-half multiply,
conditional select, SVC, NOP, and halt. Reserved condition, opcode, field,
shift, bitmask, size, and operation combinations fail closed.

Repository Boolean gates implement decode predicates, muxes, bitwise ALU
operations, equality, extension, and selection. Ripple-carry networks implement
PC sequencing, targets, effective addresses, ADD/SUB, comparison, and NZCV.
Fixed gate barrel networks implement 32/64-bit shifts, rotates, and replicated
logical bitmasks. Fixed-width shift-add multiplication and restoring division
networks implement low and high products, signed/unsigned division, division by
zero, and signed overflow. Gate networks also assemble big-endian memory and
implement bit/byte reversal and leading-zero count. Host integers may assemble
traces and index checked memory; they do not replace architectural datapath
results.

## Lifecycle and faults

The public `AArch64State`, `AArch64Error`, `AArch64StepTrace`, and
`AArch64ExecutionResult` contract is shared with `aarch64-simulator`.
`load_checked`, `load_at_checked`, `restore`, register access, stack-pointer and
NZCV access, and byte access are validated and typed. A faulting step preserves
every entry-state bit. A bounded run either returns complete traces and final
state or rolls the entire machine back on any fault or step-limit exhaustion.

Fetch is restricted to the installed aligned range. Multi-byte data accesses
require natural alignment and all accesses are bounded by the exact 64 KiB
memory. Unsupported encodings report typed faults. The zero sentinel and SVC
halt with distinct trace mnemonics; stepping an already halted machine is a
typed error and a bounded run on it is a successful zero-step result.

## Conformance

Completion requires exact topology and lifecycle suites, strict malformed
decode and fault tests, functional lockstep over every instruction family, and
the reproducible 836-vector Python corpus. Each gate transition must equal the
functional Rust trace and complete state, and its projected state hash must
equal the Python oracle. Strict Rustfmt, Clippy with warnings denied, rustdoc
with warnings denied, the functional and gate BUILD consumers, and at least
80% package line coverage must pass.

The completed package passes seven topology/datapath unit tests, six lifecycle
and fault suites, and all 836 Python full-state transitions in complete
trace/state lockstep with the functional oracle. The historical Python
AArch64 gate package's 256 tests also pass. Strict formatting, Clippy, and
rustdoc pass. Package line coverage is 97.45% (763/783).
