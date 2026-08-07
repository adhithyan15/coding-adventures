# intel-8008-packager (C++)

An Intel HEX ROM image encoder/decoder for the Intel 8008, **header-only** in
pure ISO C++17 (namespace `ca::intel_8008_packager`). A faithful port of the
Rust [`intel-8008-packager`](../../rust/intel-8008-packager) crate.

## What it does

Converts raw binary machine code into the Intel HEX format used by EPROM
programmers, and parses Intel HEX back to binary for round-trip verification.
Records are `:LLAAAATTDD...CC` with a checksum that makes every record byte sum
to 0 mod 256.

## API

- `encode_hex(binary, origin)` → `std::string` (16 bytes per data record +
  trailing EOF).
- `decode_hex(text)` → `DecodedHex { origin, binary }`.
- Both throw `PackagerError` (a `std::runtime_error`) on any malformed,
  mis-checksummed, overlapping, over-long, or unterminated input.

## Design notes

- **Exceptions, not `Result`.** `encode_hex` / `decode_hex` throw
  `PackagerError` where Rust returns `Err`; results are `std::string` /
  `std::vector<std::uint8_t>`.
- **Strict, hardened decoder.** Uses a `std::map<address, bytes>` (mirroring the
  Rust `BTreeMap`) to keep segments sorted and reject overlaps against the
  immediate neighbours; declared lengths are bounded by the remaining input and
  the decoded span is capped at the 8008's 16 KB address space.
- **Header-only.** `#include "intel_8008_packager.hpp"` and go.

## Usage

```cpp
#include "intel_8008_packager.hpp"
using namespace ca::intel_8008_packager;

std::string hex = encode_hex({0x06, 0x00, 0xFF}, 0);   // MVI B,0; HLT
DecodedHex d = decode_hex(hex);                         // d.binary == {0x06,0x00,0xFF}
```

## Building

```sh
sh BUILD           # POSIX: g++ and/or clang++ via the shared iso-harness
```

Compiles under GCC, Clang and MSVC with `-pedantic-errors` / `/permissive-` and
warnings-as-errors.
