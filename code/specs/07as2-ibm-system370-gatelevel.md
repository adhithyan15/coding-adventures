# Layer 07as2 — IBM System/370 Gate-Level Simulator

## Status

This specification defines the educational gate-level partner to Layer 07as.
It preserves the complete checked System/370 functional boundary: the modeled
System/360 problem-state integer, branch, character, and hexadecimal-floating
surface plus `MVCL`, `CLCL`, `CLM`, `STCM`, and `ICM`.

The architectural source remains IBM's *System/370 Principles of Operation*,
form GA22-7000. The gate model represents the programmer-visible architecture,
not the physical implementation, timing, or packaging of a particular 370
processor. Dynamic address translation, channels, storage protection,
interruptions, privileged control, multiprocessing, decimal arithmetic, and
additional floating-point facilities remain outside this bounded CPU cell.

## Package and oracle boundary

The Rust package is `ibm-system370-gatelevel`. Its behavioral oracle is
`ibm-system370-simulator`; its gate dependencies are the repository
`logic-gates` and `arithmetic` packages. The completed
`ibm-system360-gatelevel` package may be embedded as the gate-backed execution
engine for inherited instructions. The functional oracle may validate a
settled transition, trace, or fault, but must never author a transition that
is clocked into architectural state.

The public API owns System/370 state, traces, results, and errors. System/360
types remain an implementation detail and never appear in public signatures.

## Persistent topology

Every programmer-visible bit is held by a simulated D flip-flop:

| Component | Flip-flops |
|---|---:|
| 1 MiB byte-addressed main storage | 8,388,608 |
| 16 × 32-bit general registers | 512 |
| Four 64-bit floating registers | 256 |
| 24-bit instruction address | 24 |
| Two-bit condition code | 2 |
| Repository halt latch | 1 |
| **Total** | **8,389,403** |

`flip_flop_count()` reports exactly 8,389,403. Installed program origin and
length are checked lifecycle metadata, not architectural storage bits.
`gate_count()` is a documented educational estimate built from the storage
cost plus the inherited and extension combinational networks; it is not a
transistor, module, or SLT-card count.

Reset, load, complete-state restore, direct register and storage writes,
successful instruction transitions, condition-code updates, and halt all
clock through repository sequential primitives. Public state is reconstructed
from stable Q outputs. No host mirror may decide execution, snapshot, restore,
or fault behavior.

## Gate-authoritative execution

Fetch and checked lifecycle use the exact Layer 07as transport contract.
Host control may sequence fixed combinational stages, retain installed-range
metadata, index storage after a gate-derived address settles, and format
traces. Architecturally visible opcode selection, address and length updates,
byte selection, comparison, overlap qualification, condition-code selection,
and register merging must flow through explicit gate and mux networks.

The extension decoder produces independent one-hot lines for opcodes `0E`,
`0F`, `BD`, `BE`, and `BF`. Inherited opcodes route only to the reviewed
System/360 gate engine. Unknown and reserved classifications fail closed. A
functional-oracle call may compare the independently computed result only
after the gate transition has settled; equality with the oracle is required
before a DFF clock edge is committed.

### Long-field networks

`MVCL` and `CLCL` use gate-qualified even-register-pair selection, 24-bit
address and length extraction, fixed-width increment/decrement networks, and
byte muxes. Host loops may sequence one byte at a time, but their continuation,
source-versus-pad selection, first-unequal termination, and final
condition-code wires must be derived from reductions and comparisons over the
settled gate vectors.

For `MVCL`, a fixed-width unsigned range/ordering network detects the forbidden
destructive-overlap direction before any state changes. The overlap wire
selects condition code 3 and preserves both register pairs and storage.
Otherwise the initial length relation selects condition code 0, 1, or 2;
source exhaustion selects the pad byte through a mux; and every transferred
byte clocks the exact Layer 07as register progress.

For `CLCL`, gate byte comparators select the first unequal relation. Exhausted
operands select the second pair's pad byte through explicit muxes. The
unexamined-suffix registers and condition code are the settled results of the
same sequenced network.

### Masked-character networks

`CLM`, `STCM`, and `ICM` use the four instruction mask bits directly as byte
enable wires from most-significant to least-significant register byte. A
gate prefix-count/address network packs selected storage bytes contiguously;
unselected bytes consume no storage. Gate muxes merge `ICM` bytes into the
register and select `STCM` bytes from it.

`CLM` derives low/equal/high from the first enabled unequal byte. `ICM`
derives zero, leftmost inserted sign, and any-one reductions for condition
codes 0, 1, and 2. A zero mask still validates one designated storage byte for
`CLM` and `ICM`, but performs no transfer. A zero-mask `STCM` performs no
storage access and preserves condition code exactly.

## Lifecycle and fault atomicity

The public checked API matches Layer 07as: reset, load, immutable state,
validated restore, direct register/storage access, `step`, transactional
bounded `run`, and transactional bounded `execute`. Big-endian byte order,
24-bit addresses, halfword-aligned fetch, exact installed-program bounds, and
the `SVC 0` teaching halt sentinel are unchanged.

Every malformed, unsupported, misaligned, out-of-range, arithmetic, divide,
register-pair, supervisor-call, post-halt, and step-limit failure preserves
every architectural bit and all lifecycle metadata. Oracle divergence is a
typed specification fault and also preserves the entry state.

## Structural and differential evidence

Completion requires all of the following:

1. Exact topology, reset/load/restore/direct-write, observer-divergence, halt,
   and fault-atomicity tests prove stable-Q storage authority.
2. A first-byte classification corpus and full-state/trace/error differential
   cover every inherited System/360 family through the embedded gate engine.
3. Extension tests exhaust all sixteen masks, all condition-code outcomes,
   zero-mask exception rules, length relations, padding, overlap directions,
   pair validation, address boundaries, and multi-byte register progress.
4. Independent truth tables cover extension one-hot decode, byte comparisons,
   24-bit add/subtract and range relations, mask prefix counts, byte muxes,
   register merges, and condition-code selection.
5. Parsed-syntax structural tests reject functional-oracle substitution,
   native arithmetic/comparison decisions in the reviewed extension path,
   dead gate networks, duplicate or aliased execution modules, post-network
   overwrites, and expectation shadowing.
6. Exact raw bytes, width, conventional mnemonic, description, complete state,
   and typed-error parity hold before any transition is committed.
7. The package is enrolled in the Rust workspace and BUILD graph, documented
   in README/changelog and the expansion backlog, passes Rustfmt, strict
   Clippy, strict rustdoc, release tests, affected builds, and lesson
   validation, and exceeds 80% package line coverage with a 95% target.

This specification is committed and independently reviewed before tests or
implementation begin.
