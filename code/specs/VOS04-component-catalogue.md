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
[VOS03](VOS03-component-shape-checker.md) (SH1-SH18),
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
commands** (a byte to write, a register to set); the protection component
returns a configuration image; the HAL executes both. Every leaf's capability
footprint therefore stays **empty**, which `ca-capability-analyzer` can check
mechanically (BUILD line 3, section 5.2), and the only code that touches
hardware is the HAL, which is the one place with `unsafe` and inline assembly
and has its own inventory evidence (VOS00 section 4.6; VOS21, VOS-025a).

*Why.* A claim that a component is pure is only as strong as the check that it
has no effects. Moving every effect into one inventoried place turns a
program-wide property into a property of one small crate.

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
| R0 | Bit-packed scalar state; width-insensitive | One obligation; green at the shipped width (VOS30 table A: N = 64) |
| R1 | One symbolic index per operation | Case-split into T obligations, T up to 8 to 16 |
| R2 | Two indices, or a bounded scan | Product split, or a cut point per iteration |
| R3 | A linked structure | **Not provable today.** Redesign to R1 or R2, or wait for arrays (VOS00 section 12 row 9) |

The **Tiers** column names the contract tiers (VOS01 section 2.4): *safety*
(what every correct implementation must do), *policy* (a deterministic choice
among the safe outcomes, named in parentheses), *canonical* (a fixed
representation, so equal abstract states have one concrete form), and *any*
(the most-general implementation, which exists for every interface).

### 2.2 Execution and time

| Interface | M; ops | Tiers | Implementations | Profiles | Rung | X1 | X3 / risk |
|---|---|---|---|---|---|---|---|
| `sched` (exists, VOS10) | ready set of N bools; `init insert remove dispatch` | safety, policy (highest), canonical, any | `fp-bitmap` (in review), `rr` (representation mode), `any`; later a two-level bitmap for N = 64 | all | L3, L4 | N = 8, 65,536 (run) | REF INV FRAME STEP; R0 (A1 and A2 green at N = 64) |
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
| `region` (variable-size) | disjoint intervals; `alloc(size,align) free(base)` | safety, policy (first-fit or buddy) | `region-buddy`, `region-firstfit`, `any` | G only | L8 | 8 granules, at most 2 live | R2 or R3; **defer**, prefer fixed pools |
| `prot` (PMP, MPU or static stage-2 plan) | static per-domain regions plus the loaded domain; `init(cfg) load(d)` returns a config image; `check(d,addr,len,access)` | safety (grants are a subset of d's regions, never the kernel or another domain), policy (exact) | `prot-pmp-napot`, `prot-pmp-tor`, later `prot-stage2-static`, `prot-none`, `any` | H S F; V via stage-2 | L12 | R = 2, 8 granules, config quantified, 262,144 (65 percent of budget) | REF over a **shared** PMP-match model, 2S; R1. The match model must be the same Warrant text the emulator's PMP claims use (VOS-021) |
| `pt` (page table) | partial map vpn -> (pfn, perms); `map unmap lookup` | safety (a map changes only that vpn; only given frames), canonical | `pt-flat` (reference), `pt-sv32`, later `pt-sv39`, `any` | G; V in a stage-2 slot | L2, L8 | flat 4 entries about 131k; 2-level 2x2 about 8k | R2. The walk is the chained-access cliff; cut between levels or split by product |
| `aspace` | A spaces (root, ASID, state); `create destroy activate` | safety | `aspace-table`, `any` | G, V | L8 | A = 2 | R1 |
| `cspace` (capability lookup) | slots (kind, object, rights); `init lookup(d,slot,need) install remove` | safety (grant only if the rights cover the need: confinement), canonical | `cspace-table`, `any` | H S F (install at boot only); G | L12 | 3 slots, 65,536 | R1 |
| `cdt` (derive and revoke) | a derivation forest; `derive delete revoke_step` (one node per call) | safety (rights fall along a derivation; no descendant remains after a revoke completes) | `cdt-array`, `any` | G only | L12 | 3 nodes, 65,536 | R3 made R2 by `revoke_step`; also waits on WX-S3 |

Why `pool` ranks as the cheapest second interface: a free set is a bit-packed
scalar (R0), so its X3 route is the same width-insensitive route as the
bitmap scheduler. It is a good first test that the component kit of section 5
is mechanical and not particular to `sched`. `region` is the cautionary row: a
variable-size allocator is a linked structure (R3), so it fails the "basic"
test of rule 12 and is deferred; fixed pools plus `pool` cover the RTOS, flight
and separation profiles with no kernel heap.

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
| `emul` (trapped-instruction decoder) | stateless | safety | `emul-rv32h` | V | L9 | reduced | R0 |
| `load` (streaming image decoder) | parser state; `step(state, byte)` -> Segment, Data, Entry, Reject or More | safety (never stores outside a declared segment; malformed input rejected; bounded steps per byte) | `load-flat`, `load-elf32`, rung-local EDSAC orders | G V; rungs L0 L1 L7 | L0 | all strings of length at most 5 over a 3-bit alphabet, about 32k | R0 or R1; **fuzz is primary** (the VOS-D001 pattern) |
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

These twelve rules decide what a component is, where its boundary falls and
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
crate that executes commands.

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
are ordinary implementations with their own evidence.

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
| I4 | the loaded protection domain equals the owner of the running thread | `prot`, `thread` |
| I5 | armed timers name live threads | `timer`, `thread` |
| I6 | capability objects are live | `cspace`, the object owner |
| I7 | the partition of the running thread equals `psched`'s current partition | `thread`, `psched` |
| I8 | mapped frames are a subset of allocated frames | `pt`, `pool` |

This is the standard set; a profile uses the rows whose components it
contains. The spine specification (VOS20, VOS-022a) proves each row closed
under each spine entry over the `-any` models, which is why `-any` exists for
every interface.

**Rule 12: the "basic" test.** A piece joins the pool only if **all** of these
hold:

1. its contract mentions no profile (rule 3);
2. it has **at least two consumers**, profiles or rungs;
3. its reduced M fits in **about 20 bits**, with every claim at most 262,144
   cases (65 percent of the 400,000-case budget);
4. its risk class is **R0 to R2** (section 2.1).

Otherwise it stays rung-local or is redesigned. *Why this rule is the one that
keeps the pool honest.* It is the only rule that can **refuse** a piece.
`region` fails clause 4 (a variable-size allocator is a linked structure) and
`cdt` passes only because `revoke_step` turns a tree walk into R2; both are in
the pool with their risk on the table. A file-system `dir` that serves only the
CP/M rung fails clause 2 and stays rung-local until a second consumer exists.

### 4.1 The rules applied once: is `pool` a component?

A worked pass through rules 1 to 12, for the interface that section 2.3 ranks
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
    frames), F (buffers), the L2 and L8 rungs; B = 8 gives 8 bits of state,
    65,536 cases; risk R0. It passes all four clauses.

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
* whole-domain **lemma evaluation with mutant controls** (today a per-contract
  class H sibling, `vos-sched-contract-check`; VOS-D049 wants it generic,
  hence VOS-039);
* the **case-count table** derived from the run (never typed in by hand);
* the interface's row in section 2 filled in with the measured numbers.

**PR-I, each implementation** (`VOS-Cnni`, `VOS-Cnni2`). BUILD runs these
lines **in this order**, cheapest first, so a failure reports the cheapest
explanation:

| # | Evidence | BUILD line | Gate |
|---|---|---|---|
| 1 | Unit | `cargo test --lib --test behaviour` with and without `ghost`, plus `--doc` | Against a plain reference model written in the test, not copied from the implementation |
| 2 | Shape | `vos-shape-check .` | SH1-SH18; `clippy::arithmetic_side_effects` denied |
| 3 | Capability | `ca-capability-analyzer --dir .` | An empty `required_capabilities.json` |
| 4 | Document | `vos-link check-doc .` | The committed `.warrant` equals the regenerated one |
| 5 | Harness drift | `gen_evidence --check <crate>` | The generated test harness equals the regenerated one |
| 6 | X1 exhaustive | the generated suite | Kind `full`; case counts read from the manifest |
| 7 | Property | 4096 random cases per claim | Catches the width the reduced X1 cannot reach |
| 8 | Fuzz | 5000 iterations; parsers add a VER06 target | Crashes and invariant violations on arbitrary bytes |
| 9 | Sabotage | `WARRANT_EVIDENCE_FAULT=...` per operation and tier, expecting **not ok** | Proves the claims can fail |
| 10 | SAT | per section 5.3 | Today: lemma certificates. After VOS-029: `vos-x3 check <crate>` |
| 11 | Selection | the build-tool row | A tool or document edit selects this crate |

Why this order: a unit failure is a bug; a shape failure is a forbidden
construct; a capability failure is an effect; a document failure is drift; and
only then do the expensive generated suites run. Sabotage runs after the
suites because it re-runs them with a deliberately broken implementation and
**expects a failure**; a claim that survives sabotage is vacuous.

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
checker (`drat-check`). Together they can certify **contract lemmas, tier
ordering and pairwise spine-invariant closure at the shipped width** (for
example N = 64), where exhaustive enumeration stops at N = 8 to 10. Two
limits are part of the statement:

* **There is no Warrant-to-bit-vector translator**, so each encoding is a hand
  transcription. The mitigation is a **parametric-in-N encoder** that is
  checked against the Warrant evaluator on the **whole X1 domain at N = 8**
  with `Aig::eval`, and then solved at the shipped N. A transcription that
  agrees with the evaluator on all 65,536 cases at N = 8 is a strong, though
  not complete, argument that it is right at N = 64 (VOS-D072).
* **It proves facts about the contract, not the implementation.** A lemma has
  no published strength (VER04m V9; VOS-D005), so it must be printed as
  "lemma, SAT-certified at N = ..." and **never counted** as evidence about
  any implementation.

**What is probably not available: SAT on Rust source.** `warrant-ir`'s Rust
front end refuses every real source tried; only `ct-compare` lowers. A
branch-free helper such as `clz8` might lower. This is a **spike with a
`lowering-blocker` test** (VOS-040), and nothing is promised on it. Its result
is not read from the README: section 8 item 2 records that only the README was
read.

**What is nothing until VOS-029: proof on the image.** After VOS-029 each
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

1. **`check_evidence.sh` is an exception list.** It tolerates exactly three
   `init` and `field-zero` controls that cannot pass because the initial state
   is already zero (VOS-D065). Every zero-initial-state component would copy
   it. Fix the generator first (VOS-038), then delete the script.
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
   drops.
5. **The lemma check is a per-contract crate** (VOS-D049): every contract
   would copy `vos-sched-contract-check`. VOS-039 makes it generic.
6. **No mechanism evidences one implementation at two configurations**, a
   reduced X1 configuration and the shipped X2 configuration, because
   `contract.vos` pins one configuration (VOS-D068).

The component kit (VOS-039) is the answer to the whole list: after three real
implementations exist, the kit extracts what they have in common into a
`vos-link check-component` that verifies the file set, the BUILD lines and the
build-tool row, so the template is **checked**, not remembered.

### 5.5 What a reviewer checks in a component PR

Until `check-component` exists, this is the list a reviewer applies by hand.
Each item has a mechanical check or an artifact to look at, so a reviewer is
reading evidence and not opinion:

| Item | Where to look | Pass condition |
|---|---|---|
| The contract names a mechanism | The spec section and `contract.part` | No profile, role or other interface named (rule 3) |
| The "basic" test is answered | The PR description, in the form of section 4.1 | All four clauses of rule 12 |
| Case counts are derived | The generated table in the contract PR | Produced by a run; every claim at most 262,144 cases |
| Controls can fail | BUILD line 9 and `negctl` entries in the manifest | Every control reports `not ok` on its sabotaged build |
| An `-any` exists | `vos-<iface>-any` and its `vos.toml` kind | `any` block, no canonical block, SH16 |
| The footprint is empty | `required_capabilities.json` | Empty, and BUILD line 3 passes |
| Reduced and real width are both reported | The evidence report | Both widths printed (rule 4); the real width by property or fuzz |
| Time-carrying contracts state an overflow rule | The contract text | Saturating or a horizon precondition (rule 7) |
| Nothing claims more than it has | The README and the report | No "proved" before VOS-029; lemmas printed as "SAT-certified at N = ..." only |
| The selection row exists | `repository_inputs.go` | A tool edit selects the crate in a dry run |

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

### 6.1 The first 25 items

| # | Item | What | Why here |
|---|---|---|---|
| 1 | VOS-037 | This spec, plus the backlog surgery | Every later row needs its interface, slot, size and risk fixed |
| 2 | VOS-012a | `vos-sched-any` | The first `-any`; D054's first run |
| 3 | VOS-012b | `vos-sched-rr` (representation mode) | The first representation-mode implementation (VOS-D061) and the first swap |
| 4 | VOS-038 | Generator debts: D065 non-inert record fault, D063 heap observer, D025 array builder | Before mass production |
| 5 | VOS-039 | Component kit: generic contract check (D049), `vos-link check-component`, coverage gate | After three real implementations exist |
| 6 | VOS-028b | Capacity probe (fixture: `vos-sched-fp-bitmap`) | Fixes the X1 sizes (VOS00 D4) and decides 029's funding |
| 7 | VOS-040 | Host model-SAT lane on the `sched` contract, plus the Rust-lowering spike | Reuses the probe harness |
| 8 | VOS-005 | `emu-core` | The emulator has been idle and everything bootable waits on it |
| 9 | VOS-036 | VOS22 hermetic build spec, including a recorded single-crate RV32 object build | 029's first proof needs a compiled RV32 object (D046) |
| 10 | VOS-029a | Symbolic executor core plus simulator differential; first X3: `fp-bitmap` dispatch, insert, remove | No cut points needed for the first certificate |
| 11 | VOS-020 | RV32 privileged slice 1 | The machine the spine boots on |
| 12 | VOS-022a | VOS20 K0 spec, static family (the K record, entries, ownership, the boot ladder) | The spine's specification |
| 13 | VOS-C02c | `thread` contract | The first table interface |
| 14 | VOS-C02i | `thread-table` | |
| 15 | VOS-C02a | `thread-any` | |
| 16 | VOS-029b | Cut points, composition checker, index case-split; X3 for `thread-table` | The first table X3 |
| 17 | VOS-006a | `emu-dev-console` (UART) plus the emu-host console | The device the first boot prints on |
| 18 | VOS-025a | VOS21 HAL contract spec (effects as data; ISA and board split) | Before the HAL and the UART driver |
| 19 | VOS-C20c | `uart` contract | |
| 20 | VOS-C20i | `vos-uart-poll` | |
| 21 | VOS-034 | `check-profile` plus static SH16 | The profile gate, before an image is built |
| 22 | VOS-035 | Recorder plus image-build gate | The gate on what was compiled |
| 23 | VOS-021a | `emu-board-rv32-min` (RAM plus UART, no PMP) | The first bootable board |
| 24 | VOS-025b | `vos-hal-rv32` | The first and only `unsafe` |
| 25 | VOS-026a | `vos-spine-static` skeleton with the reset entry; `boot-hello` boots on the emulator (Boot-0) | The first image |

### 6.2 Rationale for the first ten

* **Spec first (1).** The spec is the only item with no dependency and the
  only one that can be wrong in a way that wastes every later item.
* **`-any` next (2).** Without it there is no spine model and no `any` kind has
  ever been exercised; VOS-D054 (the generator's view of `-any` claim
  subjects) can only be settled by a real run.
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
* **Model-SAT (7).** It reuses the probe harness, and from then on every
  contract PR ships SAT certificates for its lemmas (section 5.3).
* **`emu-core` (8).** The emulator has been idle, and every bootable thing
  waits on it.
* **VOS22 (9).** 029's first proof needs a compiled RV32 object, and today
  there is no hermetic way to produce one (VOS-D046).
* **029a (10).** It is split so the **first certificate needs no cut points**: a
  real proved-on-compiled-code certificate at item 10, on the bitmap
  scheduler, where VOS30 table A says one obligation is green at N = 64.

### 6.3 The boot ladder

After item 25 the program climbs a ladder of images. Each stage is a profile
that boots, adds components, and adds one claim about the spine:

| Stage | Adds | Why this rung |
|---|---|---|
| Boot-0 (item 25) | `boot-hello`: reset entry, console | Proves the initial state satisfies the spine invariant before any user instruction |
| Boot-1 cooperative | `ctx`, `sysdec`, `trapdec` | Two tasks that yield: the first context switch |
| Boot-2 preemptive | `timer`, `irq`, VOS-013 | The first interrupt and the first preemption |
| Boot-3 `rtos-min` | `prot-pmp`, VOS-021b, `cspace`, `ep`, `ntfn` | Two user tasks under PMP, with preemption: the hard RTOS of VOS00 section 11 |
| Soft RT | `budget`, `edf` | The cheap replaceability demonstration (swap the scheduler, keep every system theorem) |
| Flight | `psched`, `chan`, `sample`, `hm`, VOS-031 | Partitioning and noninterference |
| GPOS, then V | the memory components, then the hypervisor reuses | The hardest toolchain gaps last (VOS00 section 11) |

---

## 7. Profile assembly

A profile is a manifest (VOS02 section 6) that fills slots. This matrix is the
answer to "can I assemble any OS from the pieces": each cell is the
implementation that fills the slot, `-` means the profile has no such slot, and
a name with a trailing "(deg.)" is a degenerate implementation (decision 1.2).
It is a **plan**: no cell is evidenced until its row in section 2 is done, and
the cells for G and V are the least settled.

| Interface | H hard RTOS | S soft RT | F flight and separation | G GPOS | V hypervisor |
|---|---|---|---|---|---|
| spine | `spine-static` | `spine-static` | `spine-static` | `spine-gpos` | `spine-static` |
| `sched` | `fp-bitmap` | `rr` | `fp-bitmap` | `rr`, later two-level bitmap | `rr` (vcpus) |
| `edf` | - | `edf-scan` | - | - | - |
| `budget` | `budget-none` (deg.) | `budget-sporadic` | `budget-periodic` | `budget-periodic` | `budget-none` (deg.) |
| `psched` | `psched-single` (deg.) | `psched-single` (deg.) | `psched-table` | `psched-single` (deg.) | `psched-table` |
| `thread` | `thread-table` | `thread-table` | `thread-table` | `thread-table` | `thread-table` (vcpu slot) |
| `ctx` | `ctx-rv32` | `ctx-rv32` | `ctx-rv32` | `ctx-rv32` | `ctx-rv32h` |
| `timer` | `timer-scan` | `timer-scan` | `timer-delta` | `timer-wheel` | `timer-scan` |
| `mutex` | `mutex-pcp` | - | `mutex-pcp` | - | - |
| `pool` | - (static) | - (static) | `pool-bitmap` (buffers) | `pool-bitmap` (frames, ASIDs) | `pool-bitmap` |
| `untyped`, `region`, `cdt` | - | - | - | later | - |
| `prot` | `prot-pmp-napot` | `prot-pmp-napot` | `prot-pmp-tor` | - (MMU via `pt`) | `prot-stage2-static` |
| `pt`, `aspace` | - | - | - | `pt-sv32` or `pt-sv39`, `aspace-table` | `pt` in a stage-2 slot, `aspace-table` |
| `cspace` | `cspace-table` (boot install) | `cspace-table` (boot install) | `cspace-table` (boot install) | `cspace-table` | - |
| `ep` | `ep-single` | `ep-fifo` | `ep-single` | `ep-fifo` | - |
| `ntfn` | `ntfn-word` | `ntfn-word` | `ntfn-word` | `ntfn-word` | `ntfn-word` |
| `sem` | - | optional | - | optional | - |
| `chan` | - | - | `chan-ring` (queuing ports) | `chan-ring` (pipes) | - |
| `sample` | - | - | `sample-latch` | - | - |
| `hm` | `hm-halt` (deg.) | `hm-halt` (deg.) | `hm-table` | `hm-halt` (deg.) | `hm-halt` (deg.) |
| `irq` | `irq-flat` | `irq-prio` | `irq-prio` | `irq-prio` | `irq-hvip` |
| `sysdec` | `sysdec-rtos` | `sysdec-rtos` | `sysdec-rtos` | `sysdec-sbi` | `sysdec-sbi` |
| `trapdec` | `trapdec-rv32` | `trapdec-rv32` | `trapdec-rv32` | `trapdec-rv32` | `trapdec-rv32` |
| `emul` | - | - | - | - | `emul-rv32h` |
| `load` | `load-flat` | `load-flat` | `load-flat` | `load-elf32` | `load-elf32` |
| `uart` | `uart-poll` | `uart-ring` | `uart-poll` | `uart-ring` | `uart-ring` |
| `blk`, FS leaves | - | - | - | `blk-simple`, FS server | - |

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
  `sample`, `emul`, `mutex`) are the rule-12 clause 2 cases: each stays in the
  pool only because a second consumer exists on the ladder (a rung), and the
  table is where a reviewer challenges that.

The hypervisor column is the clearest demonstration of rule 12 and decision
1.2: it reuses most of the G column's mechanisms in different slots (a `thread`
in a vcpu slot, `irq-hvip`, `prot-stage2-static`) and adds one component
(`emul`).

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
   its README was read. Settled by the lowering spike in VOS-040, with a
   `lowering-blocker` test that fails loudly on the day it can.
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
   moves. Settled by item 10.
6. **Open-PR state.** The backlog was reconciled against the branch, not
   against a fetched `origin/main` and every open PR. Before each item in
   section 6 starts, the working rule of the backlog applies: reconcile first.
7. **Whether the pool's interface boundaries survive their first consumer.**
   The decisions in section 2 are made on paper. The first interface that is
   used twice (`pool`, item C01 in the backlog) and the first table interface
   (`thread`, item 13) are where a boundary is most likely to move, and the
   cost of moving it is a contract change and a digest change in every
   implementation. This is accepted: it is why the order puts `sched`-shaped
   interfaces before table-shaped ones.
8. **GPOS ISA.** The matrix assumes RV32 with Sv32 as the first GPOS target
   (`pt-sv32`), which keeps one emulator core. VOS00 section 11
   says RV64 with Sv39. Logged as VOS-D073; settled before the first `pt`
   contract.

---

## 9. What this does not buy

* **It does not prove any component.** The catalogue lists what will be proven
  and by which route. Today a component has X1 evidence, property tests,
  fuzzing and sabotage controls; the SAT line certifies contract lemmas only;
  nothing is proven on a compiled image until VOS-029 lands, and that row is
  not funded until the capacity probe decides.
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
`mutex`); VOS-D049 closed by VOS-039; four new rows (VOS-037 to VOS-040); and
the discovered-work rows VOS-D068 to VOS-D074 plus two coordinator items.

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
