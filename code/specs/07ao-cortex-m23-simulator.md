# Layer 07ao — Arm Cortex-M23 Functional Simulator

## 1. Status and public evidence

This cell defines a checked, bounded functional model of one Arm Cortex-M23
core. Cortex-M23 implements the Armv8-M Baseline profile. Its public processor
datasheet describes a two-stage Thumb/Thumb-2 core and makes the Security
Extension, Security Attribution Unit (SAU), MPU, debug, trace, and integration
features configurable rather than universal properties of every device.

The normative public references for this boundary are Arm's *Cortex-M23
Devices Generic User Guide*, DUI 1095A, the *Cortex-M23 Processor Technical
Reference Manual*, DDI 0550, the *Armv8-M Architecture Reference Manual*, DDI
0553, and Arm's public Cortex-M23 processor datasheet. In particular, these
sources establish that:

- Cortex-M23 implements Armv8-M Baseline and a Thumb/Thumb-2 subset;
- the Security Extension provides Secure and Non-secure processor states;
- the optional SAU has zero, four, or eight regions and attributes instruction
  addresses as Secure, Non-secure, or Non-secure Callable (NSC);
- overlapping enabled SAU regions resolve to Secure, regardless of their
  programmed attributes; and
- reset on a Security-Extension implementation starts with the programmable
  map Secure until Secure software changes SAU policy.

This package is an architectural teaching model, not a Cortex-M23 RTL,
pipeline, timing, power, NVIC, MPU, AHB5, debug, trace, IDAU, or vendor-SoC
model.

## 2. Bounded core and inheritance

The functional execution base is the repository's checked Cortex-M0 Armv6-M
cell. Cortex-M23 is upward-compatible with that bounded integer surface, so
ordinary instructions delegate to `cortex-m0-simulator` and must produce the
same registers, flags, memory, raw bytes, width, mnemonic, and lifecycle
faults. This cell does **not** claim every Armv8-M Baseline opcode. Later
extensions may add Armv8-M-exclusive instructions only with their own decode
and reserved-neighbor witnesses.

The M23-specific boundary adds:

- an explicit Security Extension configuration, with reset in Secure state
  when enabled and permanently Non-secure state when omitted;
- Secure and Non-secure banks for MSP, PSP, PRIMASK, and CONTROL;
- eight optional, 32-byte-granular SAU region descriptors;
- SAU enable and ALLNS controls, writable only by privileged Secure software;
- deterministic Secure, Non-secure, and NSC address attribution, including the
  architected overlap-to-Secure rule;
- checked Secure-to-Non-secure and NSC-gateway transition hooks for later
  exception/decode/SoC composition; and
- attribution checks for instruction fetch, every inherited scalar,
  stack, and multiple-register data access, and direct memory APIs.

The transition hooks model the architectural state boundary but are not
claimed to be encodings of BXNS, BLXNS, SG, exception entry, or exception
return. Those instruction and exception mechanisms remain future bounded
cells.

## 3. Complete state and reset

`CortexM23State` owns R0-R12, LR, PC, APSR, IPSR, Handler/Thread mode, the four
banked stack pointers, banked PRIMASK and CONTROL, current security state,
Security Extension configuration, SAU control and eight regions, halt/sleep/
event state, the complete 64 KiB little-endian teaching memory, and installed
image origin/length.

With the Security Extension enabled, reset enters privileged Secure Thread
mode using Secure MSP, disables the SAU, clears ALLNS and all regions, and thus
attributes the owned memory Secure. With the extension omitted, reset is
permanently Non-secure, SAU configuration is unavailable, and all owned memory
is Non-secure. All stack pointers start at the top of owned memory.

Restore validates memory size, PC and stack alignment, status reserved bits,
mode/IPSR agreement, installed-image range, extension/security consistency,
SAU control, region count, 32-byte granularity, inclusive region limits, and
owned-memory bounds before committing any state.

## 4. Security attribution and access rules

An enabled SAU region has a 32-byte-aligned base and an inclusive limit whose
low five bits are one. A single matching region returns Non-secure or NSC.
Two or more matching enabled regions return Secure. With no matching region,
SAU `ALLNS` selects Non-secure when set and Secure otherwise. A disabled SAU
uses the same ALLNS default. A processor without the Security Extension always
returns Non-secure.

Secure execution may fetch and access any attributed region. Non-secure normal
fetches and data accesses require a wholly Non-secure range. An NSC address is
accepted only by the explicit secure-gateway transition, not by a normal
Non-secure fetch or data access. Security checks happen before the inherited
core commits. Fetch attribution precedes inherited execution. The inherited
core then decodes, validates, and executes only against a private,
non-observable candidate so inherited decode and semantic faults keep their
architectural priority. Any described data access is attributed before the
candidate may commit; a rejected candidate is restored immediately. Thus an
attribution failure is typed and preserves complete architectural and private
validation state.

## 5. Checked lifecycle and traces

Program load is nonempty, halfword-aligned, in range, and resets the selected
configuration before installing bytes. Direct register and memory APIs are
checked. `step` checks fetch attribution, privately builds and validates an
inherited candidate, preflights that candidate's described data access, and
only then commits it. Rejected candidates are not observable or retained. All
failures are typed and atomic.

Every successful `StepTrace` contains exact raw bytes, width, mnemonic,
PC-before/after, security state, and complete M23 before/after snapshots.
`run` retains at most 64 complete traces under the same conservative 8 MiB
budget as the inherited core, rolls back the complete run on the first fault
or step-limit exhaustion, resynchronizes its private validation scratch, and
allocates nested snapshots fallibly. `execute` preflights the bound and
atomically combines load and run, including private-scratch rollback.

## 6. Completion gate

The bounded cell is complete only when it has:

1. exact differential witnesses for every inherited Cortex-M0 decode family;
2. Secure/Non-secure reset, bank switching, SAU default, one-region, disabled,
   overlap, NSC, privilege, and no-extension tests;
3. fetch and every inherited memory-addressing family checked against SAU
   attribution with typed atomic failures;
4. validated restore, program load, direct access, transition, halt/sleep,
   retention, run rollback, and execute rollback tests;
5. package README, changelog, BUILD, workspace membership, and backlog status;
6. release line coverage above 80%, targeting at least 90%; and
7. Rustfmt, strict Clippy, rustdoc, release tests, workspace/affected builds,
   lessons validation, and an independent security review before publication.

## 7. Deferred work

Armv8-M-exclusive opcodes, encoded SG/BXNS/BLXNS behavior, exception stacking
and return, interrupt targeting, stack-limit registers, MPU/IDAU composition,
NVIC/SysTick, AHB5 transactions, debug/trace, and implementation timing belong
to later functional, SoC, and gate-level cells. No proprietary Cortex-M23
netlist or transistor topology is inferred by this package.
