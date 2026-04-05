// Package imagecodecbmp encodes and decodes 32-bit BMP images.
//
// # BMP File Structure
//
// BMP (Bitmap) is one of the oldest and simplest raster image formats.
// It was designed by Microsoft in the late 1980s. Its defining characteristic
// is that pixel data is stored uncompressed — every pixel occupies exactly the
// same number of bytes with no encoding tricks. This makes BMP trivial to
// write and read, at the cost of large file sizes.
//
// A BMP file has three main regions:
//
//  1. BITMAPFILEHEADER (14 bytes) — magic number, file size, data offset.
//  2. BITMAPINFOHEADER (40 bytes) — image dimensions, colour depth, compression.
//  3. Pixel data — raw bytes for every pixel.
//
// In total, the header region is 54 bytes before any pixels.
//
// # Channel Order: BGRA, Not RGBA
//
// BMP stores channels in a surprising order: Blue, Green, Red, Alpha.
// This is a historical artefact from how Windows stored colours internally
// (little-endian 32-bit COLORREF values). When encoding we swap R↔B; when
// decoding we swap them back.
//
// # Top-Down vs Bottom-Up
//
// By default BMP rows are stored bottom-up (row 0 in the file is the bottom
// of the image). A negative biHeight value signals top-down storage.
// This implementation encodes top-down (negative biHeight) and handles both
// on decode.
package imagecodecbmp

import (
	"bytes"
	"encoding/binary"
	"errors"
	"fmt"

	pc "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
)

// BmpCodec implements pc.ImageCodec for the BMP format.
//
// Use it wherever the codebase expects an ImageCodec:
//
//	var codec pc.ImageCodec = imagecodecbmp.BmpCodec{}
//	data := codec.Encode(img)
type BmpCodec struct{}

// MimeType returns the IANA media type for BMP.
func (BmpCodec) MimeType() string { return "image/bmp" }

// Encode encodes a PixelContainer to 32-bit BGRA BMP bytes.
func (BmpCodec) Encode(c *pc.PixelContainer) []byte { return EncodeBmp(c) }

// Decode parses BMP bytes into a PixelContainer.
func (BmpCodec) Decode(data []byte) (*pc.PixelContainer, error) { return DecodeBmp(data) }

// ── File-header constants ─────────────────────────────────────────────────────

const (
	// fileHeaderSize is the size of BITMAPFILEHEADER in bytes.
	fileHeaderSize = 14

	// infoHeaderSize is the size of BITMAPINFOHEADER in bytes.
	infoHeaderSize = 40

	// headerTotal is the combined header size before pixel data.
	headerTotal = fileHeaderSize + infoHeaderSize

	// biBitCount for 32-bit colour (BGRA).
	bitCount32 = 32

	// biCompression value for uncompressed RGB (confusingly called BI_RGB).
	biRGB = 0

	// bytesPerPixel for 32-bit BMP.
	bytesPerPixel = 4
)

// EncodeBmp serialises a PixelContainer to 32-bit BGRA BMP bytes.
//
// The resulting file can be opened by any standard image viewer.
//
// Header layout:
//
//	Offset  Size  Field            Value
//	------  ----  -----            -----
//	 0      2     bfType           0x42 0x4D ("BM")
//	 2      4     bfSize           total file size in bytes
//	 6      2     bfReserved1      0
//	 8      2     bfReserved2      0
//	10      4     bfOffBits        54  (offset to pixel data)
//	14      4     biSize           40  (size of info header)
//	18      4     biWidth          image width in pixels
//	22      4     biHeight         –image height (negative = top-down)
//	26      2     biPlanes         1
//	28      2     biBitCount       32
//	30      4     biCompression    0   (BI_RGB)
//	34      4     biSizeImage      0   (allowed for BI_RGB)
//	38      4     biXPelsPerMeter  0
//	42      4     biYPelsPerMeter  0
//	46      4     biClrUsed        0
//	50      4     biClrImportant   0
//	54      …     pixel data       BGRA rows, top-down
func EncodeBmp(c *pc.PixelContainer) []byte {
	w := c.Width
	h := c.Height

	// Each row is w pixels × 4 bytes/pixel. 32-bit rows are already DWORD-
	// aligned for any width, so there is no row-padding needed.
	pixelDataSize := w * h * bytesPerPixel
	fileSize := uint32(headerTotal) + pixelDataSize

	// Allocate the output buffer: header + pixel data.
	buf := make([]byte, fileSize)
	le := binary.LittleEndian

	// ── BITMAPFILEHEADER ────────────────────────────────────────────────────
	// Magic "BM" identifies the file as a BMP.
	buf[0] = 0x42 // 'B'
	buf[1] = 0x4D // 'M'
	le.PutUint32(buf[2:6], fileSize)
	// Bytes 6–9: bfReserved1 and bfReserved2 — always zero (already zero).
	le.PutUint32(buf[10:14], uint32(headerTotal))

	// ── BITMAPINFOHEADER ────────────────────────────────────────────────────
	le.PutUint32(buf[14:18], uint32(infoHeaderSize)) // biSize
	le.PutUint32(buf[18:22], w)                      // biWidth
	// A negative biHeight signals top-down row order. We cast to int32, negate,
	// then reinterpret as uint32 to write into the little-endian slot.
	le.PutUint32(buf[22:26], uint32(-int32(h))) // biHeight (negative = top-down)
	le.PutUint16(buf[26:28], 1)                 // biPlanes
	le.PutUint16(buf[28:30], bitCount32)        // biBitCount
	// biCompression = 0, biSizeImage = 0, X/Y pels, ClrUsed, ClrImportant are
	// all zero — Go zeroed the buffer at allocation, so nothing more to write.

	// ── Pixel data ──────────────────────────────────────────────────────────
	// Write pixels in BGRA order (BMP convention) row by row, top-down.
	dst := buf[headerTotal:]
	for y := uint32(0); y < h; y++ {
		for x := uint32(0); x < w; x++ {
			r, g, b, a := pc.PixelAt(c, x, y)
			i := (y*w + x) * bytesPerPixel
			dst[i+0] = b // Blue first — BMP channel order
			dst[i+1] = g
			dst[i+2] = r // Red last in BGRA
			dst[i+3] = a
		}
	}

	return buf
}

// DecodeBmp parses BMP bytes into a PixelContainer.
//
// Supported variant: 32-bit BI_RGB (no compression). Both top-down
// (negative biHeight) and bottom-up (positive biHeight) files are handled.
//
// Returns an error if:
//   - The file is shorter than the 54-byte header.
//   - The magic number is not "BM".
//   - The pixel format is not 32-bit BI_RGB.
//   - The pixel data region is truncated.
func DecodeBmp(data []byte) (*pc.PixelContainer, error) {
	// ── Validate minimum header size ────────────────────────────────────────
	if len(data) < headerTotal {
		return nil, errors.New("imagecodecbmp: file too short to contain a BMP header")
	}

	le := binary.LittleEndian

	// ── BITMAPFILEHEADER ────────────────────────────────────────────────────
	// Check magic number "BM".
	if data[0] != 0x42 || data[1] != 0x4D {
		return nil, fmt.Errorf("imagecodecbmp: bad magic %02x%02x, want 424d", data[0], data[1])
	}
	pixelDataOffset := le.Uint32(data[10:14])

	// ── BITMAPINFOHEADER ────────────────────────────────────────────────────
	// biSize tells us which version of the info header this is. We only require
	// that it is at least 40 bytes (BITMAPINFOHEADER).
	biSize := le.Uint32(data[14:18])
	if biSize < uint32(infoHeaderSize) {
		return nil, fmt.Errorf("imagecodecbmp: unsupported info header size %d", biSize)
	}

	biWidth := le.Uint32(data[18:22])
	// biHeight is a signed 32-bit integer. Negative means top-down.
	biHeightRaw := int32(le.Uint32(data[22:26]))
	biPlanes := le.Uint16(data[26:28])
	biBitCount := le.Uint16(data[28:30])
	biCompression := le.Uint32(data[30:34])

	if biPlanes != 1 {
		return nil, fmt.Errorf("imagecodecbmp: biPlanes must be 1, got %d", biPlanes)
	}
	if biBitCount != bitCount32 {
		return nil, fmt.Errorf("imagecodecbmp: only 32-bit BMP supported, got %d-bit", biBitCount)
	}
	if biCompression != biRGB {
		return nil, fmt.Errorf("imagecodecbmp: only BI_RGB (0) compression supported, got %d", biCompression)
	}

	// Extract true height and row-order flag.
	topDown := biHeightRaw < 0
	height := biHeightRaw
	if height < 0 {
		height = -height
	}
	biHeight := uint32(height)
	biWidth32 := biWidth

	// ── Validate pixel data region ──────────────────────────────────────────
	pixelDataSize := biWidth32 * biHeight * bytesPerPixel
	if uint64(pixelDataOffset)+uint64(pixelDataSize) > uint64(len(data)) {
		return nil, errors.New("imagecodecbmp: pixel data region is truncated")
	}

	img := pc.New(biWidth32, biHeight)
	src := data[pixelDataOffset:]

	for y := uint32(0); y < biHeight; y++ {
		// Map file row index to image row index based on storage order.
		// Top-down: file row y maps to image row y.
		// Bottom-up: file row y maps to image row (Height-1-y).
		imgRow := y
		if !topDown {
			imgRow = biHeight - 1 - y
		}

		for x := uint32(0); x < biWidth32; x++ {
			// Source pixel at file row y, column x.
			i := (y*biWidth32 + x) * bytesPerPixel
			b := src[i+0] // Blue is first in BGRA
			g := src[i+1]
			r := src[i+2] // Red is third in BGRA
			a := src[i+3]
			pc.SetPixel(img, x, imgRow, r, g, b, a)
		}
	}

	return img, nil
}

// init registers the codec in the package's byte-pool so callers can look
// it up by MIME type. We expose the pool via a small map.
var codecByMime = map[string]pc.ImageCodec{
	"image/bmp": BmpCodec{},
}

// LookupByMime returns the BmpCodec for "image/bmp", or nil.
// This pattern mirrors how the Go standard library's image package works:
// codecs register themselves and callers look them up.
func LookupByMime(mime string) pc.ImageCodec {
	return codecByMime[mime]
}

// bmpMagic is the two-byte signature at the start of every BMP file.
// Exported so that format-detection code can use it without importing
// the full encoder.
var bmpMagic = []byte{0x42, 0x4D} // "BM"

// IsBmp reports whether the given bytes start with the BMP magic number.
func IsBmp(data []byte) bool {
	return len(data) >= 2 && bytes.Equal(data[:2], bmpMagic)
}
