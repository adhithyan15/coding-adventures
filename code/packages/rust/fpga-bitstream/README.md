# fpga-bitstream

iCE40 bitstream emitter using the Project IceStorm record-stream format.

## What it does

This crate takes an `FpgaConfig` (a map of CLB positions to LUT truth tables and flip-flop
enables) and emits a byte vector in the iCE40 record-stream format used by Project IceStorm
(`icepack`).  The output is a `.bin` file that can be flashed to an iCE40 FPGA with `iceprog`.

The emitted bitstream contains:

- Preamble bytes (`0xFF 0x00`)
- A CRAM reset command
- A CRAM bank-select command
- Per-CLB offset + data records (one per populated CLB)
- A CRC placeholder record
- An end-of-stream record

## How it fits in the stack

```
fpga-place-route-bridge → FpgaConfig
                                │
                                ▼
                         fpga-bitstream
                                │
                                ▼
                          .bin file → iceprog → iCE40 board
```

## Usage

```rust
use fpga_bitstream::{FpgaConfig, ClbConfig, Ice40Part, emit_bitstream, write_bin};
use std::path::Path;

let mut config = FpgaConfig::new(Ice40Part::Hx1k);

// Wire an AND gate: truth table [0,0,0,1] expanded to 16 entries
config.clbs.insert((0, 0), ClbConfig {
    lut_a_truth_table: vec![0,0,0,1, 0,0,0,1, 0,0,0,1, 0,0,0,1],
    lut_b_truth_table: vec![0u8; 16],
    ff_a_enabled: false,
    ff_b_enabled: false,
});

let (bytes, report) = emit_bitstream(&config);
println!("written {} bytes, {} CLBs", report.bytes_written, report.clb_count);

write_bin(Path::new("out.bin"), &config).unwrap();
```

## Supported parts

| Part       | Rows | Cols | CRAM size |
|------------|------|------|-----------|
| `Hx1k`     |  33  |  17  |  1 024    |
| `Hx8k`     |  33  |  33  |  1 024    |
| `Up5k`     |  33  |  33  |  1 024    |
| `Lp1k`     |  33  |  17  |  1 024    |

## Record format

Each record in the stream has the layout:

```
[ total_len : u8 | cmd : u8 | payload... ]
```

Where `total_len = payload.len() + 2` (includes the length byte and command byte themselves).
The maximum payload is 253 bytes (so total_len ≤ 255).

### Command codes

| Code   | Name            | Meaning                              |
|--------|-----------------|--------------------------------------|
| `0x01` | `CRAM_DATA`     | Configuration RAM data               |
| `0x05` | `CRAM_BANK`     | Select CRAM bank                     |
| `0x06` | `CRAM_OFFSET`   | Seek to CRAM position                |
| `0x07` | `CRAM_RESET`    | Reset CRAM pointer                   |
| `0x08` | `BRAM_DATA`     | Block RAM data (not used here)       |
| `0x80` | `CRC`           | CRC check value                      |

## Limitations

This is a structural stub emitter for the iCE40 format.  It emits syntactically valid record
streams that real tools (icepack, iceprog) understand, but the CRAM payload for each CLB is
derived from the LUT truth table directly without full tile geometry encoding.  For production
use, feed the JSON from `fpga-place-route-bridge` into `icepack` via the `real-fpga-export`
toolchain driver instead.
