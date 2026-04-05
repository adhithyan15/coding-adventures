// Package imagecodecppm encodes and decodes Netpbm P6 (binary PPM) images.
//
// # What Is PPM?
//
// PPM (Portable Pixmap) is part of the Netpbm family of image formats, created
// by Jef Poskanzer in the late 1980s. PPM is deliberately trivial: the entire
// spec fits on one page. There is no compression, no colour tables, and no
// layers. Its simplicity makes it the ideal "lingua franca" for piping images
// between programs in a Unix pipeline.
//
// # P6 Binary Format
//
// There are two PPM variants:
//   - P3 (ASCII): pixel values written as decimal text, one per line. Easy to
//     read with a text editor but slow and bulky.
//   - P6 (binary): pixel values packed as raw bytes. Much more compact and fast.
//
// This package uses P6. The file layout is:
//
//	"P6\n"             ← magic number (2 bytes + newline)
//	"<width> <height>\n"  ← dimensions as ASCII decimal
//	"255\n"            ← maximum value (we always use 8-bit channels)
//	<pixel bytes>      ← Width * Height * 3 bytes, RGB order, row-major
//
// Alpha is not part of the PPM format. When encoding, the alpha channel is
// dropped. When decoding, every pixel gets A=255 (fully opaque).
//
// # Comment Lines
//
// The PPM spec allows comment lines starting with '#' anywhere in the header.
// This decoder skips them during token parsing.
//
// # Parsing Strategy
//
// We use a manual byte-cursor (an integer index into the raw byte slice) rather
// than bufio.Scanner or strings.Split. This keeps the code self-contained and
// easy to follow step-by-step.
package imagecodecppm

import (
	"errors"
	"fmt"
	"strconv"

	pc "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
)

// PpmCodec implements pc.ImageCodec for PPM P6.
//
// Usage:
//
//	var codec pc.ImageCodec = imagecodecppm.PpmCodec{}
//	data := codec.Encode(img)
type PpmCodec struct{}

// MimeType returns the IANA media type for PPM.
func (PpmCodec) MimeType() string { return "image/x-portable-pixmap" }

// Encode serialises the pixel buffer to P6 PPM bytes.
func (PpmCodec) Encode(c *pc.PixelContainer) []byte { return EncodePpm(c) }

// Decode parses P6 PPM bytes into a PixelContainer.
func (PpmCodec) Decode(data []byte) (*pc.PixelContainer, error) { return DecodePpm(data) }

// EncodePpm serialises a PixelContainer to PPM P6 bytes.
//
// The header is written using fmt.Appendf so we do not need an intermediate
// string allocation. The pixel loop copies only R, G, B — alpha is discarded
// because PPM has no alpha channel.
//
// Output structure:
//
//	P6\n
//	<width> <height>\n
//	255\n
//	<R0G0B0 R1G0B1 … row-major RGB bytes>
func EncodePpm(c *pc.PixelContainer) []byte {
	w := c.Width
	h := c.Height

	// Build the ASCII header. fmt.Appendf writes into a growing slice, avoiding
	// a separate string allocation.
	var buf []byte
	buf = fmt.Appendf(buf, "P6\n%d %d\n255\n", w, h)

	// Append raw RGB bytes. We allocate exactly the right amount up front.
	pixelStart := len(buf)
	buf = append(buf, make([]byte, w*h*3)...)

	for y := uint32(0); y < h; y++ {
		for x := uint32(0); x < w; x++ {
			r, g, b, _ := pc.PixelAt(c, x, y)
			i := pixelStart + int((y*w+x)*3)
			buf[i] = r
			buf[i+1] = g
			buf[i+2] = b
		}
	}

	return buf
}

// DecodePpm parses PPM P6 data into a PixelContainer.
//
// The decoder works with a single integer cursor (pos) into the raw byte
// slice. Helper functions advance the cursor token by token.
//
// Parsing steps:
//  1. Read the "P6" magic token.
//  2. Read the width, height, and maxval tokens (skipping '#' comment lines).
//  3. Consume the single whitespace byte that separates the header from the
//     binary pixel data.
//  4. Copy RGB triples into the PixelContainer, setting A=255.
//
// Returns errors for:
//   - Non-P6 magic ("bad magic").
//   - maxval != 255 (we only handle 8-bit PPMs).
//   - Truncated pixel data.
//   - Non-numeric width/height/maxval tokens.
func DecodePpm(data []byte) (*pc.PixelContainer, error) {
	pos := 0

	// ── Token reader helpers ──────────────────────────────────────────────────
	//
	// skipWhitespaceAndComments advances pos past any whitespace characters and
	// over any comment lines (sequences starting with '#' and ending with '\n').
	// PPM allows comments between any two tokens in the header.
	skipWhitespaceAndComments := func() {
		for pos < len(data) {
			ch := data[pos]
			if ch == ' ' || ch == '\t' || ch == '\r' || ch == '\n' {
				pos++
			} else if ch == '#' {
				// Skip the rest of the comment line.
				for pos < len(data) && data[pos] != '\n' {
					pos++
				}
			} else {
				break
			}
		}
	}

	// readToken reads a whitespace-delimited token from the header. It skips
	// leading whitespace and comments, then collects non-whitespace bytes.
	readToken := func() (string, error) {
		skipWhitespaceAndComments()
		start := pos
		for pos < len(data) {
			ch := data[pos]
			if ch == ' ' || ch == '\t' || ch == '\r' || ch == '\n' {
				break
			}
			pos++
		}
		if pos == start {
			return "", errors.New("imagecodecppm: unexpected end of header")
		}
		return string(data[start:pos]), nil
	}

	// ── Step 1: Magic number ─────────────────────────────────────────────────
	magic, err := readToken()
	if err != nil {
		return nil, err
	}
	if magic != "P6" {
		return nil, fmt.Errorf("imagecodecppm: bad magic %q, want \"P6\"", magic)
	}

	// ── Step 2: Width ────────────────────────────────────────────────────────
	widthStr, err := readToken()
	if err != nil {
		return nil, err
	}
	widthVal, err := strconv.ParseUint(widthStr, 10, 32)
	if err != nil {
		return nil, fmt.Errorf("imagecodecppm: invalid width %q: %w", widthStr, err)
	}

	// ── Step 3: Height ───────────────────────────────────────────────────────
	heightStr, err := readToken()
	if err != nil {
		return nil, err
	}
	heightVal, err := strconv.ParseUint(heightStr, 10, 32)
	if err != nil {
		return nil, fmt.Errorf("imagecodecppm: invalid height %q: %w", heightStr, err)
	}

	// ── Step 4: Maxval ───────────────────────────────────────────────────────
	maxvalStr, err := readToken()
	if err != nil {
		return nil, err
	}
	maxvalVal, err := strconv.ParseUint(maxvalStr, 10, 16)
	if err != nil {
		return nil, fmt.Errorf("imagecodecppm: invalid maxval %q: %w", maxvalStr, err)
	}
	if maxvalVal != 255 {
		return nil, fmt.Errorf("imagecodecppm: only maxval=255 supported, got %d", maxvalVal)
	}

	// ── Step 5: Single separator byte ────────────────────────────────────────
	// The PPM spec requires exactly one whitespace character between the maxval
	// token and the start of the binary pixel data.
	if pos >= len(data) {
		return nil, errors.New("imagecodecppm: file ended before pixel data")
	}
	pos++ // consume the separator byte

	// ── Step 6: Pixel data ───────────────────────────────────────────────────
	w := uint32(widthVal)
	h := uint32(heightVal)

	// Reject oversized images before calling pc.New (which would panic) and
	// before computing expectedBytes (which could overflow on 32-bit platforms).
	if w > pc.MaxDimension || h > pc.MaxDimension {
		return nil, fmt.Errorf("imagecodecppm: image dimensions %dx%d exceed maximum", w, h)
	}

	// Use int64 arithmetic so that very large (but still sub-MaxDimension) images
	// cannot overflow int on 32-bit platforms.
	expectedBytes := int64(w) * int64(h) * 3
	if int64(pos)+expectedBytes > int64(len(data)) {
		return nil, errors.New("imagecodecppm: pixel data truncated")
	}

	img := pc.New(w, h)
	src := data[pos:]
	for y := uint32(0); y < h; y++ {
		for x := uint32(0); x < w; x++ {
			i := int((y*w+x) * 3)
			r := src[i]
			g := src[i+1]
			b := src[i+2]
			// PPM has no alpha; fully opaque is the only sensible default.
			pc.SetPixel(img, x, y, r, g, b, 255)
		}
	}

	return img, nil
}

// IsPpm reports whether data starts with the PPM P6 magic number.
//
// This is a quick header check — it does not validate the rest of the file.
func IsPpm(data []byte) bool {
	return len(data) >= 2 && data[0] == 'P' && data[1] == '6'
}
