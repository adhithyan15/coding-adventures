# coding_adventures_image_codec_qoi

**IC03** — QOI (Quite OK Image) encoder/decoder. Part of the
[coding-adventures](https://github.com/adhithyan15/coding-adventures) monorepo.

## What It Does

Encodes and decodes QOI image files using all 6 op-codes. QOI achieves
near-PNG compression at 10–20× faster encode/decode speeds by exploiting
pixel locality rather than general-purpose compression.

```elixir
alias CodingAdventures.{PixelContainer, ImageCodecQoi}

# Encode to QOI
c = PixelContainer.new(640, 480)
c = PixelContainer.fill_pixels(c, 135, 206, 235, 255)
qoi_data = ImageCodecQoi.encode(c)
File.write!("sky.qoi", qoi_data)

# Decode from QOI
{:ok, loaded} = ImageCodecQoi.decode(File.read!("sky.qoi"))
PixelContainer.pixel_at(loaded, 0, 0)   # => {135, 206, 235, 255}
```

## QOI Op-Codes

| Op | Bytes | Condition | Encoding |
|---|---|---|---|
| `QOI_OP_RUN` | 1 | Same pixel 1–62 times | `11xxxxxx` (run-1 in lower 6 bits) |
| `QOI_OP_INDEX` | 1 | Pixel in hash table | `00xxxxxx` (6-bit index) |
| `QOI_OP_DIFF` | 1 | Δr,Δg,Δb each in −2..+1 | `01drrgggdbb` (bias-2 per channel) |
| `QOI_OP_LUMA` | 2 | Δg in −32..+31, Δr−Δg,Δb−Δg in −8..+7 | `10dddddd` then nibbles |
| `QOI_OP_RGB` | 4 | Any RGB change, α same | `0xFE r g b` |
| `QOI_OP_RGBA` | 5 | Any RGBA change | `0xFF r g b a` |

## Hash Function

```
index = rem(r*3 + g*5 + b*7 + a*11, 64)
```

## Where It Fits

```
IC03 QOI codec  ← this package
  └── IC00 PixelContainer
```

## Running Tests

```bash
mix deps.get && mix test
```

## Version

0.1.0 — IC03 spec compliant, all 6 op-codes implemented.
