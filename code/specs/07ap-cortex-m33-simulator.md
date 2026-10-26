# Layer 07ap — Arm Cortex-M33 Functional Simulator

## 1. Status and public evidence

This cell defines a checked, bounded functional model of one Arm Cortex-M33
core. Cortex-M33 implements the Armv8-M Mainline profile. The normative public
references are Arm's *Cortex-M33 Devices Generic User Guide*, document 100235,
the *Cortex-M33 Processor Technical Reference Manual*, document 100230, the
*Armv8-M Architecture Reference Manual*, DDI 0553, and Arm's public Cortex-M33
processor datasheet.

Those sources establish the Mainline programmer-visible core, optional
Security Extension, Secure and Non-secure processor states, banked stack and
mask/control registers, stack-limit registers, and programmable Security
Attribution Unit (SAU). They also distinguish the processor core from optional
floating-point, MPU, debug, trace, integration, and vendor-SoC features.

This package is an architectural teaching model. It is not Cortex-M33 RTL, a
pipeline or cycle-timing model, a power model, an NVIC, an MPU, an AHB5
interconnect, an IDAU, or a vendor microcontroller.

## 2. Bounded core and inherited instruction surface

The functional execution base is the repository's checked Cortex-M3 Armv7-M
cell. Cortex-M33 is upward compatible with that bounded Thumb/Thumb-2 integer
surface, so ordinary instructions delegate to `cortex-m3-simulator` and must
produce the same integer registers, flags, memory, raw bytes, width, mnemonic,
and lifecycle outcomes.

The first Cortex-M33 cell adds the Armv8-M Mainline state boundary around that
oracle:

- explicit Security Extension configuration, with Secure reset when enabled
  and permanently Non-secure operation when omitted;
- Secure and Non-secure banks for MSP, PSP, MSPLIM, PSPLIM, PRIMASK, BASEPRI,
  FAULTMASK, and CONTROL;
- eight 32-byte-granular SAU region descriptors plus ENABLE and ALLNS control;
- deterministic Secure, Non-secure, and Non-secure Callable (NSC) address
  attribution, including overlap-to-Secure resolution;
- checked Secure-to-Non-secure and NSC-gateway transition hooks for later
  encoded-transition and exception cells; and
- attribution checks for instruction fetch, inherited data accesses, and
  direct memory operations.

This bounded cell does not claim the Cortex-M33 DSP extension, optional FPU,
every Armv8-M-exclusive instruction, encoded SG/BXNS/BLXNS/TT behavior, or
exception entry and return. Those surfaces require their own decode,
reserved-neighbour, context, and atomicity witnesses before they can be added.

## 3. Complete architectural state

`CortexM33State` owns R0-R12, LR, PC, APSR, IPSR, IT state, Thread/Handler
mode, the two security banks of stack pointers, stack limits, interrupt masks,
priority mask, fault mask, and CONTROL, the current security state, Security
Extension configuration, SAU control and regions, exclusive-monitor and
sleep/event state, halt state, the complete 64 KiB little-endian teaching
memory, and installed-image origin and length.

With the Security Extension enabled, reset enters privileged Secure Thread
mode using Secure MSP. The SAU is disabled, ALLNS is clear, and every owned
address is therefore Secure until Secure privileged software changes policy.
With the extension omitted, the core is permanently Non-secure, SAU
configuration is unavailable, and every owned address is Non-secure. All four
stack pointers start at the top of owned memory and all stack limits start at
zero.

Restore validates memory size, PC and 4-byte stack-pointer alignment,
8-byte stack-limit alignment,
Thumb state, mode/IPSR agreement, selected-stack consistency, stack-limit
alignment and ordering, installed-image range, extension/security consistency,
SAU availability and region count, 32-byte granularity, inclusive limits, and
owned-memory bounds before committing any state.

## 4. Security attribution and access rules

An enabled SAU region has a 32-byte-aligned base and an inclusive limit whose
low five bits are one. One matching region returns Non-secure or NSC. Two or
more matching enabled regions return Secure. With no match, ALLNS selects
Non-secure when set and Secure otherwise. A core without the Security
Extension always returns Non-secure.

Secure execution may fetch and access any attributed region. A Non-secure
normal fetch or data access requires the complete byte range to be
Non-secure. NSC is accepted only by the explicit secure-gateway transition.
Attribution, privilege, stack-limit, alignment, and bounds checks settle on a
candidate transition before state commits. Any failure is typed and preserves
the complete prior state.

Only privileged Secure software may configure the SAU or Secure banks.
Non-secure software can observe and update only the active Non-secure banks.
Switching security state saves the active special registers to their bank and
loads the target bank without changing the shared R0-R12 data registers.

## 5. Checked lifecycle and traces

Program load is nonempty, halfword aligned, in range, and resets the selected
configuration before installing bytes. Direct register, banked-register,
memory, attribution, and transition APIs are checked. `step` executes against
a candidate complete state and commits only after fetch, decode, attribution,
data-access, stack-limit, and inherited-core validation succeeds.

Every successful `StepTrace` includes exact raw bytes, width, mnemonic,
PC-before/after, security state before/after, and complete M33 before/after
snapshots. `run` retains no more than 64 complete traces under an 8 MiB
snapshot budget, preflights fallible allocation, and rolls back the complete
run on the first fault or step-limit exhaustion. `execute` atomically combines
load and bounded run.

## 6. Completion gate

The bounded cell is complete only when it has:

1. exact differential witnesses for every inherited Cortex-M3 decode family;
2. Secure/Non-secure reset, bank switching, stack-limit, SAU default,
   one-region, disabled, ALLNS, overlap, NSC, privilege, and no-extension tests;
3. fetch and inherited memory-family attribution witnesses with typed atomic
   failures;
4. validated restore, load, direct access, transition, halt/sleep, retention,
   run rollback, and execute rollback tests;
5. package README, changelog, BUILD, workspace membership, and backlog status;
6. release line coverage above 80%, targeting at least 90%; and
7. Rustfmt, strict Clippy, rustdoc, release tests, affected-package checks,
   lessons validation, and independent security review before publication.

## 7. Deferred work

DSP instructions, optional floating point, encoded Armv8-M security and Test
Target instructions, exception stacking and return, interrupt targeting,
MPU/IDAU composition, NVIC/SysTick, AHB5 transactions, cacheless bus timing,
debug/trace, and implementation timing belong to later functional, SoC, and
gate-level cells. No proprietary Cortex-M33 netlist or transistor topology is
inferred by this package.
