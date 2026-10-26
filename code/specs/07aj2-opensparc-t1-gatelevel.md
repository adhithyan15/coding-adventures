# Layer 07aj2 — OpenSPARC T1 Gate-Level Simulator

## Purpose and evidence boundary

`opensparc-t1-gatelevel` is the architectural gate-level companion to the
checked `opensparc-t1-simulator`. The normative architecture remains Oracle's
*UltraSPARC Architecture 2005* and *UltraSPARC T1 Supplement*. Oracle's public
OpenSPARC T1 synthesizable Verilog is the implementation reference for the
register-file, window, decode, integer-execution, branch, trap, and load/store
networks represented here.

This bounded cell models one T1 strand's scalar integer architectural state.
Every persistent bit is stored in repository D flip-flops and every transition
is reconstructed with repository logic gates. It does not claim cycle accuracy
for the six-stage pipeline, four-strand scheduler, shared floating-point unit,
caches, TLBs, MMU, crossbar, L2 banks, DRAM controllers, JBus, SSI, or other
strands. Those shared structures belong to later timing and SoC/interconnect
cells. The functional package supplies public value types and is an oracle in
tests only; production gate transitions never call it.

## Persistent topology

Every persistent architectural bit changes through
`logic_gates::sequential::register`. Memory may maintain a derived host-side
set of nonzero byte addresses so reset remains proportional to changed state;
that index is reconstructible and is not architectural storage.

| Component | D flip-flops |
|---|---:|
| 65,536-byte memory | 524,288 |
| 160 physical 64-bit registers | 10,240 |
| PC and nPC | 128 |
| `CCR.icc` and `CCR.xcc` | 8 |
| Y and ASI | 40 |
| PSTATE fields | 3 |
| GL and CWP | 5 |
| CANSAVE, CANRESTORE, CLEANWIN, and OTHERWIN | 12 |
| WSTATE | 6 |
| Halt and delay-slot latches | 2 |
| Installed-program origin | 64 |
| Installed-program length, including 65,536 | 17 |
| **Total** | **534,813** |

At six primitive gates per master/slave DFF, persistent storage alone
represents 3,208,878 gate equivalents. Transient decode, selection, comparison,
adder, shifter, multiplier, divider, condition-code, register-window, control,
and memory networks are additional simulated combinational circuitry.

## Gate transition engine

- Format bits and the `op2`/`op3` fields feed gate-built decoders. Implemented,
  unsupported, privileged, and malformed families are classified locally.
- Ripple adders, two's-complement subtraction, logic gates, barrel-selection
  networks, comparisons, extension, multiplication, and checked division
  produce integer results, Y, and both 32-bit `icc` and 64-bit `xcc` flags.
- Gate selectors map each logical register through GL and CWP to the exact
  physical global, out, local, or aliased in-register bank. SAVE and RESTORE
  update every window counter through the same checked DFF paths.
- PC/nPC selection models delayed branches, calls, indirect jumps, taken and
  untaken paths, annul, link writes, and the active-delay-slot restriction.
- Big-endian byte, half-word, word, and double-word memory operations derive
  alignment, range, width, extension, and byte write-enable signals before any
  DFF is clocked.
- `ta 0` clocks the deterministic halt latch. Other traps, unsupported address
  spaces, and privileged operations remain typed atomic faults at this cell's
  boundary.

The engine may assemble candidate values while its gate networks are being
evaluated, but the machine commits only through DFF banks. Complete-state and
trace differential checks make a missing gate write, incorrect decode, or
unclocked persistent bit observable.

## Checked transport and lifecycle

The machine preserves the functional simulator's complete big-endian
contract: nonempty word-aligned images, exact installed origin and length,
full 64 KiB memory, atomic load/restore/step, typed fetch/data/decode/window/
privilege/division faults, transactional bounded execution, and complete
retained traces. Restore validates fixed memory size, hardwired zero registers,
window invariants, state-field ranges, and the installed image before cloning
or clocking caller state. Failed transitions leave every DFF and derived cache
unchanged.

The retained-trace limit is derived from a fixed byte budget and the complete
state size. All nested state buffers are allocated fallibly before execution
and then reused for rollback, traces, and final state. Post-halt steps,
out-of-image fetches, malformed restored state, misalignment, range errors,
unsupported encodings, delay-slot violations, and divide faults remain typed
and atomic.

## Completion criteria

1. Pin the topology to exactly 534,813 DFFs.
2. Compare every primary format and each implemented op2/op3 arithmetic,
   logical, shift, multiply/divide, branch, control-transfer, window, trap, and
   memory subfamily against the checked functional oracle.
3. Compare exact raw bytes/word, mnemonic, PC/nPC transition, register and
   memory effects, and complete before/after state for every valid witness.
4. Cover both condition-code widths, all branch outcomes and annul cases,
   links, delay slots, register aliases, window counters and faults, every
   memory width and sign, alignment/range checks, reset, restore,
   arbitrary-origin loading, rollback, halt, and trace bounds.
5. Pass release tests, package-scoped formatting, strict Clippy, rustdoc,
   package BUILD, lessons validation, affected-package checks, independent
   security review, and more than 90% line coverage.
