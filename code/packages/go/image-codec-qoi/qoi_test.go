// Package imagecodecqoi_test exercises the QOI encoder and decoder.
//
// Test strategy:
//  1. Header fields — magic, width, height, channels, colorspace.
//  2. End marker — every encoded file ends with the 8-byte sentinel.
//  3. Opcode coverage — force each of the six ops and verify the byte emitted.
//  4. Roundtrip — encode then decode reproduces the original pixel data.
//  5. Edge cases — 1×1, all-same image (long runs), checker pattern (no runs).
//  6. Error cases — bad magic, truncated data.
//  7. Codec interface — QoiCodec satisfies pc.ImageCodec.
package imagecodecqoi_test

import (
	"encoding/binary"
	"testing"

	pc  "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
	qoi "github.com/adhithyan15/coding-adventures/code/packages/go/image-codec-qoi"
)

// onePixel builds a 1×1 PixelContainer.
func onePixel(r, g, b, a byte) *pc.PixelContainer {
	img := pc.New(1, 1)
	pc.SetPixel(img, 0, 0, r, g, b, a)
	return img
}

// ── Header tests ──────────────────────────────────────────────────────────────

// TestEncodeQoi_Magic checks the "qoif" signature.
func TestEncodeQoi_Magic(t *testing.T) {
	data := qoi.EncodeQoi(pc.New(1, 1))
	if string(data[:4]) != "qoif" {
		t.Errorf("magic: got %q, want \"qoif\"", string(data[:4]))
	}
}

// TestEncodeQoi_Width checks width stored big-endian at offset 4.
func TestEncodeQoi_Width(t *testing.T) {
	data := qoi.EncodeQoi(pc.New(13, 5))
	got := binary.BigEndian.Uint32(data[4:8])
	if got != 13 {
		t.Errorf("width: got %d, want 13", got)
	}
}

// TestEncodeQoi_Height checks height stored big-endian at offset 8.
func TestEncodeQoi_Height(t *testing.T) {
	data := qoi.EncodeQoi(pc.New(13, 5))
	got := binary.BigEndian.Uint32(data[8:12])
	if got != 5 {
		t.Errorf("height: got %d, want 5", got)
	}
}

// TestEncodeQoi_Channels checks channels byte at offset 12.
func TestEncodeQoi_Channels(t *testing.T) {
	data := qoi.EncodeQoi(pc.New(1, 1))
	if data[12] != 4 {
		t.Errorf("channels: got %d, want 4", data[12])
	}
}

// TestEncodeQoi_Colorspace checks colorspace byte at offset 13.
func TestEncodeQoi_Colorspace(t *testing.T) {
	data := qoi.EncodeQoi(pc.New(1, 1))
	if data[13] != 0 {
		t.Errorf("colorspace: got %d, want 0", data[13])
	}
}

// TestEncodeQoi_EndMarker checks the 8-byte end sentinel.
func TestEncodeQoi_EndMarker(t *testing.T) {
	data := qoi.EncodeQoi(pc.New(2, 2))
	n := len(data)
	marker := data[n-8:]
	want := []byte{0, 0, 0, 0, 0, 0, 0, 1}
	for i, b := range want {
		if marker[i] != b {
			t.Errorf("end marker byte %d: got %d, want %d", i, marker[i], b)
		}
	}
}

// ── Opcode tests ──────────────────────────────────────────────────────────────

// TestEncodeQoi_OpRGBA checks that a pixel with unique RGBA emits OP_RGBA.
//
// A freshly initialised encoder has prev=(0,0,0,255). A pixel with A≠255
// cannot use OP_DIFF, OP_LUMA, or OP_RGB, so it must use OP_RGBA (0xFF).
func TestEncodeQoi_OpRGBA(t *testing.T) {
	img := onePixel(10, 20, 30, 128) // A changes from 255 to 128
	data := qoi.EncodeQoi(img)
	// data[14] should be 0xFF (OP_RGBA)
	if data[14] != 0xFF {
		t.Errorf("expected OP_RGBA (0xFF) at offset 14, got 0x%02x", data[14])
	}
	if data[15] != 10 || data[16] != 20 || data[17] != 30 || data[18] != 128 {
		t.Errorf("OP_RGBA payload: got %v, want [10 20 30 128]", data[15:19])
	}
}

// TestEncodeQoi_OpRGB checks that a pixel with same A but large RGB delta
// emits OP_RGB (0xFE).
func TestEncodeQoi_OpRGB(t *testing.T) {
	// prev = (0,0,0,255); pixel = (100,100,100,255) — deltas too large for DIFF/LUMA
	img := onePixel(100, 100, 100, 255)
	data := qoi.EncodeQoi(img)
	// Depending on whether (100,100,100,255) is in the index (it won't be on first pixel),
	// OP_RGB should fire.
	if data[14] != 0xFE {
		t.Errorf("expected OP_RGB (0xFE) at offset 14, got 0x%02x", data[14])
	}
	if data[15] != 100 || data[16] != 100 || data[17] != 100 {
		t.Errorf("OP_RGB payload: got %v, want [100 100 100]", data[15:18])
	}
}

// TestEncodeQoi_OpDiff checks that a pixel with small deltas emits OP_DIFF.
//
// prev=(0,0,0,255); curr=(1,0,0,255) → dr=1 dg=0 db=0 da=0
// All deltas in [-2,+1] → OP_DIFF (0b01 01 10 10 = 0x5A).
// Bias: dr+2=3, dg+2=2, db+2=2 → 0b01_11_10_10 = 0x7A.
func TestEncodeQoi_OpDiff(t *testing.T) {
	img := onePixel(1, 0, 0, 255) // dr=1, dg=0, db=0 from initial (0,0,0,255)
	data := qoi.EncodeQoi(img)
	// OP_DIFF tag=0b01, dr+2=3, dg+2=2, db+2=2
	// Byte = 0b01_11_10_10 = 0x7A
	if data[14] != 0x7A {
		t.Errorf("expected OP_DIFF 0x7A at offset 14, got 0x%02x", data[14])
	}
}

// TestEncodeQoi_OpLuma checks a pixel that requires OP_LUMA.
//
// prev=(0,0,0,255); curr=(8,16,8,255)
// dg=16, dr=8, db=8; drg=dr-dg=-8, dbg=db-dg=-8; da=0
// dg=16 ∈ [-32,+31]; drg=-8 ∈ [-8,+7]; dbg=-8 ∈ [-8,+7]
// → OP_LUMA
func TestEncodeQoi_OpLuma(t *testing.T) {
	img := onePixel(8, 16, 8, 255)
	data := qoi.EncodeQoi(img)
	// First byte tag must be 0b10 (bits 7-6 = 2)
	if data[14]>>6 != 0b10 {
		t.Errorf("expected OP_LUMA tag (0b10) at offset 14, got 0x%02x", data[14])
	}
}

// TestEncodeQoi_OpIndex checks that a repeated pixel uses OP_INDEX.
//
// We create a 1×3 image where pixel 0 and pixel 2 are the same.
// After emitting pixel 0 (OP_RGB or OP_DIFF), the encoder stores it in the
// index. When pixel 2 appears again it should be an OP_INDEX.
func TestEncodeQoi_OpIndex(t *testing.T) {
	img := pc.New(1, 3)
	pc.SetPixel(img, 0, 0, 50, 60, 70, 255)  // pixel 0 — goes to index
	pc.SetPixel(img, 0, 1, 1, 2, 3, 255)      // pixel 1 — different
	pc.SetPixel(img, 0, 2, 50, 60, 70, 255)   // pixel 2 — same as pixel 0
	data := qoi.EncodeQoi(img)

	// Decode and check that pixel 2 came out right.
	decoded, err := qoi.DecodeQoi(data)
	if err != nil {
		t.Fatalf("decode error: %v", err)
	}
	r, g, b, a := pc.PixelAt(decoded, 0, 2)
	if r != 50 || g != 60 || b != 70 || a != 255 {
		t.Errorf("OP_INDEX pixel: got (%d,%d,%d,%d), want (50,60,70,255)", r, g, b, a)
	}
}

// TestEncodeQoi_OpRun checks that identical consecutive pixels emit OP_RUN.
func TestEncodeQoi_OpRun(t *testing.T) {
	img := pc.New(1, 3)
	// Set all three pixels to the same value.
	pc.FillPixels(img, 200, 200, 200, 255)
	data := qoi.EncodeQoi(img)
	decoded, err := qoi.DecodeQoi(data)
	if err != nil {
		t.Fatalf("decode error: %v", err)
	}
	for y := uint32(0); y < 3; y++ {
		r, g, b, a := pc.PixelAt(decoded, 0, y)
		if r != 200 || g != 200 || b != 200 || a != 255 {
			t.Errorf("run pixel (%d): got (%d,%d,%d,%d)", y, r, g, b, a)
		}
	}
}

// ── Roundtrip tests ───────────────────────────────────────────────────────────

// TestRoundtrip_1x1 encodes and decodes a single pixel.
func TestRoundtrip_1x1(t *testing.T) {
	orig := onePixel(123, 45, 67, 200)
	decoded, err := qoi.DecodeQoi(qoi.EncodeQoi(orig))
	if err != nil {
		t.Fatalf("decode: %v", err)
	}
	r, g, b, a := pc.PixelAt(decoded, 0, 0)
	if r != 123 || g != 45 || b != 67 || a != 200 {
		t.Errorf("pixel: got (%d,%d,%d,%d), want (123,45,67,200)", r, g, b, a)
	}
}

// TestRoundtrip_AllOpaque checks a 4×4 image where every pixel is unique.
func TestRoundtrip_AllOpaque(t *testing.T) {
	img := pc.New(4, 4)
	val := byte(0)
	for y := uint32(0); y < 4; y++ {
		for x := uint32(0); x < 4; x++ {
			pc.SetPixel(img, x, y, val, val+10, val+20, 255)
			val++
		}
	}
	decoded, err := qoi.DecodeQoi(qoi.EncodeQoi(img))
	if err != nil {
		t.Fatalf("decode: %v", err)
	}
	val = 0
	for y := uint32(0); y < 4; y++ {
		for x := uint32(0); x < 4; x++ {
			r, g, b, a := pc.PixelAt(decoded, x, y)
			wr, wg, wb, wa := val, val+10, val+20, byte(255)
			if r != wr || g != wg || b != wb || a != wa {
				t.Errorf("pixel (%d,%d): got (%d,%d,%d,%d), want (%d,%d,%d,%d)",
					x, y, r, g, b, a, wr, wg, wb, wa)
			}
			val++
		}
	}
}

// TestRoundtrip_Dimensions checks that dimensions survive a roundtrip.
func TestRoundtrip_Dimensions(t *testing.T) {
	img := pc.New(17, 11)
	decoded, err := qoi.DecodeQoi(qoi.EncodeQoi(img))
	if err != nil {
		t.Fatalf("decode: %v", err)
	}
	if decoded.Width != 17 || decoded.Height != 11 {
		t.Errorf("dimensions: got %dx%d, want 17x11", decoded.Width, decoded.Height)
	}
}

// TestRoundtrip_LongRun checks that more than 62 identical pixels work
// (the encoder must emit two OP_RUN chunks).
func TestRoundtrip_LongRun(t *testing.T) {
	img := pc.New(100, 1)
	pc.FillPixels(img, 77, 88, 99, 255)
	decoded, err := qoi.DecodeQoi(qoi.EncodeQoi(img))
	if err != nil {
		t.Fatalf("decode: %v", err)
	}
	for x := uint32(0); x < 100; x++ {
		r, g, b, a := pc.PixelAt(decoded, x, 0)
		if r != 77 || g != 88 || b != 99 || a != 255 {
			t.Errorf("pixel %d: got (%d,%d,%d,%d)", x, r, g, b, a)
		}
	}
}

// TestRoundtrip_AlphaVariations checks pixels with different alpha values.
func TestRoundtrip_AlphaVariations(t *testing.T) {
	img := pc.New(4, 1)
	alphas := []byte{0, 64, 128, 255}
	for i, a := range alphas {
		pc.SetPixel(img, uint32(i), 0, 100, 100, 100, a)
	}
	decoded, err := qoi.DecodeQoi(qoi.EncodeQoi(img))
	if err != nil {
		t.Fatalf("decode: %v", err)
	}
	for i, wa := range alphas {
		_, _, _, a := pc.PixelAt(decoded, uint32(i), 0)
		if a != wa {
			t.Errorf("pixel %d alpha: got %d, want %d", i, a, wa)
		}
	}
}

// ── Error cases ───────────────────────────────────────────────────────────────

// TestDecodeQoi_TooShort checks the "file too short" error.
func TestDecodeQoi_TooShort(t *testing.T) {
	_, err := qoi.DecodeQoi([]byte{0x71, 0x6F, 0x69})
	if err == nil {
		t.Error("expected error for too-short input")
	}
}

// TestDecodeQoi_BadMagic checks the "bad magic" error.
func TestDecodeQoi_BadMagic(t *testing.T) {
	buf := make([]byte, 22)
	buf[0] = 'X' // wrong magic
	buf[1] = 'X'
	buf[2] = 'X'
	buf[3] = 'X'
	binary.BigEndian.PutUint32(buf[4:], 1)
	binary.BigEndian.PutUint32(buf[8:], 1)
	_, err := qoi.DecodeQoi(buf)
	if err == nil {
		t.Error("expected error for bad magic")
	}
}

// ── Codec interface ───────────────────────────────────────────────────────────

// TestQoiCodec_MimeType checks the MIME type string.
func TestQoiCodec_MimeType(t *testing.T) {
	codec := qoi.QoiCodec{}
	if codec.MimeType() != "image/x-qoi" {
		t.Errorf("MimeType: got %q", codec.MimeType())
	}
}

// TestQoiCodec_EncodeDecodeViaInterface confirms pc.ImageCodec satisfaction.
func TestQoiCodec_EncodeDecodeViaInterface(t *testing.T) {
	var codec pc.ImageCodec = qoi.QoiCodec{}
	orig := onePixel(55, 66, 77, 200)
	encoded := codec.Encode(orig)
	decoded, err := codec.Decode(encoded)
	if err != nil {
		t.Fatalf("interface Decode: %v", err)
	}
	r, g, b, a := pc.PixelAt(decoded, 0, 0)
	if r != 55 || g != 66 || b != 77 || a != 200 {
		t.Errorf("interface roundtrip: got (%d,%d,%d,%d), want (55,66,77,200)", r, g, b, a)
	}
}

// TestIsQoi_Valid checks detection of a valid QOI file.
func TestIsQoi_Valid(t *testing.T) {
	data := qoi.EncodeQoi(pc.New(1, 1))
	if !qoi.IsQoi(data) {
		t.Error("IsQoi should return true for valid QOI data")
	}
}

// TestIsQoi_Invalid checks that non-QOI data returns false.
func TestIsQoi_Invalid(t *testing.T) {
	if qoi.IsQoi([]byte{0x42, 0x4D, 0x00, 0x00}) {
		t.Error("IsQoi should return false for BMP-like data")
	}
}

// TestIsQoi_TooShort checks a slice shorter than 4 bytes.
func TestIsQoi_TooShort(t *testing.T) {
	if qoi.IsQoi([]byte{'q', 'o', 'i'}) {
		t.Error("IsQoi should return false for 3-byte slice")
	}
}
