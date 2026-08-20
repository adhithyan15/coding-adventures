package imagecodecpng_test

import (
	"encoding/binary"
	"errors"
	"math"
	"testing"

	png "github.com/adhithyan15/coding-adventures/code/packages/go/image-codec-png"
	pc "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
	zipcodec "github.com/adhithyan15/coding-adventures/code/packages/go/zip"
)

func requireCode(t *testing.T, err error, want png.PngErrorCode) {
	t.Helper()
	var pngErr *png.PngError
	if !errors.As(err, &pngErr) {
		t.Fatalf("error = %T %v, want *PngError", err, err)
	}
	if pngErr.Code != want || pngErr.Error() != string(want) {
		t.Fatalf("error = (%q,%q), want %q", pngErr.Code, pngErr.Error(), want)
	}
}

func TestPngCodecInterface(t *testing.T) {
	codec, err := png.NewPngCodec()
	if err != nil {
		t.Fatalf("NewPngCodec: %v", err)
	}
	var imageCodec pc.ImageCodec = codec
	if imageCodec.MimeType() != "image/png" {
		t.Fatalf("MIME type = %q", imageCodec.MimeType())
	}
	container := pc.New(1, 1)
	pc.SetPixel(container, 0, 0, 1, 2, 3, 4)
	encoded := imageCodec.Encode(container)
	decoded, err := imageCodec.Decode(encoded)
	if err != nil {
		t.Fatalf("codec Decode: %v", err)
	}
	if decoded.Width != 1 || decoded.Height != 1 || len(decoded.Data) != 4 {
		t.Fatalf("decoded container = %#v", decoded)
	}

	one := float64(1)
	limited, err := png.NewPngCodec(png.DecodeOptions{MaxPixels: &one})
	if err != nil {
		t.Fatalf("limited NewPngCodec: %v", err)
	}
	if _, err := limited.Decode(encoded); err != nil {
		t.Fatalf("limited Decode: %v", err)
	}
	invalidLimits := []struct {
		name  string
		value float64
	}{
		{"zero", 0},
		{"negative", -1},
		{"fractional", 1.5},
		{"raised", float64(png.PNGMaxPixels) + 1},
		{"nan", math.NaN()},
		{"positive-infinity", math.Inf(1)},
		{"negative-infinity", math.Inf(-1)},
	}
	for _, test := range invalidLimits {
		t.Run("invalid-limit-"+test.name, func(t *testing.T) {
			_, err := png.NewPngCodec(png.DecodeOptions{MaxPixels: &test.value})
			requireCode(t, err, png.InvalidMaxPixels)
		})
	}
	_, err = png.DecodePNG(encoded, png.DecodeOptions{}, png.DecodeOptions{})
	requireCode(t, err, png.InvalidMaxPixels)
}

func TestPngCodecEncodePanicPreservesTypedError(t *testing.T) {
	deferred := func() {
		recovered := recover()
		pngErr, ok := recovered.(*png.PngError)
		if !ok {
			t.Fatalf("panic = %T %v, want *PngError", recovered, recovered)
		}
		if pngErr.Code != png.InvalidImageDimensions {
			t.Fatalf("panic code = %q", pngErr.Code)
		}
	}
	defer deferred()
	var codec pc.ImageCodec = png.PngCodec{}
	codec.Encode(nil)
}

func TestPNGErrorCodesReturnsCopy(t *testing.T) {
	first := png.PNGErrorCodes()
	first[0] = "changed"
	if png.PNGErrorCodes()[0] != string(png.InvalidMaxPixels) {
		t.Fatal("PNGErrorCodes exposed mutable package state")
	}
}

func TestEncodePNGRejectsResourceAndShapeErrors(t *testing.T) {
	tests := []struct {
		name   string
		pixels *pc.PixelContainer
		code   png.PngErrorCode
	}{
		{"nil", nil, png.InvalidImageDimensions},
		{"zero-width", &pc.PixelContainer{Width: 0, Height: 1}, png.InvalidImageDimensions},
		{"edge-limit", &pc.PixelContainer{Width: png.PNGMaxDimension + 1, Height: 1}, png.InvalidImageDimensions},
		{"product-limit", &pc.PixelContainer{Width: 8192, Height: 4097}, png.InvalidImageDimensions},
		{"data-length", &pc.PixelContainer{Width: 1, Height: 1, Data: []byte{1, 2, 3}}, png.InvalidPixelDataLength},
	}
	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			_, err := png.EncodePNG(test.pixels)
			requireCode(t, err, test.code)
		})
	}
}

func insertChunkAfterIHDR(t *testing.T, encoded []byte, chunkType string) []byte {
	t.Helper()
	const ihdrEnd = 8 + 12 + 13
	chunk := make([]byte, 0, 12)
	chunk = binary.BigEndian.AppendUint32(chunk, 0)
	chunk = append(chunk, chunkType...)
	chunk = binary.BigEndian.AppendUint32(chunk, zipcodec.CRC32([]byte(chunkType), 0))
	result := make([]byte, 0, len(encoded)+len(chunk))
	result = append(result, encoded[:ihdrEnd]...)
	result = append(result, chunk...)
	result = append(result, encoded[ihdrEnd:]...)
	return result
}

func TestDecodePNGRejectsAPNGControlChunks(t *testing.T) {
	encoded, err := png.EncodePNG(pc.New(1, 1))
	if err != nil {
		t.Fatalf("EncodePNG: %v", err)
	}
	for _, chunkType := range []string{"acTL", "fcTL", "fdAT"} {
		t.Run(chunkType, func(t *testing.T) {
			_, err := png.DecodePNG(insertChunkAfterIHDR(t, encoded, chunkType))
			requireCode(t, err, png.UnsupportedFeature)
		})
	}
}
