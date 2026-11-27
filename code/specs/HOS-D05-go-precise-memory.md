# HOS-D05-GO7 — Opt-in precise flat-RAM effects for the Go D05 core

## Purpose and boundary

This proposal adds a second, explicit execution mode to the Go RV32I classic
five-stage core, with hazard detection enabled, for synchronous data-memory
faults and retirement-ordered effects. The existing constructors and default pipeline keep their current
behavior. The initial mode covers aligned, in-range `LW` and `SW` to flat RAM
and terminal access-fault outcomes. It does not implement privileged traps,
CSR changes, device/MMIO rollback, interrupt delivery, instruction-fetch
faults, byte/halfword accesses, board wiring, or OS boot.

The fault outcome identifies the selected instruction and access; it is not a
claim that an architectural RISC-V trap handler ran. The commit boundary in
this slice is GPR state plus flat RAM. Branch predictor/cache statistics and
other microarchitectural observations are outside the architectural comparison.

## Existing timing and concrete alternatives

The current D05 path invokes `Core.memoryCallback` for MEM and writes stores
immediately through `MemoryController.WriteWord`; later the pipeline invokes
the WB callback. `MemoryController.ReadWord` returns zero for an invalid range
and `WriteWord` silently ignores it, so a bad flat-RAM data address is not a
typed fault. Callback signatures have no error result. The pipeline advances
and calls stage callbacks before its end-of-cycle writeback callback. A younger
load in MEM can therefore run before an older store in WB if the instructions
are adjacent.

Three minimal-looking alternatives were considered:

1. **Validate addresses in the current MEM callback.** This can notice an
   invalid range, but it cannot stop younger callbacks or report an ordered
   terminal outcome. It leaves stores at MEM and does not fix the store/load
   ordering window.
2. **Change the existing callback signatures to return errors or outcomes.**
   This changes generic pipeline clients and every constructor call site, and
   still needs an ordering rule for a younger fault and an older WB effect.
3. **Snapshot and roll back a normal core after a fault.** The core exposes no
   complete checkpoint of pipeline, predictor, caches, pending stores, and
   operation state. Reversing public RAM/register writes alone would leave a
   partially advanced machine, so rollback is not a viable precise boundary.

The selected direction is an additive opt-in precise-memory path on the
classic Go RV32I adapter, using an additive pipeline fault action/outcome and
leaving the old callback signatures and constructors intact. The mode requires
hazard detection because the selected-fault transition uses that callback to
suppress younger work; the constructor rejects precise mode when it is disabled,
while legacy constructors continue to accept hazard-disabled configurations.
Address validation is a pure EX-stage preflight. A detected fault is carried by its
token; the following cycle selects it before younger work enters EX, squashes
younger IF/ID tokens without fetching a replacement, and lets older MEM/WB
tokens drain. The faulting token advances to WB as the ordered fault-selection
point, but it is not a successful retirement: it does not increment the
retired-instruction count or appear in the retired-PC list, and causes no
memory receipt or GPR write. Only then does the new step outcome become
terminal. Older WB GPR writes and stores remain committed.

## Ordered store/load behavior

In the opt-in mode, `SW` at MEM creates a pending store intent instead of
changing RAM. The intent contains instruction PC, checked word address, and
store value. It commits to flat RAM only when that same instruction reaches
WB. `LW` at MEM first checks for the newest older pending store at its exact
word address and forwards that value; otherwise it reads checked RAM. This is
required because the current callback order evaluates a younger MEM load
before the older WB store callback in the same cycle. In-order operation has
at most one MEM callback per cycle; transient pending state is bounded to the
store already in WB plus a possible new store in MEM. A faulting or squashed
store never enters the commit queue.

RV32I computes the effective address with 32-bit modulo arithmetic:
`effective = uint32(base) + uint32(sign_extend_32(immediate))`. Alignment is
checked on that wrapped effective address. The RAM bounds check then widens the
already-wrapped address and verifies `uint64(effective) + 4 <= uint64(RAM size)`;
it must not use host `int` overflow or treat a carry past `2^32` as an address
above the RV32 address space. Exact-end access is allowed. The fault records
instruction PC, raw instruction identity, load/store kind, wrapped address,
and width. A faulting load
does not update its destination; a faulting store changes no byte. The first
mode reports one terminal typed outcome and does not retry the faulting
instruction. Repeated calls after terminal selection return the stable outcome
without advancing another pipeline cycle or re-emitting the commit receipt.

## Required evidence

1. **Legacy isolation.** The normal constructor still replays the existing
   nonfault D05 expected trace unchanged: 16 cycles, 9 retired PCs, 2 forwards,
   1 stall, and 1 redirect flush, with the same final registers and 64 RAM
   words. No default config or manifest changes.
2. **Store-to-load order.** A new in-memory `SW; LW` same-address case with no
   intervening guest instruction gets the stored word in the load, commits one
   store receipt at the store's WB cycle, and adds no wait cycle. Compare its
   ordered retirements, registers, and RAM to the independent sequential
   `RiscVSimulator`.
3. **Older state survives.** A valid older GPR write and an older valid store
   followed by an invalid load both commit before the selected fault outcome.
   The faulting load's destination retains its old value. Assert the exact
   retired-PC sequence and count: older successful instructions appear once;
   the faulting PC is reported only in the fault outcome, never as a retired
   PC or completed instruction.
4. **Faulting effects are atomic.** Test an out-of-range and an unaligned
   `LW`; neither changes a register. Test an unaligned/out-of-range `SW` over
   preinitialized bytes; no byte changes and no store receipt is emitted.
5. **Younger work is suppressed.** Put a store/register write after the
   selected fault and assert no retirement, memory receipt, or state mutation
   from it. Separately put an invalid load on a taken branch's wrong path and
   assert that the correct target retires without selecting a fault.
6. **One terminal selection.** Assert exact selected PC/address/kind, older
   retirement ordering, no extra post-terminal cycle, and no duplicate store
   receipt or fault outcome.
7. **RV32 effective-address wrap.** In 256-byte RAM, set a base register to
   `0xffffffff` and access `LW` at offset `+1`; the effective address wraps to
   aligned address `0`, succeeds, and compares with the sequential simulator.
   Separately access aligned address `0xfffffffc`; it remains a high RV32
   address after wrapping and selects a precise load fault. Assert the reported
   wrapped address and that no high address is narrowed through host `int`.

All test programs use the published Go RV32I adapter, a classic five-stage
configuration, at most 32 instructions, and 200 cycles. The current
16-cycle fixture remains a legacy-mode oracle; precise mode is compared to its
stage/retirement projection on that nonfault program but does not replace or
recapture the fixed fixture. New precise-mode cases are compared with a fresh
sequential simulator on architectural state. Because current flat memory
accessors do not expose faults, a new opt-in checked access/outcome seam is
required before those fault cases can execute.

## Limits and follow-up seams

This mode is deliberately not a general transaction API. It has one flat RAM,
one in-order core, word accesses only, and no concurrency. It does not undo
branch-predictor training or cache accounting from an instruction later
selected as faulting. It does not promise precise CSR/privilege behavior or
fault-aware instruction fetch. Byte/halfword operations, mixed-width store
forwarding, MMIO, multi-core visibility, and guest handler entry require
separate designs and tests. A later syscall or MMIO path must define whether
reads are pure/replayable and commit non-idempotent writes only at an ordered
boundary.
