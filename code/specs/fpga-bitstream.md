# FPGA Bitstream (iCE40)

## Overview

A real iCE40 bitstream — the binary file (`.bin`) that programs an iCE40-HX1K (or iCE40-UP5K) FPGA — is a structured sequence of configuration commands and tile-by-tile CRAM (configuration RAM) data. This spec defines a writer that takes our `fpga` package's JSON config (per `fpga-place-route-bridge.md`), or directly takes an HNL tech-mapped to LUT/FF cells, and emits an iCE40 `.bin`.

The format is reverse-engineered and documented by **Project IceStorm** (Clifford Wolf et al., 2015). We follow IceStorm's conventions; our bitstreams round-trip through `icepack`/`iceprog` and program real boards.

Why have this when `real-fpga-export.md` already gets us to a real iCE40 via yosys/nextpnr/icepack? Two reasons:
1. **Pedagogy** — understanding *what's in* a bitstream is essential to demystifying FPGAs. `real-fpga-export` treats the toolchain as a black box; this spec opens it.
2. **Owned path** — when we want to fix bugs or add unusual targets, we own the bitstream emitter.

## Layer Position

```
HNL or fpga-package JSON config
              │
              ▼
fpga-bitstream.md  ◀── THIS SPEC
              │
              ▼
.bin file
              │
              ▼
iceprog → real iCE40 board
```

## iCE40 Architecture (relevant subset)

The iCE40-HX1K we target (the part on the iCE40-HX1K-EVN dev board):
- 1280 logic cells (LCs); each LC has a 4-input LUT + 1 D flip-flop + carry-chain.
- Logic cells grouped into Logic Tiles (LT), each with 8 LCs.
- Other tile types: I/O Tile (IO), RAM Tile (BRAM), DSP, PLL.
- A grid of tiles: 33 × 17 for HX1K (varies by part).
- Programmable interconnect via switch boxes.

CRAM (configuration RAM) is organized as a per-tile bitmap. Each tile's CRAM controls:
- The LUT truth tables (16 bits per LUT × 8 LCs = 128 bits per Logic Tile).
- FF configuration (set/reset, enable, clock polarity).
- Local/global clock routing.
- Switchbox wires (which segments connect to which).
- IO direction and configuration.

## Bitstream Format

The iCE40 bitstream is a series of variable-length records:

```
0xff 0x00          # bitstream marker
[preamble records]
0x01 [size]        # CRAM bank sizing
0x02 [data]        # bitfields
[per-tile records]
0xff 0xff          # end marker
```

Each record is `<command_byte> <length> <payload>`. Commands include:
- `0x01`: Set CRAM size (height/width/banks).
- `0x05`: Set CRAM bank.
- `0x06`: Set CRAM offset.
- `0x07`: Reset CRAM offset.
- `0x08`: BRAM data.
- `0x25`: Set CRAM bit.
- `0x26`: Set BRAM bit.
- `0x80`: CRC.
- `0xff 0xff`: End.

The CRAM image itself is a 2-D bitmap (rows × columns); each tile occupies a fixed rectangular region. Setting bit `(row, col)` to 1 enables some configuration option (LUT bit, switch connection, etc.). The exact `(row, col) → meaning` mapping is part of IceStorm's reverse-engineered chip database.

### Chip database

IceStorm publishes a chip-DB file (`chipdb-1k.txt`, `chipdb-5k.txt`, etc.) that maps:
- `(tile_x, tile_y) → tile_kind`.
- For each tile, a list of `(option, [bit_positions])`.
- Switch matrix: `(net_a, net_b) → bit_positions to connect`.

Our writer:
1. Loads the chipdb for the target part.
2. For each LUT in the design: locates the target tile; sets its truth-table bits.
3. For each FF: sets configuration bits.
4. For each net's routed path: finds the switches and sets their bits.
5. For each IO: sets direction + pad config.
6. Emits the resulting CRAM image as a sequence of records.

## Public API

```python
from dataclasses import dataclass
from pathlib import Path
from enum import Enum


class Iice40Part(Enum):
    HX1K = "hx1k"
    HX8K = "hx8k"
    UP5K = "up5k"
    LP1K = "lp1k"


@dataclass
class ChipDb:
    """Loaded from IceStorm's chipdb-*.txt"""
    part: Iice40Part
    grid_x: int
    grid_y: int
    tiles: dict[tuple[int, int], "TileInfo"]
    pinmap: dict[str, tuple[int, int, int]]     # pad name → (tile_x, tile_y, io_index)


@dataclass
class TileInfo:
    kind: str                        # "logic" | "io" | "ramt" | "ramb" | "dsp" | ...
    options: dict[str, list[tuple[int, int]]]   # option name → [(row, col), ...]
    switches: dict[tuple[str, str], list[tuple[int, int]]]  # (src, dst) → bits


@dataclass
class BitstreamWriter:
    chipdb: ChipDb
    cram: dict[tuple[int, int], list[list[bool]]]   # per-tile bitmap
    bram: dict[tuple[int, int], list[int]]
    
    @classmethod
    def for_part(cls, part: Iice40Part, chipdb_path: Path) -> "BitstreamWriter":
        ...
    
    def configure_lut(self, tile: tuple[int, int], lc_index: int,
                      truth_table: list[int]) -> None: ...
    def configure_ff(self, tile: tuple[int, int], lc_index: int,
                     enabled: bool = False, init: int = 0) -> None: ...
    def configure_switch(self, tile: tuple[int, int],
                         src: str, dst: str) -> None: ...
    def configure_io(self, pad: str, direction: str) -> None: ...
    
    def emit(self, path: Path) -> None: ...


def from_fpga_json(json_path: Path, part: Iice40Part) -> BitstreamWriter:
    """Convert a fpga-package JSON config into a bitstream writer state."""
    ...
```

## Worked Example — 4-bit Adder bitstream

```python
from fpga_bitstream import BitstreamWriter, from_fpga_json, Iice40Part

# 4-bit adder mapped via fpga-place-route-bridge to F01 JSON
writer = from_fpga_json(Path("adder4.json"), part=Iice40Part.HX1K)

# (writer now has CRAM bits set per the placed/routed design)

writer.emit(Path("adder4.bin"))
# adder4.bin is ~135 KB (HX1K full bitstream size; mostly zero bits)
# iceprog adder4.bin → flashes to real iCE40-HX1K-EVN
```

Round-trip vs IceStorm: emit our bitstream; run `icebox_explain adder4.bin`; compare to `icebox_explain adder4_via_yosys.bin`. Differences are bugs in our writer.

## Edge Cases

| Scenario | Handling |
|---|---|
| Design too large for chosen part | Detect during chip-db lookup; suggest larger part. |
| Switch matrix path requires a switch we don't have config for | Fall back to longer route or error. |
| BRAM contents | Set per-bit via `configure_bram`. |
| PLL / DSP configuration | Out of scope for v1; document as future. |
| Hot-loading another bitstream while running | Out of scope (board-side concern). |
| Multi-die or 3D iCE40 | N/A. |
| iCE40 errata / silicon bugs | Document workarounds; apply at writer level. |

## Test Strategy

### Unit (95%+)
- ChipDb loader: parses an IceStorm chipdb without errors.
- Per-tile bit-setting: setting an option's bits flips the right CRAM positions.
- Bitstream record emission: each record has correct length and payload.

### Integration
- Emit a 4-bit-adder bitstream; `iceprog -t` (test connection only) succeeds.
- Cross-check vs yosys/nextpnr-emitted bitstream for the same design — bit-for-bit close (some differences are expected from placement variation; the design must be functionally equivalent).
- (If hardware) flash to a real board; verify behavior with hardware test vectors.
- Round-trip: emit → `icebox_explain` → re-emit; same.

## Conformance

| Reference | Coverage |
|---|---|
| **Project IceStorm** chip database | Full (use their files) |
| **iCE40-HX1K** | Full LUT/FF/switch/IO support; PLL/DSP basic |
| **iCE40-UP5K** | Full (different chipdb) |
| **iCE40-HX8K** | Full |
| **iCE40-LP variants** | Full |
| **iCE40-LM** (mobile) | Out of scope |
| **ECP5** | Out of scope; future spec via Project Trellis |
| **Xilinx 7-series** | Out of scope; future via Project X-Ray |

## Open Questions

1. **PLL / DSP / SerDes configuration** — defer; most designs don't need.
2. **Bitstream encryption / authentication** — iCE40 supports neither; out of scope.
3. **Partial reconfiguration** — iCE40 doesn't support; for ECP5 in future.

## Future Work

- ECP5 bitstream via Project Trellis.
- Xilinx 7-series via Project X-Ray.
- PLL / DSP / SerDes configuration on iCE40.
- Bitstream-level optimization (eliminate unused tile config).
- Bitstream verification (read back from board; compare).
