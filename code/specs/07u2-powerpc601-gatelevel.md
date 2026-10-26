# Layer 07u2: PowerPC 601 gate-level simulator

## Scope

This specification defines the Rust gate-level completion boundary for the
educational PowerPC 601 machine. Spec 07u remains authoritative for instruction
and lifecycle semantics. The gate implementation produces the same complete
states, traces, results, and typed failures.

This repair is intentionally narrower than a general integer-datapath rewrite.
It makes CR and XER update control authoritative at the gate boundary. It does
not implement overflow-enabled arithmetic, arithmetic record forms, or replace
the shift/count/divide datapaths assigned to their separate backlog cell.

## Primary-source locks

The architectural rules below are transcribed from these exact official NXP
documents. Completion work must reproduce the byte lengths and SHA-256 digests
before using a newly downloaded copy as evidence:

| Document | URL | Bytes | SHA-256 |
|---|---|---:|---|
| *PowerPC 601 RISC Microprocessor User's Manual*, Rev. 1, September 1995 | <https://www.nxp.com/docs/en/user-guide/MPC601UM.pdf> | 6,046,570 | `019e82f1ffb03bf326ac129fccbc5c7b94580e413f6d808c447ff65d319edb79` |
| *PowerPC Microprocessor Family: The Programming Environments*, Rev. 1 | <https://www.nxp.com/docs/en/user-guide/MPCFPE_AD_R1.pdf> | 5,518,986 | `864b0e93149100716f8a9372f64fd1c8da408bb26e79ac14c4d0b3220ee8ac07` |

The 601 manual sections 2.2.4.1 and 3.3 plus its `addc`, `adde`, compare,
`sraw`, `srawi`, and `subfic` instruction entries are primary for this profile.
The Programming Environments sections 2.1.3.1, 2.1.5/Table 2-6, and 4.2.1
cross-check CR0, XER, and modifier semantics. In particular:

- CR0 is `[LT, GT, EQ, SO]`, with SO copied from the final XER[SO];
- compare writes the selected CR field without changing XER;
- carrying arithmetic records carry out/no-borrow in XER[CA];
- arithmetic-right shift records CA only for a negative operand with a one
  shifted out, with a zero count clearing CA; and
- OE controls OV/SO in the architecture, but Spec 07u deliberately excludes
  OE-enabled arithmetic. An excluded modifier is rejected, never ignored.

## Exact persistent topology

The machine contains exactly **525,473 D flip-flops**:

| State | Bits |
|---|---:|
| 64 KiB unified memory | 524,288 |
| 32 32-bit GPR storage words | 1,024 |
| 32-bit LR, CTR, XER, CR, and CIA | 160 |
| HALT latch | 1 |
| **Total** | **525,473** |

Installed-program origin and length are validated lifecycle metadata, not
persistent circuit state. `FLIP_FLOP_COUNT` publishes the exact total. Every
simulator-owned state transition reconstructs and clocks the repository
sequential primitive. Reset establishes the same stable-Q state as construction.

## Combinational networks

The instruction word is decoded into opcode, GPR selectors, immediate,
branch, extended-opcode, record, CR, and SPR fields. Repository gates and
ripple-carry adders implement:

- 32-bit add/subtract, carry, negation, and signed/unsigned comparisons;
- AND, OR, XOR, NAND, NOR, CR masks, and XER/CR updates;
- logical/arithmetic shifts and count-leading-zero;
- effective addresses, branch predicates, CTR/LR/CIA updates;
- a fixed 32-round partial-product multiplier; and
- fixed-round restoring signed and unsigned division.

The conditional-branch predicate is itself a combinational network. Its BO
control bits drive a muxed gate-ALU CTR decrement, a gate-derived nonzero
signal, a selected CR bit/polarity XNOR, independent CTR/condition bypass
muxes, and a final AND wire. Host-language comparisons, equality, XOR, or
short-circuit Boolean operators must not decide whether a branch is taken or
whether CTR is decremented. Conversion of the final one-bit wire to the public
Boolean execution boundary is permitted.

### CR/XER status authority

The instruction decoder exposes `Rc`, `OE`, the three-bit CR field selector,
the eight-bit `mtcrf` field mask, and the relevant opcode lines as one-bit
wires. The status network consumes those wires directly. `std::cmp::Ordering`,
host relational operators, host Boolean short-circuit control, native
shift-and-mask field replacement, and direct post-network CR/XER overwrites
must not decide or reconstruct an architectural status update.

Signed and unsigned comparison produce a one-hot relation bundle
`{lt, gt, eq}` from the existing subtractor wires. With `Z` the subtractor zero
wire, `N` its sign wire, `V` its signed-overflow wire, and `C` its carry wire,
the required equations are:

| Comparison | Inputs | LT | GT | EQ |
|---|---|---:|---:|---:|
| signed | `Z=1` | 0 | 0 | 1 |
| signed | `Z=0, N XOR V=1` | 1 | 0 | 0 |
| signed | `Z=0, N XOR V=0` | 0 | 1 | 0 |
| unsigned | `Z=1` | 0 | 0 | 1 |
| unsigned | `Z=0, C=0` | 1 | 0 | 0 |
| unsigned | `Z=0, C=1` | 0 | 1 | 0 |

Every row is a gate equation, including the one-hot qualification; a host
`Ordering` value is not an intermediate or an output. The CR nibble is then
`LT*8 | GT*4 | EQ*2 | SO`, where SO is stable XER Q bit 31 in the Rust `u32`
representation. A three-to-eight gate decoder selects exactly one CR field and
per-bit muxes preserve all unselected fields. CR0 uses the same relation network
against zero. A write-enable mux selects the old or candidate CR0 for `Rc=0/1`;
`andi.` and `andis.` drive that enable high, while compare always selects its
explicit field.

XER architectural SO/OV/CA bits 0/1/2 correspond to Rust `u32` bits 31/30/29.
The CA network accepts `old_ca`, `new_ca`, and `write_enable` wires and uses a
mux to replace bit 29 only. It must preserve SO, OV, and every opaque XER bit.
`adde` obtains carry-in from stable XER Q bit 29 as a wire. `subfic`, `addc`,
`adde`, `sraw`, and `srawi` are the only modeled instruction paths that assert
the CA write-enable. `mtspr XER` remains the explicit full-word architectural
writer; all other status updates are field-preserving mux networks.

`mtcrf` uses eight field-mask wires and 32 per-bit muxes to select the source or
old CR bit. Native loops may replicate fixed hardware structure, but their
conditionals must not decide architectural field selection.

### Profile modifier classification

The modeled XO-form arithmetic set is `addc`, `subf`, `neg`, `adde`, `mullw`,
`add`, `divwu`, and `divw`. Only `OE=0,Rc=0` is valid. The other three modifier
pairs are classified as `UnknownInstruction { raw, cia }` before result, CA,
XER, CR, destination GPR, or CIA mutation. The nine-bit XO decoder therefore
must not erase OE or Rc before legality is decided. X-form logical/shift record
forms remain admitted and drive the CR0 write-enable wire; this restriction is
specific to XO-form arithmetic.

This cell does not add architectural OV generation. Ordinary modeled
instructions preserve XER[OV] and XER[SO], and a CR update copies the current
SO wire. Implementing `OE=1`, including sticky SO and operation-specific OV,
requires a separate spec-first expansion.

### Bounded `sraw` CA correction

The CA decision is part of status authority even though the shift datapath is a
separate follow-up. A small reusable gate network consumes the 32 source wires
and the full six low count wires for `sraw`:

| Count | Result class owned elsewhere | CA in this cell |
|---:|---|---|
| 0 | unchanged | 0 |
| 1..31 | arithmetic right shift | sign AND OR-reduction of discarded low bits |
| 32..63 | all sign bits | source sign |

`srawi` consumes its five-bit immediate and uses the first two rows only. The
network must cover the `0x80000000` source at counts 31, 32, and 63, which
distinguishes a true six-bit count from clamping to 31. Replacing the shift
movement, leading-zero counter, or restoring divider is explicitly outside this
cell.

Native integers are limited to package-boundary field extraction, memory
addresses, big-endian byte assembly, stable bit-vector conversion, and trace
bookkeeping. They are not substitutes for the listed datapaths.

## Lifecycle and atomicity

`PowerPc601GateSimulator` exposes the complete Spec 07u checked boundary:
exact owned state, validated restore, deterministic origin-aware loading,
checked register and byte/halfword/word access, atomic single steps, complete
traces, and transactional bounded runs. Illegal, truncated, misaligned,
division-by-zero, halted, and step-limit failures leave the full machine
unchanged. Unsupported OE/arithmetic-Rc forms have the same whole-state atomic
boundary. Successful instruction transitions clock all modified persistent
state through DFFs. Snapshots and traces observe CR/XER from stable DFF Q, and
bounded-run rollback restores those exact bits, including opaque XER bits.

## Conformance

Completion requires:

1. exact-topology and existing branch-network tests remain green;
2. signed and unsigned relation truth tables are exhaustive at the control-wire
   level and include `0`, `1`, `0x7fffffff`, `0x80000000`, and `0xffffffff`
   architectural witnesses;
3. every relation, every CR field 0 through 7, SO 0/1, and sentinel unselected
   CR fields prove exact field isolation. CR0 tests cross the same result
   boundaries with SO/OV/CA combinations and write-enable 0/1;
4. the CA mux truth table exhausts old CA, new CA, and write-enable. Instruction
   witnesses cover `subfic` borrow boundaries, `addc`/`adde` carry boundaries
   with incoming CA 0/1, and `sraw`/`srawi` source sign, discarded-bit, and count
   boundaries 0, 1, 31, 32, 33, and 63;
5. `mtcrf` masks `00`, `01`, `80`, `55`, `aa`, and `ff` prove field isolation.
   All status witnesses seed SO, OV, CA, and opaque XER bits independently;
6. each supported XO opcode crosses all four OE/Rc pairs. Only `00` succeeds;
   the other pairs prove exact typed error plus whole-state step and bounded-run
   rollback;
7. an independently calculated producer-to-consumer test executes compare or a
   record form and then feeds the resulting CR bit into the already authoritative
   branch network. Expected status values must not call functional or gate
   production helpers;
8. all 239 reproducible Python full-state vectors remain green. The legacy
   `sraw` seed with source `0x80000000` and count 63 must update its stale
   clamp-to-31 CA hash; the other 238 hashes remain unchanged. New status-edge
   tests are primary-source-derived Rust witnesses rather than copies of the
   legacy Python implementation;
9. a `syn`-parsed hostile structural suite verifies the one external,
   unattributed status-module binding and the complete decoder-to-relation-to-
   mux-to-commit chain. Compile-valid adversarial fixtures must reject host
   comparisons or Boolean decisions, direct mask/shift updates, dead gate
   anchors, helper shadowing, post-helper CR/XER overwrite, functional-oracle
   substitution, inline/path/cfg/duplicate/macro-generated module rerouting,
   and comment/string/character-literal scanner confusers; and
10. lifecycle, direct-I/O, trace, workload, differential, strict Rust formatting,
    Clippy, rustdoc, affected BUILD, lessons validation, and at least 80% real
    package line coverage pass.

The completed conditional-branch repair passes three gate/topology suites,
three lifecycle suites, the aggregate 239-vector differential, strict checks,
and 94.58% package line coverage (838/886).

The completed CR/XER authority cell passes 23 gate-package and 13
functional-package test functions, the corrected 239-vector differential,
strict formatting/Clippy/rustdoc, affected builds, lessons validation, and
95.15% gate-package line coverage (962/1,011). The functional package remains at
90.25% line coverage (639/708).

## Independent specification review

An independent exact-head review passed at
`a315f67feb795253f4074419fb6998077336fca5`. The reviewer reproduced both NXP
document byte counts and SHA-256 locks, verified the existing-profile OE/Rc
rejection and six-bit `sraw` CA boundary against those sources, confirmed the
gate/mux authority and XER preservation contracts, and found the conformance
matrix complete and bounded away from the separate shift/count/divide cell.
