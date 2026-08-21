package paintcodecpng

import (
	"bytes"
	"encoding/binary"
	"encoding/hex"
	"encoding/json"
	"errors"
	"hash/crc32"
	"image/png"
	"os"
	"path/filepath"
	"runtime"
	"testing"

	imagecodecpng "github.com/adhithyan15/coding-adventures/code/packages/go/image-codec-png"
	pixelcontainer "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
)

func requirePNGCode(t *testing.T, err error, want imagecodecpng.PngErrorCode) {
	t.Helper()
	var pngErr *imagecodecpng.PngError
	if !errors.As(err, &pngErr) {
		t.Fatalf("error = %T %v, want *imagecodecpng.PngError", err, err)
	}
	if pngErr.Code != want || pngErr.Error() != string(want) {
		t.Fatalf("error = (%q, %q), want %q", pngErr.Code, pngErr.Error(), want)
	}
}

func insertEmptyChunkAfterIHDR(t *testing.T, encoded []byte, chunkType string) []byte {
	t.Helper()
	const ihdrEnd = 8 + 12 + 13
	chunk := make([]byte, 0, 12)
	chunk = binary.BigEndian.AppendUint32(chunk, 0)
	chunk = append(chunk, chunkType...)
	chunk = binary.BigEndian.AppendUint32(chunk, crc32.ChecksumIEEE([]byte(chunkType)))
	result := make([]byte, 0, len(encoded)+len(chunk))
	result = append(result, encoded[:ihdrEnd]...)
	result = append(result, chunk...)
	return append(result, encoded[ihdrEnd:]...)
}

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
	canonical, err := imagecodecpng.EncodePNG(img)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(encoded, canonical) {
		t.Fatal("paint adapter output differs from the repository IC18 codec")
	}
	if _, err := png.Decode(bytes.NewReader(encoded)); err != nil {
		t.Fatalf("standard-library PNG decoder rejected adapter output: %v", err)
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

func TestPortableDecodeErrorsPropagateThroughEveryAlias(t *testing.T) {
	invalid := []byte("not a png")
	for name, decode := range map[string]func([]byte) (*pixelcontainer.PixelContainer, error){
		"Decode":          Decode,
		"DecodePNG":       DecodePNG,
		"PngCodec.Decode": (PngCodec{}).Decode,
	} {
		t.Run(name, func(t *testing.T) {
			_, err := decode(invalid)
			requirePNGCode(t, err, imagecodecpng.InvalidSignature)
		})
	}
}

func TestPortableResourceAndAPNGBoundaries(t *testing.T) {
	_, err := EncodePNG(nil)
	requirePNGCode(t, err, imagecodecpng.InvalidImageDimensions)

	_, err = EncodePNG(&pixelcontainer.PixelContainer{
		Width:  imagecodecpng.PNGMaxDimension + 1,
		Height: 1,
	})
	requirePNGCode(t, err, imagecodecpng.InvalidImageDimensions)

	base, err := imagecodecpng.EncodePNG(pixelcontainer.New(1, 1))
	if err != nil {
		t.Fatal(err)
	}
	_, err = DecodePNG(insertEmptyChunkAfterIHDR(t, base, "acTL"))
	requirePNGCode(t, err, imagecodecpng.UnsupportedFeature)
}

func TestRepresentativePortableFixtureErrorsThroughPaintAPI(t *testing.T) {
	_, filename, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("runtime.Caller could not locate the fixture consumer")
	}
	fixturePath := filepath.Clean(filepath.Join(
		filepath.Dir(filename), "..", "..", "..", "specs", "fixtures",
		"image-codec-png-v1", "cases.json",
	))
	data, err := os.ReadFile(fixturePath)
	if err != nil {
		t.Fatal(err)
	}
	var fixtures struct {
		SchemaVersion int    `json:"schema_version"`
		Profile       string `json:"profile"`
		Cases         []struct {
			ID       string `json:"id"`
			PNGHex   string `json:"png_hex"`
			Expected struct {
				ErrorID string `json:"error_id"`
			} `json:"expected"`
		} `json:"cases"`
	}
	if err := json.Unmarshal(data, &fixtures); err != nil {
		t.Fatal(err)
	}
	if fixtures.SchemaVersion != 1 || fixtures.Profile != "image-codec-png-v1" || len(fixtures.Cases) != 85 {
		t.Fatalf("fixture contract = schema %d profile %q cases %d", fixtures.SchemaVersion, fixtures.Profile, len(fixtures.Cases))
	}
	wanted := map[string]imagecodecpng.PngErrorCode{
		"png-v1-error-edge-limit":  imagecodecpng.DimensionLimit,
		"png-v1-error-pixel-limit": imagecodecpng.PixelLimit,
		"png-v1-error-apng-actl":   imagecodecpng.UnsupportedFeature,
		"png-v1-error-idat-cavity": imagecodecpng.IDATCavity,
		"png-v1-error-filter":      imagecodecpng.InvalidFilter,
	}
	seen := make(map[string]bool, len(wanted))
	for _, fixture := range fixtures.Cases {
		code, selected := wanted[fixture.ID]
		if !selected {
			continue
		}
		encoded, err := hex.DecodeString(fixture.PNGHex)
		if err != nil {
			t.Fatalf("%s: decode hex: %v", fixture.ID, err)
		}
		_, err = DecodePNG(encoded)
		requirePNGCode(t, err, code)
		if fixture.Expected.ErrorID != string(code) {
			t.Fatalf("%s: fixture error = %q, selected code = %q", fixture.ID, fixture.Expected.ErrorID, code)
		}
		seen[fixture.ID] = true
	}
	if len(seen) != len(wanted) {
		t.Fatalf("representative fixture coverage = %v, want %v", seen, wanted)
	}
}

func TestCodecEncodePanicPreservesTypedError(t *testing.T) {
	defer func() {
		recovered := recover()
		pngErr, ok := recovered.(*imagecodecpng.PngError)
		if !ok {
			t.Fatalf("panic = %T %v, want *imagecodecpng.PngError", recovered, recovered)
		}
		if pngErr.Code != imagecodecpng.InvalidImageDimensions {
			t.Fatalf("panic code = %q", pngErr.Code)
		}
	}()
	(PngCodec{}).Encode(nil)
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

func TestPngCodecSuccessfulInterfacePath(t *testing.T) {
	codec := PngCodec{}
	if codec.MimeType() != "image/png" {
		t.Fatalf("MIME type = %q", codec.MimeType())
	}
	var alias Codec = codec
	img := pixelcontainer.New(1, 1)
	pixelcontainer.SetPixel(img, 0, 0, 9, 8, 7, 6)
	encoded := alias.Encode(img)
	decoded, err := alias.Decode(encoded)
	if err != nil {
		t.Fatal(err)
	}
	r, g, b, a := pixelcontainer.PixelAt(decoded, 0, 0)
	if r != 9 || g != 8 || b != 7 || a != 6 {
		t.Fatalf("unexpected decoded pixel: %d %d %d %d", r, g, b, a)
	}
}
