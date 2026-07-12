# pixel-container (C)

A flat, row-major **RGBA8 pixel buffer** in pure ISO C17. A faithful port of the
`PixelContainer` type from the Rust `pixel-container` crate.

`PixelContainer` is the universal interchange type between renderers and image
codecs: 4 bytes per pixel in RGBA order, row-major from the top-left.

```
offset = (y * width + x) * 4
data[offset + 0] = R,  +1 = G,  +2 = B,  +3 = A
```

## API

```c
#include "pixel_container.h"

PixelContainer *buf = pixel_new(4, 4);       /* all-zero, transparent */
pixel_fill(buf, 255, 255, 255, 255);         /* white */
pixel_set(buf, 1, 1, 0, 128, 255, 200);

uint8_t rgba[4];
pixel_at(buf, 1, 1, rgba);                    /* {0,128,255,200} */

pixel_free(buf);
```

Constructors are malloc-owned (`pixel_free` releases). `pixel_at` returns
`{0,0,0,0}` and `pixel_set` is a no-op out of bounds. Where the Rust crate
*panics* (dimension overflow, a `from_data` length mismatch) this port returns
**NULL** — a library should not abort the process. Also: `pixel_clone`,
`pixel_equals`, `pixel_width`/`height`/`count`/`byte_count`/`data`.

## Portability

Pure ISO C17 — compiles clean under GCC, Clang, and MSVC with `-pedantic-errors`
/ `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../iso-harness).

## Development

```bash
# Compile and run the tests under every C compiler on PATH.
sh BUILD
```
