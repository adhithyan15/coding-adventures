# cfb-writer (C++)

A **Compound File Binary Format (CFB / OLE2) writer**, header-only, pure ISO
C++17. A faithful port of the Rust [`cfb-writer`](../../rust/cfb-writer) crate,
in namespace `ca::cfb_writer`. You hand it named streams; it produces a byte
buffer a conforming CFB reader (and real Office tooling) accepts. CFB is the
container inside legacy `.xls` / `.doc` / `.ppt` files.

## Mental model

A CFB file is a FAT filesystem in one file: a 512-byte header, then 512-byte
sectors linked by a File Allocation Table. A directory of 128-byte entries names
the objects; streams under the 4096-byte cutoff are packed into a mini-stream of
64-byte mini-sectors chained by a parallel mini-FAT. Output is version 3 and
fully **deterministic** (CLSIDs/timestamps zeroed).

## API

```cpp
#include "cfb_writer.hpp"
namespace cw = ca::cfb_writer;

// Builder
cw::CfbWriter w;
w.add_stream("Workbook", data);        // std::vector<std::uint8_t> or (ptr,len)
std::vector<std::uint8_t> bytes = w.finish();

// One-shot
auto out = cw::write_cfb({{"A", va}, {"B", vb}});
```

- `CfbWriter::add_stream` (UTF-8 name → UTF-16LE, truncated to 31 code units) /
  `CfbWriter::finish` (returns the CFB bytes).
- `write_cfb(streams)` — the one-shot convenience over
  `std::pair<std::string, std::vector<std::uint8_t>>`.

Ownership is automatic (`std::vector` / `std::string`). Verified clean under
ASan + UBSan, with the output round-tripped through an in-test CFB reader.

## Building

Builds through the shared [`iso-harness`](../../c/iso-harness) engine under every
ISO C++ compiler on `PATH` with `-std=c++17 -pedantic-errors -Wall -Wextra
-Werror`:

```sh
sh BUILD          # POSIX: g++ and/or clang++
```

Each compiler prints `N checks, 0 failed`.
