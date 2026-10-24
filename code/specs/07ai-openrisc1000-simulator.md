# Layer 07ai — OpenRISC 1000 Functional Simulator

## Status, priority, and sources

This layer defines the repository's checked instruction-level model of the
32-bit OpenRISC 1000 architecture. It is RCPU-069 in the Rust CPU simulator
wave. RCPU-067 (HP PA-RISC 1.0) remains in research because a sufficiently
complete public primary-source implementation boundary has not yet been
established; this ready, independent OpenRISC cell therefore runs while older
publication PRs wait for CI.

The normative source is the OpenRISC project's *OpenRISC 1000 Architecture
Manual*, architecture version 1.4, document revision 0 (20 February 2022).
The separate `openrisc/mor1kx` RTL repository is the implementation reference
for the later RCPU-070 gate-level cell. This functional layer deliberately does
not infer implementation-specific pipeline, cache, MMU, interrupt-controller,
or bus timing from the ISA manual.

## Architectural boundary

The simulator implements 32-bit, big-endian ORBIS32 scalar integer execution
with the manual's no-delay-slot configuration. It includes all documented
scalar integer Class I instructions and the architecturally useful Class II
integer operations: rotate, conditional move, bit search and extension,
multiply/divide, multiply-accumulate, synchronization no-ops, atomic load/store,
and the integer special-purpose-register interface.

ORVDX vector instructions, ORFPX floating-point instructions, 64-bit encodings,
custom instruction spaces, MMUs, caches, debug hardware, performance counters,
tick timers, and asynchronous interrupt pins are outside this CPU-only cell.
Their encodings fail as typed unsupported or unknown instructions rather than
silently executing.

## Complete state

`OpenRisc1000State` is an owned snapshot containing:

- all 32 general-purpose registers (the architecture makes zero in `r0` a
  software convention rather than a hard-wired rule);
- current and previous PCs;
- the supervision register fields needed by integer execution, including
  supervisor mode, fixed-one, flag, carry, overflow, and delay-slot state;
- EPCR0, EEAR0, ESR0, MACLO, and MACHI;
- an atomic reservation address;
- the complete 65,536-byte big-endian memory;
- deterministic simulator halt state; and
- installed-program origin and length.

Unsupported 16-bit SPRs read as zero and ignore writes, while documented
configuration SPRs expose their fixed values. Writable CPU SPRs, mapped GPRs,
and MAC registers round-trip through `l.mfspr` and `l.mtspr`; implemented
read-only writes and out-of-range addresses fail atomically. Both instructions
require supervisor mode for CPU-group registers; MACLO and MACHI retain their
architectural user-mode read/write permission.

## Transport and lifecycle

Programs are raw, big-endian 32-bit OpenRISC words. `load(program, origin)`
requires a nonempty, word-aligned, contiguous image that fits memory. A
successful load clears machine state, installs the image, and sets both PC and
previous PC to `origin`. Instruction fetch must remain inside the installed
image; data accesses may use the whole address space but must be naturally
aligned and in range.

`step` is atomic. `run(max_steps)` and `execute(program, origin, max_steps)` are
transactional as a whole. Every successful step returns the PC before and
after, exact four raw bytes and raw word, an exact conventional lowercase
`l.*` mnemonic, and complete state after the transition. At most 1,024 complete
traces may be retained; larger limits fail before allocation or mutation.

The architecture has no halt instruction. For deterministic repository
programs, the conventional semihosting encoding `l.nop 1` enters halt after
producing its trace; every other `l.nop` remains a no-op. A later step returns
`Halted` without mutation.

## Execution semantics

The functional surface includes:

- direct and linked jumps, flag branches, and register jumps;
- immediate high, arithmetic, logical, shift, rotate, and compare operations;
- register arithmetic, logical, shift, rotate, compare, conditional move,
  extension, and bit-search operations;
- signed and unsigned byte, half-word, and word loads and stores;
- signed/unsigned multiplication and division, MAC/MSB accumulation, and MAC
  read-and-clear;
- `l.mfspr`, `l.mtspr`, `l.sys`, `l.trap`, and `l.rfe`;
- `l.msync`, `l.psync`, and `l.csync` as ordering no-ops in the single-core
  memory model; and
- `l.lwa`/`l.swa` with one deterministic address reservation.

Branches and jumps have no architectural delay slot: their target is visible
immediately, link register `r9` receives the following instruction address,
and `CPUCFGR[ND]` reports the selected configuration. `l.jalr r9` is rejected
because the link write would overlap its target operand. `l.sys` and `l.trap`
enter their architectural exception vectors after saving EPCR0/ESR0 and
disabling DME, IME, IEE, and TEE; `l.rfe` restores both. A system call saves the
following PC, while a trap saves the trapping PC. Arithmetic updates carry and
overflow exactly where the manual specifies. Division by zero and signed
overflow are typed atomic faults.

## Errors and acceptance

Typed errors distinguish invalid images and restored states, misaligned or
out-of-range fetch/data access, fetch outside the installed image, unknown or
unsupported instructions, invalid SPR access, divide by zero, signed division
overflow, halted execution, excessive trace retention, and exhausted step
budgets. As the architecture specifies, a failed `l.swa` is a successful
instruction that clears SR[F] and does not write memory. Ordinary writes cancel
an atomic reservation only when their byte ranges overlap its reserved word.
Every error has stable nonempty display text and every failing transition
preserves the complete state.

Completion requires:

1. table-driven decode classification over every primary opcode and every
   implemented ALU, shift, compare, MAC, system, and SPR subfamily;
2. behavioral witnesses for both branch outcomes, signed/unsigned data,
   carry/overflow, jumps/links, exceptions, atomics, and memory widths;
3. lifecycle, restore, rollback, malformed-word, and bounded-execution tests;
4. line coverage above 80% (completion baseline: 94.10%, 749/796); and
5. Rustfmt, strict Clippy, rustdoc, release tests, BUILD integration, lessons
   validation, affected-package checks, and security review before publication.
