package paintcodecpngnative

import (
	"bytes"
	"encoding/binary"
	"image/color"
	"image/png"
	"testing"

	pixelcontainer "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
)

var pngSignature = []byte{0x89, 'P', 'N', 'G', '\r', '\n', 0x1a, '\n'}

func TestSupportedRuntimeMatchesAvailability(t *testing.T) {
	if SupportedRuntime() != Available() {
		t.Fatalf("SupportedRuntime and Available should agree")
	}
}

func TestEncodeRejectsNil(t *testing.T) {
	if out, err := Encode(nil); err == nil || out != nil {
		t.Fatalf("Encode(nil) = (%v, %v), want an error", out, err)
	}
}

func TestEncodeRejectsInconsistentContainer(t *testing.T) {
	bad := &pixelcontainer.PixelContainer{Width: 2, Height: 2, Data: make([]byte, 5)}
	if out, err := Encode(bad); err == nil || out != nil {
		t.Fatalf("Encode(bad) = (%v, %v), want an error", out, err)
	}
}

func TestEncodeRejectsEmptyContainer(t *testing.T) {
	for _, dims := range [][2]uint32{{0, 0}, {0, 3}, {3, 0}} {
		empty := pixelcontainer.New(dims[0], dims[1])
		if out, err := Encode(empty); err == nil || out != nil {
			t.Fatalf("Encode(%dx%d) = (%v, %v), want an error", dims[0], dims[1], out, err)
		}
	}
}

func TestEncodeOnePixel(t *testing.T) {
	if !Available() {
		t.Skip("native PNG codec is not linked in this build")
	}

	pixels := pixelcontainer.New(1, 1)
	pixelcontainer.SetPixel(pixels, 0, 0, 0, 0, 0, 255)

	encoded, err := Encode(pixels)
	if err != nil {
		t.Fatalf("Encode returned error: %v", err)
	}
	if len(encoded) < 8 || !bytes.Equal(encoded[:8], pngSignature) {
		t.Fatalf("unexpected PNG signature: %v", encoded)
	}
}

// The IHDR chunk follows the signature: 4-byte length, "IHDR", then width and
// height as big-endian uint32s.
func TestEncodeWritesIHDRWithDimensions(t *testing.T) {
	if !Available() {
		t.Skip("native PNG codec is not linked in this build")
	}

	pixels := pixelcontainer.New(7, 3)
	encoded, err := Encode(pixels)
	if err != nil {
		t.Fatalf("Encode returned error: %v", err)
	}
	if len(encoded) < 24 || !bytes.Equal(encoded[:8], pngSignature) {
		t.Fatalf("not a PNG: % x", encoded)
	}
	if string(encoded[12:16]) != "IHDR" {
		t.Fatalf("first chunk is %q, want IHDR", encoded[12:16])
	}
	if w, h := binary.BigEndian.Uint32(encoded[16:20]), binary.BigEndian.Uint32(encoded[20:24]); w != 7 || h != 3 {
		t.Fatalf("IHDR dimensions = %dx%d, want 7x3", w, h)
	}
}

// Round-trip through the standard library decoder to prove the bytes are a real,
// decodable PNG carrying the pixels we put in (including translucent ones).
func TestEncodeRoundTripsPixels(t *testing.T) {
	if !Available() {
		t.Skip("native PNG codec is not linked in this build")
	}

	const w, h = 5, 4
	pixels := pixelcontainer.New(w, h)
	for y := uint32(0); y < h; y++ {
		for x := uint32(0); x < w; x++ {
			pixelcontainer.SetPixel(pixels, x, y, byte(x*50), byte(y*60), byte(x*y*10), 255-byte(x*20))
		}
	}

	encoded, err := Encode(pixels)
	if err != nil {
		t.Fatalf("Encode returned error: %v", err)
	}
	img, err := png.Decode(bytes.NewReader(encoded))
	if err != nil {
		t.Fatalf("encoded bytes do not decode as PNG: %v", err)
	}
	if b := img.Bounds(); b.Dx() != w || b.Dy() != h {
		t.Fatalf("decoded size = %dx%d, want %dx%d", b.Dx(), b.Dy(), w, h)
	}
	for y := 0; y < h; y++ {
		for x := 0; x < w; x++ {
			r, g, b, a := pixelcontainer.PixelAt(pixels, uint32(x), uint32(y))
			want := color.NRGBA{R: r, G: g, B: b, A: a}
			got := color.NRGBAModel.Convert(img.At(x, y)).(color.NRGBA)
			if got != want {
				t.Fatalf("pixel (%d,%d) = %v, want %v", x, y, got, want)
			}
		}
	}
}

// Encode returns a Go-owned copy: mutating the result must not affect a second
// encode of the same input (the Rust buffer is freed before Encode returns).
func TestEncodeIsRepeatableAndOwnsItsBytes(t *testing.T) {
	if !Available() {
		t.Skip("native PNG codec is not linked in this build")
	}

	pixels := pixelcontainer.New(16, 16)
	pixelcontainer.FillPixels(pixels, 10, 20, 30, 255)

	first, err := Encode(pixels)
	if err != nil {
		t.Fatalf("first Encode: %v", err)
	}
	want := append([]byte(nil), first...)
	for i := range first {
		first[i] = 0
	}
	second, err := Encode(pixels)
	if err != nil {
		t.Fatalf("second Encode: %v", err)
	}
	if !bytes.Equal(second, want) {
		t.Fatalf("second encode differs from the first")
	}
}

func TestEncodeUnavailableReportsError(t *testing.T) {
	if Available() {
		t.Skip("native PNG codec is linked; the unavailable path does not apply")
	}
	if _, err := Encode(pixelcontainer.New(1, 1)); err == nil {
		t.Fatalf("Encode should fail when the native codec is not linked")
	}
}
