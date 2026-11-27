# EMU00 -- The Emulator Framework

**Status:** Draft; nothing implemented. No `emu-*` crate, board file, image
format or fuzz target exists yet. The listings below are design sketches in
Rust-like notation; they are not type-checked and no evidence has been
generated. This spec is delivery item VOS-003. It resolves
[VOS-D001](VOS-BACKLOG.md) in specification (section 8); the implementation of
that resolution lands in VOS-005, VOS-006, VOS-008 and VOS-014.

**Layer:** Program, track E. EMU00 sits directly under the umbrella and beside
the component contract model: it says what the machine that every VOS image
boots on is made of, and what is evidenced about each part.

**Parent:** [VOS00](VOS00-verified-os-program.md) (the verified OS program),
sections 5, 6, 9, 10 and 13.

**Builds on:** [VOS01](VOS01-component-contract-model.md) (component contract
model: shape rules SH1-SH16, contract anatomy, the `-any` discipline, the
conformance checklist), [SIM00](SIM00-simulator-protocol.md) (simulator
protocol), [08-cpu-simulator](08-cpu-simulator.md),
[CPU-SIMULATOR-ROADMAP](CPU-SIMULATOR-ROADMAP.md),
[RUST-CPU-SIMULATOR-BACKLOG](RUST-CPU-SIMULATOR-BACKLOG.md),
[VER13](VER13-assumption-catalogue.md) (assumption catalogue),
[VER06](VER06-coverage-guided-fuzz-engine.md) (fuzz engine) and
[VER29](VER29-layer-contract-linking.md) (contract linking; section 7.1 labels
for bench observations).

---

## 0. How to read this document

Marks are those of VOS01 section 0. **today** means it exists in the
repository at the paths cited. **NEW(row)** means it does not exist, and the
row names the backlog item that delivers it. The mark is never dropped.

Words used strictly:

* **Machine:** one running instance: a CPU state, a bus with devices, a virtual
  clock and a board digest. The unit that boots an image.
* **Board:** the *data* that says which devices a machine has, where they are
  mapped, how their interrupts are wired and which image slots it accepts
  (section 5).
* **Device:** a component with a contract that is neither the CPU nor the bus:
  console, timer, interrupt controller, disk, tape (section 4).
* **Adapter:** the thin crate that presents an existing, audited simulator
  through the CPU contract (section 7).
* **Tick:** one unit of *virtual* time (section 6.2). Never wall-clock time.
* **Image:** any untrusted byte string fed to a machine: a boot image, a tape,
  a deck, a disk, a board definition or a snapshot (section 8).

VOS01 is cited, not restated. Where this spec says "a component", it means
the thing VOS01 section 1 defines, subject to the class rules of section 3.3
(the one place EMU00 departs from a literal reading of VOS01).

---

## 1. Purpose

VOS00 section 5 decides that the machine we trust must be ours, built from
the CPU simulators the repository already has, and gives the contracts in one
line each. This document makes those lines precise enough that the first
implementation PR (VOS-005) can be written, reviewed and evidenced without
re-arguing the shape. It settles:

1. what each existing simulator really offers (section 2), because VOS00 was
   written from READMEs and the adapter design has to rest on the code;
2. the architecture and the classes of crate in it (section 3);
3. a contract for every kind of component: CPU core, bus, interrupt
   controller, timer, console, disk, tape, board definition, machine spine
   (section 4);
4. a concrete board-definition format with two worked boards (section 5);
5. the machine loop and why a run is exactly reproducible (section 6);
6. the adapter plan per ISA (section 7);
7. the bounds and parsing rules for every untrusted byte string, which is
   VOS-D001 (section 8);
8. snapshot and restore (section 9);
9. what evidence each contract can earn, and what is trusted (sections 10
   and 11);
10. the crate plan against the backlog (section 12).

It does not specify a kernel, a scheduler or any one device's register map;
those belong to the rung and profile specs. It is not a clone of QEMU: there
is no JIT, no performance target and no attempt at compatibility except the
optional RISC-V differential lane of section 11.

**Relation to SIM00.** SIM00 is a Python `Protocol` (`step`, `execute`,
`get_state`, `reset`) that lets compiler tests treat every simulator the same.
It describes one monolithic machine. EMU00 decomposes the machine: the CPU
asks, the bus answers, devices hold their own state. The two meet in one
place: a machine assembled under EMU00 can present a SIM00-shaped view
(`execute(image, max_steps) -> ExecutionResult`) so the existing end-to-end
and differential tests keep working. EMU00 neither replaces SIM00 nor
requires any simulator to implement a trait: the Rust simulators are plain
structs with inherent methods, and that is what the adapters wrap.

**Decisions fixed by the coordinator and not re-argued here:**

| # | Decision |
|---|---|
| E1 | Pure core: by-value steps, single-owner state, no component calls another; the machine loop is the spine, `machine_step(Machine, Inputs) -> Machine`. |
| E2 | Virtual time is an input. Host I/O lives in a thin adapter outside the core, so the core's capability certificate stays empty. |
| E3 | Two I/O kinds at the contract level (instruction/port-mapped, memory-mapped); the cost difference is why cheap machines are built first. |
| E4 | Adapters over rewrites of audited simulators; where a bus-generic step is unavoidable the old API stays and a differential claim ties old to new. |
| E5 | Each ISA model is trusted; VER13 gets `mc.<isa>.model-faithful` and `emu.board.<name>.faithful`; down routes are the functional-versus-gate-level differential and, for RISC-V only, an optional QEMU `virt` lane. |
| E6 | Evidence follows VOS00 section 9; emulator measurements are labelled per VER29 section 7.1. |
| E7 | Images, boards and snapshots are untrusted input and are bounded and fuzzed (VOS-D001, section 8). |

---

## 2. Ground truth: what the existing simulators offer

Read from the source on this branch, not from summaries. "Pub field" means the
struct field is `pub`, so a caller can mutate it with no check at all.

### 2.1 Memory, I/O and interrupts

| Crate | Who owns memory | How I/O happens | Interrupt injection |
|---|---|---|---|
| `edsac-simulator` | Inside `EdsacState`: `Box<[u32; 1024]>` short locations | **Devices are inside the state.** Paper tape `input: Vec<u8>` plus `input_cursor`; teleprinter `output: Vec<u8>` plus `last_output`. Orders `I`, `O`, `F` act on them. `set_input` replaces the tape and rewinds. `I` on an exhausted tape returns `InputExhausted` with the state unchanged | None (the machine has none) |
| `ibm704-simulator` | Private `Vec<u64>`, 1..=32,768 words | **None.** README scope: I/O devices, sense switches and lights, programmed interrupts are deferred to v2 | None |
| `pdp8-simulator` | Inside `Pdp8State`: `Box<[u16; 4096]>` | **Hook after retirement.** An IOT with device != 0 retires as a no-op and is surfaced as `StepTrace.iot: Option<IotEvent{device, pulses}>`. The result (new AC, skip) is applied afterwards through `set_accumulator` and `set_program_counter` | `set_interrupt_request(bool)`: an aggregate level wire. Taken at an instruction boundary when `interrupt_enable` is set and the ION delay has elapsed; the CPU clears `interrupt_pending` on entry, saves PC at 0 and jumps to 1 |
| `sim-pdp8-ge225-io-bridge` (not a CPU) | n/a | The pattern to generalise: `Pdp8TeletypeBridge::step_sim` steps the CPU, handles the IOT, writes AC and PC back, advances an ASR-33 serial clock one bit per instruction, and re-asserts the IRQ wire from the device flags | Level wire recomputed after every step |
| `ge225-simulator` | `Vec<i32>`, 4,096..=16,384 words; the X words are core locations | **Devices are inside the simulator,** with a host-event API: card reader and punch queues (depth 64), paper tape, console typewriter, eight controller plugs with a selector (`take_controller_commands`, `set_controller_ready`, `complete_controller`), a real-time clock (`advance_clock_sixths`), `advance_*` methods for reader events | **Internal.** The Automatic Program Interrupt latches controller and card ready transitions; at an instruction boundary it saves P at 0o201 and vectors to 0o204. The host drives it by calling the `set_*`/`complete_*` event methods |
| `intel8080-simulator` | `Memory`, pub field `mem`, sized by `new(memory_size)` | **Latch arrays.** `input_ports: [u8; 256]` and `output_ports: [u8; 256]` are pub fields. `IN` reads a latch and `OUT` writes one. There is no hook, so the host cannot know a port was touched except from the trace afterwards | `interrupts_enabled` flip-flop only. The field's own doc comment says it is not connected to any delivery mechanism |
| `z80-simulator` | Fixed 64 KiB, pub field `mem` | Same latch arrays as the 8080 (256 in, 256 out) | `interrupt(data) -> bool` (modes 0, 1, 2) and `nmi()` are methods the caller invokes between steps. No EI one-instruction-delay state exists |
| `pdp11-simulator` | Private `Vec<u8>`, 65,536 bytes | **None.** No I/O page, no devices, no MMU | None. An odd word address is the typed error `OddWordAddress`, not a vector-4 trap |
| `riscv-simulator`, `Rv32ISimulator` | Inside `Rv32IState`: `Vec<u8>`, 65,536 bytes **at address 0** | None. `ecall` with `mtvec == 0` halts; with `mtvec != 0` it takes an M-mode trap | None (no `mie`, `mip`, `mtime`) |
| `riscv-simulator`, `RiscVSimulator` | `Memory`, pub field `mem` | Host ecall services (write byte, read byte, exit, `f64` arithmetic) while `mtvec == 0`; events collect in `host_events` | None |
| `ibm-system370-simulator` | Private `Vec<u8>`, 1 MiB, 24-bit addresses | None. Problem state only | None |

### 2.2 Lifecycle, atomicity and what is deferred

| Crate | Snapshot and restore | Step atomicity and errors | Heavy or unbounded | Deferred (from its own docs) |
|---|---|---|---|---|
| `edsac-simulator` | `state()` clone; `restore(&state)` **validates** | `step` clones the machine and commits on success; typed `EdsacError` | Whole-machine clone per step. `output: Vec` grows without bound and can be read only by cloning the whole state | Mercury delay-line timing (gate-level partner) |
| `ibm704-simulator` | `get_state()`; **no restore** | **Not transactional:** an error sets `halted`. Typed `IBM704Error` | `reset` calls `expect` on a value valid by construction. FP path uses host `f64` (`powi`, `round_ties_even`) | I/O, sense switches, programmed interrupts, BCD and boolean ops, shifts, sign control, some FP variants |
| `pdp8-simulator` | `state()`; `restore` **validates** | Transactional (checkpoint clone). Typed `Pdp8Error` | 4K-word clone per step | KE8-E EAE: Group 3 returns `UnsupportedEae` |
| `ge225-simulator` | `get_state()`; **no restore** | Validates before it mutates. Errors are `String` | Host queues bounded by consts (card queue 64, punch 64, controller commands 64, character queue 65,536) | Cycle-accurate timing, full peripheral catalogue, DTSS |
| `intel8080-simulator` | `snapshot()`; **no restore** (fields are pub) | `step` checks addresses before it mutates. Typed `Intel8080Error` | **Every step clones the full state twice** (before and after, 64 KiB each) for the trace | External interrupt delivery |
| `z80-simulator` | `snapshot()`; `restore` | Typed `Z80Error`; undefined opcode is rejected before mutation | Same double snapshot per step | No EI delay |
| `pdp11-simulator` | `state()`; **no restore** (fields private) | Transactional (clones the whole simulator). `step` on a halted machine returns `Ok` | 64 KiB clone per step | MMU, devices, traps, interrupts, FP |
| `Rv32ISimulator` | `get_state()`; `restore` | `step_checked` clones the state three times. Faults are `Err`, never traps. An `unreachable!()` sits in the CSR arm | 64 KiB clones per step | M extension (rejected as `UnknownInstruction`), CSRs beyond five, U mode, PMP, interrupts, `mtime` |
| `ibm-system370-simulator` | `get_state()`; `restore(state)` | Every checked failure is atomic. Typed errors | n/a | Privileged state, channels and I/O, interrupts, translation |

Two more facts the adapters rest on. `Rv32ISimulator::check_fetch` accepts a
fetch only inside the bytes installed by the last load, so a program cannot
execute code that a store has written elsewhere; and `exec_mret` and the
checked simulator's `mret` set `MIE` unconditionally and model neither `MPIE`
nor `MPP`. A `-gatelevel` package exists in the tree for each of the nine
ISAs above (`edsac`, `ibm704`, `pdp8`, `ge225`, `intel8080`, `z80`, `pdp11`,
`riscv`, `ibm-system370`); whether each is complete is recorded in the Rust
CPU simulator backlogs and was not re-verified here.

### 2.3 What the ground truth forces

* **C1. Adapters come in three shapes, not two.** VOS00 section 5.4 sorts
  machines into hook-only and memory-mapped. The code shows a third, and a
  fourth that does not exist yet. At the *contract* level every I/O is a bus
  request and the two kinds of VOS00 hold. At the *adapter* level:
  **H (hooked)**: PDP-8, 8080, Z80, where the simulator exposes an event or a
  latch. **D (device inside)**: EDSAC and GE-225, where the peripherals are
  part of the simulator's state and must be wrapped, not replaced.
  **N (none)**: 704, PDP-11, S/370, where I/O does not exist and a
  spec-first simulator extension comes first. **B (bus-generic)**: RV32
  (and the PDP-11 for anything past 64 KiB of RAM), a new core.
* **C2. Memory is always inside the simulator today.** A board therefore
  has two ownership modes for RAM, `owner=cpu` and `owner=bus` (section 5),
  and DMA into `owner=cpu` memory goes through the adapter at an instruction
  boundary (section 7.1).
* **C3. Interrupt injection is uneven.** A wire for the PDP-8, two methods
  for the Z80, an internal event latch for the GE-225, and nothing for the
  rest. `IrqLines` (section 4.1) is the one contract; each adapter maps it
  onto what exists.
* **C4. Four simulators cannot restore a snapshot** (704, 8080, GE-225,
  PDP-11). Snapshot support (section 9) needs additive accessors, each with a
  differential test against the unchanged API (backlog VOS-D011).
* **C5. Per-step cloning** makes an instruction cost O(memory). Correctness
  is unaffected; throughput of a long boot is. Measured at VOS-007 before any
  optimisation (VOS-D012).
* **C6. Faults are `Err`, not traps.** A ladder rung may treat a fault as a
  machine check (the run stops, defined). A kernel on RV32 or the PDP-11 must
  see a trap, so those cores map each simulator `Err` to a trap cause
  (section 7.3).
* **C7. Host-side growth.** EDSAC `output` and the GE-225 typewriter output
  accumulate on the host without a drain accessor. The adapter drains into
  the bounded log of section 4.5 and the upstream accessors close the gap.

---

## 3. Architecture

### 3.1 The picture

```
  host adapter (emu-host): files, terminal, wall clock for pacing.
  Reads images with a size cap; stamps input events with a virtual tick;
  writes logs. The ONLY part of the machine with capabilities.
        |  HostEvent(tick, device, bytes)            ^  drained logs, trace
        v                                            |
  +================== pure core (empty capability certificate) ============+
  |  Machine { cpu, bus + devices, now: Tick, board digest, trace ring }   |
  |                                                                        |
  |  machine_step(Machine, Inputs) -> Machine        <- the spine          |
  |    1 deliver due host events       2 advance devices to `now`          |
  |    3 compute IrqLines              4 cpu_step loop until Retired       |
  |    5 now += cost                   6 append trace                      |
  |                                                                        |
  |   cpu_step(CpuState, BusReply, IrqLines) -> (CpuState, Out)            |
  |        | Out::Request(BusRequest)           ^ BusReply                 |
  |        v                                    |                          |
  |   +--------- bus: static map of disjoint ranges per space -------+     |
  |   |  every address -> exactly one device, or a bus error         |     |
  |   +----+---------+----------+-----------+------------+-----------+     |
  |        |         |          |           |            |                 |
  |     console    timer     irq ctl      disk       tape/cards            |
  |   (each: dev_step(DevState, DevIn) -> (DevState, DevOut); none calls   |
  |    another; the spine routes)                                          |
  +========================================================================+
  board definition: data (section 5), validated, digest pinned in reports
```

The CPU never touches a device. It emits "I want to read address A" or "I
executed an I/O instruction", the bus resolves it, and the reply is an input
to the next CPU step. The `pdp8-simulator` already does this for IOTs (section
2.1); EMU00 generalises it so every I/O is visible to the refinement monitor
of VOS00 section 10.1 and every device is swappable.

### 3.2 Why each piece is shaped this way

* **Pure by-value steps** because everything evidenced about the emulator
  (exhaustive claims over reduced configurations, deterministic replay,
  snapshot round trip) needs a transition function whose only inputs are its
  arguments (VOS01 section 1.1, laws 1-5).
* **The CPU as a resumable machine** (a request, a reply, the next request)
  rather than a function that reads memory directly, because for a
  memory-mapped ISA a load is an event on the bus and a monitor must see it.
* **Devices step against virtual time**, not on host threads, because time
  that can be paused, replayed and snapshotted is the only time a proof
  can mention.
* **A static board map** because "every address maps to exactly one device"
  is a property that can be validated once as data and then never checked
  dynamically.

### 3.3 Classes of crate, and a departure from VOS01 that is stated, not hidden

VOS00 section 5.2 says the emulator "uses the same discipline as the kernel".
VOS01 section 1.4 makes that concrete as the shape rules SH1-SH16: `no_std`,
no `alloc`, no `static`, no `dyn`, no recursion, one loop form, no division,
no panic path, fixed-width types, only `core` as a dependency. Those rules
exist so a kernel entry compiles to RV32 and can be proved on the image.
The emulator is not in any image; it runs the image on the host. And the
existing simulators it wraps use `Vec`, `Box`, `String` and `std::error`
throughout (section 2). A literal reading would forbid the adapters and make
the emulator rewrite every audited crate, which VOS00 section 5.5 rules out.

So the rules are scoped by class. The scoping changes where a rule applies,
never whether an evidence-bearing component is checked:

| Class | Crates | Shape rules | Why |
|---|---|---|---|
| **C** contract component | device contracts and the bus, interrupt, timer logic in `emu-core` and `emu-dev-*` | SH2-SH11 and SH13-SH15 in full. State is fixed capacity (const sizes, closed enums); SH1's "no `alloc`" holds for the state and step; the device set of a board is a closed enum, so SH4's no-`dyn` holds | Exhaustive evidence over a reduced instance needs a total, bounded, by-value step; these are the components with claims in section 4 |
| **S** spine | `machine_step`, recording bus, trace ring in `emu-core` | As class C, with the board's device enum supplied by the board crate through a trait (static dispatch) | Replay and isolation claims are about the spine |
| **M** model | `emu-cpu-*` adapters and the simulators they wrap | Exempt from SH1, SH10, SH12 (heap, `String`, `std::error`, in-repo dependencies). Must be `forbid(unsafe_code)` and must have no capability beyond memory. Never in an `image`-class profile's dependency graph (SH16 holds trivially: the emulator runs the image) | Trusted ISA models; their evidence is differential (sections 7 and 11), not shape |
| **P** parser | `emu-image`: every parser, the validator, the snapshot codec | `alloc` allowed, but total, panic-free, bounds-before-allocation (section 8). Pure: `&[u8]` in, value out | Untrusted input needs a different discipline than the contract components |
| **H** host | `emu-host` | `std` allowed. Capabilities (file read and write on named paths, stdin and stdout, wall clock for pacing) are declared in its certificate and nowhere else | The only place effects live |

The shape check (VOS-D002) does not exist yet, so the class tags are recorded
in each crate's README and reviewed until it does; the class split is logged
as backlog VOS-D010 for a VOS01 amendment. Class C and S crates carry an empty
VER23a certificate, which is E2 made checkable.

---

## 4. The contracts

Every contract below follows VOS01 section 2: an abstract state M small enough
to enumerate, total operations over it (errors are values), claims named
`<op>_safe` where a tier applies, a configuration, and a bar from section 10.
Policy-tier claims are written only where a profile needs more than safety.
Each device ships a most-general `-any` implementation only where a system
theorem quantifies over it (the interrupt controller and the timer; VOS01
section 3). Nothing here is Warrant-checked yet; VOS-005 writes the documents
as it writes the crates.

### 4.0 Shared vocabulary

```text
type Tick    = u64                   -- virtual time; the only time there is
type SpaceId = u8                    -- index into the board's `space` lines
type DevId   = u8                    -- index into the board's `device` lines
-- a width is a number of ISA units: bytes in byte-addressed spaces, words in
-- word-addressed ones (EDSAC, 704, GE-225, PDP-8)
struct Width(u8)                     -- 1, 2, 4 or 8

enum Access { Fetch, Load, Store, In, Out, Iot }
struct BusRequest { space: SpaceId, access: Access, addr: u64, width: Width, data: u64 }
enum BusFault { Unmapped, ReadOnly, Misaligned, Straddle, WidthUnsupported,
                DeviceRefused(u8) }
enum BusReply { Done { data: u64, skip: bool }, Fault(BusFault) }
                                     -- `skip` carries the PDP-8 IOT skip

struct IrqLines(u64)                 -- bit i set = line i asserted (a level)
enum  TrapCause { /* per ISA; closed */ }
enum  RetireEvent { Normal, Trap(TrapCause), Halt, WaitForInterrupt }
struct Retire { cost: u32, event: RetireEvent }   -- cost >= 1, in ticks
enum  CpuOut { Request(BusRequest), Retired(Retire) }
```

Why `skip` rides on the reply: the PDP-8 IOT results are "new AC" and "skip
the next instruction" (the bridge applies both after the step, section 2.1),
so a reply that carries only data would force every instruction-mapped ISA to
smuggle control flow through a side channel.

### 4.1 CPU core

**Abstract state.** The ISA's architectural state (registers, PC, mode, CSRs,
halted flag) plus a *phase*: `AtBoundary` or `Awaiting(BusRequest)`. For a
hooked or device-inside adapter the memory is part of this state
(`owner=cpu`, section 5.2); for a bus-generic core it is not.

**Operation.** `cpu_step(CpuState, BusReply, IrqLines) -> (CpuState, CpuOut)`.
At `AtBoundary` the reply is ignored and the IRQ lines are sampled; the core
either takes an interrupt or begins the next instruction. At `Awaiting` the
reply answers the outstanding request and IRQ lines are ignored.

| Claim | Statement |
|---|---|
| `cpu_total` | Defined for every state and reply the types admit, including a reply with no outstanding request (the answer is the value `Protocol` fault state, not a panic) |
| `cpu_boundary_irq` | An interrupt is taken only when the phase is `AtBoundary`; between two `Retired` outputs the IRQ argument has no effect on state |
| `cpu_precise` | At every `Retired` the architectural state is that of exactly the instructions retired so far: a faulting instruction leaves no partial effect (the simulators already commit or roll back atomically, section 2.2, except the 704 whose error halts the machine) |
| `cpu_no_device` | The crate depends on no device crate and calls no host function (class M: checked by the dependency graph and the capability certificate) |
| `cpu_agrees` | **Differential.** With the null bus (RAM only, no devices) the adapter retires the same sequence, with the same architectural state after each, as the unchanged simulator, over the corpus of section 10 (E4) |

*Why precise:* the refinement monitor checks a kernel trap against a model
(VOS00 section 10.1); a trap taken mid-instruction would have no model.
*Why interrupts only at boundaries:* every simulator in section 2 either does
this already (PDP-8, GE-225) or is stepped by a caller who can; it keeps the
adapter a function of the step sequence.

### 4.2 Bus, memory-mapped space and port space

**Abstract state.** `BusMap`: for each space, a sorted list of ranges
`(base, size, device, perms)`, plus the device array. A space has a kind
(`mem`, `port`, `iot`, `chan`; section 5.1), an address width in bits and a
unit width in bits.

**Operation.** `bus_access(Bus, BusRequest, Tick) -> (BusReply, Bus)`.

| Claim | Statement |
|---|---|
| `bus_disjoint` | Ranges in one space never overlap and never wrap past the end of the space (established by the board validator, section 5.3; the bus is built only from a `ValidBoard`) |
| `bus_total` | Every request gets a reply; no match is `Fault(Unmapped)`, a value |
| `bus_one_device` | A request is delivered to exactly the device whose range contains it, or to none; a request that spans two ranges is `Fault(Straddle)` and reaches neither |
| `bus_frame` | An access changes only the addressed device's state; the map and every other device are bit-identical afterwards |
| `bus_width` | A width the device did not declare is `WidthUnsupported`; an address not aligned to its width is `Misaligned` (the CPU decides whether that traps or wraps); multi-unit accesses in a byte-addressed space are little-endian composition of unit accesses |
| `bus_lookup_ref` | The binary-search lookup equals a linear scan of the same ranges for every address (the reference the tests use) |

*Why the bus does not decide traps:* what a misaligned load means is an ISA
fact (RISC-V traps, the PDP-11 raises a bus error vector, the 8080 has no
concept). The bus reports; the CPU adapter maps the fault to the ISA.

### 4.3 Interrupt controller

**Abstract state.** `Irq { pending: set<Line>, enabled: set<Line>,
priority: Line -> u8, in_service: set<Line>, mode: Line -> Level | Edge,
asserted: set<Line> }` for a small number of lines `L` (configuration).

**Operations.** `set_input(line, bool)` (a source's wire), `claim()`,
`complete(line)`, `set_enable`, `set_priority`, and the derived output
`cpu_line()`.

| Claim | Statement |
|---|---|
| `irq_claim_highest` | `claim` returns the highest-priority enabled pending line, ties to the lowest line number, or `None` |
| `irq_no_spurious` | A line is pending only if it was raised and not yet claimed (edge) or is still asserted (level); `claim` never returns a line that is not pending |
| `irq_no_lost` | An edge raised while its line is pending or in service is remembered: the pending bit is set once and two edges before a claim coalesce into one, which is specified behaviour, not loss. A level line stays pending until the source drops it |
| `irq_level_edge` | Level lines: `claim` does not clear pending, `complete` does not re-arm while the source still asserts. Edge lines: `claim` clears pending |
| `irq_out` | `cpu_line()` is true iff some enabled pending line outranks the current in-service priority |
| `irq_any` | The most-general `irq-any` (VOS01 section 3) may break ties arbitrarily among equal priorities; system theorems that use only `irq_no_spurious`, `irq_no_lost` and `irq_out` hold over it |

### 4.4 Timer

**Abstract state.** `Timer { now: Tick, compare: Option<u64>, latched: bool }`
per channel. `now` is not a counter the device advances; it is the value the
spine hands it (virtual time).

**Operations.** `advance(Timer, Tick) -> Timer`, `set_compare`, `read`.

| Claim | Statement |
|---|---|
| `timer_monotone` | `advance(t)` requires `t >= now` (a caller obligation, VOS01 section 2.8) and otherwise changes nothing and reports `TimeWentBack`; `read` never decreases |
| `timer_exact` | After `advance(t)`, the output line is asserted iff `compare` is set and `compare <= t`; the exact compare tick is recorded in `fired_at` and equals `compare`, never `t`, so interrupt latency is never conflated with the compare value |
| `timer_past_compare` | Writing a `compare` that is `<= now` asserts the line at once (RISC-V `mtimecmp` semantics) |
| `timer_never` | `advance(t)` also requires `t <= MAX_TICK`; a `compare` above `MAX_TICK` (for example all ones, as RISC-V software writes to mean "never") is stored as no compare, so no deadline is ever scheduled beyond `MAX_TICK` |
| `timer_virtual` | The crate has no capability and no clock; the only source of time is the `Tick` argument |

### 4.5 Console, UART and teleprinter

**Abstract state.** `Console { log: Seq<u8>, rx: Seq<u8>, flags }`. The log
is *abstractly unbounded*; an implementation has a fixed FIFO of capacity `C`
and a count of bytes already drained to the host, so `log = drained ++ fifo`.
Two shapes share the contract: `uart` (memory-mapped, status plus data
registers) and `teleprinter` (instruction-mapped, device flag plus buffer, the
ASR-33 style of the PDP-8 bridge, with serial-bit timing counted in ticks).

**Operations.** `tx(byte)`, `rx_read`, `status`, `host_push_rx(bytes, tick)`,
`host_drain_tx(n)`.

| Claim | Statement |
|---|---|
| `con_append_only` | After any operation the log equals the log before extended by zero or one byte; no byte is ever altered or removed from the abstract log |
| `con_backpressure` | If the FIFO is full, `tx` does not accept the byte and the status says so (`TXFULL` or the teleprinter's flag still clear); no byte is silently dropped |
| `con_in_order` | Input bytes are delivered to the guest in the order pushed, each exactly once |
| `con_status` | UART: `RXREADY` iff `rx` is non-empty, `TXREADY` iff the FIFO is not full: flags are functions of the queues. Teleprinter: the flag is set iff an unacknowledged completion exists and is cleared only by its acknowledging operation |
| `con_overrun` | An input push that exceeds `rx` capacity drops the newest byte and sets a sticky `OVERRUN` status; it is counted, never silent (compare the GE-225 `n_overrun`, section 2.1) |

### 4.6 Disk

**Abstract state.** `Disk { blocks: Seq<Block>, sel: u32, buf: Block, state }`
with a fixed block size and block count. v1 is programmed I/O through a
one-block buffer (the CP/M-style BIOS shape): select, command, then buffer
reads or writes. No DMA (open question Q6).

**Operations.** `select(b)`, `cmd_read`, `cmd_write`, `buf_get(i)`,
`buf_put(i, byte)`, `advance(Tick)` (a command completes after a configured
latency in ticks and then raises its interrupt line).

| Claim | Statement |
|---|---|
| `disk_raw` | After `select(b); cmd_write` completes with the buffer holding `d`, `select(b); cmd_read` yields `d` |
| `disk_bounds` | `select(b)` with `b >= count` is `OutOfRange` and changes nothing |
| `disk_one_block` | A completed write changes block `b` and no other block |
| `disk_atomic` | A write is all-or-nothing in the model; torn-write and power-loss semantics are out of scope (Q12) |
| `disk_busy` | While a command is in flight, commands and buffer access return `Busy` (a value) |

### 4.7 Tape, paper tape and cards

**Abstract state.** A *reader*: `frames: Seq<Frame>` and a cursor. A
*writer or magnetic tape*: `records: Seq<Record>` and a position. A frame is
a 5- or 8-bit row (paper tape), an 80-column card or a record (magnetic
tape), as the board's slot declares.

| Claim | Statement |
|---|---|
| `tape_sequential` | A read returns `frames[cursor]` and advances by one; frames come in order, each once |
| `tape_eof` | At `cursor == len` a read returns `EndOfData` (paper tape, cards) or `EndOfTape` (magnetic tape) and does not advance; no read ever succeeds after it |
| `tape_write_truncates` | Writing at position `p` yields `records[..p] ++ [r]` and position `p+1`: records beyond the write are discarded, as on real tape |
| `tape_mark` | A tape mark is distinct from every record and from end of tape |
| `tape_full` | A write past the configured capacity returns `Full` (a value) and changes nothing |

### 4.8 Board definition

Data, validated, with its digest pinned in every OS report. Section 5 gives
the format. The contract is a type: `ValidBoard` can be built only by
`validate(&[u8]) -> Result<ValidBoard, BoardError>`, and every constructor of
a `Bus` or a `Machine` takes a `ValidBoard`. Claims `board_canonical`,
`board_disjoint`, `board_total_ids`, `board_digest` are listed in section 5.3.

### 4.9 Machine spine

The spine claims are `replay_deterministic`, `snapshot_roundtrip`,
`device_isolation`, `time_monotone`, `trace_complete` and `input_recorded`;
section 6 states and justifies each.

---

## 5. Board definitions

### 5.1 The format

A board is a text file, a *canonical* line-oriented format: exactly one
spelling per board, so the file's SHA-256 is the board's identity and no two
different byte strings describe the same machine.

**Why not JSON.** The repository has zero-dependency JSON packages in
`code/packages/rust` (`coding_adventures_bounded_json`, `json-parser`,
`json-lexer`, `json-value`, `json-serializer`). `bounded_json` is the right
one for hostile input: no dependencies, a depth bound, closed error classes
with byte offsets. It still keeps duplicate members, accepts several
spellings of one number (`1`, `1.0`, `1e0`) and decodes escapes, so a
canonicalizer would have to be written and fuzzed on top. A canonical text
format makes "parse, re-emit, compare" the whole canonicalization and gives a
smaller attack surface. JSON remains the format of reports (outside EMU00).

**Grammar.** One record per line, `LF` endings, ASCII `0x20`-`0x7e` only, one
space between fields, no comments, no blank lines, no trailing space.

```text
board    = "emu-board 1" LF
           "name " IDENT LF
           "isa " IDENT LF
           "tick-ns " DEC LF                      ; 0 means unspecified
           "cpu " PARAMS LF                       ; e.g. reset-pc=0x80000000
           { "space " IDENT SP KIND SP DEC SP DEC LF }   ; name kind addr-bits unit-bits
           { "device " IDENT SP DEVKIND [SP PARAMS] LF } ; sorted by name
           { "map " IDENT SP HEX SP HEX SP IDENT LF }    ; space base size device
           { "irq " IDENT SP IDENT SP IDENT SP DEC SP MODE LF } ; src signal sink line mode
           { "image " IDENT SP IMAGEKIND SP HEX LF }     ; slot kind max-bytes (a size: hex)
           "end" LF
KIND     = "mem" | "port" | "iot" | "chan"
MODE     = "level" | "edge"
IDENT    = [a-z] [a-z0-9-]{0,31}
HEX      = "0x" [0-9a-f]+       ; no leading zeros except "0x0"
DEC      = "0" | [1-9][0-9]*
PARAMS   = key "=" value { SP key "=" value }   ; keys sorted; value HEX, DEC or IDENT
```

Each device kind and each parameter key has a declared value spelling in a
closed registry in `emu-image` (hex for addresses, sizes and byte capacities
such as `log-cap` and the `image` maximum; decimal for counts, bit widths and
line numbers, such as `frame-bits`, `rx-fifo`, `sources`), so `0x10` and `16`
cannot both be valid for `size`. Each key also has a ceiling in the same
registry (section 8.2), checked by V9.
Ordering is fixed: header, `cpu`, spaces by name, devices by name, maps by
(space, base), irqs by (source, signal), images by slot, `end`. The parser
accepts only this form: after parsing it re-emits the board and requires
byte equality, which makes `parse` an injection on accepted inputs.

A space has one of four kinds: `mem` addressed memory; `port` numbered I/O ports (8080,
Z80); `iot` device-select numbers with pulse bits (PDP-8); `chan`
ISA-defined fixed channels (EDSAC `I` and `O`). Instruction-mapped I/O is
thus an address space like any other, which is how the two kinds of VOS00
section 5.4 share one bus contract.

### 5.2 Two worked boards

**The EDSAC board** (`emu-board-edsac`, delivered by VOS-007). The store is
the simulator's own (`owner=cpu`, section 2.1 C2); tape and teleprinter are
device-inside peripherals wrapped by device adapters. The map lines for an
`owner=cpu` device are validated but never routed on: the adapter owns the
storage and DMA reaches it only through the adapter (section 7.1).

```text
emu-board 1
name edsac-functional
isa edsac
tick-ns 0
cpu reset-pc=0x0
space chan chan 1 5
space mem mem 10 17
device store ram owner=cpu size=0x400
device tape-in ptape-reader frame-bits=5 slot=boot
device teleprinter teleprinter frame-bits=5 log-cap=0x10000
map chan 0x0 0x1 tape-in
map chan 0x1 0x1 teleprinter
map mem 0x0 0x400 store
image boot ptape 0x100000
end
```

Line by line. `isa edsac` selects `emu-cpu-edsac`. `tick-ns 0`: ticks are
instruction counts; no historical instruction time is claimed. `space mem mem
10 17`: 1,024 locations of 17 bits (the simulator's `MEMORY_CELLS`). `space
chan chan 1 5`: two channels, 5-bit frames. `device store ... owner=cpu`:
storage lives in the simulator. `map chan 0x0 ... tape-in`: the `I` order
reads channel 0; `O` writes channel 1. `image boot ptape 0x100000`: one slot
named `boot` accepting a paper-tape image up to 1 MiB (section 8). There is no
`irq` line: the EDSAC has no interrupts.

**The RV32 board** (`emu-board-rv32-virt`, delivered by VOS-021). A
virt-like map: CLINT-style timer, PLIC-style interrupt controller, a UART and
RAM. The addresses below follow the layout the QEMU `virt` machine is
believed to use, written from memory and **[unverified]** against any
particular QEMU version; the differential lane of section 11 needs them
identical, so VOS-021 pins a QEMU version and checks them before relying on
them.

```text
emu-board 1
name rv32-virt-lite
isa rv32i
tick-ns 0
cpu reset-pc=0x80000000
space mem mem 32 8
device clint timer harts=1
device plic irq-ctl sources=8 targets=1
device ram0 ram owner=bus size=0x1000000 slot=kernel
device rom0 rom size=0x10000 slot=bootrom
device uart0 console log-cap=0x10000 rx-fifo=16 tx-fifo=16
map mem 0x1000 0x10000 rom0
map mem 0x2000000 0x10000 clint
map mem 0xc000000 0x600000 plic
map mem 0x10000000 0x100 uart0
map mem 0x80000000 0x1000000 ram0
irq clint msip cpu 3 level
irq clint mtip cpu 7 level
irq plic out cpu 11 level
irq uart0 rx plic 1 level
image bootrom meminit 0x10000
image kernel meminit 0x1000000
end
```

`owner=bus`: RAM is a bus device and every fetch, load and store is a
request. `irq clint mtip cpu 7 level`: the timer's compare output drives CPU
line 7 (the RISC-V machine-timer interrupt bit); `irq plic out cpu 11` drives
line 11 (machine external); `irq uart0 rx plic 1` wires the UART into the
controller. The image slots take `meminit` images, the flat segment format of
section 8, because ELF parsing stays outside the emulator (Q7).

### 5.3 Validation

`validate(bytes) -> Result<ValidBoard, BoardError>` runs two stages. Stage 1
is the parser of section 8 (size cap first, then line and token bounds,
canonical form). Stage 2 is semantic; each rule is a named check with a
negative-control vector:

| Rule | Checks |
|---|---|
| V1 | `isa` is in the closed ISA registry and `cpu` parameters are the ones that ISA declares |
| V2 | Every `space`, `device` and `image` name is unique; every referenced name exists |
| V3 | Every `map` names a device whose kind is allowed in that space's kind |
| V4 | `addr-bits` is in `1..=MAX_ADDR_BITS` and `unit-bits` in `1..=64` (section 8.2); `2^addr-bits` is computed in `u128`, never by a shift in the address type; `base + size` is computed with checked arithmetic and does not exceed it; `size > 0`; `base` and `size` are multiples of the device kind's granule |
| V5 | Within a space the ranges are pairwise disjoint (sort by base, compare neighbours) |
| V6 | Every bus-owned device is mapped at least once; an `owner=cpu` device is mapped exactly once and its size equals the size the adapter supports (for example 0x400 for the EDSAC store) |
| V7 | Every `irq` source signal is one the source's kind declares; the sink line is below the sink's line count; no two sources drive one sink line unless the sink kind declares it shared |
| V8 | Required devices for the ISA exist (the RV32 board must map the reset PC into a `ram` or `rom` range) |
| V9 | Every counted and sized parameter is at or below its per-kind ceiling in section 8.2 (`MAX_RAM_BYTES`, `MAX_FIFO`, `MAX_LOG_CAP`, disk blocks and bytes, irq sources) and the sum of every device's declared state is at most `MAX_TOTAL_STATE_BYTES`; the line and device counts are within their caps |
| V10 | An `image` slot's max-bytes is at most the kind's container cap (section 8.2) and, when the slot is bound to a sized device (`slot=` on a `ram`, `rom` or `disk`), at most that device's size. A slot bound to no device is a V2 error |

Both worked boards above were checked by hand against this grammar and these rules (canonical spelling, ordering including sorted parameter keys on every `device` and `cpu` line, ceilings, V10), and **both are test inputs**: `emu-image` has a test that runs each worked board through `validate` and requires `Ok`, and the spec text is the test fixture, so the examples cannot drift from the grammar. Claims, all over the validator and exhaustive on a reduced instance in the
sense of VOS00 section 9.2 X1: `board_canonical` (accepted implies re-emit is
identical), `board_disjoint` (accepted implies the ranges of every space are
disjoint, checked against a naive pairwise reference), `board_total_ids`
(accepted implies no dangling name), `board_digest` (the digest is SHA-256 of
the accepted bytes). The digest is pinned in every OS report (VOS00 section
10.3).

---

## 6. The machine loop and determinism

### 6.1 The spine

```text
struct RunBudget { max_ticks: Tick, max_output_bytes: u64,
                   max_trace_bytes: u64, max_input_log_bytes: u64 }  -- mandatory
struct Machine   { cpu: CpuState, bus: Bus, now: Tick, board: Digest,
                   budget: RunBudget, used: Usage, status: Status, trace: TraceRing }
struct HostEvent { at: Tick, dev: DevId, payload: [u8; <= MAX_EVENT_BYTES] }
struct Inputs    { events: [HostEvent]  }       -- tick-stamped, sorted, bounded
enum   Status    { Running, Halted, Quiescent, TraceFull,
                   BudgetExhausted(Budget),            -- which of the four ran out
                   Fault(MachineFault) }               -- RunawayInstruction, TimeOverflow,
                                                       -- ModelPanic, ProtocolViolation
const MAX_TICK: Tick = 1 << 62;                        -- headroom: ticks never reach u64::MAX

machine_step(Machine, Inputs) -> Machine        -- advances one retired instruction
```

One call advances exactly one retired instruction (Q1). The order inside a
call is fixed, because replay depends on it:

1. **Deliver** every event with `at <= now`, in `(at, sequence)` order, to its
   device.
2. **Advance** every device to `now` in device-index order (timers compare,
   disks complete commands), collecting interrupt-source changes.
3. **Wire** the interrupt lines: sources to the controller, controller and
   direct lines to `IrqLines`.
4. **Run the CPU** (`cpu_step`) until it returns `Retired`, answering each
   `Request` through `bus_access`. Each answered request is appended to the
   trace. More than `MAX_REQUESTS` (a per-ISA constant) requests without a
   retirement is `Fault(RunawayInstruction)`, a value.
5. **Advance time:** `now = now.checked_add(cost)`, with `cost >= 1`; overflow, or
   a result above `MAX_TICK`, is `Fault(TimeOverflow)`, a value. A retire with
   cost 0 is a protocol fault, because it could stall time and with it every
   timer. Before every call the spine checks the budget below.
6. **Classify:** `Halt` sets `Halted`. `WaitForInterrupt` sets the machine
   idle: the next call jumps `now` to the earliest of the next host event and
   the next device deadline (`advance_to_event`), but never past
   `budget.max_ticks`: if the target lies beyond it, `now` is set to
   `max_ticks` and the status is `BudgetExhausted(Ticks)`, not a long jump. A
   deadline above `MAX_TICK` is never scheduled (a timer `compare` above
   `MAX_TICK` means "never", and a host event `at` above `MAX_TICK` is rejected
   by the input-log parser). If there is no event and no deadline the status
   is `Quiescent`, a terminal state that says "nothing can ever wake this
   machine", which a report must not confuse with a clean halt.

**Run budget.** Every run carries a `RunBudget`: a configuration with no
default, so a host cannot start an unbounded run by omission. `max_ticks`
bounds virtual time and must be at most `MAX_TICK` (budget validation rejects a larger one); `max_output_bytes` bounds drained console and punch
output; `max_trace_bytes` the drained trace; `max_input_log_bytes` the recorded
input log (itself at most `MAX_INPUT_LOG_BYTES`). Exceeding any one ends the run
with `BudgetExhausted(<which>)`, distinct from `Halted` and `Quiescent`, and
the run report records the status, the counters and the budget, so an
exhausted run can never be read as a clean one. `emu-host`'s writers enforce
the three byte budgets as they write.

Trace and logs are bounded rings drained by the host between calls. A full
trace ring sets `TraceFull` and the machine refuses to step; it never drops a
record (the same backpressure rule as the console, section 4.5).

### 6.2 Virtual time

`Tick` is a `u64` and is **instruction count scaled by the retire cost**.
The default cost is 1, so on the functional models a tick is one retired
instruction. This matches VOS00 rule 8: timing composes only as instruction
counts. A per-ISA cost table (microseconds on a 704, cycles on a Z80) may be
added as a board parameter later without changing the contract (Q2); `tick-ns`
then documents a conversion and is never read by the core. No historical
timing is claimed anywhere in EMU00.

### 6.3 Deterministic replay, and what would break it

**Claim `replay_deterministic`:** for a given board digest, a given set of
image digests and a given sequence of tick-stamped host events, the run
produces a bit-identical trace and final state, every time, on every host
the host-float caveat below does not touch.

It holds because every ingredient is an input or a pure function of inputs:
time is a `Tick` argument; devices are stepped in index order, never from
threads; there is no randomness (a random-number device, if one is ever
added, takes its seed from the board); the core has no iteration over an
unordered container (fixed arrays and sorted ranges only, SH3 and SH14); and
host effects are outside the core. What would break it, and how each is
excluded:

| Breaker | Exclusion |
|---|---|
| Wall clock in a device | Class C and S crates have an empty capability certificate (VER23a) |
| Host threads or unordered maps | Shape rules (SH3, SH5) on classes C and S |
| A model that uses host floating point | **Caveat, not excluded today.** The 704 FP path uses `f64` with `powi` and `round_ties_even`; `RiscVSimulator`'s host ecalls use `f64`. IEEE arithmetic is deterministic, but `powi` depends on the host libm. Replay is claimed per host for those models until they are audited or made integer (VOS-D015). The GE-225 FP path is integer by its README; `Rv32ISimulator` has no FP |
| An event injected without a record | `input_recorded` below |

**Claim `input_recorded`:** every host event delivered to a device is also
appended to the input log with its tick, and `machine_step` consumes events
only from that log's replay. A replay needs the board, the images and the
log, and no host adapter at all. The log is itself untrusted input, parsed under
section 8 (`inputlog`), so a hostile log cannot do more than a hostile image.

**Claim `time_monotone`:** `now` never decreases and strictly increases on
every retired instruction.

**Claim `device_isolation`:** stepping or accessing device `i` leaves every
device `j != i` bit-identical. It follows from `bus_frame` and the device
`advance` frame claim, and it is why a console bug cannot corrupt a disk.

**Claim `trace_complete`:** every request the CPU issued and every reply it
received appears in the trace in order, so the refinement monitor
(VOS00 section 10.1) sees exactly the machine's observable behaviour. The
*recording bus* of VOS-005 is the component that appends; it wraps the bus
without changing any reply (claim `rec_transparent`: with and without the
recorder every reply and every device state are identical).

### 6.4 The host adapter

`emu-host` (class H) does what the core must not: opens image files (once,
then `fstat` on the open descriptor, which must be a regular file no larger
than the kind's cap, then a read of at most `cap + 1` bytes from that same
descriptor; symlinks are not followed, `O_NOFOLLOW` or the platform
equivalent; section 8.3 P9), polls a terminal, paces a real-time session with
the wall clock, and writes drained logs and the input log. Outputs are created
exclusively (`O_EXCL`) inside a per-run directory, an input path is never
reused as an output, and the writers stop at the run budget. The host also
wraps every `machine_step` in `catch_unwind` (section 7.5). It decides *when* an input arrives, but only by stamping the
event with a virtual tick, `at = now + 1` or later, and recording it. If the tick it would stamp is above `MAX_TICK`, the host refuses to stamp it and ends the run with `BudgetExhausted(Ticks)`, so every recorded log stays within what the input-log parser accepts and a recorded run stays replayable. After
that the wall clock has no part in the run. The adapter's paths come from the
run configuration, never from an image or a board (rule P10, section 8.3).

---

## 7. Adapters

### 7.1 The pattern

An adapter presents one existing simulator through `cpu_step`. Its job is to
turn the simulator's own mechanism (a trace event, a latch, an internal
queue) into `BusRequest`s and to turn `BusReply`s into the simulator's
inputs, at instruction boundaries only. The adapter does not edit the
simulator. Where an additive accessor is unavoidable (VOS-D011) it is added
to the simulator crate with a test that the unchanged API still behaves
identically (E4).

* **Shape H (hooked).** *PDP-8:* after `step`, if `trace.iot` is set, emit a
  request in the `iot` space with address `device << 3 | pulses` and `data =
  AC`; apply the reply with `set_accumulator`, and for a skip advance PC with
  `set_program_counter`; hold the `Retired` output until the reply has been
  applied (exactly what `Pdp8TeletypeBridge::step_sim` does); recompute the
  IRQ wire every step with `set_interrupt_request`, because the CPU clears it
  on entry. *8080 and Z80:* `IN` reads a latch, so the value must be placed
  before the step: the adapter **peeks** the byte at PC (and, on the Z80, the prefix bytes after it) using `wrapping_add` on the 16-bit address, as the simulators' own fetch does, so a PC at 0xFFFF cannot overflow, and reading only the simulator's own `mem` through its public read method, never through the bus; if the
  instruction is an input it requests the port first and writes
  `input_ports[p]`; after the step an output is read from `output_ports[p]`
  and requested. The peek table is *derived from the decoder*, not typed by
  hand, and a test classifies all 256 first bytes (and every prefix form on
  the Z80: `ED`-prefixed `IN r,(C)`, `OUT (C),r` and the block I/O family)
  against the executor.
* **Shape D (device inside).** *EDSAC:* on `step`, `InputExhausted` means
  "the reader is not ready", the step is transactional so the state is
  unchanged, and the adapter requests a frame on the tape channel, calls
  `set_input` with that one frame and retries; `O` retires with
  `trace.io_code`, which the adapter forwards as a request to the
  teleprinter channel. *GE-225:* the board lists the typewriter, tape and
  card devices as `owner=cpu`; the adapter wraps the simulator's host-event
  methods in the device contract, forwards `take_controller_commands` as bus
  requests, and calls `set_controller_ready` and `complete_controller` when
  the device contracts say a completion is due. The interrupt latch stays
  inside the simulator.
* **Shape N (none).** A spec-first extension of the simulator comes first
  (VOS-008 for the 704); until then the machine has no I/O and boots no
  image that needs any.
* **Shape B (bus-generic).** A new core (section 7.3).

Memory in shape H and D is `owner=cpu`. A DMA-style device access to it goes
through the adapter's `peek`/`poke`, performed between instructions, so the
CPU contract's boundary rule (`cpu_boundary_irq`) holds.

### 7.2 Per-ISA plan

| ISA (rung) | Shape | Interrupts | Needs upstream | Delivered by |
|---|---|---|---|---|
| EDSAC (L0) | D | none | additive `feed_input` and `take_output` (the output `Vec` is unbounded and `set_input` rewinds; VOS-D011) | VOS-007 |
| IBM 704 (L1) | N, then H | none in the simulator v1 | spec-first I/O orders, channels and tape, card and printer devices; `restore` | VOS-008 (spec first) |
| PDP-8 (L5) | H | wire, per step | none | devices VOS-013; rung VOS-017 |
| GE-225 (L3) | D | internal latch | `restore`; typed errors in place of `String` | VOS-016 |
| 8080 (L7) | H (latch, peek) | not available: no delivery mechanism, so a board has no `irq` lines and timing is by polling | `restore` (all fields are pub) | VOS-014 |
| Z80 (L7) | H (latch, peek, prefix forms) | `interrupt(data)` and `nmi()` at the boundary; the adapter implements the EI one-instruction delay the simulator lacks | none | no row yet (VOS-D017) |
| PDP-11 (L6, L8) | B | priority levels and vectors in the new core | none: the old crate stays | VOS-041 (VOS-D013); MMU in VOS-030 |
| RV32 (L11, M) | B | CLINT and PLIC lines into `mip` | none: the old crates stay | VOS-020, VOS-021 |
| S/370 (L9) | N | none | privileged slice, I/O | VOS-032 |
| AGC (L4) | new simulator | its own | the simulator itself | VOS-018 |

### 7.3 The bus-generic cores, and RV32 in detail

`emu-cpu-rv32` is a new core: the decoder is reused (`riscv_simulator::decode`
is a public module) but the execute stage is written again, because the old
executor reads and writes a `Memory` directly and cannot be made to emit
requests. That is the one place EMU00 accepts a rewrite of audited logic;
the weight is carried by the differential claim and by the QEMU lane. The
old crates and APIs are untouched (VOS00 risk 11).

The core adds what section 2 shows is missing: fetch, load and store as
bus requests (so code may run from any mapped executable range, which
removes the `FetchOutsideProgram` limit), M and U privilege modes, the trap
CSRs (`mstatus` with `MIE`, `MPIE`, `MPP`; `mtvec`, `mepc`, `mcause`,
`mtval`, `mscratch`, `mie`, `mip`), interrupts from `IrqLines` into `mip`,
and, in VOS-021, PMP. Each simulator `Err` becomes a trap:

| Old behaviour (`Rv32ISimulator`) | New core |
|---|---|
| `MisalignedFetch` | trap, cause 0 |
| `FetchOutsideProgram` | no equivalent; an unmapped or non-executable fetch is a trap, cause 1 |
| `UnknownInstruction` (including M ops on `rv32i`) | trap, cause 2 (illegal instruction) |
| `UnsupportedCsr` | trap, cause 2 |
| `MisalignedData` | trap, cause 4 (load) or 6 (store) |
| `DataOutOfRange` | trap, cause 5 (load) or 7 (store) |
| `ecall` with `mtvec == 0` halts | always a trap (cause 11 from M, 8 from U); a program ends through a board device (an "exit" register), not through `mtvec == 0` |
| `mret` sets `MIE` | restores `MIE` from `MPIE` and the mode from `MPP` |

**Claim `cpu_agrees` for RV32:** for every program that raises none of the old
errors and does not execute `mret` or take a trap, the new core retires the
same instruction sequence with the same registers, PC, CSR and memory
contents as `Rv32ISimulator`, checked after every instruction over the corpus
of section 10. Where the old model is incomplete (`mret`, trap entry) the
oracle is the RISC-V privileged architecture and the QEMU lane of section 11,
not the old simulator. The PDP-11 core (`emu-cpu-pdp11`) follows the same
rule: the old crate stays, and agreement is claimed on non-faulting programs.

### 7.4 The differential corpus and the adapter fuzz target

Each adapter is exercised by (a) the simulator's own test programs run
through both paths; (b) an exhaustive single-instruction sweep where the
decode space is small (the 256 first bytes of the 8080 and Z80, the 4,096
PDP-8 instruction words with representative state); (c) sampled operand
sweeps for wide ISAs, labelled `exhaustive-sampled` and never `exhaustive`
(VER29 section 6.1; VOS00 section 9.3); and (d) sabotage runs: an adapter with
the port peek disabled, or with a skip ignored, must fail a named test. Each adapter also has a fuzz and property target
`emu/fuzz/cpu-<isa>`: arbitrary program bytes are loaded into the board's
null-bus configuration and run for a bounded number of ticks; the properties
are that nothing panics, that every `cpu_step` issues at most `MAX_REQUESTS`
requests before retiring, and that the final status is one of the defined
`Status` values.

### 7.5 Containing model panics

Class M crates are exempt from the no-panic shape rules (section 3.3), and two
panic paths are already known: `IBM704Simulator::reset` calls `expect`, and
`Rv32ISimulator`'s CSR arm contains `unreachable!()` (VOS-D020). A guest
program must never be able to take the host down, so `emu-host` runs every
`machine_step` under `std::panic::catch_unwind` and turns a panic into
`Fault(ModelPanic)`; the machine value is then discarded (`machine_step` consumed it by value, so
its state is lost). To keep something to report, the host records a state
digest at checkpoints, every `checkpoint_steps` retired instructions (a
required `emu-host` configuration field) and at every budget check, and the
panic report gives the last checkpoint's digest and step number and the trace
so far. The panic hook must not print guest-derived data (register values,
memory, console bytes) to the host terminal; it writes the model name and the
source location only. Stack overflow and out-of-memory abort the process
instead of unwinding, so they are not contained by this section: the resource
caps (sections 8.2 and 6.1) remain the primary control and `catch_unwind` is a
backstop. This **requires `panic = "unwind"`**, the default: `emu-host` and
every binary that links a class M crate must not be built with `panic =
"abort"`, and a CI check reads the resolved profile. `catch_unwind` is
containment, not a safety proof: the crates `forbid(unsafe_code)`, so a panic
cannot corrupt memory, but it is no substitute for removing the known paths,
which the adapters do (guard or avoid the call) as part of VOS-D020.

---

## 8. Untrusted input and bounds (VOS-D001)

### 8.1 What is untrusted

VOS-D001 (found in the VOS-001 security review) asks for the following to be
bounded: tape, paper-tape, card and disk images, and board definitions. EMU00
widens the set to everything that crosses into the machine as bytes: the boot
`meminit` image, the recorded **input log** that a replay consumes, and the **snapshot** (it is restored state, but whoever
supplied the file chose every byte). Nothing in an image is ever executed by
the host, and nothing in an image can name a host path.

### 8.2 Limits

All limits are named constants in `emu-image`, each with a boundary vector
(section 8.5). A board's `image` line may lower a limit for its slot, never
raise it.

| Input | Constant | Value |
|---|---|---|
| Board definition | `MAX_BOARD_BYTES` | 16,384 |
| | `MAX_BOARD_LINES`, `MAX_LINE_BYTES` | 512, 128 |
| | `MAX_SPACES`, `MAX_DEVICES`, `MAX_MAPS` | 4, 32, 64 |
| | `MAX_IRQS`, `MAX_PARAMS_PER_LINE`, `MAX_IMAGE_SLOTS` | 64, 8, 8 |
| Paper tape | `MAX_PTAPE_FRAMES` | 1,048,576 |
| Card deck | `MAX_CARDS` (160 bytes each) | 16,384 |
| Magnetic tape | `MAX_MTAPE_RECORD_BYTES` | 65,536 |
| | `MAX_MTAPE_RECORDS` (records and tape marks) | 16,384 |
| | `MAX_MTAPE_BYTES` | 16 MiB |
| Disk | block sizes | 128, 256, 512, 1,024, 2,048, 4,096 |
| | `MAX_DISK_BLOCKS`, `MAX_DISK_BYTES` | 524,288, 64 MiB |
| Boot `meminit` | `MAX_SEGMENTS`, `MAX_MEMINIT_BYTES` | 64, 32 MiB |
| Snapshot | `MAX_SNAPSHOT_BYTES` | 257 MiB: a constant, `MAX_TOTAL_STATE_BYTES` plus 1 MiB of framing; a board's own bound may be lower |
| Input log | `MAX_INPUT_EVENTS`, `MAX_EVENT_BYTES` | 1,048,576, 4,096 |
| | `MAX_INPUT_LOG_BYTES` | 64 MiB |
| Time | `MAX_TICK` | 2^62 |
| Board address space | `MAX_ADDR_BITS`, unit bits | 1..=32 (an RV64 board needs an EMU00 amendment first), 1..=64 |
| Board device sizes | `MAX_RAM_BYTES` (any `ram` or `rom`) | 128 MiB |
| | `MAX_FIFO` (entries), `MAX_LOG_CAP` (bytes) | 4,096, 16 MiB |
| | disk device | at most `MAX_DISK_BLOCKS` blocks and `MAX_DISK_BYTES` |
| | `MAX_IRQ_SOURCES` per controller | 64 |
| | `MAX_TOTAL_STATE_BYTES` (sum over all devices) | 256 MiB |

The container caps of the binary kinds are total bytes including the 16-byte
header (and the 32-byte trailer for a snapshot).

The values are chosen to be far above any rung's need (an EDSAC tape is
kilobytes; a CP/M disk is under a megabyte) and low enough that a hostile
file cannot exhaust a host. They are configuration, and a rung may state a
lower one. The 704 and GE-225 queue depths (section 2.2) are precedent for
bounding by named constants.

### 8.3 Parsing rules

Every parser in `emu-image` obeys all of these. Each is checkable and each
has a negative-control vector (section 8.5).

| Rule | Text |
|---|---|
| P1 | **Size first.** If `bytes.len()` exceeds the kind's cap, return `TooLarge` before reading any byte of content |
| P2 | **Checked arithmetic.** Every sum, product and offset uses `checked_*` (or widens first); no `as` narrowing without a range check |
| P3 | **No allocation from an unvalidated length.** Each length or count field is compared with the kind's cap *and* with the bytes actually remaining before it is used; storage is then sized from the validated value, never from the claimed one. Parsers return borrowed views of the input where the format allows, and copy only after validation |
| P4 | **All or nothing.** The result is `Result<Image, ImageError>`; a failure leaves no partial image and no side effect |
| P5 | **Errors are values.** `ImageError { kind, offset }` with a closed kind enum; never source text (the `bounded_json` convention); no `unwrap`, `expect`, indexing that can panic, or `unreachable!` on any path; `forbid(unsafe_code)` |
| P6 | **Canonical only.** Reserved fields and flags must be zero, trailing bytes are `Trailing`, counts must equal what the payload holds, and a board must re-emit byte-identically (section 5.1) |
| P7 | **Bounded work.** One forward pass; the only super-linear step is the sort of at most 64 ranges; work is at most `64 * len + 4096` basic operations, which the fuzz targets enforce |
| P8 | **No recursion.** Every format is flat |
| P9 | **Pure.** `&[u8]` in, value out. File access belongs to `emu-host`: it opens once, `fstat`s the open descriptor, requires a regular file within the cap, reads at most `cap + 1` bytes from that descriptor, and does not follow symlinks (`O_NOFOLLOW` or equivalent), so a path cannot change between check and read. Outputs are `O_EXCL` in a run directory and never an input path |
| P10 | **Data cannot choose paths or effects.** Slot-to-file binding is in the run configuration; a name inside an image or board is a label, never a path |

### 8.4 The container format

Binary images share a 16-byte little-endian header. The board is text and has
none.

```text
offset  size  field
0       4     magic       "EMUP" ptape, "EMUC" cards, "EMUT" mtape,
                          "EMUD" disk,  "EMUM" meminit, "EMUS" snapshot,
                          "EMUL" input log
4       2     version     must be 1
6       2     flags       must be 0
8       4     count       frames, cards, records, blocks, segments or sections
12      4     payload_len bytes after the header (snapshot: before the trailer)
16      ...   payload     per kind; total length == 16 + payload_len (+32 snapshot)
```

| Kind | Payload |
|---|---|
| `ptape` | `count` frame bytes; `payload_len == count` |
| `cards` | `count` cards of 80 columns, each a `u16` with the top 4 bits zero; `payload_len == count * 160` |
| `mtape` | records back to back, each `u32 len` then `len` bytes; `len == 0` is a tape mark; end of payload is end of tape. `count` equals the number of records and marks |
| `disk` | `u32 block_size`, `u32 block_count` (equal to `count`), then exactly `block_size * block_count` bytes, the product taken in `u64` |
| `meminit` | `count` segments, each `u32 base`, `u32 len` (non-zero) and `len` bytes; sorted by `base`, pairwise disjoint |
| `inputlog` | `count` events, each `u64 at`, `u8 dev`, `u8 reserved` (zero), `u16 len` (at most `MAX_EVENT_BYTES`) and `len` bytes; `at` is non-decreasing and at most `MAX_TICK`; `count` at most `MAX_INPUT_EVENTS`. At attach, `dev` must be below the board's device count and name a device that accepts host events, or `BadDevice` |

Whether a frame fits the attached device (5-bit frames, 36-bit words packed in
five bytes, a segment inside one RAM or ROM range) is checked at *attach*
time against the validated board, not in the parser: the parser knows the
format, the attach step knows the machine. Both are total.

### 8.5 Negative-control vectors

`emu-image` carries a vector corpus (`vectors/v1/`, one file per vector, with
the expected `ImageErrorKind` and offset). Every parser has at least:

* empty input; header cut at every byte offset (0..=15); each payload cut one
  byte short;
* exactly at the cap (accepted) and one past it (`TooLarge`), for every
  constant in section 8.2;
* **length bombs:** `count = 0xFFFFFFFF` and `payload_len = 0xFFFFFFFF` in a
  16-byte file; a record `len` of `0xFFFFFFFF`; `block_size * block_count`
  overflowing `u32` and fitting `u64` above the cap; `count * 160` overflowing
  `u32`;
* wrong magic, version 0, version 2, non-zero flags, non-zero reserved bits,
  one trailing byte;
* disorder: unsorted and overlapping `meminit` segments, a segment of length
  zero;
* board-specific: overlapping ranges; `base + size` wrapping `u64` and
  reaching exactly `2^addr-bits` (accepted) and one past it (rejected); an
  unknown device kind, a duplicate name, a dangling name, an irq line out of
  range, a `0X10` and a `16` where `0x10` is the only spelling, a CRLF line, a
  tab, a non-ASCII byte, a line of 129 bytes, 513 lines, a missing `end`;
* **board value bombs:** `addr-bits` 0, 33 and 255; `unit-bits` 0 and 65; a `ram`
  size of `0xffffffffffffffff`, of `MAX_RAM_BYTES + 1` and of exactly
  `MAX_RAM_BYTES` (accepted); an `rx-fifo` or `tx-fifo` of `MAX_FIFO + 1`; a
  `log-cap` of `MAX_LOG_CAP + 1`; a disk of `MAX_DISK_BLOCKS + 1`; devices each
  at their ceiling whose sum exceeds `MAX_TOTAL_STATE_BYTES`; a hex token of 17
  digits; an `image` slot above its kind cap and above its bound device;
* **time vectors:** an event with `at = MAX_TICK + 1` (rejected) and `at =
  MAX_TICK` (accepted); a timer `compare` of `u64::MAX` (means never); and, on
  the toy machine, `now = MAX_TICK - 1` with a retire of cost 2 (`TimeOverflow`)
  and an idle jump to a deadline beyond `max_ticks` (`BudgetExhausted(Ticks)`);
* **budget vectors:** a `RunBudget` with `max_ticks = MAX_TICK + 1` (rejected) and with `max_ticks = MAX_TICK` (accepted); a live host asked to stamp an event beyond `MAX_TICK` (the run ends with `BudgetExhausted(Ticks)` and the recorded log still parses);
* **input-log vectors:** ticks out of order; `len = 0xFFFF`; `dev` beyond the
  board; non-zero `reserved`; `count = 0xFFFFFFFF` in a 16-byte file;
* snapshot-specific: section 9.

**The guard is pinned at the sink.** An allocation-counting test binary runs
every vector and asserts peak allocation is at most `2 * len + 4 KiB`, the
bound P3 promises. A sabotage mode (`--cfg emu_sabotage`) raises every cap to
`usize::MAX`; the bomb vectors must then fail. It is test-only by construction:
`emu-image` contains `compile_error!` when `emu_sabotage` is set and `test` is
not, so no release or library build can carry it, and a CI step asserts the cfg
is absent from the release jobs' flags. The sabotage tests live inside
`emu-image`'s own `#[cfg(test)]` modules. This was chosen over a separate
test-crate copy because a copy of the parsers would drift from the shipped
ones, and a test that exercises a copy proves nothing about the original. A guard whose test still passes
with the guard removed is not a guard (the repository's standing lesson on
DoS guards).

### 8.6 Fuzz targets

One VER06 target per parser, in the `test-bootstrap` harness style
(`not ok <N> - <target_id> [fuzz-crash:<sig>]`):

| Target id | Input | Properties |
|---|---|---|
| `emu/fuzz/board` | any bytes | `validate` never panics; `Ok(b)` implies re-emit equals input; peak allocation within P3; work within P7 |
| `emu/fuzz/ptape`, `cards`, `mtape`, `disk`, `meminit` | any bytes | parse never panics; `Ok(x)` implies `encode(x)` equals input; allocation and work bounds |
| `emu/fuzz/inputlog` | any bytes, attached to a fixed small board | parse never panics; `Ok(x)` implies `encode(x)` equals input; ticks monotone and at most `MAX_TICK`; allocation and work bounds |
| `emu/fuzz/cpu-<isa>` | arbitrary program bytes (section 7.4) | no panic; at most `MAX_REQUESTS` per retire; defined final status |
| `emu/fuzz/snapshot` | any bytes, restored against a fixed small board | restore never panics; on `Err` the live machine is unchanged; on `Ok` the restored machine re-snapshots to the input bytes |

Seeds are the valid vectors and the negative vectors. VER06 audit item 10
records that the engine has no coverage instrumentation, so "coverage guided"
is only as good as the target's own edge reporting; `emu-image` therefore
exposes an edge counter behind a `cov` cargo feature (off in shipped builds)
and the evidence label is `randomized` (VOS-D018). An `exhaustive` claim is
possible only at a reduced configuration: with the caps shrunk to a few bytes
every input of length 0..=3 can be enumerated against the header parser, and
that run is labelled `exhaustive` for that instance and printed with the
shrunken constants (VOS00 section 9.3 rule 1).

### 8.7 Resolution of VOS-D001

VOS-D001 is **resolved in this specification**: maximum sizes and counts
(section 8.2), fail-closed value-returning parsing that cannot panic and
cannot allocate from an unvalidated length (rules P1-P10), a fuzz target per
parser (section 8.6) and negative-control vectors with a sabotage mode
(section 8.5). Its parsers are the seven binary kinds (`ptape`, `cards`, `mtape`,
`disk`, `meminit`, `inputlog`, `snapshot`) and the board text format, each with
a target and vectors. The implementation lands in VOS-005 (container, board,
`meminit`, input log, snapshot), VOS-006 (paper tape), VOS-008 (cards, magnetic tape) and
VOS-014 (disk). Until each parser exists, no image of that kind is accepted
by any machine.

---

## 9. Snapshot and restore

A snapshot is `snapshot-v1`: the 16-byte header (`count` is the number of
device sections), then

```text
32 bytes   board digest (SHA-256 of the accepted board file)
 8 bytes   now: Tick (u64)
 4 bytes   cpu_len (u32), then cpu_len bytes: the adapter's canonical state
 per device, in device-index order, no gaps:
   1 byte    device index (u8, equal to its position)   4 bytes  len (u32)   then len bytes
32 bytes   trailer: SHA-256 of every byte before it
```

**Restore is an untrusted parse** under rules P1-P10, with these steps in this
order, and the first failure returns an error and leaves the live machine
untouched: (1) size cap; (2) header and flags; (3) the board digest equals the
running board's, or `WrongBoard`; (4) `cpu_len` and each device `len` are at
most that component's declared maximum state size (a per-component constant,
the source of the board-derived bound; the constant ceiling is
`MAX_SNAPSHOT_BYTES`, section 8.2); (5) every device appears once, its index
equal to its position (a `u8`, as `DevId` and `MAX_DEVICES` allow), and the
section lengths sum exactly to `payload_len`, so the sections end exactly at
the trailer with no gap and no byte left over; (6) the trailer digest matches; (7) each component's
own `validate_state` (the invariant `inv(M)` of its contract; for the
simulators with a validating `restore`, that function) accepts its section;
(8) only then is a new `Machine` built and swapped in. A passing digest proves
the bytes are the ones that were written; it does not prove they are a legal
machine state, and an attacker who builds a snapshot computes the digest
too. The SHA-256 trailer detects accidental corruption only; it does not detect
tampering, and the snapshot is never called tamper-proof. Authenticating one
would need an HMAC or a signature under a key the host holds, which is out of
scope. Step 7 is what makes the snapshot safe to load. Restore is atomic.

**Size accounting.** Storage is counted once. An `owner=cpu` store (the EDSAC `store`) is a `ram` device in the board, so V9 counts its `size` toward `MAX_TOTAL_STATE_BYTES`; the adapter's `cpu_len` section holds that storage together with the registers. Its declared maximum is therefore `size` of every `owner=cpu` device plus a fixed per-ISA register allowance, and restore buffers are sized from the sum of the declared device sizes plus those allowances, which V9 has already bounded, never from a field in the file. The `MAX_SNAPSHOT_BYTES` ceiling covers `MAX_TOTAL_STATE_BYTES` plus the allowances and framing.

**Versioning.** `version` is 1. Any other value is `BadVersion`; there is no
silent upgrade. A format change is a new version plus a separate migration
tool that reads version `n`, writes version `n+1` and records both digests.

**Claims (host-level, on a small toy machine):**

| Claim | Statement |
|---|---|
| `snapshot_roundtrip` | `restore(snapshot(m)) == m` structurally, and running `k` steps from either gives identical traces |
| `snapshot_canonical` | `snapshot(restore(b)) == b` for every accepted `b` |
| `snapshot_corruption` | Flipping any one byte of a valid toy snapshot yields an error, never an accepted different machine; enumerated over every byte position and all 255 other values (about 51,000 cases for a 200-byte toy), `exhaustive` for that instance |
| `snapshot_invariant` | A snapshot whose digest is valid but whose component state violates `inv(M)` is rejected at step 7 (negative control: a hand-built snapshot with a recomputed digest) |

State digests also serve evidence: a report may pin the digest of the machine
state at a named tick, which `snapshot_canonical` makes comparable across runs
and hosts. For the four simulators that cannot restore today (704, 8080,
GE-225, PDP-11) the additive `restore` accessors are VOS-D011; until one
lands, that ISA has no snapshot and its report says so.

---

## 10. Evidence per contract

The bars are those of VOS00 section 9.2, applied as in VOS01 section 4. Every
cell inherits VOS00 section 9.3: instance constants are printed, counts are
manifest-derived, and a claim that rests on `-any` says so. The emulator is
not an RV32 image, so **X3 ("proved on the image") does not apply as defined**.
The nearest analogue is a host-level `proved` for a leaf that fits the VER25a
subset, which needs WX-SP1 and is promised for none. The right-hand column
says what each X1 instance would be.

| Contract | X1 (reduced, exhaustive) | X2 (shipped size) | Reachability and waits |
|---|---|---|---|
| CPU core (adapter) | `cpu_agrees`: exhaustive single-instruction sweep where decode space is small (8080, Z80, PDP-8); `exhaustive-sampled` elsewhere | + whole-program differential corpus; the refinement monitor over a boot | X2 now; no X3 |
| Bus | `bus_disjoint`, `bus_total`, `bus_frame`, `bus_lookup_ref` over an 8-address space with up to two ranges (about 21 thousand cases; the real instance is fixed when VOS-005 writes the claim, VOS00 section 14.3 D4) | + randomized maps at 64 ranges and a full 32-bit space | X1 now; waits on the lemma kind for the model-level statements (VOS-D005) |
| Interrupt controller | all states and operation sequences at `L = 4` lines, priorities 0..3 | + randomized at the shipped `L`; `irq-any` soundness and coverage with a witness and a negative control (VOS01 section 3.3) | X1 now |
| Timer | `timer_exact`, `timer_monotone`, `timer_past_compare` over a small tick range | + randomized at 64-bit ticks | X1 now |
| Console | `con_*` over a 2-byte FIFO and a 3-byte input | + property and fuzz with long logs and the backpressure path | X1 now |
| Disk | `disk_*` over 4 blocks of 2 bytes | + property at 128-byte blocks | X1 now |
| Tape and cards | `tape_*` over short sequences | + property at the caps | X1 now |
| Board validator | `board_*` over a reduced grammar (section 8.6 style) | + fuzz target `emu/fuzz/board` | X1 now |
| Machine spine | `replay_deterministic`, `snapshot_roundtrip`, `device_isolation` over a toy machine (2-instruction ISA, one device) | + replay of every rung's boot twice, byte compare | X1 now; the toy machine is the first deliverable of VOS-005 |
| Parsers, snapshot | caps shrunk; every input of length 0..=3 | + `emu/fuzz/*` (`randomized`) and vectors | `exhaustive` for the reduced instance, `randomized` otherwise |

**Labelling emulator measurements (VER29 section 7.1).** Instruction counts and
other observations the emulator measures on an image (the number of
instructions a kernel entry takes; an equal-count sweep over inputs) are
exact on this machine, because a tick is one retired instruction (section
6.2), but VER29 section 7.1 classes a bench observation as `example` and
nothing here changes that: **an emulator measurement is labelled `example`
even for a complete sweep**, until a bench-kind manifest entry can declare its
domain complete (VOS-D004). Every such observation prints its *stack* as
`emu:<board>@<digest16>` (the first 16 hex digits of the pinned board digest; a report also carries the full digest, and the VER13 AC-7 equality compares the full pinned digest, never the label), so a
reader sees that it was measured on our model, not on a host binary
(`host-rustc`) or on silicon (Q14).

---

## 11. Trusted base and VER13 entries

What a machine-level result is trusted to rest on:

| Item | Status | Down route |
|---|---|---|
| Each ISA model | Trusted | Differential between the functional simulator and its gate-level partner (each exists in the tree, section 2.2); RISC-V also an optional QEMU `virt` lane |
| Each board's device models as a model of the real peripherals | Trusted; for ladder boards a concept recreation, labelled (VOS00 section 6.3) | None; the evidence shows internal consistency only |
| Adapter glue (the peek table, the retry rules) | Evidenced, not trusted, by the `cpu_agrees` corpus; a gap in the corpus is a gap in the evidence | The sweeps of section 7.4 |
| Parsers, validator, snapshot codec | Evidenced (sections 8 and 9) | Vectors, fuzz, exhaustive reduced instance |
| The SHA-256 implementation that makes digests | In-repo (`coding_adventures_sha256`, which depends on `zeroize`, also in-repo); trusted as a tested library | Its own tests; a bad hash weakens identity, not state validation |
| `emu-host` | Outside the replay claim: a replay needs no host adapter | n/a |
| Silicon | Out of scope (VOS00 section 14.4) | None |

**New catalogue entries (VER13 section 5.2 schema), both `status: assumed`.**

| Field | `mc.<isa>.model-faithful` | `emu.board.<name>.faithful` |
|---|---|---|
| id | one per ISA: `mc.edsac`, `mc.ibm704`, `mc.pdp8`, `mc.ge225`, `mc.i8080`, `mc.z80`, `mc.pdp11`, `mc.rv32i`, `mc.s370` (and `mc.agc` when it exists), each `.model-faithful` | one per board: `emu.board.edsac-functional.faithful`, `emu.board.rv32-virt-lite.faithful`, and so on |
| layer, family, kind | `mc`, `mc`, `model` | `mc`, `emu`, `model` |
| statement | The ISA model the emulator executes matches the ISA specification it names, for every instruction the model accepts; deviations are listed in the entry's notes (for example the Z80's missing EI delay, VOS-D017) | The board's map, device set and device contracts are the machine the OS evidence assumes; for a ladder board this is a concept recreation and the entry says "stand-in" where VOS00 section 6.3 labels one |
| `refines` | `mc.isa.model-faithful` (the existing RISC-V entry, section 3.6 there); the new `mc.rv32i...` entry is its child, so by VER13 AC-6 the parent discharges only when every child does | none |
| depends_on | none | the `mc.<isa>.model-faithful` of its `isa` line |
| discharge | functional versus gate-level differential (evidence kinds `isa/` and `gate/`, tools: the `-gatelevel` crate); RISC-V additionally `isa/qemu-virt-diff`, which is optional and cannot by itself move the status past `evidenced` | board validated as data; digest pinned; device contracts at X1/X2 (internal consistency only; the fidelity claim itself stays `assumed`) |
| `used_by` | every rung and profile report | every rung and profile report |
| backlog | VOS-005 onward, per ISA | VOS-007, VOS-014, VOS-016, VOS-017, VOS-021 |

VER13's schema lists `family` values `spec|src|wir|cc|mc|uarch|gate|phys|tcb|
os|gov|att`; `emu` is not among them, so the board entries need that
one-word schema amendment through the catalogue's own process (VOS-D014). The
QEMU lane is a cross-check, not a dependency: QEMU is never built by the
repository, never a CI gate, and no entry assumes it. Pins: each OS report
records the board digest, the image digests, the input-log digest and the
`mc.*` and `emu.board.*` ids it leans on (VOS00 section 10.3).

---

## 12. Crate plan and backlog mapping

All crates use the `emu-*` prefix (VOS00 section 13) and sit in
`code/packages/rust`. Two crates are added to the brief's list because they
are different classes (section 3.3): `emu-image` (class P) and `emu-host`
(class H).

| Crate | Class | Contents | Delivered by |
|---|---|---|---|
| `emu-core` | C, S | Types of section 4.0, bus map and validator use, `machine_step`, device trait (static dispatch), recording bus, trace ring, the toy machine used for X1 | VOS-005 |
| `emu-image` | P | Board and image parsers (including the input log), validator, snapshot codec, vector corpus, fuzz targets, `cov` feature | VOS-005 (container, board, `meminit`, input log, snapshot); VOS-006 (ptape); VOS-008 (cards, mtape); VOS-014 (disk) |
| `emu-host` | H | Bounded file reads, terminal, pacing, input log writer, logs | VOS-006 (console) and VOS-007 (first runnable) |
| `emu-dev-console` | C | `uart` and `teleprinter` | VOS-006 |
| `emu-dev-ptape` | C | Paper tape reader and punch | VOS-006 |
| `emu-dev-tape` | C | Card reader and punch, magnetic tape | VOS-008 |
| `emu-dev-irq`, `emu-dev-timer` | C | Interrupt controller (with `irq-any`) and timer | VOS-013 |
| `emu-dev-disk` | C | Block device | VOS-014 |
| `emu-cpu-edsac` | M | Shape D adapter | VOS-007 |
| `emu-cpu-ibm704` | M | Shape N then H, after the spec-first simulator extension | VOS-008 |
| `emu-cpu-pdp8` | M | Shape H | VOS-017 |
| `emu-cpu-ge225` | M | Shape D | VOS-016 |
| `emu-cpu-i8080` | M | Shape H (latch and peek) | VOS-014 |
| `emu-cpu-z80` | M | Shape H plus interrupts | no row (VOS-D017) |
| `emu-cpu-pdp11` | M | Shape B, new core | VOS-041 (VOS-D013); MMU VOS-030 |
| `emu-cpu-rv32` | M | Shape B, new core; M and U modes, trap CSRs, then PMP | VOS-020, VOS-021 |
| `emu-cpu-agc` | M | Over the new AGC simulator | VOS-018 |
| `emu-cpu-s370` | M | After the privileged slice | VOS-032 |
| `emu-board-<name>` | S | The board file, its closed device enum, glue, and a test pinning the file's digest | `edsac` VOS-007; `ibm704` VOS-008/009; `i8080-cpm` VOS-014; `ge225-tss` VOS-016; `pdp8` VOS-017; `agc` VOS-018/019; `rv32-virt` VOS-021 |

Order: VOS-005 first (toy machine, containers, board parser, spine), then
VOS-006, VOS-007 (the first rung boots), then the rest as in the backlog.

---

## 13. Open questions

Each has a recommended answer, which this document assumes until decided.

| # | Question | Recommendation |
|---|---|---|
| Q1 | Does `machine_step` advance one retired instruction or one bus request? | One retired instruction: the monitor and the trace work at instruction granularity, and the inner request loop is bounded by `MAX_REQUESTS` |
| Q2 | Is a tick an instruction or a cycle? | An instruction at cost 1 now; a per-ISA cost table as a later board parameter, with no contract change |
| Q3 | Board format: canonical text or JSON through `bounded_json`? | Canonical text (section 5.1): canonicalization is the parse |
| Q4 | Are the shape rules scoped by crate class (section 3.3)? | Yes; amend VOS01 section 1.4 and the conformance checklist to say which classes each SH rule binds (VOS-D010) |
| Q5 | Add additive accessors to the audited simulators (`restore`, EDSAC `feed_input`/`take_output`) or work around them? | Add them, each as a small PR to the simulator with a test that the old API is unchanged; they are needed for snapshot anyway (VOS-D011). Port peek stays in the adapter |
| Q6 | DMA devices? | Programmed I/O only for the ladder. When the first DMA device is scheduled (a virtio-like disk, GE-225 card DMA), devices emit `DmaRequest` outputs which the spine executes at an instruction boundary, so no device calls the bus |
| Q7 | Where does ELF parsing live? | Outside the emulator: a host tool turns an ELF into a `meminit` image; the emulator never parses ELF |
| Q8 | Which QEMU version for the RISC-V lane? | Pinned in VOS-021, with the memory map of section 5.2 checked against it |
| Q9 | Who owns the PDP-11 machine work between VOS-013 and VOS-023? | A new backlog row, VOS-041 (VOS-D013) |
| Q10 | One global clock or per-device clock domains? | One global tick; a device that needs a divider counts ticks itself |
| Q11 | New RV32 core or extend `Rv32ISimulator`? | New core `emu-cpu-rv32` reusing the decoder; old crate untouched (section 7.3) |
| Q12 | Torn writes and power loss on disks and tape? | Out of scope for v1; block writes are atomic in the model |
| Q13 | More than one CPU? | Out of scope; one hart, one CPU |
| Q14 | The `stack` token for emulator observations? | `emu:<board>@<digest16>` (section 10); the label is for reading and AC-7 compares the full pinned digest, so it needs no schema change |

---

## 14. What this does not buy

* **No real hardware.** An ISA model is trusted, not proved against silicon
  or against a vendor's RTL; a differential is a cross-check, not a proof.
* **No cycle accuracy.** Time is instruction count (VOS00 rule 8); a result
  about ticks says nothing about cycles or real time.
* **No fidelity of period peripherals.** A ladder device is a concept
  recreation (VOS00 section 6.3); the contracts check internal consistency.
* **No guest-escape guarantee beyond the core.** The core has an empty
  capability certificate, so a guest cannot cause host effects except through
  `emu-host`; that is a statement about the core, not about the host adapter.
* **No DMA, SMP, MMU or privilege on the ladder machines yet.** Each is a
  later row.
* **No implementation.** D001 is resolved in specification only; until the
  parsers exist the machine accepts no image.
* **No containment beyond unwinding.** A panic in a wrapped model is caught
  and reported (`Fault(ModelPanic)`, section 7.5); that needs `panic =
  "unwind"` and is not a safety proof. A run is always bounded by its budget
  (section 6.1), but the budget bounds resources, not correctness.
* **No authenticated snapshots or logs.** Digests detect corruption, not
  tampering (section 9).
* **No speed.** Per-step cloning (section 2.3 C5) is accepted until measured.
* **No proof of `rustc` and no `proved` label for any emulator component**
  (section 10).

---

## 15. Findings against VOS00, VOS01 and the brief

Found by reading the code (section 2). Each is logged in
[the backlog](VOS-BACKLOG.md) under "Discovered work". None changes a decision
in section 1; each changes a sentence.

| # | Finding | Where it bites | Backlog |
|---|---|---|---|
| G1 | VOS00 section 5.4 files EDSAC and the GE-225 under "needs only a hook". Both simulators carry their peripherals inside their state (and the GE-225 has a full device-controller and interrupt model); the 8080 and Z80 expose latches, not hooks (a peek is needed); the 704 has no I/O at all | Adapter shapes, sections 2.3 and 7 | D016 |
| G2 | VOS00 section 5.2 and VOS01 section 1.4 apply SH1-SH16 to the emulator; the simulators use heap, `String` and `std::error`, and the emulator is not an image | Class split, section 3.3 | D010 |
| G3 | VOS00 section 2.3 omits `mscratch`, the checked simulator's code-only fetch, errors-not-traps, and `mret` setting only `MIE`; and that the Z80 has interrupt methods and the 8080 none | Sections 2.1, 7.3 | D016 |
| G4 | Four simulators cannot restore state; EDSAC's output and the GE-225 typewriter output grow unbounded and cannot be drained | Snapshot, console | D011 |
| G5 | Every simulator step clones O(memory) state | Throughput | D012 |
| G6 | No backlog row builds the PDP-11 traps, interrupts and I/O page that VOS-023 (L6) needs; VOS-030 is the later MMU | VOS-023 | D013 |
| G7 | VER13's `family` enumeration lacks `emu`; `mc.isa.model-faithful` already exists for RISC-V and the new per-ISA ids must refine it | Section 11 | D014 |
| G8 | The 704 FP path and `RiscVSimulator` host ecalls use host `f64` and `powi`: replay is per-host for them | Section 6.3 | D015 |
| G9 | The Z80 lacks the EI delay; the 8080 has no interrupt delivery, so VOS-014/015 (L7) can use no timer interrupt; the Z80 adapter has no row | Section 7.2 | D017 |
| G10 | VER06 has no coverage instrumentation, so parser fuzzing is black-box unless `emu-image` reports edges | Section 8.6 | D018 |
| G11 | VOS-016 (L3) needs several teletypes; the GE-225 has controller plugs but no row builds a multi-terminal device | VOS-016 | D019 |

---

## 16. Sources

Read for this spec (paths relative to `code/`): `packages/rust/{edsac,ibm704,pdp8,ge225,intel8080,z80,pdp11,riscv,ibm-system370}-simulator/src`
and READMEs; `packages/rust/sim-pdp8-ge225-io-bridge/src/lib.rs`;
`packages/rust/bounded-json`; `packages/rust/sha256`; `packages/rust/test-bootstrap`
(the VER06 harness); and the specs named in the header. The QEMU `virt` memory
map in section 5.2 is from memory and is marked **[unverified]** there; nothing
else in this document depends on QEMU.
