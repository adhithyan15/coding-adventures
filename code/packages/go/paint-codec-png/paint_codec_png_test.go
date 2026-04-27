package paintcodecpng

import (
	"bytes"
	"testing"

	pixelcontainer "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
)

func TestEncodeAndDecodePNG(t *testing.T) {
	img := pixelcontainer.New(2, 1)
	pixelcontainer.SetPixel(img, 0, 0, 255, 255, 255, 255)
	pixelcontainer.SetPixel(img, 1, 0, 0, 0, 0, 255)

	encoded, err := EncodePNG(img)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.HasPrefix(encoded, []byte{0x89, 'P', 'N', 'G'}) {
		t.Fatalf("expected PNG signature, got %v", encoded[:4])
	}

	decoded, err := DecodePNG(encoded)
	if err != nil {
		t.Fatal(err)
	}
	if decoded.Width != 2 || decoded.Height != 1 {
		t.Fatalf("unexpected decoded size: %dx%d", decoded.Width, decoded.Height)
	}
	r, g, b, a := pixelcontainer.PixelAt(decoded, 1, 0)
	if r != 0 || g != 0 || b != 0 || a != 255 {
		t.Fatalf("unexpected decoded pixel: %d %d %d %d", r, g, b, a)
	}
}

func TestEncodeAndDecodeAliases(t *testing.T) {
	img := pixelcontainer.New(1, 1)
	pixelcontainer.SetPixel(img, 0, 0, 0x11, 0x22, 0x33, 0x44)

	encoded, err := Encode(img)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.HasPrefix(encoded, []byte{0x89, 'P', 'N', 'G'}) {
		t.Fatalf("expected PNG signature, got %v", encoded[:4])
	}

	decoded, err := Decode(encoded)
	if err != nil {
		t.Fatal(err)
	}
	r, g, b, a := pixelcontainer.PixelAt(decoded, 0, 0)
	if r != 0x11 || g != 0x22 || b != 0x33 || a != 0x44 {
		t.Fatalf("unexpected decoded pixel: %d %d %d %d", r, g, b, a)
	}
}
