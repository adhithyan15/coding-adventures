# fpga-bitstream (C)

Emit iCE40 FPGA bitstreams in the Project IceStorm record-stream format, in pure
ISO C17. A faithful port of the Rust `fpga-bitstream` crate.

A bitstream is the binary blob that programs an FPGA's configuration RAM at
power-on. The iCE40 stream is a sequence of variable-length records
`[total_len][command][payload…]`, framed by the preamble `0xFF 0x00` and the end
marker `0xFFFF`.

Like the Rust crate, this emits a **structurally correct** stream with a stub
CRAM image (all zeros) — real-hardware bit placement needs the IceStorm chip
database, which is out of scope.

## API

```c
#include "fpga_bitstream.h"

FpgaConfig *cfg = fpga_config_new(ICE40_HX1K);
FpgaClbConfig clb = fpga_clb_config_default();
fpga_config_insert_clb(cfg, 0, 0, &clb);

size_t len; FpgaBitstreamReport rep;
uint8_t *bytes = fpga_emit_bitstream(cfg, &len, &rep);   /* bytes[0]==0xFF … */
free(bytes);
fpga_config_free(cfg);
```

`emit` sorts the CLBs by `(row, col)` exactly as the Rust does, so the output is
byte-identical and deterministic. Where the Rust `cmd` panics on a payload longer
than 253 bytes, this port returns NULL / a status. `fpga_write_bin` writes the
stream to a file via `<stdio.h>`. Growable buffers guard their doubling against
`size_t` overflow.

The expected byte streams in the test suite are the **authoritative output of
the real Rust crate**, captured via a temporary oracle test.

## Portability

Pure ISO C17 — no extensions. Compiles clean under GCC, Clang, and MSVC with
`-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../iso-harness).

## Development

```bash
sh BUILD
```
