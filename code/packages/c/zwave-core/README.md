# zwave-core (C)

**Z-Wave identifier, region, and Serial API frame primitives** in pure ISO C17.
A faithful port of the Rust [`zwave-core`](../../rust/zwave-core) crate. It is not
a controller — it gives later Z-Wave code a tested byte boundary for controller
serial frames, node identity, command-class ids, and regional-profile metadata.

## What it does

- **NodeId** — classic (1..=232) and Long Range (1..=4000), range-validated.
- **HomeId** — 32-bit id → 4 big-endian bytes.
- **RegionProfile** — 12 regions with band descriptions and long-range support.
- **CommandClassId** — 1- or 2-byte encoding (ids ≥ 0x100 are "extended"), plus
  actuator / sensor / security classification.
- **CommandClassFrame** — parse / encode `[class id][command id][payload]`.
- **SerialFrame** — parse / encode `SOF | len | type | function | payload | XOR
  checksum`, with full bounds and checksum validation.
- **Summaries** — network, command-class-frame, serial-frame-batch, and a
  controller-readiness roll-up.

The two `*_parse` functions read **untrusted bytes** and bounds-check every
field, reporting a structured `ZWaveError` (its `a`/`b` fields carry the
parametric values, e.g. `Truncated{needed, remaining}`).

## API

```c
#include "zwave_core.h"

ZWaveSerialFrame f;
zw_serial_frame_init(ZW_SERIAL_REQUEST, 0x13, payload, len, &f);
uint8_t *bytes; size_t n;
zw_serial_frame_encode(&f, &bytes, &n);        /* caller frees bytes */

ZWaveSerialFrame parsed;
if (zw_serial_frame_parse(bytes, n, &parsed).kind == ZW_OK) {
    zw_serial_frame_free(&parsed);
}
free(bytes);
zw_serial_frame_free(&f);
```

Frames own a malloc'd payload copy — pair every `_init` / `_parse` with `_free`.
Verified leak-free under ASan + UBSan.

## Building

Builds through the shared [`iso-harness`](../iso-harness) engine under every ISO
C compiler on `PATH` with `-std=c17 -pedantic-errors -Wall -Wextra -Werror`:

```sh
sh BUILD          # POSIX: gcc and/or clang
```

Each compiler prints `N checks, 0 failed`.
