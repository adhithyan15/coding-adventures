# VOS00 -- The Verified OS Program

**Status:** Draft umbrella spec; nothing implemented. No crate, no `.warrant`
file and no emulator device exists for this program yet. Child specs (VOS01
and onward, EMU00 and onward, listed in section 13) are written one at a time,
each before its code. See [the delivery backlog](VOS-BACKLOG.md) for the
queue.

**Layer:** Repository-wide program. It sits beside the VER ladder, not on it,
and supplies the owned kernel that the ladder is missing (section 2.1).

**Parent:** [VER00](VER00-verified-shipping-program.md) (the verified shipping
program) and [VER27](VER27-layered-validation-roadmap.md) (the layered
validation roadmap).

**Builds on:** [VER29](VER29-layer-contract-linking.md) (layer contract
linking), [VER04m](VER04m-warrant-stateful-objects.md) (stateful objects),
[VER30](VER30-warrant-expansion-roadmap.md) (Warrant expansion roadmap),
[VER13](VER13-assumption-catalogue.md) (assumption catalogue),
[VER20](VER20-resource-timing-proofs.md) (resource and timing proofs),
[SIM00](SIM00-simulator-protocol.md) (simulator protocol) and the
[Rust CPU simulator backlog](RUST-CPU-SIMULATOR-BACKLOG.md).

**Owner direction:** 2026-11-16. Build real operating systems in Rust that boot
an unmodified image on a machine model we own; check every component
exhaustively and prove it with the Warrant/VER toolchain; keep components
loosely coupled behind proven contracts so any one can be replaced and
re-proved alone; assemble one package pool a la carte into several profiles;
build our own verified emulator framework from the CPU simulators already in
the repository; and climb the history of operating systems from the oldest
ancestors to modern ones, one concept per rung.

---

## 0. How to read this document

This is an umbrella spec. It does not specify a kernel, a scheduler or a
device. It fixes the *shape* of the program so that the child specs, which do
specify those things, can be written without re-arguing the shape each time.
It makes decisions, says why, and says what each decision costs. Where the
toolchain cannot yet do something the program needs, the text says so and
points at the row that closes the gap (section 12) instead of assuming the
gap away.

Three words recur and are used strictly:

* **Component:** a unit with a contract, an implementation and evidence. A
  scheduler, a thread table, a UART device model.
* **Contract:** the Warrant document that states what a component may do
  (section 4.4). One contract per interface.
* **Profile:** a named assembly of components into a bootable kernel image
  (section 11).

The program has three tracks that move at different speeds, and the backlog
labels every row with one of them:

| Track | Name | What it builds |
|---|---|---|
| E | Emulator | A verified machine-emulator framework and the boards on it (section 5) |
| L | Ladder | One historical rung per row: an old OS idea recreated on its period machine (section 6) |
| M | Modern | The component pool, the spine and the shipped profiles (sections 7 and 11) |

---

## 1. Policy, and what "real" means

### 1.1 The policy in one paragraph

Every operating-system artifact this program ships is a *real* OS: an image
that boots, unmodified, on a machine model whose behaviour is defined by data
and by pure step functions we wrote and checked, and that runs real user
programs under real privilege separation. It is not a teaching simulator that
walks through a kernel in a diagram. Every component of that image carries a
contract, an implementation, and evidence for the implementation against the
contract, printed with a label that says exactly what kind of evidence it is
(section 9). Anything we cannot evidence is printed as an assumption, not
hidden.

### 1.2 Why "real" needs a definition

The repository already has operating-system-shaped code (section 2.2). Most of
it is deliberately a teaching device: a boot sequence you can step through, a
scheduler written to be read. Teaching code and shippable code differ in four
ways, and "real" means the shippable side of every one:

| Question | Teaching simulator | Real (this program) |
|---|---|---|
| What does it run on? | A host object graph that mimics hardware | A machine model driven only through its bus, interrupt lines and clock |
| Is the image modified to suit the host? | Often, since the "kernel" is host code | Never. The same bytes would run on a faithful machine |
| Where does failure show up? | In an assertion in a unit test | As a trap, a fault, or a refused claim in a printed report |
| What stops a silent regression? | Review | A contract, a monitor that checks every trap, and labelled evidence |

"Unmodified image" is the load-bearing phrase. If the kernel is a host
function that the emulator calls, then a bug in the emulator can hide a bug in
the kernel and the reverse. If the kernel is a binary that only sees a bus,
the two are separated by the machine boundary, and the machine boundary is
something we can specify and test independently (section 5).

### 1.3 What "verified" does and does not mean

"Verified" in this program means the VER00 sense: unit tested, property
tested, fuzzed, and formally checked where the toolchain can reach, with every
claim labelled by the strongest evidence that actually stands behind it. It
never means "proved correct" without a qualifier. Section 9 defines the three
bars (X1, X2, X3) and section 14.4 lists what the program does not buy.

---

## 2. Relationships

### 2.1 VER00 and VER27: the missing down route

VER27 section 4.11 records, in its own words, that no VER document plans an
owned kernel, so the catalogue entry `os.kernel.trusted` has no down route. A
Rust program that passes every Warrant check still runs on a kernel somebody
else wrote, and that kernel is an assumption. VOS is the route that turns the
assumption into a component we own, specify and evidence. It does not
discharge `os.kernel.trusted` for any other program's runtime today; what it
does is create the thing that *could* discharge it, and every claim in this
program is labelled so that nobody mistakes the one for the other.

VER27 section 4.14 and VER29 supply the linking mechanism: a package publishes
proved claims by id and a layer above assumes exactly those ids. VOS is the
first large adopter of that mechanism, and section 4.7 says what it needs
from it.

### 2.2 The S-series and the existing kernel crates

[S00](S00-system-software-architecture.md) through
[S07](S07-real-os-kernel.md) are the teaching stack: ROM, bootloader,
interrupt handler, kernel, display, integration. `os-kernel` (S07) is a
71-line `lib.rs` plus a small `main.rs`, a UEFI stub. VOS leaves S07 and every
S-series crate untouched. The reasons:

* They are teaching crates and their value is that they stay small and
  readable. Growing them into a verified kernel would damage both goals.
* Several other specs and demos depend on their present behaviour.
* A verified kernel needs different internal structure (section 4), so a
  rewrite in place would be a replacement under the old name.

New crates use the prefixes `vos-*` and `emu-*`. A VOS crate may cite an
S-series spec as history; it does not import an S-series crate.

### 2.3 SIM00 and the simulator backlogs

[SIM00](SIM00-simulator-protocol.md) is the repository's uniform
step/execute/get-state/reset protocol for simulators. The Rust CPU simulator
backlog marks 26 functional and gate-level ISA pairs complete. These are the
raw material for the emulator framework (section 5). They are audited,
tested crates and the program prefers adapters around them over rewrites
(section 5.5). Their present limits matter and are stated, not assumed away:

| Simulator | Limit that matters to VOS |
|---|---|
| `riscv-simulator` | Five CSRs (`mstatus`, `mtvec`, `mscratch`, `mepc`, `mcause`); `mret` sets only `MIE`. The checked `Rv32ISimulator` is RV32I with no M extension, fetches only inside the loaded program, and reports faults as errors, not traps. No PMP, no S-mode, no Sv39, no H extension (EMU00 sections 2 and 15) |
| `pdp11-simulator` | No MMU, no devices, no traps and no interrupts (an odd word address is an error, not a vector) |
| `ibm704-simulator` | v1 defers I/O (its README, scope section) |
| `ibm-system360-simulator` / `ibm-system370-simulator` | The 360 README defers privileged control, interrupts, protection and channels |
| `pdp8-simulator` | Surfaces device IOTs as trace events and has an interrupt wire. This is the right pattern for the bus boundary (section 5.3) |
| `intel8080-simulator`, `z80-simulator` | 256 input and 256 output port *latches* each, no hook (an adapter peeks at the next instruction). The 8080 has no interrupt delivery; the Z80 has `interrupt` and `nmi` methods but no EI delay |
| `edsac-simulator`, `ge225-simulator` | Peripherals (tape, teleprinter; cards, tape, typewriter, controllers, program interrupt) live **inside** the simulator state |
| `sim-pdp8-ge225-io-bridge` | Provides teletypes |
| `system-board`, `cpu-simulator`, `device-simulator`, `device-driver-framework` | Teaching code, not framework cores |

There is no simulator in the repository for the Apollo Guidance Computer, the
Atlas, the IBM 709x line, the Electrologica X8, the RC 4000 or the GE-645.
Rungs that need those machines are late and say so (section 6).

### 2.4 VER29 and VER04m: the toolchain this program stands on

* **VER04m** (stateful objects) is spec only. Its slice S3 (the object block
  and derived inductive row) is deferred (VER04m section 7). The stateful
  work items WX-D, WX-F and WX-G are not implemented, and WX-S1 and WX-SP1
  are queued (see the [VER backlog](VER-BACKLOG.md)). So `inout` objects, the
  "Mode E" of VER04m section 3.1, are not available. What *is* available is
  passing state by value as arguments and results, which VER30 section 4.5
  item 1 names as T0 today (`sr_latch` and `d_latch` are the worked case).
  This single fact drives the most important architecture decision in the
  program (section 4.2).
* **VER29** is an unimplemented draft. Its contract ids are
  `contract.<package>.<claim>`, so an id is tied to the package that
  implements it. Its reach pass is lexical (VER29 section 15, limit 4) and
  caller-side domains are never checked (limit 9). Both limits shape the
  rules in section 4: the first is why spine call sites use one fixed call
  form, the second is why caller preconditions become explicit claims.

---

## 3. Non-goals

* **No ports of period code.** Every historical rung is an original
  recreation of an idea, written from published descriptions, on our own
  model of the machine. We do not transcribe, translate or run period source.
  Section 6.4 repeats this per rung.
* **No certification claims.** The flight-control profile is a "DAL-A-style
  evidence map", never a compliance statement. DO-178C needs MC/DC on object
  code, requirements traceability, DO-330 tool qualification and target WCET,
  none of which this program supplies (section 11).
* **No SMP, no POSIX, no networking in the first profiles.** One core, a
  small purpose-built system-call surface, and no sockets. These are
  deliberately postponed, not rejected.
* **No hardware claims.** The machine model is the trusted base. We prove
  nothing about real silicon (section 10.3).
* **No retrofit of the S-series** (section 2.2).
* **No Rust-to-WIR proof of the kernel in this program's first phase.** The
  VER25a subset has no struct, enum, match, while, trait or impl, so kernel
  Rust cannot be lowered to WIR today. The proof route (section 10) goes
  around this by working on the compiled image.

---

## 4. Architecture

### 4.1 The picture

```
          +-----------------------------------------------------+
          |                    PROFILE                          |
          |  manifest: which implementation realises which      |
          |  interface, with digests; Kernel<P: Profile>        |
          +-----------------------------------------------------+
                               |
                 one trap in   |   one (K', Resume) out
                               v
          +-----------------------------------------------------+
          |                     SPINE                           |
          |  (K, Trap) -> (K', Resume): sequences the leaves    |
          |  in a fixed order; owns the cross-component         |
          |  invariant; the ONLY place components meet          |
          +-----------------------------------------------------+
            |        |        |        |        |        |
            v        v        v        v        v        v
        +------+ +------+ +------+ +------+ +------+ +------+
        |sched | |threads| | caps | | ipc  | | pmp  | | time |   LEAVES
        | op() | | op() | | op() | | op() | | op() | | op() |   pure steps
        +------+ +------+ +------+ +------+ +------+ +------+
                               |
                               v
          +-----------------------------------------------------+
          |   HAL: trap entry/exit asm, CSR access, MMIO        |   < ~200 lines
          |   (the only unsafe and asm, inventoried)            |   of unsafe/asm
          +-----------------------------------------------------+
                               |
                               v
                  machine model (section 5), or hardware
```

Leaves never call each other and never call the spine. The spine calls each
leaf, in a fixed order, on every kernel entry. The HAL is the thinnest
possible layer between the spine and the machine.

### 4.2 Why components are pure by-value step functions

Every leaf operation has the shape

```
op(State, In) -> (Out, State)
```

total (defined for every input), with errors as values in `Out`, never as
panics or unwinding, never as hidden state. Two facts forced this.

1. **The toolchain has no `inout` yet.** Mode E objects (VER04m section 3.1)
   need WX-G and WX-S1, which are not implemented. By-value steps are
   verifiable today, and each such step is the same mathematical object as
   a VER04m step claim, so nothing is thrown away: when WX-G and WX-S1 land,
   the pure step can be wrapped at the hardware-abstraction edge (and only
   there) in an `inout` form, and the interior proofs carry over unchanged.
2. **Pure steps make the proof obligations small.** A pure step has no
   frame problem inside it: the only state it can change is the state it was
   handed and returned. The question "did this operation touch something it
   shouldn't have?" becomes a type fact, not a theorem (rule 2 below).

The cost is the usual one for functional style: states are copied or moved
through returns. In a kernel, the states are small, fixed-size and live in
statically allocated arrays, so the cost is a compile-time concern, not a
design one. The generated code is checked at the image level (section 10),
which is where an unexpected copy would show up as an instruction-count
difference.

### 4.3 The eight structural rules

The architecture is eight rules. Each says what, and each says why, because a
rule without a reason is the first one dropped under deadline.

**Rule 1. No component calls another.** Components are leaves. A single spine
crate sequences them once per kernel entry, with signature
`(K, Trap) -> (K', Resume)`.
*Why:* if the scheduler called the thread table, the scheduler's correctness
would depend on the thread table's implementation, and swapping the thread
table would force re-proving the scheduler. With no calls between leaves,
there is no such dependency: each leaf is proved against its own contract
alone. The spine is the one place where components meet, so it is the one
place that has to be proved about all of them together (rule 3).

**Rule 2. Single ownership of each field of K.** The kernel state K is a
record; every field is owned by exactly one component. No `static mut`, no
interior mutability, no globals. (Checked by the shape check of VOS01 section
1.4, rules SH3 and SH14, which does not exist yet: backlog VOS-D002. Until it
does, this rule is reviewed, not checked.)
*Why:* frame conditions ("this operation changed nothing else") are the most
expensive thing to prove about stateful code. If each field has one owner and
the type system enforces it, the frame condition is true by construction and
costs nothing to prove. VER04m has to prove frames with mandatory
`frame-leak` faults (E17) because the code it audits was not written this
way; ours is.

**Rule 3. Cross-component consistency is a spine invariant over abstract
states.** Statements such as "the running thread is in the ready set" or "a
thread has at most one capability to a given endpoint" involve more than one
leaf. They are stated once, over the abstract states obtained through each
implementation's `alpha` (section 4.4), and proved once against the
contracts, not against any implementation.
*Why:* this is what makes replacement cheap (rule 5). The invariant mentions
contract-level state only, so any implementation that refines its contract
preserves it.

**Rule 4. Every interface ships a most-general implementation
`vos-<iface>-any`.** It takes one extra oracle input and can return any
output the *safety* contract permits. System theorems that are single-trace
safety properties (isolation, integrity, confinement) are evidenced over the
`-any` components. Noninterference (a hyperproperty) and liveness do not follow
from refinement and need more: VOS01 section 3.4, backlog VOS-D007 and VOS-031.
*Why, three reasons.* First, a theorem proved over `-any` holds for every
implementation of the safety tier, including implementations nobody has
written yet, which is exactly the replaceability promise. Second, building
`-any` tests the contract itself: a contract so weak that `-any` can do
something obviously wrong is a contract that needs strengthening, and a
contract so strong that no real implementation refines it is found when the
first real one is attempted. Third, the first replacement exercise happens
early and cheaply (backlog VOS-012) rather than late and expensively.

**Rule 5. Replacing a component means proving refinement, bounds and
footprint of the new implementation only.** Section 8 states this exactly.

**Rule 6. Caller-domain obligations are explicit.** Each spine call site
carries a claim that its arguments satisfy the contract's precondition.
*Why:* VER29 never checks caller-side domains (section 15, limit 9). A proof
that "`pick` returns a ready thread" is empty if the spine can call it on an
empty set. The obligation has to be written down where the call is made, or
no tool will ask for it.

**Rule 7. Bounded by construction.** Kernel paths use loop-free code or loops
whose bound is a configuration constant; no recursion; no division; no
allocation. Prefer bitmaps to queues. (Checked by shape rules SH1 and SH5 to
SH8 of VOS01 section 1.4; until that analyzer exists, VOS-D002, reviewed only.)
*Why:* bounded code is what the enumeration budget and the future symbolic
executor can reach at all (the budget is 400,000 cases and 64 MiB per claim,
VER04m section 4.2 E7 and E10), and what lets timing compose as an
instruction-count sum (rule 8). A bitmap "highest set bit" over eight
priorities is a fixed number of operations; a linked-list scan is not.

**Rule 8. Timing composes only as instruction counts.** A kernel entry's
cost is stated as a count of instructions times a declared per-instruction
worst case, under named `uarch.*` assumptions. Partition exits are padded
to a fixed length. Cache side channels are a declared assumption, not a
claim.
*Why:* instruction counts compose by addition and can be checked on the
emulator exactly. Cycle-accurate WCET needs a hardware timing model we do not
have (section 11). Saying "instruction counts only" now keeps the report
honest; a claim of cycles would be a label we cannot back.

### 4.4 Contract anatomy

There is one Warrant document per interface, owned by a *contract package*
such as `vos-sched-contract`. A contract contains, in this order:

| Part | What it states | Why it is there |
|---|---|---|
| Abstract state M | A small mathematical model of the component's state | The thing everything else is about |
| Total operations | `op(M, In) -> (Out, M)` over M, every input defined | Totality means no panics to reason about |
| `inv(M)`, init, closure | The invariant, a state satisfying it, and a proof that every op preserves it | Without closure the invariant is a wish |
| Refinement | `alpha(Impl) -> M`: how an implementation's concrete state abstracts to M, and that each concrete op matches the abstract one | This is what "implements the contract" means |
| Safety tier | What system theorems need | Section 9; the part `-any` realises |
| Policy tier | What a profile needs beyond safety | Fixed-priority order, round-robin fairness, EDF |
| Resource bounds | Maximum instructions and bytes per op | Rule 8 |
| Timing and flow labels | Which inputs may influence which outputs and durations | Noninterference (section 11) |
| Effect footprint | An empty capability set, `no_std`, no `unsafe` | The D21 capability cage. VER08a checks only part of this; the rest needs the new shape check (VOS-D002, VOS01 section 1.4) and is reviewed until it exists |
| Caller preconditions | What the spine must establish before each call | Rule 6 |
| Configuration constants | The sizes (thread count N, priority levels P) and the values the claim was evidenced at | Section 9.3: a proof at N=8 says nothing about N=64 unless it says so |

The two-tier split is the heart of replaceability. The safety tier is what
every system theorem uses; the policy tier is what a *profile* wants. A
scheduler that satisfies the safety tier but not the fixed-priority policy is
a valid component for the soft-real-time profile and an invalid one for the
hard-real-time profile. The contract says which.

### 4.5 Rust shape

Components are Rust traits with an associated `State` type and associated
consts for configuration. The kernel is `Kernel<P: Profile>`, monomorphised
once per profile.

* No `dyn` and no function pointers. A trait object would hide which
  implementation runs, and an indirect call is something the image-level proof
  cannot follow.
* Spine call sites use UFCS through the profile's associated type
  (`<P::Sched as Sched>::pick(...)`), so a call site names the *interface* and
  one `impl Profile` block names the implementation. The spine source is then
  identical for every profile, which rule 5 needs. VER29's lexical reach pass
  cannot resolve method calls or trait paths (limit 4), so the UFCS form is a
  stopgap that keeps a grep-level audit possible until the reach pass learns
  trait paths (VER29 slice 4, section 12). Detail: VOS01 section 1.2.
* The contract package owns the contract text. Each implementation package
  carries a *generated, digest-checked* document: the contract block copied
  verbatim, its own implementation block, and the obligations blocks it carries
  (a `.warrant` stem is a package directory and a document has no imports, VER29
  sections 3.1 and 15). A profile manifest (authored `profile.vos`, generated
  `profile.lock`) records which implementation realises which interface, with
  digests of the contract and implementation. Detail: VOS02 sections 2 and 6.

### 4.6 The HAL

The hardware abstraction layer is the only place with `unsafe` and assembly:
trap entry and exit, CSR reads and writes, MMIO. Its budget is about 200
lines, inventoried, with a per-line justification. It has host-level evidence
only (it cannot run on the host), so it is the least-evidenced part of an
image and the program keeps it small on purpose (risk 6, section 14.2). Its
contract (VOS21) is shaped by the CP/M-style rung (section 6): a BIOS/BDOS
split is a HAL contract, and the same split recurs at every level of the
ladder.

### 4.7 What this needs from VER29

VER29's ids are `contract.<package>.<claim>` and are tied to the implementing
package (section 3.1 there), while VOS wants a contract owned by one package and
realised by *several* implementations. This section first said the program's
direction was for the contract package to own the ids and for implementations to
discharge them. **That direction is reversed by [VOS02](VOS02-interface-contracts-and-profile-linking.md)**
(backlog VOS-004, decision D1), for the reasons in its section 1.3: an interface
document has only lemmas and a lemma earns no strength; the realiser is chosen by
the profile, which is a sibling of the spine and not a dependency of it, so VER29's
direct-edge rule cannot reach it; refinement claims mention the implementation's
own state, so "one id, one text" cannot hold; and slice 2 of VER29 is frozen for
LV-006.

The decision, in four lines:

1. **Ids stay stock VER29**: `contract.<impl-package>.<claim>`, published by the
   implementation. No new id shape and no change to VER29 sections 3, 5.1(a)-(e),
   6 or 9.
2. **The contract package owns the text.** It owns the contract block, one
   obligations block per tier and the ceilings. Each implementation's document is
   *generated*: the contract block, then its own implementation block, then the
   obligations blocks it carries, copied verbatim between marker comments with
   digests, checked by regeneration.
3. **Conformance is digest equality**: the implementation's `claim_sha256` for each
   obligation equals the contract's, and VER29 sections 5.1(b)-(e) hold. The
   implementation block uses one fixed vocabulary (`Impl`, `build`, `alpha`, ...),
   which is what makes the equality possible.
4. **VER29 is amended only in its reach rules and its feature rule** (new slice 7,
   backlog LV-008): `<P::A as T>::f` is an interface use composed through the
   profile's `impl` block, and an evidence-only feature that gates one module is
   not `build-config-unpinned`.

The decision covers all four id kinds, functional, resource, timing and capability
(VER29 section 3.1): VOS02 section 8 states what each checks today and what the
`contract.<impl>.resource...` ids would add later. Until VER29 publishes the last
three (its slice 2) bounds and footprint are checked at the profile gate by a VOS
validator, not linked by id. A new finding there: VER29 section 7.2.1 withholds
resource, timing and capability strengths for any contract-less dependency, and
class K crates depend on `core`, so a VER13 catalogue entry for `core` is needed
(backlog AS-008).

### 4.8 A kernel entry, step by step

The best way to see rules 1 to 3 at work is to follow one timer interrupt
through the spine. The names are illustrative; VOS20 fixes the real ones.

```
  entry:   (K, Trap::TimerIrq)
  1. time:   (tick, K.time)   = time::op(K.time, Advance)          leaf A
  2. sched:  (pick, K.sched)  = sched::op(K.sched, Preempt(cur))   leaf B
  3. thread: (ctx,  K.thr)    = thread::op(K.thr, Switch(pick))    leaf C
  4. pmp:    (cfg,  K.pmp)    = pmp::op(K.pmp, Load(owner(pick)))  leaf D
  exit:    (K', Resume { ctx, cfg })
```

Four things to notice:

* Each line reads and writes *its own field* of K and nothing else (rule 2).
  Leaf B never sees `K.thr`; the spine passes it only the abstract values it
  needs, such as the current thread id.
* The order is fixed in the spine's text, and so is the set of calls: no
  data-dependent control flow chooses *which leaf* runs, only what each leaf
  returns. That is what makes the timing a sum (rule 8).
* Before line 3 the spine carries a claim that `pick` names a live thread,
  which is the precondition of `Switch` (rule 6). The claim follows from the
  scheduler contract's safety tier (the picked thread is in the ready set) and
  the spine invariant (every ready thread is live), so it is proved once, at
  the spine, from contracts alone (rule 3).
* If the scheduler is replaced, lines 1, 3 and 4 do not change, and the claim
  before line 3 is still discharged from the *contract*, not from any code.

---

## 5. The emulator framework

### 5.1 Why we need our own "QEMU"

A real OS needs a machine to boot on. We could lean on QEMU, and for RISC-V
we will keep it as an optional external differential lane (section 5.6). But
the machine we trust has to be ours, for four reasons:

* **Proof needs a specified machine.** "The image boots" is not evidence of
  anything unless the machine's behaviour is defined and checked. A QEMU
  binary is a large unspecified artifact.
* **The ladder needs period machines** (EDSAC, 704, GE-225, PDP-8, PDP-11)
  that QEMU does not provide.
* **Determinism.** Replay, snapshot and monitor checking all need a machine
  whose only inputs are its bus, interrupt lines and virtual clock.
* **The simulators already exist** and are audited. The job is to assemble
  them, not to start again (section 2.3).

### 5.2 The same rules

The emulator uses the same discipline as the kernel (section 4), scoped by
crate class (EMU00 section 3.3; VOS01 section 1.6): pure step
functions, single-owner state, a machine loop as the spine, and virtual time
as an *input*. The result is that the emulator is itself a pool of
replaceable verified components, and the kernel's refinement monitor
(section 10) runs against a machine whose every transition is explicit.

```
   +-------------------------- machine loop (spine) ---------------------------+
   |   virtual time, board definition (data), device state, CPU state           |
   |                                                                            |
   |   step(CpuState, BusReply, IrqLines) -> (CpuState, BusRequest | Retired)   |
   |            |  BusRequest                       ^  BusReply                 |
   |            v                                   |                           |
   |        +------------- bus / MMIO / port map (static) -------------+        |
   |        |  every address -> exactly one device, or a bus error     |        |
   |        +-----+-----------+-----------+-----------+----------------+        |
   |              |           |           |           |                         |
   |           console     timer       disk        irq ctl      (devices)       |
   +----------------------------------------------------------------------------+
        host adapter (thin, outside the pure core): files, tty, wall clock
```

### 5.3 Contracts

| Component | Contract in one line |
|---|---|
| CPU core | `step(CpuState, BusReply, IrqLines) -> (CpuState, BusRequest \| Retired)`. The CPU never touches a device; interrupts are taken only at instruction boundaries |
| Bus / MMIO / ports | A static map; ranges disjoint; every address maps to one device or to a bus error; an access changes only its device |
| Interrupt controller | A claim returns the highest-priority enabled pending line; no lost and no spurious interrupts |
| Timer | Monotonic; a compare fires exactly at its time; virtual time only |
| Console / UART / teleprinter | Output log is append-only; input is delivered in order |
| Disk | Read-after-write returns the written data; bounds enforced; a write changes exactly one block |
| Tape / paper tape / cards | Sequential semantics; EOF and EOT are defined |
| Board definition | Data, validated, with its digest pinned in every OS report |
| Machine spine | Deterministic replay; snapshot and restore round trip; device isolation |

Why the CPU never touches a device: the pdp8-simulator already surfaces
device IOTs as trace events, and that is the pattern to generalise. The CPU
reports "I want to read address A" or "I executed an I/O instruction", the
bus resolves it, and the reply comes back as input to the next step. It makes
the CPU pure, makes every I/O visible to the monitor, and makes devices
swappable.

### 5.4 Two kinds of I/O, and why the cheap machines go first

Memory-mapped I/O needs a *bus-generic step* for each ISA: the instruction
fetch, load and store each become a request-reply pair rather than a direct
array access. That is a real refactor of every simulator that wants to be a
CPU core. Port-mapped and instruction-mapped I/O needs only an adapter at the
I/O instruction, but the code shows the adapters differ (EMU00 sections 2.3
and 7): the PDP-8 surfaces a trace event; the 8080 and Z80 expose latch arrays
and need a peek at the next instruction; EDSAC and the GE-225 keep their
peripherals inside the simulator state and must be wrapped; the 704 and
PDP-11 have no I/O yet, so a simulator extension comes first.

This asymmetry is why the *build order* differs from the *ladder order*
(section 6.2): the cheap machines are the ones whose I/O needs only an adapter, so they
come up first and exercise the framework before the expensive RV32 privileged
work.

### 5.5 Adapters over rewrites

Audited simulators are valuable because they are audited. The preferred
integration is an adapter that presents the existing crate through the CPU
contract. Where the existing API cannot be wrapped (the bus-generic step for
RV32 is the main case), the old API is kept and a test proves the two agree
instruction for instruction. Touching an audited crate is a named risk
(risk 11).

### 5.6 Trusted base and differentials

The machine model is trusted. Two catalogue entries are added to VER13:
`mc.<isa>.model-faithful` for the ISA model of each CPU and
`emu.board.<name>.faithful` for each board definition. The down route for the
ISA entry is the gate-level simulator already present for each ISA (a
differential test between functional and gate-level), and for RISC-V
additionally an optional differential against QEMU `virt`. QEMU is never a
dependency and never part of a CI gate; it is an external cross-check run by
hand or on a schedule.

Host I/O (terminal, files, wall clock) sits in a thin adapter *outside* the
pure core. The core sees only recorded inputs, so a run can be replayed from
its recorded input stream.

---

## 6. The history ladder

### 6.1 The rungs

The repository explores history, and OS design has a history worth climbing.
Each rung recreates one ancestor's *concept* on our model of its period
machine and contributes one contract to the pool. All rungs are original
recreations (section 3).

| Rung | Ancestor | Concept kept in the pool | Target machine |
|---|---|---|---|
| L0 | EDSAC Initial Orders (Wheeler, 1949) | Boot/loader contract | `edsac-simulator` plus tape reader and teleprinter |
| L1 | GM-NAA I/O (1956, IBM 704); FMS and IBSYS | Resident monitor, job control, I/O supervisor | `ibm704-simulator` plus tape, card, printer |
| L2 | Atlas Supervisor (1962) | Extracode (system-call trap), interrupts, paging | New `atlas-simulator` (late) |
| L3 | CTSS (1961); Dartmouth time-sharing (1964) | Time-slice scheduler, terminal multiplexer, swapping | `ge225-simulator` |
| L4 | Apollo Guidance Computer Executive and Waitlist | Fixed-priority jobs, timed task list, restart and overload shedding | New `agc-simulator` |
| L5 | THE multiprogramming system (Dijkstra, 1968) | Semaphores, strict layering | `pdp8` stand-in (labelled as a concept recreation) |
| L6 | RC 4000 nucleus (Brinch Hansen, 1969) | Message passing, policy outside the nucleus | `pdp11` stand-in |
| L7 | CP/M (Kildall, 1974 onward) | BIOS/BDOS split as HAL contract; flat file system | `intel8080` then `z80` |
| L8 | Unix Version 6 (1975) | Processes, privilege modes, file descriptors, hierarchical file system, pipes | `pdp11` (with MMU and devices: large) |
| L9 | CP-67 / VM/370 | Trap-and-emulate VMM | `ibm-system370` (very large; late) |
| L10 | MINIX / Mach | Microkernel with user-space servers | `intel8086` |
| L11 | L4 (Liedtke, 1993) | Minimal synchronous IPC | RV32 board |
| L12 | seL4 (2009) and modern VOS profiles | Capabilities, proofs, partitioning, virtualisation | RV32, then RV64 |

Multics is a cited influence and not a rung. It ran on the GE-645, and there
is no GE-645 simulator in the repository.

### 6.2 Ladder order and build order differ, and the document says so

The ladder is ordered by *chronology of the concept*: it is how a reader
should learn the ideas. The build order is ordered by *cost*: it is how the
work should proceed. They differ for concrete reasons:

* L7 (CP/M) is the cheapest machine with a disk and a port console, so its
  rung is built early even though its idea arrived in the 1970s. Its HAL
  contract also shapes the modern HAL (section 4.6).
* L4 (the AGC) and L2 (the Atlas) need simulators that do not exist, so they
  are late despite being old.
* L8 (Unix) needs a PDP-11 MMU and devices, which is several PRs of machine
  work, so it follows the message-passing rung that needs less.
* The modern track (M) runs in parallel and does not wait for the ladder,
  except where a rung feeds a component (section 7).

Reading the ladder in order and building it in order are different activities;
every child spec that cites a rung names both numbers.

### 6.3 Stand-in machines are labelled

Some period machines have no Rust simulator. Where a rung's concept can be
shown on a machine we do have, the rung uses a *stand-in*, and the stand-in is
labelled in the rung's spec, its README and its evidence report. L5 (THE ran
on the Electrologica X8) uses the PDP-8; L6 (RC 4000) uses the PDP-11. The
label means: "this recreates the concept, not the machine's behaviour." No
rung may claim fidelity to a machine it does not model.

### 6.4 Evidence per rung

Period machines have no Rust target, so early rungs are written in the period
assembly, through the repository's encoders and assemblers, with a Warrant
reference model plus the refinement monitor (section 10). Their evidence is
*model plus monitor*, never source proofs, and each rung's spec states this.
A rung is not "verified" in the L12 sense; it is "specified, modelled and
monitored", and its report says so.

### 6.5 The sourcing rule

Historical dates, attributions and descriptions in this program must be
sourced. Rule: verify each with a source, prefer primary or academic ones
(the papers, the designers' accounts, museum and archive pages), cite it in
the document's Sources section, and mark anything unverifiable `[unverified]`.
The facts in the rung table were checked on 2026-11-16 and the checking is
summarised here, with sources in section 15:

| Rung | What was checked | Result |
|---|---|---|
| L0 | Initial Orders by Wheeler, 1949; the first version occupied 31 locations and loaded programs from tape | Supported. The date of the revised Initial Orders 2 differs across sources, so no date is claimed for it [unverified] |
| L1 | GM-NAA I/O, 1956, General Motors Research (Robert L. Patrick) and North American Aviation (Owen Mock), batch monitor for the 704 | Supported by Patrick's own account. The number of installations differs across sources and is not claimed [unverified]. The dates and lineage of FMS and IBSYS were not checked [unverified] |
| L2 | Atlas inaugurated at Manchester on 7 December 1962; the Supervisor, extracodes and the one-level store (paging) were part of the design | Supported. The Supervisor paper's exact year and authors were not independently confirmed here [unverified] |
| L3 | CTSS first shown at MIT in November 1961 on a modified IBM 709, described at the 1962 Spring Joint Computer Conference; Dartmouth Time-Sharing System and BASIC unveiled 1 May 1964 on a GE-225 with a DATANET-30 | Supported by museum and university sources. Some secondary sources vary on details of the 1961 demonstration, so no detail beyond the month, machine and venue is claimed |
| L4 | The Apollo Guidance Computer software had an Executive (priority jobs) and a Waitlist (short timed tasks); restart cleared both queues and used phase tables; the Apollo 11 descent raised executive-overflow alarms | Supported by the Apollo 11 exegesis hosted by the Virtual AGC project. Which of the 1201 and 1202 codes means which shortage differs across sources, so the program claims neither. Design attribution within the MIT Instrumentation Laboratory was not checked [unverified] |
| L5 | Dijkstra, "The structure of the THE multiprogramming system", CACM 11(5), 1968, pp. 341-346; layered; semaphores; Electrologica X8 | Supported |
| L6 | Brinch Hansen, "The nucleus of a multiprogramming system", CACM 13(4), April 1970; RC 4000 system dated 1969; send/wait message and answer primitives | Supported; some citations give the pages differently |
| L7 | Kildall created CP/M in 1974 on an Intel development system; BIOS is the hardware-dependent layer under the BDOS | Supported. The date at which the BIOS/BDOS split was formalised varies (1974 to 1976) across sources [unverified] |
| L8 | The Ritchie and Thompson paper appeared in CACM in 1974; Version 6 was released in 1975 for the PDP-11; pipes arrived in the 1972 Second Edition, not in Version 6 | Supported. Note that pipes predate V6, so L8 recreates them as part of the V6 feature set, not as a V6 invention |
| L9 | CP-67 ran on the System/360 model 67; VM/370, released 1972, was a reimplementation of the CP-67 idea for System/370 | Supported. The date CP-40 and CP-67 entered production differs across sources [unverified] |
| L10 | Mach was developed at Carnegie Mellon (Rashid, Tevanian), initial release dated 1985 in one source [unverified]. MINIX was written by Tanenbaum at the Vrije Universiteit, 1987, to accompany a textbook | MINIX 1.0's kernel structure is described differently across sources; the clearly microkernel form is MINIX 3 (announced 2005). The rung therefore takes its concept from Mach and MINIX 3 together |
| L11 | Liedtke, "Improving IPC by kernel design", SOSP 1993 | Supported. That paper reports on his L3 kernel and is the basis of the L4 family; the rung is "the L4-style minimal IPC", not a recreation of L3 |
| L12 | Klein et al., "seL4: formal verification of an OS kernel", SOSP 2009 | Supported |
| (Multics) | Joint MIT, GE and Bell Labs project on the GE-645; Corbato led | Supported. Start dates vary (1964 to 1965) across sources, so the document gives none [unverified] |

---

## 7. The component pool

Each rung feeds the pool a contract; each profile (section 11) draws from the
pool. The pool is the intersection.

**The table below is the summary; [VOS04](VOS04-component-catalogue.md) is the
catalogue and governs where they differ.** VOS04 lists every interface with its
state, operations, tiers, implementations, profiles, instance size and proof
risk, states the thirteen decomposition rules that decide what a component is
(one contract per mechanism, sibling contracts for a different state, degenerate
implementations instead of optional slots, and the "basic" test), fixes the
per-component evidence template, and gives the serial order of work. Notably it
splits the thread table from the register store, treats EDF, budgets and mutexes
as sibling contracts of the scheduler (VOS-D048), and adds the interfaces this
table omits (pool, timer, interrupt routing, bounded channel, sampling port,
health monitor, syscall and trap decoders).

| Contract | Fed by rung(s) | Realised by (planned) | Used by profiles |
|---|---|---|---|
| Boot / loader | L0, L7 | Loader contract, HAL boot hook | All |
| HAL (trap, CSR, MMIO, timer, console) | L7, L11 | `vos-hal-rv32` | All |
| Scheduler, safety tier | L3, L4 | `vos-sched-any` | All |
| Scheduler, fixed-priority policy | L4 | `vos-sched-fp-bitmap` | Hard RTOS, flight |
| Scheduler, round-robin | L3 | `vos-sched-rr` (the second implementation) | Soft RT, GPOS |
| Scheduler, EDF / budget | L3 | later | Soft RT |
| Thread table | L5, L6 | leaf, one PR | All |
| IPC: notification and endpoint | L6, L10, L11 | leaf | All microkernel-style |
| Semaphore | L5 | contract | Optional |
| Static capability table | L12 | leaf | RTOS, flight, separation |
| Dynamic capabilities with revocation | L12 | later | GPOS |
| PMP plan | L12 | leaf | RTOS, flight, separation |
| Partition table and major frame | L4, L12 | leaf | Flight, separation |
| Page-table component | L2, L8 | later (Sv39) | GPOS |
| Untyped memory and retype | L12 | later | GPOS only |
| Trap-and-emulate VMM | L9 | later | Hypervisor |
| File system | L7, L8 | rung-local, then pool | GPOS |
| User-space drivers | L10 | later | GPOS |

Rung feed in one line each: L4 gives the RTOS and flight ancestry; L3 gives
soft-real-time and GPOS scheduling; L2 and L8 give the GPOS; L6, L10 and L11
give the microkernel spine; L9 gives the hypervisor; L0 and L7 give boot and
HAL.

---

## 8. The replaceability statement

This is the promise in the owner direction, stated so it can be checked.

> Someone replaces the scheduler tomorrow. They must re-prove, for the new
> implementation only, three things against the existing contract:
> **refinement** (its `alpha` maps to the contract's abstract state and every
> operation matches the abstract operation), **bounds** (its instruction and
> memory bounds stay within the contract's resource bounds), and
> **footprint** (its effect set is empty, it is `no_std`, and it has no
> `unsafe`).
> They do **not** re-prove: any other component, the spine invariant, or any
> system theorem.

The list is complete only with three further items, which VOS01 section 5.8
works through:

* **Per-image artifacts are regenerated.** The image changes whenever any
  component does, so CI redoes the monitor run, the emulator instruction counts
  and any X3 image proof. They are regenerated mechanically, not re-authored.
* **Manifest and tier eligibility.** The profile manifest row changes (the swap
  is a manifest edit), and a validator checks that the new implementation
  discharges every tier its interface needs in that profile.
* **Contract ceilings.** The contract states a ceiling per operation; the new
  implementation declares a bound at or under it. The entry-level timing bound
  (rule 8) is built from ceilings, so it stays valid across a swap inside the
  ceilings and only measured numbers move.

Why this follows from the rules:

* Rule 1 (no calls) means no other component's proof mentions the scheduler's
  code.
* Rule 2 (single ownership) means the scheduler cannot have disturbed another
  component's state, so their frame conditions are untouched.
* Rule 3 (spine invariant over abstract states) means the cross-component
  invariant mentions only the contract's state, which the new implementation
  refines.
* Rule 4 (`-any`) means system theorems that are single-trace safety
  properties were evidenced (at the bar printed for each, section 9) over every
  possible safety behaviour, so a refining implementation is already covered.
  This does not extend to noninterference or liveness (VOS-D007, VOS-031).

A concrete first exercise (backlog VOS-011 and VOS-012): the fixed-priority
bitmap scheduler `vos-sched-fp-bitmap` is evidenced exhaustively at eight
unique priorities, which is inside the enumeration budget. A round-robin
scheduler is then written against the *same* contract's safety tier. The
exercise succeeds if, and only if, the second scheduler drops in without any
edit to the spine, the other leaves or the system theorems, and if the only
new evidence is the three items above. If it does not, that is a finding about
the contract (risk 3) and it is logged, not worked around.

What the promise does *not* cover, said plainly:

| Case | What is re-proved |
|---|---|
| The new implementation satisfies only the safety tier | Valid for profiles that need only safety. Profiles that need the policy tier (fixed priority) must re-prove that tier too, or exclude the component |
| The change needs a strengthened contract | A contract change is a change to every implementation's obligation and to every profile's manifest. It is a deliberate event, not a swap |
| A different configuration (N=8 to N=64) | The claim is re-evidenced at the new constants. Labels print the constants (section 9.3) |
| The new implementation has a bigger footprint than the HAL budget allows | Not accepted; this is a bound, not a proof |
| A change to the spine | Re-prove the spine invariant against the contracts. The spine is the one component that is not replaceable under this promise |

---

## 9. The evidence bar

### 9.1 Why a bar at all

Past proofs in this repository were small. VER-BACKLOG rows VER-P024 and
VER-P025 carry the AUDIT (VER-P030) notes, and one finding was that a
hard-coded count in a report had stopped matching the manifests. Solver
capacity at kernel-entry size has not been measured. A program that claims
"verified OS" without naming the bar it reaches will inflate its labels
(risk 2). So each theorem is printed with one of three bars, derived from the
manifest, never typed by hand.

### 9.2 The three bars

| Bar | Name | Means | Reachable when |
|---|---|---|---|
| X1 | Exhaustive, reduced | `exhaustive` on a *reduced configuration*, with closure and init evidenced | Now, for small N. For by-value operations closure and init are a model lemma plus subject claims (a closure conjunct, an init `example`) until WX-S1 and VER04m S3; see VOS01 section 2.2 and VOS-D005 |
| X2 | Exhaustive plus shipped-size evidence | X1, plus property, fuzz and monitor evidence at the shipped configuration | Now, for the RTOS profiles |
| X3 | Proved on the image | `proved` on the compiled image, per instance, with a certificate | After the symbolic RV32 executor (section 10) |

Resource and timing evidence measured on the emulator is labelled as VER29
section 7.1 labels it, `example`, even for a complete sweep, until a bench entry
can declare its domain complete (VOS01 section 2.5, VOS-D004). X2 therefore
claims no better than `example` for it.

X1 exists because the enumeration budget is 400,000 cases and 64 MiB per claim
(VER04m section 4.2, E7 and E10). A scheduler over 8 unique priorities can be
enumerated; a scheduler over 256 cannot. The bar says so, rather than letting
the reduced configuration stand for the shipped one.

### 9.3 Reusing the VER vocabulary unchanged

The program does not invent labels. It reuses VER29 section 6.1 (`proved`,
`exhaustive`, `bounded-proof(k=K)`, `exhaustive-sampled`, `randomized`,
`example`) and the counting kinds of VER04m section 8.1 (`full`, `sampled`,
`bounded-set`, with only `full` shown as exhaustive). X1, X2 and X3 are a
*summary* of which of those labels a theorem reaches, not a new rung. Three
printing rules follow:

1. Every label prints the configuration constants of the instance it was
   evidenced at (N, P, priority levels). Today a pinned example checks the
   value; a manifest and report field to print it, and the binding support the
   refinement claims need (private-field state, variant-result faults), are
   VOS-D006, resolved in VOS02 section 4 (a ghost face now) and VER backlog row
   WX-SV (the generator support later).
2. Counts per kind are manifest-derived. The VER-P030 audit found hard-coded
   counts; this program never has one.
3. A claim that rests on `-any` says so, because that is the evidence that it
   covers every safety implementation.

LP-012's per-kind case counts (backlog row LP-012) are a hard prerequisite of
shipping, because without them an R3 family would count as exhaustive
(section 12).

---

## 10. Proof route and trusted base

### 10.1 Three stages

```
   NOW        host-level by-value Warrant claims on each leaf and on the spine
              (refinement, closure, bounds), evidence X1 / X2
   NOW        image-level refinement monitor: boot the image in our emulator;
              at every trap, abstract K via alpha and check the transition
              against the kernel model.                 label: randomized
   LATER      symbolic RV32 execution of each compiled kernel entry, trap
              vector to mret, cut into small obligations (per path, per
              cut point), each mitered against the bit-blasted model and
              DRAT-checked, per instance.                 label: proved (X3)
```

**X3 is compositional, never one monolithic miter.** A single miter over a
whole entry is past the solver's cliff (a dependent chain of 25 ALU steps
already needs about 240,000 conflicts; section 10.2), so the proof is a set
of obligations tied together by cut points, a composition argument and a
manifest. The route, the obligation kinds and the capacity probe are specified
in [VOS30](VOS30-proof-route-and-capacity.md) (backlog VOS-028a and VOS-028b).

**Why not lower the Rust to WIR.** VER25a's subset has no struct, enum, match,
while, trait or impl, so the kernel cannot be lowered, and a kernel written to
fit that subset would no longer be the kernel we want. So the proof goes
around the source: it works on the *compiled binary*, which is what actually
runs. This matches VER13 section 4.3, the owned-compile rung, whose deliverable
2 is exactly a symbolic RV32I executor over the `riscv-simulator` decoder; that
row is the one this program needs and is listed in section 12.

**Why the monitor comes first.** It is cheap, finds real bugs, and gives every
rung (section 6) a way to check a trap against its model before any proof
exists. It is labelled `randomized` because it samples traces; the label says
so.

### 10.2 What the symbolic executor can and cannot do today

The symbolic executor in `code/packages/rust/warrant-ir/src/sym.rs` refuses
`CallFunction` (an inliner exists, capped at 64), refuses `CallCapability`
and refuses a symbolic allocation size. It has no array theory, and the
EUF/LIA/arrays work (VER10 remainder) is not done. A kernel entry with
bounded, loop-free or constant-bounded code is within reach of inlining plus
bit-blasting, and fixed-size arrays are within reach **only for concrete or
case-split indices**: with symbolic indices the calibrated limit is about two
chained accesses into an eight-word table (VOS30 section 8.2). A general
page-table walk is not within reach until arrays land. That is why the GPOS profile starts at X1 and X2 (section 11).
Solver capacity at kernel-entry size is unmeasured: an advisor calibration
exists (VOS30 section 8.2, not reproducible from the repository) and backlog
VOS-028b measures it with a committed probe; the answer may change phasing.
The re-plan that follows VOS-028b's decision (route budget, cut density, sweeping and
memory experiments, and the decision table that funds or withholds VOS-029) is
pre-registered in VOS30 section 8.6, and its result is in: the CI recording of VOS-028c3c
(run 45486250172) gives decision R3, compiled lane, reduced. VOS-029a and VOS-029b are
funded (VOS-029a unchanged; VOS-029b with equivalence sweeping as the cut source only up to
200 instructions with points at most 2 apart, cut density K = 2), and the `thread-table`
X3 runs at its shipped T. The solver-capacity limits above stand unchanged; the funding is
provisional until the named reviewer signs (VOS-028c3d) and claims no two-safety property
(VOS30 section 8.6.4).

### 10.3 The trusted base

| Trusted item | Why it is trusted | Down route |
|---|---|---|
| The ISA model (`mc.isa.model-faithful`) | The proof is about the model | Gate-level differential; QEMU `virt` differential for RISC-V |
| The board definition (`emu.board.<name>.faithful`) | The machine's map and devices | Board validated as data; digest pinned in every report |
| The compiler | The image is compiled by `rustc` | Not discharged here. VER13 owned-compile lane is the route and is separate |
| The HAL asm and unsafe | Cannot run in host-level evidence | Keep under about 200 lines, inventoried, covered by the monitor |
| The Warrant toolchain itself (the `tcb` family) | Tools are checked by self-hosting | VER27 thread T5 |
| Real silicon | Out of scope | None. The program proves nothing about silicon |

---

## 11. The profile catalogue and its honest gaps

A profile is an assembly of components selected by a manifest. Static
configuration is used for the RTOS, flight and separation profiles; untyped
memory and retype exist for the GPOS only.

[VOS04](VOS04-component-catalogue.md) section 7 gives the same five profiles as
a matrix of pool interfaces against implementations (which implementation fills
each slot, and which slots take a degenerate one); the component lists in the
table below are the summary, and VOS04 governs where they differ.

*Why static configuration first.* A static configuration has no allocation, so
rule 7 holds, the capability table has a fixed size, and every theorem can be
stated at one instance. Dynamic capability derivation, revocation and untyped
retype need the derived inductive row (WX-S3, deferred) and an arrays theory;
doing them first would put the hardest toolchain gaps at the front of the
program.

*Build order for ISAs.* RV32 with PMP first, then RV64 with Sv39, then the H
extension. The PMP-partitioned kernel *is* the separation-kernel deliverable
until the H extension exists, which is why the hypervisor profile follows the
GPOS and does not precede it.

| Profile | Components | Theorems | Reachable bar | Gap |
|---|---|---|---|---|
| Hard RTOS (RV32 M/U with PMP, static) | HAL, PMP plan, thread table, fixed-priority bitmap scheduler, notification and endpoint IPC, timer, static capability table | Spatial isolation, kernel integrity, capability confinement, bounded kernel entries, interrupt-latency bound, fixed-priority policy | X2 near, X3 later | Cycle-level WCET needs a hardware timing model. Schedulability is an application-level question. There is no real board |
| Soft real-time | The above, with a round-robin scheduler and a budget leaf, or an EDF scheduler (a sibling interface `edf`, not a tier of `sched`: VOS04 section 2.2); the safety tier has the same shape | Isolation reused unchanged; budget enforcement | X2 | No deadline guarantee. This is the cheap replaceability demonstration: swap the scheduler, keep every system theorem |
| Flight-control (ARINC-653 style) | RTOS plus a static major-frame partition scheduler, sampling and queuing ports, health monitor, padded partition exits | Time and space partitioning; intransitive noninterference by unwinding (a 2-safety property, VER20's shape) | X2 on the model, X3 later | DO-178C needs MC/DC on object code, traceability, DO-330 tool qualification and target WCET. The program calls the result a "DAL-A-style evidence map" and never compliance |
| GPOS (RV64 S/U, Sv39) | SBI shim, page-table component, untyped and retype, dynamic capabilities with bounded revocation, user-space drivers | Address-space isolation, page-table well-formedness, derivation-tree invariants | X1 and X2 on small configurations | An arrays theory is needed at real sizes. The simulator has no S-mode or Sv39 |
| Hypervisor (H extension) | Stage-2 translation, vCPU, virtual interrupts, static partition schedule | Guest isolation, noninterference | Model at X1 | The H extension is absent everywhere in the repository |

For noninterference the plan is the standard unwinding argument: define the
observation and the flow policy, prove each step preserves a per-domain
equivalence, and the 2-safety property (two runs that agree on the observer's
inputs agree on the observer's outputs) follows. VER20 section 2 has the
self-composition machinery.

Partition exits in the flight profile are padded to a fixed length so that
the time a partition *leaves* does not leak what it did. Cache and other
shared-microarchitectural channels are declared assumptions, not claims
(rule 8).

---

## 12. Toolchain dependencies, in closing order

Each row names the VER item that closes a gap. Row 0 means nothing blocks
that work today. Rows are in the order the program needs them; the backlog
sequences work accordingly. "Measure solver capacity early" applies to every
row from 4 on.

| Order | Gap | Closed by | Needed for |
|---|---|---|---|
| 0 | None: by-value claims, property tests, fuzz and sabotage work now | n/a | First components and the monitor |
| 1 | Per-kind case counts (full, sampled, bounded-set) | LP-012 | Honest X1 and X2 labels (section 9) |
| 2 | Contract linking, slices 1 and 2, and interface reach (slice 7, the VOS02 amendment) | VER29 via LV-005 and LV-008 (section 4.7, VOS02 section 9) | Contract package plus several implementations |
| 3 | `inout` and explicit-state steps | WX-G, WX-S1 (VER04m) | An `inout` wrapper at the HAL edge only |
| 4 | Functional equivalence and bounded unrolling | WX-SP1, WX-D3 | Any `proved` label. Without WX-SP1 no `proved` is possible for a stateful step |
| 5 | Symbolic RV32 executor | VER13 section 4.3 item 2 | X3 on the image |
| 6 | Privileged emulator features | Owned by this program (E track) | PMP, trap CSRs, S-mode, Sv39 |
| 7 | Reach pass for UFCS and trait paths | VER29 slice 4 | Checking that a spine call site reaches only what it claims |
| 8 | Derived inductive row | WX-S3 (VER04m section 7) | Dynamic capabilities, revocation |
| 9 | Arrays, EUF, incremental solving, k-induction | VER10 and VER11 remainder, and the planned VER18 equivalence work (the VER18 spec itself is not yet present in the repository) | Real-size page tables and trace claims |

---

## 13. Naming and numbering

* **Spec numbers.** `VOS0x` umbrella and model (VOS04 is the component
  catalogue); `VOS1x` component contracts;
  `VOS2x` kernel and HAL; `VOS3x` proofs; `VOS4x` profiles; `VOS5x` GPOS and
  hypervisor. `EMU0x` for the emulator framework. The EMU prefix is unused
  elsewhere in `code/specs`.
* **Crate prefixes.** `vos-*` for kernel components, contract packages and
  profiles; `emu-*` for the emulator framework, devices and boards. Contract
  packages end in `-contract`; most-general implementations are
  `vos-<iface>-any`; concrete implementations are named by technique
  (`vos-sched-fp-bitmap`).
* **Contract ids.** `contract.<package>.<claim>` per VER29, settled by the
  decision in section 4.7.
* **Rung ids.** L0 to L12 as in section 6.1. Build order is the backlog
  order, not the rung number.
* **Backlog ids.** `VOS-nnn` in [VOS-BACKLOG](VOS-BACKLOG.md).

---

## 14. Phasing, risks, decisions

### 14.1 Phasing

Work proceeds in three overlapping streams, never as one big bang:

1. **Contract model and emulator first** (VOS-002 to VOS-006): the component
   model, the emulator spec and core, the first devices. Nothing else can
   start without these.
2. **Cheap rungs and the first scheduler in parallel** (VOS-007 to VOS-016):
   EDSAC loader, tape devices, a batch monitor, the scheduler contract with
   `-any` and a second implementation, an interrupt controller, a disk, a
   CP/M-style rung. These test the framework and the contract model on cheap
   machines before the expensive RV32 work.
3. **RV32 privileged slice, then the kernel** (VOS-020 to VOS-029): bus-generic
   RISC-V step, PMP, UART and timer board, the kernel model, leaf components,
   HAL, spine, first profile, the refinement monitor and the first machine
   proof.

The expensive late rows (a PDP-11 MMU, an S/370 privileged slice, an Atlas
simulator, an AGC simulator) are each a separate multi-PR effort and are
never folded into a single row (risk 12).

### 14.2 Risks

| # | Risk | Mitigation |
|---|---|---|
| 1 | Solver capacity at kernel-entry size is unmeasured | Measure early (VOS-028); keep entries small; phase around the answer |
| 2 | Label inflation | Manifest-derived labels, per-kind counts, configuration constants printed (section 9) |
| 3 | Contracts too weak or too strong | `-any` implementations and an early second implementation (VOS-012) |
| 4 | VER29 drift | Amend VER29 through LV-005; never fork it |
| 5 | The trusted ISA model | Keep the gate-level and QEMU differentials (section 5.6) |
| 6 | HAL asm and unsafe outside host evidence | Keep under about 200 lines and inventoried; cover with the monitor |
| 7 | Timing claims | Instruction counts only (rule 8) |
| 8 | Contention on shared crates (`warrant-ir`, `ca-assurance-report`) | Sequence changes through the VER backlog; keep VOS-owned extensions in VOS crates |
| 9 | Scope: three programs (emulator, ladder, modern) | Tracks, a critical path, and no row that hides large work |
| 10 | Stand-in machines mistaken for the real thing | Visible labelling (section 6.3) |
| 11 | Simulator refactors touching audited crates | Adapters first; where unavoidable keep the old API and prove agreement |
| 12 | Large machine work hidden in single rows | Late rows are marked late and split when scheduled |

### 14.3 Open decisions

| # | Decision | Where it lands |
|---|---|---|
| D1 | Contract-package-owned ids versus implementation-owned ids (section 4.7) | Decided in VOS02 (VOS-004, 2026-11-16): implementation-owned ids with contract-owned verbatim obligations, conformance by digest; the VER29 amendment is LV-008 |
| D2 | When to adopt `inout` at the HAL edge | After WX-G and WX-S1; not before |
| D3 | Whether the ladder's early rungs get a numbered Warrant reference model per rung or one shared model per machine | Decided in the first rung spec (VOS-007) |
| D4 | The exact reduced configuration for X1 per component | Decided with the measurement in VOS-028b (provisional guidance in [VOS30](VOS30-proof-route-and-capacity.md) section 8.5) |
| D5 | Whether the AGC simulator and the Atlas simulator are built in this program or contributed through the CPU simulator backlog | Decided when VOS-018 is scheduled |

### 14.4 What this program does not buy

* It does not prove a real machine. The ISA model is trusted.
* It does not verify `rustc`. The image is trusted to be what the compiler
  made of the source; the monitor and the later machine proof check the image
  against the model, which helps but is not a compiler proof.
* It does not certify anything for flight or any other regulated use.
* It does not prove schedulability. That is an application-level property.
* It does not bound cycles, only instruction counts.
* It does not cover side channels beyond those it names.
* It does not make historical rungs "verified operating systems". They are
  specified, modelled and monitored.
* It does not make a period machine's I/O real. Stand-ins are stand-ins.

---

## 15. Sources

Historical facts in section 6 were checked on 2026-11-16. Where a source was a
secondary summary, the table in section 6.5 says what could not be confirmed.

* EDSAC and the Initial Orders: University of Cambridge Computer Laboratory,
  "Computer Laboratory chronology",
  https://www.cl.cam.ac.uk/relics/chron.html ; and Mark Smotherman (Clemson),
  EDSAC page, https://people.computing.clemson.edu/~mark/edsac.html
* GM-NAA I/O: Robert L. Patrick, "Operating Systems at Conception", Computer
  History Museum Software Preservation Group,
  https://softwarepreservation.computerhistory.org/os/gm.html
* Atlas: University of Manchester, Atlas computer pages,
  https://curation.cs.manchester.ac.uk/atlas ; and "The Manchester University
  Atlas Operating System", Computing History,
  https://www.computinghistory.org.uk/det/15921/The%20Manchester%20University%20Atlas%20Operating%20System
* CTSS: IEEE Computer Society, "Fernando Jose Corbato" pioneer biography,
  https://history.computer.org/pioneers/corbato.html
* Dartmouth Time-Sharing System: Dartmouth ITS, 1960s timeline,
  https://www.dartmouth.edu/its-tools/archive/history/timeline/1960s.html ;
  Computer History Museum, "In Memoriam: Thomas E. Kurtz, 1928-2024",
  https://computerhistory.org/?p=31169
* Apollo Guidance Computer Executive, Waitlist and restart: Virtual AGC
  project, Cherry, "Apollo 11 Exegesis",
  https://ibiblio.org/apollo/Documents/CherryApollo11Exegesis.pdf
* THE: E. W. Dijkstra, "The structure of the THE multiprogramming system",
  Communications of the ACM 11(5), 1968 (EWD196),
  https://www.cs.ucf.edu/courses/cop5611/spring05/item/THE.pdf
* RC 4000: P. Brinch Hansen, "The nucleus of a multiprogramming system",
  Communications of the ACM 13(4), 1970,
  https://www.read.seas.harvard.edu/cs161/2019/pdf/hansen70nucleus.pdf
* CP/M: Computer History Museum, "Gary Kildall and the 40th Anniversary of the
  Birth of the PC Operating System",
  https://computerhistory.org/blog/gary-kildall-40th-anniversary-of-the-birth-of-the-pc-operating-system/ ;
  "A Short History of CP/M", https://landley.net/history/mirror/cpm/history.html
* Unix: D. M. Ritchie and K. Thompson, "The UNIX Time-Sharing System",
  Communications of the ACM, 1974,
  https://pdos.csail.mit.edu/6.1810/2004/readings/ritchie74unix.pdf ; and D.
  M. Ritchie, "The UNIX Time-sharing System: A Retrospective",
  https://bell-labs.com/usr/dmr/www/retro.html
* CP-67 and VM/370: R. J. Creasy, "The Origin of the VM/370 Time-Sharing
  System", IBM Journal of Research and Development, 1981 (cited from
  secondary summaries; the paper itself was not read for this check
  [unverified]); Multicians, "The IBM 360/67 and CP/CMS",
  https://multicians.org/thvv/360-67.html
* MINIX: Wikipedia, "Minix", https://en.wikipedia.org/wiki/Minix (secondary;
  the 1987 date and the textbook link are from here [unverified against a
  primary source]). Mach: no primary source retrieved [unverified].
* L4: J. Liedtke, "Improving IPC by kernel design", SOSP 1993,
  https://rcs.uwaterloo.ca/papers/l4.pdf
* seL4: G. Klein et al., "seL4: formal verification of an OS kernel", SOSP
  2009, https://trustworthy.systems/publications/papers/Klein_EHACDEEKNSTW_09.abstract
* Multics: Multicians, "Multics history", https://multicians.org/history.html
  and https://multicians.org/chrono.html

### 2026-11-16: the X3 route is compositional (VOS30)

The advisor's capacity calibration for the machine-proof stage (VOS30 section 8.2,
scratch numbers) found that instruction count is not what makes a miter hard;
structural divergence between compiled code and model is, in arithmetic and in
symbolic-offset memory. A monolithic miter over a realistic entry already sits in
the "re-plan" row (a dependent ALU chain is not green from 25 steps; a mux over an
eight-word table is not green at two chained accesses), while per-instruction cut
points took the 25-step case from 6.5 s to 31 ms. VOS00 section 10.1 is corrected
accordingly: **X3 is compositional, never one monolithic miter**, and VOS-029 is
designed with cut points, per-path splitting and concrete addresses from the start.
Section 10.2's sentence that fixed-size arrays are within reach of inlining plus
bit-blasting is corrected to "only for concrete or case-split indices". The route is
specified in [VOS30](VOS30-proof-route-and-capacity.md) (VOS-028a); the probe that
decides whether the compositional route is funded is VOS-028b; VOS-029 was not funded
until that decision. The probe has decided (VOS-028c3c: 8.3 row 1, then the re-plan
R3, compiled lane, reduced; VOS30 section 8.6): VOS-029a and VOS-029b are funded, provisional
until the named reviewer signs (VOS-028c3d).
