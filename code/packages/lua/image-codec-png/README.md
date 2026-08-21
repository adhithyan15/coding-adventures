# coding-adventures-image-codec-png

A pure Lua implementation of the bounded IC18 portable PNG profile. It emits
deterministic non-interlaced RGBA8 files and decodes 8-bit grayscale,
truecolour, grayscale-alpha, and RGBA inputs.

CRC-32 and raw RFC 1951 are reused from the repository `zip` package. This
package owns PNG framing, the RFC 1950 wrapper, filters, colour expansion, and
the exact IC18 validation order.

## Public API

```lua
local pc = require("coding_adventures.pixel_container")
local png = require("coding_adventures.image_codec_png")

local pixels = pc.new(2, 2)
pc.fill_pixels(pixels, 255, 0, 0, 255)

local encoded = png.encode_png(pixels)
local decoded = png.decode_png(encoded)

local codec = png.PngCodec.new({max_pixels = 1024})
assert(codec.mime_type == "image/png")
assert(pc.equals(pixels, codec:decode(codec:encode(pixels))))
```

`PNG_MAX_DIMENSION` is 16,384 and `PNG_MAX_PIXELS` is 33,554,432.
Callers may lower, but never raise, the pixel ceiling. Failures throw a
payload-blind `PngError` table whose `code`, `message`, and string form are the
same member of the immutable 29-code taxonomy.

## Portable and resource contract

The tests consume all 85 shared `image-codec-png-v1` cases through public APIs.
They additionally use test-only LibDeflate to inspect encoded filter rows and
Windows System.Drawing as a real foreign PNG decoder for exact RGBA recovery.

Lua ordinarily stores array numbers in large boxed table slots. The prerequisite
PixelContainer and ZIP changes in this tranche retain completed bytes in compact
4 KiB strings instead. Decoder chunk bounds, dimensions, and the caller pixel
limit are checked before derived allocation. Raw inflate receives the exact
filtered-size ceiling, must consume the entire DEFLATE slice, and is followed by
Adler and filter validation before compact RGBA storage is constructed.

Production imports only repository PixelContainer and ZIP modules and performs
pure in-memory work. It has no filesystem, network, process, environment, clock,
entropy, console, native, or credential authority. Fixture file access,
LibDeflate, and the real-image decoder are test-only.

## Development

Run `BUILD` on POSIX or `BUILD_windows` on Windows. Both install sibling
dependencies, syntax-check and lint source/tests, execute 94 Busted tests, and
enforce at least 90% production line coverage.
