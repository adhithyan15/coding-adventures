# image-codec-ppm (C)

**CCPP02 port campaign — bucket A (pure-ISO), port #4.** A Netpbm **PPM (P6)**
image encoder and decoder — the simplest real image format. The C port of the
Rust `image-codec-ppm` crate, a pure-ISO crate that needs no OS, so it rides the
`iso-harness` (links nothing, `-pedantic-errors` / `/permissive-`).

```
P6\n
<width> <height>\n
255\n
<width*height*3 raw bytes: R G B per pixel, row-major from the top-left>
```

No compression, no metadata, no padding. Files this encoder writes are read by
ImageMagick / ffmpeg / any Netpbm tool, and vice-versa.

```c
PixelContainer *img = pixel_new(2, 1);
pixel_set(img, 0, 0, 255, 0, 0, 255);       /* red   */
pixel_set(img, 1, 0, 0, 0, 255, 255);       /* blue  */

unsigned char *ppm; size_t len;
ppm_encode(img, &ppm, &len);                 /* "P6\n2 1\n255\n" + 6 RGB bytes */

PixelContainer *back;
ppm_decode(ppm, len, &back);                 /* alpha restored to 255 */

ppm_free(ppm);
pixel_free(back);
pixel_free(img);
```

| Function | Purpose |
|----------|---------|
| `ppm_mime_type` | `"image/x-portable-pixmap"` |
| `ppm_encode` | container → fresh PPM P6 byte buffer (`ppm_free` to release) |
| `ppm_decode` | PPM P6 bytes → fresh `PixelContainer` (`pixel_free` to release) |
| `ppm_free` | release an encoded buffer |

## Composes `c/pixel-container`

Pixels live in a [`PixelContainer`](../pixel-container) (an RGBA8 buffer from the
pure-ISO `pixel-container` package). PPM has no alpha channel, so the bridge is:
**encode drops** the alpha byte (writes only R, G, B), and **decode restores** it
to 255 (opaque). This package is itself pure-ISO — `run.sh` compiles
pixel-container's source in; nothing is linked.

## Faithfulness & safety notes

- **`Result<_, String>` → status codes.** Each Rust error message becomes a
  distinct `ppm_status` (`PPM_ERR_MAGIC` / `_DIMENSIONS` / `_MAXVAL` /
  `_TRUNCATED` / `_OVERFLOW` / `_NOMEM`).
- **Header parser** is whitespace- and `#`-comment-aware, using the same
  whitespace set as Rust's `is_ascii_whitespace` (space / `\t` / `\n` / `\r` /
  `\f` — note: *not* the vertical tab), and consumes exactly one whitespace byte
  between the header and the binary raster.
- **Untrusted input.** The decoder parses attacker-controlled bytes; every pixel
  read is bounds-checked (`len - pos >= width*height*3` before the loop, with no
  unsigned underflow), and every `width*height*{3,4}` size is `size_t`-overflow
  checked (Rust's `checked_mul`) → `PPM_ERR_OVERFLOW`. Dimensions beyond
  `UINT32_MAX` are rejected before the cast into the container. An adversarial
  review confirmed no out-of-bounds access, overflow, or leak.
- **Only maxval 255** is supported (matching the Rust); anything else is
  `PPM_ERR_MAXVAL`.

## Build & test

Pure ISO, no OS, no link libraries.

```sh
cd code/packages/c/image-codec-ppm
sh tools/run.sh        # macOS / Linux (Windows: tools\run.ps1 via BUILD_windows)
```

Locally (macOS): 121 checks / 0 failed under gcc + clang with `-pedantic-errors`;
clean under ASan+UBSan; 0 leaks.

## Layout

```
image-codec-ppm/
├── include/image_codec_ppm/ppm_codec.h   # public API
├── src/ppm_codec.c                         # encoder + decoder — one pure-ISO source
├── tests/ppm_codec_test.c                  # the Rust tests + edge/NULL paths
├── tools/run.sh  · run.ps1                   # build via iso-harness (+ pixel-container)
├── BUILD  · BUILD_windows                    # deps: c/iso-harness c/pixel-container
└── .gitignore
```
