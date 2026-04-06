// Package imagecodecbmp_test exercises the BMP encoder and decoder.
//
// Test strategy:
//  1. Header field tests — verify exact byte values produced by EncodeBmp.
//  2. BGRA swap — confirm R and B are exchanged relative to the PixelContainer.
//  3. Roundtrip — encode then decode reproduces the original pixel data.
//  4. Error cases — DecodeBmp returns descriptive errors for bad input.
//  5. Codec interface — BmpCodec satisfies pc.ImageCodec.
package imagecodecbmp_test

import (
	"encoding/binary"
	"testing"

	bmp "github.com/adhithyan15/coding-adventures/code/packages/go/image-codec-bmp"
	pc "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
)

// helper builds a single-pixel PixelContainer with the given RGBA values.
func onePixel(r, g, b, a byte) *pc.PixelContainer {
	img := pc.New(1, 1)
	pc.SetPixel(img, 0, 0, r, g, b, a)
	return img
}

// ── Header tests ──────────────────────────────────────────────────────────────

// TestEncodeBmp_Magic checks the "BM" signature at bytes 0–1.
func TestEncodeBmp_Magic(t *testing.T) {
	data := bmp.EncodeBmp(pc.New(1, 1))
	if data[0] != 0x42 || data[1] != 0x4D {
		t.Errorf("magic: got %02x%02x, want 424d", data[0], data[1])
	}
}

// TestEncodeBmp_FileSize checks bfSize equals 54 + w*h*4.
func TestEncodeBmp_FileSize(t *testing.T) {
	img := pc.New(3, 5)
	data := bmp.EncodeBmp(img)
	want := uint32(54 + 3*5*4)
	got := binary.LittleEndian.Uint32(data[2:6])
	if got != want {
		t.Errorf("bfSize: got %d, want %d", got, want)
	}
}

// TestEncodeBmp_PixelDataOffset checks bfOffBits == 54.
func TestEncodeBmp_PixelDataOffset(t *testing.T) {
	data := bmp.EncodeBmp(pc.New(4, 4))
	got := binary.LittleEndian.Uint32(data[10:14])
	if got != 54 {
		t.Errorf("bfOffBits: got %d, want 54", got)
	}
}

// TestEncodeBmp_InfoHeaderSize checks biSize == 40.
func TestEncodeBmp_InfoHeaderSize(t *testing.T) {
	data := bmp.EncodeBmp(pc.New(1, 1))
	got := binary.LittleEndian.Uint32(data[14:18])
	if got != 40 {
		t.Errorf("biSize: got %d, want 40", got)
	}
}

// TestEncodeBmp_Width checks biWidth.
func TestEncodeBmp_Width(t *testing.T) {
	img := pc.New(7, 3)
	data := bmp.EncodeBmp(img)
	got := binary.LittleEndian.Uint32(data[18:22])
	if got != 7 {
		t.Errorf("biWidth: got %d, want 7", got)
	}
}

// TestEncodeBmp_HeightNegative checks biHeight is stored as a negative value
// (top-down layout).
func TestEncodeBmp_HeightNegative(t *testing.T) {
	img := pc.New(3, 5)
	data := bmp.EncodeBmp(img)
	// biHeight as signed int32 should be -5.
	got := int32(binary.LittleEndian.Uint32(data[22:26]))
	if got != -5 {
		t.Errorf("biHeight: got %d, want -5", got)
	}
}

// TestEncodeBmp_BitCount checks biBitCount == 32.
func TestEncodeBmp_BitCount(t *testing.T) {
	data := bmp.EncodeBmp(pc.New(2, 2))
	got := binary.LittleEndian.Uint16(data[28:30])
	if got != 32 {
		t.Errorf("biBitCount: got %d, want 32", got)
	}
}

// TestEncodeBmp_Compression checks biCompression == 0 (BI_RGB).
func TestEncodeBmp_Compression(t *testing.T) {
	data := bmp.EncodeBmp(pc.New(2, 2))
	got := binary.LittleEndian.Uint32(data[30:34])
	if got != 0 {
		t.Errorf("biCompression: got %d, want 0", got)
	}
}

// ── BGRA channel order ────────────────────────────────────────────────────────

// TestEncodeBmp_BGRAOrder checks the raw pixel bytes at offset 54 are
// stored in BGRA order (blue first, red third).
func TestEncodeBmp_BGRAOrder(t *testing.T) {
	// Pixel with distinct channel values so we can tell them apart.
	img := onePixel(10, 20, 30, 40) // RGBA: R=10, G=20, B=30, A=40
	data := bmp.EncodeBmp(img)
	// File layout at offset 54: B G R A
	if data[54] != 30 {
		t.Errorf("file[54] (Blue): got %d, want 30", data[54])
	}
	if data[55] != 20 {
		t.Errorf("file[55] (Green): got %d, want 20", data[55])
	}
	if data[56] != 10 {
		t.Errorf("file[56] (Red): got %d, want 10", data[56])
	}
	if data[57] != 40 {
		t.Errorf("file[57] (Alpha): got %d, want 40", data[57])
	}
}

// ── Roundtrip ─────────────────────────────────────────────────────────────────

// TestRoundtrip_1x1 encodes and decodes a single pixel and checks all channels.
func TestRoundtrip_1x1(t *testing.T) {
	orig := onePixel(100, 150, 200, 255)
	decoded, err := bmp.DecodeBmp(bmp.EncodeBmp(orig))
	if err != nil {
		t.Fatalf("decode error: %v", err)
	}
	r, g, b, a := pc.PixelAt(decoded, 0, 0)
	if r != 100 || g != 150 || b != 200 || a != 255 {
		t.Errorf("pixel: got (%d,%d,%d,%d), want (100,150,200,255)", r, g, b, a)
	}
}

// TestRoundtrip_MultiPixel checks multiple distinct pixels survive a roundtrip.
func TestRoundtrip_MultiPixel(t *testing.T) {
	img := pc.New(3, 2)
	pixels := [][4]byte{
		{1, 2, 3, 4}, {5, 6, 7, 8}, {9, 10, 11, 12},
		{13, 14, 15, 16}, {17, 18, 19, 20}, {21, 22, 23, 24},
	}
	for i, p := range pixels {
		x := uint32(i % 3)
		y := uint32(i / 3)
		pc.SetPixel(img, x, y, p[0], p[1], p[2], p[3])
	}
	decoded, err := bmp.DecodeBmp(bmp.EncodeBmp(img))
	if err != nil {
		t.Fatalf("decode error: %v", err)
	}
	for i, p := range pixels {
		x := uint32(i % 3)
		y := uint32(i / 3)
		r, g, b, a := pc.PixelAt(decoded, x, y)
		if r != p[0] || g != p[1] || b != p[2] || a != p[3] {
			t.Errorf("pixel (%d,%d): got (%d,%d,%d,%d), want (%d,%d,%d,%d)",
				x, y, r, g, b, a, p[0], p[1], p[2], p[3])
		}
	}
}

// TestRoundtrip_Dimensions checks width and height survive a roundtrip.
func TestRoundtrip_Dimensions(t *testing.T) {
	img := pc.New(11, 7)
	decoded, err := bmp.DecodeBmp(bmp.EncodeBmp(img))
	if err != nil {
		t.Fatalf("decode error: %v", err)
	}
	if decoded.Width != 11 || decoded.Height != 7 {
		t.Errorf("dimensions: got %dx%d, want 11x7", decoded.Width, decoded.Height)
	}
}

// TestRoundtrip_AllChannelsMax checks that 255 in all channels roundtrips.
func TestRoundtrip_AllChannelsMax(t *testing.T) {
	img := onePixel(255, 255, 255, 255)
	decoded, err := bmp.DecodeBmp(bmp.EncodeBmp(img))
	if err != nil {
		t.Fatalf("decode error: %v", err)
	}
	r, g, b, a := pc.PixelAt(decoded, 0, 0)
	if r != 255 || g != 255 || b != 255 || a != 255 {
		t.Errorf("all-max: got (%d,%d,%d,%d), want (255,255,255,255)", r, g, b, a)
	}
}

// ── Bottom-up decode ─────────────────────────────────────────────────────────

// TestDecodeBmp_BottomUp constructs a hand-crafted bottom-up BMP and verifies
// that the decoder correctly flips the rows.
func TestDecodeBmp_BottomUp(t *testing.T) {
	// Build a minimal 1×2 bottom-up BMP by hand.
	// bottom-up: file row 0 = image bottom row (y=1), file row 1 = image top row (y=0).
	w, h := uint32(1), uint32(2)
	pixelDataSize := w * h * 4
	fileSize := uint32(54) + pixelDataSize
	buf := make([]byte, fileSize)
	le := binary.LittleEndian

	buf[0], buf[1] = 0x42, 0x4D
	le.PutUint32(buf[2:], fileSize)
	le.PutUint32(buf[10:], 54)
	le.PutUint32(buf[14:], 40)
	le.PutUint32(buf[18:], w)
	le.PutUint32(buf[22:], h) // positive biHeight → bottom-up
	le.PutUint16(buf[26:], 1)
	le.PutUint16(buf[28:], 32)
	// biCompression = 0 (already zero)

	// File row 0 (bottom of image, y=1 in top-down coords): BGRA=(0,255,0,255) → G pixel
	buf[54] = 0   // B
	buf[55] = 255 // G
	buf[56] = 0   // R
	buf[57] = 255 // A
	// File row 1 (top of image, y=0 in top-down coords): BGRA=(255,0,0,255) → R pixel
	buf[58] = 0   // B
	buf[59] = 0   // G
	buf[60] = 255 // R
	buf[61] = 255 // A

	img, err := bmp.DecodeBmp(buf)
	if err != nil {
		t.Fatalf("decode error: %v", err)
	}
	// y=0 (top) should be the red pixel (from file row 1).
	r0, g0, b0, _ := pc.PixelAt(img, 0, 0)
	if r0 != 255 || g0 != 0 || b0 != 0 {
		t.Errorf("top pixel: got R=%d G=%d B=%d, want R=255 G=0 B=0", r0, g0, b0)
	}
	// y=1 (bottom) should be the green pixel (from file row 0).
	r1, g1, b1, _ := pc.PixelAt(img, 0, 1)
	if r1 != 0 || g1 != 255 || b1 != 0 {
		t.Errorf("bottom pixel: got R=%d G=%d B=%d, want R=0 G=255 B=0", r1, g1, b1)
	}
}

// ── Error cases ───────────────────────────────────────────────────────────────

// TestDecodeBmp_TooShort checks the "file too short" error.
func TestDecodeBmp_TooShort(t *testing.T) {
	_, err := bmp.DecodeBmp([]byte{0x42, 0x4D})
	if err == nil {
		t.Error("expected error for too-short input")
	}
}

// TestDecodeBmp_BadMagic checks the "bad magic" error.
func TestDecodeBmp_BadMagic(t *testing.T) {
	buf := make([]byte, 54)
	buf[0], buf[1] = 0xFF, 0xFF
	_, err := bmp.DecodeBmp(buf)
	if err == nil {
		t.Error("expected error for bad magic bytes")
	}
}

// TestDecodeBmp_Not32Bit checks the error for non-32-bit BMP.
func TestDecodeBmp_Not32Bit(t *testing.T) {
	img := pc.New(2, 2)
	data := bmp.EncodeBmp(img)
	binary.LittleEndian.PutUint16(data[28:30], 24) // change to 24-bit
	_, err := bmp.DecodeBmp(data)
	if err == nil {
		t.Error("expected error for 24-bit BMP")
	}
}

// TestDecodeBmp_NonZeroCompression checks error for unsupported compression.
func TestDecodeBmp_NonZeroCompression(t *testing.T) {
	img := pc.New(2, 2)
	data := bmp.EncodeBmp(img)
	binary.LittleEndian.PutUint32(data[30:34], 1) // BI_RLE8
	_, err := bmp.DecodeBmp(data)
	if err == nil {
		t.Error("expected error for non-BI_RGB compression")
	}
}

// TestDecodeBmp_TruncatedPixelData checks error when pixel data is cut short.
func TestDecodeBmp_TruncatedPixelData(t *testing.T) {
	img := pc.New(4, 4)
	data := bmp.EncodeBmp(img)
	_, err := bmp.DecodeBmp(data[:60]) // truncate after a few pixel bytes
	if err == nil {
		t.Error("expected error for truncated pixel data")
	}
}

// ── Codec interface ───────────────────────────────────────────────────────────

// TestBmpCodec_MimeType checks the MIME type string.
func TestBmpCodec_MimeType(t *testing.T) {
	codec := bmp.BmpCodec{}
	if codec.MimeType() != "image/bmp" {
		t.Errorf("MimeType: got %q, want \"image/bmp\"", codec.MimeType())
	}
}

// TestBmpCodec_EncodeDecodeViaInterface confirms that the codec satisfies
// pc.ImageCodec and that Encode/Decode work end-to-end through the interface.
func TestBmpCodec_EncodeDecodeViaInterface(t *testing.T) {
	var codec pc.ImageCodec = bmp.BmpCodec{}
	orig := onePixel(77, 88, 99, 111)
	encoded := codec.Encode(orig)
	decoded, err := codec.Decode(encoded)
	if err != nil {
		t.Fatalf("interface Decode: %v", err)
	}
	r, g, b, a := pc.PixelAt(decoded, 0, 0)
	if r != 77 || g != 88 || b != 99 || a != 111 {
		t.Errorf("interface roundtrip: got (%d,%d,%d,%d), want (77,88,99,111)", r, g, b, a)
	}
}

// TestIsBmp_Valid checks detection of a valid BMP header.
func TestIsBmp_Valid(t *testing.T) {
	data := bmp.EncodeBmp(pc.New(1, 1))
	if !bmp.IsBmp(data) {
		t.Error("IsBmp should return true for valid BMP data")
	}
}

// TestIsBmp_Invalid checks that non-BMP data is not detected as BMP.
func TestIsBmp_Invalid(t *testing.T) {
	if bmp.IsBmp([]byte{0x00, 0x00}) {
		t.Error("IsBmp should return false for non-BMP data")
	}
}

// TestIsBmp_TooShort checks that a slice shorter than 2 bytes returns false.
func TestIsBmp_TooShort(t *testing.T) {
	if bmp.IsBmp([]byte{0x42}) {
		t.Error("IsBmp should return false for single-byte slice")
	}
}

// TestLookupByMime_Found checks that "image/bmp" returns a non-nil codec.
func TestLookupByMime_Found(t *testing.T) {
	codec := bmp.LookupByMime("image/bmp")
	if codec == nil {
		t.Error("LookupByMime(\"image/bmp\") should not return nil")
	}
}

// TestLookupByMime_NotFound checks that an unknown MIME returns nil.
func TestLookupByMime_NotFound(t *testing.T) {
	codec := bmp.LookupByMime("image/png")
	if codec != nil {
		t.Error("LookupByMime(\"image/png\") should return nil")
	}
}
