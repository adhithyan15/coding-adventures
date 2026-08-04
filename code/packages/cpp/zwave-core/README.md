# zwave-core (C++)

**Z-Wave identifier, region, and Serial API frame primitives**, header-only, pure
ISO C++17. A faithful port of the Rust [`zwave-core`](../../rust/zwave-core)
crate, in namespace `ca::zwave_core`. Not a controller — it provides a tested
byte boundary for controller serial frames, node identity, command-class ids,
and regional-profile metadata.

## What it does

- **`NodeId`** — `classic` (1..=232) / `long_range` (1..=4000), range-validated.
- **`home_id_to_be_bytes`** — 32-bit id → `std::array<std::uint8_t, 4>`.
- **`RegionProfile`** — `band_description` / `supports_long_range`.
- **`CommandClassId`** — `encoded_len` / `encode` / actuator-sensor-security
  classification, with named constants (`kSwitchBinary`, `kSecurity2`, …).
- **`CommandClassFrame`** / **`SerialFrame`** — `parse` (throws on bad input) /
  `encode`; the serial framing validates SOF, length, type, and XOR checksum.
- **Summaries** — `NetworkSummary`, `CommandClassFrameSummary`,
  `SerialFrameBatchSummary`, and `ControllerReadinessSummary`.

`parse` reads **untrusted bytes** and throws `ca::zwave_core::Error` (carrying an
`ErrorKind` plus `a()`/`b()` parametric detail) on malformed input.

## API

```cpp
#include "zwave_core.hpp"
namespace zw = ca::zwave_core;

zw::SerialFrame f{zw::SerialFrameType::Request, 0x13, {0x02, 0x25, 0x01}};
std::vector<std::uint8_t> bytes = f.encode();
zw::SerialFrame parsed = zw::SerialFrame::parse(bytes);  // throws zw::Error
```

Ownership is automatic (`std::vector` payloads). Verified clean under ASan + UBSan.

## Building

Builds through the shared [`iso-harness`](../../c/iso-harness) engine under every
ISO C++ compiler on `PATH` with `-std=c++17 -pedantic-errors -Wall -Wextra
-Werror`:

```sh
sh BUILD          # POSIX: g++ and/or clang++
```

Each compiler prints `N checks, 0 failed`.
