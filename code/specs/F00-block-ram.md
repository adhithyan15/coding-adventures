# F00 — Block RAM

## Overview

Block RAM provides hardware-level read/write memory arrays built from logic gates. Unlike the existing ROM package (which is read-only) and the memory-controller package (which manages address spaces in software), this package models actual physical storage circuits — the kind etched into silicon in every CPU cache, FPGA, and SRAM chip.

The fundamental storage element is the **SRAM cell**: six transistors (modeled as gates) that hold one bit indefinitely as long as power is supplied. From this single cell, we build arrays, address decoders, and complete RAM modules with single-port and dual-port interfaces.

This package is a prerequisite for the FPGA abstraction (where Block RAM tiles are a key resource) and is also useful for CPU cache simulation, register file modeling, and any design that needs fast, hardware-accurate writable storage.

## Layer Position

```
Logic Gates → [YOU ARE HERE] → FPGA (as Block RAM tiles)
                             → CPU Cache (as SRAM arrays)
                             → Register Files (as small, fast arrays)
```

**Input from:** Logic gates (AND, OR, NOT, NOR — for cell construction), combinational circuits (decoder, MUX — for address decoding and data routing).
**Output to:** FPGA package (provides configurable BRAM tiles), CPU internals (cache data/tag arrays).

## Concepts

### How does a circuit "remember"?

Memory requires **feedback** — wiring a gate's output back into its own input. The logic-gates package already demonstrates this with the SR latch (two cross-coupled NOR gates). Block RAM scales this idea from one bit to millions.

The hierarchy:

```
SRAM Cell (1 bit)     → the atom of memory
    │
SRAM Array (N×M)      → grid of cells with row/column addressing
    │
RAM Module             → array + address decoder + read/write logic + I/O buffers
    │
Dual-Port RAM          → two independent ports (simultaneous read/write)
    │
Configurable BRAM      → width/depth reconfiguration (FPGA-style)
```

### The SRAM Cell — 6 Transistors That Hold One Bit

In real hardware, an SRAM (Static Random-Access Memory) cell uses 6 transistors:
- 2 cross-coupled inverters forming a bistable latch (holds the bit)
- 2 access transistors controlled by the word line (gates read/write access)

```
                     Bit Line (BL)      Bit Line Bar (BL_bar)
                         │                      │
                    ┌────┴────┐            ┌────┴────┐
                    │ Access  │            │ Access  │
  Word Line ────────┤ Transistor          │ Transistor
                    │  (N1)   │            │  (N2)   │
                    └────┬────┘            └────┬────┘
                         │                      │
                    ┌────┴────┐            ┌────┴────┐
                    │         ├────────────┤         │
                    │  INV1   │            │  INV2   │
                    │         ├────────────┤         │
                    └─────────┘            └─────────┘
                     (NOT gate)            (NOT gate)
                       Q                     Q_bar
```

How it works:
- **Hold**: Word line = 0. Access transistors are off. The two inverters form a feedback loop — each one's output drives the other's input. The cell holds its state indefinitely. This is identical to the SR latch concept from logic-gates.
- **Read**: Word line = 1. Access transistors turn on, connecting the internal nodes to the bit lines. The sense amplifier detects the voltage difference between BL and BL_bar to determine the stored value.
- **Write**: Word line = 1, and the write driver forces BL and BL_bar to the desired values. The external drive is stronger than the internal inverters, overriding the stored state.

In our simulation, we model this at the gate level: the cross-coupled inverters use NOT gates, and the access transistors use AND gates (AND(data, enable) models a transistor passing data when enabled).

### Why SRAM and Not DRAM?

| Property | SRAM | DRAM |
|----------|------|------|
| Transistors per bit | 6 | 1 + capacitor |
| Speed | Fast (sub-nanosecond) | Slower (tens of ns) |
| Refresh needed | No | Yes (capacitor leaks) |
| Cost per bit | High | Low |
| Used in | Cache, registers, FPGA BRAM | Main memory (DDR4/5) |

We model SRAM because it's what FPGAs and CPU caches use. DRAM would require modeling capacitor charge decay and refresh cycles, which is a separate concern.

### SRAM Array — From One Cell to a Grid

A RAM chip is a 2D grid of SRAM cells. To access a specific cell:

1. **Row decoder** — converts the row address bits into a one-hot word line signal. Only one row is active at a time.
2. **Column MUX** — selects which columns are read/written when the array is wider than the data bus.

```
                     Column 0   Column 1   Column 2   Column 3
                        │          │          │          │
  Row 0 (WL0) ─────── [Cell]     [Cell]     [Cell]     [Cell]
                        │          │          │          │
  Row 1 (WL1) ─────── [Cell]     [Cell]     [Cell]     [Cell]
                        │          │          │          │
  Row 2 (WL2) ─────── [Cell]     [Cell]     [Cell]     [Cell]
                        │          │          │          │
  Row 3 (WL3) ─────── [Cell]     [Cell]     [Cell]     [Cell]
                        │          │          │          │
                     Bit Line 0 Bit Line 1 Bit Line 2 Bit Line 3
                        │          │          │          │
                     ┌──┴──────────┴──────────┴──────────┴──┐
                     │           Column MUX / Sense Amps     │
                     └──────────────────┬───────────────────┘
                                        │
                                    Data Out
```

Address structure for a 4-row × 4-column array:
- Address bits [1:0] → row decoder (selects word line)
- No column MUX needed if all 4 columns are read simultaneously (4-bit wide output)
- If we want a 1-bit output, address bits [3:2] → column MUX

### Single-Port RAM

The simplest complete RAM module. One address port, one data bus. Each cycle you can do ONE operation: read OR write.

```
                ┌──────────────────────────┐
  Address ──────┤                          │
                │     Single-Port RAM      │
  Data In ──────┤                          ├──── Data Out
                │     (rows × width)       │
  Write En ─────┤                          │
                │                          │
  Clock ────────┤                          │
                └──────────────────────────┘
```

Interface:
- **Address** (N bits): selects which row to access
- **Data In** (M bits): data to write
- **Write Enable** (1 bit): 0 = read, 1 = write
- **Clock** (1 bit): operations happen on rising edge
- **Data Out** (M bits): data read from the selected address

Behavior:
- **Read** (WE=0): Data Out = contents of row at Address
- **Write** (WE=1): row at Address = Data In; Data Out = undefined (or previous value)

### Dual-Port RAM

Two completely independent ports (A and B), each with its own address, data, and write enable. Both ports can operate simultaneously — you can read from port A and write via port B in the same cycle, even at different addresses.

```
  ┌────────────────────────────────────────────┐
  │               Dual-Port RAM                │
  │                                            │
  │  Port A                      Port B        │
  │  ┌──────┐                   ┌──────┐       │
  │  │Addr A│                   │Addr B│       │
  │  │Din  A│   (shared array)  │Din  B│       │
  │  │WE   A│                   │WE   B│       │
  │  │Clk  A│                   │Clk  B│       │
  │  │Dout A│                   │Dout B│       │
  │  └──────┘                   └──────┘       │
  └────────────────────────────────────────────┘
```

**Why dual-port matters for FPGAs:**
FPGAs use dual-port BRAM everywhere. A common pattern: one port writes data from
a producer (e.g., ADC samples), the other port reads data for a consumer (e.g.,
FFT engine). The two can operate at different clock rates (asynchronous dual-port).

**Write collision:** if both ports write to the same address simultaneously, the
result is undefined. Our simulation detects this and raises an error.

### Configurable Width and Depth

FPGA Block RAMs can be configured with different aspect ratios from the same
physical storage. An 18 Kbit BRAM can be configured as:

```
Configuration    │ Depth    Width    Total bits
─────────────────┼──────────────────────────────
  16K × 1        │  16384      1       16384
   8K × 2        │   8192      2       16384
   4K × 4        │   4096      4       16384
   2K × 8 (byte) │   2048      8       16384
   1K × 16       │   1024     16       16384
  512 × 32 (word)│    512     32       16384
```

The total storage is fixed; you trade depth for width. This is achieved by
changing how the address decoder and column MUX are configured — the underlying
SRAM cells don't change at all.

### Read Modes

Real BRAMs support two read modes that affect what Data Out shows during a write:

1. **Read-first**: Data Out shows the OLD value at the address being written (read happens before write in the same cycle)
2. **Write-first** (or "read-after-write"): Data Out shows the NEW value being written
3. **No-change**: Data Out retains its previous value during writes (saves power)

Our simulation supports all three modes via a configuration parameter.

## Public API

```python
# === Low-Level: SRAM Cell ===

class SRAMCell:
    """Single-bit storage element modeled at the gate level."""

    def __init__(self) -> None: ...

    def read(self, word_line: int) -> int | None: ...
        # word_line=1: return stored bit. word_line=0: return None (not selected)

    def write(self, word_line: int, bit_line: int) -> None: ...
        # word_line=1: store bit_line value. word_line=0: no effect

    @property
    def value(self) -> int: ...
        # Current stored value (for inspection/debugging)


# === Mid-Level: SRAM Array ===

class SRAMArray:
    """2D grid of SRAM cells with row/column addressing."""

    def __init__(self, rows: int, cols: int) -> None: ...
        # rows × cols grid. rows must be power of 2.

    def read(self, row: int) -> list[int]: ...
        # Read all columns of the given row

    def write(self, row: int, data: list[int]) -> None: ...
        # Write data to the given row (len(data) must equal cols)

    @property
    def shape(self) -> tuple[int, int]: ...
        # (rows, cols)


# === High-Level: RAM Modules ===

class ReadMode(Enum):
    READ_FIRST = "read_first"
    WRITE_FIRST = "write_first"
    NO_CHANGE = "no_change"

class SinglePortRAM:
    """Single-port synchronous RAM."""

    def __init__(
        self,
        depth: int,            # Number of addressable words
        width: int,            # Bits per word
        read_mode: ReadMode = ReadMode.READ_FIRST,
    ) -> None: ...

    def tick(
        self,
        clock: int,
        address: list[int],    # Address bits (LSB first)
        data_in: list[int],    # Data to write (width bits, LSB first)
        write_enable: int,     # 0 = read, 1 = write
    ) -> list[int]: ...
        # Returns data_out (width bits). Operation happens on rising edge.

    @property
    def depth(self) -> int: ...
    @property
    def width(self) -> int: ...

    def dump(self) -> list[list[int]]: ...
        # Return all contents for inspection


class DualPortRAM:
    """True dual-port synchronous RAM."""

    def __init__(
        self,
        depth: int,
        width: int,
        read_mode_a: ReadMode = ReadMode.READ_FIRST,
        read_mode_b: ReadMode = ReadMode.READ_FIRST,
    ) -> None: ...

    def tick(
        self,
        clock: int,
        # Port A
        address_a: list[int],
        data_in_a: list[int],
        write_enable_a: int,
        # Port B
        address_b: list[int],
        data_in_b: list[int],
        write_enable_b: int,
    ) -> tuple[list[int], list[int]]: ...
        # Returns (data_out_a, data_out_b)
        # Raises WriteCollisionError if both ports write to same address

    @property
    def depth(self) -> int: ...
    @property
    def width(self) -> int: ...


class WriteCollisionError(Exception):
    """Raised when both ports write to the same address simultaneously."""
    address: int


# === FPGA-Style Configurable BRAM ===

class ConfigurableBRAM:
    """Block RAM with configurable aspect ratio.

    Total storage is fixed at initialization. Width and depth can be
    reconfigured as long as width × depth = total_bits.
    """

    def __init__(
        self,
        total_bits: int = 18432,  # 18 Kbit (Xilinx BRAM18 size)
        width: int = 8,
        dual_port: bool = True,
    ) -> None: ...

    def reconfigure(self, width: int) -> None: ...
        # Change aspect ratio. depth = total_bits // width.
        # Clears all stored data.

    # Port A and Port B interfaces (same as DualPortRAM when dual_port=True)
    def tick_a(self, clock: int, address: list[int], data_in: list[int], write_enable: int) -> list[int]: ...
    def tick_b(self, clock: int, address: list[int], data_in: list[int], write_enable: int) -> list[int]: ...

    @property
    def depth(self) -> int: ...
    @property
    def width(self) -> int: ...
    @property
    def total_bits(self) -> int: ...
```

## Data Flow

```
SRAM Cell:
  Input:  word_line (0 or 1), bit_line (0 or 1 for write)
  Output: stored bit (0 or 1) or None (not selected)

SRAM Array:
  Input:  row index (int), data (list of bits for write)
  Output: list of bits (one per column)

RAM Modules:
  Input:  clock, address (list of bits), data_in (list of bits), write_enable
  Output: data_out (list of bits)
```

All bit lists are LSB-first (least significant bit at index 0), consistent with
the arithmetic package convention.

## Test Strategy

### SRAM Cell
- Write 0, read back → 0
- Write 1, read back → 1
- Read with word_line=0 → None (not selected)
- Write with word_line=0 → no change to stored value
- Verify initial state is defined (0)
- Overwrite: write 1, verify, write 0, verify

### SRAM Array
- Write a row, read it back → same data
- Write different data to different rows, read each → correct data per row
- Read a row that was never written → all zeros (initial state)
- Write partial row → error (data length must match cols)
- Invalid row index → error

### SinglePortRAM
- Write address 0, read address 0 → written data
- Write multiple addresses, read each → correct data
- Read-first mode: during write, data_out = old value
- Write-first mode: during write, data_out = new value
- No-change mode: during write, data_out = previous read value
- Operations only happen on rising edge (clock=0→1)
- Address width matches depth (e.g., depth=1024 → 10 address bits)

### DualPortRAM
- Port A writes, Port B reads same address → correct data
- Port A reads, Port B writes different address → independent
- Simultaneous reads from both ports at different addresses → both correct
- Write collision: both ports write same address → WriteCollisionError
- Both ports read same address simultaneously → both get same data

### ConfigurableBRAM
- Default configuration: 18Kbit, 8-bit wide, 2304 deep
- Reconfigure to 16-bit wide → depth halves, data clears
- Reconfigure to 32-bit wide → depth quarters
- Invalid reconfiguration (width doesn't divide total_bits) → error
- Dual-port operations work after reconfiguration

### Edge Cases
- Depth = 1 (single-word RAM)
- Width = 1 (single-bit RAM)
- Maximum address (last row)
- All-zeros data, all-ones data

## Future Extensions

- **DRAM simulation**: Model capacitor-based storage with refresh cycles
- **CAM (Content-Addressable Memory)**: Search by content instead of address (used in CPU TLBs)
- **FIFO**: First-in-first-out buffer built from dual-port RAM + read/write pointers
- **Asynchronous dual-port**: Ports running at different clock frequencies
- **ECC (Error-Correcting Code)**: Detect and correct single-bit errors using Hamming codes
- **Memory initialization from file**: Load RAM contents from a hex/binary file at construction
