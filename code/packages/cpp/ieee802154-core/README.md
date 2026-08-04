# ieee802154-core (C++)

A **parser/encoder for IEEE 802.15.4 MAC frames** — header-only, ISO C++17 — the
byte-level foundation both Zigbee and Thread build on. A faithful port of the
Rust [`ieee802154-core`](../../rust/ieee802154-core) crate, in namespace
`ca::ieee802154_core`.

## What it covers

Frame control, MAC frame parse (with/without FCS) / encode / summary, the
auxiliary security header (32/40-bit frame counter, four key-identifier modes),
beacon payloads (superframe spec, GTS descriptors, pending addresses), and PAN
descriptor / scan-summary filtering and ranking. Every read is bounds-checked.

## API

```cpp
#include "ieee802154_core.hpp"
namespace ie = ca::ieee802154_core;

auto f = ie::MacFrame::parse_without_fcs(bytes);  // throws ie::MacError
auto s = f.summary();                              // s.frame_type, s.payload_len
auto out = f.encode();                             // std::vector<std::uint8_t>

auto bp = ie::BeaconPayload::parse(f.payload);      // throws ie::BeaconError
auto pd = ie::PanDescriptor::from_beacon_frame(f, 15, 0, 244);
```

- `MacFrame::parse_*` / `encode` / `summary` throw `ie::MacError` where the Rust
  returns `Result`; `BeaconPayload::parse` and `PanDescriptor::from_beacon_frame`
  throw `ie::BeaconError`. `std::optional` for optional fields, `std::vector` for
  payloads/addresses, `std::array` for the key source — RAII throughout.
- `FrameControl::parse/encode`, `SuperframeSpecification` accessors,
  `SecurityControl`, `encrypts`/`mic_len`, and `PanScanSummary` helpers
  (`descriptors_for_channel`, `association_candidates`,
  `best_association_candidate`).

## Building

```sh
sh BUILD          # POSIX: g++ and/or clang++, via the shared iso-harness
```

Each compiler prints `N checks, 0 failed`. Verified clean under ASan + UBSan.
