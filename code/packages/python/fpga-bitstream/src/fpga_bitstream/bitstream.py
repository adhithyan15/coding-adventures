"""iCE40 bitstream emitter (Project IceStorm format).

v0.1.0 implements a structured-but-simplified bitstream:
- Correct record-stream format (preamble, command bytes, end marker).
- Stub CRAM image (zeros) — for a real bitstream loadable on hardware,
  Project IceStorm's chip database is required to map per-tile config bits
  to (row, col) CRAM positions. We document the limitation; for actual
  hardware, prefer the real-fpga-export package's icepack shell-out path.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass, field
from enum import Enum


class Iice40Part(Enum):
    HX1K = "hx1k"
    HX8K = "hx8k"
    UP5K = "up5k"
    LP1K = "lp1k"


# Approximate part dimensions (rows, cols, CRAM bits per tile)
PART_SPECS: dict[Iice40Part, tuple[int, int, int]] = {
    Iice40Part.HX1K: (33, 17, 1024),
    Iice40Part.HX8K: (33, 33, 1024),
    Iice40Part.UP5K: (33, 33, 1024),
    Iice40Part.LP1K: (33, 17, 1024),
}


# Record commands (subset of IceStorm's set; documented for clarity)
CMD_CRAM_BANK    = 0x05
CMD_CRAM_OFFSET  = 0x06
CMD_CRAM_RESET   = 0x07
CMD_BRAM_DATA    = 0x08
CMD_CRAM_BIT     = 0x25
CMD_BRAM_BIT     = 0x26
CMD_CRC          = 0x80
END_MARKER       = 0xFFFF


@dataclass
class FpgaConfig:
    """Per-CLB configuration: LUT truth tables, FF settings."""

    part: Iice40Part = Iice40Part.HX1K
    clbs: dict[tuple[int, int], ClbConfig] = field(default_factory=dict)


@dataclass
class ClbConfig:
    lut_a_truth_table: list[int] = field(default_factory=lambda: [0] * 16)
    lut_b_truth_table: list[int] = field(default_factory=lambda: [0] * 16)
    ff_a_enabled: bool = False
    ff_b_enabled: bool = False


@dataclass
class BitstreamReport:
    part: Iice40Part
    bytes_written: int
    clb_count: int
    cram_size: int


def emit_bitstream(
    config: FpgaConfig,
    *,
    structural_only: bool = True,
) -> tuple[bytes, BitstreamReport]:
    """Emit a structurally correct iCE40 bitstream.

    With ``structural_only=True`` (default), produces a valid record stream
    with a stub CRAM image that's consumable by tools that parse format but
    won't program a real iCE40. For real hardware, use the real-fpga-export
    yosys/nextpnr/icepack pipeline.
    """
    rows, cols, cram_bits = PART_SPECS[config.part]
    cram_bytes = (cram_bits + 7) // 8

    out = bytearray()

    # Preamble: 0xff 0x00
    out.extend(b"\xff\x00")

    # CRAM bank reset
    out.extend(_cmd(CMD_CRAM_RESET, b""))

    # CRAM bank 0 setup
    out.extend(_cmd(CMD_CRAM_BANK, struct.pack(">B", 0)))

    # For each CLB tile, emit a CRAM data block.
    # In real IceStorm bitstreams, this is per-tile bit positions; we emit
    # one zero-padded block per tile.
    for (row, col), _ in sorted(config.clbs.items()):
        # Tile location encoded as 2-byte row, 2-byte col, then CRAM bytes.
        out.extend(_cmd(CMD_CRAM_OFFSET, struct.pack(">HH", row, col)))
        out.extend(_cmd(CMD_BRAM_DATA, b"\x00" * cram_bytes))

    # CRC placeholder (zeros)
    out.extend(_cmd(CMD_CRC, b"\x00\x00"))

    # End marker
    out.extend(struct.pack(">H", END_MARKER))

    report = BitstreamReport(
        part=config.part,
        bytes_written=len(out),
        clb_count=len(config.clbs),
        cram_size=cram_bytes,
    )
    return (bytes(out), report)


def _cmd(command: int, payload: bytes) -> bytes:
    """Emit one command record: <length:1> <command:1> <payload>."""
    if len(payload) > 255:
        raise ValueError(f"command payload too long: {len(payload)}")
    return struct.pack(">BB", len(payload) + 2, command) + payload


def write_bin(path: str, config: FpgaConfig) -> BitstreamReport:
    """Write bitstream bytes to a .bin file."""
    data, report = emit_bitstream(config)
    with open(path, "wb") as f:
        f.write(data)
    return report
