// Package imagecodecqoi encodes and decodes images in the QOI format.
//
// # What Is QOI?
//
// QOI (Quite OK Image) was designed by Dominic Szablewski and published in
// 2021. The spec is 330 lines. Despite its simplicity, QOI compresses most
// photographic images to within 10–20% of PNG while being 20–50× faster to
// encode and decode. It achieves this through five cheap operations that cover
// the common patterns found in pixel data.
//
// # The Five Compression Operations
//
// QOI is a streaming encoder: it walks pixels left-to-right, top-to-bottom
// and emits the cheapest operation that describes the current pixel relative
// to the previous one. It keeps two pieces of state:
//
//  1. A 64-slot index table, indexed by a hash of (R, G, B, A). If the new
//     pixel matches an entry in the table, we emit an index reference.
//  2. The previous pixel (prev). Differences from prev drive the other ops.
//
// The six opcodes (including two non-compressing ones) are:
//
//	┌──────────┬───────────────────────────────────────────────────────────┐
//	│ Opcode   │ Meaning                                                   │
//	├──────────┼───────────────────────────────────────────────────────────┤
//	│ OP_INDEX │ 0b00xxxxxx — pixel is in the index at slot xxxxxx         │
//	│ OP_DIFF  │ 0b01rrggbb — small deltas in R,G,B (–2…+1 each, biased)  │
//	│ OP_LUMA  │ 0b10gggggg + 1 byte — medium G delta; R and B relative   │
//	│ OP_RUN   │ 0b11rrrrrr — repeat prev pixel (rrrrrr+1) times          │
//	│ OP_RGB   │ 0xFE byte — full R,G,B bytes follow; A unchanged         │
//	│ OP_RGBA  │ 0xFF byte — full R,G,B,A bytes follow                    │
//	└──────────┴───────────────────────────────────────────────────────────┘
//
// # File Layout
//
//	Offset  Size  Field
//	------  ----  -----
//	0       4     Magic "qoif"
//	4       4     Width (big-endian uint32)
//	8       4     Height (big-endian uint32)
//	12      1     Channels (3=RGB, 4=RGBA)
//	13      1     Colorspace (0=sRGB+linear-alpha, 1=all-linear)
//	14      …     Compressed pixel data
//	-8      8     End marker: 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x01
//
// This implementation always writes channels=4 (RGBA) and colorspace=0 (sRGB).
package imagecodecqoi

import (
	"encoding/binary"
	"errors"
	"fmt"

	pc "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
)

// ── Opcode constants ──────────────────────────────────────────────────────────

const (
	// opRGB is the two-high-bit-clear marker for a full RGB pixel.
	// The byte 0xFE is followed by three bytes: R, G, B.
	opRGB = 0xFE

	// opRGBA is the marker for a full RGBA pixel.
	// The byte 0xFF is followed by four bytes: R, G, B, A.
	opRGBA = 0xFF

	// tagIndex is the 2-bit tag for OP_INDEX: bits 7–6 are 0b00.
	tagIndex = 0b00

	// tagDiff is the 2-bit tag for OP_DIFF: bits 7–6 are 0b01.
	tagDiff = 0b01

	// tagLuma is the 2-bit tag for OP_LUMA: bits 7–6 are 0b10.
	tagLuma = 0b10

	// tagRun is the 2-bit tag for OP_RUN: bits 7–6 are 0b11.
	tagRun = 0b11
)

// endMarker is the 8-byte sequence that terminates every QOI stream.
var endMarker = []byte{0, 0, 0, 0, 0, 0, 0, 1}

// qoiMagic is the 4-byte file signature.
var qoiMagic = []byte{'q', 'o', 'i', 'f'}

// headerSize is the number of bytes in the QOI file header.
const headerSize = 14

// ── Hash function ─────────────────────────────────────────────────────────────

// qoiHash computes the 6-bit index into the running colour table for a pixel.
//
// The formula is straight from the QOI spec:
//
//	index = (r*3 + g*5 + b*7 + a*11) % 64
//
// Why these multipliers? They are all small odd primes, which gives a good
// pseudo-random spread across the 64 slots while being trivially fast to
// compute. No branch, no division — just four multiplications and a bitmask.
//
// Because we use byte arithmetic, the multiplication wraps around modulo 256
// before we take mod 64. This is equivalent to the spec (the spec works on
// the same 8-bit values).
func qoiHash(r, g, b, a byte) int {
	return int(r*3+g*5+b*7+a*11) % 64
}

// ── Signed-delta helper ───────────────────────────────────────────────────────

// wrap interprets a raw difference (0–255) as a signed int in [–128, 127].
//
// When we subtract two byte values in Go, the result is an int. The signed
// interpretation matters because:
//
//	newR - prevR might be -1 (e.g. 10 - 11 = -1)
//
// Without wrapping, subtracting two bytes using plain int subtraction already
// gives the correct signed value when both values are in [0,255]. However, for
// OP_DIFF we need to handle the wrap-around cases where, say, R goes from 254
// to 1 (delta should be +3, not -253). The wrap function handles that:
//
//	wrap(delta) = ((delta & 0xFF) + 128) & 0xFF - 128
//
// This takes the bottom 8 bits, maps 0→-128, 127→-1, 128→0, 255→127 via
// the +128 trick, then shifts back. In effect it re-centres the range from
// [0,255] to [–128,127].
func wrap(delta int) int {
	return ((delta&0xFF)+128)&0xFF - 128
}

// ── rgba helper ───────────────────────────────────────────────────────────────

// rgba is a tiny struct to hold one pixel's channels. Using a struct avoids
// passing four byte arguments everywhere.
type rgba struct{ r, g, b, a byte }

// ── QoiCodec ─────────────────────────────────────────────────────────────────

// QoiCodec implements pc.ImageCodec for the QOI format.
type QoiCodec struct{}

// MimeType returns the (unofficial) IANA-style media type for QOI.
func (QoiCodec) MimeType() string { return "image/x-qoi" }

// Encode serialises the pixel buffer to QOI bytes.
func (QoiCodec) Encode(c *pc.PixelContainer) []byte { return EncodeQoi(c) }

// Decode parses QOI bytes into a PixelContainer.
func (QoiCodec) Decode(data []byte) (*pc.PixelContainer, error) { return DecodeQoi(data) }

// ── Encoder ───────────────────────────────────────────────────────────────────

// EncodeQoi encodes a PixelContainer to QOI bytes.
//
// Algorithm overview:
//
//  1. Write the 14-byte header.
//  2. Initialise: prev pixel = (0,0,0,255), run length = 0, index table = all zeros.
//  3. For each pixel in raster order:
//     a. If this pixel equals prev, increment run. Flush the run as OP_RUN when
//        the counter reaches 62 (the maximum) or we reach the last pixel.
//     b. Otherwise, flush any pending run, then update the index table, then
//        emit the cheapest applicable opcode.
//  4. Write the 8-byte end marker.
//
// Opcode selection priority (cheapest first):
//
//	OP_INDEX (1 byte)  if the pixel is already in the index
//	OP_DIFF  (1 byte)  if ΔR, ΔG, ΔB ∈ [-2,+1] and ΔA == 0
//	OP_LUMA  (2 bytes) if ΔG ∈ [-32,+31] and (ΔR-ΔG),(ΔB-ΔG) ∈ [-8,+7] and ΔA == 0
//	OP_RGB   (4 bytes) if A is unchanged
//	OP_RGBA  (5 bytes) always works
func EncodeQoi(c *pc.PixelContainer) []byte {
	w, h := c.Width, c.Height

	// Pre-allocate a buffer large enough for the worst case: every pixel
	// encoded as OP_RGBA (5 bytes) plus header (14) plus end marker (8).
	buf := make([]byte, 0, headerSize+int(w)*int(h)*5+len(endMarker))

	// ── Header ───────────────────────────────────────────────────────────────
	buf = append(buf, qoiMagic...)
	buf = binary.BigEndian.AppendUint32(buf, w)
	buf = binary.BigEndian.AppendUint32(buf, h)
	buf = append(buf, 4)    // channels = 4 (RGBA)
	buf = append(buf, 0)    // colorspace = 0 (sRGB + linear alpha)

	// ── Streaming encoder state ──────────────────────────────────────────────
	var index [64]rgba              // running colour lookup table
	prev := rgba{0, 0, 0, 255}    // previous pixel; starts as opaque black
	run := 0                        // current run-length (0 = no active run)
	pixelCount := int(w) * int(h)

	for i := 0; i < pixelCount; i++ {
		x := uint32(i % int(w))
		y := uint32(i / int(w))
		r, g, b, a := pc.PixelAt(c, x, y)
		curr := rgba{r, g, b, a}

		if curr == prev {
			// ── OP_RUN ────────────────────────────────────────────────────
			// This pixel is identical to the last one — extend the run.
			run++
			// Flush when the run counter hits its 6-bit maximum (62) or
			// when this is the very last pixel.
			if run == 62 || i == pixelCount-1 {
				// OP_RUN byte: tag 0b11 in the two high bits, then (run-1)
				// in the six low bits. We subtract 1 so that the value 0
				// encodes a run of 1, and 61 encodes a run of 62.
				buf = append(buf, byte(tagRun<<6|(run-1)))
				run = 0
			}
			continue
		}

		// Flush any pending run before emitting a different pixel.
		if run > 0 {
			buf = append(buf, byte(tagRun<<6|(run-1)))
			run = 0
		}

		// Update the colour index with the current pixel.
		slot := qoiHash(curr.r, curr.g, curr.b, curr.a)
		indexHit := index[slot] == curr
		index[slot] = curr

		if indexHit {
			// ── OP_INDEX ──────────────────────────────────────────────────
			// The pixel already lives in the index. Emit its slot number.
			// One byte: tag 0b00 in bits 7–6, then the 6-bit slot index.
			buf = append(buf, byte(tagIndex<<6|slot))
		} else {
			// Compute signed deltas for the DIFF and LUMA tests.
			dr := wrap(int(curr.r) - int(prev.r))
			dg := wrap(int(curr.g) - int(prev.g))
			db := wrap(int(curr.b) - int(prev.b))
			da := wrap(int(curr.a) - int(prev.a))

			if da == 0 && dr >= -2 && dr <= 1 && dg >= -2 && dg <= 1 && db >= -2 && db <= 1 {
				// ── OP_DIFF ───────────────────────────────────────────────
				// All three colour deltas fit in 2 bits (biased by +2 so
				// that –2→0, –1→1, 0→2, +1→3).
				// Bit layout: 0b 01 rr gg bb
				//                  ^tag  ^ each pair is a 2-bit biased delta
				buf = append(buf, byte(tagDiff<<6|
					(dr+2)<<4|
					(dg+2)<<2|
					(db+2)))
			} else {
				// For OP_LUMA, we encode the green delta directly, and R
				// and B relative to green. This exploits the fact that in
				// natural images, R, G, B changes tend to be correlated.
				drg := dr - dg // R delta relative to G delta
				dbg := db - dg // B delta relative to G delta

				if da == 0 && dg >= -32 && dg <= 31 && drg >= -8 && drg <= 7 && dbg >= -8 && dbg <= 7 {
					// ── OP_LUMA ───────────────────────────────────────────
					// Two bytes:
					//   byte 1: 0b 10 gggggg  (tag + 6-bit biased G delta, bias=32)
					//   byte 2: 0b rrrr bbbb   (4-bit biased drg, bias=8; same for dbg)
					buf = append(buf,
						byte(tagLuma<<6|(dg+32)),
						byte((drg+8)<<4|(dbg+8)),
					)
				} else if da == 0 {
					// ── OP_RGB ────────────────────────────────────────────
					// Alpha is unchanged — just emit the three colour bytes.
					buf = append(buf, opRGB, curr.r, curr.g, curr.b)
				} else {
					// ── OP_RGBA ───────────────────────────────────────────
					// All four channels must be written.
					buf = append(buf, opRGBA, curr.r, curr.g, curr.b, curr.a)
				}
			}
		}

		prev = curr
	}

	// ── End marker ────────────────────────────────────────────────────────────
	buf = append(buf, endMarker...)
	return buf
}

// ── Decoder ───────────────────────────────────────────────────────────────────

// DecodeQoi decodes QOI bytes into a PixelContainer.
//
// The decoder reads opcodes one by one, updating the same state the encoder
// maintained (prev pixel, colour index, run). It stops after decoding
// width * height pixels.
func DecodeQoi(data []byte) (*pc.PixelContainer, error) {
	// ── Validate header ───────────────────────────────────────────────────────
	if len(data) < headerSize {
		return nil, errors.New("imagecodecqoi: file too short to contain a QOI header")
	}

	if data[0] != 'q' || data[1] != 'o' || data[2] != 'i' || data[3] != 'f' {
		return nil, fmt.Errorf("imagecodecqoi: bad magic %q, want \"qoif\"", string(data[:4]))
	}

	w := binary.BigEndian.Uint32(data[4:8])
	h := binary.BigEndian.Uint32(data[8:12])
	// channels := data[12]  // 3 or 4 — we always decode as RGBA
	// colorspace := data[13] // informational only; no processing needed

	pixelCount := int(w) * int(h)
	img := pc.New(w, h)

	// ── Decoder state ─────────────────────────────────────────────────────────
	var index [64]rgba
	prev := rgba{0, 0, 0, 255} // same initial state as the encoder
	run := 0
	pos := headerSize
	i := 0 // pixel counter

	for i < pixelCount {
		if run > 0 {
			// We are in the middle of a run — emit the previous pixel again.
			run--
			x := uint32(i % int(w))
			y := uint32(i / int(w))
			pc.SetPixel(img, x, y, prev.r, prev.g, prev.b, prev.a)
			i++
			continue
		}

		if pos >= len(data) {
			return nil, errors.New("imagecodecqoi: unexpected end of data")
		}

		b0 := data[pos]
		pos++

		var curr rgba

		if b0 == opRGBA {
			// ── OP_RGBA ───────────────────────────────────────────────────
			if pos+4 > len(data) {
				return nil, errors.New("imagecodecqoi: truncated OP_RGBA")
			}
			curr = rgba{data[pos], data[pos+1], data[pos+2], data[pos+3]}
			pos += 4
		} else if b0 == opRGB {
			// ── OP_RGB ────────────────────────────────────────────────────
			if pos+3 > len(data) {
				return nil, errors.New("imagecodecqoi: truncated OP_RGB")
			}
			curr = rgba{data[pos], data[pos+1], data[pos+2], prev.a}
			pos += 3
		} else {
			// Determine the 2-bit tag from the two most-significant bits.
			tag := int(b0 >> 6)

			switch tag {
			case tagIndex:
				// ── OP_INDEX ──────────────────────────────────────────────
				// Lower 6 bits are the slot.
				slot := int(b0 & 0x3F)
				curr = index[slot]

			case tagDiff:
				// ── OP_DIFF ───────────────────────────────────────────────
				// Unpack 2-bit biased deltas from a single byte.
				// Bias is 2: stored value 0 → delta –2, stored value 3 → delta +1.
				dr := int((b0>>4)&0x03) - 2
				dg := int((b0>>2)&0x03) - 2
				db := int(b0&0x03) - 2
				curr = rgba{
					byte(int(prev.r) + dr),
					byte(int(prev.g) + dg),
					byte(int(prev.b) + db),
					prev.a,
				}

			case tagLuma:
				// ── OP_LUMA ───────────────────────────────────────────────
				// First byte contains the 6-bit biased G delta.
				// Second byte contains the 4-bit biased (dr–dg) and (db–dg).
				if pos >= len(data) {
					return nil, errors.New("imagecodecqoi: truncated OP_LUMA")
				}
				b1 := data[pos]
				pos++

				dg := int(b0&0x3F) - 32           // 6-bit bias = 32
				drg := int((b1>>4)&0x0F) - 8       // 4-bit bias = 8; R relative to G
				dbg := int(b1&0x0F) - 8            // 4-bit bias = 8; B relative to G
				curr = rgba{
					byte(int(prev.r) + dg + drg),
					byte(int(prev.g) + dg),
					byte(int(prev.b) + dg + dbg),
					prev.a,
				}

			case tagRun:
				// ── OP_RUN ────────────────────────────────────────────────
				// Lower 6 bits are (runLength – 1). The first occurrence of
				// the pixel has already been emitted by the encoder (it was
				// counted in the run), so we set the counter to b0&0x3F and
				// then emit prev below.
				run = int(b0 & 0x3F) // will be decremented each iteration
				curr = prev
			}
		}

		// Update the index with the new pixel.
		slot := qoiHash(curr.r, curr.g, curr.b, curr.a)
		index[slot] = curr

		x := uint32(i % int(w))
		y := uint32(i / int(w))
		pc.SetPixel(img, x, y, curr.r, curr.g, curr.b, curr.a)
		i++
		prev = curr
	}

	return img, nil
}

// IsQoi reports whether data starts with the QOI magic number "qoif".
func IsQoi(data []byte) bool {
	return len(data) >= 4 &&
		data[0] == 'q' && data[1] == 'o' && data[2] == 'i' && data[3] == 'f'
}
