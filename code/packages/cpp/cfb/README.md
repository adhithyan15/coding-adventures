# cfb (C++)

A **reader for the OLE2 / Compound File Binary Format** ([MS-CFB]) — header-only,
ISO C++17 — the container inside legacy `.xls`, `.doc`, and `.ppt` files. A
faithful port of the Rust [`cfb`](../../rust/cfb) crate, in namespace `ca::cfb`,
and the read counterpart to the ported [`cfb-writer`](../cfb-writer).

## Mental model

A CFB file is a FAT filesystem in one file: fixed-size sectors chained by a File
Allocation Table, a directory stream naming streams (files) and storages
(folders), and a mini-stream packing tiny streams. Because CFB files arrive as
attachments, every chain walk is cycle-guarded, every offset bounds-checked with
overflow-safe arithmetic, and output capped at 256 MiB.

## API

```cpp
#include "cfb.hpp"
namespace cfb = ca::cfb;

auto cf = cfb::CompoundFile::open(bytes);   // throws cfb::CfbError on failure
for (const auto& e : cf.entries()) {         // e.name, e.size, e.kind, e.id
}
if (auto data = cf.read_stream("Workbook")) { /* std::optional<vector<u8>> */ }
```

- `CompoundFile::open` throws `cfb::CfbError` where the Rust `open` returns
  `Result`.
- `entries()`, `stream_names()`, `sector_size()`, `read_stream(name)` →
  `std::optional<std::vector<std::uint8_t>>` (ASCII case-insensitive), and
  `read_stream_by_id(id)` (throws). RAII throughout (`std::vector`).

## Building

Builds through the shared [`iso-harness`](../../c/iso-harness) engine under every
ISO C++ compiler on `PATH` with `-std=c++17 -pedantic-errors -Wall -Wextra
-Werror`:

```sh
sh BUILD          # POSIX: g++ and/or clang++
```

Each compiler prints `N checks, 0 failed`. Tests craft in-memory CFB files (no
external fixture). Verified clean under ASan + UBSan, including a truncation
fuzz over every prefix of a valid file.
