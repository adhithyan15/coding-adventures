# Layer 07as2 — IBM System/370 Gate-Level Simulator

## Status

This specification defines the educational gate-level partner to Layer 07as.
It preserves the complete checked System/370 functional boundary: the modeled
System/360 problem-state integer, branch, character, and hexadecimal-floating
surface plus `MVCL`, `CLCL`, `CLM`, `STCM`, and `ICM`.

**Implementation is complete locally and awaiting independent exact-head
review.** Pull request #18363 merged into `origin/main`
as `08d9140e6c0c60f7e74f1a5ad569e9c7cd68e364`, this specification branch is
reconciled with that exact functional oracle, and all fourteen functional
release tests pass. The corrected oracle clears the ignored high bytes of
both `MVCL` address registers on destructive-overlap completion while
condition code 3 suppresses storage access, movement, length progress, and
low-24-bit address progress, exactly as this reviewed contract requires.

The pinned architectural source is IBM, *IBM System/370 Principles of
Operation*, seventh edition, March 1980, form GA22-7000-6. The bounded
extension rules are in Chapter 7: `CLM` on page 7-12, `CLCL` on page 7-13,
`ICM` on page 7-17, `MVCL` on pages 7-21 through 7-22, and `STCM` on page
7-31. The public scan is
<https://www.bitsavers.org/pdf/ibm/370/princOps/GA22-7000-6_IBM_System_370_Principles_of_Operation_7th_ed_198003.pdf>.
The reviewed PDF is pinned by SHA-256
`db4ecf8f053e3fbc0999ca3832bc16442522869bb1e59f8de7b6e405ae738055`.
The gate model represents the programmer-visible architecture, not the
physical implementation, timing, or packaging of a particular 370 processor.
Dynamic address translation, channels, storage protection, interruptions,
privileged control, multiprocessing, decimal arithmetic, and additional
floating-point facilities remain outside this bounded CPU cell.

## Package and oracle boundary

The Rust package is `ibm-system370-gatelevel`. Its behavioral oracle is
`ibm-system370-simulator`; its gate dependencies are the repository
`logic-gates` and `arithmetic` packages. The completed
`ibm-system360-gatelevel` package is the required gate-backed execution engine
for inherited instructions and the sole stable-Q store for all architectural
bits shared by the two machines. Exactly one such engine exists in a
System/370 instance. The System/370 extension decoder and datapaths settle
next-D values and clock that same bank; they do not own a second wrapper bank.
No host `System370State`, packed byte array, shadow register file, candidate
simulator, or post-transition restore may duplicate or overwrite that
authority. The functional oracle may validate a settled transition, trace, or
fault, but must never author a transition that is clocked into architectural
state. An immutable entry-Q backup may exist solely to restore the exact entry
bits on failure; execution, trace construction, successful commit, and public
observation may not read it as an architectural candidate.

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
or fault behavior. Origin and length metadata may remain host scalars, but
their reset/load/restore/rollback routing is exact-locked and transactional;
they cannot select architectural values or conceal Q divergence. Public
observation reconstructs the architectural portion solely from stable Q and
then appends the validated metadata.

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
destructive-overlap direction before any storage access or movement. The
overlap wire selects condition code 3, preserves both length registers and
storage, preserves the low 24 address bits, and clears the ignored high byte of
both address registers through the normal 24-bit address-result path.
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

The public checked API matches Layer 07as in operation while making every
allocation-bearing boundary explicitly fallible: checked construction,
reset, load, immutable state, validated restore, direct register/storage
access, `step`, transactional bounded `run`, and transactional bounded
`execute`. In particular, `new`, `get_state`, and `read_memory` return a typed
result so fixed storage, complete-state snapshots, and requested byte vectors
cannot abort on allocation pressure. Big-endian byte order, 24-bit addresses,
halfword-aligned fetch, exact installed-program bounds, and the `SVC 0`
teaching halt sentinel are unchanged.

Every malformed, unsupported, misaligned, out-of-range, arithmetic, divide,
register-pair, supervisor-call, post-halt, and step-limit failure preserves
every architectural bit and all lifecycle metadata. Oracle divergence is a
typed specification fault and also preserves the entry state.

The gate API additionally exposes a stable typed `AllocationFailed` error.
Every real fallible allocation in production has a named deterministic
test-only injection seam immediately before the same call site. The required
boundary list is maintained beside the implementation and includes, when
present: initial memory construction; public complete-state observation;
public memory reads; entry-Q and metadata snapshots for `step`, `run`, and
`execute`; raw-instruction capture; mnemonic and description construction;
extension-oracle state and trace creation; inherited System/360 entry,
oracle, and independent gate-candidate state plus inherited trace creation;
memory-write staging; trace-vector reservation and trace retention; and
complete final-state construction. Load and restore perform no duplicate
image or memory staging allocation. Adding, removing, eliminating, or moving
a production allocation changes that inventory and its exact-lock test. A
seam that exists only in test code, or that fires after an infallible
allocation already succeeded, is not evidence.

All potentially failing allocations complete before the corresponding DFF
commit. Tests inject each boundary at entry and, for both `run` and `execute`,
after at least one instruction has successfully settled and been retained.
Every injection returns `AllocationFailed` and restores the exact entry Q,
halt latch, origin, and installed length; it retains neither a partial trace
nor a partial final state. For ordinary lifecycle and architectural failures,
the gate and corrected functional oracle must return the same typed outcome
and complete state. Allocation injection is intentionally gate-only until the
functional package exposes an equivalent seam: its parity witness restores an
equivalent functional oracle to the same entry snapshot and proves the
unchanged gate state is bit-for-bit identical, without claiming the
functional oracle itself produced `AllocationFailed`.

Fault priority is part of the public contract. `Halted` precedes fetch;
invalid or misaligned PC and fetch-outside-program precede first-byte decode;
the independently classified first byte then selects either immediate
`UnknownOpcode` for an unrecognized/reserved byte or an exact legal width;
`TruncatedInstruction` precedes operand-field and semantic validation only for
a recognized opcode whose installed suffix is short. Register-pair,
effective-address, complete storage-range, overlap, arithmetic, and oracle
checks all settle before any clock edge. Allocation failure at any later
precommit boundary outranks commit and rolls back the whole public operation.

## Structural and differential evidence

Completion requires all of the following:

1. Exact topology, reset/load/restore/direct-write, observer-divergence, halt,
   and fault-atomicity tests prove stable-Q storage authority.
2. An independent literal table covers all 256 first bytes. Each entry records
   legal versus unknown/reserved, exact width 2/4/6 when legal, and inherited
   versus System/370-extension route. It is not generated from, imported from,
   or indexed through the production decoder. Every byte is exercised with
   each installed-byte count from one through six (1,536 cases), producing the
   exact transport result: immediate `UnknownOpcode`, width-specific
   `TruncatedInstruction`, or continued semantic execution/fault once the
   complete legal width is installed. Every failure preserves the exact entry
   state. The five extension bytes and every immediately adjacent
   reserved/legal neighbor receive directed single-bit toggles.
   Full-state/trace/error differentials cover every inherited System/360
   family through the embedded gate engine.
3. Extension tests exhaust all sixteen masks, all condition-code outcomes,
   zero-mask exception rules, length relations, padding, overlap directions,
   pair validation, address boundaries, and multi-byte register progress.
4. Independent truth tables cover extension one-hot decode, byte comparisons,
   24-bit add/subtract and range relations, mask prefix counts, byte muxes,
   register merges, and condition-code selection.
5. Parsed-token and syntax locks cover the complete, live route from first-byte
   decode through width, legality, inherited/extension family and operation
   selection, address/range/pair/overlap validation, datapath and
   condition-code muxes, trace construction, allocation preflight, stable-Q
   clock enable, commit, and rollback. They also lock the sole embedded
   System/360 engine instance, its visibility and exact top-level module route,
   every Q observer, and origin/length metadata routing. A changed helper body,
   helper indirection, post-dispatch assignment, or post-commit restore must
   fail the lock rather than moving authority outside the reviewed region.
6. Every hostile structural fixture writes the complete mutated production
   source and manifest to a temporary crate, proves it still passes
   `cargo check`, and only then requires the structural validator to reject
   it. The matrix includes direct Q or packed-memory writes; direct host mirror
   reads/writes; removed or constant clock enables; functional-oracle
   substitution; native opcode matches, arithmetic, ordering, or Boolean
   architectural decisions; helper indirection; duplicate, dead, inactive, or
   expectation-shadow gate networks; post-decode, post-datapath, post-trace,
   post-final-state, and post-commit overwrites; and host-authored trace or
   final-state candidates. Module-routing fixtures include inline modules,
   aliases, changed visibility/attributes, macro-created declarations, and
   macro-discarded declaration confusers. Strings, comments, byte/character
   literals, dead anchors, and similarly named identifiers never satisfy a
   source or route lock.
7. Exact raw bytes, width, conventional mnemonic, description, complete state,
   and typed-error parity hold before any transition is committed.
8. The package is enrolled in the Rust workspace and BUILD graph, documented
   in README/changelog and the expansion backlog, passes Rustfmt, strict
   Clippy, strict rustdoc, release tests, affected builds, and lesson
   validation, and exceeds 80% package line coverage with a 95% target.

This repaired specification received independent review before implementation.
The completed package has 21 release integration tests, including the literal
1,536-case transport matrix, functional differentials, allocation rollback,
gate truth tables, and compiler-checked complete hostile crates. Strict checks
and 95.27% package-source line coverage (825/866) pass. The current-source
affected closure is seven packages and all seven BUILD recipes pass, but the
strict repository planner currently stops first on stale `BUILD-EXEMPTIONS`
entries inherited unchanged from `origin/main`; this branch does not claim
that unrelated orphan
metadata validator as green. Publication remains blocked on a different
reviewer's exact-clean-head PASS.
