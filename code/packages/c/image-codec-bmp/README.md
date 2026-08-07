# image-codec-bmp (C)

**CCPP02 port campaign — bucket A (pure-ISO), port #5.** A 32-bit BGRA **BMP**
(Windows bitmap) image encoder and decoder — the PPM codec's sibling. The C port
of the Rust `image-codec-bmp` crate, a pure-ISO crate that needs no OS, so it
rides the `iso-harness` (links nothing, `-pedantic-errors` / `/permissive-`).

```
BITMAPFILEHEADER  (14 bytes)  "BM", file size, pixel-data offset
BITMAPINFOHEADER  (40 bytes)  width, height, bit depth, compression, …
pixel data        (width*height*4 bytes, BGRA)      ← all little-endian
```

```c
PixelContainer *img = pixel_new(2, 2);
pixel_fill(img, 100, 150, 200, 255);

unsigned char *bmp; size_t len;
bmp_encode(img, &bmp, &len);          /* "BM" … 54-byte header + BGRA raster */

PixelContainer *back;
bmp_decode(bmp, len, &back);

bmp_free(bmp);
pixel_free(back);
pixel_free(img);
```

| Function | Purpose |
|----------|---------|
| `bmp_mime_type` | `"image/bmp"` |
| `bmp_encode` | container → fresh BMP byte buffer (`bmp_free` to release) |
| `bmp_decode` | BMP bytes → fresh `PixelContainer` (`pixel_free` to release) |
| `bmp_free` | release an encoded buffer |

## Composes `c/pixel-container`

Pixels live in a [`PixelContainer`](../pixel-container) (RGBA8). BMP stores pixels
**BGRA** (blue first), so the only transform is swapping R and B per pixel. This
package is itself pure-ISO — `run.sh` compiles pixel-container's source in;
nothing is linked.

## Format notes

- **Top-down vs bottom-up.** Classic BMP is *bottom-up* (the last image row comes
  first in the file); a **negative** `biHeight` marks *top-down*. The encoder
  always writes a negative `biHeight` (top-down, matching the container — no row
  reversal), and the decoder accepts **both** layouts.
- **32-bit BI_RGB only.** `bmp_decode` requires `biBitCount == 32` and
  `biCompression == 0` (`BMP_ERR_BIT_DEPTH` / `BMP_ERR_COMPRESSION` otherwise),
  matching the Rust.

## Faithfulness & safety notes

- **`Result<_, String>` → status codes** (`BMP_ERR_TOO_SHORT` / `_MAGIC` /
  `_OFFSET` / `_WIDTH` / `_HEIGHT` / `_BIT_DEPTH` / `_COMPRESSION` / `_TRUNCATED`
  / `_OVERFLOW` / `_NOMEM`).
- **Portable little-endian reads.** A dedicated `read_i32` reconstructs the signed
  `biWidth`/`biHeight` via explicit two's-complement (no implementation-defined
  unsigned→signed cast), and rejects `i32::MIN` height exactly as the Rust does.
- **Untrusted input.** The decoder parses attacker-controlled bytes: every header
  field lives inside the validated 54-byte prefix, every raster byte is inside
  the checked `pixel_offset + width*height*4 <= len` window, and every
  `width*height*4` / `offset+size` is `size_t`-overflow guarded → `BMP_ERR_OVERFLOW`.
  An adversarial review confirmed no out-of-bounds access, overflow, or leak.

## Build & test

Pure ISO, no OS, no link libraries.

```sh
cd code/packages/c/image-codec-bmp
sh tools/run.sh        # macOS / Linux (Windows: tools\run.ps1 via BUILD_windows)
```

Locally (macOS): 64 checks / 0 failed under gcc + clang with `-pedantic-errors`;
clean under ASan+UBSan; 0 leaks.

## Layout

```
image-codec-bmp/
├── include/image_codec_bmp/bmp_codec.h   # public API
├── src/bmp_codec.c                         # encoder + decoder — one pure-ISO source
├── tests/bmp_codec_test.c                  # Rust tests + bottom-up + edge/NULL paths
├── tools/run.sh  · run.ps1                   # build via iso-harness (+ pixel-container)
├── BUILD  · BUILD_windows                    # deps: c/iso-harness c/pixel-container
└── .gitignore
```
