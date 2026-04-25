# fpga-bitstream

iCE40 bitstream emitter (Project IceStorm record-stream format). **Zero deps.**

See [`code/specs/fpga-bitstream.md`](../../../specs/fpga-bitstream.md).

## Important caveat

v0.1.0 emits **structurally correct** iCE40 record-stream bitstreams (preamble + commands + end marker) but uses a **stub CRAM image** (zeros). To program real silicon, the per-tile config bits need to be mapped to (row, col) CRAM positions using Project IceStorm's chip database (chipdb-1k.txt etc.); that mapping is left for v0.2.0.

For real hardware **today**, use the `real-fpga-export` package's path: emit Verilog from HIR, hand to `yosys`/`nextpnr`/`icepack`/`iceprog`. That works end-to-end now.

## Quick start

```python
from fpga_bitstream import FpgaConfig, ClbConfig, Iice40Part, emit_bitstream, write_bin

config = FpgaConfig(part=Iice40Part.HX1K)
config.clbs[(5, 7)] = ClbConfig(lut_a_truth_table=[0,0,0,1,0,0,0,1,0,0,0,1,0,0,0,1])
config.clbs[(5, 8)] = ClbConfig(lut_a_truth_table=[1,0,0,0]*4)

# Get the bytes
data, report = emit_bitstream(config)
print(report.bytes_written, report.clb_count)

# Or write directly to a .bin file
write_bin("adder4.bin", config)
```

## v0.1.0 scope

- `Iice40Part`: HX1K / HX8K / UP5K / LP1K (with approximate dimensions in `PART_SPECS`).
- `FpgaConfig` / `ClbConfig`: per-CLB LUT truth tables + FF enables.
- Record-stream emission: preamble, CRAM commands (RESET, BANK, OFFSET, DATA), CRC placeholder, end marker.
- `emit_bitstream(config)` returns (bytes, BitstreamReport).
- `write_bin(path, config)` writes to file.

## Out of scope (v0.2.0)

- Project IceStorm chip database integration (real (row, col) -> CRAM bit mapping).
- ECP5 (Project Trellis) support.
- Xilinx 7-series (Project X-Ray) support.
- Bitstream encryption / authentication.
- Partial reconfiguration.

MIT.
