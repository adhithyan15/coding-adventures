# HOS03 — Modular OS simulation architecture

## Purpose and ordering

The Unix model in [HOS00](HOS00-historical-os-simulator.md) is one guest
profile. This architecture defines a reusable semantic simulation boundary
that can host historical or modern operating-system policies without making
Unix names or transitions part of the engine. Implement this architecture
after the ordered HOS-001 and HOS-002 publications; complete the architecture
probe before expanding the guest catalogue in HOS-003 and later milestones.
Continue fixture parity across implementation lanes as the shared boundary
grows. The priority and publication sequence are tracked in
[HOS-BACKLOG.md](HOS-BACKLOG.md).

## Semantic model

The initial engine is a deterministic event machine. It owns a monotonically
increasing logical tick, a queue of external stimuli, a guest profile's opaque
state, and an ordered event trace. The engine does not infer wall time, read
host files, generate randomness, or execute guest instructions. If the input
queue is empty, one generic `clock.tick` stimulus advances the guest by one
logical step. A run budget counts state transitions, not elapsed host time.

A profile supplies an initial state and a deterministic reducer from
`(state, logicalTick, stimulus)` to `(newState, effects)`. Stimulus and effect
payloads, along with profile state, use finite JSON values: null, booleans,
finite numbers, strings, arrays, and plain records with string keys. Functions,
undefined values, symbols, class instances, accessors, and non-finite numbers
are rejected. A value is limited to 64 nested levels, 100,000 nodes, and one
million UTF-16 code units per string or key. These limits bound recursive
normalization and cloning work. The profile owns operating-system policy and
guest vocabulary: job queues, tasks, scheduling, protection checks, or IPC may
live there. The engine records effects with the current tick and a profile
identifier; it does not interpret them as syscalls, PIDs, interrupts, or
hardware operations. Effects are copied data, not callbacks or host actions.

The engine accepts data-only stimuli and exposes `step()` for one logical
transition. `step()` returns that transition's ordered effects and is equivalent
to `run(1)`; both consume the oldest queued stimulus or synthesize one
`clock.tick` when the queue is empty. The engine also runs a caller-selected
bounded number of transitions and serializes snapshots as JSON. A snapshot contains
the schema version, profile ID, logical tick, opaque profile state, queued
stimuli, and processed stimulus history. Restoring requires the matching
profile and deterministically replays the history before accepting the queued
stimuli. An invalid profile, schema, history, or run budget is an explicit
error; a failed transition does not partially advance the machine. The same
profile plus the same stimuli and budgets must produce byte-stable JSON and
the same ordered trace on the same implementation version.

## Profiles and evidence

The first probe uses two intentionally different, synthetic profiles on the
same engine:

1. A batch monitor accepts jobs, runs them in queue order for finite logical
   service ticks, and emits job lifecycle effects.
2. A modern protected message-passing scheduler accepts tasks, rotates a
   runnable task by quantum on each logical tick, and checks a recipient
   capability before enqueuing an IPC message.

These profiles are semantic examples, not faithful historical or production
operating systems. Shared portable JSON cases provide profile IDs, stimuli,
run budgets, and expected ordered effects. Tests cover public single-step parity with `run(1)`, budget
boundaries, nested JSON payloads, rejection without partial transition, snapshot/restore, replay,
and the fact that one profile's state and vocabulary do not leak into the
other.

## Fidelity boundaries and evolution

The first engine models OS policy only. It is not an instruction-set
simulator, microarchitecture, bus, board, device, or cycle-accurate model.
Those are separate future components with explicit boundaries:

- an ISA/CPU core executes a declared guest instruction stream;
- a bus routes typed memory and I/O transactions;
- devices model registers, queues, and declared input/output behavior;
- a virtual clock supplies deterministic timers and interrupt stimuli;
- an OS profile interprets machine stimuli and owns scheduling, protection,
  storage, and other policy.

Profiles may first run against synthetic stimuli and data-only device effects.
Adding a CPU, bus, device, or board does not grant host access to the model;
external inputs and outputs remain explicit, bounded data. A future machine
composition may connect these components, but this initial probe does not
pretend to emulate hardware. Extend fixtures before adding profiles so each
language can compare the same state transitions and traces.
