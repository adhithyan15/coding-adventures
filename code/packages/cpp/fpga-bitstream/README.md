# fpga-bitstream (C++)

Emit iCE40 FPGA bitstreams in the Project IceStorm record-stream format, in pure
ISO C++17, header-only, in namespace `ca::fpga`. A faithful port of the Rust
`fpga-bitstream` crate.

The iCE40 stream is a sequence of variable-length records
`[total_len][command][payload…]`, framed by the preamble `0xFF 0x00` and the end
marker `0xFFFF`. Like the Rust crate, this emits a **structurally correct** stream
with a stub (all-zero) CRAM image.

## API

```cpp
#include "fpga_bitstream.hpp"
namespace fpga = ca::fpga;

fpga::FpgaConfig cfg(fpga::Ice40Part::Hx1k);
cfg.clbs[{0, 0}] = fpga::ClbConfig{};
auto [bytes, report] = fpga::emit_bitstream(cfg);   // std::vector<uint8_t>
```

`FpgaConfig::clbs` is a `std::map`, so it iterates in `(row, col)` order — the
output is byte-identical to the Rust crate and deterministic. `cmd` throws
`std::length_error` on a payload > 253 bytes; `write_bin` throws
`std::runtime_error` on a file error.

## Portability

Pure ISO C++17 — standard library only. Compiles clean under GCC, Clang, and MSVC
with `-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../../c/iso-harness).

## Development

```bash
sh BUILD
```
