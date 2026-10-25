# Spec 07af — Intel 8085 Functional Simulator

## Purpose

The `intel8085-simulator` package models Intel's 1976 8085A as a checked,
deterministic functional machine. The 8085 retained the documented 8080
instruction encodings and added two architecturally visible instructions:
`RIM` (`0x20`) and `SIM` (`0x30`). Those instructions expose the 8085's
vectored-interrupt masks and its one-bit serial input/output pins, so an 8085
simulator is not just an 8080 simulator with a different name.

The package reuses the completed `intel8080-simulator` execution core for the
shared 244 documented opcodes. It owns the 8085-only interrupt controller,
serial latches, delayed-enable rule, loaded-program boundary, checked lifecycle,
and complete state snapshots.

Primary references:

- Intel, *MCS-80/85 Family User's Manual*, October 1979, chapters 4 and 5.
- Intel, *8080/8085 Assembly Language Programming*, 1977.
- Intel, *8085A/8085A-2 8-Bit Microprocessor* data sheet, 1978.

## Architectural state

The complete state contains:

- 8080-compatible A, B, C, D, E, H, L, SP, PC, S/Z/AC/P/CY flags;
- the full configured byte-addressable memory and 256 input/output ports;
- halt and global interrupt-enable state;
- installed-program origin and length;
- RST 5.5, 6.5, and 7.5 mask bits;
- pending TRAP, RST 7.5, RST 6.5, RST 5.5, and INTR requests;
- the delayed-EI latch;
- SID input and SOD output.

Reset clears registers, flags, memory, ports, pending requests, serial output,
and the installed-program range. RESET IN masks all three restart interrupts
and disables the global interrupt-enable flip-flop.

## Documented opcode surface

All 244 documented 8080 encodings retain their byte widths and behavior. The
8085 defines these two formerly unused bytes:

| Opcode | Mnemonic | Behavior |
|---|---|---|
| `0x20` | RIM | Load A with SID, pending restart requests, IE, and mask bits |
| `0x30` | SIM | Optionally set restart masks, clear RST 7.5, and update SOD |

The remaining ten undocumented bytes (`08`, `10`, `18`, `28`, `38`, `CB`,
`D9`, `DD`, `ED`, and `FD`) are rejected as typed unknown-opcode faults. They
have behavior on some physical steppings, but Intel did not include them in the
documented 8085 programming contract. A future compatibility mode may expose
them without weakening the default checked contract.

### RIM accumulator layout

```text
bit:  7    6      5      4     3    2      1      0
      SID  I7.5   I6.5   I5.5  IE   M7.5   M6.5   M5.5
```

Pending bits report requests even while the corresponding source is masked.
RIM does not alter flags or interrupt state.

### SIM accumulator layout

```text
bit:  7    6    5  4      3    2      1      0
      SOD  SOE  X  R7.5   MSE  M7.5   M6.5   M5.5
```

- MSE=1 copies bits 2..0 into the three mask latches; MSE=0 preserves them.
- R7.5=1 clears the edge-triggered RST 7.5 pending latch.
- SOE=1 copies bit 7 into SOD; SOE=0 preserves SOD.
- Bit 5 is ignored.

## Interrupt protocol

Interrupt requests are explicit host events. At an instruction boundary the
simulator accepts the highest-priority serviceable request:

1. TRAP, non-maskable, vector `0x0024`;
2. RST 7.5, edge-latched, vector `0x003C`;
3. RST 6.5, vector `0x0034`;
4. RST 5.5, vector `0x002C`;
5. INTR, whose acknowledge byte must be one of the eight `RST n` opcodes.

Acceptance pushes the current PC using the normal 8085 stack byte order,
clears halt, disables maskable interrupts, clears delayed EI, and transfers to
the vector. An accepted request is represented by a trace with an empty raw
byte sequence and an interrupt mnemonic. Stack bounds are checked before any
state changes.

`EI` becomes effective only after the instruction following EI completes.
`DI` disables interrupts immediately and cancels a pending delayed enable.
TRAP may release HLT regardless of IE; a serviceable maskable request may
release HLT when IE and its mask permit it.

## Checked lifecycle

- `load_program_at` installs bytes at any 16-bit origin and records the exact
  executable range. Invalid or wrapping images fail atomically.
- `step` rejects fetches outside the installed image, truncated instructions,
  undocumented opcodes, out-of-range data/stack accesses, and post-halt steps
  with typed errors and no partial mutation.
- `run` executes on a fresh candidate machine and commits only on success.
- retained execution traces own two complete snapshots apiece, so
  `max_retained_trace_steps` derives a conservative limit from configured
  memory size and a 16 MiB trace budget. `run_loaded` uses that limit, while
  oversized explicit limits fail before execution. Callers that need longer
  execution stream one trace at a time with `step` instead of retaining every
  full-memory snapshot.
- configurations so large that one complete trace exceeds the budget report a
  maximum of zero retained steps; they remain usable through streaming `step`.
- every successful instruction or interrupt acknowledgement returns owned
  before/after state plus exact raw bytes and mnemonic.
- snapshots own all memory and port bytes and therefore remain stable after
  later machine changes.

## Package contract

The Rust package must include:

- a workspace member and path dependency on `intel8080-simulator`;
- `BUILD`, `README.md`, and `CHANGELOG.md`;
- exhaustive first-byte classification tests for all 256 encodings;
- focused witnesses for RIM, SIM, EI delay, interrupt priority, HLT release,
  stack byte order, transaction rollback, reset, and owned snapshots;
- strict formatting, Clippy, rustdoc, release tests, and at least 95% line
  coverage.
