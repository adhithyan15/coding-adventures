# Layer 07aj — OpenSPARC T1 Functional Simulator

## Status, priority, and primary sources

This layer defines the checked instruction-level reference for one UltraSPARC
T1 virtual processor (a hardware thread, or *strand*). It is RCPU-073 in the
Rust CPU simulator wave. The immediately older HP PA-RISC and Intel Itanium
rows remain research items because their public, implementation-specific gate
boundaries are not yet complete; OpenSPARC T1 is ready because Oracle published
both the architecture and the complete synthesizable chip RTL.

The normative sources are Oracle's *UltraSPARC Architecture 2005*, draft
D0.9.3b (20 October 2009), and *UltraSPARC T1 Supplement*, draft D2.1
(14 May 2007). Oracle's *OpenSPARC T1 Microarchitecture Specification* is the
implementation reference. The later RCPU-074 gate cell will derive structure
from the released Verilog rather than guessing gates from this behavioral
model.

Primary-source links:

- <https://www.oracle.com/servers/technologies/opensparc-t1-page.html>
- <https://www.oracle.com/docs/tech/systems/t1-07-091020-ua2005-d0-9-3b-hp-ext.pdf>
- <https://www.oracle.com/docs/tech/systems/t1-09-ust1-uasuppl-draft-hp-ext.pdf>
- <https://www.oracle.com/docs/tech/systems/t1-01-opensparct1-micro-arch.pdf>

## Architectural boundary

UltraSPARC T1 is a 64-bit SPARC V9 chip multiprocessor with eight physical
cores and four strands per core. This CPU-only cell models one strand's scalar
integer architectural boundary. It deliberately does not invent timing for the
six-stage pipeline, strand scheduler, floating-point unit, caches, TLBs, MMU,
crossbar, L2 banks, DRAM controllers, JBus, SSI, interrupt pins, or other
strands. Those shared structures belong to the gate and SoC/interconnect cells.

The implemented instruction boundary is the source-backed scalar integer core:

- format 1 `CALL`; format 2 `SETHI`, integer branches, and predicted integer
  branches; format 3 integer arithmetic, logical, shift, multiply/divide,
  control-transfer, register-window, and memory operations;
- 32-bit and 64-bit integer condition codes (`icc` and `xcc`);
- architecturally delayed branches, calls, and indirect jumps, including the
  conditional and unconditional annul rules;
- eight register windows with exact `CWP`, `CANSAVE`, `CANRESTORE`,
  `CLEANWIN`, `OTHERWIN`, and `WSTATE` accounting; and
- naturally aligned big-endian scalar loads and stores of 8, 16, 32, and 64
  bits with the architecture's signed and unsigned extensions.

Floating-point and VIS operations, alternate address spaces, MMU translation,
cache diagnostics, hyperprivileged state, asynchronous traps, implementation-
specific ASRs, and multi-strand scheduling are outside this bounded functional
cell. Their encodings return typed unsupported or privileged-operation errors;
they never silently execute.

## Complete state

`OpenSparcT1State` is an owned instruction-boundary snapshot containing:

- four global-register sets selected by `GL`, eight windows of out/local
  registers, and the logical-to-physical overlap that makes a callee's in
  registers alias its caller's out registers (160 physical 64-bit registers in
  total, matching the T1 microarchitecture);
- `PC` and `nPC`, both 64-bit and word aligned;
- `CCR.icc`, `CCR.xcc`, the 32-bit `Y` register, `ASI`, `PSTATE`, and `GL`;
- all six window-management counters;
- the complete 65,536-byte flat physical teaching memory;
- deterministic halt and pending-annul state; and
- installed-program origin and length.

`%g0` in every global set is hardwired to zero. A state restore validates every
invariant before replacing the current state. The flat memory is deliberately
small but addresses remain 64-bit: an address outside the teaching machine is a
typed fault rather than a masked or wrapped access.

## Transport, lifecycle, and traces

Programs are raw big-endian 32-bit instruction words. `load(program, origin)`
requires a nonempty, word-aligned image whose length is a multiple of four and
which fits memory. A successful load clears all state and installs exactly that
image. Instruction fetch must remain within the installed range; data accesses
may use the whole memory but must be naturally aligned and in range.

`step` is atomic. `run(max_steps)` and `execute(program, origin, max_steps)` are
transactional as a whole. Every successful transition records `PC` and `nPC`
before and after, exact raw bytes and word, an exact conventional mnemonic, and
the complete state after the transition. Trace retention is capped at 1,024
complete states and rejected before allocation or mutation.

UltraSPARC has no halt instruction. For deterministic repository programs,
`ta 0` is the semihosting halt sentinel. It produces a normal trace, advances
the PC pipeline once, and marks the strand halted. A later step returns `Halted`
without mutation. Other taken traps are typed atomic faults in this CPU-only
cell because trap tables, MMU state, and system software are deliberately out
of scope.

## Register windows

The T1 has four global-register levels and eight windows per strand. Logical
registers map as follows:

- `%g0..%g7` select one of four global sets using `GL`;
- `%o0..%o7` and `%l0..%l7` select the current window; and
- `%i0..%i7` alias `%o0..%o7` in the preceding caller window.

Following UltraSPARC Architecture 2005, successful `SAVE` increments `CWP`,
decrements `CANSAVE`, increments `CANRESTORE`, reads its operands from the old
window, and writes its result in the new window. `RESTORE` performs the inverse,
decrementing `CWP`, reading the old window, and writing the new window. Spill,
fill, and clean-window conditions are distinct typed atomic faults.

## Control transfer and delay slots

Ordinary instructions assign `PC <- nPC` and `nPC <- nPC + 4`. A taken delayed
branch assigns `PC <- nPC` and `nPC <- target`; an untaken branch assigns
`PC <- nPC` and `nPC <- nPC + 4`. `CALL` and `JMPL` expose their link write to
the delay-slot instruction. Annulled conditional-not-taken and unconditional
branches skip the delay instruction exactly as defined by the architecture.
Misaligned targets and a control-transfer instruction in an active delay slot
fail with typed atomic errors rather than taking an unmodeled trap.

## Arithmetic and memory semantics

The simulator distinguishes 32-bit `icc` from 64-bit `xcc`. `*cc` arithmetic
updates both widths from the same operation; logical `*cc` operations set N/Z
and clear V/C. Shift encodings select 32- or 64-bit width and mask their counts
accordingly. Division by zero is a typed atomic fault; as UltraSPARC specifies,
signed minimum divided by negative one returns signed minimum. `SETHI` writes a
zero-extended low-word constant.

Instruction fetch is always big-endian. This bounded cell fixes implicit data
accesses to big-endian (`PSTATE.cle = 0`); attempts to restore little-endian
state are rejected until alternate-space and MMU work exists. Loads sign- or
zero-extend as specified, stores keep the low source bits, and all faulting
accesses preserve the complete state.

## Errors and acceptance

Typed errors distinguish malformed images and restored states, invalid
registers, misaligned or out-of-range fetch/data access, fetch outside the
installed image, unknown/unsupported/privileged instructions, divide faults,
window spill/fill/clean faults, unsupported traps, delay-slot violations,
halted execution, excessive trace retention, allocation failure, and exhausted
step budgets. Every error has stable nonempty display text.

Completion requires:

1. decode-classification tests covering every primary format and implemented
   op2/op3 family, including explicit unsupported families;
2. behavioral witnesses for both condition-code widths, register overlap,
   window counters, delay slots and annul, links, signed/unsigned data, all
   memory widths, and fault atomicity;
3. lifecycle, restore, rollback, malformed-image, retention, and bounded-run
   tests;
4. line coverage above 80% (completion baseline: 87.30%, 804/921); and
5. Rustfmt, strict Clippy, rustdoc, release tests, BUILD integration, lessons
   validation, affected-package checks, and security review before publication.
