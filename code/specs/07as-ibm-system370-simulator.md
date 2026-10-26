# Layer 07as — IBM System/370 Functional Simulator

## Status

This specification defines the repository's checked, instruction-level IBM
System/370 CPU model. The first bounded cell preserves the compatible
System/360 problem-state integer, branch, character, and hexadecimal-floating
surface and adds the System/370 long-field and masked-character instructions
that materially distinguish ordinary application execution. A later
gate-level partner may implement this same contract.

The architectural source is IBM's *System/370 Principles of Operation*, form
GA22-7000. The model follows its big-endian byte order, 24-bit addressing,
instruction formats, register-pair rules, condition codes, and left-to-right
storage access. Dynamic address translation, control registers, interruption
delivery, channels, devices, storage protection, multiprocessing, privileged
control, decimal arithmetic, and additional floating-point facilities are
outside this first cell rather than approximated.

## Architectural state

`System370State` is a complete, owned snapshot containing:

- sixteen 32-bit general registers;
- four raw 64-bit hexadecimal-floating registers numbered 0, 2, 4, and 6;
- a 24-bit instruction address and two-bit condition code;
- every byte of the installed 1 MiB big-endian storage;
- halt state; and
- the exact installed program origin and byte length.

General register zero is an ordinary operand. In an address field, a zero base
or index number contributes no address component. Every generated address is
masked to 24 bits, but a range outside installed storage is a typed fault.

## Transport and checked lifecycle

Programs are raw System/370 instruction bytes. `load(program, origin)` rejects
an empty image, odd origin, or out-of-range image atomically. A successful
load resets the complete machine, copies the complete image, records its exact
range, and selects the origin as the instruction address.

Fetch begins on a halfword boundary and must remain inside the installed
program. The opcode selects a two-, four-, or six-byte instruction, all of
which must be installed. Storage outside the installed program remains usable
for data but never silently extends executable code.

`step`, `run`, `execute`, state restoration, and memory/register helpers are
checked. Every typed failure restores the exact prior architectural state.
`run` and `execute` are transactional as a whole and require a caller-provided
maximum step count. System/370 has no ordinary problem-state HALT instruction;
the repository uses `SVC 0` (`0A00`) as its deterministic halt sentinel, while
other SVC numbers are typed unsupported-supervisor-call faults.

## Compatible System/360 surface

The System/370 core preserves the complete functional contract from Layer
07ad for its modeled instructions:

- fixed-point load/store, signed and logical arithmetic, compare, multiply,
  divide, shifts, and register-range movement;
- logical, immediate, character, and storage-to-storage operations;
- register, indexed, count, and index branches; and
- raw short/long hexadecimal floating-point movement, sign control,
  arithmetic, compare, halve, multiply, and divide.

The public System/370 error and trace vocabulary is architecture-specific even
when execution delegates a compatible instruction to the checked System/360
core. No System/360 state or error type leaks through the public API.

## System/370 extension instructions

### Move Long — `MVCL` (`0E`, RR)

R1 and R2 must be even register numbers. R1/R1+1 identify the destination
address and length; R2/R2+1 identify the source address and length, with the
high byte of R2+1 supplying the pad byte. Addresses are the low 24 bits and
lengths are the low 24 bits of their registers.

Bytes move left-to-right until the destination length is exhausted. Source
bytes are used while available, followed by the pad byte. The address and
length registers advance after every byte; updated address-register high
bytes are zero while the ignored high bytes of both length registers remain
unchanged. Condition code is 0 for equal
initial lengths, 1 when destination is shorter, and 2 when destination is
longer. If a destination byte would destructively overlap unread source in the
architecturally forbidden direction, condition code 3 is produced without
moving data or changing either register pair. Odd register pairs and invalid
storage ranges are typed atomic faults.

### Compare Logical Long — `CLCL` (`0F`, RR)

The two even register pairs identify variable-length byte strings. Missing
bytes in either operand compare as the single pad byte from the high byte of
R2+1. Comparison proceeds left-to-right
until unequal bytes are found or both fields are exhausted. Address and length
registers describe the unexamined suffix; updated address-register high bytes
are zero while the ignored high bytes of both length registers remain
unchanged. Condition code is 0 for equality, 1
when operand one is low, and 2 when operand one is high. Odd pairs or invalid
storage are typed atomic faults.

### Compare Logical Characters under Mask — `CLM` (`BD`, RS)

The four mask bits select general-register bytes from most significant to
least significant. Selected bytes are compared, in order, with consecutive
storage bytes. Unselected register bytes consume no storage. Condition code is
0 when all selected bytes are equal, 1 when the first unequal register byte is
low, and 2 when it is high. A zero mask compares no data and sets condition
code 0, but the effective address still designates a one-byte storage operand
for checked address validation, as required by the architecture.

### Store Characters under Mask — `STCM` (`BE`, RS)

The mask selects register bytes from most significant to least significant.
Selected bytes are stored consecutively from the effective address;
unselected bytes consume no storage. With a zero mask no storage operand is
accessed, so protection and addressing exceptions are not recognized.
Condition code is unchanged.

### Insert Characters under Mask — `ICM` (`BF`, RS)

Selected register bytes, from most significant to least significant, receive
consecutive storage bytes; unselected register bytes remain unchanged. A zero
mask inserts no data and sets condition code 0, but still validates the
architecturally designated one-byte storage operand. Otherwise condition code
is 0 when every inserted bit is zero, 1 when the leftmost inserted bit is one,
and 2 when that bit is zero but some inserted bit is one. Condition code 3 is
not produced.

## Traces and errors

Every successful step reports the address before and after execution, exact
raw bytes, width, conventional uppercase mnemonic, and a concise description.
The `SVC 0` trace marks the machine halted; a later step returns `Halted`
without mutation.

Errors distinguish empty, oversized, and misaligned programs; fetch outside
the installed image; truncated instructions; unknown opcodes; unsupported
supervisor calls; misaligned and out-of-range storage; invalid registers or
state; specification faults including odd long-operation register pairs;
fixed-point and floating-point overflow/divide faults; halt; and exhausted
step limits. Every error has stable, nonempty display text.

## Acceptance criteria

- This specification is committed before tests and implementation.
- Tests cover inherited format/family representatives and every new
  instruction, all condition-code outcomes, mask patterns, padding, register
  updates, overlap, pair validation, storage boundaries, transport, complete
  state restoration, bounded execution, and atomic faults.
- Differential tests prove compatible instructions match the checked
  System/360 core from equivalent state.
- Line coverage exceeds 95% where practical and never falls below 80%.
- Formatting, strict Clippy, rustdoc, package BUILD, release tests, lessons
  validation, and the repository workspace build are attempted before review.
