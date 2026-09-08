# Layer 07s2: DEC Alpha AXP 21064 gate-level simulator

## Scope

This specification defines the Rust gate-level completion boundary for the
educational Alpha AXP 21064 machine. Spec 07s remains authoritative for the
instruction and lifecycle semantics. The gate implementation must produce the
same complete states, traces, results, and typed failures.

## Exact persistent topology

The machine contains exactly **526,465 D flip-flops**:

| State | Bits |
|---|---:|
| 64 KiB unified memory | 524,288 |
| 32 64-bit GPR storage words | 2,048 |
| 64-bit PC and 64-bit nPC | 128 |
| HALT latch | 1 |
| **Total** | **526,465** |

R31 retains a physical storage word for exact register-file topology, but its
read path is hardwired to zero and architectural writes are discarded.
Installed-program origin and length are validated lifecycle metadata, not
persistent circuit state. `FLIP_FLOP_COUNT` publishes the exact total.

At stable instruction boundaries the implementation may pack each DFF's Q
value. Simulator-owned state transitions must reconstruct and clock the
repository sequential primitive. Reset establishes the same deterministic
stable-Q state as construction.

## Combinational networks

The instruction word is decoded into opcode, register selectors, literal,
function, displacement, jump, and PAL fields. The data path must use repository
gates and ripple-carry adders for:

- 64-bit and sign-extended 32-bit add/subtract;
- AND, BIC, BIS, ORNOT, XOR, and EQV;
- signed and unsigned comparisons and conditional-move predicates;
- 64-bit logical/arithmetic shifts, scaled add/subtract, extract/insert/mask,
  ZAP/ZAPNOT, and byte/word sign extension;
- fixed 64-round partial-product multiplication, including a 128-bit
  accumulator for UMULH;
- effective addresses, branch predicates, jump targets, and PC/nPC updates.

Native integers are permitted at the package boundary for field extraction,
addresses, byte assembly, and stable bit-vector conversion. They are not a
substitute for the listed gate data paths.

## Lifecycle and atomicity

`AlphaGateSimulator` exposes the complete Spec 07s checked boundary: exact
owned state, validated restore, deterministic origin-aware loading, checked
register and byte/word/long/quad access, atomic single steps, complete traces,
and transactional bounded runs. Illegal, truncated, misaligned, halted, and
unsupported-PAL transitions leave the full machine unchanged. R31 is always
zero in externally visible valid state.

## Conformance

Completion requires:

1. exact-topology and reset tests;
2. lifecycle, direct-I/O, trace, workload, and atomic-failure suites;
3. differential equality for all 624 reproducible Python full-state vectors;
4. strict Rust formatting, Clippy, and rustdoc checks; and
5. at least 80% core line coverage.
