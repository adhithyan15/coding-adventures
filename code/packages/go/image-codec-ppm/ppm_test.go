// Package imagecodecppm_test exercises the PPM encoder and decoder.
//
// Test strategy:
//  1. Header content — encoded bytes contain exactly the right ASCII header.
//  2. Pixel bytes — RGB order, no alpha.
//  3. Roundtrip — encode then decode reproduces the original data.
//  4. Comment handling — decoder skips '#' lines.
//  5. Error cases — bad magic, wrong maxval, truncated data.
//  6. Codec interface — PpmCodec satisfies pc.ImageCodec.
package imagecodecppm_test

import (
	"bytes"
	"fmt"
	"testing"

	pc  "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
	ppm "github.com/adhithyan15/coding-adventures/code/packages/go/image-codec-ppm"
)

// onePixel builds a 1×1 PixelContainer.
func onePixel(r, g, b, a byte) *pc.PixelContainer {
	img := pc.New(1, 1)
	pc.SetPixel(img, 0, 0, r, g, b, a)
	return img
}

// ── Header tests ──────────────────────────────────────────────────────────────

// TestEncodePpm_StartsWithP6 checks the magic number.
func TestEncodePpm_StartsWithP6(t *testing.T) {
	data := ppm.EncodePpm(pc.New(1, 1))
	if !bytes.HasPrefix(data, []byte("P6\n")) {
		end := 10
		if len(data) < end {
			end = len(data)
		}
		t.Errorf("expected header to start with \"P6\\n\", got %q", data[:end])
	}
}

// TestEncodePpm_DimensionsInHeader checks width and height appear in the header.
func TestEncodePpm_DimensionsInHeader(t *testing.T) {
	data := ppm.EncodePpm(pc.New(12, 34))
	header := string(data[:bytes.Index(data, []byte("255\n"))+4])
	if !bytes.Contains([]byte(header), []byte("12 34")) {
		t.Errorf("header does not contain \"12 34\": %q", header)
	}
}

// TestEncodePpm_Maxval255 checks that "255" appears in the header.
func TestEncodePpm_Maxval255(t *testing.T) {
	data := ppm.EncodePpm(pc.New(1, 1))
	if !bytes.Contains(data, []byte("255\n")) {
		t.Error("encoded PPM should contain \"255\\n\"")
	}
}

// TestEncodePpm_PixelDataLength checks total file length = header + W*H*3.
func TestEncodePpm_PixelDataLength(t *testing.T) {
	w, h := uint32(5), uint32(3)
	data := ppm.EncodePpm(pc.New(w, h))
	header := fmt.Sprintf("P6\n%d %d\n255\n", w, h)
	wantLen := len(header) + int(w*h*3)
	if len(data) != wantLen {
		t.Errorf("file length: got %d, want %d", len(data), wantLen)
	}
}

// ── Pixel byte order ──────────────────────────────────────────────────────────

// TestEncodePpm_RGBOrder checks the raw byte order is R-G-B (no alpha).
func TestEncodePpm_RGBOrder(t *testing.T) {
	img := onePixel(10, 20, 30, 255)
	data := ppm.EncodePpm(img)
	// Find where the binary data starts (after the header).
	header := fmt.Sprintf("P6\n1 1\n255\n")
	off := len(header)
	if data[off] != 10 {
		t.Errorf("R byte: got %d, want 10", data[off])
	}
	if data[off+1] != 20 {
		t.Errorf("G byte: got %d, want 20", data[off+1])
	}
	if data[off+2] != 30 {
		t.Errorf("B byte: got %d, want 30", data[off+2])
	}
}

// TestEncodePpm_AlphaDropped confirms no 4th byte per pixel.
func TestEncodePpm_AlphaDropped(t *testing.T) {
	img := pc.New(1, 1)
	pc.SetPixel(img, 0, 0, 1, 2, 3, 200)
	data := ppm.EncodePpm(img)
	header := fmt.Sprintf("P6\n1 1\n255\n")
	pixelSection := data[len(header):]
	if len(pixelSection) != 3 {
		t.Errorf("pixel section length: got %d, want 3", len(pixelSection))
	}
}

// ── Roundtrip ─────────────────────────────────────────────────────────────────

// TestRoundtrip_1x1 encodes and decodes a single pixel.
func TestRoundtrip_1x1(t *testing.T) {
	orig := onePixel(100, 150, 200, 255)
	decoded, err := ppm.DecodePpm(ppm.EncodePpm(orig))
	if err != nil {
		t.Fatalf("decode: %v", err)
	}
	r, g, b, a := pc.PixelAt(decoded, 0, 0)
	if r != 100 || g != 150 || b != 200 {
		t.Errorf("RGB: got (%d,%d,%d), want (100,150,200)", r, g, b)
	}
	// Alpha should always be 255 after decode.
	if a != 255 {
		t.Errorf("A: got %d, want 255", a)
	}
}

// TestRoundtrip_MultiPixel checks a 3×2 image roundtrip.
func TestRoundtrip_MultiPixel(t *testing.T) {
	img := pc.New(3, 2)
	pixels := [][3]byte{{1, 2, 3}, {4, 5, 6}, {7, 8, 9}, {10, 11, 12}, {13, 14, 15}, {16, 17, 18}}
	for i, p := range pixels {
		pc.SetPixel(img, uint32(i%3), uint32(i/3), p[0], p[1], p[2], 255)
	}
	decoded, err := ppm.DecodePpm(ppm.EncodePpm(img))
	if err != nil {
		t.Fatalf("decode: %v", err)
	}
	for i, p := range pixels {
		x, y := uint32(i%3), uint32(i/3)
		r, g, b, _ := pc.PixelAt(decoded, x, y)
		if r != p[0] || g != p[1] || b != p[2] {
			t.Errorf("pixel (%d,%d): got (%d,%d,%d), want (%d,%d,%d)", x, y, r, g, b, p[0], p[1], p[2])
		}
	}
}

// TestRoundtrip_AlphaForcedTo255 confirms alpha is always 255 after decode.
func TestRoundtrip_AlphaForcedTo255(t *testing.T) {
	img := pc.New(2, 2)
	pc.FillPixels(img, 50, 100, 150, 0) // alpha=0
	decoded, err := ppm.DecodePpm(ppm.EncodePpm(img))
	if err != nil {
		t.Fatalf("decode: %v", err)
	}
	_, _, _, a := pc.PixelAt(decoded, 0, 0)
	if a != 255 {
		t.Errorf("A after roundtrip: got %d, want 255", a)
	}
}

// TestRoundtrip_Dimensions checks that dimensions survive a roundtrip.
func TestRoundtrip_Dimensions(t *testing.T) {
	img := pc.New(9, 7)
	decoded, err := ppm.DecodePpm(ppm.EncodePpm(img))
	if err != nil {
		t.Fatalf("decode: %v", err)
	}
	if decoded.Width != 9 || decoded.Height != 7 {
		t.Errorf("dimensions: got %dx%d, want 9x7", decoded.Width, decoded.Height)
	}
}

// ── Comment handling ──────────────────────────────────────────────────────────

// TestDecodePpm_WithComments verifies that '#' comment lines are skipped.
func TestDecodePpm_WithComments(t *testing.T) {
	data := []byte("P6\n# This is a comment\n2 2\n# Another comment\n255\n\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C")
	img, err := ppm.DecodePpm(data)
	if err != nil {
		t.Fatalf("decode with comments: %v", err)
	}
	if img.Width != 2 || img.Height != 2 {
		t.Errorf("dimensions: got %dx%d, want 2x2", img.Width, img.Height)
	}
	r, g, b, _ := pc.PixelAt(img, 0, 0)
	if r != 1 || g != 2 || b != 3 {
		t.Errorf("first pixel: got (%d,%d,%d), want (1,2,3)", r, g, b)
	}
}

// ── Error cases ───────────────────────────────────────────────────────────────

// TestDecodePpm_BadMagic checks the error for non-P6 magic.
func TestDecodePpm_BadMagic(t *testing.T) {
	_, err := ppm.DecodePpm([]byte("P3\n1 1\n255\n\xff\xff\xff"))
	if err == nil {
		t.Error("expected error for P3 magic")
	}
}

// TestDecodePpm_WrongMaxval checks error for maxval != 255.
func TestDecodePpm_WrongMaxval(t *testing.T) {
	_, err := ppm.DecodePpm([]byte("P6\n1 1\n65535\n\xff\xff\xff\xff\xff\xff"))
	if err == nil {
		t.Error("expected error for maxval=65535")
	}
}

// TestDecodePpm_TruncatedPixelData checks error when pixel data is cut short.
func TestDecodePpm_TruncatedPixelData(t *testing.T) {
	_, err := ppm.DecodePpm([]byte("P6\n10 10\n255\n\x00\x00")) // only 2 bytes for 300 expected
	if err == nil {
		t.Error("expected error for truncated pixel data")
	}
}

// TestDecodePpm_EmptyInput checks error on empty slice.
func TestDecodePpm_EmptyInput(t *testing.T) {
	_, err := ppm.DecodePpm([]byte{})
	if err == nil {
		t.Error("expected error for empty input")
	}
}

// TestDecodePpm_InvalidWidth checks error for non-numeric width.
func TestDecodePpm_InvalidWidth(t *testing.T) {
	_, err := ppm.DecodePpm([]byte("P6\nabc 1\n255\n"))
	if err == nil {
		t.Error("expected error for non-numeric width")
	}
}

// ── Codec interface ───────────────────────────────────────────────────────────

// TestPpmCodec_MimeType checks the MIME type string.
func TestPpmCodec_MimeType(t *testing.T) {
	codec := ppm.PpmCodec{}
	if codec.MimeType() != "image/x-portable-pixmap" {
		t.Errorf("MimeType: got %q", codec.MimeType())
	}
}

// TestPpmCodec_EncodeDecodeViaInterface confirms pc.ImageCodec satisfaction.
func TestPpmCodec_EncodeDecodeViaInterface(t *testing.T) {
	var codec pc.ImageCodec = ppm.PpmCodec{}
	orig := onePixel(33, 66, 99, 255)
	encoded := codec.Encode(orig)
	decoded, err := codec.Decode(encoded)
	if err != nil {
		t.Fatalf("interface Decode: %v", err)
	}
	r, g, b, _ := pc.PixelAt(decoded, 0, 0)
	if r != 33 || g != 66 || b != 99 {
		t.Errorf("interface roundtrip: got (%d,%d,%d), want (33,66,99)", r, g, b)
	}
}

// TestIsPpm_Valid checks detection of a valid PPM header.
func TestIsPpm_Valid(t *testing.T) {
	data := ppm.EncodePpm(pc.New(1, 1))
	if !ppm.IsPpm(data) {
		t.Error("IsPpm should return true for valid PPM data")
	}
}

// TestIsPpm_Invalid checks that non-PPM data is not detected as PPM.
func TestIsPpm_Invalid(t *testing.T) {
	if ppm.IsPpm([]byte{0x42, 0x4D}) {
		t.Error("IsPpm should return false for BMP-like data")
	}
}

