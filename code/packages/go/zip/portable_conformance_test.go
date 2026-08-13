package zip_test

import (
	"bytes"
	"compress/flate"
	"encoding/binary"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"testing"

	zip "github.com/adhithyan15/coding-adventures/code/packages/go/zip"
)

type rawFixture struct {
	Limits struct {
		Default int64 `json:"default_max_output"`
		Hard    int64 `json:"hard_max_output"`
	} `json:"limits"`
	Cases []rawCase `json:"cases"`
}

type rawCase struct {
	ID           string   `json:"id"`
	Operation    string   `json:"operation"`
	InputHex     string   `json:"input_hex"`
	ChunksHex    []string `json:"chunks_hex"`
	InitialCRC32 string   `json:"initial_crc32_hex"`
	MaxOutput    *int64   `json:"max_output"`
	Expected     struct {
		Output struct {
			Hex       string `json:"hex"`
			RepeatHex string `json:"repeat_hex"`
			Count     int    `json:"count"`
		} `json:"output"`
		BytesConsumed int    `json:"bytes_consumed"`
		ErrorID       string `json:"error_id"`
		CRC32Hex      string `json:"crc32_hex"`
	} `json:"expected"`
}

func loadRawFixture(t *testing.T) rawFixture {
	t.Helper()
	path := filepath.Join("..", "..", "..", "specs", "fixtures", "zip-raw-rfc1951-v1", "cases.json")
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	var fixture rawFixture
	if err := json.Unmarshal(data, &fixture); err != nil {
		t.Fatal(err)
	}
	return fixture
}

func decodeHex(t *testing.T, value string) []byte {
	t.Helper()
	decoded, err := hex.DecodeString(value)
	if err != nil {
		t.Fatal(err)
	}
	return decoded
}

func expectedBytes(t *testing.T, testCase rawCase) []byte {
	t.Helper()
	if testCase.Expected.Output.RepeatHex == "" {
		return decodeHex(t, testCase.Expected.Output.Hex)
	}
	unit := decodeHex(t, testCase.Expected.Output.RepeatHex)
	return bytes.Repeat(unit, testCase.Expected.Output.Count)
}

func inflateLimit(testCase rawCase) int64 {
	if testCase.MaxOutput == nil {
		return zip.RawInflateMaxOutput
	}
	return *testCase.MaxOutput
}

func TestPortableRawRFC1951Conformance(t *testing.T) {
	fixture := loadRawFixture(t)
	if len(fixture.Cases) != 34 {
		t.Fatalf("closed fixture count = %d, want 34", len(fixture.Cases))
	}
	if fixture.Limits.Default != zip.RawInflateMaxOutput || fixture.Limits.Hard != zip.RawInflateMaxOutput {
		t.Fatalf("fixture limits do not match RawInflateMaxOutput")
	}

	for _, testCase := range fixture.Cases {
		testCase := testCase
		t.Run(testCase.ID, func(t *testing.T) {
			switch testCase.Operation {
			case "inflate":
				input := decodeHex(t, testCase.InputHex)
				result, err := zip.RawInflateCounted(input, inflateLimit(testCase))
				if err != nil {
					t.Fatal(err)
				}
				want := expectedBytes(t, testCase)
				if !bytes.Equal(result.Output, want) {
					t.Fatalf("output mismatch: got %d bytes, want %d", len(result.Output), len(want))
				}
				if result.BytesConsumed != testCase.Expected.BytesConsumed {
					t.Fatalf("bytes consumed = %d, want %d", result.BytesConsumed, testCase.Expected.BytesConsumed)
				}
				plain, err := zip.RawInflate(input, inflateLimit(testCase))
				if err != nil || !bytes.Equal(plain, want) {
					t.Fatalf("uncounted wrapper mismatch: %v", err)
				}
			case "inflate-error":
				result, err := zip.RawInflateCounted(decodeHex(t, testCase.InputHex), inflateLimit(testCase))
				if err == nil {
					t.Fatalf("unexpected success with %d output bytes", len(result.Output))
				}
				if len(result.Output) != 0 {
					t.Fatalf("error exposed %d partial output bytes", len(result.Output))
				}
				var typed *zip.RawInflateError
				if !errors.As(err, &typed) {
					t.Fatalf("error type = %T, want *RawInflateError", err)
				}
				if string(typed.Code) != testCase.Expected.ErrorID {
					t.Fatalf("error code = %q, want %q", typed.Code, testCase.Expected.ErrorID)
				}
				if err.Error() != testCase.Expected.ErrorID {
					t.Fatalf("error message is not the stable payload-blind identifier: %q", err)
				}
			case "deflate-interoperability":
				encoded := zip.RawDeflate(decodeHex(t, testCase.InputHex))
				reader := flate.NewReader(bytes.NewReader(encoded))
				decoded, err := io.ReadAll(reader)
				closeErr := reader.Close()
				if err != nil || closeErr != nil {
					t.Fatalf("independent flate decode: read=%v close=%v", err, closeErr)
				}
				if !bytes.Equal(decoded, expectedBytes(t, testCase)) {
					t.Fatal("independent decoder output mismatch")
				}
			case "crc32":
				var checksum uint32
				if testCase.InitialCRC32 != "" {
					if _, err := fmt.Sscanf(testCase.InitialCRC32, "%08x", &checksum); err != nil {
						t.Fatal(err)
					}
				}
				for _, chunk := range testCase.ChunksHex {
					checksum = zip.CRC32(decodeHex(t, chunk), checksum)
				}
				if got := fmt.Sprintf("%08x", checksum); got != testCase.Expected.CRC32Hex {
					t.Fatalf("CRC-32 = %s, want %s", got, testCase.Expected.CRC32Hex)
				}
			default:
				t.Fatalf("unknown fixture operation %q", testCase.Operation)
			}
		})
	}
}

func writeLE(buffer *bytes.Buffer, value any) {
	if err := binary.Write(buffer, binary.LittleEndian, value); err != nil {
		panic(err)
	}
}

func zipWithRawPayload(name string, compressed, uncompressed []byte, declaredSize uint32) []byte {
	var archive bytes.Buffer
	nameBytes := []byte(name)
	checksum := zip.CRC32(uncompressed, 0)

	writeLE(&archive, uint32(0x04034b50))
	writeLE(&archive, uint16(20))
	writeLE(&archive, uint16(0x0800))
	writeLE(&archive, uint16(8))
	writeLE(&archive, uint16(0))
	writeLE(&archive, uint16(0))
	writeLE(&archive, checksum)
	writeLE(&archive, uint32(len(compressed)))
	writeLE(&archive, declaredSize)
	writeLE(&archive, uint16(len(nameBytes)))
	writeLE(&archive, uint16(0))
	archive.Write(nameBytes)
	archive.Write(compressed)

	centralOffset := uint32(archive.Len())
	writeLE(&archive, uint32(0x02014b50))
	writeLE(&archive, uint16(0x031e))
	writeLE(&archive, uint16(20))
	writeLE(&archive, uint16(0x0800))
	writeLE(&archive, uint16(8))
	writeLE(&archive, uint16(0))
	writeLE(&archive, uint16(0))
	writeLE(&archive, checksum)
	writeLE(&archive, uint32(len(compressed)))
	writeLE(&archive, declaredSize)
	writeLE(&archive, uint16(len(nameBytes)))
	writeLE(&archive, uint16(0))
	writeLE(&archive, uint16(0))
	writeLE(&archive, uint16(0))
	writeLE(&archive, uint16(0))
	writeLE(&archive, uint32(0))
	writeLE(&archive, uint32(0))
	archive.Write(nameBytes)

	centralSize := uint32(archive.Len()) - centralOffset
	writeLE(&archive, uint32(0x06054b50))
	writeLE(&archive, uint16(0))
	writeLE(&archive, uint16(0))
	writeLE(&archive, uint16(1))
	writeLE(&archive, uint16(1))
	writeLE(&archive, centralSize)
	writeLE(&archive, centralOffset)
	writeLE(&archive, uint16(0))
	return archive.Bytes()
}

func TestZipReaderUsesStrictCountedRawInflate(t *testing.T) {
	dynamic := decodeHex(t, "0dc28911c0200c03b0d8f97028ec3f6ed129cab7dd96a0c2445bdb93809663a5d303f6b265e20c2b79ea03379d227e")
	want := decodeHex(t, "0406030b000e070909010906010a04070007000000000501010908030108050302030401000401000207090009020a0a020605020d060c01020b020302090201")

	archive := zipWithRawPayload("dynamic.bin", dynamic, want, uint32(len(want)))
	reader, err := zip.NewZipReader(archive)
	if err != nil {
		t.Fatal(err)
	}
	got, err := reader.Read(reader.Entries()[0])
	if err != nil || !bytes.Equal(got, want) {
		t.Fatalf("dynamic ZIP read: %v", err)
	}

	cavity := append(append([]byte(nil), dynamic...), 0xde, 0xad)
	reader, err = zip.NewZipReader(zipWithRawPayload("cavity.bin", cavity, want, uint32(len(want))))
	if err != nil {
		t.Fatal(err)
	}
	if _, err := reader.Read(reader.Entries()[0]); err == nil || err.Error() != "zip: compressed payload contains trailing bytes" {
		t.Fatalf("suffix cavity error = %v", err)
	}

	reader, err = zip.NewZipReader(zipWithRawPayload("size.bin", dynamic, want, uint32(len(want)+1)))
	if err != nil {
		t.Fatal(err)
	}
	if _, err := reader.Read(reader.Entries()[0]); err == nil || err.Error() != "zip: uncompressed size does not match the directory" {
		t.Fatalf("declared size error = %v", err)
	}
}

func TestRawInflateRejectsAmbiguousLimitArguments(t *testing.T) {
	_, err := zip.RawInflateCounted([]byte{1, 0, 0, 0xff, 0xff}, 0, 0)
	var typed *zip.RawInflateError
	if !errors.As(err, &typed) || typed.Code != zip.InvalidOutputLimit {
		t.Fatalf("error = %v, want invalid-output-limit", err)
	}
}

func TestRawInflateFullWindowForeignStream(t *testing.T) {
	prefix := make([]byte, 32768)
	for i := range prefix {
		prefix[i] = byte((i*73 + i/251) & 0xff)
	}
	want := append(append([]byte(nil), prefix...), prefix...)
	var compressed bytes.Buffer
	writer, err := flate.NewWriter(&compressed, flate.BestCompression)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := writer.Write(want); err != nil {
		t.Fatal(err)
	}
	if err := writer.Close(); err != nil {
		t.Fatal(err)
	}
	got, err := zip.RawInflate(compressed.Bytes(), int64(len(want)))
	if err != nil || !bytes.Equal(got, want) {
		t.Fatalf("full-window foreign stream: %v", err)
	}
}
