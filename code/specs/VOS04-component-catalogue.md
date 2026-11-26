# VOS04 -- The Component Catalogue

**Status:** Draft. A catalogue and a set of rules; no code, no crate and no
generated document is added by this spec. Every pool row below is **queued**
(track C of [the backlog](VOS-BACKLOG.md)) except `sched`, whose contract
exists (VOS10) and whose first implementation is in review (VOS-011). Sizes
marked "arithmetic" are binder products worked out from widths and have **not
been run**.

**Layer:** Program model. It sits between the umbrella
([VOS00](VOS00-verified-os-program.md) section 7, the component pool) and the
per-interface contract specs (VOS10 and the `VOS1x` family that follows it).

**Parent:** [VOS00](VOS00-verified-os-program.md) (sections 4, 7, 8 and 11).

**Builds on:** [VOS01](VOS01-component-contract-model.md) (the component shape,
tiers, `-any`, the budget), [VOS02](VOS02-interface-contracts-and-profile-linking.md)
(generated per-implementation documents, kinds, profile manifests, SH16),
[VOS03](VOS03-component-shape-checker.md) (SH1-SH19),
[VOS10](VOS10-scheduler-contract.md) (the first contract),
[VOS30](VOS30-proof-route-and-capacity.md) (the X3 route and the capacity
numbers) and [EMU00](EMU00-emulator-framework.md) (the machine side).

**Owner direction, 2026-11-16 (re-emphasis):** "The scheduler is just a small
piece of an OS. Every piece should live in its own package, and you should be
able to assemble any kind of OS from the basic pieces. Each piece should be
individually verified and proven through SAT solving, fuzzing, property
testing, unit testing and others." The owner also chose **serial execution**:
one item at a time, in a written order, and no new work starts before it is
logged in the backlog and prioritised.

---

## 0. How to read this document

VOS00 says there is a pool and lists it in one table of 18 lines (section 7).
VOS01 to VOS03 say how one component is shaped, linked and checked. VOS10
specifies one component, the scheduler, in full. Between them one thing was
missing: **the list of every piece, with its boundary, its implementations,
its evidence plan and its place in the queue.** Without that list the
scheduler is the only piece anyone can see, and "assemble any OS from pieces"
is a slogan. This document is the list, and the rules that decide what belongs
on it.

Marks are those of VOS01 section 0. **Today** exists on this branch;
**NEW(row)** does not, and the named backlog row closes it. Where this spec
states a number it says whether the number was run (a result) or worked out
(arithmetic).

Three words are used strictly, because the pool needs a vocabulary that the
earlier specs did not have to fix:

* **Interface:** a contract package's subject. `sched` is an interface. It has
  one abstract state M and a fixed set of operations.
* **Implementation:** one crate that satisfies an interface's contract by
  evidence (`vos-sched-fp-bitmap`). One interface has several, one of them its
  most-general `-any` (VOS01 section 3).
* **Slot:** a role in a profile (VOS02 section 6): "the frame allocator", "the
  queuing port". A slot is filled by an implementation of some interface. The
  same interface can fill several slots.

Section 1 states the five decisions that shape the pool. Section 2 is the pool.
Section 3 is what is deliberately not in it. Section 4 is the normative rule
set for deciding what a component is. Section 5 is the evidence every
component must ship, honestly including what the toolchain cannot yet do.
Section 6 is the serial order. Section 7 shows how the five profiles are
assembled from the pool. Section 8 lists what is not yet verified, as open
questions. Section 9 says what this buys and what it does not.

---

## 1. Five shaping decisions

Each decision has a reason that comes from a fact already established in
VOS01 to VOS30. They are listed first because every row in section 2 follows
from them.

### 1.1 Mechanism names, role slots

A contract is named for the **mechanism** (what its state machine is), and a
profile slot is named for the **role** (what the kernel uses it for). One
mechanism fills many roles.

*Why.* A frame allocator, a message-buffer pool and an address-space-id
allocator are the same machine: a set of free ids, `alloc` takes one, `free`
returns one. If each were its own contract the pool would hold three copies of
one proof, and a bug found in one would have to be re-found in the others.
Calling all three `pool` means one contract, one `-any`, one evidence set; the
profile says which slot a `pool` instance fills and with what configuration.
The same holds for `chan`: an ARINC queuing port, an RC4000 message buffer, a
Unix pipe and a UART receive buffer are all a bounded FIFO. An ARINC queuing
port is not a component at all; it is a `chan` slot plus a capability check in
the spine (section 3).

*Cost.* A contract written for several roles must not mention any of them
(rule 12 below). That is a real design constraint, and it is the point.

### 1.2 Degenerate implementations, not optional slots

When a profile does not need a feature, it gets an implementation that does
nothing, not a hole in the profile. The hard RTOS has no budgets, so it fills
the budget slot with `vos-budget-none`; it has one partition, so it fills the
partition-scheduler slot with `vos-psched-single`; its health monitor halts,
so it fills the health-monitor slot with `vos-hm-halt`.

*Why.* An optional slot means the spine has two shapes (with the component and
without), and every spine claim must be made twice. A degenerate
implementation keeps **one spine** for hard RTOS, soft real time, flight,
separation and static-partition hypervisor (`vos-spine-static`), and the
degenerate implementation is itself evidenced against the same contract, so
"does nothing" is a checked statement and not an omission. A GPOS needs a
second spine, because it has dynamic objects and revocation (section 3).

*What "does nothing" costs.* A degenerate implementation proves **less**, and
the evidence must say so. It carries a distinct, **weaker claim set**: for
example `vos-budget-none` carries no `budget` (enforcement) claim,
`vos-psched-single` carries no `partition-time` claim, `vos-hm-halt` carries no
`partition-health` (containment and recovery) claim and `vos-prot-none` carries
no `isolation` claim. (The claim names are provisional; each contract's PR-C
fixes the spelling.) Each profile theorem names the claims it **needs**
(VOS02's `needs` is a subset of `carries` check, section 7.2 here), so a
profile whose theorem needs `partition-time` cannot be assembled with
`vos-psched-single`. Without this, "the spine is the same" would let a flight
profile claim time partitioning with a slot that does nothing.

### 1.3 The control and data split

Payload stores (a thread's saved register frame, the bytes of a message) are
separate leaves from the small state machines that decide who owns what.

*Why.* A register frame is 31 words; a message is up to a page. Putting them
inside the thread table or the endpoint would multiply the exhaustive domain
by their width and make the control logic unprovable at X1 for a reason that
has nothing to do with the control logic. Split, the control component
(`thread`, `ep`) is small and fully enumerable, and the data component
(`ctx`, a message store) is a copy with a frame property (VOS30 section 3.4)
that is evidenced at a reduced word width at X1 and at real width by property
tests, fuzzing and X3 (rule 4).

### 1.4 One runtime-indexed access per operation

An operation indexes at most one runtime-selected cell. Multi-object
operations (an IPC transfer touches two threads and an endpoint; a revoke walks
a tree) are **spine sequences** of single-access calls, or one node per call.

*Why.* VOS30 section 8 measured that a symbolic index into a table costs
about one case split per cell, and that more than about two chained symbolic
accesses at T = 8 is not solvable. The way to stay on the feasible side of that
line is to make it a design rule, not a hope. This is also why a tree walk is
`revoke_step` (one node per call, the spine loops) rather than `revoke`.

### 1.5 Effects as data

No leaf touches hardware. A driver is a step function that **returns
commands** (a byte to transmit, a line to acknowledge); the protection
component returns a configuration image; the HAL executes both. A command is a
value of a **closed, per-interface enum** whose payloads are opaque ids,
bounded indices and data bytes: **no raw address and no CSR number appears in
any leaf command** (rule 6). The HAL turns an index into an address from a
table it owns, and **validates every command against a static allowlist**
derived from the board map and the owning domain before it executes it, so a
leaf that is wrong, or compromised, can ask for nothing the allowlist does not
name. Every leaf's capability
footprint therefore stays **empty**, which `ca-capability-analyzer` can check
mechanically (BUILD line 3, section 5.2), and the only code that touches
hardware is the HAL, which is the one place with `unsafe` and inline assembly
and has its own inventory evidence (VOS00 section 4.6; VOS21, VOS-025a).

*Why.* A claim that a component is pure is only as strong as the check that it
has no effects, and a command channel that can name any address is an effect
in disguise. Moving every effect into one inventoried place, behind a closed
vocabulary and an allowlist, turns a program-wide property into a property of
one small crate.

---

## 2. The pool

### 2.1 How the tables are read

**Profiles:** H hard RTOS, S soft real time, F flight and separation
(ARINC-653 style), G general-purpose OS, V hypervisor. Section 7 shows each
profile's assembly; the column here says only who uses an interface.

**Rung** is the history-ladder rung of VOS00 section 6.1 that feeds the
interface: the old OS idea that the contract has to be able to express.

**X1** is the reduced configuration of VOS00 section 9: the instance size at
which every claim is enumerated exhaustively, with the largest claim's binder
product as the case count against the 400,000-case budget (VOS01 section 5.6).
**These sizes are arithmetic, from binder widths, and have not been run.** The
two rows near the budget (`prot`, `pt-flat`, 262,144 cases, 65 percent of it)
may need trimming when run (section 8, item 3). The one row that was run is
`sched` (65,536 cases per claim, VOS10 section 10).

**X3 obligations** name the kinds of VOS30 section 3: PATH (3.1, path
feasibility and coverage), REF (3.2, per-path refinement), MDEC (3.2a, model
decomposition), INV (3.3, closure of `inv`), FRAME (3.4), STEP (3.5, step
bound), 2S (3.6, two-safety, for constant-time and timing).

**Risk classes** tell a planner how hard X3 will be:

| Class | Shape | What it costs |
|---|---|---|
| R0 | Bit-packed scalar state; width-insensitive | One obligation; green at the shipped width on the hand-written proxy code of VOS30 table A (N = 64); not yet compiler output |
| R1 | One symbolic index per operation | Case-split into T obligations, T up to 8 to 16 |
| R2 | Two indices, or a bounded scan | Product split, or a cut point per iteration |
| R3 | A linked structure | **Not provable today.** Redesign to R1 or R2, or wait for arrays (VOS00 section 12 row 9) |

**Implementations with no backlog row yet.** An implementation named in a
table or in the section 7 matrix but absent from every track C row of the
backlog is **not scheduled**; the matrix marks each with "[no row]". They are:
`pool-freelist`, `budget-sporadic`, `timer-wheel`, `irq-prio`, `irq-hvip`,
`sysdec-sbi`, `ctx-rv32h`, `load-elf32`, `prot-stage2-static`, `prot-none`
(never eligible for an image profile, section 7.2), `thread-packed`, a two-level
bitmap `sched`, `pt-*`, `aspace-table`, `mutex-pcp`, `emul-rv32h`, `blk-simple`
and the file-system server. Their interfaces are the C21+ placeholder or have a
row for a sibling implementation only. A cell without "[no row]" is a row that
exists.

The **Tiers** column names the contract tiers (VOS01 section 2.4): *safety*
(what every correct implementation must do), *policy* (a deterministic choice
among the safe outcomes, named in parentheses), *canonical* (a fixed
representation, so equal abstract states have one concrete form), and *any*
(the most-general implementation, which exists for every interface).

### 2.2 Execution and time

| Interface | M; ops | Tiers | Implementations | Profiles | Rung | X1 | X3 / risk |
|---|---|---|---|---|---|---|---|
| `sched` (exists, VOS10) | ready set of N bools; `init insert remove dispatch` | safety, policy (highest), canonical, any | `fp-bitmap` (in review), `rr` (representation mode), `any`; later a two-level bitmap for N = 64 | all | L3, L4 | N = 8, 65,536 (run) | REF INV FRAME STEP; R0 (A1 and A2 green at N = 64 on proxy code, VOS30 section 8.4) |
| `edf` (sibling; resolves VOS-D048) | ready set plus a per-thread key; `init insert(t,key) remove dispatch` | safety (same shape), policy (minimum key) | `edf-scan`, `edf-any` | S | L3 | T = 3, 2-bit keys, about 8k | PATH per scan step; R2 |
| `budget` (VOS10 section 13) | per-thread remaining, cap, refill; `init charge(t,ticks) refill_next(now) set` | safety (remaining at most cap; exhausted iff 0), policy (sporadic rule) | `budget-periodic`, `budget-sporadic`, `budget-none`, `any` | S, F, G | L3 | T = 2, 2-bit, about 8k | R1 |
| `psched` (major frame) | cursor and ticks over a W-window table; `init(cfg) tick(elapsed) current` | safety (a configured partition), policy (exact cyclic order and durations) | `psched-table`, `psched-single`, `any` | F, V (H and S take `-single`) | L4, L12 | W = 3, about 32k, config quantified | REF INV 2S (padded exit); R1 |
| `thread` (TCB lifecycle) | T slots {Free, Runnable, Blocked, Running} plus priority and partition tag; `init create exit block unblock run(t)` | safety (at most one Running; legal transitions), canonical | `thread-table`, `any`; later `thread-packed` | all (V reuses it in a vcpu slot) | L5, L6 | T = 4 with tag, about 32k | REF INV FRAME; R1. The first table component; forces VOS-D025 |
| `ctx` (context store, the data plane) | S frames of R words; `save(slot,frame) load(slot)` | safety (load returns the last save; other slots unchanged), canonical | `ctx-rv32` (31 words), `ctx-rv32h` (V), `any` | all | L8 | S = 2, R = 2, 2-bit words, about 8k; real width by property and fuzz | FRAME dominant, STEP; R1 times a concrete copy |
| `timer` | now plus an armed id -> deadline map; `arm cancel advance(now) expire_next` | safety (fires only armed and due, once; the next deadline is never late), policy (earliest first) | `timer-scan`, `timer-delta` (AGC Waitlist), `timer-wheel`, `any` | all | L3, L4 | N = 2, W = 3-bit time, 65,536 | R2 scan, R1 wheel |
| `mutex` (priority ceiling; later) | owner and ceiling per lock; `lock unlock` | safety (mutual exclusion), policy (ceiling protocol) | `mutex-pcp`, `any` | H, F | L5 | 2 locks, 3 threads | R1; the spine moves the thread in `sched` |

Why these are separate interfaces and not features of `sched`: VOS10 section 13
already argued it for budgets and EDF. Budget state is per-thread timing state
the scheduler would otherwise own a second copy of; EDF needs a key channel the
four-operation interface does not have, which is a different M, so by rule 2
below it is a sibling and not a tier. That argument is now the general rule.

### 2.3 Memory, protection and authority

| Interface | M; ops | Tiers | Implementations | Profiles | Rung | X1 | X3 / risk |
|---|---|---|---|---|---|---|---|
| `pool` (ids: frames, blocks, ASIDs, slots) | free set of B ids; `init alloc free(id)` | safety (alloc returns a free id; no double alloc), policy (lowest first, deterministic for noninterference), canonical | `pool-bitmap`, `pool-freelist` (representation mode), `any` | G, V; F for buffers | L2, L8 | B = 8, 65,536 | R0 bitmap (the cheapest second interface); freelist R2 |
| `untyped` (watermark retype) | (watermark, size); `retype(n,size) reset` | safety (disjoint, in bounds) | `untyped-bump`, `any` | G | L12 | 3-bit | R0 |
| `region` (variable-size) | disjoint intervals; `alloc(size,align) free(base)` | safety, policy (first-fit or buddy) | `region-buddy`, `region-firstfit`, `any` | G only | L8 | 8 granules, at most 2 live | R2 or R3; **not in the pool until redesigned** (rule 12); prefer fixed pools |
| `prot` (PMP, MPU or static stage-2 plan) | static per-domain regions plus the loaded domain; `init(cfg) load(d)` returns a config image; `check(d,addr,len,access)` | safety (grants are a subset of d's regions, never the kernel or another domain; **image agreement**: the config image `load(d)` returns grants exactly what `check(d,..)` allows), policy (exact) | `prot-pmp-napot`, `prot-pmp-tor`, later `prot-stage2-static`, `prot-none`, `any` | H S F; V via stage-2 | L12 | R = 2, 8 granules, config quantified, 262,144 (65 percent of budget) | REF over a **shared** PMP-match model, 2S; R1. The match model must be the same Warrant text the emulator's PMP claims use (VOS-021), and it is **differentially tested against `riscv-simulator`'s independent PMP** (VOS-D074) |
| `pt` (page table) | partial map vpn -> (pfn, perms); `map unmap lookup` | safety (a map changes only that vpn; only given frames), canonical | `pt-flat` (reference), `pt-sv32`, later `pt-sv39`, `any` | G; V in a stage-2 slot | L2, L8 | flat 4 entries about 131k; 2-level 2x2 about 8k | R2. The walk is the chained-access cliff; cut between levels or split by product |
| `aspace` | A spaces (root, ASID, state); `create destroy activate` | safety | `aspace-table`, `any` | G, V | L8 | A = 2 | R1 |
| `cspace` (capability lookup) | slots (kind, object, rights) plus a **sealed** bit; `init lookup(d,slot,need) install remove seal` | safety (grant only if the rights cover the need: confinement; **`install` or `remove` after `seal` answers `Sealed` and changes nothing**), canonical | `cspace-table`, `any` | H S F V (install at boot, then `seal`); G (never sealed) | L12 | 3 slots, sealed bit, about 131k | R1 |
| `cdt` (derive and revoke) | a derivation forest; `derive delete revoke_step` (one node per call) | safety (rights fall along a derivation; no descendant remains after a revoke completes) | `cdt-array`, `any` | G only | L12 | 3 nodes, 65,536 | R3 made R2 only if `revoke_step` holds; **not in the pool until redesigned and WX-S3 is closed** (rule 12) |

Why `cspace` has a `seal`: H, S and F install capabilities at boot only, and
"at boot only" must be an obligation, not a convention. The choice was between
one `cspace` whose M carries a sealed bit (and whose safety tier says an
`install` or `remove` after `seal` answers `Sealed` and changes nothing), and a
distinct `cspace-static` contract with no `install`. The sealed bit is chosen:
the lookup and the confinement lemmas are the same text for the static and the
dynamic case, so a second contract would duplicate them and could drift; one
extra bit and one operation is a small cost; and a profile that is not sealed
(the GPOS) is simply one that never calls `seal`. The spine then owns the
sealed-after-boot invariant (I11) and boot calls `seal` before the first user
instruction (section 6.3). The sabotage control removes the seal two ways: a
`cspace-table` whose `install` ignores the bit must fail the safety claim, and a
spine whose reset entry omits `seal` must fail I11.

Why `pool` ranks as the cheapest second interface: a free set is a bit-packed
scalar (R0), so its X3 route is the same width-insensitive route as the
bitmap scheduler. It is a good first test that the component kit of section 5
is mechanical and not particular to `sched`. `region` is the cautionary row: a
variable-size allocator is a linked structure (R3), so it fails the "basic"
test of rule 12 and is deferred; fixed pools plus `pool` cover the RTOS, flight
and separation profiles with no kernel heap. `cdt` is in the same position and
the tables say so: both are listed to record the design, and neither is a pool
member until a redesign brings it to R2 with a proof-feasibility note.

### 2.4 Communication, interrupts and partition services

| Interface | M; ops | Tiers | Implementations | Profiles | Rung | X1 | X3 / risk |
|---|---|---|---|---|---|---|---|
| `ep` (synchronous endpoint) | per endpoint Idle, SendQ or RecvQ of thread ids; `send recv cancel` | safety (the partner really waited on that endpoint; a thread is in at most one queue), policy (FIFO or priority) | `ep-single` (L4 minimum), `ep-fifo`, `any` | H S F G | L6, L10, L11 | E = 2, T = 3, depth 2, about 131k | single R1; fifo R2 |
| `ntfn` | a bit word plus an optional waiter; `signal wait poll` | safety, canonical | `ntfn-word`, `any` | all | L11 | about 32k | R1 |
| `sem` | count plus a wait queue; `p v` | safety, policy (FIFO) | `sem-count`, `any` | optional | L5 | max 3, T = 3 | R1 |
| `chan` (bounded FIFO: queuing port, RC4000 buffer, pipe, UART buffer) | a sequence of at most Q items; `send recv` | safety (no loss, duplication or reordering; Full and Empty answered), canonical | `chan-ring` (representation mode), `chan-shift`, `any` | F, G | L6 | Q = 4, 2-bit items, about 32k | R1 |
| `sample` (sampling port) | per port (msg, stamp, valid); `write(p,msg,now) read(p,now)` | safety (a read returns that port's last write only), policy (freshness) | `sample-latch`, `any` | F | L12 | P = 2, about 32k | REF 2S; R1 plus a copy |
| `hm` (health monitor) | a static (class, level) -> action table plus per-partition status; `init(cfg) report(err,src)` | safety (never touches another partition unless the level is module), policy (exact table) | `hm-table`, `hm-halt`, `any` | F (H takes `-halt`) | L4 (AGC restart) | 2x2 config quantified, about 16k | R1 |
| `irq` (kernel routing) | per line enabled, pending, in-service, binding; `bind claim ack mask` | safety (deliver only to the bound target; no redelivery before ack), policy (priority) | `irq-flat`, `irq-prio`, `irq-hvip` (V virq slot), `any` | all | L2 | L = 3, about 32k | bitsets R0; binding R1 |

The notable consequence of decision 1.1: there is no "ARINC port" row. A
queuing port is a `chan` slot whose send and receive the spine guards with a
`cspace` lookup and a partition tag; a sampling port is the one genuinely
distinct mechanism (a latch with a timestamp, not a queue), so it gets its own
interface.

### 2.5 Edge: codecs, drivers and storage

| Interface | M; ops | Tiers | Implementations | Profiles | Rung | X1 | X3 / risk |
|---|---|---|---|---|---|---|---|
| `sysdec` (syscall decoder) | stateless; `decode(regs)` -> Syscall or Invalid | safety (total; never an out-of-range argument), canonical (a round trip with the user-side encoder) | `sysdec-rtos`, `sysdec-sbi` (V and GPOS shim), `any` | all | L2 (extracode) | reduced width W = 4 | R0; the best early X3 after `sched` |
| `trapdec` (per ISA) | stateless; `decode(mcause, mtval)` | safety (total), canonical | `trapdec-rv32`, `any` | all | L2 | reduced | R0 |
| `emul` (trapped-instruction decoder) | stateless | safety | `emul-rv32h`, `any` | V | L9 | reduced | R0 |
| `load` (streaming image decoder) | parser state; `step(state, byte)` -> Segment, Data, Entry, Reject or More | safety (never stores outside a declared segment; malformed input rejected; bounded steps per byte) | `load-flat`, `load-elf32`, `any`, rung-local EDSAC orders | G V; rungs L0 L1 L7 | L0 | all strings of length at most 5 over a 3-bit alphabet, about 32k | R0 or R1; **fuzz is primary** (the VOS-D001 pattern), with the corpus and length bounds of section 5.2 line 8 |
| `uart` (console driver as a step) | tx and rx buffers plus flags; `put on_tx_ready on_rx get`, returning commands | safety (no byte invented or dropped silently) | `uart-poll`, `uart-ring`, `any` | all | L7 | depth 2, about 2k | R0 poll |
| `blk` | a request state machine; `submit(lba,dir) complete(status)` | safety (one in flight; range checked) | `blk-simple`, `any` | G | L7 | tiny | R0 |
| FS leaves `dir`, `extent`, `bcache`, `fdtable` | name -> inode, offset -> block, block -> frame, handle -> object | safety each | flat (CP/M) then tree (Unix) | G | L7, L8 | each below 65k | R1 or R2; specified with the rungs |

The decoders are stateless, so they are R0 and carry no `init`. They are the
natural early X3 targets after `sched` because there is no state to model: the
whole claim is a function from words to a small enumeration. `load` is a
parser of untrusted bytes, so the primary evidence is the fuzz target (VER06
style, EMU00 section 8.6), with exhaustive enumeration over short inputs as
the second line.

### 2.6 From the VOS00 section 7 table to this one

VOS00 section 7 listed 18 lines. This catalogue refines it, and VOS00 now
points here (the older table stays as the summary it was). Where the two
differ, this document governs:

| VOS00 section 7 line | In this catalogue |
|---|---|
| Boot / loader | Two things: `load` (a pool interface) and boot (not a component: the spine's reset entry) |
| HAL | Not a component (section 3); VOS21 |
| Scheduler, safety tier; fixed-priority; round-robin | One interface, `sched`, with tiers and the implementations `fp-bitmap`, `rr`, `any` |
| Scheduler, EDF / budget | Two sibling interfaces, `edf` and `budget` (VOS-D048, resolved) |
| Thread table | `thread`, with the register store split out as `ctx` (decision 1.3) |
| IPC: notification and endpoint | `ntfn` and `ep`, plus `chan` for buffered queues |
| Semaphore | `sem` |
| Static capability table | `cspace` |
| Dynamic capabilities with revocation | `cdt` (G only, later) |
| PMP plan | `prot`, with `prot-pmp-napot` and `prot-pmp-tor` as implementations |
| Partition table and major frame | `psched`; the partition tag is carried by `thread` |
| Page-table component | `pt` plus `aspace` |
| Untyped memory and retype | `untyped` |
| Trap-and-emulate VMM | Reuse (section 3) plus `emul` |
| File system | Edge FS leaves, specified with the rungs |
| User-space drivers | `uart`, `blk`: drivers as steps that return commands (decision 1.5) |

The interfaces with no VOS00 line are the ones the first draft missed because
it listed what a kernel *has*, not what it must *decide*: `pool`, `timer`,
`irq`, `ctx`, `chan`, `sample`, `hm`, `mutex`, `sysdec`, `trapdec`.

---

## 3. What is not a component

Eleven things look like components and are not. Naming them keeps the pool
small and keeps the proof obligations where they belong.

| Not a component | What it is instead | Why |
|---|---|---|
| **The spine** (`vos-spine-static`, later `vos-spine-gpos`) | The sequencer: one per family. It owns the cross-component invariant (rule 11) and is not replaceable | A replaceable sequencer has no stable invariant to evidence |
| **Syscall dispatch** | The spine's `match` over `sysdec`'s output | It is control flow over leaves, not a state machine |
| **Boot** | The spine's reset entry: fold the leaves' `init`, install the configuration, show `spine_inv(K0)`, lock protection before the first user instruction | Boot is the proof that the initial state satisfies the invariant (section 6.3) |
| **The HAL** | VOS21, not a leaf: the only `unsafe` and assembly, with inventory evidence; split into an ISA crate `vos-hal-rv32` and a board crate, mirroring `emu-board-*` | Effects as data (1.5): the HAL is where the data become effects |
| **An ARINC queuing port** | An assembly: `chan` plus `cspace` plus a partition tag | Mechanism names, role slots (1.1) |
| **A slab allocator** | None: fixed pool slots, no kernel heap | SH1 forbids a heap in class K code; `pool` covers every case the profiles have |
| **A file system** | A subsystem with its own server spine | It has dynamic objects and its own invariant; its leaves (`dir`, `extent`, `bcache`, `fdtable`) are in the edge table |
| **Watchdog, fault delivery, exit padding** | Spine sequences | Each is a short ordered series of leaf calls with an ordering property, not a state machine |
| **vCPU, virtual IRQ, stage-2** | Reuse: a `thread` in a vcpu slot, `ctx-rv32h`, `irq-hvip`, `prot-stage2-static`. The hypervisor adds only `emul`, `sysdec-sbi` and an H-extension HAL | A hypervisor is mostly the existing pool in different slots |
| **`vos-kmodel`, the refinement monitor, `emu-dev-*`** | Evidence and the machine side | They check or host components; they are not themselves part of an image |
| **SMP locks** | Out of scope | One hart (VOS00 section 3) |

---
## 4. Decomposition rules (normative)

These thirteen rules decide what a component is, where its boundary falls and
what a reviewer rejects. A new row in section 2 is justified against them, not
against taste. Rules 3 to 6 restate shaping decisions of section 1 as
checkable text.

**Rule 1: one interface per contract package, one implementation per crate,
one `-any` per interface; a PR ships exactly one package.** *Why.* VOS02's
generated documents and BUILD evidence are per implementation; a PR that
carries two packages cannot be reviewed against one contract and cannot be
reverted alone. It also makes the serial order of section 6 meaningful: an
item is one package.

**Rule 2: the same-contract test.** Two things share a contract only if they
have **the same M and the same operation signatures**, differing only in which
allowed outcome they choose (a tier, or an implementation). A different M, or a
new input channel, makes a **sibling contract**. *Why.* VOS10 section 13
reached this twice already: budgets need time (a new input) and EDF needs a
per-thread key (a different M), so both are siblings of `sched`, not tiers of
it. Stating it as a rule means the next question of the same kind (does a
priority-inheritance mutex belong in `sched`?) is answered by looking at M,
not by argument.

**Rule 3: mechanism names, role slots** (decision 1.1). The contract text of
an interface mentions no profile, no role and no other interface.

**Rule 4: control and data split** (decision 1.3). Payload leaves are
evidenced at a **reduced word width at X1** and at **real width by property
tests, fuzzing and X3**. The X1 report prints both widths, so a reader cannot
mistake a 2-bit-word result for a 32-bit one.

**Rule 5: one runtime-indexed access per operation** (decision 1.4). Scan
loop bounds are configuration constants, so the loop index is concrete in
every unrolled path. Tree walks are one node per call. *Why.* This is the
VOS30 section 8 cliff stated as a rule.

**Rule 6: effects as data** (decision 1.5). A leaf has an empty capability
footprint (`required_capabilities.json` is empty), and the HAL is the only
crate that executes commands. Three requirements make "commands" safe:

1. **Closed vocabulary.** Every interface that returns commands declares them
   as a closed enum in its contract package. No variant carries a raw address,
   a CSR number or an MMIO offset; payloads are opaque ids, bounded indices and
   data bytes. A leaf therefore cannot even **construct** a request for an
   arbitrary location. This is checked on the contract package, where the enums live (SH19, section 5.6; PR-I line 2
   in section 5.2), and has negative controls: a contract enum that carries a raw address must be
   rejected (VOS-D077, done in VOS-039b-2a).
2. **Allowlist in the HAL.** The HAL (VOS21, VOS-025a) validates every command
   against a **static allowlist** built from the board map and the owning
   domain, before executing it. A command outside the allowlist is rejected,
   never executed, and recorded. The HAL's evidence includes a negative
   control: an out-of-range command is rejected.
3. **Images agree with checks.** `prot` returns a configuration image, which is
   an effect too. Its safety tier carries an **image-agreement obligation**:
   `load(d)`'s image grants exactly what `check(d, ..)` allows, no more and no
   less (section 2.3). Without it the model could be proved safe while the
   registers the HAL writes grant something else.

**Rule 7: time is an input, never read.** A time-carrying contract takes the
current time as an argument and states an **overflow rule**: either saturating
arithmetic or a horizon precondition (the caller promises time stays below
H). *Why.* The reduced X1 width wraps where the shipped width does not. A
3-bit timer evidenced exhaustively would "prove" wraparound behaviour that the
32-bit shipped timer never exhibits, and miss a horizon bug that only appears
at real width. Logged as VOS-D069; it binds `timer`, `budget`, `psched` and
`sample`.

**Rule 8: static configuration is an `init(cfg)` argument and a binder in X1
claims.** *Why.* Evidence then covers **every configuration of the reduced
shape**, not one hand-picked configuration. A partition table with W = 3
windows is evidenced over all tables, not over the one the flight profile
happens to use. The mechanism for evidencing one implementation at two
configurations (a small X1 configuration and the shipped X2 configuration) is
open: VOS-D068.

**Rule 9: degenerate implementations** (kind `production`) instead of optional
slots (decision 1.2). `vos-budget-none`, `vos-psched-single` and `vos-hm-halt`
are ordinary implementations with their own evidence, **and a distinct, weaker
claim set**: they do not carry the `budget`, `partition-time`,
`partition-health` or `isolation` claims. Profile theorems name the claims they
need, and `check-profile` rejects a profile whose needed claim is not carried by
the implementation in its slot (section 7.2). `vos-prot-none` exists only for
evidence profiles and the early boot stages; it is **barred from every
`class image` profile** (H, S, F, G and V).

**Rule 10: ids are opaque.** A component never invents or interprets an id of
another component; the **spine hands ids across**. A thread id is a number the
`thread` leaf issues and `sched` stores; `sched` does not know it names a
thread (VOS10 section 2).

**Rule 11: cross-component invariants live only in the spine document**, over
a record of abstract states, stated **pairwise**, each closed per entry over
`-any` models. *Why pairwise.* A naive two-leaf claim over two full abstract
states is already 33,554,432 cases, 84 times the budget (VOS01 section 6.1), so
the standard set is a list of two-component facts, each of which is small:

| # | Invariant | Components named |
|---|---|---|
| I1 | ready is a subset of Runnable | `sched`, `thread` |
| I2 | running is not in ready | `sched`, `thread` |
| I3 | a Blocked thread is in exactly one wait structure | `thread`, `ep`, `sem`, `timer` |
| I4 | the loaded protection domain equals the domain of the owner of the running thread, and each domain belongs to exactly one partition tag | `prot`, `thread` |
| I5 | armed timers name live threads | `timer`, `thread` |
| I6 | capability objects are live | `cspace`, the object owner |
| I7 | the partition of the running thread equals `psched`'s current partition | `thread`, `psched` |
| I8 | mapped frames are a subset of allocated frames | `pt`, `pool` |
| I9 | every id an instance hands out belongs to that instance's owner partition, and no instance is shared across partition tags (rule 13) | every instance, the spine |
| I10 | no loaded domain grants kernel memory or another partition's memory | `prot`, `thread` |
| I11 | after reset, in a sealed profile (H, S, F, V), every `cspace` instance (one per partition in the xP columns) is sealed before the first user instruction | `cspace`, the spine |

This is the standard set; a profile uses the rows whose components it
contains. The spine specification (VOS20, VOS-022a) proves each row closed
under each spine entry over the `-any` models, which is why `-any` exists for
every interface.

**Rule 12: the "basic" test.** A piece joins the pool only if **all** of these
hold:

1. its contract mentions no profile (rule 3);
2. it has **at least two consumers**, **at least one of them a profile**
   (rungs alone do not count: a rung is a teaching vehicle and a second rung
   does not make a second customer);
3. its reduced M fits in **about 20 bits**, with every claim at most 262,144
   cases (65 percent of the 400,000-case budget);
4. its risk class is **R0 to R2** (section 2.1), and an **R2 entry carries a
   proof-feasibility note** in its PR-C: which split it uses (product or cut
   point per iteration), the predicted number of obligations, and the probe
   result (VOS-028b) that supports the prediction.

Otherwise it stays rung-local or is redesigned. *Why this rule is the one that
keeps the pool honest.* It is the only rule that can **refuse** a piece.
`region` fails clause 4 (a variable-size allocator is a linked structure), and
`cdt` fails it until `revoke_step` is shown to make it R2 and WX-S3 is closed.
**Both are listed in section 2.3 to record the design, and neither is in the
pool until it is redesigned.** A file-system `dir` that serves only the
CP/M rung fails clause 2 and stays rung-local until a second consumer exists.

**Rule 13: one slot, one owner.** Each component **instance** fills **exactly
one slot** and has **exactly one owner partition**. An instance is never shared
by two slots, and never by two partition tags. Cross-partition communication
goes only through a **checked spine channel** (`chan` for queued messages,
`sample` for sampled ones), with a partition-tag check on every send and
receive and **padded exits** (the sender's time on the channel does not depend
on the receiver). *Why.* A shared `pool`, `timer` or `irq` instance is a
covert channel and a denial-of-service lever between partitions: one
partition's allocation pattern or interrupt rate changes another's behaviour,
and the contract of the shared instance cannot say whose it is. One owner per
instance makes isolation a property of ownership, closed by I9, and lets
`check-profile` reject a violation from the manifest alone (section 7.2). The
flight profile therefore **instantiates** `pool`, `timer`, `irq`, `thread` and
the other state machines **per partition** (section 7); a G or V profile with
one trust domain has one owner. **The kernel is an owner too**: an instance that is by nature cross-partition
(`psched`, `prot`, `hm`) is owned by the kernel, takes the partition tag as an
input, and is closed by I9 and I10 instead of being shared. **Stateless
decoders** (`sysdec`, `trapdec`, `emul`) have no state to own and may be shared.
A *different* configuration of one interface in two slots (VOS-D071) is
allowed, as separate instances, each with its own owner and pin.

### 4.1 The rules applied once: is `pool` a component?

A worked pass through rules 1 to 13, for the interface that section 2.3 ranks
as the cheapest second one. The point is that every answer is a lookup, not a
judgement.

1. *One package.* `vos-pool-contract`, then `vos-pool-bitmap`, then
   `vos-pool-any`: three PRs (C01c, C01i, C01a).
2. *Same contract.* A frame allocator hands out frame numbers, an ASID
   allocator hands out address-space ids, a message-buffer pool hands out
   buffer indices. All three have M = "a set of B free ids" and the signature
   `alloc() -> Option<id>`, `free(id)`. They differ only in B and in what the
   spine does with the id, so they share one contract.
3. *Names.* `pool` names the mechanism. The contract text says "ids"; the words
   frame, ASID and buffer appear only in the profile manifests that fill
   slots.
4. *Control and data.* The pool owns only which ids are free, never what is in
   a frame. The bytes of a frame are not its business.
5. *One indexed access.* `free(id)` sets one bit; `alloc` is a leading-bit
   search over a word (bounded, with a concrete index in every path).
6. *Effects.* None. Zeroing a recycled frame is a command the spine returns to
   the HAL.
7. *Time.* None, so rule 7 does not apply.
8. *Configuration.* B is `init(cfg)`'s argument and a binder, so the evidence
   covers every B of the reduced shape.
9. *Degenerate.* A profile with no dynamic allocation has no `pool` slot at all
   (H and S are static); it does not get `pool-none`, because nothing in its
   spine calls the interface. A degenerate implementation exists only where the
   spine **does** call the slot and the profile wants it to do nothing.
10. *Opaque ids.* `pool` returns an id and never interprets it; the spine
    decides it is a frame number.
11. *Invariants.* I8 (mapped frames are a subset of allocated frames) is the
    only standard row that names `pool`, and it is closed in the spine document
    over `pool-any`.
12. *Basic test.* Mentions no profile; consumers: G (frames, ASIDs), V (stage-2
    frames), F (buffers; one instance per partition, rule 13), plus the L2 and
    L8 rungs, so at least one profile consumer; B = 8 gives 8 bits of state,
    65,536 cases; risk R0 (so no proof-feasibility note is needed). It passes all
    four clauses.
13. *One slot, one owner.* Each use of `pool` in a profile is its own instance:
    the frame allocator is one instance owned by the kernel (G) or by one
    partition (F), the ASID allocator is another. The manifest names the owner
    of each, and `check-profile` rejects an instance in two slots of different
    partitions.

If any answer had been "it depends on the profile", the interface would have
been split or the clause 1 of rule 12 would have refused it. That is the
intended use.

---

## 5. The per-component evidence template

The owner's direction lists SAT solving, fuzzing, property testing, unit
testing "and others". This section says exactly which evidence every component
ships, in which order BUILD runs it, and where the toolchain still falls short.
The goal is that adding a component is **mechanical**: copy a template, fill in
the interface, run a checklist (section 5.2). Where it is not yet mechanical,
section 5.4 says why.

### 5.1 The baseline: what VOS-011 already ships

The first implementation, `vos-sched-fp-bitmap`, fixed the file set. Inside the
implementation crate:

* `Cargo.toml`: a `ghost` feature, the two allowed dev-dependencies
  (VOS03 SH12), and a `warrant_evidence` test target with `harness = false`
  and `required-features = ["ghost"]`;
* `vos.toml`: class, kind, contract, obligations and evidence features;
* `src/lib.rs` with the ghost face (VOS02 section 4);
* `warrant/impl.part` and `evidence.bindings`;
* `tests/behaviour.rs`, `tests/warrant_evidence.rs` (a shim) and
  `tests/generated/warrant_evidence.rs`;
* `evidence/test-manifest.json` and `warrant/check_evidence.sh`;
* `BUILD`, `BUILD_windows`, `README.md`, `CHANGELOG.md` and
  `required_capabilities.json`.

Outside the crate: the generated `code/specs/<impl>.warrant`, and one
`vosImplementationRow("<crate>")` in
`code/programs/go/build-tool/repository_inputs.go` with its test, so a change
to any VOS tool selects the implementation's evidence (VOS-D066).

### 5.2 Three PR kinds per interface

An interface arrives as a sequence of PRs, each one package (rule 1).

**PR-C, the contract package** (backlog rows `VOS-Cnnc`):

* a spec section in a child spec (the `VOS1x` family), written first;
* the Rust trait and its enums;
* `contract.vos`: tiers, configuration, ceilings;
* `warrant/contract.part`, `lemmas.part` (admitted but **not embedded**,
  VOS-D064), `obligations-{safety,policy,canonical,any}.part`;
* whole-domain **lemma evaluation with mutant controls**: generic, by
  `vos-link check-contract` (section 5.6, VOS-039b-1), from `warrant/lemmas.part` and one
  mutant part per lemma; the per-contract class H sibling is gone (VOS02 D-GD11);
* the **case-count table** derived from the run (never typed in by hand);
* the interface's row in section 2 filled in with the measured numbers;
* **model-SAT certificates of the lemmas (opt-in)**, by the lane of
  [VOS31](VOS31-host-model-sat.md): a hand-written parametric encoder, `warrant/lemmas.sat`
  and a generated BUILD line. **Amended by VOS-040-0 (owner/coordinator decision, 2026-11-25):** this is **required for `sched` and opt-in per
  contract** through the `vos.toml` key `sat = "model"`; whether it becomes mandatory is decided
  after `thread` (item 16) has paid the cost once (VOS-D147).

**PR-I, each implementation** (`VOS-Cnni`, `VOS-Cnni2`). BUILD runs these
lines **in this order**, cheapest first, so a failure reports the cheapest
explanation:

| # | Evidence | BUILD line | Gate |
|---|---|---|---|
| 1 | Unit | `cargo test --lib --test behaviour` with and without `ghost`, plus `--doc` | Against a plain reference model written in the test, not copied from the implementation |
| 2 | Shape | `vos-shape-check .` | SH1-SH19; `clippy::arithmetic_side_effects` denied; **command-type check** (rule 6: every command enum is closed and no variant payload can carry a raw address or CSR number), built as **SH19** on the `contract` kind in VOS-039b-2a, with negative controls that a contract enum emitting one is rejected (VOS-D077, done; section 5.6) |
| 3 | Capability | `ca-capability-analyzer --dir .` | An empty `required_capabilities.json` |
| 4 | Document | `vos-link check-doc .` | The committed `.warrant` equals the regenerated one |
| 5 | Harness drift | `gen_evidence --check <crate>` | The generated test harness equals the regenerated one |
| 6 | X1 exhaustive | the generated suite | Kind `full`; case counts read from the manifest. **None ran = fail**: an empty manifest, or a claim with zero cases, fails |
| 7 | Property | 4096 random cases per claim | Catches the width the reduced X1 cannot reach. **None ran = fail**: the case count is read from the output and zero, or fewer than 4096 for any claim, fails |
| 8 | Fuzz | 5000 iterations; parsers add a VER06 target with a stated **seed corpus** (one valid input per format, one input per `Reject` class, and every one of them truncated at each byte boundary) and **length bounds** (exhaustive up to the X1 string length, fuzz up to the EMU00 section 8.2 image limit) | Crashes and invariant violations on arbitrary bytes. **None ran = fail**: zero iterations, or a parser without its target, fails |
| 9 | Sabotage | `WARRANT_EVIDENCE_FAULT=...` for **every operation times every tier**, expecting **not ok** | Proves the claims can fail. **None ran = fail**: the number of controls run must equal operations times tiers (less the exceptions the manifest names, VOS-D065), so a skipped pair fails |
| 10 | SAT | **n/a for implementations until VOS-029**; contract lemma certificates live in PR-C (opt-in, VOS31) and are labelled as lemmas (section 5.3). After VOS-029: `vos-x3 check <crate>` | Until then the evidence report prints **"no SAT evidence"** for the implementation, and the line fails if it prints any SAT claim for one. After: a manifest with zero obligations fails |
| 11 | Selection | the build-tool row | A tool or document edit selects this crate |

Why this order: a unit failure is a bug; a shape failure is a forbidden
construct; a capability failure is an effect; a document failure is drift; and
only then do the expensive generated suites run. Sabotage runs after the
suites because it re-runs them with a deliberately broken implementation and
**expects a failure**; a claim that survives sabotage is vacuous. Lines 6 to 10
each have a **"none ran = fail"** condition because a suite that is silently
skipped reports green, and `check-component` (VOS-039) **fails on a missing
line** as well as on a failing one.

**PR-A, the `-any`** (`VOS-Cnna`): the same file set with kind `any`: the `any`
block (soundness, and coverage with witnesses, VOS01 section 3.3), **no
canonical block**, the oracle field, and SH16 keeping the crate out of every
image (VOS02 section 7). `-any` is what the spine's invariants are closed over
(rule 11), so it is not optional.

### 5.3 SAT, honestly

"Proven through SAT solving" is the owner's wording and the weakest-evidenced
part of the template today, so this section separates three things.

**What works today: host model-SAT on the contract.** The repository has a
bit-blaster (`BitVec`, `Aig`, Tseitin), a CDCL solver (`cdcl-sat`) and a DRAT
checker (`drat-check`). Together they can certify **contract lemmas at the
widths of a ladder** (N = 8, 16, 32 and 64), where exhaustive enumeration stops
at N = 8 to 10. The lane is specified in [VOS31](VOS31-host-model-sat.md).
**Amended by VOS-040-0 (owner/coordinator decision, 2026-11-25):** the scope is the ten lemmas of `lemmas.part` only (tier ordering is
already the lemma `policy_implies_safety`; spine-invariant closure waits for a
spine, and the `representation` tier's claims are obligations, not lemmas); the
widths are the ladder of VOS31 section 4 (owner: "Ladder 8/16/32/64
(Recommended)"), and **no row is called "shipped"** until VOS-D068 defines a
second configuration. Two limits are part of the statement, and the N = 64 figure is **evidence, not
proof, about the transcription**:

* **There is no Warrant-to-bit-vector translator**, so each encoding is a hand
  transcription. The mitigation is a **parametric-in-N encoder** that is
  checked against the Warrant evaluator on the **whole X1 domain at N = 8**
  with `Aig::eval` (and, as amended, against a plain Rust reference at N = 1 to 10 and
  per model function; VOS31 section 6), and then solved at the ladder widths. A transcription that
  agrees with the evaluator on all 65,536 cases at N = 8 is **evidence, not
  proof**, that it is right at N = 64: nothing relates the two widths (VOS-D072).
  The N = 64 solver results quoted elsewhere (VOS30 table A) are on
  hand-written proxy code, not compiler output.
* **It proves facts about the contract, not the implementation.** A lemma has
  no published strength (VER04m V9; VOS-D005), so it must be printed as
  "lemma, SAT-certified at N = ..." and **never counted** as evidence about
  any implementation.

**What is probably not available: SAT on Rust source.** `warrant-ir`'s Rust
front end refuses every real source tried; only `ct-compare` lowers. A
branch-free helper such as `clz8` might lower. This is a **spike with a
`lowering-blocker` test** (VOS-040d), and nothing is promised on it. Its result
is not read from the README: section 8 item 2 records that only the README was
read. **Amended by VOS-040-0 (owner/coordinator decision, 2026-11-25):** a successful lowering is reported as **"lowered helper"** and is
**not evidence yet** (owner: "Not evidence yet (Recommended)"): the implementation
keeps "no SAT evidence", and VOS-D146 decides whether the template's line 10
changes. The time box, the success, failure and not-acceptable criteria are in
[VOS31](VOS31-host-model-sat.md) section 13.

**What is nothing until VOS-029: proof on compiled code.** The first proof is
on **the object from the VOS22 spike build**, not on a gated image; binding that
object's digest to the gated image's is a later step (VOS-D078, VOS30 section
5.2). After VOS-029 each
implementation gains:

* `sat/obligations.json`: per obligation, the kind, instance, path, CNF
  digest, DRAT digest, conflict count, whether it passes under the **default
  `drat-check` limits** (VOS30 section 6.3, VOS-D042), the `collapsed` flag
  (VOS30 section 4.5) and the SAT sabotage control;
* a BUILD line `vos-x3 check <crate>` that regenerates the CNFs, re-solves and
  checks the proofs.

**The manifest is committed, the proofs are not.** A DRAT proof can be hundreds
of megabytes (VOS-D044); the digest in the manifest lets the check regenerate
and compare. Section 2's X3 column says which obligation kinds apply to each
interface and whether case-splitting is needed. Until VOS-029 that column is a
plan, and no report may call a component "proved".

### 5.4 Template gaps: why it is not mechanical yet

The template above is what every component should look like. Six facts make
copying it today produce known-bad output, and each has a row that closes it:

1. **`check_evidence.sh` was an exception list.** It tolerated exactly three
   `init` and `field-zero` controls that cannot pass because the initial state
   is already zero (VOS-D065). VOS-038a landed `field-flip` and deleted
   fp-bitmap's script; closing VOS-D098 (every claim that calls an operation
   attacks its control) deleted rr's narrowed script too. A new component
   binds `init` with `field-flip` and needs no script.
2. **`heap <= 0` is falsified by the harness** for non-scalar results, because
   the harness's own marshalling allocates (VOS-D063). Until it is fixed a
   component can only declare `heap 0` on scalar operations, and the report
   must not claim it elsewhere.
3. **Array-state builders are rejected by the generator** (VOS-D025: W0701 on
   `fixed<8>`). Every table component from `thread` on hits it. Either fix the
   builder, or adopt a **record-of-fields ghost image** as the rule (the
   fallback of VOS-038).
4. **Coverage is measured once by hand and gates nothing** (VOS-D070). A
   component's line coverage is claimed in a README and nothing fails if it
   drops. **Closed by VOS-039a** (section 5.6, item G).
5. **The lemma check was a per-contract crate** (VOS-D049): every contract would have
   copied `vos-sched-contract-check`. **Closed by VOS-039b-1** (section 5.6, "Contract
   crates"): `vos-link check-contract` is generic and the crate is deleted.
6. **No mechanism evidences one implementation at two configurations**, a
   reduced X1 configuration and the shipped X2 configuration, because
   `contract.vos` pins one configuration (VOS-D068).

The component kit (VOS-039) is the answer to the whole list: after three real
implementations exist, the kit extracts what they have in common into a
`vos-link check-component` that verifies the file set, the BUILD lines and the
build-tool row, so the template is **checked**, not remembered. VOS-039 is
delivered in four slices: **039a** `gen-build`, `check-component` and the coverage gate
(section 5.6, as built); **039b** the contract kit, split into **039b-1** (the generic
contract check `check-contract`, the `contract` kind in `gen-build` and `check-component`,
the retirement of `vos-sched-contract-check`, the `config` half of VOS-D055; section 5.6)
and **039b-2**, itself split: **039b-2a** (the command-type check SH19, VOS-D077, and the
Rust-enum/Warrant-variant correspondence CK10; section 5.6, as built) and **039b-2b** (`bound` rows
against ceilings, CC09, with the dependency and attribute closure of a contract crate, CC10 and SH19 (h), VOS-D112, and
the VOS-D104 sweep, `check-publish`; section 5.6, as built); **039c** (done) the oracle-preservation claims
of every `-any` (VOS-D080) and the kit rule CC11 that requires the fault that attacks them (section 5.6, as built); **039d** the step-counter tick audit (VOS-D089); the migration of the VOS
sabotage lines to `code/scripts/warrant_negctl.py` (VOS-D099, closing VOS-D088)
moved into 039a, because the generated BUILD uses it.

### 5.5 What a reviewer checks in a component PR

`gen-build` and `check-component` (section 5.6) now produce and check the file-set, BUILD-line, guard, selection-row and coverage rows of this list; the rest a reviewer still applies by hand.
Each item has a mechanical check or an artifact to look at, so a reviewer is
reading evidence and not opinion:

| Item | Where to look | Pass condition |
|---|---|---|
| The contract names a mechanism | The spec section and `contract.part` | No profile, role or other interface named (rule 3) |
| The "basic" test is answered | The PR description, in the form of section 4.1 | All four clauses of rule 12 |
| Case counts are derived | The generated table in the contract PR | Produced by a run; every claim at most 262,144 cases |
| Controls can fail | BUILD line 9 and `negctl` entries in the manifest | Every control reports `not ok` on its sabotaged build |
| Every BUILD line 6 to 10 ran | The BUILD log and the manifest | Each line's "none ran = fail" condition holds; a missing line fails (section 5.2) |
| Property tests are present | The BUILD log | 4096 cases for every claim, read from the output |
| Fuzz is present | The BUILD log | 5000 iterations; for a parser the corpus and length bounds of line 8 are committed |
| Sabotage covers every operation and tier | The `negctl` entries | Controls run equal operations times tiers; every one is `not ok` |
| Commands are a closed enum | The contract package | No variant can carry a raw address or CSR (rule 6); the negative control rejects a mutant |
| An `-any` exists | `vos-<iface>-any` and its `vos.toml` kind | `any` block, no canonical block, SH16 |
| The footprint is empty | `required_capabilities.json` | Empty, and BUILD line 3 passes |
| Reduced and real width are both reported | The evidence report | Both widths printed (rule 4); the real width by property or fuzz |
| Time-carrying contracts state an overflow rule | The contract text | Saturating or a horizon precondition (rule 7) |
| Nothing claims more than it has | The README and the report | No "proved" before VOS-029; the implementation's report prints "no SAT evidence"; lemmas printed as "SAT-certified at N = ..." only, in the contract PR |
| The selection row exists | `repository_inputs.go` | A tool edit selects the crate in a dry run |

### 5.6 `vos-link gen-build` and `check-component`, as built (VOS-039a)

```text
vos-link gen-build        <crate-dir> [--root <repo-root>]   writes BUILD, BUILD_windows (and warrant/check_evidence.sh if declared)
vos-link check-component  <crate-dir> [--root <repo-root>]   exit 0 pass, 1 findings, 2 no verdict
```

**Why the BUILD files are generated and compared, not linted (decision, from the
security review of the first VOS-039a draft).** The first draft matched required
fragments in the author's BUILD text. A match over author-controlled text can be
satisfied by text that never runs, and the build tool runs **each non-comment BUILD
line as its own `sh -c`** (it drops blank and `#` lines), so single-line tricks pass a
lint: `echo cargo test -p C ...`; `true # cargo test ...`; `false && <line>`;
`<line> || true`; a `uname` test whose `"Linux"` sits in a comment; a second
`--fail-under 0`; `--exclude-files`; `echo warrant/check_evidence.sh`; the strings a
script check looks for placed in the script's comments; an excuse matched by
`contains` on a short or empty id; Windows `::` and `@rem` comments. That is the
failure VOS-012b0 had already met (a lint over author-written text never
converges). So the **canonical region of each BUILD file is generated**, exactly as
the document is by `gen-doc`, and `check-component` requires **byte equality**, like
`check-doc`. There is nothing left to bypass: the lines are not recognised, they are
reproduced.

**Scope (decision).** An **implementation crate**: kind `production`, `reference`
or `any`, and, since VOS-039b-1, a **contract crate** (kind `contract`, below). Any other kind
is a finding (`CC01`): fixtures and evidence-only check crates are not components. For them
VOS-D082 (`publish = false`) is swept by `vos-link check-publish` (VOS-D104, 039b-2b); the one check crate
that existed, `vos-sched-contract-check`, is deleted.

**What `gen-build` reads.**

| Input | Gives |
|---|---|
| `vos.toml` | `kind`, `evidence-features` (each `[a-z][a-z0-9_-]*` and on the central list `vos-shape-check` uses, `DEFAULT_EVIDENCE_FEATURES`: the value is pasted into BUILD, so it is validated), `coverage-floor` (an integer, at least 95), `excused-controls` (optional, below) |
| `evidence.bindings` (read by the evidence generator's own reader) | the bound operations and their faults |
| `code/specs/<crate>.warrant` (parsed, by `vos-manifest`) | the claim and example names |
| `warrant/sabotage.plan` | which claim or example attacks which fault (below) |

**The canonical region.** The file starts with a generated header, then these lines
in this order (section 5.2), then the marker line
`# --- crate-specific (run, but never counted as evidence) ---`. Every line is
exactly one command; the crate name, features and floor are substituted.

```text
 1  cargo test -p C --lib --test behaviour -- --nocapture
 1  cargo test -p C --features F --lib --test behaviour          (once per feature list; none if no evidence feature)
 1  cargo test -p C --doc
 2  cargo run -q -p vos-shape-check -- .
 3  cargo run -q -p coding_adventures_ca_capability_analyzer -- --dir .
 4  cargo run -q --manifest-path ../../../programs/rust/vos-link/Cargo.toml -- check-doc .
 5  cargo run -q -p coding_adventures_warrant_grammar --example gen_evidence -- --check C
6-8  rm -f evidence/test-manifest.json         (BUILD_windows: if exist evidence\test-manifest.json del /f /q evidence\test-manifest.json)
     WARRANT_EVIDENCE_OUT=evidence/test-manifest.json cargo test -q -p C --features F --test warrant_evidence
       (when `excused-controls` is declared: `sh warrant/check_evidence.sh`; BUILD_windows: `bash ...`)
 9  one line per plan entry:
       WARRANT_EVIDENCE_FAULT=<fault> [WARRANT_EVIDENCE_ATTACK=<op>] WARRANT_EVIDENCE_FILTER=/<tier>/<name> \
         python3 ../../../scripts/warrant_negctl.py -- cargo test -q -p C --features F --test warrant_evidence
       (BUILD_windows: `set X=..&& ... python ../../../scripts/warrant_negctl.py -- ...`)
 G  if [ "$(uname)" = "Linux" ]; then cargo tarpaulin -p C --features F --lib --test behaviour --ignore-tests --engine llvm --include-files 'C/src/*' --fail-under <floor> --skip-clean; fi
       (BUILD only: tarpaulin is a Linux gate; `if ...; fi` rather than `&&` so the line exits 0 elsewhere)
 +  cargo run -q --manifest-path ../../../programs/rust/vos-link/Cargo.toml -- check-component .
```

Line 9 uses the repository's shared checker `code/scripts/warrant_negctl.py`
(WD-HARN1c, the normative form of a sabotage line, VER04j item 53): it runs the
command with no shell, and passes only on a non-zero exit, exactly one TAP plan, at
least one non-TODO `not ok`, no `Bail out!` and in-order point numbers. This
completes the BUILD half of VOS-D099 and closes VOS-D088. The header carries
`# needs-toolchain: python`. Anything after the marker is the author's: it is run,
and it can satisfy no requirement, because the requirement is the canonical region.

**The sabotage plan** `warrant/sabotage.plan`: one entry per line,
`<tier>/<name> <fault> [attack=<op>]`, `#` comments. `<tier>` is `exhaustive` (the
name must be a claim of the document) or `example` (an example of the document);
`<fault>` must be bound to some operation in `evidence.bindings`; `field-flip` takes
`attack=<op>` (the record fault that cancels itself if two operations list it,
VER04j 4.8) and nothing else does (`field-flip-last`, VOS-039c, takes none: nothing reads the register it corrupts, and CC11 below requires it of every state-producing operation of an `-any`); no entry repeats. **Completeness:** every fault of
every bound operation has an entry, and every operation that lists `field-flip` has
an `attack=` entry; and **every claim whose text calls a bound operation appears in at
least one entry** (found by walking the claim's parsed expression, `vos_manifest::warrant::claim_calls`; calls through a `model` body are not followed, VOS-D108; the
one exemption is a claim whose operations' faults are all excused, below). An author
cannot attack one easy claim per fault and leave the rest untried. Counting controls per
operation and tier is VOS-D105. Whether the named claim really catches the fault is the run's
(the shared checker fails a control that does not). The plan cannot be derived from
the document and bindings alone, because the claim-to-operation map is inside the
Warrant expressions; deriving it is VOS-D108. The plan holds names, never command
text, so it cannot carry a shell fragment.

**The tolerance script.** A crate whose only claim over an operation cannot see a fault
(rr's `init`, VOS-D098) declares `excused-controls = [{ id = "<manifest id>", reason = "VOS-Dnnn" }, ...]` in
`vos.toml`: whole ids of the form `warrant/<module>/negctl/<tier>/<op>/<fault>`, never an
empty list, never an empty id, and every excuse carries a `reason` that is a backlog row
(`VOS-D` and three digits) which **must be the first cell of exactly one row of `code/specs/VOS-BACKLOG.md`
whose status cell begins `open` or `queued`** (an allowlist: `resolved`, `withdrawn`,
`done in ...` and the rest are closed): an excuse is traceable and expires when its row is closed (the control must then
pass). `gen-build` writes `warrant/check_evidence.sh` from the list
(it requires exit status 1, at least one `ok`, and exactly those ids failing).
`check-component` requires the script byte-equal to the generated one, and refuses a
script when nothing is declared.

**What `check-component` checks.** A rule that cannot be applied (a symlink, a file
over its cap) is *no verdict*, exit 2.

| Code | Check | Fails when |
|---|---|---|
| `CC01` | The kind | `vos.toml` is missing, malformed, or names a kind that is not production, reference, any or contract |
| `CC02` | The file set (section 5.1) | `BUILD`, `BUILD_windows`, `README.md`, `CHANGELOG.md` or `CHANGELOG.d/_meta.md`, `Cargo.toml`, `vos.toml`, `required_capabilities.json`, `warrant/impl.part`, `warrant/sabotage.plan`, `evidence.bindings`, `evidence/test-manifest.json`, `tests/generated/warrant_evidence.rs` or `code/specs/<crate>.warrant` is absent |
| `CC03` | BUILD and BUILD_windows, lines 1 to 9 and G | the file does not begin with the generated canonical region, byte for byte (the finding names the first differing line, what is expected, what is found) |
| `CC04` | The inputs of generation | `evidence.bindings` or the document does not parse; the plan is malformed, names an unknown claim, example, fault or operation, repeats, or leaves a bound fault or `field-flip` operation unattacked; the tolerance script differs from the generated one, or exists with nothing declared |
| `CC05` | The build-tool row (line 11) | `repository_inputs.go` has no `vosImplementationRow("<crate>"),` line inside the `repositoryInputConsumers = []repositoryInputConsumer{` slice, after `//` and `/* */` comments and raw strings are removed (a Go parser is not a dependency of `vos-link`, so this is a narrow, documented scan; a row in a comment, a raw string or another slice does not count) |
| `CC06` | VOS-D082 | kind `any` and `Cargo.toml` lacks `publish = false` in `[package]` |
| `CC07` | The coverage declaration (VOS-D070) | `vos.toml` has no `coverage-floor`, or it is below 95. The BUILD line that enforces it is part of the canonical region (`CC03`) |
| `CC08` | The evidence manifest | see below |
| `CC09` | `bound` rows against the ceilings (VOS-039b-2b, VOS-D055) | see below |
| `CC10` | The dependencies and build of a contract crate (VOS-D112), kind `contract` only | `Cargo.toml` has an entry in `[dependencies]`, `[build-dependencies]` or any `[target.*.dependencies]` table, a `build` key or a `build.rs`, a `[lib]` `proc-macro` that is not the literal `false`, a `[lib]` `crate-type` naming `proc-macro`, or a top-level `[patch]`, `[replace]` or `[workspace]` table |
| `CC11` | Oracle preservation (VOS-039c, VOS-D080), kind `any` only | see below |

**CC08, the manifest, bound to the document and the source.** `evidence/test-manifest.json`
is read by the repository's `bounded-json`. It must have schema `warrant-test-evidence-v1`;
its `spec_sha256` must equal the SHA-256 of `code/specs/<crate>.warrant` (the digest the
generated harness embeds) and its `source_sha256` the digest of the crate's `src/` tree
under the harness's own framing (`test-bootstrap`'s `source_tree_framing`), so a manifest
of another document, older source or another crate is refused. The generated region also
removes the manifest before the run (`rm -f`, or `if exist .. del` on cmd: both safe when
the file is absent, so idempotent), so a suite that exits 0 without rewriting it leaves no
file and fails `CC02` instead of passing on a stale one;
the claim names of its `exhaustive`, `property` and `fuzz` entries must each **equal
the document's claim set** (the manifest cannot drop a claim or invent one), and its
`example` entries the document's examples; every `exhaustive` entry has `cases` > 0;
every `property` entry `cases` at least the larger of the bindings' `property_cases`
and **4096** (this section, line 7); every `fuzz` entry `iterations` at least the
larger of the bindings' `fuzz_iterations` and **5000** (line 8); every non-control
entry has verdict `pass`; each of the tiers `exhaustive`, `property` and `fuzz` has a
`negctl` entry **with verdict `pass`** (a failing control counts for nothing); and the
set of `negctl` ids that do not pass **equals** `excused-controls` (so a stale excuse
fails as surely as an unexcused failure). The numbers 4096 and 5000 are constants of
`vos-link` whose test reads this file and asserts that this section states them.

**Not checked yet, each with a row.** The exact "controls equal operations times tiers"
count of line 9 (VOS-D105); a parser's fuzz corpus and bounds (VOS-D106); the
command-type check of line 2 on IMPLEMENTATION crates (a leaf has no command enum of its own; SH19 binds the contract crate, 039b-2a); deriving the sabotage plan (VOS-D108); line 10
(SAT, n/a until VOS-029).

**Item G, the coverage gate (VOS-D070).** `cargo tarpaulin`, which CI installs on
Linux (pinned, `--locked`, with `llvm-tools`), on the LLVM engine, with
`--include-files '<crate>/src/*'` (without it tarpaulin scores the whole workspace)
and `--fail-under <floor>` where the floor is `vos.toml`'s `coverage-floor`
(the repository standard of 95, never lower). The gate line is generated, so the
floor and the line cannot disagree. A run that measures no line of the crate exits
non-zero (checked by running tarpaulin with a pattern that matches nothing: `No
coverage results collected`, exit 1).

**Negative controls.** The fixture crate in `vos-link`'s tests is generated by
`gen-build`, then broken one way at a time. Each bypass of the first draft is a test:
`echo`-prefixed, `true #`-prefixed, `false &&`-prefixed and `|| true` lines, a
`uname` test with `"Linux"` in a comment, a second `--fail-under 0`,
`--exclude-files`, `echo warrant/check_evidence.sh`, the script's strings in comments,
an empty or short excused id, and Windows `::`, `@rem`, `rem` lines; plus a missing
file, a missing line, a plan that drops a fault, a manifest that drops a claim, a
property count below 4096, a control that does not pass, a missing or lookalike
build-tool row (comment, raw string, another slice), and a kind-`any` crate without
`publish = false`. Each fails with its own code and text.

### Contract crates: `check-contract`, and `check-component` for kind `contract` (VOS-039b-1)

```text
vos-link gen-contract    <contract-crate-dir> [--root <repo-root>]   writes warrant/lemmas.table
vos-link check-contract  <contract-crate-dir> [--root <repo-root>]   exit 0 pass, 1 findings, 2 no verdict
vos-link gen-build / check-component   accept a crate of kind `contract` (the branch below)
```

**The principle is the one of 5.6 above: generate the canonical artifact, compare it byte for
byte, never lint author text.** `check-contract` does not read a list of controls the author
wrote and trust it; it *runs* the contract's lemmas and mutants, and writes what happened into
`warrant/lemmas.table`, which `gen-contract` writes and `check-contract` regenerates and
compares byte for byte. The table is never parsed.

**What it reads.** `vos.toml` (kind `contract`), `contract.vos`, `warrant/contract.part`,
`warrant/lemmas.part` (now required for a contract crate), the `warrant/obligations-<tier>.part`
files (their correspondence with the `tier` rows is the existing GD03, factored into a shared
`load_contract`), `warrant/mutants/<lemma>.part` and `warrant/lemmas.table`.

**Evaluation.** A document is assembled in memory: the header, one generated
`model cfg_<KEY>() -> T = V<t>` per `config KEY V` row of `contract.vos` (T is the smallest of
U8, U16, U32, U64 that holds V), `contract.part`, then `lemmas.part`. It is parsed and
type-checked by `warrant-grammar` and each lemma is evaluated by the repository's ground
`Evaluator` over every value of its binders (a complete type: `Bool` or `U8`). The odometer is
the one the retired crate used, returning findings instead of panicking. Workers run on
`WARRANT_STACK_BYTES` threads, at most 8, and results are collected by index, so the output is
deterministic.

**Mutants need no new file format.** `warrant/mutants/<lemma>.part` is an ordinary admitted part
holding **exactly one** `model` declaration with a body. The tool parses it with the Warrant
parser and replaces the model of the same name in `contract.part` **by the declaration's span**
(no text search). A mutant is not gameable because: the file stems equal the lemma names, both
ways; the mutated document must type-check; the named lemma must evaluate without error and
**fail at least one case** (an evaluation error is a finding, never a kill); and the kill count
is published in the generated table, so a reviewer sees it change.

**Configuration, generated (the `config` half of VOS-D055).** For each `config` row the tool
generates the `cfg_<KEY>` model above, and holds it to two rules. Each key is **called by at
least one lemma** (found by walking the parsed claim body, as `claim_calls` does). And two
tool-generated mutants, V-1 and V+1 (a side that does not fit T is skipped), must each falsify at
least one lemma that calls the key; the author cannot write them. The scheduler contract carries
`config_n_is_the_id_range: forall t: U8. pre_insert(empty(), t) == (t < cfg_N())`, so
`contract.vos`'s `N 8` is tied to the Warrant `pre_insert` model: changing either alone fails.
`lemmas.part` is not embedded in any implementation's document, so no document digest moves.
What stays open: the literal `8` in an implementation's `n_is_eight` example against
`contract.vos` (`bound` rows against ceilings are `check-component` CC09, 039b-2b).

**`warrant/lemmas.table`** (generated):

```text
vos-lemmas 2
contract-part <sha256 of contract.part>
lemmas-part <sha256 of lemmas.part>
config N 8
variant <Name> = <C1> | <C2>(<T>, ..)
lemma <name> product <n> admitted <n> mutant <model> mutant-part <sha256> failed <n>
config-mutant N 7 kills <lemma>[,<lemma>]
config-mutant N 9 kills <lemma>[,<lemma>]
end
```

| Code | Check | Fails when |
|---|---|---|
| `CK01` | Inputs | the crate directory, `warrant/` or `warrant/mutants/` is a symlink; `vos.toml` kind is not `contract`; a part is not admitted; `contract.vos` does not parse; a required file is missing (tier/part correspondence is the shared GD03) |
| `CK02` | Type check | `contract.part` + the cfg preamble + `lemmas.part` does not type-check; `lemmas.part` holds anything but claims, or no claim |
| `CK03` | Budget and domain | a lemma is not a `forall`; a binder type is not `Bool` or `U8`; the product of the binder domains exceeds **400,000** (VOS04 5.4, VER04h 3); `lemmas.part` holds more than **64** lemmas; no case is admitted by the refinements |
| `CK04` | The lemma holds | a lemma fails on some case (the first failing binder values are printed), or does not evaluate |
| `CK05` | Mutant set | the stems of `warrant/mutants/*.part` are not exactly the lemma names |
| `CK06` | Mutant form | a mutant part is not exactly one `model` with a body, names a model the contract does not declare, does not type-check, or its lemma does not evaluate on it |
| `CK07` | Mutant kills | a lemma survives its mutant (zero failing cases) |
| `CK08` | Config | a config key is called by no lemma, or a generated V-1 or V+1 mutant falsifies no lemma that calls it |
| `CK09` | The table | `warrant/lemmas.table` is absent or differs from the regenerated bytes (the first differing line is printed) |
| `CK10` | Enum/variant correspondence | a Rust `enum` of the crate has no Warrant `variant` of the same name, or the two generated lines differ (both lines are printed) |
| `CK11` (planned, VOS-040c; **Amended by VOS-040-0 (owner/coordinator decision, 2026-11-25)**) | Model-SAT line | `vos.toml` says `sat = "model"` and the BUILD lacks the generated `vos-model-sat check .` line, `warrant/lemmas.sat` is absent, names fewer than every lemma at every ladder width, or holds zero obligations (none ran = fail). Specified in [VOS31](VOS31-host-model-sat.md) section 10 |

The budget is VER04h's X1 per-claim case budget of **400,000** (the `exhaustive_budget` of every
`evidence.bindings`); it is a constant of `vos-link`, and a test of `vos-link` reads this section
and asserts that it states the number.  A `lemmas.part` is limited to **64** lemmas (the limit of the
mutants directory), so the work of a run is bounded; and when CK02, CK05 or CK06 findings exist, the
check returns them without evaluating anything.

**Enum/variant correspondence, CK10 (VOS-039b-2a).** The Rust `enum`s of the contract crate and the
Warrant `variant`s of `contract.part` must say the same thing, so a renamed constructor, a changed
payload or a reordered variant is caught at the contract and not only by a failing generated harness.
The check follows the one principle of this section, **generate and compare**: it renders a line from
each side with ONE function and compares the bytes.

```text
variant Pick = Idle | Run(U8)          <- rendered from the Rust enum  (src/**/*.rs)
variant Pick = Idle | Run(U8)          <- rendered from the Warrant variant (warrant/contract.part)
```

The Rust side is `vos_shape_check::items::collect_types` over the crate's `src/**/*.rs` (the RS01
lexer; no new lexer), the Warrant side the AST of `warrant-grammar`'s parser (`VariantDecl`).
`variant <Name> = <C1> | <C2>(<T>, ..)`: constructors in source order separated by ` | `, a
payload as the comma-and-space joined TYPES. The type map is `u8` to `U8`, `bool` to `Bool`,
`[u8; N]` to `Bytes<N>`, a same-crate name to itself, and anything else to `?` and the type as written
(a character no Warrant type has, so it never matches). Warrant names a constructor's fields freely, so a
Rust TUPLE variant's field names are not compared: `Run(t: U8)` renders `Run(U8)` (the harness marshals
by position). A Rust NAMED-field variant (`Put { slot: u8 }`) is different, its field names are part of
the Rust type, so it renders `Put(slot: U8)` and the Warrant constructor of that name must spell the same
names: the one rendering function is told by the Rust side which constructors are named. CK10 fires when
a Rust enum has no Warrant variant of its name (a rename on either side leaves a Rust enum without a
twin) or the lines differ. The direction is Rust to Warrant: a Warrant variant with no Rust enum is a
model-only type and is allowed. A Rust type SH19 would reject cannot render (the Rust side prints it
as written, which never equals a Warrant type), so SH19 and CK10 agree.

The rendered lines are also published, in the same order as the Rust enums appear in the source,
as `variant ...` rows after the `config` rows of `warrant/lemmas.table` (`vos-lemmas 2`), so a reviewer
sees the correspondence in the generated artifact and CK09 byte-pins it. What this does NOT check:
the evidence bindings' `enum ... ctors=` lines against the contract. The generated harness compiles
against the contract's own enums (VOS10 section 12, fate table), so a mismatch fails the harness.

**The command-type check, SH19 (VOS-D077, built in VOS-039b-2a).** `vos-shape-check`'s rule SH19
(VOS03 section 4) is checked when `vos.toml` says `kind = "contract"`. The command set is derived,
never listed by the author: **every `enum` in the crate**, `pub` or not. An enum fails when it
carries `#[non_exhaustive]` (on the enum or a variant) or a generic list, or when a variant field is
not one of exactly `u8`, `bool`, `[u8; <integer literal>]` or the bare name of a same-crate `struct`
or `enum` that itself passes. So `usize`, `u16` and wider, `i*`, references, pointers, tuples,
`Option<..>`, any `::` path, a `type` alias (not resolved, therefore rejected) and a const-named
length are all findings. Negative controls, each asserted by code AND text: `Poke(usize)` and
`Write { addr: u32 }` are rejected, `Pick::Run(u8)` and `Upd` pass; `#[non_exhaustive]`, `Cmd<T>`,
`X(&'static u8)`, `X(core::num::NonZeroU8)`, `X(Addr)` with `type Addr = usize`, and a nested struct
with a `usize` field are rejected; `[u8; 4]` and a nested passing enum pass; the same enum in a
`kind = "production"` crate raises no SH19; the real `vos-sched-contract` passes.

**The vocabulary of a contract crate is closed (security review of 039b-2a).** The four spellings are
only safe if the four names mean what they look like, and a lexical check cannot resolve names. So
SH19 refuses, as whole constructs, everything that can change a name's meaning: any `use` or
`extern crate` (a contract crate imports nothing), any `mod` (one flat namespace), any macro
definition or invocation, and any item named like a Rust primitive or a Warrant built-in type
(`type u8 = u32;`, `struct u8`, `struct U8(u8)`). Attributes and doc comments stay; tests live in
`tests/`. An `enum $n` that a macro body would write is a VT03 gap in every crate. Each bypass the
review reproduced is a negative control in class K, P and H. The real `vos-sched-contract` uses none
of these. CK10 compares only the NAME of a nested same-crate type; SH19 judges its contents (VOS-D111).

**`bound` rows against ceilings, CC09 (VOS-039b-2b, the rest of VOS-D055).** VOS02 section 4.7 specifies a
`realises=` field that would say which interface operation an `op` of the document realises. It is not
built: `warrant-grammar`'s binding reader rejects an unknown `op` option (a change to a verification crate with
a large fan-out), and an author-declared mapping is gameable (point `insert_img` at the loosest ceiling). So
the mapping is DERIVED from the generated document `code/specs/<crate>.warrant`, parsed by `warrant-grammar`
(never a text scan), and compared with the parsed `contract.vos`:

```text
ceil(X) = X                                  when contract.vos has a ceiling row for X
        = X with ONE suffix `_out` or `_img`  when that is a ceiling operation   (VOS02 4.4's interim vocabulary)
        = none                               otherwise: ghost-only (`alpha_w`, `cfg_n`), listed in a note
```

The suffix list is the constant `MAP_SUFFIXES` in `vos-link`; a test reads this section and asserts it. CC09
fails when: (a) a `bound` row of a mapped operation has a metric the contract ceils and a bound that is not a
plain integer literal (a non-literal is an unknown, which fails closed), or a value above the ceiling of the
mapped operation and metric; (b) for kind `production` or `any`, a mapped operation has no `steps` row or no
`heap` row (the two metrics every scheduler declares; a missing row is an unbounded operation); (c) a ceiling
operation has no mapped operation at all (a contract operation nobody realises). `depth` is compared when
present and otherwise reported as a note, "not evidenced" (VER04h 7.3 has no depth observer yet; requiring it
would move every digest). Kind `reference` has rows that are optional and compared when present; a `reference`
crate with no `bound` row passes. A `bound` row of an unmapped (ghost) operation is not compared and is listed
in a note. Not checked: that the bound is the TRUE worst case (that is the evidence's step counter, VOS-D089),
and the profile-side comparison against the LOCKED ceilings digest, which stays with `check-profile` (VOS02 NC9).
Negative controls, each in a fixture crate: `steps <= 161` on `insert_img` only; `heap <= 1`; a deleted `heap`
row; a deleted `bound remove_out` block (a ceiling operation with no operation); a non-literal bound; and the
passing cases (a `reference` crate with no rows; an `alpha_w` row above every ceiling).

**The sweep, `vos-link check-publish [--root R]` (VOS-D104, codes PB01, PB02 and PB03).** `check-component` judges
implementation and contract crates; the evidence-only crates (a check program, a probe, the tooling itself)
are nothing to it, and `publish = false` (VOS-D082) used to be applied by hand. The sweep is over the DIRECT
children named `vos-*` of `code/packages/rust` and `code/programs/rust` that hold a `Cargo.toml` (no recursion:
a fixture tree has no `Cargo.toml` of its own at that level). A crate's `vos.toml` is read with the shape
checker's reader; kinds `production`, `reference` and `contract` are EXEMPT, with a note that the publish
policy of a shipped crate is undecided and belongs to the release gate (that half of VOS-D104 stays open),
but ONLY when the crate is also registered in `code/programs/go/build-tool/repository_inputs.go` by a
`vosImplementationRow("<crate>")` or `vosContractRow("<crate>")` line inside `repositoryInputConsumers` (the
scanner of CC05, so comments and raw strings do not count): the kind is the author's word, and a tooling crate
must not escape PB01 with a one-line `vos.toml`. A kind claim without a row is PB03 (register it, so
`check-component` runs in its BUILD, or set `publish = false`); a missing or unreadable row file, when a crate
needs it, is no verdict. Every directory name printed in a message is escaped (`esc`), so a hostile name cannot
forge a report line.
Every other crate (kind `any`, no kind, no `vos.toml`) must say `publish = false` in `[package]` (read with the
`toml_reader`, as CC06 does): PB01 when it is missing or not the literal `false`; PB02 when its `Cargo.toml` or
`vos.toml` cannot be read as TOML. A symlinked crate directory is NO VERDICT (exit 2). The sweep runs as one
identical line of `vos-link`'s BUILD and BUILD_windows, and a build-tool row selects `vos-link` when any
`code/{packages,programs}/rust/vos-*/Cargo.toml` or `vos.toml` changes. It forced `publish = false` onto
`vos-manifest`, `vos-probe-ready`, `vos-link` and `vos-capacity-probe`.

**Oracle preservation of every `-any`, CC11 (VOS-039c, VOS-D080).** The safety claims quantify over `build(M)`, whose
oracle is a constant, and `alpha` drops the oracle, so an `insert` that overwrote the oracle passed every generated
claim of `vos-sched-any` (the hand-written behaviour test caught it). The contract now carries the claims, in
`warrant/obligations-any.part` (VOS10 9.3): `insert_oracle_lo`, `_hi` and `_ids`, the three of `remove`, and
`alpha_build_with_oracle`, so CD07 requires them of every `any` crate by `claim_sha256` and no implementation
authors them. A claim only proves what its fault can fail, and no record fault reached the last field of `Impl`, so
VER04j 4.8 gained `field-flip-last`. CC11 is the generic half, DERIVED from the parsed document, the contract's `any`
claims and the bindings, never declared by the author, and it never reads claim text:

```text
  (a) the Impl field outside the cone of `build` (the oracle; `BuildShape::Constructor { in_cone }`, the walk
      CD07 uses) is exactly ONE field, it is the LAST field of `Impl`, and its type is NUMERIC (a fixed-width
      integer or `Nat`: the generator's own predicate for `field-flip-last`, the `FixedTy` table); anything
      else, and any shape the walk cannot decide, fails closed
  (b) every SUCCESSOR operation lists the fault `field-flip-last` in `faults=`; the successors are the `op`s of
      the GENERATED DOCUMENT whose return type mentions `Impl` and that have a parameter that mentions `Impl`,
      or an `inout` parameter that does (`insert_img`, `remove_img`, `dispatch_img`); "mentions" is judged on
      the RESOLVED type, the generator's own resolution (`warrant-grammar` `evidence::type_mentions`, over the
      `TyEnv` that `ty_of_type` uses), so `model State = Impl` (or a chain of aliases, an alias of
      `Option<Impl>`, a record with such a field) is a state; the bindings are read only afterwards, for each
      successor's faults
  (c) for every successor operation `sabotage.plan` has an entry `exhaustive/<claim> field-flip-last` whose claim
      is CONTRACT-OWNED (the document's claim has the name and `claim_sha256` of an obligation of the contract's
      `any` tier, the identity CD05 and CD06 use; never the `tier=` attribute) and calls that operation and no
      other successor (the existing `claim_calls`)
```

The successors come from the document because `ret=` and `args=` of an `op` line are optional: omitted, the
generator falls back to the host type of the document's signature, so `op insert_img export=.. faults=field-zero`
is still a state-to-state operation for the generator, and a rule that read the bindings' spelling would not see
it (VOS-039c security review F1). "Mentions" includes `Option<Impl>`, `Result<Impl, E>`, tuples and lists, and is
read through the document's type aliases by the generator's own resolution, not by CC11's own reading of the
text (round-2 finding: `model State = Impl` with `op insert_img(s: State, ..) -> State` was invisible to a rule
that matched the literal name `Impl`, so dropping `field-flip-last` passed; a gate that re-interprets author text
with its own logic does not converge, so CC11 calls the function the generator's types come from). A type the
resolution cannot resolve (an unknown name, an alias cycle, a chain deeper than 64) counts as a state, so a type
the rule cannot classify counts as a state (the generator then refuses `field-flip-last` on a non-record
result, which is a finding); a binding that names no `op` of the document is a finding (it cannot be
classified); a document with no successor is a finding. A claim that the implementation adds itself and that
calls one successor does not satisfy (c) (F3): the contract's preservation claims are the ones that must be
attacked. The contract-owned test trusts the `contract =` package of `vos.toml`, which is author-declared: CD05,
CD06 and CD07 (`check.rs`) judge the document against whichever package `vos.toml` names and the marker
`contract-package=` is compared with the same key (CD01), so none of them catches a swapped contract; a well-formed
fake contract package whose `any` tier lists the author's own claims would make `owned` include them (VOS-D114, tied
to the registry with VOS-D030). The numeric-oracle clause is F2: a `Bool` or `Bytes` oracle leaves `field-flip-last` nothing to flip.

The constant is `ORACLE_FAULT` in `vos-link` (`src/oracle.rs`); a test reads this section and asserts it. Whether
the claim really FAILS under the fault is not decided by `check-component`: the generated BUILD runs the plan through
`warrant_negctl.py`, and CC08 already requires every in-harness control to pass (the successors now have controls of
their own, which closes the part of VOS-D080 that said `insert_img` and `remove_img` had none). A future contract
whose `any` block lacks preservation claims cannot have a compliant `-any`, with no `contract.vos` format change.
Deliberate limits: an oracle that is not the last field, or two oracle fields, is refused (a named-field fault is the
extension, adjacent to VOS-D101); and there is no lexical rule over the implementation's source ("insert and remove
never read the oracle", VOS10 11.2), because it is neither sufficient (a mutant that writes `choice: 0` never reads
it) nor cheap (`..s` and `return s` read the field lexically in the honest code). Negative controls, each a fixture
`-any` crate: a successor without the fault; an entry on a claim that calls no successor, and on an `example`; a claim
that calls two successors; the oracle first, absent, doubled, or not numeric; a `build` that is not a constructor or is
missing; the successor bound with `args=` and `ret=` omitted (each alone, and both), without the fault; a binding that
names no `op` of the document; an `inout` state operation; a decoy claim of the implementation's own carrying the
entry while the contract's claim has none; a document with no state-producing operation; a document the analysis
refuses; and the production crate that is not subject.

**The dependencies and attributes of a contract crate, CC10 and SH19 (h) (VOS-D112).** SH19 is a lexical check
of the crate's own text: a path dependency could supply an attribute macro (`#[evil::rewrite]`) or a foreign
derive (`#[derive(evil::Gen)]`) that rewrites a command enum after SH19 judged it. Two closures, each fail
closed. (1) `check-component` CC10, kind `contract`: `Cargo.toml` has NO entry in `[dependencies]`,
`[build-dependencies]` or a `[target.*.dependencies]` table (empty tables are fine; the real crate has an empty
`[dependencies]` with a comment), no `build` key (`package.build`) and no `build.rs`, a `[lib]` `proc-macro`
absent or `false`, no `[lib]` `crate-type` naming `proc-macro`, and no top-level `[patch]`, `[replace]` or
`[workspace]` (each can redirect or inherit code, so CC10 stands alone and does not lean on SH15). `[dev-dependencies]` are allowed (they run in tests only, SH12 judges that allowlist; the
real contract crate has none). (2) SH19 (h): in a contract crate every attribute, outer or inner, is one of a
closed list: `derive` whose arguments are only the BARE names `Copy`, `Clone`, `Debug`, `PartialEq`, `Eq`,
`PartialOrd`, `Ord`, `Hash`, `Default`; `repr` with arguments from `u8`, `C`, `transparent`; `no_std`,
`forbid`, `deny`, `warn`, `allow` (lists of lint names), `doc` (`= "text"` or `(hidden)`), `must_use`, `inline`, `default`
(the helper of `derive(Default)` on an enum, added to the memo's list for that reason); and `cfg` whose argument is exactly `test`.
Any other attribute is a finding: every path attribute (`a::b`), `cfg_attr`, `path`, `macro_use`, `proc_macro*`,
`test` and anything else. (`non_exhaustive` is its own finding, SH19 (a).) Doc comments stay.

**Cost, measured.** `check-contract` on the real scheduler contract (10 lemmas, 10 mutants, 2
config mutants; N = 8, 65,536 cases per lemma) takes about **10 s wall** (about 57 s of CPU, on 8 worker threads on a 14-core Apple-silicon host; measured twice: 9.84 s and 9.89 s) in a debug build, once, in
the contract crate's generated BUILD. `vos-link`'s own tests use a toy contract fixture with
two threads, so they stay fast (VOS02 D-GD11).

**`check-component` on a contract crate.** `read_component_vos_toml` accepts kind `contract` and
dispatches to a contract branch:

* CC02, the file set: `BUILD`, `BUILD_windows`, `README.md`, `Cargo.toml`, `vos.toml`,
  `required_capabilities.json`, `src/lib.rs`, `contract.vos`, `warrant/contract.part`,
  `warrant/lemmas.part`, `warrant/lemmas.table`, a changelog, and `warrant/obligations-<tier>.part`
  for each `tier` row of `contract.vos`.
* CC03, BUILD and BUILD_windows are byte-equal to `canonical_contract_build(crate)` (the same
  marker, the same trailer rule). The two files are identical, with no environment assignment and
  no python line, so there is no shell syntax for Windows to disagree about:

```text
cargo test -p <c> --lib --tests -- --nocapture
cargo test -p <c> --doc
cargo run -q -p vos-shape-check -- .
cargo run -q -p coding_adventures_ca_capability_analyzer -- --dir .
cargo run -q --manifest-path ../../../programs/rust/vos-link/Cargo.toml -- check-contract .
cargo run -q --manifest-path ../../../programs/rust/vos-link/Cargo.toml -- check-component .
# --- crate-specific (run, but never counted as evidence) ---
```

* CC05, the build-tool row is `vosContractRow("<crate>"),` inside the same
  `repositoryInputConsumers` slice (it selects the crate when a VOS tool or grammar changes; it
  carries no `code/specs/<crate>.warrant`, a contract has no document).
* CC04, CC06, CC07, CC08 do not apply: a contract has no evidence manifest, no `-any` rule and no
  coverable code (VOS10 section 15).

**Negative controls.** The toy fixture contract in `vos-link`'s tests is generated, then broken
one way at a time: a failing lemma, a missing, extra, surviving, ill-typed and two-declaration
mutant, an unused config key, a config-insensitive lemma (CK08), a stale table, an over-budget
binder, a binder that is not a complete type, and the `echo` and `|| true` tricks of the first
039a draft on a contract BUILD (rejected by byte inequality).

**Command-type check (VOS-D077), designed here, built in 039b-2.** SH19 on every `pub enum` of a
`kind = contract` crate, derived, no author list: no `#[non_exhaustive]`; every variant field is
`u8`, `bool`, `[u8; LIT]` or a same-crate type that itself passes; `usize`, wider integers,
pointers, references, generics and foreign paths are findings. Negative controls: fixture contracts
with `Poke(usize)` and `Write { addr: u32 }` are rejected; `Pick::Run(u8)` passes. 039b-2 also
takes the Rust-enum/Warrant-variant correspondence and `bound` rows against ceilings (which needs
`evidence.bindings` to map `insert_out` and `insert_img` to `insert`).

---

## 6. The serial order

The owner chose serial execution. Parallel work on the same toolchain (the
evidence generator, `vos-link`, the build tool) is what produced the stale-base
conflicts of earlier PRs, and the pool multiplies the number of PRs by about
ten. So the queue is explicit, one item at a time, and the backlog carries it
as the "Priority order" section ([VOS-BACKLOG](VOS-BACKLOG.md)).

**In flight first** (not renumbered here): VOS-033 (document generation, done)
and VOS-011 (`vos-sched-fp-bitmap`, in review). Everything below starts after
them.

### 6.1 The first 26 items

| # | Item | What | Why here |
|---|---|---|---|
| 1 | VOS-037 | This spec, plus the backlog surgery | Every later row needs its interface, slot, size and risk fixed |
| 2 | VOS-012a | `vos-sched-any` | The first `-any`; D054 settled by running it (controls attach to `dispatch_img` and `dispatch_out`; no claim rewritten for it) |
| 3 | VOS-D081 | Safety-tier `init` claim | The VOS-012a run showed nothing constrains `init` outside `canonical`; it changes every implementation's safety digests, so it lands before the second implementation of the contract (`rr`) |
| 4 | VOS-012b | `vos-sched-rr` (representation mode) | The first representation-mode implementation (VOS-D061) and the first swap |
| 5 | VOS-038 | Generator debts: D065 non-inert record fault, D063 heap observer, D025 array builder | Before mass production |
| 6 | VOS-039 | Component kit: 039a `vos-link check-component` and coverage gate (done), 039b-1 generic contract check (D049, done), 039b-2a command-type check (D077) and enum/variant correspondence (done), 039b-2b bounds (CC09), D104 sweep and D112 (done), 039c oracle preservation of every `-any` (D080, done), 039d sabotage lines to `warrant_negctl.py` (D099) | After three real implementations exist |
| 7 | VOS-028b | Capacity probe (fixture: `vos-sched-fp-bitmap`) | Fixes the X1 sizes (VOS00 D4) and decides 029's funding |
| 8 | VOS-040 | Host model-SAT lane on the `sched` contract, plus the Rust-lowering spike | Depends on `bit-blaster`, `cdcl-sat` and `drat-check` directly, not on the probe's harness (VOS31 section 7.3; amended by VOS-040-0, 2026-11-25) |
| 9 | VOS-005 | `emu-core` | The emulator has been idle and everything bootable waits on it |
| 10 | VOS-036 | VOS22 hermetic build spec, including a recorded single-crate RV32 object build | 029's first proof needs a compiled RV32 object (D046) |
| 11 | VOS-029a | Symbolic executor core plus simulator differential; first X3: `fp-bitmap` dispatch, insert, remove, on the object from the VOS22 spike build | No cut points needed for the first certificate; binding the object to a gated image is VOS-D078 |
| 12 | VOS-020 | RV32 privileged slice 1 | The machine the spine boots on |
| 13 | VOS-022a | VOS20 K0 spec, static family (the K record, entries, ownership, the boot ladder) | The spine's specification |
| 14 | VOS-C02c | `thread` contract | The first table interface |
| 15 | VOS-C02i | `thread-table` | |
| 16 | VOS-C02a | `thread-any` | |
| 17 | VOS-029b | Cut points, composition checker, index case-split; X3 for `thread-table` | The first table X3 |
| 18 | VOS-006a | `emu-dev-console` (UART) plus the emu-host console | The device the first boot prints on |
| 19 | VOS-025a | VOS21 HAL contract spec (effects as data, the command allowlist of rule 6; ISA and board split) | Before the HAL and the UART driver. Needs no board: the board-specific allowlist is 025b's (item 25) |
| 20 | VOS-C20c | `uart` contract | |
| 21 | VOS-C20i | `vos-uart-poll` | |
| 22 | VOS-034 | `check-profile` plus static SH16, with the requirements of section 7.2 (instance ownership, needed claims, `prot-none`) | The profile gate, before an image is built |
| 23 | VOS-035 | Recorder plus image-build gate | The gate on what was compiled |
| 24 | VOS-021a | `emu-board-rv32-min` (RAM plus UART, no PMP) | The first bootable board |
| 25 | VOS-025b | `vos-hal-rv32` | The first and only `unsafe` |
| 26 | VOS-026a | `vos-spine-static` skeleton with the reset entry; `boot-hello` boots on the emulator (Boot-0) | The first image |

**What the first 26 items do not include.** Item 22 is the *static* part of the
profile gate. Three things must exist before any image is **released** (not
before Boot-0 is built and run on the emulator): CODEOWNERS and branch
protection (VOS-D026, VOS-D067) and the `alpha`-read and oracle-write bindings
checks (VOS-D060, a hard prerequisite of any `image` profile). The backlog
lists them as gates after item 26 and before the Boot-3 release, so the
exclusion is explicit and not an oversight.

### 6.2 Rationale for the first eleven

* **Spec first (1).** The spec is the only item with no dependency and the
  only one that can be wrong in a way that wastes every later item.
* **`-any` next (2).** Without it there is no spine model and no `any` kind has
  ever been exercised; VOS-D054 (the generator's view of `-any` claim
  subjects) could only be settled by a real run. It was (VOS-012a): the
  controls attach to `dispatch_img` and `dispatch_out`, not `alpha_w`, and no
  claim needed a rewrite for it.
* **`init` claim (3).** The same run showed that no safety-tier claim
  constrains `init` (VOS-D081); adding one changes every implementation's
  safety digests, so it lands before `rr` is generated.
* **`rr` (3).** It is the first representation-mode implementation (VOS-D061)
  and the first swap that tests replaceability, and its state is not an
  array, so it avoids VOS-D025.
* **Generator debts before mass production (4).** Otherwise every component
  copies a known-failing exception script and a false `heap` row (section
  5.4).
* **The kit after three real implementations (5).** Extracted from reality (the
  `any`, `rr` and `fp-bitmap` crates), it makes the serial pipeline
  mechanical. Written earlier it would encode guesses.
* **The probe (6).** It fixes the X1 sizes (VOS00 D4), and decides whether
  VOS-029 is funded and what T is for tables. It must precede the first table
  contract (13) and the executor (10).
* **Model-SAT (7).** It reuses the probe's *solver and checker crates* but **not its
  harness** (the probe's `src` is a recorded input, so extracting the harness would
  stale the recording; a shared harness is VOS-D145, after VOS-028c3d signs).
  **Amended by VOS-040-0 (owner/coordinator decision, 2026-11-25):** from then on a contract PR ships SAT certificates for its lemmas **when
  its `vos.toml` says `sat = "model"`**; that is required for `sched` and opt-in
  elsewhere until `thread` has paid the cost once (VOS-D147). See
  [VOS31](VOS31-host-model-sat.md) sections 7.3 and 10.3.
* **`emu-core` (8).** The emulator has been idle, and every bootable thing
  waits on it.
* **VOS22 (9).** 029's first proof needs a compiled RV32 object, and today
  there is no hermetic way to produce one (VOS-D046).
* **029a (10).** It is split so the **first certificate needs no cut points**: a
  proved-on-compiled-code certificate at item 11, on the bitmap scheduler. It
  is proved on **the object from the VOS22 spike build, not on a gated
  image**, and the N = 64 green result of VOS30 table A is on hand-written
  proxy code, so whether real compiler output needs cut points is open
  (section 8 item 5).

### 6.3 The boot ladder

After item 26 the program climbs a ladder of images. Each stage is a profile
that boots, adds components, and adds one claim about the spine. Boot-0 to Boot-2
run before `prot` exists, so their reports and manifests **label them "no
isolation claim"**; `vos-prot-none` is not used by any of them as an image
(section 7.2):

| Stage | Adds | Why this rung |
|---|---|---|
| Boot-0 (item 26) | `boot-hello`: reset entry (fold `init`, install the configuration, **`seal` `cspace` and lock `prot` before the first user instruction**, I11), console | Proves the initial state satisfies the spine invariant before any user instruction. **No isolation claim** |
| Boot-1 cooperative | `ctx`, `sysdec`, `trapdec` | Two tasks that yield: the first context switch. **No isolation claim** |
| Boot-2 preemptive | `timer`, `irq`, VOS-013 | The first interrupt and the first preemption. **No isolation claim** (no `prot` yet) |
| Boot-3 `rtos-min` | `prot-pmp`, VOS-021b, `cspace` (sealed), `ep`, `ntfn` | Two user tasks under PMP, with preemption: the hard RTOS of VOS00 section 11. The first stage that may carry an `isolation` claim |
| Soft RT | `budget`, `edf` | The cheap replaceability demonstration (swap the scheduler, keep every system theorem) |
| Flight | `psched`, `chan`, `sample`, `hm`, VOS-031 | Partitioning and noninterference |
| GPOS, then V | the memory components, then the hypervisor reuses | The hardest toolchain gaps last (VOS00 section 11) |

---

## 7. Profile assembly

A profile is a manifest (VOS02 section 6) that fills slots. This matrix is the
answer to "can I assemble any OS from the pieces": each cell is the
implementation that fills the slot, `-` means the profile has no such slot, and
a name with a trailing "(deg.)" is a degenerate implementation (decision 1.2),
and the marks (xP), (kernel) and [no row] are explained after the table.
It is a **plan**: no cell is evidenced until its row in section 2 is done, and
the cells for G and V are the least settled.

| Interface | H hard RTOS | S soft RT | F flight and separation | G GPOS | V hypervisor |
|---|---|---|---|---|---|
| spine | `spine-static` | `spine-static` | `spine-static` | `spine-gpos` [no row] | `spine-static` |
| `sched` | `fp-bitmap` | `rr` | `fp-bitmap` (xP) | `rr`, later two-level bitmap [no row] | `rr` (vcpus) |
| `edf` | - | `edf-scan` | - | - | - |
| `budget` | `budget-none` (deg.) | `budget-sporadic` [no row] | `budget-periodic` (xP) | `budget-periodic` | `budget-none` (deg.) |
| `psched` | `psched-single` (deg.) | `psched-single` (deg.) | `psched-table` (kernel) | `psched-single` (deg.) | `psched-table` (kernel) |
| `thread` | `thread-table` | `thread-table` | `thread-table` (xP) | `thread-table` | `thread-table` (vcpu slot) |
| `ctx` | `ctx-rv32` | `ctx-rv32` | `ctx-rv32` (xP) | `ctx-rv32` | `ctx-rv32h` [no row] |
| `timer` | `timer-scan` | `timer-scan` | `timer-delta` (xP) | `timer-wheel` [no row] | `timer-scan` |
| `mutex` | `mutex-pcp` [no row] | - | `mutex-pcp` [no row] (xP) | - | - |
| `pool` | - (static) | - (static) | `pool-bitmap` (xP, buffers) | `pool-bitmap` (frames, ASIDs: two instances) | `pool-bitmap` |
| `untyped`, `region`, `cdt` | - | - | - | later [no row] | - |
| `prot` | `prot-pmp-napot` | `prot-pmp-napot` | `prot-pmp-tor` (kernel) | - (MMU via `pt`) | `prot-stage2-static` [no row] |
| `pt`, `aspace` | - | - | - | `pt-sv32` or `pt-sv39`, `aspace-table` [no row] | `pt` in a stage-2 slot, `aspace-table` [no row] |
| `cspace` | `cspace-table` (sealed) | `cspace-table` (sealed) | `cspace-table` (sealed, xP) | `cspace-table` (never sealed) | `cspace-table` (sealed) |
| `ep` | `ep-single` | `ep-fifo` | `ep-single` (xP, partition-local) | `ep-fifo` | - |
| `ntfn` | `ntfn-word` | `ntfn-word` | `ntfn-word` (xP) | `ntfn-word` | `ntfn-word` |
| `sem` | - | optional | - | optional | - |
| `chan` | - | - | `chan-ring` (spine channel, tag-checked) | `chan-ring` (pipes) | - |
| `sample` | - | - | `sample-latch` (spine channel, tag-checked) | - | - |
| `hm` | `hm-halt` (deg.) | `hm-halt` (deg.) | `hm-table` (kernel) | `hm-halt` (deg.) | `hm-halt` (deg.) |
| `irq` | `irq-flat` | `irq-prio` [no row] | `irq-prio` [no row] (xP, disjoint lines) | `irq-prio` [no row] | `irq-hvip` [no row] |
| `sysdec` | `sysdec-rtos` | `sysdec-rtos` | `sysdec-rtos` | `sysdec-sbi` [no row] | `sysdec-sbi` [no row] |
| `trapdec` | `trapdec-rv32` | `trapdec-rv32` | `trapdec-rv32` | `trapdec-rv32` | `trapdec-rv32` |
| `emul` | - | - | - | - | `emul-rv32h` [no row] |
| `load` | `load-flat` | `load-flat` | `load-flat` | `load-elf32` [no row] | `load-elf32` [no row] |
| `uart` | `uart-poll` | `uart-ring` | `uart-poll` | `uart-ring` | `uart-ring` |
| `blk`, FS leaves | - | - | - | `blk-simple`, FS server [no row] | - |

Reading the matrix: **(xP)** means one instance **per partition** (rule 13),
each with its own owner and its own configuration pin; **(kernel)** means one
instance owned by the kernel, taking the partition tag as an input (rule 13);
**(deg.)** is a degenerate implementation with a weaker claim set (decision
1.2, section 7.2); **[no row]** means no track C row schedules that
implementation yet (section 2.1). No `prot-none` appears: it is barred from
every image profile.

The flight column is the strictest: every state machine that a partition can
influence (`pool`, `timer`, `irq`, `thread`, `sched`, `budget`, `ctx`, `cspace`,
`ep`, `ntfn`) is instantiated per partition, and the only paths between
partitions are the spine's checked channels `chan` and `sample`, each guarded
by a partition-tag check and padded exits. `psched`, `prot` and `hm` are
kernel-owned because the major frame, the memory plan and the health table are
by nature global; their operations take the partition tag, and I9 and I10 are
closed over them. In the V column a guest-to-guest `ntfn` signal is
**authority-checked by the spine through a sealed `cspace` lookup in the
signalling guest's domain** before it reaches the target's `ntfn`; without that
slot a guest could signal any other.

Three things the matrix is for:

* **Swap, do not fork.** The soft-RT column differs from the hard-RTOS column
  in several cells (`sched`, `edf`, `budget`, `ep`, `irq`, `uart`), and the
  cell that matters for the replaceability claim (VOS00 section 8) is one:
  filling the scheduler slot with `rr` instead of `fp-bitmap`. Every system theorem
  evidenced over `-any` is unchanged; only the per-implementation evidence is
  re-run.
* **See what is shared.** `thread`, `ctx`, `ntfn`, `trapdec` and `pool` appear
  in all or nearly all columns, so their evidence is the best investment in the
  program. That is also why they come first in section 6.
* **See what is single-use.** Rows with one or two non-dash cells (`edf`,
  `sample`, `emul`, `mutex`) are the rule-12 clause 2 cases: each has one profile consumer and one rung,
  which satisfies the clause (at least one profile) and no more, and the table
  is where a reviewer challenges that.

The hypervisor column is the clearest demonstration of rule 12 and decision
1.2: it reuses most of the G column's mechanisms in different slots (a `thread`
in a vcpu slot, `irq-hvip`, `prot-stage2-static`) and adds one component
(`emul`).

### 7.1 Instances and owners

A profile manifest lists **instances**, not interfaces. Each instance names its
interface, its implementation, its configuration pin, its slot and its owner
(a partition, or the kernel). Two uses of `pool` in G (frames and ASIDs) are
two instances with two slots and two configurations; the flight profile's
per-partition `thread-table` is one instance per partition. Nothing in this
section changes the contracts: ownership is a property of the profile and the
spine (I9), and the contract text still names no profile (rule 3).

### 7.2 What this spec requires of `check-profile`

VOS02 section 6 defines the manifest and VOS-034 builds `check-profile`. The
requirements below are written here because they follow from the pool rules,
and VOS-034's row carries them. Each has a **negative control**: a profile that
must be rejected, with the reason.

| # | Requirement | Negative control (must be rejected) |
|---|---|---|
| P1 | An instance fills exactly one slot and has exactly one owner (rule 13); no instance appears in two slots of different partitions | A flight manifest in which one `pool` instance fills a buffer slot of partition A and a buffer slot of partition B |
| P2 | Every theorem a profile states lists the claims it **needs**, and each must be in the `carries` of the implementation in the slot that supplies it (VOS02 section 6; decision 1.2) | A flight profile with `psched-single` (no `partition-time`), with `hm-halt` (no `partition-health`), with `budget-none` (no `budget`), or with `prot-none` (no `isolation`), each stating the theorem that needs that claim |
| P3 | `prot-none` is barred from every `class image` profile (H, S, F, G, V) | Any image profile with a `prot-none` slot |
| P4 | A profile of a sealed family (H, S, F, V) declares its `cspace` slot sealed, and its spine entry list contains the `seal` call before the first user entry | A hard-RTOS profile with a `cspace` slot that is not declared sealed |
| P5 | The per-instance configuration pin of each slot (VOS-D071) agrees with the lock | A profile pinning B = 8 for an instance the lock pins at B = 4 |
| P6 | The profile's evidence profile for Boot-0 to Boot-2 states **no isolation claim** and needs none | A Boot-1 manifest whose theorem list includes `isolation` |

The theorem and claim names are provisional (section 1.2); the requirement is
that the names exist and are checked.

---

## 8. Not verified: open questions to settle

These are statements this spec relies on or implies that were **not checked**.
Each says how it will be settled. A reviewer who finds one wrong should change
the row, not the rule.

1. **One contract in two slots with per-slot configuration.** The spec assumes
   the VOS02 profile grammar and lock (sections 6.2 and 6.3) can fill two slots
   from one interface with different configurations (for example `pool` as a
   frame allocator of B = 8 and an ASID allocator of B = 4). The worked example
   in VOS02 section 6.4 shows a profile-wide configuration N = 8 and a
   per-interface `cfg` line in the lock, which suggests it can, but the
   grammar's treatment of two uses of one interface was not checked. Settled by
   VOS-D071, before `pool` is used twice.
2. **Whether `warrant-ir`'s Rust front end can lower any class K helper.** Only
   its README was read. Settled by the lowering spike in VOS-040d, with a
   `lowering-blocker` test that fails loudly on the day it can.
   **Amended by VOS-040-0 (owner/coordinator decision, 2026-11-25):** the blocker test asserts the refusal's kind and never its message text,
   the helper is extracted by span from the real file, no front-end or component
   change is allowed, and a success is "lowered helper", not evidence
   ([VOS31](VOS31-host-model-sat.md) section 13).
3. **The X1 sizes are arithmetic.** They come from binder widths and were not
   run. The two 262,144-case rows (`prot`, `pt-flat`) may need trimming.
   Settled by the capacity probe (VOS-028b) and, per interface, by the PR-C
   run that derives the case-count table.
4. **That VOS-038's fixes are small.** They are in `warrant-grammar`, which
   belongs to the VER program and has its own owners. The fallback for
   VOS-D025 is the record-of-fields ghost image, which would change no
   generator. Settled when VOS-038 starts, by reading the generator before
   estimating it.
5. **That 029a's first proof needs no cut points on real compiler output.**
   The A1 and A2 results are hand-written proxies (VOS30 section 8.4). If real
   `rustc` output for `dispatch` has structural divergence from the model, the
   first certificate needs cut points, and the split of VOS-029 into a and b
   moves. Settled by item 11.
6. **Open-PR state.** The backlog was reconciled against the branch, not
   against a fetched `origin/main` and every open PR. Before each item in
   section 6 starts, the working rule of the backlog applies: reconcile first.
7. **Whether the pool's interface boundaries survive their first consumer.**
   The decisions in section 2 are made on paper. The first interface that is
   used twice (`pool`, VOS-C01 in the backlog) and the first table interface
   (`thread`, item 14) are where a boundary is most likely to move, and the
   cost of moving it is a contract change and a digest change in every
   implementation. This is accepted: it is why the order puts `sched`-shaped
   interfaces before table-shaped ones.
8. **GPOS ISA.** The matrix assumes RV32 with Sv32 as the first GPOS target
   (`pt-sv32`), which keeps one emulator core. VOS00 section 11
   says RV64 with Sv39. Logged as VOS-D073; settled before the first `pt`
   contract.
9. **The HAL's allowlist format and the claim names.** Rule 6 requires a
   static allowlist built from the board map and the owning domain, and
   decision 1.2 names provisional claims (`budget`, `partition-time`,
   `partition-health`, `isolation`). Their exact form is settled in VOS21
   (VOS-025a) and in each contract's PR-C; until then the requirements of
   section 7.2 are written against the names.

---

## 9. What this does not buy

* **It does not prove any component.** The catalogue lists what will be proven
  and by which route. Today a component has X1 evidence, property tests,
  fuzzing and sabotage controls; the SAT line certifies contract lemmas only;
  nothing is proven on a compiled image until VOS-029 lands. That row was not
  funded until the capacity probe decided; it has (VOS-028c3c: re-plan R3,
  VOS-029a and VOS-029b funded, provisional until VOS-028c3d; VOS30 section 8.6).
* **It does not make composition free.** Pairwise invariants (rule 11) are
  small, but the spine is the one place where components meet, so a bug in the
  spine's sequencing breaks a system theorem that every leaf's evidence
  supports. The spine's evidence is therefore the highest-value item after the
  leaves, and it is not replaceable.
* **It does not make a profile quickly.** "Assemble any OS from pieces"
  describes the structure. Each piece costs three PRs (contract,
  implementation, `-any`) and each profile needs a spine specification;
  section 6 lists 25 items before the first image boots.
* **It does not settle the GPOS and hypervisor rows.** Their components have
  risk R2 and R3 entries (`pt`, `region`, `cdt`) that wait on an arrays theory
  or a redesign, and the hypervisor needs the H extension, which is absent
  from every simulator in the repository (VOS00 section 11).
* **It does not claim a real machine is proven.** The ISA model is trusted
  (VOS00 section 14.4), the HAL's assembly is outside host evidence, and every
  timing claim is an instruction count.
* **It is not final.** The catalogue is an index: each row becomes true when its
  contract PR lands and states the real M, and a row whose M turns out different
  is changed here in the same PR.

---

## 10. Backlog consequences

This spec is VOS-037. Its backlog surgery, made in the same PR, is recorded in
the backlog's decision log (2026-11-16): a new track C (one row per package,
`VOS-Cnn` with suffixes `c`, `i`, `i2`, `a`, `x` for contract, implementations,
`-any` and X3); the splits of VOS-012, 022, 025, 026, 029, 006 and 021; the
retirement of VOS-024 (its leaf components are now the C rows); the re-pointing
of the rung rows 007, 015, 016, 017, 019, 023 and 027 so each rung depends on
its C row; VOS-D048 resolved by section 2.2 (sibling contracts `edf`, `budget`,
`mutex`) in this spec, with the implementation in the C rows; VOS-D049 planned
to close by VOS-039; four new rows (VOS-037 to VOS-040); and the discovered-work
rows VOS-D068 to VOS-D078 (D068 to D074 from the catalogue, D075 and D076 from
the #19529 CI fixes, D077 and D078 from its security review).

## 11. Sources

* [VOS00](VOS00-verified-os-program.md) sections 4 to 8, 11 and 14: the
  pool, the replaceability statement, the profiles.
* [VOS01](VOS01-component-contract-model.md) section 6.1: the cost of a
  two-leaf invariant; section 3: `-any`.
* [VOS02](VOS02-interface-contracts-and-profile-linking.md) sections 2, 3, 6
  and 7: documents, kinds, profile grammar, SH16.
* [VOS10](VOS10-scheduler-contract.md) sections 10, 11 and 13: the case
  counts, the next implementations, the budget and EDF argument.
* [VOS30](VOS30-proof-route-and-capacity.md) sections 3, 4, 6 and 8: the
  obligation kinds, decomposition, certificates, the capacity numbers.
* [EMU00](EMU00-emulator-framework.md) sections 8 and 12: untrusted input
  bounds and the crate plan.
* The delivery backlog, [VOS-BACKLOG](VOS-BACKLOG.md): the queue and every
  `VOS-D` row cited above.
