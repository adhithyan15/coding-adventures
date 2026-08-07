# ieee802154-core (C)

A **parser/encoder for IEEE 802.15.4 MAC frames** in pure ISO C17 — the
byte-level foundation both Zigbee and Thread build on. A faithful port of the
Rust [`ieee802154-core`](../../rust/ieee802154-core) crate.

## What it covers

- **Frame control** — the 16-bit field: frame type, addressing modes, version,
  security/ack/pending/compression/suppression bits.
- **MAC frame** — parse (with or without a trailing 2-byte FCS) and encode:
  sequence number, PAN ids, short/extended addresses (with PAN-id compression),
  the auxiliary security header, payload, and FCS. Plus a body-free `summary`.
- **Auxiliary security header** — security control, 32/40-bit frame counter, and
  the four key-identifier modes.
- **Beacon payload** — superframe specification, GTS descriptors, and pending
  short/extended addresses.
- **PAN descriptor / scan summary** — derive a descriptor from a beacon frame
  and filter/rank association candidates.

Every multi-byte field is little-endian and every read is bounds-checked, so a
truncated or hostile frame yields an error, never an out-of-bounds access.

## API

```c
#include "ieee802154_core.h"

IE_MacFrame f;
if (ie_mac_frame_parse(bytes, len, /*has_fcs=*/0, &f) == IE_MAC_OK) {
    IE_MacFrameSummary s;
    ie_mac_frame_summary(&f, &s);          /* s.frame_type, s.payload_len, ... */
    uint8_t *out; size_t out_len;
    ie_mac_frame_encode(&f, &out, &out_len);   /* caller frees out */
    free(out);
    ie_mac_frame_free(&f);
}
```

- `ie_mac_frame_parse` / `_encode` / `_summary` / `_free`; `ie_frame_control_parse`
  / `_encode`; `ie_beacon_payload_parse` / `_free`;
  `ie_pan_descriptor_from_beacon_frame` and the scan helpers; plus the superframe
  and security-level accessors.
- Parse-produced payloads are heap-owned (freed by the `_free` functions);
  bounded MAC counts (GTS/pending ≤ 7) use fixed arrays. The encode buffer guards
  `size_t` overflow.

Verified clean under ASan + UBSan, the macOS `leaks` tool (0 leaks), and a
300k-iteration random-input parse/encode fuzz.

### Divergences from the Rust

Error enums drop the diagnostic `field`/`needed`/`remaining` payloads the Rust
variants carry (the variant itself is preserved). `IE_MAC_OK` / `IE_BEACON_OK`
are added as success sentinels.

## Building

```sh
sh BUILD          # POSIX: gcc and/or clang, via the shared iso-harness
```

Each compiler prints `N checks, 0 failed`.
