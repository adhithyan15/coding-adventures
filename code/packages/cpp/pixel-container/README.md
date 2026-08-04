# pixel-container (C++)

A flat, row-major **RGBA8 pixel buffer** and an **image-codec interface** in pure
ISO C++17, header-only, in namespace `ca`. A faithful port of the Rust
`pixel-container` crate.

`PixelContainer` is the universal interchange type between renderers and image
codecs: 4 bytes per pixel in RGBA order, row-major from the top-left. `ImageCodec`
is the abstract base every codec (BMP, PPM, QOI, PNG, …) implements to
encode/decode a `PixelContainer` — no rendering types in scope.

```
offset = (y * width + x) * 4
data[offset + 0] = R,  +1 = G,  +2 = B,  +3 = A
```

## API

```cpp
#include "pixel_container.hpp"
using ca::PixelContainer;

PixelContainer buf(4, 4);                 // all-zero, transparent
buf.fill(255, 255, 255, 255);             // white
buf.set_pixel(1, 1, 0, 128, 255, 200);
auto px = buf.pixel_at(1, 1);             // std::array<uint8_t,4>{0,128,255,200}

auto p = PixelContainer::from_data(1, 1, {255, 0, 0, 255});  // throws on mismatch
```

`PixelContainer` has value semantics (deep copy) and `operator==`; `pixel_at`
returns `{0,0,0,0}` and `set_pixel` is a no-op out of bounds. `from_data` throws
`std::invalid_argument` on a length mismatch and the constructor throws
`std::length_error` on dimension overflow (the Rust crate panics). Implement
`ca::ImageCodec` (`mime_type` / `encode` / `decode`) for a format.

## Portability

Pure ISO C++17 — compiles clean under GCC, Clang, and MSVC with
`-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../../c/iso-harness). Standard library only.

## Development

```bash
# Compile and run the tests under every C++ compiler on PATH.
sh BUILD
```
