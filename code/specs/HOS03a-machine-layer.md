# HOS03a — Deterministic CPU, bus, and board probe

## Scope and fidelity

This probe extends the OS-neutral semantic engine with separate machine-layer
components: a CPU instruction-stepper, an address bus, memory, a mapped device,
and two board configurations. It demonstrates component boundaries; it does
not claim to emulate a historical ISA, commercial processor, chipset, or real
board. The toy instruction set exists only to exercise deterministic fetch,
data access, device writes, and halt.

The HOS03 semantic profiles continue to model policy directly from abstract
stimuli. This machine probe executes a small declared bytecode sequence. It is
therefore instruction-level within the tiny toy ISA, but is not cycle-accurate
and does not simulate pipelining, interrupts, DMA, cache behavior, or silicon
timing. It is not required by, and does not change, the HOS00 Unix model or VOS.

## Bus contract

The probe uses a byte-addressed 16-bit bus. An address is an integer in
`[0, 65536)`; address arithmetic that leaves that range fails with `EADDRESS`
and never wraps. Each mapped region is a non-empty half-open interval
`[start, end)` with `0 <= start < end <= 65536`. Region registration rejects
any overlap, regardless of permissions or target type. For each access, the
bus resolves exactly one region or reports `EUNMAPPED`; it checks the requested
read, write, or execute permission before invoking the target. Denied or
unmapped accesses do not mutate target state. The bus does not guess aliases,
mirroring, endianness, or memory-mapped device behavior.

An access rejected by the bus before dispatch (for example, an unmapped,
out-of-range, or permission-denied access) leaves its target untouched. A
custom `BusTarget` has a separate implementation obligation: it must validate
all target-specific failure conditions before mutating state. If its `read` or
`write` operation throws, that operation must leave target state and emitted
effects unchanged. The bus cannot snapshot and roll back arbitrary device
state; atomic failure for custom targets is therefore an interface contract,
not a generic rollback feature.

Regions declare independent read, write, and execute permissions. CPU
instruction fetch uses execute permission; operand loads use read permission;
stores use write permission. The bus returns typed errors rather than
substituting open-bus values. These rules make ambiguity and protection
failures visible to fixture runners.

## Components and probe guest

- `Bus` owns the address width, non-overlapping regions, range checks, and
  permission enforcement. Targets implement byte reads or writes and return
  data-only effects.
- `Ram` is byte-addressable mutable memory with a declared range and
  permissions. Loading the test program is a board setup operation before
  execution, not a guest write.
- `ConsoleDevice` accepts byte writes and records them in simulated device
  state. It never writes to the host console. Effects describe the addressed
  byte and resulting text.
- `Cpu` stores a program counter, accumulator, address register, and halted
  flag. One step executes at most one instruction and records its instruction
  index. A failed bus access or invalid opcode leaves CPU state and device
  effects unchanged.
- `Board` composes a bus, configuration ROM, RAM, and console mapping. Board
  identity is configuration data; it does not change CPU instruction rules.

The synthetic `ascii-console-demo-v1` program reads a 16-bit console address
from little-endian bytes at `0x0000`, loads each byte of `OK\n` into the
accumulator, stores through the address register, then halts. The toy ISA is:

| Opcode | Bytes | Operation |
|---|---:|---|
| `0x10` | 3 | `LDX_ABS`: load the address register from a little-endian word at an absolute address |
| `0x20` | 2 | `LDA_IMM`: load one immediate byte into the accumulator |
| `0x30` | 1 | `STAX`: store the accumulator byte at the address register |
| `0xFF` | 1 | `HALT` |

Both boards expose read-only configuration bytes at `[0x0000, 0x0002)` and RAM
at `[0x0100, end)` with read/write/execute permission. The first board has
16 KiB of RAM ending at `0x4100` and a console at `0xFF00`; the second has
8 KiB of RAM ending at `0x2100` and a console at `0xE000`. The program image
and CPU implementation are identical. The configuration ROM selects the
different device address.

## Determinism and evidence

The runner steps to halt or until a caller-supplied nonnegative instruction
budget is consumed. Each instruction step is one logical CPU step; no wall
clock is consulted. Board snapshots and traces contain JSON data only. Replaying
the same board configuration, guest bytes, and sequence of budgets yields the
same CPU state, device contents, and ordered effects. Tests cover both boards,
console effects, instruction budget boundaries, replay, address overflow,
unmapped access, permission denial, and overlap rejection.

Future real ISAs, bus widths, interrupt controllers, timers, and device models
must define their own contracts and portable fixtures. They can reuse these
interfaces only when the behavior actually matches; this toy CPU is not
evidence of compatibility with a repository CPU simulator.
