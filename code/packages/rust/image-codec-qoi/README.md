# image-codec-qoi

QOI "Quite OK Image Format" encoder and decoder. Implements `ImageCodec` from
`pixel-container`. Zero external dependencies.

QOI is a lossless image format designed for simplicity and speed. It achieves
good compression on natural images through three primitives:

- **Run-length encoding** (`QOI_OP_RUN`) — repeat the previous pixel N times
- **Hash back-references** (`QOI_OP_INDEX`) — reference a recently-seen pixel by hash slot
- **Delta coding** (`QOI_OP_DIFF`/`QOI_OP_LUMA`) — store small RGB differences

## Usage

```rust
use pixel_container::PixelContainer;
use image_codec_qoi::{encode_qoi, decode_qoi};

// Encode
let mut buf = PixelContainer::new(320, 240);
buf.fill(200, 100, 50, 255);
let qoi_bytes = encode_qoi(&buf);
std::fs::write("output.qoi", &qoi_bytes).unwrap();

// Decode
let qoi_bytes = std::fs::read("input.qoi").unwrap();
let pixels = decode_qoi(&qoi_bytes).unwrap();
println!("{}×{}", pixels.width, pixels.height);
```

## Why Learn QOI?

QOI is a stepping stone toward JPEG and WebP:

| Technique | QOI op | Also in |
|-----------|--------|---------|
| Run-length encoding | `QOI_OP_RUN` | PackBits, TIFF, fax |
| Hash back-references | `QOI_OP_INDEX` | LZ77 (ZIP, PNG, WebP lossless) |
| Delta coding | `QOI_OP_DIFF`/`QOI_OP_LUMA` | JPEG DC coefficients, PNG filters, FLAC |

The LUMA operation is a simplified YCbCr transform — the same insight that
makes JPEG compression work on natural images.

## Format Details

- Header: 14 bytes (`b"qoif"`, width/height u32 BE, channels, colorspace)
- Chunks: variable-length, 6 operation types
- End marker: `[0,0,0,0, 0,0,0,1]` (8 bytes)
- Hash: `(r*3 + g*5 + b*7 + a*11) % 64`

See `code/specs/IC03-image-codec-qoi.md` for the full specification.
