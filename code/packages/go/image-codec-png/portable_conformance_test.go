package imagecodecpng_test

import (
	"bytes"
	"compress/zlib"
	"encoding/binary"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"image/color"
	stdpng "image/png"
	"io"
	"math"
	"os"
	"path/filepath"
	"reflect"
	"runtime"
	"testing"

	png "github.com/adhithyan15/coding-adventures/code/packages/go/image-codec-png"
	pc "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
)

type fixturePixels struct {
	Width   float64 `json:"width"`
	Height  float64 `json:"height"`
	RGBAHex string  `json:"rgba_hex"`
}

type fixtureOptions struct {
	MaxPixels float64 `json:"max_pixels"`
}

type fixtureExpected struct {
	Width       float64  `json:"width"`
	Height      float64  `json:"height"`
	RGBAHex     string   `json:"rgba_hex"`
	ErrorID     string   `json:"error_id"`
	ChunkTypes  []string `json:"chunk_types"`
	FilterTypes []byte   `json:"filter_types"`
	BitDepth    byte     `json:"bit_depth"`
	ColourType  byte     `json:"colour_type"`
	Interlace   byte     `json:"interlace"`
	Adler32Hex  string   `json:"adler32_hex"`
}

type fixtureCase struct {
	ID        string          `json:"id"`
	Operation string          `json:"operation"`
	PNGHex    string          `json:"png_hex"`
	InputHex  string          `json:"input_hex"`
	Input     *fixturePixels  `json:"input"`
	Options   *fixtureOptions `json:"options"`
	Expected  fixtureExpected `json:"expected"`
}

type fixtureDocument struct {
	Limits struct {
		MaxDimension     uint32 `json:"max_dimension"`
		DefaultMaxPixels uint64 `json:"default_max_pixels"`
	} `json:"limits"`
	ErrorIDs []string      `json:"error_ids"`
	Cases    []fixtureCase `json:"cases"`
}

type pngChunk struct {
	Type string
	Data []byte
}

func fixturePath(t *testing.T) string {
	t.Helper()
	_, filename, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("runtime.Caller could not locate the fixture consumer")
	}
	return filepath.Clean(filepath.Join(filepath.Dir(filename), "..", "..", "..", "specs", "fixtures", "image-codec-png-v1", "cases.json"))
}

func loadFixtures(t *testing.T) fixtureDocument {
	t.Helper()
	data, err := os.ReadFile(fixturePath(t))
	if err != nil {
		t.Fatalf("read fixture corpus: %v", err)
	}
	var fixtures fixtureDocument
	if err := json.Unmarshal(data, &fixtures); err != nil {
		t.Fatalf("parse fixture corpus: %v", err)
	}
	return fixtures
}

func fromHex(t *testing.T, value string) []byte {
	t.Helper()
	data, err := hex.DecodeString(value)
	if err != nil {
		t.Fatalf("decode fixture hex: %v", err)
	}
	return data
}

func decodeOptions(fixture *fixtureOptions) []png.DecodeOptions {
	if fixture == nil {
		return nil
	}
	value := fixture.MaxPixels
	return []png.DecodeOptions{{MaxPixels: &value}}
}

func fixtureEncode(t *testing.T, input fixturePixels) ([]byte, error) {
	t.Helper()
	if math.IsNaN(input.Width) || math.IsInf(input.Width, 0) ||
		math.IsNaN(input.Height) || math.IsInf(input.Height, 0) ||
		math.Trunc(input.Width) != input.Width || math.Trunc(input.Height) != input.Height ||
		input.Width < 0 || input.Height < 0 ||
		input.Width > math.MaxUint32 || input.Height > math.MaxUint32 {
		// PixelContainer has uint32 dimensions, so a fixture adapter must reject
		// non-integral JSON values before converting them to the typed boundary.
		return nil, &png.PngError{Code: png.InvalidImageDimensions}
	}
	container := &pc.PixelContainer{
		Width:  uint32(input.Width),
		Height: uint32(input.Height),
		Data:   fromHex(t, input.RGBAHex),
	}
	return png.EncodePNG(container)
}

func assertPngError(t *testing.T, err error, want string) {
	t.Helper()
	if err == nil {
		t.Fatal("fixture unexpectedly succeeded")
	}
	var pngErr *png.PngError
	if !errors.As(err, &pngErr) {
		t.Fatalf("error type = %T, want *PngError", err)
	}
	if string(pngErr.Code) != want {
		t.Fatalf("error code = %q, want %q", pngErr.Code, want)
	}
}

func parseChunks(t *testing.T, encoded []byte) []pngChunk {
	t.Helper()
	var chunks []pngChunk
	for offset := 8; offset < len(encoded); {
		if len(encoded)-offset < 12 {
			t.Fatalf("encoded PNG has truncated chunk at %d", offset)
		}
		length := int(binary.BigEndian.Uint32(encoded[offset : offset+4]))
		end := offset + 12 + length
		if end > len(encoded) {
			t.Fatalf("encoded PNG chunk exceeds output at %d", offset)
		}
		chunks = append(chunks, pngChunk{
			Type: string(encoded[offset+4 : offset+8]),
			Data: append([]byte(nil), encoded[offset+8:offset+8+length]...),
		})
		offset = end
	}
	return chunks
}

func foreignRGBA(t *testing.T, encoded []byte) (uint32, uint32, []byte) {
	t.Helper()
	decoded, err := stdpng.Decode(bytes.NewReader(encoded))
	if err != nil {
		t.Fatalf("standard-library PNG decode: %v", err)
	}
	bounds := decoded.Bounds()
	width, height := bounds.Dx(), bounds.Dy()
	rgba := make([]byte, 0, width*height*4)
	for y := bounds.Min.Y; y < bounds.Max.Y; y++ {
		for x := bounds.Min.X; x < bounds.Max.X; x++ {
			pixel := color.NRGBAModel.Convert(decoded.At(x, y)).(color.NRGBA)
			rgba = append(rgba, pixel.R, pixel.G, pixel.B, pixel.A)
		}
	}
	return uint32(width), uint32(height), rgba
}

func filteredBytes(t *testing.T, chunks []pngChunk) []byte {
	t.Helper()
	var idat bytes.Buffer
	for _, chunk := range chunks {
		if chunk.Type == "IDAT" {
			idat.Write(chunk.Data)
		}
	}
	reader, err := zlib.NewReader(bytes.NewReader(idat.Bytes()))
	if err != nil {
		t.Fatalf("standard-library zlib reader: %v", err)
	}
	filtered, err := io.ReadAll(reader)
	if err != nil {
		t.Fatalf("standard-library zlib inflate: %v", err)
	}
	if err := reader.Close(); err != nil {
		t.Fatalf("close standard-library zlib reader: %v", err)
	}
	return filtered
}

func TestPortableConformance(t *testing.T) {
	fixtures := loadFixtures(t)
	if len(fixtures.Cases) != 82 {
		t.Fatalf("portable case count = %d, want 82", len(fixtures.Cases))
	}
	if fixtures.Limits.MaxDimension != png.PNGMaxDimension || fixtures.Limits.DefaultMaxPixels != png.PNGMaxPixels {
		t.Fatalf("public limits = (%d,%d), fixture = (%d,%d)", png.PNGMaxDimension, png.PNGMaxPixels, fixtures.Limits.MaxDimension, fixtures.Limits.DefaultMaxPixels)
	}
	if !reflect.DeepEqual(png.PNGErrorCodes(), fixtures.ErrorIDs) {
		t.Fatalf("error taxonomy = %#v, want %#v", png.PNGErrorCodes(), fixtures.ErrorIDs)
	}

	for _, fixture := range fixtures.Cases {
		fixture := fixture
		t.Run(fixture.ID, func(t *testing.T) {
			switch fixture.Operation {
			case "decode":
				actual, err := png.DecodePNG(fromHex(t, fixture.PNGHex), decodeOptions(fixture.Options)...)
				if err != nil {
					t.Fatalf("DecodePNG: %v", err)
				}
				if actual.Width != uint32(fixture.Expected.Width) || actual.Height != uint32(fixture.Expected.Height) {
					t.Fatalf("dimensions = %dx%d, want %.0fx%.0f", actual.Width, actual.Height, fixture.Expected.Width, fixture.Expected.Height)
				}
				if want := fromHex(t, fixture.Expected.RGBAHex); !bytes.Equal(actual.Data, want) {
					t.Fatalf("RGBA = %x, want %x", actual.Data, want)
				}
			case "decode-error":
				_, err := png.DecodePNG(fromHex(t, fixture.PNGHex), decodeOptions(fixture.Options)...)
				assertPngError(t, err, fixture.Expected.ErrorID)
			case "encode":
				encoded, err := fixtureEncode(t, *fixture.Input)
				if err != nil {
					t.Fatalf("EncodePNG: %v", err)
				}
				chunks := parseChunks(t, encoded)
				gotTypes := make([]string, len(chunks))
				for i := range chunks {
					gotTypes[i] = chunks[i].Type
				}
				if !reflect.DeepEqual(gotTypes, fixture.Expected.ChunkTypes) {
					t.Fatalf("chunk types = %#v, want %#v", gotTypes, fixture.Expected.ChunkTypes)
				}
				if encoded[24] != fixture.Expected.BitDepth || encoded[25] != fixture.Expected.ColourType || encoded[28] != fixture.Expected.Interlace {
					t.Fatalf("IHDR profile = (%d,%d,%d), want (%d,%d,%d)", encoded[24], encoded[25], encoded[28], fixture.Expected.BitDepth, fixture.Expected.ColourType, fixture.Expected.Interlace)
				}
				filtered := filteredBytes(t, chunks)
				stride := int(fixture.Input.Width) * 4
				filters := make([]byte, int(fixture.Input.Height))
				for row := range filters {
					filters[row] = filtered[row*(stride+1)]
				}
				if !bytes.Equal(filters, fixture.Expected.FilterTypes) {
					t.Fatalf("filter types = %v, want %v", filters, fixture.Expected.FilterTypes)
				}
				width, height, rgba := foreignRGBA(t, encoded)
				if width != uint32(fixture.Input.Width) || height != uint32(fixture.Input.Height) || !bytes.Equal(rgba, fromHex(t, fixture.Input.RGBAHex)) {
					t.Fatalf("foreign decode = %dx%d %x", width, height, rgba)
				}
			case "encode-error":
				_, err := fixtureEncode(t, *fixture.Input)
				assertPngError(t, err, fixture.Expected.ErrorID)
			case "adler32":
				got := fmt.Sprintf("%08x", png.Adler32(fromHex(t, fixture.InputHex)))
				if got != fixture.Expected.Adler32Hex {
					t.Fatalf("Adler32 = %s, want %s", got, fixture.Expected.Adler32Hex)
				}
			default:
				t.Fatalf("unknown fixture operation %q", fixture.Operation)
			}
		})
	}
}
