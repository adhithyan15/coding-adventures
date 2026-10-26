# Layer 07ai2 — OpenRISC 1000 Gate-Level Simulator

## Purpose and evidence boundary

`openrisc1000-gatelevel` is the architectural gate-level companion to the
checked `openrisc1000-simulator`. The normative architectural source remains
the OpenRISC 1000 Architecture Manual 1.4. The public `openrisc/mor1kx` RTL is
the implementation reference for the register file, decode, arithmetic,
branch, memory, special-register, multiply/divide, multiply-accumulate, and
atomic-control networks represented here.

This layer models every persistent architectural bit with repository D flip-
flops and reconstructs each transition through repository logic gates and
arithmetic networks. It is intentionally an educational architectural model:
it does not claim mor1kx pipeline timing, cache/MMU behavior, Wishbone cycle
timing, FPGA primitive placement, or transistor-accurate physical layout.
The functional package supplies shared public value types and is an oracle in
tests only; production gate transitions never call it.

## Persistent topology

Every persistent bit changes through `logic_gates::sequential::register`.
Memory may keep a derived host-side set of nonzero byte addresses so reset can
remain proportional to changed state; that index is reconstructible and is not
architectural storage.

| Component | D flip-flops |
|---|---:|
| 65,536-byte memory | 524,288 |
| 32 general-purpose 32-bit registers | 1,024 |
| PC, previous PC, SR, EPCR0, EEAR0, ESR0, MACLO, MACHI | 256 |
| Atomic reservation address plus valid latch | 33 |
| Halt latch | 1 |
| Installed-program origin | 32 |
| Installed-program length, including 65,536 | 17 |
| **Total** | **525,651** |

At six primitive gates per master/slave DFF, persistent storage alone
represents 3,153,906 gate equivalents. Transient decode, selection, comparison,
adder, shifter, multiplier, divider, SPR, exception, and memory-control logic is
additional simulated combinational circuitry.

## Gate transition engine

- Six input opcode bits feed a gate-built primary decoder. Implemented and
  unsupported primary families are classified locally, independent of the
  functional package.
- ALU selection is reconstructed from the eleven function bits. Ripple adders,
  two's-complement subtraction, bitwise gates, barrel-selection networks,
  comparisons, extension, bit search, multiplication, and checked division
  produce the scalar integer results and SR flags.
- Branch and jump control selects sequential or target PC through mux networks;
  link writes, no-delay-slot behavior, and exception entry use the same
  DFF-backed state paths.
- Byte, half-word, and word memory operations derive alignment, range, width,
  sign extension, reservation overlap, and write-enable signals before any
  DFF is clocked. Every committed byte change is accompanied by a gate-derived
  write enable.
- SPR decode and privilege checks gate reads and writes for the documented CPU,
  GPR, and MAC register groups. MAC and atomic-reservation state are ordinary
  persistent DFF banks.
- `l.nop 1` clocks the deterministic halt latch; other synchronization and NOP
  forms retain architectural state except for the normal PC transition.

The transition engine may construct a candidate value-level state while its
gate networks are evaluated, but the machine commits only through its DFF
banks. Complete trace and state differential checks make a missing gate write,
incorrect decode, or unclocked persistent bit observable.

## Checked transport and lifecycle

The machine preserves the functional simulator's complete big-endian contract:
nonempty word-aligned images, exact installed origin and length, full 64 KiB
memory, atomic load/restore/step, typed fetch/data/decode/privilege/division
faults, transactional bounded execution, and complete retained traces. Restore
validates fixed memory size and image bounds before cloning or clocking caller
state. Failed transitions leave every DFF and derived cache unchanged.

The retained-trace limit is derived from a fixed byte budget and the complete
state size. Allocation is attempted before execution. Post-halt steps,
out-of-image fetches, malformed restored state, misalignment, range errors,
unsupported encodings, and divide faults remain typed and atomic.

## Completion criteria

1. Pin the topology to exactly 525,651 DFFs.
2. Compare every primary opcode and each implemented ALU, shift, compare, MAC,
   system, and SPR subfamily against the checked functional oracle.
3. Compare exact raw bytes/word, mnemonic, PC transition, write effects, and
   complete before/after state for every successful witness.
4. Cover both branch outcomes, links, exceptions, privilege, memory widths and
   signs, alignment/range checks, MAC, multiply/divide, atomics, reset, restore,
   arbitrary-origin loading, rollback, and trace bounds.
5. Pass release tests, package-scoped formatting, strict Clippy, rustdoc,
   package BUILD, lessons validation, independent security review, and more
   than 95% line coverage.
